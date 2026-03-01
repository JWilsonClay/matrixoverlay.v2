// benches/render_bench.rs
extern crate criterion; // Critical for macro expansion

use criterion::{Criterion, criterion_group, criterion_main};
use cairo::{ImageSurface, Format, Context};
use pangocairo::pango::FontDescription;

fn benchmark_text_rendering(c: &mut Criterion) {
    let width = 1920;
    let height = 1080;
    let surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    let cr = Context::new(&surface).unwrap();
    
    let font_str = "Monospace 14";
    let desc = FontDescription::from_string(font_str);
    
    c.bench_function("render_text_with_glow", |b| b.iter(|| {
        cr.set_source_rgb(0.0, 0.0, 0.0);
        cr.paint().unwrap();
        
        let layout = pangocairo::functions::create_layout(&cr);
        layout.set_font_description(Some(&desc));
        layout.set_text("CPU : 12.5%");
        
        cr.set_source_rgba(0.0, 1.0, 0.0, 0.15);
        for _ in 0..4 {
            pangocairo::functions::show_layout(&cr, &layout);
        }
        
        cr.set_source_rgba(0.0, 1.0, 0.0, 1.0);
        pangocairo::functions::show_layout(&cr, &layout);
    }));
}

criterion_group!(benches, benchmark_text_rendering);
criterion_main!(benches);
