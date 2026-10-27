use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate};

use crate::provider::Day;

/// Một mốc dự báo 3 giờ.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// unix, UTC
    pub dt: i64,
    pub temp: f64,
    /// `rain.3h`; thiếu trường = 0
    pub rain_3h: Option<f64>,
    /// `pop` 0.0–1.0
    pub pop: f64,
    pub description: String,
    pub icon: String,
}

/// Số mốc 3 giờ của một ngày đầy đủ.
const SLOTS_PER_DAY: usize = 8;

/// Gộp các mốc 3 giờ thành từng ngày theo giờ địa phương (`tz_offset` giây).
pub fn aggregate(slots: &[Slot], tz_offset: i32) -> Vec<Day> {
    let mut by_day: BTreeMap<NaiveDate, Vec<&Slot>> = BTreeMap::new();
    for s in slots {
        if let Some(local) = DateTime::from_timestamp(s.dt + tz_offset as i64, 0) {
            by_day.entry(local.date_naive()).or_default().push(s);
        }
    }
    by_day
        .into_iter()
        .map(|(date, mut group)| {
            group.sort_by_key(|s| s.dt);
            let (description, icon) = most_common(&group);
            Day {
                date,
                temp_min: group.iter().map(|s| s.temp).fold(f64::INFINITY, f64::min),
                temp_max: group.iter().map(|s| s.temp).fold(f64::NEG_INFINITY, f64::max),
                rain_mm: group.iter().map(|s| s.rain_3h.unwrap_or(0.0)).sum(),
                pop_percent: (group.iter().map(|s| s.pop).fold(0.0, f64::max) * 1000.0).round() / 10.0,
                description,
                icon,
                partial: group.len() < SLOTS_PER_DAY,
            }
        })
        .collect()
}

/// Mô tả xuất hiện nhiều nhất (hòa thì lấy mốc sớm hơn) cùng biểu tượng của nó.
fn most_common(group: &[&Slot]) -> (String, String) {
    let count = |d: &str| group.iter().filter(|s| s.description == d).count();
    let best = group.iter().rev().max_by_key(|s| count(&s.description)).unwrap();
    let first = group.iter().find(|s| s.description == best.description).unwrap();
    (first.description.clone(), first.icon.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: i32 = 7 * 3600;

    /// Mốc theo giờ địa phương `ngày giờ` (UTC+7).
    fn slot(date: &str, hour: i64, temp: f64, rain: Option<f64>, pop: f64, desc: &str) -> Slot {
        let d: NaiveDate = date.parse().unwrap();
        let local_midnight = d.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();
        Slot {
            dt: local_midnight + hour * 3600 - TZ as i64,
            temp,
            rain_3h: rain,
            pop,
            description: desc.into(),
            icon: "10d".into(),
        }
    }

    fn full_day(date: &str, temp: f64) -> Vec<Slot> {
        (0..8).map(|k| slot(date, k * 3, temp, None, 0.0, "mây")).collect()
    }

    #[test]
    fn groups_by_local_calendar_day_not_utc() {
        // 00:00 giờ địa phương 28/10 là 17:00 UTC 27/10: phải thuộc ngày 28.
        let days = aggregate(&[slot("2026-10-28", 0, 25.0, None, 0.0, "mây")], TZ);
        assert_eq!(days.len(), 1);
        assert_eq!(days[0].date, "2026-10-28".parse::<NaiveDate>().unwrap());
    }

    #[test]
    fn min_max_temperature_rain_sum_and_max_pop() {
        let slots = vec![
            slot("2026-10-28", 0, 21.0, Some(2.5), 0.2, "mưa nhẹ"),
            slot("2026-10-28", 3, 27.5, Some(10.0), 0.9, "mưa nhẹ"),
            slot("2026-10-28", 6, 24.0, None, 0.4, "mưa nhẹ"),
        ];
        let d = &aggregate(&slots, TZ)[0];
        assert_eq!((d.temp_min, d.temp_max), (21.0, 27.5));
        assert_eq!(d.rain_mm, 12.5);
        assert_eq!(d.pop_percent, 90.0);
    }

    #[test]
    fn description_is_the_most_common_one() {
        let slots = vec![
            slot("2026-10-28", 0, 20.0, None, 0.0, "mây"),
            slot("2026-10-28", 3, 20.0, None, 0.0, "mưa"),
            slot("2026-10-28", 6, 20.0, None, 0.0, "mưa"),
        ];
        assert_eq!(aggregate(&slots, TZ)[0].description, "mưa");
    }

    #[test]
    fn days_are_sorted_and_incomplete_days_flagged_partial() {
        let mut slots = vec![slot("2026-10-27", 21, 22.0, None, 0.0, "mây")];
        slots.extend(full_day("2026-10-28", 25.0));
        slots.extend(full_day("2026-10-29", 26.0));
        slots.push(slot("2026-10-30", 0, 23.0, None, 0.0, "mây"));
        slots.reverse(); // thứ tự đầu vào không được ảnh hưởng
        let days = aggregate(&slots, TZ);
        let flags: Vec<_> = days.iter().map(|d| d.partial).collect();
        assert_eq!(days.len(), 4);
        assert_eq!(flags, vec![true, false, false, true]);
        assert!(days.windows(2).all(|w| w[0].date < w[1].date));
    }

    #[test]
    fn empty_input_gives_no_days() {
        assert!(aggregate(&[], TZ).is_empty());
    }
}
