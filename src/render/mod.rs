// src/render/mod.rs
pub mod physics;
pub mod layout;

use std::collections::HashMap;
use std::time::Duration;
use chrono::Local;
use std::cell::RefCell;
use anyhow::Result;
use cairo::{Context as CairoContext, Format, ImageSurface, Operator};
use pangocairo::pango::FontDescription;
use xcb::x;

use crate::core::config::Config;
use crate::core::layout::Layout as ConfigLayout;
use crate::metrics::{MetricData, MetricId, MetricValue};
use self::physics::RainManager;

/// Handles drawing to an offscreen surface and presenting it to the X11 window.
pub struct Renderer {
    pub surface: ImageSurface,
    pub base_font_desc: FontDescription,
    pub width: i32,
    pub height: i32,
    pub color_rgb: (f64, f64, f64),
    config_layout: ConfigLayout,
    #[allow(dead_code)]
    monitor_index: usize,
    scroll_offsets: RefCell<HashMap<String, f64>>,
    rain_manager: RainManager,
    frame_count: RefCell<u64>,
    pub item_states: RefCell<Vec<crate::core::logging::ItemState>>,
    pub logger: Option<crate::core::logging::Logger>,
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

        let font_str = format!("{} {}", "Monospace", config.general.font_size);
        let mut font_desc = FontDescription::from_string(&font_str);
        
        if font_desc.family().map_or(true, |f| f.is_empty()) {
            font_desc.set_family("Monospace");
        }

        let color_rgb = layout::parse_hex_color(&config.general.color)?;

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
            logger: if config.logging.enabled {
                Some(crate::core::logging::Logger::new(&config.logging.log_path, config.logging.max_files, config.logging.max_file_size_mb))
            } else {
                None
            },
        };
        
        renderer.clear(&cr)?;
        Ok(renderer)
    }

    pub fn clear(&self, cr: &CairoContext) -> Result<()> {
        cr.set_operator(Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.paint()?;
        cr.set_operator(Operator::Over);
        Ok(())
    }

    pub fn update_config(&mut self, config: Config) {
        self.config_layout = crate::core::layout::compute(
            &config, 
            self.monitor_index,
            self.surface.width() as u16, 
            self.surface.height() as u16, 
        );
        self.rain_manager.realism_scale = config.cosmetics.realism_scale;
        
        self.color_rgb = match config.general.theme.as_str() {
            "calm" => (0.0, 0.8, 1.0),
            "alert" => (1.0, 0.2, 0.2),
            "classic" => (0.0, 1.0, 65.0 / 255.0),
            _ => layout::parse_hex_color(&config.general.color).unwrap_or((0.0, 1.0, 65.0 / 255.0)),
        };

        if config.logging.enabled {
            if self.logger.is_none() {
                self.logger = Some(crate::core::logging::Logger::new(&config.logging.log_path, config.logging.max_files, config.logging.max_file_size_mb));
            }
        } else {
            self.logger = None;
        }
    }

    pub fn draw(
        &mut self, 
        conn: &xcb::Connection, 
        window: x::Window, 
        config: &Config, 
        metrics: &MetricData,
        dt: Duration
    ) -> Result<()> {
        *self.frame_count.borrow_mut() += 1;
        let frame_count = *self.frame_count.borrow();

        let cr = CairoContext::new(&self.surface)?;
        self.clear(&cr)?;

        self.rain_manager.update(
            dt, 
            self.surface.width(),
            self.surface.height(),
            config
        );

        self.item_states.borrow_mut().clear();

        if config.cosmetics.rain_mode == "fall" {
            self.rain_manager.draw(&cr, self.width as f64, self.height as f64, frame_count, config)?;
            
            if config.logging.enabled {
                let mut states = self.item_states.borrow_mut();
                for (i, stream) in self.rain_manager.streams.iter().enumerate() {
                    if i % 5 == 0 {
                        states.push(crate::core::logging::ItemState {
                            id: format!("rain_{}", i),
                            item_type: "rain".to_string(),
                            x: stream.x,
                            y: stream.y,
                            width: 10.0,
                            height: 10.0,
                        });
                    }
                }
            }
        } else if config.cosmetics.rain_mode == "pulse" {
            let pulse = ( (frame_count as f64 * 0.05).sin() * 0.2 ) + 0.3;
            let theme_color = match config.general.theme.as_str() {
                "calm" => (0.0, 0.8, 1.0),
                "alert" => (1.0, 0.2, 0.2),
                _ => (0.0, 1.0, 65.0/255.0),
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

            let box_w = 400.0;
            let box_h = config.general.font_size as f64 * 3.0;
            let box_x = (self.width as f64 - box_w) / 2.0;
            let box_y = 60.0;

            if config.cosmetics.occlusion_enabled {
                layout::draw_occlusion_box(&cr, box_x, box_y, box_w, box_h, config)?;
            }
            
            layout::draw_day_of_week(&cr, &header_text, box_x, box_y, box_w, box_h, &config.general.glow_passes, config, self.color_rgb)?;
            
            if config.logging.enabled {
                let (w, h) = (200.0, 40.0 * 1.8);
                self.item_states.borrow_mut().push(crate::core::logging::ItemState {
                    id: "day_of_week".to_string(),
                    item_type: "metric".to_string(),
                    x: (self.width as f64 - 200.0) / 2.0,
                    y: 100.0,
                    width: w,
                    height: h,
                });
            }
        }

        let items = self.config_layout.items.clone();
        for item in &items {
            let metric_id_enum = MetricId::from_str(&item.metric_id);
            if item.metric_id == "day_of_week" {
                continue;
            }

            if let Some(id) = metric_id_enum {
                if let Some(value) = metrics.values.get(&id) {
                    let value_str = layout::format_metric_value(value);
                    let label = if item.label.is_empty() { id.label() } else { item.label.clone() };
                    let allow_scroll = item.metric_id == "network_details" || item.metric_id.contains("weather");
                    
                    layout::draw_metric_pair(
                        &cr,
                        &label, 
                        &value_str, 
                        item.x as f64, 
                        item.y as f64, 
                        item.max_width as f64,
                        &item.metric_id,
                        item.clip || allow_scroll,
                        &config.general.glow_passes,
                        config,
                        item,
                        self.color_rgb,
                        &mut self.scroll_offsets.borrow_mut()
                    )?;

                    if config.logging.enabled {
                        self.item_states.borrow_mut().push(crate::core::logging::ItemState {
                            id: item.metric_id.clone(),
                            item_type: "metric".to_string(),
                            x: item.x as f64,
                            y: item.y as f64,
                            width: item.max_width as f64,
                            height: 24.0,
                        });
                    }
                }
            }
        }

        if let Some(ref logger) = self.logger {
            let capture = crate::core::logging::StateCapture {
                timestamp: Local::now().to_rfc3339(),
                monitor: self.monitor_index,
                items: self.item_states.borrow().clone(),
            };
            logger.log_state(&capture);
        }

        drop(cr);
        self.present(conn, window)?;
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
