#!/usr/bin/env bash
set -euo pipefail
root=$(cd "$(dirname "$0")/../../.." && pwd)
cd "$root"
cargo build --release --locked --bin openplayer-desktop
metadata=$(cargo metadata --no-deps --format-version=1)
target=$(jq -r '.target_directory' <<< "$metadata")
version=$(jq -r '.packages[] | select(.name=="openplayer-desktop") | .version' <<< "$metadata")
stem="OpenPlayer_$version"
bundle="$target/release/bundle"
mkdir -p "$bundle/deb" "$bundle/appimage"
work=$(mktemp -d "$bundle/linux-XXXXXXXX")
trap 'rm -rf -- "$work"' EXIT
tree="$work/deb"
desktop=dev.openplayer.app.desktop
icon=apps/desktop/native/icons/128x128.png
install -Dm755 "$target/release/openplayer-desktop" "$tree/usr/bin/openplayer-desktop"
install -Dm644 "$icon" "$tree/usr/share/icons/hicolor/128x128/apps/openplayer.png"
install -Dm644 LICENSE "$tree/usr/share/doc/open-player/copyright"
install -Dm644 "apps/desktop/native/$desktop" "$tree/usr/share/applications/$desktop"
desktop-file-validate "$tree/usr/share/applications/$desktop"
mkdir "$tree/DEBIAN"
cat > "$tree/DEBIAN/control" <<CONTROL
Package: open-player
Version: $version
Architecture: amd64
Maintainer: AreChen <AreChen@users.noreply.github.com>
Section: video
Priority: optional
Depends: libmpv2, libxkbcommon-x11-0, libxkbcommon0, libwayland-client0, libxcb1, libvulkan1, libfontconfig1, libssl3t64 | libssl3, pkexec
Description: Native desktop media player built with Rust, GPUI Fast and libmpv
CONTROL
dpkg-deb --root-owner-group --build "$tree" "$bundle/deb/$stem"_amd64.deb
tool="$work/linuxdeploy"
curl --fail --location --retry 3 -o "$tool" \
  https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage
echo "c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d  $tool" | sha256sum --check --status
chmod +x "$tool"
export APPIMAGE_EXTRACT_AND_RUN=1
export OUTPUT="$bundle/appimage/$stem"_amd64.AppImage
appdir="$work/AppDir"
install -Dm644 "$tree/usr/share/doc/open-player/copyright" "$appdir/usr/share/doc/open-player/copyright"
cp -R /usr/share/doc/libmpv2 "$appdir/usr/share/doc/"
"$tool" --appdir "$appdir" --executable "$target/release/openplayer-desktop" \
  --desktop-file "$tree/usr/share/applications/$desktop" --icon-file "$tree/usr/share/icons/hicolor/128x128/apps/openplayer.png" --output appimage
test -s "$OUTPUT"
