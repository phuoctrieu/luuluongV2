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
    pub source: Source,
    /// `system` | `light` | `dark`
    pub theme: String,
    pub operator: String,
}

impl SettingsForm {
    pub fn from_config(c: &Config) -> SettingsForm {
        SettingsForm {
            q_h1: vn(c.turbine.q_h1),
            q_h2: vn(c.turbine.q_h2),
            lat: vn(c.weather.lat),
            lon: vn(c.weather.lon),
            refresh_minutes: c.weather.refresh_minutes.to_string(),
            rain_alert_mm: vn(c.weather.rain_alert_mm),
            source: Source::from_id(&c.weather.source).unwrap_or(Source::OpenMeteo(luuluong_weather::openmeteo::Model::BestMatch)),
            theme: c.ui.theme.clone(),
            operator: c.ui.operator.clone(),
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
        c.weather.source = self.source.id();
        c.ui.theme = self.theme.clone();
        c.ui.operator = self.operator.trim().to_string();
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
