// agy-pl-v2/rust/src/main.rs
pub mod core;
pub mod platforms;
pub mod ipc;
use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::os::unix::process::CommandExt;
use tokio::sync::Mutex;
use log::{info, error, LevelFilter};
use env_logger::Builder;

use platforms::linux::input::uinput::UInputDevice;
use ipc::uds::{UdsServer, send_frame, read_command};
use core::protocol::Command;


// ─────────────────────────────────────────────────────────────────────────────
// Custom Full-Range PC/JPEG RGB-to-YUV420p Conversion Buffer
// ─────────────────────────────────────────────────────────────────────────────
pub struct FullRangeYUVBuffer {
    width: usize,
    height: usize,
    y_plane: Vec<u8>,
    u_plane: Vec<u8>,
    v_plane: Vec<u8>,
}

impl FullRangeYUVBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            y_plane: vec![0u8; width * height],
            u_plane: vec![0u8; (width / 2) * (height / 2)],
            v_plane: vec![0u8; (width / 2) * (height / 2)],
        }
    }

    pub fn read_rgb(&mut self, rgb: &[u8]) {
        let width = self.width;
        let height = self.height;
        let half_width = width / 2;

        // Populate Y plane using optimized integer full-range coefficients:
        // Y = ((77 * R + 150 * G + 29 * B) >> 8)
        let src_chunks = rgb.chunks_exact(3);
        for (idx, rgb_pixel) in src_chunks.enumerate() {
            let r = rgb_pixel[0] as i32;
            let g = rgb_pixel[1] as i32;
            let b = rgb_pixel[2] as i32;
            self.y_plane[idx] = ((77 * r + 150 * g + 29 * b) >> 8).clamp(0, 255) as u8;
        }

        // Populate U and V planes by subsampling 2x2 blocks with full-range coefficients
        for j in 0..height / 2 {
            let y0 = j * 2;
            let y1 = y0 + 1;
            for i in 0..width / 2 {
                let x0 = i * 2;
                let x1 = x0 + 1;

                let p00_idx = (x0 + y0 * width) * 3;
                let p01_idx = (x1 + y0 * width) * 3;
                let p10_idx = (x0 + y1 * width) * 3;
                let p11_idx = (x1 + y1 * width) * 3;

                let r_avg = (rgb[p00_idx] as i32 + rgb[p01_idx] as i32 + rgb[p10_idx] as i32 + rgb[p11_idx] as i32) / 4;
                let g_avg = (rgb[p00_idx + 1] as i32 + rgb[p01_idx + 1] as i32 + rgb[p10_idx + 1] as i32 + rgb[p11_idx + 1] as i32) / 4;
                let b_avg = (rgb[p00_idx + 2] as i32 + rgb[p01_idx + 2] as i32 + rgb[p10_idx + 2] as i32 + rgb[p11_idx + 2] as i32) / 4;

                let u_val = (((-43 * r_avg - 85 * g_avg + 128 * b_avg) >> 8) + 128).clamp(0, 255) as u8;
                let v_val = (((128 * r_avg - 107 * g_avg - 21 * b_avg) >> 8) + 128).clamp(0, 255) as u8;

                let dst_idx = i + j * half_width;
                self.u_plane[dst_idx] = u_val;
                self.v_plane[dst_idx] = v_val;
            }
        }
    }
}

impl openh264::formats::YUVSource for FullRangeYUVBuffer {
    fn width(&self) -> i32 {
        self.width as i32
    }

    fn height(&self) -> i32 {
        self.height as i32
    }

    fn y(&self) -> &[u8] {
        &self.y_plane
    }

    fn u(&self) -> &[u8] {
        &self.u_plane
    }

    fn v(&self) -> &[u8] {
        &self.v_plane
    }

    fn y_stride(&self) -> i32 {
        self.width as i32
    }

    fn u_stride(&self) -> i32 {
        (self.width / 2) as i32
    }

    fn v_stride(&self) -> i32 {
        (self.width / 2) as i32
    }
}


