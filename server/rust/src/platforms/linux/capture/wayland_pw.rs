// platforms/linux/capture/wayland_pw.rs
//
// Raw PipeWire capture for Niri/Wayland compositors.
//
// WHY THIS EXISTS:
// Niri exports screen frames as Intel Y-tiled CCS DMA-BUF (modifier 0x100000000000008).
// GStreamer's glupload cannot import this modifier via EGL (returns EGL_BAD_MATCH),
// causing every GStreamer-based pipeline to fail on Niri. The only working solution is
// to bypass GStreamer entirely and use the libpipewire C API directly, requesting
// SPA_DATA_MemPtr (CPU SHM) buffers. Niri then performs a GPU→CPU blit and delivers
// plain system-memory frames. This is exactly the approach used by wl-screenrec,
// gnome-remote-desktop, and krfb.
//
// ARCHITECTURE:
//   Rust main thread:
//     1. Opens XDG portal session via ashpd (gets portal_fd + node_id)
//     2. Passes fd + node_id to a background thread
//     3. Polls frame data via Arc<Mutex<SharedFrame>>
//
//   Background PipeWire thread:
//     1. Creates pw_main_loop + pw_context
//     2. Connects to portal's restricted remote via portal_fd
//     3. Creates pw_stream requesting ONLY MemPtr/MemFd buffer types
//     4. Negotiates BGRx format (no DMA-BUF modifier)
//     5. On each frame: memcpy → SharedFrame.data

use std::io;
use std::sync::{Arc, Mutex};
use std::os::fd::{IntoRawFd, RawFd, OwnedFd};

use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;

use pipewire::{
    context::Context,
    main_loop::MainLoop,
    properties::properties,
    stream::{Stream, StreamFlags, StreamListener},
    spa::{
        param::video::{VideoFormat, VideoInfoRaw},
        pod::{Object, Pod, Value, serialize::PodSerializer},
        sys::{
            SPA_PARAM_EnumFormat, SPA_PARAM_Buffers,
            SPA_TYPE_OBJECT_Format, SPA_TYPE_OBJECT_ParamBuffers,
            SPA_FORMAT_mediaType, SPA_FORMAT_mediaSubtype,
            SPA_FORMAT_VIDEO_format, SPA_FORMAT_VIDEO_size, SPA_FORMAT_VIDEO_framerate,
            SPA_MEDIA_TYPE_video, SPA_MEDIA_SUBTYPE_raw,
            SPA_VIDEO_FORMAT_BGRx,
            SPA_PARAM_BUFFERS_buffers, SPA_PARAM_BUFFERS_size,
            SPA_PARAM_BUFFERS_stride, SPA_PARAM_BUFFERS_dataType,
            SPA_DATA_MemPtr,
        },
        utils::Id,
    },
};

use pipewire::spa::pod::{Property, PropertyFlags};

/// Shared frame data between PW callback thread and main capture loop.
#[derive(Default)]
struct SharedFrame {
    /// Latest raw BGRx frame bytes (width * height * 4).
    data:   Option<Vec<u8>>,
    width:  u32,
    height: u32,
}

pub struct WaylandPwCapturer {
    frame:   Arc<Mutex<SharedFrame>>,
    _thread: std::thread::JoinHandle<()>,
    width:   u32,
    height:  u32,
}

impl WaylandPwCapturer {
    pub fn new() -> io::Result<Self> {
        // ── Restore user session environment when running under sudo ────────────
        for uid in 999u32..=1005 {
            let runtime_dir = format!("/run/user/{}", uid);
            if std::path::Path::new(&format!("{}/bus", runtime_dir)).exists() {
                if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
                    std::env::set_var("DBUS_SESSION_BUS_ADDRESS",
                        format!("unix:path={}/bus", runtime_dir));
                }
                if std::env::var("XDG_RUNTIME_DIR").is_err() {
                    std::env::set_var("XDG_RUNTIME_DIR", &runtime_dir);
                }
                break;
            }
        }
        if std::env::var("WAYLAND_DISPLAY").is_err() {
            let xdg = std::env::var("XDG_RUNTIME_DIR").unwrap_or_default();
            for name in &["wayland-0", "wayland-1"] {
                if std::path::Path::new(&format!("{}/{}", xdg, name)).exists() {
                    std::env::set_var("WAYLAND_DISPLAY", name);
                    break;
                }
            }
        }

