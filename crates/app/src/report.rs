//! Báo cáo lưu lượng về hồ và mưa thượng nguồn theo ngày, tháng, năm. Thuần logic (thời gian truyền vào).

use chrono::{Datelike, Days, Months, NaiveDate};
use luuluong_storage::history::{RainObs, Record};
use luuluong_storage::xlsx_report::{Cell, Sheet};

use crate::averages::flow_between;
use crate::live::VN_TZ;
use crate::rain::rain_between;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Granularity {
    Day,
    Month,
    Year,
}

/// Một kỳ báo cáo `[start, end)` (epoch giây), đã cắt theo khoảng người dùng chọn và theo thời điểm hiện tại.
#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    pub label: String,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReportRow {
    pub period: Period,
    pub q_avg: Option<f64>,
    pub volume_m3: Option<f64>,
    pub covered: f64,
    pub rain_mm: [Option<f64>; 2],
    /// Trung bình hai trạm; chỉ có khi cả hai trạm đều có số liệu.
    pub rain_mean: Option<f64>,
    /// Trung bình tỷ lệ giờ có số liệu của hai trạm (0..=1); `None` nếu kỳ ngắn hơn 1 giờ.
    pub rain_covered: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Totals {
    pub volume_m3: Option<f64>,
    pub q_avg: Option<f64>,
    pub rain_mm: [Option<f64>; 2],
    pub rain_mean: Option<f64>,
}

/// Hai trạm đưa vào báo cáo (id, tên).
pub const REPORT_STATIONS: [(&str, &str); 2] = [("841697470576", "Hương Nguyên"), ("84396786852", "A Đớt")];

/// Nửa đêm giờ Việt Nam của ngày `d`.
pub fn midnight(d: NaiveDate) -> i64 {
    d.and_hms_opt(0, 0, 0).map_or(0, |t| t.and_utc().timestamp()) - i64::from(VN_TZ)
}

/// Ngày đầu của kỳ lịch chứa `d`.
fn align(d: NaiveDate, g: Granularity) -> NaiveDate {
    match g {
        Granularity::Day => d,
        Granularity::Month => d.with_day(1).unwrap_or(d),
        Granularity::Year => NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap_or(d),
    }
}

/// Ngày đầu của kỳ lịch kế tiếp.
fn step(d: NaiveDate, g: Granularity) -> NaiveDate {
    match g {
        Granularity::Day => d.checked_add_days(Days::new(1)),
        Granularity::Month => d.checked_add_months(Months::new(1)),
        Granularity::Year => d.checked_add_months(Months::new(12)),
    }
    .unwrap_or(NaiveDate::MAX)
}

fn label(d: NaiveDate, g: Granularity) -> String {
    match g {
        Granularity::Day => d.format("%d/%m/%Y").to_string(),
        Granularity::Month => d.format("%m/%Y").to_string(),
        Granularity::Year => d.format("%Y").to_string(),
    }
}

/// Các kỳ lịch (ngày/tháng/năm) giao với `[từ nửa đêm from, hết ngày to]`, cắt đầu cuối theo khoảng chọn và theo `now`.
pub fn periods(from: NaiveDate, to: NaiveDate, g: Granularity, now: i64) -> Vec<Period> {
    let range_start = midnight(from);
    let range_end = midnight(to.checked_add_days(Days::new(1)).unwrap_or(to)).min(now);
    let mut out = Vec::new();
    let mut cur = align(from, g);
    while cur <= to && cur < NaiveDate::MAX {
        let next = step(cur, g);
        let (start, end) = (midnight(cur).max(range_start), midnight(next).min(range_end));
        if start < end {
            out.push(Period { label: label(cur, g), start, end });
        }
        cur = next;
    }
    out
}

