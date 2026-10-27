//! Màn hình Lịch sử: xem, lọc theo ngày, xóa, xuất CSV (F6, F7).

use chrono::{DateTime, NaiveDate};
use eframe::egui::{self, Align, FontId, Layout, RichText};
use luuluong_core::validate::format_vn;
use luuluong_storage::csv_export::write_csv;
use luuluong_storage::history::{vn_timestamp, Record};

use super::{card, open_folder, theme, title, unix_now, view, App, Palette};

const ROW_HEIGHT: f32 = 30.0;

fn when(r: &Record) -> String {
    DateTime::parse_from_rfc3339(&r.created_at).map_or_else(|_| r.created_at.clone(), |t| t.format("%d/%m/%Y %H:%M").to_string())
}

impl App {
    /// `Err` nếu ô lọc không phải ngày hợp lệ.
    fn history_date(&self) -> Result<Option<NaiveDate>, ()> {
        let t = self.history_filter.trim();
        if t.is_empty() {
            return Ok(None);
        }
        NaiveDate::parse_from_str(t, "%d/%m/%Y").map(Some).map_err(|_| ())
    }

    fn reload_history(&mut self) {
        self.history_dirty = false;
        let Some(h) = &self.history else { return };
        let date = self.history_date().unwrap_or(None);
        match h.list(date) {
            Ok(rows) => self.history_rows = rows,
            Err(e) => self.storage_warning = Some(e.to_string()),
        }
    }

    fn export_history(&mut self) {
        let Some(dir) = self.data_dir.as_ref().map(|d| d.join("exports")) else { return };
        let stamp: String = vn_timestamp(unix_now()).chars().take(16).filter(|c| c.is_ascii_digit()).collect();
        let path = dir.join(format!("lich-su-{stamp}.csv"));
        let result = std::fs::create_dir_all(&dir).and_then(|_| write_csv(&path, &self.history_rows));
        self.export_notice = Some(match result {
            Ok(()) => format!("Đã xuất {} dòng: {}", self.history_rows.len(), path.display()),
            Err(_) => "Không xuất được file CSV. Kiểm tra ổ đĩa còn chỗ trống.".into(),
        });
    }

    pub(super) fn history_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        if self.history_dirty {
            self.reload_history();
        }
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Lịch sử tính toán");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Lọc theo ngày").size(13.0));
                let r = ui.add(egui::TextEdit::singleline(&mut self.history_filter).desired_width(120.0).hint_text("31/12/2026"));
                if r.changed() {
                    self.history_dirty = true;
                }
                if ui.button("Xóa lọc").clicked() {
                    self.history_filter.clear();
                    self.history_dirty = true;
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add_enabled(!self.history_rows.is_empty(), egui::Button::new("Xuất CSV")).clicked() {
                        self.export_history();
                    }
                });
            });
            if self.history_date().is_err() {
                ui.label(RichText::new("⚠ Nhập ngày dạng 31/12/2026").size(11.0).color(p.error));
            }
            if let Some(n) = self.export_notice.clone() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(n).size(11.0).color(p.muted));
                    if ui.small_button("Mở thư mục").clicked() {
                        if let Some(d) = &self.data_dir {
                            open_folder(&d.join("exports"));
                        }
                    }
                });
            }
            if let Some(w) = &self.storage_warning {
                ui.label(RichText::new(format!("⚠ {w}")).size(11.0).color(p.error));
            }
            ui.add_space(8.0);
            if self.history_rows.is_empty() {
                let msg = if self.history_filter.trim().is_empty() {
                    "Chưa có lần tính nào. Mỗi lần nhấn TÍNH sẽ được lưu tại đây."
                } else {
                    "Không có lần tính nào trong ngày này."
                };
                ui.label(RichText::new(msg).color(p.muted));
                return;
            }
            let head = |ui: &mut egui::Ui, t: &str| {
                ui.label(RichText::new(t).font(FontId::new(12.0, theme::semibold())).color(p.primary));
            };
            egui::Grid::new("history_head").num_columns(8).min_col_width(70.0).spacing([16.0, 6.0]).show(ui, |ui| {
                for t in ["Thời điểm", "Z đầu → cuối (m)", "Tính (giờ)", "H1 / H2 (giờ)", "Q (m³/s)", "W (m³)", "Người nhập", ""] {
                    head(ui, t);
                }
                ui.end_row();
            });
            let mut delete = None;
            let rows = &self.history_rows;
            egui::ScrollArea::vertical().max_height(420.0).show_rows(ui, ROW_HEIGHT, rows.len(), |ui, range| {
                egui::Grid::new("history_rows").num_columns(8).min_col_width(70.0).spacing([16.0, 6.0]).show(ui, |ui| {
                    for r in &rows[range] {
                        ui.label(when(r));
                        ui.label(format!("{} → {}", view::vn(r.z_start), view::vn(r.z_end)));
                        ui.label(view::vn(r.hours));
                        ui.label(format!("{} / {}", view::vn(r.h1), view::vn(r.h2)));
                        ui.label(RichText::new(format_vn(r.q_in, 2)).font(FontId::new(13.0, theme::semibold())).color(p.accent_text));
                        ui.label(format_vn(r.w_in, 2));
                        ui.label(&r.operator);
                        if ui.small_button("Xóa").clicked() {
                            delete = Some(r.id);
                        }
                        ui.end_row();
                    }
                });
            });
            if delete.is_some() {
                self.history_confirm_delete = delete;
            }
        });

        if let Some(id) = self.history_confirm_delete {
            egui::Window::new("Xác nhận xóa")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.label("Xóa dòng lịch sử này? Không hoàn tác được.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Xóa").clicked() {
                            if let Some(h) = &self.history {
                                if let Err(e) = h.delete(id) {
                                    self.storage_warning = Some(e.to_string());
                                }
                            }
                            self.history_confirm_delete = None;
                            self.history_dirty = true;
                        }
                        if ui.button("Giữ lại").clicked() {
                            self.history_confirm_delete = None;
                        }
                    });
                });
        }
    }
}
