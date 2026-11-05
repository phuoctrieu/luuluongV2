//! Đăng nhập Vrain (cookie `sid`) và lấy mưa theo giờ. Không log header, cookie hay body.

use std::fmt;
use std::time::Duration;

use crate::{parse_details, ParseOutcome, VrainError};

const USER_AGENT: &str =
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/156.0.0.0 Safari/537.36";
const TIMEOUT: Duration = Duration::from_secs(15);
const ATTEMPTS: u32 = 3;

#[derive(Clone, PartialEq)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials").field("username", &self.username).field("password", &"***").finish()
    }
}

pub struct Client {
    base: String,
    org_uuid: String,
    agent: ureq::Agent,
    sid: Option<String>,
    delay: Duration,
}

/// Kết quả một bước gọi mạng.
enum Step {
    /// 401 khi lấy dữ liệu: phiên hết hạn.
    Unauthorized,
    /// Lỗi mạng hoặc 5xx: đáng thử lại.
    Transient,
    /// Lỗi dứt khoát: dừng.
    Fail(VrainError),
}

impl Client {
    pub fn new(base: &str, org_uuid: &str) -> Client {
        Client {
            base: base.trim_end_matches('/').to_string(),
            org_uuid: org_uuid.to_string(),
            agent: ureq::AgentBuilder::new().timeout(TIMEOUT).build(),
            sid: None,
            delay: Duration::from_secs(1),
        }
    }

    /// Đổi thời gian chờ giữa các lần thử lại (mặc định 1 giây, lần sau gấp đôi); dùng cho test.
    pub fn retry_delay(mut self, d: Duration) -> Client {
        self.delay = d;
        self
    }

    /// Lấy mưa theo giờ từ ngày `from` tới `to` (`YYYY-MM-DD`). Mất phiên thì đăng nhập lại đúng một lần.
    pub fn fetch_hourly(
        &mut self,
        c: &Credentials,
        from: &str,
        to: &str,
        ids: &[String],
        now: i64,
    ) -> Result<ParseOutcome, VrainError> {
        for attempt in 0..ATTEMPTS {
            match self.attempt(c, from, to) {
                Ok(body) => return parse_details(&body, ids, now),
                Err(Step::Fail(e)) => return Err(e),
                Err(Step::Unauthorized) => return Err(VrainError::Credentials),
                Err(Step::Transient) => {
                    if attempt + 1 < ATTEMPTS {
                        std::thread::sleep(self.delay * (attempt + 1));
                    }
                }
            }
        }
        Err(VrainError::Unreachable)
    }

    fn attempt(&mut self, c: &Credentials, from: &str, to: &str) -> Result<String, Step> {
        if self.sid.is_none() {
            self.login(c)?;
        }
        match self.details(from, to) {
            Err(Step::Unauthorized) => {
                self.sid = None;
                self.login(c)?;
                self.details(from, to)
            }
            other => other,
        }
    }

    fn login(&mut self, c: &Credentials) -> Result<(), Step> {
        let resp = self
            .agent
            .post(&format!("{}/api/vrain/public/v1/login", self.base))
            .set("x-org-uuid", &self.org_uuid)
            .set("x-vrain-user-agent", USER_AGENT)
            .set("User-Agent", USER_AGENT)
            .set("Origin", "https://www.vrain.vn")
            .set("Referer", "https://www.vrain.vn/landing")
            .send_json(ureq::json!({"username": c.username, "password": c.password}));
        match resp {
            Ok(r) => {
                let sid = r.all("set-cookie").iter().find_map(|v| {
                    v.split(';').next().and_then(|kv| kv.trim().strip_prefix("sid=")).map(str::to_string)
                });
                match sid {
                    Some(sid) if !sid.is_empty() => {
                        self.sid = Some(sid);
                        Ok(())
                    }
                    _ => Err(Step::Fail(VrainError::Format)),
                }
            }
            Err(e) => Err(classify(e, true)),
        }
    }

    fn details(&self, from: &str, to: &str) -> Result<String, Step> {
        let sid = self.sid.as_deref().unwrap_or_default();
        let resp = self
            .agent
            .get(&format!("{}/api/kttv/private/v1/organizations/details", self.base))
            .query("from", from)
            .query("to", to)
            .query("i", "_1h")
            .set("Cookie", &format!("sid={sid}"))
            .set("x-org-uuid", &self.org_uuid)
            .set("x-vrain-user-agent", USER_AGENT)
            .set("User-Agent", USER_AGENT)
            .set("Origin", "https://www.vrain.vn")
            .set("Referer", "https://www.vrain.vn/")
            .call();
        match resp {
            Ok(r) => r.into_string().map_err(|_| Step::Transient),
            Err(e) => Err(classify(e, false)),
        }
    }
}

