//! Selection and viewport math, shared by mouse, fields and precision buttons.
pub const MIN_LENGTH: f64 = 0.001;

pub fn move_selection(start: f64, length: f64, delta: f64, total: f64) -> (f64, f64) {
    crate::media::selection(start + delta, length, total)
}

pub fn resize_start(start: f64, length: f64, new_start: f64, total: f64) -> (f64, f64) {
    let end = (start + length).min(total);
    let start = new_start.clamp(0.0, (end - MIN_LENGTH.min(total)).max(0.0));
    (start, end - start)
}

pub fn resize_end(start: f64, new_end: f64, total: f64) -> (f64, f64) {
    let end = new_end.clamp((start + MIN_LENGTH.min(total)).min(total), total);
    (start, end - start)
}

pub fn mark_start(at: f64, length: f64, total: f64) -> (f64, f64) {
    let start = at.clamp(0.0, (total - MIN_LENGTH.min(total)).max(0.0));
    (start, length.max(MIN_LENGTH.min(total)).min(total - start))
}

pub fn mark_end(start: f64, at: f64, total: f64) -> Option<(f64, f64)> {
    if at - start < MIN_LENGTH.min(total) - 1e-9 { return None; }
    Some(resize_end(start, at, total))
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Target { Start, End, #[default] Selection }

pub fn nudge(start: f64, length: f64, delta: f64, total: f64, target: Target) -> (f64, f64) {
    match target {
        Target::Start => resize_start(start, length, start + delta, total),
        Target::End => resize_end(start, start + length + delta, total),
        Target::Selection => move_selection(start, length, delta, total),
    }
}

#[derive(Clone, Copy)]
pub struct View { pub start: f64, pub span: f64 }
impl View {
    pub fn full(total: f64) -> Self { Self { start: 0.0, span: total } }
    pub fn clamp(&mut self, total: f64) {
        self.span = self.span.clamp(0.05_f64.min(total), total);
        self.start = self.start.clamp(0.0, (total - self.span).max(0.0));
    }
    pub fn zoom(&mut self, factor: f64, anchor: f64, total: f64) {
        let fraction = ((anchor - self.start) / self.span).clamp(0.0, 1.0);
        self.span = (self.span / factor).clamp(0.05_f64.min(total), total);
        self.start = anchor - fraction * self.span;
        self.clamp(total);
    }
    pub fn fit(&mut self, start: f64, length: f64, total: f64) {
        self.span = length * 1.25;
        self.start = start - length * 0.125;
        self.clamp(total);
    }
    pub fn follow(&mut self, time: f64, total: f64) {
        if time < self.start || time > self.start + self.span {
            self.start = time - self.span / 2.0;
            self.clamp(total);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn near(actual: f64, expected: f64) { assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}"); }
    #[test] fn playback_markers_use_absolute_track_time_and_keep_valid_range() {
        let (start, length) = mark_start(42.125, 9.0, 100.0);
        near(start, 42.125); near(length, 9.0);
        let (start, length) = mark_end(start, 44.375, 100.0).unwrap();
        near(start, 42.125); near(length, 2.25);
        assert!(mark_end(start, 41.0, 100.0).is_none());
        let (start, length) = mark_start(99.5, 11.0, 100.0);
        near(start, 99.5); near(length, 0.5);
    }
    #[test] fn steps_are_precise_and_selection_keeps_length() {
        for delta in [0.001, 0.005, 0.100, 0.500, 60.0] {
            let (start, length) = nudge(10.0, 13.5, delta, 200.0, Target::Selection);
            near(start, 10.0 + delta); near(length, 13.5);
            near(nudge(start, length, -delta, 200.0, Target::Selection).0, 10.0);
        }
    }
    #[test] fn independent_edges_can_extend_beyond_presets() {
        let (start, length) = nudge(10.0, 9.0, 0.5, 100.0, Target::End);
        near(start, 10.0); near(length, 9.5);
        let (start, length) = nudge(start, length, -0.1, 100.0, Target::Start);
        near(start, 9.9); near(start + length, 19.5);
    }
    #[test] fn direct_duration_changes_add_fractions_to_both_presets() {
        for base in [9.0, 11.0] {
            for delta in [0.001, 0.005, 0.100, 0.500, 60.0] {
                let (start, length) = nudge(2.125, base, delta, 200.0, Target::End);
                near(start, 2.125); near(length, base + delta);
                let (start, length) = nudge(start, length, -delta, 200.0, Target::End);
                near(start, 2.125); near(length, base);
            }
        }
        let mut length = 9.0;
        for delta in [0.001, 0.005, 0.100, 0.500] { length = nudge(2.125, length, delta, 200.0, Target::End).1; }
        near(length, 9.606);
    }
    #[test] fn edges_do_not_cross_or_leave_track() {
        assert_eq!(nudge(10.0, 9.0, 1000.0, 100.0, Target::End), (10.0, 90.0));
        let (start, length) = nudge(10.0, 9.0, 1000.0, 100.0, Target::Start);
        near(start + length, 19.0); near(length, 0.001);
        assert_eq!(nudge(10.0, 9.0, -60.0, 100.0, Target::Selection), (0.0, 9.0));
        assert_eq!(nudge(10.0, 9.0, 1000.0, 100.0, Target::Selection), (91.0, 9.0));
    }
    #[test] fn zoom_preserves_anchor_and_limits_view() {
        let mut view = View::full(200.0);
        view.zoom(2.0, 50.0, 200.0);
        near(view.span, 100.0); near(view.start, 25.0);
        view.zoom(1e9, 50.0, 200.0); near(view.span, 0.05);
        view.fit(190.0, 10.0, 200.0); near(view.start + view.span, 200.0);
        view.follow(0.0, 200.0); near(view.start, 0.0);
    }
}
