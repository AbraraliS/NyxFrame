use std::io;
use log::{info, error};
use super::CaptureFrame;
use gstreamer::prelude::*;
use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;
use std::os::fd::IntoRawFd;

pub struct WaylandCapturer {
    pipeline: gstreamer::Pipeline,
    appsink: gstreamer_app::AppSink,
}

impl WaylandCapturer {
    pub fn new() -> io::Result<Self> {
        info!("Initializing Wayland/PipeWire capture engine with ashpd...");
        gstreamer::init().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Run ashpd portal request in an isolated tokio runtime
        let rt = tokio::runtime::Runtime::new()?;
        let (fd, node_id) = rt.block_on(async {
            info!("Requesting Screencast portal session...");
            let screencast = Screencast::new().await.map_err(|e| {
                error!("ashpd Screencast::new failed: {:?}", e);
                io::Error::new(io::ErrorKind::Other, e.to_string())
            })?;
            let session = screencast.create_session().await.map_err(|e| {
                error!("ashpd create_session failed: {:?}", e);
                io::Error::new(io::ErrorKind::Other, e.to_string())
            })?;
            
            screencast.select_sources(
                &session,
                CursorMode::Metadata,
                SourceType::Monitor | SourceType::Window,
                true,
                None,
                PersistMode::DoNot
            ).await.map_err(|e| {
                error!("ashpd select_sources failed: {:?}", e);
                io::Error::new(io::ErrorKind::Other, e.to_string())
            })?;

            info!("Starting screencast session. Waiting for user approval...");
            let response = screencast.start(&session, &WindowIdentifier::default()).await
                .map_err(|e| {
                    error!("ashpd start failed: {:?}", e);
                    io::Error::new(io::ErrorKind::Other, e.to_string())
                })?;

            let payload = response.response().map_err(|e| {
                error!("ashpd response failed: {:?}", e);
                io::Error::new(io::ErrorKind::Other, e.to_string())
            })?;
            let streams = payload.streams();
            if streams.is_empty() {
                error!("ashpd streams empty! User denied or no streams selected");
                return Err(io::Error::new(io::ErrorKind::Other, "User denied or no streams selected"));
            }

            let node_id = streams[0].pipe_wire_node_id();
            let fd = screencast.open_pipe_wire_remote(&session).await
                .map_err(|e| {
                    error!("ashpd open_pipe_wire_remote failed: {:?}", e);
                    io::Error::new(io::ErrorKind::Other, e.to_string())
                })?;

            info!("Screencast portal approved! node_id: {}", node_id);
            Ok::<_, io::Error>((fd.into_raw_fd(), node_id))
        })?;

        // Pass fd and node_id to pipewiresrc
        let pipeline_str = format!(
            "pipewiresrc fd={} path={} ! videoconvert ! video/x-raw,format=BGRA ! appsink name=sink max-buffers=1 drop=true",
            fd, node_id
        );
        
        info!("Launching GStreamer pipeline with specific portal node...");
        let pipeline = gstreamer::parse::launch(&pipeline_str)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
            .downcast::<gstreamer::Pipeline>()
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "Not a pipeline"))?;

        let appsink = pipeline
            .by_name("sink")
            .unwrap()
            .downcast::<gstreamer_app::AppSink>()
            .unwrap();

        pipeline.set_state(gstreamer::State::Playing)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(Self { pipeline, appsink })
    }

    pub fn capture_frame(&mut self) -> io::Result<CaptureFrame> {
        let sample = self.appsink.pull_sample().map_err(|_| io::Error::new(io::ErrorKind::Other, "End of stream"))?;
        let buffer = sample.buffer().ok_or_else(|| io::Error::new(io::ErrorKind::Other, "No buffer"))?;
        let map = buffer.map_readable().map_err(|_| io::Error::new(io::ErrorKind::Other, "Cannot map buffer"))?;
        
        let caps = sample.caps().unwrap();
        let structure = caps.structure(0).unwrap();
        let width = structure.get::<i32>("width").unwrap_or(1920) as u32;
        let height = structure.get::<i32>("height").unwrap_or(1080) as u32;

        Ok(CaptureFrame {
            data: map.as_slice().to_vec(),
            width,
            height,
            format: "bgra",
        })
    }
}
