//! Lưu lượng về trung bình theo các khoảng 3 giờ … 1 năm, tính từ các chu kỳ tự động đã lưu trong lịch sử.
//! Thuần logic (thời gian truyền vào).

use chrono::{DateTime, NaiveDate};
use luuluong_storage::history::Record;

use crate::live::{is_automatic, VN_TZ};

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
    /// Lượng mưa cộng dồn (mm) của phần có số liệu mưa; `None` nếu khoảng này chưa có số liệu mưa nào.
    pub rain_mm: Option<f64>,
    /// Tỷ lệ thời gian của khoảng có số liệu mưa, 0..=1.
    pub rain_covered: f64,
}

/// Thống kê một ngày (giờ Việt Nam).
#[derive(Debug, Clone, PartialEq)]
pub struct DayStat {
    pub date: NaiveDate,
    pub q_avg: Option<f64>,
    pub rain_mm: Option<f64>,
    /// Tỷ lệ thời gian của ngày (tính đến hiện tại với hôm nay) có số liệu lưu lượng, 0..=1.
    pub covered: f64,
}

/// Một chu kỳ đã lưu: [đầu, cuối], lưu lượng về (m³/s) và lượng mưa cả chu kỳ (mm) nếu có đo.
struct Segment {
    start: i64,
    end: i64,
    q: f64,
    rain_mm: Option<f64>,
}

/// Các chu kỳ tự động theo thứ tự thời gian; phần chồng lên chu kỳ trước (dòng trùng) bị bỏ để không tính hai lần.
fn segments(records: &[Record]) -> Vec<Segment> {
    let mut cycles: Vec<Segment> = records
        .iter()
        .filter(|r| is_automatic(r))
        .filter_map(|r| {
            let end = DateTime::parse_from_rfc3339(&r.created_at).ok()?.timestamp();
            let seconds = (r.hours * 3600.0).round() as i64;
            (seconds > 0).then_some(Segment { start: end - seconds, end, q: r.q_in, rain_mm: r.rain_mm })
        })
        .collect();
    cycles.sort_by_key(|c| c.end);
    let mut out: Vec<Segment> = Vec::with_capacity(cycles.len());
    let mut covered_until = i64::MIN;
    for c in cycles {
        let start = c.start.max(covered_until);
        if start < c.end {
            // Mưa của chu kỳ bị cắt bớt giảm theo tỷ lệ thời gian còn lại.
            let rain_mm = c.rain_mm.map(|mm| mm * (c.end - start) as f64 / (c.end - c.start) as f64);
            covered_until = c.end;
            out.push(Segment { start, rain_mm, ..c });
        }
    }
    out
}

/// Thống kê phần của các chu kỳ nằm trong `[from, to]`.
struct Span {
    q_avg: Option<f64>,
    covered: f64,
    rain_mm: Option<f64>,
    rain_covered: f64,
}

fn span(segments: &[Segment], from: i64, to: i64) -> Span {
    let length = (to - from).max(1) as f64;
    let (mut seconds, mut weighted, mut rain_seconds, mut rain, mut has_rain) = (0.0, 0.0, 0.0, 0.0, false);
    for c in segments {
        let overlap = (c.end.min(to) - c.start.max(from)).max(0) as f64;
        if overlap == 0.0 {
            continue;
        }
        seconds += overlap;
        weighted += overlap * c.q;
        if let Some(mm) = c.rain_mm {
            // `mm` là lượng mưa của đoạn [start, end] đã cắt; lấy phần chồng với khoảng đang xét.
            rain += mm * overlap / (c.end - c.start) as f64;
            rain_seconds += overlap;
            has_rain = true;
        }
    }
    Span { q_avg: (seconds > 0.0).then(|| weighted / seconds), covered: seconds / length, rain_mm: has_rain.then_some(rain), rain_covered: rain_seconds / length }
}

/// Mỗi dòng tự động là một chu kỳ kết thúc lúc `created_at`, kéo dài `hours`. Chỉ phần nằm trong
/// khoảng `[now - W, now]` được tính; hai dòng chồng nhau thì dòng lưu trước (đứng trước trong `records`) được giữ.
/// Khoảng trống không được bù: trung bình lấy thời gian có số liệu, `covered` cho biết đã có bao nhiêu phần của khoảng.
pub fn window_averages(records: &[Record], now: i64) -> Vec<WindowAverage> {
    let segs = segments(records);
    WINDOWS
        .iter()
        .map(|&(label, window)| {
            let s = span(&segs, now - window, now);
            WindowAverage { label, q_avg: s.q_avg, covered: s.covered, rain_mm: s.rain_mm, rain_covered: s.rain_covered }
        })
        .collect()
}

