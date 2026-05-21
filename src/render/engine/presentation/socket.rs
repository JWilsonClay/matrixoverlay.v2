use anyhow::Result;
use cairo::{Format, ImageSurface};
use xcb::x;
use super::Presenter;

pub struct SocketPresenter {
    surface: ImageSurface,
}

impl SocketPresenter {
    pub fn new(w: u16, h: u16) -> Result<Self> {
        let surface = ImageSurface::create(Format::ARgb32, w as i32, h as i32)
            .map_err(|e| anyhow::anyhow!("Failed to create ImageSurface: {}", e))?;
        Ok(Self { surface })
    }
}

impl Presenter for SocketPresenter {
    fn surface(&self) -> &ImageSurface {
        &self.surface
    }

    fn pre_draw(&mut self, _conn: &xcb::Connection) -> Result<()> {
        Ok(())
    }

    fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()> {
        self.surface.flush();
        // get_maximum_request_length() accounts for the BigRequests extension;
        // units are 4-byte words, so multiply by 4 to get bytes.
        let max_bytes = conn.get_maximum_request_length() as usize * 4;
        // stride(), height(), width() must all be called before data() — data() takes a mutable borrow.
        let stride = self.surface.stride() as usize;
        let height = self.surface.height() as usize;
        let width = self.surface.width() as u16;
        let data = self.surface.data()
            .map_err(|e| anyhow::anyhow!("Cairo data access failed: {}", e))?;
        // PutImage wire header: 28 bytes (7 four-byte words). Subtract from budget.
        let stripe_height = (max_bytes.saturating_sub(28) / stride).max(1);
        let gc: x::Gcontext = conn.generate_id();
        conn.send_request(&x::CreateGc {
            cid: gc,
            drawable: x::Drawable::Window(window),
            value_list: &[],
        });
        let mut row = 0usize;
        while row < height {
            let rows_this_stripe = (row + stripe_height).min(height) - row;
            let byte_start = row * stride;
            let byte_end = byte_start + rows_this_stripe * stride;
            conn.send_request(&x::PutImage {
                format: x::ImageFormat::ZPixmap,
                drawable: x::Drawable::Window(window),
                gc,
                width,
                height: rows_this_stripe as u16,
                dst_x: 0,
                dst_y: row as i16,
                left_pad: 0,
                depth: 32,
                data: &data[byte_start..byte_end],
            });
            row += stripe_height;
        }
        conn.send_request(&x::FreeGc { gc });
        Ok(())
    }

    fn resize(&mut self, _w: u16, _h: u16) -> Result<()> {
        Ok(())
    }
}
