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

# Team ID (override with APPLE_TEAM_ID env var if needed)
TEAM_ID="${APPLE_TEAM_ID:-438CH4PXMR}"
BUNDLE_DIR="target/dx/AIFixParser/bundle/macos/bundle"
APP_PATH="$BUNDLE_DIR/macos/AiFixParser.app"

echo "Building signed macOS bundle (Team ID: $TEAM_ID)..."
dx bundle --macos \
    --package-types macos \
    --package-types dmg \
    --release \
    --codesign \
    --apple-team-id "$TEAM_ID" \
    --apple-entitlements "entitlements.plist"

# Re-sign with hardened runtime (required for notarization)
# dx bundle may not enable it; Apple rejects without it
echo ""
echo "Re-signing with hardened runtime for notarization..."
SIGN_ID=$(security find-identity -v -p codesigning | grep "Developer ID Application" | head -1 | sed -E 's/^[^"]*"([^"]+)".*/\1/')
if [[ -z "$SIGN_ID" ]]; then
    echo "ERROR: No 'Developer ID Application' certificate found in Keychain."
    exit 1
fi

codesign --force --deep --timestamp --options runtime \
    --entitlements "entitlements.plist" \
    --sign "$SIGN_ID" \
    "$APP_PATH"

# Rebuild DMG with the properly signed app (old DMG has invalid signature)
echo "Creating DMG..."
DMG_NAME="AiFixParser_1.0.0_aarch64.dmg"
DMG_PATH="$BUNDLE_DIR/dmg/$DMG_NAME"
rm -f "$DMG_PATH"
hdiutil create -volname "AiFixParser" -srcfolder "$APP_PATH" \
    -ov -format UDZO "$DMG_PATH"

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

echo ""
echo "✓ Release complete!"
echo "  DMG: $DMG_PATH"
echo "  Ready to distribute."
