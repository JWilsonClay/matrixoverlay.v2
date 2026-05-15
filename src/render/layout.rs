use std::collections::HashMap;
use anyhow::Result;
use cairo::Context as CairoContext;
use pangocairo::pango::{self, Layout as PangoLayout, Weight};
use crate::core::config::Config;
use crate::core::layout::LayoutItem;
use crate::metrics::MetricValue;

pub fn format_metric_value(value: &MetricValue) -> String {
    match value {
        MetricValue::Float(v) => format!("{:.1}", v),
        MetricValue::Int(v) => format!("{}", v),
        MetricValue::String(s) => s.clone(),
        MetricValue::NetworkMap(map) => {
            let mut parts = Vec::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for k in keys {
                if let Some((rx, tx)) = map.get(k) {
                    if *rx > 0 || *tx > 0 {
                        parts.push(format!("{}: ↓{} ↑{}", k, format_bytes(*rx), format_bytes(*tx)));
                    }
                }
            }
            if parts.is_empty() {
                "Idle".to_string()
            } else {
                parts.join(" | ")
            }
        },
        MetricValue::Location(lat, lon) => format!("({:.2}, {:.2})", lat, lon),
        MetricValue::None => "---".to_string(),
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        format!("{:.1}GB/s", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB/s", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1}KB/s", bytes as f64 / KB as f64)
    } else {
        format!("{}B/s", bytes)
    }
}

pub fn draw_occlusion_box(cr: &CairoContext, x: f64, y: f64, w: f64, h: f64, config: &Config) -> Result<()> {
    cr.save()?;
    cr.set_source_rgba(0.0, 0.0, 0.0, config.cosmetics.background_opacity); 
    cr.rectangle(x, y, w, h);
    cr.fill()?;

    if config.cosmetics.border_enabled {
        let border_color = parse_hex_color(&config.cosmetics.border_color).unwrap_or((0.0, 1.0, 65.0/255.0));
        cr.set_source_rgb(border_color.0, border_color.1, border_color.2);
        cr.set_line_width(1.0);
        cr.rectangle(x, y, w, h);
        cr.stroke()?;
    }

    cr.restore()?;
    Ok(())
}

pub fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(anyhow::anyhow!("Invalid hex color length"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16)? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16)? as f64 / 255.0;
    Ok((r, g, b))
}

pub fn draw_text_glow_at(
    cr: &CairoContext, 
    layout: &PangoLayout, 
    x: f64, 
    y: f64, 
    color: (f64, f64, f64), 
    glow_passes: &[(f64, f64, f64)], 
    config: &Config
) -> Result<()> {
    let (r, g, b) = color;
    let global_brightness = config.cosmetics.metrics_brightness;

    for (ox, oy, alpha) in glow_passes {
        cr.save()?;
        cr.translate(x + ox, y + oy);
        cr.move_to(0.0, 0.0);
        cr.set_source_rgba(r, g, b, *alpha * global_brightness);
        pangocairo::functions::show_layout(cr, layout);
        cr.restore()?;
    }

    cr.save()?;
    cr.translate(x, y);
    cr.move_to(0.0, 0.0);
    cr.set_source_rgba(r, g, b, 1.0 * global_brightness);
    pangocairo::functions::show_layout(cr, layout);
    cr.restore()?;

    Ok(())
}

