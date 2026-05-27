#!/usr/bin/env bash
# Single source of truth = Cargo.toml `version = "X.Y.Z"`.
# Pushes that version into every place the rest of the project references it:
#
#   • latest-version        (consumed by the in-app auto-update check)
#   • index.html            (JSON-LD softwareVersion + GitHub-release URLs)
#
# Idempotent — run as often as you like. Exits non-zero if anything fails so
# the calling build script aborts before producing an out-of-sync release.

set -euo pipefail
cd "$(dirname "$0")/.."

# ── Read the canonical version ──────────────────────────────────────────────
VERSION=$(grep -E '^version[[:space:]]*=' Cargo.toml \
    | head -1 \
    | sed -E 's/.*"([^"]+)".*/\1/')

if [[ -z "$VERSION" ]]; then
    echo "ERROR: could not read 'version' from Cargo.toml" >&2
    exit 1
fi
echo "Sync to version: $VERSION"

# ── latest-version (auto-update check endpoint payload) ────────────────────
echo -n "$VERSION" > latest-version

# ── index.html (download URLs + JSON-LD softwareVersion) ───────────────────
# Use perl for portable in-place edit + ERE without macOS-vs-GNU sed quirks.
perl -i -pe "s|releases/download/v[0-9]+\.[0-9]+\.[0-9]+/|releases/download/v$VERSION/|g" \
    web/index.html
perl -i -pe "s|\"softwareVersion\": \"[^\"]*\"|\"softwareVersion\": \"$VERSION\"|g" \
    web/index.html

echo "✓ Synced: latest-version + index.html"
