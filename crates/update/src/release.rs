//! Đọc phản hồi `releases/latest` của GitHub và chọn gói theo nền tảng.

use serde_json::Value;

use crate::{parse_version, UpdateError, Version};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Windows,
}

impl Platform {
    /// Nền tảng của bản đang chạy.
    pub fn current() -> Platform {
        if cfg!(target_os = "windows") { Platform::Windows } else { Platform::MacOs }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: Version,
    pub notes: String,
    pub asset_name: String,
    pub url: String,
    pub size: u64,
    /// SHA-256 viết thường, 64 ký tự hex.
    pub sha256: String,
}

/// Mọi gói cập nhật phải nằm ở đây.
pub const DOWNLOAD_PREFIX: &str = "https://github.com/phuoctrieu/luuluongV2/releases/download/";
pub const MAX_ASSET_BYTES: u64 = 100 * 1024 * 1024;

pub fn parse_release(json: &str, platform: Platform) -> Result<Release, UpdateError> {
    let v: Value = serde_json::from_str(json).map_err(|_| UpdateError::BadResponse)?;
    // Bản nháp và tiền phát hành không bao giờ được cài.
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return Err(UpdateError::NoAsset);
    }
    let version = v["tag_name"].as_str().and_then(parse_version).ok_or(UpdateError::BadResponse)?;
    let suffix = match platform {
        Platform::MacOs => "-macos-universal.app.zip",
        Platform::Windows => "-windows-x64.zip",
    };
    let asset = v["assets"]
        .as_array()
        .and_then(|list| list.iter().find(|a| a["name"].as_str().is_some_and(|n| n.starts_with("luuluong-") && n.ends_with(suffix))))
        .ok_or(UpdateError::NoAsset)?;
    let sha256 = asset["digest"]
        .as_str()
        .and_then(|d| d.strip_prefix("sha256:"))
        .filter(|h| h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase)
        .ok_or(UpdateError::BadDigest)?;
    let url = asset["browser_download_url"].as_str().filter(|u| u.starts_with(DOWNLOAD_PREFIX)).ok_or(UpdateError::BadResponse)?;
    let size = asset["size"].as_u64().ok_or(UpdateError::BadResponse)?;
    if size > MAX_ASSET_BYTES {
        return Err(UpdateError::TooLarge);
    }
    Ok(Release {
        version,
        notes: v["body"].as_str().unwrap_or_default().to_string(),
        asset_name: asset["name"].as_str().unwrap_or_default().to_string(),
        url: url.to_string(),
        size,
        sha256,
    })
}

