# Releasing AI FIX Parser for macOS

## Prerequisites (Apple Developer Account)

1. **Team ID** — [developer.apple.com/account](https://developer.apple.com/account) → Membership
2. **Developer ID Application** certificate — Certificates → + → Developer ID Application
3. **App-specific password** — [appleid.apple.com](https://appleid.apple.com) → Sign-In and Security → App-Specific Passwords

## One-time: Store notarization credentials

```bash
xcrun notarytool store-credentials "AC_PASSWORD" \
  --apple-id "your@email.com" \
  --team-id "YOUR_TEAM_ID" \
  --password "xxxx-xxxx-xxxx-xxxx"
```

Use your app-specific password for `--password`.

## Build signed app + DMG

```bash
export APPLE_TEAM_ID="YOUR_TEAM_ID"
./scripts/release-macos.sh
```

Or run `dx bundle` directly:

```bash
dx bundle --macos \
  --package-types macos \
  --package-types dmg \
  --release \
  --codesign \
  --apple-team-id "YOUR_TEAM_ID" \
  --apple-entitlements "entitlements.plist"
```

## Output

- **App:** `target/dx/AIFixParser/bundle/macos/bundle/macos/AI FIX Parser.app`
- **DMG:** `target/dx/AIFixParser/bundle/macos/bundle/dmg/AI FIX Parser_*.dmg`

## If notarization is needed manually

If `dx bundle` does not notarize automatically:

```bash
DMG="target/dx/AIFixParser/bundle/macos/bundle/dmg/AI FIX Parser_0.1.0_aarch64.dmg"

# Submit for notarization
xcrun notarytool submit "$DMG" --keychain-profile "AC_PASSWORD" --wait

# Staple ticket to DMG
xcrun stapler staple "$DMG"
```

Then distribute the stapled DMG — users can install without the "damaged" error.


"A developer debugging tool that parses and inspects FIX protocol log files — similar to Wireshark for network packets. It is not a trading platform, does not execute trades, and does not connect to any exchange."



Todo: 
- Rewrite the blog
- Re-test it all
- Break parser and benchmark into aifixparser repo
- Share blog and aifixparser 
- Make the parser as a lib






