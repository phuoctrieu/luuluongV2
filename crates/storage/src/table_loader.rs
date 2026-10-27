//! Nạp bảng cao trình – dung tích do người dùng thay (CSV hoặc XLSX), kiểm tra độ hợp lệ.

use std::path::Path;

use luuluong_core::{CalcError, VolumeTable};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TableLoadError {
    #[error("Không mở được file bảng dung tích. Kiểm tra đường dẫn và quyền đọc file.")]
    Io,
    #[error("Chỉ nhận file .csv hoặc .xlsx.")]
    UnsupportedType,
    #[error("File Excel không có dữ liệu ở hai cột đầu của trang tính đầu tiên.")]
    Empty,
    #[error("{0}")]
    Table(#[from] CalcError),
}

pub fn load_volume_table(path: &Path) -> Result<VolumeTable, TableLoadError> {
    let ext = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase);
    let csv = match ext.as_deref() {
        Some("csv") => std::fs::read_to_string(path).map_err(|_| TableLoadError::Io)?,
        Some("xlsx" | "xlsm" | "xls") => xlsx_to_csv(path)?,
        _ => return Err(TableLoadError::UnsupportedType),
    };
    Ok(VolumeTable::from_csv_str(&csv)?)
}

/// Lấy hai cột đầu của trang tính đầu tiên; dòng tiêu đề/trống (cả hai ô không phải số) bị bỏ qua.
fn xlsx_to_csv(path: &Path) -> Result<String, TableLoadError> {
    use calamine::{open_workbook_auto, Data, Reader};
    let mut book = open_workbook_auto(path).map_err(|_| TableLoadError::Io)?;
    let range = book.worksheet_range_at(0).ok_or(TableLoadError::Empty)?.map_err(|_| TableLoadError::Io)?;
    let number = |c: Option<&Data>| match c? {
        Data::Float(f) => Some(*f),
        Data::Int(i) => Some(*i as f64),
        _ => None,
    };
    let mut csv = String::from("cao_trinh_m,dung_tich_trieu_m3\n");
    let mut rows = 0;
    for (i, row) in range.rows().enumerate() {
        match (number(row.first()), number(row.get(1))) {
            (Some(z), Some(v)) => {
                csv.push_str(&format!("{z},{v}\n"));
                rows += 1;
            }
            (None, None) => {}
            _ => {
                return Err(CalcError::InvalidTable { line: i + 1, reason: "giá trị không phải số".into() }.into());
            }
        }
    }
    if rows == 0 {
        return Err(TableLoadError::Empty);
    }
    Ok(csv)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str, content: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-table-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join(name);
        std::fs::write(&p, content).unwrap();
        p
    }

    #[test]
    fn loads_a_valid_csv() {
        let p = temp("ok.csv", "cao_trinh_m,dung_tich_trieu_m3\n149,3\n150,4\n");
        let t = load_volume_table(&p).unwrap();
        assert_eq!((t.z_min(), t.z_max()), (149.0, 150.0));
    }

    #[test]
    fn invalid_csv_reports_the_bad_line() {
        let p = temp("bad.csv", "cao_trinh_m,dung_tich_trieu_m3\n149,3\n150,3\n");
        let e = load_volume_table(&p).err().unwrap();
        assert!(matches!(e, TableLoadError::Table(CalcError::InvalidTable { line: 3, .. })), "{e:?}");
    }

    #[test]
    fn xlsx_of_the_old_app_matches_the_builtin_table() {
        let t = load_volume_table(Path::new("tests/data/tinhtoan.xlsx")).unwrap();
        let builtin = VolumeTable::builtin();
        assert_eq!(t.len(), 125);
        assert_eq!(t.volume(160.05).unwrap(), builtin.volume(160.05).unwrap());
        assert_eq!(t.volume(165.0).unwrap(), builtin.volume(165.0).unwrap());
    }

    #[test]
    fn rejects_other_extensions_and_missing_files() {
        let p = temp("x.txt", "abc");
        assert!(matches!(load_volume_table(&p), Err(TableLoadError::UnsupportedType)));
        assert!(matches!(load_volume_table(Path::new("/khong/co/file.csv")), Err(TableLoadError::Io)));
    }
}
