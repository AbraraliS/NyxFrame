use std::env;
use std::process::Command;

#[derive(Clone, Debug)]
pub struct EnvState {
    pub wayland_confidence: u32,
    pub x11_confidence: u32,
    pub xdg_session_type: Option<String>,
    pub xdg_current_desktop: Option<String>,
    pub display: Option<String>,
    pub wayland_display: Option<String>,
    pub has_xdotool: bool,
    pub has_xclip: bool,
    pub has_adb: bool,
}

impl EnvState {
    pub fn print_report(&self) {
        println!("--- NyxFrame Session Diagnostics Report ---");
        println!("XDG_SESSION_TYPE: {:?}", self.xdg_session_type);
        println!("XDG_CURRENT_DESKTOP: {:?}", self.xdg_current_desktop);
        println!("DISPLAY: {:?}", self.display);
        println!("WAYLAND_DISPLAY: {:?}", self.wayland_display);
        println!("xdotool installed: {}", self.has_xdotool);
        println!("xclip installed: {}", self.has_xclip);
        println!("adb installed: {}", self.has_adb);
        println!("Wayland confidence: {}", self.wayland_confidence);
        println!("X11 confidence: {}", self.x11_confidence);
        println!("-------------------------------------------");
    }
}

pub fn recover_environment(_sudo_uid: Option<u32>, _sudo_user: Option<String>) -> EnvState {
    let xdg_session_type = env::var("XDG_SESSION_TYPE").ok();
    let xdg_current_desktop = env::var("XDG_CURRENT_DESKTOP").ok();
    let display = env::var("DISPLAY").ok();
    let wayland_display = env::var("WAYLAND_DISPLAY").ok();

    let has_xdotool = Command::new("which")
        .arg("xdotool")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    let has_xclip = Command::new("which")
        .arg("xclip")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    let has_adb = Command::new("which")
        .arg("adb")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);

    let mut wayland_confidence = 0;
    let mut x11_confidence = 0;

    if xdg_session_type.as_deref() == Some("wayland") {
        wayland_confidence = 100;
    } else if xdg_session_type.as_deref() == Some("x11") {
        x11_confidence = 100;
    } else {
        if wayland_display.is_some() {
            wayland_confidence = 80;
        }
        if display.is_some() {
            x11_confidence = 80;
        }
    }

    EnvState {
        wayland_confidence,
        x11_confidence,
        xdg_session_type,
        xdg_current_desktop,
        display,
        wayland_display,
        has_xdotool,
        has_xclip,
        has_adb,
    }
}

pub fn validate_backends(env: &EnvState) -> bool {
    if !env.has_xdotool {
        println!("[WARN] xdotool is missing. Input emulation will fail.");
    }
    if !env.has_xclip {
        println!("[WARN] xclip is missing. Clipboard sync will fail.");
    }
    env.wayland_confidence > 0 || env.x11_confidence > 0
}
