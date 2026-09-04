# Hardware Notes & Pitfalls: Dell G15 5515

## 1. Hardware Specifics
**Target System**: Dell G15 5515 (Ryzen 7 5800H + RTX 3050 Ti Mobile + AMD iGPU)

### CPU: AMD Ryzen 7 5800H
- **Driver**: `k10temp` kernel module.
  - `temp1_input` (Tctl): Instantaneous temperature, erratic.
  - `temp2_input` (Tdie): Die temperature, generally more stable.
- **Fans**: Dell laptops often hide fan control/sensing from standard `hwmon`. Requires `dell-smm-hwmon` module, but often conflicts with BIOS fan control. If `/sys/class/hwmon/hwmon*/fan1_input` is missing, fan stats may be unavailable without `i8kutils`.

### GPU 1: AMD Radeon Graphics (Integrated)
- **Driver**: `amdgpu`
- **Role**: Handles X11 desktop composition (Mutter) in hybrid mode.
- **Hwmon**: Search for `amdgpu` in `name`.

### GPU 2: NVIDIA GeForce RTX 3050 Ti Mobile
- **Driver**: Proprietary NVIDIA (e.g., 535.x, 550.x).
- **Querying**:
  - **NVML**: Preferred via `nvml-wrapper` crate.
  - **CLI**: `nvidia-smi --query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total --format=csv,noheader,nounits`
- **Hybrid Mode Pitfall**: In "On-Demand" mode, the dGPU sleeps (D3 state). Querying it via `nvidia-smi` can wake it up, causing system-wide micro-stutters. `nvml` is generally lighter but still may incur wake-up latency.

## X11 & Mutter Pitfalls

### Layering & Input
- **Layering**: To sit *below* desktop icons (handled by DING/Nautilus) but *above* wallpaper:
  - Atom: `_NET_WM_WINDOW_TYPE_DESKTOP`
  - State: `_NET_WM_STATE_BELOW`
- **Click-Through**: Essential for a desktop overlay.
  - **Mechanism**: XShape extension (`xcb_shape_rectangles` or `x11rb::protocol::shape`). Set the Input region to an empty list of rectangles.
  - **Verification**: `xprop -id <window_id>` should show `_NET_WM_WINDOW_TYPE(ATOM) = _NET_WM_WINDOW_TYPE_DESKTOP`.

### OverrideRedirect Pitfall (CONFIRMED BUG — 2026-05-21)
**Never set `OverrideRedirect(true)` on the overlay window.** This bypasses the window manager entirely. All EWMH atoms (`_NET_WM_WINDOW_TYPE_DESKTOP`, `_NET_WM_STATE_BELOW`) are silently ignored, and the window stacks above everything — including application windows. The overlay appears "in front of everything" instead of on the desktop. The correct value is `OverrideRedirect(false)`.

### Operator::Clear Pitfall (CONFIRMED BUG — 2026-05-21)
**Do not use `Operator::Clear` in the `clear()` rendering step.** It sets the background to fully transparent (ARGB 0,0,0,0), removing the black backing. The correct operator is `Operator::Source` with `set_source_rgba(0.0, 0.0, 0.0, 1.0)`. These two regressions were introduced by a prior AI session and are subtle — the overlay can still render metrics correctly while appearing broken in layering and background.

## MIT-SHM Pitfalls

### SHM Race Condition: Cairo Writes Directly Into SHM (CONFIRMED BUG — 2026-05-21)
When using `ImageSurface::create_for_data(ShmBuffer, ...)`, Cairo paints **directly into the SHM region** — there is no copy. The X server reads from the same region asynchronously after `ShmPutImage`. If the next frame's `clear()` fires before the X server has finished reading, it overwrites the buffer with black mid-read. The X server displays a corrupted frame (flash to black, partial content, or rain stutter).

**Fix**: Before Cairo touches the buffer (before `clear()`), issue a synchronous round-trip to confirm the X server has finished:
```rust
// In Presenter::pre_draw() for ShmPresenter:
let cookie = conn.send_request(&x::GetInputFocus {});
conn.wait_for_reply(cookie)?;
```
By the time the reply arrives, the X server has sequentially processed all prior requests including the `ShmPutImage`. The SHM region is safe to overwrite. Round-trip overhead on a local X socket is <1ms.

