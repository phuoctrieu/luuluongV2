//! Cập nhật ứng dụng: lịch tự kiểm tra và máy trạng thái của thẻ "Cập nhật". Thuần logic (thời gian truyền vào).

use luuluong_update::{is_newer, Release, UpdateError, Version};

/// Khoảng cách tối thiểu giữa hai lần tự kiểm tra.
pub const CHECK_INTERVAL_SECS: i64 = 24 * 3600;

/// Có nên tự kiểm tra bản mới lúc `now` không. Đồng hồ máy bị lùi (`now < last_check`) coi là đã quá hạn.
pub fn check_due(auto_check: bool, last_check: i64, now: i64) -> bool {
    auto_check && (last_check == 0 || now < last_check || now - last_check >= CHECK_INTERVAL_SECS)
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Downloading { done: u64, total: u64 },
    Installing,
    /// Câu tiếng Việt cho người vận hành.
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum UpdateEvent {
    CheckStarted,
    Checked(Result<Release, UpdateError>),
    InstallClicked,
    Progress(u64, u64),
    /// Đã thay xong, sắp mở lại.
    Installed,
    Error(UpdateError),
}

/// Chuyển trạng thái. Sự kiện không hợp lệ với trạng thái hiện tại bị bỏ qua (giữ nguyên), nhờ vậy bấm đúp hay kết quả đến muộn không làm hỏng luồng.
pub fn next(state: UpdateState, event: UpdateEvent, current: Version) -> UpdateState {
    use UpdateState::*;
    match (state, event) {
        (Idle | UpToDate | Failed(_) | Available(_), UpdateEvent::CheckStarted) => Checking,
        (Checking, UpdateEvent::Checked(Ok(r))) => {
            if is_newer(r.version, current) { Available(r) } else { UpToDate }
        }
        (Checking, UpdateEvent::Checked(Err(e))) => Failed(e.to_string()),
        (Available(r), UpdateEvent::InstallClicked) => Downloading { done: 0, total: r.size },
        (Downloading { .. }, UpdateEvent::Progress(done, total)) => Downloading { done, total },
        (Downloading { .. }, UpdateEvent::Installed) => Installing,
        (Downloading { .. } | Installing, UpdateEvent::Error(e)) => Failed(e.to_string()),
        (state, _) => state,
    }
}

pub fn can_install(state: &UpdateState) -> bool {
    matches!(state, UpdateState::Available(_))
}

/// Bản mới đang chờ người dùng quyết định (cho chip trên thanh đầu).
pub fn available(state: &UpdateState) -> Option<&Release> {
    match state {
        UpdateState::Available(r) => Some(r),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CURRENT: Version = Version(0, 2, 0);

    fn release(v: Version) -> Release {
        Release { version: v, notes: "ghi chú".into(), asset_name: "luuluong-x-windows-x64.zip".into(), url: "https://github.com/phuoctrieu/luuluongV2/releases/download/v/x.zip".into(), size: 1000, sha256: "0".repeat(64) }
    }

    fn step(state: UpdateState, event: UpdateEvent) -> UpdateState {
        next(state, event, CURRENT)
    }

    #[test]
    fn checking_then_newer_becomes_available() {
        let s = step(UpdateState::Idle, UpdateEvent::CheckStarted);
        assert_eq!(s, UpdateState::Checking);
        let r = release(Version(0, 3, 0));
        assert_eq!(step(s, UpdateEvent::Checked(Ok(r.clone()))), UpdateState::Available(r));
    }

    #[test]
    fn checked_same_or_older_is_up_to_date() {
        for v in [Version(0, 2, 0), Version(0, 1, 9)] {
            assert_eq!(step(UpdateState::Checking, UpdateEvent::Checked(Ok(release(v)))), UpdateState::UpToDate, "{v}");
        }
    }

    #[test]
    fn checked_error_becomes_failed_with_vietnamese_text() {
        let s = step(UpdateState::Checking, UpdateEvent::Checked(Err(UpdateError::Offline)));
        assert_eq!(s, UpdateState::Failed(UpdateError::Offline.to_string()));
        assert!(matches!(&s, UpdateState::Failed(m) if !m.is_ascii()));
    }

    #[test]
    fn a_check_result_arriving_when_not_checking_is_ignored() {
        let downloading = UpdateState::Downloading { done: 1, total: 2 };
        let ev = UpdateEvent::Checked(Ok(release(Version(9, 0, 0))));
        for s in [UpdateState::Idle, UpdateState::UpToDate, UpdateState::Installing, downloading] {
            assert_eq!(step(s.clone(), ev.clone()), s);
        }
    }

    #[test]
    fn install_click_only_works_from_available() {
        let r = release(Version(0, 3, 0));
        assert_eq!(step(UpdateState::Available(r), UpdateEvent::InstallClicked), UpdateState::Downloading { done: 0, total: 1000 });
        for s in [UpdateState::Idle, UpdateState::Checking, UpdateState::UpToDate, UpdateState::Downloading { done: 5, total: 9 }, UpdateState::Installing, UpdateState::Failed("x".into())] {
            assert_eq!(step(s.clone(), UpdateEvent::InstallClicked), s, "bấm đúp/bấm sai lúc không được phép");
        }
    }

    #[test]
    fn progress_only_in_downloading() {
        assert_eq!(step(UpdateState::Downloading { done: 0, total: 10 }, UpdateEvent::Progress(7, 10)), UpdateState::Downloading { done: 7, total: 10 });
        assert_eq!(step(UpdateState::Idle, UpdateEvent::Progress(7, 10)), UpdateState::Idle);
        assert_eq!(step(UpdateState::Installing, UpdateEvent::Progress(7, 10)), UpdateState::Installing);
    }

    #[test]
    fn installed_moves_downloading_to_installing_and_errors_fail_it() {
        assert_eq!(step(UpdateState::Downloading { done: 10, total: 10 }, UpdateEvent::Installed), UpdateState::Installing);
        assert_eq!(step(UpdateState::Idle, UpdateEvent::Installed), UpdateState::Idle);
        for s in [UpdateState::Downloading { done: 1, total: 2 }, UpdateState::Installing] {
            assert_eq!(step(s, UpdateEvent::Error(UpdateError::HashMismatch)), UpdateState::Failed(UpdateError::HashMismatch.to_string()));
        }
        assert_eq!(step(UpdateState::UpToDate, UpdateEvent::Error(UpdateError::Io)), UpdateState::UpToDate);
    }

    #[test]
    fn a_new_check_is_allowed_unless_busy() {
        for s in [UpdateState::Idle, UpdateState::UpToDate, UpdateState::Failed("x".into()), UpdateState::Available(release(Version(0, 3, 0)))] {
            assert_eq!(step(s, UpdateEvent::CheckStarted), UpdateState::Checking);
        }
        for s in [UpdateState::Checking, UpdateState::Downloading { done: 1, total: 2 }, UpdateState::Installing] {
            assert_eq!(step(s.clone(), UpdateEvent::CheckStarted), s);
        }
    }

    #[test]
    fn can_install_and_available_only_when_available() {
        let a = UpdateState::Available(release(Version(0, 3, 0)));
        assert!(can_install(&a));
        assert_eq!(available(&a).map(|r| r.version), Some(Version(0, 3, 0)));
        for s in [UpdateState::Idle, UpdateState::Checking, UpdateState::UpToDate, UpdateState::Downloading { done: 0, total: 1 }, UpdateState::Installing, UpdateState::Failed("x".into())] {
            assert!(!can_install(&s));
            assert!(available(&s).is_none());
        }
    }

    const DAY: i64 = 24 * 3600;

    #[test]
    fn check_due_after_24h_and_when_never_checked() {
        assert!(check_due(true, 0, 1_790_000_000), "chưa từng kiểm tra");
        assert!(check_due(true, 1_000, 1_000 + DAY), "đúng 24 giờ");
        assert!(check_due(true, 1_000, 1_000 + DAY + 1));
    }

    #[test]
    fn check_not_due_within_24h() {
        assert!(!check_due(true, 1_000, 1_000 + DAY - 1));
        assert!(!check_due(true, 1_000, 1_000));
    }

    #[test]
    fn check_not_due_when_auto_check_is_off() {
        assert!(!check_due(false, 0, 1_790_000_000));
    }

    #[test]
    fn check_due_when_the_clock_went_backwards() {
        assert!(check_due(true, 2_000_000, 1_000_000));
    }
}
