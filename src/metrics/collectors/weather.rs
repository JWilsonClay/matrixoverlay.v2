use std::collections::HashMap;
use std::time::Duration;
use serde::Deserialize;
use anyhow::Result;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Deserialize)]
struct OpenMeteoResponse {
    current: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    weather_code: i64,
}

/// Collector for Weather data from Open-Meteo.
#[derive(Debug)]
pub struct OpenMeteoCollector {
    lat: f64,
    lon: f64,
    enabled: bool,
    auto_location: bool,
    url_base: String,
    temp_unit: String,
    last_fetch: Option<std::time::Instant>,
    last_success: Option<std::time::Instant>,
    cache_temp: Option<String>,
    cache_condition: Option<String>,
}

impl OpenMeteoCollector {
    pub fn new(lat: f64, lon: f64, enabled: bool, auto_location: bool) -> Self {
        Self {
            lat,
            lon,
            enabled,
            auto_location,
            url_base: "https://api.open-meteo.com".to_string(),
            temp_unit: "celsius".to_string(),
            last_fetch: None,
            last_success: None,
            cache_temp: None,
            cache_condition: None,
        }
    }

    pub fn new_with_unit(lat: f64, lon: f64, enabled: bool, auto_location: bool, temp_unit: String) -> Self {
        Self {
            lat,
            lon,
            enabled,
            auto_location,
            url_base: "https://api.open-meteo.com".to_string(),
            temp_unit,
            last_fetch: None,
            last_success: None,
            cache_temp: None,
            cache_condition: None,
        }
    }

    pub fn new_with_url(_metric_id: MetricId, lat: f64, lon: f64, url: String) -> Self {
        Self {
            lat,
            lon,
            enabled: true,
            auto_location: false,
            url_base: url,
            temp_unit: "celsius".to_string(),
            last_fetch: None,
            last_success: None,
            cache_temp: None,
            cache_condition: None,
        }
    }

    fn weather_code_str(code: i64) -> String {
        match code {
            0 => "Clear sky",
            1 | 2 | 3 => "Partly cloudy",
            45 | 48 => "Fog",
            51 | 53 | 55 => "Drizzle",
            56 | 57 => "Freezing Drizzle",
            61 | 63 | 65 => "Rain",
            66 | 67 => "Freezing Rain",
            71 | 73 | 75 => "Snow",
            77 => "Snow grains",
            80 | 81 | 82 => "Rain showers",
            85 | 86 => "Snow showers",
            95 => "Thunderstorm",
            96 | 99 => "Thunderstorm (Hail)",
            _ => "Unknown",
        }.to_string()
    }
}

pub fn fetch_geoip_location() -> Result<(f64, f64)> {
    log::info!("Fetching Geo-IP coordinates...");
    
    // **[HARDENING: Network Resilience]**
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .user_agent("MatrixOverlay/2.0 (Security Hardened)")
        .build()?;

    let mut resp = client.get("http://ip-api.com/json").send()?;

    #[derive(Deserialize)]
    struct IpApiResponse {
        lat: f64,
        lon: f64,
    }

    // **[HARDENING: Response Size Limit]**
    use std::io::Read;
    let mut buffer = Vec::new();
    resp.by_ref().take(1024 * 5).read_to_end(&mut buffer)?;

    let geo = serde_json::from_slice::<IpApiResponse>(&buffer)?;
    log::info!("Geo-IP Detected Location: ({}, {})", geo.lat, geo.lon);
    Ok((geo.lat, geo.lon))
}

