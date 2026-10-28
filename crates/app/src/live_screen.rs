//! Thẻ trực tuyến trong màn Tính toán: mực nước, tổ máy H1/H2 và lưu lượng về theo chu kỳ, đọc từ data logger ECAPRO.

use std::time::Duration;

use app::form::Field;
use app::live::{history_record, next_boundary, FlowView, VN_TZ};
use app::paint::{self, Icon};
use app::view;
use eframe::egui::{self, FontId, RichText};
use luuluong_core::validate::format_vn;
use luuluong_logger::Status;

use super::{card, theme, title, unix_now, App, Palette};

fn big(p: &Palette, text: String) -> RichText {
    RichText::new(text).font(FontId::new(34.0, theme::semibold())).color(p.accent_text)
}

impl App {
    /// Nhận các lần đọc từ luồng nền vào `Live`.
    pub(super) fn poll_live(&mut self) {
        while let Ok((now, result)) = self.live_rx.try_recv() {
            // Chi tiết kỹ thuật chỉ ghi log, và chỉ khi lỗi đổi (tránh ghi lặp mỗi vài giây).
            if let Err(e) = &result {
                if self.live.error() != Some(e) {
                    eprintln!("không đọc được thiết bị ECAPRO: {e}");
                }
            }
            self.live.record(now, result);
        }
        if let (Some(flow), Some(h)) = (self.live.take_unlogged(&self.table), &self.history) {
            let record = history_record(&flow, self.live.period(), self.config.logger.eco_q);
            match h.insert(&record) {
                Ok(_) => {
                    self.history_dirty = true;
                    self.storage_warning = None;
                }
                Err(e) => self.storage_warning = Some(e.to_string()),
            }
        }
    }

    /// Áp cấu hình `[logger]` lên trạng thái và luồng nền.
    pub(super) fn sync_live(&mut self) {
        let l = &self.config.logger;
        self.live.set_period(l.period());
        self.live.set_eco_q(l.eco_q);
        if let Ok(mut t) = self.live_target.lock() {
            t.url = l.url.clone();
            t.poll = Duration::from_secs(l.poll().into());
        }
    }

    /// Nút điền mực nước từ thiết bị vào form Tính bằng tay; đọc không được thì nút tắt kèm lý do.
    pub(super) fn device_buttons(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let level = self.live.level().filter(|_| !self.live.stale(unix_now(), self.config.logger.poll()));
        let mark = self.live.last_mark();
        ui.horizontal_wrapped(|ui| {
            let now_btn = ui.add_enabled(level.is_some(), egui::Button::new("Lấy mực nước hiện tại"));
            if let (true, Some((_, z))) = (now_btn.on_hover_text("Điền vào ô mực nước cuối").clicked(), level) {
                self.form.set_level(Field::ZEnd, z);
            }
            let label = mark.map_or("Lấy mực nước chu kỳ trước".to_string(), |m| format!("Lấy mực nước lúc {}", view::clock(m.time, VN_TZ)));
            let prev_btn = ui.add_enabled(mark.is_some(), egui::Button::new(label));
            if let (true, Some(m)) = (prev_btn.on_hover_text("Điền vào ô mực nước đầu (mốc đồng hồ gần nhất)").clicked(), mark) {
                self.form.set_level(Field::ZStart, m.z);
            }
        });
        if level.is_none() {
            ui.label(RichText::new("Chưa có mực nước từ thiết bị ECAPRO, nhập tay.").size(12.0).color(p.muted));
        }
    }

