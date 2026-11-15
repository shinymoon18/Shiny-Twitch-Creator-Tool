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
        Self { extra, audio: None, receiver: Some(rx), start, length, view, step: 0.001, target: Target::Selection, player: None, drag: None, status: "Cargando la onda del audio…".into(), cancel }
    }
    fn stop(&mut self) { self.player = None; }
    fn waveform(&mut self, ui: &mut egui::Ui) {
        let Some(audio) = &self.audio else { return; }; let total = audio.duration;
        self.view.clamp(total); let accent = ui.visuals().selection.stroke.color;
        ui.horizontal_wrapped(|ui| {
            let anchor = (self.start + self.length / 2.0).clamp(self.view.start, self.view.start + self.view.span);
            if ui.button("Acercar +").clicked() { self.view.zoom(2.0, anchor, total); }
            if ui.button("Alejar -").clicked() { self.view.zoom(0.5, anchor, total); }
            if ui.button("Ver selección").clicked() { self.view.fit(self.start, self.length, total); }
            if ui.button("Ver todo").clicked() { self.view = View::full(total); }
        });
        ui.add(egui::Slider::new(&mut self.view.start, 0.0..=(total - self.view.span).max(0.0)).text("Explorar audio").show_value(false));
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
            let height = (audio.waveform.peak(from, to) * 62.0).max(1.0);
            let x = rect.left() + col as f32 / columns as f32 * rect.width();
            painter.line_segment([egui::pos2(x, rect.center().y - height), egui::pos2(x, rect.center().y + height)], Stroke::new(1.0_f32, accent));
        }
        for at in [self.start, self.start + self.length] { let x = x_at(at); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(3.0_f32, accent)); }
        if let Some(at) = self.player.as_ref().and_then(|p| p.position()) { let x = x_at(at); painter.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(2.0_f32, Color32::WHITE)); }
        if response.drag_started() {
            if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
                self.drag = Some(if (pos.x - selected.left()).abs() < 9.0 { Drag::Start } else if (pos.x - selected.right()).abs() < 9.0 { Drag::End } else { Drag::Move(time_at(pos.x), if pos.x >= selected.left() && pos.x <= selected.right() { self.start } else { time_at(pos.x) }) });
            }
        }
        if response.dragged() {
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
        if response.clicked() { if let Some(pos) = response.interact_pointer_pos() { self.stop(); (self.start, self.length) = media::selection(time_at(pos.x), self.length, total); } }
        ui.horizontal(|ui| { ui.small(crate::time(view.start)); ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| { ui.small(crate::time(view.start + view.span)); }); });
    }
    pub fn show(&mut self, ctx: &egui::Context) -> Outcome {
        if let Some(result) = self.receiver.as_ref().and_then(|r| r.try_recv().ok()) {
            self.receiver = None;
            match result { Ok(audio) => { self.audio = Some(audio); self.status = "Arrastra los extremos o escribe los tiempos para elegir tu fragmento.".into(); }, Err(e) => self.status = e }
        }
        if let Some(player) = &mut self.player { match player.finished() { Ok(true) => self.player = None, Err(e) => { self.player = None; self.status = e; }, _ => () } }
        if self.receiver.is_some() || self.player.is_some() { ctx.request_repaint_after(Duration::from_millis(30)); }
        let mut open = true; let mut apply = false; let mut cancel = false;
        egui::Window::new("Recortar audio para el video").open(&mut open).default_width(760.0).max_height(760.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new(self.extra.path.file_name().unwrap_or_default().to_string_lossy()).size(19.0).strong());
            ui.label("Este fragmento sustituirá todo el sonido original del video.");
            if self.receiver.is_some() { ui.spinner(); }
            if self.audio.is_some() {
                self.waveform(ui); let total = self.extra.duration; let min = 0.001_f64.min(total);
                ui.horizontal_wrapped(|ui| {
                    for length in [9.0, 11.0] { if ui.button(format!("{length:.0} segundos")).clicked() { self.stop(); (self.start, self.length) = media::selection(self.start, length, total); } }
                    if ui.button("Todo el audio").clicked() { self.stop(); self.start = 0.0; self.length = total; }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut start = self.start; let mut end = self.start + self.length; let mut length = self.length;
                    ui.label("Inicio (s):");
                    if ui.add(egui::DragValue::new(&mut start).speed(self.step).fixed_decimals(3).range(0.0..=(end-min).max(0.0))).changed() { self.stop(); (self.start,self.length)=editor::resize_start(self.start,self.length,start,total); }
                    ui.label("Final (s):");
                    if ui.add(egui::DragValue::new(&mut end).speed(self.step).fixed_decimals(3).range((self.start+min).min(total)..=total)).changed() { self.stop(); (self.start,self.length)=editor::resize_end(self.start,end,total); }
                    ui.label("Duración (s):");
                    if ui.add(egui::DragValue::new(&mut length).speed(self.step).fixed_decimals(3).range(min..=(total-self.start).max(min))).changed() { self.stop(); (self.start,self.length)=editor::resize_end(self.start,self.start+length,total); }
                });
                ui.horizontal_wrapped(|ui| { ui.label("Paso:"); for (label, step) in [("1 ms",0.001),("5 ms",0.005),("100 ms",0.1),("500 ms",0.5),("1 min",60.0)] { ui.selectable_value(&mut self.step,step,label); } });
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.target,Target::Start,"Inicio"); ui.selectable_value(&mut self.target,Target::End,"Final"); ui.selectable_value(&mut self.target,Target::Selection,"Selección");
                    for (label, sign) in [("- Paso",-1.0),("+ Paso",1.0)] { if ui.button(label).clicked() { self.stop(); (self.start,self.length)=editor::nudge(self.start,self.length,sign*self.step,total,self.target); self.view.follow(self.start,total); } }
                    for (label, sign) in [("- Duración",-1.0),("+ Duración",1.0)] { if ui.button(label).clicked() { self.stop(); (self.start,self.length)=editor::nudge(self.start,self.length,sign*self.step,total,Target::End); } }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Escuchar fragmento").clicked() { self.stop(); match media::preview(self.audio.as_ref().unwrap(),self.start,self.length) { Ok(p)=>self.player=Some(p),Err(e)=>self.status=e } }
                    if ui.button("Escuchar desde aquí").clicked() { self.stop(); match media::preview(self.audio.as_ref().unwrap(),self.start,total-self.start) { Ok(p)=>self.player=Some(p),Err(e)=>self.status=e } }
                    if ui.button("Detener").clicked() { self.stop(); }
                    if ui.button("Marcar inicio al escuchar").clicked() { if let Some(at)=self.player.as_ref().and_then(|p|p.position()) { (self.start,self.length)=editor::mark_start(at,self.length,total); } }
                    if ui.button("Marcar final al escuchar").clicked() { if let Some(at)=self.player.as_ref().and_then(|p|p.position()) { if let Some(range)=editor::mark_end(self.start,at,total) { (self.start,self.length)=range; } } }
                });
                ui.label(format!("Se usará {} hasta {} / {:.3} segundos",crate::time(self.start),crate::time(self.start+self.length),self.length));
                ui.horizontal(|ui| { apply=ui.button(RichText::new("Usar este fragmento en el video").strong()).clicked(); cancel=ui.button("Cancelar").clicked(); });
                ui.small("Si el fragmento es más corto que el video, su resto queda en silencio. Si es más largo, se limita al corte de video.");
            }
            ui.label(&self.status);
            if self.audio.is_none() { cancel |= ui.button("Cancelar carga").clicked(); }
        });
        if !open || cancel { return Outcome::Cancel; }
        if apply { let mut extra=self.extra.clone(); extra.start=self.start; extra.length=Some(self.length); return Outcome::Apply(extra); }
        Outcome::Editing
    }
}
impl Drop for Editor { fn drop(&mut self) { self.cancel.store(true,Ordering::Relaxed); } }
