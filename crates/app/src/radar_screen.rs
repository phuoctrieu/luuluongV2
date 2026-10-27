//! Thẻ radar mưa: bản đồ OpenStreetMap + khung radar RainViewer, phát lại khoảng 1 giờ gần nhất.

use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use eframe::egui::{self, Color32, ColorImage, CornerRadius, FontId, Rect, RichText, Stroke, TextureHandle, TextureOptions, Vec2};
use luuluong_weather::radar::{grid_around, Grid, RadarClient, RadarError, Rgba};

use super::{card, title, unix_now, view, App, Palette, Tab};

const ZOOM: u8 = 7;
const FRAME_COUNT: usize = 6;
const REFRESH_SECS: i64 = 600;
const MAP_HEIGHT: f32 = 230.0;
const STEP_SECS: f64 = 0.7;
/// Nhà máy ở Việt Nam: hiển thị giờ UTC+7.
const VN_OFFSET: i32 = 7 * 3600;
const UV: Rect = Rect { min: egui::pos2(0.0, 0.0), max: egui::pos2(1.0, 1.0) };

enum Msg {
    Base(usize, Rgba),
    Frame { time: i64, tiles: Vec<(usize, Rgba)> },
    /// Danh sách thời điểm khung còn hiệu lực sau lần tải này.
    Done(Vec<i64>),
    Failed(RadarError),
}

struct RadarFrame {
    time: i64,
    tiles: [Option<TextureHandle>; 9],
}

pub struct RadarState {
    center: (f64, f64),
    grid: Grid,
    base: [Option<TextureHandle>; 9],
    frames: Vec<RadarFrame>,
    loading: bool,
    error: Option<RadarError>,
    index: usize,
    playing: bool,
    first_load: bool,
    last_step: f64,
    last_start: i64,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    cache_dir: Option<PathBuf>,
}

fn retry<T>(mut f: impl FnMut() -> Result<T, RadarError>) -> Result<T, RadarError> {
    f().or_else(|_| f())
}

fn texture(ctx: &egui::Context, name: String, img: &Rgba) -> TextureHandle {
    ctx.load_texture(name, ColorImage::from_rgba_unmultiplied([img.width, img.height], &img.data), TextureOptions::LINEAR)
}

impl RadarState {
    pub fn new(lat: f64, lon: f64, cache_dir: Option<PathBuf>) -> Self {
        let (tx, rx) = channel();
        Self {
            center: (lat, lon),
            grid: grid_around(lat, lon, ZOOM),
            base: Default::default(),
            frames: Vec::new(),
            loading: false,
            error: None,
            index: 0,
            playing: false,
            first_load: true,
            last_step: 0.0,
            last_start: 0,
            tx,
            rx,
            cache_dir,
        }
    }

    /// Tải bản đồ nền (nếu chưa có) và các khung radar mới, song song từng tile.
    fn start_fetch(&mut self, ctx: &egui::Context) {
        self.loading = true;
        self.error = None;
        self.last_start = unix_now();
        let (tx, ctx) = (self.tx.clone(), ctx.clone());
        let (tiles, z) = (self.grid.tiles, self.grid.z);
        let need_base = self.base.iter().any(Option::is_none);
        let known: Vec<i64> = self.frames.iter().map(|f| f.time).collect();
        let cache_dir = self.cache_dir.clone();
        std::thread::spawn(move || {
            let client = RadarClient::new(cache_dir);
            let client = &client;
            if need_base {
                std::thread::scope(|s| {
                    for (i, (x, y)) in tiles.iter().enumerate() {
                        let (tx, ctx) = (&tx, &ctx);
                        s.spawn(move || {
                            if let Ok(img) = retry(|| client.base_tile(z, *x, *y)) {
                                let _ = tx.send(Msg::Base(i, img));
                                ctx.request_repaint();
                            }
                        });
                    }
                });
            }
            let frames = match retry(|| client.frames(FRAME_COUNT)) {
                Ok(f) => f,
                Err(e) => {
                    let _ = tx.send(Msg::Failed(e));
                    ctx.request_repaint();
                    return;
                }
            };
            for frame in frames.frames.iter().rev() {
                if known.contains(&frame.time) {
                    continue;
                }
                let frames = &frames;
                let tiles_out: Vec<(usize, Rgba)> = std::thread::scope(|s| {
                    let handles: Vec<_> = tiles
                        .iter()
                        .enumerate()
                        .map(|(i, (x, y))| s.spawn(move || retry(|| client.radar_tile(frames, frame, z, *x, *y)).ok().map(|img| (i, img))))
                        .collect();
                    handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect()
                });
                let _ = tx.send(Msg::Frame { time: frame.time, tiles: tiles_out });
                ctx.request_repaint();
            }
            let _ = tx.send(Msg::Done(frames.frames.iter().map(|f| f.time).collect()));
            ctx.request_repaint();
        });
    }

