#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_NAME="skills"
BUILD_DEB=false
BUILD_RPM=false

for arg in "$@"; do
    case "$arg" in
        --deb) BUILD_DEB=true ;;
        --rpm) BUILD_RPM=true ;;
        --all) BUILD_DEB=true; BUILD_RPM=true ;;
        --help|-h)
            echo "Usage: build.sh [--deb] [--rpm] [--all]"
            echo "  --deb   Build .deb package (requires cargo-deb)"
            echo "  --rpm   Build .rpm package (requires cargo-generate-rpm)"
            echo "  --all   Build both .deb and .rpm"
            exit 0
            ;;
        *) echo "Unknown option: $arg"; exit 1 ;;
    esac
done

echo "Building $BIN_NAME (release)..."
cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"

BINARY="$SCRIPT_DIR/target/release/$BIN_NAME"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: binary not found at $BINARY"
    exit 1
fi

SIZE=$(du -h "$BINARY" | cut -f1)
VERSION=$("$BINARY" --version 2>/dev/null || echo "unknown")

echo ""
echo "Build complete:"
echo "  Binary:  $BINARY"
echo "  Size:    $SIZE"
echo "  Version: $VERSION"

if $BUILD_DEB; then
    echo ""
    echo "Building .deb package..."
    if ! command -v cargo-deb &>/dev/null; then
        echo "Installing cargo-deb..."
        cargo install cargo-deb --locked
    fi
    DEB_PATH=$(cargo deb --no-build --no-strip --manifest-path "$SCRIPT_DIR/Cargo.toml" 2>&1 | tail -1)
    echo "  .deb: $DEB_PATH"
fi

if $BUILD_RPM; then
    echo ""
    echo "Building .rpm package..."
    if ! command -v cargo-generate-rpm &>/dev/null; then
        echo "Installing cargo-generate-rpm..."
        cargo install cargo-generate-rpm --locked
    fi
    cargo generate-rpm --manifest-path "$SCRIPT_DIR/Cargo.toml"
    RPM_PATH=$(find "$SCRIPT_DIR/target/generate-rpm" -name "*.rpm" -type f | head -1)
    echo "  .rpm: $RPM_PATH"
fi

echo ""
echo "Install with:"
echo "  cp $BINARY ~/.local/bin/"
echo "  # or"
echo "  sudo cp $BINARY /usr/local/bin/"

if $BUILD_DEB; then
    echo "  # or"
    echo "  sudo dpkg -i $DEB_PATH"
fi

if $BUILD_RPM; then
    echo "  # or"
    echo "  sudo rpm -i $RPM_PATH"
fi
