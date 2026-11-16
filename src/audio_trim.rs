use crate::{editor::{self, Target, View}, media, video::ExternalAudio};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{sync::{mpsc::{self, Receiver}, Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

#[derive(Clone, Copy)]
enum Drag { Start, End, Move(f64, f64) }
pub enum Outcome { Editing, Apply(ExternalAudio), Cancel }
pub struct Editor {
    extra: ExternalAudio, audio: Option<media::Audio>, receiver: Option<Receiver<media::Result<media::Audio>>>,
    start: f64, length: f64, view: View, step: f64, target: Target,
    player: Option<media::Playback>, drag: Option<Drag>, status: String,
    cancel: Arc<AtomicBool>,
}
impl Editor {
    pub fn new(extra: ExternalAudio, video_length: f64, ctx: &egui::Context) -> Self {
        let path = extra.path.clone(); let ctx = ctx.clone(); let (tx, rx) = mpsc::channel();
        let cancel=Arc::new(AtomicBool::new(false)); let worker_cancel=cancel.clone();
        std::thread::spawn(move || { media::set_cancellation(worker_cancel); let _ = tx.send(media::load(path)); ctx.request_repaint(); });
        let start = extra.start;
        let length = extra.length.unwrap_or(video_length).min(extra.duration - start).max(0.001_f64.min(extra.duration));
        let view = View::full(extra.duration);
        Self { extra, audio: None, receiver: Some(rx), start, length, view, step: 0.001, target: Target::Selection, player: None, drag: None, status: tr!("Loading the audio waveform…").into(), cancel }
    }
    fn stop(&mut self) { self.player = None; }
    fn waveform(&mut self, ui: &mut egui::Ui) {
        let Some(audio) = &self.audio else { return; }; let total = audio.duration;
        self.view.clamp(total); let accent = ui.visuals().selection.stroke.color;
        ui.horizontal_wrapped(|ui| {
            let anchor = (self.start + self.length / 2.0).clamp(self.view.start, self.view.start + self.view.span);
            if ui.button(tr!("Zoom in +")).clicked() { self.view.zoom(2.0, anchor, total); }
            if ui.button(tr!("Zoom out -")).clicked() { self.view.zoom(0.5, anchor, total); }
            if ui.button(tr!("Fit selection")).clicked() { self.view.fit(self.start, self.length, total); }
            if ui.button(tr!("Fit all")).clicked() { self.view = View::full(total); }
        });
        ui.add(egui::Slider::new(&mut self.view.start, 0.0..=(total - self.view.span).max(0.0)).text(tr!("Scroll audio")).show_value(false));
        let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 150.0), egui::Sense::click_and_drag());
        let painter = ui.painter().with_clip_rect(rect); painter.rect_filled(rect, 8.0, ui.visuals().extreme_bg_color);
        let view = self.view;
        let x_at = |time: f64| rect.left() + ((time - view.start) / view.span) as f32 * rect.width();
        let time_at = |x: f32| (view.start + ((x - rect.left()) / rect.width()) as f64 * view.span).clamp(0.0, total);
        let selected = egui::Rect::from_min_max(egui::pos2(x_at(self.start), rect.top()), egui::pos2(x_at(self.start + self.length), rect.bottom()));
        if selected.intersects(rect) { painter.rect_filled(selected.intersect(rect), 0.0, accent.gamma_multiply(0.25)); }
        let columns = (rect.width() / 2.0).max(1.0) as usize;
        for col in 0..columns {
            let from = view.start + col as f64 / columns as f64 * view.span;
            let to = view.start + (col + 1) as f64 / columns as f64 * view.span;
            let envelope = if from >= self.start && from <= self.start + self.length { self.extra.fades.envelope((from + to) / 2.0 - self.start, self.length) } else { 1.0 };
            let height = (audio.waveform.peak(from, to) * 62.0 * envelope).max(1.0);
            let x = rect.left() + col as f32 / columns as f32 * rect.width();
            painter.line_segment([egui::pos2(x, rect.center().y - height), egui::pos2(x, rect.center().y + height)], Stroke::new(1.0_f32, accent));
        }
        for at in [self.start, self.start + self.length] { let x = x_at(at); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(3.0_f32, accent)); }
        if let Some(at) = self.player.as_ref().and_then(|p| p.position()) { let x = x_at(at); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE)); }
        let (fade_active, fade_changed) = crate::fades::overlay(ui, rect, view, self.start, self.length, &mut self.extra.fades);
        if fade_changed { self.stop(); }
        if response.drag_started() && !fade_active {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                self.drag = Some(if (pos.x - selected.left()).abs() < 9.0 { Drag::Start } else if (pos.x - selected.right()).abs() < 9.0 { Drag::End } else { Drag::Move(time_at(pos.x), if pos.x >= selected.left() && pos.x <= selected.right() { self.start } else { time_at(pos.x) }) });
            }
        }
        if response.dragged() && !fade_active {
            if let (Some(pos), Some(drag)) = (response.interact_pointer_pos(), self.drag) {
                self.stop();
                (self.start, self.length) = match drag {
                    Drag::Start => editor::resize_start(self.start, self.length, time_at(pos.x), total),
                    Drag::End => editor::resize_end(self.start, time_at(pos.x), total),
                    Drag::Move(pointer, start) => media::selection(start + time_at(pos.x) - pointer, self.length, total),
                };
            }
        }
        if response.drag_stopped() { self.drag = None; }
        if response.clicked() && !fade_active { if let Some(pos) = response.interact_pointer_pos() { self.stop(); (self.start, self.length) = media::selection(time_at(pos.x), self.length, total); } }
        ui.horizontal(|ui| { ui.small(crate::time(view.start)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.small(crate::time(view.start + view.span)); }); });
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Outcome {
        if let Some(result) = self.receiver.as_ref().and_then(|r| r.try_recv().ok()) {
            self.receiver = None;
            match result { Ok(audio) => { self.audio = Some(audio); self.status = tr!("Drag the edges or enter times to select your audio clip.").into(); }, Err(e) => self.status = e }
        }
        if let Some(player) = &mut self.player { match player.finished() { Ok(true) => self.player = None, Err(e) => { self.player = None; self.status = e; }, _ => () } }
        if self.receiver.is_some() || self.player.is_some() { ctx.request_repaint_after(Duration::from_millis(30)); }
        let mut open = true; let mut apply = false; let mut cancel = false;
        egui::Window::new(tr!("Trim audio for the video")).open(&mut open).default_width(760.0).max_height(760.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new(self.extra.path.file_name().unwrap_or_default().to_string_lossy()).size(19.0).strong());
            ui.label(tr!("This clip will replace all original video audio."));
            if self.receiver.is_some() { ui.spinner(); }
            if self.audio.is_some() {
                self.waveform(ui); if crate::fades::controls(ui, &mut self.extra.fades, self.length) { self.stop(); } let total = self.extra.duration; let min = 0.001_f64.min(total);
                ui.horizontal_wrapped(|ui| {
                    for length in [9.0, 11.0] { if ui.button(tf!("{length:.0} seconds")).clicked() { self.stop(); (self.start, self.length) = media::selection(self.start, length, total); } }
                    if ui.button(tr!("Entire audio")).clicked() { self.stop(); self.start = 0.0; self.length = total; }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut start = self.start; let mut end = self.start + self.length; let mut length = self.length;
                    ui.label(tr!("Start (s):"));
                    if ui.add(egui::DragValue::new(&mut start).speed(self.step).fixed_decimals(3).range(0.0..=(end-min).max(0.0))).changed() { self.stop(); (self.start,self.length)=editor::resize_start(self.start,self.length,start,total); }
                    ui.label(tr!("End (s):"));
                    if ui.add(egui::DragValue::new(&mut end).speed(self.step).fixed_decimals(3).range((self.start+min).min(total)..=total)).changed() { self.stop(); (self.start,self.length)=editor::resize_end(self.start,end,total); }
                    ui.label(tr!("Duration (s):"));
                    if ui.add(egui::DragValue::new(&mut length).speed(self.step).fixed_decimals(3).range(min..=(total-self.start).max(min))).changed() { self.stop(); (self.start,self.length)=editor::resize_end(self.start,self.start+length,total); }
                });
                ui.horizontal_wrapped(|ui| { ui.label(tr!("Step:")); for (label, step) in [("1 ms",0.001),("5 ms",0.005),("100 ms",0.1),("500 ms",0.5),("1 min",60.0)] { ui.selectable_value(&mut self.step,step,label); } });
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.target,Target::Start,tr!("Start")); ui.selectable_value(&mut self.target,Target::End,tr!("End")); ui.selectable_value(&mut self.target,Target::Selection,tr!("Selection"));
                    for (label, sign) in [(tr!("- Step"),-1.0),(tr!("+ Step"),1.0)] { if ui.button(label).clicked() { self.stop(); (self.start,self.length)=editor::nudge(self.start,self.length,sign*self.step,total,self.target); self.view.follow(self.start,total); } }
                    for (label, sign) in [(tr!("- Duration"),-1.0),(tr!("+ Duration"),1.0)] { if ui.button(label).clicked() { self.stop(); (self.start,self.length)=editor::nudge(self.start,self.length,sign*self.step,total,Target::End); } }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr!("Preview clip")).clicked() { self.stop(); match media::preview_effects(self.audio.as_ref().unwrap(),self.start,self.length,self.extra.db,self.extra.fades.bounded(self.length)) { Ok(p)=>self.player=Some(p),Err(e)=>self.status=e } }
                    if ui.button(tr!("Preview from here")).clicked() { self.stop(); match media::preview_effects(self.audio.as_ref().unwrap(),self.start,total-self.start,self.extra.db,self.extra.fades.bounded(self.length)) { Ok(p)=>self.player=Some(p),Err(e)=>self.status=e } }
                    if ui.button(tr!("Stop")).clicked() { self.stop(); }
                    if ui.button(tr!("Mark start during playback")).clicked() { if let Some(at)=self.player.as_ref().and_then(|p|p.position()) { (self.start,self.length)=editor::mark_start(at,self.length,total); } }
                    if ui.button(tr!("Mark end during playback")).clicked() { if let Some(at)=self.player.as_ref().and_then(|p|p.position()) { if let Some(range)=editor::mark_end(self.start,at,total) { (self.start,self.length)=range; } } }
                });
                ui.label(tf!("Using {} to {} / {:.3} seconds",crate::time(self.start),crate::time(self.start+self.length),self.length));
                ui.horizontal(|ui| { apply=ui.button(RichText::new(tr!("Use this audio clip in the video")).strong()).clicked(); cancel=ui.button(tr!("Cancel")).clicked(); });
                ui.small(tr!("If the audio clip is shorter than the video, the remaining time is silent. If longer, it is trimmed to the video selection."));
            }
            ui.label(&self.status);
            if self.audio.is_none() { cancel |= ui.button(tr!("Cancel loading")).clicked(); }
        });
        if !open || cancel { return Outcome::Cancel; }
        if apply { let mut extra=self.extra.clone(); extra.start=self.start; extra.length=Some(self.length); extra.fades=extra.fades.bounded(self.length); return Outcome::Apply(extra); }
        Outcome::Editing
    }
}
impl Drop for Editor { fn drop(&mut self) { self.cancel.store(true,Ordering::Relaxed); } }
