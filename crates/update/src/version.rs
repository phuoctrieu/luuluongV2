//! Phiên bản dạng `x.y.z` và so sánh.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u32, pub u32, pub u32);

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.0, self.1, self.2)
    }
}

/// Chấp nhận `v0.2.0` và `0.2.0`; mọi dạng khác (`1.0`, `0.2.0-beta`, `abc`) là `None`.
pub fn parse_version(tag: &str) -> Option<Version> {
    let t = tag.trim();
    let t = t.strip_prefix(['v', 'V']).unwrap_or(t);
    let parts: Vec<&str> = t.split('.').collect();
    let [a, b, c] = parts.as_slice() else { return None };
    let num = |s: &str| (!s.is_empty() && s.bytes().all(|x| x.is_ascii_digit())).then(|| s.parse::<u32>().ok()).flatten();
    Some(Version(num(a)?, num(b)?, num(c)?))
}

pub fn is_newer(latest: Version, current: Version) -> bool {
    latest > current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_version_accepts_v_prefix_and_rejects_oddities() {
        assert_eq!(parse_version("v0.2.0"), Some(Version(0, 2, 0)));
        assert_eq!(parse_version("0.2.0"), Some(Version(0, 2, 0)));
        assert_eq!(parse_version(" v10.20.30 "), Some(Version(10, 20, 30)));
        for bad in ["", "v", "1.0", "1.0.0.0", "0.2.0-beta", "abc", "v1.x.0", "1..0", "-1.0.0", "+1.0.0", "1.0.0 beta"] {
            assert_eq!(parse_version(bad), None, "{bad:?}");
        }
        assert_eq!(Version(0, 2, 10).to_string(), "0.2.10");
    }

    #[test]
    fn ten_beats_nine() {
        assert!(is_newer(Version(0, 10, 0), Version(0, 9, 0)));
        assert!(is_newer(Version(1, 0, 0), Version(0, 99, 99)));
        assert!(is_newer(Version(0, 2, 1), Version(0, 2, 0)));
    }

    #[test]
    fn equal_or_older_is_not_newer() {
        assert!(!is_newer(Version(0, 2, 0), Version(0, 2, 0)));
        assert!(!is_newer(Version(0, 1, 9), Version(0, 2, 0)));
    }
}
