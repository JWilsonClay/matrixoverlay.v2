//! Layout calculation and validation.
//! Handles adaptive positioning, safe zones, and config validation.

use crate::config::Config;
use anyhow::Result;
use std::collections::HashSet;

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

/// Validates the configuration for logical consistency and uniqueness.
pub fn validate_config(config: &Config) -> Result<()> {
    // Uniqueness Check: Ensure monitors aren't displaying identical content
    // We use a Jaccard similarity threshold.
    let mut metric_sets: Vec<HashSet<String>> = Vec::new();
    for screen in &config.screens {
        let mut set = HashSet::new();
        for m in &screen.metrics {
            set.insert(m.clone());
        }
        metric_sets.push(set);
    }

    for i in 0..metric_sets.len() {
        for j in (i + 1)..metric_sets.len() {
            let set_a = &metric_sets[i];
            let set_b = &metric_sets[j];
            
            let intersection = set_a.intersection(set_b).count();
            let union = set_a.union(set_b).count();
            
            if union > 0 {
                let similarity = intersection as f64 / union as f64;
                let uniqueness = 1.0 - similarity;
                // Requirement: 75-85% uniqueness enforcement.
                // We warn if uniqueness is below 75%.
                if uniqueness < 0.75 {
                    log::warn!("Monitors {} and {} have low content uniqueness ({:.1}%). Recommended > 75%.", 
                        i, j, uniqueness * 100.0);
                }
            }
        }
    }
    Ok(())
}

/// Computes the layout for a specific monitor based on its dimensions and config.
pub fn compute(config: &Config, screen_index: usize, width: u16, _height: u16) -> Layout {
    let screen = match config.screens.get(screen_index) {
        Some(s) => s,
        None => &config.screens[0],
    };
    
    let mut items = Vec::new();
    
    // Use screen offsets from config
    let left_margin = screen.x_offset;
    let top_margin = screen.y_offset;
    
    // Icon Avoidance: Fixed top safe zone of 180px for desktop icons and header
    let safe_top = 180;
    let start_y = std::cmp::max(top_margin, safe_top);
    
    let line_height = config.general.metric_spacing;
    let columns = std::cmp::max(1, config.general.metric_columns);
    
    // Filter out weather_condition as it's typically paired with weather_temp
    let metrics: Vec<_> = screen.metrics.iter()
        .filter(|m| *m != "weather_condition")
        .collect();
    
    if metrics.is_empty() {
        return Layout { items };
    }

    // Calculate metrics per column
    let metrics_per_column = (metrics.len() as f64 / columns as f64).ceil() as usize;
    
    // Calculate column width
    let total_available_width = width as i32 - left_margin * 2;
    let col_width = if columns > 1 {
        total_available_width / columns as i32
    } else {
        std::cmp::min(400, total_available_width) // Classic width for single column
    };

    // Block alignment offset
    let block_alignment = config.general.metric_alignment.to_lowercase();
    let total_block_width = columns as i32 * col_width;
    
    let block_x_offset = match block_alignment.as_str() {
        "center" => (width as i32 - total_block_width) / 2,
        "right" => width as i32 - total_block_width - left_margin,
        _ => left_margin, // left
    };

    for (i, metric_id) in metrics.iter().enumerate() {
        let col = i / metrics_per_column;
        let row = i % metrics_per_column;
        
        let x = block_x_offset + (col as i32 * col_width);
        let y = start_y + (row as i32 * line_height);

        items.push(LayoutItem {
            metric_id: (*metric_id).clone(),
            label: metric_id.replace("_", " ").to_uppercase(),
            x,
            y,
            label_value_spacing: config.general.label_value_spacing,
            max_width: col_width - 40, // Padding for safety
            alignment: "left".to_string(),
            clip: false,
        });
    }

    Layout { items }
}