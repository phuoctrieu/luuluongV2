//! Open-Meteo: không cần API key, gom nhiều mô hình (NOAA GFS, ECMWF, DWD ICON) trong một lần gọi.

use std::collections::HashMap;
use std::time::Duration;

use chrono::{NaiveDate, NaiveDateTime};
use serde::Deserialize;

use crate::error::WeatherError;
use crate::provider::{read_json, Current, Day, Snapshot, WeatherProvider};

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
            Model::BestMatch => "Tổng hợp",
            Model::Gfs => "NOAA GFS",
            Model::Ecmwf => "ECMWF",
            Model::Icon => "DWD ICON",
        }
    }
}

pub struct OpenMeteoProvider {
    model: Model,
    base_url: String,
    agent: ureq::Agent,
}

/// Kết quả từng mô hình; một mô hình thiếu dữ liệu không làm mất các mô hình còn lại.
pub type ModelResults = Vec<(Model, Result<Snapshot, WeatherError>)>;

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

    /// Một lần gọi lấy nhiều mô hình. Lỗi mạng/HTTP/JSON trả ở ngoài (áp dụng cho mọi mô hình).
    /// Thời tiết "hiện tại" do Open-Meteo trả chung một bộ cho cả lần gọi.
    pub fn fetch_models(&self, lat: f64, lon: f64, models: &[Model]) -> Result<ModelResults, WeatherError> {
        if models.is_empty() {
            return Ok(Vec::new());
        }
        let ids = models.iter().map(|m| m.id()).collect::<Vec<_>>().join(",");
        let response = self
            .agent
            .get(&format!("{}/forecast", self.base_url))
            .query("latitude", &lat.to_string())
            .query("longitude", &lon.to_string())
            .query("timezone", "auto")
            .query("wind_speed_unit", "ms")
            .query("forecast_days", "5")
            .query("models", &ids)
            .query("current", "temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code")
            .query(
                "daily",
                "temperature_2m_max,temperature_2m_min,precipitation_sum,precipitation_probability_max,weather_code",
            )
            .call();
        let resp: Resp = read_json(response)?;
        let local = NaiveDateTime::parse_from_str(&resp.current.time, "%Y-%m-%dT%H:%M").map_err(|_| WeatherError::BadResponse)?;
        let current = Current {
            temp: resp.current.temperature_2m,
            humidity: resp.current.relative_humidity_2m,
            wind_speed: resp.current.wind_speed_10m,
            description: wmo_description(resp.current.weather_code).into(),
            icon: String::new(),
            observed_at: local.and_utc().timestamp() - resp.utc_offset_seconds as i64,
        };
        let fetched_at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        Ok(models
            .iter()
            .map(|m| {
                let snapshot = days_for(&resp.daily, *m).map(|days| Snapshot {
                    current: current.clone(),
                    days,
                    fetched_at,
                    tz_offset: resp.utc_offset_seconds,
                    source: format!("Open-Meteo · {}", m.label()),
                });
                (*m, snapshot)
            })
            .collect())
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
    /// Nhiều mô hình: khóa `<biến>_<mô hình>`; một mô hình: khóa không hậu tố.
    fn value(&self, name: &str, model: Model, day: usize) -> Option<f64> {
        let series = self.series.get(&format!("{name}_{}", model.id())).or_else(|| self.series.get(name))?;
        series.get(day).copied().flatten()
    }
}

/// Gộp dữ liệu ngày của một mô hình; thiếu giá trị bắt buộc thì cả mô hình đó lỗi.
fn days_for(d: &Daily, model: Model) -> Result<Vec<Day>, WeatherError> {
    d.time
        .iter()
        .enumerate()
        .map(|(i, date)| {
            let need = |name: &str| d.value(name, model, i).ok_or(WeatherError::BadResponse);
            Ok(Day {
                date: *date,
                temp_min: need("temperature_2m_min")?,
                temp_max: need("temperature_2m_max")?,
                rain_mm: need("precipitation_sum")?,
                pop_percent: d.value("precipitation_probability_max", model, i).unwrap_or(0.0),
                description: wmo_description(d.value("weather_code", model, i).map_or(-1, |c| c as i64)).into(),
                icon: String::new(),
                partial: false,
            })
        })
        .collect()
}

