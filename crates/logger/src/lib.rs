//! Đọc mực nước hồ "MNTL hien tai" (kênh 16) từ data logger ECAPRO GPIs6.6DA
//! qua macro WebIOPi `/macros/UpdateMonitor` (xem `huong-dan-lay-mntl-hien-tai.md`).

use std::time::Duration;

pub mod poller;
pub mod tracker;

use thiserror::Error;

/// Số kênh của "MNTL hien tai" trên thiết bị.
const CHANNEL: &str = "16";

/// Trạng thái kênh theo thiết bị: 0 = không kết nối, 1 = bình thường, 2 = báo thấp, 3 = báo cao.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Disconnected,
    Normal,
    Low,
    High,
}

/// Lưu lượng qua tổ máy lớn hơn mức này (m³/s) thì coi là đang chạy; máy dừng đọc 0.
pub const RUNNING_MIN_Q: f64 = 1.0;

pub fn is_running(q: f64) -> bool {
    q > RUNNING_MIN_Q
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Mực nước hồ (m).
    pub z: f64,
    pub status: Status,
    /// Lưu lượng qua tổ máy H1, H2 (m³/s); `None` nếu thiếu kênh, kênh không kết nối hoặc không phải số.
    pub q_h1: Option<f64>,
    pub q_h2: Option<f64>,
}

impl Reading {
    pub fn new(z: f64, status: Status) -> Self {
        Reading { z, status, q_h1: None, q_h2: None }
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
        Some("0") => Some(Status::Disconnected),
        Some("1") => Some(Status::Normal),
        Some("2") => Some(Status::Low),
        Some("3") => Some(Status::High),
        _ => None,
    }
}

/// Lưu lượng tổ máy: `None` nếu thiếu kênh, kênh không kết nối hoặc giá trị không phải số.
fn turbine_flow(text: &str, ch: &str) -> Option<f64> {
    let f = channel_fields(text, ch)?;
    let q: f64 = f.get(3)?.parse().ok().filter(|q: &f64| q.is_finite())?;
    (status_of(&f)? != Status::Disconnected).then_some(q)
}

/// Đọc mực nước (kênh 16) và lưu lượng hai tổ máy (kênh 5, 6) từ phản hồi của thiết bị.
pub fn parse_mntl(text: &str) -> Result<Reading, LoggerError> {
    let f = channel_fields(text, CHANNEL).ok_or(LoggerError::MissingChannel)?;
    let z: f64 = f.get(3).and_then(|v| v.parse().ok()).ok_or(LoggerError::BadResponse)?;
    if !z.is_finite() {
        return Err(LoggerError::BadResponse);
    }
    let status = status_of(&f).ok_or(LoggerError::BadResponse)?;
    Ok(Reading { z, status, q_h1: turbine_flow(text, "5"), q_h2: turbine_flow(text, "6") })
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
    fn missing_disconnected_or_bad_turbine_channels_are_none_not_errors() {
        let r = parse_mntl(SAMPLE).unwrap();
        assert_eq!((r.q_h1, r.q_h2), (None, None));
        let r = parse_mntl(&format!("a,5,M1,16.54,m3/s,0,0,0\na,6,M2,abc,m3/s,1,0,0\n{SAMPLE}")).unwrap();
        assert_eq!((r.q_h1, r.q_h2), (None, None));
    }

    #[test]
    fn running_means_more_than_one_cubic_metre_per_second() {
        assert!(is_running(16.54) && is_running(33.0));
        assert!(!is_running(0.0) && !is_running(0.3) && !is_running(1.0));
    }

    #[test]
    fn finds_channel_16_among_many_lines() {
        let text = format!("1/ 0/ 0,15,Kenh khac,9.9,m,1,0,0\n{SAMPLE}\n1/ 0/ 0,17,Khac,1.0,m,1,0,0\n");
        assert_eq!(parse_mntl(&text).unwrap().z, 157.192);
    }

    #[test]
    fn status_zero_is_disconnected() {
        let r = parse_mntl("160.8/ 157.2/ 1.0,16,MNTL hien tai,157.192,m,0,0,0").unwrap();
        assert_eq!(r.status, Status::Disconnected);
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
