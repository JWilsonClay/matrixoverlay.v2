//! Layout calculation and validation.
//! Handles adaptive positioning, safe zones, and config validation.

use crate::config::{Config, Screen};
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
pub fn compute(screen: &Screen, width: u16, _height: u16, global_font_size: f64) -> Layout {
    let mut items = Vec::new();
    
    // Use screen offsets from config
    let left = screen.x_offset;
    let top = screen.y_offset;
    
    // Icon Avoidance: Fixed top safe zone of 180px for desktop icons and header
    let safe_top = 180;
    let start_y = std::cmp::max(top, safe_top);
    
    let mut cursor_y = start_y;
    // Approximate line height: font size + padding
    let line_height = (global_font_size * 1.5) as i32; 

    for metric_id in &screen.metrics {
        // Simple vertical list layout
        let x = left;
        let y = cursor_y;
        cursor_y += line_height;

        // Calculate max width for clipping (simple bounds check against screen edges)
        let max_width = (width as i32) - left * 2;

        items.push(LayoutItem {
            metric_id: metric_id.clone(),
            label: metric_id.replace("_", " ").to_uppercase(),
            x,
            y,
            max_width,
            alignment: "left".to_string(),
            clip: false,
        });
    }

    Layout { items }
}