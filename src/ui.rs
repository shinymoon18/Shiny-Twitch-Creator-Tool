use super::*;

thread_local! { static PALETTE: std::cell::Cell<theme::Palette> = std::cell::Cell::new(theme::Appearance::default().palette()); }
fn palette() -> theme::Palette { PALETTE.with(|p| p.get()) }

pub fn configure(ctx: &egui::Context, appearance: &theme::Appearance) {
    PALETTE.with(|p| p.set(appearance.palette()));
    ctx.set_zoom_factor(appearance.scale.clamp(0.85, 1.3));
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.override_text_color = Some(palette().text);
    style.visuals.panel_fill = palette().bg;
    style.visuals.window_fill = palette().card;
    style.visuals.extreme_bg_color = palette().bg;
    style.visuals.faint_bg_color = palette().card;
    style.visuals.selection.bg_fill = palette().accent.gamma_multiply(0.40);
    style.visuals.selection.stroke = Stroke::new(1.0_f32, palette().accent);
    for widget in [&mut style.visuals.widgets.inactive, &mut style.visuals.widgets.noninteractive] {
        widget.bg_fill = palette().card.gamma_multiply(1.25);
        widget.weak_bg_fill = palette().card.gamma_multiply(1.25);
        widget.bg_stroke = Stroke::new(1.0_f32, palette().border);
        widget.fg_stroke = Stroke::new(1.0_f32, palette().text);
        widget.rounding = egui::Rounding::same(7.0);
    }
    style.visuals.widgets.hovered.bg_fill = palette().accent.gamma_multiply(0.25);
    style.visuals.widgets.hovered.weak_bg_fill = palette().accent.gamma_multiply(0.25);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, palette().accent);
    style.visuals.widgets.active.bg_fill = palette().accent.gamma_multiply(0.4);
    style.visuals.widgets.active.weak_bg_fill = palette().accent.gamma_multiply(0.4);
    style.spacing.item_spacing = Vec2::new(8.0, 9.0);
    style.spacing.button_padding = Vec2::new(12.0, 9.0);
    style.spacing.interact_size.y = 32.0;
    style.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    style.text_styles.insert(egui::TextStyle::Small, egui::FontId::proportional(12.0));
    ctx.set_style(style);
}

fn card(ui: &mut egui::Ui, title: &str, subtitle: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::none().fill(palette().card).stroke(Stroke::new(1.0_f32, palette().border)).rounding(12.0).inner_margin(16.0).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.label(RichText::new(title).size(17.0).strong());
        if !subtitle.is_empty() { ui.label(RichText::new(subtitle).small().color(palette().muted)); }
        ui.add_space(4.0);
        body(ui);
    });
}

