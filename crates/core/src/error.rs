use thiserror::Error;

use crate::validate::format_vn;

/// Mọi thông báo bằng tiếng Việt, nêu việc cần làm tiếp theo.
#[derive(Debug, Error, PartialEq)]
pub enum CalcError {
    #[error("Mực nước phải trong khoảng {} – {} m", format_vn(*.min, 1), format_vn(*.max, 1))]
    LevelOutOfRange { min: f64, max: f64 },
    #[error("Thời gian phải lớn hơn 0")]
    HoursNotPositive,
    #[error("Giờ chạy máy không được lớn hơn thời gian tính")]
    RunHoursExceed,
    #[error("Số giờ xả tràn không được lớn hơn thời gian tính")]
    SpillHoursExceed,
    #[error("Giá trị không được âm")]
    Negative,
    #[error("Vui lòng nhập số, ví dụ 160,5")]
    InvalidNumber,
    #[error("Chỉ dùng một dấu thập phân (dấu phẩy hoặc dấu chấm), ví dụ 160,5")]
    AmbiguousNumber,
    #[error("Bảng dung tích lỗi ở dòng {line}: {reason}")]
    InvalidTable { line: usize, reason: String },
}
