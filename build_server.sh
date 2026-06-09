#!/bin/bash
set -e

# Resolve script directory to allow running from anywhere
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SERVER_DIR="$SCRIPT_DIR/server"

print_usage() {
    echo "Usage: $0 [debug | release | all]"
    echo "  debug   - Builds debug targets for both Rust and Go servers"
    echo "  release - Builds optimized release targets (default)"
    echo "  all     - Builds both debug and release targets"
}

MODE=${1:-release}

xcb_package_hint() {
    local os_id=""
    local os_like=""

    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        os_id=${ID:-}
        os_like=${ID_LIKE:-}
    fi

    case "$os_id" in
        arch|manjaro|endeavouros)
            echo "sudo pacman -S libxcb"
            return
            ;;
        ubuntu|debian|linuxmint|pop)
            echo "sudo apt install libxcb1-dev libxcb-shm0-dev"
            return
            ;;
        fedora|rhel|centos|rocky|almalinux)
            echo "sudo dnf install libxcb-devel"
            return
            ;;
        opensuse*|sles)
            echo "sudo zypper install libxcb-devel"
            return
            ;;
    esac

    case "$os_like" in
        *debian*)
            echo "sudo apt install libxcb1-dev libxcb-shm0-dev"
            return
            ;;
        *rhel*|*fedora*)
            echo "sudo dnf install libxcb-devel"
            return
            ;;
    esac

    echo "install the XCB development package for your distribution"
}

jpeg_package_hint() {
    local os_id=""
    local os_like=""

    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        os_id=${ID:-}
        os_like=${ID_LIKE:-}
    fi

    case "$os_id" in
        arch|manjaro|endeavouros)
            echo "sudo pacman -S libjpeg-turbo"
            return
            ;;
        ubuntu|debian|linuxmint|pop)
            echo "sudo apt install libjpeg-dev"
            return
            ;;
        fedora|rhel|centos|rocky|almalinux)
            echo "sudo dnf install libjpeg-turbo-devel"
            return
            ;;
        opensuse*|sles)
            echo "sudo zypper install libjpeg8-devel"
            return
            ;;
    esac

    case "$os_like" in
        *debian*)
            echo "sudo apt install libjpeg-dev"
            return
            ;;
        *rhel*|*fedora*)
            echo "sudo dnf install libjpeg-turbo-devel"
            return
            ;;
    esac

    echo "install the libjpeg development package for your distribution"
}
gstreamer_package_hint() {
    local os_id=""
    local os_like=""

    if [ -r /etc/os-release ]; then
        # shellcheck disable=SC1091
        . /etc/os-release
        os_id=${ID:-}
        os_like=${ID_LIKE:-}
    fi

    case "$os_id" in
        arch|manjaro|endeavouros)
            echo "sudo pacman -S gstreamer gst-plugins-base dbus"
            return
            ;;
        ubuntu|debian|linuxmint|pop)
            echo "sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libdbus-1-dev libxcb-randr0-dev"
            return
            ;;
        fedora|rhel|centos|rocky|almalinux)
            echo "sudo dnf install gstreamer1-devel gstreamer1-plugins-base-devel dbus-devel libxcb-devel"
            return
            ;;
        opensuse*|sles)
            echo "sudo zypper install gstreamer-devel gstreamer-plugins-base-devel dbus-1-devel libxcb-devel"
            return
            ;;
    esac

    case "$os_like" in
        *debian*)
            echo "sudo apt install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libdbus-1-dev libxcb-randr0-dev"
            return
            ;;
        *rhel*|*fedora*)
            echo "sudo dnf install gstreamer1-devel gstreamer1-plugins-base-devel dbus-devel libxcb-devel"
            return
            ;;
    esac

    echo "install the GStreamer, D-Bus, and XCB-RandR development packages for your distribution"
}

build_rust() {
    local mode=$1
    echo "=== Building Rust Engine ($mode) ==="
    cd "$SERVER_DIR/rust"

    if ! command -v pkg-config >/dev/null 2>&1; then
        echo "ERROR: pkg-config is required to verify native Rust dependencies."
        echo "Install your distro's pkg-config package and libxcb development package, then retry."
        return 1
    fi

    if ! pkg-config --exists xcb >/dev/null 2>&1; then
        echo "ERROR: Missing native XCB development files required by the Rust capture engine."
        echo "The linker error 'unable to find library -lxcb' means the unversioned libxcb.so is not installed."
        echo "Install it with: $(xcb_package_hint)"
        return 1
    fi

    if ! pkg-config --exists gstreamer-1.0 gstreamer-app-1.0 gstreamer-video-1.0 dbus-1 >/dev/null 2>&1; then
        echo "ERROR: Missing native GStreamer or D-Bus development files required by the Wayland capture engine."
        echo "Install it with: $(gstreamer_package_hint)"
        return 1
    fi

    if [ "$mode" = "release" ]; then
        cargo build --release
        
        echo "✓ Server binary built at: ./target/release/nyxframe-server"
        rm -f ../../nyxframe-server
        cp target/release/nyxframe-server ../../nyxframe-server
        echo "✓ Server binary copied to root as: nyxframe-server"
        echo "Run with: sudo ./nyxframe-server -m x11"
    else
        cargo build
    fi
}

build_go() {
    local mode=$1
    echo "=== Building Go Network Gateway ($mode) ==="
    cd "$SERVER_DIR/go"

    if ! command -v go >/dev/null 2>&1; then
        echo "ERROR: Go is required to build the network gateway, but 'go' is not installed or not on PATH."
        echo "Install it with your distro package manager and rerun the build."
        echo "  Arch:      sudo pacman -S go"
        echo "  Debian/Ubuntu: sudo apt install golang-go"
        echo "  Fedora:    sudo dnf install golang"
        return 1
    fi

    if ! command -v pkg-config >/dev/null 2>&1; then
        echo "ERROR: pkg-config is required to verify native Go image dependencies."
        echo "Install pkg-config and the libjpeg development package, then retry."
        return 1
    fi

    if ! pkg-config --exists libjpeg >/dev/null 2>&1; then
        echo "ERROR: Missing native libjpeg headers required by github.com/pixiv/go-libjpeg."
        echo "The compiler error 'fatal error: jpeglib.h: No such file or directory' means the libjpeg development package is not installed."
        echo "Install it with: $(jpeg_package_hint)"
        return 1
    fi

    if [ "$mode" = "release" ]; then
        go build -o server main.go
    else
        go build -gcflags="all=-N -l" -o server main.go
    fi
}

case "$MODE" in
    debug)
        build_rust "debug"
        build_go "debug"
        echo "✓ Server debug build completed successfully!"
        ;;
    release)
        build_rust "release"
        build_go "release"
        echo "✓ Server release build completed successfully!"
        ;;
    all)
        build_rust "debug"
        build_go "debug"
        build_rust "release"
        build_go "release"
        echo "✓ All server targets compiled successfully!"
        ;;
    *)
        print_usage
        exit 1
        ;;
esac
