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
    let resp = reqwest::blocking::Client::new()
        .get("http://ip-api.com/json")
        .timeout(Duration::from_secs(3))
        .send()?;

    #[derive(Deserialize)]
    struct IpApiResponse {
        lat: f64,
        lon: f64,
    }

    let geo = resp.json::<IpApiResponse>()?;
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

        if self.auto_location && (self.lat == 0.0 || self.lon == 0.0) {
             if let Ok((lat, lon)) = fetch_geoip_location() {
                 self.lat = lat;
                 self.lon = lon;
             }
        }

        let url = format!("{}/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code", self.url_base, self.lat, self.lon);

        match reqwest::blocking::Client::new().get(&url).timeout(std::time::Duration::from_secs(5)).send() {
            Ok(resp) => {
                if let Ok(json) = resp.json::<OpenMeteoResponse>() {
                    let mut temp = json.current.temperature_2m;
                    let mut suffix = "°C";
                    if self.temp_unit == "fahrenheit" {
                        temp = (temp * 9.0 / 5.0) + 32.0;
                        suffix = "°F";
                    }
                    map.insert(MetricId::WeatherTemp, MetricValue::String(format!("{:.1}{}", temp, suffix)));
                    map.insert(MetricId::WeatherCondition, MetricValue::String(Self::weather_code_str(json.current.weather_code)));
                }
            },
            Err(e) => {
                log::warn!("Weather fetch failed: {}", e);
                map.insert(MetricId::WeatherTemp, MetricValue::String("N/A".to_string()));
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
