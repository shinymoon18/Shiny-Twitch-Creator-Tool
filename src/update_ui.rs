use crate::*;
use updates::Stage;

impl App {
    pub fn updates_ui(&mut self, ctx: &egui::Context) {
        if !self.updates.show { return; }
        let mut open = true;
        let mut download = false; let mut install = false; let mut check = false;
        egui::Window::new(tr!("Software updates")).id(egui::Id::new("software-updates")).open(&mut open).default_width(500.0).max_height(600.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(tf!("Installed version: {version}", version = env!("CARGO_PKG_VERSION")));
            match &self.updates.stage {
                Stage::Idle => { ui.label(tr!("Updates are provided through GitHub Releases.")); }
                Stage::Checking => { ui.horizontal(|ui| { ui.spinner(); ui.label(tr!("Checking GitHub Releases…")); }); }
                Stage::Current => { ui.label(tr!("No newer stable release is available.")); }
                Stage::Available => { ui.label(tr!("A new version is available.")); }
                Stage::Downloading => {
                    let size = self.updates.update.as_ref().map_or(1, |u| u.size).max(1);
                    let bytes = self.updates.progress.load(Ordering::Relaxed);
                    ui.add(egui::ProgressBar::new((bytes as f64 / size as f64) as f32).show_percentage());
                    ui.label(tf!("Downloaded {done:.1} / {total:.1} MB", done = bytes as f64 / 1_048_576.0, total = size as f64 / 1_048_576.0));
                    if ui.button(tr!("Cancel update download")).clicked() { self.updates.cancel_download(); }
                }
                Stage::Ready => { ui.label(tr!("The download passed SHA256 verification and is ready to install.")); }
                Stage::Installing => { ui.horizontal(|ui| { ui.spinner(); ui.label(tr!("Preparing installation and restart…")); }); }
                Stage::Error(error) => { ui.colored_label(Color32::from_rgb(255, 151, 155), tr!("The update could not be completed.")); ui.label(error); }
            }
            if let Some(update) = &self.updates.update {
                ui.separator();
                ui.label(RichText::new(tf!("Available version: {version}", version = &update.version)).strong());
                ui.label(&update.filename);
                ui.small(&update.page);
                egui::CollapsingHeader::new(tr!("Release notes")).show(ui, |ui| { ui.label(&update.notes); });
                if matches!(self.updates.stage, Stage::Available | Stage::Error(_)) {
                    download = ui.button(tr!("Download update")).clicked();
                }
            }
            if matches!(self.updates.stage, Stage::Ready) {
                ui.label(tr!("The app will close and restart. Your history and preferences will be kept. Unsaved selections are not restored."));
                let ready = self.receiver.is_none() && self.audio_editor.is_none() && !self.show_video_export;
                install = ui.add_enabled(ready, egui::Button::new(tr!("Install and restart"))).clicked();
                if !ready { ui.small(tr!("Finish the current operation and close editing/export dialogs before installing.")); }
            }
            if !self.updates.busy() && !matches!(self.updates.stage, Stage::Ready) { check = ui.button(tr!("Check again")).clicked(); }
            ui.small(tr!("Only stable releases from the official repository are used. The previous executable is kept as a backup."));
        });
        self.updates.show = open;
        if check { self.updates.check(ctx); }
        if download { self.updates.download(ctx); }
        if install { self.stop(); self.updates.install(&self.data, ctx); }
    }
}
