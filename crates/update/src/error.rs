//! Lỗi cập nhật; thông báo tiếng Việt cho người vận hành, không có chi tiết kỹ thuật.

use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum UpdateError {
    #[error("Không kết nối được GitHub. Kiểm tra mạng rồi thử lại.")]
    Offline,
    #[error("GitHub đang giới hạn số lần kiểm tra. Thử lại sau ít phút.")]
    RateLimited,
    #[error("Chưa có bản phát hành nào trên GitHub.")]
    NotFound,
    #[error("Thông tin bản phát hành không đọc được.")]
    BadResponse,
    #[error("Bản phát hành mới chưa có gói cài đặt cho máy này.")]
    NoAsset,
    #[error("Bản phát hành thiếu mã kiểm tra an toàn, không cài đặt.")]
    BadDigest,
    #[error("Gói cập nhật quá lớn, không cài đặt.")]
    TooLarge,
    #[error("File tải về không khớp mã kiểm tra, đã hủy cập nhật.")]
    HashMismatch,
    #[error("Không ghi được vào thư mục cài đặt. Hãy tải bản mới từ trang phát hành.")]
    NotWritable,
    #[error("App đang chạy từ môi trường phát triển, không tự cập nhật được.")]
    NotInstalled,
    #[error("Không ghi được file cập nhật. Kiểm tra ổ đĩa còn chỗ trống.")]
    Io,
}
