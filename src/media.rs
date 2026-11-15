use std::{fs, path::{Path, PathBuf}, process::{Child, Command, Stdio}, thread, time::{Duration, Instant}};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

thread_local! { static CANCEL: std::cell::RefCell<Option<Arc<AtomicBool>>> = const { std::cell::RefCell::new(None) }; }
pub fn set_cancellation(flag: Arc<AtomicBool>) { CANCEL.with(|slot| *slot.borrow_mut() = Some(flag)); }
fn cancelled() -> bool { CANCEL.with(|slot| slot.borrow().as_ref().is_some_and(|flag| flag.load(Ordering::Relaxed))) }
fn terminate(child: &mut Child) {
    #[cfg(windows)] { let _ = command("taskkill").args(["/PID", &child.id().to_string(), "/T", "/F"]).stdout(Stdio::null()).stderr(Stdio::null()).status(); }
    let _ = child.kill(); let _ = child.wait();
}

pub type Result<T> = std::result::Result<T, String>;

pub fn tool(name: &str) -> PathBuf {
    let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    if let Some(folder) = crate::portable::tools_dir() {
        let path = folder.join(&filename);
        if path.is_file() { return path; }
    }
    if let Ok(exe) = std::env::current_exe() {
        for folder in [exe.parent().unwrap().to_path_buf(), exe.parent().unwrap().join("tools")] {
            let path = folder.join(&filename);
            if path.is_file() { return path; }
        }
    }
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join(".tools/media").join(&filename);
    if bundled.is_file() { bundled } else { PathBuf::from(filename) }
}

pub fn command(name: &str) -> Command {
    let mut cmd = Command::new(tool(name));
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    cmd.stdin(Stdio::null());
    cmd
}

// Files avoid pipe-buffer deadlocks, including when a downloader is verbose.
pub fn run(cmd: &mut Command, timeout: u64) -> Result<Vec<u8>> {
    run_report(cmd, timeout).map(|(bytes, _)| bytes)
}
pub fn run_report(cmd: &mut Command, timeout: u64) -> Result<(Vec<u8>, String)> {
    if cancelled() { return Err("Operación cancelada.".into()); }
    let mut out = tempfile::tempfile().map_err(|e| e.to_string())?;
    let mut err = tempfile::tempfile().map_err(|e| e.to_string())?;
    cmd.stdout(out.try_clone().map_err(|e| e.to_string())?)
        .stderr(err.try_clone().map_err(|e| e.to_string())?);
    let mut child = cmd.spawn().map_err(|e| format!("No se pudo iniciar {:?}: {e}. Revisa la carpeta tools.", cmd.get_program()))?;
    let started = Instant::now();
    let status = loop {
        if cancelled() { terminate(&mut child); return Err("Operación cancelada.".into()); }
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? { break status; }
        if started.elapsed().as_secs() > timeout {
            terminate(&mut child);
            return Err("La operación excedió el tiempo de espera. Revisa la conexión o prueba otro archivo.".into());
        }
        thread::sleep(Duration::from_millis(70));
    };
    use std::io::{Read, Seek, SeekFrom};
    if !status.success() {
        err.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut message = String::new(); err.read_to_string(&mut message).map_err(|e| e.to_string())?;
        return Err(message.chars().rev().take(1800).collect::<String>().chars().rev().collect());
    }
    out.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new(); out.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
    err.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    let mut report = Vec::new(); err.read_to_end(&mut report).map_err(|e| e.to_string())?;
    Ok((bytes, String::from_utf8_lossy(&report).into_owned()))
}

