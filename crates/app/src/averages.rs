//! Lưu lượng về trung bình theo các khoảng 3 giờ … 1 năm, tính từ các chu kỳ tự động đã lưu trong lịch sử.
//! Thuần logic (thời gian truyền vào).

use chrono::DateTime;
use luuluong_storage::history::Record;

use crate::live::is_automatic;

/// Nhãn và độ dài (giây) các khoảng; tháng = 30 ngày, năm = 365 ngày.
pub const WINDOWS: [(&str, i64); 6] = [
    ("3 giờ", 3 * 3600),
    ("12 giờ", 12 * 3600),
    ("1 ngày", 86_400),
    ("1 tuần", 7 * 86_400),
    ("1 tháng", 30 * 86_400),
    ("1 năm", 365 * 86_400),
];

#[derive(Debug, Clone, PartialEq)]
pub struct WindowAverage {
    pub label: &'static str,
    /// Bình quân theo thời gian (m³/s); `None` nếu khoảng này chưa có chu kỳ nào.
    pub q_avg: Option<f64>,
    /// Tỷ lệ thời gian của khoảng có số liệu, 0..=1.
    pub covered: f64,
}

/// Mỗi dòng tự động là một chu kỳ kết thúc lúc `created_at`, kéo dài `hours`. Chỉ phần nằm trong
/// khoảng `[now - W, now]` được tính; hai dòng chồng nhau thì dòng lưu trước (đứng trước trong `records`) được giữ. Khoảng trống không được bù: trung bình chỉ lấy thời gian có số liệu,
/// `covered` cho biết đã có bao nhiêu phần của khoảng.
pub fn window_averages(records: &[Record], now: i64) -> Vec<WindowAverage> {
    // (đầu, cuối, Q) theo thứ tự thời gian; phần chồng lên chu kỳ trước (dòng trùng) bị bỏ để không tính hai lần.
    let mut cycles: Vec<(i64, i64, f64)> = records
        .iter()
        .filter(|r| is_automatic(r))
        .filter_map(|r| {
            let end = DateTime::parse_from_rfc3339(&r.created_at).ok()?.timestamp();
            let seconds = (r.hours * 3600.0).round() as i64;
            (seconds > 0).then_some((end - seconds, end, r.q_in))
        })
        .collect();
    cycles.sort_by_key(|c| c.1);
    let mut segments = Vec::with_capacity(cycles.len());
    let mut covered_until = i64::MIN;
    for (start, end, q) in cycles {
        let start = start.max(covered_until);
        if start < end {
            segments.push((start, end, q));
            covered_until = end;
        }
    }
    WINDOWS
        .iter()
        .map(|&(label, window)| {
            let from = now - window;
            let (mut seconds, mut weighted) = (0.0, 0.0);
            for &(start, end, q) in &segments {
                let overlap = (end.min(now) - start.max(from)).max(0) as f64;
                seconds += overlap;
                weighted += overlap * q;
            }
            WindowAverage { label, q_avg: (seconds > 0.0).then(|| weighted / seconds), covered: seconds / window as f64 }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::AUTO_NOTE;
    use luuluong_storage::history::vn_timestamp;

    const NOW: i64 = 1_800_000_000;

    /// Chu kỳ `minutes` phút kết thúc cách `NOW` `ago_min` phút.
    fn cycle(ago_min: i64, minutes: i64, q: f64) -> Record {
        Record {
            id: 0,
            created_at: vn_timestamp(NOW - ago_min * 60),
            z_start: 157.0,
            z_end: 157.0,
            hours: minutes as f64 / 60.0,
            h1: 0.0,
            h2: 0.0,
            spill_q: 0.0,
            spill_h: 0.0,
            eco_q: 1.5,
            q_in: q,
            w_in: q * minutes as f64 * 60.0,
            operator: String::new(),
            note: AUTO_NOTE.into(),
        }
    }

    fn get<'a>(v: &'a [WindowAverage], label: &str) -> &'a WindowAverage {
        v.iter().find(|w| w.label == label).unwrap()
    }

    #[test]
    fn six_windows_in_order_and_empty_without_data() {
        let v = window_averages(&[], NOW);
        assert_eq!(v.iter().map(|w| w.label).collect::<Vec<_>>(), ["3 giờ", "12 giờ", "1 ngày", "1 tuần", "1 tháng", "1 năm"]);
        assert!(v.iter().all(|w| w.q_avg.is_none() && w.covered == 0.0));
    }

    #[test]
    fn two_equal_cycles_average_and_cover_a_sixth_of_three_hours() {
        let v = window_averages(&[cycle(30, 15, 10.0), cycle(15, 15, 20.0)], NOW);
        let w = get(&v, "3 giờ");
        assert!((w.q_avg.unwrap() - 15.0).abs() < 1e-9);
        assert!((w.covered - 30.0 / 180.0).abs() < 1e-9);
        assert!((get(&v, "1 năm").covered - 30.0 / 525_600.0).abs() < 1e-12);
    }

    #[test]
    fn fifteen_and_thirty_minute_cycles_are_weighted_by_duration() {
        let v = window_averages(&[cycle(45, 15, 10.0), cycle(15, 30, 40.0)], NOW);
        assert!((get(&v, "3 giờ").q_avg.unwrap() - 30.0).abs() < 1e-9); // (10·15 + 40·30) / 45
    }

    #[test]
    fn a_cycle_crossing_the_window_start_is_clipped() {
        // Kết thúc cách 175 phút, dài 15 phút nên bắt đầu cách 190 phút; cửa sổ 3 giờ = 180 phút → chỉ 5 phút trong cửa sổ.
        let v = window_averages(&[cycle(175, 15, 10.0), cycle(60, 15, 30.0)], NOW);
        let w = get(&v, "3 giờ");
        assert!((w.covered - 20.0 / 180.0).abs() < 1e-9);
        assert!((w.q_avg.unwrap() - (10.0 * 5.0 + 30.0 * 15.0) / 20.0).abs() < 1e-9);
        // khoảng 12 giờ thì lấy đủ cả hai
        assert!((get(&v, "12 giờ").covered - 30.0 / 720.0).abs() < 1e-9);
    }

    #[test]
    fn cycles_outside_a_window_do_not_count_in_it() {
        let v = window_averages(&[cycle(5 * 60, 15, 10.0)], NOW);
        assert_eq!(get(&v, "3 giờ").q_avg, None);
        assert!(get(&v, "12 giờ").q_avg.is_some());
    }

    #[test]
    fn hand_calculations_and_unparseable_rows_are_ignored() {
        let mut manual = cycle(30, 15, 999.0);
        manual.note = String::new();
        let mut broken = cycle(30, 15, 999.0);
        broken.created_at = "không rõ".into();
        let v = window_averages(&[manual, broken, cycle(15, 15, 12.0)], NOW);
        assert_eq!(get(&v, "3 giờ").q_avg, Some(12.0));
    }

    #[test]
    fn when_rows_clash_on_the_same_cycle_the_first_saved_is_kept() {
        let v = window_averages(&[cycle(15, 15, 5.0), cycle(15, 15, 99.0)], NOW);
        assert_eq!(get(&v, "3 giờ").q_avg, Some(5.0));
    }

    #[test]
    fn duplicate_rows_for_one_cycle_are_counted_once() {
        let rows: Vec<_> = (0..20).map(|_| cycle(15, 15, 5.0)).collect();
        let v = window_averages(&rows, NOW);
        assert!((get(&v, "3 giờ").covered - 15.0 / 180.0).abs() < 1e-9);
        assert_eq!(get(&v, "3 giờ").q_avg, Some(5.0));
    }
}
