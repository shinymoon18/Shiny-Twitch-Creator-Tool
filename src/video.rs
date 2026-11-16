use crate::media::{self, Audio, Result};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Info { pub width: u32, pub height: u32, pub fps: f64, pub has_audio: bool, pub alpha: bool }

pub fn load(path: PathBuf) -> Result<Audio> {
    let bytes = media::run(media::command("ffprobe").args(["-v", "error", "-show_format", "-show_streams", "-of", "json"]).arg(&path), 30)?;
    let metadata: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    let streams = metadata["streams"].as_array().ok_or(tr!("No tracks were found in the file."))?;
    let stream = streams.iter().find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1).ok_or(tr!("The file does not contain video. Open a video or an animated GIF."))?;
    let duration = metadata["format"]["duration"].as_str().or_else(|| stream["duration"].as_str()).and_then(|s| s.parse::<f64>().ok()).ok_or(tr!("Could not determine the duration. Choose a video or animated GIF with a defined duration."))?;
    if !duration.is_finite() || duration <= 0.0 || duration > 14400.0 { return Err(tr!("Choose a video up to four hours long.").into()); }
    let mut width = stream["width"].as_u64().unwrap_or(0) as u32;
    let mut height = stream["height"].as_u64().unwrap_or(0) as u32;
    let rotation = stream["side_data_list"].as_array().and_then(|s| s.iter().find_map(|v| v["rotation"].as_f64())).unwrap_or(0.0);
    if (rotation.abs() % 180.0 - 90.0).abs() < 1.0 { std::mem::swap(&mut width, &mut height); }
    if width < 2 || height < 2 { return Err(tr!("The video dimensions are invalid.").into()); }
    let rate = stream["avg_frame_rate"].as_str().unwrap_or("30/1");
    let mut values = rate.split('/').filter_map(|s| s.parse::<f64>().ok());
    let fps = values.next().unwrap_or(30.0) / values.next().unwrap_or(1.0);
    let fps = if fps.is_finite() && fps > 0.0 { fps } else { 30.0 };
    let has_audio = streams.iter().any(|s| s["codec_type"] == "audio");
    let pixel = stream["pix_fmt"].as_str().unwrap_or("");
    let alpha = pixel.contains("yuva") || pixel.contains("rgba") || pixel.contains("bgra") || stream["codec_name"] == "gif" || stream["tags"]["alpha_mode"] == "1";
    let mut audio = if has_audio { media::load(path.clone())? } else { Audio { path: path.clone(), duration, waveform: std::sync::Arc::new(media::Waveform::silent(duration)), video: None } };
    audio.duration = duration;
    audio.video = Some(Info { width, height, fps, has_audio, alpha });
    Ok(audio)
}

