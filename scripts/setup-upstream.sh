#!/bin/bash
# setup-upstream.sh - Transform upstream latte into latte-core library
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHARYBDIS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
UPSTREAM_DIR="$CHARYBDIS_DIR/upstream"
BUILD_DIR="$CHARYBDIS_DIR/build"
TEMPLATES_DIR="$CHARYBDIS_DIR/templates"

log() { echo "[charybdis] $1"; }

# Check upstream
if [ ! -d "$UPSTREAM_DIR/src" ]; then
    echo "Error: upstream submodule not found. Run: git submodule update --init"
    exit 1
fi

# Get version and substitute in template
UPSTREAM_VERSION=$(grep '^version = ' "$UPSTREAM_DIR/Cargo.toml" | head -1 | sed 's/version = "\(.*\)"/\1/')
log "upstream latte version: $UPSTREAM_VERSION"

# Prepare build/latte-core
log "preparing build/latte-core..."
rm -rf "$BUILD_DIR/latte-core"
mkdir -p "$BUILD_DIR/latte-core"

# Copy upstream source
cp -r "$UPSTREAM_DIR/src" "$BUILD_DIR/latte-core/"
cp -r "$UPSTREAM_DIR/resources" "$BUILD_DIR/latte-core/" 2>/dev/null || true
cp "$UPSTREAM_DIR/rust-toolchain.toml" "$BUILD_DIR/" 2>/dev/null || true

# Apply templates
log "applying templates..."
sed "s/\${UPSTREAM_VERSION}/$UPSTREAM_VERSION/g" "$TEMPLATES_DIR/latte-core.Cargo.toml" > "$BUILD_DIR/latte-core/Cargo.toml"
cp "$TEMPLATES_DIR/latte-core.lib.rs" "$BUILD_DIR/latte-core/src/lib.rs"
cp "$TEMPLATES_DIR/latte-core.build.rs" "$BUILD_DIR/latte-core/build.rs"

# Append callback variant to exec/mod.rs
log "adding par_execute_with_callback..."
cat "$TEMPLATES_DIR/exec_callback.rs" >> "$BUILD_DIR/latte-core/src/exec/mod.rs"

log "done: $BUILD_DIR/latte-core"
