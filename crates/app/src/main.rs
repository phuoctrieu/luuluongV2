#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app::form::{validate, Field, Form};
use app::schedule::Scheduler;
use app::settings::{SettingsError, SettingsForm};
use app::theme::{self, Palette};
use app::view;
use eframe::egui::{self, Align, Color32, CornerRadius, FontId, Frame, Id, Key, Layout, Margin, Modifiers, RichText, Stroke};
use luuluong_core::validate::format_vn;
use luuluong_core::{compute, CalcError, Outputs, Turbine, VolumeTable};
use luuluong_storage::config::Config;
use luuluong_storage::history::{vn_timestamp, History, Record};
use luuluong_storage::secret::{KeyringStore, SecretStore};
use luuluong_storage::table_loader::load_volume_table;
use luuluong_weather::chain::{fetch_chain, Fetched};
use luuluong_weather::free::FreeProvider;
use luuluong_weather::openmeteo::{Model, OpenMeteoProvider};
use luuluong_weather::{cache, Snapshot, Source, WeatherError, WeatherProvider};

mod history_screen;
mod settings_screen;

const VERSION: &str = env!("CARGO_PKG_VERSION");

type WeatherResult = Result<Fetched, WeatherError>;

fn unix_now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("vn", "songbo", "luuluong")
}

fn make_provider(source: Source, key: &str) -> Box<dyn WeatherProvider + Send> {
    match source {
        Source::OpenWeather => Box::new(FreeProvider::new(key)),
        Source::OpenMeteo(model) => Box::new(OpenMeteoProvider::new(model)),
    }
}

fn apply_theme(ctx: &egui::Context, theme: &str) {
    ctx.set_theme(match theme {
        "light" => egui::ThemePreference::Light,
        "dark" => egui::ThemePreference::Dark,
        _ => egui::ThemePreference::System,
    });
}

