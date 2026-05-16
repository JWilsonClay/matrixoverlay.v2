// src/render/physics/rain_stream.rs
use rand::Rng;
use rand::thread_rng;

pub struct RainStream {
    pub x: f64, pub y: f64, pub speed: f64,
    pub glyphs: Vec<char>, pub depth: f64,
}

impl RainStream {
    pub fn new(x: f64, y: f64, speed: f64, depth: f64) -> Self {
        Self { x, y, speed, glyphs: (0..10).map(|_| random_char()).collect(), depth }
    }
}

pub fn random_char() -> char {
    let code = thread_rng().gen_range(0x30A1..=0x30F6);
    std::char::from_u32(code).unwrap_or('?')
}
