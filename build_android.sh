#!/bin/bash
set -e

# Resolve script directory to allow running from anywhere
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANDROID_DIR="$SCRIPT_DIR/android"

print_usage() {
    echo "Usage: $0 [debug | release | all]"
    echo "  debug   - Builds the Android debug APK (installs/runs out-of-the-box)"
    echo "  release - Builds the Android release APK (default)"
    echo "  all     - Builds both debug and release APKs"
}

MODE=${1:-release}

# Map common typos/variations for absolute CLI user-friendliness
if [ "$MODE" = "relese" ] || [ "$MODE" = "rel" ]; then
    MODE="release"
elif [ "$MODE" = "deb" ]; then
    MODE="debug"
fi

java_package_hint() {
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
            echo "sudo pacman -S jdk17-openjdk"
            return
            ;;
        ubuntu|debian|linuxmint|pop)
            echo "sudo apt install openjdk-17-jdk"
            return
            ;;
        fedora|rhel|centos|rocky|almalinux)
            echo "sudo dnf install java-17-openjdk-devel"
            return
            ;;
        opensuse*|sles)
            echo "sudo zypper install java-17-openjdk-devel"
            return
            ;;
    esac

    case "$os_like" in
        *debian*)
            echo "sudo apt install openjdk-17-jdk"
            return
            ;;
        *rhel*|*fedora*)
            echo "sudo dnf install java-17-openjdk-devel"
            return
            ;;
    esac

    echo "install JDK 17 and make sure java is on your PATH"
}

detect_java_home() {
    if [ -n "$JAVA_HOME" ] && [ -x "$JAVA_HOME/bin/java" ]; then
        return 0
    fi

    local candidates=(
        "$HOME/jdk17"
        "/home/$SUDO_USER/jdk17"
        "/usr/lib/jvm/java-17-openjdk"
        "/usr/lib/jvm/java-17-openjdk-amd64"
        "/usr/lib/jvm/java-17-openjdk-arm64"
        "/usr/lib/jvm/java-17"
        "/usr/lib/jvm/jdk-17"
        "/usr/lib/jvm/default-java"
    )

    for candidate in "${candidates[@]}"; do
        if [ -n "$candidate" ] && [ -x "$candidate/bin/java" ]; then
            export JAVA_HOME="$candidate"
            return 0
        fi
    done

    if command -v java >/dev/null 2>&1; then
        local java_binary
        java_binary=$(command -v java)
        export JAVA_HOME="$(cd "$(dirname "$(dirname "$(readlink -f "$java_binary")")")" && pwd)"
        return 0
    fi

    return 1
}

android_sdk_package_hint() {
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
            echo "install android-sdk-platform-tools android-sdk-cmdline-tools and set ANDROID_HOME to your SDK directory"
            return
            ;;
        ubuntu|debian|linuxmint|pop)
            echo "sudo apt install google-android-platform-34-installer google-android-build-tools-34.0.0-installer sdkmanager"
            return
            ;;
        fedora|rhel|centos|rocky|almalinux)
            echo "install Android Studio or the Android SDK command-line tools and set ANDROID_HOME to your SDK directory"
            return
            ;;
        opensuse*|sles)
            echo "install Android Studio or the Android SDK command-line tools and set ANDROID_HOME to your SDK directory"
            return
            ;;
    esac

    case "$os_like" in
        *debian*|*rhel*|*fedora*)
            echo "sudo apt install google-android-platform-34-installer google-android-build-tools-34.0.0-installer sdkmanager"
            return
            ;;
    esac

    echo "install the Android SDK platform 34 and build-tools 34 packages, then set ANDROID_HOME to the SDK root"
}

