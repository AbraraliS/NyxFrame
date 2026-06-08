# NyxFrame Repository Structure: Cross-Platform Evolution

**Date:** 2026-06-08
**Status:** Design Document
**Author:** Architecture Analysis

---

## 1. Current Architecture Map

### 1.1 Top-Level Layout

```
nyxframe/
├── android/          # Android client (Kotlin/Jetpack Compose)
├── server/           # Host-side runtime (dual-process: Rust + Go)
├── docs/             # Wiki documentation
├── build_server.sh   # Rust + Go build script
├── build_android.sh  # Android APK build script
└── generate_icons.py # Icon generation
```

### 1.2 Current server/ Internal Structure

```
server/
├── rust/                         # Linux Rust engine (root daemon)
│   ├── Cargo.toml                # Dependencies: xcb, gstreamer, tokio, ashpd, libc
│   └── src/
│       ├── main.rs               # 907 lines: entry, config, UDS server, Go watchdog
│       ├── capture/
│       │   ├── mod.rs            # ScreenCapturer enum, auto_select()
│       │   ├── x11.rs            # Stub - xcb SHM+RandR
│       │   └── wayland.rs        # Working - GStreamer PipeWire + ashpd portal
│       ├── input/
│       │   ├── mod.rs            # Module declaration
│       │   └── uinput.rs         # 273 lines: /dev/uinput virtual device
│       ├── ipc/
│       │   ├── mod.rs            # Module declaration
│       │   └── uds.rs            # 154 lines: UDS server, frame+command protocol
│       └── env_recovery.rs       # 318 lines: DISPLAY, XAUTHORITY, DBUS recovery
│
├── go/                            # Go gateway (user process)
│   ├── go.mod                    # Dependencies: gorilla/websocket, pion/webrtc, go-libjpeg
│   └── main.go                   # 1792 lines: HTTP server, WebSocket, WebRTC, file transfer,
│                                 #            xdotool, xclip, discovery, session management
│
├── config.json                   # Runtime configuration
└── config.json.example           # Example config
```

### 1.3 Current Data Flow

```
   Android App                    Linux Host
   ┌──────────┐                  ┌──────────────────────────────────┐
   │          │   WebSocket      │  Go Gateway (port 9090)          │
   │  Touch   │◄──────────────►  │  ├── /ws      ← input commands  │
   │  Video   │   H.264 stream   │  ├── /stream  → frame stream     │
   │  Input   │                  │  ├── /offer   ← WebRTC SDP       │
   │  Files   │                  │  ├── /api/fs/* ← file transfer   │
   │          │                  │  └── xdotool/xclip (input exec)  │
   └──────────┘                  │                    │             │
                                  │                    │ UDS socket │
                                  │                    ▼             │
                                  │  Rust Engine (root daemon)       │
                                  │  ├── Capture: X11/Wayland       │
                                  │  ├── Input: uinput (kernel)     │
                                  │  └── IPC: command/frame protocol│
                                  └──────────────────────────────────┘
```

### 1.4 Platform Coupling Summary

| File | Lines | Platform Dependency | Coupling |
|------|-------|-------------------|----------|
| `rust/src/capture/x11.rs` | 22 | `xcb` crate + X11 libs | Linux |
| `rust/src/capture/wayland.rs` | 114 | `gstreamer` + `ashpd` + PipeWire | Linux |
| `rust/src/input/uinput.rs` | 273 | `/dev/uinput`, `libc` ioctl | Linux |
| `rust/src/ipc/uds.rs` | 154 | Unix Domain Socket | Unix |
| `rust/src/env_recovery.rs` | 318 | `loginctl`, `/proc`, `/run/user/` | Linux |
| `rust/src/main.rs` | 907 | All of the above | Linux |
| `go/main.go` | 1792 | `xdotool`, `xclip`, `fuser`, `i3-msg` | Linux |

**Total Linux-coupled code: ~3,600 lines**

---

## 2. Ownership Boundaries

### 2.1 Design Principle

Each piece of code has exactly one owner. Ownership is determined by:

1. **Is the code OS-specific?** → Owned by platform crate (`linux/`, `windows/`, `macos/`)
2. **Is the code pure business logic?** → Owned by `core/`
3. **Is the code a trait/interface?** → Owned by `core/` (defines contract), implemented by platform crate

