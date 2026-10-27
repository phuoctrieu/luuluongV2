//! Chuỗi nguồn dự phòng: thử lần lượt, lấy nguồn đầu tiên thành công.

use crate::error::WeatherError;
use crate::provider::{Snapshot, WeatherProvider};

pub struct Fetched {
    pub snapshot: Snapshot,
    /// Lỗi của nguồn chính nếu phải dùng nguồn dự phòng.
    pub primary_error: Option<WeatherError>,
}

/// Nếu mọi nguồn đều lỗi, trả lỗi của nguồn đầu tiên (nguồn chính).
pub fn fetch_chain(providers: &[&dyn WeatherProvider], lat: f64, lon: f64) -> Result<Fetched, WeatherError> {
    let mut primary_error = None;
    for provider in providers {
        match provider.fetch(lat, lon) {
            Ok(snapshot) => return Ok(Fetched { snapshot, primary_error }),
            Err(e) => {
                primary_error.get_or_insert(e);
            }
        }
    }
    Err(primary_error.unwrap_or(WeatherError::Unavailable))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Current;

    struct Fake(Result<Snapshot, WeatherError>);

    impl WeatherProvider for Fake {
        fn fetch(&self, _lat: f64, _lon: f64) -> Result<Snapshot, WeatherError> {
            self.0.clone()
        }
    }

    fn snap(source: &str) -> Snapshot {
        Snapshot {
            current: Current { temp: 1.0, humidity: 1.0, wind_speed: 1.0, description: String::new(), icon: String::new(), observed_at: 0 },
            days: vec![],
            fetched_at: 0,
            tz_offset: 0,
            source: source.into(),
        }
    }

    #[test]
    fn first_success_wins_without_error() {
        let (a, b) = (Fake(Ok(snap("A"))), Fake(Ok(snap("B"))));
        let f = fetch_chain(&[&a, &b], 0.0, 0.0).unwrap();
        assert_eq!(f.snapshot.source, "A");
        assert_eq!(f.primary_error, None);
    }

    #[test]
    fn falls_back_and_reports_the_primary_error() {
        let (a, b) = (Fake(Err(WeatherError::RateLimited)), Fake(Ok(snap("B"))));
        let f = fetch_chain(&[&a, &b], 0.0, 0.0).unwrap();
        assert_eq!(f.snapshot.source, "B");
        assert_eq!(f.primary_error, Some(WeatherError::RateLimited));
    }

    #[test]
    fn all_failing_returns_the_primary_error() {
        let (a, b) = (Fake(Err(WeatherError::InvalidKey)), Fake(Err(WeatherError::Unavailable)));
        assert_eq!(fetch_chain(&[&a, &b], 0.0, 0.0).err(), Some(WeatherError::InvalidKey));
    }

    #[test]
    fn empty_chain_is_unavailable() {
        assert_eq!(fetch_chain(&[], 0.0, 0.0).err(), Some(WeatherError::Unavailable));
    }
}
