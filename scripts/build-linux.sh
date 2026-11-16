#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Linux && "$(uname -m)" == x86_64 ]] || { echo 'Linux x86_64 is required.' >&2; exit 1; }
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd -- "$root"
case "${1:---build}" in
  --check) cargo check --locked ;;
  --test)
    # Preview tests use a clocked dummy audio device, without opening windows.
    export PATH="$root/.tools/linux-media:$PATH"
    SDL_AUDIODRIVER=dummy SDL_VIDEODRIVER=dummy cargo test --locked
    ;;
  --youtube-test)
    export PATH="$root/.tools/linux-media:$PATH"
    cargo test --locked live_youtube_audio_and_video_download -- --ignored
    ;;
  --build) cargo build --release --locked ;;
  *) echo 'Usage: bash scripts/build-linux.sh [--build|--check|--test|--youtube-test]' >&2; exit 2 ;;
esac
