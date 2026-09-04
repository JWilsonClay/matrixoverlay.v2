use std::sync::OnceLock;
use std::time::Duration;
use anyhow::Result;
use cairo::{Context as CairoContext, Operator};
use xcb::x;
use crate::core::config::Config;
use crate::metrics::{MetricData, MetricId};
use crate::render::layout;
use super::renderer::Renderer;

impl Renderer {
    pub fn clear(&self, cr: &CairoContext) -> Result<()> {
        cr.set_operator(Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 1.0);
        cr.paint()?;
        cr.set_operator(Operator::Over);
        Ok(())
    }

    pub fn update_config(&mut self, config: Config) {
        self.config_layout = crate::core::layout::compute(&config, self.monitor_index, self.width as u16, self.height as u16);
        self.rain.realism = config.cosmetics.realism;
        self.color_rgb = match config.general.theme.as_str() {
            "calm" => (0.0, 0.8, 1.0), "alert" => (1.0, 0.2, 0.2),
            _ => layout::parse_hex_color(&config.general.color).unwrap_or((0.0, 1.0, 0.25)),
        };
    }

    pub fn draw(&mut self, conn: &xcb::Connection, window: x::Window, config: &Config, metrics: &MetricData, dt: Duration) -> Result<()> {
        *self.frames.borrow_mut() += 1;
        self.presenter.pre_draw(conn)?;
        let cr = CairoContext::new(self.presenter.surface())?;
        // [5.8 F-B] `clear` was last measured at 30 fps on a standalone surface
        // (S-13a, ~3.29 ms). This times it on the live SHM surface at the
        // configured rate, which is the figure the identity actually needs.
        let (dbg, _, _) = Self::debug_flags();
        let t_clear = if dbg { Some(std::time::Instant::now()) } else { None };
        self.clear(&cr)?;
        if let Some(t) = t_clear {
            crate::core::telemetry::phase58::record_clear(
                self.width as u16, self.height as u16, t.elapsed().as_nanos() as u64);
        }
        self.rain.update(dt, self.width, self.height, config);
        self.visual_elements.borrow_mut().clear();
        if config.cosmetics.rain_mode == "fall" {
            self.draw_rain_timed(&cr, config)?;
        }
        // [5.8 F-A] The metrics panel — six `show_layout` calls per metric per
        // frame through `draw_text_glow_at`. Named as unmeasured in three
        // consecutive receipts; measured here.
        let t_glow = if dbg { Some(std::time::Instant::now()) } else { None };
        self.draw_metrics(&cr, config, metrics)?;
        if let Some(t) = t_glow {
            crate::core::telemetry::phase58::record_glow(
                self.width as u16, self.height as u16, t.elapsed().as_nanos() as u64);
        }
        drop(cr);
        self.presenter.present(conn, window)?;
        Ok(())
    }

    /// Debug-path flags, resolved ONCE. The previous version called
    /// `env::var_os` on every frame of the path it was measuring.
    fn debug_flags() -> (bool, bool, bool) {
        static F: OnceLock<(bool, bool, bool)> = OnceLock::new();
        *F.get_or_init(|| (
            std::env::var_os("MATRIX_OVERLAY_DEBUG_METRICS").is_some(),
            std::env::var_os("MATRIX_OVERLAY_DEBUG_GLYPHS").is_some(),
            std::env::var_os("MATRIX_OVERLAY_DEBUG_CONTROL").is_some(),
        ))
    }

