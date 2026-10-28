//! Xuất lịch sử ra CSV mở được bằng Excel tiếng Việt: UTF-8 có BOM, ngăn cách `;`, thập phân `,`.

use std::path::Path;

use crate::history::Record;

const HEADER: [&str; 14] = [
    "Thời điểm",
    "Mực nước đầu (m)",
    "Mực nước cuối (m)",
    "Thời gian tính (giờ)",
    "H1 (giờ)",
    "H2 (giờ)",
    "Xả tràn (m3/s)",
    "Xả tràn (giờ)",
    "Xả tối thiểu (m3/s)",
    "Lưu lượng về hồ (m3/s)",
    "Tổng lượng về hồ (m3)",
    "Lượng mưa (mm)",
    "Người nhập",
    "Ghi chú",
];

/// Thập phân dùng dấu phẩy để Excel tiếng Việt đọc thành số.
fn num(x: f64) -> String {
    x.to_string().replace('.', ",")
}

fn num2(x: f64) -> String {
    format!("{x:.2}").replace('.', ",")
}

pub fn to_csv(records: &[Record]) -> String {
    let mut w = csv::WriterBuilder::new().delimiter(b';').from_writer(Vec::new());
    w.write_record(HEADER).expect("ghi vào bộ nhớ không lỗi");
    for r in records {
        w.write_record([
            r.created_at.clone(),
            num(r.z_start),
            num(r.z_end),
            num(r.hours),
            num(r.h1),
            num(r.h2),
            num(r.spill_q),
            num(r.spill_h),
            num(r.eco_q),
            num2(r.q_in),
            num2(r.w_in),
            r.rain_mm.map(num2).unwrap_or_default(),
            r.operator.clone(),
            r.note.clone(),
        ])
        .expect("ghi vào bộ nhớ không lỗi");
    }
    let bytes = w.into_inner().expect("ghi vào bộ nhớ không lỗi");
    format!("\u{feff}{}", String::from_utf8(bytes).expect("csv do chính ta ghi là UTF-8"))
}

pub fn write_csv(path: &Path, records: &[Record]) -> std::io::Result<()> {
    std::fs::write(path, to_csv(records))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(note: &str) -> Record {
        Record {
            id: 1,
            created_at: "2026-10-27T10:30:00+07:00".into(),
            z_start: 160.0,
            z_end: 161.0,
            hours: 1.0,
            h1: 0.5,
            h2: 0.0,
            spill_q: 0.0,
            spill_h: 0.0,
            eco_q: 0.0,
            q_in: 254.5155,
            w_in: 916255.1374,
            rain_mm: None,
            operator: "Linh".into(),
            note: note.into(),
        }
    }

    #[test]
    fn starts_with_bom_and_vietnamese_header() {
        let out = to_csv(&[]);
        assert!(out.starts_with('\u{feff}'));
        let header = out.trim_start_matches('\u{feff}').lines().next().unwrap();
        assert!(header.starts_with("Thời điểm;Mực nước đầu (m);Mực nước cuối (m);"));
        assert!(header.ends_with(";Người nhập;Ghi chú"));
        assert_eq!(header.split(';').count(), 14);
        assert!(header.contains(";Tổng lượng về hồ (m3);Lượng mưa (mm);Người nhập"));
    }

    #[test]
    fn numbers_use_decimal_comma_and_results_two_decimals() {
        let out = to_csv(&[rec("")]);
        let row = out.lines().nth(1).unwrap();
        assert_eq!(row, "2026-10-27T10:30:00+07:00;160;161;1;0,5;0;0;0;0;254,52;916255,14;;Linh;");
    }

    #[test]
    fn rain_is_written_with_two_decimals_and_left_empty_when_unknown() {
        let mut r = rec("");
        r.rain_mm = Some(3.456);
        let row = to_csv(&[r]).lines().nth(1).unwrap().to_string();
        assert!(row.contains(";916255,14;3,46;Linh;"), "{row}");
    }

    #[test]
    fn text_with_separator_or_quotes_is_quoted_so_columns_stay_aligned() {
        let out = to_csv(&[rec("đã chạy; \"thử\"")]);
        let row = out.lines().nth(1).unwrap();
        assert!(row.ends_with(";\"đã chạy; \"\"thử\"\"\""));
    }
}
