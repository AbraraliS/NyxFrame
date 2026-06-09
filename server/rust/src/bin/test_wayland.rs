use ashpd::desktop::screencast::{CursorMode, Screencast, SourceType};
use ashpd::desktop::PersistMode;
use ashpd::WindowIdentifier;
use gstreamer::prelude::*;
use std::os::fd::IntoRawFd;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Initializing Wayland capture test...");
    gstreamer::init()?;

    let screencast = Screencast::new().await?;
    let session = screencast.create_session().await?;
    
    screencast.select_sources(
        &session,
        CursorMode::Metadata,
        SourceType::Monitor.into(),
        false,
        None,
        PersistMode::ExplicitlyRevoked,
    ).await?;
    
    println!("Please check your screen for a screen sharing prompt and click Allow!");
    let response = screencast.start(&session, &WindowIdentifier::default()).await?;
    let payload = response.response()?;
    let streams = payload.streams();
    
    if streams.is_empty() {
        println!("No streams selected!");
        return Ok(());
    }
    
    let node_id = streams[0].pipe_wire_node_id();
    let fd = screencast.open_pipe_wire_remote(&session).await?.into_raw_fd();
    
    println!("Got node_id: {}, fd: {}", node_id, fd);
    
    // Create GStreamer pipeline to save 1 frame to jpeg
    let pipeline_str = format!(
        "pipewiresrc fd={} path={} always-copy=true ! \
         videoconvert ! video/x-raw,format=BGRx ! \
         appsink name=sink max-buffers=2 drop=true emit-signals=false",
        fd, node_id
    );
    println!("Pipeline: {}", pipeline_str);
    
    let pipeline = gstreamer::parse::launch(&pipeline_str)?
        .downcast::<gstreamer::Pipeline>()
        .unwrap();
        
    pipeline.set_state(gstreamer::State::Playing)?;
    
    let appsink = pipeline.by_name("sink").unwrap().downcast::<gstreamer_app::AppSink>().unwrap();
    
    // Pull a sample
    println!("Waiting for frame...");
    let sample = appsink.pull_sample().map_err(|_| "Failed to pull sample")?;
    let buffer = sample.buffer().unwrap();
    let map = buffer.map_readable().unwrap();
    
    let slice = map.as_slice();
    println!("Successfully captured frame! Size: {} bytes", slice.len());
    
    // Check if it's all black (zeroes)
    let non_zero = slice.iter().filter(|&&b| b > 0).count();
    println!("Non-zero bytes: {} / {}", non_zero, slice.len());
    
    pipeline.set_state(gstreamer::State::Null)?;
    
    Ok(())
}
