# Shiny Twitch Creator Tool 1.4.0

## Changes and improvements

- Added built-in stable update checks from the official GitHub Releases repository, automatically at startup and every six hours while the app is open.
- Added an Updates section in Options with a saved automatic-check preference and a manual check button.
- Added platform-specific package selection, background downloading, progress display, cancellation, HTTPS requirements, and size/SHA256 verification.
- Added user-controlled installation and restart, preserving history and preferences while retaining the previous executable as a backup.
- Added recovery when replacement or relaunch fails, a Windows installer helper that waits for the running app to close, and Linux AppImage replacement in writable locations.
- Added English and Spanish updater messages and kept existing language preferences compatible.