    fn poll(&mut self, ctx: &egui::Context, autoplay: bool) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Base(i, img) => self.base[i] = Some(texture(ctx, format!("osm-{}-{i}", self.grid.z), &img)),
                Msg::Frame { time, tiles } => {
                    let mut slots: [Option<TextureHandle>; 9] = Default::default();
                    for (i, img) in tiles {
                        slots[i] = Some(texture(ctx, format!("radar-{time}-{i}"), &img));
                    }
                    let at = self.frames.partition_point(|f| f.time < time);
                    self.frames.insert(at, RadarFrame { time, tiles: slots });
                    // Khung mới nhất luôn hiện đầu tiên khi đang tải.
                    if !self.playing {
                        self.index = self.frames.len() - 1;
                    }
                }
                Msg::Done(keep) => {
                    self.loading = false;
                    self.frames.retain(|f| keep.contains(&f.time));
                    self.index = self.index.min(self.frames.len().saturating_sub(1));
                    if self.first_load && autoplay && self.frames.len() > 1 {
                        self.playing = true;
                        self.index = 0;
                    }
                    self.first_load = false;
                }
                Msg::Failed(e) => {
                    self.loading = false;
                    self.error = Some(e);
                }
            }
        }
    }
}

impl App {
    /// Gọi mỗi khung hình: nạp dữ liệu mới, tự làm mới mỗi 10 phút, tự phát lại.
    pub(super) fn radar_tick(&mut self, ctx: &egui::Context) {
        let center = (self.config.weather.lat, self.config.weather.lon);
        if center != self.radar.center {
            self.radar = RadarState::new(center.0, center.1, self.radar.cache_dir.clone());
        }
        let animate = self.config.ui.animations;
        self.radar.poll(ctx, animate);
        if !self.radar.loading && unix_now() - self.radar.last_start >= REFRESH_SECS {
            self.radar.start_fetch(ctx);
        }
        let on_screen = self.tab == Tab::Calc;
        if on_screen && animate && self.radar.playing && self.radar.frames.len() > 1 {
            let now = ctx.input(|i| i.time);
            if now - self.radar.last_step >= STEP_SECS {
                self.radar.index = view::next_frame(self.radar.index, self.radar.frames.len());
                self.radar.last_step = now;
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    pub(super) fn radar_refresh(&mut self, ctx: &egui::Context) {
        if !self.radar.loading {
            self.radar.start_fetch(ctx);
        }
    }

    pub(super) fn radar_card(&mut self, ui: &mut egui::Ui, p: &Palette) {
        let dark = ui.ctx().theme() == egui::Theme::Dark;
        card(p).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                title(ui, p, "Radar mưa");
                if self.radar.loading {
                    ui.label(RichText::new("đang tải…").size(11.0).color(p.muted));
                }
            });
            ui.add_space(4.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), MAP_HEIGHT), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let sea = if dark { Color32::from_rgb(0x12, 0x1B, 0x30) } else { Color32::from_rgb(0xE6, 0xEA, 0xF0) };
            painter.rect_filled(rect, CornerRadius::same(6), sea);

            let (px, py) = self.radar.grid.point_px;
            let origin = rect.center() - Vec2::new(px as f32, py as f32);
            let tile_rect = |i: usize| Rect::from_min_size(origin + Vec2::new((i % 3) as f32 * 256.0, (i / 3) as f32 * 256.0), Vec2::splat(256.0));
            let base_tint = if dark { Color32::from_gray(150) } else { Color32::WHITE };
            for (i, tex) in self.radar.base.iter().enumerate() {
                if let Some(t) = tex {
                    painter.image(t.id(), tile_rect(i), UV, base_tint);
                }
            }
            if dark {
                painter.rect_filled(rect, CornerRadius::same(6), Color32::from_rgba_unmultiplied(8, 14, 30, 110));
            }
            let frame = self.radar.frames.get(self.radar.index);
            if let Some(f) = frame {
                for (i, tex) in f.tiles.iter().enumerate() {
                    if let Some(t) = tex {
                        painter.image(t.id(), tile_rect(i), UV, Color32::from_white_alpha(225));
                    }
                }
            }

            // Vị trí nhà máy.
            let c = rect.center();
            painter.circle_stroke(c, 12.0, Stroke::new(1.5, p.accent));
            painter.circle_filled(c, 6.0, p.accent);
            painter.circle_stroke(c, 6.0, Stroke::new(2.0, Color32::WHITE));
            let pill = |text: &str, pos: egui::Pos2, align_right: bool| {
                let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(11.0), Color32::WHITE);
                let size = galley.size();
                let min = if align_right { pos - Vec2::new(size.x + 8.0, size.y + 4.0) } else { pos };
                painter.rect_filled(Rect::from_min_size(min, size + Vec2::new(8.0, 4.0)), CornerRadius::same(4), Color32::from_black_alpha(175));
                painter.galley(min + Vec2::new(4.0, 2.0), galley, Color32::WHITE);
            };
            pill("Nhà máy", c + Vec2::new(16.0, -9.0), false);
            pill("© OpenStreetMap · Radar: RainViewer", rect.right_bottom() - Vec2::new(4.0, 4.0), true);
            if frame.is_none() {
                let msg = match (&self.radar.error, self.radar.loading) {
                    (Some(e), _) => format!("⚠ {e}"),
                    (None, true) => "Đang tải radar mưa…".to_owned(),
                    (None, false) => "Chưa có dữ liệu radar.".to_owned(),
                };
                pill(&msg, rect.center() + Vec2::new(-60.0, 28.0), false);
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let n = self.radar.frames.len();
                let label = if self.radar.playing { "⏸" } else { "▶" };
                let play = egui::Button::new(RichText::new(label).size(15.0)).min_size(Vec2::new(36.0, 32.0));
                if ui.add_enabled(n > 1, play).on_hover_text("Phát / dừng").clicked() {
                    self.radar.playing = !self.radar.playing;
                    self.radar.last_step = ui.input(|i| i.time);
                }
                let mut idx = self.radar.index.min(n.saturating_sub(1));
                let slider = egui::Slider::new(&mut idx, 0..=n.saturating_sub(1)).show_value(false);
                if ui.add_enabled(n > 1, slider).changed() {
                    self.radar.index = idx;
                    self.radar.playing = false;
                }
                if let Some(f) = self.radar.frames.get(self.radar.index) {
                    let text = format!("{} · {}", view::clock(f.time, VN_OFFSET), view::age_label(f.time, unix_now()));
                    ui.label(RichText::new(text).size(12.0));
                }
                if self.radar.error.is_some() && ui.button("Thử lại").clicked() {
                    let ctx = ui.ctx().clone();
                    self.radar.start_fetch(&ctx);
                }
            });
            ui.label(
                RichText::new("Xanh nhạt: mưa nhẹ · xanh đậm, vàng: mưa vừa · đỏ: mưa rất to. Mỗi khung cách nhau 10 phút.")
                    .size(11.0)
                    .color(p.muted),
            );
        });
    }
}
