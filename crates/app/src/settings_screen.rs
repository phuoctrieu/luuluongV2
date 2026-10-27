//! Màn hình Cài đặt (F8, F9): tổ máy, nguồn thời tiết, API key, giao diện, bảng dung tích, thư mục dữ liệu.

use app::settings::{SettingsError, SettingsField};
use eframe::egui::{self, FontId, Margin, RichText};
use luuluong_storage::secret::SecretStore;
use luuluong_storage::table_loader::load_volume_table;
use luuluong_weather::Source;

use super::{apply_theme, card, open_folder, theme, title, App, Palette, Turbine, VolumeTable};
use luuluong_core::validate::format_vn;

fn error_for(errors: &[SettingsError], f: SettingsField) -> Option<&str> {
    errors.iter().find(|e| e.field == f).map(|e| e.message.as_str())
}

fn text_row(ui: &mut egui::Ui, p: &Palette, label: &str, unit: &str, value: &mut String, error: Option<&str>) {
    ui.label(RichText::new(label).size(13.0));
    ui.horizontal(|ui| {
        let r = ui.add(egui::TextEdit::singleline(value).desired_width(160.0).margin(Margin::symmetric(8, 8)));
        if error.is_some() {
            ui.painter().rect_stroke(r.rect, egui::CornerRadius::same(8), egui::Stroke::new(1.5, p.error), egui::StrokeKind::Outside);
        }
        ui.label(RichText::new(unit).color(p.muted));
    });
    if let Some(e) = error {
        ui.label(RichText::new(format!("⚠ {e}")).size(11.0).color(p.error));
    }
    ui.add_space(6.0);
}

impl App {
    fn apply_settings(&mut self, ctx: &egui::Context) {
        match self.settings.to_config(&self.config) {
            Ok(config) => {
                let source_changed = config.weather.source != self.config.weather.source
                    || config.weather.sources != self.config.weather.sources
                    || config.weather.lat != self.config.weather.lat
                    || config.weather.lon != self.config.weather.lon;
                self.turbine = Turbine { q_h1: config.turbine.q_h1, q_h2: config.turbine.q_h2 };
                apply_theme(ctx, &config.ui.theme);
                let minutes = config.weather.refresh_minutes;
                self.config = config;
                self.sync_sources();
                if source_changed || minutes != self.scheduler_minutes() {
                    self.scheduler = super::Scheduler::new(minutes);
                }
                self.result = None;
                self.settings_errors.clear();
                self.settings_notice = Some(match self.save_config() {
                    Ok(()) => "Đã lưu cài đặt.".into(),
                    Err(e) => format!("Áp dụng được nhưng chưa lưu được vào file: {e}"),
                });
            }
            Err(errors) => {
                self.settings_errors = errors;
                self.settings_notice = None;
            }
        }
    }

    fn scheduler_minutes(&self) -> u32 {
        self.config.weather.refresh_minutes
    }

    fn load_table(&mut self) {
        let path = self.table_path_input.trim().to_string();
        match load_volume_table(std::path::Path::new(&path)) {
            Ok(t) => {
                self.table_notice = Some(format!(
                    "Đã nạp bảng: {} dòng, từ {} đến {} m.",
                    t.len(),
                    format_vn(t.z_min(), 1),
                    format_vn(t.z_max(), 1)
                ));
                self.table = t;
                self.config.reservoir.volume_table = path;
                let _ = self.save_config();
                self.result = None;
            }
            Err(e) => {
                // Đặc tả 8.3: nêu lỗi rồi quay về bảng nhúng sẵn.
                self.modal = Some(format!("{e}\nĐang dùng bảng dung tích nhúng sẵn."));
                self.use_builtin_table();
            }
        }
    }

    fn use_builtin_table(&mut self) {
        self.table = VolumeTable::builtin();
        self.config.reservoir.volume_table = "builtin".into();
        let _ = self.save_config();
        self.result = None;
        self.table_notice = Some("Đang dùng bảng dung tích nhúng sẵn.".into());
    }

