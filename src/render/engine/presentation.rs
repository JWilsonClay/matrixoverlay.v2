// src/render/engine/presentation.rs
use anyhow::Result;
use xcb::x;
use super::renderer::Renderer;

impl Renderer {
    pub fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()> {
        self.surface.flush();
        // get_maximum_request_length() accounts for the BigRequests extension;
        // units are 4-byte words, so multiply by 4 to get bytes.
        let max_bytes = conn.get_maximum_request_length() as usize * 4;
        // stride() must be called before data() — data() takes a mutable borrow.
        let stride = self.surface.stride() as usize;
        let data = self.surface.data().map_err(|e| anyhow::anyhow!("Cairo data access failed: {}", e))?;
        // PutImage wire header: 28 bytes (7 four-byte words). Subtract from budget.
        let stripe_height = (max_bytes.saturating_sub(28) / stride).max(1);
        let gc: x::Gcontext = conn.generate_id();
        conn.send_request(&x::CreateGc { cid: gc, drawable: x::Drawable::Window(window), value_list: &[] });
        let height = self.height as usize;
        let mut row = 0usize;
        while row < height {
            let rows_this_stripe = (row + stripe_height).min(height) - row;
            let byte_start = row * stride;
            let byte_end = byte_start + rows_this_stripe * stride;
            conn.send_request(&x::PutImage {
                format: x::ImageFormat::ZPixmap, drawable: x::Drawable::Window(window), gc,
                width: self.width as u16, height: rows_this_stripe as u16,
                dst_x: 0, dst_y: row as i16, left_pad: 0, depth: 32,
                data: &data[byte_start..byte_end],
            });
            row += stripe_height;
        }
        conn.send_request(&x::FreeGc { gc });
        Ok(())
    }
}
