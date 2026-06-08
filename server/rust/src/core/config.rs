use serde::Deserialize;
use std::path::Path;
use log::info;

#[derive(Deserialize, Debug, Clone)]
pub struct OpenH264Config {
    pub target_bitrate_bps: u32,
    pub enable_skip_frame: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NvencConfig {
    pub target_bitrate_bps: u32,
    pub max_bitrate_bps: u32,
    pub buffer_size_kb: u32,
    pub keyframe_interval: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerSideConfig {
    pub network_port: String,
    pub uds_socket_path: String,
    pub window_manager_cmd: String,
    pub default_discovery_hostname: String,
    pub default_capture_protocol: String,
    pub openh264_settings: OpenH264Config,
    pub nvenc_settings: NvencConfig,
    pub log_dir: Option<String>,
}

pub fn load_server_config(exe_dir: &Path) -> ServerSideConfig {
    let paths = vec![
        exe_dir.join("server/config.json"),
        exe_dir.join("../../../server/config.json"),
        exe_dir.join("../../../config.json"),
        std::path::PathBuf::from("server/config.json"),
        std::path::PathBuf::from("config.json"),
    ];

    for p in paths {
        if p.exists() {
            if let Ok(file_content) = std::fs::read_to_string(&p) {
                if let Ok(conf) = serde_json::from_str::<ServerSideConfig>(&file_content) {
                    info!("[INFO] Config JSON loaded successfully from: {:?}", p);
                    return conf;
                }
            }
        }
    }

    info!("[INFO] config.json not found or failed to parse. Using hardcoded system defaults.");
    ServerSideConfig {
        network_port: "9090".to_string(),
        uds_socket_path: "/tmp/nyxframe3.sock".to_string(),
        window_manager_cmd: "i3-msg".to_string(),
        default_discovery_hostname: "nyxframe".to_string(),
        default_capture_protocol: "x11".to_string(),
        openh264_settings: OpenH264Config {
            target_bitrate_bps: 5_000_000,
            enable_skip_frame: false,
        },
        nvenc_settings: NvencConfig {
            target_bitrate_bps: 3_000_000,
            max_bitrate_bps: 4_000_000,
            buffer_size_kb: 256,
            keyframe_interval: 300,
        },
        log_dir: Some("/var/log/nyxframe/logs".to_string()),
    }
}
