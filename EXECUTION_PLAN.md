# NyxFrame — Final Execution Plan

> **Status:** Ready for execution  
> **Strategy:** Incremental · Never break main · Prefer move over rewrite  
> **Assumption:** Streaming engine exists and is isolated behind interfaces

---

## Contents

- [Phase 0 — Engineering Foundation](#phase-0--engineering-foundation)
- [Phase 1 — Repository Foundation](#phase-1--repository-foundation)
- [Phase 2 — Go Gateway Refactor](#phase-2--go-gateway-refactor)
- [Phase 3 — Rust Agent Refactor](#phase-3--rust-agent-refactor)
- [Phase 4 — Android Refactor](#phase-4--android-refactor)
- [Phase 5 — Protocol Platform](#phase-5--protocol-platform)
- [Phase 6 — Discovery Layer](#phase-6--discovery-layer)
- [Phase 7 — Authentication & Security](#phase-7--authentication--security)
- [Phase 8 — Platform Layer](#phase-8--platform-layer)
- [Phase 9 — Session Management](#phase-9--session-management)
- [Phase 10 — Engine Isolation](#phase-10--engine-isolation)
- [Phase 11 — Hardware Encoder Abstraction](#phase-11--hardware-encoder-abstraction)
- [Phase 12 — File System Layer](#phase-12--file-system-layer)
- [Phase 13 — Terminal System](#phase-13--terminal-system)
- [Phase 14 — Recording & Playback](#phase-14--recording--playback)
- [Phase 15 — Plugin Runtime](#phase-15--plugin-runtime)
- [Phase 16 — Observability](#phase-16--observability)
- [Phase 17 — AI Runtime](#phase-17--ai-runtime)
- [Phase 18 — Windows Support](#phase-18--windows-support)
- [Phase 19 — macOS Support](#phase-19--macos-support)
- [Phase 20 — Future Readiness](#phase-20--future-readiness)
- [Final Deliverables](#final-deliverables)

---

## Phase 0 — Engineering Foundation

**Goal:** CI/CD, builds, release pipeline, observability infrastructure.

**Duration:** 2 weeks  
**Owner:** Infrastructure Engineer

### GitHub Actions

**File:** `.github/workflows/ci.yml`

```yaml
name: CI
on: [push, pull_request]

jobs:
  rust-linux:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --manifest-path apps/desktop-agent/Cargo.toml
      - run: cargo clippy --manifest-path apps/desktop-agent/Cargo.toml -- -D warnings
      - run: cargo test --manifest-path apps/desktop-agent/Cargo.toml

  go:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-go@v5
        with:
          go-version: '1.22'
      - run: cd services/gateway && go build ./...
      - run: cd services/gateway && go vet ./...
      - run: cd services/gateway && go test ./...

  android:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-java@v4
        with:
          distribution: 'zulu'
          java-version: '17'
      - run: cd apps/mobile-client/android && ./gradlew assembleDebug
      - run: cd apps/mobile-client/android && ./gradlew lint

  rust-windows:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --manifest-path apps/desktop-agent/Cargo.toml

  rust-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --manifest-path apps/desktop-agent/Cargo.toml
```

**File:** `.github/workflows/release.yml`

```yaml
name: Release
on:
  push:
    tags: ['v*']

jobs:
  release:
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, x86_64-pc-windows-msvc, aarch64-apple-darwin]
    runs-on: ${{ matrix.target == 'x86_64-unknown-linux-gnu' && 'ubuntu-latest' || matrix.target == 'x86_64-pc-windows-msvc' && 'windows-latest' || 'macos-latest' }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --release --manifest-path apps/desktop-agent/Cargo.toml
      - uses: actions/upload-artifact@v4
        with:
          name: nyxframe-desktop-${{ matrix.target }}
          path: apps/desktop-agent/target/release/nyxframe-server*
```

### Build Scripts

**File:** `scripts/build/build-all.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "Building NyxFrame — ALL"
cargo build --workspace
(cd services/gateway && go build -o build/gateway ./cmd/server/...)
echo "Desktop agent + Gateway built."
```

**File:** `scripts/build/build-desktop.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "Building Desktop Agent"
cargo build --manifest-path apps/desktop-agent/Cargo.toml "$@"
echo "Done: apps/desktop-agent/target/debug/nyxframe-server"
```

**File:** `scripts/build/build-mobile.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "Building Android client"
cd apps/mobile-client/android
./gradlew assembleDebug "$@"
echo "Done: apps/mobile-client/android/app/build/outputs/apk/debug/"
```

**File:** `scripts/ci/lint.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "Linting Rust"
cargo clippy --workspace -- -D warnings
echo "Linting Go"
(cd services/gateway && go vet ./...)
echo "Linting Android"
(cd apps/mobile-client/android && ./gradlew lint)
echo "All lint checks passed."
```

**File:** `scripts/ci/test.sh`

```bash
#!/usr/bin/env bash
set -euo pipefail
echo "Running Rust tests"
cargo test --workspace
echo "Running Go tests"
(cd services/gateway && go test ./...)
echo "Running Android unit tests"
(cd apps/mobile-client/android && ./gradlew test)
echo "All tests passed."
```

### Versioning Strategy

| Artifact | Scheme | Example |
|---|---|---|
| Desktop agent | Semver | `v2.1.0` |
| Android app | versionCode (int) + versionName (semver) | `versionCode=4, versionName="2.2.0"` |
| Go gateway | Same as desktop | Bundled |
| Protocol | Semver (proto package) | `nyxframe.v2` |
| Releases | Git tag + GitHub Release | `v2.2.0` → release artifacts |

### .gitignore

**File:** `.gitignore`

```
# Build artifacts
apps/desktop-agent/target/
apps/desktop-agent/nyxframe-server
services/gateway/go_server
services/gateway/nyxframe-server-go
services/gateway/server
services/gateway/build/
*.apk
*.log
*.zip

# IDE
.idea/
*.iml
.gradle/
build/
local.properties

# Python
venv/
__pycache__/
*.pyc

# OS
.DS_Store
Thumbs.db

# Config overrides
*.local.json
```

### Effort: 2 engineer-days

| Task | Hours |
|---|---|
| Create `.github/workflows/ci.yml` | 4 |
| Create `.github/workflows/release.yml` | 4 |
| Create build scripts (4 files) | 4 |
| Create lint/test scripts | 2 |
| Versioning strategy doc | 1 |
| `.gitignore` update | 1 |

---

## Phase 1 — Repository Foundation

**Goal:** Create final directory structure. Move code. Change no logic.

**Duration:** 2 days  
**Owner:** Platform Engineer  
**Branch:** `refactor/phase-1`

### Commands

```bash
# ── Create top-level directory structure ──
mkdir -p apps/desktop-agent/src
mkdir -p apps/mobile-client/android
mkdir -p apps/web-dashboard          # placeholder

mkdir -p core/remote-session
mkdir -p core/devices
mkdir -p core/files
mkdir -p core/terminal
mkdir -p core/clipboard
mkdir -p core/automation
mkdir -p core/workspace
mkdir -p core/security
mkdir -p core/protocol
mkdir -p core/discovery

mkdir -p platform/linux
mkdir -p platform/windows
mkdir -p platform/macos

mkdir -p engine/remote-session
mkdir -p engine/streaming
mkdir -p engine/input
mkdir -p engine/networking
mkdir -p engine/encoding

mkdir -p packages/protocol/proto
mkdir -p packages/protocol/rust
mkdir -p packages/protocol/java
mkdir -p packages/protocol/kotlin
mkdir -p packages/protocol/python
mkdir -p packages/protocol/ts
mkdir -p packages/config

mkdir -p sdk/plugins
mkdir -p sdk/rust
mkdir -p sdk/python

mkdir -p ai/providers
mkdir -p ai/tools
mkdir -p ai/runtime

mkdir -p services/gateway/cmd/server
mkdir -p services/gateway/internal/http
mkdir -p services/gateway/internal/ws
mkdir -p services/gateway/internal/files
mkdir -p services/gateway/internal/macros
mkdir -p services/gateway/internal/webrtc
mkdir -p services/gateway/internal/ipc
mkdir -p services/gateway/proto

mkdir -p infrastructure/docker
mkdir -p infrastructure/ansible
mkdir -p infrastructure/terraform

mkdir -p scripts/build
mkdir -p scripts/deploy
mkdir -p scripts/dev
mkdir -p scripts/ci

mkdir -p test/unit
mkdir -p test/integration
mkdir -p test/e2e

mkdir -p docs/architecture
mkdir -p docs/api
mkdir -p docs/development
mkdir -p docs/specs

mkdir -p .github/workflows

# ── Move existing code ──
mv server/rust/* apps/desktop-agent/
mv server/go/* services/gateway/
mv android/* apps/mobile-client/android/

mv docs/System-Architecture.md docs/architecture/
mv docs/Video-Streaming-Engine.md docs/architecture/
mv docs/Input-Emulation.md docs/architecture/
mv docs/Macros-and-Automation.md docs/architecture/
mv docs/System-Robustness-and-Troubleshooting.md docs/development/
mv docs/Getting-Started.md docs/development/
mv docs/robust_implementation_plan.md docs/specs/
mv docs/CHANGELOG.md CHANGELOG.md
mv docs/photos docs/architecture/photos
rm -f docs/Home.md docs/ALL.md

mv build_android.sh scripts/build/build-mobile.sh
mv build_server.sh scripts/build/build-desktop.sh
mv build_and_install_android.sh scripts/deploy/install-mobile.sh
mv fix_settings.py scripts/dev/fix-settings.py
mv generate_icons.py scripts/dev/generate-icons.py
mv update_headers.py scripts/dev/update-headers.py

# ── Remove dead files ──
rm -rf node_modules
rm -f package.json package-lock.json

# ── Copy config files ──
cp server/config.json apps/desktop-agent/config.json
cp apps/mobile-client/android/config.json apps/mobile-client/android/config.json

# ── Verify moves ──
test -f apps/desktop-agent/Cargo.toml
test -f apps/desktop-agent/src/main.rs
test -f services/gateway/go.mod
test -f services/gateway/main.go
test -f apps/mobile-client/android/app/build.gradle.kts
test -f apps/mobile-client/android/app/src/main/java/com/nyxframe/app/MainActivity.kt
```

### Cargo Workspace

**File:** `Cargo.toml` (root)

```toml
[workspace]
resolver = "2"
members = [
    "apps/desktop-agent",
]

[workspace.package]
version = "2.1.0"
edition = "2021"
license = "MIT"
```

### Validation

```bash
cargo build --manifest-path apps/desktop-agent/Cargo.toml  # Rust builds
(cd services/gateway && go build ./...)                     # Go builds
(cd apps/mobile-client/android && ./gradlew assembleDebug)  # Android builds
```

### Rollback

```bash
git revert HEAD --no-edit
```

---

## Phase 2 — Go Gateway Refactor

**Goal:** Split `main.go` (1792 lines) into packages. Zero behavior change.

**Duration:** 5 days  
**Owner:** Backend Engineer  
**Branch:** `refactor/phase-2`

### Package Boundaries

```
services/gateway/
├── cmd/server/
│   └── main.go              # Entry: ~50 lines
├── internal/
│   ├── http/
│   │   ├── router.go        # Route registration
│   │   ├── stream.go        # handleStreamWebSocket
│   │   ├── commands.go      # handleCommandWebSocket
│   │   └── config.go        # handleStreamConfigAPI + ServerConfig
│   ├── files/
│   │   └── handler.go       # All /api/fs/* handlers
│   ├── macros/
│   │   └── handler.go       # /api/macros/* handlers
│   ├── ws/
│   │   ├── client.go        # SafeConn, ping handler
│   │   └── pool.go          # Client pools, ServerState
│   ├── webrtc/
│   │   └── signaling.go     # WebRTC SDP exchange
│   └── ipc/
│       ├── uds.go           # UDS connection/reconnect
│       ├── frame.go         # Frame parsing + broadcast
│       └── protocol.go      # Command/StreamConfig structs
├── go.mod
└── go.sum
```

### File Contents (abbreviated — full content in codebase)

**`services/gateway/cmd/server/main.go`:**

```go
package main

import (
    "log"
    "net/http"
    "time"
    "nyxframe-server/internal/http"
    "nyxframe-server/internal/files"
    "nyxframe-server/internal/macros"
)

var Port = "9090"

func main() {
    loadServerConfig()
    initLogFile()

    ip := getTailscaleOrLocalIP()
    log.Printf("Binding to %s\n", ip)

    go monitorAndProcessUDS()

    mux := http.NewServeMux()
    http.RegisterRoutes(mux)
    files.RegisterRoutes(mux)
    macros.RegisterRoutes(mux)

    addr := "0.0.0.0:" + Port
    freePort(Port)

    srv := &http.Server{Addr: addr, ReadTimeout: 30 * time.Second, WriteTimeout: 120 * time.Second, IdleTimeout: 60 * time.Second}
    log.Printf("Listening on %s\n", addr)
    if err := srv.ListenAndServe(); err != nil {
        log.Fatalf("Fatal: %v\n", err)
    }
}
```

**`services/gateway/internal/http/router.go`:**

```go
package http

import "net/http"

func RegisterRoutes(mux *http.ServeMux) {
    mux.HandleFunc("/ws", handleCommandWebSocket)
    mux.HandleFunc("/stream", handleStreamWebSocket)
    mux.HandleFunc("/offer", handleWebRTCOffer)
    mux.HandleFunc("/api/stream/config", handleStreamConfigAPI)
}
```

**Function mapping (all line numbers reference original `main.go`):**

| Function | Lines | Package |
|---|---|---|
| `loadServerConfig()` | 363–394 | `internal/http/config.go` |
| `initLogFile()` | 165–176 | `cmd/server/main.go` |
| `freePort()` | 449–459 | `cmd/server/main.go` |
| `getTailscaleOrLocalIP()` | 495–546 | `internal/ws/pool.go` |
| `handleStreamWebSocket` | 923–974 | `internal/http/stream.go` |
| `handleCommandWebSocket` | 1145–1165 | `internal/http/commands.go` |
| `handleStreamConfigAPI` | 462–487 | `internal/http/config.go` |
| `handleWebRTCOffer` | 1210–1298 | `internal/http/webrtc.go` |
| `handleFsListAPI` | 1398–1459 | `internal/files/handler.go` |
| `handleFsUploadAPI` | 1483–1592 | `internal/files/handler.go` |
| `handleFsDownloadAPI` | 1598–1769 | `internal/files/handler.go` |
| `handleFsMkdirAPI` | 1771–1792 | `internal/files/handler.go` |
| `handleMacrosExportAPI` | 1310–1354 | `internal/macros/handler.go` |
| `handleMacrosImportAPI` | 1356–1379 | `internal/macros/handler.go` |
| `SafeConn` | 94–123 | `internal/ws/client.go` |
| `removeWSClient()` | 125–136 | `internal/ws/client.go` |
| `ServerState` | 138–160 | `internal/ws/pool.go` |
| `monitorAndProcessUDS()` | 554–592 | `internal/ipc/uds.go` |
| `parseFrameStream()` | 712–854 | `internal/ipc/frame.go` |
| `broadcastFrame()` | 857–915 | `internal/ipc/frame.go` |
| `startPipelineBroadcaster()` | 604–619 | `internal/ipc/frame.go` |
| `startBackpressureMonitor()` | 261–295 | `internal/ipc/frame.go` |
| `Command` struct | 56–70 | `internal/ipc/protocol.go` |
| `StreamConfig` struct | 72–78 | `internal/ipc/protocol.go` |
| `handleCommand()` | 977–1129 | `internal/ws/client.go` |
| `forwardCommandToUds()` | 1168–1199 | `internal/ipc/uds.go` |
| `getI3SocketPath()` | 1132–1142 | `internal/ws/client.go` |
| `countNalTypes()` | 633–665 | `internal/ipc/frame.go` |
| `extractConfigNals()` | 667–708 | `internal/ipc/frame.go` |

### Migration Order

```
Day 1:
  mkdir -p services/gateway/internal/{http,ws,files,macros,webrtc,ipc}
  mkdir -p services/gateway/cmd/server
  Extract protocol.go (structs only)

Day 2:
  Extract ws/client.go (SafeConn, handleCommand)
  Extract ws/pool.go (ServerState, client pools, getTailscaleOrLocalIP)

Day 3:
  Extract ipc/uds.go (UDS connection, forwardCommandToUds)
  Extract ipc/frame.go (frame parsing, broadcast)

Day 4:
  Extract http/*.go (all HTTP handlers)
  Extract files/handler.go (file operations)
  Extract macros/handler.go (macro operations)
  Extract cmd/server/main.go (entry point)

Day 5:
  Update go.mod module path if needed
  Remove old main.go (rename to main.go.legacy)
  Test all endpoints
```

### Validation

```bash
cd services/gateway
go build ./cmd/server/...

# Smoke test all routes
go run ./cmd/server/... &
sleep 2
for route in /ws /stream /offer /api/stream/config /api/macros/export /api/macros/import /api/fs/list /api/fs/upload /api/fs/download /api/fs/mkdir; do
  code=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:9090$route")
  echo "$route → $code"
done
kill %1
```

### Rollback

```bash
git revert HEAD --no-edit
# OR: restore main.go.legacy → main.go, delete internal/ packages
```

---

## Phase 3 — Rust Agent Refactor

**Goal:** Split `main.rs` (907 lines) into modules. No behavior change.

**Duration:** 4 days  
**Owner:** Rust Engineer  
**Branch:** `refactor/phase-3`

### Module Structure

```
apps/desktop-agent/src/
├── main.rs              # Entry (~80 lines)
├── config.rs            # Config loading + structs (extract main.rs:271-338)
├── capture/
│   ├── mod.rs           # ScreenCapturer enum (from capture/mod.rs)
│   ├── x11.rs           # X11 capture (from capture/x11.rs)
│   └── wayland.rs       # Wayland capture (from capture/wayland.rs)
├── encoding/
│   ├── mod.rs           # NAL helpers, module exports
│   ├── yuv.rs           # FullRangeYUVBuffer (extract main.rs:23-117)
│   └── openh264.rs      # OpenH264 encoder (extract main.rs:754-834)
├── input/
│   ├── mod.rs           # Module exports
│   ├── parser.rs        # execute_command (extract main.rs:883-906)
│   └── uinput.rs        # UInputDevice (from input/uinput.rs)
├── ipc/
│   ├── mod.rs           # Module exports
│   └── uds.rs           # UDS protocol (from ipc/uds.rs)
├── runtime/
│   ├── mod.rs           # SessionManager (extract main.rs:620-680)
│   ├── pipeline.rs      # Capture→encode→send (extract main.rs:714-862)
│   └── command.rs       # Command listener (extract main.rs:648-680)
└── env_recovery.rs      # Environment detection (from env_recovery.rs)
```

### Exact Extraction Map

| Lines | Content | Target | Action |
|---|---|---|---|
| 1-5 | `pub mod` declarations | `main.rs` | Update |
| 6-18 | Imports | Split across modules | Reorganize |
| 23-117 | `FullRangeYUVBuffer` | `encoding/yuv.rs` | Extract |
| 119-207 | `broadcast_workstation_ips_to_adb()` | `runtime/mod.rs` | Extract |
| 209-269 | `spawn_tee_logging()` | `runtime/mod.rs` | Extract |
| 271-297 | Config structs | `config.rs` | Extract |
| 299-338 | `load_server_config()` | `config.rs` | Extract |
| 344-345 | `#[tokio::main]` | `main.rs` | Keep |
| 346-615 | Init code | `main.rs` | Keep (shrinks) |
| 620-680 | Connection loop | `runtime/mod.rs` | Extract |
| 682-712 | `count_nal_types()` | `encoding/mod.rs` | Extract |
| 714-862 | Capture task | `runtime/pipeline.rs` | Extract |
| 864-879 | Teardown | `runtime/mod.rs` | Extract |
| 882-906 | `execute_command()` | `input/parser.rs` | Extract |

### New main.rs (target)

```rust
pub mod config;
pub mod capture;
pub mod encoding;
pub mod input;
pub mod ipc;
pub mod runtime;
pub mod env_recovery;

use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;
use log::info;
use env_logger::Builder;

use input::uinput::UInputDevice;
use capture::ScreenCapturer;
use ipc::uds::UdsServer;
use runtime::SessionManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().unwrap();
    let config = Arc::new(config::load_server_config(exe_dir));

    // Logging setup (unchanged from original)
    Builder::new().filter(None, log::LevelFilter::Info).init();

    let env_state = env_recovery::recover_environment(None, None);
    let capturer = Arc::new(Mutex::new(ScreenCapturer::auto_select(&env_state)?));
    let device = Arc::new(Mutex::new(UInputDevice::new()?));
    let uds_server = UdsServer::new(&config.uds_socket_path)?;

    // Launch Go gateway child process
    runtime::spawn_go_gateway(&config, &exe_dir);

    let mut manager = SessionManager::new(capturer, device, uds_server, config);
    manager.run().await;

    Ok(())
}
```

### Validation

```bash
cd apps/desktop-agent && cargo build
cargo clippy -- -D warnings
cargo test

# Runtime smoke test
cargo run -- --diagnostics 2>&1 | grep -q "Backend validation"
echo $? # Expected: 0
```

### Rollback

```bash
git checkout -- src/main.rs
# Keep new module files or revert all
```

---

## Phase 4 — Android Refactor

**Goal:** Split God files into feature modules. No UI behavior change.

**Duration:** 3 weeks  
**Owner:** Android Engineer  
**Branch:** `refactor/phase-4`

### Step 1: Create Package Structure

```bash
cd apps/mobile-client/android/app/src/main/java/com/nyxframe/app

mkdir -p core/di
mkdir -p core/navigation
mkdir -p data/remote/dto
mkdir -p data/local
mkdir -p data/repository
mkdir -p domain/model
mkdir -p domain/repository
mkdir -p domain/usecase
mkdir -p presentation/theme
mkdir -p presentation/common
mkdir -p presentation/features/connect
mkdir -p presentation/features/streaming
mkdir -p presentation/features/files
mkdir -p presentation/features/settings
mkdir -p presentation/features/macros
```

### Step 2: Move Existing Files

```bash
# Move data layer files
mv data/webrtc/WebRtcManager.kt data/remote/WebRtcManager.kt
mv data/webrtc/WebSocketManager.kt data/remote/WebSocketManager.kt

# Move presentation utilities
mv data/webrtc/H264Decoder.kt presentation/features/streaming/H264Decoder.kt
mv data/webrtc/PipelineTracker.kt presentation/features/streaming/PipelineTracker.kt

# Update package declarations
sed -i 's/package com.nyxframe.app.data.webrtc/package com.nyxframe.app.data.remote/' data/remote/WebRtcManager.kt
sed -i 's/package com.nyxframe.app.data.webrtc/package com.nyxframe.app.data.remote/' data/remote/WebSocketManager.kt
sed -i 's/package com.nyxframe.app.data.webrtc/package com.nyxframe.app.presentation.features.streaming/' presentation/features/streaming/H264Decoder.kt
sed -i 's/package com.nyxframe.app.data.webrtc/package com.nyxframe.app.presentation.features.streaming/' presentation/features/streaming/PipelineTracker.kt
```

### Step 3: Extract Domain Models

**File:** `domain/model/CustomMacro.kt`

```kotlin
package com.nyxframe.app.domain.model
data class CustomMacro(val name: String, val formula: String, val delayMs: Int = 120)
```

**File:** `domain/model/Workspace.kt`

```kotlin
package com.nyxframe.app.domain.model
data class WorkspaceItem(val id: Int, val display: String, val value: String)
data class WorkspaceOptions(val defaultActiveId: Int, val swipeNavigationWrap: Boolean)
```

**File:** `domain/model/Device.kt`

```kotlin
package com.nyxframe.app.domain.model
data class DiscoveredDevice(val name: String, val ip: String, val type: String)
```

**File:** `domain/model/FileItem.kt`

```kotlin
package com.nyxframe.app.domain.model
data class FileItem(val name: String, val path: String, val isDir: Boolean, val size: Long)
```

### Step 4: Create Feature ViewModels (Wrapper Pattern)

Each new ViewModel wraps the existing `AgentViewModel` for backward compatibility:

**File:** `presentation/features/connect/ConnectViewModel.kt`

```kotlin
package com.nyxframe.app.presentation.features.connect

import androidx.lifecycle.ViewModel
import com.nyxframe.app.ui.viewmodel.AgentViewModel

class ConnectViewModel(private val agent: AgentViewModel) : ViewModel() {
    val serverHost get() = agent.serverHost
    val isConnected get() = agent.isConnected
    val isConnecting get() = agent.isConnecting
    val discoveredWorkstations get() = agent.discoveredWorkstations
    val isScanning get() = agent.isScanning
    val isFileSharingActive get() = agent.isFileSharingActive
    val currentRemotePath get() = agent.currentRemotePath

    fun connect(host: String) = agent.connectToWorkstation(host)
    fun disconnect() = agent.disconnect()
    fun scan() = agent.scanLocalSubnet()
}
```

**File:** `presentation/features/streaming/StreamViewModel.kt`

```kotlin
package com.nyxframe.app.presentation.features.streaming

import androidx.lifecycle.ViewModel
import com.nyxframe.app.ui.viewmodel.AgentViewModel

class StreamViewModel(private val agent: AgentViewModel) : ViewModel() {
    val displayBitmap get() = agent.displayBitmap
    val virtualMousePos get() = agent.virtualMousePos
    val currentMode get() = agent.currentMode
    val ctrlActive get() = agent.ctrlActive
    val altActive get() = agent.altActive
    val shiftActive get() = agent.shiftActive
    val superActive get() = agent.superActive

    fun sendKey(keyCode: Int, pressed: Boolean) = agent.sendKey(keyCode, pressed)
    fun sendMouseClick(button: Int, pressed: Boolean) = agent.sendMouseClick(button, pressed)
    fun sendMouseMove(x: Int, y: Int) = agent.sendMouseAbsolute(x, y)
    fun sendScroll(steps: Int) = agent.sendScroll(steps)
    fun requestKeyframe() = agent.requestKeyframe()
}
```

**File:** `presentation/features/files/FileBrowserViewModel.kt`

```kotlin
package com.nyxframe.app.presentation.features.files

import android.net.Uri
import androidx.lifecycle.ViewModel
import com.nyxframe.app.ui.viewmodel.AgentViewModel

class FileBrowserViewModel(private val agent: AgentViewModel) : ViewModel() {
    val remoteItems get() = agent.remoteItems
    val remoteSelectedItems get() = agent.remoteSelectedItems
    val currentRemotePath get() = agent.currentRemotePath
    val remoteParentPath get() = agent.remoteParentPath
    val isTransferringFiles get() = agent.isTransferringFiles
    val fileTransferProgress get() = agent.fileTransferProgress

    fun listDirectory(path: String?) = agent.fetchRemoteDirectory(path)
    fun upload(uris: List<Uri>, dest: String) = agent.uploadSelectedAndroidItems(uris, false, dest, {}, {})
    fun download(paths: List<String>, dest: Uri) = agent.downloadSelectedHostItems(paths, dest, {}, {})
    fun createDir(path: String) = agent.createRemoteDirectory(path)
}
```

**File:** `presentation/features/settings/SettingsViewModel.kt`

```kotlin
package com.nyxframe.app.presentation.features.settings

import androidx.lifecycle.ViewModel
import com.nyxframe.app.ui.viewmodel.AgentViewModel

class SettingsViewModel(private val agent: AgentViewModel) : ViewModel() {
    val themeBackground get() = agent.themeBackground
    val themePrimary get() = agent.themePrimary
    val themeSecondary get() = agent.themeSecondary
    val themePanel get() = agent.themePanel
    val workspacesList get() = agent.workspacesList
    val appConfig get() = agent.appConfig

    fun updateTheme(bg: androidx.compose.ui.graphics.Color, primary: androidx.compose.ui.graphics.Color) {
        agent.themeBackground = bg
        agent.themePrimary = primary
    }
}
```

**File:** `presentation/features/macros/MacrosViewModel.kt`

```kotlin
package com.nyxframe.app.presentation.features.macros

import androidx.lifecycle.ViewModel
import com.nyxframe.app.ui.viewmodel.AgentViewModel

class MacrosViewModel(private val agent: AgentViewModel) : ViewModel() {
    fun list() = agent.macrosList
    fun save(name: String, formula: String) = agent.saveMacro(name, formula)
    fun delete(name: String) = agent.deleteMacro(name)
    fun export() = agent.exportMacrosToServer()
    fun import() = agent.importMacrosFromServer()
}
```

### Step 5: Update MainActivity

```kotlin
class MainActivity : ComponentActivity() {
    private val agentViewModel: AgentViewModel by viewModels()
    // New: feature ViewModels wrapping the legacy ViewModel
    private val connectViewModel = ConnectViewModel(agentViewModel)
    private val streamViewModel = StreamViewModel(agentViewModel)
    private val fileBrowserViewModel = FileBrowserViewModel(agentViewModel)
    private val settingsViewModel = SettingsViewModel(agentViewModel)
    private val macrosViewModel = MacrosViewModel(agentViewModel)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            // Pass feature ViewModels to screens
            val navController = rememberNavController()
            NavHost(navController = navController, startDestination = "connect") {
                composable("connect") {
                    ConnectScreen(
                        viewModel = connectViewModel,
                        fileBrowserViewModel = fileBrowserViewModel,
                        onNavigateToStream = { navController.navigate("stream") },
                        onNavigateToSettings = { navController.navigate("settings") }
                    )
                }
                composable("stream") {
                    StreamScreen(
                        viewModel = streamViewModel,
                        onBack = { navController.popBackStack() }
                    )
                }
                composable("settings") {
                    SettingsScreen(
                        settingsViewModel = settingsViewModel,
                        macrosViewModel = macrosViewModel,
                        onBack = { navController.popBackStack() }
                    )
                }
            }
        }
    }
}
```

### Step 6: Screen Split Plan

| Current File | New Files | Action |
|---|---|---|
| `ConnectScreen.kt` | `ConnectScreen.kt` (connection UI only, ~200 lines) | Extract file browser |
| | `FileBrowserScreen.kt` (file dialog UI, ~300 lines) | New file |
| `StreamScreen.kt` | `StreamScreen.kt` (layout, ~200 lines) | Extract components |
| | `VideoViewport.kt` (canvas rendering) | New file |
| | `TouchInputHandler.kt` (gesture → mouse) | New file |
| | `VirtualKeyboard.kt` (keyboard composable) | New file |
| `SettingsScreen.kt` | `SettingsScreen.kt` (layout/ tabs, ~200 lines) | Extract sections |
| | `ThemeEditor.kt` (color customization) | New file |
| | `WorkspaceConfig.kt` (workspace grid) | New file |
| | `MacrosScreen.kt` (macro CRUD) | New file |

### Validation

```bash
cd apps/mobile-client/android && ./gradlew assembleDebug
# BUILD SUCCESSFUL

./gradlew lint
# No new errors

# Manual test on device:
# 1. Launch app → Connect screen renders
# 2. Connect to workstation → Stream screen shows viewport
# 3. Tap input → keyboard/mouse works
# 4. Settings → Theme + workspace config editable
# 5. Files → Browse, upload, download work
```

### Rollback

```bash
git revert HEAD --no-edit
# AgentViewModel.kt remains at original location
```

---

## Phase 5 — Protocol Platform

**Goal:** Single `.proto` definition used by Rust, Go, Android. No behavior change.

**Duration:** 1.5 weeks  
**Owner:** Platform Engineer  
**Branch:** `refactor/phase-5`

### Proto File

**File:** `packages/protocol/proto/nyxframe.proto`

```protobuf
syntax = "proto3";
package nyxframe.v2;

// ── Session Messages ──
message StreamConfig {
  bool frame_dropping = 1;
  string transport = 2;
  bool backpressure = 3;
  string codec = 4;
  uint32 target_fps = 5;
}

message SessionInfo {
  string session_id = 1;
  string device_id = 2;
  uint32 width = 3;
  uint32 height = 4;
  string state = 5;
  int64 started_at = 6;
}

// ── Input Messages ──
message InputCommand {
  string type = 1;
  uint32 key_code = 2;
  bool pressed = 3;
  int32 dx = 4;
  int32 dy = 5;
  int32 x = 6;
  int32 y = 7;
  int32 max_x = 8;
  int32 max_y = 9;
  uint32 button = 10;
  int32 steps = 11;
  string text = 12;
}

// ── Filesystem Messages ──
message FileEntry {
  string name = 1;
  string path = 2;
  bool is_dir = 3;
  int64 size = 4;
}

message FileListResponse {
  string current_path = 1;
  string parent_path = 2;
  repeated FileEntry items = 3;
}

// ── Clipboard Messages ──
message ClipboardEvent {
  string text = 1;
  string origin = 2;  // "host" | "client"
}

// ── Automation Messages ──
message MacroDef {
  string name = 1;
  string formula = 2;
  int32 delay_ms = 3;
}

message MacrosPayload {
  string json = 1;
  string toml = 2;
  string yaml = 3;
}

// ── Device Messages ──
message DiscoveredDevice {
  string name = 1;
  string ip = 2;
  string type = 3;
}

// ── Terminal Messages ──
message TerminalOpen {
  string shell = 1;
  uint32 cols = 2;
  uint32 rows = 3;
}

message TerminalData {
  string terminal_id = 1;
  bytes data = 2;
}
```

### Rust Codegen

**File:** `packages/protocol/rust/Cargo.toml`

```toml
[package]
name = "nyxframe-protocol"
version = "0.1.0"
edition = "2021"

[dependencies]
prost = "0.12"
prost-types = "0.12"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[build-dependencies]
prost-build = "0.12"
```

**File:** `packages/protocol/rust/build.rs`

```rust
fn main() {
    prost_build::compile_protos(&["../proto/nyxframe.proto"], &["../proto/"]).unwrap();
}
```

### Go Codegen

```bash
# Install protoc-gen-go
go install google.golang.org/protobuf/cmd/protoc-gen-go@latest

# Generate Go types
protoc --go_out=services/gateway/proto \
       --go_opt=paths=source_relative \
       --go_opt=Mnyxframe.proto=nyxframe/proto \
       packages/protocol/proto/nyxframe.proto
```

**File:** `services/gateway/proto/nyxframe.pb.go` (generated)

### Android Codegen

```bash
# Generate Java types
protoc --java_out=lite:apps/mobile-client/android/app/src/main/java \
       packages/protocol/proto/nyxframe.proto
```

**File:** `apps/mobile-client/android/app/src/main/java/com/nyxframe/protocol/Nyxframe.java` (generated)

Add to `build.gradle.kts`:

```kotlin
dependencies {
    implementation("com.google.protobuf:protobuf-javalite:3.25.1")
}
```

### Migration Sequence

```
Day 1-2: Create .proto file with all message types
Day 3:   Set up Rust codegen (prost-build)
Day 4:   Replace Rust hand-written structs with generated types
Day 5:   Set up Go codegen (protoc-gen-go)
Day 6:   Replace Go hand-written structs with generated types
Day 7:   Set up Android codegen (protoc-java)
Day 8:   Replace Android data classes with generated types
Day 9-10: Integration test — all platforms compile, streaming works
```

### Backward Compatibility

Add JSON serialization annotations to ensure existing field names don't change:

```rust
// Rust: serde rename attributes
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputCommand {
    #[serde(rename = "type")]
    pub type_: String,
    pub key_code: u32,
    // ...
}
```

### Validation

```bash
# Rust
cd packages/protocol/rust && cargo build
cd apps/desktop-agent && cargo build

# Go
cd services/gateway && go build ./...

# Android
cd apps/mobile-client/android && ./gradlew assembleDebug
```

### Rollback

Old structs remain in `deprecated/` directories for 2 weeks. Revert = restore old imports.

---

## Phase 6 — Discovery Layer

**Goal:** Device discovery (LAN, mDNS, Tailscale, manual).

**Duration:** 2 weeks  
**Owner:** Backend Engineer  
**Branch:** `feature/discovery`

### Architecture

```
core/discovery/
├── mod.rs
├── types.rs              # Device, Endpoint, DiscoveryMethod
├── registry.rs           # DeviceRegistry — persist + query known devices
├── providers/
│   ├── mod.rs            # DiscoveryProvider trait
│   ├── lan.rs            # LAN subnet scan
│   ├── mdns.rs           # mDNS service discovery
│   ├── tailscale.rs      # Tailscale IP detection
│   └── manual.rs         # Manual IP/hostname entry
└── health.rs             # Connection health checks
```

### Interfaces

```rust
// core/discovery/src/providers/mod.rs

#[async_trait]
pub trait DiscoveryProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn discover(&self) -> Result<Vec<DeviceInfo>, DiscoveryError>;
    fn priority(&self) -> u32;
}

// core/discovery/src/types.rs

pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub device_type: String,
    pub addresses: Vec<String>,
    pub provider: String,
    pub last_seen: SystemTime,
}

pub struct DeviceRegistry {
    known_devices: Vec<DeviceInfo>,
}
```

### Migration Order

```
Step 1: Create core/discovery module
Step 2: Implement LAN provider (ICMP ping subnet scan — currently in AgentViewModel.scanLocalSubnet)
Step 3: Implement mDNS provider (use libmdns or simple multicast DNS)
Step 4: Implement manual provider (IP/hostname input — currently in ConnectScreen)
Step 5: Implement Tailscale provider (detect Tailscale IP range — currently in Go gateway)
Step 6: Create DeviceRegistry (in-memory, persist recent devices)
Step 7: Create health checks (ping + connection quality)
Step 8: Wire into Android connect screen
```

---

## Phase 7 — Authentication & Security

**Goal:** Device pairing, session tokens, TLS, access control.

**Duration:** 3 weeks  
**Owner:** Security Engineer  
**Branch:** `feature/auth`

### Architecture

```
core/security/
├── mod.rs
├── pairing.rs            # QR code device pairing
├── tokens.rs             # JWT session tokens
├── tls.rs                # TLS configuration
├── permissions.rs        # Permission model
├── storage.rs            # Secure credential storage
└── crypto.rs             # Key generation + rotation
```

### Interfaces

```rust
// core/security/src/pairing.rs

pub struct PairingService {
    pending_pairs: HashMap<String, PairingRequest>,
}

impl PairingService {
    pub fn create_pairing_request(&self) -> PairingRequest {
        // Generate one-time code + QR data
    }
    pub async fn verify_pairing(&self, code: &str) -> Result<DeviceId, AuthError>;
}

// core/security/src/tokens.rs

pub struct TokenService {
    secret: Vec<u8>,
}

impl TokenService {
    pub fn generate_session_token(&self, device_id: &str) -> Result<String, AuthError>;
    pub fn validate_token(&self, token: &str) -> Result<SessionInfo, AuthError>;
    pub fn revoke_token(&self, token: &str) -> Result<(), AuthError>;
}
```

### Migration Order

```
Step 1: Create core/security module
Step 2: Implement TLS (wrap native TLS libraries per platform)
Step 3: Implement pairing flow (QR code on desktop, scan on mobile)
Step 4: Implement session tokens (JWT)
Step 5: Implement permission model (read-only, input, full)
Step 6: Implement secure storage (keychain on macOS, DPAPI on Windows, Secret Service on Linux)
Step 7: Wire into gateway (add auth middleware to all routes)
Step 8: Update Android to send auth tokens
```

### Security Checklist

```markdown
- [ ] `usesCleartextTraffic="true"` removed from AndroidManifest.xml
- [ ] `CheckOrigin: return true` replaced with proper origin check
- [ ] All /api/* endpoints require auth token
- [ ] WebSocket connections require token in initial message
- [ ] TLS enabled by default on all connections
- [ ] Session tokens expire after 24h
- [ ] Pairing codes expire after 5 minutes
- [ ] Credentials stored in OS keychain (not plaintext)
```

---

## Phase 8 — Platform Layer

**Goal:** Platform abstraction traits. Linux implementation. Prepare Windows/macOS stubs.

**Duration:** 3 weeks  
**Owner:** Rust Engineer  
**Branch:** `refactor/phase-8`

### Crate Structure

```
platform/
├── Cargo.toml            # Core trait definitions
├── src/
│   ├── lib.rs
│   ├── screen.rs         # ScreenProvider trait
│   ├── input.rs          # InputProvider trait
│   ├── clipboard.rs      # ClipboardProvider trait
│   ├── window.rs         # WindowProvider trait
│   ├── audio.rs          # AudioProvider trait
│   ├── notification.rs   # NotificationProvider trait
│   ├── terminal.rs       # TerminalProvider trait
│   └── filesystem.rs     # FileSystemProvider trait

platform/linux/
├── Cargo.toml            # Linux implementations
├── src/
│   ├── lib.rs
│   ├── screen.rs         # XCB MIT-SHM + Wayland GStreamer
│   ├── input.rs          # /dev/uinput
│   ├── clipboard.rs      # xclip/wl-clipboard
│   ├── window.rs         # EWMH + i3 IPC
│   └── env.rs            # DISPLAY/WAYLAND_DISPLAY detection
```

### Trait Definitions

**File:** `platform/src/screen.rs`

```rust
pub struct CaptureFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: &'static str,
}

pub trait ScreenProvider: Send {
    fn new() -> io::Result<Self> where Self: Sized;
    fn capture_frame(&mut self) -> io::Result<Option<CaptureFrame>>;
    fn resolution(&self) -> (u32, u32);
    fn backend_name(&self) -> &'static str;
}
```

**File:** `platform/src/input.rs`

```rust
pub trait InputProvider: Send {
    fn inject_key(&mut self, key_code: u16, pressed: bool) -> io::Result<()>;
    fn move_relative(&mut self, dx: i32, dy: i32) -> io::Result<()>;
    fn move_absolute(&mut self, x: i32, y: i32) -> io::Result<()>;
    fn click(&mut self, button: u16, pressed: bool) -> io::Result<()>;
    fn scroll(&mut self, steps: i32) -> io::Result<()>;
    fn type_text(&mut self, text: &str) -> io::Result<()>;
}
```

**File:** `platform/src/clipboard.rs`

```rust
pub trait ClipboardProvider: Send + Sync {
    fn get_text(&self) -> io::Result<String>;
    fn set_text(&self, text: &str) -> io::Result<()>;
    fn watch(&self) -> watch::Receiver<String>;
}
```

### Linux Implementation Mapping

| Trait | Current Code | New File |
|---|---|---|
| `ScreenProvider` | `capture/x11.rs` + `capture/wayland.rs` | `platform/linux/src/screen.rs` |
| `InputProvider` | `input/uinput.rs` | `platform/linux/src/input.rs` |
| `ClipboardProvider` | Go `main.go:1055-1077` (xclip) | `platform/linux/src/clipboard.rs` |
| `WindowProvider` | Go `main.go:999-1046` (i3-msg) | `platform/linux/src/window.rs` |

### Windows/macOS Stubs

**File:** `platform/windows/src/lib.rs`

```rust
// Platform stubs — return "not supported" errors
// Implemented in Phase 18
```

**File:** `platform/macos/src/lib.rs`

```rust
// Platform stubs — return "not supported" errors
// Implemented in Phase 19
```

### Validation

```bash
cd platform && cargo build
cd platform/linux && cargo build
cargo build --workspace
```

---

## Phase 9 — Session Management

**Goal:** Session lifecycle management. Multiple simultaneous sessions.

**Duration:** 2 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/sessions`

### Architecture

```
core/remote-session/
├── mod.rs
├── session.rs            # Session struct + lifecycle
├── manager.rs            # SessionManager (create/destroy/list)
├── events.rs             # SessionEvent types
└── config.rs             # SessionConfig
```

### Interfaces

```rust
pub struct SessionConfig {
    pub width: u32,
    pub height: u32,
    pub codec: String,
    pub target_bitrate: u32,
    pub target_fps: u32,
    pub transport: String,
}

pub struct SessionHandle {
    pub id: SessionId,
    pub state: watch::Receiver<SessionState>,
}

pub struct SessionManager {
    sessions: DashMap<SessionId, SessionHandle>,
}

impl SessionManager {
    pub async fn create(&self, config: SessionConfig) -> Result<SessionHandle, SessionError>;
    pub async fn destroy(&self, id: SessionId) -> Result<(), SessionError>;
    pub async fn list(&self) -> Result<Vec<SessionInfo>, SessionError>;
    pub fn events(&self) -> broadcast::Receiver<SessionEvent>;
}
```

### Migration

```
Step 1: Extract session lifecycle from current main.rs connection loop
Step 2: Create SessionManager struct
Step 3: Implement create/destroy/list
Step 4: Implement session events
Step 5: Wire into engine (SessionProvider)
```

---

## Phase 10 — Engine Isolation

**Goal:** Seal streaming engine behind `SessionProvider` trait.

**Duration:** 2 weeks  
**Owner:** Rust Engineer  
**Branch:** `refactor/phase-10`

### Architecture

```
engine/
├── remote-session/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs            # SessionProvider trait
│       └── default.rs        # DefaultSessionProvider (wraps existing logic)
├── streaming/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── pipeline.rs       # Capture → encode → transport
│       └── frame.rs          # Frame types
├── input/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs            # InputProvider trait
└── networking/
    ├── Cargo.toml
    └── src/
        └── lib.rs            # TransportProvider trait
```

### SessionProvider Trait

```rust
// engine/remote-session/src/lib.rs

#[async_trait]
pub trait SessionProvider: Send + Sync {
    async fn create_session(&self, config: SessionConfig) -> Result<SessionHandle, EngineError>;
    async fn destroy_session(&self, id: SessionId) -> Result<(), EngineError>;
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>, EngineError>;
    fn events(&self) -> broadcast::Receiver<SessionEvent>;
}

pub struct DefaultSessionProvider {
    config: Arc<ServerSideConfig>,
}

#[async_trait]
impl SessionProvider for DefaultSessionProvider {
    async fn create_session(&self, config: SessionConfig) -> Result<SessionHandle, EngineError> {
        // 1. Accept UDS connection
        // 2. Create transport
        // 3. Create input provider
        // 4. Spawn capture → encode → send pipeline
        // 5. Return handle
        todo!("Wrap existing pipeline from runtime/session.rs")
    }
    // ...
}
```

### Migration Strategy

**Wrap, do not rewrite.** The `DefaultSessionProvider` initially calls the exact same internal functions from `runtime/pipeline.rs`. Only the public API changes.

```
Before:
  apps/desktop-agent/src/runtime/session.rs
  → SessionManager::run() starts capture loop directly

After:
  apps/desktop-agent/src/main.rs
  → engine::DefaultSessionProvider::create_session(config)
  → engine internally calls runtime/pipeline.rs functions
```

---

## Phase 11 — Hardware Encoder Abstraction

**Goal:** Encoder selection based on platform + hardware.

**Duration:** 2 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/encoders`

### Architecture

```
engine/encoding/
├── Cargo.toml
└── src/
    ├── lib.rs              # Encoder trait, selection logic
    ├── openh264.rs         # OpenH264 (software, all platforms)
    ├── nvenc.rs            # NVENC (Linux/Windows, NVIDIA GPU)
    ├── vaapi.rs            # VAAPI (Linux, Intel/AMD GPU)
    ├── videotoolbox.rs     # VideoToolbox (macOS)
    └── quicksync.rs        # QuickSync (Windows, Intel GPU)
```

### Encoder Trait

```rust
pub trait VideoEncoder: Send {
    fn encode(&mut self, frame: &CaptureFrame) -> Result<EncodedFrame, EncodeError>;
    fn force_keyframe(&mut self);
    fn codec_name(&self) -> &str;
    fn is_hardware(&self) -> bool;
    fn reset(&mut self, width: u32, height: u32) -> Result<(), EncodeError>;
}

pub fn select_encoder(config: &EncoderConfig) -> Result<Box<dyn VideoEncoder>, EncodeError> {
    // Priority: NVENC > VAAPI > OpenH264
    #[cfg(feature = "nvenc")]
    if NvencEncoder::is_available() { return Ok(Box::new(NvencEncoder::new()?)); }
    #[cfg(feature = "vaapi")]
    if VaapiEncoder::is_available() { return Ok(Box::new(VaapiEncoder::new()?)); }
    Ok(Box::new(OpenH264Encoder::new(config)?))
}
```

### Migration

```
Step 1: Create engine/encoding crate with Encoder trait
Step 2: Wrap existing OpenH264 code in OpenH264Encoder
Step 3: Add NVENC stub (returns "unsupported" until Phase 18)
Step 4: Add VAAPI stub
Step 5: Update pipeline to use Encoder trait
Step 6: Remove direct openh264 dependency from desktop-agent (moved to engine)
```

---

## Phase 12 — File System Layer

**Goal:** File operations as a core service.

**Duration:** 1 week  
**Owner:** Backend Engineer  
**Branch:** `feature/files`

### Architecture

```
core/files/
├── mod.rs
├── service.rs            # FileService
├── provider.rs           # FileProvider trait
├── transfer.rs           # Upload/download streaming
└── search.rs             # File search
```

### Migration

```
Step 1: Create core/files module
Step 2: Create FileService wrapping Go gateway file handlers
Step 3: Add file provider interface
Step 4: Wire into Android file browser (via FileBrowserViewModel)
Step 5: Move file handlers from services/gateway to core/files (eventually)
```

---

## Phase 13 — Terminal System

**Goal:** Remote terminal access via PTY.

**Duration:** 3 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/terminal`

### Architecture

```
core/terminal/
├── Cargo.toml
├── src/
│   ├── lib.rs             # TerminalService
│   ├── session.rs         # PTY session management
│   ├── config.rs          # TerminalConfig
│   └── protocol.rs        # Terminal protocol types

platform/{linux,windows,macos}/src/terminal.rs
  # Platform-specific PTY creation
```

### Implementation

```rust
// core/terminal/src/lib.rs

pub struct TerminalService {
    sessions: DashMap<String, TerminalSession>,
}

impl TerminalService {
    pub async fn open(&self, config: TerminalConfig) -> Result<String, TerminalError>;
    pub async fn write(&self, id: &str, data: &[u8]) -> Result<(), TerminalError>;
    pub async fn resize(&self, id: &str, cols: u32, rows: u32) -> Result<(), TerminalError>;
    pub async fn close(&self, id: &str) -> Result<(), TerminalError>;
    pub fn subscribe(&self, id: &str) -> Result<watch::Receiver<Vec<u8>>, TerminalError>;
}
```

### Protocol

Add to `packages/protocol/proto/nyxframe.proto`:

```protobuf
message TerminalOpenRequest { string shell = 1; uint32 cols = 2; uint32 rows = 3; }
message TerminalData { string terminal_id = 1; bytes data = 2; }
message TerminalResize { string terminal_id = 1; uint32 cols = 2; uint32 rows = 3; }
```

### Android Integration

```kotlin
// presentation/features/terminal/TerminalScreen.kt
// Uses VT100-compatible rendering view
// Communicates via WebSocket to gateway terminal endpoint
```

---

## Phase 14 — Recording & Playback

**Goal:** Screen and input recording for macros and replay.

**Duration:** 2 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/recording`

### Architecture

```
core/recording/
├── mod.rs
├── screen.rs             # Screen recording
├── input.rs              # Input recording
├── macro.rs              # Macro recording/playback
├── storage.rs            # Recording storage
└── playback.rs           # Session playback
```

### Interfaces

```rust
pub trait Recorder: Send {
    fn start_recording(&mut self) -> Result<RecordingId, RecordError>;
    fn stop_recording(&mut self) -> Result<RecordingMetadata, RecordError>;
    fn pause(&mut self) -> Result<(), RecordError>;
    fn resume(&mut self) -> Result<(), RecordError>;
}

pub trait Playback: Send {
    fn load(&mut self, id: RecordingId) -> Result<(), PlaybackError>;
    fn play(&mut self) -> Result<(), PlaybackError>;
    fn pause(&mut self) -> Result<(), PlaybackError>;
    fn seek(&mut self, position: Duration) -> Result<(), PlaybackError>;
    fn speed(&mut self, multiplier: f32) -> Result<(), PlaybackError>;
}
```

---

## Phase 15 — Plugin Runtime

**Goal:** Plugin system foundation (empty, ready for future use).

**Duration:** 2 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/plugins`

### Architecture

```
sdk/plugins/
├── Cargo.toml
└── src/
    ├── lib.rs             # Plugin trait, Capability enum
    ├── registry.rs        # PluginRegistry
    ├── tools.rs           # ToolRegistry, Tool trait
    ├── events.rs          # EventBus
    └── manifest.rs        # PluginManifest
```

### Interfaces

```rust
pub trait Plugin: Send {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<Capability>;
    async fn init(&mut self, host: &dyn HostContext) -> Result<(), PluginError>;
    async fn handle(&mut self, req: PluginRequest) -> Result<PluginResponse, PluginError>;
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> serde_json::Value;
    async fn execute(&self, params: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

pub struct PluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
    tools: ToolRegistry,
    events: EventBus,
}
```

---

## Phase 16 — Observability

**Goal:** Metrics, tracing, structured logging.

**Duration:** 1 week  
**Owner:** Infrastructure Engineer  
**Branch:** `feature/telemetry`

### Architecture

```
core/telemetry/
├── mod.rs
├── metrics.rs            # Prometheus metrics
├── tracing.rs            # OpenTelemetry tracing
├── logging.rs            # Structured logging
├── health.rs             # Health check endpoints
└── diagnostics.rs        # Diagnostic report generation
```

### Implementation

```rust
// core/telemetry/src/metrics.rs
pub struct Metrics {
    pub frames_encoded: Counter,
    pub frames_sent: Counter,
    pub frames_dropped: Counter,
    pub active_sessions: Gauge,
    pub encode_latency: Histogram,
    pub network_latency: Histogram,
    pub errors_total: Counter,
}
```

```rust
// core/telemetry/src/health.rs
pub struct HealthService;

impl HealthService {
    pub async fn check_all(&self) -> HealthReport {
        HealthReport {
            engine: self.check_engine().await,
            network: self.check_network().await,
            platform: self.check_platform().await,
            storage: self.check_storage().await,
        }
    }
}
```

### Endpoints

```
GET /health          → {"status": "ok", "checks": {...}}
GET /metrics         → Prometheus text format
GET /diagnostics     → JSON diagnostic report
```

---

## Phase 17 — AI Runtime

**Goal:** AI agent can use system tools via LLM.

**Duration:** 3 weeks  
**Owner:** Rust Engineer  
**Branch:** `feature/ai`

### Architecture

```
ai/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── providers/
│   │   ├── mod.rs        # LlmProvider trait
│   │   ├── openai.rs     # OpenAI implementation
│   │   └── ollama.rs     # Ollama (local) implementation
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── type_text.rs
│   │   ├── click_mouse.rs
│   │   ├── read_screen.rs
│   │   ├── read_file.rs
│   │   ├── write_file.rs
│   │   ├── list_files.rs
│   │   └── run_command.rs
│   └── runtime/
│       ├── mod.rs
│       └── agent.rs      # Agent loop
```

### LlmProvider

```rust
#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn complete(&self, req: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn stream(&self, req: LlmRequest) -> Result<Stream<LlmChunk>, LlmError>;
}
```

### Agent Loop

```rust
pub async fn run_agent(
    llm: Arc<dyn LlmProvider>,
    tools: Vec<Arc<dyn Tool>>,
    task: &str,
) -> Result<String, AgentError> {
    let mut messages = vec![system_prompt(&tools), user_message(task)];
    for _ in 0..MAX_ITERATIONS {
        let response = llm.complete(LlmRequest { messages: messages.clone(), .. }).await?;
        if let Some(result) = response.final_answer() {
            return Ok(result);
        }
        for call in response.tool_calls() {
            let result = execute_tool(&tools, &call).await?;
            messages.push(tool_result_message(&call, result));
        }
    }
    Err(AgentError::MaxIterations)
}
```

---

## Phase 18 — Windows Support

**Goal:** Desktop agent ships for Windows.

**Duration:** 6-8 weeks  
**Owner:** Rust Engineer (Windows)  
**Branch:** `platform/windows`

### File-by-File Implementation List

| File | Depends On | Effort |
|---|---|---|
| `platform/windows/Cargo.toml` | None | 2h |
| `platform/windows/src/lib.rs` | None | 2h |
| `platform/windows/src/screen.rs` | `platform::ScreenProvider` | 3 weeks |
| `platform/windows/src/input.rs` | `platform::InputProvider` | 1 week |
| `platform/windows/src/clipboard.rs` | `platform::ClipboardProvider` | 2 days |
| `platform/windows/src/window.rs` | `platform::WindowProvider` | 1 week |
| `platform/windows/src/env.rs` | None | 1 day |
| `engine/networking/src/named_pipe.rs` | None | 1 week |
| CI: `.github/workflows/ci.yml` (add windows target) | None | 1 day |

**Total: ~6-8 weeks**

### Dependencies

```
ScreenProvider (trait) ←── WindowsScreen (DXGI)
InputProvider (trait)  ←── WindowsInput (SendInput)
ClipboardProvider      ←── WindowsClipboard (Win32)
WindowProvider         ←── WindowsWindow (EnumWindows)
TransportProvider      ←── NamedPipeTransport (CreateNamedPipe)
```

### Validation

```bash
# Build
cargo build --target x86_64-pc-windows-msvc

# Full integration test:
# 1. Run nyxframe-server.exe on Windows
# 2. Connect Android client from LAN
# 3. Verify: screen capture, input injection, clipboard, file browser
```

---

## Phase 19 — macOS Support

**Goal:** Desktop agent ships for macOS.

**Duration:** 8-10 weeks  
**Owner:** Rust Engineer (macOS)  
**Branch:** `platform/macos`

### File-by-File Implementation List

| File | Depends On | Effort |
|---|---|---|
| `platform/macos/Cargo.toml` | None | 2h |
| `platform/macos/src/lib.rs` | None | 2h |
| `platform/macos/src/screen.rs` | `platform::ScreenProvider` | 3 weeks |
| `platform/macos/src/input.rs` | `platform::InputProvider` | 2 weeks |
| `platform/macos/src/clipboard.rs` | `platform::ClipboardProvider` | 2 days |
| `platform/macos/src/window.rs` | `platform::WindowProvider` | 1 week |
| `platform/macos/src/env.rs` | None | 1 day |
| `platform/macos/Info.plist` | None | 1 day |
| `platform/macos/entitlements.plist` | None | 1 day |

**Total: ~8-10 weeks**

### macOS-Specific

```xml
<!-- platform/macos/entitlements.plist -->
<key>com.apple.security.device.camera</key>       <!-- Screen capture? No, this is camera -->
<key>com.apple.security.automation</key>           <!-- Input injection -->
<key>com.apple.security.device.audio-input</key>   <!-- Audio -->

<!-- Screen Recording is handled outside entitlements — user must grant in System Preferences -->
```

### Validation

```bash
# Build
cargo build --target aarch64-apple-darwin

# Full integration test:
# 1. Grant Screen Recording permission
# 2. Grant Accessibility permission
# 3. Run nyxframe-server
# 4. Connect Android client
```

---

## Phase 20 — Future Readiness

**Goal:** Structure supports iOS, web, cloud without future restructuring.

### iOS Readiness

| Need | Prepared By |
|---|---|
| Shared protocol | `packages/protocol/kotlin/` (KMP) or `packages/protocol/swift/` (future) |
| Platform traits | `platform/` already separated by OS |
| UI layer | Placeholder `apps/mobile-client/ios/` directory |
| Streaming client | `TransportProvider` trait abstracts transport |

### Web Dashboard Readiness

| Need | Prepared By |
|---|---|
| REST API | `services/gateway` HTTP handlers are RESTful |
| WebSocket stream | `/stream` and `/ws` endpoints exist |
| Shared protocol | `packages/protocol/ts/` for TypeScript |
| Auth | `core/security` provides token-based auth |

### Cloud Readiness

| Need | Prepared By |
|---|---|
| Session registry | `core/remote-session` SessionManager (scalable) |
| Device registry | `core/devices` DeviceRepository (scalable) |
| Auth | `core/security` TokenService (stateless) |
| Docker | `infrastructure/docker/` |
| Orchestration | `infrastructure/kubernetes/` |
| Infrastructure as Code | `infrastructure/terraform/` |

### Enterprise Readiness

| Need | Prepared By |
|---|---|
| SSO | `core/security` PairingService (extensible) |
| Audit | `core/telemetry` logs all actions |
| Roles | `core/security/permissions.rs` role model |
| Multi-tenant | Session isolation via `SessionManager` |

---

## Final Deliverables

### 1. Exact Repository Tree

```
NyxFrame/
├── apps/
│   ├── desktop-agent/          # Rust desktop daemon
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── config.rs
│   │       ├── capture/        # ScreenCapturer enum + backends
│   │       ├── encoding/       # YUV + encoder dispatch
│   │       ├── input/          # Input parser + dispatch
│   │       ├── ipc/            # UDS protocol
│   │       ├── runtime/        # Session lifecycle + pipeline
│   │       └── env_recovery.rs
│   ├── mobile-client/          # Android (+ iOS future)
│   │   ├── android/app/src/main/java/com/nyxframe/app/
│   │   │   ├── core/
│   │   │   ├── data/           # Data layer (remote, local, repository)
│   │   │   ├── domain/         # Domain layer (model, repository, usecase)
│   │   │   └── presentation/   # Presentation layer (theme, common, features/)
│   │   │       └── features/
│   │   │           ├── connect/
│   │   │           ├── streaming/
│   │   │           ├── files/
│   │   │           ├── settings/
│   │   │           ├── macros/
│   │   │           ├── terminal/    (future)
│   │   │           ├── ai/          (future)
│   │   │           └── devices/     (future)
│   │   ├── ios/                (placeholder)
│   │   └── config.json
│   └── web-dashboard/          (placeholder)
├── core/
│   ├── remote-session/         # Session lifecycle
│   ├── devices/                # Device registry + discovery
│   ├── files/                  # File operations
│   ├── terminal/               # PTY sessions
│   ├── clipboard/              # Clipboard sync
│   ├── automation/             # Macros + triggers
│   ├── workspace/              # Desktop management
│   ├── security/               # Auth, TLS, permissions
│   ├── discovery/              # LAN, mDNS, VPN discovery
│   ├── recording/              # Screen + input recording
│   └── telemetry/              # Metrics, tracing, logging
├── platform/
│   ├── src/                    # Shared trait definitions
│   ├── linux/                  # XCB, uinput, xclip, i3
│   ├── windows/                # DXGI, SendInput, Win32
│   └── macos/                  # CGDisplay, CGEvent, NSPasteboard
├── engine/
│   ├── remote-session/         # SessionProvider interface
│   ├── streaming/              # Capture + encode pipeline
│   ├── encoding/               # Encoder trait + codecs
│   ├── input/                  # Input injection
│   └── networking/             # UDS, named pipes, transport
├── packages/
│   ├── protocol/               # Proto definitions + generated bindings
│   │   ├── proto/nyxframe.proto
│   │   ├── rust/
│   │   ├── java/
│   │   ├── kotlin/
│   │   ├── python/
│   │   └── ts/
│   └── config/                 # Shared config schemas
├── sdk/
│   ├── plugins/                # Plugin runtime + registry
│   ├── rust/                   # Rust plugin SDK
│   └── python/                 # Python AI SDK
├── ai/
│   ├── providers/              # LLM providers (OpenAI, Ollama, Claude)
│   ├── tools/                  # Tool implementations
│   └── runtime/                # Agent loop
├── services/
│   └── gateway/                # Go network gateway
├── infrastructure/             # Docker, Ansible, Terraform
├── docs/                       # Architecture, API, development docs
├── scripts/                    # Build, deploy, dev, CI scripts
├── test/                       # Integration + E2E tests
├── Cargo.toml                  # Workspace root
├── Makefile
├── CLAUDE.md
├── CHANGELOG.md
├── LICENSE
└── README.md
```

### 2. Cargo Workspace

```toml
[workspace]
resolver = "2"
members = [
    "apps/desktop-agent",
    "engine/remote-session",
    "engine/streaming",
    "engine/input",
    "engine/networking",
    "engine/encoding",
    "platform/src",
    "platform/linux",
    "platform/windows",
    "platform/macos",
    "packages/protocol/rust",
    "packages/config",
    "sdk/plugins",
    "sdk/rust",
    "ai/providers",
    "ai/tools",
    "ai/runtime",
    "core/remote-session",
    "core/devices",
    "core/files",
    "core/terminal",
    "core/clipboard",
    "core/automation",
    "core/workspace",
    "core/security",
    "core/discovery",
    "core/recording",
    "core/telemetry",
]
```

### 3. Go Package Structure

```
services/gateway/
├── cmd/server/main.go
├── internal/
│   ├── http/     (router.go, stream.go, commands.go, config.go)
│   ├── ws/       (client.go, pool.go)
│   ├── files/    (handler.go)
│   ├── macros/   (handler.go)
│   ├── webrtc/   (signaling.go)
│   └── ipc/      (uds.go, frame.go, protocol.go)
├── proto/        (generated protobuf)
├── go.mod
└── go.sum
```

### 4. Android Module Structure

```
apps/mobile-client/android/
└── app/src/main/java/com/nyxframe/app/
    ├── NyxFrameApp.kt
    ├── MainActivity.kt
    ├── core/
    │   ├── di/
    │   └── navigation/
    ├── data/
    │   ├── remote/     (WebSocketManager, WebRtcManager, HttpClient)
    │   ├── local/      (ConfigStorage, MacrosStorage)
    │   └── repository/ (Impls)
    ├── domain/
    │   ├── model/
    │   ├── repository/ (Interfaces)
    │   └── usecase/
    └── presentation/
        ├── theme/
        ├── common/
        └── features/
            ├── connect/
            ├── streaming/
            ├── files/
            ├── settings/
            ├── macros/
            ├── terminal/  (future)
            ├── ai/        (future)
            └── devices/   (future)
```

### 5. Protocol Structure

```
packages/protocol/
├── proto/
│   └── nyxframe.proto       # Session, Input, Filesystem, Clipboard, Terminal, AI, Device
├── rust/                     # prost-generated Rust types
├── java/                     # protoc-generated Java types
├── kotlin/                   # protoc-generated Kotlin types
├── python/                   # protoc-generated Python types
└── ts/                       # protoc-gen-ts generated TypeScript types
```

### 6. Implementation Sequence

```
Phase 0:  CI/CD + Build infrastructure          Week 1-2
Phase 1:  Repository foundation                 Week 1-2     (parallel with P0)
Phase 2:  Go gateway refactor                   Week 2-3
Phase 3:  Rust agent refactor                   Week 3-4
Phase 4:  Android refactor                      Week 4-7
Phase 5:  Protocol platform                     Week 4-6     (parallel with P4)
Phase 6:  Discovery layer                       Week 6-8
Phase 7:  Authentication & security              Week 7-9
Phase 8:  Platform layer                        Week 8-11    (parallel with P6/P7)
Phase 9:  Session management                    Week 9-10
Phase 10: Engine isolation                       Week 10-12
Phase 11: Hardware encoder abstraction           Week 11-13
Phase 12: File system layer                     Week 12-13
Phase 13: Terminal system                       Week 13-16
Phase 14: Recording & playback                  Week 14-16   (parallel with P13)
Phase 15: Plugin runtime                        Week 16-18
Phase 16: Observability                         Week 16-17
Phase 17: AI runtime                            Week 18-21
Phase 18: Windows support                       Week 20-28   (starts after P8)
Phase 19: macOS support                         Week 28-36   (starts after P18)
Phase 20: Future readiness                      Ongoing
```

### 7. Dependency Graph

```
P0: CI/CD ──► all phases (parallel)
P1: Foundation ──► P2, P3, P4
P2: Go refactor ──► P5, P7
P3: Rust refactor ──► P5, P8, P9, P10
P4: Android refactor ──► P12, P13
P5: Protocol ──► P6, P7, P12, P13, P14, P15, P17
P6: Discovery ──► P7
P7: Auth ──► P15, P17
P8: Platform ──► P10, P11, P18, P19
P9: Sessions ──► P10, P15
P10: Engine isolation ──► P11, P13, P14, P15, P17
P11: Encoders ──► P18, P19
P12: Files ──► P17
P13: Terminal ──► P17
P14: Recording ──► P15
P15: Plugins ──► P17
P16: Telemetry ──► all (enables monitoring)
P17: AI ──► (leaf)
P18: Windows ──► (leaf)
P19: macOS ──► (leaf)
P20: Future ──► ongoing
```

### 8. Critical Path

```
P1 → P3 → P5 → P8 → P10 → P18 → P19    (multi-platform path)
P1 → P4 → P12 → P13                     (terminal path)
P1 → P2 → P6 → P7                       (security path)
P5 → P9 → P15 → P17                     (AI path)

Total: P1(2w) → P3(2w) → P5(2w) → P8(3w) → P10(2w) = 11 weeks to engine isolation milestone
```

### 9. Parallelizable Workstreams

| Workstream | Lead | Phases | Duration |
|---|---|---|---|
| **Infrastructure** | Infra Eng | P0, P16 | 3 weeks (parallel) |
| **Mobile** | Android Eng | P4, P12, P13 | 10 weeks |
| **Core** | Rust Eng | P3, P5, P8, P9, P10, P11 | 12 weeks |
| **Backend** | Backend Eng | P2, P6, P7 | 6 weeks |
| **AI** | AI Eng | P14, P15, P17 | 8 weeks (starts week 12) |
| **Windows** | Win Eng | P18 | 8 weeks (starts week 16) |
| **macOS** | Mac Eng | P19 | 8 weeks (starts week 24) |

### 10. Team Allocation

| Role | Phases | Allocation |
|---|---|---|
| 1× Infrastructure Engineer | P0, P16, CI/CD maintenance | 100% (weeks 1-4, then 25%) |
| 1× Android Engineer | P4, P12, P13, Android maintenance | 100% (weeks 4-16) |
| 1× Rust Engineer (Core) | P3, P5, P8, P9, P10, P11 | 100% (weeks 3-14) |
| 1× Backend Engineer (Go) | P2, P6, P7 | 100% (weeks 2-9) |
| 1× Rust Engineer (AI) | P14, P15, P17 | 100% (weeks 12-21) |
| 1× Rust Engineer (Windows) | P18 | 100% (weeks 16-24) |
| 1× Rust Engineer (macOS) | P19 | 100% (weeks 24-32) |
| **Total team** | | **5-7 engineers** |

### 11. Weekly Roadmap

```
 Week 1-2:   P0 CI/CD + P1 Foundation
 Week 3-4:   P2 Go refactor + P3 Rust refactor
 Week 5-7:   P4 Android refactor + P5 Protocol + P6 Discovery
 Week 8-10:  P7 Auth + P8 Platform layer + P9 Sessions
 Week 11-13: P10 Engine isolation + P11 Encoders + P12 Files
 Week 14-16: P13 Terminal + P14 Recording + P15 Plugins
 Week 17-18: P16 Observability + P15 continues
 Week 19-21: P17 AI runtime
 Week 22-28: P18 Windows support
 Week 29-36: P19 macOS support
 Week 37+:   P20 Future readiness, stabilization, performance
```

### 12. Risks

| Risk | Phase | Likelihood | Impact | Mitigation |
|---|---|---|---|---|
| X11 stub blocks streaming | P3/P8 | High (stub exists) | Critical | Fix X11 capture in Phase 3 immediately |
| Android ViewModel split breaks UI | P4 | Medium | High | Wrapper pattern: no behavior change until verified |
| Protocol field rename breaks clients | P5 | Medium | High | Backward-compat JSON annotations, test before merge |
| Windows DXGI capture is complex | P18 | Medium | Medium | BitBlt fallback if DXGI unavailable |
| macOS permissions UX friction | P19 | High | Medium | Clear setup wizard, docs |
| AI agent loop is unproven | P17 | Medium | Low | Start with simple tools, validate before complex |
| Team unfamiliar with codebase | All | Medium | Medium | This document as reference, pair programming early |

### 13. Rollback Strategy

| Situation | Action |
|---|---|
| Phase merge breaks build | `git revert <merge-commit>` — instant rollback |
| Android UI regression | Keep `AgentViewModel.kt` as fallback; new ViewModels wrap it |
| Protocol field mismatch | Old structs remain in `deprecated/` for 2 weeks |
| Engine change breaks streaming | `SessionProvider` wrapper delegates to old code; swap back |
| Platform trait breaks Linux build | Keep old Linux code path as default; traits opt-in |

### 14. Things That Must NEVER Be Rewritten

| Code | Reason |
|---|---|
| `capture/wayland.rs` | Working Wayland capture via GStreamer/ashpd |
| `input/uinput.rs` | Working kernel-level input injection |
| `ipc/uds.rs` | Working UDS protocol between Rust and Go |
| Go WebRTC signaling | Working WebRTC handshake (pion/webrtc) |
| Go file server handlers | Working file upload/download/listing |
| Android `H264Decoder.kt` | Working MediaCodec H.264 hardware decode |
| Android `WebSocketManager.kt` | Working WebSocket transport |
| Frame header format | Wire protocol between Rust and Go |

### 15. Things Safe to Replace

| Code | Why | When |
|---|---|---|
| `x11.rs` (stub) | Returns black frames; not functional | Immediately (Phase 3) |
| Go `main.go` | Monolithic; extract not rewrite | Phase 2 |
| `AgentViewModel.kt` | God object | Phase 4 |
| React Native assets | Unused | Phase 1 |
| `package.json` | Empty placeholder | Phase 1 |

### 16-20. Strategic Paths

```
MVP PATH (ship working product):                    Months 1-3
  Fix X11 capture → Repository foundation → Engine isolation → Protocol extraction
  → Enable WebRTC transport

PRODUCTION PATH (stable, shippable):               Months 1-6
  MVP + Go refactor + Android refactor + Auth + Discovery + Platform layer
  
MULTI-PLATFORM PATH (Windows + macOS):             Months 4-10
  Linux (months 1-3) → Windows (months 4-7) → macOS (months 8-10)

AI PATH:                                           Months 5-8
  Plugin runtime (month 5) → Tool registry → AI runtime (month 6) → Providers

12-MONTH VISION:
  Q1 (Months 1-3):   Repository reorganized, engine isolated, Android cleaned up
  Q2 (Months 4-6):   Protocol unified, auth + TLS, terminal + recording, plugins
  Q3 (Months 7-9):   AI runtime, Windows shipping, telemetry
  Q4 (Months 10-12): macOS shipping, stabilization, enterprise readiness
```
