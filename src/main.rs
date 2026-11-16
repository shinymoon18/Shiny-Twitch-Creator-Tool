#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
macro_rules! tr { ($text:literal) => { crate::i18n::text($text) }; }
macro_rules! tf { ($text:literal $(, $($args:tt)*)?) => { crate::i18n::formatted($text, format!($text $(, $($args)*)?)) }; }
mod i18n;
mod fades;
mod media;
mod editor;
mod ui;
mod video;
mod theme;
mod sound;
mod portable;
mod audio_trim;
use editor::{Target, View};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use media::{Audio, HistoryEntry};
use std::{path::PathBuf, sync::mpsc::{self, Receiver}, time::Duration};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

enum Event { Loaded(media::Result<Audio>), Exported(media::Result<HistoryEntry>), Measured(media::Result<(String, sound::Measurement)>) }
#[derive(Clone, Copy)]
enum WaveDrag { Start, End, Move { pointer: f64, start: f64 } }
struct Workspace {
    audio: Option<Audio>, start: f64, length: f64, preset: Option<f64>, url: String,
    view: View, step: f64, target: Target, tracks: Vec<PathBuf>, track_index: usize,
}
impl Default for Workspace { fn default() -> Self { Self { audio: None, start: 0.0, length: 9.0, preset: Some(9.0), url: String::new(), view: View::full(1.0), step: 0.001, target: Target::Selection, tracks: vec![], track_index: 0 } } }
struct App {
    audio: Option<Audio>, start: f64, length: f64, preset: Option<f64>, url: String,
    view: View, step: f64, target: Target, drag: Option<WaveDrag>,
    tracks: Vec<PathBuf>, track_index: usize, receiver: Option<Receiver<Event>>,
    status: String, error: bool, history: Vec<HistoryEntry>, data: PathBuf,
    player: Option<media::Playback>, playhead: f64, format_mp3: bool,
    last_folder: Option<PathBuf>, last_output: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
    video_mode: bool, other: Workspace,
    appearance: theme::Appearance, show_themes: bool,
    video_options: video::Options, show_video_export: bool, convert_full: bool,
    frame_receiver: Option<Receiver<(PathBuf, f64, media::Result<video::Frame>)>>,
    frame_texture: Option<egui::TextureHandle>, frame_at: Option<f64>, frame_error: String,
    show_sound: bool, audio_gain_db: f32, target_lufs: f64, level_report: Option<(String, sound::Measurement)>,
    audio_editor: Option<audio_trim::Editor>,
    settings: i18n::Settings, show_options: bool,
    audio_fades: fades::Fades,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, data: PathBuf) -> Self {
        let settings: i18n::Settings = std::fs::read(data.join("settings.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        settings.apply();
        let appearance: theme::Appearance = std::fs::read(data.join("apariencia.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        ui::configure(&cc.egui_ctx, &appearance);
        let history_path = data.join("historial.json");
        let (history, status, error) = match std::fs::read(&history_path) {
            Ok(bytes) => match serde_json::from_slice(&bytes) {
                Ok(entries) => (entries, tr!("Open a track or paste a YouTube link to get started.").into(), false),
                Err(e) => (Vec::new(), tf!("Could not read the history. It will be preserved until you save a new clip: {e}"), true),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Vec::new(), tr!("Open a track or paste a YouTube link to get started.").into(), false),
            Err(e) => (Vec::new(), tf!("Could not open the history: {e}"), true),
        };
        Self { audio: None, start: 0.0, length: 9.0, preset: Some(9.0), url: String::new(), tracks: vec![], track_index: 0,
            view: View::full(1.0), step: 0.001, target: Target::Selection, drag: None,
            receiver: None, status, error, history, data, player: None, playhead: 0.0, format_mp3: false,
            last_folder: None, last_output: None, cancel: Arc::new(AtomicBool::new(false)),
            video_mode: false, other: Workspace::default(), appearance, show_themes: false,
            video_options: video::Options::default(), show_video_export: false, convert_full: false,
            frame_receiver: None, frame_texture: None, frame_at: None, frame_error: String::new(),
            show_sound: false, audio_gain_db: 0.0, target_lufs: -18.0, level_report: None, audio_editor: None, settings, show_options: false, audio_fades: fades::Fades::default() }
    }
    fn message(&mut self, result: media::Result<()>) {
        if let Err(e) = result { self.status = e; self.error = true; }
    }
    fn stop(&mut self) {
        if let Some(player) = self.player.take() { if let Some(position) = player.position() { self.playhead = position; } }
    }
    fn measurement_key(&self) -> String {
        serde_json::json!({ "path": self.audio.as_ref().map(|a| &a.path), "start": self.start, "length": self.length, "video": self.video_mode, "options": if self.video_mode { serde_json::to_value(&self.video_options).unwrap_or_default() } else { serde_json::json!({"gain": self.audio_gain_db, "fades": self.audio_fades.bounded(self.length)}) } }).to_string()
    }
    fn analyze_sound(&mut self, ctx: &egui::Context) {
        let Some(audio) = self.audio.clone() else { return; };
        self.stop();
        let key = self.measurement_key(); let start = self.start; let length = self.length;
        let options = if self.video_mode { Some(self.video_options.clone()) } else { None };
        let gain = self.audio_gain_db; let fades = self.audio_fades; let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx);
        self.cancel = Arc::new(AtomicBool::new(false)); let cancel = self.cancel.clone();
        self.status = tr!("Measuring the selected clip's loudness…").into(); self.error = false;
        std::thread::spawn(move || { media::set_cancellation(cancel); let result = sound::measure_fades(&audio, start, length, options.as_ref(), gain, fades).map(|report| (key, report)); let _ = tx.send(Event::Measured(result)); ctx.request_repaint(); });
    }
    fn load(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.stop(); self.audio = None;
        self.frame_texture = None; self.frame_at = None; self.frame_error.clear();
        self.status = tr!("Loading and analyzing the file…").into(); self.error = false;
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone();
        self.cancel = Arc::new(AtomicBool::new(false)); let cancel = self.cancel.clone();
        let video_mode = self.video_mode;
        std::thread::spawn(move || { media::set_cancellation(cancel); let result = if video_mode { video::load(path) } else { media::load(path) }; let _ = tx.send(Event::Loaded(result)); ctx.request_repaint(); });
    }
    fn switch_mode(&mut self, video_mode: bool) {
        if video_mode == self.video_mode || self.receiver.is_some() { return; }
        self.stop();
        macro_rules! swap { ($($field:ident),*) => { $(std::mem::swap(&mut self.$field, &mut self.other.$field);)* }; }
        swap!(audio, start, length, preset, url, view, step, target, tracks, track_index);
        self.video_mode = video_mode; self.playhead = self.start; self.drag = None;
        self.frame_texture = None; self.frame_at = None; self.frame_error.clear(); self.show_video_export = false;
        self.status = if video_mode { tr!("Video: open a local file or download from YouTube.") } else { tr!("Audio: choose a track and prepare your clip.") }.into(); self.error = false;
    }
    fn download(&mut self, ctx: &egui::Context) {
        self.stop(); self.audio = None; self.frame_texture = None; self.frame_at = None;
        let video_mode = self.video_mode;
        let url = self.url.trim().to_owned(); let folder = self.data.join(if video_mode { "videos" } else { "descargas" });
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone();
        self.status = if video_mode { tr!("Downloading YouTube video…") } else { tr!("Downloading YouTube audio…") }.into(); self.error = false;
        self.cancel = Arc::new(AtomicBool::new(false)); let cancel = self.cancel.clone();
        std::thread::spawn(move || { media::set_cancellation(cancel);
            let result = if video_mode { media::youtube_media(&url, &folder, true).and_then(video::load) } else { media::youtube(&url, &folder).and_then(media::load) };
            let _ = tx.send(Event::Loaded(result)); ctx.request_repaint(); });
    }
    fn poll_frame(&mut self, ctx: &egui::Context) {
        if let Some(frame) = self.player.as_ref().and_then(|p| p.take_frame()) {
            let image = egui::ColorImage::from_rgb([frame.width, frame.height], &frame.rgb);
            if let Some(texture) = &mut self.frame_texture { texture.set(image, egui::TextureOptions::LINEAR); }
            else { self.frame_texture = Some(ctx.load_texture("video-frame", image, egui::TextureOptions::LINEAR)); }
        }
        if let Some((path, at, result)) = self.frame_receiver.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.frame_receiver = None;
            if self.video_mode && self.player.is_none() && self.audio.as_ref().is_some_and(|a| a.path == path) {
                match result {
                    Ok(frame) => { self.frame_texture = Some(ctx.load_texture("video-frame", egui::ColorImage::from_rgb([frame.width, frame.height], &frame.rgb), egui::TextureOptions::LINEAR)); self.frame_error.clear(); },
                    Err(e) => { self.frame_error = e; },
                }
                self.frame_at = Some(at);
            }
        }
        if self.video_mode && self.frame_receiver.is_none() && self.receiver.is_none() && self.player.is_none() {
            if let Some(audio) = &self.audio {
                if self.frame_at.is_none_or(|at| (at - self.start).abs() >= 0.001) {
                    let audio = audio.clone(); let at = self.start; let ctx = ctx.clone();
                    let (tx, rx) = mpsc::channel(); self.frame_receiver = Some(rx);
                    std::thread::spawn(move || { let result = video::frame(&audio, at); let _ = tx.send((audio.path, at, result)); ctx.request_repaint(); });
                }
            }
        }
    }
    fn normalize(&mut self) {
        if let Some(audio) = &self.audio { (self.start, self.length) = media::selection(self.start, self.length.max(editor::MIN_LENGTH.min(audio.duration)), audio.duration); }
    }
    fn apply_preset(&mut self, seconds: f64) {
        self.stop();
        self.preset = Some(seconds);
        self.length = seconds;
        self.normalize();
    }
    fn nudge(&mut self, delta: f64) {
        self.stop();
        if let Some(audio) = &self.audio {
            (self.start, self.length) = editor::nudge(self.start, self.length, delta, audio.duration, self.target);
            if self.target != Target::Selection { self.preset = None; }
            self.view.follow(if self.target == Target::End { self.start + self.length } else { self.start }, audio.duration);
        }
    }
    fn change_duration(&mut self, delta: f64) {
        self.stop();
        if let Some(audio) = &self.audio {
            let previous = self.length;
            (self.start, self.length) = editor::nudge(self.start, self.length, delta, audio.duration, Target::End);
            self.preset = None;
            self.view.follow(self.start + self.length, audio.duration);
            let limited = ((self.length - previous) - delta).abs() > 1e-9;
            self.status = tf!("Clip duration: {:.3} s. Start fixed at {}.{}", self.length, time(self.start),
                if limited { tr!(" Reached the end of the track or the minimum duration.") } else { "" });
            self.error = false;
        }
    }
    fn play(&mut self, whole: bool) {
        self.stop();
        if let Some(audio) = &self.audio {
            let length = if whole { audio.duration - self.start } else { self.length };
            let playback = if self.video_mode { media::preview_video(audio, self.start, length, &self.video_options) } else { media::preview_effects(audio, self.start, length, self.audio_gain_db, self.audio_fades.bounded(self.length)) };
            match playback {
                Ok(player) => { self.playhead = self.start; self.player = Some(player); }
                Err(e) => self.message(Err(e)),
            }
        }
    }
    fn mark_playback(&mut self, end: bool) {
        let Some(at) = self.player.as_ref().and_then(|p| p.position()) else { return; };
        let Some(audio) = &self.audio else { return; };
        let range = if end { editor::mark_end(self.start, at, audio.duration) } else { Some(editor::mark_start(at, self.length, audio.duration)) };
        let Some((start, length)) = range else { self.message(Err(tr!("The end must be after the start. Wait a moment or mark a new start.").into())); return; };
        self.start = start; self.length = length; self.preset = None;
        self.view.follow(at, audio.duration);
        self.status = tf!("{} marked at {}. Playback continues.", if end { tr!("End") } else { tr!("Start") }, time(if end { start + length } else { start }));
        self.error = false;
    }
    fn poll(&mut self) {
        if let Some(event) = self.receiver.as_ref().and_then(|rx| rx.try_recv().ok()) {
            self.receiver = None;
            match event {
                Event::Loaded(Ok(audio)) => { self.view = View::full(audio.duration); self.drag = None; self.audio = Some(audio); self.start = 0.0; self.playhead = 0.0; self.length = self.preset.unwrap_or(self.length); self.normalize(); self.status = tr!("File ready. Play and mark your start and end points.").into(); self.error = false; }
                Event::Loaded(Err(e)) | Event::Exported(Err(e)) | Event::Measured(Err(e)) => self.message(Err(e)),
                Event::Measured(Ok(report)) => { self.level_report = Some(report); self.status = tr!("Measurement complete. Adjust the volume and preview your alert.").into(); self.error = false; },
                Event::Exported(Ok(entry)) => {
                    self.last_output = Some(entry.output.clone());
                    self.last_folder = entry.output.parent().map(|p| p.to_path_buf());
                    self.status = tf!("Clip saved to {}", entry.output.display()); self.error = false;
                    self.history.insert(0, entry);
                    if let Err(e) = media::save_history(&self.data.join("historial.json"), &self.history) { self.message(Err(tf!("The clip was saved, but its history entry could not be saved: {e}"))); }
                }
            }
        }
        if let Some(player) = self.player.as_mut() {
            if let Some(position) = player.position() { self.playhead = position; }
            match player.finished() {
                Ok(true) => { self.player = None; }
                Err(e) => { self.stop(); self.message(Err(tf!("Playback error: {e}"))); }
                _ => (),
            }
        }
    }
    fn save(&mut self, ctx: &egui::Context) {
        let Some(audio) = self.audio.clone() else { return; };
        self.stop();
        let video_mode = self.video_mode;
        let ext = if video_mode { "webm" } else if self.format_mp3 { "mp3" } else { "wav" };
        let (start, length) = if video_mode && self.convert_full { (0.0, audio.duration) } else { (self.start, self.length) };
        let stem = audio.path.file_stem().unwrap_or_default().to_string_lossy();
        let mut dialog = rfd::FileDialog::new().set_title(tr!("Save Twitch alert")).add_filter(ext.to_uppercase(), &[ext])
            .set_file_name(tf!("{stem}_clip_{length:.3}s_{}.{}", time(start).replace(':', "-"), ext));
        if let Some(folder) = &self.last_folder { dialog = dialog.set_directory(folder); }
        let Some(mut output) = dialog.save_file() else { return; };
        if output.extension().is_none() { output.set_extension(ext); }
        if !output.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
            self.message(Err(tf!("The filename must end in .{ext}."))); return;
        }
        let (tx, rx) = mpsc::channel(); self.receiver = Some(rx); let ctx = ctx.clone();
        self.status = if video_mode { tr!("Encoding WebM VP9… this may take a while. You can cancel the operation.") } else { tr!("Exporting alert…") }.into(); self.error = false;
        let video_options = self.video_options.clone();
        let audio_gain = self.audio_gain_db;
        let audio_fades = self.audio_fades;
        self.cancel = Arc::new(AtomicBool::new(false)); let cancel = self.cancel.clone();
        std::thread::spawn(move || {
            media::set_cancellation(cancel);
            let result = if video_mode { video::export(&audio, start, length, &output, &video_options) } else { media::export_effects(&audio, start, length, &output, audio_gain, audio_fades) };
            let result = result.map(|_| HistoryEntry {
                source: audio.path, output, start, length, date: chrono::Local::now().format("%d/%m/%Y %H:%M").to_string(), video: video_mode,
            });
            let _ = tx.send(Event::Exported(result)); ctx.request_repaint();
        });
    }
    fn waveform(&mut self, ui: &mut egui::Ui) {
        let Some(audio) = &self.audio else { return; };
        let duration = audio.duration;
        self.view.clamp(duration);
        ui.horizontal_wrapped(|ui| {
            ui.label("Zoom:");
            let anchor = (self.start + self.length / 2.0).clamp(self.view.start, self.view.start + self.view.span);
            if ui.button(tr!("Zoom in +")).clicked() { self.view.zoom(2.0, anchor, duration); }
            if ui.button(tr!("Zoom out -")).clicked() { self.view.zoom(0.5, anchor, duration); }
            if ui.button(tr!("Fit selection")).clicked() { self.view.fit(self.start, self.length, duration); }
            if ui.button(tr!("Fit entire track")).clicked() { self.view = View::full(duration); }
            ui.label(tf!("{:.1}x · {:.3} s visible", duration / self.view.span, self.view.span));
        });
        ui.horizontal(|ui| {
            if ui.small_button(tr!("Previous view")).clicked() { self.view.start -= self.view.span * 0.5; self.view.clamp(duration); }
            ui.add(egui::Slider::new(&mut self.view.start, 0.0..=(duration - self.view.span).max(0.0)).text(tr!("Scroll track")).show_value(false));
            if ui.small_button(tr!("Next view")).clicked() { self.view.start += self.view.span * 0.5; self.view.clamp(duration); }
        });
        let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 170.0), egui::Sense::click_and_drag());
        let colors = self.appearance.palette();
        let painter = ui.painter().with_clip_rect(rect); painter.rect_filled(rect, 8.0, colors.bg);
        let view = self.view;
        let x_at = |seconds: f64| rect.left() + rect.width() * ((seconds - view.start) / view.span) as f32;
        let time_at = |x: f32| (view.start + ((x - rect.left()) / rect.width()) as f64 * view.span).clamp(0.0, duration);
        let selected = egui::Rect::from_min_max(egui::pos2(x_at(self.start), rect.top()), egui::pos2(x_at(self.start + self.length), rect.bottom()));
        if selected.intersects(rect) { painter.rect_filled(selected.intersect(rect), 0.0, colors.accent.gamma_multiply(0.3)); }
        let columns = (rect.width() / 2.0).max(1.0) as usize;
        for col in 0..columns {
            let from = view.start + col as f64 / columns as f64 * view.span;
            let to = view.start + (col + 1) as f64 / columns as f64 * view.span;
            let envelope = if !self.video_mode && from >= self.start && from <= self.start + self.length { self.audio_fades.envelope((from + to) / 2.0 - self.start, self.length) } else { 1.0 };
            let height = audio.waveform.peak(from, to) * 65.0 * envelope;
            let x = rect.left() + col as f32 * rect.width() / columns as f32;
            let color = if x >= selected.left() && x <= selected.right() { colors.accent } else { colors.muted.gamma_multiply(0.6) };
            painter.line_segment([egui::pos2(x, rect.center().y - height.max(1.0)), egui::pos2(x, rect.center().y + height.max(1.0))], Stroke::new(1.0_f32, color));
        }
        for (seconds, label) in [(self.start, tr!("START")), (self.start + self.length, tr!("END"))] {
            let x = x_at(seconds);
            painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(3.0_f32, colors.accent));
            if rect.contains(egui::pos2(x, rect.center().y)) {
                painter.rect_filled(egui::Rect::from_center_size(egui::pos2(x, rect.center().y), Vec2::new(10.0, 28.0)), 3.0, colors.accent);
                painter.text(egui::pos2(x.clamp(rect.left() + 30.0, rect.right() - 30.0), rect.top() + 8.0), egui::Align2::CENTER_TOP, label, egui::FontId::proportional(11.0), Color32::WHITE);
            }
        }
        if self.player.is_some() { let x = x_at(self.playhead); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::from_rgb(81, 225, 197))); }
        let (fade_active, fade_changed) = if !self.video_mode { fades::overlay(ui, rect, view, self.start, self.length, &mut self.audio_fades) } else { (false, false) };
        if fade_changed { self.stop(); }
        if let Some(pos) = response.hover_pos() {
            if (pos.x - x_at(self.start)).abs() < 9.0 || (pos.x - x_at(self.start + self.length)).abs() < 9.0 { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
            else if pos.x >= selected.left() && pos.x <= selected.right() { ui.ctx().set_cursor_icon(egui::CursorIcon::Grab); }
        }
        if response.drag_started() && !fade_active {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                self.drag = Some(if (pos.x - x_at(self.start)).abs() < 9.0 { WaveDrag::Start }
                    else if (pos.x - x_at(self.start + self.length)).abs() < 9.0 { WaveDrag::End }
                    else { WaveDrag::Move { pointer: time_at(pos.x), start: if pos.x >= selected.left() && pos.x <= selected.right() { self.start } else { time_at(pos.x) } } });
            }
        }
        if response.dragged() && !fade_active {
            if let (Some(pos), Some(drag)) = (response.interact_pointer_pos(), self.drag) {
                self.stop();
                (self.start, self.length) = match drag {
                    WaveDrag::Start => { self.preset = None; editor::resize_start(self.start, self.length, time_at(pos.x), duration) },
                    WaveDrag::End => { self.preset = None; editor::resize_end(self.start, time_at(pos.x), duration) },
                    WaveDrag::Move { pointer, start } => media::selection(start + time_at(pos.x) - pointer, self.length, duration),
                };
            }
        }
        if response.drag_stopped() { self.drag = None; }
        if response.clicked() && !fade_active {
            if let Some(pos) = response.interact_pointer_pos() { self.stop(); self.start = time_at(pos.x); self.normalize(); }
        }
        if !self.video_mode && fades::controls(ui, &mut self.audio_fades, self.length) { self.stop(); }
        let (ruler, _) = ui.allocate_exact_size(Vec2::new(rect.width(), 18.0), egui::Sense::hover());
        for index in 0..=4 {
            let align = if index == 0 { egui::Align2::LEFT_TOP } else if index == 4 { egui::Align2::RIGHT_TOP } else { egui::Align2::CENTER_TOP };
            ui.painter().text(egui::pos2(ruler.left() + ruler.width() * index as f32 / 4.0, ruler.top()), align, time(view.start + view.span * index as f64 / 4.0), egui::FontId::proportional(11.0), Color32::GRAY);
        }
    }
}

