// Wayland screen capturer via XDG Desktop Portal + PipeWire + GStreamer
//
// Works on GNOME Wayland, Niri, Sway and any compositor with xdg-desktop-portal.
// Requires the user to approve the screenshare dialog ONCE.
//
// Environment variables are set automatically when running under sudo so the
// portal D-Bus request reaches the user's session.

use std::io;
use gstreamer::prelude::*;
use gstreamer_app::AppSink;
use gstreamer_video::VideoFrame;
use gstreamer_video::VideoInfo;
use gstreamer_video::VideoFrameExt;
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;
use std::os::fd::IntoRawFd;

pub struct WaylandCapturer {
    pipeline: gstreamer::Pipeline,
    appsink: AppSink,
    width: u32,
    height: u32,
}

impl WaylandCapturer {
    pub fn new() -> io::Result<Self> {
        // ── Ensure the user's Wayland/D-Bus session is reachable ──────────────
        // When running as root, these are stripped by sudo. Probe and restore them
        // so the XDG portal can be contacted and PipeWire can connect.
        for uid in 999u32..=1005 {
            let runtime_dir = format!("/run/user/{}", uid);
            let bus_path = format!("{}/bus", runtime_dir);
            if std::path::Path::new(&bus_path).exists() {
                if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
                    let addr = format!("unix:path={}", bus_path);
                    std::env::set_var("DBUS_SESSION_BUS_ADDRESS", &addr);
                    log::info!("WaylandCapturer: auto-set DBUS_SESSION_BUS_ADDRESS={}", addr);
                }
                if std::env::var("XDG_RUNTIME_DIR").is_err() {
                    std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
                    log::info!("WaylandCapturer: auto-set XDG_RUNTIME_DIR={}", runtime_dir);
                }
                break;
            }
        }

