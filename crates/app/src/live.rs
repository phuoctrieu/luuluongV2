//! Mực nước hồ hiện tại và lưu lượng về theo chu kỳ, từ data logger ECAPRO.
//! Thuần logic (thời gian truyền vào), không phụ thuộc giao diện.

use luuluong_core::{inflow_between_levels, VolumeTable};
use luuluong_logger::tracker::{IntervalState, LevelTracker, Mark, OutflowMean};
use luuluong_logger::{is_running, LoggerError, Reading, Status};
use luuluong_storage::history::{vn_timestamp, Record};

/// Múi giờ Việt Nam (giây), cố định UTC+7.
pub const VN_TZ: i32 = 7 * 3600;

/// Lưu lượng về của một chu kỳ đã đủ hai mốc.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub from: Mark,
    pub to: Mark,
    /// Mực nước mốc sau trừ mốc trước (m).
    pub dz: f64,
    /// Lưu lượng tổ máy, xả môi trường trung bình và tỷ lệ thời gian chạy trong chu kỳ.
    pub outflow: OutflowMean,
    /// m³/s, đã cộng tuabin và xả môi trường đo được.
    pub q_in: f64,
}

/// Một tổ máy theo lần đọc gần nhất.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machine {
    /// m³/s.
    pub q: f64,
    pub running: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Machines {
    pub h1: Machine,
    pub h2: Machine,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlowView {
    /// Chưa qua mốc nào.
    Waiting,
    /// Chu kỳ vừa qua thiếu mực nước ở một mốc.
    Missing { to: i64 },
    /// Đủ hai mốc nhưng ngoài bảng dung tích (149–165 m) hoặc lỗi tính; chứa thông báo tiếng Việt.
    Invalid(String),
    Ready(Flow),
}

/// Mốc đồng hồ kế tiếp (giây epoch) sau `now`, với chu kỳ 15 hoặc 30 phút.
pub fn next_boundary(now: i64, period_minutes: u32) -> i64 {
    let p = i64::from(period_minutes) * 60;
    (now.div_euclid(p) + 1) * p
}

/// Ghi chú đánh dấu dòng lịch sử do ứng dụng tự lưu từ thiết bị.
pub const AUTO_NOTE: &str = "Tự động từ thiết bị ECAPRO";

pub fn is_automatic(r: &Record) -> bool {
    r.note == AUTO_NOTE
}

fn mm(z: f64) -> f64 {
    (z * 1000.0).round() / 1000.0
}

/// Dòng lịch sử cho một chu kỳ: Z đầu/cuối là hai mốc, giờ chạy H1/H2 theo thực tế, không xả tràn.
pub fn history_record(f: &Flow, period_minutes: u32) -> Record {
    Record {
        id: 0,
        created_at: vn_timestamp(f.to.time),
        z_start: mm(f.from.z),
        z_end: mm(f.to.z),
        hours: f64::from(period_minutes) / 60.0,
        h1: f.outflow.run_h1 * f64::from(period_minutes) / 60.0,
        h2: f.outflow.run_h2 * f64::from(period_minutes) / 60.0,
        spill_q: 0.0,
        spill_h: 0.0,
        eco_q: f.outflow.q_eco,
        q_in: f.q_in,
        w_in: f.q_in * f64::from(period_minutes) * 60.0,
        operator: String::new(),
        note: AUTO_NOTE.into(),
    }
}

pub struct Live {
    tracker: LevelTracker,
    period: u32,
    last: Option<Result<Reading, LoggerError>>,
    latest: Option<(i64, f64)>,
    machines: Option<(i64, Machines)>,
    logged_to: Option<i64>,
}

impl Live {
    pub fn new(period_minutes: u32) -> Self {
        Self { tracker: LevelTracker::new(period_minutes), period: period_minutes, last: None, latest: None, machines: None, logged_to: None }
    }

    /// Đổi chu kỳ: chốt lại từ đầu vì các mốc cũ nằm trên lưới khác.
    pub fn set_period(&mut self, period_minutes: u32) {
        if period_minutes != self.period {
            self.period = period_minutes;
            self.tracker = LevelTracker::new(period_minutes);
        }
    }

    /// Mực nước ở mốc đồng hồ gần nhất (làm "mực nước chu kỳ trước" cho tính bằng tay).
    pub fn last_mark(&self) -> Option<Mark> {
        self.tracker.last_mark()
    }

    /// Chu kỳ vừa tính xong mà chưa được lấy lần nào (mỗi chu kỳ chỉ trả một lần, để lưu lịch sử).
    pub fn take_unlogged(&mut self, table: &VolumeTable) -> Option<Flow> {
        match self.flow(table) {
            FlowView::Ready(f) if self.logged_to != Some(f.to.time) => {
                self.logged_to = Some(f.to.time);
                Some(f)
            }
            _ => None,
        }
    }

    pub fn period(&self) -> u32 {
        self.period
    }

    pub fn record(&mut self, now: i64, result: Result<Reading, LoggerError>) {
        let flows = result.as_ref().ok().and_then(|r| Some((r.q_h1?, r.q_h2?, r.q_eco?)));
        if let Some((q1, q2, _)) = flows {
            let machine = |q: f64| Machine { q, running: is_running(q) };
            self.machines = Some((now, Machines { h1: machine(q1), h2: machine(q2) }));
        }
        self.tracker.push_flows(now, flows);
        match &result {
            Ok(r) => {
                self.latest = Some((now, r.z));
                self.tracker.push(now, Some(r.z));
            }
            Err(_) => self.tracker.push(now, None),
        }
        self.last = Some(result);
    }

    /// Mực nước hợp lệ mới nhất (giây, m).
    pub fn level(&self) -> Option<(i64, f64)> {
        self.latest
    }

    /// Tình trạng hai tổ máy theo lần đọc gần nhất có đủ số liệu (giây, tình trạng).
    pub fn machines(&self) -> Option<(i64, Machines)> {
        self.machines
    }

    /// Lỗi của lần đọc gần nhất, nếu có.
    pub fn error(&self) -> Option<&LoggerError> {
        self.last.as_ref().and_then(|r| r.as_ref().err())
    }

    /// Trạng thái cảm biến theo lần đọc gần nhất (None nếu chưa đọc được lần nào).
    pub fn status(&self) -> Option<Status> {
        self.last.as_ref().and_then(|r| r.as_ref().ok()).map(|r| r.status)
    }

    /// Quá 3 chu kỳ đọc không có mực nước hợp lệ mới.
    pub fn stale(&self, now: i64, poll_secs: u32) -> bool {
        self.latest.is_none_or(|(t, _)| now - t > 3 * i64::from(poll_secs))
    }

    pub fn flow(&self, table: &VolumeTable) -> FlowView {
        match self.tracker.state() {
            IntervalState::NoData => FlowView::Waiting,
            IntervalState::Missing { to } => FlowView::Missing { to },
            IntervalState::Ready(i) => match inflow_between_levels(table, i.from.z, i.to.z, f64::from(self.period), i.outflow.q_eco, i.outflow.q_h1, i.outflow.q_h2) {
                Ok(o) => FlowView::Ready(Flow { from: i.from, to: i.to, dz: i.to.z - i.from.z, outflow: i.outflow, q_in: o.q_in }),
                Err(e) => FlowView::Invalid(e.to_string()),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_800 * 1_000_000;

    fn ok(z: f64) -> Result<Reading, LoggerError> {
        ok_with(z, 0.0, 0.0)
    }

    fn ok_with(z: f64, q1: f64, q2: f64) -> Result<Reading, LoggerError> {
        ok_eco(z, q1, q2, 1.5)
    }

    fn ok_eco(z: f64, q1: f64, q2: f64, eco: f64) -> Result<Reading, LoggerError> {
        Ok(Reading { q_h1: Some(q1), q_h2: Some(q2), q_eco: Some(eco), ..Reading::new(z, Status::Normal) })
    }

    fn feed(l: &mut Live, from: i64, to: i64, r: Result<Reading, LoggerError>) {
        let mut t = from;
        while t < to {
            l.record(t, r.clone());
            t += 5;
        }
    }

    fn two_periods_without_machines(l: &mut Live) {
        let plain = Ok(Reading::new(157.0, Status::Normal));
        feed(l, T0 - 120, T0 + 900 + 20, plain);
    }

    fn two_periods(l: &mut Live, z0: f64, z1: f64) {
        feed(l, T0 - 120, T0 + 900 - 60, ok(z0));
        feed(l, T0 + 900 - 60, T0 + 900 + 20, ok(z1));
    }

    #[test]
    fn next_boundary_is_strictly_after_now_on_the_clock_grid() {
        assert_eq!(next_boundary(T0 + 1, 15), T0 + 900);
        assert_eq!(next_boundary(T0, 15), T0 + 900);
        assert_eq!(next_boundary(T0 + 899, 15), T0 + 900);
        assert_eq!(next_boundary(T0 + 901, 30), T0 + 1_800);
    }

    #[test]
    fn last_mark_follows_the_tracker() {
        let mut l = Live::new(15);
        assert_eq!(l.last_mark(), None);
        two_periods(&mut l, 160.0, 160.1);
        assert_eq!(l.last_mark(), Some(Mark { time: T0 + 900, z: 160.1 }));
    }

    #[test]
    fn history_record_maps_a_flow_without_turbine_or_spill() {
        let mut l = Live::new(15);
        two_periods(&mut l, 160.0, 160.1);
        let FlowView::Ready(f) = l.flow(&VolumeTable::builtin()) else { panic!() };
        let r = history_record(&f, 15);
        assert_eq!(r.created_at, vn_timestamp(T0 + 900));
        assert!(r.created_at.ends_with("+07:00"));
        assert_eq!((r.z_start, r.z_end, r.hours, r.eco_q), (160.0, 160.1, 0.25, 1.5));
        assert_eq!((r.h1, r.h2, r.spill_q, r.spill_h), (0.0, 0.0, 0.0, 0.0));
        assert_eq!(r.q_in, f.q_in);
        assert!((r.w_in - f.q_in * 900.0).abs() < 1e-6);
        assert!(is_automatic(&r));
    }

    #[test]
    fn history_levels_are_rounded_to_a_millimetre() {
        let outflow = OutflowMean { q_h1: 0.0, q_h2: 0.0, run_h1: 0.0, run_h2: 0.0, q_eco: 1.5 };
        let f = Flow { from: Mark { time: T0, z: 157.19200000000001 }, to: Mark { time: T0 + 900, z: 157.2499999999 }, dz: 0.058, outflow, q_in: 2.0 };
        let r = history_record(&f, 15);
        assert_eq!((r.z_start, r.z_end), (157.192, 157.25));
    }

    #[test]
    fn hand_calculations_are_not_automatic() {
        let mut l = Live::new(15);
        two_periods(&mut l, 160.0, 160.1);
        let FlowView::Ready(f) = l.flow(&VolumeTable::builtin()) else { panic!() };
        let mut r = history_record(&f, 15);
        r.note = String::new();
        assert!(!is_automatic(&r));
    }

    #[test]
    fn each_finished_interval_is_handed_out_once() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15);
        assert!(l.take_unlogged(&t).is_none());
        two_periods(&mut l, 160.0, 160.1);
        let first = l.take_unlogged(&t).expect("chu kỳ đầu");
        assert_eq!(first.to.time, T0 + 900);
        assert!(l.take_unlogged(&t).is_none());
        feed(&mut l, T0 + 900 + 20, T0 + 1_800 - 60, ok(160.1));
        feed(&mut l, T0 + 1_800 - 60, T0 + 1_800 + 20, ok(160.2));
        assert_eq!(l.take_unlogged(&t).expect("chu kỳ kế").to.time, T0 + 1_800);
        assert!(l.take_unlogged(&t).is_none());
    }

    #[test]
    fn starts_waiting_with_no_level() {
        let l = Live::new(15);
        assert_eq!(l.flow(&VolumeTable::builtin()), FlowView::Waiting);
        assert_eq!((l.level(), l.status(), l.error()), (None, None, None));
    }

    #[test]
    fn flow_is_storage_change_plus_eco_release() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15);
        two_periods(&mut l, 160.0, 160.1);
        let want = inflow_between_levels(&t, 160.0, 160.1, 15.0, 1.5, 0.0, 0.0).unwrap().q_in;
        match l.flow(&t) {
            FlowView::Ready(f) => {
                assert_eq!(f.q_in, want);
                assert!((f.dz - 0.1).abs() < 1e-9);
                assert_eq!((f.from.time, f.to.time), (T0, T0 + 900));
            }
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn running_h1_adds_its_measured_flow_to_the_inflow() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15);
        feed(&mut l, T0 - 120, T0 + 900 - 60, ok_with(158.0, 16.54, 0.0));
        feed(&mut l, T0 + 900 - 60, T0 + 900 + 20, ok_with(158.0, 16.54, 0.0));
        match l.flow(&t) {
            FlowView::Ready(f) => {
                assert!((f.q_in - (1.5 + 16.54)).abs() < 1e-9, "{}", f.q_in);
                assert_eq!((f.outflow.run_h1, f.outflow.run_h2), (1.0, 0.0));
                let r = history_record(&f, 15);
                assert_eq!((r.h1, r.h2), (0.25, 0.0));
            }
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn machines_are_detected_from_the_latest_reading() {
        let mut l = Live::new(15);
        assert_eq!(l.machines(), None);
        l.record(T0, ok_with(157.0, 16.54, 0.0));
        let (t, m) = l.machines().unwrap();
        assert_eq!(t, T0);
        assert_eq!((m.h1, m.h2), (Machine { q: 16.54, running: true }, Machine { q: 0.0, running: false }));
        l.record(T0 + 5, ok_with(157.0, 16.5, 16.6));
        assert!(l.machines().unwrap().1.h2.running);
        // lần đọc thiếu kênh tổ máy: giữ lần đọc đủ gần nhất
        l.record(T0 + 10, Ok(Reading::new(157.0, Status::Normal)));
        assert_eq!(l.machines().unwrap().0, T0 + 5);
    }

    #[test]
    fn missing_machine_channels_make_the_interval_missing_not_zero() {
        let mut l = Live::new(15);
        two_periods_without_machines(&mut l);
        assert!(matches!(l.flow(&VolumeTable::builtin()), FlowView::Missing { .. }));
    }

    #[test]
    fn steady_level_gives_exactly_the_eco_release() {
        let mut l = Live::new(15);
        two_periods(&mut l, 158.0, 158.0);
        match l.flow(&VolumeTable::builtin()) {
            FlowView::Ready(f) => assert!((f.q_in - 1.5).abs() < 1e-9),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn level_outside_the_table_is_invalid_not_extrapolated() {
        let mut l = Live::new(15);
        two_periods(&mut l, 148.0, 148.1);
        assert!(matches!(l.flow(&VolumeTable::builtin()), FlowView::Invalid(_)));
    }

    #[test]
    fn read_failures_keep_old_level_and_break_the_interval() {
        let mut l = Live::new(15);
        feed(&mut l, T0 - 120, T0 + 900 - 120, ok(157.0));
        feed(&mut l, T0 + 900 - 120, T0 + 900 + 20, Err(LoggerError::Unreachable));
        assert_eq!(l.level().map(|(_, z)| z), Some(157.0));
        assert_eq!(l.error(), Some(&LoggerError::Unreachable));
        assert_eq!(l.flow(&VolumeTable::builtin()), FlowView::Missing { to: T0 + 900 });
    }

    #[test]
    fn network_error_is_remembered_until_the_next_good_reading() {
        let mut l = Live::new(15);
        l.record(T0, ok(157.0));
        l.record(T0 + 5, Err(LoggerError::Unreachable));
        assert_eq!(l.error(), Some(&LoggerError::Unreachable));
        assert_eq!(l.level(), Some((T0, 157.0)));
        l.record(T0 + 10, ok(157.1));
        assert_eq!(l.error(), None);
    }

    #[test]
    fn stale_after_three_poll_intervals_without_a_valid_level() {
        let mut l = Live::new(15);
        assert!(l.stale(T0, 5), "chưa có số liệu nào thì coi là cũ");
        l.record(T0, ok(157.0));
        assert!(!l.stale(T0 + 15, 5));
        assert!(l.stale(T0 + 16, 5));
    }

    #[test]
    fn changing_period_restarts_the_intervals() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15);
        two_periods(&mut l, 160.0, 160.1);
        l.set_period(30);
        assert_eq!(l.period(), 30);
        assert_eq!(l.flow(&t), FlowView::Waiting);
        assert!(l.level().is_some(), "mực nước hiện tại vẫn hiển thị");
    }

    #[test]
    fn eco_release_is_the_measured_flow_of_the_interval() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15);
        feed(&mut l, T0 - 120, T0 + 900 + 20, ok_eco(158.0, 0.0, 0.0, 2.0));
        match l.flow(&t) {
            FlowView::Ready(f) => {
                assert!((f.q_in - 2.0).abs() < 1e-9);
                assert_eq!(history_record(&f, 15).eco_q, 2.0);
            }
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn missing_eco_channel_makes_the_interval_missing() {
        let mut l = Live::new(15);
        let no_eco = Ok(Reading { q_h1: Some(0.0), q_h2: Some(0.0), ..Reading::new(158.0, Status::Normal) });
        feed(&mut l, T0 - 120, T0 + 900 + 20, no_eco);
        assert!(matches!(l.flow(&VolumeTable::builtin()), FlowView::Missing { .. }));
    }
}