// ─────────────────────────────────────────────────────────────────────────────
// Automated ADB Workstation IP Auto-Discovery Broadcaster
// ─────────────────────────────────────────────────────────────────────────────
fn broadcast_workstation_ips_to_adb() {
    tokio::spawn(async move {
        // Wait 1.5 seconds for the Android app activity to fully load in the foreground
        tokio::time::sleep(Duration::from_millis(1500)).await;
        
        info!("Scanning for connected Android devices via ADB for auto-discovery...");
        let adb_devices = std::process::Command::new("adb")
            .arg("devices")
            .output();
            
        match adb_devices {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines()
                    .filter(|line| !line.is_empty() && !line.starts_with("List of devices") && line.contains("device"))
                    .collect();
                if lines.is_empty() {
                    info!("No Android devices connected via USB debugging. Auto-discovery broadcast skipped.");
                    return;
                }
                
                info!("Connected Android device detected! Resolving workstation network interfaces...");
                
                // Get hostname
                let hostname = std::process::Command::new("uname")
                    .arg("-n")
                    .output()
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
                    .unwrap_or_else(|_| "nyxframe".to_string());
                    
                // Get LAN IP (Route to internet interface primary IP)
                let lan_ip_output = std::process::Command::new("sh")
                    .args(&["-c", "ip route get 1.1.1.1 | grep -oP 'src \\K\\S+'"])
                    .output();
                let lan_ip = match lan_ip_output {
                    Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    Err(_) => "".to_string(),
                };
                
                // Get Tailscale IP
                let ts_ip_output = std::process::Command::new("tailscale")
                    .args(&["ip", "-4"])
                    .output();
                let ts_ip = match ts_ip_output {
                    Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
                    Err(_) => "".to_string(),
                };
                
                info!("✓ Workstation identified: {} (Wi-Fi LAN: {}, Tailscale: {})", hostname, lan_ip, ts_ip);
                
                // Fire ADB broadcast!
                let adb_broadcast = std::process::Command::new("adb")
                    .args(&[
                        "shell",
                        "am",
                        "broadcast",
                        "-a",
                        "com.nyxframe.app.DISCOVER_IP",
                        "--es",
                        "host_name",
                        &hostname,
                        "--es",
                        "lan_ip",
                        &lan_ip,
                        "--es",
                        "tailscale_ip",
                        &ts_ip,
                    ])
                    .status();
                    
                match adb_broadcast {
                    Ok(status) if status.success() => {
                        info!("✓ Auto-discovery workstation details successfully broadcasted to phone!");
                    }
                    _ => {
                        error!("✖ Failed to broadcast workstation details over ADB.");
                    }
                }
            }
            Err(e) => {
                error!("ADB command not available: {}", e);
            }
        }
    });
}

fn spawn_tee_logging(log_file: std::fs::File) -> Result<(), std::io::Error> {
    use std::os::unix::io::FromRawFd;
    use std::io::{Read, Write};

    // 1. Duplicate original stdout/stderr descriptors so we can write to the terminal
    let orig_stdout_fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
    let orig_stderr_fd = unsafe { libc::dup(libc::STDERR_FILENO) };
    if orig_stdout_fd < 0 || orig_stderr_fd < 0 {
        return Err(std::io::Error::last_os_error());
    }

    // Convert raw FDs to safe File instances for writing to the terminal
    let mut terminal_out = unsafe { std::fs::File::from_raw_fd(orig_stdout_fd) };

    // 2. Create the pipe
    let mut pipe_fds = [0; 2];
    if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } < 0 {
        return Err(std::io::Error::last_os_error());
    }

    let read_fd = pipe_fds[0];
    let write_fd = pipe_fds[1];

    // 3. Redirect STDOUT and STDERR of this process (and future children) to the write-end of the pipe
    unsafe {
        if libc::dup2(write_fd, libc::STDOUT_FILENO) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::dup2(write_fd, libc::STDERR_FILENO) < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // Close the write-end of the pipe now that it is duplicated to 1 and 2
        libc::close(write_fd);
    }

    // Convert the read-end of the pipe to a safe File instance for reading
    let mut pipe_read = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut log_file_clone = log_file.try_clone()?;

    // 4. Spawn a background thread to read from the pipe and write to both the terminal and the log file
    std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match pipe_read.read(&mut buffer) {
                Ok(0) => break, // EOF reached (all write descriptors closed)
                Ok(n) => {
                    // Write to the original terminal stdout
                    let _ = terminal_out.write_all(&buffer[..n]);
                    let _ = terminal_out.flush();
                    
                    // Write to the datetime log file
                    let _ = log_file_clone.write_all(&buffer[..n]);
                    let _ = log_file_clone.flush();
                }
                Err(_) => break,
            }
        }
    });

    Ok(())
}