#[derive(Clone)]
pub struct Waveform { levels: Vec<Vec<f32>>, duration: f64 }
impl Waveform {
    pub fn silent(duration: f64) -> Self { Self { levels: vec![vec![0.0; 1024]], duration } }
    fn from_pcm(pcm: &[u8]) -> Self {
        // One peak per millisecond, then a pyramid for fast whole-track rendering.
        let base: Vec<f32> = pcm.chunks(16).map(|chunk| chunk.chunks_exact(2)
            .map(|b| (i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0).abs()).fold(0.0, f32::max)).collect();
        let duration = pcm.len() as f64 / 16000.0;
        let mut levels = vec![base];
        while levels.last().unwrap().len() > 1024 {
            levels.push(levels.last().unwrap().chunks(4).map(|c| c.iter().copied().fold(0.0, f32::max)).collect());
        }
        Self { levels, duration }
    }
    pub fn peak(&self, start: f64, end: f64) -> f32 {
        let mut level = 0;
        while level + 1 < self.levels.len() && (end - start) * self.levels[level + 1].len() as f64 / self.duration >= 1.0 { level += 1; }
        let peaks = &self.levels[level];
        let from = ((start / self.duration * peaks.len() as f64).floor().max(0.0) as usize).min(peaks.len());
        let to = ((end / self.duration * peaks.len() as f64).ceil().max(0.0) as usize).min(peaks.len());
        peaks[from..to.max(from)].iter().copied().fold(0.0, f32::max)
    }
}

#[derive(Clone)]
pub struct Audio { pub path: PathBuf, pub duration: f64, pub waveform: Arc<Waveform>, pub video: Option<crate::video::Info> }

pub fn load(path: PathBuf) -> Result<Audio> {
    if !path.is_file() { return Err("La pista no existe. Comprueba la ruta de la lista M3U.".into()); }
    let bytes = run(command("ffprobe").args(["-v", "error", "-show_entries", "format=duration", "-of", "default=noprint_wrappers=1:nokey=1"]).arg(&path), 30)?;
    let duration: f64 = String::from_utf8_lossy(&bytes).trim().parse().map_err(|_| "No se pudo leer la duración del audio.")?;
    if !duration.is_finite() || duration <= 0.0 || duration > 14400.0 { return Err("Selecciona un audio con duración válida de hasta 4 horas.".into()); }
    let pcm = run(command("ffmpeg").args(["-v", "error", "-nostdin", "-i"]).arg(&path)
        .args(["-t", "14400", "-vn", "-ac", "1", "-ar", "8000", "-f", "s16le", "pipe:1"]), 240)?;
    if pcm.is_empty() { return Err("El archivo no contiene una pista de audio decodificable.".into()); }
    let waveform = Arc::new(Waveform::from_pcm(&pcm));
    Ok(Audio { path, duration, waveform, video: None })
}

pub fn playlist(path: &Path) -> Result<Vec<PathBuf>> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "La lista debe estar en UTF-8. Guárdala como M3U8.")?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut tracks = Vec::new();
    for line in text.trim_start_matches('\u{feff}').lines().map(str::trim) {
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.contains("://") { return Err("Esta versión admite M3U con pistas locales. Para YouTube, utiliza el campo de enlace.".into()); }
        let track = PathBuf::from(line);
        tracks.push(if track.is_absolute() { track } else { base.join(track) });
    }
    if tracks.is_empty() { return Err("La lista M3U no contiene pistas.".into()); }
    Ok(tracks)
}

pub fn selection(start: f64, length: f64, total: f64) -> (f64, f64) {
    let length = length.min(total).max(0.0);
    (start.clamp(0.0, (total - length).max(0.0)), length)
}