impl Drop for App { fn drop(&mut self) { self.stop(); } }

fn time(seconds: f64) -> String {
    let ms = (seconds.max(0.0) * 1000.0).round() as u64;
    format!("{:02}:{:02}.{:03}", ms / 60000, ms / 1000 % 60, ms % 1000)
}
fn reveal(path: &std::path::Path) -> media::Result<()> {
    if !path.exists() { return Err(tr!("The file has been moved or deleted.").into()); }
    #[cfg(windows)] { media::command("explorer").arg(format!("/select,{}", path.display())).spawn().map_err(|e| e.to_string())?; }
    #[cfg(not(windows))] {
        let mut open = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" });
        // Host file managers must use their own libraries, not the AppImage's.
        if std::env::var_os("APPIMAGE").is_some() { open.env_remove("LD_LIBRARY_PATH"); }
        open.arg(path.parent().unwrap()).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn main() -> eframe::Result {
    i18n::set(i18n::system_language());
    let portable_data = match portable::initialize() {
        Ok(data) => data,
        Err(error) => { rfd::MessageDialog::new().set_title("Shiny Twitch Creator Tool portable").set_description(error).set_level(rfd::MessageLevel::Error).show(); std::process::exit(1); }
    };
    let data = portable_data.or_else(|| directories::ProjectDirs::from("com", "AudioTwitchCut", "Audio Twitch Cut").map(|p| p.data_local_dir().to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default().join("datos"));
    if let Err(error) = std::fs::create_dir_all(&data) {
        rfd::MessageDialog::new().set_title("Shiny Twitch Creator Tool").set_description(tf!("Could not create the data folder: {error}")).set_level(rfd::MessageLevel::Error).show(); return Ok(());
    }
    if std::env::args().any(|arg| arg == "--portable-check") {
        if let Err(error) = portable::verify(&data) { let _ = std::fs::write(data.join("error-portable.txt"), error); std::process::exit(1); }
        return Ok(());
    }
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1280.0, 920.0]).with_min_inner_size([1000.0, 720.0]), ..Default::default() };
    eframe::run_native("Shiny Twitch Creator Tool", options, Box::new(move |cc| Ok(Box::new(App::new(cc, data)))))
}
