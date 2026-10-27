//! Thời tiết A Lưới qua OpenWeather. Không phụ thuộc giao diện.

pub mod aggregate;
pub mod cache;
pub mod error;
pub mod free;
pub mod provider;

pub use error::WeatherError;
pub use provider::{Current, Day, Snapshot, WeatherProvider};
