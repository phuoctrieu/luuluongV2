use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::provider::Snapshot;

/// Dữ liệu cache của một nguồn (`source_id` là `Source::id`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub source_id: String,
    pub snapshot: Snapshot,
}

pub fn save(path: &Path, entries: &[Entry]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string(entries)?)
}

/// Rỗng nếu chưa có cache hoặc file hỏng/định dạng cũ (không làm app dừng).
pub fn load(path: &Path) -> Vec<Entry> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::Current;

    fn snapshot() -> Snapshot {
        Snapshot {
            current: Current {
                temp: 24.5,
                humidity: 90.0,
                wind_speed: 2.1,
                description: "mưa rào".into(),
                icon: "09d".into(),
                observed_at: 1_700_000_000,
            },
            days: vec![],
            fetched_at: 1_700_000_100,
            tz_offset: 25200,
            source: "OpenWeather".into(),
        }
    }

    fn entries() -> Vec<Entry> {
        vec![
            Entry { source_id: "openweather".into(), snapshot: snapshot() },
            Entry { source_id: "open-meteo:gfs_seamless".into(), snapshot: snapshot() },
        ]
    }

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-weather-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn roundtrip_keeps_every_source() {
        let path = temp_path("ok.json");
        save(&path, &entries()).unwrap();
        assert_eq!(load(&path), entries());
    }

    #[test]
    fn missing_corrupt_or_old_format_file_is_empty() {
        assert!(load(&temp_path("khong-co.json")).is_empty());
        let bad = temp_path("hong.json");
        std::fs::write(&bad, "{ không phải json").unwrap();
        assert!(load(&bad).is_empty());
        // Định dạng cũ: một Snapshot đơn lẻ thay vì danh sách.
        let old = temp_path("cu.json");
        std::fs::write(&old, serde_json::to_string(&snapshot()).unwrap()).unwrap();
        assert!(load(&old).is_empty());
    }
}
