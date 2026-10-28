//! Chốt mực nước tại các mốc cố định theo đồng hồ (:00/:15/:30/:45 hoặc :00/:30)
//! và cho biết chu kỳ vừa qua có đủ số liệu để tính lưu lượng về không.
//!
//! Thời gian là giây epoch UTC. Múi giờ Việt Nam lệch đúng 7 giờ nên mốc chia hết cho
//! chu kỳ khớp mốc đồng hồ địa phương.

use std::collections::VecDeque;

/// Mực nước tại mốc chốt dùng trung vị các mẫu trong khoảng này trước mốc (khử nhiễu 1 mm).
const WINDOW_SECS: i64 = 60;
/// Mẫu cũ hơn mức này bị bỏ.
const KEEP_SECS: i64 = 600;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mark {
    pub time: i64,
    pub z: f64,
}

/// Lưu lượng ra khỏi hồ trong một chu kỳ: hai tổ máy và xả môi trường.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutflowMean {
    /// Trung bình các lần đọc (m³/s), kể cả lúc máy dừng.
    pub q_h1: f64,
    pub q_h2: f64,
    /// Tỷ lệ các lần đọc mà máy đang chạy (0..=1).
    pub run_h1: f64,
    pub run_h2: f64,
    /// Xả môi trường trung bình (m³/s).
    pub q_eco: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub from: Mark,
    pub to: Mark,
    pub outflow: OutflowMean,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntervalState {
    /// Chưa đủ hai mốc liên tiếp để xét một chu kỳ (mới mở app hoặc mới qua mốc đầu tiên).
    NoData,
    /// Chu kỳ vừa qua thiếu mực nước ở một trong hai mốc (mất kết nối, máy ngủ, mới mở app).
    Missing { to: i64 },
    Ready(Interval),
}

pub struct LevelTracker {
    period_secs: i64,
    samples: VecDeque<(i64, f64)>,
    latest: Option<(i64, f64)>,
    slot: Option<i64>,
    /// Lưu lượng tổ máy (giây, H1, H2) trong khoảng một chu kỳ gần đây.
    flows: VecDeque<(i64, f64, f64, f64)>,
    /// Hai mốc gần nhất: (mốc, mực nước, lưu lượng tổ máy của chu kỳ kết thúc ở mốc) nếu chốt được.
    marks: VecDeque<(i64, Option<f64>, Option<OutflowMean>)>,
}

impl LevelTracker {
    pub fn new(period_minutes: u32) -> Self {
        Self {
            period_secs: i64::from(period_minutes) * 60,
            samples: VecDeque::new(),
            latest: None,
            flows: VecDeque::new(),
            slot: None,
            marks: VecDeque::new(),
        }
    }

    /// Ghi lưu lượng H1, H2 và xả môi trường lúc `now` (`None` nếu không đọc được). Gọi trước `push` của cùng lần đọc.
    pub fn push_flows(&mut self, now: i64, flows: Option<(f64, f64, f64)>) {
        if let Some((a, b, c)) = flows.filter(|(a, b, c)| a.is_finite() && b.is_finite() && c.is_finite()) {
            self.flows.push_back((now, a, b, c));
        }
        while self.flows.front().is_some_and(|(t, _, _, _)| *t < now - self.period_secs - WINDOW_SECS * 2) {
            self.flows.pop_front();
        }
    }

    /// Ghi một lần đọc lúc `now`; `z = None` là đọc không được hoặc thiết bị báo không kết nối.
    pub fn push(&mut self, now: i64, z: Option<f64>) {
        if let Some(z) = z.filter(|z| z.is_finite()) {
            self.samples.push_back((now, z));
            self.latest = Some((now, z));
        }
        while self.samples.front().is_some_and(|(t, _)| *t < now - KEEP_SECS) {
            self.samples.pop_front();
        }
        let slot = now.div_euclid(self.period_secs);
        match self.slot {
            Some(prev) if slot > prev => {
                let boundary = slot * self.period_secs;
                self.marks.push_back((boundary, self.median_before(boundary), self.outflow_mean(boundary)));
                if self.marks.len() > 2 {
                    self.marks.pop_front();
                }
                self.slot = Some(slot);
            }
            None => self.slot = Some(slot),
            _ => {}
        }
    }

    /// Lần đọc hợp lệ mới nhất (giây, m).
    pub fn latest(&self) -> Option<(i64, f64)> {
        self.latest
    }

