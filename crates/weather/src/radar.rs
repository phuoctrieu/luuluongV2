//! Radar mưa: khung hình từ RainViewer + bản đồ nền OpenStreetMap (lưu cache trên đĩa).
//! RainViewer chỉ miễn phí cho cá nhân/giáo dục; bản đồ cần ghi nguồn © OpenStreetMap.

use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

const API_BASE: &str = "https://api.rainviewer.com";
const OSM_BASE: &str = "https://tile.openstreetmap.org";
const USER_AGENT: &str = "luuluong-songbo/0.1 (ung dung noi bo nha may thuy dien)";
const TILE_SIZE: f64 = 256.0;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RadarError {
    #[error("Không tải được radar mưa (mạng hoặc dịch vụ gặp sự cố).")]
    Unavailable,
    #[error("Ảnh radar không đọc được.")]
    BadImage,
}

/// Ảnh RGBA 8 bit, hàng nối tiếp nhau.
#[derive(Debug, Clone, PartialEq)]
pub struct Rgba {
    pub width: usize,
    pub height: usize,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// unix, UTC
    pub time: i64,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frames {
    pub host: String,
    /// Tăng dần theo thời gian; khung cuối là mới nhất.
    pub frames: Vec<Frame>,
}

/// Tọa độ tile phân số (x, y) của một điểm ở mức zoom `z` (Web Mercator).
pub fn tile_coords(lat: f64, lon: f64, z: u8) -> (f64, f64) {
    let n = f64::from(1u32 << z);
    let x = (lon + 180.0) / 360.0 * n;
    let y = (1.0 - lat.to_radians().tan().asinh() / std::f64::consts::PI) / 2.0 * n;
    (x, y)
}

/// Lưới 3×3 tile quanh một điểm, xếp theo hàng (trái→phải, trên→dưới).
#[derive(Debug, Clone, PartialEq)]
pub struct Grid {
    pub z: u8,
    pub tiles: [(i32, i32); 9],
    /// Vị trí điểm trong ảnh ghép 768×768 (pixel).
    pub point_px: (f64, f64),
}

pub fn grid_around(lat: f64, lon: f64, z: u8) -> Grid {
    let (fx, fy) = tile_coords(lat, lon, z);
    let (cx, cy) = (fx.floor() as i32, fy.floor() as i32);
    let mut tiles = [(0, 0); 9];
    for (i, tile) in tiles.iter_mut().enumerate() {
        *tile = (cx + (i % 3) as i32 - 1, cy + (i / 3) as i32 - 1);
    }
    Grid { z, tiles, point_px: ((fx - f64::from(cx - 1)) * TILE_SIZE, (fy - f64::from(cy - 1)) * TILE_SIZE) }
}

pub fn decode_png(bytes: &[u8]) -> Result<Rgba, RadarError> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(|_| RadarError::BadImage)?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).map_err(|_| RadarError::BadImage)?;
    let raw = &buf[..info.buffer_size()];
    let data: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => raw.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        png::ColorType::Grayscale => raw.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::GrayscaleAlpha => raw.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect(),
        png::ColorType::Indexed => return Err(RadarError::BadImage),
    };
    Ok(Rgba { width: info.width as usize, height: info.height as usize, data })
}

#[derive(serde::Deserialize)]
struct ApiResponse {
    host: String,
    radar: ApiRadar,
}

#[derive(serde::Deserialize)]
struct ApiRadar {
    past: Vec<ApiFrame>,
}

#[derive(serde::Deserialize)]
struct ApiFrame {
    time: i64,
    path: String,
}

pub struct RadarClient {
    api_base: String,
    osm_base: String,
    agent: ureq::Agent,
    cache_dir: Option<PathBuf>,
}

impl RadarClient {
    pub fn new(cache_dir: Option<PathBuf>) -> Self {
        Self {
            api_base: API_BASE.into(),
            osm_base: OSM_BASE.into(),
            agent: ureq::AgentBuilder::new().timeout(Duration::from_secs(8)).user_agent(USER_AGENT).build(),
            cache_dir,
        }
    }

    /// Chỉ dùng trong test (server giả).
    pub fn with_bases(mut self, api_base: impl Into<String>, osm_base: impl Into<String>) -> Self {
        self.api_base = api_base.into();
        self.osm_base = osm_base.into();
        self
    }