### 2.2 Ownership Matrix

```
┌─────────────────────┬──────────┬──────────┬───────────┬───────────┐
│ Component           │ core/    │ linux/   │ windows/  │ macos/    │
├─────────────────────┼──────────┼──────────┼───────────┼───────────┤
│ Capture trait       │  OWN     │ impl     │ impl      │ impl      │
│ Input trait         │  OWN     │ impl     │ impl      │ impl      │
│ HTTP server         │  OWN     │          │           │           │
│ WebSocket server    │  OWN     │          │           │           │
│ WebRTC signaling    │  OWN     │          │           │           │
│ File transfer       │  OWN     │          │           │           │
│ Device discovery    │  OWN     │          │           │           │
│ Session management  │  OWN     │          │           │           │
│ Config              │  OWN     │          │           │           │
│ Protocol types      │  OWN     │          │           │           │
│ X11 capture         │          │  OWN     │           │           │
│ Wayland capture     │          │  OWN     │           │           │
│ uinput input        │          │  OWN     │           │           │
│ Env recovery        │          │  OWN     │           │           │
│ DXGI capture        │          │          │  OWN      │           │
│ SendInput           │          │          │  OWN      │           │
│ AVFoundation capt.  │          │          │           │  OWN      │
│ CGEvent input       │          │          │           │  OWN      │
└─────────────────────┴──────────┴──────────┴───────────┴───────────┘
```

### 2.3 Why File Transfer Goes to core/

File transfer logic is:

- **Pure I/O operations** (read file, write file, list directory, zip archive)
- **No OS-specific syscalls** beyond standard file operations
- **HTTP-based** (multipart upload, streaming download)
- **No capture, input, or display dependencies**

It belongs in `core/` because it has zero platform coupling. The Rust `std::fs` and `std::io` APIs are identical on Linux, Windows, and macOS. Putting it in a platform crate would force every platform to reimplement the same logic.

The Go file transfer code (`POST /api/fs/upload`, `POST /api/fs/download`, `GET /api/fs/list`) moves to `core/src/file_transfer/` as pure Rust, with HTTP handlers in `core/src/net/`.

### 2.4 Why the Go Gateway Is Eliminated

The Go gateway exists because:

- Originally handled HTTP/WS/WebRTC (Go is good at this)
- Executed xdotool commands in user space (complementing kernel-level uinput)

As of this design:

- **HTTP/WS/WebRTC** → Rust (`core/src/net/`) — `axum` or `actix-web` handles this natively
- **xdotool commands** → Move to Rust `linux/src/input/` alongside uinput, using `xcb` or direct X11/Wayland protocol for input where possible
- **File transfer** → Rust (`core/src/file_transfer/`)
- **Clipboard** → Rust (`linux/src/input/` via X11/Wayland clipboard protocols)

**Eliminating Go means:**
- Single process (no IPC, no UDS, no watchdog)
- One build system (Cargo only)
- One language to maintain
- Cross-compilation to any target from any host
- Smaller binary, simpler deployment

---

## 3. Shared vs Platform-Specific Matrix

### 3.1 Classification

| Category | Code | Current Location | Destination |
|----------|------|-----------------|-------------|
| **Platform-independent** | Network server (HTTP/WS/WRTC) | `go/main.go` | `core/src/net/` |
| | File transfer | `go/main.go` | `core/src/file_transfer/` |
| | Device discovery protocol | `go/main.go` + `rust/src/main.rs` | `core/src/discovery/` |
| | Session management | `go/main.go` | `core/src/session/` |
| | Config loading/parsing | `go/main.go` + `rust/src/main.rs` | `core/src/config/` |
| | Protocol types (Command, Frame) | `rust/src/ipc/uds.rs` | `core/src/protocol/` |
| | Capture trait | *does not exist* | `core/src/capture/mod.rs` |
| | Input trait | *does not exist* | `core/src/input/mod.rs` |
| **Linux-specific** | X11 capture | `rust/src/capture/x11.rs` | `linux/src/capture/x11.rs` |
| | Wayland capture | `rust/src/capture/wayland.rs` | `linux/src/capture/wayland.rs` |
| | uinput driver | `rust/src/input/uinput.rs` | `linux/src/input/uinput.rs` |
| | Env recovery | `rust/src/env_recovery.rs` | `linux/src/env_recovery.rs` |
| | xdotool/xclip/clipboard | `go/main.go` | `linux/src/input/` |
| **Unix-specific (POSIX)** | UDS IPC | `rust/src/ipc/uds.rs` | **Delete** (no longer needed) |
| **Windows (future)** | DXGI capture | *does not exist* | `windows/src/capture/dxgi.rs` |
| | SendInput | *does not exist* | `windows/src/input/sendinput.rs` |
| | Win32 clipboard | *does not exist* | `windows/src/input/clipboard.rs` |
| **macOS (future)** | AVFoundation capture | *does not exist* | `macos/src/capture/avfoundation.rs` |
| | CGEvent input | *does not exist* | `macos/src/input/cgevent.rs` |
| | NSPasteboard | *does not exist* | `macos/src/input/clipboard.rs` |

