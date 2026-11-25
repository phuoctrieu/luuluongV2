//! Phân tích phản hồi `d/kttv/private/v1/organizations/details?i=_1h` (dạng cột: `ids` + `rows[{ts, v}]`).

use std::collections::HashSet;

use chrono::{DateTime, NaiveDateTime};
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

/// `"YYYY-MM-DDTHH:mm:ss"` (giờ Việt Nam) → epoch; chỉ nhận mốc tròn giờ (mốc giữa giờ là giờ đang chạy, chưa đủ).
fn parse_hour_stamp(ts: &str) -> Option<i64> {
    let t = NaiveDateTime::parse_from_str(ts.trim(), "%Y-%m-%dT%H:%M:%S").ok()?;
    let epoch = t.and_utc().timestamp();
    (epoch % 3600 == 0).then_some(epoch - VN_OFFSET)
}

/// Con trỏ trang kế tiếp (`next`), nếu còn.
pub fn next_cursor(json: &str) -> Option<String> {
    let v: Value = serde_json::from_str(json).ok()?;
    v.get("next").and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

pub fn parse_details(json: &str, ids: &[String]) -> Result<ParseOutcome, VrainError> {
    let v: Value = serde_json::from_str(json).map_err(|_| VrainError::Format)?;
    let col_ids = v.get("ids").and_then(Value::as_array).ok_or(VrainError::Format)?;
    let rows = v.get("rows").and_then(Value::as_array).ok_or(VrainError::Format)?;
    // Cột của từng trạm cần lấy.
    let cols: Vec<(usize, &String)> = ids
        .iter()
        .filter_map(|w| col_ids.iter().position(|c| c.as_str() == Some(w.as_str())).map(|i| (i, w)))
        .collect();
    let mut obs = Vec::new();
    for row in rows {
        let Some(ts) = row.get("ts").and_then(Value::as_str).and_then(parse_hour_stamp) else { continue };
        let Some(values) = row.get("v").and_then(Value::as_array) else { continue };
        for (i, id) in &cols {
            let mm = values.get(*i).and_then(Value::as_f64);
            if let Some(mm) = mm.filter(|m| m.is_finite() && *m >= 0.0) {
                obs.push(Obs { station_id: (*id).clone(), ts, mm });
            }
        }
    }
    let seen: HashSet<&str> = cols.iter().map(|(_, id)| id.as_str()).collect();
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
    fn parses_three_stations_and_skips_partial_hour() {
        let o = parse_details(FIXTURE, &ids()).unwrap();
        assert_eq!(o.obs.len(), 6); // 3 trạm × 2 mốc tròn giờ; mốc 10:50 bị bỏ
        assert!(o.missing.is_empty());
        assert!(o.obs.iter().all(|x| x.station_id != "84845529137"));
        assert!(o.obs.iter().all(|x| x.ts % 3600 == 0));
        assert_eq!(find(&o, HN, vn(2026, 11, 25, 10, 0)).unwrap().mm, 1.0);
        assert_eq!(find(&o, TL, vn(2026, 11, 25, 9, 0)).unwrap().mm, 0.6);
    }

    #[test]
    fn missing_station_listed() {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        let pos = v["ids"].as_array().unwrap().iter().position(|x| x == TL).unwrap();
        v["ids"][pos] = json!("khac");
        let o = parse_details(&v.to_string(), &ids()).unwrap();
        assert_eq!(o.missing, vec![TL.to_string()]);
        assert_eq!(o.obs.len(), 4);
    }

    #[test]
    fn bad_value_skipped() {
        let mut v: Value = serde_json::from_str(FIXTURE).unwrap();
        v["rows"][1]["v"][0] = json!("abc");
        v["rows"][1]["v"][1] = Value::Null;
        v["rows"][1]["v"][2] = json!(-1.0);
        let o = parse_details(&v.to_string(), &ids()).unwrap();
        assert_eq!(o.obs.len(), 3);
        assert!(o.obs.iter().all(|x| x.ts == vn(2026, 11, 25, 9, 0)));
        assert!(o.missing.is_empty());
    }

    #[test]
    fn year_comes_from_the_stamp() {
        let j = json!({"ids":[HN],"rows":[{"ts":"2026-12-31T23:00:00","v":[1.5]},{"ts":"2027-01-01T00:00:00","v":[2.5]}]}).to_string();
        let o = parse_details(&j, &[HN.to_string()]).unwrap();
        assert_eq!(o.obs[0].ts, vn(2026, 12, 31, 23, 0));
        assert_eq!(o.obs[1].ts, vn(2027, 1, 1, 0, 0));
    }

    #[test]
    fn wrong_structure_is_format_error() {
        assert_eq!(parse_details("{}", &ids()), Err(VrainError::Format));
        assert_eq!(parse_details("không phải json", &ids()), Err(VrainError::Format));
        assert_eq!(parse_details(r#"{"ids":"x","rows":[]}"#, &ids()), Err(VrainError::Format));
    }

    #[test]
    fn next_cursor_read() {
        assert_eq!(next_cursor(FIXTURE), None);
        assert_eq!(next_cursor(r#"{"next":"abc"}"#), Some("abc".to_string()));
        assert_eq!(next_cursor(r#"{"next":""}"#), None);
    }

    #[test]
    fn vn_date_uses_vietnam_time() {
        assert_eq!(vn_date(vn(2026, 11, 5, 0, 10)), "2026-11-05"); // 17:10 UTC hôm trước vẫn là ngày 05 giờ VN
        assert_eq!(vn_date(vn(2026, 11, 5, 23, 59)), "2026-11-05");
    }
}
