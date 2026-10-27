//! Open-Meteo: không cần API key, gom nhiều mô hình (NOAA GFS, ECMWF, DWD ICON) trong một lần gọi.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

use crate::error::WeatherError;
use crate::provider::{read_json, Current, Day, ModelRain, Snapshot, WeatherProvider};

const BASE_URL: &str = "https://api.open-meteo.com/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model {
    BestMatch,
    /// NOAA GFS (Mỹ). `api.weather.gov` không phủ Việt Nam nên lấy GFS qua Open-Meteo.
    Gfs,
    Ecmwf,
    Icon,
}

impl Model {
    pub const ALL: [Model; 4] = [Model::BestMatch, Model::Gfs, Model::Ecmwf, Model::Icon];

    pub fn id(self) -> &'static str {
        match self {
            Model::BestMatch => "best_match",
            Model::Gfs => "gfs_seamless",
            Model::Ecmwf => "ecmwf_ifs025",
            Model::Icon => "icon_seamless",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Model::BestMatch => "Tự chọn tốt nhất",
            Model::Gfs => "NOAA GFS",
            Model::Ecmwf => "ECMWF",
            Model::Icon => "DWD ICON",
        }
    }
}

/// Các mô hình luôn được so sánh lượng mưa.
const COMPARE: [Model; 3] = [Model::Gfs, Model::Ecmwf, Model::Icon];

pub struct OpenMeteoProvider {
    model: Model,
    base_url: String,
    agent: ureq::Agent,
}

impl OpenMeteoProvider {
    pub fn new(model: Model) -> Self {
        Self { model, base_url: BASE_URL.into(), agent: ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).build() }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.agent = ureq::AgentBuilder::new().timeout(timeout).build();
        self
    }
}

/// Mô tả tiếng Việt cho mã thời tiết WMO.
pub fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Trời quang",
        1 => "Ít mây",
        2 => "Mây rải rác",
        3 => "Nhiều mây",
        45 | 48 => "Sương mù",
        51 => "Mưa phùn nhẹ",
        53 => "Mưa phùn vừa",
        55 => "Mưa phùn dày",
        56 | 57 => "Mưa phùn lạnh",
        61 => "Mưa nhẹ",
        63 => "Mưa vừa",
        65 => "Mưa to",
        66 | 67 => "Mưa lạnh",
        71 | 73 | 75 | 77 | 85 | 86 => "Tuyết",
        80 => "Mưa rào nhẹ",
        81 => "Mưa rào vừa",
        82 => "Mưa rào rất to",
        95 => "Dông",
        96 | 99 => "Dông kèm mưa đá",
        _ => "Không rõ",
    }
}

#[derive(Deserialize)]
struct Resp {
    utc_offset_seconds: i32,
    current: Cur,
    daily: Daily,
}

#[derive(Deserialize)]
struct Cur {
    time: String,
    temperature_2m: f64,
    relative_humidity_2m: f64,
    wind_speed_10m: f64,
    weather_code: i64,
}

/// Nhiều mô hình: khóa dạng `<biến>_<mô hình>`, giá trị có thể `null`.
#[derive(Deserialize)]
struct Daily {
    time: Vec<NaiveDate>,
    #[serde(flatten)]
    series: HashMap<String, Vec<Option<f64>>>,
}

impl Daily {
    fn value(&self, name: &str, model: Model, day: usize) -> Option<f64> {
        self.series.get(&format!("{name}_{}", model.id()))?.get(day).copied().flatten()
    }
}

