//! Mực nước hồ hiện tại và lưu lượng về theo chu kỳ, từ data logger ECAPRO.
//! Thuần logic (thời gian truyền vào), không phụ thuộc giao diện.

use luuluong_core::{inflow_between_levels, VolumeTable};
use luuluong_logger::tracker::{IntervalState, LevelTracker, Mark};
use luuluong_logger::{LoggerError, Reading, Status};

/// Múi giờ Việt Nam (giây), cố định UTC+7.
pub const VN_TZ: i32 = 7 * 3600;

/// Lưu lượng về của một chu kỳ đã đủ hai mốc.
#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub from: Mark,
    pub to: Mark,
    /// Mực nước mốc sau trừ mốc trước (m).
    pub dz: f64,
    /// m³/s, đã cộng xả môi trường.
    pub q_in: f64,
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

pub struct Live {
    tracker: LevelTracker,
    period: u32,
    eco_q: f64,
    last: Option<Result<Reading, LoggerError>>,
    latest: Option<(i64, f64)>,
}

impl Live {
    pub fn new(period_minutes: u32, eco_q: f64) -> Self {
        Self { tracker: LevelTracker::new(period_minutes), period: period_minutes, eco_q, last: None, latest: None }
    }

    /// Đổi chu kỳ: chốt lại từ đầu vì các mốc cũ nằm trên lưới khác.
    pub fn set_period(&mut self, period_minutes: u32) {
        if period_minutes != self.period {
            self.period = period_minutes;
            self.tracker = LevelTracker::new(period_minutes);
        }
    }

    pub fn set_eco_q(&mut self, eco_q: f64) {
        self.eco_q = eco_q;
    }

    pub fn period(&self) -> u32 {
        self.period
    }

    pub fn record(&mut self, now: i64, result: Result<Reading, LoggerError>) {
        match &result {
            Ok(r) if r.status != Status::Disconnected => {
                self.latest = Some((now, r.z));
                self.tracker.push(now, Some(r.z));
            }
            _ => self.tracker.push(now, None),
        }
        self.last = Some(result);
    }

    /// Mực nước hợp lệ mới nhất (giây, m).
    pub fn level(&self) -> Option<(i64, f64)> {
        self.latest
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
            IntervalState::Ready(i) => match inflow_between_levels(table, i.from.z, i.to.z, f64::from(self.period), self.eco_q) {
                Ok(o) => FlowView::Ready(Flow { from: i.from, to: i.to, dz: i.to.z - i.from.z, q_in: o.q_in }),
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
        Ok(Reading { z, status: Status::Normal })
    }

    fn feed(l: &mut Live, from: i64, to: i64, r: Result<Reading, LoggerError>) {
        let mut t = from;
        while t < to {
            l.record(t, r.clone());
            t += 5;
        }
    }

    fn two_periods(l: &mut Live, z0: f64, z1: f64) {
        feed(l, T0 - 120, T0 + 900 - 60, ok(z0));
        feed(l, T0 + 900 - 60, T0 + 900 + 20, ok(z1));
    }

    #[test]
    fn starts_waiting_with_no_level() {
        let l = Live::new(15, 1.5);
        assert_eq!(l.flow(&VolumeTable::builtin()), FlowView::Waiting);
        assert_eq!((l.level(), l.status(), l.error()), (None, None, None));
    }

    #[test]
    fn flow_is_storage_change_plus_eco_release() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15, 1.5);
        two_periods(&mut l, 160.0, 160.1);
        let want = inflow_between_levels(&t, 160.0, 160.1, 15.0, 1.5).unwrap().q_in;
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
    fn steady_level_gives_exactly_the_eco_release() {
        let mut l = Live::new(15, 1.5);
        two_periods(&mut l, 158.0, 158.0);
        match l.flow(&VolumeTable::builtin()) {
            FlowView::Ready(f) => assert!((f.q_in - 1.5).abs() < 1e-9),
            v => panic!("{v:?}"),
        }
    }

    #[test]
    fn level_outside_the_table_is_invalid_not_extrapolated() {
        let mut l = Live::new(15, 1.5);
        two_periods(&mut l, 148.0, 148.1);
        assert!(matches!(l.flow(&VolumeTable::builtin()), FlowView::Invalid(_)));
    }

    #[test]
    fn disconnected_sensor_keeps_old_level_and_breaks_the_interval() {
        let mut l = Live::new(15, 1.5);
        feed(&mut l, T0 - 120, T0 + 900 - 120, ok(157.0));
        feed(&mut l, T0 + 900 - 120, T0 + 900 + 20, Ok(Reading { z: 0.0, status: Status::Disconnected }));
        assert_eq!(l.level().map(|(_, z)| z), Some(157.0));
        assert_eq!(l.status(), Some(Status::Disconnected));
        assert_eq!(l.flow(&VolumeTable::builtin()), FlowView::Missing { to: T0 + 900 });
    }

    #[test]
    fn network_error_is_remembered_until_the_next_good_reading() {
        let mut l = Live::new(15, 1.5);
        l.record(T0, ok(157.0));
        l.record(T0 + 5, Err(LoggerError::Unreachable));
        assert_eq!(l.error(), Some(&LoggerError::Unreachable));
        assert_eq!(l.level(), Some((T0, 157.0)));
        l.record(T0 + 10, ok(157.1));
        assert_eq!(l.error(), None);
    }

    #[test]
    fn stale_after_three_poll_intervals_without_a_valid_level() {
        let mut l = Live::new(15, 1.5);
        assert!(l.stale(T0, 5), "chưa có số liệu nào thì coi là cũ");
        l.record(T0, ok(157.0));
        assert!(!l.stale(T0 + 15, 5));
        assert!(l.stale(T0 + 16, 5));
    }

    #[test]
    fn changing_period_restarts_the_intervals() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15, 1.5);
        two_periods(&mut l, 160.0, 160.1);
        l.set_period(30);
        assert_eq!(l.period(), 30);
        assert_eq!(l.flow(&t), FlowView::Waiting);
        assert!(l.level().is_some(), "mực nước hiện tại vẫn hiển thị");
    }

    #[test]
    fn changing_eco_release_applies_to_the_next_computation() {
        let t = VolumeTable::builtin();
        let mut l = Live::new(15, 1.5);
        two_periods(&mut l, 158.0, 158.0);
        l.set_eco_q(2.0);
        match l.flow(&t) {
            FlowView::Ready(f) => assert!((f.q_in - 2.0).abs() < 1e-9),
            v => panic!("{v:?}"),
        }
    }
}