### 3.2 Abstraction Pattern: Capture Trait

```rust
// core/src/capture/mod.rs — OWNED by core/
pub trait CaptureBackend: Send {
    /// Initialize the capture backend
    async fn init(&mut self, config: &CaptureConfig) -> Result<()>;

    /// Capture a single frame. Returns raw BGRA or NV12 data.
    async fn capture_frame(&mut self) -> Result<CapturedFrame>;

    /// Reconfigure capture parameters at runtime
    async fn reconfigure(&mut self, config: &CaptureConfig) -> Result<()>;
}

// linux/src/capture/x11.rs — IMPLEMENTS the trait
pub struct X11Capturer { /* ... */ }
impl CaptureBackend for X11Capturer { /* ... */ }

// linux/src/capture/wayland.rs — IMPLEMENTS the trait
pub struct WaylandCapturer { /* ... */ }
impl CaptureBackend for WaylandCapturer { /* ... */ }
```

### 3.3 Abstraction Pattern: Input Trait

```rust
// core/src/input/mod.rs — OWNED by core/
pub trait InputBackend: Send {
    async fn inject_key(&self, key: KeyCode, pressed: bool) -> Result<()>;
    async fn inject_mouse_move(&self, x: f64, y: f64, absolute: bool) -> Result<()>;
    async fn inject_mouse_click(&self, button: MouseButton, pressed: bool) -> Result<()>;
    async fn inject_mouse_scroll(&self, dx: f32, dy: f32) -> Result<()>;
    async fn inject_text(&self, text: &str) -> Result<()>;
    async fn clipboard_get(&self) -> Result<String>;
    async fn clipboard_set(&self, text: &str) -> Result<()>;
}

// linux/src/input/uinput.rs — IMPLEMENTS via kernel
pub struct UinputBackend { /* ... */ }
impl InputBackend for UinputBackend { /* ... */ }
```

---

## 4. Proposed Repository Structure

