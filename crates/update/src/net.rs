//! Gọi API GitHub và tải gói cập nhật, kiểm SHA-256 trong lúc tải.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::{parse_release, Platform, Release, UpdateError};

pub const API_BASE: &str = "https://api.github.com";

const USER_AGENT: &str = "luuluong-updater";

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(15)).timeout_read(Duration::from_secs(60)).build()
}

/// Lỗi HTTP/mạng → lỗi thân thiện.
fn classify(e: ureq::Error) -> UpdateError {
    match e {
        ureq::Error::Status(403 | 429, _) => UpdateError::RateLimited,
        ureq::Error::Status(404, _) => UpdateError::NotFound,
        ureq::Error::Status(code, _) if code >= 500 => UpdateError::Offline,
        ureq::Error::Status(..) => UpdateError::BadResponse,
        ureq::Error::Transport(_) => UpdateError::Offline,
    }
}

/// Bản phát hành mới nhất (không tính bản nháp/tiền phát hành) cho `platform`. `base` thường là [`API_BASE`].
pub fn check(base: &str, platform: Platform) -> Result<Release, UpdateError> {
    let url = format!("{}/repos/phuoctrieu/luuluongV2/releases/latest", base.trim_end_matches('/'));
    let body = agent()
        .get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(classify)?
        .into_string()
        .map_err(|_| UpdateError::Offline)?;
    parse_release(&body, platform)
}