impl WeatherProvider for OpenMeteoProvider {
    fn fetch(&self, lat: f64, lon: f64) -> Result<Snapshot, WeatherError> {
        let (_, result) = self.fetch_models(lat, lon, &[self.model])?.into_iter().next().ok_or(WeatherError::BadResponse)?;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    /// Rút gọn từ phản hồi thật của Open-Meteo (nhiều mô hình: khóa có hậu tố `_<mô hình>`).
    /// ICON thiếu mưa ngày 2 (null) để thử lỗi riêng một mô hình.
    const BODY: &str = r#"{"utc_offset_seconds":25200,
      "current":{"time":"2026-10-27T10:30","temperature_2m":25.2,"relative_humidity_2m":77,"wind_speed_10m":2.24,"weather_code":53},
      "daily":{"time":["2026-10-27","2026-10-28"],
        "temperature_2m_max_best_match":[25.7,23.0],"temperature_2m_min_best_match":[18.5,20.4],
        "precipitation_sum_best_match":[7.0,40.9],"precipitation_probability_max_best_match":[99,100],"weather_code_best_match":[80,95],
        "temperature_2m_max_gfs_seamless":[27.6,25.8],"temperature_2m_min_gfs_seamless":[18.6,19.9],
        "precipitation_sum_gfs_seamless":[0.9,6.0],"precipitation_probability_max_gfs_seamless":[100,100],"weather_code_gfs_seamless":[51,55],
        "temperature_2m_max_ecmwf_ifs025":[25.7,22.9],"temperature_2m_min_ecmwf_ifs025":[19.9,20.5],
        "precipitation_sum_ecmwf_ifs025":[8.8,44.5],"precipitation_probability_max_ecmwf_ifs025":[99,100],"weather_code_ecmwf_ifs025":[53,65],
        "temperature_2m_max_icon_seamless":[27.4,23.5],"temperature_2m_min_icon_seamless":[20.1,21.4],
        "precipitation_sum_icon_seamless":[0.9,null],"precipitation_probability_max_icon_seamless":[75,99],"weather_code_icon_seamless":[80,95]}}"#;

