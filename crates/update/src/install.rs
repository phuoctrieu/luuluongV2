//! Thay thế bản đang chạy bằng bản mới bằng cách đổi tên (nhanh, quay lui được). Không đụng dữ liệu người dùng.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::UpdateError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Đường dẫn tới gói `….app`.
    MacApp(PathBuf),
    /// Đường dẫn tới `luuluong.exe` đang chạy.
    WinExe(PathBuf),
}

/// Tiền tố thư mục tải về trong thư mục tạm của hệ điều hành.
pub const DOWNLOAD_PREFIX_DIR: &str = "luuluong-update-dl-";
/// Tiền tố thư mục giải nén, đặt cạnh bản cài đặt để đổi tên không phải qua ổ đĩa khác.
pub const STAGING_PREFIX: &str = ".luuluong-update-";

/// `…/X.app/Contents/MacOS/luuluong` → `MacApp(…/X.app)`; `…/luuluong.exe` → `WinExe`; còn lại (vd. `target/debug/app`) → `NotInstalled`.
pub fn detect_target(current_exe: &Path) -> Result<Target, UpdateError> {
    let file = current_exe.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase());
    if file.as_deref() == Some("luuluong.exe") {
        return Ok(Target::WinExe(current_exe.to_path_buf()));
    }
    let name_of = |p: Option<&Path>| p.and_then(Path::file_name).map(|n| n.to_string_lossy().into_owned());
    let macos = current_exe.parent();
    let contents = macos.and_then(Path::parent);
    let app = contents.and_then(Path::parent);
    let is_bundle = name_of(macos).as_deref() == Some("MacOS")
        && name_of(contents).as_deref() == Some("Contents")
        && name_of(app).is_some_and(|n| n.ends_with(".app"));
    match app {
        Some(app) if is_bundle => Ok(Target::MacApp(app.to_path_buf())),
        _ => Err(UpdateError::NotInstalled),
    }
}

fn install_path(target: &Target) -> &Path {
    match target {
        Target::MacApp(p) | Target::WinExe(p) => p,
    }
}

/// Thư mục giải nén: cùng thư mục cha với bản cài đặt.
pub fn staging_dir(target: &Target) -> PathBuf {
    let parent = install_path(target).parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{STAGING_PREFIX}{}", std::process::id()))
}

/// Thư mục tải về (trong `temp_root`) cho tiến trình hiện tại.
pub fn download_dir(temp_root: &Path) -> PathBuf {
    temp_root.join(format!("{DOWNLOAD_PREFIX_DIR}{}", std::process::id()))
}

/// Bản cũ sau khi bị thay: `X.app.old` hoặc `luuluong.old.exe`.
fn old_path(target: &Target) -> PathBuf {
    match target {
        Target::MacApp(app) => {
            let mut name = app.as_os_str().to_owned();
            name.push(".old");
            name.into()
        }
        Target::WinExe(exe) => exe.with_file_name("luuluong.old.exe"),
    }
}

fn map_io(e: &std::io::Error) -> UpdateError {
    match e.kind() {
        ErrorKind::PermissionDenied | ErrorKind::ReadOnlyFilesystem => UpdateError::NotWritable,
        _ => UpdateError::Io,
    }
}

fn remove_any(path: &Path) -> std::io::Result<()> {
    match path.symlink_metadata() {
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(_) => Ok(()),
    }
}

/// `current` → `old`, `new` → `current`; bước sau lỗi thì đổi `old` về lại `current`.
fn replace(current: &Path, new: &Path, old: &Path) -> Result<(), UpdateError> {
    remove_any(old).map_err(|e| map_io(&e))?;
    std::fs::rename(current, old).map_err(|e| map_io(&e))?;
    if let Err(e) = std::fs::rename(new, current) {
        let _ = std::fs::rename(old, current);
        return Err(map_io(&e));
    }
    Ok(())
}

/// `new_payload_dir` chứa một `….app` (macOS) hoặc `luuluong.exe` (Windows). Lỗi ở bước nào cũng quay lui về bản cũ.
pub fn swap(target: &Target, new_payload_dir: &Path) -> Result<(), UpdateError> {
    let old = old_path(target);
    match target {
        Target::MacApp(app) => {
            let new_app = std::fs::read_dir(new_payload_dir)
                .map_err(|e| map_io(&e))?
                .flatten()
                .map(|e| e.path())
                .find(|p| p.is_dir() && p.extension().is_some_and(|x| x == "app"))
                .ok_or(UpdateError::BadResponse)?;
            replace(app, &new_app, &old)
        }
        Target::WinExe(exe) => {
            let new_exe = new_payload_dir.join("luuluong.exe");
            if !new_exe.is_file() {
                return Err(UpdateError::BadResponse);
            }
            replace(exe, &new_exe, &old)
        }
    }
}

