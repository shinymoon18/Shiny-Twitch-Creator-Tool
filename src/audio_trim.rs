use crate::{editor::{self, Target, View}, media, video::ExternalAudio};
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::{sync::{mpsc::{self, Receiver}, Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

#[derive(Clone, Copy)]
enum Drag { Start, End, Move(f64, f64) }
pub enum Outcome { Editing, Apply(ExternalAudio), Cancel }
pub struct Editor {
    extra: ExternalAudio, audio: Option<media::Audio>, receiver: Option<Receiver<media::Result<media::Audio>>>,
    start: f64, length: f64, video_length: f64, view: View, step: f64, target: Target,
    player: Option<media::Playback>, drag: Option<Drag>, status: String,
    cancel: Arc<AtomicBool>,
    #[cfg(test)] waveform_rect: Option<egui::Rect>,
}
impl Editor {
    pub fn new(extra: ExternalAudio, video_length: f64, ctx: &egui::Context) -> Self {
        let path = extra.path.clone(); let ctx = ctx.clone(); let (tx, rx) = mpsc::channel();
        let cancel=Arc::new(AtomicBool::new(false)); let worker_cancel=cancel.clone();
        std::thread::spawn(move || { media::set_cancellation(worker_cancel); let _ = tx.send(media::load(path)); ctx.request_repaint(); });
        let (start, length) = range_for_video(&extra, video_length);
        let mut view = View::full(extra.duration); view.fit(start, length, extra.duration);
        Self { extra, audio: None, receiver: Some(rx), start, length, video_length, view, step: 0.001, target: Target::Selection, player: None, drag: None, status: tr!("Loading the audio waveform…").into(), cancel, #[cfg(test)] waveform_rect: None }
    }
    fn stop(&mut self) { self.player = None; }
    fn move_start(&mut self, at: f64) {
        self.stop();
        (self.start, self.length) = media::selection(at, self.length, self.extra.duration);
        self.view.fit(self.start, self.length, self.extra.duration);
    }
    fn match_video_duration(&mut self) {
        self.stop();
        (self.start, self.length) = media::selection(self.start, self.video_length, self.extra.duration);
        self.extra.fades = self.extra.fades.bounded(self.length);
        self.view.fit(self.start, self.length, self.extra.duration);
    }
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
        #[cfg(test)] { self.waveform_rect = Some(rect); }
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
        for (at, label) in [(self.start, tr!("START")), (self.start + self.length, tr!("END"))] {
            let x = x_at(at);
            painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(3.0_f32, accent));
            if rect.contains(egui::pos2(x, rect.center().y)) {
                painter.rect_filled(egui::Rect::from_center_size(egui::pos2(x, rect.center().y), Vec2::new(10.0, 28.0)), 3.0, accent);
                painter.text(egui::pos2(x.clamp(rect.left()+30.0, rect.right()-30.0), rect.top()+5.0), egui::Align2::CENTER_TOP, label, egui::FontId::proportional(11.0), Color32::WHITE);
            }
        }
        if let Some(at) = self.player.as_ref().and_then(|p| p.position()) { let x = x_at(at); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE)); }
        let (fade_active, fade_changed) = crate::fades::overlay(ui, rect, view, self.start, self.length, &mut self.extra.fades);
        if fade_changed { self.stop(); }
        if !fade_active {
            if let Some(pos) = response.hover_pos() {
                ui.ctx().set_cursor_icon(if (pos.x-selected.left()).abs() < 9.0 || (pos.x-selected.right()).abs() < 9.0 { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::Grab });
            }
        }
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
        egui::Window::new(tr!("Trim audio for the video")).id(egui::Id::new("external-audio-editor")).open(&mut open).default_width(760.0).max_height(760.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new(self.extra.path.file_name().unwrap_or_default().to_string_lossy()).size(19.0).strong());
            ui.label(tr!("This clip will replace all original video audio."));
            ui.label(tf!("Video clip duration: {seconds:.3} s", seconds = self.video_length));
            if self.receiver.is_some() { ui.spinner(); }
            if self.audio.is_some() {
                self.waveform(ui); if crate::fades::controls(ui, &mut self.extra.fades, self.length) { self.stop(); } let total = self.extra.duration; let min = 0.001_f64.min(total);
                let mut position = self.start;
                if ui.add(egui::Slider::new(&mut position, 0.0..=(total-self.length).max(0.0)).text(tr!("Choose audio start")).fixed_decimals(3)).changed() { self.move_start(position); }
                ui.small(tr!("Move the slider or change Start to choose any part of the audio without changing the clip duration."));
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr!("Match video duration")).clicked() { self.match_video_duration(); }
                    for length in [9.0, 11.0] { if ui.button(tf!("{length:.0} seconds")).clicked() { self.stop(); (self.start, self.length) = media::selection(self.start, length, total); } }
                    if ui.button(tr!("Entire audio")).clicked() { self.stop(); self.start = 0.0; self.length = total; }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut start = self.start; let mut end = self.start + self.length; let mut length = self.length;
                    ui.label(tr!("Start (s):"));
                    if ui.add(egui::DragValue::new(&mut start).speed(self.step).fixed_decimals(3).range(0.0..=(total-self.length).max(0.0))).changed() { self.move_start(start); }
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

fn range_for_video(extra: &ExternalAudio, video_length: f64) -> (f64, f64) {
    media::selection(extra.start, video_length.max(editor::MIN_LENGTH.min(extra.duration)), extra.duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Harness { ctx: egui::Context, editor: Editor, time: f64 }
    impl Harness {
        fn new() -> Self {
            let total = 180.0;
            let ctx = egui::Context::default();
            let mut editor = Editor::new(ExternalAudio { path: "test.mp3".into(), duration: total, start: 0.0, db: 0.0, length: None, fades: Default::default() }, 11.0, &ctx);
            editor.receiver = None;
            editor.audio = Some(media::Audio { path: "test.mp3".into(), duration: total, waveform: Arc::new(media::Waveform::silent(total)), video: None });
            Self { ctx, editor, time: 0.0 }
        }
        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 0.1;
            let _ = self.ctx.run(egui::RawInput { time: Some(self.time), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(1200.0, 1000.0))), events, ..Default::default() }, |ctx| { self.editor.show(ctx); });
        }
        fn pointer(&mut self, at: egui::Pos2, pressed: bool) {
            self.frame(vec![egui::Event::PointerMoved(at), egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE }]);
        }
    }
    #[test] fn trim_window_can_seek_and_drag_audio_selection() {
        let mut h = Harness::new(); h.frame(vec![]); h.frame(vec![]); h.frame(vec![]);
        // Test full-track navigation as well as the initial focused view.
        h.editor.view = View::full(180.0); h.frame(vec![]);
        let rect = h.editor.waveform_rect.unwrap();
        let at = egui::pos2(rect.center().x, rect.center().y);
        h.frame(vec![egui::Event::PointerMoved(at)]); h.pointer(at, true); h.pointer(at, false);
        assert!(h.editor.start > 30.0, "clicking the MP3 waveform did not select a later section: {}", h.editor.start);
        assert!((h.editor.length - 11.0).abs() < 0.001);
        let at = egui::pos2(rect.left() + ((h.editor.start + 5.5 - h.editor.view.start) / h.editor.view.span) as f32 * rect.width(), rect.center().y);
        let old = h.editor.start;
        h.frame(vec![egui::Event::PointerMoved(at)]); h.pointer(at, true);
        h.frame(vec![egui::Event::PointerMoved(at + Vec2::new(50.0, 0.0))]);
        h.pointer(at + Vec2::new(50.0, 0.0), false);
        assert!(h.editor.start > old + 1.0, "dragging the MP3 selection did not move it: {} -> {}", old, h.editor.start);
        assert!((h.editor.length - 11.0).abs() < 0.001);
    }
    #[test] fn editor_automatically_matches_current_video_and_can_seek_past_old_end() {
        let mut h = Harness::new();
        assert_eq!((h.editor.start, h.editor.length), (0.0, 11.0));
        assert!(h.editor.view.span < 15.0, "the initial selection should be visible at a useful zoom");
        h.editor.move_start(75.125);
        assert_eq!((h.editor.start, h.editor.length), (75.125, 11.0));
        h.editor.move_start(179.0);
        assert_eq!((h.editor.start, h.editor.length), (169.0, 11.0));
        h.editor.length = 2.0;
        h.editor.match_video_duration();
        assert_eq!(h.editor.length, 11.0);
        let extra = ExternalAudio { length: Some(2.0), start: 175.0, ..h.editor.extra.clone() };
        assert_eq!(range_for_video(&extra, 9.5), (170.5, 9.5));
        let short = ExternalAudio { duration: 3.0, start: 0.0, ..extra };
        assert_eq!(range_for_video(&short, 11.0), (0.0, 3.0));
    }
}
