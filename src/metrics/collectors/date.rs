use std::collections::HashMap;
use chrono::Local;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

/// Collector for Date/Time (Day of Week).
#[derive(Debug)]
pub struct DateCollector;

impl MetricCollector for DateCollector {
    fn id(&self) -> &'static str { "date" }
    fn label(&self) -> &'static str { "Date" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        let day = Local::now().format("%A").to_string();
        log::debug!("Collected DayOfWeek: {}", day);
        map.insert(MetricId::DayOfWeek, MetricValue::String(day));
        map
    }
}
