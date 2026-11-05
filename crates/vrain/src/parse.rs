//! Phân tích phản hồi `organizations/details?i=_1h`.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, NaiveDate};
use serde_json::Value;

use crate::VrainError;

/// Mưa một trạm trong một giờ. `ts` = epoch giây của CUỐI giờ (mốc "10:00" là mưa 09:00–10:00).
#[derive(Debug, Clone, PartialEq)]
pub struct Obs {
    pub station_id: String,
    pub ts: i64,
    pub mm: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutcome {
    pub obs: Vec<Obs>,
    /// Id trạm cần lấy nhưng không có trong phản hồi.
    pub missing: Vec<String>,
}

const VN_OFFSET: i64 = 7 * 3600;

/// Ngày `YYYY-MM-DD` theo giờ Việt Nam.
pub fn vn_date(unix: i64) -> String {
    DateTime::from_timestamp(unix + VN_OFFSET, 0).map_or_else(String::new, |t| t.format("%Y-%m-%d").to_string())
}

/// Epoch của giờ Việt Nam y-m-d h:mi; `None` nếu ngày giờ không có thật.
fn vn_epoch(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> Option<i64> {
    Some(NaiveDate::from_ymd_opt(y, mo, d)?.and_hms_opt(h, mi, 0)?.and_utc().timestamp() - VN_OFFSET)
}

/// `"HH:mm dd/MM"` (không có năm) → epoch. Năm của `now`; nếu kết quả quá 1 giờ trong tương lai thì lùi 1 năm.
fn parse_time_point(tp: &str, now: i64) -> Option<i64> {
    let (clock, date) = tp.trim().split_once(' ')?;
    let (h, mi) = clock.split_once(':')?;
    let (d, mo) = date.trim().split_once('/')?;
    let (h, mi, d, mo): (u32, u32, u32, u32) = (h.parse().ok()?, mi.parse().ok()?, d.parse().ok()?, mo.parse().ok()?);
    let year = DateTime::from_timestamp(now + VN_OFFSET, 0)?.year();
    let ts = vn_epoch(year, mo, d, h, mi)?;
    if ts > now + 3600 {
        vn_epoch(year - 1, mo, d, h, mi)
    } else {
        Some(ts)
    }
}

pub fn parse_details(json: &str, ids: &[String], now: i64) -> Result<ParseOutcome, VrainError> {
    let v: Value = serde_json::from_str(json).map_err(|_| VrainError::Format)?;
    let stats = v.get("stats").and_then(Value::as_array).ok_or(VrainError::Format)?;
    let mut obs = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for point in stats {
        let Some(ts) = point.get("timePoint").and_then(Value::as_str).and_then(|t| parse_time_point(t, now)) else {
            continue; // "Tổng" hoặc nhãn lạ
        };
        let Some(stations) = point.get("stations").and_then(Value::as_array) else { continue };
        for s in stations {
            let Some(id) = s.get("id").and_then(Value::as_str) else { continue };
            let Some(wanted) = ids.iter().find(|w| w.as_str() == id) else { continue };
            seen.insert(wanted);
            let mm = s.get("depth").and_then(Value::as_str).and_then(|d| d.trim().parse::<f64>().ok());
            if let Some(mm) = mm.filter(|m| m.is_finite() && *m >= 0.0) {
                obs.push(Obs { station_id: id.to_string(), ts, mm });
            }
        }
    }
    let missing = ids.iter().filter(|i| !seen.contains(i.as_str())).cloned().collect();
    Ok(ParseOutcome { obs, missing })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use serde_json::{json, Value};

    const FIXTURE: &str = include_str!("../tests/data/vrain_details.json");
    const HN: &str = "841697470576";
    const AD: &str = "84396786852";
    const TL: &str = "84846302387";

    fn ids() -> Vec<String> {
        [HN, AD, TL].iter().map(|s| s.to_string()).collect()
    }

    /// Epoch của giờ Việt Nam y-m-d h:mi.
    fn vn(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        NaiveDate::from_ymd_opt(y, mo, d).unwrap().and_hms_opt(h, mi, 0).unwrap().and_utc().timestamp() - 7 * 3600
    }

    fn find<'a>(o: &'a ParseOutcome, id: &str, ts: i64) -> Option<&'a Obs> {
        o.obs.iter().find(|x| x.station_id == id && x.ts == ts)
    }