/// Cắt ghi chú còn khoảng 600 ký tự tại ranh giới từ, thêm "…".
pub fn notes_excerpt(notes: &str) -> String {
    const MAX_CHARS: usize = 600;
    let notes = notes.trim();
    if notes.chars().count() <= MAX_CHARS {
        return notes.to_string();
    }
    let head: String = notes.chars().take(MAX_CHARS).collect();
    let cut = head.rfind(char::is_whitespace).map_or(head.as_str(), |i| &head[..i]);
    format!("{}…", cut.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &str = include_str!("../tests/data/release_latest.json");

    fn edited(f: impl FnOnce(&mut Value)) -> String {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        f(&mut v);
        v.to_string()
    }

    #[test]
    fn release_picks_asset_per_platform() {
        let mac = parse_release(FIXTURE, Platform::MacOs).unwrap();
        assert_eq!(mac.asset_name, "luuluong-0.2.0-macos-universal.app.zip");
        assert_eq!(mac.version, Version(0, 2, 0));
        assert_eq!(mac.sha256, "ab".repeat(32));
        assert!(mac.url.ends_with("macos-universal.app.zip"));
        let win = parse_release(FIXTURE, Platform::Windows).unwrap();
        assert_eq!(win.asset_name, "luuluong-0.2.0-windows-x64.zip");
        assert_eq!(win.size, 4_028_944);
        assert_eq!(win.sha256.len(), 64);
        assert!(win.notes.contains("Bản 0.2.0"));
    }

    #[test]
    fn release_without_digest_is_rejected() {
        let j = edited(|v| v["assets"][1]["digest"] = Value::Null);
        assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::BadDigest));
    }

    #[test]
    fn release_with_other_or_malformed_digest_is_rejected() {
        for bad in ["md5:d41d8cd98f00b204e9800998ecf8427e", "sha256:abc", "sha256:zz", "abc", &format!("sha256:{}", "g".repeat(64))] {
            let j = edited(|v| v["assets"][1]["digest"] = Value::String(bad.to_string()));
            assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::BadDigest), "{bad}");
        }
    }

    #[test]
    fn uppercase_digest_is_normalised_to_lowercase() {
        let j = edited(|v| v["assets"][1]["digest"] = Value::String(format!("sha256:{}", "AB".repeat(32))));
        assert_eq!(parse_release(&j, Platform::Windows).unwrap().sha256, "ab".repeat(32));
    }

    #[test]
    fn release_url_outside_github_is_rejected() {
        for bad in ["http://github.com/phuoctrieu/luuluongV2/releases/download/v0.2.0/x.zip", "https://evil.example/luuluongV2/releases/download/v0.2.0/x.zip", "https://github.com/someone/else/releases/download/v1/x.zip"] {
            let j = edited(|v| v["assets"][1]["browser_download_url"] = Value::String(bad.to_string()));
            assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::BadResponse), "{bad}");
        }
    }

    #[test]
    fn release_over_100mb_is_rejected() {
        let j = edited(|v| v["assets"][1]["size"] = Value::from(MAX_ASSET_BYTES + 1));
        assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::TooLarge));
        let ok = edited(|v| v["assets"][1]["size"] = Value::from(MAX_ASSET_BYTES));
        assert!(parse_release(&ok, Platform::Windows).is_ok());
    }

    #[test]
    fn prerelease_and_draft_are_ignored() {
        for key in ["prerelease", "draft"] {
            let j = edited(|v| v[key] = Value::Bool(true));
            assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::NoAsset), "{key}");
        }
    }

    #[test]
    fn release_missing_platform_asset_is_no_asset() {
        let j = edited(|v| v["assets"].as_array_mut().unwrap().retain(|a| !a["name"].as_str().unwrap().ends_with(".app.zip")));
        assert_eq!(parse_release(&j, Platform::MacOs), Err(UpdateError::NoAsset), "chỉ còn .dmg và Windows: không lấy .dmg");
        assert!(parse_release(&j, Platform::Windows).is_ok());
    }

    #[test]
    fn odd_tag_or_bad_json_is_bad_response() {
        assert_eq!(parse_release("không phải json", Platform::Windows), Err(UpdateError::BadResponse));
        assert_eq!(parse_release("{}", Platform::Windows), Err(UpdateError::BadResponse));
        for tag in ["abc", "v1.0", "latest"] {
            let j = edited(|v| v["tag_name"] = Value::String(tag.into()));
            assert_eq!(parse_release(&j, Platform::Windows), Err(UpdateError::BadResponse), "{tag}");
        }
    }

    #[test]
    fn notes_are_cut_at_a_word_boundary() {
        let long = "chữ ".repeat(400);
        let cut = notes_excerpt(&long);
        assert!(cut.ends_with('…'));
        assert!(cut.chars().count() <= 601);
        assert!(cut.trim_end_matches('…').ends_with("chữ"), "không cắt giữa từ: {cut:?}");
        assert_eq!(notes_excerpt("  ngắn thôi \n"), "ngắn thôi");
        assert_eq!(notes_excerpt(""), "");
    }

    #[test]
    fn error_messages_are_vietnamese_and_have_no_technical_text() {
        use UpdateError::*;
        for e in [Offline, RateLimited, NotFound, BadResponse, NoAsset, BadDigest, TooLarge, HashMismatch, NotWritable, NotInstalled, Io] {
            let m = e.to_string();
            assert!(!m.is_ascii(), "thiếu dấu tiếng Việt: {m}");
            for tech in ["http", "HTTP", "json", "JSON", "sha", "SHA", "error", "Error", "panic", "403", "404", "errno"] {
                assert!(!m.contains(tech), "{m:?} chứa {tech:?}");
            }
        }
    }
}
