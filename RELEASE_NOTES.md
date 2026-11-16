# Shiny Twitch Creator Tool 1.2.1

## Fixes and improvements

- Fixed audio selection before replacing a video's sound: the Start field now moves the complete clip instead of being limited by the previous end time.
- Added a full-track Choose audio start slider and clearer waveform selection handles.
- Automatically select the current video clip's duration and focus that range when opening or reopening the local-audio editor. Shorter audio uses its actual duration.
- Added Match video duration to restore the required length after manual changes. Full-file video conversion uses the full output duration when opening the audio editor from its export dialog.
- Retained English/Spanish options, fades, manual volume adjustment, and the YouTube download correction.

## Run

Open `Shiny-Twitch-Creator-Tool-Portable-1.2.1.exe` in a writable folder. The app includes its media tools and extracts them into `Audio Twitch Cut Datos` beside the executable.

Download the executable and its SHA256 file from the release assets. No separate installation of Rust or the media tools is required.

## Validation

29 local tests passed, including interaction tests for the trim window, automatic duration selection, selection movement, exports, and audio replacement. The optional network test was not rerun for this editor change; it passed during the preceding download update.
