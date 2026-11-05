//! Tab "Báo cáo": chọn khoảng ngày, xem trước theo tháng, xuất Excel (Ngày / Tháng / Năm).

use app::live::VN_TZ;
use app::paint::{self, Icon};
use app::report::{self, Granularity, REPORT_STATIONS};
use chrono::{DateTime, Datelike, NaiveDate};
use eframe::egui::{self, FontId, RichText};
use luuluong_core::validate::format_vn;
use luuluong_storage::xlsx_report::write_workbook;

use super::{accent_button, card, open_folder, theme, title, unix_now, App, Palette};

fn today_vn(now: i64) -> NaiveDate {
    DateTime::from_timestamp(now + i64::from(VN_TZ), 0).map_or(NaiveDate::MIN, |t| t.date_naive())
}

fn num(v: Option<f64>, decimals: usize) -> String {
    v.map_or("—".into(), |x| format_vn(x, decimals))
}

impl App {
    /// Khoảng ngày mặc định: từ 01/01 năm nay đến hôm nay.
    pub(super) fn default_report_range() -> (String, String) {
        let today = today_vn(unix_now());
        (format!("01/01/{}", today.year()), today.format("%d/%m/%Y").to_string())
    }

    /// Dữ liệu nguồn của báo cáo: các chu kỳ lưu lượng và mưa Vrain từ trước `from` một ngày.
    fn report_source(&self, from: NaiveDate) -> Option<(Vec<luuluong_storage::history::Record>, Vec<luuluong_storage::history::RainObs>)> {
        let h = self.history.as_ref()?;
        let since = report::midnight(from) - 86_400;
        Some((h.list_since(since).ok()?, h.rain_since(since).ok()?))
    }

    /// Bảng xem trước theo tháng; dựng lại khi đổi khoảng hoặc sau 60 giây (không đọc SQLite mỗi khung hình).
    fn refresh_report_preview(&mut self, from: NaiveDate, to: NaiveDate) {
        let key = format!("{from}-{to}");
        let now = unix_now();
        if matches!(&self.report_preview, Some((k, at, _)) if *k == key && now - *at < 60) {
            return;
        }
        self.report_preview = self.report_source(from).map(|(records, obs)| {
            let ids = REPORT_STATIONS.map(|s| s.0);
            (key, now, report::build(&records, &obs, ids, from, to, Granularity::Month, now))
        });
    }

    fn export_report(&mut self, from: NaiveDate, to: NaiveDate) {
        let (Some(dir), Some((records, obs))) = (self.data_dir.as_ref().map(|d| d.join("exports")), self.report_source(from)) else {
            self.report_notice = Some("Chưa có dữ liệu lịch sử để lập báo cáo.".into());
            return;
        };
        let now = unix_now();
        let ids = REPORT_STATIONS.map(|s| s.0);
        let by = |g| (g, report::build(&records, &obs, ids, from, to, g, now));
        let sheets = report::to_sheets([by(Granularity::Day), by(Granularity::Month), by(Granularity::Year)], REPORT_STATIONS.map(|s| s.1), from, to);
        let path = dir.join(report::file_name(from, to));
        let result = std::fs::create_dir_all(&dir).map_err(|_| ()).and_then(|_| write_workbook(&path, &sheets).map_err(|_| ()));
        self.report_notice = Some(match result {
            Ok(()) => format!("Đã xuất báo cáo: {}", path.display()),
            Err(()) => "Không xuất được file Excel. Kiểm tra ổ đĩa còn chỗ trống.".into(),
        });
    }

    fn report_preview_table(&self, ui: &mut egui::Ui, p: &Palette) {
        let Some((_, _, rows)) = &self.report_preview else {
            paint::note(ui, Icon::Info, p.muted, 13.0, RichText::new("Chưa có dữ liệu lịch sử để xem trước."));
            return;
        };
        let t = report::totals(rows);
        let head = |ui: &mut egui::Ui, text: &str| {
            ui.label(RichText::new(text).font(FontId::new(12.0, theme::semibold())).color(p.primary));
        };
        let cell = |ui: &mut egui::Ui, text: String, bold: bool| {
            let rich = RichText::new(text).size(13.0).color(p.text);
            ui.label(if bold { rich.font(FontId::new(13.0, theme::semibold())) } else { rich });
        };
        egui::Grid::new("report_preview").num_columns(6).min_col_width(90.0).spacing([16.0, 6.0]).show(ui, |ui| {
            for h in ["Tháng", "Lưu lượng về TB (m³/s)", "Tổng lượng về (m³)", "Có số liệu", "Mưa Hương Nguyên (mm)", "Mưa A Đớt (mm)"] {
                head(ui, h);
            }
            ui.end_row();
            for r in rows {
                cell(ui, r.period.label.clone(), false);
                cell(ui, num(r.q_avg, 2), false);
                cell(ui, num(r.volume_m3, 0), false);
                cell(ui, app::view::coverage_label(r.covered), false);
                cell(ui, num(r.rain_mm[0], 1), false);
                cell(ui, num(r.rain_mm[1], 1), false);
                ui.end_row();
            }
            cell(ui, "Tổng / bình quân".into(), true);
            cell(ui, num(t.q_avg, 2), true);
            cell(ui, num(t.volume_m3, 0), true);
            cell(ui, String::new(), true);
            cell(ui, num(t.rain_mm[0], 1), true);
            cell(ui, num(t.rain_mm[1], 1), true);
            ui.end_row();
        });
    }

    pub(super) fn report_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let today = today_vn(unix_now());
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            title(ui, p, "Báo cáo lưu lượng về hồ và mưa thượng nguồn");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Từ ngày").size(13.0));
                ui.add(egui::TextEdit::singleline(&mut self.report_from).desired_width(110.0).hint_text("01/01/2026"));
                ui.label(RichText::new("Đến ngày").size(13.0));
                ui.add(egui::TextEdit::singleline(&mut self.report_to).desired_width(110.0).hint_text("31/12/2026"));
                if ui.button("Tháng này").clicked() {
                    self.report_from = format!("01/{}", today.format("%m/%Y"));
                    self.report_to = today.format("%d/%m/%Y").to_string();
                }
                if ui.button("Năm nay").clicked() {
                    (self.report_from, self.report_to) = Self::default_report_range();
                }
            });
            match report::parse_range(&self.report_from, &self.report_to, today) {
                Err(e) => {
                    paint::note(ui, Icon::Warn, p.error, 12.0, RichText::new(e));
                }
                Ok((from, to)) => {
                    self.refresh_report_preview(from, to);
                    ui.add_space(8.0);
                    self.report_preview_table(ui, p);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.add(accent_button(p, "Xuất Excel")).clicked() {
                            self.export_report(from, to);
                        }
                        if let Some(dir) = self.data_dir.as_ref().map(|d| d.join("exports")) {
                            if ui.button("Mở thư mục").clicked() {
                                open_folder(&dir);
                            }
                        }
                    });
                }
            }
            if let Some(n) = &self.report_notice {
                ui.label(RichText::new(n).size(12.0).color(p.muted));
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new("File Excel có 3 sheet: Ngày, Tháng, Năm. Chỉ gồm thời gian app đã chạy và có số liệu; cột \"Có số liệu\" cho biết độ đầy đủ. Tháng hoặc năm trước khi dùng app sẽ để trống. Mưa lấy từ Vrain (Hương Nguyên, A Đớt) và không đi vào công thức lưu lượng về.")
                    .size(12.0)
                    .color(p.muted),
            );
        });
    }
}
