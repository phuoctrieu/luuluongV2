//! Mưa quan trắc Vrain: tổng hợp, mức mưa, nhận xét so với trạm nhà máy, cổng nút "Cập nhật ngay".
//! Thuần logic (thời gian truyền vào), không phụ thuộc giao diện.

use chrono::{DateTime, Days, NaiveDate};
use luuluong_storage::history::RainObs;

use crate::live::VN_TZ;

/// Mốc mới nhất (epoch giây) của một trạm.
pub fn latest_ts(obs: &[RainObs], station_id: &str) -> Option<i64> {
    obs.iter().filter(|o| o.station_id == station_id).map(|o| o.ts).max()
}

/// Tổng mưa `hours` giờ tính đến MỐC MỚI NHẤT của trạm (không phải đến `now`: mốc giờ tới trễ vài phút sau giờ tròn).
/// `None` nếu trạm chưa có mốc nào hoặc thiếu mốc trong khoảng (không nội suy).
pub fn rain_sum(obs: &[RainObs], station_id: &str, hours: u32) -> Option<f64> {
    if hours == 0 {
        return None;
    }
    let last = latest_ts(obs, station_id)?;
    let from = last - i64::from(hours) * 3600;
    let mut points: Vec<(i64, f64)> =
        obs.iter().filter(|o| o.station_id == station_id && o.ts > from && o.ts <= last).map(|o| (o.ts, o.mm)).collect();
    points.sort_by_key(|p| p.0);
    points.dedup_by_key(|p| p.0);
    (points.len() >= hours as usize).then(|| points.iter().map(|p| p.1).sum())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RainLevel {
    None,
    Light,
    Moderate,
    Heavy,
}

/// Mức mưa theo mm trong một giờ: 0 không mưa; ≤ 2,5 nhỏ; ≤ 7,6 vừa; còn lại to.
pub fn level(mm_per_hour: f64) -> RainLevel {
    if mm_per_hour <= 0.0 {
        RainLevel::None
    } else if mm_per_hour <= 2.5 {
        RainLevel::Light
    } else if mm_per_hour <= 7.6 {
        RainLevel::Moderate
    } else {
        RainLevel::Heavy
    }
}

pub fn level_label(l: RainLevel) -> &'static str {
    match l {
        RainLevel::None => "Không mưa",
        RainLevel::Light => "Mưa nhỏ",
        RainLevel::Moderate => "Mưa vừa",
        RainLevel::Heavy => "Mưa to",
    }
}

/// Quá 2 giờ kể từ lần lấy thành công (hoặc chưa từng lấy được) là dữ liệu cũ.
pub fn is_stale(last_ok: Option<i64>, now: i64) -> bool {
    last_ok.is_none_or(|t| now - t > 2 * 3600)
}

/// Mưa từng giờ của một trạm trong `hours` giờ gần nhất tính đến `now`, cũ trước (cho biểu đồ).
pub fn hourly_series(obs: &[RainObs], station_id: &str, now: i64, hours: u32) -> Vec<(i64, f64)> {
    let from = now - i64::from(hours) * 3600;
    let mut v: Vec<(i64, f64)> =
        obs.iter().filter(|o| o.station_id == station_id && o.ts > from && o.ts <= now).map(|o| (o.ts, o.mm)).collect();
    v.sort_by_key(|p| p.0);
    v
}

/// So mưa trung bình 3 trạm Vrain với trạm nhà máy (cùng một khoảng, mm). Chênh ≥ nửa ngưỡng cảnh báo thì nhắc;
/// thiếu một trong hai vế thì không nhận xét.
pub fn compare_note(vrain_mean_mm: Option<f64>, ecapro_mm: Option<f64>, warn_mm: f64) -> Option<String> {
    let (v, e) = (vrain_mean_mm?, ecapro_mm?);
    if (v - e).abs() < warn_mm / 2.0 {
        return None;
    }
    let vn = |x: f64| format!("{x:.1}").replace('.', ",");
    Some(format!(
        "Trạm nhà máy lệch so với trung bình các trạm Vrain ({} mm so với {} mm). Kiểm tra cảm biến mưa hoặc vị trí mưa.",
        vn(e),
        vn(v)
    ))
}

/// Mưa một ngày (giờ Việt Nam) của một trạm.
#[derive(Debug, Clone, PartialEq)]
pub struct DayRain {
    pub date: NaiveDate,
    /// Tổng các mốc giờ đã có; `None` nếu ngày chưa có mốc nào.
    pub mm: Option<f64>,
    /// Số mốc giờ đã có trong ngày (đủ ngày = 24).
    pub hours: u32,
}

