// src/core/logging/visual.rs
use serde::{Deserialize, Serialize};
use super::Logger;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ItemState {
    pub id: String, pub item_type: String,
    pub x: f64, pub y: f64, pub width: f64, pub height: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VisualElement {
    pub label: String, pub value: String,
    pub x: f64, pub y: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VisualFrame {
    pub timestamp: String, pub monitor: usize,
    pub elements: Vec<VisualElement>, pub rain_density: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct StateCapture {
    pub timestamp: String, pub monitor: usize, pub items: Vec<ItemState>,
}

impl Logger {
    pub fn log_state(&self, capture: &StateCapture) {
        let json = serde_json::to_string(capture).unwrap_or_default();
        self.write_to_file("state.log", &json);
        self.write_to_file("visual.log", &self.render_ascii_view(capture));
    }

    pub fn log_visual_frame(&self, frame: &VisualFrame) {
        let json = serde_json::to_string(frame).unwrap_or_default();
        self.write_to_file("trace.log", &json);
        let mut line = format!("Monitor {}: ", frame.monitor);
        for el in &frame.elements { line.push_str(&format!("[{}: {}] ", el.label, el.value)); }
        self.write_to_file("manifest.log", &line);
    }

    fn render_ascii_view(&self, capture: &StateCapture) -> String {
        let (w, h) = (80, 24);
        let mut grid = vec![vec![' '; w]; h];
        for x in 0..w { grid[0][x] = '-'; grid[h-1][x] = '-'; }
        for y in 0..h { grid[y][0] = '|'; grid[y][w-1] = '|'; }

        for item in &capture.items {
            let gx = (item.x / 1920.0 * w as f64) as usize;
            let gy = (item.y / 1080.0 * h as f64) as usize;
            if gx < w && gy < h {
                grid[gy][gx] = match item.item_type.as_str() { "rain" => ':', "metric" => 'M', _ => '?' };
            }
        }
        let mut output = format!("Monitor: {}\n", capture.monitor);
        for row in grid { output.push_str(&row.iter().collect::<String>()); output.push('\n'); }
        output
    }
}
