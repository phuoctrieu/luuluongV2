//! Cập nhật ứng dụng: lịch tự kiểm tra và máy trạng thái của thẻ "Cập nhật". Thuần logic (thời gian truyền vào).

/// Khoảng cách tối thiểu giữa hai lần tự kiểm tra.
pub const CHECK_INTERVAL_SECS: i64 = 24 * 3600;

/// Có nên tự kiểm tra bản mới lúc `now` không. Đồng hồ máy bị lùi (`now < last_check`) coi là đã quá hạn.
pub fn check_due(auto_check: bool, last_check: i64, now: i64) -> bool {
    auto_check && (last_check == 0 || now < last_check || now - last_check >= CHECK_INTERVAL_SECS)
}

#[cfg(test)]
mod tests {
    use super::*;

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