/// Xóa `.old` và thư mục tạm còn sót lại (kể cả của lần cập nhật dở); không lỗi nếu không có gì.
pub fn cleanup(target: &Target, temp_root: &Path) {
    let _ = remove_any(&old_path(target));
    let sweep = |dir: Option<&Path>, prefix: &str| {
        let Some(entries) = dir.and_then(|d| std::fs::read_dir(d).ok()) else { return };
        for e in entries.flatten() {
            if e.file_name().to_string_lossy().starts_with(prefix) {
                let _ = remove_any(&e.path());
            }
        }
    };
    sweep(install_path(target).parent(), STAGING_PREFIX);
    sweep(Some(temp_root), DOWNLOAD_PREFIX_DIR);
}

/// Lệnh mở lại app: chương trình và đối số.
pub fn relaunch_command(target: &Target) -> (String, Vec<String>) {
    match target {
        Target::MacApp(app) => ("open".to_string(), vec!["-n".to_string(), app.to_string_lossy().into_owned()]),
        Target::WinExe(exe) => (exe.to_string_lossy().into_owned(), Vec::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-update-inst-{}-{name}", std::process::id()));
        let _ = make_writable(&dir);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn make_writable(dir: &Path) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(dir, fs::Permissions::from_mode(0o755))?;
        }
        let _ = dir;
        Ok(())
    }

    /// Dựng gói `.app` giả có file `Contents/MacOS/luuluong` chứa `marker`.
    fn fake_app(parent: &Path, name: &str, marker: &str) -> PathBuf {
        let app = parent.join(name);
        fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        fs::write(app.join("Contents/MacOS/luuluong"), marker).unwrap();
        app
    }

    fn marker(app: &Path) -> String {
        fs::read_to_string(app.join("Contents/MacOS/luuluong")).unwrap()
    }

    #[test]
    fn detect_target_for_mac_bundle_path() {
        let t = detect_target(Path::new("/Applications/Luu luong Song Bo.app/Contents/MacOS/luuluong")).unwrap();
        assert_eq!(t, Target::MacApp(PathBuf::from("/Applications/Luu luong Song Bo.app")));
    }

    #[test]
    fn detect_target_for_windows_exe_path() {
        for p in ["/home/u/luuluong.exe", "/home/u/LUULUONG.EXE"] {
            assert_eq!(detect_target(Path::new(p)).unwrap(), Target::WinExe(PathBuf::from(p)), "{p}");
        }
    }

    #[test]
    fn detect_target_rejects_dev_binaries_and_odd_layouts() {
        for p in ["/proj/target/debug/app", "/proj/target/release/app", "/x/Foo.app/Contents/Resources/luuluong", "/x/Foo/Contents/MacOS/luuluong", "/usr/bin/other.exe"] {
            assert_eq!(detect_target(Path::new(p)), Err(UpdateError::NotInstalled), "{p}");
        }
    }

    #[test]
    fn staging_dir_is_a_sibling_so_rename_stays_on_one_volume() {
        let t = Target::MacApp(PathBuf::from("/Applications/X.app"));
        let s = staging_dir(&t);
        assert_eq!(s.parent(), Some(Path::new("/Applications")));
        assert!(s.file_name().unwrap().to_string_lossy().starts_with(STAGING_PREFIX));
        let w = staging_dir(&Target::WinExe(PathBuf::from("/home/u/luuluong.exe")));
        assert_eq!(w.parent(), Some(Path::new("/home/u")));
        assert!(download_dir(Path::new("/tmp")).file_name().unwrap().to_string_lossy().starts_with(DOWNLOAD_PREFIX_DIR));
    }

    #[test]
    fn swap_mac_replaces_bundle_and_keeps_old_as_dot_old() {
        let r = root("mac-ok");
        let app = fake_app(&r.join("Apps"), "X.app", "cu");
        fake_app(&r.join("Apps/.stage"), "X.app", "moi");
        swap(&Target::MacApp(app.clone()), &r.join("Apps/.stage")).unwrap();
        assert_eq!(marker(&app), "moi");
        assert_eq!(marker(&r.join("Apps/X.app.old")), "cu");
        assert!(!r.join("Apps/.stage/X.app").exists(), "bản mới đã được chuyển đi");
    }

    #[test]
    fn swap_replaces_a_stale_dot_old() {
        let r = root("stale");
        let app = fake_app(&r.join("Apps"), "X.app", "cu");
        fake_app(&r.join("Apps"), "X.app.old", "rac");
        fake_app(&r.join("stage"), "X.app", "moi");
        swap(&Target::MacApp(app.clone()), &r.join("stage")).unwrap();
        assert_eq!(marker(&app), "moi");
        assert_eq!(marker(&r.join("Apps/X.app.old")), "cu", ".old phải là bản vừa bị thay, không phải rác cũ");
    }

    #[test]
    fn swap_without_a_new_bundle_changes_nothing() {
        let r = root("empty");
        let app = fake_app(&r.join("Apps"), "X.app", "cu");
        fs::create_dir_all(r.join("stage")).unwrap();
        assert_eq!(swap(&Target::MacApp(app.clone()), &r.join("stage")), Err(UpdateError::BadResponse));
        assert_eq!(marker(&app), "cu");
        assert!(!r.join("Apps/X.app.old").exists());
    }

    #[cfg(unix)]
    #[test]
    fn swap_mac_rolls_back_when_the_second_rename_fails() {
        use std::os::unix::fs::PermissionsExt;
        let r = root("rollback");
        let app = fake_app(&r.join("Apps"), "X.app", "cu");
        fake_app(&r.join("stage"), "X.app", "moi");
        // Thư mục chứa bản mới không ghi được → không rút bản mới ra được ở bước 2, sau khi bản cũ đã bị đổi tên ở bước 1.
        fs::set_permissions(r.join("stage"), fs::Permissions::from_mode(0o555)).unwrap();
        let result = swap(&Target::MacApp(app.clone()), &r.join("stage"));
        make_writable(&r.join("stage")).unwrap();
        assert!(result.is_err());
        assert_eq!(marker(&app), "cu", "bản cũ phải còn nguyên tại chỗ");
        assert!(!r.join("Apps/X.app.old").exists(), "không để .old treo sau khi quay lui");
    }

    #[cfg(unix)]
    #[test]
    fn swap_fails_cleanly_when_parent_is_read_only() {
        use std::os::unix::fs::PermissionsExt;
        let r = root("readonly");
        let app = fake_app(&r.join("Apps"), "X.app", "cu");
        fake_app(&r.join("stage"), "X.app", "moi");
        fs::set_permissions(r.join("Apps"), fs::Permissions::from_mode(0o555)).unwrap();
        let result = swap(&Target::MacApp(app.clone()), &r.join("stage"));
        make_writable(&r.join("Apps")).unwrap();
        assert_eq!(result, Err(UpdateError::NotWritable));
        assert_eq!(marker(&app), "cu");
        assert_eq!(marker(&r.join("stage/X.app")), "moi", "bản mới vẫn nguyên, không mất");
    }

    #[test]
    fn swap_windows_style_exe() {
        let r = root("win");
        fs::create_dir_all(r.join("Prog")).unwrap();
        fs::write(r.join("Prog/luuluong.exe"), "cu").unwrap();
        fs::create_dir_all(r.join("stage")).unwrap();
        fs::write(r.join("stage/luuluong.exe"), "moi").unwrap();
        swap(&Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("stage")).unwrap();
        assert_eq!(fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "moi");
        assert_eq!(fs::read_to_string(r.join("Prog/luuluong.old.exe")).unwrap(), "cu");
        // Gói thiếu luuluong.exe thì không đụng gì.
        fs::create_dir_all(r.join("empty")).unwrap();
        assert_eq!(swap(&Target::WinExe(r.join("Prog/luuluong.exe")), &r.join("empty")), Err(UpdateError::BadResponse));
        assert_eq!(fs::read_to_string(r.join("Prog/luuluong.exe")).unwrap(), "moi");
    }

    #[test]
    fn cleanup_removes_old_and_temp_dirs_and_tolerates_absence() {
        let r = root("clean");
        let app = fake_app(&r.join("Apps"), "X.app", "hien-tai");
        fake_app(&r.join("Apps"), "X.app.old", "cu");
        fs::create_dir_all(r.join("Apps/.luuluong-update-123/X.app")).unwrap();
        fs::create_dir_all(r.join("Apps/Khac")).unwrap();
        fs::create_dir_all(r.join("tmp/luuluong-update-dl-9")).unwrap();
        fs::create_dir_all(r.join("tmp/luuluong-update-net-keep")).unwrap();
        let t = Target::MacApp(app.clone());
        cleanup(&t, &r.join("tmp"));
        assert!(!r.join("Apps/X.app.old").exists());
        assert!(!r.join("Apps/.luuluong-update-123").exists());
        assert!(!r.join("tmp/luuluong-update-dl-9").exists());
        assert_eq!(marker(&app), "hien-tai");
        assert!(r.join("Apps/Khac").exists() && r.join("tmp/luuluong-update-net-keep").exists(), "chỉ xóa đúng tiền tố của mình");
        cleanup(&t, &r.join("tmp"));
        cleanup(&t, &r.join("khong-co-thu-muc"));
        let win = Target::WinExe(r.join("Prog/luuluong.exe"));
        fs::create_dir_all(r.join("Prog")).unwrap();
        fs::write(r.join("Prog/luuluong.old.exe"), "cu").unwrap();
        cleanup(&win, &r.join("tmp"));
        assert!(!r.join("Prog/luuluong.old.exe").exists());
    }

    #[test]
    fn relaunch_command_for_both_targets() {
        let (cmd, args) = relaunch_command(&Target::MacApp(PathBuf::from("/Applications/X.app")));
        assert_eq!((cmd.as_str(), args), ("open", vec!["-n".to_string(), "/Applications/X.app".to_string()]));
        let (cmd, args) = relaunch_command(&Target::WinExe(PathBuf::from("/p/luuluong.exe")));
        assert_eq!(cmd, "/p/luuluong.exe");
        assert!(args.is_empty());
    }
}
