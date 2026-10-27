//! Định dạng hiển thị thời tiết (thuần, test được).

use chrono::{DateTime, Datelike, NaiveDate, Weekday};

use luuluong_weather::Day;

/// Số kiểu Việt giữ nguyên độ chính xác (17,3 chứ không phải 17,30; 160 chứ không phải 160,0).
pub fn vn(x: f64) -> String {
    x.to_string().replace('.', ",")
}

#[derive(Debug, PartialEq, Eq)]
pub enum RainAlert {
    None,
    /// Mô hình đang hiển thị báo mưa ≥ ngưỡng.
    Primary,
    /// Mô hình chính chưa tới ngưỡng nhưng có mô hình khác báo ≥ ngưỡng.
    OtherModels,
}

/// Các mô hình bất đồng nhiều về mưa, nên cảnh báo cả khi chỉ một mô hình báo mưa lớn.
pub fn rain_alert(day: &Day, threshold_mm: f64) -> RainAlert {
    if day.rain_mm >= threshold_mm {
        RainAlert::Primary
    } else if day.model_rain.iter().any(|m| m.rain_mm >= threshold_mm) {
        RainAlert::OtherModels
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

    fn day(rain: f64, models: &[f64]) -> Day {
        Day {
            date: "2026-10-27".parse().unwrap(),
            temp_min: 20.0,
            temp_max: 30.0,
            rain_mm: rain,
            pop_percent: 90.0,
            description: String::new(),
            icon: String::new(),
            partial: false,
            model_rain: models.iter().map(|m| luuluong_weather::ModelRain { model: "m".into(), rain_mm: *m }).collect(),
        }
    }

    #[test]
    fn rain_alert_levels() {
        assert_eq!(rain_alert(&day(49.9, &[10.0, 20.0]), 50.0), RainAlert::None);
        assert_eq!(rain_alert(&day(50.0, &[]), 50.0), RainAlert::Primary);
        assert_eq!(rain_alert(&day(40.9, &[6.0, 62.0]), 50.0), RainAlert::OtherModels);
        assert_eq!(rain_alert(&day(60.0, &[62.0]), 50.0), RainAlert::Primary);
    }

    #[test]
    fn rain_alert_threshold_is_configurable() {
        assert_eq!(rain_alert(&day(30.0, &[]), 25.0), RainAlert::Primary);
    }
}
