//! Luồng nền lấy mưa Vrain định kỳ và khi có tín hiệu "Cập nhật ngay"; giao diện nhận qua `mpsc`.

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{vn_date, Client, Credentials, ParseOutcome, VrainError};

/// Cấu hình đọc lại mỗi vòng, nên đổi trong Cài đặt có hiệu lực ngay. `Debug` che mật khẩu (qua `Credentials`).
#[derive(Debug, Clone)]
pub struct Target {
    pub base: String,
    pub org_uuid: String,
    pub creds: Credentials,
    pub ids: Vec<String>,
    pub every: Duration,
    /// Số ngày lấy bù ở lần đầu thành công.
    pub backfill_days: u32,
}

/// Giây epoch lúc lấy xong và kết quả.
pub type PollResult = (i64, Result<ParseOutcome, VrainError>);

/// Chạy tới khi `tx` hoặc `trigger` bị bỏ. `wake` gọi sau mỗi lần gửi (vd. `ctx.request_repaint()`).
pub fn spawn(
    target: Arc<Mutex<Target>>,
    tx: Sender<PollResult>,
    trigger: Receiver<()>,
    wake: impl Fn() + Send + 'static,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut client: Option<((String, String), Client)> = None;
        let mut backfilled = false;
        loop {
            let t = target.lock().map(|t| t.clone()).unwrap_or_else(|e| e.into_inner().clone());
            let key = (t.base.clone(), t.org_uuid.clone());
            if client.as_ref().is_none_or(|(k, _)| *k != key) {
                client = Some((key, Client::new(&t.base, &t.org_uuid)));
            }
            let now = unix_now();
            let to = vn_date(now);
            let from = if backfilled { to.clone() } else { vn_date(now - i64::from(t.backfill_days) * 86_400) };
            let result = match client.as_mut() {
                Some((_, c)) => c.fetch_hourly(&t.creds, &from, &to, &t.ids, now),
                None => Err(VrainError::Unreachable),
            };
            backfilled |= result.is_ok();
            if tx.send((now, result)).is_err() {
                break;
            }
            wake();
            match trigger.recv_timeout(t.every) {
                Ok(()) => while trigger.try_recv().is_ok() {},
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
    })
}

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vn_date;
    use mockito::{Matcher, Server};
    use std::sync::mpsc::channel;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    const FIXTURE: &str = include_str!("../tests/data/vrain_details.json");
    const LOGIN: &str = "/api/vrain/public/v1/login";
    const DETAILS: &str = "/api/kttv/private/v1/organizations/details";

    fn unix_now() -> i64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64
    }

    fn target(url: &str) -> Arc<Mutex<Target>> {
        Arc::new(Mutex::new(Target {
            base: url.into(),
            org_uuid: "org-1".into(),
            creds: Credentials { username: "u".into(), password: "hunter2".into() },
            ids: vec!["841697470576".into()],
            every: Duration::from_secs(3600),
            backfill_days: 2,
        }))
    }

    fn login_mock(s: &mut Server) -> mockito::Mock {
        s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=abc; Path=/").create()
    }

    #[test]
    fn fetches_on_start_and_on_trigger_and_wakes_the_ui() {
        let mut s = Server::new();
        let _l = login_mock(&mut s);
        let details = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(200).with_body(FIXTURE).expect_at_least(2).create();
        let (tx, rx) = channel();
        let (trig, trig_rx) = channel();
        let wakes = Arc::new(AtomicUsize::new(0));
        let w = wakes.clone();
        spawn(target(&s.url()), tx, trig_rx, move || {
            w.fetch_add(1, Ordering::SeqCst);
        });
        let (t, r) = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(t > 1_700_000_000);
        assert!(r.is_ok());
        trig.send(()).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).unwrap().1.is_ok());
        assert!(wakes.load(Ordering::SeqCst) >= 2);
        details.assert();
    }

    #[test]
    fn stops_when_receiver_dropped() {
        let mut s = Server::new();
        let _l = login_mock(&mut s);
        let _d = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(200).with_body(FIXTURE).create();
        let (tx, rx) = channel();
        let (trig, trig_rx) = channel();
        let handle = spawn(target(&s.url()), tx, trig_rx, || {});
        let _first = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(rx);
        trig.send(()).unwrap();
        handle.join().unwrap();
    }

    #[test]
    fn stops_when_trigger_dropped() {
        let mut s = Server::new();
        let _l = login_mock(&mut s);
        let _d = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(200).with_body(FIXTURE).create();
        let (tx, rx) = channel();
        let (trig, trig_rx) = channel();
        let handle = spawn(target(&s.url()), tx, trig_rx, || {});
        let _first = rx.recv_timeout(Duration::from_secs(3)).unwrap();
        drop(trig);
        handle.join().unwrap();
    }

    #[test]
    fn target_debug_hides_password() {
        let text = format!("{:?}", target("http://x").lock().unwrap().clone());
        assert!(!text.contains("hunter2"), "{text}");
    }

    #[test]
    fn first_fetch_backfills_then_only_today() {
        let mut s = Server::new();
        let _l = login_mock(&mut s);
        let today = vn_date(unix_now());
        let backfill_from = vn_date(unix_now() - 2 * 86_400);
        let first = s
            .mock("GET", DETAILS)
            .match_query(Matcher::UrlEncoded("from".into(), backfill_from))
            .with_status(200)
            .with_body(FIXTURE)
            .expect(1)
            .create();
        let later = s
            .mock("GET", DETAILS)
            .match_query(Matcher::UrlEncoded("from".into(), today))
            .with_status(200)
            .with_body(FIXTURE)
            .expect(1)
            .create();
        let (tx, rx) = channel();
        let (trig, trig_rx) = channel();
        spawn(target(&s.url()), tx, trig_rx, || {});
        assert!(rx.recv_timeout(Duration::from_secs(3)).unwrap().1.is_ok());
        trig.send(()).unwrap();
        assert!(rx.recv_timeout(Duration::from_secs(3)).unwrap().1.is_ok());
        first.assert();
        later.assert();
    }
}