pub struct Frame { pub width: usize, pub height: usize, pub rgb: Vec<u8> }
pub fn preview_size(info: &Info) -> (usize, usize) {
    let ratio = (640.0 / info.width as f64).min(320.0 / info.height as f64).min(1.0);
    let width = ((info.width as f64 * ratio).round() as usize).max(2);
    let height = ((info.height as f64 * ratio).round() as usize).max(2);
    (width, height)
}
pub fn frame(audio: &Audio, at: f64) -> Result<Frame> {
    let info = audio.video.as_ref().ok_or(tr!("No video is loaded."))?;
    let (width, height) = preview_size(info);
    let at = at.clamp(0.0, (audio.duration - 1.0 / info.fps).max(0.0));
    let filter = format!("scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2");
    let rgb = media::run(media::command("ffmpeg").args(["-v", "error", "-nostdin", "-ss", &at.to_string(), "-i"]).arg(&audio.path)
        .args(["-map", "0:v:0", "-frames:v", "1", "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"]), 30)?;
    if rgb.len() != width * height * 3 { return Err(tr!("Could not read the video frame.").into()); }
    Ok(Frame { width, height, rgb })
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Quality { Light, #[default] Balanced, High }
#[derive(Clone, Serialize, Deserialize)]
pub struct ExternalAudio { pub path: PathBuf, pub duration: f64, pub start: f64, pub db: f32, #[serde(default)] pub length: Option<f64>, #[serde(default)] pub fades: crate::fades::Fades }
#[derive(Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum SoundMode { Original, #[default] Silent, Replace }
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Options { pub height: u32, pub quality: Quality, pub limit_fps: bool, pub mode: SoundMode, pub original_db: f32, pub external: Option<ExternalAudio>, pub master_db: f32, pub fades: crate::fades::Fades }
impl Default for Options { fn default() -> Self { Self { height: 720, quality: Quality::Balanced, limit_fps: true, mode: SoundMode::Silent, original_db: 0.0, external: None, master_db: 0.0, fades: crate::fades::Fades::default() } } }

pub fn external_audio(path: PathBuf) -> Result<ExternalAudio> {
    let bytes = media::run(media::command("ffprobe").args(["-v", "error", "-select_streams", "a:0", "-show_entries", "stream=codec_type:format=duration", "-of", "json"]).arg(&path), 15)?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if value["streams"].as_array().is_none_or(|s| s.is_empty()) { return Err(tr!("The file does not contain an audio track.").into()); }
    let duration = value["format"]["duration"].as_str().and_then(|v| v.parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0 && *v <= 14400.0).ok_or(tr!("The audio must have a valid duration of up to four hours."))?;
    Ok(ExternalAudio { path, duration, start: 0.0, db: 0.0, length: None, fades: crate::fades::Fades::default() })
}

pub fn audio_graph(info: &Info, options: &Options, length: f64) -> Result<Option<String>> {
    if !options.master_db.is_finite() || !(-30.0..=18.0).contains(&options.master_db) { return Err(tr!("The output adjustment must be between -30 and +18 dB.").into()); }
    if !options.original_db.is_finite() || !(-60.0..=12.0).contains(&options.original_db) { return Err(tr!("The original audio volume must be between -60 and +12 dB.").into()); }
    let mut graph = String::new(); let mut inputs = Vec::new();
    if options.mode == SoundMode::Original && info.has_audio {
        graph.push_str(&format!("[0:a:0]asetpts=PTS-STARTPTS,aresample=48000,aformat=channel_layouts=stereo,volume={:.3}dB,apad,atrim=duration={length:.6}[original];", options.original_db));
        inputs.push("[original]");
    }
    if options.mode == SoundMode::Replace {
        let extra = options.external.as_ref().ok_or(tr!("Choose a local audio file or select Export without audio."))?;
        if !extra.db.is_finite() || !(-60.0..=12.0).contains(&extra.db) || !extra.start.is_finite() || extra.start < 0.0 || extra.start >= extra.duration { return Err(tr!("Check the added audio's volume and start time.").into()); }
        if !extra.path.is_file() { return Err(tr!("The added audio file could not be found. Select it again.").into()); }
        let take = extra.length.unwrap_or(extra.duration - extra.start);
        if !take.is_finite() || take <= 0.0 || extra.start + take > extra.duration + 0.001 { return Err(tr!("The added audio selection is invalid. Check its start and end times.").into()); }
        graph.push_str(&format!("[1:a:0]atrim=duration={:.6},asetpts=PTS-STARTPTS,aresample=48000,aformat=channel_layouts=stereo,{},volume={:.3}dB,apad,atrim=duration={length:.6}[extra];", take.min(length), extra.fades.filter(take.min(length)), extra.db));
        inputs.push("[extra]");
    }
    if inputs.is_empty() { return Ok(None); }
    graph.push_str(&inputs.concat());
    graph.push_str(&format!("{},volume={:.3}dB,alimiter=limit=0.95:level=0:latency=1[mixed]", options.fades.filter(length), options.master_db));
    Ok(Some(graph))
}

pub fn add_external_input(cmd: &mut std::process::Command, options: &Options) {
    if options.mode == SoundMode::Replace { if let Some(extra) = &options.external { cmd.args(["-ss", &format!("{:.6}", extra.start), "-i"]).arg(&extra.path); } }
}

pub fn output_size(info: &Info, height: u32) -> (u32, u32) {
    let h = if height == 0 { info.height } else { info.height.min(height) };
    let h = (h / 2 * 2).max(2);
    let w = ((info.width as f64 * h as f64 / info.height as f64).round() as u32 / 2 * 2).max(2);
    (w, h)
}

pub fn export(audio: &Audio, start: f64, length: f64, output: &Path, options: &Options) -> Result<()> {
    let info = audio.video.as_ref().ok_or(tr!("No video is loaded."))?;
    if !start.is_finite() || !length.is_finite() || start < 0.0 || length <= 0.0 || start + length > audio.duration + 0.001 { return Err(tr!("The selected range is invalid.").into()); }
    if !output.extension().is_some_and(|e| e.eq_ignore_ascii_case("webm")) { return Err(tr!("The output filename must end in .webm.").into()); }
    if output.exists() && std::fs::canonicalize(output).ok() == std::fs::canonicalize(&audio.path).ok() { return Err(tr!("Choose a different filename to preserve the original file.").into()); }
    if let Some(extra) = &options.external { if output.exists() && std::fs::canonicalize(output).ok() == std::fs::canonicalize(&extra.path).ok() { return Err(tr!("The output cannot overwrite the added audio file.").into()); } }
    let graph = audio_graph(info, options, length)?;
    let (width, height) = output_size(info, options.height);
    let (crf, bitrate) = match options.quality { Quality::Light => ("37", "64k"), Quality::Balanced => ("32", "96k"), Quality::High => ("26", "128k") };
    let mut filter = format!("scale={width}:{height}:flags=lanczos,setsar=1");
    if options.limit_fps && info.fps > 30.0 { filter.push_str(",fps=30"); }
    let temp = tempfile::NamedTempFile::new_in(output.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    let mut cmd = media::command("ffmpeg");
    cmd.args(["-v", "error", "-nostdin", "-y", "-ss", &format!("{start:.6}"), "-i"]).arg(&audio.path);
    add_external_input(&mut cmd, options);
    cmd.args(["-t", &format!("{length:.6}"), "-map", "0:v:0", "-vf", &filter, "-c:v", "libvpx-vp9", "-b:v", "0", "-crf", crf, "-deadline", "good", "-cpu-used", "3", "-row-mt", "1", "-pix_fmt", if info.alpha { "yuva420p" } else { "yuv420p" }]);
    if info.alpha { cmd.args(["-auto-alt-ref", "0"]); }
    if let Some(graph) = graph { cmd.args(["-filter_complex", &graph, "-map", "[mixed]", "-c:a", "libopus", "-b:a", bitrate]); } else { cmd.arg("-an"); }
    media::run(cmd.args(["-map_metadata", "-1", "-f", "webm"]).arg(temp.path()), 7200)?;
    temp.persist(output).map_err(|e| tf!("Could not save the WebM file: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn samples(path: &Path) -> Vec<f32> {
        let pcm = media::run(media::command("ffmpeg").args(["-v", "error", "-i"]).arg(path).args(["-vn", "-ar", "48000", "-ac", "1", "-f", "f32le", "pipe:1"]), 20).unwrap();
        pcm.chunks_exact(4).map(|b| f32::from_le_bytes([b[0],b[1],b[2],b[3]])).collect()
    }
    fn amplitude(pcm: &[f32], hz: f64) -> f64 {
        let section = &pcm[9600..24000]; let mut sin = 0.0; let mut cos = 0.0;
        for (i, value) in section.iter().enumerate() { let phase = std::f64::consts::TAU * hz * i as f64 / 48000.0; sin += *value as f64 * phase.sin(); cos += *value as f64 * phase.cos(); }
        2.0 * sin.hypot(cos) / section.len() as f64
    }
    #[test] fn webm_is_muted_by_default_and_external_audio_mixes_at_selected_decibels() {
        let dir = tempfile::tempdir().unwrap(); let source = dir.path().join("video.mp4");
        media::run(media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=96x64:rate=10:duration=2", "-f", "lavfi", "-i", "sine=frequency=440:duration=2", "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-shortest"]).arg(&source), 20).unwrap();
        let source = load(source).unwrap();
        let extra_path = dir.path().join("audio añadido.wav");
        media::run(media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=660:duration=1", "-f", "lavfi", "-i", "sine=frequency=880:duration=1", "-f", "lavfi", "-i", "sine=frequency=1100:duration=1", "-filter_complex", "[0:a][1:a][2:a]concat=n=3:v=0:a=1[a]", "-map", "[a]"]).arg(&extra_path), 20).unwrap();
        let mut extra = external_audio(extra_path).unwrap(); extra.start = 1.0; extra.length = Some(0.75);
        let muted = dir.path().join("silencioso.webm");
        export(&source, 0.125, 1.5, &muted, &Options { external: Some(extra.clone()), ..Default::default() }).unwrap(); assert!(!load(muted).unwrap().video.unwrap().has_audio);
        let original = dir.path().join("original.webm");
        export(&source, 0.125, 1.5, &original, &Options { mode: SoundMode::Original, external: Some(extra.clone()), ..Default::default() }).unwrap();
        let original_pcm = samples(&original);
        let base_original = amplitude(&original_pcm, 440.0); assert!(base_original > 0.05);
        assert!(amplitude(&original_pcm, 880.0) < base_original * 0.05, "cached local file leaked into original mode");
        let added = dir.path().join("añadido.webm");
        export(&source, 0.125, 1.5, &added, &Options { mode: SoundMode::Replace, external: Some(extra.clone()), ..Default::default() }).unwrap();
        let added_pcm = samples(&added); let base_extra = amplitude(&added_pcm, 880.0); assert!(base_extra > 0.05);
        assert!(amplitude(&added_pcm, 440.0) < base_extra * 0.05, "original leaked into muted mix");
        assert!(amplitude(&added_pcm, 660.0) < base_extra * 0.05 && amplitude(&added_pcm, 1100.0) < base_extra * 0.05, "audio started outside the selected section");
        assert!(added_pcm[48000..60000].iter().all(|v| v.abs() < 0.005), "short external audio was not padded with silence");
        extra.db = -12.0; extra.length = Some(0.35);
        let mixed = dir.path().join("mezcla.webm");
        export(&source, 0.125, 1.5, &mixed, &Options { mode: SoundMode::Replace, original_db: -6.0, external: Some(extra), ..Default::default() }).unwrap();
        let pcm = samples(&mixed);
        assert!(amplitude(&pcm, 440.0) < base_original * 0.05, "original audio remained in replacement");
        assert!(pcm[24000..60000].iter().all(|v| v.abs() < 0.005), "audio played past the chosen end");
        assert!((load(mixed).unwrap().duration - 1.5).abs() < 0.15);
    }
    #[test] fn real_video_and_gif_convert_to_webm_with_selected_size_and_duration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("original.mp4");
        media::run(media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=320x180:rate=24:duration=14", "-f", "lavfi", "-i", "anullsrc=r=48000:cl=stereo", "-t", "14", "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac"]).arg(&path), 40).unwrap();
        let audio = load(path.clone()).unwrap(); let info = audio.video.as_ref().unwrap();
        assert!(info.has_audio); assert_eq!(output_size(info, 1080), (320, 180));
        assert_eq!(frame(&audio, 2.0).unwrap().rgb.len(), 320 * 180 * 3);
        for length in [9.0, 11.0] {
            let dest = dir.path().join(format!("corte-{length}.webm"));
            export(&audio, 1.125, length, &dest, &Options { height: 120, mode: SoundMode::Original, ..Default::default() }).unwrap();
            let result = load(dest).unwrap();
            assert!((result.duration - length).abs() < 0.1);
            let info = result.video.unwrap(); assert_eq!((info.width, info.height), (212, 120)); assert!(info.has_audio);
        }
        let gif = dir.path().join("animacion.gif");
        media::run(media::command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=96x64:rate=10:duration=2"]).arg(&gif), 20).unwrap();
        let gif = load(gif).unwrap(); assert!(!gif.video.as_ref().unwrap().has_audio);
        let dest = dir.path().join("animacion.webm"); export(&gif, 0.0, gif.duration, &dest, &Options::default()).unwrap();
        let converted = load(dest).unwrap(); assert!((converted.duration - gif.duration).abs() < 0.15); assert!(!converted.video.unwrap().has_audio);
    }
}
