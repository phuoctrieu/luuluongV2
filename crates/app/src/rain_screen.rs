//! Mưa quan trắc Vrain: thẻ tóm tắt ở tab Tính toán và tab "Mưa" (bảng, biểu đồ theo ngày, xuất CSV).

use std::time::Duration;

use app::chart;
use app::live::VN_TZ;
use app::paint::{self, Icon};
use app::rain::{self, RainLevel, RefreshState};
use chrono::DateTime;
use eframe::egui::{self, Align, FontId, Layout, RichText};
use luuluong_core::validate::format_vn;
use luuluong_storage::config::VrainStation;
use luuluong_storage::csv_export::rain_to_csv;
use luuluong_storage::history::vn_timestamp;

use super::{accent_button, card, theme, title, unix_now, view, App, Palette, Tab};

/// Cột của bảng: nhãn và số giờ cộng dồn.
const WINDOWS: [(&str, u32); 6] = [("1 giờ", 1), ("3 giờ", 3), ("6 giờ", 6), ("12 giờ", 12), ("24 giờ", 24), ("7 ngày", 168)];
const CHART_DAYS: usize = 7;

fn mm_text(v: Option<f64>) -> String {
    v.map_or("—".into(), |x| format!("{} mm", format_vn(x, 1)))
}

fn level_style(p: &Palette, l: RainLevel) -> (Icon, egui::Color32) {
    match l {
        RainLevel::None => (Icon::Info, p.muted),
        RainLevel::Light | RainLevel::Moderate => (Icon::Info, p.primary),
        RainLevel::Heavy => (Icon::Warn, p.error),
    }
}

impl App {
    /// Nút "Cập nhật ngay" theo bốn trạng thái của cổng: rảnh, đang tải, chờ N giây, chưa cài tài khoản.
    fn rain_refresh_button(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let modifier = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
        match self.rain_gate.state(unix_now(), self.rain_configured()) {
            RefreshState::Idle => {
                if ui.add(accent_button(p, "Cập nhật ngay")).on_hover_text(format!("{modifier} + Shift + R")).clicked() {
                    self.request_rain_refresh();
                }
            }
            RefreshState::Loading => {
                ui.add_enabled(false, egui::Button::new("Đang cập nhật…").min_size(egui::vec2(96.0, 34.0)));
            }
            RefreshState::Wait(n) => {
                ui.add_enabled(false, egui::Button::new(format!("Thử lại sau {n} giây")).min_size(egui::vec2(96.0, 34.0)));
                ui.ctx().request_repaint_after(Duration::from_millis(500));
            }
            RefreshState::NeedsSetup => {
                if ui.add(egui::Button::new("Cài đặt Vrain").min_size(egui::vec2(96.0, 34.0))).clicked() {
                    self.tab = Tab::Settings;
                }
            }
        }
    }