pub struct Playback {
    child: Option<Child>,
    decoder: Option<Child>,
    mixer: Option<Child>,
    audio_error: Arc<Mutex<String>>,
    clock: Arc<Mutex<Option<(f64, Instant)>>>,
    frame: Arc<Mutex<Option<crate::video::Frame>>>,
    stopped: Arc<AtomicBool>,
    video_done: Option<Arc<AtomicBool>>,
    start: f64,
    end: f64,
}
impl Playback {
    pub fn take_frame(&self) -> Option<crate::video::Frame> { self.frame.lock().ok()?.take() }
    pub fn finished(&mut self) -> Result<bool> {
        let audio_done = if let Some(child) = &mut self.child {
            match child.try_wait().map_err(|e| e.to_string())? {
                Some(status) if !status.success() => return Err(self.audio_error.lock().ok().filter(|s| !s.is_empty()).map(|s| format!("No se pudo reproducir el audio: {s}")).unwrap_or_else(|| "Falló la reproducción de audio. Revisa el archivo y el dispositivo de sonido.".into())),
                Some(_) => true, None => false,
            }
        } else { true };
        if let Some(decoder) = &mut self.decoder {
            if let Some(status) = decoder.try_wait().map_err(|e| e.to_string())? {
                if !status.success() { return Err("No se pudo decodificar el video para su vista previa.".into()); }
            }
        }
        if let Some(mixer) = &mut self.mixer {
            if let Some(status) = mixer.try_wait().map_err(|e| e.to_string())? { if !status.success() { return Err("No se pudo preparar la mezcla de audio. Revisa el archivo añadido.".into()); } }
        }
        Ok(self.video_done.as_ref().map_or(audio_done, |done| done.load(Ordering::Relaxed) && audio_done))
    }
    pub fn position(&self) -> Option<f64> {
        let clock = self.clock.lock().ok()?;
        let (position, received) = (*clock)?;
        // Never advance far beyond the last audio clock update if playback stalls.
        Some((position + received.elapsed().as_secs_f64().min(if self.video_done.is_some() { 1.0 } else { 0.1 })).clamp(self.start, self.end))
    }
}
impl Drop for Playback {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        for child in [&mut self.child, &mut self.decoder, &mut self.mixer].into_iter().flatten() { let _ = child.kill(); let _ = child.wait(); }
    }
}

fn audio_clock(line: &[u8]) -> Option<f64> {
    let line = std::str::from_utf8(line).ok()?;
    if !line.contains("M-A:") && !line.contains("A-V:") && !line.contains("M-V:") { return None; }
    line.split_whitespace().next()?.parse::<f64>().ok().filter(|v| v.is_finite())
}

pub fn preview(audio: &Audio, start: f64, length: f64) -> Result<Playback> {
    spawn_preview(audio, start, length, 100)
}
pub fn preview_gain(audio: &Audio, start: f64, length: f64, gain: f32) -> Result<Playback> {
    if gain.abs() < 0.001 { return preview(audio, start, length); }
    let options = crate::video::Options { mode: crate::video::SoundMode::Original, master_db: gain, ..Default::default() };
    spawn_preview_with_options(audio, start, length, 100, Some(&options))
}
pub fn preview_video(audio: &Audio, start: f64, length: f64, options: &crate::video::Options) -> Result<Playback> {
    spawn_preview_with_options(audio, start, length, 100, Some(options))
}

