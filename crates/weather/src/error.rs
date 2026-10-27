use thiserror::Error;

/// Thông báo cho người vận hành (đặc tả 5.5). Không chứa URL hay API key.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum WeatherError {
    #[error("API key không hợp lệ. Key mới tạo có thể cần đợi vài giờ để kích hoạt.")]
    InvalidKey,
    #[error("Quá nhiều yêu cầu, thử lại sau.")]
    RateLimited,
    #[error("Không lấy được thời tiết (máy chủ hoặc mạng gặp sự cố), sẽ thử lại sau 5 phút.")]
    Unavailable,
    #[error("Dữ liệu thời tiết trả về không đúng định dạng, đang dùng dữ liệu cũ.")]
    BadResponse,
    #[error("Chưa nhập API key thời tiết (vào Cài đặt).")]
    MissingKey,
}