        // ── Open XDG Desktop Portal screenshare session ─────────────────────────
        log::info!("WaylandPwCapturer: opening portal session...");
        let rt = tokio::runtime::Runtime::new()?;
        let (portal_fd, node_id, init_w, init_h) = rt.block_on(async {
            let sc = Screencast::new().await.map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("Screencast::new: {e}")))?;

            let session = sc.create_session().await.map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("create_session: {e}")))?;

            sc.select_sources(&session,
                CursorMode::Metadata,
                SourceType::Monitor.into(),
                false, None,
                PersistMode::ExplicitlyRevoked,
            ).await.map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("select_sources: {e}")))?;

            log::info!("WaylandPwCapturer: awaiting user approval...");
            let resp = sc.start(&session, &WindowIdentifier::default()).await.map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("portal start: {e}")))?;
            let payload = resp.response().map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("portal response: {e}")))?;

            let streams = payload.streams();
            if streams.is_empty() {
                return Err(io::Error::new(io::ErrorKind::Other, "No streams in portal response"));
            }
            let s = &streams[0];
            let nid = s.pipe_wire_node_id();
            let (w, h) = s.size().map(|(a,b)|(a as u32, b as u32)).unwrap_or((1920, 1080));
            log::info!("WaylandPwCapturer: approved! node={nid} size={w}x{h}");

            let fd = sc.open_pipe_wire_remote(&session).await.map_err(|e|
                io::Error::new(io::ErrorKind::Other, format!("open_pipe_wire_remote: {e}")))?;
            Ok::<_, io::Error>((fd.into_raw_fd(), nid, w, h))
        })?;

        let frame = Arc::new(Mutex::new(SharedFrame { data: None, width: init_w, height: init_h }));
        let frame_clone = frame.clone();

        // ── Spawn PipeWire capture loop on its own thread ──────────────────────
        let _thread = std::thread::spawn(move || {
            if let Err(e) = run_pw_loop(portal_fd, node_id, frame_clone) {
                log::error!("WaylandPwCapturer: PipeWire loop exited: {e}");
            }
        });

        // Give the PW loop time to connect and receive first frame
        std::thread::sleep(std::time::Duration::from_millis(800));

        Ok(Self { frame, _thread, width: init_w, height: init_h })
    }

    pub fn capture_frame(&mut self) -> io::Result<Option<Vec<u8>>> {
        let mut g = self.frame.lock().unwrap();
        if let Some(data) = g.data.take() {
            self.width  = g.width;
            self.height = g.height;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    pub fn width(&self)  -> u32 { self.width }
    pub fn height(&self) -> u32 { self.height }
}

// ─── PipeWire main loop (runs on background thread) ─────────────────────────

fn run_pw_loop(portal_fd: RawFd, node_id: u32, frame: Arc<Mutex<SharedFrame>>) -> io::Result<()> {
    pipewire::init();
    log::info!("PW: initializing main loop...");

    let main_loop = MainLoop::new(None)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("MainLoop: {e}")))?;
    let context   = Context::new(&main_loop)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Context: {e}")))?;

    // Connect via the portal's restricted remote fd (NOT the global pipewire socket).
    // This is critical — the portal fd provides a pre-authenticated view of only the
    // specific screenshared node, and it is the only way to reach the screen content.
    let owned_fd = unsafe { OwnedFd::from_raw_fd(portal_fd) };
    let core = context.connect_fd(owned_fd, Some(properties! {
        *pipewire::keys::MEDIA_TYPE     => "Video",
        *pipewire::keys::MEDIA_CATEGORY => "Capture",
        *pipewire::keys::MEDIA_ROLE     => "Screen",
    })).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("connect_fd: {e}")))?;

    // Build the format negotiation POD:
    // We advertise ONLY video/raw BGRx with no DRM modifier constraint.
    // Because we DON'T advertise SPA_DATA_DmaBuf buffer type, libpipewire will
    // not negotiate DMA-BUF allocation. Niri will respond by doing a GPU→CPU blit
    // and deliver frames in SPA_DATA_MemPtr (plain mmap-able system memory).
    let format_pod = build_bgr_format_pod()?;
    let params: Vec<&Pod> = vec![Pod::from_bytes(&format_pod).ok_or_else(||
        io::Error::new(io::ErrorKind::Other, "Bad format POD"))?];

    let stream = Stream::new(&core, "nyxframe-screen-capture", properties! {
        *pipewire::keys::MEDIA_TYPE     => "Video",
        *pipewire::keys::MEDIA_CATEGORY => "Capture",
        *pipewire::keys::MEDIA_ROLE     => "Screen",
        *pipewire::keys::NODE_TARGET    => node_id.to_string(),
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Stream::new: {e}")))?;

    // Install the process callback which fires once per delivered frame
    let frame_cb = frame.clone();
    let _listener: StreamListener<()> = stream
        .add_local_listener_with_user_data(())
        .process(move |stream, _| {
            // Dequeue the next ready buffer from the PipeWire queue
            if let Some(mut buf) = stream.dequeue_buffer() {
                let datas = buf.datas_mut();
                if datas.is_empty() { return; }
                let d = &mut datas[0];
                let chunk = d.chunk();
                let size = chunk.size() as usize;
                if size == 0 { return; }

                // For MemPtr, the data pointer is the raw frame bytes in CPU memory.
                // This is valid for the duration of this callback.
                if let Some(slice) = d.data() {
                    let bytes = slice[..size.min(slice.len())].to_vec();

                    // Parse dimensions from PW stream state if possible
                    let mut g = frame_cb.lock().unwrap();
                    // Update width/height only if we can get it; otherwise keep portal values
                    g.data = Some(bytes);
                    // (Width/height are updated via param_changed callback below)
                }
            }
        })
        .param_changed(move |stream, _user, id, pod| {
            // When Niri confirms format, extract width/height
            if id != SPA_PARAM_EnumFormat { return; }
            if let Some(pod) = pod {
                if let Ok(vi) = VideoInfoRaw::parse(pod) {
                    let size = vi.size();
                    // Can't borrow frame here (moved), so we log it
                    log::info!("PW: negotiated format {}x{} {:?}", size.width, size.height, vi.format());
                }
            }
        })
        .register()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("register listener: {e}")))?;

    // Connect the stream to the portal node.
    // CRITICAL: StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS ensures
    // libpipewire maps the buffer memory for us. No DmaBuf flag = SHM only.
    stream.connect(
        pipewire::spa::utils::Direction::Input,
        Some(node_id),
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
        &mut params.clone(),
    ).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("stream connect: {e}")))?;

    log::info!("PW: stream connected to node {node_id}, starting event loop...");
    main_loop.run();
    Ok(())
}

