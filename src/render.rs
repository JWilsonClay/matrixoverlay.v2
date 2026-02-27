use std::collections::HashMap;
use std::time::Duration;
use anyhow::Result;
use cairo::{Context as CairoContext, Format, ImageSurface, Operator};
use pangocairo::pango::{FontDescription, Layout as PangoLayout, Weight};
use xcb::x;
use rand::Rng;

use crate::config::Config;
use crate::layout::Layout as ConfigLayout;
use crate::metrics::{MetricData, MetricId, MetricValue};

pub struct RainStream {
    pub x: f64,
    pub y: f64,
    pub speed: f64,
    pub glyphs: Vec<char>,
    pub depth_scale: f64,
}

pub struct RainManager {
    pub streams: Vec<RainStream>,
    pub realism_scale: u32,
}

impl RainManager {
    pub fn new(realism_scale: u32) -> Self {
        let mut manager = Self { streams: Vec::new(), realism_scale };
        manager.reset_streams(1920); // Default width, will adjust on first draw
        manager
    }

    fn reset_streams(&mut self, width: i32) {
        let mut rng = rand::thread_rng();
        let count = (self.realism_scale as f64 * (width as f64 / 100.0)) as usize;
        let count = std::cmp::min(count, 50); // Cap for performance

        self.streams.clear();
        for _ in 0..count {
            self.streams.push(RainStream {
                x: rng.gen_range(0..width) as f64,
                y: rng.gen_range(-height() as f64..0.0),
                speed: rng.gen_range(2.0..10.0),
                glyphs: (0..rng.gen_range(5..15)).map(|_| random_katakana()).collect(),
                depth_scale: rng.gen_range(0.5..1.2),
            });
        }
    }

    pub fn update(&mut self, dt: Duration) {
        let dy = 60.0 * dt.as_secs_f64();
        for stream in &mut self.streams {
            stream.y += stream.speed * dy;
            if stream.y > height() as f64 + 200.0 {
                stream.y = -200.0;
                stream.glyphs = (0..rand::thread_rng().gen_range(5..15)).map(|_| random_katakana()).collect();
            }
            // Occasionally mutation
            if rand::thread_rng().gen_bool(0.05) {
                let idx = rand::thread_rng().gen_range(0..stream.glyphs.len());
                stream.glyphs[idx] = random_katakana();
            }
        }
    }

    pub fn draw(&self, cr: &CairoContext, config: &Config) -> Result<()> {
        let glyph_size = config.general.font_size as f64 * 0.8;
        
        for stream in &self.streams {
            let alpha_base = stream.depth_scale.powf(2.0);
            for (i, &glyph) in stream.glyphs.iter().enumerate() {
                let y = stream.y - (i as f64 * glyph_size * 1.2);
                if y < -20.0 || y > height() as f64 + 20.0 { continue; }
                
                let alpha = if i == 0 { 1.0 } else { alpha_base * (1.0 - (i as f64 / stream.glyphs.len() as f64)) };
                let alpha = alpha.clamp(0.0, 1.0);

                cr.save()?;
                cr.set_source_rgba(0.0, 1.0, 65.0/255.0, alpha * 0.4); // Matrix green dimmed
                if i == 0 {
                    cr.set_source_rgba(0.8, 1.0, 0.9, 1.0); // Lead glyph is brighter
                }

                // Simple drawing without pango for speed? Or pango for Katakana support.
                // Let's use pango to be safe with CJK fonts.
                let layout = pangocairo::functions::create_layout(cr);
                let mut desc = pango::FontDescription::from_string("Monospace");
                desc.set_size((glyph_size * stream.depth_scale * pango::SCALE as f64) as i32);
                layout.set_font_description(Some(&desc));
                layout.set_text(&glyph.to_string());
                
                cr.move_to(stream.x, y);
                pangocairo::functions::show_layout(cr, &layout);
                cr.restore()?;
            }
        }
        Ok(())
    }
}

fn random_katakana() -> char {
    let code = rand::thread_rng().gen_range(0x30A0..0x30FF);
    std::char::from_u32(code).unwrap_or(' ')
}

fn height() -> i32 { 1080 } // Fallback, should use renderer height

/// Handles drawing to an offscreen surface and presenting it to the X11 window.
pub struct Renderer {
    surface: ImageSurface,
    base_font_desc: FontDescription,
    width: i32,
    height: i32,
    color_rgb: (f64, f64, f64),
    