    /// Danh sách khung radar đã qua, giữ tối đa `max` khung mới nhất.
    pub fn frames(&self, max: usize) -> Result<Frames, RadarError> {
        let body = self.get(&format!("{}/public/weather-maps.json", self.api_base))?;
        let resp: ApiResponse = serde_json::from_slice(&body).map_err(|_| RadarError::Unavailable)?;
        let mut frames: Vec<Frame> = resp.radar.past.into_iter().map(|f| Frame { time: f.time, path: f.path }).collect();
        frames.sort_by_key(|f| f.time);
        let skip = frames.len().saturating_sub(max);
        Ok(Frames { host: resp.host, frames: frames.split_off(skip) })
    }

    fn get(&self, url: &str) -> Result<Vec<u8>, RadarError> {
        use std::io::Read;
        let response = self.agent.get(url).call().map_err(|_| RadarError::Unavailable)?;
        let mut body = Vec::new();
        response.into_reader().take(8 * 1024 * 1024).read_to_end(&mut body).map_err(|_| RadarError::Unavailable)?;
        Ok(body)
    }

    /// Màu "Universal Blue" (2), làm mượt (1), không tuyết (0).
    pub fn radar_tile(&self, frames: &Frames, frame: &Frame, z: u8, x: i32, y: i32) -> Result<Rgba, RadarError> {
        let url = format!("{}{}/256/{z}/{x}/{y}/2/1_0.png", frames.host, frame.path);
        decode_png(&self.get(&url)?)
    }