### SHM Drop Ordering (CRITICAL)
The Cairo `ImageSurface` holds a live pointer into the SHM region. `shmdt()` must never be called while the surface is alive. In `Drop`:
1. `self.surface.take()` — drop Cairo surface first
2. `conn.send_request(&xcb::shm::Detach { ... })` — unregister from X server
3. `libc::shmdt(self.shmaddr)` — unmap local address range

Store the surface as `Option<ImageSurface>` to enable `take()` in `Drop`.

### SHM PutImage: format is u8, not ImageFormat
`xcb::shm::PutImage::format` is typed as `u8`. Use `2u8` for ZPixmap. The `x::ImageFormat::ZPixmap` enum variant does **not** coerce here — it will not compile.

### Hybrid Graphics (Prime)
- **Flicker**: Rendering to an X11 window on the dGPU while the iGPU handles composition can cause tearing.
- **Mitigation**: Ensure the overlay window is created on the screen/CRTC driven by the compositor. Use software double-buffering (Cairo ImageSurface -> X11 Pixmap -> Window) to decouple rendering from display scanout.

### Animations & ASD Considerations
- **ASD-Friendly**:
  - **High Contrast**: Use bright green/white on semi-transparent black.
  - **Stability**: No flashing or blinking elements.
  - **Refresh Rate**: 1Hz or 0.5Hz is sufficient. Avoid 60fps animations to prevent distraction and battery drain.
  - **Scroll**: If text overflows, slow scrolling is acceptable, but static text is preferred.

## Implementation Plan

### Starter `main.rs` Skeleton
The application structure ties together configuration, metrics collection, and the X11 event loop.

