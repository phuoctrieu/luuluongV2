//! Đọc mực nước hồ "MNTL hien tai" (kênh 16), lưu lượng tổ máy (kênh 5, 6) và xả môi trường (kênh 2) từ data logger ECAPRO GPIs6.6DA
//! qua macro WebIOPi `/macros/UpdateMonitor` (xem `huong-dan-lay-mntl-hien-tai.md`).

use std::time::Duration;

pub mod poller;
pub mod tracker;

use thiserror::Error;

/// Số kênh của "MNTL hien tai" trên thiết bị.
const CHANNEL: &str = "16";

/// Cột trạng thái của kênh. Trên thiết bị thật cả 0 và 1 đều đi kèm số đo đúng (kênh 16 báo 0 nhưng
/// khớp kênh 9 và 15), nên 0 KHÔNG được coi là mất kết nối; chỉ 2 và 3 là cảnh báo của thiết bị.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Normal,
    Low,
    High,
}

/// Tổ máy chạy thì lưu lượng khoảng 16,5 m³/s; số đọc từ ngưỡng này trở xuống (thường 1–3 m³/s) là sai số
/// cảm biến, máy đang dừng. Chủ dự án xác nhận 2026-10-28.
pub const RUNNING_MIN_Q: f64 = 5.0;

pub fn is_running(q: f64) -> bool {
    q > RUNNING_MIN_Q
}

/// Lưu lượng tổ máy sau khi bỏ nhiễu cảm biến: máy dừng thì 0, không cộng nhiễu vào lưu lượng về.
fn machine_flow(q: f64) -> f64 {
    if is_running(q) { q } else { 0.0 }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Mực nước hồ (m).
    pub z: f64,
    pub status: Status,
    /// Lưu lượng qua tổ máy H1, H2 (m³/s); `None` nếu thiếu kênh hoặc không phải số.
    pub q_h1: Option<f64>,
    pub q_h2: Option<f64>,
    /// Lưu lượng xả môi trường (kênh 2, m³/s).
    pub q_eco: Option<f64>,
}

impl Reading {
    pub fn new(z: f64, status: Status) -> Self {
        Reading { z, status, q_h1: None, q_h2: None, q_eco: None }
    }
}

/// Thông báo cho người vận hành; chi tiết kỹ thuật ghi log ở nơi gọi.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum LoggerError {
    #[error("Không kết nối được thiết bị ECAPRO.")]
    Unreachable,
    #[error("Thiết bị ECAPRO không trả về mực nước hồ (kênh 16).")]
    MissingChannel,
    #[error("Dữ liệu từ thiết bị ECAPRO không đúng định dạng.")]
    BadResponse,
}

/// Các cột của dòng kênh `ch`: `max/min/avg,số kênh,tên,giá trị,đơn vị,trạng thái,...`.
fn channel_fields<'a>(text: &'a str, ch: &str) -> Option<Vec<&'a str>> {
    let line = text.lines().find(|l| l.split(',').nth(1).is_some_and(|c| c.trim() == ch))?;
    Some(line.split(',').map(str::trim).collect())
}

fn status_of(f: &[&str]) -> Option<Status> {
    match f.get(5).copied() {
        Some("0" | "1") => Some(Status::Normal),
        Some("2") => Some(Status::Low),
        Some("3") => Some(Status::High),
        _ => None,
    }
}

/// Lưu lượng của một kênh (tổ máy, xả môi trường): `None` nếu thiếu kênh hoặc giá trị không phải số.
fn channel_flow(text: &str, ch: &str) -> Option<f64> {
    let f = channel_fields(text, ch)?;
    let q: f64 = f.get(3)?.parse().ok().filter(|q: &f64| q.is_finite())?;
    Some(q)
}

/// Đọc mực nước (kênh 16) và lưu lượng hai tổ máy (kênh 5, 6) từ phản hồi của thiết bị.
pub fn parse_mntl(text: &str) -> Result<Reading, LoggerError> {
    let f = channel_fields(text, CHANNEL).ok_or(LoggerError::MissingChannel)?;
    let z: f64 = f.get(3).and_then(|v| v.parse().ok()).ok_or(LoggerError::BadResponse)?;
    if !z.is_finite() {
        return Err(LoggerError::BadResponse);
    }
    let status = status_of(&f).ok_or(LoggerError::BadResponse)?;
    Ok(Reading { z, status, q_h1: channel_flow(text, "5").map(machine_flow), q_h2: channel_flow(text, "6").map(machine_flow), q_eco: channel_flow(text, "2") })
}