pub fn build(
    records: &[Record],
    obs: &[RainObs],
    stations: [&str; 2],
    from: NaiveDate,
    to: NaiveDate,
    g: Granularity,
    now: i64,
) -> Vec<ReportRow> {
    periods(from, to, g, now)
        .into_iter()
        .map(|period| {
            let flow = flow_between(records, period.start, period.end);
            let rain = stations.map(|id| rain_between(obs, id, period.start, period.end));
            let rain_mm = [rain[0].mm, rain[1].mm];
            let rain_mean = match rain_mm {
                [Some(a), Some(b)] => Some((a + b) / 2.0),
                _ => None,
            };
            let rain_covered = (rain[0].expected > 0)
                .then(|| rain.iter().map(|r| (f64::from(r.hours) / f64::from(r.expected)).min(1.0)).sum::<f64>() / 2.0);
            ReportRow { period, q_avg: flow.q_avg, volume_m3: flow.volume_m3, covered: flow.covered, rain_mm, rain_mean, rain_covered }
        })
        .collect()
}

fn sum_some(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    values.flatten().fold(None, |acc, v| Some(acc.unwrap_or(0.0) + v))
}

pub fn totals(rows: &[ReportRow]) -> Totals {
    let volume_m3 = sum_some(rows.iter().map(|r| r.volume_m3));
    // Thời gian có số liệu (giây) của những kỳ có lưu lượng.
    let seconds: f64 = rows.iter().filter(|r| r.volume_m3.is_some()).map(|r| r.covered * (r.period.end - r.period.start) as f64).sum();
    let q_avg = volume_m3.filter(|_| seconds > 0.0).map(|v| v / seconds);
    let rain_mm = [0, 1].map(|i| sum_some(rows.iter().map(|r| r.rain_mm[i])));
    let rain_mean = match rain_mm {
        [Some(a), Some(b)] => Some((a + b) / 2.0),
        _ => None,
    };
    Totals { volume_m3, q_avg, rain_mm, rain_mean }
}

/// `Err` là câu tiếng Việt hiện ngay dưới ô nhập.
pub fn parse_range(from: &str, to: &str, today: NaiveDate) -> Result<(NaiveDate, NaiveDate), String> {
    let parse = |s: &str| NaiveDate::parse_from_str(s.trim(), "%d/%m/%Y").map_err(|_| "Nhập ngày dạng 31/12/2026.".to_string());
    let (from, to) = (parse(from)?, parse(to)?);
    if from > to {
        return Err("Ngày bắt đầu phải trước hoặc bằng ngày kết thúc.".into());
    }
    if to > today {
        return Err("Ngày kết thúc không được ở tương lai.".into());
    }
    if (to - from).num_days() > 3660 {
        return Err("Khoảng báo cáo tối đa 10 năm.".into());
    }
    Ok((from, to))
}

fn num(v: Option<f64>) -> Cell {
    v.map_or(Cell::Empty, Cell::Num)
}

fn vn_day(ts: i64) -> NaiveDate {
    chrono::DateTime::from_timestamp(ts + i64::from(VN_TZ), 0).map_or(NaiveDate::MIN, |t| t.date_naive())
}

/// Khoảng thực sự được tính của kỳ, dạng `05/11/2026 – 05/11/2026`.
pub fn span_label(p: &Period) -> String {
    format!("{} – {}", vn_day(p.start).format("%d/%m/%Y"), vn_day(p.end - 1).format("%d/%m/%Y"))
}

fn percent(v: f64) -> Cell {
    Cell::Num(v * 100.0)
}

