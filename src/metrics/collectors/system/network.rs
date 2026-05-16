// src/metrics/collectors/system/network.rs
use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Debug)]
pub struct NetworkCollector {
    last_snapshot: HashMap<String, (u64, u64)>,
    last_time: Instant,
}

impl NetworkCollector {
    pub fn new() -> Self { Self { last_snapshot: HashMap::new(), last_time: Instant::now() } }

    fn read_proc_net(&self) -> HashMap<String, (u64, u64)> {
        let mut map = HashMap::new();
        if let Ok(c) = fs::read_to_string("/proc/net/dev") {
            for line in c.lines().skip(2) {
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 10 {
                    let iface = parts[0].trim_end_matches(':').to_string();
                    if let (Ok(rx), Ok(tx)) = (parts[1].parse(), parts[9].parse()) {
                        map.insert(iface, (rx, tx));
                    }
                }
            }
        }
        map
    }
}

impl MetricCollector for NetworkCollector {
    fn id(&self) -> &'static str { "network" }
    fn label(&self) -> &'static str { "Net" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let now = Instant::now();
        let current = self.read_proc_net();
        let mut details = HashMap::new();

        for (iface, (c_rx, c_tx)) in &current {
            if iface == "lo" { continue; }
            if let Some((l_rx, l_tx)) = self.last_snapshot.get(iface) {
                details.insert(iface.clone(), (c_rx.saturating_sub(*l_rx), c_tx.saturating_sub(*l_tx)));
            }
        }

        self.last_snapshot = current;
        self.last_time = now;
        let mut map = HashMap::new();
        map.insert(MetricId::NetworkDetails, MetricValue::NetworkMap(details));
        map
    }
}
