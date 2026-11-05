//! Biểu mẫu Cài đặt: chuỗi người dùng gõ <-> `Config`, lỗi theo từng ô.

use luuluong_core::validate::parse_vn_number;

use crate::view::vn;
use luuluong_storage::config::Config;
use luuluong_weather::Source;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsField {
    QH1,
    QH2,
    Lat,
    Lon,
    RefreshMinutes,
    RainAlert,
    LoggerUrl,
    LoggerPoll,
    VrainUsername,
    VrainRefresh,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsError {
    pub field: SettingsField,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SettingsForm {
    pub q_h1: String,
    pub q_h2: String,
    pub lat: String,
    pub lon: String,
    pub refresh_minutes: String,
    pub rain_alert_mm: String,
    pub logger_url: String,
    pub logger_poll: String,
    pub vrain_enabled: bool,
    pub vrain_username: String,
    pub vrain_refresh: String,
    pub update_auto_check: bool,
    /// Nguồn chính (thẻ ngày).
    pub source: Source,
    /// Các nguồn bật để so sánh; nguồn chính luôn được tính vào.
    pub compare: Vec<Source>,
    /// `system` | `light` | `dark`
    pub theme: String,
    pub operator: String,
    pub animations: bool,
}

impl SettingsForm {
    pub fn from_config(c: &Config) -> SettingsForm {
        let source = Source::from_id(&c.weather.source).unwrap_or(Source::OpenMeteo(luuluong_weather::openmeteo::Model::BestMatch));
        SettingsForm {
            q_h1: vn(c.turbine.q_h1),
            q_h2: vn(c.turbine.q_h2),
            lat: vn(c.weather.lat),
            lon: vn(c.weather.lon),
            refresh_minutes: c.weather.refresh_minutes.to_string(),
            rain_alert_mm: vn(c.weather.rain_alert_mm),
            logger_url: c.logger.url.clone(),
            logger_poll: c.logger.poll_seconds.to_string(),
            vrain_enabled: c.vrain.enabled,
            vrain_username: c.vrain.username.clone(),
            vrain_refresh: c.vrain.refresh_minutes.to_string(),
            update_auto_check: c.update.auto_check,
            source,
            compare: c.weather.sources.iter().filter_map(|id| Source::from_id(id)).collect(),
            theme: c.ui.theme.clone(),
            operator: c.ui.operator.clone(),
            animations: c.ui.animations,
        }
    }

    /// Áp các ô lên `base` (giữ nguyên những mục không có trong biểu mẫu).
    pub fn to_config(&self, base: &Config) -> Result<Config, Vec<SettingsError>> {
        let mut errors = Vec::new();
        let mut number = |field: SettingsField, raw: &str, rule: &dyn Fn(f64) -> Option<&'static str>| {
            let value = match parse_vn_number(raw) {
                Ok(x) if x.is_finite() => x,
                Ok(_) => {
                    errors.push(SettingsError { field, message: luuluong_core::CalcError::InvalidNumber.to_string() });
                    return 0.0;
                }
                Err(e) => {
                    errors.push(SettingsError { field, message: e.to_string() });
                    return 0.0;
                }
            };
            if let Some(message) = rule(value) {
                errors.push(SettingsError { field, message: message.into() });
            }
            value
        };
        let positive = |x: f64| (x <= 0.0).then_some("Lưu lượng phải lớn hơn 0");
        let q_h1 = number(SettingsField::QH1, &self.q_h1, &positive);
        let q_h2 = number(SettingsField::QH2, &self.q_h2, &positive);
        let lat = number(SettingsField::Lat, &self.lat, &|x| (!(-90.0..=90.0).contains(&x)).then_some("Vĩ độ phải từ -90 đến 90"));
        let lon = number(SettingsField::Lon, &self.lon, &|x| (!(-180.0..=180.0).contains(&x)).then_some("Kinh độ phải từ -180 đến 180"));
        let refresh = number(SettingsField::RefreshMinutes, &self.refresh_minutes, &|x| {
            (x.fract() != 0.0 || !(5.0..=1440.0).contains(&x)).then_some("Nhập số phút nguyên từ 5 đến 1440")
        });
        let rain = number(SettingsField::RainAlert, &self.rain_alert_mm, &|x| (x <= 0.0).then_some("Ngưỡng mưa phải lớn hơn 0"));
        let poll = number(SettingsField::LoggerPoll, &self.logger_poll, &|x| {
            (x.fract() != 0.0 || !(1.0..=60.0).contains(&x)).then_some("Nhập số giây nguyên từ 1 đến 60")
        });
        let vrain_refresh = number(SettingsField::VrainRefresh, &self.vrain_refresh, &|x| {
            (x.fract() != 0.0 || !(5.0..=1440.0).contains(&x)).then_some("Nhập số phút nguyên từ 5 đến 1440")
        });
        if self.vrain_enabled && self.vrain_username.trim().is_empty() {
            errors.push(SettingsError { field: SettingsField::VrainUsername, message: "Vui lòng nhập tài khoản Vrain.".into() });
        }
        let url = self.logger_url.trim();
        if !(url.starts_with("http://") || url.starts_with("https://")) {
            errors.push(SettingsError { field: SettingsField::LoggerUrl, message: "Địa chỉ phải bắt đầu bằng http:// hoặc https://".into() });
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut c = base.clone();
        c.turbine.q_h1 = q_h1;
        c.turbine.q_h2 = q_h2;
        c.weather.lat = lat;
        c.weather.lon = lon;
        c.weather.refresh_minutes = refresh as u32;
        c.weather.rain_alert_mm = rain;
        c.logger.url = url.to_string();
        c.logger.poll_seconds = poll as u32;
        c.update.auto_check = self.update_auto_check; // last_check giữ nguyên từ `base`
        c.vrain.enabled = self.vrain_enabled;
        c.vrain.username = self.vrain_username.trim().to_string();
        c.vrain.refresh_minutes = vrain_refresh as u32;
        c.weather.source = self.source.id();
        // Thứ tự cố định theo `Source::ALL`; nguồn chính luôn có mặt.
        c.weather.sources = Source::ALL.iter().filter(|s| **s == self.source || self.compare.contains(s)).map(|s| s.id()).collect();
        c.ui.theme = self.theme.clone();
        c.ui.operator = self.operator.trim().to_string();
        c.ui.animations = self.animations;
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form() -> SettingsForm {
        SettingsForm::from_config(&Config::default())
    }

    fn errors(f: &SettingsForm) -> Vec<(SettingsField, String)> {
        f.to_config(&Config::default()).unwrap_err().into_iter().map(|e| (e.field, e.message)).collect()
    }

    #[test]
    fn default_config_roundtrips_through_the_form() {
        assert_eq!(form().to_config(&Config::default()).unwrap(), Config::default());
    }

    #[test]
    fn form_shows_vietnamese_decimals() {
        let f = form();
        assert_eq!((f.q_h1.as_str(), f.lat.as_str(), f.refresh_minutes.as_str()), ("17,3", "16,22", "60"));
    }

    #[test]
    fn accepts_comma_and_dot_and_keeps_untouched_sections() {
        let mut base = Config::default();
        base.reservoir.volume_table = "C:/bang.csv".into();
        let mut f = SettingsForm::from_config(&base);
        f.q_h1 = "18.5".into();
        f.q_h2 = "19,25".into();
        f.operator = "Linh".into();
        f.source = Source::from_id("open-meteo:gfs_seamless").unwrap();
        f.theme = "dark".into();
        let c = f.to_config(&base).unwrap();
        assert_eq!((c.turbine.q_h1, c.turbine.q_h2), (18.5, 19.25));
        assert_eq!((c.ui.operator.as_str(), c.ui.theme.as_str()), ("Linh", "dark"));
        assert_eq!(c.weather.source, "open-meteo:gfs_seamless");
        assert_eq!(c.reservoir.volume_table, "C:/bang.csv");
    }

    #[test]
    fn animations_toggle_roundtrips() {
        let mut f = form();
        assert!(f.animations);
        f.animations = false;
        assert!(!f.to_config(&Config::default()).unwrap().ui.animations);
    }

    #[test]
    fn primary_source_is_always_among_the_compared_ones() {
        let mut f = form();
        f.source = Source::OpenWeather;
        f.compare = vec![];
        assert_eq!(f.to_config(&Config::default()).unwrap().weather.sources, vec!["openweather"]);
    }

    #[test]
    fn compared_sources_keep_a_canonical_order() {
        let mut f = form();
        f.source = Source::from_id("open-meteo:icon_seamless").unwrap();
        f.compare = vec![Source::from_id("open-meteo:gfs_seamless").unwrap(), Source::OpenWeather];
        let c = f.to_config(&Config::default()).unwrap();
        assert_eq!(c.weather.sources, vec!["openweather", "open-meteo:gfs_seamless", "open-meteo:icon_seamless"]);
    }

    #[test]
    fn turbine_flow_must_be_positive() {
        let mut f = form();
        f.q_h1 = "0".into();
        f.q_h2 = "-3".into();
        let e = errors(&f);
        assert_eq!(e[0], (SettingsField::QH1, "Lưu lượng phải lớn hơn 0".into()));
        assert_eq!(e[1].0, SettingsField::QH2);
    }

    #[test]
    fn coordinates_must_be_on_earth() {
        let mut f = form();
        f.lat = "91".into();
        f.lon = "-181".into();
        let e = errors(&f);
        assert_eq!(e[0], (SettingsField::Lat, "Vĩ độ phải từ -90 đến 90".into()));
        assert_eq!(e[1], (SettingsField::Lon, "Kinh độ phải từ -180 đến 180".into()));
    }

    #[test]
    fn refresh_is_a_whole_number_of_at_least_five_minutes() {
        for bad in ["4", "10,5", "abc", ""] {
            let mut f = form();
            f.refresh_minutes = bad.into();
            assert_eq!(errors(&f)[0].0, SettingsField::RefreshMinutes, "{bad:?}");
        }
        let mut f = form();
        f.refresh_minutes = "5".into();
        assert!(f.to_config(&Config::default()).is_ok());
    }

    #[test]
    fn logger_settings_roundtrip_and_keep_the_period() {
        let mut base = Config::default();
        base.logger.period_minutes = 30;
        let mut f = SettingsForm::from_config(&base);
        assert_eq!(f.logger_poll, "5");
        f.logger_url = " http://10.0.0.5:8880/macros/UpdateMonitor ".into();
        f.logger_poll = "10".into();
        let c = f.to_config(&base).unwrap();
        assert_eq!(
            c.logger,
            luuluong_storage::config::Logger { url: "http://10.0.0.5:8880/macros/UpdateMonitor".into(), poll_seconds: 10, period_minutes: 30 }
        );
    }

    #[test]
    fn logger_url_must_be_http() {
        for bad in ["", "192.168.200.211", "ftp://x"] {
            let mut f = form();
            f.logger_url = bad.into();
            assert_eq!(errors(&f).iter().map(|e| e.0).collect::<Vec<_>>(), [SettingsField::LoggerUrl], "{bad:?}");
        }
    }

    #[test]
    fn logger_poll_is_whole_seconds() {
        for bad in ["0", "2,5", "61"] {
            let mut f = form();
            f.logger_poll = bad.into();
            assert_eq!(errors(&f).iter().map(|e| e.0).collect::<Vec<_>>(), [SettingsField::LoggerPoll], "{bad:?}");
        }
    }

    #[test]
    fn vrain_requires_username_when_enabled() {
        let mut f = form();
        f.vrain_username = "   ".into();
        assert!(f.to_config(&Config::default()).is_ok(), "tắt Vrain thì không đòi tài khoản");
        f.vrain_enabled = true;
        assert!(errors(&f).contains(&(SettingsField::VrainUsername, "Vui lòng nhập tài khoản Vrain.".to_string())));
        f.vrain_username = " user1 ".into();
        assert_eq!(f.to_config(&Config::default()).unwrap().vrain.username, "user1");
    }

    #[test]
    fn vrain_refresh_minutes_range_5_to_1440() {
        for bad in ["4", "1441", "5,5", "abc", ""] {
            let mut f = form();
            f.vrain_refresh = bad.into();
            assert!(errors(&f).iter().any(|e| e.0 == SettingsField::VrainRefresh), "{bad:?} phải bị từ chối");
        }
        for good in ["5", "60", "1440"] {
            let mut f = form();
            f.vrain_refresh = good.into();
            assert_eq!(f.to_config(&Config::default()).unwrap().vrain.refresh_minutes, good.parse::<u32>().unwrap());
        }
    }

    #[test]
    fn vrain_settings_roundtrip_and_keep_stations() {
        let mut f = form();
        f.vrain_enabled = true;
        f.vrain_username = "user1".into();
        f.vrain_refresh = "30".into();
        let c = f.to_config(&Config::default()).unwrap();
        assert!(c.vrain.enabled);
        assert_eq!((c.vrain.username.as_str(), c.vrain.refresh_minutes), ("user1", 30));
        assert_eq!(c.vrain.stations, Config::default().vrain.stations);
        assert_eq!(SettingsForm::from_config(&c), f);
    }

    #[test]
    fn settings_roundtrip_keeps_last_check() {
        let mut base = Config::default();
        base.update.last_check = 1_790_000_123;
        let mut f = SettingsForm::from_config(&base);
        assert!(f.update_auto_check);
        f.update_auto_check = false;
        let c = f.to_config(&base).unwrap();
        assert!(!c.update.auto_check);
        assert_eq!(c.update.last_check, 1_790_000_123, "lần kiểm tra gần nhất không bị biểu mẫu xóa");
    }

    #[test]
    fn config_file_never_contains_a_password() {
        let mut c = Config::default();
        c.vrain.enabled = true;
        c.vrain.username = "user1".into();
        let path = std::env::temp_dir().join(format!("luuluong-vrain-cfg-{}.toml", std::process::id()));
        c.save(&path).unwrap();
        let text = std::fs::read_to_string(&path).unwrap().to_lowercase();
        let _ = std::fs::remove_file(&path);
        assert!(text.contains("user1"));
        assert!(!text.contains("password") && !text.contains("mat_khau"), "config không được có mật khẩu");
    }

    #[test]
    fn rain_threshold_must_be_positive() {
        let mut f = form();
        f.rain_alert_mm = "0".into();
        assert_eq!(errors(&f)[0].0, SettingsField::RainAlert);
    }

    #[test]
    fn non_numbers_get_the_standard_message() {
        let mut f = form();
        f.lat = "mười sáu".into();
        assert_eq!(errors(&f)[0].1, "Vui lòng nhập số, ví dụ 160,5");
    }
}
