# Third-party components

The single-file portable executable contains compressed media tools and prepares them, together with their license notices, in the portable data folder on first launch. Rust dependency versions are recorded in `Cargo.lock`.

## Rust libraries

- [egui / eframe](https://github.com/emilk/egui): MIT or Apache-2.0.
- [rfd](https://github.com/PolyMeilex/rfd): MIT.
- [serde / serde_json](https://github.com/serde-rs): MIT or Apache-2.0.
- [directories](https://github.com/dirs-dev/directories-rs): MIT or Apache-2.0.
- [tempfile](https://github.com/Stebalien/tempfile): MIT or Apache-2.0.
- [chrono](https://github.com/chronotope/chrono): MIT or Apache-2.0.
- [flate2](https://github.com/rust-lang/flate2-rs): MIT or Apache-2.0; used to decompress the portable bundle.

## Media tools

**FFmpeg, ffprobe, and ffplay** are downloaded from [Gyan's builds](https://www.gyan.dev/ffmpeg/builds/), a distributor linked from the [official FFmpeg download page](https://ffmpeg.org/download.html). The essentials builds include GPL components. Refer to the included license and the distributor's build details for the specific binary. [FFmpeg source information](https://ffmpeg.org/download.html) is available upstream. The app runs FFmpeg as a separate process.

**yt-dlp** is downloaded from its [official releases](https://github.com/yt-dlp/yt-dlp/releases). Its core source is distributed under the Unlicense; packaged executables include components with their own licenses. See the [upstream license section](https://github.com/yt-dlp/yt-dlp#license) and the notice included beside the tools.

**Deno** is included as the JavaScript runtime used by yt-dlp for YouTube support. It is distributed under MIT with additional third-party notices in its license. See the [Deno repository](https://github.com/denoland/deno) and the included notice.

For YouTube JavaScript challenges, yt-dlp may download its official [EJS component](https://github.com/yt-dlp/ejs) using `--remote-components ejs:github`. Consult that project's license and the [EJS documentation](https://github.com/yt-dlp/yt-dlp/wiki/EJS).

When redistributing third-party binaries, review the terms, notices, and corresponding source-delivery requirements that apply to the exact builds being distributed.