    /// Mốc gần nhất nếu chốt được mực nước.
    pub fn last_mark(&self) -> Option<Mark> {
        self.marks.back().and_then(|&(time, z, _)| z.map(|z| Mark { time, z }))
    }

    pub fn state(&self) -> IntervalState {
        // Mới có một mốc (vừa mở app): chưa có chu kỳ nào để thiếu, chỉ là đang chờ mốc thứ hai.
        if self.marks.len() < 2 {
            return IntervalState::NoData;
        }
        let (from, z_from, _) = self.marks[0];
        let (to, z_to, outflow) = self.marks[1];
        match (z_from, z_to, outflow) {
            (Some(z_from), Some(z_to), Some(outflow)) if from + self.period_secs == to => IntervalState::Ready(Interval {
                from: Mark { time: from, z: z_from },
                to: Mark { time: to, z: z_to },
                outflow,
            }),
            _ => IntervalState::Missing { to },
        }
    }

    /// Trung bình lưu lượng tổ máy trong chu kỳ kết thúc ở `boundary`; `None` nếu dữ liệu phủ dưới 80 % chu kỳ.
    fn outflow_mean(&self, boundary: i64) -> Option<OutflowMean> {
        let v: Vec<&(i64, f64, f64, f64)> = self.flows.iter().filter(|(t, _, _, _)| (boundary - self.period_secs < *t) && (*t <= boundary)).collect();
        let (first, last) = (v.first()?.0, v.last()?.0);
        if v.len() < 2 || (last - first) * 5 < self.period_secs * 4 {
            return None;
        }
        let n = v.len() as f64;
        let share = |f: fn(&(i64, f64, f64, f64)) -> f64| v.iter().filter(|s| crate::is_running(f(s))).count() as f64 / n;
        Some(OutflowMean {
            q_h1: v.iter().map(|s| s.1).sum::<f64>() / n,
            q_h2: v.iter().map(|s| s.2).sum::<f64>() / n,
            run_h1: share(|s| s.1),
            run_h2: share(|s| s.2),
            q_eco: v.iter().map(|s| s.3).sum::<f64>() / n,
        })
    }

    fn median_before(&self, boundary: i64) -> Option<f64> {
        let mut v: Vec<f64> = self
            .samples
            .iter()
            .filter(|(t, _)| (boundary - WINDOW_SECS..=boundary).contains(t))
            .map(|(_, z)| *z)
            .collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(f64::total_cmp);
        let n = v.len();
        Some(if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P15: i64 = 900;
    const T0: i64 = 1_800 * 1_000_000; // chia hết cho 900 và 1800

    /// Đọc mỗi 5 giây trong [from, to) với cùng một mực nước.
    fn feed(t: &mut LevelTracker, from: i64, to: i64, z: f64) {
        let mut s = from;
        while s < to {
            t.push_flows(s, Some((0.0, 0.0, 1.5)));
            t.push(s, Some(z));
            s += 5;
        }
    }

    #[test]
    fn nothing_pushed_means_no_data() {
        let t = LevelTracker::new(15);
        assert_eq!(t.state(), IntervalState::NoData);
        assert_eq!(t.latest(), None);
    }

    #[test]
    fn first_boundary_alone_is_still_waiting_not_missing() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 + 20, 157.0);
        assert_eq!(t.state(), IntervalState::NoData);
        assert_eq!(t.last_mark(), Some(Mark { time: T0, z: 157.0 }));
    }

