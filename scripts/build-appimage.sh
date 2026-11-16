#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'Linux x86_64 is required.' >&2; exit 1; }
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$root"
for tool in curl jq file patchelf desktop-file-validate ldconfig; do
  command -v "$tool" >/dev/null || { echo "Missing build dependency: $tool. See LINUX.md." >&2; exit 1; }
done
bash scripts/setup-linux-tools.sh
bash scripts/build-linux.sh --build
version="$(cargo metadata --no-deps --format-version 1 | jq -er '.packages[] | select(.name == "audio-twitch-cut") | .version')"
mkdir -p -- "$root/.tools" "$root/dist"
work="$(mktemp -d "$root/.tools/appimage-build.XXXXXX")"
appdir="$work/Shiny.AppDir"
mkdir -p -- "$appdir/usr/bin" "$appdir/usr/share/doc/shiny-twitch-creator-tool"
install -m 755 target/release/audio-twitch-cut "$appdir/usr/bin/audio-twitch-cut"
for tool in ffmpeg ffprobe ffplay; do
  install -m 755 "$(command -v "$tool")" "$appdir/usr/bin/$tool"
done
for tool in yt-dlp deno; do
  install -m 755 "$root/.tools/linux-media/$tool" "$appdir/usr/bin/$tool"
done
cp README.md THIRD-PARTY.md LINUX.md "$appdir/usr/share/doc/shiny-twitch-creator-tool/"
cp -R .tools/linux-media/licenses "$appdir/usr/share/doc/shiny-twitch-creator-tool/"
cp .tools/linux-media/BUILD-INFO.txt "$appdir/usr/share/doc/shiny-twitch-creator-tool/"
install -m 755 packaging/linux/AppRun "$appdir/AppRun"
desktop="packaging/linux/io.github.shinymoon18.ShinyTwitchCreatorTool.desktop"
desktop-file-validate "$desktop"

deploy="$root/.tools/linuxdeploy-x86_64.AppImage"
curl -fsSL --retry 3 https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage -o "$deploy"
chmod 755 "$deploy"
args=(--appdir "$appdir" --desktop-file "$root/$desktop" --icon-file "$root/packaging/linux/io.github.shinymoon18.ShinyTwitchCreatorTool.svg")
for tool in audio-twitch-cut ffmpeg ffprobe ffplay deno; do args+=(--executable "$appdir/usr/bin/$tool"); done
# These are loaded dynamically by winit rather than appearing in ldd output.
for name in libX11.so.6 libXcursor.so.1 libXrandr.so.2 libXi.so.6 libxkbcommon.so.0 libxkbcommon-x11.so.0 libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1; do
  path="$(ldconfig -p | awk -v name="$name" '$1 == name {found=$NF} END {print found}')"
  [[ -n "$path" ]] || { echo "Missing desktop library: $name" >&2; exit 1; }
  args+=(--library "$path")
done
export ARCH=x86_64 VERSION="$version" APPIMAGE_EXTRACT_AND_RUN=1
export OUTPUT="$root/dist/Shiny-Twitch-Creator-Tool-$version-x86_64.AppImage"
"$deploy" "${args[@]}" --output appimage
[[ -s "$OUTPUT" ]] || { echo 'AppImage was not generated' >&2; exit 1; }
chmod 755 "$OUTPUT"
(cd -- "$root/dist" && sha256sum "$(basename -- "$OUTPUT")" > "$(basename -- "$OUTPUT").sha256")
echo "AppImage ready: $OUTPUT"
