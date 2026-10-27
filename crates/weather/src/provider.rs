use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::WeatherError;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Current {
    pub temp: f64,
    pub humidity: f64,
    /// m/s
    pub wind_speed: f64,
    pub description: String,
    pub icon: String,
    /// Giờ cập nhật của trạm (unix, UTC).
    pub observed_at: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Day {
    pub date: NaiveDate,
    pub temp_min: f64,
    pub temp_max: f64,
    /// Tổng lượng mưa dự kiến (mm).
    pub rain_mm: f64,
    /// Xác suất mưa cao nhất trong ngày, 0–100 (%).
    pub pop_percent: f64,
    pub description: String,
    pub icon: String,
    /// Ngày thiếu mốc 3 giờ (thường là ngày đầu/cuối): chỉ là dữ liệu một phần.
    pub partial: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub current: Current,
    pub days: Vec<Day>,
    /// Thời điểm app lấy dữ liệu (unix, UTC) — để biết cũ hay mới.
    pub fetched_at: i64,
    /// Độ lệch giờ địa phương so với UTC (giây).
    pub tz_offset: i32,
    /// Tên nguồn dữ liệu để hiển thị, ví dụ "Open-Meteo · NOAA GFS".
    #[serde(default)]
    pub source: String,
}

pub trait WeatherProvider {
    fn fetch(&self, lat: f64, lon: f64) -> Result<Snapshot, WeatherError>;
}

/// Phân loại phản hồi HTTP thành lỗi cho người vận hành. Không đưa URL (có thể chứa key) vào lỗi.
pub(crate) fn read_json<T: serde::de::DeserializeOwned>(
    response: Result<ureq::Response, ureq::Error>,
) -> Result<T, WeatherError> {
    match response {
        Ok(r) => r.into_json::<T>().map_err(|e| {
            if e.kind() == std::io::ErrorKind::InvalidData {
                WeatherError::BadResponse
            } else {
                WeatherError::Unavailable
            }
        }),
        Err(ureq::Error::Status(401, _)) => Err(WeatherError::InvalidKey),
        Err(ureq::Error::Status(429, _)) => Err(WeatherError::RateLimited),
        Err(_) => Err(WeatherError::Unavailable),
    }
}
