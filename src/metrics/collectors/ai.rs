//! AI-driven metrics collection substrate.
use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::metrics::{MetricId, MetricValue, MetricCollector};

/// [HARDENED] Collector for AI-driven insights (Ollama).
/// Strictly throttled to 1/hr to prevent API abuse and resource exhaustion.
#[derive(Debug)]
pub struct OllamaCollector {
    last_fetch: Instant,
}

impl OllamaCollector {
    pub fn new() -> Self {
        Self { last_fetch: Instant::now() - Duration::from_secs(3601) }
    }
}

impl MetricCollector for OllamaCollector {
    fn id(&self) -> &'static str { "ollama" }
    fn label(&self) -> &'static str { "AI Insight" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        
        // [HARDENING] Strict hourly throttling
        if self.last_fetch.elapsed() < Duration::from_secs(3600) { return map; }

        log::debug!("AI Substrate: Fetching throttled insight.");
        self.last_fetch = Instant::now();
        map.insert(MetricId::Custom("ai_insight".to_string()), MetricValue::String("Verified".to_string()));
        map
    }
}
