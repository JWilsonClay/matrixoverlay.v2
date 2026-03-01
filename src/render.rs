// src/render.rs
use std::collections::HashMap;
use std::time::Duration;
use std::cell::RefCell;
use anyhow::Result;
use cairo::{Context as CairoContext, Format, ImageSurface, Operator};
use pangocairo::pango::{self, FontDescription, Layout as PangoLayout, Weight};
use xcb::x;
use rand::Rng;
use rand::thread_rng;

use crate::config::Config;
use crate::layout::Layout as ConfigLayout;
use crate::metrics::{MetricData, MetricId, MetricValue};

/// Represents a single falling stream of glyphs in the Matrix rain.
pub struct RainStream {
    /// Horizontal position of the stream.
    pub x: f64,
    /// Vertical position of the lead glyph.
    pub y: f64,
    /// Vertical falling speed.
    pub speed: f64,
    /// List of characters (glyphs) currently in the stream.
    pub glyphs: Vec<char>,
    /// Scaling factor for depth (parallax) effect.
    pub depth_scale: f64,
}

/// Manages the physics and state of the Matrix rain effect.
pub struct RainManager {
    /// Collection of active rain streams.
    pub streams: Vec<RainStream>,
    /// Density of the rain effect (0-10).
    pub realism_scale: u32,
    /// Detected realism change
    pub last_realism_scale: u32,
    /// Last known width of the rendering surface.
    pub last_width: i32,
    /// Last known height of the rendering surface.
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
        let count = std::cmp::min(count, 500); // Increased cap for realism_scale up to 50

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
            // Static effect: No vertical movement, but letters slowly mutation and fade
            for stream in &mut self.streams {
                // Occasional mutation even when static
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
            // Occasionally mutation
            if thread_rng().gen_bool(0.05) {
                let idx = thread_rng().gen_range(0..stream.glyphs.len());
                stream.glyphs[idx] = random_matrix_char();
            }
        }
    }

    pub fn draw(&self, cr: &CairoContext, _width: f64, height: f64, frame_count: u64, config: &Config) -> Result<()> {
        let glyph_size = config.general.font_size as f64 * 0.8;
        
        if self.streams.is_empty() {
            log::warn!("RainManager: No streams to draw! Realism scale might be 0.");
        }
        
        // Create local layout for isolation
        let layout = pangocairo::functions::create_layout(cr);
        let mut desc = pango::FontDescription::from_string("Monospace");

        for stream in &self.streams {
            let alpha_base = stream.depth_scale.powf(2.0);
            
            // Configure font size for this stream
            desc.set_size((glyph_size * stream.depth_scale * pango::SCALE as f64) as i32);
            layout.set_font_description(Some(&desc));

            for (i, &glyph) in stream.glyphs.iter().enumerate() {
                let y = stream.y - (i as f64 * glyph_size * 1.2);
                if y < -20.0 || y > height + 20.0 { continue; }
                
                let alpha = if i == 0 { 1.0 } else { alpha_base * (1.0 - (i as f64 / stream.glyphs.len() as f64)) };
                let alpha = alpha.clamp(0.0, 1.0);

                // Static speed 0.0 specific fade-to-black simulation
                let alpha = if config.cosmetics.rain_speed == 0.0 {
                    // Pulse-fade over 1.5s (simulated by frame count)
                    let fc = frame_count as f64;
                    let pulse = ( (fc * 0.05).sin() * 0.5 ) + 0.5;
                    alpha * pulse
                } else {
                    alpha
                };

                cr.save()?;
                let (r, g, b) = match config.general.theme.as_str() {
                    "calm" => (0.0, 0.8, 1.0),
                    "alert" => (1.0, 0.2, 0.2),
                    _ => (0.0, 1.0, 65.0/255.0), // Classic Matrix Green
                };
                cr.set_source_rgba(r, g, b, alpha * 0.9 * config.cosmetics.matrix_brightness); // Split brightness applied
                if i == 0 {
                    let (hr, hg, hb) = match config.general.theme.as_str() {
                        "calm" => (0.8, 0.9, 1.0),
                        "alert" => (1.0, 0.8, 0.8),
                        _ => (0.8, 1.0, 0.9), // Bright Green lead
                    };
                    cr.set_source_rgba(hr, hg, hb, 1.0 * config.cosmetics.matrix_brightness); // Lead glyph brightness
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

fn random_matrix_char() -> char {
    // Use Katakana (0x30A0 - 0x30FF) for authentic Matrix look
    let code = thread_rng().gen_range(0x30A1..=0x30F6);
    std::char::from_u32(code).unwrap_or('?')
}

/// Handles drawing to an offscreen surface and presenting it to the X11 window.
pub struct Renderer {
    /// The target Cairo image surface.
    pub surface: ImageSurface,
    /// Default font description used for metrics.
    pub base_font_desc: FontDescription,
    /// Width of the renderer's surface.
    pub width: i32,
    /// Height of the renderer's surface.
    pub height: i32,
    /// Base color for rendering (from config).
    pub color_rgb: (f64, f64, f64),
    /// Layout configuration from config.json.
    config_layout: ConfigLayout,
    #[allow(dead_code)]
    monitor_index: usize,
    /// Map of metric IDs to their current scroll offset (for long text).
    scroll_offsets: RefCell<HashMap<String, f64>>,
    /// manager for the background rain effect.
    rain_manager: RainManager,
    /// Monotonically increasing frame counter for animations.
    frame_count: RefCell<u64>,
    /// State of items for logging
    pub item_states: RefCell<Vec<crate::logging::ItemState>>,
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

        let cr = CairoContext::new(&surface)?;
        
        let renderer = Self {
            surface,
            base_font_desc: font_desc,
            width: width as i32,
            height: height as i32,
            color_rgb,
            config_layout: layout,
            monitor_index,
            scroll_offsets: RefCell::new(HashMap::new()),
            rain_manager: RainManager::new(config.cosmetics.realism_scale),
            frame_count: RefCell::new(0),
            item_states: RefCell::new(Vec::new()),
        };
        
        // Initial clear
        renderer.clear(&cr)?;
        
        Ok(renderer)
    }

    pub fn clear(&self, cr: &CairoContext) -> Result<()> {
        cr.set_operator(Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0); // Opaque Black
        cr.paint()?;
        cr.set_operator(Operator::Over);
        Ok(())
    }

    pub fn update_config(&mut self, config: Config) {
        let screen = &config.screens[self.monitor_index];
        self.config_layout = crate::layout::compute(
            screen, 
            self.surface.width() as u16, 
            self.surface.height() as u16, 
            config.general.font_size as f64
        );
        self.rain_manager.realism_scale = config.cosmetics.realism_scale;
        
        // Update color based on theme if it's one of the presets
        self.color_rgb = match config.general.theme.as_str() {
            "calm" => (0.0, 0.8, 1.0),
            "alert" => (1.0, 0.2, 0.2),
            "classic" => (0.0, 1.0, 65.0 / 255.0),
            _ => parse_hex_color(&config.general.color).unwrap_or((0.0, 1.0, 65.0 / 255.0)),
        };
    }

    /// Main draw loop.
    pub fn draw(
        &mut self, 
        conn: &xcb::Connection, 
        window: x::Window, 
        config: &Config, 
        metrics: &MetricData
    ) -> Result<()> {
        // FPS Capping logic
        *self.frame_count.borrow_mut() += 1;
        let frame_count = *self.frame_count.borrow();

        let cr = CairoContext::new(&self.surface)?;
        self.clear(&cr)?;

        // Update physics
        self.rain_manager.update(
            Duration::from_millis(33), // Fixed 30 FPS delta (approx 33ms)
            self.surface.width(),
            self.surface.height(),
            config
        );

        // Clear item states for this frame
        self.item_states.borrow_mut().clear();

        // 1. Draw Rain
        if config.cosmetics.rain_mode == "fall" {
            self.rain_manager.draw(&cr, self.width as f64, self.height as f64, *self.frame_count.borrow(), config)?;
            
            // Log rain positions (sampled for performance)
            if config.logging.enabled {
                let mut states = self.item_states.borrow_mut();
                for (i, stream) in self.rain_manager.streams.iter().enumerate() {
                    if i % 5 == 0 { // Only log every 5th stream to save space
                        states.push(crate::logging::ItemState {
                            id: format!("rain_{}", i),
                            item_type: "rain".to_string(),
                            x: stream.x,
                            y: stream.y,
                            width: 10.0, // approx
                            height: 10.0,
                        });
                    }
                }
            }
        } else if config.cosmetics.rain_mode == "pulse" {
            // Optimization: Pulse Mode (Very low CPU)
            let pulse = ( (frame_count as f64 * 0.05).sin() * 0.2 ) + 0.3;
            let theme_color = match config.general.theme.as_str() {
                "calm" => (0.0, 0.8, 1.0),
                "alert" => (1.0, 0.2, 0.2),
                _ => (0.0, 1.0, 65.0/255.0), // classic
            };
            cr.save()?;
            cr.set_source_rgba(theme_color.0, theme_color.1, theme_color.2, pulse);
            cr.rectangle(0.0, 0.0, self.width as f64, self.height as f64);
            cr.set_operator(Operator::Atop); 
            cr.paint_with_alpha(pulse)?;
            cr.restore()?;
        }

        if let Some(MetricValue::String(dow)) = metrics.values.get(&MetricId::DayOfWeek) {
            let header_text = if config.general.show_monitor_label {
                format!("{} (Monitor {})", dow, self.monitor_index + 1)
            } else {
                dow.to_string()
            };

            // Calculate Box dimensions
            let box_w = 400.0;
            let box_h = config.general.font_size as f64 * 3.0; // Dynamic box height
            let box_x = (self.width as f64 - box_w) / 2.0;
            let box_y = 60.0; // Moved slightly up for better aesthetic

            // Draw occlusion box
            if config.cosmetics.occlusion_enabled {
                self.draw_occlusion_box(&cr, box_x, box_y, box_w, box_h, config)?;
            }
            
            self.draw_day_of_week(&cr, &header_text, box_x, box_y, box_w, box_h, &config.general.glow_passes, config)?;
            
            if config.logging.enabled {
                let (w, h) = (200.0, 40.0 * 1.8); // Appoximate size for Day of Week
                self.item_states.borrow_mut().push(crate::logging::ItemState {
                    id: "day_of_week".to_string(),
                    item_type: "metric".to_string(),
                    x: (self.width as f64 - 200.0) / 2.0, // approx center
                    y: 100.0,
                    width: w,
                    height: h,
                });
            }
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
                    let box_h = config.general.metric_font_size as f64 * 1.5;
                    if config.cosmetics.occlusion_enabled {
                        self.draw_occlusion_box(&cr, item.x as f64 - 5.0, item.y as f64 - 2.0, item.max_width as f64 + 10.0, box_h, config)?;
                    }

                    let label = if item.label.is_empty() { id.label() } else { item.label.clone() };
                    
                    // Enable scrolling for network or weather which might be long
                    let allow_scroll = item.metric_id == "network_details" || item.metric_id.contains("weather");
                    
                    log::trace!("Drawing metric {:?} at y={}", id, item.y);

                    self.draw_metric_pair(
                        &cr,
                        &label, 
                        &value_str, 
                        item.x as f64, 
                        item.y as f64, 
                        item.max_width as f64,
                        &item.metric_id,
                        item.clip || allow_scroll,
                        &config.general.glow_passes,
                        config
                    )?;

                    if config.logging.enabled {
                        self.item_states.borrow_mut().push(crate::logging::ItemState {
                            id: item.metric_id.clone(),
                            item_type: "metric".to_string(),
                            x: item.x as f64,
                            y: item.y as f64,
                            width: item.max_width as f64,
                            height: 24.0,
                        });
                    }
                } else {
                    log::debug!("Skipping metric {:?} (No data available)", id);
                }
            }
        }

        // Explicitly drop context to release surface lock
        drop(cr);

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
    fn draw_day_of_week(&self, cr: &CairoContext, header_text: &str, box_x: f64, box_y: f64, box_w: f64, box_h: f64, glow_passes: &[(f64, f64, f64)], config: &Config) -> Result<()> {
        log::debug!("Drawing Day of Week: '{}' in box at {},{}", header_text, box_x, box_y);
        
        cr.save()?;
        // Removed cr.identity_matrix() to maintain global scaling consistency
        
        let layout = pangocairo::functions::create_layout(cr);
        
        let mut desc = self.base_font_desc.clone();
        let size = desc.size();
        desc.set_size((size as f64 * 1.8) as i32);
        desc.set_weight(Weight::Bold);
        layout.set_font_description(Some(&desc));
        
        layout.set_text(header_text);
        let (_, logical) = layout.pixel_extents();
        let text_width = logical.width as f64; 
        let text_height = logical.height as f64;
        
        // Center horizontally and vertically within the box
        let x = box_x + (box_w - text_width) / 2.0;
        let y = box_y + (box_h - text_height) / 2.0;
        
        // Theme-aware colors
        let theme_color = match config.general.theme.as_str() {
            "calm" => (0.0, 0.8, 1.0),
            "alert" => (1.0, 0.2, 0.2),
            _ => (0.0, 1.0, 65.0 / 255.0), // classic
        };
        
        self.draw_text_glow_at(cr, &layout, x, y, Some(theme_color), glow_passes, config)?;
        
        cr.restore()?;
        Ok(())
    }

    /// Draws a Label: Value pair.
    fn draw_metric_pair(
        &self, 
        cr: &CairoContext,
        label: &str, 
        value: &str, 
        x: f64, 
        y: f64, 
        max_width: f64,
        metric_id: &str,
        allow_scroll: bool,
        glow_passes: &[(f64, f64, f64)],
        config: &Config
    ) -> Result<()> {
        let layout = pangocairo::functions::create_layout(cr);
        let mut desc = pango::FontDescription::from_string("Monospace");
        desc.set_size((config.general.metric_font_size as f64 * pango::SCALE as f64) as i32);
        layout.set_font_description(Some(&desc));

        let box_h = config.general.metric_font_size as f64 * 1.5;
        
        // 1. Draw Label
        layout.set_text(label);
        let (_, label_h_px) = layout.pixel_size();
        let label_h = label_h_px as f64;
        
        // Vertical centering: box_h vs label_h
        let centered_y = y + (box_h - label_h) / 2.0 - 2.0;

        self.draw_text_glow_at(cr, &layout, x, centered_y, None, glow_passes, config)?;
        
        let (label_w_px, _) = layout.pixel_size();
        let label_width = label_w_px as f64;

        // 2. Prepare Value
        layout.set_text(value);
        let (val_w_px, _) = layout.pixel_size();
        let value_width = val_w_px as f64;

        // Calculate available space for value
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
            let mut offsets = self.scroll_offsets.borrow_mut();
            let offset = offsets.entry(metric_id.to_string()).or_insert(0.0);
            
            // Slow scroll: 0.5px per frame
            *offset += 0.5;
            
            // Reset if scrolled past
            let scroll_span = value_width + value_area_width; 
            if *offset > scroll_span {
                *offset = -value_area_width; // Start entering from right
            }

            // Override draw_x for scrolling
            draw_x = (x + max_width) - *offset;
            
            // If we have scrolled so far that the text is gone, reset
            if draw_x + value_width < value_area_start {
                 *offset = 0.0; // Reset to start
            }
        } else {
            // Ensure right alignment if fitting, or clamped if not scrolling
            if value_width > value_area_width {
                // If too big and no scroll, align left of value area (show start of string)
                draw_x = value_area_start;
            }
        }

        // Draw Value
        self.draw_text_glow_at(cr, &layout, draw_x, centered_y, None, glow_passes, config)?;

        cr.restore()?; // Restore clip

        Ok(())
    }

    fn draw_text_glow_at(&self, cr: &CairoContext, layout: &PangoLayout, x: f64, y: f64, color: Option<(f64, f64, f64)>, glow_passes: &[(f64, f64, f64)], config: &Config) -> Result<()> {
        let (r, g, b) = color.unwrap_or(self.color_rgb);
        let global_brightness = config.cosmetics.metrics_brightness;

        for (ox, oy, alpha) in glow_passes {
            cr.save()?;
            cr.translate(x + ox, y + oy);
            cr.move_to(0.0, 0.0); // CRITICAL FIX: Reset current point for Cairo/Pango
            cr.set_source_rgba(r, g, b, *alpha * global_brightness);
            pangocairo::functions::show_layout(cr, layout);
            cr.restore()?;
        }

        // Main Text
        cr.save()?;
        cr.translate(x, y);
        cr.move_to(0.0, 0.0); // CRITICAL FIX: Reset current point for Cairo/Pango
        cr.set_source_rgba(r, g, b, 1.0 * global_brightness);
        pangocairo::functions::show_layout(cr, layout);
        cr.restore()?;

        Ok(())
    }

    fn draw_occlusion_box(&self, cr: &CairoContext, x: f64, y: f64, w: f64, h: f64, config: &Config) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_rain_manager_scale_density() {
        let mut manager_v1 = RainManager::new(1);
        manager_v1.update(Duration::from_millis(16), 1920, 1080);
        let count_v1 = manager_v1.streams.len();

        let mut manager_v10 = RainManager::new(10);
        manager_v10.update(Duration::from_millis(16), 1920, 1080);
        let count_v10 = manager_v10.streams.len();

        assert!(count_v10 > count_v1, "Scale 10 should have more streams than Scale 1: {} vs {}", count_v10, count_v1);
        assert!(count_v10 <= 50, "Density should be capped at 50 for performance");
    }

    #[test]
    fn test_rain_stream_reset() {
        let mut manager = RainManager::new(5);
        manager.update(Duration::from_millis(16), 1920, 1080);
        // Move stream far off bottom
        manager.streams[0].y = 10000.0;
        manager.update(Duration::from_millis(16), 1920, 1080);
        assert!(manager.streams[0].y < 0.0, "Stream should have reset to top after falling below height");
    }
}
