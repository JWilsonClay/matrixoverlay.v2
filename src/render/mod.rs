//! Matrix Overlay Rendering Substrate.
pub mod physics;
pub mod layout;
pub mod engine;

pub use self::engine::renderer::Renderer;

/// Render one Cairo context's effective font options as a comparable string.
///
/// [2.9 probe E2, round-8] Antialias / hint style / hint metrics / subpixel
/// order are part of the scaled-font cache key **and** of the rasterization
/// cost, so a difference here is a candidate mechanism for the cargo-test vs
/// overlay-process divergence. Diagnostic only; nothing reads it at runtime.
/// Takes a `Context` rather than a `Surface` because `cairo_get_font_options`
/// is the context-level call, and it returns the surface defaults merged with
/// anything the context set — which is what the text path actually rasterizes
/// with.
pub fn describe_font_options(cr: &cairo::Context) -> String {
    match cr.font_options() {
        Ok(o) => format!(
            "antialias={:?} hint_style={:?} hint_metrics={:?} subpixel_order={:?}",
            o.antialias(), o.hint_style(), o.hint_metrics(), o.subpixel_order()
        ),
        Err(e) => format!("unavailable: {e}"),
    }
}
