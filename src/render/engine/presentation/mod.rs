pub mod socket;
pub mod shm;

use std::sync::Arc;
use anyhow::Result;
use cairo::ImageSurface;
use xcb::x;

pub trait Presenter {
    fn surface(&self) -> &ImageSurface;
    // Called at the start of each draw cycle, before Cairo touches the buffer.
    // ShmPresenter uses this to block until the X server has finished reading
    // the previous frame's SHM region; SocketPresenter is a no-op.
    fn pre_draw(&mut self, conn: &xcb::Connection) -> Result<()>;
    fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()>;
    fn resize(&mut self, w: u16, h: u16) -> Result<()>;
}

pub fn create_presenter(conn: &Arc<xcb::Connection>, w: u16, h: u16) -> Result<Box<dyn Presenter>> {
    let shm_cookie = conn.send_request(&xcb::shm::QueryVersion {});
    match conn.wait_for_reply(shm_cookie) {
        Ok(_) => {
            match shm::ShmPresenter::new(conn, w, h) {
                Ok(p) => {
                    log::info!("[Presenter] MIT-SHM available — using zero-copy ShmPresenter");
                    Ok(Box::new(p))
                }
                Err(e) => {
                    log::warn!("[Presenter] ShmPresenter init failed ({}), falling back to SocketPresenter", e);
                    Ok(Box::new(socket::SocketPresenter::new(w, h)?))
                }
            }
        }
        Err(_) => {
            log::info!("[Presenter] MIT-SHM not available — using SocketPresenter");
            Ok(Box::new(socket::SocketPresenter::new(w, h)?))
        }
    }
}
