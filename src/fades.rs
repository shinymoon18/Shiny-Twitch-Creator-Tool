use eframe::egui::{self, Color32, Stroke, Vec2};
use serde::{Deserialize, Serialize};
use crate::editor::View;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fades { pub input: f64, pub output: f64 }
impl Fades {
    pub fn bounded(self, length: f64) -> Self {
        let length = if length.is_finite() { length.max(0.0) } else { 0.0 };
        let clean = |v: f64| if v.is_finite() { v.clamp(0.0, length) } else { 0.0 };
        // Independent envelopes may overlap on very short clips.
        Self { input: clean(self.input), output: clean(self.output) }
    }
    pub fn envelope(self, at: f64, length: f64) -> f32 {
        let fade = self.bounded(length);
        let input = if fade.input > 0.0 { (at / fade.input).clamp(0.0, 1.0) } else { 1.0 };
        let output = if fade.output > 0.0 { ((length - at) / fade.output).clamp(0.0, 1.0) } else { 1.0 };
        (input * output) as f32
    }
    pub fn filter(self, length: f64) -> String {
        let fade = self.bounded(length); let mut filters = Vec::new();
        if fade.input > 0.0 { filters.push(format!("afade=t=in:st=0:d={:.6}", fade.input)); }
        if fade.output > 0.0 { filters.push(format!("afade=t=out:st={:.6}:d={:.6}", length - fade.output, fade.output)); }
        if filters.is_empty() { "anull".into() } else { filters.join(",") }
    }
}

pub fn controls(ui: &mut egui::Ui, fades: &mut Fades, length: f64) -> bool {
    *fades = fades.bounded(length); let previous = *fades;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr!("Fade in (s):"));
        ui.add(egui::DragValue::new(&mut fades.input).speed(0.01).range(0.0..=length).fixed_decimals(3));
        ui.label(tr!("Fade out (s):"));
        ui.add(egui::DragValue::new(&mut fades.output).speed(0.01).range(0.0..=length).fixed_decimals(3));
        if ui.button(tr!("Reset fades")).clicked() { *fades = Fades::default(); }
    });
    ui.small(tr!("Drag the green IN and OUT handles on the waveform to adjust the fades."));
    previous != *fades
}

// Separate handles have priority over the selection's drag area. The different
// vertical positions keep zero-length and overlapping fades independently usable.
pub fn overlay(ui: &mut egui::Ui, rect: egui::Rect, view: View, start: f64, length: f64, fades: &mut Fades) -> (bool, bool) {
    *fades = fades.bounded(length); let previous = *fades;
    let x_at = |at: f64| rect.left() + ((at - view.start) / view.span) as f32 * rect.width();
    let painter = ui.painter().with_clip_rect(rect);
    let color = Color32::from_rgb(95, 222, 196);
    let mut active = false;
    for input in [true, false] {
        let at = if input { start + fades.input } else { start + length - fades.output };
        let pos = egui::pos2(x_at(at), if input { rect.top() + 30.0 } else { rect.bottom() - 25.0 });
        let edge = egui::pos2(x_at(if input { start } else { start + length }), rect.center().y);
        painter.line_segment([edge, pos], Stroke::new(2.0_f32, color));
        if !rect.contains(pos) { continue; }
        let response = ui.interact(egui::Rect::from_center_size(pos, Vec2::splat(24.0)), ui.id().with(("fade-handle", input)), egui::Sense::click_and_drag());
        painter.circle_filled(pos, 8.0, color);
        painter.text(pos + Vec2::new(if input { 13.0 } else { -13.0 }, 0.0), if input { egui::Align2::LEFT_CENTER } else { egui::Align2::RIGHT_CENTER }, if input { tr!("IN") } else { tr!("OUT") }, egui::FontId::proportional(11.0), color);
        if response.hovered() || response.dragged() { ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal); }
        active |= response.hovered() || response.dragged() || response.clicked() || response.drag_stopped();
        if response.dragged() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let at = view.start + ((pointer.x - rect.left()) / rect.width()) as f64 * view.span;
                if input { fades.input = (at - start).clamp(0.0, length); }
                else { fades.output = (start + length - at).clamp(0.0, length); }
            }
        }
    }
    (active, previous != *fades)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn dragging_waveform_handles_changes_fades_without_moving_selection() {
        let ctx = egui::Context::default(); let mut fades = Fades::default();
        let mut draw = |events: Vec<egui::Event>| {
            let mut region = egui::Rect::NOTHING; let mut selection_dragged = false;
            let _ = ctx.run(egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, Vec2::new(800.0, 400.0))), events, ..Default::default() }, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let (rect, response) = ui.allocate_exact_size(Vec2::new(700.0, 170.0), egui::Sense::click_and_drag());
                    region = rect;
                    let (active, _) = overlay(ui, rect, View::full(10.0), 1.0, 4.0, &mut fades);
                    selection_dragged = response.dragged() && !active;
                });
            });
            (region, selection_dragged, fades)
        };
        let (rect, _, _) = draw(vec![]); draw(vec![]);
        for (input, x) in [(true, rect.left() + 70.0), (false, rect.left() + 350.0)] {
            let origin = egui::pos2(x, if input { rect.top() + 30.0 } else { rect.bottom() - 25.0 });
            draw(vec![egui::Event::PointerMoved(origin)]);
            draw(vec![egui::Event::PointerButton { pos: origin, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE }]);
            let target = origin + Vec2::new(if input { 70.0 } else { -70.0 }, 0.0);
            let (_, selection_dragged, values) = draw(vec![egui::Event::PointerMoved(target)]);
            assert!(!selection_dragged, "fade drag was handled as a selection drag");
            assert!(((if input { values.input } else { values.output }) - 1.0).abs() < 0.01);
            draw(vec![egui::Event::PointerButton { pos: target, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE }]);
        }
    }
    #[test] fn fades_follow_clip_duration_and_reject_nonfinite_values() {
        assert_eq!(Fades { input: 9.0, output: f64::NAN }.bounded(0.005), Fades { input: 0.005, output: 0.0 });
        let f = Fades { input: 0.5, output: 1.0 };
        assert_eq!(f.envelope(0.0, 3.0), 0.0);
        assert_eq!(f.envelope(1.0, 3.0), 1.0);
        assert!((f.envelope(2.5, 3.0) - 0.5).abs() < 1e-6);
        assert_eq!(f.envelope(3.0, 3.0), 0.0);
        assert!(f.filter(3.0).contains("afade=t=out:st=2.000000:d=1.000000"));
    }
}