    /// Production rain draw, optionally instrumented.
    ///
    /// [X-LIVE] Times the production draw so the MRC's figure can be reconciled
    /// against the running substrate; [Q1] counts the glyphs that survive the
    /// clip guard; [Q3] times an in-process single-size control, which is the
    /// only sanctioned Phase 3 re-entry denominator. All three are inert unless
    /// their env var was set at startup: no allocation, no logging, no per-frame
    /// env lookup.
    fn draw_rain_timed(&mut self, cr: &CairoContext, config: &Config) -> Result<()> {
        let (dbg, glyphs, control) = Self::debug_flags();
        if dbg { crate::core::telemetry::record_font_options(
            || crate::render::describe_font_options(cr)); }
        let (w, h) = (self.width as f64, self.height as f64);
        let (gw, gh) = (self.width as u16, self.height as u16);
        let fc = *self.frames.borrow();
        if glyphs { crate::render::physics::count_show_layout(true); let _ = crate::render::physics::take_survived(); }

        let t = if dbg { Some(std::time::Instant::now()) } else { None };
        self.rain.draw(cr, w, h, fc, config)?;
        if let Some(t) = t {
            crate::core::telemetry::record_rain_draw(gw, gh, t.elapsed().as_nanos() as u64);
        }
        if glyphs {
            crate::core::telemetry::record_survived(gw, gh, crate::render::physics::take_survived());
            crate::render::physics::count_show_layout(false);
        }
        if control {
            // Clone so the live simulation is never mutated; flatten depth so
            // every stream resolves to ONE font size. Same production `draw`.
            let mut flat = self.rain.clone();
            for s in &mut flat.streams { s.depth = 1.0; }
            let t = std::time::Instant::now();
            flat.draw(cr, w, h, fc, config)?;
            crate::core::telemetry::record_live_control(gw, gh, t.elapsed().as_nanos() as u64);
        }
        Ok(())
    }

    /// [GL-2, Phase 8.7] `general.show_monitor_label` was stored, exposed in the
    /// GUI, and read by nothing — the checkbox toggled a value with no effect.
    /// Wired rather than deleted: the field is already present in users'
    /// `config.json`, and `#[serde(deny_unknown_fields)]` (C-02) would reject
    /// those files if the struct lost it.
    ///
    /// Drawn at the panel origin with the same glow the metrics use. Costs one
    /// short string per frame per monitor, and only when the flag is on.
    fn draw_monitor_label(&self, cr: &CairoContext, config: &Config) -> Result<()> {
        if !config.general.show_monitor_label { return Ok(()); }
        // Anchor above the first laid-out metric; if the panel is empty there is
        // nothing to label, so draw nothing rather than float a label in space.
        let Some(first) = self.config_layout.items.first() else { return Ok(()) };
        let (x, y) = (first.x as f64, first.y as f64);
        let text = format!("Monitor {}", self.monitor_index + 1);
        let lay = pangocairo::functions::create_layout(cr);
        let mut d = pangocairo::pango::FontDescription::from_string("Monospace");
        d.set_size(config.general.metric_font_size as i32 * pangocairo::pango::SCALE);
        lay.set_font_description(Some(&d));
        lay.set_text(&text);
        layout::draw_text_glow_at(cr, &lay, x, y - 28.0, self.color_rgb, &config.general.glow_passes, config)
    }

    fn draw_metrics(&self, cr: &CairoContext, config: &Config, metrics: &MetricData) -> Result<()> {
        self.draw_monitor_label(cr, config)?;
        for item in &self.config_layout.items {
            if let Some(id) = MetricId::from_str(&item.metric_id) {
                if let Some(v) = metrics.values.get(&id) {
                    let v_s = layout::format_metric_value(v);
                    let res = if item.metric_id == "day_of_week" {
                        layout::draw_day_of_week(cr, &v_s, item.x as f64, item.y as f64, 200.0, 50.0, &config.general.glow_passes, config, self.color_rgb)
                    } else {
                        let lbl = if item.label.is_empty() { id.label() } else { item.label.as_str() };
                        layout::draw_metric_pair(cr, &lbl, &v_s, item.x as f64, item.y as f64, item.max_width as f64, &item.metric_id, true, &config.general.glow_passes, config, item, self.color_rgb, &mut self.scroll.borrow_mut())
                    };

                    if let Ok(Some(el)) = res {
                        self.visual_elements.borrow_mut().push(el);
                    }
                }
            }
        }
        Ok(())
    }
}
