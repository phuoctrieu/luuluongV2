//! Thẻ trực tuyến trong màn Tính toán: mực nước, tổ máy H1/H2 và lưu lượng về theo chu kỳ, đọc từ data logger ECAPRO.

use std::time::Duration;

use app::averages::{daily_stats, window_averages, WINDOWS};
use app::form::Field;
use app::live::{history_record, next_boundary, FlowView, VN_TZ};
use app::paint::{self, Icon};
use app::view;
use eframe::egui::{self, FontId, RichText};
use luuluong_core::validate::format_vn;
use luuluong_logger::Status;

use super::{card, theme, title, unix_now, App, Palette};

/// Số ngày tối đa tải cho biểu đồ ngày.
const MAX_DAYS: usize = 30;

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
            let record = history_record(&flow, self.live.period());
            match h.insert(&record) {
                Ok(_) => {
                    self.history_dirty = true;
                    self.averages_at = 0;
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
            ui.separator();
            self.averages_row(ui, p, now);
            ui.add_space(8.0);
            let note = "Lưu lượng về = (thể tích hồ ở mốc sau − mốc trước) ÷ thời gian + lưu lượng qua tổ máy + xả môi trường (cả hai lấy trung bình số đo trong chu kỳ). \
Mốc theo đồng hồ; mực nước mỗi mốc là trung vị các lần đọc trong 1 phút trước mốc.";
            ui.label(RichText::new(note).size(12.0).color(p.muted));
        });
    }

    /// Tải lại lịch sử và tính trung bình mỗi phút, hoặc ngay khi có chu kỳ mới được lưu.
    fn refresh_averages(&mut self, now: i64) {
        if now - self.averages_at < 60 {
            return;
        }
        self.averages_at = now;
        let Some(h) = &self.history else { return };
        match h.list_since(now - WINDOWS[WINDOWS.len() - 1].1) {
            Ok(rows) => {
                self.averages = window_averages(&rows, now);
                self.daily = daily_stats(&rows, now, MAX_DAYS);
            }
            Err(e) => self.storage_warning = Some(e.to_string()),
        }
    }

    /// Thẻ biểu đồ từng ngày: lưu lượng về trung bình và lượng mưa.
    pub(super) fn daily_card(&mut self, ui: &mut egui::Ui, p: &Palette) {
        self.refresh_averages(unix_now());
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                title(ui, p, "Lưu lượng về và mưa theo ngày");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    for days in [30, 14, 7] {
                        ui.selectable_value(&mut self.daily_range, days, format!("{days} ngày"));
                    }
                });
            });
            ui.add_space(6.0);
            let from = self.daily.len().saturating_sub(self.daily_range);
            app::chart::daily_chart(ui, p, &self.daily[from..], 130.0);
            ui.label(
                RichText::new("Mỗi cột là một ngày (giờ Việt Nam). Trung bình và mưa chỉ tính phần thời gian app đã chạy và có số liệu; ngày chưa có số liệu để trống.")
                    .size(12.0)
                    .color(p.muted),
            );
        });
    }

    fn averages_row(&mut self, ui: &mut egui::Ui, p: &Palette, now: i64) {
        self.refresh_averages(now);
        ui.label(RichText::new("Lưu lượng về trung bình và lượng mưa cộng dồn").font(FontId::new(13.0, theme::semibold())).color(p.primary));
        ui.add_space(4.0);
        ui.columns(self.averages.len().max(1), |cols| {
            for (col, w) in cols.iter_mut().zip(&self.averages) {
                col.label(RichText::new(w.label).size(13.0).color(p.muted));
                let value = w.q_avg.map_or("—".to_string(), |q| format!("{} m³/s", format_vn(q, 2)));
                col.label(RichText::new(value).font(FontId::new(19.0, theme::semibold())).color(p.accent_text));
                col.label(RichText::new(format!("có số liệu {}", view::coverage_label(w.covered))).size(12.0).color(p.muted));
                let rain = w.rain_mm.map_or("—".to_string(), |mm| format!("{} mm", format_vn(mm, 1)));
                col.add_space(2.0);
                col.label(RichText::new(format!("Mưa {rain}")).font(FontId::new(15.0, theme::semibold())).color(p.text));
                col.label(RichText::new(format!("có số liệu {}", view::coverage_label(w.rain_covered))).size(12.0).color(p.muted));
            }
        });
        ui.add_space(2.0);
        ui.label(
            RichText::new("Bình quân theo thời gian của các chu kỳ đã lưu (kể cả những lần mở app trước). Khoảng nào app chưa chạy thì không có số liệu và không được bù; tỷ lệ \"có số liệu\" cho biết trung bình dựa trên bao nhiêu phần của khoảng.")
                .size(12.0)
                .color(p.muted),
        );
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
                None => (Icon::Info, p.muted, "Đang chờ dữ liệu từ thiết bị…".into()),
            }
        };
        paint::note(ui, icon, color, 13.0, RichText::new(text));
        if self.live.level().is_some() && self.live.stale(now, poll) {
            paint::note(ui, Icon::Warn, p.error, 13.0, RichText::new("Dữ liệu cũ: chưa có lần đọc mới."));
        }
        ui.add_space(4.0);
        let rain = match self.live.rain() {
            Some((_, mm_h)) => format!("Trạm đo mưa: {} mm/h", format_vn(mm_h, 1)),
            None => "Trạm đo mưa: chưa có số liệu".into(),
        };
        ui.label(RichText::new(rain).font(FontId::new(14.0, theme::semibold())).color(p.text));
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
                        "Chu kỳ {period} phút: {} – {}; mực nước {sign}{} m; tổ máy trung bình H1 {} · H2 {} m³/s.",
                        clock(f.from.time),
                        clock(f.to.time),
                        format_vn(f.dz, 3),
                        format_vn(f.cycle.q_h1, 2),
                        format_vn(f.cycle.q_h2, 2)
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