    #[test]
    fn parses_three_stations_and_skips_total() {
        let o = parse_details(FIXTURE, &ids(), vn(2026, 11, 5, 10, 5)).unwrap();
        assert_eq!(o.obs.len(), 6);
        assert!(o.missing.is_empty());
        assert!(o.obs.iter().all(|x| x.station_id != "84845529137"));
        assert_eq!(find(&o, HN, vn(2026, 11, 5, 10, 0)).unwrap().mm, 0.2);
        assert_eq!(find(&o, TL, vn(2026, 11, 5, 9, 0)).unwrap().mm, 0.4);
    }

    #[test]
    fn missing_station_listed() {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        for s in v["stats"].as_array_mut().unwrap() {
            s["stations"].as_array_mut().unwrap().retain(|x| x["id"] != TL);
        }
        let o = parse_details(&v.to_string(), &ids(), vn(2026, 11, 5, 10, 5)).unwrap();
        assert_eq!(o.missing, vec![TL.to_string()]);
        assert_eq!(o.obs.len(), 4);
    }

    #[test]
    fn bad_depth_skipped() {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let last = v["stats"].as_array_mut().unwrap().last_mut().unwrap();
        for s in last["stations"].as_array_mut().unwrap() {
            match s["id"].as_str().unwrap() {
                HN => s["depth"] = json!("abc"),
                AD => s["depth"] = json!(""),
                TL => s["depth"] = Value::Null,
                _ => {}
            }
        }
        let o = parse_details(&v.to_string(), &ids(), vn(2026, 11, 5, 10, 5)).unwrap();
        assert_eq!(o.obs.len(), 3);
        assert!(o.obs.iter().all(|x| x.ts == vn(2026, 11, 5, 9, 0)));
        assert!(o.missing.is_empty());
    }

    fn one_point(time_point: &str) -> String {
        json!({"stats":[{"timePoint": time_point, "stations":[{"id": HN, "depth": "1.5"}]}]}).to_string()
    }

    #[test]
    fn new_year_rollover() {
        let now = vn(2027, 1, 1, 0, 30);
        let o = parse_details(&one_point("23:00 31/12"), &[HN.to_string()], now).unwrap();
        assert_eq!(o.obs[0].ts, vn(2026, 12, 31, 23, 0));
        let o = parse_details(&one_point("00:00 01/01"), &[HN.to_string()], now).unwrap();
        assert_eq!(o.obs[0].ts, vn(2027, 1, 1, 0, 0));
    }

    #[test]
    fn future_stamp_goes_back_a_year() {
        let o = parse_details(&one_point("23:00 31/12"), &[HN.to_string()], vn(2026, 1, 1, 0, 30)).unwrap();
        assert_eq!(o.obs[0].ts, vn(2025, 12, 31, 23, 0));
    }

    #[test]
    fn wrong_structure_is_format_error() {
        assert_eq!(parse_details("{}", &ids(), 0), Err(VrainError::Format));
        assert_eq!(parse_details("không phải json", &ids(), 0), Err(VrainError::Format));
        assert_eq!(parse_details(r#"{"stats":"x"}"#, &ids(), 0), Err(VrainError::Format));
    }

    #[test]
    fn vn_date_uses_vietnam_time() {
        assert_eq!(vn_date(vn(2026, 11, 5, 0, 10)), "2026-11-05"); // 17:10 UTC hôm trước vẫn là ngày 05 giờ VN
        assert_eq!(vn_date(vn(2026, 11, 5, 23, 59)), "2026-11-05");
    }
}
