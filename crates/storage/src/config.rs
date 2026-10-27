//! `config.toml` (đặc tả mục 4). Thiếu khóa thì dùng mặc định; API key KHÔNG nằm ở đây.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub reservoir: Reservoir,
    pub turbine: Turbine,
    pub weather: Weather,
    pub ui: Ui,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Reservoir {
    pub z_min: f64,
    pub z_max: f64,
    /// `"builtin"` hoặc đường dẫn tới file CSV/XLSX.
    pub volume_table: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Turbine {
    pub q_h1: f64,
    pub q_h2: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Weather {
    pub lat: f64,
    pub lon: f64,
    pub units: String,
    pub lang: String,
    pub refresh_minutes: u32,
    /// Mã nguồn, xem `luuluong_weather::Source::id`.
    pub source: String,
    /// Ngưỡng cảnh báo mưa lớn (mm/ngày).
    pub rain_alert_mm: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ui {
    /// `system` | `light` | `dark`
    pub theme: String,
    /// Tên người vận hành, ghi vào mỗi dòng lịch sử (tùy chọn).
    pub operator: String,
}

impl Default for Reservoir {
    fn default() -> Self {
        Self { z_min: 149.0, z_max: 165.0, volume_table: "builtin".into() }
    }
}

impl Default for Turbine {
    fn default() -> Self {
        Self { q_h1: 17.3, q_h2: 17.3 }
    }
}

impl Default for Weather {
    fn default() -> Self {
        Self {
            lat: 16.22,
            lon: 107.25,
            units: "metric".into(),
            lang: "vi".into(),
            refresh_minutes: 60,
            source: "open-meteo:best_match".into(),
            rain_alert_mm: 50.0,
        }
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self { theme: "system".into(), operator: String::new() }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Không đọc được file cấu hình, đang dùng giá trị mặc định.")]
    Invalid,
    #[error("Không lưu được cấu hình.")]
    Write(#[from] std::io::Error),
}

impl Config {
    /// File chưa có là bình thường (dùng mặc định); file hỏng trả lỗi để app cảnh báo.
    pub fn load(path: &Path) -> Result<Config, ConfigError> {
        match std::fs::read_to_string(path) {
            Ok(text) => toml::from_str(&text).map_err(|_| ConfigError::Invalid),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
            Err(_) => Err(ConfigError::Invalid),
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = toml::to_string_pretty(self).map_err(|_| ConfigError::Invalid)?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("luuluong-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn defaults_match_the_specification() {
        let c = Config::default();
        assert_eq!((c.reservoir.z_min, c.reservoir.z_max), (149.0, 165.0));
        assert_eq!(c.reservoir.volume_table, "builtin");
        assert_eq!((c.turbine.q_h1, c.turbine.q_h2), (17.3, 17.3));
        assert_eq!((c.weather.lat, c.weather.lon, c.weather.refresh_minutes), (16.22, 107.25, 60));
        assert_eq!(c.weather.rain_alert_mm, 50.0);
        assert_eq!(c.ui.theme, "system");
    }

    #[test]
    fn missing_file_gives_defaults() {
        assert_eq!(Config::load(&temp("khong-co.toml")).unwrap(), Config::default());
    }

    #[test]
    fn roundtrip_keeps_changes() {
        let path = temp("ok.toml");
        let mut c = Config::default();
        c.turbine.q_h1 = 18.5;
        c.ui.operator = "Linh".into();
        c.weather.source = "open-meteo:gfs_seamless".into();
        c.save(&path).unwrap();
        assert_eq!(Config::load(&path).unwrap(), c);
    }

    #[test]
    fn partial_file_fills_missing_keys_with_defaults() {
        let path = temp("partial.toml");
        std::fs::write(&path, "[turbine]\nq_h1 = 20.0\n").unwrap();
        let c = Config::load(&path).unwrap();
        assert_eq!((c.turbine.q_h1, c.turbine.q_h2), (20.0, 17.3));
        assert_eq!(c.weather, Weather::default());
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let path = temp("hong.toml");
        std::fs::write(&path, "[turbine\nq_h1 = ???").unwrap();
        assert!(matches!(Config::load(&path), Err(ConfigError::Invalid)));
    }

    #[test]
    fn saved_file_never_contains_an_api_key_field() {
        let path = temp("nokey.toml");
        Config::default().save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
        assert!(!text.contains("key") && !text.contains("appid"));
    }
}