fn part_path(dest: &Path) -> PathBuf {
    let mut p = dest.as_os_str().to_owned();
    p.push(".part");
    p.into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Tải `url` về `dest` (qua `dest.part`), gọi `progress(đã tải, tổng)`; chỉ đổi tên thành `dest` khi đúng kích thước và SHA-256.
/// Mọi lỗi đều xóa file tạm.
pub fn download(url: &str, dest: &Path, expected_size: u64, expected_sha256: &str, progress: &mut dyn FnMut(u64, u64)) -> Result<(), UpdateError> {
    let resp = agent().get(url).set("User-Agent", USER_AGENT).call().map_err(classify)?;
    let part = part_path(dest);
    let result = stream_to(resp, &part, expected_size, expected_sha256, progress);
    match result {
        Ok(()) => std::fs::rename(&part, dest).map_err(|_| {
            let _ = std::fs::remove_file(&part);
            UpdateError::Io
        }),
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

fn stream_to(resp: ureq::Response, part: &Path, expected_size: u64, expected_sha256: &str, progress: &mut dyn FnMut(u64, u64)) -> Result<(), UpdateError> {
    // Đọc thêm 1 byte so với khai báo để biết gói dài hơn khai báo.
    let mut reader = resp.into_reader().take(expected_size.saturating_add(1));
    let mut file = std::fs::File::create(part).map_err(|_| UpdateError::Io)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0_u8; 64 * 1024];
    let mut done: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|_| UpdateError::Offline)?;
        if n == 0 {
            break;
        }
        done += n as u64;
        if done > expected_size {
            return Err(UpdateError::HashMismatch);
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|_| UpdateError::Io)?;
        progress(done, expected_size);
    }
    file.flush().map_err(|_| UpdateError::Io)?;
    if done != expected_size || !hex(&hasher.finalize()).eq_ignore_ascii_case(expected_sha256) {
        return Err(UpdateError::HashMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;
    use sha2::{Digest, Sha256};
    use std::path::PathBuf;

    const FIXTURE: &str = include_str!("../tests/data/release_latest.json");
    const PATH: &str = "/repos/phuoctrieu/luuluongV2/releases/latest";

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-update-net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
    }

    fn payload(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    fn part_of(dest: &Path) -> PathBuf {
        let mut p = dest.as_os_str().to_owned();
        p.push(".part");
        p.into()
    }

    #[test]
    fn check_parses_a_good_response() {
        let mut s = Server::new();
        let m = s.mock("GET", PATH).match_header("user-agent", "luuluong-updater").with_status(200).with_body(FIXTURE).expect(1).create();
        let r = check(&s.url(), Platform::Windows).unwrap();
        assert_eq!(r.asset_name, "luuluong-0.2.0-windows-x64.zip");
        m.assert();
    }

    #[test]
    fn check_maps_http_failures_to_friendly_errors() {
        for (status, want) in [(403, UpdateError::RateLimited), (429, UpdateError::RateLimited), (404, UpdateError::NotFound), (500, UpdateError::Offline), (503, UpdateError::Offline), (418, UpdateError::BadResponse)] {
            let mut s = Server::new();
            let _m = s.mock("GET", PATH).with_status(status).create();
            assert_eq!(check(&s.url(), Platform::Windows), Err(want), "HTTP {status}");
        }
    }

    #[test]
    fn check_unreachable_is_offline() {
        assert_eq!(check("http://127.0.0.1:1", Platform::Windows), Err(UpdateError::Offline));
    }

    #[test]
    fn check_bad_body_is_bad_response() {
        let mut s = Server::new();
        let _m = s.mock("GET", PATH).with_status(200).with_body("oops").create();
        assert_eq!(check(&s.url(), Platform::Windows), Err(UpdateError::BadResponse));
    }

    #[test]
    fn download_writes_file_and_reports_progress() {
        let data = payload(200_000);
        let mut s = Server::new();
        let _m = s.mock("GET", "/f.zip").with_status(200).with_body(&data).create();
        let dest = temp("ok.zip");
        let mut seen: Vec<(u64, u64)> = Vec::new();
        download(&format!("{}/f.zip", s.url()), &dest, data.len() as u64, &sha(&data), &mut |d, t| seen.push((d, t))).unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), data);
        assert!(!part_of(&dest).exists());
        assert!(seen.len() > 1, "tiến trình phải được báo nhiều lần");
        assert!(seen.windows(2).all(|w| w[0].0 < w[1].0), "tiến trình tăng dần");
        assert_eq!(*seen.last().unwrap(), (200_000, 200_000));
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn download_accepts_an_uppercase_expected_hash() {
        let data = payload(1000);
        let mut s = Server::new();
        let _m = s.mock("GET", "/u.zip").with_status(200).with_body(&data).create();
        let dest = temp("upper.zip");
        assert!(download(&format!("{}/u.zip", s.url()), &dest, 1000, &sha(&data).to_uppercase(), &mut |_, _| {}).is_ok());
        let _ = std::fs::remove_file(&dest);
    }

    #[test]
    fn download_with_wrong_hash_deletes_everything() {
        let data = payload(3072);
        let mut s = Server::new();
        let _m = s.mock("GET", "/bad.zip").with_status(200).with_body(&data).create();
        let dest = temp("bad.zip");
        let r = download(&format!("{}/bad.zip", s.url()), &dest, 3072, &"0".repeat(64), &mut |_, _| {});
        assert_eq!(r, Err(UpdateError::HashMismatch));
        assert!(!dest.exists() && !part_of(&dest).exists());
    }

    #[test]
    fn download_longer_or_shorter_than_declared_is_rejected() {
        let data = payload(5000);
        let mut s = Server::new();
        let _m = s.mock("GET", "/size.zip").with_status(200).with_body(&data).create();
        let url = format!("{}/size.zip", s.url());
        for declared in [3000_u64, 6000] {
            let dest = temp(&format!("size{declared}.zip"));
            let r = download(&url, &dest, declared, &sha(&data), &mut |_, _| {});
            assert_eq!(r, Err(UpdateError::HashMismatch), "khai {declared}");
            assert!(!dest.exists() && !part_of(&dest).exists());
        }
    }

    #[test]
    fn download_http_error_is_offline_and_leaves_no_file() {
        let mut s = Server::new();
        let _m = s.mock("GET", "/e.zip").with_status(500).create();
        let dest = temp("err.zip");
        let r = download(&format!("{}/e.zip", s.url()), &dest, 10, &"0".repeat(64), &mut |_, _| {});
        assert_eq!(r, Err(UpdateError::Offline));
        assert!(!dest.exists() && !part_of(&dest).exists());
        let nf = download(&format!("{}/missing.zip", s.url()), &dest, 10, &"0".repeat(64), &mut |_, _| {});
        assert!(nf.is_err());
    }

    #[test]
    fn download_into_missing_directory_is_io_error() {
        let dest = temp("no-such").join("nested").join("x.zip");
        let r = download("http://127.0.0.1:1/x", &dest, 1, &"0".repeat(64), &mut |_, _| {});
        assert!(matches!(r, Err(UpdateError::Io) | Err(UpdateError::Offline)));
    }
}