detect_android_sdk_home() {
    local candidates=(
        "$ANDROID_HOME"
        "$ANDROID_SDK_ROOT"
        "$HOME/Android/Sdk"
        "$HOME/Android/SdkRoot"
        "$HOME/Android/Sdk/android-sdk"
        "/opt/android-sdk"
        "/opt/android-sdk-linux"
        "/usr/lib/android-sdk"
        "/usr/local/android-sdk"
        "/home/$SUDO_USER/Android/Sdk"
    )

    local candidate
    for candidate in "${candidates[@]}"; do
        if [ -n "$candidate" ] && [ -d "$candidate/platforms" ] && [ -d "$candidate/platform-tools" ]; then
            export ANDROID_HOME="$candidate"
            export ANDROID_SDK_ROOT="$candidate"
            return 0
        fi
    done

    return 1
}

build_android() {
    local mode=$1
    echo "=== Building Android Companion App ($mode) ==="
    cd "$ANDROID_DIR"
    
    # Gradle Compatibility Check: Force JDK 17 if available on system to bypass Java 26 errors
    if detect_java_home; then
        echo "Using Java JDK at: $JAVA_HOME"
    else
        echo "✖ Error: Java 17 is required to build the Android app, but no valid JDK was found."
        echo "  Install a JDK 17 package and rerun the script."
        echo "  Install it with: $(java_package_hint)"
        exit 1
    fi

    # Dynamically generate local.properties to avoid hardcoded absolute home paths
    if detect_android_sdk_home; then
        echo "sdk.dir=$ANDROID_HOME" > local.properties
        echo "Using Android SDK at: $ANDROID_HOME"
    else
        echo "✖ Error: Android SDK was not found."
        echo "  Set ANDROID_HOME or ANDROID_SDK_ROOT to a valid SDK directory and rerun the script."
        echo "  Install it with: $(android_sdk_package_hint)"
        exit 1
    fi

    # Make sure gradlew is executable
    chmod +x gradlew
    
    # Extract version name from build.gradle.kts dynamically, defaulting to "2.1"
    local version_name="2.1"
    if [ -f "$ANDROID_DIR/app/build.gradle.kts" ]; then
        local extracted=$(grep "versionName =" "$ANDROID_DIR/app/build.gradle.kts" | sed -E 's/.*versionName = "([^"]*)".*/\1/')
        if [ -n "$extracted" ]; then
            version_name="$extracted"
        fi
    fi

    if [ "$mode" = "release" ]; then
        ./gradlew assembleRelease
        
        # Copy release APK to root
        local local_apk="$ANDROID_DIR/app/build/outputs/apk/release/app-release-unsigned.apk"
        if [ ! -f "$local_apk" ]; then
            # Check other possible paths
            local_apk=$(find app/build/outputs/apk/release -name "*.apk" | head -n 1)
        fi
        
        if [ -n "$local_apk" ] && [ -f "$local_apk" ]; then
            cp "$local_apk" "$SCRIPT_DIR/nyxframe-$version_name-release.apk"
            echo "✓ Success! Backup release APK saved to root as: nyxframe-$version_name-release.apk"
        else
            echo "✖ Warning: Could not find generated release APK."
        fi
    else
        ./gradlew assembleDebug
        
        # Copy debug APK to root
        local local_apk="$ANDROID_DIR/app/build/outputs/apk/debug/app-debug.apk"
        if [ ! -f "$local_apk" ]; then
            local_apk=$(find app/build/outputs/apk/debug -name "*.apk" | head -n 1)
        fi
        
        if [ -n "$local_apk" ] && [ -f "$local_apk" ]; then
            cp "$local_apk" "$SCRIPT_DIR/nyxframe-$version_name-debug.apk"
            echo "✓ Success! Backup debug APK saved to root as: nyxframe-$version_name-debug.apk"
        else
            echo "✖ Warning: Could not find generated debug APK."
        fi
    fi
}

case "$MODE" in
    debug)
        build_android "debug"
        echo "✓ Android debug build completed successfully!"
        ;;
    release)
        build_android "release"
        echo "✓ Android release build completed successfully!"
        ;;
    all)
        build_android "debug"
        build_android "release"
        echo "✓ All Android targets built successfully!"
        ;;
    *)
        print_usage
        exit 1
        ;;
esac