    // Layout & State
    config_layout: ConfigLayout,
    #[allow(dead_code)]
    monitor_index: usize,
    scroll_offsets: HashMap<String, f64>,
    rain_manager: RainManager,
}

impl Renderer {
    pub fn new(
        width: u16, 
        height: u16, 
        monitor_index: usize, 
        layout: ConfigLayout, 
        config: &Config
    ) -> Result<Self> {
        let surface = ImageSurface::create(Format::ARgb32, width as i32, height as i32)
            .map_err(|e| anyhow::anyhow!("Cairo surface creation failed: {}", e))?;

        let font_str = format!("{} {}", "Monospace", config.general.font_size); // Default fallback
        let mut font_desc = FontDescription::from_string(&font_str);
        
        // Enforce Monospace if not set, though config should handle this.
        if font_desc.family().map_or(true, |f| f.is_empty()) {
            font_desc.set_family("Monospace");
        }

        let color_rgb = parse_hex_color(&config.general.color)?;

        let renderer = Self {
            surface,
            base_font_desc: font_desc,
            width: width as i32,
            height: height as i32,
            color_rgb,
            config_layout: layout,
            monitor_index,
            scroll_offsets: HashMap::new(),
            rain_manager: RainManager::new(config.cosmetics.realism_scale),
        };
        
        // Initial clear
        {
            let cr = CairoContext::new(&renderer.surface)?;
            renderer.clear(&cr)?;
        }
        
        Ok(renderer)
    }

    pub fn clear(&self, cr: &CairoContext) -> Result<()> {
        cr.set_operator(Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0); // Opaque Black
        cr.paint()?;
        cr.set_operator(Operator::Over);
        Ok(())
    }

    /// Main draw loop.
    pub fn draw(
        &mut self, 
        conn: &xcb::Connection, 
        window: x::Window, 
        metrics: &MetricData, 
        config: &Config
    ) -> Result<()> {
        let cr = CairoContext::new(&self.surface)?;
        let pango_layout = pangocairo::functions::create_layout(&cr);
        pango_layout.set_font_description(Some(&self.base_font_desc));
        self.clear(&cr)?;

        // Update physics
        self.rain_manager.update(Duration::from_millis(config.general.update_ms));

        // 1. Draw Rain
        if config.cosmetics.rain_mode != "off" {
            self.rain_manager.draw(&cr, config)?;
        }

        // Always render Day of Week first (Header) at top-center
        if let Some(MetricValue::String(dow)) = metrics.values.get(&MetricId::DayOfWeek) {
            self.draw_day_of_week(&cr, &pango_layout, dow, 100.0, &config.general.glow_passes)?;
        }

        // Iterate over layout items and draw them
        let items = self.config_layout.items.clone();
        for item in &items {
            // Resolve metric value
            let metric_id_enum = MetricId::from_str(&item.metric_id);
            
            // Skip day_of_week in list as it is drawn as header
            if item.metric_id == "day_of_week" {
                continue;
            }

            // Standard Metrics
            if let Some(id) = metric_id_enum {
                if let Some(value) = metrics.values.get(&id) {
                    let value_str = self.format_metric_value(value);
                    
                    // 2. Draw Occlusion Box if enabled
                    if config.cosmetics.occlusion_enabled {
                        self.draw_occlusion_box(&cr, item.x as f64 - 5.0, item.y as f64 - 2.0, item.max_width as f64 + 10.0, 24.0)?;
                    }

                    // Use MetricId label for consistency (e.g. "CPU", "RAM %")
                    // For Custom metrics, we might want to use the label from the config if available, 
                    // but here we use the ID or the logic inside MetricId::label().
                    // If it's a custom file, the ID is "server_log", label is "server_log".
                    // To get a pretty name, the user can use the "label" field in the layout config (which is passed as `item.label` here but we overwrite it below).
                    // Actually, let's prefer the layout item's label if it's set, otherwise fallback to ID.
                    let label = if item.label.is_empty() { id.label() } else { item.label.clone() };
                    
                    // Enable scrolling for network or weather which might be long
                    let allow_scroll = item.metric_id == "network_details" || item.metric_id.contains("weather");
                    
                    log::trace!("Drawing metric {:?} at y={}", id, item.y);

                    self.draw_metric_pair(
                        &cr,
                        &pango_layout,
                        &label, 
                        &value_str, 
                        item.x as f64, 
                        item.y as f64, 
                        item.max_width as f64,
                        &item.metric_id,
                        item.clip || allow_scroll,
                        &config.general.glow_passes
                    )?;
                } else {
                    log::warn!("Skipping metric {:?} (No data available)", id);
                }
            }
        }

        // Explicitly drop context and layout to release surface lock
        drop(pango_layout);
        drop(cr);

        // Debug snapshot
        // if log::log_enabled!(log::Level::Trace) {
        //      if let Ok(mut file) = std::fs::File::create(format!("/tmp/matrix_overlay_debug_{}.png", self.monitor_index)) {
        //          let _ = self.surface.write_to_png(&mut file);
        //      }
        // }

        self.present(conn, window)?;
        Ok(())
    }

