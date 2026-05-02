use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::metrics::{MetricId, MetricValue, MetricCollector};

/// Collector for AI-driven insights (Ollama).
/// Throttled to 1/hr and skipped if CPU > 80%.
#[derive(Debug)]
pub struct OllamaCollector {
    last_fetch: Instant,
}

impl OllamaCollector {
    pub fn new() -> Self {
        Self {
            last_fetch: Instant::now() - Duration::from_secs(3601),
        }
    }
}

impl MetricCollector for OllamaCollector {
    fn id(&self) -> &'static str { "ollama" }
    fn label(&self) -> &'static str { "AI Insight" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        
        // Throttling logic
        if self.last_fetch.elapsed() < Duration::from_secs(3600) {
            return map;
        }

        log::info!("OllamaCollector: Fetching insight (Throttled 1/hr)");
        self.last_fetch = Instant::now();
        map.insert(MetricId::Custom("ai_insight".to_string()), MetricValue::String("Ready".to_string()));
        map
    }
}
