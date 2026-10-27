//! Lõi tính toán lưu lượng nước về hồ Sông Bồ. Không phụ thuộc giao diện.

pub mod balance;
pub mod error;
pub mod reservoir;
pub mod validate;

pub use balance::{compute, Inputs, Outputs, Turbine};
pub use error::CalcError;
pub use reservoir::VolumeTable;
