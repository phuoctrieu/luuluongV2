//! Định dạng hiển thị thời tiết (thuần, test được).

use chrono::{DateTime, Datelike, NaiveDate, Weekday};
use luuluong_core::validate::format_vn;

/// Số kiểu Việt giữ nguyên độ chính xác (17,3 chứ không phải 17,30; 160 chứ không phải 160,0).
pub fn vn(x: f64) -> String {
    x.to_string().replace('.', ",")
}

/// `2026-10-27T14:09:13+07:00` → `27/10/2026 14:09:13`; chuỗi không phải RFC 3339 giữ nguyên.
pub fn display_time(iso: &str) -> String {
    DateTime::parse_from_rfc3339(iso).map_or_else(|_| iso.to_owned(), |t| t.format("%d/%m/%Y %H:%M:%S").to_string())
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

/// Dạng bầu trời để chọn biểu tượng và hiệu ứng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    HeavyRain,
    Thunder,
}

impl Sky {
    /// Phân loại từ mô tả tiếng Việt của OpenWeather / mã WMO đã dịch.
    pub fn from_description(desc: &str) -> Sky {
        let d = desc.to_lowercase();
        let has = |words: &[&str]| words.iter().any(|w| d.contains(w));
        if has(&["dông", "giông", "sấm"]) {
            Sky::Thunder
        } else if has(&["sương"]) {
            Sky::Fog
        } else if has(&["mưa phùn", "mưa nhẹ"]) {
            Sky::Drizzle
        } else if d.contains("mưa") {
            if has(&["mưa to", "rất to", "mưa lớn"]) {
                Sky::HeavyRain
            } else {
                Sky::Rain
            }
        } else if has(&["nhiều mây", "u ám", "âm u", "mây đen", "mây cụm", "mây dày"]) {
            Sky::Cloudy
        } else if has(&["ít mây", "mây rải", "mây thưa"]) {
            Sky::PartlyCloudy
        } else if has(&["quang", "nắng", "trong xanh"]) {
            Sky::Clear
        } else {
            Sky::Cloudy
        }
    }

    pub fn is_rainy(self) -> bool {
        matches!(self, Sky::Drizzle | Sky::Rain | Sky::HeavyRain | Sky::Thunder)
    }
}

