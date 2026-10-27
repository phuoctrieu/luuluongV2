//! Bảng màu sáng/tối (đặc tả 6.3) và font nhúng. Mọi cặp chữ/nền đã đo ≥ 4,5:1.

use eframe::egui::{self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, Stroke, Theme};

pub struct Palette {
    pub bg: Color32,
    pub card: Color32,
    pub border: Color32,
    pub header_bg: Color32,
    pub header_text: Color32,
    pub header_muted: Color32,
    pub primary: Color32,
    /// Nền nút TÍNH.
    pub accent: Color32,
    /// Chữ trên nút TÍNH.
    pub on_accent: Color32,
    /// Màu cam dùng cho chữ (số kết quả): đậm hơn ở giao diện sáng để đạt 4,5:1.
    pub accent_text: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub error: Color32,
    /// Chữ trên nền màu lỗi (chip cảnh báo).
    pub on_error: Color32,
}

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

pub const LIGHT: Palette = Palette {
    bg: rgb(0xF5F7FA),
    card: rgb(0xFFFFFF),
    border: rgb(0xD5DBE6),
    header_bg: rgb(0x324E7B),
    header_text: rgb(0xFFFFFF),
    header_muted: rgb(0xD6E0F5),
    primary: rgb(0x324E7B),
    accent: rgb(0xFF8C32),
    on_accent: rgb(0x06113C),
    accent_text: rgb(0xB84A00),
    text: rgb(0x06113C),
    muted: rgb(0x4A5575),
    error: rgb(0xC62828),
    on_error: rgb(0xFFFFFF),
};

pub const DARK: Palette = Palette {
    bg: rgb(0x0F1626),
    card: rgb(0x18233A),
    border: rgb(0x2C3A57),
    header_bg: rgb(0x18233A),
    header_text: rgb(0xE8EDF7),
    header_muted: rgb(0xA9B6D0),
    primary: rgb(0x86A6DF),
    accent: rgb(0xFF9F52),
    on_accent: rgb(0x06113C),
    accent_text: rgb(0xFF9F52),
    text: rgb(0xE8EDF7),
    muted: rgb(0xA9B6D0),
    error: rgb(0xFF6B6B),
    on_error: rgb(0x06113C),
};

pub fn palette(ctx: &egui::Context) -> &'static Palette {
    if ctx.theme() == Theme::Dark {
        &DARK
    } else {
        &LIGHT
    }
}

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let faces: [(&str, &'static [u8]); 3] = [
        ("bvp-regular", include_bytes!("../assets/BeVietnamPro-Regular.ttf")),
        ("bvp-medium", include_bytes!("../assets/BeVietnamPro-Medium.ttf")),
        ("bvp-semibold", include_bytes!("../assets/BeVietnamPro-SemiBold.ttf")),
    ];
    for (name, bytes) in faces {
        fonts.font_data.insert(name.into(), std::sync::Arc::new(FontData::from_static(bytes)));
    }
    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.insert(0, "bvp-regular".into());
    // Giữ font dự phòng của egui để ký hiệu như ✓ → ⚠ không hiện thành "?".
    let mut bold = vec!["bvp-semibold".to_owned()];
    bold.extend(prop.iter().cloned());
    fonts.families.insert(semibold(), bold);
    ctx.set_fonts(fonts);

    for (theme, p) in [(Theme::Light, &LIGHT), (Theme::Dark, &DARK)] {
        ctx.set_visuals_of(theme, visuals(theme, p));
    }
}

fn visuals(theme: Theme, p: &Palette) -> egui::Visuals {
    let mut v = if theme == Theme::Dark { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.bg;
    v.window_fill = p.card;
    v.extreme_bg_color = p.card; // nền ô nhập
    v.override_text_color = Some(p.text);
    v.selection.bg_fill = p.primary.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.primary);
    let r = CornerRadius::same(8);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_stroke = Stroke::new(1.5, p.primary);
    v.window_corner_radius = r;
    v
}