fn spawn_preview(audio: &Audio, start: f64, length: f64, volume: u8) -> Result<Playback> {
    spawn_preview_with_options(audio, start, length, volume, None)
}
fn spawn_preview_with_options(audio: &Audio, start: f64, length: f64, volume: u8, options: Option<&crate::video::Options>) -> Result<Playback> {
    let clock = Arc::new(Mutex::new(None));
    let mut player = Playback { child: None, decoder: None, mixer: None, audio_error: Arc::new(Mutex::new(String::new())), clock: clock.clone(), frame: Arc::new(Mutex::new(None)), stopped: Arc::new(AtomicBool::new(false)), video_done: None, start, end: start + length };
    let audio_clock_state = if audio.video.is_some() { Arc::new(Mutex::new(None)) } else { clock };
    let graph = if let Some(options) = options {
        let fallback = crate::video::Info { width: 0, height: 0, fps: 0.0, has_audio: true, alpha: false };
        crate::video::audio_graph(audio.video.as_ref().unwrap_or(&fallback), options, length)?
    } else { None };
    let has_audio = if options.is_some() { graph.is_some() } else { audio.video.as_ref().is_none_or(|v| v.has_audio) };
    let audio_alive = Arc::new(AtomicBool::new(has_audio));
    if has_audio {
    let mut cmd = command("ffplay");
    // Audio has no native window; video pixels are decoded into egui textures below.
    cmd.args(["-hide_banner", "-loglevel", "info", "-stats", "-sync", "audio", "-nodisp", "-vn", "-autoexit", "-volume", &volume.to_string()]);
    let mut clock_offset = 0.0;
    if let (Some(graph), Some(options)) = (&graph, options) {
        let mut mixer_cmd = command("ffmpeg");
        mixer_cmd.args(["-v", "error", "-nostdin", "-ss", &start.to_string(), "-i"]).arg(&audio.path);
        crate::video::add_external_input(&mut mixer_cmd, options);
        let mut mixer = mixer_cmd.args(["-filter_complex", graph, "-map", "[mixed]", "-t", &length.to_string(), "-ar", "48000", "-ac", "2", "-f", "f32le", "pipe:1"]).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| e.to_string())?;
        let pipe = mixer.stdout.take().ok_or("No se pudo iniciar el audio de la mezcla.")?;
        player.mixer = Some(mixer);
        cmd.args(["-f", "f32le", "-ar", "48000", "-ch_layout", "stereo", "-i", "pipe:0"]).stdin(Stdio::from(pipe));
        clock_offset = start;
    } else { cmd.args(["-ss", &start.to_string(), "-t", &length.to_string()]).arg(&audio.path); }
    let mut child = cmd.stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| format!("No se pudo reproducir el archivo: {e}. Comprueba ffplay."))?;
    let stderr = child.stderr.take().ok_or("No se pudo leer el reloj de reproducción.")?;
    player.child = Some(child);
    let updates = audio_clock_state.clone(); let alive = audio_alive.clone();
    let errors = player.audio_error.clone();
    thread::spawn(move || {
        use std::io::{BufReader, Read};
        let mut line = Vec::new();
        for byte in BufReader::new(stderr).bytes() {
            let Ok(byte) = byte else { break; };
            if byte == b'\r' || byte == b'\n' {
                if let Some(position) = audio_clock(&line) {
                    if let Ok(mut clock) = updates.lock() { *clock = Some((position + clock_offset, Instant::now())); }
                } else {
                    let text = String::from_utf8_lossy(&line); let lower = text.to_lowercase();
                    if ["error", "invalid", "failed", "not found"].iter().any(|word| lower.contains(word)) {
                        if let Ok(mut error) = errors.lock() { *error = text.chars().take(500).collect(); }
                    }
                }
                line.clear();
            } else if line.len() < 16384 { line.push(byte); }
        }
        alive.store(false, Ordering::Relaxed);
    });
    }
    if let Some(info) = &audio.video {
        let (width, height) = crate::video::preview_size(info);
        let fps = info.fps.clamp(1.0, 30.0);
        let filter = format!("fps={fps:.6},scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2");
        let mut decoder = command("ffmpeg").args(["-v", "error", "-nostdin", "-ss", &start.to_string(), "-i"]).arg(&audio.path)
            .args(["-t", &length.to_string(), "-map", "0:v:0", "-an", "-vf", &filter, "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1"])
            .stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("No se pudo iniciar la vista previa integrada: {e}"))?;
        let stdout = decoder.stdout.take().ok_or("No se pudo leer el video.")?;
        player.decoder = Some(decoder);
        let done = Arc::new(AtomicBool::new(false)); player.video_done = Some(done.clone());
        let stop = player.stopped.clone(); let output = player.frame.clone(); let shown_clock = player.clock.clone();
        thread::spawn(move || {
            use std::io::{BufReader, Read};
            struct Done(Arc<AtomicBool>);
            impl Drop for Done { fn drop(&mut self) { self.0.store(true, Ordering::Relaxed); } }
            let _done = Done(done);
            let mut reader = BufReader::new(stdout); let mut index = 0u64;
            let mut anchor = (start, Instant::now()); let began = Instant::now();
            loop {
                let mut rgb = vec![0; width * height * 3];
                if reader.read_exact(&mut rgb).is_err() || stop.load(Ordering::Relaxed) { break; }
                if index == 0 { anchor = (start, Instant::now()); }
                let at = start + index as f64 / fps;
                if at >= start + length { break; }
                loop {
                    if stop.load(Ordering::Relaxed) { return; }
                    if let Ok(clock) = audio_clock_state.lock() { if let Some(sample) = *clock { anchor = sample; } }
                    let ready = !has_audio || !audio_alive.load(Ordering::Relaxed) || audio_clock_state.lock().ok().is_some_and(|c| c.is_some()) || began.elapsed().as_secs() >= 5;
                    if ready && anchor.0 + anchor.1.elapsed().as_secs_f64() >= at { break; }
                    thread::sleep(Duration::from_millis(2));
                }
                if let Ok(mut frame) = output.lock() { *frame = Some(crate::video::Frame { width, height, rgb }); }
                if let Ok(mut clock) = shown_clock.lock() { *clock = Some((at, Instant::now())); }
                index += 1;
            }
            // Keep the last frame on screen until the requested interval actually ends.
            while index > 0 && !stop.load(Ordering::Relaxed) && anchor.0 + anchor.1.elapsed().as_secs_f64() < start + length { thread::sleep(Duration::from_millis(4)); }
        });
    }
    Ok(player)
}

