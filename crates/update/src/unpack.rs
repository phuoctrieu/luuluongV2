//! Giải nén gói cập nhật (.zip) an toàn: chặn đường dẫn thoát khỏi thư mục đích, liên kết tượng trưng và gói quá lớn.

use std::fs::File;
use std::io::Read;
use std::path::Path;

use zip::ZipArchive;

use crate::UpdateError;

/// Tổng dung lượng sau giải nén tối đa.
pub const MAX_UNPACKED_BYTES: u64 = 300 * 1024 * 1024;

/// Giải nén `zip_path` vào `dest` (tạo nếu chưa có). Kiểm tra TOÀN BỘ mục trước khi ghi: có mục nào thoát khỏi `dest`
/// hoặc là liên kết tượng trưng thì không ghi gì.
pub fn unpack(zip_path: &Path, dest: &Path) -> Result<(), UpdateError> {
    unpack_with_limit(zip_path, dest, MAX_UNPACKED_BYTES)
}

/// Metadata AppleDouble (`._tên`) và thư mục `__MACOSX` do `ditto` sinh ra: bỏ qua, vì nếu ghi vào gói `.app` sẽ phá niêm phong chữ ký.
fn is_apple_metadata(rel: &Path) -> bool {
    rel.components().any(|c| c.as_os_str() == "__MACOSX") || rel.file_name().is_some_and(|n| n.to_string_lossy().starts_with("._"))
}

