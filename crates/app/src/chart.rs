//! Biểu đồ cột nhóm: mưa dự kiến theo ngày, mỗi nguồn một màu, có nhãn giá trị và vạch ngưỡng cảnh báo.

use eframe::egui::{self, Color32, FontId, RichText};
use egui_plot::{Bar, BarChart, HLine, Legend, Plot, PlotPoint, Text};
use luuluong_core::validate::format_vn;

use crate::theme::Palette;
use crate::view::Series;

/// Bảng màu Okabe–Ito (phân biệt được cả khi mù màu); thứ tự cố định theo nguồn.
const SERIES_COLORS: [Color32; 5] = [
    Color32::from_rgb(0x00, 0x72, 0xB2),
    Color32::from_rgb(0xE6, 0x9F, 0x00),
    Color32::from_rgb(0x00, 0x9E, 0x73),
    Color32::from_rgb(0xD5, 0x5E, 0x00),
    Color32::from_rgb(0xCC, 0x79, 0xA7),
];

pub fn series_color(i: usize) -> Color32 {
    SERIES_COLORS[i % SERIES_COLORS.len()]
}

pub fn rain_chart(ui: &mut egui::Ui, p: &Palette, day_labels: &[String], series: &[Series], threshold_mm: f64, height: f32) {
    let n = series.len().max(1) as f64;
    let group_width = 0.8;
    let bar_width = group_width / n;
    let labels = day_labels.to_vec();
    let max_value = series.iter().flat_map(|s| s.values.iter().flatten()).cloned().fold(threshold_mm, f64::max);

    Plot::new("rain_compare")
        .height(height)
        .legend(Legend::default())
        .allow_zoom(false)
        .allow_drag(false)
        .allow_scroll(false)
        .allow_boxed_zoom(false)
        .include_y(0.0)
        .include_y(max_value * 1.15)
        .y_axis_label("mm / ngày")
        .x_axis_formatter(move |mark, _| {
            let i = mark.value.round();
            if (mark.value - i).abs() < 0.05 && i >= 0.0 { labels.get(i as usize).cloned().unwrap_or_default() } else { String::new() }
        })
        .x_grid_spacer(|_| (0..8).map(|i| egui_plot::GridMark { value: i as f64, step_size: 1.0 }).collect())
        .label_formatter(|name, value| format!("{name}\n{} mm", format_vn(value.y, 1)))
        .show(ui, |plot| {
            for (si, s) in series.iter().enumerate() {
                let offset = (si as f64 - (n - 1.0) / 2.0) * bar_width;
                let color = series_color(si);
                let bars: Vec<Bar> = s
                    .values
                    .iter()
                    .enumerate()
                    .filter_map(|(di, v)| v.map(|v| Bar::new(di as f64 + offset, v).width(bar_width * 0.92).fill(color).stroke(egui::Stroke::new(1.0, p.card))))
                    .collect();
                plot.bar_chart(BarChart::new(s.label.clone(), bars).color(color));
                // Nhãn giá trị luôn hiện trên đầu cột (không chỉ dựa vào màu).
                for (di, v) in s.values.iter().enumerate() {
                    if let Some(v) = v {
                        let text = RichText::new(format_vn(*v, 0)).font(FontId::proportional(10.0)).color(p.text);
                        plot.text(Text::new("", PlotPoint::new(di as f64 + offset, *v + max_value * 0.05), text));
                    }
                }
            }
            plot.hline(HLine::new("Ngưỡng cảnh báo", threshold_mm).color(p.error).style(egui_plot::LineStyle::dashed_loose()).width(1.5));
        });
}