```
nyxframe/
├── android/                  # UNCHANGED — Android client
│   └── ...                   # (Jetpack Compose, WebSocket, WebRTC, H.264)
│
├── server/                   # RESTRUCTURED — Workspace
│   ├── Cargo.toml            # [workspace] members = ["core", "linux", "server"]
│   ├── config.json           # STAYS — Runtime config
│   ├── config.json.example   # STAYS — Example config
│   │
│   ├── core/                 # NEW — Platform-independent library
│   │   ├── Cargo.toml        # Pure Rust, no OS-specific deps
│   │   └── src/
│   │       ├── lib.rs        # Public API, re-exports
│   │       ├── net/
│   │       │   ├── mod.rs    # HTTP, WebSocket, WebRTC server
│   │       │   ├── http.rs   # REST API handlers
│   │       │   ├── ws.rs     # WebSocket command/stream
│   │       │   └── webrtc.rs # WebRTC signaling
│   │       ├── file_transfer/
│   │       │   ├── mod.rs    # Upload, download, list, mkdir
│   │       │   └── zip.rs    # Multi-file packaging
│   │       ├── discovery/
│   │       │   ├── mod.rs    # Device discovery protocols
│   │       │   └── adb.rs    # ADB broadcast
│   │       ├── session/
│   │       │   └── mod.rs    # Session state, connections
│   │       ├── config/
│   │       │   └── mod.rs    # Config loading/serialization
│   │       ├── protocol/
│   │       │   └── mod.rs    # Command enum, FrameHeader, etc.
│   │       ├── capture/
│   │       │   ├── mod.rs    # CaptureBackend trait
│   │       │   └── types.rs  # CapturedFrame, CaptureConfig
│   │       └── input/
│   │           ├── mod.rs    # InputBackend trait
│   │           └── types.rs  # KeyCode, MouseButton, etc.
│   │
│   ├── linux/                # MOVED/RENAMED — Linux-specific implementations
│   │   ├── Cargo.toml        # Deps: xcb, gstreamer, ashpd
│   │   └── src/
│   │       ├── lib.rs        # Re-exports Linux backends
│   │       ├── capture/
│   │       │   ├── mod.rs    # Auto-select: Wayland → X11
│   │       │   ├── x11.rs    # MOVED from rust/src/capture/x11.rs
│   │       │   └── wayland.rs# MOVED from rust/src/capture/wayland.rs
│   │       ├── input/
│   │       │   ├── mod.rs    # Module, factory function
│   │       │   ├── uinput.rs # MOVED from rust/src/input/uinput.rs
│   │       │   ├── xdotool.rs# NEW — replaces Go xdotool/text/clipboard
│   │       │   └── clipboard.rs # NEW — X11/Wayland clipboard
│   │       └── env_recovery.rs# MOVED from rust/src/env_recovery.rs
│   │
│   ├── windows/              # FUTURE — Windows-specific implementations
│   │   ├── Cargo.toml        # Deps: windows-rs, dxgi, etc.
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capture/
│   │       │   ├── mod.rs
│   │       │   └── dxgi.rs   # DXGI desktop duplication
│   │       └── input/
│   │           ├── mod.rs
│   │           └── sendinput.rs # SendInput API
│   │
│   ├── macos/                # FUTURE — macOS-specific implementations
│   │   ├── Cargo.toml        # Deps: objc2, AVFoundation, etc.
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capture/
│   │       │   ├── mod.rs
│   │       │   └── avfoundation.rs # AVFoundation screen capture
│   │       └── input/
│   │           ├── mod.rs
│   │           └── cgevent.rs # CGEvent API
│   │
│   └── server/               # NEW — Binary entry point
│       ├── Cargo.toml        # Depends on core + platform crate
│       └── src/
│           └── main.rs       # Replaces rust/src/main.rs
│
├── docs/                     # UNCHANGED — Documentation
│   ├── superpowers/
│   │   └── specs/
│   │       └── 2026-06-08-repository-cross-platform-structure-design.md
│   ├── Home.md
│   └── ...
│
├── build_server.sh           # REMOVED — replaced by `cargo build --release`
├── build_android.sh          # STAYS — Android build separate
├── build_and_install_android.sh  # STAYS
├── package.json              # STAYS — icon generation
├── generate_icons.py         # STAYS
└── fix_settings.py           # STAYS
```

### 4.1 Crate Dependency Graph

```
server (binary)
  ├── core      (platform-independent library)
  └── linux     (platform-specific library)
       └── core (depends on core for traits and types)
```

```
Dependency direction:
  server → core
  server → linux
  linux  → core

No circular dependencies.
core has zero platform dependencies — compiles on any Rust target.
```

### 4.2 `server/Cargo.toml` (Workspace)

```toml
[workspace]
resolver = "2"
members = [
    "core",
    "linux",
    "server",
]

[workspace.package]
version = "2.1.0"
edition = "2021"
license = "MIT"
```

### 4.3 `server/server/Cargo.toml` (Binary)

```toml
[package]
name = "nyxframe-server"
version.workspace = true
edition.workspace = true

[dependencies]
nyxframe-core = { path = "../core" }
nyxframe-linux = { path = "../linux" }
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }

[target.'cfg(target_os = "linux")'.dependencies]
nyxframe-linux = { path = "../linux" }

# Future:
# [target.'cfg(target_os = "windows")'.dependencies]
# nyxframe-windows = { path = "../windows" }
# [target.'cfg(target_os = "macos")'.dependencies]
# nyxframe-macos = { path = "../macos" }
```

### 4.4 `server/core/Cargo.toml`

