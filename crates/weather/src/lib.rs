//! Thời tiết A Lưới qua OpenWeather. Không phụ thuộc giao diện.

pub mod aggregate;
pub mod cache;
pub mod error;
pub mod free;
pub mod openmeteo;
pub mod multi;
pub mod provider;
pub mod radar;
pub mod source;

pub use error::WeatherError;
pub use source::Source;
pub use provider::{Current, Day, Snapshot, WeatherProvider};