/// Trời tối (trước 6h hoặc từ 18h giờ địa phương) để vẽ trăng sao thay vì mặt trời.
pub fn is_night(unix: i64, tz_offset: i32) -> bool {
    let hour = (unix + tz_offset as i64).rem_euclid(86_400) / 3600;
    !(6..18).contains(&hour)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agreement {
    /// Chưa đủ nguồn để đối chiếu.
    Single,
    /// Các nguồn tương đối thống nhất.
    Agree,
    /// Các nguồn bất đồng lớn.
    Differ,
    /// Có nguồn báo mưa từ ngưỡng cảnh báo trở lên.
    Heavy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    pub level: Agreement,
    pub text: String,
}

/// Kết luận bằng lời cho một ngày. Bất đồng lớn khi chênh lệch từ nửa ngưỡng cảnh báo trở lên.
pub fn verdict(day: &str, rains: &[(&str, f64)], threshold_mm: f64) -> Verdict {
    let say = |level, text: String| Verdict { level, text };
    if rains.is_empty() {
        return say(Agreement::Single, format!("{day}: chưa có dữ liệu."));
    }
    if rains.len() == 1 {
        return say(Agreement::Single, format!("{day}: chỉ có 1 nguồn nên chưa đối chiếu được."));
    }
    let by_rain = |a: &&(&str, f64), b: &&(&str, f64)| a.1.total_cmp(&b.1);
    let (lo_name, lo) = *rains.iter().min_by(by_rain).expect("không rỗng");
    let (hi_name, hi) = *rains.iter().max_by(by_rain).expect("không rỗng");
    let big_spread = hi - lo >= threshold_mm / 2.0;
    let over: Vec<&str> = rains.iter().filter(|r| r.1 >= threshold_mm).map(|r| r.0).collect();
    let (lo_s, hi_s) = (format_vn(lo, 1), format_vn(hi, 1));
    if !over.is_empty() {
        let tail = if big_spread { ", chênh lệch lớn, nên theo dõi sát." } else { "." };
        let text = format!(
            "{day}: {} báo mưa lớn (từ {} mm). Các nguồn dự báo từ {lo_s} đến {hi_s} mm{tail}",
            over.join(", "),
            format_vn(threshold_mm, 0)
        );
        say(Agreement::Heavy, text)
    } else if big_spread {
        say(Agreement::Differ, format!("{day}: các nguồn bất đồng, dự báo từ {lo_s} mm ({lo_name}) đến {hi_s} mm ({hi_name}). Nên theo dõi thêm."))
    } else if hi < 1.0 {
        say(Agreement::Agree, format!("{day}: các nguồn đều báo gần như không mưa."))
    } else {
        say(Agreement::Agree, format!("{day}: các nguồn tương đối thống nhất, mưa khoảng {lo_s}–{hi_s} mm."))
    }
}

/// "vừa xong", "20 phút trước", "2 giờ trước" cho khung radar có thời điểm `frame_time`.
pub fn age_label(frame_time: i64, now: i64) -> String {
    let minutes = (now - frame_time).max(0) / 60;
    match minutes {
        0 => "vừa xong".into(),
        1..=89 => format!("{minutes} phút trước"),
        _ => format!("{} giờ trước", minutes / 60),
    }
}

/// Khung radar kế tiếp khi phát tự động: quay vòng về khung đầu sau khung cuối.
pub fn next_frame(index: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else {
        (index + 1) % len
    }
}

/// Một nguồn trên biểu đồ cột: giá trị mưa theo từng ngày (`None` nếu nguồn không có ngày đó).
#[derive(Debug, Clone, PartialEq)]
pub struct Series {
    pub label: String,
    pub values: Vec<Option<f64>>,
}

pub fn rain_series(dates: &[NaiveDate], sources: &[(&str, &[luuluong_weather::Day])]) -> Vec<Series> {
    sources
        .iter()
        .map(|(label, days)| Series {
            label: (*label).to_string(),
            values: dates.iter().map(|d| days.iter().find(|x| x.date == *d).map(|x| x.rain_mm)).collect(),
        })
        .collect()
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
    fn display_time_is_day_first_with_seconds() {
        assert_eq!(display_time("2026-10-27T14:09:13+07:00"), "27/10/2026 14:09:13");
    }

    #[test]
    fn display_time_keeps_unparseable_text() {
        assert_eq!(display_time("không rõ"), "không rõ");
    }

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

    #[test]
    fn sky_from_vietnamese_descriptions() {
        let cases = [
            ("Trời quang", Sky::Clear),
            ("bầu trời quang đãng", Sky::Clear),
            ("Ít mây", Sky::PartlyCloudy),
            ("Mây rải rác", Sky::PartlyCloudy),
            ("Nhiều mây", Sky::Cloudy),
            ("mây đen u ám", Sky::Cloudy),
            ("Sương mù", Sky::Fog),
            ("Mưa phùn nhẹ", Sky::Drizzle),
            ("Mưa phùn dày", Sky::Drizzle),
            ("Mưa nhẹ", Sky::Drizzle),
            ("Mưa vừa", Sky::Rain),
            ("Mưa rào nhẹ", Sky::Rain),
            ("Mưa rào vừa", Sky::Rain),
            ("Mưa to", Sky::HeavyRain),
            ("Mưa rào rất to", Sky::HeavyRain),
            ("Dông", Sky::Thunder),
            ("Dông kèm mưa đá", Sky::Thunder),
            ("giông bão", Sky::Thunder),
            ("không rõ", Sky::Cloudy),
        ];
        for (text, sky) in cases {
            assert_eq!(Sky::from_description(text), sky, "{text}");
        }
    }

    #[test]
    fn rainy_skies() {
        assert!(Sky::Rain.is_rainy() && Sky::Thunder.is_rainy() && Sky::Drizzle.is_rainy());
        assert!(!Sky::Clear.is_rainy() && !Sky::Fog.is_rainy() && !Sky::Cloudy.is_rainy());
    }

    #[test]
    fn night_follows_local_hour() {
        let at = |h: u32| "2026-10-27".parse::<NaiveDate>().unwrap().and_hms_opt(h, 0, 0).unwrap().and_utc().timestamp();
        // 22:00 UTC = 05:00 giờ VN (tối); 23:00 UTC hôm trước... dùng mốc rõ ràng theo UTC+7
        assert!(is_night(at(22), 7 * 3600)); // 05:00
        assert!(!is_night(at(23), 7 * 3600)); // 06:00
        assert!(!is_night(at(10), 7 * 3600)); // 17:00
        assert!(is_night(at(11), 7 * 3600)); // 18:00
        assert!(!is_night(at(5), 7 * 3600)); // 12:00
    }

    #[test]
    fn verdict_when_sources_agree() {
        let v = verdict("29/10", &[("NOAA GFS", 7.7), ("ECMWF", 10.1), ("DWD ICON", 3.5)], 50.0);
        assert_eq!(v.level, Agreement::Agree);
        assert_eq!(v.text, "29/10: các nguồn tương đối thống nhất, mưa khoảng 3,5–10,1 mm.");
    }

    #[test]
    fn verdict_when_nearly_no_rain() {
        let v = verdict("31/10", &[("A", 0.0), ("B", 0.4)], 50.0);
        assert_eq!(v.level, Agreement::Agree);
        assert_eq!(v.text, "31/10: các nguồn đều báo gần như không mưa.");
    }

    #[test]
    fn verdict_when_sources_disagree_names_the_extremes() {
        let v = verdict("28/10", &[("NOAA GFS", 6.0), ("ECMWF", 44.5), ("DWD ICON", 12.1)], 50.0);
        assert_eq!(v.level, Agreement::Differ);
        assert_eq!(v.text, "28/10: các nguồn bất đồng, dự báo từ 6,0 mm (NOAA GFS) đến 44,5 mm (ECMWF). Nên theo dõi thêm.");
    }

    #[test]
    fn verdict_heavy_rain_names_the_sources_over_the_threshold() {
        let v = verdict("28/10", &[("NOAA GFS", 6.0), ("ECMWF", 63.0), ("DWD ICON", 55.0)], 50.0);
        assert_eq!(v.level, Agreement::Heavy);
        assert_eq!(v.text, "28/10: ECMWF, DWD ICON báo mưa lớn (từ 50 mm). Các nguồn dự báo từ 6,0 đến 63,0 mm, chênh lệch lớn, nên theo dõi sát.");
    }

    #[test]
    fn verdict_heavy_without_big_spread_omits_the_warning_tail() {
        let v = verdict("28/10", &[("A", 60.0), ("B", 70.0)], 50.0);
        assert_eq!(v.level, Agreement::Heavy);
        assert_eq!(v.text, "28/10: A, B báo mưa lớn (từ 50 mm). Các nguồn dự báo từ 60,0 đến 70,0 mm.");
    }

    #[test]
    fn verdict_with_fewer_than_two_sources_cannot_compare() {
        assert_eq!(verdict("28/10", &[("A", 5.0)], 50.0).level, Agreement::Single);
        assert_eq!(verdict("28/10", &[("A", 5.0)], 50.0).text, "28/10: chỉ có 1 nguồn nên chưa đối chiếu được.");
        assert_eq!(verdict("28/10", &[], 50.0).text, "28/10: chưa có dữ liệu.");
    }

    fn day_on(date: &str, rain: f64) -> luuluong_weather::Day {
        luuluong_weather::Day {
            date: date.parse().unwrap(),
            temp_min: 20.0,
            temp_max: 30.0,
            rain_mm: rain,
            pop_percent: 50.0,
            description: String::new(),
            icon: String::new(),
            partial: false,
        }
    }

    #[test]
    fn rain_series_aligns_sources_to_the_given_dates() {
        let dates: Vec<NaiveDate> = ["2026-10-27", "2026-10-28", "2026-10-29"].iter().map(|d| d.parse().unwrap()).collect();
        let a = vec![day_on("2026-10-27", 1.0), day_on("2026-10-28", 2.0), day_on("2026-10-29", 3.0)];
        let b = vec![day_on("2026-10-28", 9.0)];
        let out = rain_series(&dates, &[("A", &a), ("B", &b)]);
        assert_eq!(out[0], Series { label: "A".into(), values: vec![Some(1.0), Some(2.0), Some(3.0)] });
        assert_eq!(out[1], Series { label: "B".into(), values: vec![None, Some(9.0), None] });
    }

    #[test]
    fn radar_age_labels() {
        let now = 1_000_000;
        assert_eq!(age_label(now, now), "vừa xong");
        assert_eq!(age_label(now - 59, now), "vừa xong");
        assert_eq!(age_label(now - 600, now), "10 phút trước");
        assert_eq!(age_label(now - 3600, now), "60 phút trước");
        assert_eq!(age_label(now - 90 * 60, now), "1 giờ trước");
        assert_eq!(age_label(now - 3 * 3600, now), "3 giờ trước");
        assert_eq!(age_label(now + 500, now), "vừa xong"); // đồng hồ lệch
    }

    #[test]
    fn playback_loops_and_handles_empty() {
        assert_eq!(next_frame(0, 6), 1);
        assert_eq!(next_frame(5, 6), 0);
        assert_eq!(next_frame(0, 1), 0);
        assert_eq!(next_frame(0, 0), 0);
    }
}
