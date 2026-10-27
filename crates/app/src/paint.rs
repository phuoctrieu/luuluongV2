//! Hiệu ứng nền thời tiết vẽ bằng code và biểu tượng thời tiết nhúng sẵn (chạy được khi mất mạng).
//! Hiệu ứng chỉ động khi `animate` bật; tắt thì vẽ khung tĩnh.

use eframe::egui::{self, epaint::Mesh, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, Vec2};

use crate::view::Sky;

/// Số giả ngẫu nhiên ổn định theo (i, salt) trong [0, 1].
fn hash01(i: u32, salt: u32) -> f32 {
    let mut x = i.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA6B);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    (x & 0xFFFF) as f32 / 65535.0
}

/// Màu nền (trên, dưới) cho từng dạng trời; chữ trắng trên đó đạt ≥ 5:1 (đã đo).
pub fn sky_gradient(sky: Sky, night: bool) -> (Color32, Color32) {
    let c = |h: u32| Color32::from_rgb((h >> 16) as u8, (h >> 8) as u8, h as u8);
    if night && !sky.is_rainy() {
        return (c(0x0B1230), c(0x1B2A52));
    }
    match sky {
        Sky::Clear => (c(0x1B5A9E), c(0x2C6DB3)),
        Sky::PartlyCloudy => (c(0x24528A), c(0x3A669F)),
        Sky::Cloudy => (c(0x3F5473), c(0x52688A)),
        Sky::Fog => (c(0x4A5A70), c(0x5A6B82)),
        Sky::Drizzle | Sky::Rain | Sky::HeavyRain => (c(0x2B3A58), c(0x40557A)),
        Sky::Thunder => (c(0x221D3B), c(0x37325C)),
    }
}

fn cloud(painter: &Painter, center: Pos2, size: f32, fill: Color32) {
    let r = size * 0.5;
    for (dx, dy, k) in [(-0.9, 0.15, 0.62), (-0.2, -0.25, 0.85), (0.6, 0.0, 0.7), (1.2, 0.2, 0.5)] {
        painter.circle_filled(center + Vec2::new(dx * r, dy * r), k * r, fill);
    }
    let base = Rect::from_center_size(center + Vec2::new(0.15 * r, 0.35 * r), Vec2::new(3.0 * r, 0.7 * r));
    painter.rect_filled(base, CornerRadius::same((0.35 * r) as u8), fill);
}

