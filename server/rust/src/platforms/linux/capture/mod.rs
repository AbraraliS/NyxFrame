// NyxFrame Capture Engine
// Supports: Linux X11 (XCB MIT-SHM), Linux Wayland (GStreamer/PipeWire)

pub mod x11;
pub mod wayland;

#[cfg(target_os = "linux")]
pub use self::x11::X11Capturer;
#[cfg(target_os = "linux")]
pub use self::wayland::WaylandCapturer;

use std::io;
use crate::platforms::linux::env::EnvState;

pub struct CaptureFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: &'static str,
}

pub enum ScreenCapturer {
    #[cfg(target_os = "linux")]
    X11(X11Capturer),
    #[cfg(target_os = "linux")]
    Wayland(WaylandCapturer),
}

impl ScreenCapturer {
    pub fn auto_select(env: &EnvState) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            if env.xdg_session_type.as_deref() == Some("wayland") {
                log::info!("Session Detection: Wayland preferred. Attempting PipeWire capture...");
                if let Ok(capturer) = WaylandCapturer::new() {
                    return Ok(ScreenCapturer::Wayland(capturer));
                }
                log::warn!("Wayland capture failed. Falling back to X11 (Xwayland)...");
            }
            if let Ok(capturer) = X11Capturer::new() {
                return Ok(ScreenCapturer::X11(capturer));
            }
        }
        Err(io::Error::new(io::ErrorKind::Other, "No capture backend available"))
    }

    pub fn capture_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        let frame = match self {
            #[cfg(target_os = "linux")]
            ScreenCapturer::X11(c) => c.capture_frame()?,
            #[cfg(target_os = "linux")]
            ScreenCapturer::Wayland(c) => c.capture_frame()?,
        };
        Ok(Some(frame.data))
    }

    pub fn width(&self) -> u32 {
        1920
    }

    pub fn height(&self) -> u32 {
        1080
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            #[cfg(target_os = "linux")]
            ScreenCapturer::X11(_) => "X11",
            #[cfg(target_os = "linux")]
            ScreenCapturer::Wayland(_) => "Wayland (PipeWire)",
        }
    }
}