fn classify(e: ureq::Error, is_login: bool) -> Step {
    match e {
        ureq::Error::Status(401 | 403, _) if !is_login => Step::Unauthorized,
        ureq::Error::Status(400 | 401 | 403, _) => Step::Fail(VrainError::Credentials),
        ureq::Error::Status(code, _) if code >= 500 => Step::Transient,
        ureq::Error::Status(..) => Step::Fail(VrainError::Unreachable),
        ureq::Error::Transport(_) => Step::Transient,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use mockito::{Matcher, Server};

    const FIXTURE: &str = include_str!("../tests/data/vrain_details.json");
    const LOGIN: &str = "/api/vrain/public/v1/login";
    const DETAILS: &str = "/api/kttv/private/v1/organizations/details";
    const ORG: &str = "org-1";

    fn creds() -> Credentials {
        Credentials { username: "u".into(), password: "hunter2".into() }
    }

    fn ids() -> Vec<String> {
        vec!["841697470576".into(), "84396786852".into(), "84846302387".into()]
    }

    fn now() -> i64 {
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap().and_hms_opt(10, 5, 0).unwrap().and_utc().timestamp() - 7 * 3600
    }

    fn client(server: &Server) -> Client {
        Client::new(&server.url(), ORG).retry_delay(Duration::from_millis(1))
    }

    fn fetch(c: &mut Client) -> Result<ParseOutcome, VrainError> {
        c.fetch_hourly(&creds(), "2026-11-05", "2026-11-05", &ids(), now())
    }

    #[test]
    fn logs_in_then_fetches() {
        let mut s = Server::new();
        let login = s
            .mock("POST", LOGIN)
            .match_header("x-org-uuid", ORG)
            .match_body(Matcher::Json(serde_json::json!({"username":"u","password":"hunter2"})))
            .with_status(200)
            .with_header("set-cookie", "sid=abc; Path=/; HttpOnly")
            .with_body("{}")
            .expect(1)
            .create();
        let details = s
            .mock("GET", DETAILS)
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("from".into(), "2026-11-05".into()),
                Matcher::UrlEncoded("to".into(), "2026-11-05".into()),
                Matcher::UrlEncoded("i".into(), "_1h".into()),
            ]))
            .match_header("cookie", "sid=abc")
            .match_header("x-org-uuid", ORG)
            .with_status(200)
            .with_body(FIXTURE)
            .expect(1)
            .create();
        let out = fetch(&mut client(&s)).unwrap();
        assert_eq!(out.obs.len(), 6);
        login.assert();
        details.assert();
    }

    #[test]
    fn relogins_once_on_401() {
        let mut s = Server::new();
        let login_old = s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=old; Path=/").expect(1).create();
        let login_new = s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=new; Path=/").expect(1).create();
        let expired = s.mock("GET", DETAILS).match_query(Matcher::Any).match_header("cookie", "sid=old").with_status(401).expect(1).create();
        let ok = s.mock("GET", DETAILS).match_query(Matcher::Any).match_header("cookie", "sid=new").with_status(200).with_body(FIXTURE).expect(1).create();
        let out = fetch(&mut client(&s)).unwrap();
        assert_eq!(out.obs.len(), 6);
        login_old.assert();
        login_new.assert();
        expired.assert();
        ok.assert();
    }

    #[test]
    fn no_endless_relogin() {
        let mut s = Server::new();
        let login = s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=abc; Path=/").expect(2).create();
        let details = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(401).expect(2).create();
        assert_eq!(fetch(&mut client(&s)), Err(VrainError::Credentials));
        login.assert();
        details.assert();
    }

    #[test]
    fn wrong_password() {
        let mut s = Server::new();
        let login = s.mock("POST", LOGIN).with_status(401).with_body(r#"{"code":"x"}"#).expect(1).create();
        assert_eq!(fetch(&mut client(&s)), Err(VrainError::Credentials));
        login.assert();
    }

    #[test]
    fn server_500_retries_then_unreachable() {
        let mut s = Server::new();
        let _login = s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=abc; Path=/").create();
        let details = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(500).expect(3).create();
        assert_eq!(fetch(&mut client(&s)), Err(VrainError::Unreachable));
        details.assert();
    }

    #[test]
    fn unreachable_server_is_unreachable() {
        let mut c = Client::new("http://127.0.0.1:1", ORG).retry_delay(Duration::from_millis(1));
        assert_eq!(fetch(&mut c), Err(VrainError::Unreachable));
    }

    #[test]
    fn bad_json_is_format() {
        let mut s = Server::new();
        let _login = s.mock("POST", LOGIN).with_status(200).with_header("set-cookie", "sid=abc; Path=/").create();
        let details = s.mock("GET", DETAILS).match_query(Matcher::Any).with_status(200).with_body("xin chào").expect(1).create();
        assert_eq!(fetch(&mut client(&s)), Err(VrainError::Format));
        details.assert();
    }

    #[test]
    fn debug_hides_password() {
        let text = format!("{:?}", creds());
        assert!(!text.contains("hunter2"), "{text}");
    }
}
