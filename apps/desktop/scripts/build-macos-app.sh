#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
cargo build --release --locked --bin openplayer-desktop
metadata=$(cargo metadata --no-deps --format-version=1)
target=$(jq -r '.target_directory' <<< "$metadata")
version=$(jq -r '.packages[] | select(.name=="openplayer-desktop") | .version' <<< "$metadata")
app="${1:-$target/release/bundle/macos/OpenPlayer.app}"
contents="$app/Contents"
mkdir -p "$contents/MacOS" "$contents/Resources"
install -m755 "$target/release/openplayer-desktop" "$contents/MacOS/openplayer-desktop"
install -m644 apps/desktop/native/icons/icon.icns "$contents/Resources/OpenPlayer.icns"
cat > "$contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.openplayer.desktop</string>
<key>CFBundleExecutable</key><string>openplayer-desktop</string>
<key>CFBundleName</key><string>OpenPlayer</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleVersion</key><string>$version</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleIconFile</key><string>OpenPlayer.icns</string>
<key>LSApplicationCategoryType</key><string>public.app-category.video</string>
<key>NSHighResolutionCapable</key><true/>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Media</string>
<key>CFBundleTypeRole</key><string>Viewer</string>
<key>LSHandlerRank</key><string>Alternate</string>
<key>LSItemContentTypes</key><array><string>public.movie</string><string>public.audio</string><string>public.audiovisual-content</string><string>org.matroska.mkv</string></array>
</dict></array>
</dict></plist>
PLIST
plutil -lint "$contents/Info.plist"
