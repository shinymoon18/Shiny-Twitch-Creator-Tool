# Validation

Latest local updater checks: November 16, 2026. The published Windows/Linux platform baseline was checked on November 15, 2026.

## Local updater checks

- 35 Windows tests passed, including platform/version selection, digest requirements, URL restrictions, download size/hash checks, cancellation, backup preservation, recovery after a real invalid-executable launch failure, and waiting for the previous Windows process to exit.
- The updated source passed `cargo check` for Linux x86_64 using a cross compiler for its TLS dependency. This is a compilation check, not a native Linux updater-installation test.
- A separate optional GitHub download test exercises the actual HTTPS client, redirects, complete package download, and SHA256 verification without installing the downloaded program. Run it with `powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -UpdateTest`.
- The optional GitHub test passed against the actual latest published package, including its complete download and digest verification.
- The real Windows installer helper passed an isolated integration check: it waited for a previous fixture process to exit, replaced the fixture executable, preserved the original backup, and restarted the replacement in CLI-only update-check mode. No graphical window or existing user application was touched.
- No desktop access is used for the local updater checks. Before publishing an AppImage, the Linux release workflow also requires native tests, a real package-download verification, and an isolated AppImage installation/restart probe. The probe verifies replacement, preservation of the previous file and an already-open handle, and the restarted program's reported version without opening its interface.

## Automated checks

The published platform baseline passed **29 tests on Windows** and **30 tests on native Linux**. Linux adds a Unix process-group cancellation test. The optional YouTube test is excluded from the regular suite and passed for the preceding download update.

The Linux AppImage passed its bundled-tool check and real nine-second WAV export in Debian 12 and Arch Linux containers without installing FFmpeg on those test hosts. Graphical startup under Xvfb also passed on the Debian build system. These are automated checks; hardware audio, interactive desktop portals, and native Wayland desktop sessions are not covered by them.

Coverage includes:

- Preset durations, short tracks, selection boundaries, independent edge resizing, fractional duration changes, and precise step controls.
- Zoom limits, viewport anchoring, and detection of a one-millisecond waveform transient.
- Absolute playback timing and start/end markers, including rejection of an end before the start.
- Local M3U paths with spaces, relative paths, UTF-8 BOM handling, rejection of network playlists, and rejection of non-YouTube hosts.
- History persistence and replacement.
- Real WAV/MP3 exports with fractional start positions, a five-millisecond clip, source-file protection, and sample-level WAV duration checks.
- Real MP4/GIF conversion to WebM, output resolution, nine- and eleven-second selections, expected audio-track presence, and unchanged video duration.
- Replacement of original video sound with only the chosen local-audio section. A fixture with distinct tones verifies that original and unselected audio do not leak into the output and that a short selection is followed by silence.
- Integrated video preview with changing decoded frames, with and without audio, including local-audio playback and absolute video-clock offsets.
- Manual gain: measured loudness and true-peak changes after a +6 dB adjustment and a comparison with the exported WAV.
- Fade limits and non-finite values, attenuation at the start/end of a real export, and agreement between measured and exported loudness.
- Waveform interaction: dragging each fade handle changes the appropriate fade without dragging the selection.
- Language selection for Spanish regional locales with an English fallback, preference serialization, and translated messages preserving paths and numeric precision.
- Portable tool extraction, cache reuse, rejection of escaping paths, and detection of corrupted compressed payloads.
- Audio replacement editor: seeking and dragging inside the actual egui trim window, preserving the selected duration, matching the current video length on opening/reopening, jumping beyond the previous end, fitting near the end of the audio, and handling a shorter source. These checks simulate egui input without accessing the desktop.

## YouTube verification

The reported video was `https://www.youtube.com/watch?v=qLhj3HJ-5v8`.

A direct video download reproduced HTTP 403. Enabling `--remote-components ejs:github` allowed the download to complete. The application's actual download functions were then tested separately for MP3 audio and video with audio, and both outputs loaded successfully.

This verifies the reported public video on the tested network. It does not guarantee availability of private or restricted videos, or compatibility with future YouTube changes.

## Portable executable

The final portable executable completed `--portable-check` successfully:

- All five media tools resolved from the embedded bundle: FFmpeg, ffprobe, ffplay, yt-dlp, and Deno.
- A real nine-second WAV clip was exported successfully.
- The executable used a data folder beside itself.

The editor is built with the static C runtime. The package includes third-party notices and extracts the tools on first use. Existing data-folder and preference filenames are retained for compatibility.

## Interface checks and limits

A visual check confirmed the Options menu, switching the interface to Spanish, and writing the `Spanish` preference to `settings.json`. Fade-handle interaction was also verified through an egui input test.

File-based tests were run outside the development sandbox because its temporary-directory protections blocked atomic file replacement. Export content and duration were checked through the same media engine used by the app. Device audio output was not verified by listening, and a complete export through the graphical save dialog was not part of this verification.

No desktop access is required to run the automated checks.

## Reproduce the checks

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Check
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Test
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -YouTubeTest
```

The final command requires network access and downloads a public test video. Run the generated portable executable with `--portable-check` to verify its bundled tools and export path.