```rust
fn main() -> Result<()> {
    // 1. Load Config
    let config = Config::load()?;

    // 2. Setup Channels
    let (tx, rx) = crossbeam_channel::unbounded();

    // 3. Spawn Metrics Thread
    // Runs in background, sleeps for config.refresh_rate_ms
    metrics::spawn_metrics_thread(config.metrics, tx);

    // 4. Initialize X11 Window Manager
    // Connects to X server, detects monitors, creates transparent windows
    let mut wm = WindowManager::new()?;

    // 5. Initialize Renderer
    // Creates Cairo surfaces
    let mut renderer = Renderer::new(wm.width, wm.height)?;

    // 6. Event Loop
    loop {
        // Non-blocking check for X11 events (resize, expose)
        while let Some(event) = wm.poll_event() {
            handle_x11_event(event, &mut wm, &mut renderer);
        }

        // Check for new metrics
        if let Ok(data) = rx.try_recv() {
            let text = format_metrics(&data);
            renderer.draw(&wm, &text)?;
        }

        // Sleep briefly to avoid busy loop (e.g., 100ms)
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

---

## Pitfall: a `cargo test` benchmark is not a measurement of this program

*(Added 2026-09-03, Render Substrate Remediation campaign, round-8 finding. Stub for the Phase 10
write-up; the numbers are already final.)*

Pango font-size churn — cycling a distinct `FontDescription` size per rain stream, every frame —
costs **74×** inside `cargo test` and **1.25×** inside the overlay process. Same function
(`RainManager::draw`), same geometry, same config literals, same glyph volume:

| | per glyph | vs its own single-size control |
|---|---|---|
| `cargo test --release` | 438.66 µs | 74× |
| overlay process (in-process timing) | 7.42 µs | 1.25× |

**What it is not.** Ruled out by measurement, not by argument: glyph volume (surviving `show_layout`
counts match within 6% — 1380.8 vs 1297.0), the clip guard, `rain_speed` priming (pinning the live
0.1 moved the number 1.1%), per-frame surface creation, glyph-cache warm-up, unit errors, and
cross-test cache pollution. Cairo font options are **identical** on both sides
(`antialias/hint_style/hint_metrics/subpixel_order` all `Default`). Calling `gtk::init()` in the test
moves it about 18% — real, but not the mechanism.

**Leading remaining mechanism:** the GTK / `PangoCairoFontMap` / Xft font-map state the overlay
process holds versus the bare font map a test binary gets. Not SHM-versus-`ImageSurface`.

**The rule this buys you.** A performance test that calls production code with production-shaped
inputs can still be measuring a different program, because *the process is an input too*. Before
acting on a benchmark, reconcile it against the same function timed inside the running binary. The
campaign's MRC satisfied every anti-Mock-Trap rule that was written down and still disagreed with the
substrate by 59× per glyph. **The in-process measurement is the one that decides.**

---

## Pitfall: a performance test that checks one step will pass a broken implementation

*(Added 2026-09-04, Render Substrate Remediation campaign, round-9 standing rule.)*

**A performance assertion must test behaviour under load — N events and the achieved rate — never a
property of a single step.** Verify it by reinstating the defect and watching the test go red before
you accept it green.

Three recurrences in one campaign, all the same shape:

1. **Phase 1.** A test for the `overlay_cpu` normalization defined a *local copy* of the production
   expression and asserted against that. It would have passed with the 16× bug reinstated.
2. **Phase 2.** The replacement MRC called production `RainManager::draw` — satisfying the written
   anti-Mock-Trap rule — but primed it from `Config::default()`, whose `rain_speed` is 10× the live
   value. Production-shaped was asserted, not verified.
3. **Phase 5.** The S-07 governor test asserted that the next tick lands *after* the slow frame and
   *within one period* of it. Both are true of a `next_deadline` that returns `now + 1ms` — which is
   exactly the fail-open behaviour the test existed to forbid. It passed. Rewritten to drive 20
   overrunning frames and assert the achieved rate, it fails the broken version with
   `20 frames took 4.8s, under the 19s the rate allows`.

Each was caught by reinstating the defect, never by reading the test. Write the red-check into the
procedure, not into good intentions.

---

## Pitfall: tightening a range into a point gate before you have measured anything

*(Added 2026-09-04, Render Substrate Remediation campaign, round-10.)*

`concept.md` §III asked for **"< 1–3%"** of one core. The remediation campaign wrote that down as a
point gate of **3.0%** — before it knew the frame rate (inferred 1.3, actually 30.2), before it knew
the rain draw cost (inferred ~750 ms/frame, actually 10 ms), and before it knew a frame-rate-independent
floor existed at all (0.5368%).

The finished work measures **3.0166 / 2.9966 / 2.9966** across three 300 s windows — mean **3.0033**,
spread **0.020 pp**. The gate sits inside the instrument's own noise. One run is over it, two are
under, and which you quote decides whether the campaign "passed."

Two failure modes open up here and both are wrong:

- **Select the two low runs** and declare victory. That is the number choosing the conclusion.
- **Spend architecture to buy 0.17–0.37%** against a 0.020 pp band. That is Context Erosion — rigor
  decaying into motion, because a gate is easier to chase than to re-examine.

The correct move is to **publish the series, state the verdict as "at gate", and write the exception
against the original requirement** — which in this case was a *range* whose top the result meets. A
point gate derived from a range is an assumption, and assumptions get audited like everything else.

---

## CONFIRMED BUG: Pango font-cache eviction under size churn — *in a test process*

*(Added 2026-09-04. Lab-confirmed, live-refuted. Read both halves.)*

**The mechanism.** Cycling N distinct `FontDescription` **sizes** through a single `pango::Layout`,
every frame, evicts each scaled font before it is reused. `RainManager::draw` does exactly this: one
`desc.set_size(size * s.depth * SCALE)` per stream, ~162 distinct sizes per frame at 4096×2160 with
`realism = 4`, because `s.depth` is a continuous `f64` in `0.5..1.2`.

**The cost lands on the first `show_layout` after the size change, not on `set_font_description`.**
Timing the setter shows nothing; the rasterization is where the miss is paid.

**Two measurements that disagree, and both are true:**

| | cost | vs its own single-size control |
|---|---|---|
| `cargo test --release` (`LAB_F1`) | 605 ms/frame, 438.66 µs/glyph | **74×** |
| the overlay process (in-process) | 9.62 ms/frame, 7.42 µs/glyph | **1.25×** |

**The lab reproduces F1. The live process does not.** Glyph volume is identical (1380.8 vs 1297.0
surviving `show_layout` calls, within 6%), so it is not the clip guard, not `rain_speed`, not warm-up,
and not cross-test pollution. Cairo font options are byte-identical on both sides. `gtk::init()` in
the test recovers ~18% and no more. The leading remaining mechanism is the GTK / `PangoCairoFontMap` /
Xft font-map state the overlay holds versus the bare font map a test binary gets.

**Consequence:** the planned fix (bucket `s.depth` into N discrete sizes, then a glyph atlas) was
**demoted**. It would attack a cost the live process does not pay. `s.depth` stays continuous. The
re-entry criterion, if anyone wants to reopen it, is `live_rain_draw / live_single_size_control ≥ 3`,
both measured **in-process** — it is 1.25.

---

## CONFIRMED BUG: the Mock Trap that guarded this code for months

*(Added 2026-09-04.)*

`tests/performance_tests.rs::test_render_optimization_bench` asserted `< 500 ms` for 50,000 glyphs
rendered through **one** `pango::Layout` at **one** font size, and commented itself as proof that
"with caching, we can render 50k glyphs in milliseconds." Production cycled ~162 sizes per frame. The
test passed continuously while the code it claimed to cover ran roughly **90× slower**. Deleted.

**The rule:** a performance assertion calls production code with production-shaped inputs, or it is
labeled a **control** and may never be cited as validation.

Four more instruments in the same tree were found broken in the same audit:

- `test_stability_no_flicker` asserted `general.update_ms >= 500` — the *metrics collector* period —
  while the render tick was hard-coded 33 ms. C-05 was green against a clock production never used.
- `test_layout_predictability` shipped with **every assertion commented out**. It could not fail.
- `tests/window_integration.rs` asserts 1920×1080 at (0,0) beneath a comment claiming the geometry is
  hardcoded. It is not — RandR yields 4096×2160 + 1920×1080 on this host (R-11).
- `tests/metrics_tests.rs` **has never compiled** — it calls
  `NvidiaSmiCollector::new_with_command`, which does not exist (MT-3).

### The standing rule, and its four recurrences

**A performance AC asserts behaviour under load — N events and the achieved rate — never a property
of one step, and never against a local copy of the production expression.** Verify by reinstating the
defect and watching the test go red.

1. **Phase 1** — the `overlay_cpu` test defined a *local copy* of the production normalization and
   asserted against that. It would have passed with the 16× bug reinstated.
2. **Phase 2** — the replacement MRC called production `RainManager::draw` but primed it from
   `Config::default()`, whose `rain_speed` is 10× the live value.
3. **Phase 5** — the S-07 governor test asserted the next tick lands after the slow frame and within
   one period. Both hold for a `next_deadline` returning `now + 1ms` — the exact fail-open it forbade.
   It passed.
4. **Phase 8** — the preset verification script re-derived the preset table in Python and asserted
   against the copy. Caught before it ran.

Every one was caught by reinstating the defect. None was caught by reading the test.

---

## Two more traps this codebase set

*(Added 2026-09-04.)*

**A config field in a user's live `config.json` cannot simply be deleted from the struct.** Every
config struct carries `#[serde(deny_unknown_fields)]`. Removing a field the user's file still contains
makes that file **fail to load** — not degrade, fail. Two Ghost Logic flags (`show_monitor_label`,
`build_logging_enabled`) were therefore *wired* rather than deleted: "wire or delete" had only one
safe branch. Deleting a config field is a migration, not a deletion.

**`src/core/timer.rs` was a second, dead copy of the metrics loop — carrying the same bug.** It held
the identical `else { thread::sleep(Duration::from_millis(1)); }` fail-open that the tick thread had
(F4), and it had **no callers**; `src/metrics/factory.rs` existed solely to serve it. Both deleted. Had
the F4 hunt started here, this file would have looked like the fix site and fixing it would have
changed nothing — an argument for measuring the running process before reading the source.

**F8 — a debugging override that outlived its debugging session by seven months.**
`src/core/main.rs` overwrote `config.cosmetics.rain_mode` with `"fall"` immediately after
`Config::load()`, making every other mode unreachable. It entered in `d2f61a1` (2026-02-28) commented
*"FORCE OVERRIDE: Ensure rain is enabled for verification"*. Its companion from the same commit and
comment (`realism_scale = 8`) was cleaned up; this one was not.