/// Mở thư mục bằng trình quản lý file của hệ điều hành.
fn open_folder(path: &std::path::Path) {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(program).arg(path).spawn();
}

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
    snapshot: Option<Snapshot>,
    /// Đang hiển thị dữ liệu cache/cũ (lần làm mới gần nhất lỗi hoặc chưa xong).
    stale: bool,
    weather_error: Option<WeatherError>,
    loading: bool,
    scheduler: Scheduler,
    tx: Sender<WeatherResult>,
    rx: Receiver<WeatherResult>,
    api_key: String,
    cache_path: Option<PathBuf>,
    config: Config,
    config_path: Option<PathBuf>,
    data_dir: Option<PathBuf>,
    secrets: Option<KeyringStore>,
    /// Hộp thoại thông báo (lỗi bảng dung tích, cấu hình hỏng…).
    modal: Option<String>,
    weather_warning: Option<String>,
    history: Option<History>,
    history_rows: Vec<Record>,
    history_filter: String,
    history_dirty: bool,
    history_confirm_delete: Option<i64>,
    /// Lỗi ghi/đọc lịch sử: chỉ cảnh báo nhỏ, không chặn kết quả.
    storage_warning: Option<String>,
    export_notice: Option<String>,
    settings: SettingsForm,
    settings_errors: Vec<SettingsError>,
    settings_notice: Option<String>,
    key_input: String,
    table_path_input: String,
    table_notice: Option<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);
        let (tx, rx) = channel();
        let dirs = project_dirs();
        let cache_path = dirs.as_ref().map(|d| d.cache_dir().join("weather_cache.json"));
        let config_path = dirs.as_ref().map(|d| d.config_dir().join("config.toml"));
        let data_dir = dirs.as_ref().map(|d| d.data_dir().to_path_buf());
        let mut modal = None;
        let config = match config_path.as_deref().map(Config::load) {
            Some(Ok(c)) => c,
            Some(Err(e)) => {
                modal = Some(e.to_string());
                Config::default()
            }
            None => Config::default(),
        };
        let mut table = VolumeTable::builtin();
        if config.reservoir.volume_table != "builtin" {
            match load_volume_table(std::path::Path::new(&config.reservoir.volume_table)) {
                Ok(t) => table = t,
                Err(e) => modal = Some(format!("{e}\nĐang dùng bảng dung tích nhúng sẵn.")),
            }
        }
        let mut storage_warning = None;
        let history = data_dir.as_ref().and_then(|d| {
            std::fs::create_dir_all(d).ok()?;
            History::open(&d.join("history.db")).map_err(|e| storage_warning = Some(e.to_string())).ok()
        });
        let secrets = KeyringStore::new("luuluong", "openweather-api-key").ok();
        // Key trong Keychain/Credential Manager; biến môi trường chỉ để dev/test.
        let api_key = secrets
            .as_ref()
            .and_then(|s| s.get())
            .or_else(|| std::env::var("OPENWEATHER_API_KEY").ok())
            .unwrap_or_default();
        apply_theme(&cc.egui_ctx, &config.ui.theme);
        let snapshot = cache_path.as_deref().and_then(cache::load);
        let tab = match std::env::args().find_map(|a| a.strip_prefix("--tab=").map(str::to_owned)).as_deref() {
            Some("history") => Tab::History,
            Some("settings") => Tab::Settings,
            _ => Tab::Calc,
        };
        Self {
            table,
            turbine: Turbine { q_h1: config.turbine.q_h1, q_h2: config.turbine.q_h2 },
            form: Form::default(),
            result: None,
            submitted: false,
            tab,
            show_spill: false,
            confirm_quit: false,
            allow_close: false,
            focus_next: None,
            stale: snapshot.is_some(),
            snapshot,
            weather_error: None,
            loading: false,
            scheduler: Scheduler::new(config.weather.refresh_minutes),
            tx,
            rx,
            api_key,
            cache_path,
            settings: SettingsForm::from_config(&config),
            table_path_input: if config.reservoir.volume_table == "builtin" { String::new() } else { config.reservoir.volume_table.clone() },
            config,
            config_path,
            data_dir,
            secrets,
            modal,
            weather_warning: None,
            history,
            history_rows: Vec::new(),
            history_filter: String::new(),
            history_dirty: true,
            history_confirm_delete: None,
            storage_warning,
            export_notice: None,
            settings_errors: Vec::new(),
            settings_notice: None,
            key_input: String::new(),
            table_notice: None,
        }
    }

    fn source(&self) -> Source {
        Source::from_id(&self.config.weather.source).unwrap_or(Source::OpenMeteo(Model::BestMatch))
    }

    /// Nguồn chính do người dùng chọn; nguồn dự phòng khác dịch vụ để không cùng lỗi một lúc.
    fn start_fetch(&mut self, ctx: &egui::Context) {
        self.loading = true;
        self.scheduler.mark_attempt(unix_now());
        let source = self.source();
        let key = self.api_key.clone();
        let fallback = match source {
            Source::OpenWeather => Some(make_provider(Source::OpenMeteo(Model::BestMatch), &key)),
            Source::OpenMeteo(_) if !key.trim().is_empty() => Some(make_provider(Source::OpenWeather, &key)),
            Source::OpenMeteo(_) => None,
        };
        let primary = make_provider(source, &key);
        let (lat, lon) = (self.config.weather.lat, self.config.weather.lon);
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        std::thread::spawn(move || {
            let mut chain: Vec<&dyn WeatherProvider> = vec![primary.as_ref()];
            if let Some(f) = &fallback {
                chain.push(f.as_ref());
            }
            let _ = tx.send(fetch_chain(&chain, lat, lon));
            ctx.request_repaint();
        });
    }

    fn poll_weather(&mut self, ctx: &egui::Context) {
        while let Ok(result) = self.rx.try_recv() {
            let now = unix_now();
            self.loading = false;
            match result {
                Ok(Fetched { snapshot, primary_error }) => {
                    self.weather_warning = primary_error
                        .map(|e| format!("Nguồn đã chọn lỗi ({e}) Đang dùng nguồn dự phòng: {}.", snapshot.source));
                    if let Some(path) = &self.cache_path {
                        if let Err(e) = cache::save(path, &snapshot) {
                            eprintln!("không ghi được cache thời tiết: {e}");
                        }
                    }
                    self.snapshot = Some(snapshot);
                    self.stale = false;
                    self.weather_error = None;
                    self.scheduler.record_ok(now);
                }
                Err(e) => {
                    self.scheduler.record_err(now, &e);
                    self.weather_warning = None;
                    self.stale = true;
                    self.weather_error = Some(e);
                }
            }
        }
        if !self.loading && self.scheduler.due(unix_now()) {
            self.start_fetch(ctx);
        }
        ctx.request_repaint_after(Duration::from_secs(30));
    }

    fn refresh_manually(&mut self, ctx: &egui::Context) {
        if !self.loading && self.scheduler.manual_allowed(unix_now()) {
            self.start_fetch(ctx);
        }
    }

    fn save_config(&mut self) -> Result<(), String> {
        let path = self.config_path.as_ref().ok_or("Không xác định được thư mục cấu hình.")?;
        self.config.save(path).map_err(|e| e.to_string())
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
                if let (Some(o), Some(h)) = (&self.result, &self.history) {
                    let record = Record {
                        id: 0,
                        created_at: vn_timestamp(unix_now()),
                        z_start: inputs.z_start,
                        z_end: inputs.z_end,
                        hours: inputs.hours,
                        h1: inputs.h1,
                        h2: inputs.h2,
                        spill_q: inputs.spill_q,
                        spill_h: inputs.spill_h,
                        eco_q: inputs.eco_q,
                        q_in: o.q_in,
                        w_in: o.w_in,
                        operator: self.config.ui.operator.clone(),
                        note: String::new(),
                    };
                    match h.insert(&record) {
                        Ok(_) => {
                            self.history_dirty = true;
                            self.storage_warning = None;
                        }
                        Err(e) => self.storage_warning = Some(e.to_string()),
                    }
                }
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
        let (calc, clear, t1, t2, t3, refresh) = ctx.input_mut(|i| {
            (
                i.consume_key(Modifiers::COMMAND, Key::Enter),
                i.consume_key(Modifiers::COMMAND, Key::Backspace),
                i.consume_key(Modifiers::COMMAND, Key::Num1),
                i.consume_key(Modifiers::COMMAND, Key::Num2),
                i.consume_key(Modifiers::COMMAND, Key::Num3),
                i.consume_key(Modifiers::COMMAND, Key::R),
            )
        });
        if refresh {
            self.refresh_manually(ctx);
        }
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
        self.poll_weather(ctx);

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
                            let next = if dark { "light" } else { "dark" };
                            apply_theme(ctx, next);
                            self.config.ui.theme = next.into();
                            self.settings.theme = next.into();
                            let _ = self.save_config();
                        }
                    });
                });
            });

        egui::TopBottomPanel::bottom("status")
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::symmetric(16, 6)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.weather_status()).size(11.0).color(p.muted));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(RichText::new(format!("Phiên bản {VERSION}")).size(11.0).color(p.muted));
                    });
                });
            });

        egui::CentralPanel::default().frame(Frame::new().fill(p.bg).inner_margin(Margin::same(16))).show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match self.tab {
                Tab::Calc => self.calc_screen(ui, p),
                Tab::History => self.history_screen(ui, p),
                Tab::Settings => self.settings_screen(ui, p),
            });
        });

        if let Some(message) = self.modal.clone() {
            egui::Window::new("Thông báo")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.set_max_width(420.0);
                    ui.label(message);
                    ui.add_space(8.0);
                    if ui.button("Đã hiểu").clicked() {
                        self.modal = None;
                    }
                });
        }

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
                for pair in REQUIRED.chunks(2) {
                    ui.columns(2, |cols| {
                        for (col, f) in cols.iter_mut().zip(pair) {
                            enter_pressed |= self.input_row(col, p, *f, &errors, &hint);
                        }
                    });
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

        ui.add_space(16.0);
        self.weather_strip(ui, p);

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

    fn weather_status(&self) -> String {
        match (&self.weather_error, &self.snapshot) {
            (Some(e), _) => format!("Thời tiết: {e}"),
            (None, Some(s)) => format!("Thời tiết: {}", view::updated_label(s.fetched_at, s.tz_offset, self.stale)),
            (None, None) if self.loading => "Thời tiết: đang tải…".into(),
            (None, None) => "Thời tiết: chưa có dữ liệu".into(),
        }
    }

    fn weather_strip(&mut self, ui: &mut egui::Ui, p: &Palette) {
        ui.horizontal(|ui| {
            title(ui, p, "Thời tiết A Lưới");
            if let Some(s) = &self.snapshot {
                let label = format!("{} · {}", s.source, view::updated_label(s.fetched_at, s.tz_offset, self.stale));
                ui.label(RichText::new(label).size(11.0).color(p.muted));
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let can = !self.loading && self.scheduler.manual_allowed(unix_now());
                let label = if self.loading { "Đang tải…" } else { "Làm mới" };
                let ctx = ui.ctx().clone();
                if ui.add_enabled(can, egui::Button::new(label)).on_hover_text("Ctrl/Cmd + R (tối đa 1 lần / 30 giây)").clicked() {
                    self.refresh_manually(&ctx);
                }
            });
        });
        if let Some(e) = &self.weather_error {
            ui.label(RichText::new(format!("⚠ {e}")).size(12.0).color(p.error));
        }
        if let Some(w) = &self.weather_warning {
            ui.label(RichText::new(format!("⚠ {w}")).size(12.0).color(p.error));
        }
        ui.add_space(4.0);
        let Some(s) = self.snapshot.clone() else {
            card(p).show(ui, |ui| {
                ui.set_width(ui.available_width());
                let msg = if self.loading {
                    "Đang tải thời tiết…"
                } else if self.api_key.trim().is_empty() {
                    "Chưa có API key OpenWeather: đặt biến môi trường OPENWEATHER_API_KEY rồi mở lại app (mục Cài đặt sẽ có ở giai đoạn 4). Phần tính lưu lượng vẫn dùng bình thường."
                } else {
                    "Chưa có dữ liệu thời tiết. Phần tính lưu lượng vẫn dùng bình thường."
                };
                ui.label(RichText::new(msg).color(p.muted));
            });
            return;
        };
        ui.columns(6, |cols| {
            card(p).show(&mut cols[0], |ui| {
                ui.set_width(ui.available_width());
                ui.label(RichText::new("Hiện tại").size(13.0).color(p.primary));
                ui.label(RichText::new(format!("{}°C", format_vn(s.current.temp, 1))).font(FontId::new(24.0, theme::semibold())));
                ui.label(RichText::new(&s.current.description).size(12.0));
                ui.label(RichText::new(format!("Ẩm {}% · Gió {} m/s", format_vn(s.current.humidity, 0), format_vn(s.current.wind_speed, 1))).size(11.0).color(p.muted));
            });
            for (col, day) in cols[1..].iter_mut().zip(s.days.iter().take(5)) {
                let alert = view::rain_alert(day, self.config.weather.rain_alert_mm);
                let frame = if alert == view::RainAlert::Primary { card(p).stroke(Stroke::new(2.0, p.error)) } else { card(p) };
                frame.show(col, |ui| {
                    ui.set_width(ui.available_width());
                    let head = format!("{} · {}", view::weekday_vn(day.date), day.date.format("%d/%m"));
                    ui.label(RichText::new(head).font(FontId::new(13.0, theme::semibold())).color(p.primary));
                    ui.label(RichText::new(&day.description).size(12.0));
                    ui.label(RichText::new(format!("{}° / {}°", format_vn(day.temp_min, 0), format_vn(day.temp_max, 0))).size(14.0));
                    let rain = RichText::new(format!("Mưa {} mm", format_vn(day.rain_mm, 1))).font(FontId::new(13.0, theme::semibold()));
                    ui.label(if alert == view::RainAlert::Primary { rain.color(p.error) } else { rain.color(p.accent_text) });
                    ui.label(RichText::new(format!("Xác suất {}%", format_vn(day.pop_percent, 0))).size(11.0).color(p.muted));
                    for r in &day.model_rain {
                        ui.label(RichText::new(format!("{} {} mm", r.model, format_vn(r.rain_mm, 1))).size(11.0).color(p.muted));
                    }
                    match alert {
                        view::RainAlert::Primary => {
                            ui.label(RichText::new("⚠ Mưa lớn").size(11.0).color(p.error));
                        }
                        view::RainAlert::OtherModels => {
                            ui.label(RichText::new("⚠ Có mô hình báo mưa lớn").size(11.0).color(p.error));
                        }
                        view::RainAlert::None => {}
                    }
                    if day.partial {
                        ui.label(RichText::new("dữ liệu một phần").size(11.0).color(p.muted));
                    }
                });
            }
        });
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
                    .desired_width((ui.available_width() - 64.0).clamp(90.0, 200.0))
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
            if let Some(w) = &self.storage_warning {
                ui.add_space(6.0);
                ui.label(RichText::new(format!("⚠ {w} Kết quả vẫn đúng.")).size(11.0).color(p.error));
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
