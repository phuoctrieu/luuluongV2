//! Lịch làm mới thời tiết (đặc tả 5.4, 5.5). Thuần logic, thời gian truyền vào (giây).

use luuluong_weather::WeatherError;

const RETRY_SECS: i64 = 300;
const BACKOFF_CAP_SECS: i64 = 3600;
const MANUAL_MIN_GAP_SECS: i64 = 30;

pub struct Scheduler {
    interval_secs: i64,
    next_at: i64,
    last_attempt: Option<i64>,
    failures: u32,
    /// Lỗi key: không tự thử lại (vô ích cho tới khi sửa key).
    blocked: bool,
}

impl Scheduler {
    pub fn new(refresh_minutes: u32) -> Self {
        Self { interval_secs: refresh_minutes as i64 * 60, next_at: 0, last_attempt: None, failures: 0, blocked: false }
    }

    /// Có nên tự động làm mới lúc `now` không.
    pub fn due(&self, now: i64) -> bool {
        !self.blocked && now >= self.next_at
    }

    /// Nút làm mới thủ công: tối đa 1 lần / 30 giây.
    pub fn manual_allowed(&self, now: i64) -> bool {
        self.last_attempt.is_none_or(|t| now - t >= MANUAL_MIN_GAP_SECS)
    }

    pub fn mark_attempt(&mut self, now: i64) {
        self.last_attempt = Some(now);
    }

    pub fn record_ok(&mut self, now: i64) {
        self.failures = 0;
        self.blocked = false;
        self.next_at = now + self.interval_secs;
    }

    pub fn record_err(&mut self, now: i64, err: &WeatherError) {
        self.failures += 1;
        let delay = match err {
            WeatherError::InvalidKey | WeatherError::MissingKey => {
                self.blocked = true;
                return;
            }
            WeatherError::RateLimited => {
                (RETRY_SECS << (self.failures - 1).min(10)).min(BACKOFF_CAP_SECS)
            }
            WeatherError::Unavailable | WeatherError::BadResponse => RETRY_SECS,
        };
        self.next_at = now + delay;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_scheduler_is_due_immediately() {
        assert!(Scheduler::new(60).due(1_000));
    }

    #[test]
    fn after_success_waits_the_refresh_interval() {
        let mut s = Scheduler::new(60);
        s.record_ok(1_000);
        assert!(!s.due(1_000 + 3_599));
        assert!(s.due(1_000 + 3_600));
    }

    #[test]
    fn server_or_network_error_retries_after_five_minutes() {
        for e in [WeatherError::Unavailable, WeatherError::BadResponse] {
            let mut s = Scheduler::new(60);
            s.record_err(1_000, &e);
            assert!(!s.due(1_000 + 299), "{e:?}");
            assert!(s.due(1_000 + 300), "{e:?}");
        }
    }

    #[test]
    fn rate_limit_backs_off_exponentially_up_to_an_hour() {
        let mut s = Scheduler::new(60);
        let mut waits = Vec::new();
        for _ in 0..6 {
            s.record_err(0, &WeatherError::RateLimited);
            waits.push(s.next_at);
        }
        assert_eq!(waits, vec![300, 600, 1200, 2400, 3600, 3600]);
    }

    #[test]
    fn success_resets_the_backoff() {
        let mut s = Scheduler::new(60);
        s.record_err(0, &WeatherError::RateLimited);
        s.record_err(0, &WeatherError::RateLimited);
        s.record_ok(0);
        s.record_err(0, &WeatherError::RateLimited);
        assert_eq!(s.next_at, 300);
    }

    #[test]
    fn key_errors_stop_automatic_retries_but_not_manual_ones() {
        for e in [WeatherError::InvalidKey, WeatherError::MissingKey] {
            let mut s = Scheduler::new(60);
            s.mark_attempt(0);
            s.record_err(0, &e);
            assert!(!s.due(1_000_000), "{e:?}");
            assert!(s.manual_allowed(31), "{e:?}");
        }
    }

    #[test]
    fn manual_refresh_needs_30_seconds_between_attempts() {
        let mut s = Scheduler::new(60);
        assert!(s.manual_allowed(5));
        s.mark_attempt(100);
        assert!(!s.manual_allowed(129));
        assert!(s.manual_allowed(130));
    }
}