    pub(super) fn settings_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let ctx = ui.ctx().clone();
        let errors = self.settings_errors.clone();
        ui.set_max_width(760.0);

        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Tổ máy");
            ui.add_space(8.0);
            text_row(ui, p, "Lưu lượng mỗi giờ chạy máy H1", "m³/s", &mut self.settings.q_h1, error_for(&errors, SettingsField::QH1));
            text_row(ui, p, "Lưu lượng mỗi giờ chạy máy H2", "m³/s", &mut self.settings.q_h2, error_for(&errors, SettingsField::QH2));
        });
        ui.add_space(12.0);

        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Thời tiết");
            ui.add_space(8.0);
            ui.label(RichText::new("Nguồn chính (các thẻ ngày)").size(13.0));
            egui::ComboBox::from_id_salt("source").selected_text(self.settings.source.label()).width(320.0).show_ui(ui, |ui| {
                for s in Source::ALL {
                    ui.selectable_value(&mut self.settings.source, s, s.label());
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("Nguồn hiện trong bảng so sánh").size(13.0));
            for s in Source::ALL {
                let is_primary = s == self.settings.source;
                let mut on = is_primary || self.settings.compare.contains(&s);
                if ui.add_enabled(!is_primary, egui::Checkbox::new(&mut on, s.label())).changed() {
                    if on {
                        self.settings.compare.push(s);
                    } else {
                        self.settings.compare.retain(|x| *x != s);
                    }
                }
            }
            let note = "Nguồn chính luôn được bật. NOAA GFS lấy qua Open-Meteo vì api.weather.gov chỉ phủ nước Mỹ. \
Các nguồn có thể báo mưa rất khác nhau nên bảng so sánh hiện lượng mưa của từng nguồn. \
Open-Meteo miễn phí cho mục đích phi thương mại; OpenWeather cần API key bên dưới.";
            ui.label(RichText::new(note).size(11.0).color(p.muted));
            ui.add_space(8.0);
            text_row(ui, p, "Vĩ độ", "°", &mut self.settings.lat, error_for(&errors, SettingsField::Lat));
            text_row(ui, p, "Kinh độ", "°", &mut self.settings.lon, error_for(&errors, SettingsField::Lon));
            text_row(ui, p, "Làm mới sau mỗi", "phút", &mut self.settings.refresh_minutes, error_for(&errors, SettingsField::RefreshMinutes));
            text_row(ui, p, "Cảnh báo mưa lớn từ", "mm/ngày", &mut self.settings.rain_alert_mm, error_for(&errors, SettingsField::RainAlert));

            ui.separator();
            ui.label(RichText::new("API key OpenWeather (lưu trong Keychain / Credential Manager)").size(13.0));
            let has_key = !self.api_key.trim().is_empty();
            let status = if has_key { "Đã có key." } else { "Chưa có key." };
            ui.label(RichText::new(status).size(11.0).color(p.muted));
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut self.key_input).password(true).desired_width(280.0).margin(Margin::symmetric(8, 8)));
                let can_save = !self.key_input.trim().is_empty() && self.secrets.is_some();
                if ui.add_enabled(can_save, egui::Button::new("Lưu key")).clicked() {
                    let key = self.key_input.trim().to_string();
                    let saved = self.secrets.as_ref().map(|s| s.set(&key));
                    self.settings_notice = Some(match saved {
                        Some(Ok(())) => {
                            self.api_key = key;
                            self.key_input.clear();
                            self.scheduler = super::Scheduler::new(self.config.weather.refresh_minutes);
                            "Đã lưu API key.".into()
                        }
                        Some(Err(e)) => e.to_string(),
                        None => "Không truy cập được kho khóa của hệ điều hành.".into(),
                    });
                }
                if ui.add_enabled(has_key, egui::Button::new("Xóa key")).clicked() {
                    if let Some(s) = &self.secrets {
                        let _ = s.delete();
                    }
                    self.api_key.clear();
                    self.settings_notice = Some("Đã xóa API key.".into());
                }
            });
        });
        ui.add_space(12.0);

        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Giao diện");
            ui.add_space(8.0);
            ui.label(RichText::new("Màu giao diện").size(13.0));
            egui::ComboBox::from_id_salt("theme")
                .selected_text(match self.settings.theme.as_str() {
                    "light" => "Sáng",
                    "dark" => "Tối",
                    _ => "Theo hệ thống",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.settings.theme, "system".into(), "Theo hệ thống");
                    ui.selectable_value(&mut self.settings.theme, "light".into(), "Sáng");
                    ui.selectable_value(&mut self.settings.theme, "dark".into(), "Tối");
                });
            ui.add_space(8.0);
            ui.label(RichText::new("Tên người vận hành (ghi vào lịch sử, có thể để trống)").size(13.0));
            ui.add(egui::TextEdit::singleline(&mut self.settings.operator).desired_width(240.0).margin(Margin::symmetric(8, 8)));
        });
        ui.add_space(12.0);

        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Bảng dung tích hồ");
            ui.add_space(8.0);
            let current = if self.config.reservoir.volume_table == "builtin" { "bảng nhúng sẵn".to_string() } else { self.config.reservoir.volume_table.clone() };
            ui.label(RichText::new(format!("Đang dùng: {current} ({} dòng, {} – {} m)", self.table.len(), format_vn(self.table.z_min(), 1), format_vn(self.table.z_max(), 1))).size(12.0).color(p.muted));
            ui.label(RichText::new("Đường dẫn file .csv hoặc .xlsx").size(13.0));
            ui.add(egui::TextEdit::singleline(&mut self.table_path_input).desired_width(420.0).margin(Margin::symmetric(8, 8)));
            ui.horizontal(|ui| {
                if ui.add_enabled(!self.table_path_input.trim().is_empty(), egui::Button::new("Nạp bảng")).clicked() {
                    self.load_table();
                }
                if ui.button("Dùng bảng nhúng sẵn").clicked() {
                    self.table_path_input.clear();
                    self.use_builtin_table();
                }
            });
            if let Some(n) = &self.table_notice {
                ui.label(RichText::new(n).size(11.0).color(p.muted));
            }
        });
        ui.add_space(12.0);

        ui.horizontal(|ui| {
            let save = egui::Button::new(RichText::new("Lưu cài đặt").font(FontId::new(15.0, theme::semibold())).color(p.on_accent))
                .fill(p.accent)
                .min_size(egui::vec2(140.0, 44.0));
            if ui.add(save).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                self.apply_settings(&ctx);
            }
            if ui.add(egui::Button::new("Mở thư mục dữ liệu").min_size(egui::vec2(0.0, 44.0))).clicked() {
                if let Some(d) = &self.data_dir {
                    open_folder(d);
                }
            }
        });
        if let Some(n) = &self.settings_notice {
            ui.add_space(6.0);
            ui.label(RichText::new(n).size(12.0).color(p.muted));
        }
        if !errors.is_empty() {
            ui.label(RichText::new("⚠ Còn ô chưa hợp lệ, xem các dòng đỏ ở trên.").size(12.0).color(p.error));
        }
    }
}