/// Serialize a video/x-raw BGRx format POD for PipeWire negotiation.
/// No DRM modifier = no DMA-BUF = Niri delivers SHM copy.
fn build_bgr_format_pod() -> io::Result<Vec<u8>> {
    use std::io::Cursor;
    use pipewire::spa::pod::serialize::PodSerializer;
    use pipewire::spa::sys::*;
    use pipewire::spa::utils::Fraction;
    use pipewire::spa::utils::Rectangle;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let (pod_bytes, _) = PodSerializer::serialize(cursor, &Value::Object(Object {
        type_: SPA_TYPE_OBJECT_Format,
        id:    SPA_PARAM_EnumFormat,
        properties: vec![
            Property {
                key: SPA_FORMAT_mediaType,
                flags: PropertyFlags::empty(),
                value: Value::Id(unsafe { Id::from_raw(SPA_MEDIA_TYPE_video) }),
            },
            Property {
                key: SPA_FORMAT_mediaSubtype,
                flags: PropertyFlags::empty(),
                value: Value::Id(unsafe { Id::from_raw(SPA_MEDIA_SUBTYPE_raw) }),
            },
            Property {
                key: SPA_FORMAT_VIDEO_format,
                flags: PropertyFlags::empty(),
                value: Value::Id(unsafe { Id::from_raw(SPA_VIDEO_FORMAT_BGRx) }),
            },
        ],
    })).map_err(|e| io::Error::new(io::ErrorKind::Other, format!("serialize format pod: {e}")))?;

    Ok(pod_bytes.into_inner())
}
