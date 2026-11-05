//! Ghi báo cáo ra file Excel (.xlsx) nhiều sheet. Độc lập với kiểu dữ liệu của app: app dựng `Sheet`, ở đây chỉ trình bày và ghi.

use std::path::Path;

use rust_xlsxwriter::{Color, Format, FormatAlign, FormatBorder, Workbook, Worksheet, XlsxError};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("Không xuất được file Excel. Kiểm tra ổ đĩa còn chỗ trống.")]
pub struct ReportError;

#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    /// Ô trống: không ghi gì (khác với số 0).
    Empty,
    /// Luôn ghi là chuỗi, kể cả khi bắt đầu bằng `=`, `+`, `-`, `@`.
    Text(String),
    Num(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub name: String,
    pub title: String,
    pub subtitle: String,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
    /// Dòng tổng ngay sau dữ liệu (in đậm).
    pub footer: Vec<Cell>,
    pub note: String,
}

impl From<XlsxError> for ReportError {
    fn from(_: XlsxError) -> Self {
        ReportError
    }
}

/// Dòng 1 tiêu đề, dòng 2 phụ đề, dòng 4 tiêu đề cột (đóng băng), dữ liệu từ dòng 5, dòng tổng, một dòng trống rồi ghi chú.
const HEADER_ROW: u32 = 3;

fn put(ws: &mut Worksheet, row: u32, col: u16, cell: &Cell, text: &Format, num: &Format) -> Result<(), XlsxError> {
    match cell {
        Cell::Empty => {}
        Cell::Text(t) => {
            ws.write_string_with_format(row, col, t, text)?;
        }
        // Số không hữu hạn (NaN, vô cực) để trống thay vì ghi giá trị lỗi.
        Cell::Num(n) if n.is_finite() => {
            ws.write_number_with_format(row, col, *n, num)?;
        }
        Cell::Num(_) => {}
    }
    Ok(())
}

