// X11 screen capturer using scrap (XCB MIT-SHM)
// Used on native X11 sessions. Auto-sets XAUTHORITY when running under sudo.

use std::io;
use scrap::{Capturer, Display};

pub struct X11Capturer {
    capturer: Capturer,
    width: u32,
    height: u32,
}

impl X11Capturer {
    pub fn new() -> io::Result<Self> {
        // When running as root (sudo), XAUTHORITY is stripped.
        // Probe common locations so XCB can authenticate.
        if std::env::var("XAUTHORITY").is_err() {
            'outer: for uid in 999u32..=1005 {
                let dir = format!("/run/user/{}", uid);
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let name = entry.file_name();
                        let s = name.to_string_lossy();
                        if s.starts_with(".mutter-Xwaylandauth")
                            || s.to_lowercase().contains("xauth")
                        {
                            let path = entry.path();
                            std::env::set_var("XAUTHORITY", &path);
                            log::info!("X11Capturer: auto-set XAUTHORITY={}", path.display());
                            break 'outer;
                        }
                    }
                }
            }
        }

        let display = Display::primary().map_err(|e| {
            io::Error::new(
                io::ErrorKind::Other,
                format!("scrap Display::primary failed: {}. NOTE: on Wayland the X11 backend only captures the blank Xwayland root window — use the Wayland backend instead.", e),
            )
        })?;

        let width = display.width() as u32;
        let height = display.height() as u32;
        let capturer = Capturer::new(display)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        log::info!("✓ X11Capturer ready: {}x{} (XAUTHORITY={})",
            width, height,
            std::env::var("XAUTHORITY").unwrap_or_else(|_| "not set".into()));
        Ok(Self { capturer, width, height })
    }

    pub fn capture_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        match self.capturer.frame() {
            Ok(frame) => Ok(Some(frame.to_vec())),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
}
