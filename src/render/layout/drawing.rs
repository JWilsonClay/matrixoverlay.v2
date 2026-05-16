// src/render/layout/drawing.rs
use anyhow::Result;
use cairo::Context as CairoContext;
use pangocairo::pango::Layout as PangoLayout;
use crate::core::config::Config;
use super::formatting::parse_hex_color;

pub fn draw_occlusion_box(cr: &CairoContext, x: f64, y: f64, w: f64, h: f64, config: &Config) -> Result<()> {
    cr.save()?;
    cr.set_source_rgba(0.0, 0.0, 0.0, config.cosmetics.background_opacity); 
    cr.rectangle(x, y, w, h);
    cr.fill()?;
    if config.cosmetics.border_enabled {
        let border = parse_hex_color(&config.cosmetics.border_color).unwrap_or((0.0, 1.0, 65.0/255.0));
        cr.set_source_rgb(border.0, border.1, border.2);
        cr.set_line_width(1.0);
        cr.rectangle(x, y, w, h);
        cr.stroke()?;
    }
    cr.restore()?;
    Ok(())
}

pub fn draw_text_glow_at(cr: &CairoContext, layout: &PangoLayout, x: f64, y: f64, color: (f64, f64, f64), passes: &[(f64, f64, f64)], config: &Config) -> Result<()> {
    let (r, g, b) = color;
    let brightness = config.cosmetics.metrics_brightness;
    for (ox, oy, alpha) in passes {
        cr.save()?;
        cr.translate(x + ox, y + oy);
        cr.move_to(0.0, 0.0);
        cr.set_source_rgba(r, g, b, *alpha * brightness);
        pangocairo::functions::show_layout(cr, layout);
        cr.restore()?;
    }
    cr.save()?;
    cr.translate(x, y); cr.move_to(0.0, 0.0);
    cr.set_source_rgba(r, g, b, 1.0 * brightness);
    pangocairo::functions::show_layout(cr, layout);
    cr.restore()?;
    Ok(())
}