pub fn write_workbook(path: &Path, sheets: &[Sheet]) -> Result<(), ReportError> {
    let mut book = Workbook::new();
    let title = Format::new().set_bold().set_font_size(14);
    let head = Format::new().set_bold().set_background_color(Color::RGB(0x00E5_E7EB)).set_align(FormatAlign::Center).set_text_wrap().set_border(FormatBorder::Thin);
    let (text, num) = (Format::new(), Format::new().set_num_format("#,##0.00"));
    let (text_bold, num_bold) = (Format::new().set_bold(), Format::new().set_bold().set_num_format("#,##0.00"));
    for s in sheets {
        let ws = book.add_worksheet();
        ws.set_name(&s.name)?;
        ws.write_string_with_format(0, 0, &s.title, &title)?;
        ws.write_string(1, 0, &s.subtitle)?;
        ws.set_row_height(HEADER_ROW, 32)?;
        for (c, h) in s.headers.iter().enumerate() {
            ws.write_string_with_format(HEADER_ROW, c as u16, h, &head)?;
            ws.set_column_width(c as u16, if c < 2 { 24 } else { 18 })?;
        }
        ws.set_freeze_panes(HEADER_ROW + 1, 0)?;
        for (r, row) in s.rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                put(ws, HEADER_ROW + 1 + r as u32, c as u16, cell, &text, &num)?;
            }
        }
        let footer_row = HEADER_ROW + 1 + s.rows.len() as u32;
        for (c, cell) in s.footer.iter().enumerate() {
            put(ws, footer_row, c as u16, cell, &text_bold, &num_bold)?;
        }
        ws.write_string(footer_row + 2, 0, &s.note)?;
    }
    book.save(path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use calamine::{open_workbook_auto, Data, Reader};

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-xlsx-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    fn sheet(name: &str, rows: Vec<Vec<Cell>>) -> Sheet {
        Sheet {
            name: name.into(),
            title: "Báo cáo lưu lượng về hồ".into(),
            subtitle: "Từ 01/01/2026 đến 05/11/2026".into(),
            headers: vec!["Thời gian".into(), "Lưu lượng về TB (m³/s)".into(), "Mưa (mm)".into()],
            rows,
            footer: vec![Cell::Text("Tổng".into()), Cell::Num(10.0), Cell::Empty],
            note: "Chỉ gồm thời gian app đã chạy.".into(),
        }
    }

    /// Ghi rồi đọc lại bằng calamine: tên sheet và lưới ô (dòng bắt đầu từ A1).
    fn roundtrip(file: &str, sheets: &[Sheet]) -> (Vec<String>, Vec<Vec<Vec<Data>>>) {
        let path = temp(file);
        write_workbook(&path, sheets).unwrap();
        let mut book = open_workbook_auto(&path).unwrap();
        let names = book.sheet_names();
        let grids = names.iter().map(|n| book.worksheet_range(n).unwrap().rows().map(|r| r.to_vec()).collect()).collect();
        let _ = std::fs::remove_file(&path);
        (names, grids)
    }

    #[test]
    fn workbook_roundtrips_sheets_headers_and_numbers() {
        let a = sheet("Ngày", vec![vec![Cell::Text("05/11/2026".into()), Cell::Num(1234.5), Cell::Num(2.0)]]);
        let (names, grids) = roundtrip("a.xlsx", &[a, sheet("Tháng", vec![])]);
        assert_eq!(names, ["Ngày", "Tháng"]);
        let g = &grids[0];
        assert_eq!(g[0][0], Data::String("Báo cáo lưu lượng về hồ".into()));
        assert_eq!(g[1][0], Data::String("Từ 01/01/2026 đến 05/11/2026".into()));
        assert_eq!(g[3][..3], [Data::String("Thời gian".into()), Data::String("Lưu lượng về TB (m³/s)".into()), Data::String("Mưa (mm)".into())]);
        assert_eq!(g[4][0], Data::String("05/11/2026".into()));
        assert_eq!(g[4][1], Data::Float(1234.5));
    }

    #[test]
    fn empty_cell_stays_blank_but_zero_is_kept() {
        let r = vec![Cell::Text("x".into()), Cell::Empty, Cell::Num(0.0)];
        let (_, grids) = roundtrip("b.xlsx", &[sheet("Ngày", vec![r])]);
        assert_eq!(grids[0][4][1], Data::Empty);
        assert_eq!(grids[0][4][2], Data::Float(0.0));
    }

    #[test]
    fn formula_like_text_is_stored_as_text() {
        let rows = ["=1+1", "+SUM(A1)", "-2+3", "@x"].iter().map(|t| vec![Cell::Text((*t).into()), Cell::Empty, Cell::Empty]).collect();
        let (_, grids) = roundtrip("c.xlsx", &[sheet("Ngày", rows)]);
        let got: Vec<&Data> = (4..8).map(|i| &grids[0][i][0]).collect();
        assert_eq!(got, [&Data::String("=1+1".into()), &Data::String("+SUM(A1)".into()), &Data::String("-2+3".into()), &Data::String("@x".into())]);
    }

    #[test]
    fn footer_follows_data_and_note_after_a_blank_row() {
        let rows = vec![vec![Cell::Text("a".into()), Cell::Num(1.0), Cell::Empty], vec![Cell::Text("b".into()), Cell::Num(2.0), Cell::Empty]];
        let (_, grids) = roundtrip("d.xlsx", &[sheet("Ngày", rows)]);
        let g = &grids[0];
        assert_eq!(g[6][0], Data::String("Tổng".into()));
        assert_eq!(g[6][1], Data::Float(10.0));
        assert!(g[7].iter().all(|c| *c == Data::Empty));
        assert_eq!(g[8][0], Data::String("Chỉ gồm thời gian app đã chạy.".into()));
    }

    #[test]
    fn unwritable_path_is_an_error_not_a_panic() {
        let path = temp("no-such-dir").join("nested").join("x.xlsx");
        assert!(write_workbook(&path, &[sheet("Ngày", vec![])]).is_err());
    }

    #[test]
    fn bad_sheet_name_is_an_error_not_a_panic() {
        let too_long = "x".repeat(40); // Excel giới hạn tên sheet 31 ký tự
        assert!(write_workbook(&temp("e.xlsx"), &[sheet(&too_long, vec![])]).is_err());
    }
}