    #[test]
    fn two_consecutive_boundaries_give_an_interval() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 + P15 - 60, 157.000);
        feed(&mut t, T0 + P15 - 60, T0 + P15 + 20, 157.050);
        assert_eq!(
            t.state(),
            IntervalState::Ready(Interval {
                from: Mark { time: T0, z: 157.000 },
                to: Mark { time: T0 + P15, z: 157.050 },
                outflow: OutflowMean { q_h1: 0.0, q_h2: 0.0, run_h1: 0.0, run_h2: 0.0, q_eco: 1.5 },
            })
        );
    }

    #[test]
    fn mark_uses_median_so_one_outlier_is_ignored() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 - 40, 157.000);
        for (dt, z) in [(-30, 157.001), (-25, 157.9), (-20, 157.000), (-15, 157.001), (-10, 157.000)] {
            t.push(T0 + dt, Some(z));
        }
        feed(&mut t, T0 + 1, T0 + P15 - 60, 157.000);
        feed(&mut t, T0 + P15 - 60, T0 + P15 + 20, 157.000);
        match t.state() {
            IntervalState::Ready(i) => assert!((i.from.z - 157.000).abs() < 1e-9, "from = {}", i.from.z),
            s => panic!("{s:?}"),
        }
    }

    #[test]
    fn disconnection_around_a_boundary_makes_the_interval_missing() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 + P15 - 120, 157.0);
        let mut s = T0 + P15 - 120;
        while s < T0 + P15 + 20 {
            t.push(s, None);
            s += 5;
        }
        assert_eq!(t.state(), IntervalState::Missing { to: T0 + P15 });
        assert_eq!(t.latest().map(|(_, z)| z), Some(157.0));
    }

    #[test]
    fn skipped_boundary_does_not_bridge_two_periods() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 + 20, 157.0);
        // máy ngủ qua một mốc, thức dậy ở mốc thứ ba
        feed(&mut t, T0 + 2 * P15 - 60, T0 + 2 * P15 + 20, 157.2);
        assert_eq!(t.state(), IntervalState::Missing { to: T0 + 2 * P15 });
    }

    #[test]
    fn thirty_minute_period_uses_half_hour_boundaries() {
        let mut t = LevelTracker::new(30);
        feed(&mut t, T0 - 120, T0 + 1_800 - 60, 157.0);
        feed(&mut t, T0 + 1_800 - 60, T0 + 1_800 + 20, 157.1);
        match t.state() {
            IntervalState::Ready(i) => assert_eq!((i.from.time, i.to.time), (T0, T0 + 1_800)),
            s => panic!("{s:?}"),
        }
    }

    /// Chu kỳ thứ hai [T0, T0+P15] với H1 chạy 16 m³/s ở 2/3 đầu rồi dừng; H2 luôn dừng.
    fn with_h1_running_two_thirds() -> LevelTracker {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 900 + 5, T0 + 1, 157.0);
        for k in 1..=180 {
            let now = T0 + 5 * k;
            let q1 = if k <= 120 { 16.0 } else { 0.0 };
            t.push_flows(now, Some((q1, 0.0, 1.5)));
            t.push(now, Some(157.05));
        }
        t.push_flows(T0 + 905, Some((0.0, 0.0, 1.5)));
        t.push(T0 + 905, Some(157.05));
        t
    }

    #[test]
    fn outflow_mean_and_run_share_cover_only_the_interval() {
        match with_h1_running_two_thirds().state() {
            IntervalState::Ready(i) => {
                assert!((i.outflow.q_h1 - 16.0 * 120.0 / 180.0).abs() < 1e-9, "{:?}", i.outflow);
                assert!((i.outflow.run_h1 - 120.0 / 180.0).abs() < 1e-9);
                assert_eq!((i.outflow.q_h2, i.outflow.run_h2), (0.0, 0.0));
                assert_eq!(i.outflow.q_eco, 1.5);
            }
            s => panic!("{s:?}"),
        }
    }

    #[test]
    fn missing_outflow_data_makes_the_interval_missing() {
        let mut t = LevelTracker::new(15);
        for s in (T0 - 120..T0 + P15 + 20).step_by(5) {
            t.push(s, Some(157.0));
        }
        assert_eq!(t.state(), IntervalState::Missing { to: T0 + P15 });
    }

    #[test]
    fn outflow_data_covering_under_eighty_percent_is_missing() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 900 + 5, T0 + 1, 157.0);
        for s in (T0 + 5..T0 + P15 + 20).step_by(5) {
            if s > T0 + 400 {
                t.push_flows(s, Some((16.0, 0.0, 1.5)));
            }
            t.push(s, Some(157.0));
        }
        assert_eq!(t.state(), IntervalState::Missing { to: T0 + P15 });
    }

    #[test]
    fn last_mark_is_the_newest_boundary_with_a_level() {
        let mut t = LevelTracker::new(15);
        assert_eq!(t.last_mark(), None);
        feed(&mut t, T0 - 120, T0 + 20, 157.0);
        assert_eq!(t.last_mark(), Some(Mark { time: T0, z: 157.0 }));
        // mốc sau mất kết nối: không có mốc mới, nhưng mốc cũ không bị coi là "gần nhất" nữa
        for s in (T0 + 20..T0 + P15 + 20).step_by(5) {
            t.push(s, None);
        }
        assert_eq!(t.last_mark(), None);
    }

    #[test]
    fn latest_is_the_newest_valid_reading() {
        let mut t = LevelTracker::new(15);
        t.push(T0 + 1, Some(157.1));
        t.push(T0 + 6, None);
        assert_eq!(t.latest(), Some((T0 + 1, 157.1)));
    }
}