```toml
[package]
name = "nyxframe-core"
version.workspace = true
edition.workspace = true

[dependencies]
# Web framework
axum = { version = "0.7", features = ["ws", "multipart"] }
# WebRTC
webrtc = "0.11"
# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# Async
tokio = { version = "1", features = ["full"] }
# Config
toml = "0.8"
# Logging
tracing = "0.1"
tracing-subscriber = "0.3"
# File transfer
zip = "2"
uuid = { version = "1", features = ["v4"] }
# Protocol
byteorder = "1"
```

### 4.5 `server/linux/Cargo.toml`

```toml
[package]
name = "nyxframe-linux"
version.workspace = true
edition.workspace = true

[dependencies]
nyxframe-core = { path = "../core" }
xcb = { version = "1", features = ["shm", "randr"] }
gstreamer = { version = "0.23", optional = true }
gstreamer-app = { version = "0.23", optional = true }
ashpd = "0.9"
libc = "0.2"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"

[features]
default = ["wayland-capture"]
wayland-capture = ["gstreamer", "gstreamer-app"]

[target.'cfg(target_os = "linux")'.dependencies]
# Linux-only crates
```

---

## 5. Exact Folder Move Recommendations

### 5.1 What Moves

| Source | Destination | Reason |
|--------|-------------|--------|
| `server/rust/src/main.rs` | `server/server/src/main.rs` | Thin binary, platform selection only |
| `server/rust/src/capture/` | `server/linux/src/capture/` | Linux-specific capture backends |
| `server/rust/src/input/uinput.rs` | `server/linux/src/input/uinput.rs` | Linux kernel input |
| `server/rust/src/env_recovery.rs` | `server/linux/src/env_recovery.rs` | Linux-specific env diagnostics |
| `server/go/main.go` (file transfer) | `server/core/src/file_transfer/` | Cross-platform, pure Rust |
| `server/go/main.go` (HTTP/WS/WRTC) | `server/core/src/net/` | Cross-platform networking |
| `server/go/main.go` (discovery) | `server/core/src/discovery/` | Cross-platform discovery protocol |
| `server/go/main.go` (session) | `server/core/src/session/` | Cross-platform session state |
| `server/rust/src/ipc/` | **Delete** | Single process, no IPC needed |
| `server/rust/src/main.rs` (config) | `server/core/src/config/` | Cross-platform config |
| `server/rust/src/ipc/` (protocol types) | `server/core/src/protocol/` | Cross-platform protocol definitions |

### 5.2 What Stays

| File | Reason |
|------|--------|
| `android/` | Unchanged — Android client is independent |
| `docs/` | Unchanged — documentation |
| `server/config.json` | Stays as runtime config |
| `server/config.json.example` | Stays as example |
| `build_android.sh` | Stays — Android build is separate |
| `build_and_install_android.sh` | Stays — helper script |
| `package.json` | Stays — icon generation |
| `generate_icons.py` | Stays — icon generation |

### 5.3 What Is Created New

| File | Content |
|------|---------|
| `server/Cargo.toml` | Workspace root |
| `server/core/` | Cross-platform library crate |
| `server/core/Cargo.toml` | Pure Rust dependencies |
| `server/core/src/lib.rs` | Public API |
| `server/core/src/net/` | HTTP, WS, WebRTC server |
| `server/core/src/capture/mod.rs` | `CaptureBackend` trait |
| `server/core/src/input/mod.rs` | `InputBackend` trait |
| `server/core/src/protocol/` | `Command`, `FrameHeader` types |
| `server/linux/Cargo.toml` | Linux crate manifest |
| `server/linux/src/lib.rs` | Linux crate root |
| `server/linux/src/input/xdotool.rs` | xdotool text/clipboard |
| `server/linux/src/input/clipboard.rs` | X11/Wayland clipboard |
| `server/server/Cargo.toml` | Binary crate manifest |
| `server/server/src/main.rs` | Entry point |

### 5.4 What Is Deleted

| Path | Reason |
|------|--------|
| `server/go/` | Go gateway eliminated |
| `server/rust/` | Replaced by core/ + linux/ + server/ |
| `build_server.sh` | Replaced by `cargo build --release` |
| `nyxframe-server` (binary at root) | Generated by `cargo build`, not committed |

