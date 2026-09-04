// src/core/config/presets.rs
//! [Phase 8] Performance presets — the table the three buttons write.
//!
//! Before this phase the buttons existed and did nothing (GL-2 / Ghost Logic).
//! Each preset now writes four derived fields, and the config is persisted
//! through the normal atomic `.tmp`-then-rename path.
//!
//! `Minimal` is deliberately absent. It needs Pulse Mode (Phase 7); a Minimal
//! that merely stopped drawing would blank the user's desktop rather than make
//! it cheap, which is not the same product.

use super::{defaults::default_glow_passes, Config};

/// One preset's derived values.
struct Preset {
    target_fps: u32,
    realism: u32,
    glow_passes: usize,
    rain_mode: &'static str,
}

/// `medium` is the S-04 configuration: measured at **3.0033% mean** across
/// three 300 s M-1 windows (`S04_AT_GATE`).
///
/// `extreme` is opt-in and **exempt from S-04** — 30 fps × the same per-tick
/// cost is roughly 30× the ambience budget, and its GUI label says so.
fn table(name: &str) -> Option<Preset> {
    match name {
        "medium" => Some(Preset { target_fps: 1, realism: 4, glow_passes: 3, rain_mode: "fall" }),
        "extreme" => Some(Preset { target_fps: 30, realism: 10, glow_passes: 5, rain_mode: "fall" }),
        _ => None,
    }
}

/// The first `n` passes of the default glow ramp, centred pass always kept.
fn glow(n: usize) -> Vec<(f64, f64, f64)> {
    let all = default_glow_passes();
    if n >= all.len() { return all; }
    // Take from the middle outward so the centre pass (0,0) survives a trim.
    let start = (all.len() - n) / 2;
    all[start..start + n].to_vec()
}

/// Apply a named preset. Returns false for an unknown name (including
/// `"minimal"` and `"custom"`), leaving the config untouched.
pub fn apply(config: &mut Config, name: &str) -> bool {
    let Some(p) = table(name) else { return false };
    config.general.target_fps = p.target_fps;
    config.cosmetics.realism = p.realism;
    config.general.glow_passes = glow(p.glow_passes);
    config.cosmetics.rain_mode = p.rain_mode.to_string();
    config.cosmetics.perf_preset = name.to_string();
    true
}

/// Does this config still match the preset it claims? [Phase 8.4] An individual
/// edit through the GUI must demote `perf_preset` to `"custom"` rather than let
/// the label keep asserting a configuration the user has since changed.
pub fn matches(config: &Config, name: &str) -> bool {
    match table(name) {
        None => true, // "custom"/"minimal" claim nothing, so nothing can contradict them
        Some(p) => {
            config.general.target_fps == p.target_fps
                && config.cosmetics.realism == p.realism
                && config.general.glow_passes.len() == p.glow_passes
                && config.cosmetics.rain_mode == p.rain_mode
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_is_the_s04_configuration() {
        let mut c = Config::default();
        assert!(apply(&mut c, "medium"));
        assert_eq!(c.general.target_fps, 1);
        assert_eq!(c.cosmetics.realism, 4);
        assert_eq!(c.general.glow_passes.len(), 3);
        assert_eq!(c.cosmetics.rain_mode, "fall");
        assert_eq!(c.cosmetics.perf_preset, "medium");
        assert!(matches(&c, "medium"));
    }

    #[test]
    fn extreme_moves_the_frame_rate() {
        let mut c = Config::default();
        assert!(apply(&mut c, "extreme"));
        assert_eq!(c.general.target_fps, 30);
        assert_eq!(c.general.glow_passes.len(), 5);
        assert!(matches(&c, "extreme"));
        assert!(!matches(&c, "medium"));
    }

    /// [8.4] Editing a derived field must break the preset claim.
    #[test]
    fn an_individual_edit_no_longer_matches() {
        let mut c = Config::default();
        apply(&mut c, "medium");
        c.general.target_fps = 5;
        assert!(!matches(&c, "medium"), "an edited config must not still claim its preset");
    }

    /// Minimal is not in the table, so applying it changes nothing — the button
    /// must not silently blank the overlay.
    #[test]
    fn minimal_is_not_applied() {
        let mut c = Config::default();
        let before = c.general.target_fps;
        assert!(!apply(&mut c, "minimal"));
        assert_eq!(c.general.target_fps, before);
    }

    /// The centre glow pass survives a trim to 3.
    #[test]
    fn trimmed_glow_keeps_the_centre_pass() {
        assert!(glow(3).iter().any(|p| p.0 == 0.0 && p.1 == 0.0));
        assert_eq!(glow(99).len(), default_glow_passes().len());
    }
}
