use std::path::Path;

use crate::provider::Snapshot;

pub fn save(path: &Path, snapshot: &Snapshot) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, serde_json::to_string(snapshot)?)
}

/// `None` nếu chưa có cache hoặc file hỏng (không làm app dừng).
pub fn load(path: &Path) -> Option<Snapshot> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
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

    fn temp_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-weather-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn roundtrip_keeps_snapshot() {
        let path = temp_path("ok.json");
        save(&path, &snapshot()).unwrap();
        assert_eq!(load(&path), Some(snapshot()));
    }

    #[test]
    fn missing_or_corrupt_file_is_none() {
        assert_eq!(load(&temp_path("khong-co.json")), None);
        let bad = temp_path("hong.json");
        std::fs::write(&bad, "{ không phải json").unwrap();
        assert_eq!(load(&bad), None);
    }
}
