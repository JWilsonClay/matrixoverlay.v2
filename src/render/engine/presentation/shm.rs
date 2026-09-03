use std::sync::Arc;
use anyhow::Result;
use cairo::{Format, ImageSurface};
use xcb::x;
use super::Presenter;
use crate::core::telemetry;

// Wraps the raw SHM pointer so that ImageSurface::create_for_data can accept it.
// No Drop impl — libc::shmdt in ShmPresenter::drop owns this memory. Adding a
// Drop that calls shmdt would double-unmap when the surface is eventually dropped.
struct ShmBuffer {
    ptr: *mut u8,
    len: usize,
}
// Safety: SHM memory is valid for the lifetime of ShmPresenter, which lives
// entirely within the single overlay thread. The Send impl is required by
// cairo's ImageSurface::create_for_data bound; we uphold it by ensuring
// ShmPresenter is never moved across thread boundaries.
unsafe impl Send for ShmBuffer {}
impl AsMut<[u8]> for ShmBuffer {
    fn as_mut(&mut self) -> &mut [u8] {
        // Safety: ptr is non-null, aligned, and `len` bytes are accessible until
        // ShmPresenter::drop calls shmdt — which only happens after the surface
        // referencing this buffer has been explicitly dropped first.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

pub struct ShmPresenter {
    // Option so that drop() can take() the surface before calling shmdt.
    // It is Some for the entire lifetime of the presenter outside of drop().
    surface: Option<ImageSurface>,
    shmseg: xcb::shm::Seg,
    shmaddr: *mut libc::c_void,
    // Stored so that Drop can call shm::Detach without receiving conn as a parameter.
    conn: Arc<xcb::Connection>,
    width: u16,
    height: u16,
    // True after a PutImage has been sent but before the X server has confirmed
    // it is done reading from the SHM region. Cleared in pre_draw() via sync.
    frame_pending: bool,
}

impl ShmPresenter {
    pub fn new(conn: &Arc<xcb::Connection>, w: u16, h: u16) -> Result<Self> {
        let len = w as usize * h as usize * 4;

        // Allocate a System V IPC shared memory segment.
        let shmid = unsafe {
            libc::shmget(libc::IPC_PRIVATE, len, libc::IPC_CREAT | 0o600)
        };
        if shmid == -1 {
            anyhow::bail!("shmget failed: {}", std::io::Error::last_os_error());
        }

        // Attach the segment to this process's address space.
        let shmaddr = unsafe { libc::shmat(shmid, std::ptr::null(), 0) };
        if shmaddr as isize == -1isize {
            // Clean up the segment we just created before bailing.
            unsafe { libc::shmctl(shmid, libc::IPC_RMID, std::ptr::null_mut()); }
            anyhow::bail!("shmat failed: {}", std::io::Error::last_os_error());
        }

        // Mark the segment for auto-destruction when the last attachment is
        // released. This covers process-crash cleanup at the OS level — the
        // kernel reclaims the segment even if we never reach our Drop impl.
        unsafe { libc::shmctl(shmid, libc::IPC_RMID, std::ptr::null_mut()); }

        // stride = width * 4: freshly-allocated SHM is contiguous with no row padding.
        let stride = w as i32 * 4;
        let buf = ShmBuffer { ptr: shmaddr as *mut u8, len };
        let surface = ImageSurface::create_for_data(buf, Format::ARgb32, w as i32, h as i32, stride)
            .map_err(|e| anyhow::anyhow!("Failed to create SHM ImageSurface: {:?}", e))?;

        // Register the segment with the X server.
        let shmseg: xcb::shm::Seg = conn.generate_id();
        conn.send_request(&xcb::shm::Attach {
            shmseg,
            shmid: shmid as u32,
            read_only: false,
        });
        let _ = conn.flush();

        Ok(Self {
            surface: Some(surface),
            shmseg,
            shmaddr,
            conn: Arc::clone(conn),
            width: w,
            height: h,
            frame_pending: false,
        })
    }
}

impl Presenter for ShmPresenter {
    fn surface(&self) -> &ImageSurface {
        // surface is always Some during normal operation; only None inside drop().
        self.surface.as_ref().expect("ShmPresenter surface accessed after drop")
    }

    fn pre_draw(&mut self, conn: &xcb::Connection) -> Result<()> {
        let mut lap = telemetry::Lap::new();
        if self.frame_pending {
            // Force a synchronous round-trip. By the time the X server replies to
            // GetInputFocus it has sequentially processed all prior requests,
            // including the ShmPutImage. The SHM region is safe to overwrite.
            let cookie = conn.send_request(&x::GetInputFocus {});
            conn.wait_for_reply(cookie)
                .map_err(|e| anyhow::anyhow!("SHM sync failed: {:?}", e))?;
            self.frame_pending = false;
            telemetry::record_pre_draw(self.width, self.height, lap.lap());
        }
        Ok(())
    }

    fn present(&mut self, conn: &xcb::Connection, window: x::Window) -> Result<()> {
        // flush() commits any pending Cairo drawing operations to the SHM buffer.
        if let Some(ref surface) = self.surface {
            surface.flush();
        }
        let mut lap = telemetry::Lap::new();
        let gc: x::Gcontext = conn.generate_id();
        conn.send_request(&x::CreateGc {
            cid: gc,
            drawable: x::Drawable::Window(window),
            value_list: &[],
        });
        let gc_create_ns = lap.lap();
        // PutImage over SHM: the X server reads directly from the shared memory
        // segment — no pixel data travels over the socket.
        conn.send_request(&xcb::shm::PutImage {
            drawable: x::Drawable::Window(window),
            gc,
            total_width: self.width,
            total_height: self.height,
            src_x: 0,
            src_y: 0,
            src_width: self.width,
            src_height: self.height,
            dst_x: 0,
            dst_y: 0,
            depth: 32,
            format: 2, // ZPixmap
            send_event: false,
            shmseg: self.shmseg,
            offset: 0,
        });
        let put_image_ns = lap.lap();
        conn.send_request(&x::FreeGc { gc });
        let _ = conn.flush();
        telemetry::record_present(self.width, self.height, put_image_ns, gc_create_ns + lap.lap());
        self.frame_pending = true;
        Ok(())
    }

    fn resize(&mut self, _w: u16, _h: u16) -> Result<()> {
        Ok(())
    }
}

impl Drop for ShmPresenter {
    fn drop(&mut self) {
        // Step 1: drop the Cairo surface FIRST. It holds a live pointer into the
        // SHM region; calling shmdt while the surface is alive is undefined behavior.
        drop(self.surface.take());
        // Step 2: unregister the segment from the X server.
        self.conn.send_request(&xcb::shm::Detach { shmseg: self.shmseg });
        let _ = self.conn.flush();
        // Step 3: unmap the local process virtual address range.
        // Safe only after step 1 has completed.
        unsafe { libc::shmdt(self.shmaddr); }
    }
}
