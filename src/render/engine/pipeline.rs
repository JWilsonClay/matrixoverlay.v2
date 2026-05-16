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
        self.rain.realism_scale = config.cosmetics.realism_scale;
        self.color_rgb = match config.general.theme.as_str() {
            "calm" => (0.0, 0.8, 1.0), "alert" => (1.0, 0.2, 0.2),
            _ => layout::parse_hex_color(&config.general.color).unwrap_or((0.0, 1.0, 0.25)),
        };
    }

    pub fn draw(&mut self, conn: &xcb::Connection, window: x::Window, config: &Config, metrics: &MetricData, dt: Duration) -> Result<()> {
        *self.frames.borrow_mut() += 1;
        let cr = CairoContext::new(&self.surface)?;
        self.clear(&cr)?;
        self.rain.update(dt, self.width, self.height, config);
        self.visual_elements.borrow_mut().clear();
        if config.cosmetics.rain_mode == "fall" { self.rain.draw(&cr, self.width as f64, self.height as f64, *self.frames.borrow(), config)?; }
        self.draw_metrics(&cr, config, metrics)?;
        drop(cr);
        self.present(conn, window)?;
        Ok(())
    }

    fn draw_metrics(&self, cr: &CairoContext, config: &Config, metrics: &MetricData) -> Result<()> {
        for item in &self.config_layout.items {
            if let Some(id) = MetricId::from_str(&item.metric_id) {
                if let Some(v) = metrics.values.get(&id) {
                    let v_s = layout::format_metric_value(v);
                    let lbl = if item.label.is_empty() { id.label() } else { item.label.clone() };
                    if let Ok(Some(el)) = layout::draw_metric_pair(cr, &lbl, &v_s, item.x as f64, item.y as f64, item.max_width as f64, &item.metric_id, true, &config.general.glow_passes, config, item, self.color_rgb, &mut self.scroll.borrow_mut()) {
                        self.visual_elements.borrow_mut().push(el);
                    }
                }
            }
        }
        Ok(())
    }
}
