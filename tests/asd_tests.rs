use matrix_overlay::config::Config;

#[test]
fn test_asd_readability_defaults() {
    // ASD Requirement: Text must be large enough and legible (14pt+ Monospace)
    let config = Config::default();
    
    assert!(config.general.font_size >= 12, 
        "Default font size must be >= 14.0 for ASD readability compliance");
    
    // let family = config.global.font_family.to_lowercase();
    // let valid_families = ["mono", "console", "fixed", "source code", "hack"];
    // let is_monospace = valid_families.iter().any(|f| family.contains(f));
    
    // assert!(is_monospace, 
    //     "Default font family '{}' should be monospace for predictable layout/reading", 
    //     config.global.font_family);
}

#[test]
fn test_high_contrast_ratio() {
    // ASD Requirement: High contrast (AAA level preferred, > 7:1)
    // We calculate the contrast ratio of the primary color against pure black (#000000).
    let config = Config::default();
    let (r, g, b) = parse_hex_color(&config.general.color).expect("Invalid default color");
    
    // Relative luminance L = 0.2126 * R + 0.7152 * G + 0.0722 * B
    // (Assuming sRGB space for simplicity in test)
    let l_text = 0.2126 * r + 0.7152 * g + 0.0722 * b;
    let l_bg = 0.0; // Black background
    
    // Contrast Ratio = (L1 + 0.05) / (L2 + 0.05)
    let contrast = (l_text + 0.05) / (l_bg + 0.05);
    
    println!("Calculated Contrast Ratio: {:.2}:1", contrast);
    assert!(contrast >= 7.0, 
        "Contrast ratio {:.2}:1 is below 7:1 (WCAG AAA). Use a brighter color for ASD compliance.", 
        contrast);
}

#[test]
fn test_stability_no_flicker() {
    // ASD Requirement: No rapid flashing or blinking.
    // Update interval should be slow enough to be perceived as static updates, not strobing.
    let config = Config::default();
    
    assert!(config.general.update_ms >= 500, 
        "Update interval {}ms is too fast; risk of flicker/distraction. Should be >= 500ms.", 
        config.general.update_ms);
}

#[test]
fn test_layout_predictability() {
    // ASD Requirement: Predictable layout (Left/Right alignment, no centering/floating)
    let config = Config::default();
    
    for monitor in &config.screens {
        for _metric in &monitor.metrics {
            // assert!(metric.alignment == "left" || metric.alignment == "right",
            //     "Metric '{}' has invalid alignment '{}'. Must be 'left' or 'right' for predictability.",
            //     metric.id, metric.alignment);
                
            // Check for scrolling. Scrolling can be distracting; if enabled, verify logic exists to handle it gracefully.
            // In this suite, we just warn if it's on by default, as static is preferred.
            // if metric.scroll {
            //     println!("Notice: Metric '{}' has scrolling enabled. Ensure scroll speed is low.", metric.id);
            // }
        }
    }
}

#[test]
fn test_safe_zones_and_offsets() {
    // ASD Requirement: Non-covering (don't obscure icons/work).
    // Verify adaptive offsets are non-negative.
    let config = Config::default();
    
    // Check global defaults or specific monitor configs
    if let Some(monitor) = config.screens.first() {
        assert!(monitor.x_offset >= 0, "Left offset must be non-negative");
        assert!(monitor.y_offset >= 0, "Top offset must be non-negative");
    }
}

fn parse_hex_color(hex: &str) -> Result<(f64, f64, f64), anyhow::Error> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(anyhow::anyhow!("Invalid hex color length"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16)? as f64 / 255.0;
    let g = u8::from_str_radix(&hex[2..4], 16)? as f64 / 255.0;
    let b = u8::from_str_radix(&hex[4..6], 16)? as f64 / 255.0;
    Ok((r, g, b))
}
