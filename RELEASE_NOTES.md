# Shiny Twitch Creator Tool 1.3.0

## Changes and improvements

- Added a native Linux x86_64 AppImage for Debian 12+ and Arch Linux, supporting both AMD and Intel 64-bit processors.
- Enabled X11 and Wayland desktop backends while retaining the native Windows application.
- Bundled FFmpeg, ffprobe, ffplay, yt-dlp, Deno, and multimedia dependencies in the AppImage, including audio libraries omitted by generic packaging defaults.
- Added writable XDG storage for Linux history, preferences, and downloads, separate from the read-only AppImage contents.
- Adapted Linux folder opening, locale detection, and process-group cancellation for downloads and playback.
- Isolated the host file manager from the AppImage's bundled libraries.
- Expanded the English documentation with Windows/Linux compatibility, CPU architecture, glibc baseline, desktop requirements, and known testing limits.
- Preserved audio/video editing, local-audio replacement, automatic clip duration, fades, volume controls, and English/Spanish interface options across both platforms.
