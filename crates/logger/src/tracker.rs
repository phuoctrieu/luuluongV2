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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub from: Mark,
    pub to: Mark,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntervalState {
    /// Chưa qua mốc nào có số liệu.
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
    /// Hai mốc gần nhất (mốc, mực nước nếu chốt được).
    marks: VecDeque<(i64, Option<f64>)>,
}

impl LevelTracker {
    pub fn new(period_minutes: u32) -> Self {
        Self {
            period_secs: i64::from(period_minutes) * 60,
            samples: VecDeque::new(),
            latest: None,
            slot: None,
            marks: VecDeque::new(),
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
                self.marks.push_back((boundary, self.median_before(boundary)));
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
        self.marks.back().and_then(|&(time, z)| z.map(|z| Mark { time, z }))
    }

    pub fn state(&self) -> IntervalState {
        let Some(&(to, z_to)) = self.marks.back() else {
            return IntervalState::NoData;
        };
        if self.marks.len() == 2 {
            let (from, z_from) = self.marks[0];
            if let (Some(z_from), Some(z_to)) = (z_from, z_to) {
                if from + self.period_secs == to {
                    return IntervalState::Ready(Interval {
                        from: Mark { time: from, z: z_from },
                        to: Mark { time: to, z: z_to },
                    });
                }
            }
        }
        IntervalState::Missing { to }
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
    fn first_boundary_alone_is_not_an_interval() {
        let mut t = LevelTracker::new(15);
        feed(&mut t, T0 - 120, T0 + 20, 157.0);
        assert_eq!(t.state(), IntervalState::Missing { to: T0 });
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
