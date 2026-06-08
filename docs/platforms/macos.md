# macOS Platform Support

## Status
Not yet implemented. Placeholder for future macOS support.

## Planned Capture Approach
- **Primary**: AVFoundation screen capture via `objc2` crate or `core-foundation-rs`
- **Alternative**: `CGDisplayStream` API for real-time frame streaming
- **Pipeline**: BGRA pixel data → H.264 encoding via OpenH264 (same as Linux)
- **Permission**: Screen recording requires `Screen Recording` entitlement and user approval
- **Integration**: Implement `CaptureBackend` trait in `server/rust/src/platforms/macos/capture/`

## Planned Input Approach
- **Keyboard/Mouse**: Core Graphics `CGEvent` API via `objc2` or `core-graphics` crate
- **Text entry**: `CGEventPost` with key events
- **Clipboard**: `NSPasteboard` via AppKit/Foundation bindings
- **Permission**: Input monitoring requires `Accessibility` permission
- **Integration**: Implement `InputBackend` trait in `server/rust/src/platforms/macos/input/`

## Planned Filesystem Approach
- Already platform-independent: Go file transfer code uses Go standard library
- Note: macOS file system is case-insensitive by default (unlike Linux)

## Directory Structure (future)
```
server/rust/src/platforms/macos/
├── mod.rs
├── capture/
│   ├── mod.rs
│   └── avfoundation.rs
└── input/
    ├── mod.rs
    └── cgevent.rs
```

## Key Dependencies
- `objc2` crate for Objective-C runtime bindings
- `core-foundation-rs` for Core Foundation types
- `core-graphics` for CGEvent/CGDisplayStream

## Build Requirements
- macOS SDK (xcode-select)
- Rust `aarch64-apple-darwin` or `x86_64-apple-darwin` target
- Code signing for permissions
