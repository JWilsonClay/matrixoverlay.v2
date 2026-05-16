//! Weather metrics collection substrate via Open-Meteo.
use std::collections::HashMap;
use std::time::{Duration, Instant};
use anyhow::Result;
use serde::Deserialize;
use crate::metrics::{MetricId, MetricValue, MetricCollector};

#[derive(Deserialize)] struct OpenMeteoResponse { current: CurrentWeather }
#[derive(Deserialize)] struct CurrentWeather { temperature_2m: f64, weather_code: i64 }

#[derive(Debug)]
pub struct OpenMeteoCollector {
    lat: f64, lon: f64, enabled: bool, auto_loc: bool, temp_unit: String,
    url: Option<String>,
    last_fetch: Option<Instant>, last_ok: Option<Instant>,
    c_temp: Option<String>, c_cond: Option<String>,
}

impl OpenMeteoCollector {
    pub fn new(lat: f64, lon: f64, enabled: bool, auto_loc: bool, temp_unit: String) -> Self {
        Self { lat, lon, enabled, auto_loc, temp_unit, url: None, last_fetch: None, last_ok: None, c_temp: None, c_cond: None }
    }

    pub fn new_with_url(_id: crate::metrics::MetricId, lat: f64, lon: f64, url: String) -> Self {
        Self { lat, lon, enabled: true, auto_loc: false, temp_unit: "celsius".to_string(), url: Some(url), last_fetch: None, last_ok: None, c_temp: None, c_cond: None }
    }

    fn code_to_str(c: i64) -> &'static str {
        match c { 0 => "Clear", 1..=3 => "Cloudy", 45|48 => "Fog", 51..=55 => "Drizzle", 61..=65 => "Rain", 71..=75 => "Snow", 95..=99 => "Storm", _ => "N/A" }
    }
}

impl MetricCollector for OpenMeteoCollector {
    fn id(&self) -> &'static str { "open_meteo" }
    fn label(&self) -> &'static str { "Weather" }
    fn collect(&mut self) -> HashMap<MetricId, MetricValue> {
        let mut map = HashMap::new();
        if !self.enabled { return map; }
        let now = Instant::now();
        if let Some(t) = self.last_fetch { if now.duration_since(t) < Duration::from_secs(60) {
            if let Some(ct) = &self.c_temp { map.insert(MetricId::WeatherTemp, MetricValue::String(ct.clone())); }
            if let Some(cc) = &self.c_cond { map.insert(MetricId::WeatherCondition, MetricValue::String(cc.clone())); }
            return map;
        }}
        self.last_fetch = Some(now);

        let url = self.url.clone().unwrap_or_else(|| {
            format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code", self.lat, self.lon)
        });
        let client = reqwest::blocking::Client::builder().timeout(Duration::from_secs(5)).build().ok();
        if let Some(c) = client { if let Ok(resp) = c.get(&url).send() {
            if let Ok(json) = resp.json::<OpenMeteoResponse>() {
                let mut t = json.current.temperature_2m;
                let suf = if self.temp_unit == "fahrenheit" { t = (t * 1.8) + 32.0; "°F" } else { "°C" };
                let t_s = format!("{:.1}{}", t, suf);
                let c_s = Self::code_to_str(json.current.weather_code).to_string();
                self.c_temp = Some(t_s.clone()); self.c_cond = Some(c_s.clone()); self.last_ok = Some(now);
                map.insert(MetricId::WeatherTemp, MetricValue::String(t_s));
                map.insert(MetricId::WeatherCondition, MetricValue::String(c_s));
                return map;
            }
        }}
        if let Some(t) = &self.c_temp { map.insert(MetricId::WeatherTemp, MetricValue::String(t.clone())); }
        map
    }
}

/// [HARDENED] Fetches current lat/lon via GeoIP with failure isolation.
pub fn fetch_geoip_location() -> Result<(f64, f64)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    
    let resp = client.get("http://ip-api.com/json")
        .send()?
        .json::<serde_json::Value>()?;
    
    let lat = resp["lat"].as_f64().ok_or_else(|| anyhow::anyhow!("Lat missing"))?;
    let lon = resp["lon"].as_f64().ok_or_else(|| anyhow::anyhow!("Lon missing"))?;
    
    Ok((lat, lon))
}