pub fn fetch(url: &str, timeout: Duration) -> Result<Reading, LoggerError> {
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let response = agent.post(url).call().map_err(|_| LoggerError::Unreachable)?;
    let text = response.into_string().map_err(|_| LoggerError::Unreachable)?;
    parse_mntl(&text)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "160.8/ 157.2/ 1.0,16,MNTL hien tai,157.192,m,1,0,0";

    #[test]
    fn parses_sample_line() {
        let r = parse_mntl(SAMPLE).unwrap();
        assert_eq!(r, Reading::new(157.192, Status::Normal));
    }

    const H1_ON: &str = "16.5/ 0.0/ 8.0,5,LUU LUONG MAY 1,16.54,m3/s,1,0,0\n16.5/ 0.0/ 8.2,6,LUU LUONG MAY 2,0,m3/s,1,0,0";

    #[test]
    fn parses_turbine_flows_from_channels_5_and_6() {
        let r = parse_mntl(&format!("{H1_ON}\n{SAMPLE}")).unwrap();
        assert_eq!((r.q_h1, r.q_h2, r.z), (Some(16.54), Some(0.0), 157.192));
    }

    #[test]
    fn missing_or_bad_turbine_channels_are_none_not_errors() {
        let r = parse_mntl(SAMPLE).unwrap();
        assert_eq!((r.q_h1, r.q_h2), (None, None));
        let r = parse_mntl(&format!("a,5,M1,xyz,m3/s,1,0,0\na,6,M2,abc,m3/s,1,0,0\n{SAMPLE}")).unwrap();
        assert_eq!((r.q_h1, r.q_h2), (None, None));
        let r = parse_mntl(&format!("a,5,M1,16.54,m3/s,0,0,0\n{SAMPLE}")).unwrap();
        assert_eq!(r.q_h1, Some(16.54), "trạng thái 0 không làm mất số đo");
    }

    #[test]
    fn sensor_noise_up_to_a_few_cubic_metres_is_not_running() {
        assert!(is_running(16.54) && is_running(16.5) && is_running(33.0));
        for noise in [0.0, 0.3, 1.0, 2.0, 3.0, 5.0] {
            assert!(!is_running(noise), "{noise} m³/s là nhiễu cảm biến");
        }
    }

    #[test]
    fn machine_noise_reads_as_zero_but_eco_release_is_kept_as_measured() {
        let text = format!("a,2,DCTT,1.5,m3/s,1,0,0\na,5,M1,2.0,m3/s,1,0,0\na,6,M2,16.5,m3/s,1,0,0\n{SAMPLE}");
        let r = parse_mntl(&text).unwrap();
        assert_eq!((r.q_h1, r.q_h2, r.q_eco), (Some(0.0), Some(16.5), Some(1.5)));
    }

    #[test]
    fn finds_channel_16_among_many_lines() {
        let text = format!("1/ 0/ 0,15,Kenh khac,9.9,m,1,0,0\n{SAMPLE}\n1/ 0/ 0,17,Khac,1.0,m,1,0,0\n");
        assert_eq!(parse_mntl(&text).unwrap().z, 157.192);
    }

    /// Phản hồi thật của thiết bị (curl ngày 2026-10-28). Kênh 16 có trạng thái 0 nhưng giá trị vẫn đúng.
    const REAL: &str = r#"33.0/ 0.0/ 3.0,1,LUU LUONG MUA,2.0,mm/h,1,0,0
1.5/ 1.5/ 0.0,2,DCTT|LUULUONG,1.5,m3/s,1,0,0
160.8/ 160.0/ 0.3,3,THUONGLUU|MUCNUOC,160.0,m,1,0,0
20.1/ 0.0/ 2.8,4,LUU LUONG VE old,16.81,m3/s,0,0,0
16.5/ 0.0/ 8.0,5,LUU LUONG MAY 1,16.54,m3/s,1,0,0
16.5/ 0.0/ 8.2,6,LUU LUONG MAY 2,0,m3/s,1,0,0
33.1/ 0.0/ 16.0,7,NHAMAY|LUULUONG,16.54,m3/s,0,0,0
0/ 0/ 0.0,8,QUATRAN|LUULUONG,0,m3/s,0,0,0
160.8/ 157.2/ 1.0,9,Htltc Old,157.188,m,0,0,0
582/ 0/ 171.1,10,Time,213,,0,0,0
34.6/ 2.5/ 12.8,11,DENHO|LUULUONG,18.04,m3/s,0,0,0
1/ 0/ 0.5,12,TT,1,,0,0,0
1.0/ 0.0/ 0.5,13,RS Time,1,,0,0,0
20.1/ 0.0/ 2.8,14,LLVT new,17.67,m3/s,0,0,0
160.8/ 157.2/ 1.0,15,Htltc Old t,157.188,m,0,0,0
160.8/ 157.2/ 1.0,16,MNTL hien tai,157.189,m,0,0,0
"#;

    #[test]
    fn real_device_output_gives_level_and_machines_despite_status_zero() {
        let r = parse_mntl(REAL).unwrap();
        assert_eq!(r, Reading { z: 157.189, status: Status::Normal, q_h1: Some(16.54), q_h2: Some(0.0), q_eco: Some(1.5) });
    }

    #[test]
    fn eco_release_comes_from_channel_2_not_from_channel_12() {
        let r = parse_mntl(&format!("a,12,TT,9.9,,0,0,0\na,2,DCTT|LUULUONG,1.5,m3/s,1,0,0\n{SAMPLE}")).unwrap();
        assert_eq!(r.q_eco, Some(1.5));
        assert_eq!(parse_mntl(SAMPLE).unwrap().q_eco, None);
        assert_eq!(parse_mntl(&format!("a,2,D,abc,m3/s,1,0,0\n{SAMPLE}")).unwrap().q_eco, None);
    }

    #[test]
    fn status_zero_still_carries_a_valid_value() {
        let r = parse_mntl("160.8/ 157.2/ 1.0,16,MNTL hien tai,157.192,m,0,0,0").unwrap();
        assert_eq!((r.z, r.status), (157.192, Status::Normal));
    }

    #[test]
    fn maps_low_and_high() {
        assert_eq!(parse_mntl("a,16,M,150.0,m,2,0,0").unwrap().status, Status::Low);
        assert_eq!(parse_mntl("a,16,M,150.0,m,3,0,0").unwrap().status, Status::High);
    }

    #[test]
    fn missing_channel_16_is_error() {
        assert_eq!(parse_mntl("a,15,M,150.0,m,1,0,0\n").unwrap_err(), LoggerError::MissingChannel);
        assert_eq!(parse_mntl("").unwrap_err(), LoggerError::MissingChannel);
    }

    #[test]
    fn non_numeric_value_is_error() {
        assert_eq!(parse_mntl("a,16,M,abc,m,1,0,0").unwrap_err(), LoggerError::BadResponse);
        assert_eq!(parse_mntl("a,16,M,NaN,m,1,0,0").unwrap_err(), LoggerError::BadResponse);
    }

    #[test]
    fn unknown_status_is_error() {
        assert_eq!(parse_mntl("a,16,M,150.0,m,9,0,0").unwrap_err(), LoggerError::BadResponse);
        assert_eq!(parse_mntl("a,16,M,150.0").unwrap_err(), LoggerError::BadResponse);
    }

    #[test]
    fn fetch_reads_reading_over_post() {
        let mut server = mockito::Server::new();
        let m = server
            .mock("POST", "/macros/UpdateMonitor")
            .with_status(200)
            .with_body(SAMPLE)
            .create();
        let url = format!("{}/macros/UpdateMonitor", server.url());
        let r = fetch(&url, Duration::from_secs(2)).unwrap();
        assert_eq!(r.z, 157.192);
        m.assert();
    }

    #[test]
    fn fetch_server_error_is_unreachable() {
        let mut server = mockito::Server::new();
        let _m = server.mock("POST", "/x").with_status(500).create();
        let url = format!("{}/x", server.url());
        assert_eq!(fetch(&url, Duration::from_secs(2)).unwrap_err(), LoggerError::Unreachable);
    }

    #[test]
    fn fetch_timeout_is_unreachable() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("POST", "/x")
            .with_chunked_body(|w| {
                std::thread::sleep(Duration::from_millis(600));
                w.write_all(b"late")
            })
            .create();
        let url = format!("{}/x", server.url());
        assert_eq!(fetch(&url, Duration::from_millis(150)).unwrap_err(), LoggerError::Unreachable);
    }
}