/// Ba sheet theo thứ tự đã cho; `names` = tên hai trạm mưa.
pub fn to_sheets(rows_by_g: [(Granularity, Vec<ReportRow>); 3], names: [&str; 2], from: NaiveDate, to: NaiveDate) -> Vec<Sheet> {
    let range = format!("{} đến {}", from.format("%d/%m/%Y"), to.format("%d/%m/%Y"));
    rows_by_g
        .into_iter()
        .map(|(g, rows)| {
            let (name, unit) = match g {
                Granularity::Day => ("Ngày", "ngày"),
                Granularity::Month => ("Tháng", "tháng"),
                Granularity::Year => ("Năm", "năm"),
            };
            let t = totals(&rows);
            let total_secs: f64 = rows.iter().map(|r| (r.period.end - r.period.start) as f64).sum();
            let covered_secs: f64 = rows.iter().map(|r| r.covered * (r.period.end - r.period.start) as f64).sum();
            let rain_cov: Vec<f64> = rows.iter().filter_map(|r| r.rain_covered).collect();
            let footer = vec![
                Cell::Text("Tổng / bình quân".into()),
                Cell::Empty,
                num(t.q_avg),
                num(t.volume_m3),
                if total_secs > 0.0 { percent(covered_secs / total_secs) } else { Cell::Empty },
                num(t.rain_mm[0]),
                num(t.rain_mm[1]),
                num(t.rain_mean),
                if rain_cov.is_empty() { Cell::Empty } else { percent(rain_cov.iter().sum::<f64>() / rain_cov.len() as f64) },
            ];
            Sheet {
                name: name.into(),
                title: "Báo cáo lưu lượng về hồ Sông Bồ và mưa thượng nguồn".into(),
                subtitle: format!("Theo {unit}, từ {range}"),
                headers: vec![
                    "Thời gian".into(),
                    "Khoảng tính".into(),
                    "Lưu lượng về TB (m³/s)".into(),
                    "Tổng lượng về (m³)".into(),
                    "Có số liệu (%)".into(),
                    format!("Mưa {} (mm)", names[0]),
                    format!("Mưa {} (mm)", names[1]),
                    "Mưa TB 2 trạm (mm)".into(),
                    "Có số liệu mưa (%)".into(),
                ],
                rows: rows
                    .iter()
                    .map(|r| {
                        vec![
                            Cell::Text(r.period.label.clone()),
                            Cell::Text(span_label(&r.period)),
                            num(r.q_avg),
                            num(r.volume_m3),
                            percent(r.covered),
                            num(r.rain_mm[0]),
                            num(r.rain_mm[1]),
                            num(r.rain_mean),
                            r.rain_covered.map_or(Cell::Empty, percent),
                        ]
                    })
                    .collect(),
                footer,
                note: "Chỉ gồm thời gian app đã chạy và có số liệu; xem cột Có số liệu. Mưa là tổng các mốc giờ của Vrain (mốc 00:00 thuộc ngày trước). Mưa không đi vào công thức lưu lượng về.".into(),
            }
        })
        .collect()
}

/// Đường dẫn người dùng chọn trong hộp thoại lưu; thiếu đuôi `.xlsx` thì thêm vào (không thay đuôi khác).
pub fn with_xlsx_extension(path: &std::path::Path) -> std::path::PathBuf {
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xlsx")) {
        return path.to_path_buf();
    }
    let mut name = path.as_os_str().to_owned();
    name.push(".xlsx");
    name.into()
}

