//! Cập nhật ứng dụng từ GitHub: thẻ trong Cài đặt, hộp xác nhận, luồng nền kiểm tra/tải/cài, mở lại sau khi cài.

use std::time::Duration;

use app::paint::{self, Icon};
use app::update_state::{self, UpdateEvent, UpdateState};
use eframe::egui::{self, FontId, RichText};
use luuluong_core::validate::format_vn;
use luuluong_update as update;
use update::{notes_excerpt, Platform, Version};

use super::{accent_button, card, open_url, theme, title, unix_now, App, Palette};

pub const RELEASES_URL: &str = "https://github.com/phuoctrieu/luuluongV2/releases/latest";

/// Phiên bản đang chạy (từ `Cargo.toml`).
pub fn current_version() -> Version {
    update::parse_version(env!("CARGO_PKG_VERSION")).unwrap_or(Version(0, 0, 0))
}

fn mb(bytes: u64) -> String {
    format_vn(bytes as f64 / 1_048_576.0, 1)
}

impl App {
    fn apply_update_event(&mut self, event: UpdateEvent) {
        let state = std::mem::replace(&mut self.update_state, UpdateState::Idle);
        self.update_state = update_state::next(state, event, current_version());
    }

    /// Dọn `.old` và thư mục tạm của lần cập nhật trước (chỉ khi chạy từ bản đã cài).
    pub(super) fn cleanup_after_update(&self) {
        if let Some(target) = std::env::current_exe().ok().and_then(|p| update::detect_target(&p).ok()) {
            update::cleanup(&target, &std::env::temp_dir());
        }
    }

