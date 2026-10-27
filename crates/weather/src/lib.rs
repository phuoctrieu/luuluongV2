//! Thời tiết A Lưới qua OpenWeather. Không phụ thuộc giao diện.

pub mod aggregate;
pub mod cache;
pub mod error;
pub mod chain;
pub mod free;
pub mod openmeteo;
pub mod provider;
pub mod source;

pub use error::WeatherError;
pub use source::Source;
pub use provider::{Current, Day, ModelRain, Snapshot, WeatherProvider};