pub fn file_name(from: NaiveDate, to: NaiveDate) -> String {
    format!("bao-cao-luu-luong-ve-{}-{}.xlsx", from.format("%d%m%Y"), to.format("%d%m%Y"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::live::AUTO_NOTE;
    use luuluong_storage::history::vn_timestamp;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Nửa đêm giờ Việt Nam của ngày `date`.
    fn mid(date: NaiveDate) -> i64 {
        date.and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp() - i64::from(VN_TZ)
    }

    const FAR: i64 = 4_000_000_000;

    fn labels(p: &[Period]) -> Vec<&str> {
        p.iter().map(|x| x.label.as_str()).collect()
    }

    #[test]
    fn periods_day_count_and_labels() {
        let p = periods(d(2026, 11, 3), d(2026, 11, 5), Granularity::Day, FAR);
        assert_eq!(labels(&p), ["03/11/2026", "04/11/2026", "05/11/2026"]);
        assert!(p.iter().all(|x| x.end - x.start == 86_400));
    }

    #[test]
    fn periods_month_clips_first_and_last() {
        let p = periods(d(2026, 11, 10), d(2026, 12, 20), Granularity::Month, FAR);
        assert_eq!(labels(&p), ["11/2026", "12/2026"]);
        assert_eq!((p[0].start, p[0].end), (mid(d(2026, 11, 10)), mid(d(2026, 12, 1))));
        assert_eq!((p[1].start, p[1].end), (mid(d(2026, 12, 1)), mid(d(2026, 12, 21))));
    }

    #[test]
    fn periods_handle_leap_february_2028() {
        let p = periods(d(2028, 2, 28), d(2028, 3, 1), Granularity::Day, FAR);
        assert_eq!(labels(&p), ["28/02/2028", "29/02/2028", "01/03/2028"]);
        let feb = periods(d(2028, 2, 1), d(2028, 2, 29), Granularity::Month, FAR);
        assert_eq!(feb[0].end - feb[0].start, 29 * 86_400);
    }

    #[test]
    fn periods_year_boundary() {
        let (a, b) = (d(2026, 12, 31), d(2027, 1, 1));
        assert_eq!(labels(&periods(a, b, Granularity::Day, FAR)), ["31/12/2026", "01/01/2027"]);
        assert_eq!(labels(&periods(a, b, Granularity::Month, FAR)), ["12/2026", "01/2027"]);
        let y = periods(a, b, Granularity::Year, FAR);
        assert_eq!(labels(&y), ["2026", "2027"]);
        assert_eq!((y[0].start, y[0].end), (mid(a), mid(b)));
    }

    #[test]
    fn periods_never_start_after_now() {
        let now = mid(d(2026, 11, 5)) + 3600;
        let p = periods(d(2026, 11, 3), d(2026, 11, 10), Granularity::Day, now);
        assert_eq!(labels(&p), ["03/11/2026", "04/11/2026", "05/11/2026"]);
        assert_eq!(p.last().unwrap().end, now);
        assert!(p.iter().all(|x| x.start < x.end && x.end <= now));
        assert!(periods(d(2026, 11, 6), d(2026, 11, 7), Granularity::Day, now).is_empty());
    }

    fn cycle(end: &str, minutes: i64, q: f64) -> Record {
        Record {
            id: 0,
            created_at: end.into(),
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

    fn ro(id: &str, ts: i64, mm: f64) -> RainObs {
        RainObs { station_id: id.into(), ts, mm }
    }

    #[test]
    fn build_blank_when_no_data() {
        let rows = build(&[], &[], ["a", "b"], d(2026, 11, 5), d(2026, 11, 5), Granularity::Day, FAR);
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!((r.q_avg, r.volume_m3, r.covered), (None, None, 0.0));
        assert_eq!((r.rain_mm, r.rain_mean), ([None, None], None));
        assert_eq!(r.rain_covered, Some(0.0));
    }

    #[test]
    fn build_one_station_missing_leaves_mean_blank() {
        let t = mid(d(2026, 11, 5)) + 3600;
        let rows = build(&[], &[ro("a", t, 2.0)], ["a", "b"], d(2026, 11, 5), d(2026, 11, 5), Granularity::Day, FAR);
        assert_eq!(rows[0].rain_mm, [Some(2.0), None]);
        assert_eq!(rows[0].rain_mean, None);
    }

    #[test]
    fn build_combines_flow_and_rain_for_a_day() {
        let day = d(2026, 11, 5);
        let one_am = mid(day) + 3600;
        let rec = cycle(&vn_timestamp(one_am), 60, 20.0); // chu kỳ 00:00–01:00, q = 20
        let obs = [ro("a", one_am, 1.0), ro("a", one_am + 3600, 2.0), ro("b", one_am, 4.0), ro("zz", one_am, 99.0)];
        let rows = build(&[rec], &obs, ["a", "b"], day, day, Granularity::Day, mid(d(2026, 11, 6)));
        let r = &rows[0];
        assert_eq!(r.q_avg, Some(20.0));
        assert!((r.covered - 1.0 / 24.0).abs() < 1e-9);
        assert!((r.volume_m3.unwrap() - 72_000.0).abs() < 1e-6);
        assert_eq!(r.rain_mm, [Some(3.0), Some(4.0)]);
        assert_eq!(r.rain_mean, Some(3.5));
        assert!((r.rain_covered.unwrap() - 1.5 / 24.0).abs() < 1e-9);
    }

    fn row(len: i64, covered: f64, q: Option<f64>, rain: [Option<f64>; 2]) -> ReportRow {
        ReportRow {
            period: Period { label: String::new(), start: 0, end: len },
            q_avg: q,
            volume_m3: q.map(|q| q * covered * len as f64),
            covered,
            rain_mm: rain,
            rain_mean: None,
            rain_covered: None,
        }
    }

    #[test]
    fn totals_volume_is_sum_and_q_is_time_weighted() {
        let rows = [row(3600, 1.0, Some(10.0), [Some(1.0), Some(2.0)]), row(7200, 0.5, Some(30.0), [Some(3.0), None])];
        let t = totals(&rows);
        assert_eq!(t.volume_m3, Some(144_000.0));
        assert!((t.q_avg.unwrap() - 20.0).abs() < 1e-9);
        assert_eq!(t.rain_mm, [Some(4.0), Some(2.0)]);
        assert_eq!(t.rain_mean, Some(3.0));
    }

    #[test]
    fn totals_blank_when_all_blank() {
        let t = totals(&[row(3600, 0.0, None, [None, None])]);
        assert_eq!(t, Totals { volume_m3: None, q_avg: None, rain_mm: [None, None], rain_mean: None });
        assert_eq!(totals(&[]), t);
    }

    fn day_row(date: NaiveDate, q: Option<f64>, covered: f64, rain: [Option<f64>; 2]) -> ReportRow {
        let period = Period { label: date.format("%d/%m/%Y").to_string(), start: mid(date), end: mid(date) + 86_400 };
        ReportRow {
            volume_m3: q.map(|q| q * covered * 86_400.0),
            period,
            q_avg: q,
            covered,
            rain_mm: rain,
            rain_mean: match rain {
                [Some(a), Some(b)] => Some((a + b) / 2.0),
                _ => None,
            },
            rain_covered: Some(0.5),
        }
    }

    fn three(rows: Vec<ReportRow>) -> [(Granularity, Vec<ReportRow>); 3] {
        [(Granularity::Day, rows.clone()), (Granularity::Month, rows.clone()), (Granularity::Year, rows)]
    }

    fn sheets(rows: Vec<ReportRow>) -> Vec<Sheet> {
        to_sheets(three(rows), ["Hương Nguyên", "A Đớt"], d(2026, 1, 1), d(2026, 11, 5))
    }

    #[test]
    fn sheets_from_rows_have_three_named_sheets_in_order() {
        let s = sheets(vec![]);
        assert_eq!(s.iter().map(|x| x.name.as_str()).collect::<Vec<_>>(), ["Ngày", "Tháng", "Năm"]);
        assert!(s[0].subtitle.contains("01/01/2026") && s[0].subtitle.contains("05/11/2026"));
    }

    #[test]
    fn sheet_columns_match_the_design() {
        let r = day_row(d(2026, 11, 5), Some(20.0), 0.5, [Some(3.0), Some(4.0)]);
        let s = &sheets(vec![r])[0];
        assert_eq!(
            s.headers,
            [
                "Thời gian",
                "Khoảng tính",
                "Lưu lượng về TB (m³/s)",
                "Tổng lượng về (m³)",
                "Có số liệu (%)",
                "Mưa Hương Nguyên (mm)",
                "Mưa A Đớt (mm)",
                "Mưa TB 2 trạm (mm)",
                "Có số liệu mưa (%)"
            ]
        );
        assert_eq!(
            s.rows[0],
            vec![
                Cell::Text("05/11/2026".into()),
                Cell::Text("05/11/2026 – 05/11/2026".into()),
                Cell::Num(20.0),
                Cell::Num(864_000.0),
                Cell::Num(50.0),
                Cell::Num(3.0),
                Cell::Num(4.0),
                Cell::Num(3.5),
                Cell::Num(50.0),
            ]
        );
    }

    #[test]
    fn blank_rows_become_empty_cells_but_coverage_stays_a_number() {
        let r = day_row(d(2026, 11, 5), None, 0.0, [None, None]);
        let row = &sheets(vec![r])[0].rows[0];
        assert_eq!(row[2..4], [Cell::Empty, Cell::Empty]);
        assert_eq!(row[4], Cell::Num(0.0));
        assert_eq!(row[5..8], [Cell::Empty, Cell::Empty, Cell::Empty]);
    }

    #[test]
    fn footer_contains_totals() {
        let rows = vec![day_row(d(2026, 11, 4), Some(10.0), 1.0, [Some(1.0), Some(2.0)]), day_row(d(2026, 11, 5), Some(30.0), 1.0, [Some(3.0), None])];
        let f = &sheets(rows)[0].footer;
        assert_eq!(f[0], Cell::Text("Tổng / bình quân".into()));
        assert_eq!(f[2], Cell::Num(20.0)); // bình quân theo thời gian
        assert_eq!(f[3], Cell::Num((10.0 + 30.0) * 86_400.0)); // cộng
        assert_eq!(f[4], Cell::Num(100.0));
        assert_eq!(f[5..7], [Cell::Num(4.0), Cell::Num(2.0)]);
    }

    #[test]
    fn xlsx_extension_is_added_only_when_missing() {
        use std::path::Path;
        assert_eq!(with_xlsx_extension(Path::new("/tmp/bao-cao")), Path::new("/tmp/bao-cao.xlsx"));
        assert_eq!(with_xlsx_extension(Path::new("/tmp/bao-cao.xlsx")), Path::new("/tmp/bao-cao.xlsx"));
        assert_eq!(with_xlsx_extension(Path::new("/tmp/BAO-CAO.XLSX")), Path::new("/tmp/BAO-CAO.XLSX"));
        assert_eq!(with_xlsx_extension(Path::new("/tmp/bao-cao.csv")), Path::new("/tmp/bao-cao.csv.xlsx"));
    }

    #[test]
    fn file_name_uses_range() {
        assert_eq!(file_name(d(2026, 1, 1), d(2026, 11, 5)), "bao-cao-luu-luong-ve-01012026-05112026.xlsx");
    }


    #[test]
    fn parse_range_ok_and_errors() {
        let today = d(2026, 11, 5);
        assert_eq!(parse_range("5/11/2026", " 05/11/2026 ", today), Ok((d(2026, 11, 5), d(2026, 11, 5))));
        assert_eq!(parse_range("01/01/2026", "05/11/2026", today), Ok((d(2026, 1, 1), today)));
        let format = "Nhập ngày dạng 31/12/2026.".to_string();
        assert_eq!(parse_range("abc", "05/11/2026", today), Err(format.clone()));
        assert_eq!(parse_range("01/01/2026", "", today), Err(format));
        assert_eq!(parse_range("32/01/2026", "05/11/2026", today).unwrap_err(), "Nhập ngày dạng 31/12/2026.");
        assert_eq!(parse_range("06/11/2026", "05/11/2026", today).unwrap_err(), "Ngày bắt đầu phải trước hoặc bằng ngày kết thúc.");
        assert_eq!(parse_range("01/01/2026", "06/11/2026", today).unwrap_err(), "Ngày kết thúc không được ở tương lai.");
        assert_eq!(parse_range("01/01/2016", "05/11/2026", today).unwrap_err(), "Khoảng báo cáo tối đa 10 năm.");
        assert!(parse_range("06/11/2016", "05/11/2026", today).is_ok(), "đúng 10 năm vẫn được");
    }
}