---

## 6. Migration Order

### Phase 1: Create Workspace Structure

**Goal:** Set up the Cargo workspace without moving any code yet.

1. Create `server/Cargo.toml` as workspace root
2. Create `server/core/` directory structure (no implementation yet)
3. Create `server/linux/` directory structure (no implementation yet)
4. Create `server/server/` directory structure (no implementation yet)
5. Verify `cargo build` works with empty crates

**Risks:** None — no code is moved yet.

### Phase 2: Extract Core Library

**Goal:** Move platform-independent code into `core/`.

1. Move protocol types (`Command`, `FrameHeader`) from `rust/src/ipc/` to `core/src/protocol/`
2. Move config logic from `rust/src/main.rs` to `core/src/config/`
3. Move networking (HTTP, WS, WebRTC) from `go/main.go` to `core/src/net/`
4. Move session management from `go/main.go` to `core/src/session/`
5. Move discovery from `go/main.go` to `core/src/discovery/`
6. Extract `CaptureBackend` trait to `core/src/capture/mod.rs`
7. Extract `InputBackend` trait to `core/src/input/mod.rs`

**Risks:**
- Go network code needs translation to Rust (axum or actix-web)
- WebRTC library differences between Go pion vs Rust webrtc-rs
- Service discovery (mDNS/ADB) needs Rust equivalents

### Phase 3: Move Linux Backends

**Goal:** Move all Linux-specific code to `linux/` crate.

1. Move `capture/x11.rs` → `linux/src/capture/x11.rs`
2. Move `capture/wayland.rs` → `linux/src/capture/wayland.rs`
3. Move `input/uinput.rs` → `linux/src/input/uinput.rs`
4. Move `env_recovery.rs` → `linux/src/env_recovery.rs`
5. Implement xdotool replacement in `linux/src/input/xdotool.rs`

**Risks:** Low — mostly file moves with import path updates.

### Phase 4: Rewrite Binary Entry Point

**Goal:** `server/main.rs` becomes a thin platform selector.

```rust
// server/server/src/main.rs (conceptual)
#[cfg(target_os = "linux")]
type PlatformCapture = nyxframe_linux::capture::LinuxCapture;
#[cfg(target_os = "linux")]
type PlatformInput = nyxframe_linux::input::LinuxInput;

#[tokio::main]
async fn main() {
    let config = nyxframe_core::config::load();
    let capture = PlatformCapture::auto_select(&config).await.unwrap();
    let input = PlatformInput::new().await.unwrap();
    nyxframe_core::server::run(config, capture, input).await;
}
```

**Risks:** Medium — need to ensure all functionality is ported.

### Phase 5: Replace Go Functionality

**Goal:** Eliminate Go entirely.

1. Port HTTP API handlers from Go to Rust (axum)
2. Port WebSocket handler from Go to Rust (axum::extract::ws)
3. Port WebRTC signaling from Go to Rust
4. Port file transfer endpoints to Rust
5. Port clipboard to Rust (x11 clipboard crate or Wayland data-device)

**Risks:** 
- Most complex phase
- WebRTC in Rust has fewer examples than Go pion
- File transfer needs careful parity testing

### Phase 6: Remove Old Code

**Goal:** Clean up.

1. Delete `server/rust/`
2. Delete `server/go/`
3. Delete `build_server.sh` (document `cargo build --release` in README)

---

## 7. Risks

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| WebRTC Rust ecosystem immature | Medium | High | Keep Go WebRTC temporarily; phase migration |
| axum WebSocket different from gorilla/websocket | Low | Medium | Test all WS message types during migration |
| File transfer parity bugs | Medium | Medium | Write integration tests before removing Go |
| xdotool replacement incomplete | Medium | High | Keep xdotool as fallback via std::process::Command |
| Windows/macOS capture complexity | High | High | Design trait now; implement when ready; RustDesk code as reference |
| Clipboard API differences | Medium | Low | Per-platform clipboard impl in each platform crate |
| Build script fragmentation | Low | Medium | Single `cargo build --release` replaces shell scripts |
| Linux-specific code mixed in core/ | Medium | Medium | Strict code review: no `#[cfg(unix)]` in core/ |

---

## 8. Future: Windows & macOS Expansion