    pub(super) fn start_update_check(&mut self, ctx: &egui::Context) {
        if matches!(self.update_state, UpdateState::Checking | UpdateState::Downloading { .. } | UpdateState::Installing) {
            return;
        }
        self.apply_update_event(UpdateEvent::CheckStarted);
        let (tx, ctx) = (self.update_tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(UpdateEvent::Checked(update::check(update::API_BASE, Platform::current())));
            ctx.request_repaint();
        });
    }

    /// Bấm "Tải và cài đặt" (sau khi đã xác nhận).
    fn start_update_install(&mut self, ctx: &egui::Context) {
        let Some(release) = update_state::available(&self.update_state).cloned() else { return };
        let target = std::env::current_exe().map_err(|_| update::UpdateError::NotInstalled).and_then(|p| update::detect_target(&p));
        let target = match target {
            Ok(t) => t,
            Err(e) => {
                self.update_state = UpdateState::Failed(e.to_string());
                return;
            }
        };
        self.apply_update_event(UpdateEvent::InstallClicked);
        self.update_target = Some(target.clone());
        let (tx, ctx) = (self.update_tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let mut progress = |done, total| {
                let _ = tx.send(UpdateEvent::Progress(done, total));
                ctx.request_repaint();
            };
            let result = update::install_release(&release, &target, &std::env::temp_dir(), &mut progress);
            let _ = tx.send(match result {
                Ok(()) => UpdateEvent::Installed,
                Err(e) => UpdateEvent::Error(e),
            });
            ctx.request_repaint();
        });
    }

    /// Mở bản mới rồi đóng bản đang chạy.
    fn relaunch_after_update(&mut self, ctx: &egui::Context) {
        let Some(target) = self.update_target.take() else { return };
        let (program, args) = update::relaunch_command(&target);
        match std::process::Command::new(program).args(args).spawn() {
            Ok(_) => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(_) => {
                self.update_state = UpdateState::Failed("Đã cài xong nhưng không tự mở lại được. Hãy đóng và mở lại app.".into());
            }
        }
    }

    pub(super) fn poll_update(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.update_rx.try_recv() {
            match &event {
                // Chỉ ghi nhận lần kiểm tra thành công; lỗi mạng thì lần mở sau thử lại.
                UpdateEvent::Checked(Ok(_)) => {
                    self.config.update.last_check = unix_now();
                    let _ = self.save_config();
                }
                // Chi tiết chỉ ghi log; người vận hành chỉ thấy câu tiếng Việt.
                UpdateEvent::Checked(Err(e)) | UpdateEvent::Error(e) => eprintln!("Cập nhật: {e}"),
                _ => {}
            }
            let installed = event == UpdateEvent::Installed;
            self.apply_update_event(event);
            if installed {
                self.relaunch_after_update(ctx);
            }
        }
    }

    /// Hộp xác nhận trước khi cài.
    pub(super) fn update_confirm_window(&mut self, ctx: &egui::Context, p: &Palette) {
        if !self.update_confirm {
            return;
        }
        let Some(version) = update_state::available(&self.update_state).map(|r| r.version) else {
            self.update_confirm = false;
            return;
        };
        egui::Window::new("Cập nhật ứng dụng")
            .frame(card(p).inner_margin(egui::Margin::same(20)))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_max_width(440.0);
                ui.label(RichText::new(format!("Cài đặt bản {version}?")).font(FontId::new(16.0, theme::semibold())));
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "App sẽ tải bản mới, kiểm tra tính toàn vẹn rồi tự khởi động lại trong vài giây. Lịch sử và cài đặt được giữ nguyên.\n\n\
Trong lúc khởi động lại, số liệu trực tuyến bị gián đoạn và chu kỳ đang chạy sẽ ghi \"thiếu số liệu\". Nên cài ngay sau một mốc chu kỳ.",
                    )
                    .size(13.0),
                );
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.add(accent_button(p, "Cài đặt")).clicked() {
                        self.update_confirm = false;
                        self.start_update_install(ctx);
                    }
                    if ui.button("Để sau").clicked() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                        self.update_confirm = false;
                    }
                });
            });
    }

    /// Thẻ "Cập nhật ứng dụng" trong Cài đặt.
    pub(super) fn update_card(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let busy = matches!(self.update_state, UpdateState::Checking | UpdateState::Downloading { .. } | UpdateState::Installing);
        let ctx = ui.ctx().clone();
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Cập nhật ứng dụng");
            ui.add_space(6.0);
            ui.label(RichText::new(format!("Phiên bản đang chạy: {}", current_version())).size(13.0));
            ui.checkbox(&mut self.settings.update_auto_check, "Tự kiểm tra bản mới khi mở app (tối đa một lần mỗi ngày)");
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, egui::Button::new("Kiểm tra cập nhật")).clicked() {
                    self.start_update_check(&ctx);
                }
                match &self.update_state {
                    UpdateState::Checking => ui.label(RichText::new("Đang kiểm tra…").size(13.0).color(p.muted)),
                    UpdateState::UpToDate => ui.label(RichText::new("Đang dùng bản mới nhất.").size(13.0).color(p.muted)),
                    _ => ui.label(""),
                };
            });
            match self.update_state.clone() {
                UpdateState::Available(r) => {
                    ui.add_space(6.0);
                    ui.label(RichText::new(format!("Có bản mới: {}", r.version)).font(FontId::new(15.0, theme::semibold())).color(p.primary));
                    let notes = notes_excerpt(&r.notes);
                    if !notes.is_empty() {
                        ui.label(RichText::new(notes).size(12.0).color(p.muted));
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.add(accent_button(p, "Tải và cài đặt")).clicked() {
                            self.update_confirm = true;
                        }
                        if ui.button("Mở trang tải").clicked() {
                            open_url(RELEASES_URL);
                        }
                    });
                }
                UpdateState::Downloading { done, total } => {
                    ui.add_space(6.0);
                    let fraction = if total == 0 { 0.0 } else { (done as f32 / total as f32).clamp(0.0, 1.0) };
                    ui.add(egui::ProgressBar::new(fraction).text(format!("Đang tải {} / {} MB", mb(done), mb(total))));
                }
                UpdateState::Installing => {
                    ui.add_space(6.0);
                    paint::note(ui, Icon::Info, p.muted, 13.0, RichText::new("Đang cài đặt, app sẽ tự mở lại…"));
                }
                UpdateState::Failed(message) => {
                    ui.add_space(6.0);
                    paint::note(ui, Icon::Warn, p.error, 13.0, RichText::new(message));
                    if ui.button("Mở trang tải").clicked() {
                        open_url(RELEASES_URL);
                    }
                }
                UpdateState::Idle | UpdateState::Checking | UpdateState::UpToDate => {}
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new("App chỉ cài khi bạn bấm, và kiểm tra mã SHA-256 của gói tải về trước khi thay thế. Bản đầu có tính năng này vẫn phải cài tay.")
                    .size(12.0)
                    .color(p.muted),
            );
        });
        if busy {
            ctx.request_repaint_after(Duration::from_millis(300));
        }
    }
}