/// `days` ngày gần nhất tính cả hôm nay, cũ trước. Mốc "00:00" thuộc về ngày trước nó (mưa 23:00–24:00).
pub fn daily_rain(obs: &[RainObs], station_id: &str, now: i64, days: usize) -> Vec<DayRain> {
    let day_of = |ts: i64| DateTime::from_timestamp(ts + i64::from(VN_TZ), 0).map_or(NaiveDate::MIN, |t| t.date_naive());
    let today = day_of(now);
    (0..days)
        .rev()
        .map(|back| {
            let date = today - Days::new(back as u64);
            let mut hours: Vec<(i64, f64)> =
                obs.iter().filter(|o| o.station_id == station_id && day_of(o.ts - 1) == date).map(|o| (o.ts, o.mm)).collect();
            hours.sort_by_key(|h| h.0);
            hours.dedup_by_key(|h| h.0);
            DayRain { date, mm: (!hours.is_empty()).then(|| hours.iter().map(|h| h.1).sum()), hours: hours.len() as u32 }
        })
        .collect()
}

/// Mưa của một trạm trong khoảng `(from, to]` (epoch giây), cho báo cáo.
#[derive(Debug, Clone, PartialEq)]
pub struct RainSpan {
    /// Tổng các mốc giờ đã có; `None` nếu không có mốc nào.
    pub mm: Option<f64>,
    /// Số mốc giờ đã có (mốc trùng đếm một lần).
    pub hours: u32,
    /// Số mốc giờ lẽ ra có = `(to − from) / 3600`, làm tròn xuống.
    pub expected: u32,
}

pub fn rain_between(obs: &[RainObs], station_id: &str, from: i64, to: i64) -> RainSpan {
    let mut marks: Vec<(i64, f64)> =
        obs.iter().filter(|o| o.station_id == station_id && o.ts > from && o.ts <= to).map(|o| (o.ts, o.mm)).collect();
    marks.sort_by_key(|m| m.0);
    marks.dedup_by_key(|m| m.0);
    RainSpan {
        mm: (!marks.is_empty()).then(|| marks.iter().map(|m| m.1).sum()),
        hours: marks.len() as u32,
        expected: ((to - from) / 3600).max(0) as u32,
    }
}

/// Cảnh báo mưa lớn Vrain cho thanh đầu.
#[derive(Debug, Clone, PartialEq)]
pub struct HeavyAlert {
    pub text: String,
    pub station_id: String,
}

/// `stations` = (id, tên). Chỉ xét trạm có mốc mới nhất cách `now` không quá 3 giờ. Ưu tiên mưa 24 giờ ≥ `warn_mm`
/// (trạm cao nhất), nếu không thì mưa 1 giờ > 7,6 mm (trạm cao nhất).
pub fn heavy_alert(obs: &[RainObs], stations: &[(String, String)], now: i64, warn_mm: f64) -> Option<HeavyAlert> {
    const FRESH_SECS: i64 = 3 * 3600;
    const HEAVY_MM_PER_HOUR: f64 = 7.6;
    let vn = |x: f64| format!("{x:.1}").replace('.', ",");
    let fresh: Vec<&(String, String)> =
        stations.iter().filter(|(id, _)| latest_ts(obs, id).is_some_and(|t| now - t <= FRESH_SECS)).collect();
    let highest = |hours: u32, keep: &dyn Fn(f64) -> bool| {
        fresh
            .iter()
            .filter_map(|(id, name)| rain_sum(obs, id, hours).filter(|mm| keep(*mm)).map(|mm| (mm, id, name)))
            .max_by(|a, b| a.0.total_cmp(&b.0))
    };
    if let Some((mm, id, name)) = highest(24, &|mm| mm >= warn_mm) {
        return Some(HeavyAlert { text: format!("Mưa lớn {name}: {} mm / 24 giờ (Vrain)", vn(mm)), station_id: id.clone() });
    }
    highest(1, &|mm| mm > HEAVY_MM_PER_HOUR)
        .map(|(mm, id, name)| HeavyAlert { text: format!("Mưa to {name}: {} mm / giờ (Vrain)", vn(mm)), station_id: id.clone() })
}

