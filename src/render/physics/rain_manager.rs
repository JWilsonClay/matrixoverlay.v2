// src/render/physics/rain_manager.rs
use std::time::Duration;
use anyhow::Result;
use cairo::Context as CairoContext;
use pangocairo::pango;
use rand::{Rng, thread_rng};
use crate::core::config::Config;
use super::rain_stream::{RainStream, random_char};

pub struct RainManager {
    pub streams: Vec<RainStream>, pub realism: u32,
    pub last_w: i32, pub last_h: i32,
}

impl RainManager {
    pub fn new(realism: u32) -> Self { Self { streams: Vec::new(), realism, last_w: 0, last_h: 0 } }

    pub fn update(&mut self, dt: Duration, w: i32, h: i32, config: &Config) {
        if self.streams.is_empty() || w != self.last_w || h != self.last_h { self.reset(w, h); }
        let dy = 60.0 * dt.as_secs_f64() * config.cosmetics.rain_speed;
        if dy.is_nan() || dy.is_infinite() { return; }
        for s in &mut self.streams {
            s.y += s.speed * dy;
            if s.y > h as f64 + 200.0 { s.y = -200.0; }
            if thread_rng().gen_bool(0.05) { let i = thread_rng().gen_range(0..s.glyphs.len()); s.glyphs[i] = random_char(); }
        }
        self.last_w = w; self.last_h = h;
    }

    pub fn draw(&self, cr: &CairoContext, _w: f64, h: f64, fc: u64, config: &Config) -> Result<()> {
        let size = config.general.font_size as f64 * 0.8;
        let layout = pangocairo::functions::create_layout(cr);
        let mut desc = pango::FontDescription::from_string("Monospace");
        for s in &self.streams {
            desc.set_size((size * s.depth * pango::SCALE as f64) as i32);
            layout.set_font_description(Some(&desc));
            for (i, &g) in s.glyphs.iter().enumerate() {
                let y = s.y - (i as f64 * size * 1.2);
                if y < -20.0 || y > h + 20.0 { continue; }
                let a = if i == 0 { 1.0 } else { (s.depth * s.depth * (1.0 - (i as f64 / s.glyphs.len() as f64))).clamp(0.0, 1.0) };
                cr.save()?;
                cr.set_source_rgba(0.0, 1.0, 0.25, a * config.cosmetics.matrix_brightness);
                layout.set_text(&g.to_string()); cr.move_to(s.x, y);
                pangocairo::functions::show_layout(cr, &layout);
                cr.restore()?;
            }
        }
        Ok(())
    }

    fn reset(&mut self, w: i32, h: i32) {
        let mut rng = thread_rng();
        let count = ((self.realism as f64 * (w as f64 / 100.0)) as usize).min(500);
        self.streams.clear();
        for _ in 0..count { self.streams.push(RainStream::new(rng.gen_range(0.0..w as f64), rng.gen_range(-(h as f64)..0.0), rng.gen_range(2.0..10.0), rng.gen_range(0.5..1.2))); }
    }
}
