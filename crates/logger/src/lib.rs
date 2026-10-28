//! Đọc mực nước hồ "MNTL hien tai" (kênh 16) từ data logger ECAPRO GPIs6.6DA
//! qua macro WebIOPi `/macros/UpdateMonitor` (xem `huong-dan-lay-mntl-hien-tai.md`).

use std::time::Duration;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reading {
    /// Mực nước hồ (m).
    pub z: f64,
    pub status: Status,
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

/// Tìm dòng kênh 16 trong phản hồi: `max/min/avg,số kênh,tên,giá trị,đơn vị,trạng thái,...`.
pub fn parse_mntl(text: &str) -> Result<Reading, LoggerError> {
    let line = text
        .lines()
        .find(|l| l.split(',').nth(1).is_some_and(|c| c.trim() == CHANNEL))
        .ok_or(LoggerError::MissingChannel)?;
    let f: Vec<&str> = line.split(',').map(str::trim).collect();
    let z: f64 = f.get(3).and_then(|v| v.parse().ok()).ok_or(LoggerError::BadResponse)?;
    if !z.is_finite() {
        return Err(LoggerError::BadResponse);
    }
    let status = match f.get(5).copied() {
        Some("0") => Status::Disconnected,
        Some("1") => Status::Normal,
        Some("2") => Status::Low,
        Some("3") => Status::High,
        _ => return Err(LoggerError::BadResponse),
    };
    Ok(Reading { z, status })
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
        assert_eq!(r, Reading { z: 157.192, status: Status::Normal });
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