pub(crate) fn unpack_with_limit(zip_path: &Path, dest: &Path, max_bytes: u64) -> Result<(), UpdateError> {
    let file = File::open(zip_path).map_err(|_| UpdateError::Io)?;
    let mut archive = ZipArchive::new(file).map_err(|_| UpdateError::BadResponse)?;

    // Bước 1: kiểm tra mọi mục trước khi ghi bất cứ thứ gì.
    let mut declared: u64 = 0;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|_| UpdateError::BadResponse)?;
        if entry.name().starts_with('/') || entry.name().contains('\\') || entry.enclosed_name().is_none() || entry.is_symlink() {
            return Err(UpdateError::BadResponse);
        }
        declared = declared.saturating_add(entry.size());
        if declared > max_bytes {
            return Err(UpdateError::TooLarge);
        }
    }

    // Bước 2: ghi. Kích thước khai báo có thể nói dối, nên đếm lại số byte thật.
    std::fs::create_dir_all(dest).map_err(|_| UpdateError::Io)?;
    let mut written: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|_| UpdateError::BadResponse)?;
        let Some(rel) = entry.enclosed_name() else { return Err(UpdateError::BadResponse) };
        if is_apple_metadata(&rel) {
            continue;
        }
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|_| UpdateError::Io)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|_| UpdateError::Io)?;
        }
        let mut f = File::create(&out).map_err(|_| UpdateError::Io)?;
        let copied = std::io::copy(&mut (&mut entry).take(max_bytes - written + 1), &mut f).map_err(|_| UpdateError::Io)?;
        written += copied;
        if written > max_bytes {
            return Err(UpdateError::TooLarge);
        }
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&out, std::fs::Permissions::from_mode(mode & 0o777)).map_err(|_| UpdateError::Io)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use zip::write::SimpleFileOptions;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-update-unpack-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Dựng zip: (tên, nội dung, quyền unix); tên kết thúc bằng `/` là thư mục.
    fn make_zip(path: &Path, entries: &[(&str, &[u8], u32)]) {
        let mut w = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
        for (name, data, mode) in entries {
            let opts = SimpleFileOptions::default().unix_permissions(*mode);
            if name.ends_with('/') {
                w.add_directory(*name, opts).unwrap();
            } else {
                w.start_file(*name, opts).unwrap();
                w.write_all(data).unwrap();
            }
        }
        w.finish().unwrap();
    }

    #[test]
    fn unpack_extracts_files_and_directories() {
        let dir = temp_dir("ok");
        let z = dir.join("a.zip");
        make_zip(&z, &[("App.app/", b"", 0o755), ("App.app/Contents/Info.plist", b"<plist/>", 0o644), ("App.app/Contents/MacOS/luuluong", b"binary", 0o755)]);
        let out = dir.join("out");
        unpack(&z, &out).unwrap();
        assert_eq!(std::fs::read(out.join("App.app/Contents/Info.plist")).unwrap(), b"<plist/>");
        assert_eq!(std::fs::read(out.join("App.app/Contents/MacOS/luuluong")).unwrap(), b"binary");
    }

    #[cfg(unix)]
    #[test]
    fn unpack_keeps_executable_bit() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("exec");
        let z = dir.join("a.zip");
        make_zip(&z, &[("run", b"x", 0o755), ("data", b"y", 0o644)]);
        let out = dir.join("out");
        unpack(&z, &out).unwrap();
        assert_ne!(std::fs::metadata(out.join("run")).unwrap().permissions().mode() & 0o111, 0);
        assert_eq!(std::fs::metadata(out.join("data")).unwrap().permissions().mode() & 0o111, 0);
    }

    #[test]
    fn unpack_rejects_parent_traversal_and_writes_nothing() {
        let dir = temp_dir("trav");
        let z = dir.join("a.zip");
        make_zip(&z, &[("good.txt", b"ok", 0o644), ("../evil.txt", b"bad", 0o644)]);
        let out = dir.join("out");
        assert_eq!(unpack(&z, &out), Err(UpdateError::BadResponse));
        assert!(!dir.join("evil.txt").exists());
        assert!(!out.join("good.txt").exists(), "không ghi gì khi có mục xấu");
    }

    #[test]
    fn unpack_rejects_absolute_and_nested_traversal() {
        for bad in ["/abs.txt", "a/../../evil.txt", "..\\evil.txt"] {
            let dir = temp_dir("abs");
            let z = dir.join("a.zip");
            make_zip(&z, &[(bad, b"bad", 0o644)]);
            let out = dir.join("out");
            let r = unpack(&z, &out);
            assert_eq!(r, Err(UpdateError::BadResponse), "{bad}");
            assert!(!dir.join("evil.txt").exists() && !Path::new("/abs.txt").exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn unpack_rejects_symlinks() {
        let dir = temp_dir("sym");
        let z = dir.join("a.zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&z).unwrap());
        w.add_symlink("link", "/etc/passwd", SimpleFileOptions::default()).unwrap();
        w.finish().unwrap();
        assert_eq!(unpack(&z, &dir.join("out")), Err(UpdateError::BadResponse));
        assert!(!dir.join("out/link").exists());
    }

    #[test]
    fn unpack_rejects_oversized_archive() {
        let dir = temp_dir("big");
        let z = dir.join("a.zip");
        make_zip(&z, &[("big.bin", &vec![7_u8; 5000], 0o644)]);
        assert_eq!(unpack_with_limit(&z, &dir.join("out"), 4999), Err(UpdateError::TooLarge));
        assert!(unpack_with_limit(&z, &dir.join("out2"), 5000).is_ok());
    }

    #[test]
    fn unpack_skips_appledouble_metadata_so_the_bundle_seal_stays_intact() {
        // `ditto` có thể lưu thuộc tính mở rộng thành các mục `._*` và `__MACOSX/`; ghi chúng vào gói .app sẽ phá chữ ký.
        let dir = temp_dir("appledouble");
        let z = dir.join("a.zip");
        make_zip(
            &z,
            &[
                ("App.app/Contents/MacOS/luuluong", b"binary", 0o755),
                ("App.app/Contents/MacOS/._luuluong", b"meta", 0o644),
                ("App.app/Contents/._MacOS", b"meta", 0o644),
                ("App.app/Contents/.__CodeSignature", b"meta", 0o644),
                ("__MACOSX/App.app/._x", b"meta", 0o644),
            ],
        );
        let out = dir.join("out");
        unpack(&z, &out).unwrap();
        assert_eq!(std::fs::read(out.join("App.app/Contents/MacOS/luuluong")).unwrap(), b"binary");
        let mut names: Vec<String> = Vec::new();
        for e in walk(&out) {
            names.push(e.strip_prefix(&out).unwrap().to_string_lossy().into_owned());
        }
        assert!(names.iter().all(|n| !n.contains("._") && !n.contains("__MACOSX")), "còn file metadata: {names:?}");
    }

    #[test]
    fn unpack_still_rejects_traversal_hidden_inside_a_metadata_entry() {
        let dir = temp_dir("appledouble-trav");
        let z = dir.join("a.zip");
        make_zip(&z, &[("good.txt", b"ok", 0o644), ("../._evil", b"x", 0o644)]);
        assert_eq!(unpack(&z, &dir.join("out")), Err(UpdateError::BadResponse));
        assert!(!dir.join("out/good.txt").exists());
    }

    /// Mọi file (không phải thư mục) dưới `root`.
    fn walk(root: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        for e in std::fs::read_dir(root).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(walk(&p));
            } else {
                out.push(p);
            }
        }
        out
    }

    #[test]
    fn unpack_corrupt_or_missing_zip_is_an_error() {
        let dir = temp_dir("bad");
        let z = dir.join("a.zip");
        std::fs::write(&z, b"this is not a zip file").unwrap();
        assert_eq!(unpack(&z, &dir.join("out")), Err(UpdateError::BadResponse));
        assert_eq!(unpack(&dir.join("missing.zip"), &dir.join("out")), Err(UpdateError::Io));
    }
}
