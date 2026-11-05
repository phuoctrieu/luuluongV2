//! Mưa quan trắc Vrain: tổng hợp, mức mưa, nhận xét so với trạm nhà máy, cổng nút "Cập nhật ngay".
//! Thuần logic (thời gian truyền vào), không phụ thuộc giao diện.

use luuluong_storage::history::RainObs;

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
