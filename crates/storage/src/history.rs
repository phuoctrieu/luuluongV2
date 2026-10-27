//! Lịch sử tính toán trong SQLite (bảng `calc`, đặc tả mục 4).

use std::path::Path;

use chrono::{DateTime, FixedOffset, NaiveDate};
use rusqlite::Connection;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    /// 0 khi chưa lưu.
    pub id: i64,
    /// ISO 8601 giờ Việt Nam, ví dụ `2026-10-27T10:30:00+07:00`.
    pub created_at: String,
    pub z_start: f64,
    pub z_end: f64,
    pub hours: f64,
    pub h1: f64,
    pub h2: f64,
    pub spill_q: f64,
    pub spill_h: f64,
    pub eco_q: f64,
    pub q_in: f64,
    pub w_in: f64,
    pub operator: String,
    pub note: String,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("Không ghi hoặc đọc được lịch sử tính toán (kiểm tra ổ đĩa còn chỗ trống).")]
    Db(#[from] rusqlite::Error),
}

pub struct History {
    conn: Connection,
}

/// Thời điểm `unix` (giây, UTC) dưới dạng ISO 8601 giờ Việt Nam (UTC+7).
pub fn vn_timestamp(unix: i64) -> String {
    let vn = FixedOffset::east_opt(7 * 3600).expect("UTC+7 hợp lệ");
    DateTime::from_timestamp(unix, 0).map_or_else(String::new, |t| t.with_timezone(&vn).to_rfc3339())
}

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS calc (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    created_at TEXT NOT NULL,
    z_start    REAL NOT NULL,
    z_end      REAL NOT NULL,
    hours      REAL NOT NULL,
    h1         REAL NOT NULL,
    h2         REAL NOT NULL,
    spill_q    REAL NOT NULL,
    spill_h    REAL NOT NULL,
    eco_q      REAL NOT NULL,
    q_in       REAL NOT NULL,
    w_in       REAL NOT NULL,
    operator   TEXT NOT NULL DEFAULT '',
    note       TEXT NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS idx_calc_created ON calc(created_at);";

const COLUMNS: &str = "id, created_at, z_start, z_end, hours, h1, h2, spill_q, spill_h, eco_q, q_in, w_in, operator, note";

impl History {
    pub fn open(path: &Path) -> Result<History, StorageError> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<History, StorageError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<History, StorageError> {
        conn.execute_batch(SCHEMA)?;
        Ok(History { conn })
    }

    pub fn insert(&self, r: &Record) -> Result<i64, StorageError> {
        self.conn.execute(
            "INSERT INTO calc (created_at, z_start, z_end, hours, h1, h2, spill_q, spill_h, eco_q, q_in, w_in, operator, note)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                r.created_at, r.z_start, r.z_end, r.hours, r.h1, r.h2, r.spill_q, r.spill_h, r.eco_q, r.q_in, r.w_in,
                r.operator, r.note
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Mới nhất trước; `date` lọc theo ngày (giờ Việt Nam).
    pub fn list(&self, date: Option<NaiveDate>) -> Result<Vec<Record>, StorageError> {
        let mut stmt = self.conn.prepare(&format!(
            "SELECT {COLUMNS} FROM calc WHERE (?1 IS NULL OR substr(created_at, 1, 10) = ?1)
             ORDER BY created_at DESC, id DESC"
        ))?;
        let day = date.map(|d| d.format("%Y-%m-%d").to_string());
        let rows = stmt.query_map([day], |row| {
            Ok(Record {
                id: row.get(0)?,
                created_at: row.get(1)?,
                z_start: row.get(2)?,
                z_end: row.get(3)?,
                hours: row.get(4)?,
                h1: row.get(5)?,
                h2: row.get(6)?,
                spill_q: row.get(7)?,
                spill_h: row.get(8)?,
                eco_q: row.get(9)?,
                q_in: row.get(10)?,
                w_in: row.get(11)?,
                operator: row.get(12)?,
                note: row.get(13)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn delete(&self, id: i64) -> Result<(), StorageError> {
        self.conn.execute("DELETE FROM calc WHERE id = ?1", [id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(created_at: &str, q: f64) -> Record {
        Record {
            id: 0,
            created_at: created_at.into(),
            z_start: 160.0,
            z_end: 161.0,
            hours: 1.0,
            h1: 0.0,
            h2: 0.0,
            spill_q: 0.0,
            spill_h: 0.0,
            eco_q: 0.0,
            q_in: q,
            w_in: q * 3600.0,
            operator: "Linh".into(),
            note: String::new(),
        }
    }

    #[test]
    fn vn_timestamp_is_iso_with_plus_seven() {
        // 2026-10-27 03:30:00 UTC = 10:30 giờ Việt Nam
        let utc = "2026-10-27".parse::<NaiveDate>().unwrap().and_hms_opt(3, 30, 0).unwrap().and_utc().timestamp();
        assert_eq!(vn_timestamp(utc), "2026-10-27T10:30:00+07:00");
        // 20:00 UTC đã sang ngày hôm sau ở Việt Nam
        let late = "2026-10-27".parse::<NaiveDate>().unwrap().and_hms_opt(20, 0, 0).unwrap().and_utc().timestamp();
        assert_eq!(vn_timestamp(late), "2026-10-28T03:00:00+07:00");
    }

    #[test]
    fn insert_then_list_returns_the_same_record_with_an_id() {
        let h = History::open_in_memory().unwrap();
        let id = h.insert(&rec("2026-10-27T10:30:00+07:00", 254.52)).unwrap();
        let rows = h.list(None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].id, id);
        assert_eq!(rows[0], Record { id, ..rec("2026-10-27T10:30:00+07:00", 254.52) });
    }

    #[test]
    fn list_is_newest_first() {
        let h = History::open_in_memory().unwrap();
        h.insert(&rec("2026-10-27T08:00:00+07:00", 1.0)).unwrap();
        h.insert(&rec("2026-10-27T09:00:00+07:00", 2.0)).unwrap();
        let q: Vec<_> = h.list(None).unwrap().iter().map(|r| r.q_in).collect();
        assert_eq!(q, vec![2.0, 1.0]);
    }

    #[test]
    fn filter_by_local_date() {
        let h = History::open_in_memory().unwrap();
        h.insert(&rec("2026-10-26T23:59:00+07:00", 1.0)).unwrap();
        h.insert(&rec("2026-10-27T00:01:00+07:00", 2.0)).unwrap();
        let day = "2026-10-27".parse().unwrap();
        let q: Vec<_> = h.list(Some(day)).unwrap().iter().map(|r| r.q_in).collect();
        assert_eq!(q, vec![2.0]);
    }

    #[test]
    fn delete_removes_only_that_row() {
        let h = History::open_in_memory().unwrap();
        let a = h.insert(&rec("2026-10-27T08:00:00+07:00", 1.0)).unwrap();
        let b = h.insert(&rec("2026-10-27T09:00:00+07:00", 2.0)).unwrap();
        h.delete(a).unwrap();
        let ids: Vec<_> = h.list(None).unwrap().iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![b]);
    }

    #[test]
    fn data_survives_reopening_the_file() {
        let dir = std::env::temp_dir().join(format!("luuluong-history-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("history.db");
        let _ = std::fs::remove_file(&path);
        History::open(&path).unwrap().insert(&rec("2026-10-27T10:30:00+07:00", 3.0)).unwrap();
        assert_eq!(History::open(&path).unwrap().list(None).unwrap().len(), 1);
    }
}
