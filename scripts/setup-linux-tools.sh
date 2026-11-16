#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'Linux x86_64 is required.' >&2; exit 1; }
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
tools="$root/.tools/linux-media"
mkdir -p -- "$tools/licenses"
for tool in curl jq unzip sha256sum ffmpeg ffprobe ffplay; do
  command -v "$tool" >/dev/null || { echo "Missing dependency: $tool. See LINUX.md." >&2; exit 1; }
done

# Official standalone binaries, with checksum verification from the same release.
yt_release="$(curl -fsSL --retry 3 https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest)"
yt_base="$(jq -er '.assets[] | select(.name == "yt-dlp_linux") | .browser_download_url' <<<"$yt_release")"
yt_checksums="$(jq -er '.assets[] | select(.name == "SHA2-256SUMS") | .browser_download_url' <<<"$yt_release")"
curl -fsSL --retry 3 "$yt_base" -o "$tools/yt-dlp"
curl -fsSL --retry 3 "$yt_checksums" -o "$tools/yt-dlp-checksums.txt"
yt_hash="$(awk '$2 == "yt-dlp_linux" {print $1}' "$tools/yt-dlp-checksums.txt")"
[[ "$yt_hash" =~ ^[a-fA-F0-9]{64}$ ]] || { echo 'Missing yt-dlp checksum' >&2; exit 1; }
printf '%s  %s\n' "$yt_hash" "$tools/yt-dlp" | sha256sum -c -
curl -fsSL --retry 3 https://raw.githubusercontent.com/yt-dlp/yt-dlp/master/LICENSE -o "$tools/licenses/yt-dlp-LICENSE.txt"

deno_release="$(curl -fsSL --retry 3 https://api.github.com/repos/denoland/deno/releases/latest)"
deno_url="$(jq -er '.assets[] | select(.name == "deno-x86_64-unknown-linux-gnu.zip") | .browser_download_url' <<<"$deno_release")"
deno_digest="$(jq -er '.assets[] | select(.name == "deno-x86_64-unknown-linux-gnu.zip") | .digest | select(startswith("sha256:")) | sub("sha256:"; "")' <<<"$deno_release")"
curl -fsSL --retry 3 "$deno_url" -o "$tools/deno.zip"
printf '%s  %s\n' "$deno_digest" "$tools/deno.zip" | sha256sum -c -
unzip -oq "$tools/deno.zip" deno -d "$tools"
curl -fsSL --retry 3 https://raw.githubusercontent.com/denoland/deno/main/LICENSE.md -o "$tools/licenses/Deno-LICENSE.txt"
chmod 755 "$tools/yt-dlp" "$tools/deno"
if [[ -f /usr/share/doc/ffmpeg/copyright ]]; then
  cp /usr/share/doc/ffmpeg/copyright "$tools/licenses/FFmpeg-copyright.txt"
fi
{
  printf 'yt-dlp release: %s\n' "$(jq -r .tag_name <<<"$yt_release")"
  printf 'Deno release: %s\n' "$(jq -r .tag_name <<<"$deno_release")"
  ffmpeg -version | sed -n '1p'
} > "$tools/BUILD-INFO.txt"
echo "Linux media tools prepared in $tools"
