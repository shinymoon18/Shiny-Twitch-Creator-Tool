# Linux port

The Linux target is **x86_64 (AMD64)**: the same package supports 64-bit AMD and Intel CPUs. ARM and 32-bit CPUs are not targeted.

The portable format is AppImage, built on Debian 12 with a glibc 2.36 baseline. The package is checked in Debian 12 and Arch Linux containers. The supported targets are Debian 12 or newer and an updated Arch Linux x86_64 installation; this does not guarantee compatibility with every distribution.

## Runtime specifications

- **CPU:** x86_64 / AMD64, for both AMD and Intel processors. No ARM or 32-bit build is provided.
- **System libraries:** glibc 2.36 or newer and standard C/C++ runtime libraries.
- **Desktop graphics:** a working X11 or Wayland session and OpenGL/Mesa or vendor graphics drivers. GPU drivers are supplied by the host system.
- **Sound:** an available audio device and working host audio stack for audible playback.
- **Desktop integration:** `xdg-desktop-portal` and the backend appropriate to the desktop for file dialogs; `xdg-utils` for opening folders.
- **AppImage runtime:** FUSE support, or the documented extraction mode as an alternative.
- **Network:** YouTube access and, when EJS is needed, access to its official GitHub download.
- **Included tools:** FFmpeg, ffprobe, ffplay, yt-dlp, Deno, and non-system multimedia libraries. Separate Rust or FFmpeg installations are not required for the released AppImage.

## Application behavior

- X11 and Wayland backends are enabled on Linux.
- The editor keeps the audio/video tools beside its Linux executable inside the AppImage.
- The read-only AppImage mount is not used for history or preferences. Data follows XDG conventions through the existing application identity, normally under `~/.local/share/audiotwitchcut` (or `$XDG_DATA_HOME/audiotwitchcut`).
- Downloads and exported clips use Linux paths. Folder opening uses the system's `xdg-open`.
- File dialogs use the desktop portal through rfd. A working `xdg-desktop-portal` service and a backend appropriate for the desktop are required; minimal window-manager installations may need to install them.
- System language follows Linux locale variables, with English/Spanish overrides retained in Options.
- Cancellation terminates the Unix process group so FFmpeg/Deno children are stopped with a canceled download.
- The built-in updater checks stable GitHub Releases and selects the Linux x86_64 asset. Installation requires a writable AppImage location; it replaces the AppImage, keeps a backup, and leaves XDG user data intact. Plain native builds and system-managed installations must be updated separately.

## Install build dependencies

On Debian 12 or newer:

```bash
sudo apt-get update
sudo apt-get install --no-install-recommends \
  build-essential pkg-config curl ca-certificates git jq unzip file patchelf \
  desktop-file-utils ffmpeg libx11-dev libxi-dev libxcursor-dev libxrandr-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libwayland-dev libegl1-mesa-dev \
  libgl1-mesa-dev libssl-dev libdbus-1-dev xdg-utils \
  xdg-desktop-portal xdg-desktop-portal-gtk
```

On Arch Linux:

```bash
sudo pacman -Syu --needed base-devel pkgconf curl ca-certificates git jq unzip \
  file patchelf desktop-file-utils ffmpeg libx11 libxi libxcursor libxrandr \
  libxkbcommon libxkbcommon-x11 wayland mesa libglvnd openssl dbus \
  xdg-utils xdg-desktop-portal xdg-desktop-portal-gtk
```

Use the portal backend for your desktop instead of the GTK backend when appropriate. Install stable Rust using the official instructions at https://rustup.rs/ and ensure `cargo` is on PATH. Build AppImages on the oldest supported base (Debian 12); building on current Arch can raise the minimum required glibc version.

## Build and test

```bash
bash scripts/setup-linux-tools.sh
bash scripts/build-linux.sh --check
bash scripts/build-linux.sh --test
bash scripts/build-appimage.sh
```

The AppImage and its `.sha256` file appear in `dist`. FFmpeg, ffprobe, ffplay, yt-dlp, Deno, and their required non-system libraries are included. Graphics drivers, glibc, the desktop portal, and the desktop session come from the host.

The setup script obtains official yt-dlp and Deno Linux binaries and checks their release-provided SHA256 values. linuxdeploy packages shared libraries from the build system. See `THIRD-PARTY.md` and the included license notices before redistribution.

To build only the native executable, run `bash scripts/build-linux.sh --build`. To run the optional live YouTube test, use `bash scripts/build-linux.sh --youtube-test`.

## Open an AppImage

```bash
chmod +x Shiny-Twitch-Creator-Tool-*-x86_64.AppImage
./Shiny-Twitch-Creator-Tool-<version>-x86_64.AppImage
```

Replace `<version>` with the downloaded filename. If the host lacks FUSE support, the AppImage runtime supports extraction mode:

```bash
APPIMAGE_EXTRACT_AND_RUN=1 ./Shiny-Twitch-Creator-Tool-<version>-x86_64.AppImage
```

`--portable-check` verifies that bundled tools resolve correctly and performs a real nine-second WAV export, without opening the editor. Linux data stays in the writable user data directory.

## Build automation

`.github/workflows/linux-appimage.yml` prepares the AppImage on Debian 12, runs Linux engine tests and a headless startup check, and checks the generated AppImage in Debian and Arch containers. It uploads build artifacts, not GitHub Releases. It is triggered manually or by a push to `port/linux-appimage`; changing local files does not publish anything.

Real audio-device playback, interactive portal dialogs, Wayland desktop integration, and behavior on physical Debian/Arch desktops still need interactive testing on those systems. No screen access is used by the automated checks.
