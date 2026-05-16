//! Layout calculation and validation.
//! Handles adaptive positioning, safe zones, and config validation.

use crate::core::config::Config;

#[derive(Debug, Clone)]
pub struct Layout {
    pub items: Vec<LayoutItem>,
}

#[derive(Debug, Clone)]
pub struct LayoutItem {
    pub metric_id: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub label_value_spacing: i32,
    pub max_width: i32,
    pub alignment: String,
    pub clip: bool,
}

/// Computes the layout for a specific monitor based on its dimensions and config.
pub fn compute(config: &Config, screen_index: usize, width: u16, _height: u16) -> Layout {
    let screen = config.screens.get(screen_index).unwrap_or(&config.screens[0]);
    let mut items = Vec::new();
    
    let left_margin = screen.x_offset;
    let top_margin = screen.y_offset;
    let safe_top = 180; // Desktop icon/header safety
    let start_y = std::cmp::max(top_margin, safe_top);
    
    // [HARDENING] Sovereign Top-Center Day of Week
    items.push(LayoutItem {
        metric_id: "day_of_week".to_string(),
        label: "".to_string(),
        x: (width as i32 / 2) - 100,
        y: top_margin + 20,
        label_value_spacing: 0,
        max_width: 600,
        alignment: "center".to_string(),
        clip: false,
    });
    
    let line_height = config.general.metric_spacing;
    let columns = std::cmp::max(1, config.general.metric_columns);
    
    let metrics: Vec<_> = screen.metrics.iter()
        .filter(|m| *m != "weather_condition")
        .collect();
    
    if metrics.is_empty() { return Layout { items }; }

    let metrics_per_col = (metrics.len() as f64 / columns as f64).ceil() as usize;
    let total_w = width as i32 - left_margin * 2;
    let col_w = if columns > 1 { total_w / columns as i32 } else { std::cmp::min(400, total_w) };

    let block_align = config.general.metric_alignment.to_lowercase();
    let total_block_w = columns as i32 * col_w;
    
    let block_x = match block_align.as_str() {
        "center" => (width as i32 - total_block_w) / 2,
        "right" => width as i32 - total_block_w - left_margin,
        _ => left_margin,
    };

    for (i, metric_id) in metrics.iter().enumerate() {
        let col = i / metrics_per_col;
        let row = i % metrics_per_col;
        
        let x = block_x + (col as i32 * col_w);
        let y = start_y + (row as i32 * line_height);

        items.push(LayoutItem {
            metric_id: (*metric_id).clone(),
            label: metric_id.replace("_", " ").to_uppercase(),
            x,
            y,
            label_value_spacing: config.general.label_value_spacing,
            max_width: col_w - 40,
            alignment: "left".to_string(),
            clip: false,
        });
    }

    Layout { items }
}