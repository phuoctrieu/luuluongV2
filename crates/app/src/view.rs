//! Định dạng hiển thị thời tiết (thuần, test được).

use chrono::{DateTime, Datelike, NaiveDate, Weekday};

/// Số kiểu Việt giữ nguyên độ chính xác (17,3 chứ không phải 17,30; 160 chứ không phải 160,0).
pub fn vn(x: f64) -> String {
    x.to_string().replace('.', ",")
}

#[derive(Debug, PartialEq, Eq)]
pub enum RainAlert {
    None,
    /// Nguồn chính báo mưa ≥ ngưỡng.
    Primary,
    /// Nguồn chính chưa tới ngưỡng nhưng có nguồn khác báo ≥ ngưỡng.
    OtherSources,
}

/// Các nguồn bất đồng nhiều về mưa, nên cảnh báo cả khi chỉ một nguồn báo mưa lớn.
pub fn rain_alert(primary_mm: f64, others_mm: &[f64], threshold_mm: f64) -> RainAlert {
    if primary_mm >= threshold_mm {
        RainAlert::Primary
    } else if others_mm.iter().any(|m| *m >= threshold_mm) {
        RainAlert::OtherSources
    } else {
        RainAlert::None
    }
}

pub fn weekday_vn(date: NaiveDate) -> &'static str {
    match date.weekday() {
        Weekday::Mon => "T2",
        Weekday::Tue => "T3",
        Weekday::Wed => "T4",
        Weekday::Thu => "T5",
        Weekday::Fri => "T6",
        Weekday::Sat => "T7",
        Weekday::Sun => "CN",
    }
}

/// `HH:MM` theo giờ địa phương (`tz_offset` giây).
pub fn clock(unix: i64, tz_offset: i32) -> String {
    DateTime::from_timestamp(unix + tz_offset as i64, 0).map_or_else(|| "--:--".into(), |t| t.format("%H:%M").to_string())
}

/// "Cập nhật lúc 14:05" hoặc "Cập nhật lúc 14:05 – dữ liệu cũ".
pub fn updated_label(fetched_at: i64, tz_offset: i32, stale: bool) -> String {
    let base = format!("Cập nhật lúc {}", clock(fetched_at, tz_offset));
    if stale {
        format!("{base} – dữ liệu cũ")
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weekdays_in_vietnamese() {
        let d = |s: &str| s.parse::<NaiveDate>().unwrap();
        assert_eq!(weekday_vn(d("2026-10-26")), "T2");
        assert_eq!(weekday_vn(d("2026-10-27")), "T3");
        assert_eq!(weekday_vn(d("2026-10-31")), "T7");
        assert_eq!(weekday_vn(d("2026-11-01")), "CN");
    }

    #[test]
    fn clock_uses_local_offset() {
        // 2026-10-27 03:05 UTC = 10:05 giờ Việt Nam
        let t = "2026-10-27".parse::<NaiveDate>().unwrap().and_hms_opt(3, 5, 0).unwrap().and_utc().timestamp();
        assert_eq!(clock(t, 7 * 3600), "10:05");
        assert_eq!(clock(t, 0), "03:05");
    }

    #[test]
    fn stale_label_says_old_data() {
        let t = "2026-10-27".parse::<NaiveDate>().unwrap().and_hms_opt(3, 5, 0).unwrap().and_utc().timestamp();
        assert_eq!(updated_label(t, 7 * 3600, false), "Cập nhật lúc 10:05");
        assert_eq!(updated_label(t, 7 * 3600, true), "Cập nhật lúc 10:05 – dữ liệu cũ");
    }

    #[test]
    fn rain_alert_levels() {
        assert_eq!(rain_alert(49.9, &[10.0, 20.0], 50.0), RainAlert::None);
        assert_eq!(rain_alert(50.0, &[], 50.0), RainAlert::Primary);
        assert_eq!(rain_alert(40.9, &[6.0, 62.0], 50.0), RainAlert::OtherSources);
        assert_eq!(rain_alert(60.0, &[62.0], 50.0), RainAlert::Primary);
    }

    #[test]
    fn rain_alert_threshold_is_configurable() {
        assert_eq!(rain_alert(30.0, &[], 25.0), RainAlert::Primary);
    }
}
