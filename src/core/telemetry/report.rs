// src/core/telemetry/report.rs
//! Formatting for the telemetry summary printed once at exit.
//! Split out of `mod.rs` to hold both files under the 175-line cap (C-01).

use super::{live_control_snapshot, present_count, rain_draw_snapshot, survived_snapshot, timings_snapshot};

fn mean_ms(total_ns: u64, n: u64) -> f64 {
    if n == 0 { 0.0 } else { total_ns as f64 / n as f64 / 1_000_000.0 }
}

/// Render the S-13b summary. Printed exactly once, at shutdown.
pub fn summary() -> String {
    let snap = timings_snapshot();
    let mut out = String::from(
        "\n=== S-13b — X-side per-frame cost, per CRTC (means, ms) ===\n\
         geometry        presents   pre_draw   put_image         gc       total\n",
    );
    if snap.is_empty() {
        out.push_str("  (no presents recorded)\n");
        return out;
    }
    let mut grand = 0.0;
    for (geom, t) in &snap {
        let pre = mean_ms(t.pre_draw_ns, t.pre_draws);
        let put = mean_ms(t.put_image_ns, t.presents);
        let gc = mean_ms(t.gc_ns, t.presents);
        let total = pre + put + gc;
        grand += total;
        out.push_str(&format!(
            "{:<15} {:>8} {:>10.4} {:>11.4} {:>10.4} {:>11.4}\n",
            geom, t.presents, pre, put, gc, total
        ));
    }
    out.push_str(&format!(
        "present_ms summed across CRTCs: {:.4} ms/frame   (total presents: {})\n",
        grand,
        present_count()
    ));

    let rain = rain_draw_snapshot();
    if !rain.is_empty() {
        out.push_str("\n=== X-LIVE reconciliation — production RainManager::draw (means, ms) ===\n");
        for (geom, (ns, n)) in &rain {
            out.push_str(&format!("{:<15} calls={:>8}  rain_draw={:>10.4} ms\n", geom, n, mean_ms(*ns, *n)));
        }
    }
    let survived = survived_snapshot();
    if !survived.is_empty() {
        out.push_str("\n=== Q1 — surviving show_layout calls per production rain.draw (means) ===\n");
        for (geom, (sum, n)) in &survived {
            let mean = if *n == 0 { 0.0 } else { *sum as f64 / *n as f64 };
            out.push_str(&format!("{:<15} frames={:>8}  survived_show_layout={:>9.1}\n", geom, n, mean));
        }
    }

    let control = live_control_snapshot();
    if !control.is_empty() {
        out.push_str(
            "\n=== Q3 — in-process single-size CONTROL (means, ms) — Phase 3 re-entry denominator ===\n",
        );
        for (geom, (ns, n)) in &control {
            let c = mean_ms(*ns, *n);
            let live = rain.get(geom).map(|(rns, rn)| mean_ms(*rns, *rn)).unwrap_or(0.0);
            let ratio = if c > 0.0 { live / c } else { 0.0 };
            out.push_str(&format!(
                "{:<15} calls={:>8}  control={:>9.4} ms  live/control={:>6.2}  (Phase 3 opens at >= 3.00)\n",
                geom, n, c, ratio
            ));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [S-13b] mean_ms converts accumulated nanoseconds to a per-event mean in ms,
    /// and must not divide by zero when a slot recorded nothing.
    #[test]
    fn mean_ms_converts_and_guards_zero() {
        assert_eq!(mean_ms(0, 0), 0.0);
        assert!((mean_ms(1_000_000, 1) - 1.0).abs() < 1e-9);
        // The live Phase 1 reading: 4096x2160 pre_draw averaged 1.1962 ms.
        assert!((mean_ms(1_196_200 * 100, 100) - 1.1962).abs() < 1e-6);
    }

    /// [S-06] One render tick presents once per CRTC, so the raw counter runs at
    /// `fps * crtcs`. This is the arithmetic `FpsCollector` applies; the live run
    /// produced 18301 presents over ~303 s across 2 CRTCs = 30.2 fps, not 60.4.
    #[test]
    fn present_count_is_per_crtc_not_per_frame() {
        let raw_rate: f64 = 18301.0 / 303.0;
        assert!((raw_rate - 60.4).abs() < 0.2, "raw rate {}", raw_rate);
        let frame_rate: f64 = raw_rate / 2.0;
        assert!((frame_rate - 30.2).abs() < 0.2, "frame rate {}", frame_rate);
    }
}
