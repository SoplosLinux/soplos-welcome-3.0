#!/usr/bin/env bash
# =============================================================================
# package.sh
# Builds Soplos Welcome's .deb and moves it straight to the shared Descargas
# folder, then removes the build's bundle directory so old .debs (one new
# filename per version/revision bump, never cleaned up on its own) don't
# keep piling up in target/release/bundle.
#
# Usage: ./package.sh
# =============================================================================

set -e

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEST_DIR="$HOME/Descargas/SoplosApps"
BUNDLE_DIR="$PROJECT_DIR/src-tauri/target/release/bundle"

cd "$PROJECT_DIR"
npm run tauri build

mkdir -p "$DEST_DIR"
DEB="$(find "$BUNDLE_DIR/deb" -maxdepth 1 -name '*.deb' -print -quit)"
[[ -n "$DEB" ]] || { echo "ERROR: no .deb found in $BUNDLE_DIR/deb" >&2; exit 1; }

cp "$DEB" "$DEST_DIR/"
echo "Copied: $DEST_DIR/$(basename "$DEB")"

rm -rf "$BUNDLE_DIR"
echo "Cleaned up: $BUNDLE_DIR"