#[cfg(test)]
pub fn export(audio: &Audio, start: f64, length: f64, output: &Path) -> Result<()> {
    export_gain(audio, start, length, output, 0.0)
}
pub fn export_gain(audio: &Audio, start: f64, length: f64, output: &Path, gain: f32) -> Result<()> {
    if !gain.is_finite() || !(-30.0..=18.0).contains(&gain) { return Err("El ajuste final debe estar entre -30 y +18 dB.".into()); }
    if !start.is_finite() || !length.is_finite() || length <= 0.0 || start < 0.0 || start + length > audio.duration + 0.001 { return Err("El intervalo de corte no es válido.".into()); }
    if output.exists() && fs::canonicalize(output).ok() == fs::canonicalize(&audio.path).ok() { return Err("Elige otro nombre para conservar el audio original.".into()); }
    let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let (format, codec) = match ext.as_str() { "wav" => ("wav", "pcm_s16le"), "mp3" => ("mp3", "libmp3lame"), _ => return Err("El archivo de salida debe terminar en .wav o .mp3.".into()) };
    let temp = tempfile::NamedTempFile::new_in(output.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    let mut cmd = command("ffmpeg");
    cmd.args(["-v", "error", "-nostdin", "-y", "-ss", &format!("{start:.6}"), "-i"]).arg(&audio.path)
        .args(["-t", &format!("{length:.6}"), "-map", "0:a:0", "-vn", "-c:a", codec, "-ar", "44100", "-ac", "2"]);
    if gain.abs() >= 0.001 { cmd.args(["-af", &format!("volume={gain:.3}dB,alimiter=limit=0.95:level=0:latency=1")]); }
    run(cmd.args(["-f", format]).arg(temp.path()), 120)?;
    temp.persist(output).map_err(|e| format!("No se pudo guardar el corte: {e}"))?;
    Ok(())
}

pub fn youtube(url: &str, folder: &Path) -> Result<PathBuf> {
    youtube_media(url, folder, false)
}

pub fn youtube_media(url: &str, folder: &Path, video: bool) -> Result<PathBuf> {
    let rest = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")).ok_or("Pega un enlace HTTP o HTTPS de YouTube.")?;
    let host = rest.split('/').next().unwrap_or("").to_ascii_lowercase();
    if !["youtube.com", "www.youtube.com", "m.youtube.com", "music.youtube.com", "youtu.be"].contains(&host.as_str()) { return Err("El enlace debe pertenecer a YouTube.".into()); }
    fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let mut cmd = command("yt-dlp");
    cmd.args(["--ignore-config", "--no-playlist", "--no-progress", "--socket-timeout", "25", "--retries", "2", "--max-filesize", if video { "2G" } else { "500M" }, "--match-filter", "!is_live & duration <= 14400", "--windows-filenames", "--print", "after_move:filepath", "--no-simulate"]);
    if video { cmd.args(["-f", "bv*+ba/b", "--merge-output-format", "mkv"]); }
    else { cmd.args(["-f", "bestaudio/best", "-x", "--audio-format", "mp3", "--audio-quality", "0"]); }
    let ffmpeg = tool("ffmpeg");
    if ffmpeg.is_absolute() { cmd.arg("--ffmpeg-location").arg(ffmpeg.parent().unwrap()); }
    let deno = tool("deno");
    if deno.is_absolute() { cmd.arg("--js-runtimes").arg(format!("deno:{}", deno.display())); }
    let bytes = run(cmd.arg("-o").arg(folder.join("%(title).120s [%(id)s].%(ext)s")).arg("--").arg(url), 900)?;
    let text = String::from_utf8_lossy(&bytes);
    let path = text.lines().rev().find(|line| !line.trim().is_empty()).ok_or("La descarga no produjo un archivo. Comprueba el enlace y actualiza yt-dlp.")?;
    let path = PathBuf::from(path.trim());
    if !path.is_file() { return Err("No se encontró el audio descargado.".into()); }
    Ok(path)
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryEntry { pub source: PathBuf, pub output: PathBuf, pub start: f64, pub length: f64, pub date: String, #[serde(default)] pub video: bool }

pub fn save_history(path: &Path, entries: &[HistoryEntry]) -> Result<()> {
    use std::io::Write;
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().unwrap()).map_err(|e| e.to_string())?;
    temp.write_all(&serde_json::to_vec_pretty(entries).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    temp.persist(path).map_err(|e| e.to_string())?; Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn playback_reports_real_absolute_audio_clock_after_seek() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("clock.wav");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo", "-t", "5"]).arg(&path), 20).unwrap();
        let audio = load(path).unwrap();
        let mut player = spawn_preview(&audio, 2.125, 1.0, 0).unwrap();
        let begin = Instant::now(); let mut observed = None;
        while begin.elapsed().as_secs_f64() < 10.0 {
            if let Some(position) = player.position() { if position > 2.225 { observed = Some(position); break; } }
            if player.finished().unwrap() { break; }
            thread::sleep(Duration::from_millis(20));
        }
        drop(player);
        let position = observed.expect("ffplay did not report a usable audio clock");
        assert!((2.225..=3.125).contains(&position), "wrong seek clock: {position}");
    }
    #[test] fn embedded_preview_streams_changing_frames_with_and_without_audio() {
        let dir = tempfile::tempdir().unwrap();
        let video_path = dir.path().join("con audio.mp4");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=160x90:rate=10:duration=2", "-f", "lavfi", "-i", "anullsrc=r=44100:cl=stereo", "-t", "2", "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac"]).arg(&video_path), 20).unwrap();
        let gif_path = dir.path().join("sin audio.gif");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=160x90:rate=10:duration=2"]).arg(&gif_path), 20).unwrap();
        for path in [video_path, gif_path] {
            let audio = crate::video::load(path).unwrap();
            let began = Instant::now(); let mut player = spawn_preview(&audio, 0.25, 1.0, 0).unwrap();
            let mut frames = Vec::new(); let mut completed = false;
            while began.elapsed().as_secs_f64() < 10.0 {
                if let Some(frame) = player.take_frame() {
                    assert_eq!((frame.width, frame.height), (160, 90));
                    frames.push(frame.rgb.iter().map(|v| *v as u64).sum::<u64>());
                }
                if player.finished().unwrap() { completed = true; break; }
                thread::sleep(Duration::from_millis(10));
            }
            assert!(completed, "embedded playback did not finish");
            assert!(began.elapsed().as_secs_f64() >= 0.9, "playback finished before frames were presented");
            assert!(frames.len() >= 3, "not enough embedded frames: {}", frames.len());
            assert!(frames.windows(2).any(|f| f[0] != f[1]), "video frame did not change");
            assert!((0.25..=1.25).contains(&player.position().unwrap()));
        }
    }
    #[test] fn embedded_preview_plays_external_audio_using_absolute_video_clock() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("video.gif");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "testsrc2=size=96x64:rate=10:duration=3"]).arg(&path), 20).unwrap();
        let audio_path = dir.path().join("audio.wav");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=880:duration=1"]).arg(&audio_path), 20).unwrap();
        let audio = crate::video::load(path).unwrap();
        let options = crate::video::Options { mode: crate::video::SoundMode::Replace, external: Some(crate::video::external_audio(audio_path).unwrap()), ..Default::default() };
        let mut player = spawn_preview_with_options(&audio, 1.0, 1.0, 0, Some(&options)).unwrap();
        let began = Instant::now(); let mut count = 0;
        while began.elapsed().as_secs_f64() < 10.0 {
            if player.take_frame().is_some() { count += 1; }
            if player.finished().unwrap() { break; }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(player.finished().unwrap(), "mixed preview did not finish");
        assert!(count >= 3, "mixed preview did not present video frames");
        assert!(player.position().unwrap() > 1.8, "mixed audio clock did not keep video offset");
    }
    #[test] fn zoom_resolves_a_millisecond_transient() {
        let mut pcm = vec![0u8; 8000 * 2 * 2];
        for sample in 9872..9880 { pcm[sample * 2..sample * 2 + 2].copy_from_slice(&16384_i16.to_le_bytes()); }
        let waveform = Waveform::from_pcm(&pcm);
        assert_eq!(waveform.peak(1.234, 1.235), 0.5);
        assert_eq!(waveform.peak(1.0, 1.1), 0.0);
        assert_eq!(waveform.peak(0.0, 2.0), 0.5);
    }
    #[test] fn range_keeps_exact_preset_at_end() { assert_eq!(selection(99.0, 11.0, 100.0), (89.0, 11.0)); }
    #[test] fn short_track_is_not_padded() { assert_eq!(selection(4.0, 9.0, 5.0), (0.0, 5.0)); }
    #[test] fn m3u_resolves_relative_paths_and_bom() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("lista.m3u8");
        fs::write(&path, "\u{feff}#EXTM3U\n#EXTINF:15,Pista\naudio con espacios.wav\n\n").unwrap();
        assert_eq!(playlist(&path).unwrap(), vec![dir.path().join("audio con espacios.wav")]);
    }
    #[test] fn rejects_remote_playlist_and_other_hosts() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("lista.m3u");
        fs::write(&path, "https://example.com/live").unwrap(); assert!(playlist(&path).is_err());
        assert!(youtube("https://youtube.com.evil.test/video", dir.path()).is_err());
    }
    #[test] fn history_roundtrip_and_replacement() {
        let dir = tempfile::tempdir().unwrap(); let path = dir.path().join("history.json");
        save_history(&path, &[]).unwrap();
        let entries = vec![HistoryEntry { source: "a.wav".into(), output: "b.mp3".into(), start: 1.25, length: 9.0, date: "hoy".into(), video: false }];
        save_history(&path, &entries).unwrap();
        let read: Vec<HistoryEntry> = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(read[0].start, 1.25);
    }
    #[test] fn actual_audio_exports_precise_wav_and_mp3() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("original con espacios.wav");
        run(command("ffmpeg").args(["-v", "error", "-f", "lavfi", "-i", "sine=frequency=880:duration=25", "-c:a", "pcm_s16le"]).arg(&source), 20).unwrap();
        let audio = load(source.clone()).unwrap();
        assert!((audio.duration - 25.0).abs() < 0.001);
        assert!(audio.waveform.peak(1.0, 1.1) > 0.0);
        for length in [9.0, 11.0, 13.125, 0.005] {
            let output = dir.path().join(format!("alerta {length}.wav"));
            export(&audio, 1.125, length, &output).unwrap();
            let pcm = run(command("ffmpeg").args(["-v", "error", "-i"]).arg(&output).args(["-f", "s16le", "-ac", "2", "-ar", "44100", "pipe:1"]), 20).unwrap();
            assert!((pcm.len() as f64 / (44100.0 * 4.0) - length).abs() <= 1.0 / 44100.0);
            export(&audio, 2.5, length, &output).unwrap(); // replacement remains usable
            assert!((load(output).unwrap().duration - length).abs() < 0.001);
        }
        let mp3 = dir.path().join("alerta.mp3");
        export(&audio, 3.75, 9.0, &mp3).unwrap();
        assert!((load(mp3).unwrap().duration - 9.0).abs() < 0.1);
        assert!(export(&audio, 0.0, 9.0, &source).is_err());
        assert!(export(&audio, 24.0, 9.0, &dir.path().join("invalido.wav")).is_err());
    }
}
