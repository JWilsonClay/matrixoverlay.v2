use std::time::Duration;
use anyhow::Result;
use cairo::Context as CairoContext;
use pangocairo::pango;
use rand::Rng;
use rand::thread_rng;
use crate::core::config::Config;

/// Represents a single falling stream of glyphs in the Matrix rain.
pub struct RainStream {
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub glyphs: Vec<char>,
    pub depth_scale: f64,
}

/// Manages the physics and state of the Matrix rain effect.
pub struct RainManager {
    pub streams: Vec<RainStream>,
    pub realism_scale: u32,
    pub last_realism_scale: u32,
    pub last_width: i32,
    pub last_height: i32,
}

impl RainManager {
    pub fn new(realism_scale: u32) -> Self {
        Self { 
            streams: Vec::new(), 
            realism_scale,
            last_realism_scale: realism_scale,
            last_width: 1920,
            last_height: 1080,
        }
    }

    fn reset_streams(&mut self, width: i32, height: i32) {
        let mut rng = thread_rng();
        let count = (self.realism_scale as f64 * (width as f64 / 100.0)) as usize;
        let count = std::cmp::min(count, 500);

        self.streams.clear();
        for _ in 0..count {
            self.streams.push(RainStream {
                x: rng.gen_range(0.0..width as f64),
                y: rng.gen_range(-(height as f64)..0.0),
                speed: rng.gen_range(2.0..10.0),
                glyphs: (0..rng.gen_range(5..15)).map(|_| random_matrix_char()).collect(),
                depth_scale: rng.gen_range(0.5..1.2),
            });
        }
        self.last_width = width;
        self.last_height = height;
    }

    pub fn update(&mut self, dt: Duration, width: i32, height: i32, config: &Config) {
        if self.streams.is_empty() || width != self.last_width || height != self.last_height || config.cosmetics.realism_scale != self.last_realism_scale {
            self.realism_scale = config.cosmetics.realism_scale;
            self.last_realism_scale = config.cosmetics.realism_scale;
            self.reset_streams(width, height);
        }

        if config.cosmetics.rain_speed == 0.0 {
            for stream in &mut self.streams {
                if thread_rng().gen_bool(0.01) {
                    let idx = thread_rng().gen_range(0..stream.glyphs.len());
                    stream.glyphs[idx] = random_matrix_char();
                }
            }
            return;
        }

        let dy = 60.0 * dt.as_secs_f64() * config.cosmetics.rain_speed;
        for stream in &mut self.streams {
            stream.y += stream.speed * dy;
            if stream.y > height as f64 + 200.0 {
                stream.y = -200.0;
                stream.glyphs = (0..thread_rng().gen_range(5..15)).map(|_| random_matrix_char()).collect();
            }
            if thread_rng().gen_bool(0.05) {
                let idx = thread_rng().gen_range(0..stream.glyphs.len());
                stream.glyphs[idx] = random_matrix_char();
            }
        }
    }

    pub fn draw(&self, cr: &CairoContext, _width: f64, height: f64, frame_count: u64, config: &Config) -> Result<()> {
        let glyph_size = config.general.font_size as f64 * 0.8;
        
        if self.streams.is_empty() {
            log::warn!("RainManager: No streams to draw!");
        }
        
        let layout = pangocairo::functions::create_layout(cr);
        let mut desc = pango::FontDescription::from_string("Monospace");

        for stream in &self.streams {
            let alpha_base = stream.depth_scale.powf(2.0);
            desc.set_size((glyph_size * stream.depth_scale * pango::SCALE as f64) as i32);
            layout.set_font_description(Some(&desc));

            for (i, &glyph) in stream.glyphs.iter().enumerate() {
                let y = stream.y - (i as f64 * glyph_size * 1.2);
                if y < -20.0 || y > height + 20.0 { continue; }
                
                let alpha = if i == 0 { 1.0 } else { alpha_base * (1.0 - (i as f64 / stream.glyphs.len() as f64)) };
                let alpha = alpha.clamp(0.0, 1.0);

                let alpha = if config.cosmetics.rain_speed == 0.0 {
                    let fc = frame_count as f64;
                    let pulse = ( (fc * 0.05).sin() * 0.4 ) + 0.6;
                    alpha * pulse
                } else {
                    alpha
                };

                cr.save()?;
                let (r, g, b) = match config.general.theme.as_str() {
                    "calm" => (0.0, 0.8, 1.0),
                    "alert" => (1.0, 0.2, 0.2),
                    _ => (0.0, 1.0, 65.0/255.0),
                };
                cr.set_source_rgba(r, g, b, alpha * 0.9 * config.cosmetics.matrix_brightness);
                if i == 0 {
                    let (hr, hg, hb) = match config.general.theme.as_str() {
                        "calm" => (0.8, 0.9, 1.0),
                        "alert" => (1.0, 0.8, 0.8),
                        _ => (0.8, 1.0, 0.9),
                    };
                    cr.set_source_rgba(hr, hg, hb, 1.0 * config.cosmetics.matrix_brightness);
                }

                layout.set_text(&glyph.to_string());
                cr.move_to(stream.x, y);
                pangocairo::functions::show_layout(cr, &layout);
                cr.restore()?;
            }
        }
        Ok(())
    }
}

pub fn random_matrix_char() -> char {
    let code = thread_rng().gen_range(0x30A1..=0x30F6);
    std::char::from_u32(code).unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rain_manager_scale_density() {
        let mut config_v1 = Config::default();
        config_v1.cosmetics.realism_scale = 1;
        let mut manager_v1 = RainManager::new(1);
        manager_v1.update(Duration::from_millis(16), 1920, 1080, &config_v1);
        let count_v1 = manager_v1.streams.len();

        let mut config_v10 = Config::default();
        config_v10.cosmetics.realism_scale = 10;
        let mut manager_v10 = RainManager::new(10);
        manager_v10.update(Duration::from_millis(16), 1920, 1080, &config_v10);
        let count_v10 = manager_v10.streams.len();

        assert!(count_v10 > count_v1);
        assert!(count_v10 <= 500);
    }

    #[test]
    fn test_rain_stream_reset() {
        let config = Config::default();
        let mut manager = RainManager::new(5);
        manager.update(Duration::from_millis(16), 1920, 1080, &config);
        manager.streams[0].y = 10000.0;
        manager.update(Duration::from_millis(16), 1920, 1080, &config);
        assert!(manager.streams[0].y < 0.0);
    }

    #[test]
    fn test_rain_pause_mode() {
        let mut config = Config::default();
        config.cosmetics.rain_speed = 0.0;
        let mut manager = RainManager::new(5);
        manager.update(Duration::from_millis(16), 1920, 1080, &config);
        let start_y = manager.streams[0].y;
        manager.update(Duration::from_millis(100), 1920, 1080, &config);
        assert_eq!(manager.streams[0].y, start_y, "Y position should not change when speed is 0");
    }

    #[test]
    fn test_rain_stream_glyph_mutation() {
        let config = Config::default();
        let mut manager = RainManager::new(1);
        manager.update(Duration::from_millis(16), 1920, 1080, &config);
        let initial_glyphs = manager.streams[0].glyphs.clone();
        
        // Run many updates to trigger probabilistic glyph mutation (gen_bool(0.05))
        for _ in 0..200 {
            manager.update(Duration::from_millis(16), 1920, 1080, &config);
        }
        
        let mutated = manager.streams[0].glyphs != initial_glyphs;
        assert!(mutated, "Glyphs should eventually mutate during updates");
    }
}
