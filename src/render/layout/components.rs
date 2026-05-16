// src/render/layout/components.rs
use std::collections::HashMap;
use anyhow::Result;
use cairo::Context as CairoContext;
use pangocairo::pango::{self, Weight};
use crate::core::config::Config;
use crate::core::layout::LayoutItem;
use crate::core::logging::VisualElement;
use super::drawing::draw_text_glow_at;
use super::drawing::draw_occlusion_box;

pub fn draw_day_of_week(cr: &CairoContext, text: &str, x: f64, y: f64, w: f64, h: f64, passes: &[(f64, f64, f64)], config: &Config, color: (f64, f64, f64)) -> Result<Option<VisualElement>> {
    cr.save()?;
    let layout = pangocairo::functions::create_layout(cr);
    let mut desc = pango::FontDescription::from_string("Monospace");
    desc.set_size((config.general.font_size as f64 * 1.8 * pango::SCALE as f64) as i32);
    desc.set_weight(Weight::Bold);
    layout.set_font_description(Some(&desc));
    layout.set_text(text);
    let (_, logical) = layout.pixel_extents();
    let tx = x + (w - logical.width() as f64) / 2.0 - logical.x() as f64;
    let ty = y + (h - logical.height() as f64) / 2.0 - logical.y() as f64;
    let theme_color = match config.general.theme.as_str() { "calm" => (0.0, 0.8, 1.0), "alert" => (1.0, 0.2, 0.2), _ => color };
    draw_text_glow_at(cr, &layout, tx, ty, theme_color, passes, config)?;
    cr.restore()?;
    Ok(Some(VisualElement { label: "DayOfWeek".to_string(), value: text.to_string(), x, y }))
}

#[allow(clippy::too_many_arguments)]
pub fn draw_metric_pair(cr: &CairoContext, label: &str, value: &str, x: f64, y: f64, max_w: f64, id: &str, allow_scroll: bool, passes: &[(f64, f64, f64)], config: &Config, item: &LayoutItem, color: (f64, f64, f64), scroll: &mut HashMap<String, f64>) -> Result<Option<VisualElement>> {
    let layout = pangocairo::functions::create_layout(cr);
    let mut desc = pango::FontDescription::from_string("Monospace");
    desc.set_size((config.general.metric_font_size as f64 * pango::SCALE as f64) as i32);
    layout.set_font_description(Some(&desc));
    let box_h = config.general.metric_font_size as f64 * 1.5;
    layout.set_text(label);
    let (lw, lh) = layout.pixel_size();
    layout.set_text(value);
    let (vw, _) = layout.pixel_size();
    let (label_w, label_h, val_w) = (lw as f64, lh as f64, vw as f64);
    let sf = (item.label_value_spacing as f64).clamp(0.0, 200.0) / 200.0;
    let min_v_x = x + label_w + 10.0;
    let max_v_x = x + max_w - val_w;
    let mut draw_x = min_v_x + (f64::max(min_v_x, max_v_x) - min_v_x) * sf;
    if config.cosmetics.occlusion_enabled { draw_occlusion_box(cr, x - 5.0, y - 2.0, (draw_x + val_w) - x + 10.0, box_h, config)?; }
    layout.set_text(label);
    let cy = y + (box_h - label_h) / 2.0 - 2.0;
    draw_text_glow_at(cr, &layout, x, cy, color, passes, config)?;
    layout.set_text(value);
    let v_a_w = max_w - label_w - 5.0;
    cr.save()?;
    cr.rectangle(min_v_x - 5.0, y, (draw_x + val_w) - (min_v_x - 5.0) + 10.0, 1000.0); cr.clip();
    if val_w > v_a_w && allow_scroll {
        let off = scroll.entry(id.to_string()).or_insert(0.0);
        *off += 0.5;
        if *off > (val_w + v_a_w) { *off = -v_a_w; }
        draw_x = (x + max_w) - *off;
        if draw_x + val_w < (min_v_x - 5.0) { *off = 0.0; }
    }
    draw_text_glow_at(cr, &layout, draw_x, cy, color, passes, config)?;
    cr.restore()?;
    Ok(Some(VisualElement { label: label.to_string(), value: value.to_string(), x, y }))
}