    /// Khi chỉ yêu cầu một mô hình, Open-Meteo trả khóa KHÔNG có hậu tố (đã kiểm với API thật).
    const SINGLE: &str = r#"{"utc_offset_seconds":25200,
      "current":{"time":"2026-10-27T10:30","temperature_2m":27.0,"relative_humidity_2m":61,"wind_speed_10m":2.37,"weather_code":0},
      "daily":{"time":["2026-10-27"],"temperature_2m_max":[27.6],"temperature_2m_min":[18.6],
        "precipitation_sum":[0.9],"precipitation_probability_max":[100],"weather_code":[51]}}"#;

    fn mock(server: &mut mockito::Server, models: &str, body: &str) -> mockito::Mock {
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
            .with_body(body)
            .create()
    }

    fn provider(server: &mockito::Server) -> OpenMeteoProvider {
        OpenMeteoProvider::new(Model::BestMatch).with_base_url(server.url())
    }

    const ALL4: &str = "best_match,gfs_seamless,ecmwf_ifs025,icon_seamless";

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
    fn one_request_returns_each_model_with_its_own_values() {
        let mut server = mockito::Server::new();
        let m = mock(&mut server, ALL4, BODY);
        let results = provider(&server).fetch_models(16.22, 107.25, &Model::ALL).unwrap();
        m.assert();
        let models: Vec<_> = results.iter().map(|(m, _)| *m).collect();
        assert_eq!(models, Model::ALL);
        let day2 = |i: usize| results[i].1.as_ref().unwrap().days[1].clone();
        assert_eq!((day2(0).rain_mm, day2(0).pop_percent, day2(0).description.as_str()), (40.9, 100.0, "Dông"));
        assert_eq!((day2(1).rain_mm, day2(1).temp_max), (6.0, 25.8));
        assert_eq!((day2(2).rain_mm, day2(2).temp_min), (44.5, 20.5));
        let first = results[1].1.as_ref().unwrap();
        assert_eq!(first.source, "Open-Meteo · NOAA GFS");
        assert_eq!(first.tz_offset, 25200);
        assert!(!first.days[0].partial);
    }

    #[test]
    fn a_model_with_missing_values_fails_alone() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, ALL4, BODY);
        let results = provider(&server).fetch_models(16.22, 107.25, &Model::ALL).unwrap();
        let icon = &results[3].1;
        assert_eq!(icon.as_ref().err(), Some(&WeatherError::BadResponse));
        assert!(results[..3].iter().all(|(_, r)| r.is_ok()));
    }

    #[test]
    fn current_conditions_use_wmo_text_and_local_time() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, ALL4, BODY);
        let results = provider(&server).fetch_models(16.22, 107.25, &Model::ALL).unwrap();
        let c = &results[0].1.as_ref().unwrap().current;
        assert_eq!((c.temp, c.humidity, c.wind_speed), (25.2, 77.0, 2.24));
        assert_eq!(c.description, "Mưa phùn vừa");
        // 10:30 giờ địa phương (UTC+7) = 03:30 UTC
        let utc = "2026-10-27".parse::<chrono::NaiveDate>().unwrap().and_hms_opt(3, 30, 0).unwrap().and_utc().timestamp();
        assert_eq!(c.observed_at, utc);
    }

    #[test]
    fn single_model_response_has_unsuffixed_keys() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, "gfs_seamless", SINGLE);
        let results = provider(&server).fetch_models(16.22, 107.25, &[Model::Gfs]).unwrap();
        let day = &results[0].1.as_ref().unwrap().days[0];
        assert_eq!((day.rain_mm, day.temp_max, day.pop_percent), (0.9, 27.6, 100.0));
    }

    #[test]
    fn provider_fetch_returns_its_own_model() {
        let mut server = mockito::Server::new();
        let _m = mock(&mut server, "gfs_seamless", SINGLE);
        let p = OpenMeteoProvider::new(Model::Gfs).with_base_url(server.url());
        assert_eq!(p.fetch(16.22, 107.25).unwrap().source, "Open-Meteo · NOAA GFS");
    }

    #[test]
    fn empty_model_list_makes_no_request() {
        let p = OpenMeteoProvider::new(Model::Gfs).with_base_url("http://127.0.0.1:1");
        assert_eq!(p.fetch_models(16.22, 107.25, &[]).unwrap().len(), 0);
    }

    #[test]
    fn http_errors_and_bad_json_apply_to_the_whole_request() {
        for (status, expect) in [(429, WeatherError::RateLimited), (500, WeatherError::Unavailable), (400, WeatherError::Unavailable)] {
            let mut server = mockito::Server::new();
            let _m = server.mock("GET", Matcher::Any).with_status(status).with_body("{}").create();
            assert_eq!(provider(&server).fetch_models(16.22, 107.25, &Model::ALL).err(), Some(expect), "{status}");
        }
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_body(r#"{"hello":1}"#).create();
        assert_eq!(provider(&server).fetch_models(16.22, 107.25, &Model::ALL).err(), Some(WeatherError::BadResponse));
    }

    #[test]
    fn no_network_is_unavailable() {
        let p = OpenMeteoProvider::new(Model::BestMatch).with_base_url("http://127.0.0.1:1");
        assert_eq!(p.fetch_models(16.22, 107.25, &Model::ALL).err(), Some(WeatherError::Unavailable));
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
    fn live_open_meteo_parses_all_models_in_one_call() {
        let results = OpenMeteoProvider::new(Model::BestMatch).fetch_models(16.22, 107.25, &Model::ALL).unwrap();
        for (model, r) in results {
            let s = r.unwrap_or_else(|e| panic!("{model:?}: {e}"));
            assert_eq!(s.days.len(), 5, "{model:?}");
            assert_eq!(s.tz_offset, 25200);
            println!("{}: {:?}", s.source, s.days.iter().map(|d| d.rain_mm).collect::<Vec<_>>());
        }
        // Một mô hình riêng lẻ (khóa không hậu tố) cũng phải đọc được.
        let one = OpenMeteoProvider::new(Model::Gfs).fetch(16.22, 107.25).unwrap();
        assert_eq!(one.days.len(), 5);
    }
}
