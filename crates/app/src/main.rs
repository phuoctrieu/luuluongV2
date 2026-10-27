#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use app::form::{validate, Field, Form};
use app::theme::{self, Palette};
use eframe::egui::{self, Align, Color32, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Modifiers, RichText, Stroke};
use luuluong_core::validate::format_vn;
use luuluong_core::{compute, CalcError, Outputs, Turbine, VolumeTable};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Calc,
    History,
    Settings,
}

const REQUIRED: [Field; 5] = [Field::ZStart, Field::ZEnd, Field::Hours, Field::H1, Field::H2];

struct App {
    table: VolumeTable,
    turbine: Turbine,
    form: Form,
    result: Option<Outputs>,
    submitted: bool,
    tab: Tab,
    show_spill: bool,
    confirm_quit: bool,
    allow_close: bool,
    focus_next: Option<Field>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        Self {
            table: VolumeTable::builtin(),
            turbine: Turbine::default(),
            form: Form::default(),
            result: None,
            submitted: false,
            tab: Tab::Calc,
            show_spill: false,
            confirm_quit: false,
            allow_close: false,
            focus_next: None,
        }
    }

    fn field_id(f: Field) -> Id {
        Id::new(("field", f as usize))
    }

    /// Tính nếu đủ dữ liệu; nếu thiếu, nhảy tới ô đầu tiên bị lỗi.
    fn try_compute(&mut self) {
        self.submitted = true;
        match validate(&self.form, &self.table) {
            Ok(inputs) => {
                self.result = compute(&self.table, &self.turbine, &inputs).ok();
            }
            Err(errors) => {
                self.result = None;
                let first = errors[0].0;
                if matches!(first, Field::SpillQ | Field::SpillH | Field::EcoQ) {
                    self.show_spill = true;
                }
                self.focus_next = Some(first);
            }
        }
    }

    fn clear(&mut self) {
        self.form.clear();
        self.result = None;
        self.submitted = false;
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let (calc, clear, t1, t2, t3) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::Enter),
                i.consume_key(Modifiers::COMMAND, Key::Backspace),
                i.consume_key(Modifiers::COMMAND, Key::Num1),
                i.consume_key(Modifiers::COMMAND, Key::Num2),
                i.consume_key(Modifiers::COMMAND, Key::Num3),
            )
        });
        if calc {
            self.try_compute();
        }
        if clear {
            self.clear();
        }
        if t1 {
            self.tab = Tab::Calc;
        } else if t2 {
            self.tab = Tab::History;
        } else if t3 {
            self.tab = Tab::Settings;
        }
    }
}

fn card(p: &Palette) -> Frame {
    Frame::new()
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(16))
}

fn title(ui: &mut egui::Ui, p: &Palette, text: &str) {
    ui.label(RichText::new(text).font(FontId::new(16.0, theme::semibold())).color(p.primary));
}

