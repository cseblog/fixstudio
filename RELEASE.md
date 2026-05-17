run `dx bundle` directly:

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






