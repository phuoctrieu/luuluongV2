//! Gói miễn phí: `/weather` + `/forecast` (đặc tả 5.1).

use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::aggregate::{aggregate, Slot};
use crate::error::WeatherError;
use crate::provider::{read_json, Current, Snapshot, WeatherProvider};

const BASE_URL: &str = "https://api.openweathermap.org/data/2.5";

pub struct FreeProvider {
    api_key: String,
    base_url: String,
    agent: ureq::Agent,
}

impl FreeProvider {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: BASE_URL.into(),
            agent: agent(Duration::from_secs(8)),
        }
    }

    /// Chỉ dùng trong test (server giả) hoặc proxy nội bộ.
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.agent = agent(timeout);
        self
    }
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(timeout).build()
}

#[derive(Deserialize)]
struct Condition {
    description: String,
    icon: String,
}

#[derive(Deserialize)]
struct CurrentResp {
    dt: i64,
    main: CurrentMain,
    wind: Wind,
    weather: Vec<Condition>,
}

#[derive(Deserialize)]
struct CurrentMain {
    temp: f64,
    humidity: f64,
}

#[derive(Deserialize)]
struct Wind {
    speed: f64,
}

#[derive(Deserialize)]
struct ForecastResp {
    city: City,
    list: Vec<Item>,
}

#[derive(Deserialize)]
struct City {
    timezone: i32,
}

#[derive(Deserialize)]
struct Item {
    dt: i64,
    main: ItemMain,
    #[serde(default)]
    pop: f64,
    rain: Option<Rain>,
    weather: Vec<Condition>,
}

#[derive(Deserialize)]
struct ItemMain {
    temp: f64,
}

#[derive(Deserialize)]
struct Rain {
    #[serde(rename = "3h")]
    h3: Option<f64>,
}

impl FreeProvider {
    fn get<T: DeserializeOwned>(&self, path: &str, lat: f64, lon: f64) -> Result<T, WeatherError> {
        let response = self
            .agent
            .get(&format!("{}/{path}", self.base_url))
            .query("lat", &lat.to_string())
            .query("lon", &lon.to_string())
            .query("appid", &self.api_key)
            .query("units", "metric")
            .query("lang", "vi")
            .call();
        read_json(response)
    }
}

impl WeatherProvider for FreeProvider {
    fn fetch(&self, lat: f64, lon: f64) -> Result<Snapshot, WeatherError> {
        if self.api_key.trim().is_empty() {
            return Err(WeatherError::MissingKey);
        }
        let now: CurrentResp = self.get("weather", lat, lon)?;
        let forecast: ForecastResp = self.get("forecast", lat, lon)?;
        let first = |c: &[Condition]| c.first().map(|c| (c.description.clone(), c.icon.clone()));
        let (description, icon) = first(&now.weather).ok_or(WeatherError::BadResponse)?;
        let slots = forecast
            .list
            .iter()
            .map(|i| {
                let (description, icon) = first(&i.weather).ok_or(WeatherError::BadResponse)?;
                Ok(Slot {
                    dt: i.dt,
                    temp: i.main.temp,
                    rain_3h: i.rain.as_ref().and_then(|r| r.h3),
                    pop: i.pop,
                    description,
                    icon,
                })
            })
            .collect::<Result<Vec<_>, WeatherError>>()?;
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        Ok(Snapshot {
            current: Current {
                temp: now.main.temp,
                humidity: now.main.humidity,
                wind_speed: now.wind.speed,
                description,
                icon,
                observed_at: now.dt,
            },
            days: aggregate(&slots, forecast.city.timezone),
            fetched_at,
            tz_offset: forecast.city.timezone,
            source: "OpenWeather".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    const WEATHER: &str = r#"{"dt":1698400000,"timezone":25200,
        "main":{"temp":24.6,"humidity":92},"wind":{"speed":1.8},
        "weather":[{"description":"mưa nhẹ","icon":"10d"}]}"#;

    /// 2 mốc ngày 27/10 (UTC+7) có mưa, 1 mốc thiếu `rain`.
    const FORECAST: &str = r#"{"city":{"timezone":25200},"list":[
        {"dt":1698397200,"main":{"temp":23.0},"pop":0.5,"rain":{"3h":4.2},"weather":[{"description":"mưa","icon":"10d"}]},
        {"dt":1698408000,"main":{"temp":26.0},"pop":0.8,"weather":[{"description":"mưa","icon":"10d"}]}]}"#;

