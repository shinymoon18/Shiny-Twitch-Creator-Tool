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
    fn video_sound_controls(&mut self, ui: &mut egui::Ui, has_audio: bool) {
        use video::SoundMode;
        let previous = self.video_options.mode;
        ui.label(RichText::new("¿Qué sonido quieres guardar?").strong());
        ui.add_enabled_ui(has_audio, |ui| { ui.radio_value(&mut self.video_options.mode, SoundMode::Original, "Mantener audio original"); });
        ui.radio_value(&mut self.video_options.mode, SoundMode::Silent, "Cortar sin audio");
        ui.radio_value(&mut self.video_options.mode, SoundMode::Replace, "Reemplazar con archivo local");
        if !has_audio { ui.small("Este video no tiene sonido original."); }
        if self.video_options.mode == SoundMode::Replace {
            if ui.button("Elegir archivo de audio…").clicked() {
                if let Some(path) = rfd::FileDialog::new().add_filter("Audio", &["wav", "mp3", "flac", "ogg", "m4a", "aac", "opus", "webm"]).pick_file() {
                    match video::external_audio(path) { Ok(extra) => { self.stop(); self.audio_editor = Some(audio_trim::Editor::new(extra, self.length, ui.ctx())); }, Err(e) => self.message(Err(e)) }
                }
            }
            if let Some(extra) = &self.video_options.external {
                ui.label(RichText::new(extra.path.file_name().unwrap_or_default().to_string_lossy()).strong());
                ui.small(format!("Audio: {} hasta {} / {:.3} s", time(extra.start), time(extra.start + extra.length.unwrap_or(extra.duration-extra.start)), extra.length.unwrap_or(extra.duration-extra.start)));
                if ui.button("Recortar este audio…").clicked() { let extra=extra.clone(); self.stop(); self.audio_editor=Some(audio_trim::Editor::new(extra,self.length,ui.ctx())); }
            } else { ui.colored_label(palette().mint, "Falta elegir tu archivo de audio."); }
            ui.small("Reemplaza por completo el sonido original. Solo se usa el fragmento de audio elegido.");
        }
        if ui.button("Normalizar volumen de la alerta…").clicked() { self.show_sound = true; }
        if self.video_options.mode != previous { self.stop(); }
    }
    fn external_editor_ui(&mut self, ctx: &egui::Context) {
        if let Some(mut editor) = self.audio_editor.take() {
            match editor.show(ctx) {
                audio_trim::Outcome::Editing => self.audio_editor=Some(editor),
                audio_trim::Outcome::Apply(extra) => {
                    self.stop(); self.video_options.mode=video::SoundMode::Replace;
                    self.status=format!("Audio recortado: {} hasta {}. Se sustituirá todo el sonido original del video.",time(extra.start),time(extra.start+extra.length.unwrap_or(extra.duration-extra.start)));
                    self.video_options.external=Some(extra); self.error=false;
                },
                audio_trim::Outcome::Cancel => (),
            }
        }
    }

    fn sound_window_ui(&mut self, ctx: &egui::Context) {
        if !self.show_sound || self.audio_editor.is_some() { return; }
        let mut open = true; let busy = self.receiver.is_some(); let ready = self.audio.is_some() && !busy;
        egui::Window::new("Volumen de alertas de Twitch").open(&mut open).default_width(570.0).max_height(720.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.label(RichText::new("Iguala tus alertas a tu gusto").size(23.0).strong());
            ui.label("Tú controlas el ajuste. Mide, cambia el volumen y escucha antes de guardar.");
            ui.add_space(8.0);
            ui.add_enabled_ui(ready, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("1. Medir este corte").clicked() { self.analyze_sound(ctx); }
                    if ui.button("Escuchar resultado").clicked() { self.play(false); }
                    if ui.button("Detener").clicked() { self.stop(); }
                });
                ui.separator();
                ui.label(RichText::new("2. Ajusta el volumen final").size(18.0).strong());
                ui.small("Izquierda: más suave / Centro: nivel original / Derecha: más fuerte");
                let mut gain = if self.video_mode { self.video_options.master_db } else { self.audio_gain_db };
                let previous = gain;
                ui.add(egui::Slider::new(&mut gain, -30.0..=18.0).text("Ajuste / dB").fixed_decimals(1));
                ui.horizontal(|ui| {
                    if ui.button("-1 dB").clicked() { gain = (gain - 1.0).max(-30.0); }
                    ui.label(RichText::new(format!("{gain:+.1} dB")).monospace().size(27.0).color(palette().accent));
                    if ui.button("+1 dB").clicked() { gain = (gain + 1.0).min(18.0); }
                    if ui.button("Restablecer").clicked() { gain = 0.0; }
                });
                if gain != previous { self.stop(); if self.video_mode { self.video_options.master_db = gain; } else { self.audio_gain_db = gain; } }
                ui.separator();
                let valid = self.level_report.as_ref().is_some_and(|(key, _)| *key == self.measurement_key());
                if let Some((_, report)) = &self.level_report {
                    if !valid { ui.colored_label(palette().muted, "Cambió el corte o el volumen. Vuelve a medir para actualizar los valores."); }
                    if let Some(level) = report.loudness {
                        ui.label(RichText::new(format!("Volumen percibido: {level:.1} LUFS")).size(17.0));
                        ui.add(egui::ProgressBar::new(((level + 50.0) / 50.0).clamp(0.0, 1.0) as f32).fill(palette().accent).text("Suave                                              Fuerte"));
                    } else { ui.label("El corte es silencioso o demasiado corto para medir volumen percibido."); }
                    if let Some(peak) = report.peak {
                        ui.label(format!("Pico más alto: {peak:.1} dBTP"));
                        ui.add(egui::ProgressBar::new(((peak + 60.0) / 60.0).clamp(0.0, 1.0) as f32).fill(if peak > -1.0 { Color32::from_rgb(245,151,135) } else { palette().mint }).text("Margen de los picos"));
                    }
                } else { ui.label("Pulsa Medir este corte para ver el volumen y sus picos reales."); }
                ui.add_space(8.0);
                egui::CollapsingHeader::new("Referencia para igualar varias alertas").default_open(true).show(ui, |ui| {
                    ui.label("Elige el mismo objetivo para comparar tus alertas. Es una referencia personal, no un requisito de Twitch.");
                    ui.add(egui::Slider::new(&mut self.target_lufs, -30.0..=-9.0).text("Objetivo / LUFS").fixed_decimals(1));
                    if valid {
                        if let Some((_, report)) = &self.level_report {
                            if let Some(level) = report.loudness {
                                let difference = self.target_lufs - level;
                                ui.label(format!("Para acercarte a tu objetivo: {difference:+.1} dB."));
                                if report.peak.is_some_and(|peak| peak + difference > -0.45) { ui.small("Ese aumento puede activar la protección de picos. Escucha el resultado y vuelve a medir."); }
                                if ui.button("Usar esta diferencia como ajuste manual").clicked() {
                                    self.stop(); let next = (gain + difference as f32).clamp(-30.0, 18.0);
                                    if self.video_mode { self.video_options.master_db = next; } else { self.audio_gain_db = next; }
                                }
                            }
                        }
                    }
                });
                if self.video_mode {
                    egui::CollapsingHeader::new("Volumen de cada fuente (avanzado)").show(ui, |ui| {
                        let mut changed = false;
                        changed |= ui.add(egui::Slider::new(&mut self.video_options.original_db, -60.0..=12.0).text("Original / dB").fixed_decimals(1)).changed();
                        if let Some(extra) = &mut self.video_options.external { changed |= ui.add(egui::Slider::new(&mut extra.db, -60.0..=12.0).text("Archivo local / dB").fixed_decimals(1)).changed(); }
                        if changed { self.stop(); }
                    });
                }
                ui.separator(); ui.small("3. Cuando te guste cómo suena, cierra esta ventana y guarda el corte. El ajuste se aplica también al archivo exportado.");
            });
            if busy { ui.horizontal(|ui| { ui.spinner(); ui.label("Midiendo o procesando…"); if ui.button("Cancelar").clicked() { self.cancel.store(true, Ordering::Relaxed); } }); }
            if self.error { ui.colored_label(Color32::from_rgb(255,151,155), &self.status); }
        });
        self.show_sound = open;
    }

    fn video_sound_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        let has_audio = self.audio.as_ref().and_then(|a| a.video.as_ref()).is_some_and(|v| v.has_audio);
        card(ui, "Audio del WebM", "Sin sonido original por defecto", |ui| { ui.add_enabled_ui(!busy, |ui| self.video_sound_controls(ui, has_audio)); });
    }

    fn themes_ui(&mut self, ctx: &egui::Context) {
        if !self.show_themes { return; }
        let mut open = true; let mut changed = false;
        egui::Window::new("Personalizar interfaz").open(&mut open).resizable(false).default_width(380.0).show(ctx, |ui| {
            ui.label("Temas oscuros");
            ui.horizontal_wrapped(|ui| {
                for (name, theme) in [("Noche", theme::Theme::Night), ("OLED", theme::Theme::Oled), ("Océano", theme::Theme::Ocean), ("Rosa", theme::Theme::Rose), ("Bosque", theme::Theme::Forest)] {
                    if ui.selectable_value(&mut self.appearance.theme, theme, name).changed() { self.appearance.accent = None; changed = true; }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Color de acento:");
                let color = self.appearance.palette().accent;
                let mut rgb = [color.r(), color.g(), color.b()];
                if ui.color_edit_button_srgb(&mut rgb).changed() { self.appearance.accent = Some(rgb); changed = true; }
            });
            if ui.button("Restablecer acento del tema").clicked() { self.appearance.accent = None; changed = true; }
            changed |= ui.add(egui::Slider::new(&mut self.appearance.scale, 0.85..=1.3).text("Tamaño de interfaz").fixed_decimals(2)).changed();
            ui.small("Tu elección se guarda para la próxima vez que abras el programa.");
        });
        self.show_themes = open;
        if changed {
            configure(ctx, &self.appearance);
            let result = serde_json::to_vec_pretty(&self.appearance).map_err(|e| e.to_string()).and_then(|bytes| std::fs::write(self.data.join("apariencia.json"), bytes).map_err(|e| e.to_string()));
            if let Err(e) = result { self.message(Err(format!("No se pudo guardar el tema: {e}"))); }
        }
    }

    fn video_preview_ui(&mut self, ui: &mut egui::Ui) {
        let info = self.audio.as_ref().and_then(|a| a.video.as_ref()).unwrap().clone();
        card(ui, "Vista previa de video", if self.player.is_some() { "Reproduciendo dentro de Shiny Twitch Creator Tool" } else { "Pulsa Reproducir para ver el video aquí mismo" }, |ui| {
            if let Some(texture) = &self.frame_texture {
                let size = texture.size_vec2();
                let scale = (ui.available_width() / size.x).min(300.0 / size.y).min(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(((ui.available_width() - size.x * scale) / 2.0).max(0.0));
                    ui.image((texture.id(), size * scale));
                });
            } else if self.frame_receiver.is_some() { ui.spinner(); }
            if !self.frame_error.is_empty() { ui.label(RichText::new(&self.frame_error).small().color(palette().muted)); }
            ui.label(RichText::new(format!("{} x {} / {:.2} fps / {}{}", info.width, info.height, info.fps, if info.has_audio { "Con audio" } else { "Sin audio" }, if info.alpha { " / Transparencia detectada" } else { "" })).small().color(palette().muted));
            ui.small("Los cortes de video se ajustan a sus fotogramas; los pasos de milisegundos siguen disponibles.");
        });
    }

    fn video_export_ui(&mut self, ctx: &egui::Context) {
        if !self.show_video_export || !self.video_mode || self.receiver.is_some() || self.audio_editor.is_some() { return; }
        let Some(audio) = &self.audio else { return; };
        let Some(info) = audio.video.clone() else { return; };
        let length = if self.convert_full { audio.duration } else { self.length };
        let mut open = true; let mut save = false;
        egui::Window::new("Guardar video WebM").open(&mut open).resizable(false).default_width(460.0).max_height(650.0).vscroll(true).collapsible(false).show(ctx, |ui| {
            ui.heading(if self.convert_full { "Convertir archivo completo" } else { "Exportar selección" });
            ui.label(format!("Duración a guardar: {length:.3} segundos"));
            ui.separator();
            ui.label("Resolución / altura máxima");
            egui::ComboBox::from_id_salt("output-resolution").selected_text(if self.video_options.height == 0 { "Original".into() } else { format!("{}p", self.video_options.height) }).show_ui(ui, |ui| {
                for (label, height) in [("Original", 0), ("2160p / 4K", 2160), ("1440p", 1440), ("1080p / Full HD", 1080), ("720p / HD", 720), ("480p", 480), ("360p", 360)] { ui.selectable_value(&mut self.video_options.height, height, label); }
            });
            let (width, height) = video::output_size(&info, self.video_options.height);
            ui.label(RichText::new(format!("Salida: {width} x {height}. Mantiene proporción, sin ampliar el original.")).small().color(palette().muted));
            ui.add_space(4.0); ui.label("Calidad y peso");
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.video_options.quality, video::Quality::Light, "Más ligero");
                ui.selectable_value(&mut self.video_options.quality, video::Quality::Balanced, "Equilibrado");
                ui.selectable_value(&mut self.video_options.quality, video::Quality::High, "Más calidad");
            });
            ui.checkbox(&mut self.video_options.limit_fps, "Limitar a 30 fps si el original tiene más");
            egui::CollapsingHeader::new("Audio del WebM / mezcla y volumen").default_open(true).show(ui, |ui| self.video_sound_controls(ui, info.has_audio));
            ui.small("WebM VP9 usa compresión eficiente. El tamaño final depende del movimiento, la resolución y la duración; más calidad suele pesar más.");
            if info.alpha { ui.small("Se intentará conservar la transparencia del GIF o video original."); }
            ui.add_space(8.0);
            if ui.add_sized([ui.available_width(), 38.0], egui::Button::new("Elegir carpeta y guardar WebM…").fill(palette().accent.gamma_multiply(0.5))).clicked() { save = true; }
        });
        self.show_video_export = open;
        if save { self.show_video_export = false; self.save(ctx); }
    }

    fn sources_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
        card(ui, "Tu biblioteca", if self.video_mode { "Video / GIF local o YouTube" } else { "Audio local o un enlace de YouTube" }, |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                if ui.add_sized([ui.available_width(), 40.0], egui::Button::new(if self.video_mode { "Abrir video o GIF" } else { "Abrir audio o lista M3U" }).fill(palette().accent.gamma_multiply(0.4))).clicked() {
                    let dialog = if self.video_mode { rfd::FileDialog::new().add_filter("Videos y GIF", &["mp4", "mkv", "webm", "gif", "mov", "avi", "m4v", "flv", "wmv", "mpeg", "mpg", "ts", "ogv"]).add_filter("Todos los archivos", &["*"]) } else { rfd::FileDialog::new().add_filter("Audio y listas", &["wav", "mp3", "m3u", "m3u8", "flac", "ogg", "m4a"]) };
                    if let Some(path) = dialog.pick_file() {
                        let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                        if !self.video_mode && (ext == "m3u" || ext == "m3u8") {
                            match media::playlist(&path) { Ok(tracks) => { self.tracks = tracks; self.track_index = 0; self.load(self.tracks[0].clone(), ctx); }, Err(e) => self.message(Err(e)) }
                        } else { self.tracks.clear(); self.load(path, ctx); }
                    }
                }
                ui.label(RichText::new(if self.video_mode { "MP4 / GIF / WEBM / MKV / MOV / AVI…" } else { "WAV / MP3 / M3U / FLAC / OGG / M4A" }).small().color(palette().muted));
                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut self.url).hint_text("Pega el enlace de YouTube").desired_width(ui.available_width()));
                if ui.add_sized([ui.available_width(), 34.0], egui::Button::new(if self.video_mode { "Descargar video de YouTube" } else { "Descargar solo audio" })).clicked() { self.download(ctx); }
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
        card(ui, "Duraciones rápidas", "Elige una base y ajústala a tu gusto", |ui| {
            ui.add_enabled_ui(!busy && self.audio.is_some(), |ui| {
                ui.label(RichText::new("ALERTA DE TWITCH").small().color(palette().accent));
                ui.horizontal(|ui| { for seconds in [9.0, 11.0] {
                    if ui.add(egui::Button::new(format!("{seconds:.0} segundos")).selected(self.preset == Some(seconds))).clicked() { self.apply_preset(seconds); }
                } });
                ui.add_space(4.0);
                if ui.add_sized([ui.available_width(), 36.0], egui::Button::new("Ringtone / 30 segundos").selected(self.preset == Some(30.0))).clicked() { self.apply_preset(30.0); }
                ui.add_space(4.0);
                ui.label(RichText::new("NOTIFICACIÓN").small().color(palette().mint));
                ui.horizontal_wrapped(|ui| { for (label, seconds) in [("1 s", 1.0), ("1.5 s", 1.5), ("3 s", 3.0), ("4 s", 4.0), ("5 s", 5.0)] {
                    if ui.add(egui::Button::new(label).selected(self.preset == Some(seconds))).clicked() { self.apply_preset(seconds); }
                } });
                if self.preset.is_none() { ui.label(RichText::new("Duración personalizada activa").small().color(palette().muted)); }
            });
        });
    }

    fn selection_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        let duration = self.audio.as_ref().unwrap().duration;
        card(ui, "02 / Tu selección", "Escribe los tiempos o marca los puntos mientras escuchas", |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                let min = editor::MIN_LENGTH.min(duration);
                let mut start = self.start; let mut end = self.start + self.length; let mut length = self.length;
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("INICIO / segundos").small().color(palette().accent));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut start).speed(self.step).range(0.0..=(end - min).max(0.0)).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_start(self.start, self.length, start, duration); self.preset = None; self.view.follow(self.start, duration);
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label(RichText::new("FINAL / segundos").small().color(palette().mint));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut end).speed(self.step).range((self.start + min).min(duration)..=duration).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_end(self.start, end, duration); self.preset = None; self.view.follow(self.start + self.length, duration);
                        }
                    });
                    ui.vertical(|ui| {
                        ui.label(RichText::new("DURACIÓN / segundos").small().color(palette().muted));
                        if ui.add_sized([135.0, 34.0], egui::DragValue::new(&mut length).speed(self.step).range(min..=(duration - self.start).max(min)).fixed_decimals(3)).changed() {
                            self.stop(); (self.start, self.length) = editor::resize_end(self.start, self.start + length, duration); self.preset = None;
                        }
                    });
                });
                if self.preset.is_some_and(|preset| duration < preset) { ui.colored_label(Color32::from_rgb(244, 194, 115), "La pista es más corta: se seleccionó su duración completa."); }
                let mut position = self.start;
                if ui.add(egui::Slider::new(&mut position, 0.0..=(duration - self.length).max(0.0)).text("Mover corte completo").show_value(false)).changed() { self.stop(); self.start = position; self.normalize(); self.view.follow(self.start, duration); }
            });
        });
    }

    fn precision_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        card(ui, "03 / Ajuste fino", "Suma o quita duración sin mover el inicio", |ui| {
            ui.add_enabled_ui(!busy, |ui| {
                for (label, sign) in [("Agregar", 1.0), ("Quitar", -1.0)] {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(label).color(if sign > 0.0 { palette().mint } else { palette().muted }));
                        for (name, seconds) in [("1 ms", 0.001), ("5 ms", 0.005), ("100 ms", 0.100), ("500 ms", 0.500), ("1 min", 60.0)] {
                            if ui.button(format!("{} {name}", if sign > 0.0 { "+" } else { "-" })).clicked() { self.change_duration(sign * seconds); }
                        }
                    });
                }
                ui.add_space(2.0);
                egui::CollapsingHeader::new("Mover por pasos / inicio, final o selección").show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Paso:");
                        for (label, seconds) in [("1 ms", 0.001), ("5 ms", 0.005), ("100 ms", 0.100), ("500 ms", 0.500), ("1 min", 60.0)] { ui.selectable_value(&mut self.step, seconds, label); }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.selectable_value(&mut self.target, Target::Start, "Inicio"); ui.selectable_value(&mut self.target, Target::End, "Final"); ui.selectable_value(&mut self.target, Target::Selection, "Selección");
                        if ui.button("- Retroceder").clicked() { self.nudge(-self.step); }
                        if ui.button("+ Avanzar").clicked() { self.nudge(self.step); }
                    });
                });
            });
        });
    }

    fn history_ui(&mut self, ui: &mut egui::Ui, ctx: &egui::Context, busy: bool) {
        card(ui, "Tus últimos cortes", "Se conservan entre sesiones", |ui| {
            if self.history.is_empty() { ui.label(RichText::new("Todavía no hay cortes. Tu próxima alerta aparecerá aquí.").color(palette().muted)); }
            let mut open = None; let mut reload = None;
            egui::ScrollArea::vertical().id_salt("history-scroll").max_height(240.0).show(ui, |ui| {
                for entry in &self.history {
                    ui.label(RichText::new(entry.output.file_name().unwrap_or_default().to_string_lossy()).strong()).on_hover_text(entry.output.display().to_string());
                    ui.label(RichText::new(format!("{} / {:.3} s / {}", entry.date, entry.length, if entry.video { "Video" } else { "Audio" })).small().color(palette().muted));
                    ui.horizontal(|ui| {
                        if ui.small_button("Ver carpeta").clicked() { open = Some(entry.output.clone()); }
                        if ui.add_enabled(!busy, egui::Button::new("Abrir original").small()).clicked() { reload = Some((entry.source.clone(), entry.video)); }
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
                if ui.button("Normalizar volumen…").clicked() { self.show_sound = true; }
                if ui.add(egui::Button::new(if self.video_mode { "Reproducir selección" } else { "Escuchar selección" }).fill(palette().accent.gamma_multiply(0.4))).clicked() { self.play(false); }
                if ui.button(if self.video_mode { "Reproducir desde aquí" } else { "Escuchar desde aquí" }).clicked() { self.play(true); }
                if ui.button("Detener").clicked() { self.stop(); }
            });
            ui.add_space(12.0);
            ui.label(RichText::new(time(self.playhead)).monospace().size(22.0).color(if self.player.is_some() { palette().mint } else { palette().muted }));
            let can_mark = ready && self.player.as_ref().and_then(|p| p.position()).is_some();
            ui.add_enabled_ui(can_mark, |ui| {
                if ui.add(egui::Button::new(RichText::new("Marcar inicio").color(palette().accent)).stroke(Stroke::new(1.0_f32, palette().accent))).on_hover_text("Captura el momento que está sonando. La reproducción continúa.").clicked() { self.mark_playback(false); }
                if ui.add(egui::Button::new(RichText::new("Marcar final").color(palette().mint)).stroke(Stroke::new(1.0_f32, palette().mint))).on_hover_text("Cierra tu selección en el momento que está sonando.").clicked() { self.mark_playback(true); }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{}  hasta  {}", time(self.start), time(self.start + self.length))).color(palette().muted));
            ui.label(RichText::new(format!("{:.3} s", self.length)).strong().color(palette().accent));
            ui.separator();
            ui.add_enabled_ui(ready, |ui| {
                if self.video_mode {
                    if ui.add(egui::Button::new("Guardar corte WebM…").fill(palette().accent.gamma_multiply(0.5))).clicked() { self.convert_full = false; self.show_video_export = true; }
                    if ui.button("Convertir todo a WebM…").clicked() { self.convert_full = true; self.show_video_export = true; }
                } else {
                    ui.selectable_value(&mut self.format_mp3, false, "WAV"); ui.selectable_value(&mut self.format_mp3, true, "MP3");
                    if ui.add(egui::Button::new(RichText::new("Cortar y guardar…").strong()).fill(palette().accent.gamma_multiply(0.5))).clicked() { self.save(ctx); }
                }
            });
            if let Some(output) = self.last_output.clone() { if ui.button("Mostrar último corte").on_hover_text(output.display().to_string()).clicked() { self.message(reveal(&output)); } }
            if busy && self.receiver.is_some() { ui.spinner(); if ui.small_button("Cancelar operación").clicked() { self.cancel.store(true, Ordering::Relaxed); self.status = "Cancelando…".into(); } }
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
            self.status = "Cancelando la operación. Puedes cerrar cuando termine…".into();
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
                ui.vertical(|ui| { ui.label(RichText::new("Shiny Twitch Creator Tool").size(25.0).strong()); ui.label(RichText::new("Encuentra el momento. Dale tu sonido.").small().color(palette().muted)); });
                ui.add_space(20.0);
                ui.add_enabled_ui(!busy, |ui| {
                    if ui.selectable_label(!self.video_mode, RichText::new("Audio").size(17.0)).clicked() { self.switch_mode(false); }
                    if ui.selectable_label(self.video_mode, RichText::new("Video / GIF").size(17.0)).clicked() { self.switch_mode(true); }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Temas").clicked() { self.show_themes = true; }
                    ui.label(RichText::new("STUDIO / 1.1").small().color(palette().accent));
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
                ui.label(RichText::new("Datos y descargas locales").small().color(palette().muted)).on_hover_text(self.data.display().to_string());
            });
        });
        egui::CentralPanel::default().frame(egui::Frame::none().fill(palette().bg).inner_margin(egui::Margin::symmetric(20.0, 8.0))).show(ctx, |ui| {
            egui::ScrollArea::vertical().id_salt("editor-scroll").show(ui, |ui| {
                if let Some(audio) = &self.audio {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(audio.path.file_name().unwrap_or_default().to_string_lossy()).size(18.0).strong());
                        ui.label(RichText::new(format!("PISTA / {}", time(audio.duration))).small().color(palette().muted));
                    });
                    ui.add_space(8.0);
                    if self.video_mode { self.video_preview_ui(ui); ui.add_space(8.0); }
                    card(ui, if self.video_mode { "01 / Línea de tiempo del video" } else { "01 / Explora tu pista" }, "Arrastra los extremos para ajustar el corte; su interior para moverlo", |ui| { ui.add_enabled_ui(!busy, |ui| self.waveform(ui)); });
                    ui.add_space(8.0); self.selection_ui(ui, busy);
                    ui.add_space(8.0); self.precision_ui(ui, busy);
                } else {
                    ui.add_space(65.0);
                    card(ui, "Tu próxima alerta empieza aquí", "Abre una pista desde el panel de la derecha para comenzar", |ui| {
                        ui.add_space(18.0);
                        ui.label(RichText::new(if self.video_mode { "01   Carga un video o GIF" } else { "01   Carga tu audio" }).size(20.0).color(palette().accent));
                        ui.label(RichText::new("02   Reproduce y marca inicio / final").size(20.0).color(palette().text));
                        ui.label(RichText::new("03   Ajusta y guarda tu fragmento").size(20.0).color(palette().mint));
                        ui.add_space(18.0);
                        ui.label(RichText::new("Alertas de Twitch, ringtones y notificaciones en un mismo lugar.").color(palette().muted));
                    });
                }
            });
        });
        self.themes_ui(ctx);
        self.video_export_ui(ctx);
        self.sound_window_ui(ctx);
        self.external_editor_ui(ctx);
    }
}