// ─────────────────────────────────────────────────────────────────────────────
// Primary Rust Daemon Entry Point
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sudo_uid = env::var("SUDO_UID").ok().and_then(|s| s.parse::<u32>().ok());
    let sudo_gid = env::var("SUDO_GID").ok().and_then(|s| s.parse::<u32>().ok());
    let sudo_user = env::var("SUDO_USER").ok();

    // 0. Resolve unified logging path and redirect stdout/stderr to logs/<datetime>.log
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Failed to get parent directory")?;
    
    // Load dynamic server configuration
    let config = Arc::new(core::config::load_server_config(exe_dir));

    // Resolve target log directory
    let logs_dir = if let Some(ref custom_dir) = config.log_dir {
        std::path::PathBuf::from(custom_dir)
    } else {
        if let Ok(user_name) = env::var("USER") {
            std::path::PathBuf::from(format!("/home/{}/.local/share/nyxframe/logs", user_name))
        } else {
            std::path::PathBuf::from("/root/.local/share/nyxframe/logs")
        }
    };

    std::fs::create_dir_all(&logs_dir)?;

    // Set permissions to 777 for directory
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&logs_dir, std::fs::Permissions::from_mode(0o777));
    }

    let date_output = std::process::Command::new("date")
        .arg("+%Y-%m-%d_%H-%M-%S")
        .output();
    let time_str = match date_output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "log".to_string(),
    };
    let log_filename = format!("{}.log", time_str);
    let log_path = logs_dir.join(&log_filename);

    println!("[INFO] Server unified logs are being saved directly in: {}/{}", logs_dir.to_string_lossy(), log_filename);

    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)?;

    // Set permissions to 666 for log file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&log_path, std::fs::Permissions::from_mode(0o666));
        
        // Also chown to standard user if running under sudo
        if let (Some(uid), Some(gid)) = (sudo_uid, sudo_gid) {
            let logs_c_str = std::ffi::CString::new(logs_dir.to_string_lossy().to_string()).unwrap();
            let log_path_c_str = std::ffi::CString::new(log_path.to_string_lossy().to_string()).unwrap();
            unsafe {
                libc::chown(logs_c_str.as_ptr(), uid, gid);
                libc::chown(log_path_c_str.as_ptr(), uid, gid);
            }
        }
    }

    spawn_tee_logging(log_file)?;

    // 1. Initialize systems logger
    Builder::new()
        .filter(None, LevelFilter::Info)
        .init();

    println!("======================================================");
    println!("             NYXFRAME VIRTUAL SYSTEMS DAEMON          ");
    println!("======================================================");

    // 2. Parse active capture mode flags and recover environment
    let args: Vec<String> = env::args().collect();
    let diagnostics_mode = args.iter().any(|arg| arg == "--diagnostics");

    let env_state = platforms::linux::env::recover_environment(sudo_uid, sudo_user.clone());
    if diagnostics_mode {
        env_state.print_report();
        let pass = platforms::linux::env::validate_backends(&env_state);
        if !pass {
            error!("Diagnostics mode: Backend validation failed.");
        } else {
            info!("Diagnostics mode: Backend validation passed.");
        }
        std::process::exit(if pass { 0 } else { 1 });
    }

    // Auto-select the best capture backend based on session environment
    // CLI overrides: --wayland forces PipeWire, --x11 forces X11 MIT-SHM
    let force_x11 = args.iter().any(|arg| arg == "--x11");
    let force_wayland = args.iter().any(|arg| arg == "--wayland");
    
    let select_env = if force_x11 {
        info!("CLI override: Forcing X11 MIT-SHM backend.");
        platforms::linux::env::EnvState { wayland_confidence: 0, x11_confidence: 100, ..env_state.clone() }
    } else if force_wayland {
        info!("CLI override: Forcing Wayland/PipeWire backend.");
        platforms::linux::env::EnvState { wayland_confidence: 100, x11_confidence: 0, ..env_state.clone() }
    } else {
        env_state.clone()
    };
    
    let shared_capturer = match platforms::linux::capture::ScreenCapturer::auto_select(&select_env) {
        Ok(cap) => {
            info!("✓ Screen Capture Backend: {}", cap.backend_name());
            Arc::new(Mutex::new(cap))
        }
        Err(e) => {
            error!("✖ CRITICAL: Could not initialize any screen capture backend: {}", e);
            error!("  Run with --diagnostics to debug the session environment.");
            std::process::exit(1);
        }
    };

    // 3. Initialize kernel-level uinput device client
    let device = match UInputDevice::new() {
        Ok(dev) => Arc::new(Mutex::new(dev)),
        Err(e) => {
            error!("✖ CRITICAL ERROR: Could not create uinput virtual device.");
            error!("  Ensure you are running as root or have verified '/dev/uinput' permission mappings.");
            error!("  Error details: {}", e);
            std::process::exit(1);
        }
    };

    // 4. Initialize Unix Domain Socket Server
    let uds_server = match UdsServer::new(&config.uds_socket_path) {
        Ok(srv) => srv,
        Err(e) => {
            error!("✖ CRITICAL ERROR: Could not bind Unix Domain Socket Server: {}", e);
            std::process::exit(1);
        }
    };

    // 4.5. Automatically compile and spawn Go Gateway Server as a background child process
    // Check multiple candidate locations relative to exe_dir
    let mut go_dir_path = exe_dir.join("server/go");
    if !go_dir_path.exists() {
        if let Ok(path) = exe_dir.join("../../../go").canonicalize() {
            go_dir_path = path;
        } else if let Ok(path) = exe_dir.join("../../../server/go").canonicalize() {
            go_dir_path = path;
        }
    }
    
    let go_dir = go_dir_path.to_string_lossy().to_string();
    let go_bin_path = go_dir_path.join("server");
    let go_bin = go_bin_path.to_string_lossy().to_string();
    
    // sudo_uid, sudo_gid, sudo_user are already in scope from the top of main()

    let mut build_needed = !std::path::Path::new(&go_bin).exists();
    if !build_needed {
        if let (Ok(bin_meta), Ok(src_meta)) = (std::fs::metadata(&go_bin), std::fs::metadata(format!("{}/main.go", go_dir))) {
            if let (Ok(bin_mod), Ok(src_mod)) = (bin_meta.modified(), src_meta.modified()) {
                if src_mod > bin_mod {
                    info!("Go source main.go is newer than binary. Recompiling...");
                    build_needed = true;
                }
            }
        }
    }

    if build_needed {
        if !std::path::Path::new(&go_bin).exists() {
            info!("Go gateway server binary not found. Compiling automatically...");
        }
        let mut compile_cmd = std::process::Command::new("go");
        compile_cmd.args(&["build", "-o", "server", "main.go"])
            .current_dir(&go_dir);
        
        if let Some(uid) = sudo_uid {
            compile_cmd.uid(uid);
        }
        if let Some(gid) = sudo_gid {
            compile_cmd.gid(gid);
        }
        
        let compile_status = compile_cmd.status();
        match compile_status {
            Ok(status) if status.success() => {
                info!("✓ Go gateway server successfully compiled!");
            }
            _ => {
                error!("✖ Compilation failed. Make sure Go is installed and build manually.");
            }
        }
    }

    info!("Launching Go Network Gateway server dynamically as a child process...");
    let mut go_cmd = std::process::Command::new(&go_bin);
    go_cmd.current_dir(&go_dir);
    
    if let Some(uid) = sudo_uid {
        go_cmd.uid(uid);
    }
    if let Some(gid) = sudo_gid {
        go_cmd.gid(gid);
    }
    
    if let Some(user) = &sudo_user {
        let xauth_path = format!("/home/{}/.Xauthority", user);
        if std::path::Path::new(&xauth_path).exists() {
            go_cmd.env("XAUTHORITY", xauth_path);
        }
        go_cmd.env("HOME", format!("/home/{}", user));
        go_cmd.env("USER", user);
    }
    go_cmd.env("DISPLAY", ":0");
    go_cmd.env("LOG_FILE_PATH", &log_path);

    // R6 — Go child process watchdog: monitor and respawn Go gateway on exit
    match go_cmd.spawn() {
        Ok(initial_child) => {
            let go_bin_w       = go_bin.clone();
            let go_dir_w       = go_dir.clone();
            let log_path_str   = log_path.to_string_lossy().to_string();
            let sudo_uid_w     = sudo_uid;
            let sudo_gid_w     = sudo_gid;
            let sudo_user_w    = sudo_user.clone();
            tokio::spawn(async move {
                let mut current_child = initial_child;
                let mut backoff_secs: u64 = 1;
                loop {
                    // Block until the child process exits
                    let exit_status = tokio::task::spawn_blocking(move || current_child.wait()).await;
                    match &exit_status {
                        Ok(Ok(s))  => error!("[Watchdog] Go gateway exited ({}). Respawning in {}s...", s, backoff_secs),
                        Ok(Err(e)) => error!("[Watchdog] Go gateway wait() failed ({}). Respawning in {}s...", e, backoff_secs),
                        Err(e)     => error!("[Watchdog] spawn_blocking join error ({}). Respawning in {}s...", e, backoff_secs),
                    }
                    tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(30);

                    let mut cmd = std::process::Command::new(&go_bin_w);
                    cmd.current_dir(&go_dir_w);
                    if let Some(uid) = sudo_uid_w { cmd.uid(uid); }
                    if let Some(gid) = sudo_gid_w { cmd.gid(gid); }
                    if let Some(ref user) = sudo_user_w {
                        let xauth = format!("/home/{}/.Xauthority", user);
                        if std::path::Path::new(&xauth).exists() { cmd.env("XAUTHORITY", xauth); }
                        cmd.env("HOME", format!("/home/{}", user));
                        cmd.env("USER", user);
                    }
                    cmd.env("DISPLAY", ":0");
                    cmd.env("LOG_FILE_PATH", &log_path_str);

                    match cmd.spawn() {
                        Ok(new_child) => {
                            info!("[Watchdog] Go gateway respawned successfully.");
                            current_child = new_child;
                        }
                        Err(e) => {
                            error!("[Watchdog] Cannot respawn Go gateway: {}. Watchdog exiting.", e);
                            break;
                        }
                    }
                }
            });
        }
        Err(e) => {
            error!("✖ Cannot launch Go gateway server: {}", e);
        }
    }

    // Run the automated ADB workstation IP discovery broadcast in a background thread
    broadcast_workstation_ips_to_adb();

    // 5. Main connection orchestrator loop
    loop {
        info!("Waiting for Go Signaling Client to connect over UDS...");
        
        match uds_server.accept().await {
            Ok(stream) => {
                let (mut read_half, mut write_half) = stream.into_split();
                let dev_clone = Arc::clone(&device);
                let config_for_capture = Arc::clone(&config);
                
                let session_active = Arc::new(AtomicBool::new(true));
                let session_active_clone1 = Arc::clone(&session_active);
                let session_active_clone2 = Arc::clone(&session_active);

                // Shared streaming settings
                let target_fps = Arc::new(std::sync::atomic::AtomicU32::new(60));
                let target_fps_clone1 = Arc::clone(&target_fps);
                let target_fps_clone2 = Arc::clone(&target_fps);

                let active_codec = Arc::new(Mutex::new("h264".to_string()));
                let active_codec_clone1 = Arc::clone(&active_codec);
                let active_codec_clone2 = Arc::clone(&active_codec);

                let force_keyframe = Arc::new(std::sync::atomic::AtomicBool::new(true)); // Start with true!
                let force_keyframe_clone1 = Arc::clone(&force_keyframe);
                let force_keyframe_clone2 = Arc::clone(&force_keyframe);

                // Spawn independent async UDS command listener task
                let command_task = tokio::spawn(async move {
                    info!("UDS Command Listener spawned.");
                    loop {
                        match read_command(&mut read_half).await {
                            Ok(Some(cmd)) => {
                                match cmd {
                                    Command::StreamConfig { backpressure: _, codec, target_fps } => {
                                        info!("Received StreamConfig update: codec={}, target_fps={}", codec, target_fps);
                                        target_fps_clone1.store(target_fps, Ordering::SeqCst);
                                        let mut c = active_codec_clone1.lock().await;
                                        *c = codec;
                                        force_keyframe_clone1.store(true, Ordering::SeqCst);
                                    }
                                    _ => {
                                        let mut dev = dev_clone.lock().await;
                                        if let Err(e) = execute_command(&mut *dev, cmd) {
                                            error!("Failed to inject virtual command event: {}", e);
                                        }
                                    }
                                }
                            }
                            Ok(None) => {
                                info!("Go client disconnected (EOF). Tearing down listener task.");
                                break;
                            }
                            Err(e) => {
                                error!("UDS read error: {}", e);
                                break;
                            }
                        }
                    }
                    session_active_clone1.store(false, Ordering::SeqCst);
                });

                let force_keyframe_active = Arc::clone(&force_keyframe_clone2);

                let count_nal_types = |data: &[u8]| -> (usize, usize, usize) {
                    let mut sps = 0;
                    let mut pps = 0;
                    let mut idr = 0;
                    let mut i = 0;
                    while i < data.len().saturating_sub(3) {
                        if data[i] == 0 && data[i+1] == 0 {
                            let (start_len, type_idx) = if data[i+2] == 1 {
                                (3, i + 3)
                            } else if data[i+2] == 0 && data[i+3] == 1 {
                                (4, i + 4)
                            } else {
                                i += 1;
                                continue;
                            };
                            
                            if type_idx < data.len() {
                                let nal_type = data[type_idx] & 0x1F;
                                if nal_type == 7 { sps += 1; }
                                else if nal_type == 8 { pps += 1; }
                                else if nal_type == 5 { idr += 1; }
                            }
                            i += start_len;
                        } else {
                            i += 1;
                        }
                    }
                    (sps, pps, idr)
                };

                // Spawn independent async screen capture and transmission loop task
                let shared_capturer_clone = Arc::clone(&shared_capturer);
                let capture_task = tokio::spawn(async move {
                    info!("UDS Screen Capture and Video Stream task spawned.");

                    let mut h264_encoder: Option<openh264::encoder::Encoder> = None;
                    let mut yuv_buf: Option<FullRangeYUVBuffer> = None;
                    let mut first_frame = true;
                    let mut frame_count = 0u64;
                    let stream_start_time = tokio::time::Instant::now();

                    // Lock the capturer for the duration of this session
                    let mut capturer = shared_capturer_clone.lock().await;
                    let width = capturer.width();
                    let height = capturer.height();
                    let mut rgb_buf = vec![0u8; (width * height * 3) as usize];
                    info!("Capture resolution: {}x{} via {}", width, height, capturer.backend_name());

                    while session_active_clone2.load(Ordering::SeqCst) {
                        let frame_start = tokio::time::Instant::now();

                        match capturer.capture_frame() {
                            Ok(Some(pixels)) => {
                                let ts = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap()
                                    .as_millis() as u64;

                                if first_frame {
                                    info!("First Frame Captured");
                                    info!("Resolution: {}x{}", width, height);
                                    info!("Raw Frame Size: {} bytes", pixels.len());
                                    info!("Timestamp: {}", ts);
                                }

                                let codec = {
                                    let c = active_codec_clone2.lock().await;
                                    c.clone()
                                };

                                if codec == "h264" {
                                    let encoder = match &mut h264_encoder {
                                        Some(enc) => enc,
                                        None => {
                                            let config = openh264::encoder::EncoderConfig::new(width, height)
                                                .set_bitrate_bps(config_for_capture.openh264_settings.target_bitrate_bps)
                                                .enable_skip_frame(config_for_capture.openh264_settings.enable_skip_frame);
                                            let enc = match openh264::encoder::Encoder::with_config(config) {
                                                Ok(enc) => enc,
                                                Err(e) => {
                                                    error!("Failed to initialize OpenH264 encoder: {:?}", e);
                                                    return;
                                                }
                                            };
                                            h264_encoder = Some(enc);
                                            h264_encoder.as_mut().unwrap()
                                        }
                                    };

                                    // BGRA → RGB conversion
                                    let src_chunks = pixels.chunks_exact(4);
                                    let dst_chunks = rgb_buf.chunks_exact_mut(3);
                                    for (src, dst) in src_chunks.zip(dst_chunks) {
                                        dst[0] = src[2]; // R
                                        dst[1] = src[1]; // G
                                        dst[2] = src[0]; // B
                                    }

                                    let yuv = match &mut yuv_buf {
                                        Some(buf) => {
                                            buf.read_rgb(&rgb_buf);
                                            buf
                                        }
                                        None => {
                                            let mut buf = FullRangeYUVBuffer::new(width as usize, height as usize);
                                            buf.read_rgb(&rgb_buf);
                                            yuv_buf = Some(buf);
                                            yuv_buf.as_mut().unwrap()
                                        }
                                    };

                                    frame_count += 1;
                                    let force_iframe = force_keyframe_active.swap(false, Ordering::SeqCst)
                                        || (frame_count % 300 == 0);
                                    if force_iframe {
                                        unsafe {
                                            let _ = encoder.raw_api().force_intra_frame(true);
                                        }
                                    }

                                    let bytes = match encoder.encode(yuv) {
                                        Ok(bitstream) => {
                                            let bytes_vec = bitstream.to_vec();
                                            let (sps, pps, idr) = count_nal_types(&bytes_vec);
                                            if first_frame {
                                                info!("SPS Generated: {}", sps > 0);
                                                info!("PPS Generated: {}", pps > 0);
                                                info!("IDR Generated: {}", idr > 0);
                                                info!("Encoded Frame Size: {} bytes", bytes_vec.len());
                                            } else if sps > 0 || pps > 0 || idr > 0 {
                                                info!("NAL config: SPS={} PPS={} IDR={}", sps, pps, idr);
                                            }
                                            Some(bytes_vec)
                                        }
                                        Err(e) => {
                                            error!("OpenH264 encode error: {:?}", e);
                                            None
                                        }
                                    };

                                    if let Some(bytes) = bytes {
                                        if let Err(e) = send_frame(&mut write_half, width, height, ts, bytes.len() as u32, &bytes).await {
                                            error!("Failed to stream H.264 frame: {}", e);
                                            break;
                                        }
                                        if first_frame {
                                            info!("First Frame Sent To UDS — {} bytes, t={}ms",
                                                bytes.len(), stream_start_time.elapsed().as_millis());
                                            first_frame = false;
                                        }
                                    }
                                } else {
                                    // MJPEG mode
                                    let payload_len = (width * height * 4) as u32;
                                    if let Err(e) = send_frame(&mut write_half, width, height, ts, payload_len, &pixels).await {
                                        error!("Failed to stream MJPEG frame: {}", e);
                                        break;
                                    }
                                }
                            }
                            Ok(None) => {
                                // No new frame — screen unchanged
                            }
                            Err(e) => {
                                error!("Capture error: {}", e);
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        }

                        let fps = target_fps_clone2.load(Ordering::SeqCst);
                        let budget_ms = if fps > 0 { 1000 / fps } else { 16 };
                        let elapsed = frame_start.elapsed().as_millis() as u64;
                        if elapsed < budget_ms as u64 {
                            tokio::time::sleep(Duration::from_millis(budget_ms as u64 - elapsed)).await;
                        }
                    }

                    info!("Capture loop stopped.");
                });

                // R4 — Panic-safe command task + graceful session teardown
                match command_task.await {
                    Ok(_) => info!("Command task completed normally."),
                    Err(e) if e.is_panic() => error!("Command task panicked: {:?}", e),
                    Err(e) => error!("Command task join error: {:?}", e),
                }
                capture_task.abort();
                let _ = capture_task.await;
                info!("Session resources released successfully.");
            }
            Err(e) => {
                error!("Socket connection accept failure: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

/// Helper method to execute virtual mouse/keyboard event injection commands.
fn execute_command(device: &mut UInputDevice, cmd: Command) -> Result<(), std::io::Error> {
    info!("Injecting Virtual Event: {:?}", cmd);
    match cmd {
        Command::Key { key_code, pressed } => {
            device.inject_key(key_code, pressed)
        }
        Command::MouseRelative { dx, dy } => {
            device.inject_mouse_move_relative(dx, dy)
        }
        Command::MouseAbsolute { x, y, max_x, max_y } => {
            device.inject_mouse_move_absolute(x, y, max_x, max_y)
        }
        Command::MouseClick { button, pressed } => {
            device.inject_mouse_click(button, pressed)
        }
        Command::MouseScroll { steps } => {
            device.inject_mouse_scroll(steps)
        }
        Command::StreamConfig { .. } => {
            Ok(())
        }
    }
}