impl MetricCollector for OpenMeteoCollector {
    fn id(&self) -> &'static str { "open_meteo" }
    fn label(&self) -> &'static str { "Weather" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if !self.enabled {
            return map;
        }

        // **[NEW: Throttling & Caching Logic]**
        let now = std::time::Instant::now();
        let minute = Duration::from_secs(60);
        let stale_limit = Duration::from_secs(15 * 60);

        // If we fetched recently, return cached values immediately
        if let Some(last) = self.last_fetch {
            if now.duration_since(last) < minute {
                if let Some(temp) = &self.cache_temp {
                    map.insert(MetricId::WeatherTemp, MetricValue::String(temp.clone()));
                }
                if let Some(cond) = &self.cache_condition {
                    map.insert(MetricId::WeatherCondition, MetricValue::String(cond.clone()));
                }
                return map;
            }
        }

        self.last_fetch = Some(now);

        // **[HARDENING: Resource Resilience]**
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("MatrixOverlay/2.0 (Security Hardened)")
            .build() 
        {
            Ok(c) => c,
            Err(_) => return map,
        };

        let mut fresh_location = false;
        if self.auto_location && (self.lat == 0.0 || self.lon == 0.0) {
             if let Ok((lat, lon)) = fetch_geoip_location() {
                 self.lat = lat;
                 self.lon = lon;
                 fresh_location = true;
                 // Report fresh location for anchoring to config.json
                 map.insert(MetricId::LocationData, MetricValue::Location(lat, lon));
             }
        }

        let url = format!("{}/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code", self.url_base, self.lat, self.lon);

        match client.get(&url).send() {
            Ok(mut resp) => {
                use std::io::Read;
                let mut buffer = Vec::new();
                if let Ok(_) = resp.by_ref().take(1024 * 10).read_to_end(&mut buffer) {
                    if let Ok(json) = serde_json::from_slice::<OpenMeteoResponse>(&buffer) {
                        let mut temp_val = json.current.temperature_2m;
                        let mut suffix = "°C";
                        if self.temp_unit == "fahrenheit" {
                            temp_val = (temp_val * 9.0 / 5.0) + 32.0;
                            suffix = "°F";
                        }
                        
                        let temp_str = format!("{:.1}{}", temp_val, suffix);
                        let cond_str = Self::weather_code_str(json.current.weather_code);

                        self.cache_temp = Some(temp_str.clone());
                        self.cache_condition = Some(cond_str.clone());
                        self.last_success = Some(now);

                        map.insert(MetricId::WeatherTemp, MetricValue::String(temp_str));
                        map.insert(MetricId::WeatherCondition, MetricValue::String(cond_str));
                    }
                }
            },
            Err(e) => {
                log::warn!("Weather fetch failed: {}", e);
                
                // **[NEW: Stale Cache Handling]**
                if let Some(last_ok) = self.last_success {
                    let elapsed = now.duration_since(last_ok);
                    if elapsed < stale_limit {
                        // Return silent cache
                        if let Some(temp) = &self.cache_temp {
                            map.insert(MetricId::WeatherTemp, MetricValue::String(temp.clone()));
                        }
                        if let Some(cond) = &self.cache_condition {
                            map.insert(MetricId::WeatherCondition, MetricValue::String(cond.clone()));
                        }
                    } else {
                        // Return stale cache with asterisk
                        if let Some(temp) = &self.cache_temp {
                            map.insert(MetricId::WeatherTemp, MetricValue::String(format!("{}*", temp)));
                        }
                        if let Some(cond) = &self.cache_condition {
                            map.insert(MetricId::WeatherCondition, MetricValue::String(format!("{}*", cond)));
                        }
                    }
                } else {
                    map.insert(MetricId::WeatherTemp, MetricValue::String("N/A".to_string()));
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[test]
    fn test_open_meteo_collector() {
        let mut server = Server::new();
        let _m = server.mock("GET", "/v1/forecast?latitude=51.5074&longitude=-0.1278&current=temperature_2m,weather_code")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"current": {"temperature_2m": 15.5, "weather_code": 3}}"#)
            .create();

        let url = server.url();
        let mut collector = OpenMeteoCollector::new_with_url(MetricId::WeatherTemp, 51.5074, -0.1278, url);
        let values = collector.collect();
        let value = values.get(&MetricId::WeatherTemp).unwrap();
        if let MetricValue::String(v) = value {
            assert!(v.contains("15.5"), "Expected 15.5 in string, got {}", v);
        }

        let value_cond = values.get(&MetricId::WeatherCondition).unwrap();
        if let MetricValue::String(v) = value_cond {
            assert_eq!(v, "Partly cloudy");
        }
    }
}
