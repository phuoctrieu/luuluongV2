//! Luồng nền đọc thiết bị định kỳ; UI nhận kết quả qua `mpsc` (không bao giờ chặn giao diện).

use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{fetch, LoggerError, Reading};

const FETCH_TIMEOUT: Duration = Duration::from_secs(5);

/// Địa chỉ và chu kỳ đọc; UI sửa được khi người dùng đổi Cài đặt, luồng đọc lại mỗi vòng.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub url: String,
    pub poll: Duration,
}

/// Một lần đọc: giây epoch lúc đọc xong và kết quả.
pub type Sample = (i64, Result<Reading, LoggerError>);

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

/// Chạy tới khi bên nhận (`Receiver`) bị bỏ. `wake` gọi sau mỗi lần gửi (vd. `ctx.request_repaint()`).
pub fn spawn(target: Arc<Mutex<Target>>, tx: Sender<Sample>, wake: impl Fn() + Send + 'static) -> JoinHandle<()> {
    std::thread::spawn(move || loop {
        let Target { url, poll } = target.lock().map(|t| t.clone()).unwrap_or_else(|e| e.into_inner().clone());
        let result = fetch(&url, FETCH_TIMEOUT);
        if tx.send((unix_now(), result)).is_err() {
            break;
        }
        wake();
        std::thread::sleep(poll);
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::channel;

    #[test]
    fn sends_repeated_samples_and_wakes_the_ui() {
        let mut server = mockito::Server::new();
        let _m = server
            .mock("POST", "/m")
            .with_status(200)
            .with_body("a,16,M,157.192,m,1,0,0")
            .expect_at_least(2)
            .create();
        let target = Arc::new(Mutex::new(Target { url: format!("{}/m", server.url()), poll: Duration::from_millis(30) }));
        let (tx, rx) = channel();
        let wakes = Arc::new(AtomicUsize::new(0));
        let w = wakes.clone();
        spawn(target, tx, move || {
            w.fetch_add(1, Ordering::SeqCst);
        });
        for _ in 0..2 {
            let (t, r) = rx.recv_timeout(Duration::from_secs(3)).unwrap();
            assert!(t > 1_700_000_000);
            assert_eq!(r.unwrap().z, 157.192);
        }
        assert!(wakes.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn reports_errors_and_stops_when_receiver_is_dropped() {
        let target = Arc::new(Mutex::new(Target { url: "http://127.0.0.1:1/x".into(), poll: Duration::from_millis(10) }));
        let (tx, rx) = channel();
        let handle = spawn(target, tx, || {});
        let (_, r) = rx.recv_timeout(Duration::from_secs(8)).unwrap();
        assert_eq!(r.unwrap_err(), LoggerError::Unreachable);
        drop(rx);
        handle.join().unwrap();
    }

    #[test]
    fn picks_up_a_changed_target_on_the_next_round() {
        let mut server = mockito::Server::new();
        let _m = server.mock("POST", "/new").with_status(200).with_body("a,16,M,150.5,m,1,0,0").create();
        let target = Arc::new(Mutex::new(Target { url: "http://127.0.0.1:1/x".into(), poll: Duration::from_millis(20) }));
        let (tx, rx) = channel();
        spawn(target.clone(), tx, || {});
        assert!(rx.recv_timeout(Duration::from_secs(8)).unwrap().1.is_err());
        target.lock().unwrap().url = format!("{}/new", server.url());
        let ok = (0..50).find_map(|_| rx.recv_timeout(Duration::from_secs(3)).ok().and_then(|(_, r)| r.ok()));
        assert_eq!(ok.unwrap().z, 150.5);
    }
}
