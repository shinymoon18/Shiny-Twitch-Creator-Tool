# Shiny Twitch Creator Tool

A portable Windows desktop editor built with Rust for creating Twitch alerts, ringtones, and notifications from audio, video, and animated GIFs.

[Download the latest portable app](https://github.com/shinymoon18/Shiny-Twitch-Creator-Tool/releases/latest)

## Features

- Open local audio, video, and animated GIF files, or download audio and video from YouTube.
- Start with 9- or 11-second Twitch alerts, a 30-second ringtone, or 1-, 1.5-, 3-, 4-, and 5-second notifications.
- Set any custom duration using editable start/end times, draggable selection edges, and waveform zoom.
- Move the start, end, or entire selection in steps of 1 ms, 5 ms, 100 ms, 500 ms, or 1 minute.
- Extend or shorten a clip by those amounts while keeping its start fixed.
- Preview audio and video inside the editor and mark start/end points during playback.
- Adjust fade-in and fade-out by dragging the green waveform handles or entering precise durations.
- Keep a video's original audio, export it silently, or replace its sound with a selected section of a local audio file.
- Measure loudness and true peaks, adjust gain manually, and compare alerts against a personal LUFS target.
- Export WAV/MP3 audio and VP9 WebM video with selectable resolution and quality.
- Keep a history of exported clips and choose where to save each file.
- Choose English or Spanish, follow the Windows display language, and customize dark themes, accent colors, and interface scale.
- Run from a single portable executable with the media tools included.

## Download and run

1. Open [Releases](https://github.com/shinymoon18/Shiny-Twitch-Creator-Tool/releases/latest) and download the portable `.exe` under **Assets**.
2. Place it in a folder where you have write permission, then run it. Rust and the media tools do not need to be installed separately.
3. The first launch extracts the bundled tools into **Audio Twitch Cut Datos** beside the executable. This can take a few seconds and requires space for the extracted tools as well as the executable.

To move just the app, copy the executable. To keep your history, preferences, and downloads too, copy **Audio Twitch Cut Datos** as well. Source files and exported clips saved elsewhere must be copied separately; history entries retain their original paths.

The existing data-folder name is retained for compatibility. You do not need to run the app as administrator.

## Edit audio

1. Select **Audio**, then **Open audio or M3U playlist**. WAV, MP3, FLAC, OGG, and M4A files are supported. M3U/M3U8 playlists can reference local files using absolute paths or paths relative to the playlist.
2. Alternatively, paste a YouTube link and choose **Download audio only**.
3. Choose a preset or set the start, end, and duration directly. Drag the selection edges to resize it, or drag inside the selection to move it.
4. Use **Preview selection** or **Preview from here**, then **Mark start** and **Mark end** to capture points during playback.
5. Choose WAV or MP3 and select **Trim and save…**. The save dialog lets you choose the folder and filename.

The status message shows the output path. **Show latest clip** opens its location, and **Recent clips** preserves the source, interval, date, and destination between sessions.

If a preset fits the track but not the remaining time, its start is adjusted to preserve its duration. If the entire track is shorter, its actual duration is used without adding silence. WAV is the best choice when sample-accurate duration matters; MP3 can include encoder delay.

### Zoom and precise selection

**Zoom in +**, **Zoom out -**, **Fit selection**, and **Fit entire track** control the visible range. The minimum visible window is 50 ms. The waveform stores peaks at millisecond resolution and includes a time ruler.

Use **Scroll track**, **Previous view**, and **Next view** to explore without changing the selection. In the step controls, choose **Start**, **End**, or **Selection** before moving backward or forward. Editing the start keeps the end fixed; editing the end keeps the start fixed; moving the selection keeps its duration.

The direct **Add / Remove** buttons change duration while keeping the start fixed. For example, adding 1 ms to a 9-second clip produces 9.001 seconds. Repeated clicks accumulate. The selection cannot cross the track boundaries or become shorter than 1 ms, except when the whole source is shorter.

### Fade-in and fade-out

Drag the green **IN** handle to change the fade-in duration and the green **OUT** handle to change the fade-out duration. The waveform displays the attenuation. You can also enter exact times in **Fade in (s)** and **Fade out (s)**; **Reset fades** disables both.

Fades apply to selection preview, exported audio, and loudness measurements. When the clip becomes shorter, fade durations are limited to its length. The local-audio editor used for replacing video sound includes the same controls.

## Edit video and GIFs

Select **Video / GIF** and open a local file or choose **Download YouTube video**. The app supports MP4, GIF, WebM, MKV, MOV, AVI, and other formats FFmpeg can decode; **All files** allows selecting additional compatible formats.

Audio and video keep separate files, selections, and zoom settings while you switch tabs. Presets, precision steps, custom durations, and playback markers work in both workspaces. Video cuts align to the source frames, even when millisecond steps are used.

The video preview plays inside the editor. It uses reduced resolution and up to 30 fps for responsiveness; exported video uses the source and your chosen output settings.

Use **Save WebM clip…** to export the selection, or **Convert entire file to WebM…** to convert the complete video or animated GIF. In the export dialog:

- Choose the original resolution or a maximum height. The app keeps the aspect ratio and does not upscale.
- Choose **Smaller file**, **Balanced**, or **Higher quality**.
- Optionally limit sources above 30 fps to 30 fps.
- Select the audio source and preview the result before saving.

Video is encoded with VP9; included audio uses Opus. The app attempts to preserve source transparency. Final file size depends on duration, resolution, motion, frame rate, and quality, so a fixed size cannot be guaranteed.

### Choose the video's audio

In **WebM audio**, choose one of these modes:

- **Keep original audio:** use only the video's own audio, if present.
- **Export without audio:** create a WebM with no audio track. This is the default.
- **Replace with local audio:** remove the original audio completely and use a selected section of a local audio file.

When replacing audio, choose a file to open **Trim audio for the video**. Use its waveform, zoom, start/end/duration fields, precision steps, playback markers, and fades to select the desired section. Choose **Use this audio clip in the video** to apply it; **Trim this audio…** reopens the editor.

The selected audio starts at the beginning of the video clip. If it is shorter, the remainder is silent; if longer, it is trimmed to the video duration. It is not looped. The video duration and original audio file are preserved. Preview and export use the same source selection.

## Match alert volume

Select **Normalize volume…** to open the alert-volume window. The adjustment is manual:

1. Select the desired clip and choose **Measure this clip** to see perceived loudness in LUFS and true peak in dBTP.
2. Adjust **Adjustment / dB**, use the -1/+1 dB buttons, or enter a value. Negative values reduce volume; positive values increase it. **Reset** returns to 0 dB. The output gain range is -30 to +18 dB.
3. Choose **Preview result**. Once satisfied, close the window and save the clip normally.
4. To compare alerts, use the same **Target / LUFS**. The app shows the difference toward that target; applying it remains your choice.
5. Measure again after changing the selection, audio sources, fades, or gain. Previous measurements are marked as outdated.

The initial -18 LUFS target is an editable personal reference, not a Twitch requirement. Silent or very short clips may not produce a valid loudness reading. Peak protection can limit large boosts, so preview and measure again. Source-specific volume controls are available under **Source volume (advanced)**.

## Language and appearance

Open **Options → Language / Idioma** and choose **System default**, **English**, or **Español**. On first launch, Spanish Windows display languages select Spanish; other display languages use English. A manual choice is saved for future launches.

Inside Options, **Customize appearance…** offers Night, OLED, Ocean, Rose, and Forest themes, a custom accent color, and interface scale. All themes are dark.

## YouTube downloads and limitations

YouTube downloads use the bundled yt-dlp, FFmpeg, and Deno. JavaScript challenge support is enabled through `--remote-components ejs:github`, allowing yt-dlp to obtain its official EJS component when required. Downloads need access to YouTube and, when that component is needed, GitHub.

Private, region-restricted, age-restricted, protected, unavailable, or live videos may not download. The app does not import browser cookies or sign in to a YouTube account. Service changes can require updating yt-dlp. See the [official EJS documentation](https://github.com/yt-dlp/yt-dlp/wiki/EJS).

Audio downloads are stored in `descargas` and video downloads in `videos` inside the app's data folder. Exports are saved wherever you choose.

Supported source duration is up to four hours. Downloads time out after 15 minutes and request limits of 500 MB for audio or 2 GB for video when the server reports file size. Individual streams may have separate sizes, so these are not guaranteed combined-size limits. Network M3U playlists are not supported.

Use **Cancel operation** to stop a load, download, or export. Closing during an operation requests cancellation; close again once it finishes.

## Build from source

Requirements: Windows 10/11 x64, stable Rust with Cargo, Visual Studio C++ Build Tools, and the Windows SDK.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-media.ps1
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Check
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Test
powershell -ExecutionPolicy Bypass -File scripts/build-single.ps1
```

The single-file portable executable is written to `dist`. Its C runtime is linked statically. For a folder-based build instead, run:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

Copy the entire resulting `dist/Shiny Twitch Creator Tool` folder when using that build. During development, media tools are resolved from the embedded bundle, next to the executable, its `tools` folder, `.tools/media`, or PATH. The build scripts also support a local Rust installation under `.tools/cargo` and `.tools/rustup`.

Run the optional live download test separately:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -YouTubeTest
```

A bundled executable can verify its tools and perform a real nine-second WAV export with `--portable-check`.

## Project files

- [Release notes](RELEASE_NOTES.md): changes included in the current update.
- [Validation](VALIDATION.md): checks performed and their limits.
- [Third-party components](THIRD-PARTY.md): dependencies, upstream sources, and notices.

This repository contains source code and build scripts. Generated executables, downloaded tools, and personal app data are excluded from Git; portable binaries are distributed through Releases. Feature documentation describes the current app without per-feature version headings.