pub fn draw_day_of_week(
    cr: &CairoContext, 
    header_text: &str, 
    box_x: f64, 
    box_y: f64, 
    box_w: f64, 
    box_h: f64, 
    glow_passes: &[(f64, f64, f64)], 
    config: &Config,
    base_color: (f64, f64, f64)
) -> Result<Option<crate::core::logging::VisualElement>> {
    cr.save()?;
    let layout = pangocairo::functions::create_layout(cr);
    
    let mut desc = pango::FontDescription::from_string("Monospace");
    desc.set_size((config.general.font_size as f64 * 1.8 * pango::SCALE as f64) as i32);
    desc.set_weight(Weight::Bold);
    layout.set_font_description(Some(&desc));
    
    layout.set_text(header_text);
    let (_, logical) = layout.pixel_extents();
    let text_width = logical.width() as f64; 
    let text_height = logical.height() as f64;
    
    let x = box_x + (box_w - text_width) / 2.0 - logical.x() as f64;
    let y = box_y + (box_h - text_height) / 2.0 - logical.y() as f64;
    
    let theme_color = match config.general.theme.as_str() {
        "calm" => (0.0, 0.8, 1.0),
        "alert" => (1.0, 0.2, 0.2),
        _ => base_color,
    };
    
    draw_text_glow_at(cr, &layout, x, y, theme_color, glow_passes, config)?;
    
    cr.restore()?;
    Ok(Some(crate::core::logging::VisualElement { label: "DayOfWeek".to_string(), value: header_text.to_string(), x: box_x, y: box_y }))
}

#[allow(clippy::too_many_arguments)]
pub fn draw_metric_pair(
    cr: &CairoContext,
    label: &str, 
    value: &str, 
    x: f64, 
    y: f64, 
    max_width: f64,
    metric_id: &str,
    allow_scroll: bool,
    glow_passes: &[(f64, f64, f64)],
    config: &Config,
    item: &LayoutItem,
    base_color: (f64, f64, f64),
    scroll_offsets: &mut HashMap<String, f64>
) -> Result<Option<crate::core::logging::VisualElement>> {
    let layout = pangocairo::functions::create_layout(cr);
    let mut desc = pango::FontDescription::from_string("Monospace");
    desc.set_size((config.general.metric_font_size as f64 * pango::SCALE as f64) as i32);
    layout.set_font_description(Some(&desc));

    let box_h = config.general.metric_font_size as f64 * 1.5;
    
    layout.set_text(label);
    let (label_w_px, label_h_px) = layout.pixel_size();
    let label_width = label_w_px as f64;
    let label_h = label_h_px as f64;
    
    layout.set_text(value);
    let (val_w_px, _) = layout.pixel_size();
    let value_width = val_w_px as f64;

    let spacing_factor = (item.label_value_spacing as f64).clamp(0.0, 200.0) / 200.0;
    let min_value_x = x + label_width + 10.0;
    let max_value_x = x + max_width - value_width;
    let target_max_x = f64::max(min_value_x, max_value_x);
    let mut draw_x = min_value_x + (target_max_x - min_value_x) * spacing_factor;

    let box_width = (draw_x + value_width) - x;

    if config.cosmetics.occlusion_enabled {
        draw_occlusion_box(cr, x - 5.0, y - 2.0, box_width + 10.0, box_h, config)?;
    }

    layout.set_text(label);
    let centered_y = y + (box_h - label_h) / 2.0 - 2.0;
    draw_text_glow_at(cr, &layout, x, centered_y, base_color, glow_passes, config)?;
    
    layout.set_text(value);
    let value_area_start = min_value_x - 5.0; 
    let value_area_width = max_width - label_width - 5.0;

    cr.save()?;
    // We need surface dimensions for proper clipping, but for now we clip to a large area
    cr.rectangle(value_area_start, y, (draw_x + value_width) - value_area_start + 10.0, 10000.0); 
    cr.clip();

    if value_width > value_area_width && allow_scroll {
        let offset = scroll_offsets.entry(metric_id.to_string()).or_insert(0.0);
        *offset += 0.5;
        let scroll_span = value_width + value_area_width; 
        if *offset > scroll_span {
            *offset = -value_area_width;
        }
        draw_x = (x + max_width) - *offset;
        if draw_x + value_width < value_area_start {
             *offset = 0.0;
        }
    }

    draw_text_glow_at(cr, &layout, draw_x, centered_y, base_color, glow_passes, config)?;

    cr.restore()?;
    Ok(Some(crate::core::logging::VisualElement { label: label.to_string(), value: value.to_string(), x, y }))
}
