// src/render/engine/presentation.rs
use anyhow::Result;
use xcb::x;
use super::renderer::Renderer;

impl Renderer {
    pub fn present(&self, conn: &xcb::Connection, window: x::Window) -> Result<()> {
        self.surface.flush();
        let data = self.surface.data().map_err(|e| anyhow::anyhow!("Cairo data access failed: {}", e))?;
        let gc: x::Gcontext = conn.generate_id();
        conn.send_request(&x::CreateGc { cid: gc, drawable: x::Drawable::Window(window), value_list: &[] });
        conn.send_request(&x::PutImage {
            format: x::ImageFormat::ZPixmap, drawable: x::Drawable::Window(window), gc,
            width: self.width as u16, height: self.height as u16,
            dst_x: 0, dst_y: 0, left_pad: 0, depth: 32, data: &data,
        });
        conn.send_request(&x::FreeGc { gc });
        Ok(())
    }
}