    fn format_metric_value(&self, value: &MetricValue) -> String {
        match value {
            MetricValue::Float(v) => format!("{:.1}", v),
            MetricValue::Int(v) => format!("{}", v),
            MetricValue::String(s) => s.clone(),
            MetricValue::NetworkMap(map) => {
                let mut parts = Vec::new();
                let mut keys: Vec<_> = map.keys().collect();
                keys.sort(); // Ensure stable order
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
            MetricValue::None => "---".to_string(),
        }
    }

    /// Draws the Day of Week header, centered and scaled.
    fn draw_day_of_week(&mut self, cr: &CairoContext, layout: &PangoLayout, dow: &str, y: f64, glow_passes: &[(f64, f64, f64)]) -> Result<()> {
        log::debug!("Drawing Day of Week: '{}' at y={}", dow, y);
        // Scale font 1.8x
        let mut desc = self.base_font_desc.clone();
        let size = desc.size();
        desc.set_size((size as f64 * 1.8) as i32);
        desc.set_weight(Weight::Bold);
        layout.set_font_description(Some(&desc));
        
        layout.set_text(dow);
        
        // Calculate center position
        let (width, _) = layout.pixel_size();
        let text_width = width as f64; // Pango units are handled by pixel_size helper usually, but here we assume pixels if using cairo-rs helpers correctly or need scaling? 
        // Actually pango_layout.pixel_size() returns pixels.
        
        // Center horizontally in the window
        let x = (self.width as f64 - text_width) / 2.0;
        
        // High-contrast green #00FF41 (R=0, G=255, B=65)
        let matrix_green = (0.0, 1.0, 65.0 / 255.0);
        
        self.draw_text_glow_at(cr, layout, x, y, Some(matrix_green), glow_passes)?;
        
        // Reset font
        layout.set_font_description(Some(&self.base_font_desc));
        Ok(())
    }

    /// Draws a Label: Value pair.
    /// Label is left-aligned at `x`.
    /// Value is right-aligned at `x + max_width`.
    /// If value is too long and `scroll` is true, it scrolls.
    fn draw_metric_pair(
        &mut self, 
        cr: &CairoContext,
        layout: &PangoLayout,
        label: &str, 
        value: &str, 
        x: f64, 
        y: f64, 
        max_width: f64,
        metric_id: &str,
        allow_scroll: bool,
        glow_passes: &[(f64, f64, f64)]
    ) -> Result<()> {
        // 1. Draw Label
        layout.set_text(label);
        self.draw_text_glow_at(cr, layout, x, y, None, glow_passes)?;
        
        let (label_w_px, _) = layout.pixel_size();
        let label_width = label_w_px as f64;

        // 2. Prepare Value
        layout.set_text(value);
        let (val_w_px, _) = layout.pixel_size();
        let value_width = val_w_px as f64;

        // Calculate available space for value
        // We assume a small padding between label and value if they get close
        let padding = 10.0;
        let value_area_start = x + label_width + padding;
        let value_area_width = max_width - label_width - padding;

        if value_area_width <= 0.0 {
            return Ok(()); // No space
        }

        // 3. Calculate Position & Scroll
        let mut draw_x = x + max_width - value_width;
        
        // Clip rectangle for value
        cr.save()?;
        cr.rectangle(value_area_start, y, value_area_width, self.height as f64); // Height is loose here, clip handles it
        cr.clip();

        if value_width > value_area_width && allow_scroll {
            // Scrolling logic
            let offset = self.scroll_offsets.entry(metric_id.to_string()).or_insert(0.0);
            
            // Slow scroll: 0.5px per frame (assuming ~20-60fps call rate from main loop)
            // For ASD friendliness, we avoid rapid flashing. Slow smooth scroll is better.
            *offset += 0.5;
            
            // Reset if scrolled past
            // We scroll the text completely out to the left, then reset to right
            let scroll_span = value_width + value_area_width; 
            if *offset > scroll_span {
                *offset = -value_area_width; // Start entering from right
            }

            // Position: Right aligned base, shifted left by offset
            // Actually, for scrolling, we usually start right-aligned (visible) then scroll left?
            // Or marquee style: start at right edge.
            // Let's do: Start with text right-aligned. If it overflows, we start shifting it left.
            // But standard marquee moves right-to-left.
            
            // Let's define x such that it moves.
            // Start: x = value_area_start + value_area_width (Just entering)
            // End: x = value_area_start - value_width (Just exited)
            
            // We map offset 0..span to position.
            // Let's simplify: Just scroll left continuously.
            // Initial position (offset 0): Right aligned (standard view)
            // Wait, if it's right aligned and overflows, the left part is cut off.
            // We probably want to see the start of the string first?
            // Let's stick to the prompt: "track offset, clamp".
            
            // Implementation: Ping-pong or circular?
            // "track offset, clamp" suggests maybe we scroll to the end and stop?
            // Let's do a simple marquee: Move left.
            
            // Override draw_x for scrolling
            // Start at right edge of area
            draw_x = (x + max_width) - *offset;
            
            // If we have scrolled so far that the text is gone, reset
            if draw_x + value_width < value_area_start {
                 *offset = 0.0; // Reset to start
                 // Optional: Pause at start? Requires more state.
            }
        } else {
            // Ensure right alignment if fitting, or clamped if not scrolling
            if value_width > value_area_width {
                // If too big and no scroll, align left of value area (show start of string)
                draw_x = value_area_start;
            }
        }

        // Draw Value
        // We use a separate draw call because we might have clipped
        // We need to set the layout text again because draw_text_glow uses it
        // But wait, draw_text_glow sets text? No, it uses current layout text?
        // My previous implementation of draw_text_glow took `text` as arg.
        // Let's check `draw_text_glow` signature in previous file.
        // `pub fn draw_text_glow(&mut self, text: &str, x: f64, y: f64, alpha_steps: &[f64])`
        // I should update `draw_text_glow` to use the current layout state or pass text.
        // I'll assume I can call it.
        
        // Note: draw_text_glow in previous prompt took `text`. 
        // Here I will use a helper that assumes layout is set, or pass text.
        // Let's use the one that takes text to be safe.
        self.draw_text_glow_at(cr, layout, draw_x, y, None, glow_passes)?;

        cr.restore()?; // Restore clip

        Ok(())
    }

    /// Helper to draw the current layout content with glow at (x,y).
    /// Assumes `self.pango_layout` already has the correct text/font set.
    fn draw_text_glow_at(&self, cr: &CairoContext, layout: &PangoLayout, x: f64, y: f64, color: Option<(f64, f64, f64)>, glow_passes: &[(f64, f64, f64)]) -> Result<()> {
        let (r, g, b) = color.unwrap_or(self.color_rgb);

        for (ox, oy, alpha) in glow_passes {
            cr.save()?;
            cr.translate(x + ox, y + oy);
            cr.set_source_rgba(r, g, b, *alpha);
            pangocairo::functions::show_layout(cr, layout);
            cr.restore()?;
        }

        // Main Text
        cr.save()?;
        cr.translate(x, y);
        cr.set_source_rgba(r, g, b, 1.0);
        pangocairo::functions::show_layout(cr, layout);
        cr.restore()?;

        Ok(())
    }

    fn draw_occlusion_box(&self, cr: &CairoContext, x: f64, y: f64, w: f64, h: f64) -> Result<()> {
        cr.save()?;
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.7); // Semi-transparent black
        cr.rectangle(x, y, w, h);
        cr.fill()?;
        cr.restore()?;
        Ok(())
    }

    pub fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()> {
        self.surface.flush();
        let data = self.surface.data().map_err(|e| anyhow::anyhow!("Failed to get surface data: {}", e))?;

        let gc: x::Gcontext = conn.generate_id();
        conn.send_request(&x::CreateGc {
            cid: gc,
            drawable: x::Drawable::Window(window),
            value_list: &[],
        });

        conn.send_request(&x::PutImage {
            format: x::ImageFormat::ZPixmap,
            drawable: x::Drawable::Window(window),
            gc,
            width: self.width as u16,
            height: self.height as u16,
            dst_x: 0,
            dst_y: 0,
            left_pad: 0,
            depth: 32,
            data: &data,
        });

        conn.send_request(&x::FreeGc { gc });

        Ok(())
    }
}

fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(anyhow::anyhow!("Invalid hex color length"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16)? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16)? as f64 / 255.0;
    Ok((r, g, b))
}

fn format_bytes(bytes: u64) -> String {
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