### 8.1 Adding a New Platform

When adding Windows support:

1. Create `server/windows/Cargo.toml` with `windows-rs`, `dxgi` deps
2. Implement `CaptureBackend` using DXGI Desktop Duplication API
3. Implement `InputBackend` using `SendInput` API
4. Implement clipboard using Win32 `GetClipboardData`/`SetClipboardData`
5. Add `windows` to workspace members in `server/Cargo.toml`
6. Add `#[cfg(target_os = "windows")]` block in `server/server/src/main.rs`

**Zero changes to `core/`.** The trait abstraction absorbs the new platform.

### 8.2 Platform-Specific Capabilities

Not all platforms support all features. The trait design handles this via `Result`:

```rust
pub trait InputBackend: Send {
    /// Returns Ok for supported operations, Err(Unsupported) otherwise
    async fn inject_text(&self, text: &str) -> Result<(), InputError>;
}
```

Platforms that don't support text injection (e.g., Wayland without wl_keyboard) return `InputError::Unsupported`. The server gracefully degrades.

### 8.3 RustDesk Integration Path

If using RustDesk's existing capture/input code:

- RustDesk's **scrap** crate for Windows DXGI capture → lives in `windows/src/capture/`
- RustDesk's **xcap** or similar for macOS → lives in `macos/src/capture/`
- These are single-file integrations behind the `CaptureBackend` trait

No changes to `core/` or `linux/` needed.

---

## 9. Final Recommended Repository Tree

```
nyxframe/
│
├── android/                          # Android client (Jetpack Compose)
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   ├── gradle.properties
│   ├── app/
│   │   ├── build.gradle.kts
│   │   └── src/main/
│   │       ├── AndroidManifest.xml
│   │       ├── assets/config.json
│   │       └── java/com/nyxframe/app/
│   │           ├── MainActivity.kt
│   │           ├── AgentViewModel.kt        # Central orchestrator
│   │           ├── WebSocketManager.kt       # WS client
│   │           ├── WebRtcManager.kt          # WebRTC client
│   │           ├── H264Decoder.kt            # MediaCodec decoder
│   │           ├── PipelineTracker.kt        # Metrics
│   │           └── ui/
│   │               ├── ConnectScreen.kt
│   │               ├── StreamScreen.kt
│   │               └── SettingsScreen.kt
│   └── config.json.example
│
├── server/                           # Cargo workspace
│   ├── Cargo.toml                    # [workspace] members = ["core", "linux", "server"]
│   ├── config.json                   # Runtime config
│   ├── config.json.example
│   │
│   ├── core/                         # ═══ Platform-independent ═══
│   │   ├── Cargo.toml                # Pure Rust deps only
│   │   └── src/
│   │       ├── lib.rs                # Public re-exports
│   │       ├── capture/
│   │       │   ├── mod.rs            # CaptureBackend trait
│   │       │   └── types.rs          # CapturedFrame, CaptureConfig
│   │       ├── input/
│   │       │   ├── mod.rs            # InputBackend trait
│   │       │   └── types.rs          # KeyCode, MouseButton, InputError
│   │       ├── net/
│   │       │   ├── mod.rs
│   │       │   ├── http.rs           # REST API routes
│   │       │   ├── ws.rs             # WebSocket handler
│   │       │   └── webrtc.rs         # WebRTC signaling
│   │       ├── file_transfer/
│   │       │   ├── mod.rs            # Upload/download/list/mkdir
│   │       │   └── zip.rs            # Multi-file archive
│   │       ├── discovery/
│   │       │   ├── mod.rs            # Discovery protocol
│   │       │   └── adb.rs            # ADB broadcast
│   │       ├── session/
│   │       │   └── mod.rs            # Session state
│   │       ├── config/
│   │       │   └── mod.rs            # Config load/parse
│   │       └── protocol/
│   │           └── mod.rs            # Command enum, FrameHeader
│   │
│   ├── linux/                        # ═══ Linux implementation ═══
│   │   ├── Cargo.toml                # xcb, gstreamer, ashpd, libc
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capture/
│   │       │   ├── mod.rs            # Auto-select Wayland → X11
│   │       │   ├── x11.rs            # xcb SHM+RandR
│   │       │   └── wayland.rs        # GStreamer PipeWire + ashpd
│   │       ├── input/
│   │       │   ├── mod.rs
│   │       │   ├── uinput.rs         # /dev/uinput kernel input
│   │       │   ├── xdotool.rs        # Text typing, i3-msg
│   │       │   └── clipboard.rs      # X11/Wayland clipboard
│   │       └── env_recovery.rs       # Env diagnostics
│   │
│   ├── server/                       # ═══ Binary entry point ═══
│   │   ├── Cargo.toml                # Depends on core + linux
│   │   └── src/
│   │       └── main.rs               # CLI, platform selection, launch
│   │
│   ├── windows/                      # FUTURE: Windows implementation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── capture/
│   │       │   └── dxgi.rs
│   │       └── input/
│   │           └── sendinput.rs
│   │
│   └── macos/                        # FUTURE: macOS implementation
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── capture/
│           │   └── avfoundation.rs
│           └── input/
│               └── cgevent.rs
│
├── docs/
│   ├── superpowers/specs/
│   │   └── 2026-06-08-repository-cross-platform-structure-design.md
│   ├── Home.md
│   ├── Getting-Started.md
│   ├── System-Architecture.md
│   ├── Video-Streaming-Engine.md
│   ├── Input-Emulation.md
│   ├── Macros-and-Automation.md
│   ├── System-Robustness-and-Troubleshooting.md
│   ├── CHANGELOG.md
│   ├── ALL.md
│   ├── robust_implementation_plan.md
│   └── photos/
│
├── build_android.sh                  # STAYS: separate Android build
├── build_and_install_android.sh      # STAYS
├── package.json                      # STAYS: sharp for icon generation
├── generate_icons.py                 # STAYS
├── fix_settings.py                   # STAYS
├── update_headers.py                 # STAYS
└── .gitignore
```

