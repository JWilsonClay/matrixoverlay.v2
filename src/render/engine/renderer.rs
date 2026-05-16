// src/render/engine/renderer.rs
use std::collections::HashMap;
use std::cell::RefCell;
use anyhow::Result;
use cairo::{Format, ImageSurface};
use pangocairo::pango::FontDescription;
use crate::core::config::Config;
use crate::core::layout::Layout as ConfigLayout;
use crate::render::physics::RainManager;
use crate::render::layout::parse_hex_color;

pub struct Renderer {
    pub surface: ImageSurface,
    pub base_font: FontDescription,
    pub width: i32, pub height: i32,
    pub color_rgb: (f64, f64, f64),
    pub config_layout: ConfigLayout,
    pub monitor_index: usize,
    pub scroll: RefCell<HashMap<String, f64>>,
    pub rain: RainManager,
    pub frames: RefCell<u64>,
    pub item_states: RefCell<Vec<crate::core::logging::ItemState>>,
    pub visual_elements: RefCell<Vec<crate::core::logging::VisualElement>>,
    pub logger: Option<crate::core::logging::Logger>,
}

impl Renderer {
    pub fn new(w: u16, h: u16, idx: usize, layout: ConfigLayout, config: &Config) -> Result<Self> {
        let s = ImageSurface::create(Format::ARgb32, w as i32, h as i32).map_err(|e| anyhow::anyhow!(e))?;
        let font = FontDescription::from_string(&format!("Monospace {}", config.general.font_size));
        Ok(Self {
            surface: s, base_font: font, width: w as i32, height: h as i32,
            color_rgb: parse_hex_color(&config.general.color).unwrap_or((0.0, 1.0, 0.25)),
            config_layout: layout, monitor_index: idx, scroll: RefCell::new(HashMap::new()),
            rain: RainManager::new(config.cosmetics.realism), frames: RefCell::new(0),
            item_states: RefCell::new(Vec::new()), visual_elements: RefCell::new(Vec::new()),
            logger: if config.logging.enabled { Some(crate::core::logging::Logger::new(&config.logging.log_path, config.logging.max_files, config.logging.max_file_size_mb)) } else { None },
        })
    }
}
