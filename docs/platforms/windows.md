# Windows Platform Support

## Status
Not yet implemented. Placeholder for future Windows support.

## Planned Capture Approach
- **Primary**: DXGI Desktop Duplication API via Rust's `windows-rs` crate
- **Alternative**: DirectX 11 back-buffer capture (lower overhead, single-display)
- **Pipeline**: BGRA pixel data → H.264 encoding via OpenH264 (same as Linux)
- **Integration**: Implement `CaptureBackend` trait in `server/rust/src/platforms/windows/capture/`

## Planned Input Approach
- **Keyboard/Mouse**: `SendInput` API via Rust's `windows-rs`
- **Text entry**: `SendInput` with `KEYBDINPUT` Unicode packets
- **Clipboard**: Win32 `GetClipboardData`/`SetClipboardData` APIs
- **Integration**: Implement `InputBackend` trait in `server/rust/src/platforms/windows/input/`

## Planned Filesystem Approach
- Already platform-independent: Go file transfer code uses Go standard library which abstracts platform differences
- No changes needed for file transfer

## Directory Structure (future)
```
server/rust/src/platforms/windows/
├── mod.rs
├── capture/
│   ├── mod.rs
│   └── dxgi.rs
└── input/
    ├── mod.rs
    └── sendinput.rs
```

## Key Dependencies
- `windows-rs` (Microsoft official Rust bindings)
- DXGI (DirectX Graphics Infrastructure)
- Win32 API for input

## Build Requirements
- Windows SDK
- Rust `x86_64-pc-windows-msvc` target
- Cross-compilation from Linux possible with `x86_64-pc-windows-gnu` target
