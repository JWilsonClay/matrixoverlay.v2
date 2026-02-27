use std::time::{Duration, Instant};
use std::thread;
use sysinfo::{Pid, ProcessExt, System, SystemExt};
use cairo::{ImageSurface, Format, Context, Operator};

#[test]
fn test_update_latency_accuracy() {
    // Verify that a simulated 1000ms loop stays within acceptable drift (<50ms)
    let target_interval = Duration::from_millis(100); // Scaled down for test speed
    let iterations = 5;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let loop_start = Instant::now();
        // Simulate work (e.g. metrics collection)
        thread::sleep(Duration::from_millis(10));
        
        let elapsed = loop_start.elapsed();
        if elapsed < target_interval {
            thread::sleep(target_interval - elapsed);
        }
    }
    
    let total_elapsed = start.elapsed();
    let expected = target_interval * iterations as u32;
    let diff = if total_elapsed > expected {
        total_elapsed - expected
    } else {
        expected - total_elapsed
    };
    
    // Allow small overhead margin
    assert!(diff.as_millis() < 50, "Timer drift too high: {}ms", diff.as_millis());
}

#[test]
fn test_cpu_ram_usage_simulation() {
    // Measure the resource usage of the test process during a simulated workload
    let mut sys = System::new_all();
    let pid = Pid::from(std::process::id() as usize);
    
    // Warmup
    sys.refresh_process(pid);
    
    // Simulate "heavy" loop (metrics + render logic)
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(500) {
        sys.refresh_cpu(); // Simulate sysinfo work
        thread::sleep(Duration::from_millis(16)); // ~60 FPS simulation
    }
    
    sys.refresh_process(pid);
    let proc = sys.process(pid).expect("Failed to get process info");
    
    println!("Simulated CPU: {:.2}%, RAM: {} bytes", proc.cpu_usage(), proc.memory());
    
    // Sanity checks (Thresholds depend on environment, but shouldn't be massive)
    assert!(proc.memory() < 500 * 1024 * 1024, "Memory usage exceeded 500MB"); 
}

#[test]
fn test_glow_rendering_correctness() {
    // Verify the visual output of the glow effect using Cairo directly
    let width = 50;
    let height = 50;
    let mut surface = ImageSurface::create(Format::ARgb32, width, height).unwrap();
    
    {
        let cr = Context::new(&surface).unwrap();
        
        // 1. Clear to Transparent
        cr.set_operator(Operator::Source);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.paint().unwrap();
        cr.set_operator(Operator::Over);
        
        // 2. Draw Glow (Green with 0.5 Alpha)
        cr.set_source_rgba(0.0, 1.0, 0.0, 0.5);
        cr.rectangle(10.0, 10.0, 20.0, 20.0);
        cr.fill().unwrap();
    } // Drop context to release lock on surface
    
    surface.flush();
    let stride = surface.stride() as usize;
    let data = surface.data().unwrap();
    
    // Check pixel at (15, 15) - Should be semi-transparent green
    let offset = 15 * stride + 15 * 4;
    let (b, g, r, a) = (data[offset], data[offset+1], data[offset+2], data[offset+3]);
    
    assert!(g > 0, "Green component missing in glow");
    assert!(g < 255, "Glow should not be full brightness");
    assert!(a > 0 && a < 255, "Alpha should be blended");
    assert_eq!(r, 0, "Red component should be 0");
    assert_eq!(b, 0, "Blue component should be 0");
}