fn field_label(f: Field) -> (&'static str, &'static str) {
    match f {
        Field::ZStart => ("Mực nước đầu", "m"),
        Field::ZEnd => ("Mực nước cuối", "m"),
        Field::Hours => ("Thời gian tính", "giờ"),
        Field::H1 => ("Giờ chạy máy H1", "giờ"),
        Field::H2 => ("Giờ chạy máy H2", "giờ"),
        Field::SpillQ => ("Xả tràn: lưu lượng", "m³/s"),
        Field::SpillH => ("Xả tràn: số giờ", "giờ"),
        Field::EcoQ => ("Xả dòng chảy tối thiểu", "m³/s"),
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let p = theme::palette(ctx);

        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.form.is_empty() || self.result.is_some() {
                self.allow_close = true;
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.confirm_quit = true;
            }
        }
        self.shortcuts(ctx);

        egui::TopBottomPanel::top("nav")
            .frame(Frame::new().fill(p.header_bg).inner_margin(Margin::symmetric(16, 8)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("NMTĐ Sông Bồ · Lưu lượng về hồ")
                            .font(FontId::new(16.0, theme::semibold()))
                            .color(p.header_text),
                    );
                    ui.add_space(24.0);
                    for (tab, name, key) in [
                        (Tab::Calc, "Tính toán", "1"),
                        (Tab::History, "Lịch sử", "2"),
                        (Tab::Settings, "Cài đặt", "3"),
                    ] {
                        let active = self.tab == tab;
                        let color = if active { p.header_text } else { p.header_muted };
                        let mut text = RichText::new(name).size(14.0).color(color);
                        if active {
                            text = text.font(FontId::new(14.0, theme::semibold())).underline();
                        }
                        let r = ui
                            .add(egui::Button::new(text).frame(false).min_size(egui::vec2(0.0, 32.0)))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        if r.on_hover_text(format!("Ctrl/Cmd + {key}")).clicked() {
                            self.tab = tab;
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let dark = ctx.theme() == egui::Theme::Dark;
                        let label = if dark { "Giao diện sáng" } else { "Giao diện tối" };
                        let toggle = egui::Button::new(RichText::new(label).color(p.header_text))
                            .fill(Color32::TRANSPARENT)
                            .stroke(Stroke::new(1.0, p.header_muted))
                            .min_size(egui::vec2(0.0, 32.0));
                        if ui.add(toggle).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            ctx.set_theme(if dark { egui::Theme::Light } else { egui::Theme::Dark });
                        }
                    });
                });
            });

        egui::TopBottomPanel::bottom("status")
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::symmetric(16, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Thời tiết: chưa kết nối (giai đoạn 3)").size(11.0).color(p.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("Phiên bản {VERSION}")).size(11.0).color(p.muted));
                    });
                });
            });

        egui::CentralPanel::default().frame(Frame::new().fill(p.bg).inner_margin(Margin::same(16))).show(ctx, |ui| match self.tab {
            Tab::Calc => self.calc_screen(ui, p),
            Tab::History => placeholder(ui, p, "Lịch sử tính toán", "Sẽ có ở giai đoạn 4: tự lưu mỗi lần tính, lọc theo ngày, xuất CSV."),
            Tab::Settings => placeholder(ui, p, "Cài đặt", "Sẽ có ở giai đoạn 4: lưu lượng tổ máy, API key, giao diện, bảng dung tích."),
        });

        if self.confirm_quit {
            egui::Window::new("Xác nhận")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Bạn có muốn thoát không? Dữ liệu đã nhập chưa được tính.");
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("Thoát").clicked() {
                            self.allow_close = true;
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui.button("Ở lại").clicked() || ctx.input(|i| i.key_pressed(Key::Escape)) {
                            self.confirm_quit = false;
                        }
                    });
                });
        }
    }
}

fn placeholder(ui: &mut egui::Ui, p: &Palette, name: &str, note: &str) {
    card(p).show(ui, |ui| {
        title(ui, p, name);
        ui.add_space(8.0);
        ui.label(RichText::new(note).color(p.muted));
    });
}

