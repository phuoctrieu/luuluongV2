//! Cập nhật ứng dụng từ GitHub Releases: so phiên bản, đọc bản phát hành, tải có kiểm SHA-256,
//! giải nén an toàn và thay thế bằng đổi tên. Không phụ thuộc giao diện.

mod error;
mod install;
mod net;
mod pipeline;
mod release;
mod unpack;
mod version;

pub use error::UpdateError;
pub use install::{cleanup, detect_target, download_dir, relaunch_command, staging_dir, swap, Target, DOWNLOAD_PREFIX_DIR, STAGING_PREFIX};
pub use net::{check, download, API_BASE};
pub use unpack::{unpack, MAX_UNPACKED_BYTES};
pub use pipeline::install_release;
pub use release::{notes_excerpt, parse_release, Platform, Release, DOWNLOAD_PREFIX, MAX_ASSET_BYTES};
pub use version::{is_newer, parse_version, Version};