impl App {
    fn options_ui(&mut self, ctx: &egui::Context) {
        if !self.show_options { return; }
        let mut open = true;
        let previous = self.settings.language;
        egui::Window::new(tr!("Options")).id(egui::Id::new("options-window")).open(&mut open).resizable(false).default_width(390.0).show(ctx, |ui| {
            ui.heading("Language / Idioma");
            ui.radio_value(&mut self.settings.language, None, tr!("System default"));
            ui.radio_value(&mut self.settings.language, Some(i18n::Language::English), "English");
            ui.radio_value(&mut self.settings.language, Some(i18n::Language::Spanish), "Español");
            ui.small(tr!("Language changes apply immediately and are saved for next time."));
            ui.separator();
            if ui.button(tr!("Customize appearance…")).clicked() { self.show_themes = true; }
        });
        self.show_options = open;
        if previous != self.settings.language {
            self.settings.apply();
            self.error = false;
            self.status = tr!("Open a track or paste a YouTube link to get started.").into();
            let result = serde_json::to_vec_pretty(&self.settings).map_err(|e| e.to_string()).and_then(|bytes| std::fs::write(self.data.join("settings.json"), bytes).map_err(|e| e.to_string()));
            if let Err(e) = result { self.message(Err(tf!("Could not save options: {e}"))); }
        }
    }
    fn video_sound_controls(&mut self, ui: &mut egui::Ui, has_audio: bool) {
        use video::SoundMode;
        let previous = self.video_options.mode;
        ui.label(RichText::new(tr!("Which audio would you like to save?")).strong());
        ui.add_enabled_ui(has_audio, |ui| { ui.radio_value(&mut self.video_options.mode, SoundMode::Original, tr!("Keep original audio")); });
        ui.radio_value(&mut self.video_options.mode, SoundMode::Silent, tr!("Export without audio"));
        ui.radio_value(&mut self.video_options.mode, SoundMode::Replace, tr!("Replace with local audio"));
        if !has_audio { ui.small(tr!("This video has no original audio.")); }
        if self.video_options.mode == SoundMode::Replace {
            if ui.button(tr!("Choose audio file…")).clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac", "opus", "webm"]).pick_file() {
                    match video::external_audio(path) { Ok(extra) => { self.stop(); self.audio_editor = Some(audio_trim::Editor::new(extra, self.length, ui.ctx())); }, Err(e) => self.message(Err(e)) }
                }
            }
            if let Some(extra) = &self.video_options.external {
                ui.label(RichText::new(extra.path.file_name().unwrap_or_default().to_string_lossy()).strong());
                ui.small(tf!("Audio: {} to {} / {:.3} s", time(extra.start), time(extra.start + extra.length.unwrap_or(extra.duration-extra.start)), extra.length.unwrap_or(extra.duration-extra.start)));
                if ui.button(tr!("Trim this audio…")).clicked() { let extra=extra.clone(); self.stop(); self.audio_editor=Some(audio_trim::Editor::new(extra,self.length,ui.ctx())); }
            } else { ui.colored_label(palette().mint, tr!("Choose an audio file first.")); }
            ui.small(tr!("Completely replaces the original audio. Only the selected audio clip is used."));
        }
        if ui.button(tr!("Normalize alert volume…")).clicked() { self.show_sound = true; }
        if self.video_options.mode != previous { self.stop(); }
    }
    fn external_editor_ui(&mut self, ctx: &egui::Context) {
        if let Some(mut editor) = self.audio_editor.take() {
            match editor.show(ctx) {
                audio_trim::Outcome::Editing => self.audio_editor=Some(editor),
                audio_trim::Outcome::Apply(extra) => {
                    self.stop(); self.video_options.mode=video::SoundMode::Replace;
                    self.status=tf!("Audio trimmed: {} to {}. This will replace all original video audio.",time(extra.start),time(extra.start+extra.length.unwrap_or(extra.duration-extra.start)));
                    self.video_options.external=Some(extra); self.error=false;
                },
                audio_trim::Outcome::Cancel => (),
            }
        }
    }

    fn sound_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_sound || self.audio_editor.is_some() { return; }
        let mut open = true; let busy = self.receiver.is_some(); let ready = self.audio.is_some() && !busy;
        egui::Window::new(tr!("Twitch alert volume")).open(&mut open).default_width(570.0).max_height(720.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new(tr!("Match your alert volumes")).size(23.0).strong());
            ui.label(tr!("You control the adjustment. Measure, adjust the volume, and preview before saving."));
            ui.add_space(8.0);
            ui.add_enabled_ui(ready, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr!("1. Measure this clip")).clicked() { self.analyze_sound(ctx); }
                    if ui.button(tr!("Preview result")).clicked() { self.play(false); }
                    if ui.button(tr!("Stop")).clicked() { self.stop(); }
                });
                ui.separator();
                ui.label(RichText::new(tr!("2. Adjust the output volume")).size(18.0).strong());
                ui.small(tr!("Left: quieter / Center: original level / Right: louder"));
                let mut gain = if self.video_mode { self.video_options.master_db } else { self.audio_gain_db };
                let previous = gain;
                ui.add(egui::Slider::new(&mut gain, -30.0..=18.0).text(tr!("Adjustment / dB")).fixed_decimals(1));
                ui.horizontal(|ui| {
                    if ui.button("-1 dB").clicked() { gain = (gain - 1.0).max(-30.0); }
                    ui.label(RichText::new(format!("{gain:+.1} dB")).monospace().size(27.0).color(palette().accent));
                    if ui.button("+1 dB").clicked() { gain = (gain + 1.0).min(18.0); }
                    if ui.button(tr!("Reset")).clicked() { gain = 0.0; }
                });
                if gain != previous { self.stop(); if self.video_mode { self.video_options.master_db = gain; } else { self.audio_gain_db = gain; } }
                ui.separator();
                let valid = self.level_report.as_ref().is_some_and(|(key, _)| *key == self.measurement_key());
                if let Some((_, report)) = &self.level_report {
                    if !valid { ui.colored_label(palette().muted, tr!("The selection or volume has changed. Measure again to update the values.")); }
                    if let Some(level) = report.loudness {
                        ui.label(RichText::new(tf!("Perceived loudness: {level:.1} LUFS")).size(17.0));
                        ui.add(egui::ProgressBar::new(((level + 50.0) / 50.0).clamp(0.0, 1.0) as f32).fill(palette().accent).text(tr!("Quiet                                              Loud")));
                    } else { ui.label(tr!("This clip is silent or too short to measure perceived loudness.")); }
                    if let Some(peak) = report.peak {
                        ui.label(tf!("Highest peak: {peak:.1} dBTP"));
                        ui.add(egui::ProgressBar::new(((peak + 60.0) / 60.0).clamp(0.0, 1.0) as f32).fill(if peak > -1.0 { Color32::from_rgb(245,151,135) } else { palette().mint }).text(tr!("Peak headroom")));
                    }
                } else { ui.label(tr!("Click Measure this clip to see its loudness and true peaks.")); }
                ui.add_space(8.0);
                egui::CollapsingHeader::new(tr!("Reference for matching multiple alerts")).default_open(true).show(ui, |ui| {
                    ui.label(tr!("Use the same target to compare your alerts. This is a personal reference, not a Twitch requirement."));
                    ui.add(egui::Slider::new(&mut self.target_lufs, -30.0..=-9.0).text(tr!("Target / LUFS")).fixed_decimals(1));
                    if valid {
                        if let Some((_, report)) = &self.level_report {
                            if let Some(level) = report.loudness {
                                let difference = self.target_lufs - level;
                                ui.label(tf!("Adjustment toward your target: {difference:+.1} dB."));
                                if report.peak.is_some_and(|peak| peak + difference > -0.45) { ui.small(tr!("This increase may activate peak protection. Preview the result and measure again.")); }
                                if ui.button(tr!("Apply this difference as a manual adjustment")).clicked() {
                                    self.stop(); let next = (gain + difference as f32).clamp(-30.0, 18.0);
                                    if self.video_mode { self.video_options.master_db = next; } else { self.audio_gain_db = next; }
                                }
                            }
                        }
                    }
                });
                if self.video_mode {
                    egui::CollapsingHeader::new(tr!("Source volume (advanced)")).show(ui, |ui| {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut self.video_options.original_db, -60.0..=12.0).text("Original / dB").fixed_decimals(1)).changed();
                        if let Some(extra) = &mut self.video_options.external { changed |= ui.add(egui::Slider::new(&mut extra.db, -60.0..=12.0).text(tr!("Local audio / dB")).fixed_decimals(1)).changed(); }
                        if changed { self.stop(); }
                    });
                }
                ui.separator(); ui.small(tr!("3. Once you like the sound, close this window and save the clip. The adjustment also applies to the exported file."));
            });
            if busy { ui.horizontal(|ui| { ui.spinner(); ui.label(tr!("Measuring or processing…")); if ui.button(tr!("Cancel")).clicked() { self.cancel.store(true, Ordering::Relaxed); } }); }
            if self.error { ui.colored_label(Color32::from_rgb(255,151,155), &self.status); }
        });
        self.show_sound = open;
    }

    fn video_sound_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        let has_audio = self.audio.as_ref().and_then(|a| a.video.as_ref()).is_some_and(|v| v.has_audio);
        card(ui, tr!("WebM audio"), tr!("Original audio is muted by default"), |ui| { ui.add_enabled_ui(!busy, |ui| self.video_sound_controls(ui, has_audio)); });
    }

    fn themes_ui(&mut self, ctx: &egui::Context) {
        if !self.show_themes { return; }
        let mut open = true; let mut changed = false;
        egui::Window::new(tr!("Customize appearance")).open(&mut open).resizable(false).default_width(380.0).show(ctx, |ui| {
            ui.label(tr!("Dark themes"));
            ui.horizontal_wrapped(|ui| {
                for (name, theme) in [(tr!("Night"), theme::Theme::Night), ("OLED", theme::Theme::Oled), (tr!("Ocean"), theme::Theme::Ocean), (tr!("Rose"), theme::Theme::Rose), (tr!("Forest"), theme::Theme::Forest)] {
                    if ui.selectable_value(&mut self.appearance.theme, theme, name).changed() { self.appearance.accent = None; changed = true; }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(tr!("Accent color:"));
                let color = self.appearance.palette().accent;
                let mut rgb = [color.r(), color.g(), color.b()];
                if ui.color_edit_button_srgb(&mut rgb).changed() { self.appearance.accent = Some(rgb); changed = true; }
            });
            if ui.button(tr!("Reset theme accent")).clicked() { self.appearance.accent = None; changed = true; }
            changed |= ui.add(egui::Slider::new(&mut self.appearance.scale, 0.85..=1.3).text(tr!("Interface scale")).fixed_decimals(2)).changed();
            ui.small(tr!("Your preferences are saved for the next time you open the app."));
        });
        self.show_themes = open;
        if changed {
            configure(ctx, &self.appearance);
            let result = serde_json::to_vec_pretty(&self.appearance).map_err(|e| e.to_string()).and_then(|bytes| std::fs::write(self.data.join("apariencia.json"), bytes).map_err(|e| e.to_string()));
            if let Err(e) = result { self.message(Err(tf!("Could not save the theme: {e}"))); }
        }
    }

    fn video_preview_ui(&mut self, ui: &mut egui::Ui) {
        let info = self.audio.as_ref().and_then(|a| a.video.as_ref()).unwrap().clone();
        card(ui, tr!("Video preview"), if self.player.is_some() { tr!("Playing inside Shiny Twitch Creator Tool") } else { tr!("Click Play to preview the video here") }, |ui| {
            if let Some(texture) = &self.frame_texture {
                let size = texture.size_vec2();
                let scale = (ui.available_width() / size.x).min(300.0 / size.y).min(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - size.x * scale) / 2.0).max(0.0));
                    ui.image((texture.id(), size * scale));
                });
            } else if self.frame_receiver.is_some() { ui.spinner(); }
            if !self.frame_error.is_empty() { ui.label(RichText::new(&self.frame_error).small().color(palette().muted)); }
            ui.label(RichText::new(format!("{} x {} / {:.2} fps / {}{}", info.width, info.height, info.fps, if info.has_audio { tr!("With audio") } else { tr!("Without audio") }, if info.alpha { tr!(" / Transparency detected") } else { "" })).small().color(palette().muted));
            ui.small(tr!("Video cuts align to frames; millisecond steps are still available."));
        });
    }

    fn video_export_ui(&mut self, ctx: &egui::Context) {
        if !self.show_video_export || !self.video_mode || self.receiver.is_some() || self.audio_editor.is_some() { return; }
        let Some(audio) = &self.audio else { return; };
        let Some(info) = audio.video.clone() else { return; };
        let length = if self.convert_full { audio.duration } else { self.length };
        let mut open = true; let mut save = false;
        egui::Window::new(tr!("Save WebM video")).open(&mut open).resizable(false).default_width(460.0).max_height(650.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.heading(if self.convert_full { tr!("Convert entire file") } else { tr!("Export selection") });
            ui.label(tf!("Output duration: {length:.3} seconds"));
            ui.separator();
            ui.label(tr!("Resolution / maximum height"));
            egui::ComboBox::from_id_salt("output-resolution").selected_text(if self.video_options.height == 0 { "Original".into() } else { format!("{}p", self.video_options.height) }).show_ui(ui, |ui| {
                for (label, height) in [("Original", 0), ("2160p / 4K", 2160), ("1440p", 1440), ("1080p / Full HD", 1080), ("720p / HD", 720), ("480p", 480), ("360p", 360)] { ui.selectable_value(&mut self.video_options.height, height, label); }
            });
            let (width, height) = video::output_size(&info, self.video_options.height);
            ui.label(RichText::new(tf!("Output: {width} x {height}. Keeps the aspect ratio without upscaling.")).small().color(palette().muted));
            ui.add_space(4.0); ui.label(tr!("Quality and file size"));
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.video_options.quality, video::Quality::Light, tr!("Smaller file"));
                ui.selectable_value(&mut self.video_options.quality, video::Quality::Balanced, tr!("Balanced"));
                ui.selectable_value(&mut self.video_options.quality, video::Quality::High, tr!("Higher quality"));
            });
            ui.checkbox(&mut self.video_options.limit_fps, tr!("Limit to 30 fps when the source frame rate is higher"));
            egui::CollapsingHeader::new(tr!("WebM audio / source and volume")).default_open(true).show(ui, |ui| self.video_sound_controls(ui, info.has_audio));
            ui.small(tr!("WebM VP9 uses efficient compression. File size depends on motion, resolution, and duration; higher quality usually means a larger file."));
            if info.alpha { ui.small(tr!("The app will try to preserve transparency from the original GIF or video.")); }
            ui.add_space(8.0);
            if ui.add_sized([ui.available_width(), 38.0], egui::Button::new(tr!("Choose location and save WebM…")).fill(palette().accent.gamma_multiply(0.5))).clicked() { save = true; }
        });
        self.show_video_export = open;
        if save { self.show_video_export = false; self.save(ctx); }
    }

    fn sources_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
        card(ui, tr!("Your library"), if self.video_mode { tr!("Local video / GIF or YouTube") } else { tr!("Local audio or a YouTube link") }, |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                if ui.add_sized([ui.available_width(), 40.0], egui::Button::new(if self.video_mode { tr!("Open video or GIF") } else { tr!("Open audio or M3U playlist") }).fill(palette().accent.gamma_multiply(0.4))).clicked() {
                    let dialog = if self.video_mode { rfd::FileDialog::new().add_filter(tr!("Videos and GIFs"), &["mp4", "mkv", "webm", "gif", "mov", "avi", "m4v", "flv", "wmv", "mpeg", "mpg", "ts", "ogv"]).add_filter(tr!("All files"), &["*"]) } else { rfd::FileDialog::new().add_filter(tr!("Audio and playlists"), &["wav", "mp3", "m3u", "m3u8", "flac", "ogg", "m4a"]) };
                    if let Some(path) = dialog.pick_file() {
                        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                        if !self.video_mode && (ext == "m3u" || ext == "m3u8") {
                            match media::playlist(&path) { Ok(tracks) => { self.tracks = tracks; self.track_index = 0; self.load(self.tracks[0].clone(), ctx); }, Err(e) => self.message(Err(e)) }
                        } else { self.tracks.clear(); self.load(path, ctx); }
                    }
                }
                ui.label(RichText::new(if self.video_mode { "MP4 / GIF / WEBM / MKV / MOV / AVI…" } else { "WAV / MP3 / M3U / FLAC / OGG / M4A" }).small().color(palette().muted));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut self.url).hint_text(tr!("Paste a YouTube link")).desired_width(ui.available_width()));
                if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(if self.video_mode { tr!("Download YouTube video") } else { tr!("Download audio only") })).clicked() { self.download(ctx); }
                if !self.tracks.is_empty() {
                    let previous = self.track_index;
                    egui::ComboBox::from_id_salt("playlist").width(ui.available_width()).selected_text(self.tracks[self.track_index].file_name().unwrap_or_default().to_string_lossy()).show_ui(ui, |ui| {
                        for (index, track) in self.tracks.iter().enumerate() { ui.selectable_value(&mut self.track_index, index, track.file_name().unwrap_or_default().to_string_lossy()); }
                    });
                    if previous != self.track_index { self.load(self.tracks[self.track_index].clone(), ctx); }
                }
            });
        });
    }

    fn presets_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        card(ui, tr!("Quick durations"), tr!("Choose a preset and fine-tune it"), |ui| {
            ui.add_enabled_ui(!busy && self.audio.is_some(), |ui| {
                ui.label(RichText::new(tr!("TWITCH ALERT")).small().color(palette().accent));
                ui.horizontal(|ui| { for seconds in [9.0, 11.0] {
                    if ui.add(egui::Button::new(tf!("{seconds:.0} seconds")).selected(self.preset == Some(seconds))).clicked() { self.apply_preset(seconds); }
                } });
                ui.add_space(4.0);
                if ui.add_sized([ui.available_width(), 36.0], egui::Button::new(tr!("Ringtone / 30 seconds")).selected(self.preset == Some(30.0))).clicked() { self.apply_preset(30.0); }
                ui.add_space(4.0);
                ui.label(RichText::new(tr!("NOTIFICATION")).small().color(palette().mint));
                ui.horizontal_wrapped(|ui| { for (label, seconds) in [("1 s", 1.0), ("1.5 s", 1.5), ("3 s", 3.0), ("4 s", 4.0), ("5 s", 5.0)] {
                    if ui.add(egui::Button::new(label).selected(self.preset == Some(seconds))).clicked() { self.apply_preset(seconds); }
                } });
                if self.preset.is_none() { ui.label(RichText::new(tr!("Custom duration selected")).small().color(palette().muted)); }
            });
        });
    }

    fn selection_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        let duration = self.audio.as_ref().unwrap().duration;
        card(ui, tr!("02 / Your selection"), tr!("Enter times or mark points during playback"), |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                let min = editor::MIN_LENGTH.min(duration);
                let mut start = self.start; let mut end = self.start + self.length; let mut length = self.length;
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr!("START / seconds")).small().color(palette().accent));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut start).speed(self.step).range(0.0..=(end - min).max(0.0)).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_start(self.start, self.length, start, duration); self.preset = None; self.view.follow(self.start, duration);
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr!("END / seconds")).small().color(palette().mint));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut end).speed(self.step).range((self.start + min).min(duration)..=duration).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_end(self.start, end, duration); self.preset = None; self.view.follow(self.start + self.length, duration);
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label(RichText::new(tr!("DURATION / seconds")).small().color(palette().muted));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut length).speed(self.step).range(min..=(duration - self.start).max(min)).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_end(self.start, self.start + length, duration); self.preset = None;
                        }
                    });
                });
                if self.preset.is_some_and(|preset| duration < preset) { ui.colored_label(Color32::from_rgb(244, 194, 115), tr!("The track is shorter, so its full duration has been selected.")); }
                let mut position = self.start;
                if ui.add(egui::Slider::new(&mut position, 0.0..=(duration - self.length).max(0.0)).text(tr!("Move entire selection")).show_value(false)).changed() { self.stop(); self.start = position; self.normalize(); self.view.follow(self.start, duration); }
            });
        });
    }

    fn precision_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        card(ui, tr!("03 / Fine adjustments"), tr!("Extend or shorten the clip without moving its start"), |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                for (label, sign) in [(tr!("Add"), 1.0), (tr!("Remove"), -1.0)] {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(label).color(if sign > 0.0 { palette().mint } else { palette().muted }));
                        for (name, seconds) in [("1 ms", 0.001), ("5 ms", 0.005), ("100 ms", 0.100), ("500 ms", 0.500), ("1 min", 60.0)] {
                            if ui.button(format!("{} {name}", if sign > 0.0 { "+" } else { "-" })).clicked() { self.change_duration(sign * seconds); }
                        }
                    });
                }
                ui.add_space(2.0);
                egui::CollapsingHeader::new(tr!("Step controls / start, end, or selection")).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(tr!("Step:"));
                        for (label, seconds) in [("1 ms", 0.001), ("5 ms", 0.005), ("100 ms", 0.100), ("500 ms", 0.500), ("1 min", 60.0)] { ui.selectable_value(&mut self.step, seconds, label); }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.target, Target::Start, tr!("Start")); ui.selectable_value(&mut self.target, Target::End, tr!("End")); ui.selectable_value(&mut self.target, Target::Selection, tr!("Selection"));
                        if ui.button(tr!("- Backward")).clicked() { self.nudge(-self.step); }
                        if ui.button(tr!("+ Forward")).clicked() { self.nudge(self.step); }
                    });
                });
            });
        });
    }

    fn history_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
        card(ui, tr!("Recent clips"), tr!("Saved between sessions"), |ui| {
            if self.history.is_empty() { ui.label(RichText::new(tr!("No clips yet. Your next alert will appear here.")).color(palette().muted)); }
            let mut open = None; let mut reload = None;
            egui::ScrollArea::vertical().id_salt("history-scroll").max_height(240.0).show(ui, |ui| {
                for entry in &self.history {
                    ui.label(RichText::new(entry.output.file_name().unwrap_or_default().to_string_lossy()).strong()).on_hover_text(entry.output.display().to_string());
                    ui.label(RichText::new(format!("{} / {:.3} s / {}", entry.date, entry.length, if entry.video { "Video" } else { "Audio" })).small().color(palette().muted));
                    ui.horizontal(|ui| {
                        if ui.small_button(tr!("Show folder")).clicked() { open = Some(entry.output.clone()); }
                        if ui.add_enabled(!busy, egui::Button::new(tr!("Open source")).small()).clicked() { reload = Some((entry.source.clone(), entry.video)); }
                    });
                    ui.separator();
                }
            });
            if let Some(path) = open { self.message(reveal(&path)); }
            if let Some((path, video)) = reload { self.switch_mode(video); self.load(path, ctx); }
        });
    }

    fn transport_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
        let ready = self.audio.is_some() && !busy;
        ui.horizontal_wrapped(|ui| {
            ui.add_enabled_ui(ready, |ui| {
                if ui.button(tr!("Normalize volume…")).clicked() { self.show_sound = true; }
                if ui.add(egui::Button::new(if self.video_mode { tr!("Play selection") } else { tr!("Preview selection") }).fill(palette().accent.gamma_multiply(0.4))).clicked() { self.play(false); }
                if ui.button(if self.video_mode { tr!("Play from here") } else { tr!("Preview from here") }).clicked() { self.play(true); }
                if ui.button(tr!("Stop")).clicked() { self.stop(); }
            });
            ui.add_space(12.0);
            ui.label(RichText::new(time(self.playhead)).monospace().size(22.0).color(if self.player.is_some() { palette().mint } else { palette().muted }));
            let can_mark = ready && self.player.as_ref().and_then(|p| p.position()).is_some();
            ui.add_enabled_ui(can_mark, |ui| {
                if ui.add(egui::Button::new(RichText::new(tr!("Mark start")).color(palette().accent)).stroke(Stroke::new(1.0_f32, palette().accent))).on_hover_text(tr!("Mark the current playback position. Playback continues.")).clicked() { self.mark_playback(false); }
                if ui.add(egui::Button::new(RichText::new(tr!("Mark end")).color(palette().mint)).stroke(Stroke::new(1.0_f32, palette().mint))).on_hover_text(tr!("End your selection at the current playback position.")).clicked() { self.mark_playback(true); }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tf!("{}  to  {}", time(self.start), time(self.start + self.length))).color(palette().muted));
            ui.label(RichText::new(format!("{:.3} s", self.length)).strong().color(palette().accent));
            ui.separator();
            ui.add_enabled_ui(ready, |ui| {
                if self.video_mode {
                    if ui.add(egui::Button::new(tr!("Save WebM clip…")).fill(palette().accent.gamma_multiply(0.5))).clicked() { self.convert_full = false; self.show_video_export = true; }
                    if ui.button(tr!("Convert entire file to WebM…")).clicked() { self.convert_full = true; self.show_video_export = true; }
                } else {
                    ui.selectable_value(&mut self.format_mp3, false, "WAV"); ui.selectable_value(&mut self.format_mp3, true, "MP3");
                    if ui.add(egui::Button::new(RichText::new(tr!("Trim and save…")).strong()).fill(palette().accent.gamma_multiply(0.5))).clicked() { self.save(ctx); }
                }
            });
            if let Some(output) = self.last_output.clone() { if ui.button(tr!("Show latest clip")).on_hover_text(output.display().to_string()).clicked() { self.message(reveal(&output)); } }
            if busy && self.receiver.is_some() { ui.spinner(); if ui.small_button(tr!("Cancel operation")).clicked() { self.cancel.store(true, Ordering::Relaxed); self.status = tr!("Canceling…").into(); } }
        });
        ui.add(egui::Label::new(RichText::new(&self.status).small().color(if self.error { Color32::from_rgb(255, 151, 155) } else { palette().muted })).truncate()).on_hover_text(&self.status);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.poll();
        self.poll_frame(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && self.receiver.is_some() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose); self.cancel.store(true, Ordering::Relaxed);
            self.status = tr!("Canceling the operation. You can close the app when it finishes…").into();
        }
        if self.receiver.is_some() || self.player.is_some() || self.frame_receiver.is_some() { ctx.request_repaint_after(Duration::from_millis(30)); }
        let busy = self.receiver.is_some() || self.audio_editor.is_some();
        egui::TopBottomPanel::top("brand").frame(egui::Frame::none().fill(palette().bg).inner_margin(egui::Margin::symmetric(22.0, 15.0))).show(ctx, |ui| {
            ui.horizontal(|ui| {
                let (logo, _) = ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::hover());
                ui.painter().rect_filled(logo, 12.0, palette().accent.gamma_multiply(0.3));
                for (i, height) in [10.0, 23.0, 30.0, 17.0].into_iter().enumerate() {
                    let x = logo.left() + 10.0 + i as f32 * 7.0;
                    ui.painter().line_segment([egui::pos2(x, logo.center().y - height / 2.0), egui::pos2(x, logo.center().y + height / 2.0)], Stroke::new(3.0_f32, palette().accent));
                }
                ui.vertical(|ui| { ui.label(RichText::new("Shiny Twitch Creator Tool").size(25.0).strong()); ui.label(RichText::new(tr!("Find the moment. Make it sound like you.")).small().color(palette().muted)); });
                ui.add_space(20.0);
                ui.add_enabled_ui(!busy, |ui| {
                    if ui.selectable_label(!self.video_mode, RichText::new("Audio").size(17.0)).clicked() { self.switch_mode(false); }
                    if ui.selectable_label(self.video_mode, RichText::new("Video / GIF").size(17.0)).clicked() { self.switch_mode(true); }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr!("Options")).clicked() { self.show_options = true; }
                    ui.label(RichText::new("STUDIO / 1.2").small().color(palette().accent));
                });
            });
        });
        egui::TopBottomPanel::bottom("transport").frame(egui::Frame::none().fill(palette().card).stroke(Stroke::new(1.0_f32, palette().border)).inner_margin(egui::Margin::symmetric(20.0, 12.0))).show(ctx, |ui| self.transport_ui(ui, ctx, busy));
        egui::SidePanel::right("library").resizable(true).default_width(300.0).width_range(270.0..=380.0).frame(egui::Frame::none().fill(palette().bg).inner_margin(egui::Margin::symmetric(12.0, 8.0))).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("library-scroll").show(ui, |ui| {
                self.sources_ui(ui, ctx, busy); ui.add_space(8.0);
                if self.video_mode { self.video_sound_ui(ui, busy); ui.add_space(8.0); }
                self.presets_ui(ui, busy); ui.add_space(8.0);
                self.history_ui(ui, ctx, busy); ui.add_space(8.0);
                ui.label(RichText::new(tr!("Local data and downloads")).small().color(palette().muted)).on_hover_text(self.data.display().to_string());
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::none().fill(palette().bg).inner_margin(egui::Margin::symmetric(20.0, 8.0))).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("editor-scroll").show(ui, |ui| {
                if let Some(audio) = &self.audio {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(audio.path.file_name().unwrap_or_default().to_string_lossy()).size(18.0).strong());
                        ui.label(RichText::new(tf!("TRACK / {}", time(audio.duration))).small().color(palette().muted));
                    });
                    ui.add_space(8.0);
                    if self.video_mode { self.video_preview_ui(ui); ui.add_space(8.0); }
                    card(ui, if self.video_mode { tr!("01 / Video timeline") } else { tr!("01 / Explore your track") }, tr!("Drag the edges to resize the selection, or drag inside it to move it"), |ui| { ui.add_enabled_ui(!busy, |ui| self.waveform(ui)); });
                    ui.add_space(8.0); self.selection_ui(ui, busy);
                    ui.add_space(8.0); self.precision_ui(ui, busy);
                } else {
                    ui.add_space(65.0);
                    card(ui, tr!("Your next alert starts here"), tr!("Open a file from the panel on the right to get started"), |ui| {
                        ui.add_space(18.0);
                        ui.label(RichText::new(if self.video_mode { tr!("01   Open a video or GIF") } else { tr!("01   Open your audio") }).size(20.0).color(palette().accent));
                        ui.label(RichText::new(tr!("02   Play and mark the start / end")).size(20.0).color(palette().text));
                        ui.label(RichText::new(tr!("03   Fine-tune and save your clip")).size(20.0).color(palette().mint));
                        ui.add_space(18.0);
                        ui.label(RichText::new(tr!("Twitch alerts, ringtones, and notifications in one place.")).color(palette().muted));
                    });
                }
            });
        });
        self.options_ui(ctx);
        self.themes_ui(ctx);
        self.video_export_ui(ctx);
        self.sound_window_ui(ctx);
        self.external_editor_ui(ctx);
    }
}