        // Set WAYLAND_DISPLAY if not present (try wayland-0 and wayland-1)
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            for name in &["wayland-0", "wayland-1"] {
                let xdg = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
                if std::path::Path::new(&format!("{}/{}", xdg, name)).exists() {
                    std::env::set_var("WAYLAND_DISPLAY", name);
                    log::info!("WaylandCapturer: auto-set WAYLAND_DISPLAY={}", name);
                    break;
                }
            }
        }

        log::info!("WaylandCapturer: DBUS_SESSION_BUS_ADDRESS={:?}",
            std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap_or_else(|_| "NOT SET".into()));
        log::info!("WaylandCapturer: WAYLAND_DISPLAY={:?}",
            std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "NOT SET".into()));

        // ── Init GStreamer ─────────────────────────────────────────────────────
        let is_niri = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default().to_lowercase().contains("niri");
        
        if is_niri {
            // Force GStreamer GL to use software rendering (llvmpipe) to bypass the
            // EGL_BAD_MATCH crash caused by Intel Y-tiled CCS DMA-BUFs on Niri.
            std::env::set_var("LIBGL_ALWAYS_SOFTWARE", "1");
            log::info!("WaylandCapturer: Niri detected. Forcing LIBGL_ALWAYS_SOFTWARE=1");
        }
        
        gstreamer::init()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // ── Request screencast via XDG Desktop Portal ──────────────────────────
        log::info!("Requesting screenshare via XDG portal (user approval may appear)...");
        let rt = tokio::runtime::Runtime::new()?;
        let (fd, node_id, width, height) = rt.block_on(async {
            let screencast = Screencast::new().await.map_err(|e| {
                io::Error::new(io::ErrorKind::Other,
                    format!("Portal Screencast::new failed: {} — is xdg-desktop-portal running?", e))
            })?;

            let session = screencast.create_session().await.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("create_session: {}", e))
            })?;

            screencast.select_sources(
                &session,
                CursorMode::Metadata,
                SourceType::Monitor.into(),
                false,                       // multiple = false (one monitor)
                None,                        // no restore token
                PersistMode::ExplicitlyRevoked,
            ).await.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("select_sources: {}", e))
            })?;

            log::info!("Portal: waiting for user approval of screen share...");
            let response = screencast
                .start(&session, &WindowIdentifier::default())
                .await
                .map_err(|e| {
                    io::Error::new(io::ErrorKind::Other,
                        format!("Portal start failed: {} — did you approve the screen share dialog?", e))
                })?;

            let payload = response.response().map_err(|e| {
                io::Error::new(io::ErrorKind::Other,
                    format!("Portal response error: {} — screen share was denied or no monitors selected", e))
            })?;

            let streams = payload.streams();
            if streams.is_empty() {
                return Err(io::Error::new(io::ErrorKind::Other,
                    "Portal returned no streams — did you select a monitor in the dialog?"));
            }

            let stream = &streams[0];
            let node_id = stream.pipe_wire_node_id();

            // Extract reported resolution (may be None on some portals)
            let (w, h) = stream.size()
                .map(|(sw, sh)| (sw as u32, sh as u32))
                .unwrap_or((1920, 1080));

            log::info!("Portal approved! PipeWire node_id={} reported_size={}x{}", node_id, w, h);

            let fd = screencast.open_pipe_wire_remote(&session).await.map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("open_pipe_wire_remote: {}", e))
            })?;

            Ok::<_, io::Error>((fd.into_raw_fd(), node_id, w, h))
        })?;

        // ── GStreamer Pipeline Setup ───────────────────────────────────────────
        // On GNOME/standard Wayland: videoconvert natively imports DMA-BUF (DMA_DRM)
        // On Niri: glupload with software GL forces fallback to CPU SHM, bypassing EGL bug.
        let pipeline_str = if is_niri {
            format!(
                "pipewiresrc fd={fd} path={path} ! \
                 glupload ! glcolorconvert ! gldownload ! video/x-raw,format=BGRx ! \
                 appsink name=sink max-buffers=2 drop=true emit-signals=false",
                fd = fd, path = node_id
            )
        } else {
            format!(
                "pipewiresrc fd={fd} path={path} ! \
                 videoconvert ! video/x-raw,format=BGRx ! \
                 appsink name=sink max-buffers=2 drop=true emit-signals=false",
                fd = fd, path = node_id
            )
        };
        log::info!("GStreamer pipeline: {}", pipeline_str);

        let pipeline = gstreamer::parse::launch(&pipeline_str)
            .map_err(|e| io::Error::new(io::ErrorKind::Other,
                format!("GStreamer parse failed: {} — is gst-plugin-pipewire installed?", e)))?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Not a GStreamer Pipeline"))?;

        let appsink = pipeline
            .by_name("sink")
            .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "appsink 'sink' not found"))?
            .downcast::<AppSink>()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Not an AppSink"))?;

        pipeline
            .set_state(gstreamer::State::Playing)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        log::info!("✓ WaylandCapturer ready: {}x{} via PipeWire node {}", width, height, node_id);
        Ok(Self { pipeline, appsink, width, height })
    }

    pub fn capture_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        // Try pulling a sample with 50ms timeout (non-blocking feel)
        match self.appsink.try_pull_sample(gstreamer::ClockTime::from_mseconds(50)) {
            Some(sample) => {
                let caps = match sample.caps() {
                    Some(c) => c,
                    None => return Ok(None),
                };

                // Parse VideoInfo from caps so we know the real stride/layout
                let video_info = VideoInfo::from_caps(&caps).map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "Cannot parse VideoInfo from caps")
                })?;

                // Update our stored dimensions
                self.width  = video_info.width();
                self.height = video_info.height();

                let buffer = sample.buffer_owned().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::Other, "No buffer in GStreamer sample")
                })?;

                // Use VideoFrame instead of buffer.map_readable().
                // This is mandatory when GstVideoMeta is present (DMA-BUF path from Niri):
                // VideoFrame respects the meta strides/offsets, while map_readable() does
                // not — causing the 'gst_video_frame_map_id: assertion failed' crash that
                // dropped every frame and produced a blank/disappeared screen.
                let frame = VideoFrame::from_buffer_readable(buffer, &video_info)
                    .map_err(|_| io::Error::new(io::ErrorKind::Other,
                        "Cannot map GStreamer buffer as VideoFrame"))?;

                // Copy all plane data into a contiguous BGRx byte vector
                let n_planes = frame.n_planes() as usize;
                let mut out = Vec::with_capacity(
                    (self.width * self.height * 4) as usize
                );
                for p in 0..n_planes {
                    out.extend_from_slice(frame.plane_data(p as u32).map_err(|_| {
                        io::Error::new(io::ErrorKind::Other, "Cannot read plane data")
                    })?);
                }
                Ok(Some(out))
            }
            None => Ok(None), // Timeout — no new frame yet
        }
    }

    pub fn width(&self) -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
}

impl Drop for WaylandCapturer {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gstreamer::State::Null);
        log::info!("WaylandCapturer: GStreamer pipeline stopped.");
    }
}
