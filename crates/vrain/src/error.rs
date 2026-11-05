//! Lỗi của Vrain; thông báo tiếng Việt cho người vận hành, chi tiết kỹ thuật không đưa vào đây.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum VrainError {
    #[error("Tài khoản Vrain không đúng, kiểm tra trong Cài đặt.")]
    Credentials,
    #[error("Không kết nối được Vrain.")]
    Unreachable,
    #[error("Vrain trả dữ liệu không đọc được.")]
    Format,
}
