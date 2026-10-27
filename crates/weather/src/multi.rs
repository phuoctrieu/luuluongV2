//! Lấy nhiều nguồn cùng lúc để so sánh. Mỗi nguồn có kết quả/lỗi riêng.

use crate::error::WeatherError;
use crate::free::FreeProvider;
use crate::openmeteo::{Model, OpenMeteoProvider};
use crate::provider::{Snapshot, WeatherProvider};
use crate::source::Source;

#[derive(Debug, PartialEq)]
pub struct SourceResult {
    pub source: Source,
    pub result: Result<Snapshot, WeatherError>,
}

/// Kết quả trả theo đúng thứ tự `sources`. OpenWeather chạy luồng riêng song song với Open-Meteo;
/// các mô hình Open-Meteo chung một lần gọi.
pub fn fetch_all(sources: &[Source], free: &FreeProvider, meteo: &OpenMeteoProvider, lat: f64, lon: f64) -> Vec<SourceResult> {
    let want_openweather = sources.contains(&Source::OpenWeather);
    let models: Vec<Model> = sources
        .iter()
        .filter_map(|s| match s {
            Source::OpenMeteo(m) => Some(*m),
            Source::OpenWeather => None,
        })
        .collect();
    let (openweather, open_meteo) = std::thread::scope(|scope| {
        let ow = want_openweather.then(|| scope.spawn(|| free.fetch(lat, lon)));
        let om = meteo.fetch_models(lat, lon, &models);
        (ow.map(|h| h.join().unwrap_or(Err(WeatherError::Unavailable))), om)
    });
    sources
        .iter()
        .map(|&source| {
            let result = match (source, &open_meteo) {
                (Source::OpenWeather, _) => openweather.clone().unwrap_or(Err(WeatherError::Unavailable)),
                (_, Err(e)) => Err(e.clone()),
                (Source::OpenMeteo(m), Ok(list)) => {
                    list.iter().find(|(x, _)| *x == m).map_or(Err(WeatherError::BadResponse), |(_, r)| r.clone())
                }
            };
            SourceResult { source, result }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    const WEATHER: &str = r#"{"dt":1698400000,"main":{"temp":24.6,"humidity":92},"wind":{"speed":1.8},
        "weather":[{"description":"mưa nhẹ","icon":"10d"}]}"#;
    const FORECAST: &str = r#"{"city":{"timezone":25200},"list":[
        {"dt":1698397200,"main":{"temp":23.0},"pop":0.5,"rain":{"3h":4.2},"weather":[{"description":"mưa","icon":"10d"}]}]}"#;
    const METEO: &str = r#"{"utc_offset_seconds":25200,
      "current":{"time":"2026-10-27T10:30","temperature_2m":25.2,"relative_humidity_2m":77,"wind_speed_10m":2.24,"weather_code":53},
      "daily":{"time":["2026-10-28"],
        "temperature_2m_max_gfs_seamless":[25.8],"temperature_2m_min_gfs_seamless":[19.9],"precipitation_sum_gfs_seamless":[6.0],
        "precipitation_probability_max_gfs_seamless":[100],"weather_code_gfs_seamless":[55],
        "temperature_2m_max_ecmwf_ifs025":[22.9],"temperature_2m_min_ecmwf_ifs025":[20.5],"precipitation_sum_ecmwf_ifs025":[44.5],
        "precipitation_probability_max_ecmwf_ifs025":[100],"weather_code_ecmwf_ifs025":[65]}}"#;

    struct Servers {
        ow: mockito::ServerGuard,
        om: mockito::ServerGuard,
    }

    fn servers() -> Servers {
        Servers { ow: mockito::Server::new(), om: mockito::Server::new() }
    }

    fn run(s: &Servers, sources: &[Source], key: &str) -> Vec<SourceResult> {
        let free = FreeProvider::new(key).with_base_url(s.ow.url());
        let meteo = OpenMeteoProvider::new(Model::BestMatch).with_base_url(s.om.url());
        fetch_all(sources, &free, &meteo, 16.22, 107.25)
    }

    fn mock_ow(s: &mut Servers) -> (mockito::Mock, mockito::Mock) {
        (
            s.ow.mock("GET", "/weather").match_query(Matcher::Any).with_body(WEATHER).create(),
            s.ow.mock("GET", "/forecast").match_query(Matcher::Any).with_body(FORECAST).create(),
        )
    }

    fn mock_om(s: &mut Servers) -> mockito::Mock {
        s.om.mock("GET", "/forecast")
            .match_query(Matcher::UrlEncoded("models".into(), "gfs_seamless,ecmwf_ifs025".into()))
            .with_body(METEO)
            .create()
    }

    const MIXED: [Source; 3] = [Source::OpenWeather, Source::OpenMeteo(Model::Gfs), Source::OpenMeteo(Model::Ecmwf)];

    #[test]
    fn returns_every_source_in_the_requested_order() {
        let mut s = servers();
        let (w, f) = mock_ow(&mut s);
        let om = mock_om(&mut s);
        let out = run(&s, &MIXED, "KEY");
        w.assert();
        f.assert();
        om.assert(); // một lần gọi duy nhất cho cả hai mô hình
        let sources: Vec<_> = out.iter().map(|r| r.source).collect();
        assert_eq!(sources, MIXED);
        let rain: Vec<_> = out.iter().map(|r| r.result.as_ref().unwrap().days[0].rain_mm).collect();
        assert_eq!(rain, vec![4.2, 6.0, 44.5]);
    }

    #[test]
    fn a_bad_openweather_key_does_not_hide_open_meteo() {
        let mut s = servers();
        let _ow = s.ow.mock("GET", Matcher::Any).with_status(401).with_body("{}").expect_at_least(1).create();
        let _om = mock_om(&mut s);
        let out = run(&s, &MIXED, "BAD");
        assert_eq!(out[0].result, Err(WeatherError::InvalidKey));
        assert!(out[1].result.is_ok() && out[2].result.is_ok());
    }

    #[test]
    fn open_meteo_outage_marks_only_its_models() {
        let mut s = servers();
        let _ = mock_ow(&mut s);
        let _om = s.om.mock("GET", Matcher::Any).with_status(500).with_body("{}").create();
        let out = run(&s, &MIXED, "KEY");
        assert!(out[0].result.is_ok());
        assert_eq!(out[1].result, Err(WeatherError::Unavailable));
        assert_eq!(out[2].result, Err(WeatherError::Unavailable));
    }

    #[test]
    fn unselected_services_are_not_called() {
        let mut s = servers();
        let ow = s.ow.mock("GET", Matcher::Any).expect(0).create();
        let _om = mock_om(&mut s);
        let out = run(&s, &MIXED[1..], "KEY");
        ow.assert();
        assert_eq!(out.len(), 2);

        let mut s = servers();
        let (w, f) = mock_ow(&mut s);
        let om = s.om.mock("GET", Matcher::Any).expect(0).create();
        let out = run(&s, &[Source::OpenWeather], "KEY");
        w.assert();
        f.assert();
        om.assert();
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn missing_openweather_key_is_reported_for_that_source_only() {
        let mut s = servers();
        let _om = mock_om(&mut s);
        let out = run(&s, &MIXED, "  ");
        assert_eq!(out[0].result, Err(WeatherError::MissingKey));
        assert!(out[1].result.is_ok());
    }

    #[test]
    fn no_sources_means_no_results() {
        let s = servers();
        assert!(run(&s, &[], "KEY").is_empty());
    }
}