---

## 10. Summary of Rationale

### Why `server/core/` exists

`core/` holds every line of code that can compile on any Rust target. This includes:

- **Networking** (HTTP, WebSocket, WebRTC) — network APIs are cross-platform
- **File transfer** — pure I/O, no OS-specific calls
- **Discovery protocols** — protocol logic is cross-platform; only ADB broadcast is Linux-specific (stays in `linux/`)
- **Config, session, protocol types** — pure data structures
- **Capture/Input traits** — the abstraction boundary that lets platform crates plug in

**Owner:** Core team. No platform-specific `#[cfg]` allowed.

### Why `server/linux/` exists

`linux/` contains Linux-specific implementations of the capture and input traits, plus environment recovery that only makes sense on Linux. This is the only crate that depends on `xcb`, `gstreamer`, `/dev/uinput`, and `/proc`.

**Owner:** Linux platform team.

### Why `server/windows/` and `server/macos/` exist (future)

These will contain platform-specific implementations behind the same traits. They do not exist yet — the structure is designed so they can be added without touching `core/`.

**Owner:** Windows/macOS platform teams.

### Why the binary is `server/server/` (not `server/bin/` or `server/main/`)

The binary crate is the thinnest possible entry point: parse CLI flags, select platform backend, call `core::server::run()`. Its only job is to link `core` + platform crate together. Convention: binary crate name matches the output binary name (`nyxframe-server`).

### Why Go was eliminated

Single-process architecture eliminates:
- UDS IPC protocol (no Command/Frame serialization over socket)
- Process watchdog (no Go process to monitor)
- Two-language build (no Go toolchain requirement)
- Cross-platform duplication (WebSocket server would need porting to Win/Mac anyway)

### Why file transfer moved to Rust/core

File transfer is pure I/O on all platforms. Keeping it in Go would require Go runtime on every platform. Moving it to Rust (`core/`) means one implementation, zero platform coupling, compiles everywhere.

### Why `rust/src/` was split into `core/` + `linux/` + `server/`

The old `rust/src/` mixed:
- Platform-independent code (config, protocol types, IPC module)
- Linux-specific code (capture, input, env recovery)
- Binary entry point (main.rs with CLI parsing, platform selection, Go watchdog)

These have different lifecycles and ownership. Splitting them means:
- `core/` changes rarely, affects all platforms
- `linux/` changes with Linux-specific features/bugfixes
- `server/` changes when CLI flags or platform selection logic changes