    fn provider(server: &mockito::Server) -> FreeProvider {
        FreeProvider::new("KEY123").with_base_url(server.url())
    }

    fn mock_ok(server: &mut mockito::Server) -> (mockito::Mock, mockito::Mock) {
        let q = Matcher::AllOf(vec![
            Matcher::UrlEncoded("lat".into(), "16.22".into()),
            Matcher::UrlEncoded("lon".into(), "107.25".into()),
            Matcher::UrlEncoded("appid".into(), "KEY123".into()),
            Matcher::UrlEncoded("units".into(), "metric".into()),
            Matcher::UrlEncoded("lang".into(), "vi".into()),
        ]);
        let w = server.mock("GET", "/weather").match_query(q.clone()).with_body(WEATHER).create();
        let f = server.mock("GET", "/forecast").match_query(q).with_body(FORECAST).create();
        (w, f)
    }

    #[test]
    fn default_endpoint_is_https() {
        assert!(FreeProvider::new("k").base_url.starts_with("https://"));
    }

    #[test]
    fn fetches_current_and_aggregates_forecast() {
        let mut server = mockito::Server::new();
        let (w, f) = mock_ok(&mut server);
        let s = provider(&server).fetch(16.22, 107.25).unwrap();
        w.assert();
        f.assert();
        assert_eq!(s.current.temp, 24.6);
        assert_eq!(s.current.humidity, 92.0);
        assert_eq!(s.current.wind_speed, 1.8);
        assert_eq!(s.current.description, "mưa nhẹ");
        assert_eq!(s.tz_offset, 25200);
        assert_eq!(s.days.len(), 1);
        assert_eq!(s.days[0].rain_mm, 4.2);
        assert_eq!(s.days[0].pop_percent, 80.0);
        assert!(s.days[0].partial);
        assert!(s.fetched_at > 1_700_000_000);
    }

    fn error_for(status: usize) -> WeatherError {
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_status(status).with_body("{}").expect_at_least(1).create();
        provider(&server).fetch(16.22, 107.25).unwrap_err()
    }

    #[test]
    fn maps_http_errors_to_vietnamese_causes() {
        assert_eq!(error_for(401), WeatherError::InvalidKey);
        assert_eq!(error_for(429), WeatherError::RateLimited);
        assert_eq!(error_for(500), WeatherError::Unavailable);
        assert_eq!(error_for(503), WeatherError::Unavailable);
    }

    #[test]
    fn wrong_json_shape_is_bad_response_not_a_panic() {
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_body(r#"{"cod":"ok"}"#).create();
        assert_eq!(provider(&server).fetch(16.22, 107.25), Err(WeatherError::BadResponse));
    }

    #[test]
    fn slow_server_times_out() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("GET", Matcher::Any)
            .with_chunked_body(|w| {
                std::thread::sleep(Duration::from_millis(400));
                w.write_all(b"{}")
            })
            .create();
        let p = provider(&server).with_timeout(Duration::from_millis(80));
        assert_eq!(p.fetch(16.22, 107.25), Err(WeatherError::Unavailable));
    }

    #[test]
    fn no_network_is_unavailable_and_message_hides_key() {
        let p = FreeProvider::new("KEY123").with_base_url("http://127.0.0.1:1");
        let e = p.fetch(16.22, 107.25).unwrap_err();
        assert_eq!(e, WeatherError::Unavailable);
        assert!(!format!("{e} {e:?}").contains("KEY123"));
    }

    #[test]
    fn blank_key_fails_without_calling_the_network() {
        let p = FreeProvider::new("  ").with_base_url("http://127.0.0.1:1");
        assert_eq!(p.fetch(16.22, 107.25), Err(WeatherError::MissingKey));
    }
}
