# Shiny Twitch Creator Tool 1.2.0

Portable audio, video, and GIF editor for Twitch alerts on Windows 10/11 x64.

## Changes

- Added English and Spanish interfaces, automatic selection from the Windows display language, and a saved manual language preference under Options.
- Fixed a reproduced YouTube HTTP 403 download failure by enabling yt-dlp's official EJS JavaScript component. Audio and video downloads were verified with the reported link.
- Added fade-in and fade-out controls: drag the green waveform handles or enter durations with millisecond precision.
- Applied fades consistently to selection preview, exported audio, and loudness measurements, including local audio used to replace a video's sound.
- Retained complete replacement of original video audio, the independent local-audio trim editor, and the original-audio and silent-export options.

## Download and run

Download the portable `.exe` from Assets and open it in a writable folder. Rust, FFmpeg, and yt-dlp do not need to be installed separately.

The first launch extracts its media tools into `Audio Twitch Cut Datos` beside the executable. Copy the executable to move the app; copy that data folder too if you want to retain history and preferences. YouTube features require an internet connection.

A SHA256 file is provided to verify the downloaded executable. The app includes third-party notices; see `THIRD-PARTY.md` in the repository.

## Validation

- 27 local automated tests passed.
- The separate live-network test downloaded both MP3 audio and video with audio from the reported YouTube link.
- The final portable executable passed its embedded-tool check and a real nine-second WAV export.
