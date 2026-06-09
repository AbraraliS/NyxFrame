pub mod wayland;
pub mod x11;

use std::io;
use crate::platforms::linux::env::EnvState;

pub enum ScreenCapturer {
    Wayland(wayland::WaylandCapturer),
    X11(x11::X11Capturer),
}

impl ScreenCapturer {
    pub fn auto_select(env: &EnvState) -> io::Result<Self> {
        let mut is_wayland = env.xdg_session_type.as_deref() == Some("wayland")
            || env.wayland_display.is_some();

        // If sudo stripped the env vars, we might still be on Wayland.
        // Probe for Wayland sockets in common runtime directories.
        if !is_wayland {
            for uid in 999u32..=1005 {
                let rtdir = format!("/run/user/{}", uid);
                if std::path::Path::new(&format!("{}/wayland-0", rtdir)).exists()
                    || std::path::Path::new(&format!("{}/wayland-1", rtdir)).exists()
                {
                    is_wayland = true;
                    break;
                }
            }
        }

        if is_wayland {
            log::info!("Detected Wayland session. Attempting PipeWire portal capture...");
            match wayland::WaylandCapturer::new() {
                Ok(capturer) => {
                    let _ = std::fs::write("/tmp/capturer_used.txt", "Wayland");
                    return Ok(ScreenCapturer::Wayland(capturer));
                }
                Err(e) => {
                    log::warn!("Wayland capture init failed: {}", e);
                    log::warn!("Falling back to X11 capturer (which will capture the Xwayland root window)...");
                }
            }
        } else {
            log::info!("Detected X11 session.");
        }

        log::info!("Attempting X11 capture...");
        let _ = std::fs::write("/tmp/capturer_used.txt", "X11");
        let capturer = x11::X11Capturer::new()?;
        Ok(ScreenCapturer::X11(capturer))
    }

    pub fn capture_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        match self {
            ScreenCapturer::Wayland(c) => c.capture_frame(),
            ScreenCapturer::X11(c) => c.capture_frame(),
        }
    }

    pub fn width(&self) -> u32 {
        match self {
            ScreenCapturer::Wayland(c) => c.width(),
            ScreenCapturer::X11(c) => c.width(),
        }
    }

    pub fn height(&self) -> u32 {
        match self {
            ScreenCapturer::Wayland(c) => c.height(),
            ScreenCapturer::X11(c) => c.height(),
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            ScreenCapturer::Wayland(_) => "Wayland (PipeWire)",
            ScreenCapturer::X11(_) => "X11 (XCB)",
        }
    }
}