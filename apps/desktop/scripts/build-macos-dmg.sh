#!/usr/bin/env bash
set -euo pipefail

version="${1:?Expected a version}"
arch="${2:?Expected a package architecture}"
distribution="${3:-false}"
machine_arch="$(uname -m)"
case "$machine_arch:$arch" in
  arm64:arm64|x86_64:x64) ;;
  *) echo "Runner architecture does not match $arch" >&2; exit 1 ;;
esac

bundle="${CARGO_TARGET_DIR:-target}/release/bundle"
dmg="$bundle/dmg/OpenPlayer_${version}_${arch}.dmg"
temporary="$(mktemp -d)"
dmg_root="$temporary/dmg"
app="$dmg_root/OpenPlayer.app"
keychain=""
probe=""
mountpoint="$temporary/mounted"

cleanup() {
  hdiutil detach "$mountpoint" -quiet 2>/dev/null || true
  if [[ -n "$probe" ]]; then
    rm -f "$probe"
  fi
  if [[ -n "$keychain" ]]; then
    security delete-keychain "$keychain" || true
  fi
  rm -rf "$temporary"
}
trap cleanup EXIT

# Build straight into a fresh volume tree so earlier packages cannot contribute
# dylibs, their rewritten install names, or signatures to the next DMG.
export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-headerpad_max_install_names"
bash apps/desktop/scripts/build-macos-app.sh "$app"
node apps/desktop/scripts/bundle-macos-libmpv.mjs "$app"

exe_name="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app/Contents/Info.plist")"
exe="$app/Contents/MacOS/$exe_name"
lipo "$exe" -verify_arch "$machine_arch"

identity="-"
sign_options=(--timestamp=none)
mode=ad-hoc
if [[ "$distribution" == true ]]; then
  if [[ -z "${OPENPLAYER_APPLE_CERTIFICATE:-}" || -z "${OPENPLAYER_APPLE_CERTIFICATE_PASSWORD:-}" ||
        -z "${OPENPLAYER_APPLE_SIGNING_IDENTITY:-}" || -z "${OPENPLAYER_APPLE_ID:-}" ||
        -z "${OPENPLAYER_APPLE_PASSWORD:-}" || -z "${OPENPLAYER_APPLE_TEAM_ID:-}" ]]; then
    echo '::warning::Apple signing or notarization secrets are missing. This DMG is for draft validation only.'
  else
    identity="$OPENPLAYER_APPLE_SIGNING_IDENTITY"
    if [[ "$identity" != 'Developer ID Application: '* ]]; then
      echo 'APPLE_SIGNING_IDENTITY must name a Developer ID Application certificate' >&2
      exit 1
    fi
    keychain="$temporary/openplayer-signing.keychain-db"
    keychain_password="$(openssl rand -hex 24)"
    printf '%s' "$OPENPLAYER_APPLE_CERTIFICATE" | base64 -D > "$temporary/certificate.p12"
    security create-keychain -p "$keychain_password" "$keychain"
    security set-keychain-settings -lut 21600 "$keychain"
    security unlock-keychain -p "$keychain_password" "$keychain"
    security import "$temporary/certificate.p12" -P "$OPENPLAYER_APPLE_CERTIFICATE_PASSWORD" \
      -k "$keychain" -T /usr/bin/codesign -T /usr/bin/security
    security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$keychain_password" "$keychain"
    rm -f "$temporary/certificate.p12"
    sign_options=(--keychain "$keychain" --timestamp)
    mode=notarized
  fi
fi

sign() {
  codesign --force --sign "$identity" "${sign_options[@]}" "$@"
}

sign_code() {
  if [[ "$mode" == notarized ]]; then
    sign --options runtime "$1"
  else
    sign "$1"
  fi
}

notarize_and_staple() {
  local upload="$1"
  local staple="$2"
  xcrun notarytool submit "$upload" --wait \
    --apple-id "$OPENPLAYER_APPLE_ID" --password "$OPENPLAYER_APPLE_PASSWORD" \
    --team-id "$OPENPLAYER_APPLE_TEAM_ID"
  xcrun stapler staple "$staple"
  xcrun stapler validate "$staple"
}

while IFS= read -r -d '' binary; do
  if file -b "$binary" | grep -q 'Mach-O'; then
    lipo "$binary" -verify_arch "$machine_arch"
    sign_code "$binary"
  fi
done < <(find "$app/Contents/Frameworks" -type f \( -name '*.dylib' -o -name '*.so' -o -perm +111 \) -print0)

while IFS= read -r -d '' framework; do
  sign_code "$framework"
done < <(find "$app/Contents/Frameworks" -depth -type d -name '*.framework' -print0)

probe="$(mktemp "$app/Contents/MacOS/.mpv-load.XXXXXX")"
xcrun clang -x c -o "$probe" - <<'C'
#include <dlfcn.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 2) return 1;

    void *mpv = dlopen(argv[1], RTLD_NOW | RTLD_LOCAL);
    if (!mpv || !dlsym(mpv, "mpv_client_api_version")) {
        fprintf(stderr, "Could not load bundled libmpv: %s\n", dlerror());
        return 1;
    }

    dlclose(mpv);
    return 0;
}
C
chmod 700 "$probe"
frameworks="$(cd "$app/Contents/Frameworks" && pwd -P)"
for libmpv in "$frameworks"/libmpv*.dylib; do
  test -f "$libmpv"
  env -i PATH=/usr/bin:/bin "$probe" "$libmpv"
  echo "Loaded bundled libmpv at $libmpv"
done
rm -f "$probe"
probe=""

sign_code "$app"
codesign --verify --deep --strict --verbose=2 "$app"

if [[ "$mode" == notarized ]]; then
  ditto -c -k --keepParent "$app" "$temporary/OpenPlayer.zip"
  notarize_and_staple "$temporary/OpenPlayer.zip" "$app"
fi

mkdir -p "$(dirname "$dmg")" "$mountpoint"
ln -s /Applications "$dmg_root/Applications"
for attempt in 1 2 3; do
  if hdiutil create -volname OpenPlayer -srcfolder "$dmg_root" -ov -format UDZO "$dmg"; then
    break
  else
    status=$?
    if [[ "$attempt" == 3 ]]; then
      exit "$status"
    fi
  fi
  rm -f "$dmg"
  sleep "$((attempt * 5))"
done

if [[ "$mode" == notarized ]]; then
  sign "$dmg"
  notarize_and_staple "$dmg" "$dmg"
  spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
fi

hdiutil verify "$dmg"
hdiutil attach "$dmg" -readonly -nobrowse -mountpoint "$mountpoint"
test "$(readlink "$mountpoint/Applications")" == /Applications
codesign --verify --deep --strict --verbose=2 "$mountpoint/OpenPlayer.app"
node apps/desktop/scripts/bundle-macos-libmpv.mjs --verify "$mountpoint/OpenPlayer.app"
if [[ "$mode" == notarized ]]; then
  xcrun stapler validate "$mountpoint/OpenPlayer.app"
  spctl --assess --type execute --verbose=2 "$mountpoint/OpenPlayer.app"
fi

node apps/desktop/scripts/stage-release-packages.mjs stage "v$version" "macos-$arch" \
  "${PACKAGE_OUTPUT:?Expected a package output directory}" "$mode"