/// Cách nhau tối thiểu giữa hai lần bấm "Cập nhật ngay".
pub const MANUAL_MIN_GAP_SECS: i64 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshState {
    Idle,
    Loading,
    /// Còn bao nhiêu giây mới được bấm lại.
    Wait(u32),
    /// Chưa nhập tài khoản Vrain.
    NeedsSetup,
}

#[derive(Debug, Default)]
pub struct RefreshGate {
    last_request: Option<i64>,
    busy: bool,
}

impl RefreshGate {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self, now: i64, configured: bool) -> RefreshState {
        if !configured {
            return RefreshState::NeedsSetup;
        }
        if self.busy {
            return RefreshState::Loading;
        }
        match self.last_request {
            Some(t) if now - t < MANUAL_MIN_GAP_SECS => RefreshState::Wait((MANUAL_MIN_GAP_SECS - (now - t)) as u32),
            _ => RefreshState::Idle,
        }
    }

    /// `true` = được phép gửi tín hiệu lấy dữ liệu ngay; ghi nhận đang bận.
    pub fn try_start(&mut self, now: i64, configured: bool) -> bool {
        if self.state(now, configured) != RefreshState::Idle {
            return false;
        }
        self.busy = true;
        self.last_request = Some(now);
        true
    }

    pub fn finish(&mut self) {
        self.busy = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const H: i64 = 3600;
    const BASE: i64 = 1_790_000_000 - 1_790_000_000 % 3600;

    fn ro(id: &str, hour: i64, mm: f64) -> RainObs {
        RainObs { station_id: id.into(), ts: BASE + hour * H, mm }
    }

    /// Trạm "a": mốc giờ 1..=24, mỗi mốc 1,0 mm, riêng mốc cuối 2,0 mm.
    fn day() -> Vec<RainObs> {
        (1..=24).map(|h| ro("a", h, if h == 24 { 2.0 } else { 1.0 })).collect()
    }

    #[test]
    fn sum_1h_3h_24h_exact() {
        let o = day();
        assert_eq!(rain_sum(&o, "a", 1), Some(2.0));
        assert_eq!(rain_sum(&o, "a", 3), Some(4.0));
        assert_eq!(rain_sum(&o, "a", 24), Some(25.0));
        assert_eq!(rain_sum(&o, "a", 25), None); // chưa đủ 25 giờ
    }

    #[test]
    fn sum_anchors_to_latest_obs_even_when_old() {
        let o = vec![ro("a", 1, 0.5), ro("a", 2, 0.7)];
        assert_eq!(rain_sum(&o, "a", 1), Some(0.7));
        assert_eq!(latest_ts(&o, "a"), Some(BASE + 2 * H));
        assert_eq!(latest_ts(&o, "zz"), None);
    }

    #[test]
    fn sum_none_when_an_hour_is_missing() {
        let o = vec![ro("a", 1, 1.0), ro("a", 2, 1.0), ro("a", 4, 1.0)]; // thiếu giờ 3
        assert_eq!(rain_sum(&o, "a", 1), Some(1.0));
        assert_eq!(rain_sum(&o, "a", 3), None);
        assert_eq!(rain_sum(&o, "a", 4), None); // giờ 3 và giờ 0 đều thiếu
    }

    #[test]
    fn sum_other_station_ignored() {
        let mut o = day();
        o.push(ro("b", 30, 99.0));
        assert_eq!(rain_sum(&o, "a", 1), Some(2.0));
        assert_eq!(rain_sum(&o, "b", 1), Some(99.0));
    }

    fn vn_epoch(y: i32, mo: u32, d: u32, h: u32) -> i64 {
        NaiveDate::from_ymd_opt(y, mo, d).unwrap().and_hms_opt(h, 0, 0).unwrap().and_utc().timestamp() - i64::from(VN_TZ)
    }

    #[test]
    fn daily_rain_assigns_midnight_mark_to_the_previous_day() {
        let o = vec![
            RainObs { station_id: "a".into(), ts: vn_epoch(2026, 11, 4, 1), mm: 1.0 },
            RainObs { station_id: "a".into(), ts: vn_epoch(2026, 11, 5, 0), mm: 3.0 }, // 23:00–24:00 ngày 04
            RainObs { station_id: "a".into(), ts: vn_epoch(2026, 11, 5, 1), mm: 0.5 }, // ngày 05
            RainObs { station_id: "b".into(), ts: vn_epoch(2026, 11, 4, 2), mm: 99.0 },
        ];
        let d = daily_rain(&o, "a", vn_epoch(2026, 11, 5, 10), 3);
        assert_eq!(d.iter().map(|x| x.date.to_string()).collect::<Vec<_>>(), ["2026-11-03", "2026-11-04", "2026-11-05"]);
        assert_eq!(d[0], DayRain { date: d[0].date, mm: None, hours: 0 });
        assert_eq!((d[1].mm, d[1].hours), (Some(4.0), 2));
        assert_eq!((d[2].mm, d[2].hours), (Some(0.5), 1));
    }

    #[test]
    fn daily_rain_counts_a_duplicate_hour_once() {
        let ts = vn_epoch(2026, 11, 5, 5);
        let o = vec![RainObs { station_id: "a".into(), ts, mm: 1.0 }, RainObs { station_id: "a".into(), ts, mm: 1.0 }];
        let d = daily_rain(&o, "a", vn_epoch(2026, 11, 5, 10), 1);
        assert_eq!(d[0].hours, 1);
    }

    fn marks(id: &str, hours: std::ops::RangeInclusive<i64>, mm: impl Fn(i64) -> f64) -> Vec<RainObs> {
        hours.map(|h| ro(id, h, mm(h))).collect()
    }

    fn stations() -> Vec<(String, String)> {
        [("hn", "Hương Nguyên"), ("ad", "A Đớt"), ("tl", "Tà Lương")].iter().map(|(i, n)| (i.to_string(), n.to_string())).collect()
    }

    #[test]
    fn rain_between_sums_marks_in_half_open_range() {
        let o: Vec<RainObs> = (1..=5).map(|h| ro("a", h, 1.0)).collect();
        let r = rain_between(&o, "a", BASE + H, BASE + 4 * H); // (1h, 4h] → mốc 2, 3, 4
        assert_eq!((r.mm, r.hours, r.expected), (Some(3.0), 3, 3));
    }

    #[test]
    fn rain_between_counts_hours_and_expected() {
        let o: Vec<RainObs> = (1..=10).map(|h| ro("a", h, 0.5)).collect();
        let r = rain_between(&o, "a", BASE, BASE + 24 * H);
        assert_eq!((r.mm, r.hours, r.expected), (Some(5.0), 10, 24));
    }

    #[test]
    fn rain_between_none_without_marks_and_ignores_other_stations_and_duplicates() {
        let r = rain_between(&[ro("b", 1, 9.0)], "a", BASE, BASE + 2 * H);
        assert_eq!((r.mm, r.hours, r.expected), (None, 0, 2));
        let dup = rain_between(&[ro("a", 1, 1.0), ro("a", 1, 1.0)], "a", BASE, BASE + 2 * H);
        assert_eq!((dup.mm, dup.hours), (Some(1.0), 1));
    }

    #[test]
    fn heavy_alert_by_24h_picks_highest_station() {
        let mut o = marks("hn", 7..=30, |_| 0.5);
        o.extend(marks("ad", 7..=30, |h| if h == 30 { 3.6 } else { 2.6 })); // 63,4 mm
        o.extend(marks("tl", 7..=30, |h| if h == 30 { 9.0 } else { 2.0 })); // 55 mm
        let a = heavy_alert(&o, &stations(), BASE + 30 * H + 600, 50.0).unwrap();
        assert_eq!(a.text, "Mưa lớn A Đớt: 63,4 mm / 24 giờ (Vrain)");
        assert_eq!(a.station_id, "ad");
    }

    #[test]
    fn heavy_alert_by_hour_when_24h_below_threshold() {
        let o = marks("hn", 7..=30, |h| if h == 30 { 9.2 } else { 0.4 });
        let a = heavy_alert(&o, &stations(), BASE + 30 * H, 50.0).unwrap();
        assert_eq!(a.text, "Mưa to Hương Nguyên: 9,2 mm / giờ (Vrain)");
    }

    #[test]
    fn heavy_alert_prefers_24h_over_hour() {
        let mut o = marks("hn", 7..=30, |h| if h == 30 { 9.2 } else { 0.0 });
        o.extend(marks("ad", 7..=30, |_| 2.5)); // 60 mm
        let a = heavy_alert(&o, &stations(), BASE + 30 * H, 50.0).unwrap();
        assert!(a.text.starts_with("Mưa lớn A Đớt"), "{}", a.text);
    }

    #[test]
    fn heavy_alert_exactly_at_threshold_fires_and_below_does_not() {
        let o = marks("hn", 7..=30, |_| 2.0); // đúng 48 mm
        assert!(heavy_alert(&o, &stations(), BASE + 30 * H, 48.0).is_some());
        assert!(heavy_alert(&o, &stations(), BASE + 30 * H, 48.1).is_none());
    }

    #[test]
    fn heavy_alert_ignores_stale_stations() {
        let o = marks("hn", 7..=30, |_| 5.0);
        assert!(heavy_alert(&o, &stations(), BASE + 33 * H, 50.0).is_some(), "đúng 3 giờ vẫn xét");
        assert!(heavy_alert(&o, &stations(), BASE + 33 * H + 1, 50.0).is_none(), "quá 3 giờ là mưa cũ");
    }

    #[test]
    fn heavy_alert_none_without_data_or_stations() {
        assert!(heavy_alert(&[], &stations(), BASE, 50.0).is_none());
        assert!(heavy_alert(&marks("hn", 7..=30, |_| 9.0), &[], BASE + 30 * H, 50.0).is_none());
    }


    #[test]
    fn level_thresholds() {
        assert_eq!(level(0.0), RainLevel::None);
        assert_eq!(level(0.1), RainLevel::Light);
        assert_eq!(level(2.5), RainLevel::Light);
        assert_eq!(level(2.6), RainLevel::Moderate);
        assert_eq!(level(7.6), RainLevel::Moderate);
        assert_eq!(level(7.7), RainLevel::Heavy);
        assert_eq!(level_label(RainLevel::None), "Không mưa");
        assert_eq!(level_label(RainLevel::Light), "Mưa nhỏ");
        assert_eq!(level_label(RainLevel::Moderate), "Mưa vừa");
        assert_eq!(level_label(RainLevel::Heavy), "Mưa to");
    }

    #[test]
    fn stale_after_two_hours_or_never() {
        let now = BASE + 10 * H;
        assert!(is_stale(None, now));
        assert!(!is_stale(Some(now - 2 * H), now));
        assert!(is_stale(Some(now - 2 * H - 1), now));
    }

    #[test]
    fn series_orders_and_limits() {
        let o: Vec<RainObs> = (1..=10).rev().map(|h| ro("a", h, h as f64)).collect();
        let now = BASE + 10 * H + 600;
        let s = hourly_series(&o, "a", now, 4);
        assert_eq!(s.iter().map(|x| x.1).collect::<Vec<_>>(), vec![7.0, 8.0, 9.0, 10.0]);
        assert!(s.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn compare_note_threshold() {
        // ngưỡng cảnh báo 50 mm → nhắc khi chênh ≥ 25 mm
        assert!(compare_note(Some(10.0), Some(40.0), 50.0).unwrap().contains("lệch"));
        assert!(compare_note(Some(40.0), Some(10.0), 50.0).is_some());
        assert_eq!(compare_note(Some(10.0), Some(34.9), 50.0), None);
        assert_eq!(compare_note(None, Some(10.0), 50.0), None);
        assert_eq!(compare_note(Some(10.0), None, 50.0), None);
    }

    #[test]
    fn gate_blocks_second_click_within_30s_then_allows() {
        let mut g = RefreshGate::new();
        assert!(g.try_start(100, true));
        g.finish();
        assert!(!g.try_start(110, true));
        assert_eq!(g.state(110, true), RefreshState::Wait(20));
        assert!(g.try_start(130, true));
    }

    #[test]
    fn gate_wait_counts_down() {
        let mut g = RefreshGate::new();
        assert!(g.try_start(0, true));
        g.finish();
        assert_eq!(g.state(1, true), RefreshState::Wait(29));
        assert_eq!(g.state(29, true), RefreshState::Wait(1));
        assert_eq!(g.state(30, true), RefreshState::Idle);
    }

    #[test]
    fn gate_busy_blocks_until_finish() {
        let mut g = RefreshGate::new();
        assert_eq!(g.state(0, true), RefreshState::Idle);
        assert!(g.try_start(0, true));
        assert_eq!(g.state(100, true), RefreshState::Loading);
        assert!(!g.try_start(100, true));
        g.finish();
        assert_eq!(g.state(100, true), RefreshState::Idle);
    }

    #[test]
    fn gate_needs_setup_when_not_configured() {
        let mut g = RefreshGate::new();
        assert_eq!(g.state(0, false), RefreshState::NeedsSetup);
        assert!(!g.try_start(0, false));
        assert_eq!(g.state(0, true), RefreshState::Idle); // không bị ghi nhận lần bấm
    }
}