impl WeatherProvider for OpenMeteoProvider {
    fn fetch(&self, lat: f64, lon: f64) -> Result<Snapshot, WeatherError> {
        let mut models = vec![self.model];
        models.extend(COMPARE.iter().copied().filter(|m| *m != self.model));
        let models = models.iter().map(|m| m.id()).collect::<Vec<_>>().join(",");
        let response = self
            .agent
            .get(&format!("{}/forecast", self.base_url))
            .query("latitude", &lat.to_string())
            .query("longitude", &lon.to_string())
            .query("timezone", "auto")
            .query("wind_speed_unit", "ms")
            .query("forecast_days", "5")
            .query("models", &models)
            .query("current", "temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code")
            .query(
                "daily",
                "temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,weather_code",
            )
            .call();
        let resp: Resp = read_json(response)?;
        let local = NaiveDateTime::parse_from_str(&resp.current.time, "%Y-%m-%dT%H:%M").map_err(|_| WeatherError::BadResponse)?;
        let days = resp
            .daily
            .time
            .iter()
            .enumerate()
            .map(|(i, date)| {
                let d = &resp.daily;
                let need = |name: &str| d.value(name, self.model, i).ok_or(WeatherError::BadResponse);
                Ok(Day {
                    date: *date,
                    temp_min: need("temperature_2m_min")?,
                    temp_max: need("temperature_2m_max")?,
                    rain_mm: need("precipitation_sum")?,
                    pop_percent: d.value("precipitation_probability_max", self.model, i).unwrap_or(0.0),
                    description: wmo_description(d.value("weather_code", self.model, i).map_or(-1, |c| c as i64)).into(),
                    icon: String::new(),
                    partial: false,
                    model_rain: COMPARE
                        .iter()
                        .filter_map(|m| {
                            d.value("precipitation_sum", *m, i).map(|mm| ModelRain { model: m.label().into(), rain_mm: mm })
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, WeatherError>>()?;
        let fetched_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        Ok(Snapshot {
            current: Current {
                temp: resp.current.temperature_2m,
                humidity: resp.current.relative_humidity_2m,
                wind_speed: resp.current.wind_speed_10m,
                description: wmo_description(resp.current.weather_code).into(),
                icon: String::new(),
                observed_at: local.and_utc().timestamp() - resp.utc_offset_seconds as i64,
            },
            days,
            fetched_at,
            tz_offset: resp.utc_offset_seconds,
            source: format!("Open-Meteo · {}", self.model.label()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    /// Rút gọn từ phản hồi thật của Open-Meteo (nhiều mô hình: khóa có hậu tố `_<mô hình>`).
    const BODY: &str = r#"{"utc_offset_seconds":25200,
      "current":{"time":"2026-10-27T10:30","temperature_2m":25.2,"relative_humidity_2m":77,"wind_speed_10m":2.24,"weather_code":53},
      "daily":{"time":["2026-10-27","2026-10-28"],
        "temperature_2m_max_best_match":[25.7,23.0],"temperature_2m_min_best_match":[18.5,20.4],
        "precipitation_sum_best_match":[7.0,40.9],"precipitation_probability_max_best_match":[99,100],
        "weather_code_best_match":[80,95],
        "temperature_2m_max_gfs_seamless":[27.6,25.8],"temperature_2m_min_gfs_seamless":[18.6,19.9],
        "precipitation_sum_gfs_seamless":[0.9,6.0],"precipitation_probability_max_gfs_seamless":[100,100],
        "weather_code_gfs_seamless":[51,55],
        "precipitation_sum_ecmwf_ifs025":[8.8,44.5],
        "precipitation_sum_icon_seamless":[0.9,null]}}"#;

    fn mock(server: &mut mockito::Server, models: &str) -> mockito::Mock {
        server
            .mock("GET", "/forecast")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("latitude".into(), "16.22".into()),
                Matcher::UrlEncoded("longitude".into(), "107.25".into()),
                Matcher::UrlEncoded("timezone".into(), "auto".into()),
                Matcher::UrlEncoded("wind_speed_unit".into(), "ms".into()),
                Matcher::UrlEncoded("forecast_days".into(), "5".into()),
                Matcher::UrlEncoded("models".into(), models.into()),
            ]))
            .with_body(BODY)
            .create()
    }

    fn fetch(server: &mockito::Server, model: Model) -> Result<Snapshot, WeatherError> {
        OpenMeteoProvider::new(model).with_base_url(server.url()).fetch(16.22, 107.25)
    }

    #[test]
    fn default_endpoint_is_https() {
        assert!(OpenMeteoProvider::new(Model::BestMatch).base_url.starts_with("https://"));
    }

    #[test]
    fn model_ids_match_the_live_api() {
        let ids: Vec<_> = Model::ALL.iter().map(|m| m.id()).collect();
        assert_eq!(ids, ["best_match", "gfs_seamless", "ecmwf_ifs025", "icon_seamless"]);
    }

    #[test]
    fn best_match_is_primary_and_other_models_compare_rain() {
        let mut server = mockito::Server::new();
        let m = mock(&mut server, "best_match,gfs_seamless,ecmwf_ifs025,icon_seamless");
        let s = fetch(&server, Model::BestMatch).unwrap();
        m.assert();
        assert_eq!(s.source, "Open-Meteo · Tự chọn tốt nhất");
        assert_eq!(s.tz_offset, 25200);
        assert_eq!(s.days.len(), 2);
        let d = &s.days[1];
        assert_eq!((d.temp_min, d.temp_max, d.rain_mm, d.pop_percent), (20.4, 23.0, 40.9, 100.0));
        assert_eq!(d.description, "Dông");
        assert!(!d.partial);
        // Ngày 2: ICON null nên bị bỏ; còn GFS và ECMWF.
        let rain: Vec<_> = d.model_rain.iter().map(|r| (r.model.as_str(), r.rain_mm)).collect();
        assert_eq!(rain, vec![("NOAA GFS", 6.0), ("ECMWF", 44.5)]);
        assert_eq!(s.days[0].model_rain.len(), 3);
    }

    #[test]
    fn current_conditions_use_wmo_text_and_local_time() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, "best_match,gfs_seamless,ecmwf_ifs025,icon_seamless");
        let c = fetch(&server, Model::BestMatch).unwrap().current;
        assert_eq!((c.temp, c.humidity, c.wind_speed), (25.2, 77.0, 2.24));
        assert_eq!(c.description, "Mưa phùn vừa");
        // 10:30 giờ địa phương (UTC+7) = 03:30 UTC
        let utc = "2026-10-27".parse::<chrono::NaiveDate>().unwrap().and_hms_opt(3, 30, 0).unwrap().and_utc().timestamp();
        assert_eq!(c.observed_at, utc);
    }

    #[test]
    fn noaa_gfs_as_primary_uses_its_own_values() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, "gfs_seamless,ecmwf_ifs025,icon_seamless");
        let s = fetch(&server, Model::Gfs).unwrap();
        assert_eq!(s.source, "Open-Meteo · NOAA GFS");
        assert_eq!(s.days[1].rain_mm, 6.0);
        assert_eq!(s.days[1].temp_max, 25.8);
    }

    #[test]
    fn missing_primary_values_are_a_bad_response() {
        let mut server = mockito::Server::new();
        // Mô hình chính là ECMWF nhưng phản hồi không có nhiệt độ của nó.
        let _m = mock(&mut server, "ecmwf_ifs025,gfs_seamless,icon_seamless");
        assert_eq!(fetch(&server, Model::Ecmwf), Err(WeatherError::BadResponse));
    }

    #[test]
    fn maps_http_errors_and_bad_json() {
        for (status, expect) in [(429, WeatherError::RateLimited), (500, WeatherError::Unavailable), (400, WeatherError::Unavailable)] {
            let mut server = mockito::Server::new();
            let _m = server.mock("GET", Matcher::Any).with_status(status).with_body("{}").create();
            assert_eq!(fetch(&server, Model::BestMatch), Err(expect), "{status}");
        }
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_body(r#"{"hello":1}"#).create();
        assert_eq!(fetch(&server, Model::BestMatch), Err(WeatherError::BadResponse));
    }

    #[test]
    fn no_network_is_unavailable() {
        let p = OpenMeteoProvider::new(Model::BestMatch).with_base_url("http://127.0.0.1:1");
        assert_eq!(p.fetch(16.22, 107.25), Err(WeatherError::Unavailable));
    }

    #[test]
    fn wmo_codes_in_vietnamese() {
        assert_eq!(wmo_description(0), "Trời quang");
        assert_eq!(wmo_description(3), "Nhiều mây");
        assert_eq!(wmo_description(65), "Mưa to");
        assert_eq!(wmo_description(95), "Dông");
        assert_eq!(wmo_description(12345), "Không rõ");
    }

    /// Gọi API thật: `cargo test -p luuluong-weather -- --ignored live`.
    #[test]
    #[ignore]
    fn live_open_meteo_parses_for_every_model() {
        for model in Model::ALL {
            let s = OpenMeteoProvider::new(model).fetch(16.22, 107.25).unwrap_or_else(|e| panic!("{model:?}: {e}"));
            assert_eq!(s.days.len(), 5, "{model:?}");
            assert_eq!(s.tz_offset, 25200);
            assert_eq!(s.days[0].model_rain.len(), 3, "{model:?}");
            println!("{}: {:?}", s.source, s.days.iter().map(|d| d.rain_mm).collect::<Vec<_>>());
        }
    }
}
