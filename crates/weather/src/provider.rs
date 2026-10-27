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
}

pub trait WeatherProvider {
    fn fetch(&self, lat: f64, lon: f64) -> Result<Snapshot, WeatherError>;
}