    /// Tile bản đồ nền; dùng cache trên đĩa nếu có và chưa quá 14 ngày.
    pub fn base_tile(&self, z: u8, x: i32, y: i32) -> Result<Rgba, RadarError> {
        let cached = self.cache_dir.as_ref().map(|d| d.join(format!("osm/{z}/{x}/{y}.png")));
        if let Some(path) = &cached {
            let fresh = std::fs::metadata(path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.elapsed().ok())
                .is_some_and(|age| age < Duration::from_secs(14 * 24 * 3600));
            if fresh {
                if let Some(img) = std::fs::read(path).ok().and_then(|b| decode_png(&b).ok()) {
                    return Ok(img);
                }
            }
        }
        let bytes = self.get(&format!("{}/{z}/{x}/{y}.png", self.osm_base))?;
        let img = decode_png(&bytes)?;
        if let Some(path) = &cached {
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir).and_then(|()| std::fs::write(path, &bytes));
            }
        }
        Ok(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Matcher;

    /// PNG RGBA 2×2 tạo bằng chính crate `png`.
    fn png_bytes(color: png::ColorType, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut enc = png::Encoder::new(&mut out, 2, 2);
            enc.set_color(color);
            enc.set_depth(png::BitDepth::Eight);
            enc.write_header().unwrap().write_image_data(data).unwrap();
        }
        out
    }

    fn rgba_png() -> Vec<u8> {
        png_bytes(png::ColorType::Rgba, &[255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 9, 9, 9, 9])
    }

    #[test]
    fn tile_coords_match_the_slippy_map_formula() {
        let (x, y) = tile_coords(16.22, 107.25, 6);
        assert_eq!((x.floor() as i32, y.floor() as i32), (51, 29)); // đã kiểm với API thật
        assert_eq!(tile_coords(0.0, 0.0, 1), (1.0, 1.0));
        assert_eq!(tile_coords(0.0, -180.0, 3).0, 0.0);
    }

    #[test]
    fn grid_is_3x3_around_the_point_with_pixel_position() {
        let g = grid_around(16.22, 107.25, 7);
        let (fx, fy) = tile_coords(16.22, 107.25, 7);
        let (cx, cy) = (fx.floor() as i32, fy.floor() as i32);
        assert_eq!(g.tiles[4], (cx, cy));
        assert_eq!(g.tiles[0], (cx - 1, cy - 1));
        assert_eq!(g.tiles[8], (cx + 1, cy + 1));
        assert_eq!(g.tiles[1], (cx, cy - 1));
        assert!((g.point_px.0 - (fx - (cx - 1) as f64) * 256.0).abs() < 1e-9);
        assert!((g.point_px.1 - (fy - (cy - 1) as f64) * 256.0).abs() < 1e-9);
        assert!(g.point_px.0 > 256.0 && g.point_px.0 < 512.0); // nằm trong tile giữa
    }

    #[test]
    fn decodes_rgba_and_adds_opaque_alpha_to_rgb() {
        let img = decode_png(&rgba_png()).unwrap();
        assert_eq!((img.width, img.height), (2, 2));
        assert_eq!(&img.data[..8], &[255, 0, 0, 255, 0, 255, 0, 128]);
        let rgb = decode_png(&png_bytes(png::ColorType::Rgb, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12])).unwrap();
        assert_eq!(&rgb.data[..8], &[1, 2, 3, 255, 4, 5, 6, 255]);
    }

    #[test]
    fn garbage_is_a_bad_image_not_a_panic() {
        assert_eq!(decode_png(b"khong phai png").err(), Some(RadarError::BadImage));
        assert_eq!(decode_png(&[]).err(), Some(RadarError::BadImage));
    }

    fn frames_body(host: &str) -> String {
        format!(
            r#"{{"version":"2.0","generated":1793073324,"host":"{host}",
            "radar":{{"past":[{{"time":1793071200,"path":"/v2/radar/a1"}},{{"time":1793072400,"path":"/v2/radar/b2"}},
                              {{"time":1793073000,"path":"/v2/radar/c3"}}],"nowcast":[]}}}}"#
        )
    }

    #[test]
    fn frames_keep_the_latest_n_in_time_order() {
        let mut server = mockito::Server::new();
        let m = server.mock("GET", "/public/weather-maps.json").with_body(frames_body("https://tilecache.example")).create();
        let c = RadarClient::new(None).with_bases(server.url(), server.url());
        let f = c.frames(2).unwrap();
        m.assert();
        assert_eq!(f.host, "https://tilecache.example");
        let paths: Vec<_> = f.frames.iter().map(|x| x.path.as_str()).collect();
        assert_eq!(paths, ["/v2/radar/b2", "/v2/radar/c3"]);
    }

    #[test]
    fn frames_errors_are_classified() {
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_status(500).create();
        assert_eq!(RadarClient::new(None).with_bases(server.url(), server.url()).frames(6).err(), Some(RadarError::Unavailable));
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_body(r#"{"nope":1}"#).create();
        assert_eq!(RadarClient::new(None).with_bases(server.url(), server.url()).frames(6).err(), Some(RadarError::Unavailable));
        assert_eq!(RadarClient::new(None).with_bases("http://127.0.0.1:1", "http://127.0.0.1:1").frames(6).err(), Some(RadarError::Unavailable));
    }

    #[test]
    fn radar_tile_uses_the_documented_url_and_decodes() {
        let mut server = mockito::Server::new();
        let m = server.mock("GET", "/v2/radar/c3/256/7/102/57/2/1_0.png").with_body(rgba_png()).create();
        let c = RadarClient::new(None).with_bases(server.url(), server.url());
        let frames = Frames { host: server.url(), frames: vec![Frame { time: 1, path: "/v2/radar/c3".into() }] };
        let img = c.radar_tile(&frames, &frames.frames[0], 7, 102, 57).unwrap();
        m.assert();
        assert_eq!((img.width, img.height), (2, 2));
    }

    #[test]
    fn base_tile_identifies_the_app_and_is_cached_on_disk() {
        let dir = std::env::temp_dir().join(format!("luuluong-radar-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut server = mockito::Server::new();
        let m = server
            .mock("GET", "/7/102/57.png")
            .match_header("user-agent", Matcher::Regex("luuluong".into()))
            .with_body(rgba_png())
            .expect(1)
            .create();
        let c = RadarClient::new(Some(dir.clone())).with_bases(server.url(), server.url());
        assert!(c.base_tile(7, 102, 57).is_ok());
        assert!(c.base_tile(7, 102, 57).is_ok()); // lần hai đọc từ đĩa
        m.assert();
        assert!(dir.join("osm/7/102/57.png").exists());
    }

    #[test]
    fn base_tile_failure_is_unavailable() {
        let mut server = mockito::Server::new();
        let _m = server.mock("GET", Matcher::Any).with_status(429).create();
        let c = RadarClient::new(None).with_bases(server.url(), server.url());
        assert_eq!(c.base_tile(7, 1, 1).err(), Some(RadarError::Unavailable));
    }

    #[test]
    fn default_endpoints_are_https() {
        let c = RadarClient::new(None);
        assert!(c.api_base.starts_with("https://") && c.osm_base.starts_with("https://"));
    }

    /// Gọi dịch vụ thật: `cargo test -p luuluong-weather -- --ignored live_radar`.
    #[test]
    #[ignore]
    fn live_radar_frames_radar_tile_and_base_tile() {
        let c = RadarClient::new(None);
        let frames = c.frames(6).unwrap();
        assert_eq!(frames.frames.len(), 6);
        let grid = grid_around(16.22, 107.25, 7);
        let (x, y) = grid.tiles[4];
        let radar = c.radar_tile(&frames, frames.frames.last().unwrap(), 7, x, y).unwrap();
        let base = c.base_tile(7, x, y).unwrap();
        println!("radar {}x{} base {}x{} frame {}", radar.width, radar.height, base.width, base.height, frames.frames[5].time);
        assert_eq!((radar.width, base.width), (256, 256));
    }
}