    /// Thẻ trực tuyến ở đầu màn Tính toán: mực nước, tổ máy H1/H2, lưu lượng về theo chu kỳ.
    pub(super) fn live_panel(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let now = unix_now();
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Số liệu trực tuyến từ thiết bị ECAPRO");
            ui.add_space(8.0);
            ui.columns(3, |cols| {
                self.level_column(&mut cols[0], p, now);
                self.machines_column(&mut cols[1], p, now);
                self.flow_column(&mut cols[2], p, now);
            });
            ui.add_space(8.0);
            let note = format!(
                "Lưu lượng về = (thể tích hồ ở mốc sau − mốc trước) ÷ thời gian + lưu lượng qua tổ máy (trung bình đo trong chu kỳ) + {} m³/s xả môi trường. \
Mốc theo đồng hồ; mực nước mỗi mốc là trung vị các lần đọc trong 1 phút trước mốc.",
                format_vn(self.config.logger.eco_q, 1)
            );
            ui.label(RichText::new(note).size(12.0).color(p.muted));
        });
    }

    fn level_column(&mut self, ui: &mut egui::Ui, p: &Palette, now: i64) {
        let poll = self.config.logger.poll();
        ui.label(RichText::new("Mực nước hồ hiện tại").font(FontId::new(13.0, theme::semibold())).color(p.primary));
        match self.live.level() {
            Some((_, z)) => ui.label(big(p, format!("{} m", format_vn(z, 3)))),
            None => ui.label(big(p, "—".into())),
        };
        if let Some((t, _)) = self.live.level() {
            ui.label(RichText::new(format!("Đọc lúc {}", view::clock_secs(t, VN_TZ))).size(13.0).color(p.muted));
        }
        let (icon, color, text) = if let Some(e) = self.live.error() {
            (Icon::Warn, p.error, e.to_string())
        } else {
            match self.live.status() {
                Some(Status::Normal) => (Icon::Ok, p.primary, "Thiết bị đo hoạt động bình thường.".into()),
                Some(Status::Low) => (Icon::Warn, p.error, "Thiết bị báo mực nước thấp.".into()),
                Some(Status::High) => (Icon::Warn, p.error, "Thiết bị báo mực nước cao.".into()),
                Some(Status::Disconnected) => (Icon::Warn, p.error, "Cảm biến mực nước không kết nối.".into()),
                None => (Icon::Info, p.muted, "Đang chờ dữ liệu từ thiết bị…".into()),
            }
        };
        paint::note(ui, icon, color, 13.0, RichText::new(text));
        if self.live.level().is_some() && self.live.stale(now, poll) {
            paint::note(ui, Icon::Warn, p.error, 13.0, RichText::new("Dữ liệu cũ: chưa có lần đọc mới."));
        }
    }

    fn machines_column(&mut self, ui: &mut egui::Ui, p: &Palette, now: i64) {
        ui.label(RichText::new("Tổ máy").font(FontId::new(13.0, theme::semibold())).color(p.primary));
        let Some((t, m)) = self.live.machines() else {
            ui.label(big(p, "—".into()));
            paint::note(ui, Icon::Info, p.muted, 13.0, RichText::new("Chưa có lưu lượng tổ máy từ thiết bị."));
            return;
        };
        for (name, machine) in [("H1", m.h1), ("H2", m.h2)] {
            ui.horizontal(|ui| {
                ui.label(RichText::new(name).font(FontId::new(22.0, theme::semibold())).color(p.text));
                let (icon, color, state) = if machine.running { (Icon::Ok, p.primary, "Đang chạy") } else { (Icon::Info, p.muted, "Dừng") };
                paint::note(ui, icon, color, 15.0, RichText::new(state).font(FontId::new(15.0, theme::semibold())));
            });
            ui.label(RichText::new(format!("{} m³/s", format_vn(machine.q, 2))).size(15.0).color(p.accent_text));
        }
        if now - t > 3 * i64::from(self.config.logger.poll()) {
            paint::note(ui, Icon::Warn, p.error, 13.0, RichText::new("Dữ liệu cũ."));
        }
    }

    fn flow_column(&mut self, ui: &mut egui::Ui, p: &Palette, now: i64) {
        ui.label(RichText::new("Lưu lượng về hồ").font(FontId::new(13.0, theme::semibold())).color(p.primary));
        let period = self.live.period();
        let clock = |t: i64| view::clock(t, VN_TZ);
        let (value, icon, color, text) = match self.live.flow(&self.table) {
            FlowView::Ready(f) => {
                let sign = if f.dz >= 0.0 { "+" } else { "" };
                (
                    format!("{} m³/s", format_vn(f.q_in, 2)),
                    Icon::Info,
                    p.muted,
                    format!(
                        "Chu kỳ {} → {}: mực nước {sign}{} m; tổ máy trung bình H1 {} · H2 {} m³/s.",
                        clock(f.from.time),
                        clock(f.to.time),
                        format_vn(f.dz, 3),
                        format_vn(f.turbine.q_h1, 2),
                        format_vn(f.turbine.q_h2, 2)
                    ),
                )
            }
            FlowView::Waiting => ("—".into(), Icon::Info, p.muted, format!("Đang chờ đủ hai mốc. Mốc kế tiếp lúc {}.", clock(next_boundary(now, period)))),
            FlowView::Missing { to } => (
                "—".into(),
                Icon::Warn,
                p.error,
                format!("Thiếu số liệu mực nước hoặc lưu lượng tổ máy ở chu kỳ kết thúc lúc {}, chưa tính được. Chu kỳ kế tiếp sẽ tính lại.", clock(to)),
            ),
            FlowView::Invalid(message) => ("—".into(), Icon::Warn, p.error, message),
        };
        ui.label(big(p, value));
        paint::note(ui, icon, color, 13.0, RichText::new(text));
        let mut chosen = period;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Chu kỳ tính").size(13.0).color(p.muted));
            ui.selectable_value(&mut chosen, 15, "15 phút");
            ui.selectable_value(&mut chosen, 30, "30 phút");
        });
        if chosen != period {
            self.config.logger.period_minutes = chosen;
            self.sync_live();
            let _ = self.save_config();
        }
    }
}
