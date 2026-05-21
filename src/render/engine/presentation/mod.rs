pub mod socket;

use anyhow::Result;
use cairo::ImageSurface;
use xcb::x;

pub trait Presenter {
    fn surface(&self) -> &ImageSurface;
    fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()>;
    // Placeholder — no callers exist yet; both impls return Ok(()) until a resize path is needed.
    fn resize(&mut self, w: u16, h: u16) -> Result<()>;
}

pub fn create_presenter(w: u16, h: u16) -> Result<Box<dyn Presenter>> {
    Ok(Box::new(socket::SocketPresenter::new(w, h)?))
}
