//! Đường ống cài đặt: kiểm quyền ghi → tải (kiểm SHA-256) → giải nén cạnh bản cài → đổi tên thay thế → dọn.
//! Mọi lỗi để bản đang chạy nguyên vẹn và không để lại file tạm.

use std::path::Path;

use crate::install::map_io;
use crate::{download, download_dir, staging_dir, swap, unpack, Release, Target, UpdateError};

/// Cài `release` thay cho `target`. `temp_root` là thư mục tạm của hệ điều hành; `progress(đã tải, tổng)` báo tiến trình tải.
/// Không mở lại app: việc đó do bên gọi làm sau khi hàm trả `Ok`.
pub fn install_release(release: &Release, target: &Target, temp_root: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<(), UpdateError> {
    let stage = staging_dir(target);
    let downloads = download_dir(temp_root);
    let result = run(release, target, &stage, &downloads, progress);
    // Dù thành công hay lỗi, thư mục tạm của lần này không được để lại.
    let _ = std::fs::remove_dir_all(&downloads);
    let _ = std::fs::remove_dir_all(&stage);
    result
}

fn run(release: &Release, target: &Target, stage: &Path, downloads: &Path, progress: &mut dyn FnMut(u64, u64)) -> Result<(), UpdateError> {
    // Tạo thư mục giải nén cạnh bản cài TRƯỚC khi tải: không ghi được thì báo ngay, khỏi tải vô ích.
    let _ = std::fs::remove_dir_all(stage);
    std::fs::create_dir_all(stage).map_err(|e| map_io(&e))?;
    std::fs::create_dir_all(downloads).map_err(|_| UpdateError::Io)?;
    let file_name = Path::new(&release.asset_name).file_name().ok_or(UpdateError::BadResponse)?;
    let zip = downloads.join(file_name);
    download(&release.url, &zip, release.size, &release.sha256, progress)?;
    unpack(&zip, stage)?;
    swap(target, stage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Version;
    use mockito::Server;
    use sha2::{Digest, Sha256};
    use std::io::Write;
    use std::path::PathBuf;
    use zip::write::SimpleFileOptions;

    fn root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-update-pipe-{}-{name}", std::process::id()));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755));
        }
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn zip_bytes(entries: &[(&str, &str)]) -> Vec<u8> {
        let mut w = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, data) in entries {
            w.start_file(*name, SimpleFileOptions::default().unix_permissions(0o755)).unwrap();
            w.write_all(data.as_bytes()).unwrap();
        }
        w.finish().unwrap().into_inner()
    }

    fn release_for(url: String, bytes: &[u8]) -> Release {
        Release {
            version: Version(0, 3, 0),
            notes: String::new(),
            asset_name: "luuluong-0.3.0-windows-x64.zip".into(),
            url,
            size: bytes.len() as u64,
            sha256: Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect(),
        }
    }

    fn leftovers(parent: &Path, temp: &Path) -> Vec<String> {
        let mut found = Vec::new();
        for dir in [parent, temp] {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let n = e.file_name().to_string_lossy().into_owned();
                if n.starts_with(crate::STAGING_PREFIX) || n.starts_with(crate::DOWNLOAD_PREFIX_DIR) {
                    found.push(n);
                }
            }
        }
        found
    }

    #[test]
    fn installs_a_windows_style_exe_end_to_end() {
        let r = root("win");
        std::fs::create_dir_all(r.join("Prog")).unwrap();
        std::fs::write(r.join("Prog/luuluong.exe"), "cu").unwrap();
        let bytes = zip_bytes(&[("luuluong.exe", "moi")]);
        let mut s = Server::new();
        let m = s.mock("GET", "/w.zip").with_status(200).with_body(&bytes).expect(1).create();
        let rel = release_for(format!("{}/w.zip", s.url()), &bytes);
        let mut last = (0, 0);
        install_release(&rel, &Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("tmp"), &mut |d, t| last = (d, t)).unwrap();
        m.assert();
        assert_eq!(std::fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "moi");
        assert_eq!(std::fs::read_to_string(r.join("Prog/luuluong.old.exe")).unwrap(), "cu");
        assert_eq!(last, (bytes.len() as u64, bytes.len() as u64));
        assert!(leftovers(&r.join("Prog"), &r.join("tmp")).is_empty(), "không để lại thư mục tạm");
    }

    #[test]
    fn installs_a_mac_bundle_end_to_end() {
        let r = root("mac");
        let app = r.join("Apps/X.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        std::fs::write(app.join("Contents/MacOS/luuluong"), "cu").unwrap();
        let bytes = zip_bytes(&[("X.app/Contents/MacOS/luuluong", "moi"), ("X.app/Contents/Info.plist", "<plist/>")]);
        let mut s = Server::new();
        let _m = s.mock("GET", "/m.zip").with_status(200).with_body(&bytes).create();
        let rel = release_for(format!("{}/m.zip", s.url()), &bytes);
        install_release(&rel, &Target::MacApp(app.clone()), &r.join("tmp"), &mut |_, _| {}).unwrap();
        assert_eq!(std::fs::read_to_string(app.join("Contents/MacOS/luuluong")).unwrap(), "moi");
        assert!(app.join("Contents/Info.plist").exists());
        assert_eq!(std::fs::read_to_string(r.join("Apps/X.app.old/Contents/MacOS/luuluong")).unwrap(), "cu");
        assert!(leftovers(&r.join("Apps"), &r.join("tmp")).is_empty());
    }

    #[test]
    fn wrong_hash_leaves_the_installed_app_untouched_and_no_leftovers() {
        let r = root("hash");
        std::fs::create_dir_all(r.join("Prog")).unwrap();
        std::fs::write(r.join("Prog/luuluong.exe"), "cu").unwrap();
        let bytes = zip_bytes(&[("luuluong.exe", "gia mao")]);
        let mut s = Server::new();
        let _m = s.mock("GET", "/w.zip").with_status(200).with_body(&bytes).create();
        let mut rel = release_for(format!("{}/w.zip", s.url()), &bytes);
        rel.sha256 = "0".repeat(64);
        let result = install_release(&rel, &Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("tmp"), &mut |_, _| {});
        assert_eq!(result, Err(UpdateError::HashMismatch));
        assert_eq!(std::fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "cu");
        assert!(!r.join("Prog/luuluong.old.exe").exists());
        assert!(leftovers(&r.join("Prog"), &r.join("tmp")).is_empty());
    }

    #[test]
    fn a_zip_with_a_traversal_entry_is_rejected_after_download_and_cleaned_up() {
        let r = root("trav");
        std::fs::create_dir_all(r.join("Prog")).unwrap();
        std::fs::write(r.join("Prog/luuluong.exe"), "cu").unwrap();
        let bytes = zip_bytes(&[("luuluong.exe", "moi"), ("../evil.txt", "x")]);
        let mut s = Server::new();
        let _m = s.mock("GET", "/w.zip").with_status(200).with_body(&bytes).create();
        let rel = release_for(format!("{}/w.zip", s.url()), &bytes);
        let result = install_release(&rel, &Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("tmp"), &mut |_, _| {});
        assert_eq!(result, Err(UpdateError::BadResponse));
        assert_eq!(std::fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "cu");
        assert!(!r.join("Prog/evil.txt").exists() && !r.join("evil.txt").exists());
        assert!(leftovers(&r.join("Prog"), &r.join("tmp")).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unwritable_install_dir_fails_before_downloading_anything() {
        use std::os::unix::fs::PermissionsExt;
        let r = root("ro");
        std::fs::create_dir_all(r.join("Prog")).unwrap();
        std::fs::write(r.join("Prog/luuluong.exe"), "cu").unwrap();
        let mut s = Server::new();
        let m = s.mock("GET", "/w.zip").with_status(200).with_body("x").expect(0).create();
        let rel = release_for(format!("{}/w.zip", s.url()), b"x");
        std::fs::set_permissions(r.join("Prog"), std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = install_release(&rel, &Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("tmp"), &mut |_, _| {});
        std::fs::set_permissions(r.join("Prog"), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(result, Err(UpdateError::NotWritable));
        m.assert();
        assert_eq!(std::fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "cu");
    }
}