/// `days` ngày gần nhất tính cả hôm nay, cũ trước. Chu kỳ vắt qua nửa đêm được chia cho hai ngày.
pub fn daily_stats(records: &[Record], now: i64, days: usize) -> Vec<DayStat> {
    let segs = segments(records);
    let today = DateTime::from_timestamp(now + i64::from(VN_TZ), 0).map_or(NaiveDate::MIN, |t| t.date_naive());
    (0..days)
        .rev()
        .map(|back| {
            let date = today - chrono::Days::new(back as u64);
            let midnight = date.and_hms_opt(0, 0, 0).map_or(0, |t| t.and_utc().timestamp()) - i64::from(VN_TZ);
            let s = span(&segs, midnight, (midnight + 86_400).min(now));
            DayStat { date, q_avg: s.q_avg, rain_mm: s.rain_mm, covered: s.covered }
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
            rain_mm: None,
            operator: String::new(),
            note: AUTO_NOTE.into(),
        }
    }

    fn wet(ago_min: i64, minutes: i64, q: f64, rain_mm: f64) -> Record {
        Record { rain_mm: Some(rain_mm), ..cycle(ago_min, minutes, q) }
    }

    #[test]
    fn rain_accumulates_over_cycles_that_have_rain_data() {
        let v = window_averages(&[wet(30, 15, 10.0, 1.5), wet(15, 15, 20.0, 0.5), cycle(60, 15, 5.0)], NOW);
        let w = get(&v, "3 giờ");
        assert!((w.rain_mm.unwrap() - 2.0).abs() < 1e-9);
        assert!((w.rain_covered - 30.0 / 180.0).abs() < 1e-9);
        assert!((w.covered - 45.0 / 180.0).abs() < 1e-9, "chu kỳ không có mưa vẫn tính lưu lượng");
    }

    #[test]
    fn no_rain_data_means_unknown_not_zero() {
        let w = window_averages(&[cycle(15, 15, 10.0)], NOW);
        assert_eq!(get(&w, "3 giờ").rain_mm, None);
        assert_eq!(get(&w, "3 giờ").rain_covered, 0.0);
        let dry = window_averages(&[wet(15, 15, 10.0, 0.0)], NOW);
        assert_eq!(get(&dry, "3 giờ").rain_mm, Some(0.0), "đo được và không mưa là 0, khác với không có số liệu");
    }

    #[test]
    fn rain_of_a_clipped_cycle_is_cut_in_proportion() {
        // chu kỳ 15 phút kết thúc cách 175 phút: chỉ 5 phút nằm trong cửa sổ 3 giờ → 1/3 lượng mưa
        let v = window_averages(&[wet(175, 15, 10.0, 3.0)], NOW);
        assert!((get(&v, "3 giờ").rain_mm.unwrap() - 1.0).abs() < 1e-9);
        assert!((get(&v, "12 giờ").rain_mm.unwrap() - 3.0).abs() < 1e-9);
    }

    fn at(rfc3339: &str) -> i64 {
        DateTime::parse_from_rfc3339(rfc3339).unwrap().timestamp()
    }

    fn rec_at(end: &str, minutes: i64, q: f64, rain_mm: Option<f64>) -> Record {
        Record { created_at: end.into(), hours: minutes as f64 / 60.0, q_in: q, rain_mm, ..cycle(0, minutes, q) }
    }

    #[test]
    fn daily_stats_lists_days_oldest_first_ending_today() {
        let now = at("2026-10-28T11:15:00+07:00");
        let d = daily_stats(&[], now, 3);
        let dates: Vec<_> = d.iter().map(|x| x.date.to_string()).collect();
        assert_eq!(dates, ["2026-10-26", "2026-10-27", "2026-10-28"]);
        assert!(d.iter().all(|x| x.q_avg.is_none() && x.rain_mm.is_none() && x.covered == 0.0));
    }

    #[test]
    fn daily_stats_averages_and_sums_per_day() {
        let now = at("2026-10-28T11:15:00+07:00");
        let rows = [
            rec_at("2026-10-27T10:00:00+07:00", 60, 10.0, Some(2.0)),
            rec_at("2026-10-27T12:00:00+07:00", 60, 20.0, Some(3.0)),
            rec_at("2026-10-28T11:00:00+07:00", 15, 30.0, None),
        ];
        let d = daily_stats(&rows, now, 2);
        assert_eq!((d[0].q_avg, d[0].rain_mm), (Some(15.0), Some(5.0)));
        assert!((d[0].covered - 2.0 / 24.0).abs() < 1e-9);
        // hôm nay: 11h15 đã trôi qua, 15 phút có số liệu
        assert_eq!((d[1].q_avg, d[1].rain_mm), (Some(30.0), None));
        assert!((d[1].covered - 15.0 / 675.0).abs() < 1e-9);
    }

    #[test]
    fn a_cycle_crossing_midnight_is_split_between_the_two_days() {
        let now = at("2026-10-28T11:15:00+07:00");
        // 23:45 → 00:15, 30 phút, Q = 12, mưa 3 mm: mỗi ngày 15 phút, 1,5 mm
        let d = daily_stats(&[rec_at("2026-10-28T00:15:00+07:00", 30, 12.0, Some(3.0))], now, 2);
        assert_eq!((d[0].q_avg, d[1].q_avg), (Some(12.0), Some(12.0)));
        assert!((d[0].rain_mm.unwrap() - 1.5).abs() < 1e-9 && (d[1].rain_mm.unwrap() - 1.5).abs() < 1e-9);
        assert!((d[0].covered - 15.0 / 1440.0).abs() < 1e-9);
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
