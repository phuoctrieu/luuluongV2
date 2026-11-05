//! Cập nhật ứng dụng từ GitHub Releases: so phiên bản, đọc bản phát hành, tải có kiểm SHA-256,
//! giải nén an toàn và thay thế bằng đổi tên. Không phụ thuộc giao diện.

mod error;
mod release;
mod version;

pub use error::UpdateError;
pub use release::{notes_excerpt, parse_release, Platform, Release, DOWNLOAD_PREFIX, MAX_ASSET_BYTES};
pub use version::{is_newer, parse_version, Version};
