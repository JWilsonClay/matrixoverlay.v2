// src/render/layout/formatting.rs
use crate::metrics::MetricValue;
use anyhow::Result;

pub fn format_metric_value(value: &MetricValue) -> String {
    match value {
        MetricValue::Float(v) => if v.is_nan() || v.is_infinite() { "ERR".to_string() } else { format!("{:.1}", v) },
        MetricValue::Int(v) => format!("{}", v),
        MetricValue::String(s) => s.clone(),
        MetricValue::NetworkMap(map) => {
            let mut parts = Vec::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for k in keys {
                if let Some((rx, tx)) = map.get(k) {
                    if *rx > 0 || *tx > 0 {
                        parts.push(format!("{}: ↓{} ↑{}", k, format_bytes(*rx), format_bytes(*tx)));
                    }
                }
            }
            if parts.is_empty() { "Idle".to_string() } else { parts.join(" | ") }
        },
        MetricValue::Location(lat, lon) => format!("({:.2}, {:.2})", lat, lon),
        MetricValue::None => "---".to_string(),
    }
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB { format!("{:.1}GB/s", bytes as f64 / GB as f64) }
    else if bytes >= MB { format!("{:.1}MB/s", bytes as f64 / MB as f64) }
    else if bytes >= KB { format!("{:.1}KB/s", bytes as f64 / KB as f64) }
    else { format!("{}B/s", bytes) }
}

pub fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 { return Err(anyhow::anyhow!("Invalid hex color length")); }
    let r = u8::from_str_radix(&hex[0..2], 16)? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16)? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16)? as f64 / 255.0;
    Ok((r, g, b))
}
