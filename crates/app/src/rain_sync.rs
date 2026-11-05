//! Nối luồng nền Vrain vào app: áp cấu hình, nhận kết quả, ghi SQLite. Giao diện chỉ đọc trạng thái ở đây.

use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use luuluong_storage::history::RainObs;
use luuluong_vrain::{Credentials, Target};

use super::{unix_now, App};

const BASE_URL: &str = "https://www.vrain.vn";
/// Số ngày mưa nạp từ SQLite để hiển thị (đủ cho biểu đồ 7 ngày + dư).
const KEEP_SECS: i64 = 8 * 86_400;
const BACKFILL_DAYS: u32 = 2;

impl App {
    /// Tài khoản: Cài đặt; biến môi trường `VRAIN_USER` chỉ để dev/test.
    pub(super) fn vrain_username(&self) -> String {
        let u = self.config.vrain.username.trim();
        if u.is_empty() { std::env::var("VRAIN_USER").unwrap_or_default() } else { u.to_string() }
    }

    pub(super) fn rain_configured(&self) -> bool {
        self.config.vrain.enabled && !self.vrain_username().is_empty() && !self.vrain_password.is_empty()
    }

    pub(super) fn load_rain(&mut self, now: i64) {
        if let Some(h) = &self.history {
            match h.rain_since(now - KEEP_SECS) {
                Ok(v) => self.rain_obs = v,
                Err(e) => self.storage_warning = Some(e.to_string()),
            }
        }
    }

    /// Áp cấu hình Vrain: bật/tắt luồng nền, cập nhật tài khoản và chu kỳ (luồng đọc lại mỗi vòng).
    pub(super) fn sync_vrain(&mut self, ctx: &egui::Context) {
        if !self.rain_configured() {
            // Bỏ `trigger` thì luồng nền tự dừng.
            self.vrain_target = None;
            self.vrain_trigger = None;
            self.vrain_rx = None;
            return;
        }
        let target = Target {
            base: BASE_URL.into(),
            org_uuid: self.config.vrain.org_uuid.clone(),
            creds: Credentials { username: self.vrain_username(), password: self.vrain_password.clone() },
            ids: self.config.vrain.stations.iter().map(|s| s.id.clone()).collect(),
            every: Duration::from_secs(u64::from(self.config.vrain.refresh()) * 60),
            backfill_days: BACKFILL_DAYS,
        };
        if let Some(t) = &self.vrain_target {
            if let Ok(mut guard) = t.lock() {
                *guard = target;
            }
            return;
        }
        let shared = Arc::new(Mutex::new(target));
        let (tx, rx) = channel();
        let (trigger_tx, trigger_rx) = channel();
        let wake = ctx.clone();
        luuluong_vrain::spawn(shared.clone(), tx, trigger_rx, move || wake.request_repaint());
        self.vrain_target = Some(shared);
        self.vrain_trigger = Some(trigger_tx);
        self.vrain_rx = Some(rx);
    }

    /// Nút "Cập nhật ngay": `true` nếu đã gửi yêu cầu (cổng cho phép).
    pub(super) fn request_rain_refresh(&mut self) -> bool {
        if !self.rain_gate.try_start(unix_now(), self.rain_configured()) {
            return false;
        }
        match &self.vrain_trigger {
            Some(t) if t.send(()).is_ok() => true,
            _ => {
                self.rain_gate.finish();
                false
            }
        }
    }

    pub(super) fn poll_vrain(&mut self) {
        let Some(rx) = &self.vrain_rx else { return };
        let batch: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        for (now, result) in batch {
            self.rain_gate.finish();
            match result {
                Ok(out) => {
                    if !out.missing.is_empty() {
                        eprintln!("Vrain: thiếu trạm {:?} trong phản hồi", out.missing);
                    }
                    let obs: Vec<RainObs> =
                        out.obs.into_iter().map(|o| RainObs { station_id: o.station_id, ts: o.ts, mm: o.mm }).collect();
                    match self.history.as_ref().map(|h| h.upsert_rain(&obs)) {
                        Some(Err(e)) => self.storage_warning = Some(e.to_string()),
                        Some(Ok(_)) => {}
                        None => self.rain_obs = obs, // không có SQLite: giữ trong bộ nhớ
                    }
                    self.rain_last_ok = Some(now);
                    self.rain_error = None;
                    self.load_rain(now);
                }
                Err(e) => {
                    // Chi tiết chỉ ghi log, và chỉ khi lỗi đổi.
                    let text = e.to_string();
                    if self.rain_error.as_deref() != Some(text.as_str()) {
                        eprintln!("Vrain: {text}");
                    }
                    self.rain_error = Some(text);
                }
            }
        }
    }
}