    /// Giờ số liệu mới nhất, lỗi lần lấy gần nhất và nhãn dữ liệu cũ.
    fn rain_status(&self, ui: &mut egui::Ui, p: &Palette, now: i64) {
        let newest = self.config.vrain.stations.iter().filter_map(|s| rain::latest_ts(&self.rain_obs, &s.id)).max();
        let text = newest.map_or("Chưa có số liệu Vrain.".to_string(), |t| {
            let when = DateTime::from_timestamp(t + i64::from(VN_TZ), 0).map_or_else(String::new, |d| d.format("%H:%M %d/%m").to_string());
            format!("Số liệu mới nhất: giờ {when}")
        });
        ui.label(RichText::new(text).size(12.0).color(p.muted));
        if let Some(e) = &self.rain_error {
            paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new(format!("{e} Đang hiện số liệu đã lưu.")));
        } else if self.rain_configured() && rain::is_stale(self.rain_last_ok, now) {
            paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new("Dữ liệu cũ: chưa lấy được số liệu mới."));
        }
    }

    /// Trung bình mưa 24 giờ của các trạm Vrain có số liệu, và mưa trạm nhà máy cùng khoảng (nếu app đã chạy gần đủ ngày).
    fn rain_compare_note(&self, stations: &[VrainStation]) -> Option<String> {
        let sums: Vec<f64> = stations.iter().filter_map(|s| rain::rain_sum(&self.rain_obs, &s.id, 24)).collect();
        let mean = (!sums.is_empty()).then(|| sums.iter().sum::<f64>() / sums.len() as f64);
        let day = self.averages.iter().find(|w| w.label == "1 ngày")?;
        let ecapro = day.rain_mm.filter(|_| day.rain_covered >= 0.9);
        rain::compare_note(mean, ecapro, self.config.weather.rain_alert_mm)
    }

    fn rain_station_column(&self, ui: &mut egui::Ui, p: &Palette, st: &VrainStation, warn: f64) {
        ui.label(RichText::new(&st.name).font(FontId::new(14.0, theme::semibold())).color(p.primary));
        let h1 = rain::rain_sum(&self.rain_obs, &st.id, 1);
        ui.label(RichText::new(mm_text(h1)).font(FontId::new(26.0, theme::semibold())).color(p.accent_text));
        ui.label(RichText::new("trong 1 giờ gần nhất").size(12.0).color(p.muted));
        if let Some(mm) = h1 {
            let l = rain::level(mm);
            let (icon, color) = level_style(p, l);
            paint::note(ui, icon, color, 13.0, RichText::new(rain::level_label(l)));
        }
        ui.add_space(2.0);
        ui.label(RichText::new(format!("3 giờ: {}", mm_text(rain::rain_sum(&self.rain_obs, &st.id, 3)))).size(13.0).color(p.text));
        let day = rain::rain_sum(&self.rain_obs, &st.id, 24);
        let over = day.is_some_and(|d| d >= warn);
        let color = if over { p.error } else { p.text };
        ui.label(RichText::new(format!("24 giờ: {}", mm_text(day))).font(FontId::new(13.0, theme::semibold())).color(color));
        if over {
            paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new("Vượt ngưỡng cảnh báo"));
        }
    }

    /// Thẻ tóm tắt trong tab Tính toán.
    pub(super) fn rain_card(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let now = unix_now();
        let stations = self.config.vrain.stations.clone();
        let warn = self.config.weather.rain_alert_mm;
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                title(ui, p, "Mưa quan trắc thượng nguồn (Vrain, A Lưới)");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| self.rain_refresh_button(ui, p));
            });
            ui.add_space(6.0);
            if !self.config.vrain.enabled || stations.is_empty() {
                paint::note(ui, Icon::Info, p.muted, 13.0, RichText::new("Chưa bật lấy mưa từ Vrain. Bật và nhập tài khoản trong Cài đặt."));
                return;
            }
            ui.columns(stations.len(), |cols| {
                for (col, st) in cols.iter_mut().zip(&stations) {
                    self.rain_station_column(col, p, st, warn);
                }
            });
            ui.add_space(6.0);
            let ecapro = self.averages.iter().find(|w| w.label == "3 giờ").and_then(|w| w.rain_mm);
            let line = match ecapro {
                Some(mm) => format!("Trạm nhà máy (ECAPRO): {} trong 3 giờ qua", mm_text(Some(mm))),
                None => "Trạm nhà máy (ECAPRO): chưa có số liệu 3 giờ".into(),
            };
            ui.label(RichText::new(line).size(13.0).color(p.text));
            if let Some(note) = self.rain_compare_note(&stations) {
                paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new(note));
            }
            self.rain_status(ui, p, now);
        });
    }

    fn export_rain(&mut self) {
        let Some(dir) = self.data_dir.as_ref().map(|d| d.join("exports")) else { return };
        let obs = self.history.as_ref().and_then(|h| h.rain_since(0).ok()).unwrap_or_else(|| self.rain_obs.clone());
        let names: Vec<(String, String)> = self.config.vrain.stations.iter().map(|s| (s.id.clone(), s.name.clone())).collect();
        let stamp: String = vn_timestamp(unix_now()).chars().take(16).filter(|c| c.is_ascii_digit()).collect();
        let path = dir.join(format!("vrain-mua-{stamp}.csv"));
        let result = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, rain_to_csv(&obs, &names)));
        self.rain_notice = Some(match result {
            Ok(()) => format!("Đã xuất {} dòng: {}", obs.len(), path.display()),
            Err(_) => "Không xuất được file CSV. Kiểm tra ổ đĩa còn chỗ trống.".into(),
        });
    }

    /// Tab "Mưa": bảng 3 trạm × các khoảng (kèm hàng trạm nhà máy), biểu đồ theo ngày, xuất CSV.
    pub(super) fn rain_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let now = unix_now();
        let stations = self.config.vrain.stations.clone();
        let warn = self.config.weather.rain_alert_mm;
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                title(ui, p, "Mưa quan trắc Vrain (A Lưới)");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    self.rain_refresh_button(ui, p);
                    if ui.add_enabled(!self.rain_obs.is_empty(), egui::Button::new("Xuất CSV")).clicked() {
                        self.export_rain();
                    }
                });
            });
            ui.add_space(6.0);
            if !self.config.vrain.enabled || self.rain_obs.is_empty() {
                let msg = if self.config.vrain.enabled {
                    "Chưa có số liệu Vrain. Số liệu sẽ hiện sau lần lấy đầu tiên (nhấn Cập nhật ngay)."
                } else {
                    "Chưa có số liệu Vrain — bật trong Cài đặt."
                };
                paint::note(ui, Icon::Info, p.muted, 13.0, RichText::new(msg));
                self.rain_status(ui, p, now);
                return;
            }
            let head = |ui: &mut egui::Ui, t: &str| {
                ui.label(RichText::new(t).font(FontId::new(12.0, theme::semibold())).color(p.primary));
            };
            egui::Grid::new("rain_table").num_columns(WINDOWS.len() + 1).min_col_width(80.0).spacing([16.0, 8.0]).show(ui, |ui| {
                head(ui, "Trạm");
                for (label, _) in WINDOWS {
                    head(ui, label);
                }
                ui.end_row();
                for st in &stations {
                    ui.label(RichText::new(&st.name).font(FontId::new(13.0, theme::semibold())));
                    for (_, hours) in WINDOWS {
                        let v = rain::rain_sum(&self.rain_obs, &st.id, hours);
                        let over = hours == 24 && v.is_some_and(|x| x >= warn);
                        let color = if over { p.error } else { p.text };
                        ui.label(RichText::new(mm_text(v)).size(13.0).color(color));
                    }
                    ui.end_row();
                }
                ui.label(RichText::new("Trạm nhà máy (ECAPRO)").font(FontId::new(13.0, theme::semibold())));
                for (_, hours) in WINDOWS {
                    let label = match hours {
                        3 => "3 giờ",
                        12 => "12 giờ",
                        24 => "1 ngày",
                        168 => "1 tuần",
                        _ => "",
                    };
                    let v = self.averages.iter().find(|w| w.label == label).and_then(|w| w.rain_mm);
                    ui.label(RichText::new(mm_text(v)).size(13.0).color(p.text));
                }
                ui.end_row();
            });
            ui.add_space(4.0);
            if let Some(note) = self.rain_compare_note(&stations) {
                paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new(note));
            }
            ui.label(
                RichText::new("Mưa Vrain cộng các mốc giờ tính đến mốc mới nhất của từng trạm; khoảng nào thiếu mốc giờ thì để —. Hàng ECAPRO chỉ có các khoảng trùng với bảng trung bình ở tab Tính toán. Mưa Vrain chỉ để theo dõi, không đi vào công thức lưu lượng về.")
                    .size(12.0)
                    .color(p.muted),
            );
            self.rain_status(ui, p, now);
            if let Some(n) = &self.rain_notice {
                ui.label(RichText::new(n).size(12.0).color(p.muted));
            }
        });
        if !self.config.vrain.enabled || self.rain_obs.is_empty() {
            return;
        }
        ui.add_space(12.0);
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Mưa theo ngày: 3 trạm Vrain và trạm nhà máy");
            ui.add_space(6.0);
            let per_station: Vec<Vec<rain::DayRain>> = stations.iter().map(|s| rain::daily_rain(&self.rain_obs, &s.id, now, CHART_DAYS)).collect();
            let Some(first) = per_station.first() else { return };
            let dates: Vec<chrono::NaiveDate> = first.iter().map(|d| d.date).collect();
            let labels: Vec<String> = dates.iter().map(|d| format!("{} {}", view::weekday_vn(*d), d.format("%d/%m"))).collect();
            let mut series: Vec<view::Series> = stations
                .iter()
                .zip(&per_station)
                .map(|(s, days)| view::Series { label: s.name.clone(), values: days.iter().map(|d| d.mm).collect() })
                .collect();
            series.push(view::Series {
                label: "Trạm nhà máy".into(),
                values: dates.iter().map(|d| self.daily.iter().find(|x| x.date == *d).and_then(|x| x.rain_mm)).collect(),
            });
            let notes: Vec<String> = (0..dates.len())
                .map(|i| {
                    let parts: Vec<String> = stations.iter().zip(&per_station).map(|(s, days)| format!("{} {}/24 giờ", s.name, days[i].hours)).collect();
                    format!("Số mốc giờ có số liệu: {}", parts.join(", "))
                })
                .collect();
            ui.label(
                RichText::new(format!(
                    "Mỗi cột là lượng mưa cả ngày (mm, giờ Việt Nam), có số trên đầu cột; rê chuột để xem số mốc giờ đã có. Ngày hôm nay chưa trọn. Vạch đỏ nét đứt: ngưỡng cảnh báo {} mm.",
                    format_vn(warn, 0)
                ))
                .size(12.0)
                .color(p.muted),
            );
            ui.add_space(6.0);
            chart::rain_chart(ui, p, &labels, &series, &notes, warn, 230.0);
        });
    }
}
