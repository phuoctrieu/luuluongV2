//! Nguồn thời tiết người dùng chọn trong Cài đặt, lưu dạng chuỗi trong `config.toml`.

use crate::openmeteo::Model;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    OpenWeather,
    OpenMeteo(Model),
}

impl Source {
    pub const ALL: [Source; 5] = [
        Source::OpenWeather,
        Source::OpenMeteo(Model::BestMatch),
        Source::OpenMeteo(Model::Gfs),
        Source::OpenMeteo(Model::Ecmwf),
        Source::OpenMeteo(Model::Icon),
    ];

    /// Chuỗi lưu trong cấu hình.
    pub fn id(self) -> String {
        match self {
            Source::OpenWeather => "openweather".into(),
            Source::OpenMeteo(m) => format!("open-meteo:{}", m.id()),
        }
    }

    pub fn from_id(id: &str) -> Option<Source> {
        Source::ALL.into_iter().find(|s| s.id() == id)
    }

    /// Tên hiển thị trong Cài đặt.
    pub fn label(self) -> String {
        match self {
            Source::OpenWeather => "OpenWeather (cần API key)".into(),
            Source::OpenMeteo(m) => format!("Open-Meteo · {}", m.label()),
        }
    }

    pub fn needs_key(self) -> bool {
        self == Source::OpenWeather
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_for_every_source() {
        for s in Source::ALL {
            assert_eq!(Source::from_id(&s.id()), Some(s), "{s:?}");
        }
    }

    #[test]
    fn stable_ids_in_config_files() {
        assert_eq!(Source::OpenWeather.id(), "openweather");
        assert_eq!(Source::OpenMeteo(Model::Gfs).id(), "open-meteo:gfs_seamless");
    }

    #[test]
    fn unknown_id_is_none() {
        assert_eq!(Source::from_id("noaa"), None);
        assert_eq!(Source::from_id(""), None);
        assert_eq!(Source::from_id("open-meteo:khong-co"), None);
    }

    #[test]
    fn labels_say_where_data_comes_from() {
        assert_eq!(Source::OpenWeather.label(), "OpenWeather (cần API key)");
        assert_eq!(Source::OpenMeteo(Model::Gfs).label(), "Open-Meteo · NOAA GFS");
        assert!(!Source::OpenMeteo(Model::Icon).needs_key());
    }
}