/// Nền hiện tượng thời tiết cho thẻ "hiện tại". Vẽ trong `rect` (cắt theo `rect`).
pub fn hero_background(painter: &Painter, rect: Rect, sky: Sky, night: bool, time: f64, animate: bool) {
    let painter = painter.with_clip_rect(rect);
    let (top, bottom) = sky_gradient(sky, night);
    let cap = 8.0;
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(rect.width(), cap)),
        CornerRadius { nw: 8, ne: 8, sw: 0, se: 0 },
        top,
    );
    painter.rect_filled(
        Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - cap), rect.max),
        CornerRadius { nw: 0, ne: 0, sw: 8, se: 8 },
        bottom,
    );
    let mut mesh = Mesh::default();
    let (y0, y1) = (rect.min.y + cap, rect.max.y - cap);
    for (y, c) in [(y0, top), (y1, bottom)] {
        mesh.colored_vertex(Pos2::new(rect.min.x, y), c);
        mesh.colored_vertex(Pos2::new(rect.max.x, y), c);
    }
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(Shape::mesh(mesh));

    let t = if animate { time as f32 } else { 0.0 };
    let (w, h) = (rect.width(), rect.height());
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y);

    // Mặt trời hoặc trăng sao ở góc phải.
    let clear_sky = matches!(sky, Sky::Clear | Sky::PartlyCloudy);
    if clear_sky && !night {
        let c = at(w * 0.82, h * 0.34);
        for (k, a) in [(70.0, 14), (52.0, 22), (38.0, 34), (28.0, 60)] {
            painter.circle_filled(c, k, Color32::from_rgba_unmultiplied(255, 214, 102, a));
        }
        painter.circle_filled(c, 20.0, Color32::from_rgb(255, 214, 102));
        for i in 0..12 {
            let ang = i as f32 / 12.0 * std::f32::consts::TAU + t * 0.12;
            let (s, co) = ang.sin_cos();
            painter.line_segment(
                [c + Vec2::new(co, s) * 28.0, c + Vec2::new(co, s) * 38.0],
                Stroke::new(2.0, Color32::from_rgba_unmultiplied(255, 214, 102, 170)),
            );
        }
    }
    if night && !sky.is_rainy() {
        for i in 0..28 {
            let tw = if animate { 0.5 + 0.5 * (t * 1.4 + hash01(i, 3) * std::f32::consts::TAU).sin() } else { 0.8 };
            let a = (80.0 + 150.0 * tw) as u8;
            painter.circle_filled(at(hash01(i, 1) * w, hash01(i, 2) * h * 0.8), 1.0 + hash01(i, 4), Color32::from_rgba_unmultiplied(255, 255, 255, a));
        }
        if clear_sky {
            let c = at(w * 0.82, h * 0.34);
            painter.circle_filled(c, 18.0, Color32::from_rgb(244, 241, 222));
            painter.circle_filled(c + Vec2::new(8.0, -5.0), 16.0, sky_gradient(sky, true).0);
        }
    }

    // Mây trôi chậm.
    let cloud_count = match sky {
        Sky::Clear => 0,
        Sky::PartlyCloudy => 2,
        Sky::Fog => 0,
        _ => 4,
    };
    let cloud_tint = if night || sky.is_rainy() { Color32::from_rgba_unmultiplied(160, 175, 200, 70) } else { Color32::from_rgba_unmultiplied(255, 255, 255, 90) };
    for i in 0..cloud_count {
        let size = 54.0 + hash01(i, 5) * 40.0;
        let speed = 6.0 + hash01(i, 6) * 8.0;
        let x = (hash01(i, 7) * (w + size * 3.0) + t * speed) % (w + size * 3.0) - size * 1.5;
        cloud(&painter, at(x, h * (0.15 + 0.5 * hash01(i, 8))), size, cloud_tint);
    }

    // Sương mù: vài dải mờ trôi ngang.
    if sky == Sky::Fog {
        for i in 0..4 {
            let y = h * (0.2 + 0.2 * i as f32);
            let dx = (t * (4.0 + i as f32) + hash01(i, 9) * 40.0).sin() * 24.0;
            painter.rect_filled(Rect::from_min_size(at(-30.0 + dx, y), Vec2::new(w + 60.0, 16.0)), CornerRadius::same(8), Color32::from_rgba_unmultiplied(255, 255, 255, 34));
        }
    }

    // Mưa: vệt rơi, nhiều và nhanh hơn khi mưa to.
    let (drops, speed, alpha) = match sky {
        Sky::Drizzle => (22, 220.0, 70),
        Sky::Rain => (48, 330.0, 95),
        Sky::HeavyRain => (90, 440.0, 115),
        Sky::Thunder => (80, 420.0, 105),
        _ => (0, 0.0, 0),
    };
    for i in 0..drops {
        let len = 9.0 + hash01(i, 11) * 9.0;
        let travel = h + len;
        let y = (hash01(i, 12) * travel + t * speed * (0.8 + 0.4 * hash01(i, 13))) % travel - len;
        let x = hash01(i, 14) * (w + 40.0) - 20.0 + y * 0.18;
        painter.line_segment([at(x, y), at(x - len * 0.18, y + len)], Stroke::new(1.3, Color32::from_rgba_unmultiplied(190, 215, 255, alpha)));
    }

    // Dông: chớp sáng thỉnh thoảng (khoảng 6 giây một lần) và tia sét.
    if sky == Sky::Thunder && animate {
        let phase = (t / 6.3).fract();
        if phase < 0.05 {
            let k = 1.0 - phase / 0.05;
            painter.rect_filled(rect, CornerRadius::ZERO, Color32::from_rgba_unmultiplied(255, 255, 255, (70.0 * k) as u8));
            let pts: Vec<Pos2> = [(0.0, 0.0), (-10.0, 22.0), (4.0, 24.0), (-12.0, 54.0)].iter().map(|(dx, dy)| at(w * 0.7 + dx, h * 0.12 + dy)).collect();
            painter.add(Shape::line(pts, Stroke::new(2.5, Color32::from_rgba_unmultiplied(255, 240, 160, (255.0 * k) as u8))));
        }
    }
}

/// Biểu tượng thời tiết dạng ảnh (Meteocons của Bas Milius, giấy phép MIT, xem `assets/weather/`), nạp một lần rồi giữ trong bộ nhớ tạm của egui.
pub fn sky_icon(ctx: &egui::Context, sky: Sky, night: bool) -> egui::TextureHandle {
    let (name, bytes): (&str, &[u8]) = match (sky, night) {
        (Sky::Clear, false) => ("clear-day", include_bytes!("../assets/weather/clear-day.png")),
        (Sky::Clear, true) => ("clear-night", include_bytes!("../assets/weather/clear-night.png")),
        (Sky::PartlyCloudy, false) => ("partly-cloudy-day", include_bytes!("../assets/weather/partly-cloudy-day.png")),
        (Sky::PartlyCloudy, true) => ("partly-cloudy-night", include_bytes!("../assets/weather/partly-cloudy-night.png")),
        (Sky::Cloudy, _) => ("overcast", include_bytes!("../assets/weather/overcast.png")),
        (Sky::Fog, _) => ("fog", include_bytes!("../assets/weather/fog.png")),
        (Sky::Drizzle, _) => ("drizzle", include_bytes!("../assets/weather/drizzle.png")),
        (Sky::Rain, _) => ("rain", include_bytes!("../assets/weather/rain.png")),
        (Sky::HeavyRain, _) => ("extreme-rain", include_bytes!("../assets/weather/extreme-rain.png")),
        (Sky::Thunder, _) => ("thunderstorms-rain", include_bytes!("../assets/weather/thunderstorms-rain.png")),
    };
    let id = egui::Id::new(("sky_icon", name));
    if let Some(texture) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return texture;
    }
    // Ảnh nhúng sẵn lúc build nên giải mã không thể lỗi.
    let img = luuluong_weather::radar::decode_png(bytes).expect("ảnh biểu tượng nhúng sẵn hợp lệ");
    let texture = ctx.load_texture(name, egui::ColorImage::from_rgba_unmultiplied([img.width, img.height], &img.data), egui::TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, texture.clone()));
    texture
}