impl App {
    fn calc_screen(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let errors = match validate(&self.form, &self.table) {
            Ok(_) => Vec::new(),
            Err(e) => e,
        };
        let hint = format!("{} – {} m", format_vn(self.table.z_min(), 1), format_vn(self.table.z_max(), 1));
        let previous = self.form.clone();
        let mut enter_pressed = false;

        ui.columns(2, |cols| {
            card(p).show(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                title(ui, p, "Thông số tính toán");
                ui.add_space(8.0);
                for f in REQUIRED {
                    enter_pressed |= self.input_row(ui, p, f, &errors, &hint);
                }
                ui.add_space(4.0);
                ui.collapsing(RichText::new("Xả nước (không bắt buộc)").size(13.0), |ui| {
                    for f in [Field::SpillQ, Field::SpillH, Field::EcoQ] {
                        enter_pressed |= self.input_row(ui, p, f, &errors, &hint);
                    }
                });
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let calc = egui::Button::new(RichText::new("TÍNH").font(FontId::new(15.0, theme::semibold())).color(p.on_accent))
                        .fill(p.accent)
                        .min_size(egui::vec2(120.0, 44.0));
                    if ui.add(calc).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                        self.try_compute();
                    }
                    if ui.add(egui::Button::new("Xóa các ô").min_size(egui::vec2(0.0, 44.0))).on_hover_text("Ctrl/Cmd + Backspace").clicked() {
                        self.clear();
                    }
                });
            });
            self.result_card(&mut cols[1], p);
        });

        if enter_pressed {
            self.try_compute();
        }
        if previous.differs(&self.form) {
            self.result = None;
        }
        if let Some(f) = self.focus_next.take() {
            ui.memory_mut(|m| m.request_focus(Self::field_id(f)));
        }
    }

    /// Trả về true nếu người dùng nhấn Enter trong ô này.
    fn input_row(&mut self, ui: &mut egui::Ui, p: &Palette, f: Field, errors: &[(Field, CalcError)], hint: &str) -> bool {
        let (label, unit) = field_label(f);
        let error = errors.iter().find(|(ef, _)| *ef == f).map(|(_, e)| e);
        let show_error = error.is_some() && (self.submitted || !self.form.get(f).trim().is_empty());
        let mut enter = false;
        ui.label(RichText::new(label).size(13.0));
        ui.horizontal(|ui| {
            let stroke_color = if show_error { p.error } else { p.border };
            let r = ui.add(
                egui::TextEdit::singleline(self.form.get_mut(f))
                    .id(Self::field_id(f))
                    .desired_width(180.0)
                    .font(FontId::proportional(15.0))
                    .margin(Margin::symmetric(8, 8)),
            );
            if show_error {
                ui.painter().rect_stroke(r.rect, CornerRadius::same(8), Stroke::new(1.5, stroke_color), egui::StrokeKind::Outside);
            }
            ui.label(RichText::new(unit).color(p.muted));
            if r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                enter = true;
            }
        });
        let note = match f {
            Field::ZStart | Field::ZEnd => hint.to_string(),
            Field::Hours => "lớn hơn 0".to_string(),
            Field::H1 | Field::H2 => "từ 0 đến thời gian tính; để trống là 0".to_string(),
            _ => "để trống nếu không xả".to_string(),
        };
        if show_error {
            // Màu lỗi luôn đi kèm chữ, không chỉ dựa vào màu.
            ui.label(RichText::new(format!("⚠ {}", error.unwrap())).size(11.0).color(p.error));
        } else {
            ui.label(RichText::new(note).size(11.0).color(p.muted));
        }
        ui.add_space(6.0);
        enter
    }

    fn result_card(&self, ui: &mut egui::Ui, p: &Palette) {
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(300.0);
            title(ui, p, "Kết quả");
            ui.add_space(8.0);
            let Some(o) = &self.result else {
                ui.label(RichText::new("Nhập thông số ở bên trái rồi nhấn TÍNH (hoặc Enter).").color(p.muted));
                return;
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new(format_vn(o.q_in, 2)).font(FontId::new(44.0, theme::semibold())).color(p.accent_text));
                ui.label(RichText::new("m³/s").size(16.0).color(p.muted));
            });
            ui.label(RichText::new(format!("Tổng lượng nước về hồ: {} m³", format_vn(o.w_in, 2))).size(15.0));
            ui.add_space(12.0);
            egui::Grid::new("breakdown").num_columns(2).spacing([24.0, 6.0]).show(ui, |ui| {
                for (name, v) in [
                    ("V đầu", o.v_start),
                    ("V cuối", o.v_end),
                    ("ΔV (cuối − đầu)", o.delta_v),
                    ("V qua tuabin", o.v_turbine),
                    ("V xả", o.v_release),
                ] {
                    ui.label(RichText::new(name).color(p.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(format!("{} m³", format_vn(v, 2)));
                    });
                    ui.end_row();
                }
            });
            ui.add_space(12.0);
            if ui.button("Sao chép kết quả").clicked() {
                ui.ctx().copy_text(format!(
                    "Lưu lượng nước về hồ: {} m³/s\nTổng lượng nước về hồ: {} m³",
                    format_vn(o.q_in, 2),
                    format_vn(o.w_in, 2)
                ));
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new("Lưu ý: đây là lưu lượng đến hồ ước tính (chưa tính bốc hơi, thấm; lưu lượng tổ máy là hằng số cài đặt).")
                    .size(11.0)
                    .color(p.muted),
            );
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("NMTĐ Sông Bồ – Lưu lượng nước về hồ")
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("luuluong", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}
