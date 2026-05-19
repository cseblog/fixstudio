#!/usr/bin/env bash
# Build signed and notarized macOS app + DMG for AI FIX Parser.
# Requires: Apple Developer account, Developer ID cert, app-specific password.
#
# Setup (one-time):
#   1. Get Team ID from developer.apple.com/account
#   2. Create Developer ID Application certificate in Certificates
#   3. Create app-specific password at appleid.apple.com
#   4. Run: xcrun notarytool store-credentials "AC_PASSWORD" \
#        --apple-id "YOUR_EMAIL" --team-id "YOUR_TEAM_ID" \
#        --password "YOUR_APP_SPECIFIC_PASSWORD"

set -e
cd "$(dirname "$0")/.."

# Sync version refs (latest-version, index.html JSON-LD + download URLs) from
# the canonical Cargo.toml version BEFORE building, so the shipped DMG, the
# landing page download links, and the auto-update payload all agree.
./scripts/sync-version.sh

# Team ID (override with APPLE_TEAM_ID env var if needed)
TEAM_ID="${APPLE_TEAM_ID:-438CH4PXMR}"
BUNDLE_DIR="target/dx/AIFixParser/bundle/macos/bundle"
APP_PATH="$BUNDLE_DIR/macos/AiFixParser.app"

echo "Building unsigned macOS bundle..."
# Build only — skip `dx`'s built-in --codesign because it picks the cert by
# Team ID alone, which is ambiguous when both
#   "3rd Party Mac Developer Application: … (438CH4PXMR)"
# and
#   "Developer ID Application: … (438CH4PXMR)"
# exist in the keychain. We sign with the full identity name below instead.
dx bundle --macos \
    --package-types macos \
    --package-types dmg \
    --release

# Resolve the Developer ID identity by name (not by team) so we never pick
# the 3rd-Party / Mac App Store cert by accident.
echo ""
echo "Resolving Developer ID signing identity..."
SIGN_ID=$(security find-identity -v -p codesigning \
    | grep "Developer ID Application" \
    | head -1 \
    | sed -E 's/^[^"]*"([^"]+)".*/\1/')
if [[ -z "$SIGN_ID" ]]; then
    echo "ERROR: No 'Developer ID Application' certificate found in Keychain."
    echo "       Install one from https://developer.apple.com/account → Certificates."
    exit 1
fi
echo "  → $SIGN_ID"

echo ""
echo "Signing app with hardened runtime (required for notarization)..."

codesign --force --deep --timestamp --options runtime \
    --entitlements "entitlements.plist" \
    --sign "$SIGN_ID" \
    "$APP_PATH"

# Rebuild DMG with app + Applications folder (drag-to-install layout)
echo "Creating DMG..."
DMG_NAME="AiFixParser_aarch64.dmg"
DMG_PATH="$BUNDLE_DIR/dmg/$DMG_NAME"
rm -f "$DMG_PATH"

DMG_LAYOUT=$(mktemp -d)
cp -R "$APP_PATH" "$DMG_LAYOUT/"
ln -s /Applications "$DMG_LAYOUT/Applications"

hdiutil create -volname "AiFixParser" -srcfolder "$DMG_LAYOUT" \
    -ov -format UDZO "$DMG_PATH"

rm -rf "$DMG_LAYOUT"

# Sign the DMG
echo "Signing DMG..."
codesign --force --timestamp --sign "$SIGN_ID" "$DMG_PATH"

# Notarize
echo "Submitting for notarization..."
xcrun notarytool submit "$DMG_PATH" \
    --keychain-profile "AC_PASSWORD" \
    --wait

# Staple
echo "Stapling notarization ticket..."
xcrun stapler staple "$DMG_PATH"

# Copy the final, signed + notarized + stapled DMG into release/ at the
# repo root so the upload step has a stable, easy-to-find path. We do this
# AFTER stapling — copying earlier would lose the embedded ticket if the
# DMG is rewritten by a later step.
RELEASE_DIR="release"
mkdir -p "$RELEASE_DIR"
RELEASE_PATH="$RELEASE_DIR/$DMG_NAME"
cp -f "$DMG_PATH" "$RELEASE_PATH"

# Quick verification — staple must validate against the copy too.
if ! xcrun stapler validate "$RELEASE_PATH" >/dev/null 2>&1; then
    echo "ERROR: stapler validation failed on copied DMG ($RELEASE_PATH)."
    exit 1
fi

echo ""
echo "✓ Release complete!"
echo "  Build DMG: $DMG_PATH"
echo "  Release  : $RELEASE_PATH"
echo "  Ready to distribute."
