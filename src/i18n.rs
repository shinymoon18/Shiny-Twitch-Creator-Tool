use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Language { English, Spanish }
static LANGUAGE: AtomicU8 = AtomicU8::new(0);
pub fn current() -> Language { if LANGUAGE.load(Ordering::Relaxed) == 1 { Language::Spanish } else { Language::English } }
pub fn set(language: Language) { LANGUAGE.store(if language == Language::Spanish { 1 } else { 0 }, Ordering::Relaxed); }
pub fn from_locale(locale: &str) -> Language {
    if locale.split(['-', '_', '.', ':']).next().is_some_and(|s| s.eq_ignore_ascii_case("es")) { Language::Spanish } else { Language::English }
}
pub fn system_language() -> Language {
    #[cfg(windows)] {
        #[link(name = "kernel32")]
        extern "system" { fn GetUserDefaultUILanguage() -> u16; }
        // Use the user's Windows display language, rather than date/currency formats.
        let language = unsafe { GetUserDefaultUILanguage() };
        if language != 0 { return if language & 0x03ff == 0x000a { Language::Spanish } else { Language::English }; }
    }
    for key in ["LC_ALL", "LANGUAGE", "LC_MESSAGES", "LANG"] {
        if let Ok(locale) = std::env::var(key) { if !locale.is_empty() { return from_locale(&locale); } }
    }
    Language::English
}
#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings { pub language: Option<Language> }
impl Settings { pub fn apply(&self) { set(self.language.unwrap_or_else(system_language)); } }

pub fn text(english: &'static str) -> &'static str {
    if current() == Language::Spanish { spanish(english) } else { english }
}

// Translate only app-owned templates. Formatting happens first, preserving Rust's
// precision rules and keeping paths and external diagnostics out of the catalog.
pub fn formatted(template: &'static str, rendered: String) -> String {
    if current() != Language::Spanish { return rendered; }
    formatted_spanish(template, rendered)
}
fn formatted_spanish(template: &'static str, rendered: String) -> String {
    let translated = spanish(template);
    if translated == template { return rendered; }
    let source = segments(template);
    let target = segments(translated);
    if source.len() != target.len() || !rendered.starts_with(source[0]) { return rendered; }
    let mut remaining = &rendered[source[0].len()..];
    let mut values = Vec::new();
    for (index, suffix) in source.iter().enumerate().skip(1) {
        let end = if suffix.is_empty() && index == source.len() - 1 { remaining.len() }
            else if let Some(end) = remaining.find(suffix) { end } else { return rendered; };
        values.push(&remaining[..end]);
        remaining = &remaining[end + suffix.len()..];
    }
    if !remaining.is_empty() { return rendered; }
    let mut result = target[0].to_owned();
    for (value, suffix) in values.into_iter().zip(target.into_iter().skip(1)) { result.push_str(value); result.push_str(suffix); }
    result
}
fn segments(template: &str) -> Vec<&str> {
    let mut result = Vec::new(); let mut rest = template;
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else { break; };
        result.push(&rest[..start]); rest = &rest[start + end + 1..];
    }
    result.push(rest); result
}

// Catalog generated from the existing Spanish interface; English is the key.
fn spanish(english: &str) -> &str {
    match english {
        "Video clip duration: {seconds:.3} s" => "Duración del corte de video: {seconds:.3} s",
        "Choose audio start" => "Elegir inicio del audio",
        "Match video duration" => "Igualar duración del video",
        "Move the slider or change Start to choose any part of the audio without changing the clip duration." => "Mueve la barra o cambia Inicio para elegir cualquier parte del audio sin cambiar la duración del fragmento.",
        "Fade in (s):" => "Entrada gradual (s):",
        "Fade out (s):" => "Salida gradual (s):",
        "Reset fades" => "Quitar desvanecimientos",
        "Drag the green IN and OUT handles on the waveform to adjust the fades." => "Arrastra los controles verdes ENTRADA y SALIDA sobre la onda para ajustar los desvanecimientos.",
        "IN" => "ENTRADA",
        "OUT" => "SALIDA",
        "YouTube rejected the download (403). Retry once; if it persists, the video or your network may be restricted." => "YouTube rechazó la descarga (403). Intenta de nuevo; si continúa, el video o tu red podrían tener restricciones.",
        "YouTube requires access or this video is unavailable. Try a public video available in your region." => "YouTube requiere acceso o el video no está disponible. Prueba con un video público disponible en tu región.",
        "YouTube download failed. Check your connection and the video link." => "Falló la descarga de YouTube. Revisa tu conexión y el enlace del video.",
        " / Transparency detected" => " / Transparencia detectada",
        " Reached the end of the track or the minimum duration." => " Se alcanzó el límite de la pista o la duración mínima.",
        "- Backward" => "- Retroceder",
        "- Duration" => "- Duración",
        "- Step" => "- Paso",
        "{:.1}x · {:.3} s visible" => "{:.1}x · {:.3} s visibles",
        "{}  to  {}" => "{}  hasta  {}",
        "{} marked at {}. Playback continues." => "{} marcado en {}. La reproducción continúa.",
        "{length:.0} seconds" => "{length:.0} segundos",
        "{name} is not from the embedded bundle." => "{name} no procede del paquete integrado.",
        "{seconds:.0} seconds" => "{seconds:.0} segundos",
        "{stem}_clip_{length:.3}s_{}.{}" => "{stem}_corte_{length:.3}s_{}.{}",
        "+ Duration" => "+ Duración",
        "+ Forward" => "+ Avanzar",
        "+ Step" => "+ Paso",
        "01   Open a video or GIF" => "01   Carga un video o GIF",
        "01   Open your audio" => "01   Carga tu audio",
        "01 / Explore your track" => "01 / Explora tu pista",
        "01 / Video timeline" => "01 / Línea de tiempo del video",
        "02   Play and mark the start / end" => "02   Reproduce y marca inicio / final",
        "02 / Your selection" => "02 / Tu selección",
        "03   Fine-tune and save your clip" => "03   Ajusta y guarda tu fragmento",
        "03 / Fine adjustments" => "03 / Ajuste fino",
        "1. Measure this clip" => "1. Medir este corte",
        "2. Adjust the output volume" => "2. Ajusta el volumen final",
        "3. Once you like the sound, close this window and save the clip. The adjustment also applies to the exported file." => "3. Cuando te guste cómo suena, cierra esta ventana y guarda el corte. El ajuste se aplica también al archivo exportado.",
        "Accent color:" => "Color de acento:",
        "Add" => "Agregar",
        "Adjustment / dB" => "Ajuste / dB",
        "Adjustment toward your target: {difference:+.1} dB." => "Para acercarte a tu objetivo: {difference:+.1} dB.",
        "All files" => "Todos los archivos",
        "Apply this difference as a manual adjustment" => "Usar esta diferencia como ajuste manual",
        "Audio and playlists" => "Audio y listas",
        "Audio playback failed. Check the file and your audio device." => "Falló la reproducción de audio. Revisa el archivo y el dispositivo de sonido.",
        "Audio trimmed: {} to {}. This will replace all original video audio." => "Audio recortado: {} hasta {}. Se sustituirá todo el sonido original del video.",
        "Audio: {} to {} / {:.3} s" => "Audio: {} hasta {} / {:.3} s",
        "Audio: choose a track and prepare your clip." => "Audio: selecciona tu pista y prepara el corte.",
        "Balanced" => "Equilibrado",
        "Cancel" => "Cancelar",
        "Cancel loading" => "Cancelar carga",
        "Cancel operation" => "Cancelar operación",
        "Canceling the operation. You can close the app when it finishes…" => "Cancelando la operación. Puedes cerrar cuando termine…",
        "Canceling…" => "Cancelando…",
        "Check the added audio's volume and start time." => "Revisa el volumen y el inicio del audio añadido.",
        "Choose a different filename to preserve the original audio." => "Elige otro nombre para conservar el audio original.",
        "Choose a different filename to preserve the original file." => "Elige otro nombre para conservar el archivo original.",
        "Choose a local audio file or select Export without audio." => "Elige un archivo de audio local o selecciona Sin audio.",
        "Choose a preset and fine-tune it" => "Elige una base y ajústala a tu gusto",
        "Choose a video up to four hours long." => "Selecciona un video de hasta cuatro horas.",
        "Choose an audio file first." => "Falta elegir tu archivo de audio.",
        "Choose an audio file with a valid duration of up to 4 hours." => "Selecciona un audio con duración válida de hasta 4 horas.",
        "Choose audio file…" => "Elegir archivo de audio…",
        "Choose location and save WebM…" => "Elegir carpeta y guardar WebM…",
        "Click Measure this clip to see its loudness and true peaks." => "Pulsa Medir este corte para ver el volumen y sus picos reales.",
        "Click Play to preview the video here" => "Pulsa Reproducir para ver el video aquí mismo",
        "Clip duration: {:.3} s. Start fixed at {}.{}" => "Duración del corte: {:.3} s. Inicio fijo en {}.{}",
        "Clip saved to {}" => "Corte guardado en {}",
        "Completely replaces the original audio. Only the selected audio clip is used." => "Reemplaza por completo el sonido original. Solo se usa el fragmento de audio elegido.",
        "Convert entire file" => "Convertir archivo completo",
        "Convert entire file to WebM…" => "Convertir todo a WebM…",
        "Could not create the data folder: {error}" => "No se pudo crear la carpeta de datos: {error}",
        "Could not decode the video for preview." => "No se pudo decodificar el video para su vista previa.",
        "Could not determine the duration. Choose a video or animated GIF with a defined duration." => "No se pudo obtener la duración. Selecciona un video o GIF animado con duración definida.",
        "Could not find the executable's folder." => "No se encontró la carpeta del ejecutable.",
        "Could not measure this clip's loudness." => "No se pudo medir el volumen de este fragmento.",
        "Could not open the history: {e}" => "No se pudo abrir el historial: {e}",
        "Could not play the audio: {s}" => "No se pudo reproducir el audio: {s}",
        "Could not play the file: {e}. Check ffplay." => "No se pudo reproducir el archivo: {e}. Comprueba ffplay.",
        "Could not prepare {}: {e}" => "No se pudo preparar {}: {e}",
        "Could not prepare {}: {e}. Place the executable in a writable folder." => "No se pudo preparar {}: {e}. Coloca el exe en una carpeta donde puedas escribir.",
        "Could not prepare the audio mix. Check the added audio file." => "No se pudo preparar la mezcla de audio. Revisa el archivo añadido.",
        "Could not read the audio duration." => "No se pudo leer la duración del audio.",
        "Could not read the history. It will be preserved until you save a new clip: {e}" => "No se pudo leer el historial. Se conservará hasta guardar un nuevo corte: {e}",
        "Could not read the playback clock." => "No se pudo leer el reloj de reproducción.",
        "Could not read the video frame." => "No se pudo leer el fotograma del video.",
        "Could not read the video." => "No se pudo leer el video.",
        "Could not save options: {e}" => "No se pudieron guardar las opciones: {e}",
        "Could not save the clip: {e}" => "No se pudo guardar el corte: {e}",
        "Could not save the theme: {e}" => "No se pudo guardar el tema: {e}",
        "Could not save the WebM file: {e}" => "No se pudo guardar el WebM: {e}",
        "Could not start {:?}: {e}. Check the tools folder." => "No se pudo iniciar {:?}: {e}. Revisa la carpeta tools.",
        "Could not start the embedded preview: {e}" => "No se pudo iniciar la vista previa integrada: {e}",
        "Could not start the mixed audio." => "No se pudo iniciar el audio de la mezcla.",
        "Custom duration selected" => "Duración personalizada activa",
        "Customize appearance" => "Personalizar interfaz",
        "Customize appearance…" => "Personalizar interfaz…",
        "Dark themes" => "Temas oscuros",
        "Download audio only" => "Descargar solo audio",
        "Download YouTube video" => "Descargar video de YouTube",
        "Downloading YouTube audio…" => "Descargando audio de YouTube…",
        "Downloading YouTube video…" => "Descargando video de YouTube…",
        "Drag the edges or enter times to select your audio clip." => "Arrastra los extremos o escribe los tiempos para elegir tu fragmento.",
        "Drag the edges to resize the selection, or drag inside it to move it" => "Arrastra los extremos para ajustar el corte; su interior para moverlo",
        "Duration (s):" => "Duración (s):",
        "DURATION / seconds" => "DURACIÓN / segundos",
        "Encoding WebM VP9… this may take a while. You can cancel the operation." => "Codificando WebM VP9… puede tardar. Puedes cancelar la operación.",
        "End" => "FIN",
        "End (s):" => "Final (s):",
        "END / seconds" => "FINAL / segundos",
        "End your selection at the current playback position." => "Cierra tu selección en el momento que está sonando.",
        "Enter times or mark points during playback" => "Escribe los tiempos o marca los puntos mientras escuchas",
        "Entire audio" => "Todo el audio",
        "Export selection" => "Exportar selección",
        "Export without audio" => "Cortar sin audio",
        "Exporting alert…" => "Exportando alerta…",
        "Extend or shorten the clip without moving its start" => "Suma o quita duración sin mover el inicio",
        "File ready. Play and mark your start and end points." => "Archivo listo. Reproduce y marca tu inicio y final.",
        "Find the moment. Make it sound like you." => "Encuentra el momento. Dale tu sonido.",
        "Fit all" => "Ver todo",
        "Fit entire track" => "Ver toda la pista",
        "Fit selection" => "Ver selección",
        "Forest" => "Bosque",
        "Higher quality" => "Más calidad",
        "Highest peak: {peak:.1} dBTP" => "Pico más alto: {peak:.1} dBTP",
        "If the audio clip is shorter than the video, the remaining time is silent. If longer, it is trimmed to the video selection." => "Si el fragmento es más corto que el video, su resto queda en silencio. Si es más largo, se limita al corte de video.",
        "Incomplete measurement." => "Medición incompleta.",
        "Interface scale" => "Tamaño de interfaz",
        "Invalid bundle size." => "Tamaño del paquete no válido.",
        "Invalid portable bundle: {e}" => "Paquete portable no válido: {e}",
        "Keep original audio" => "Mantener audio original",
        "Language / Idioma" => "Language / Idioma",
        "Language changes apply immediately and are saved for next time." => "El idioma cambia al instante y se guarda para la próxima vez.",
        "Left: quieter / Center: original level / Right: louder" => "Izquierda: más suave / Centro: nivel original / Derecha: más fuerte",
        "Limit to 30 fps when the source frame rate is higher" => "Limitar a 30 fps si el original tiene más",
        "Loading and analyzing the file…" => "Cargando archivo y analizando su contenido…",
        "Loading the audio waveform…" => "Cargando la onda del audio…",
        "Local audio / dB" => "Archivo local / dB",
        "Local audio or a YouTube link" => "Audio local o un enlace de YouTube",
        "Local data and downloads" => "Datos y descargas locales",
        "Local video / GIF or YouTube" => "Video / GIF local o YouTube",
        "Mark end" => "Marcar final",
        "Mark end during playback" => "Marcar final al escuchar",
        "Mark start" => "Marcar inicio",
        "Mark start during playback" => "Marcar inicio al escuchar",
        "Mark the current playback position. Playback continues." => "Captura el momento que está sonando. La reproducción continúa.",
        "Match your alert volumes" => "Iguala tus alertas a tu gusto",
        "Measurement complete. Adjust the volume and preview your alert." => "Medición lista. Ajusta el volumen y escucha tu alerta.",
        "Measuring or processing…" => "Midiendo o procesando…",
        "Measuring the selected clip's loudness…" => "Midiendo el volumen del corte seleccionado…",
        "Move entire selection" => "Mover corte completo",
        "Next view" => "Vista siguiente",
        "Night" => "Noche",
        "No clips yet. Your next alert will appear here." => "Todavía no hay cortes. Tu próxima alerta aparecerá aquí.",
        "No tracks were found in the file." => "No se encontraron pistas en el archivo.",
        "No video is loaded." => "No hay un video cargado.",
        "Normalize alert volume…" => "Normalizar volumen de la alerta…",
        "Normalize volume…" => "Normalizar volumen…",
        "NOTIFICATION" => "NOTIFICACIÓN",
        "Ocean" => "Océano",
        "Open a file from the panel on the right to get started" => "Abre una pista desde el panel de la derecha para comenzar",
        "Open a track or paste a YouTube link to get started." => "Abre una pista o pega un enlace de YouTube para empezar.",
        "Open audio or M3U playlist" => "Abrir audio o lista M3U",
        "Open source" => "Abrir original",
        "Open video or GIF" => "Abrir video o GIF",
        "Operation canceled." => "Operación cancelada.",
        "Options" => "Opciones",
        "Original audio is muted by default" => "Sin sonido original por defecto",
        "Output duration: {length:.3} seconds" => "Duración a guardar: {length:.3} segundos",
        "Output: {width} x {height}. Keeps the aspect ratio without upscaling." => "Salida: {width} x {height}. Mantiene proporción, sin ampliar el original.",
        "Paste a YouTube link" => "Pega el enlace de YouTube",
        "Paste an HTTP or HTTPS YouTube link." => "Pega un enlace HTTP o HTTPS de YouTube.",
        "Peak headroom" => "Margen de los picos",
        "Perceived loudness: {level:.1} LUFS" => "Volumen percibido: {level:.1} LUFS",
        "Play from here" => "Reproducir desde aquí",
        "Play selection" => "Reproducir selección",
        "Playback error: {e}" => "Error al reproducir: {e}",
        "Playing inside Shiny Twitch Creator Tool" => "Reproduciendo dentro de Shiny Twitch Creator Tool",
        "Preview clip" => "Escuchar fragmento",
        "Preview from here" => "Escuchar desde aquí",
        "Preview result" => "Escuchar resultado",
        "Preview selection" => "Escuchar selección",
        "Previous view" => "Vista anterior",
        "Quality and file size" => "Calidad y peso",
        "Quick durations" => "Duraciones rápidas",
        "Quiet                                              Loud" => "Suave                                              Fuerte",
        "Recent clips" => "Tus últimos cortes",
        "Reference for matching multiple alerts" => "Referencia para igualar varias alertas",
        "Remove" => "Quitar",
        "Replace with local audio" => "Reemplazar con archivo local",
        "Reset" => "Restablecer",
        "Reset theme accent" => "Restablecer acento del tema",
        "Resolution / maximum height" => "Resolución / altura máxima",
        "Ringtone / 30 seconds" => "Ringtone / 30 segundos",
        "Rose" => "Rosa",
        "Save Twitch alert" => "Guardar alerta de Twitch",
        "Save WebM clip…" => "Guardar corte WebM…",
        "Save WebM video" => "Guardar video WebM",
        "Saved between sessions" => "Se conservan entre sesiones",
        "Scroll audio" => "Explorar audio",
        "Scroll track" => "Explorar pista",
        "Select Keep original audio or a local audio file to measure loudness." => "Selecciona Mantener original o un archivo local para medir sonido.",
        "Selection" => "Selección",
        "Show folder" => "Ver carpeta",
        "Show latest clip" => "Mostrar último corte",
        "Smaller file" => "Más ligero",
        "Source volume (advanced)" => "Volumen de cada fuente (avanzado)",
        "Start" => "INICIO",
        "Start (s):" => "Inicio (s):",
        "START / seconds" => "INICIO / segundos",
        "Step controls / start, end, or selection" => "Mover por pasos / inicio, final o selección",
        "Step:" => "Paso:",
        "Stop" => "Detener",
        "System default" => "Idioma del equipo",
        "Target / LUFS" => "Objetivo / LUFS",
        "The {} tool is incomplete." => "La herramienta {} está incompleta.",
        "The added audio file could not be found. Select it again." => "No se encuentra el archivo de audio añadido. Vuelve a seleccionarlo.",
        "The added audio selection is invalid. Check its start and end times." => "El fragmento del audio añadido no es válido. Revisa su inicio y final.",
        "The app will try to preserve transparency from the original GIF or video." => "Se intentará conservar la transparencia del GIF o video original.",
        "The audio must have a valid duration of up to four hours." => "El audio debe tener una duración válida de hasta cuatro horas.",
        "The bundle contains an invalid path or size." => "El paquete contiene una ruta o tamaño no válido.",
        "The bundle does not contain valid tools." => "El paquete no contiene herramientas válidas.",
        "The clip was saved, but its history entry could not be saved: {e}" => "El audio se guardó, pero no el historial: {e}",
        "The download did not produce a file. Check the link and update yt-dlp." => "La descarga no produjo un archivo. Comprueba el enlace y actualiza yt-dlp.",
        "The downloaded audio could not be found." => "No se encontró el audio descargado.",
        "The end must be after the start. Wait a moment or mark a new start." => "El final debe quedar después del inicio. Espera un momento o marca un nuevo inicio.",
        "The file does not contain a decodable audio track." => "El archivo no contiene una pista de audio decodificable.",
        "The file does not contain an audio track." => "El archivo no contiene una pista de audio.",
        "The file does not contain video. Open a video or an animated GIF." => "El archivo no contiene video. Abre una película o un GIF animado.",
        "The file has been moved or deleted." => "El archivo se movió o fue eliminado.",
        "The filename must end in .{ext}." => "El nombre debe terminar en .{ext}.",
        "The link must be a YouTube URL." => "El enlace debe pertenecer a YouTube.",
        "The M3U playlist does not contain any tracks." => "La lista M3U no contiene pistas.",
        "The operation timed out. Check your connection or try another file." => "La operación excedió el tiempo de espera. Revisa la conexión o prueba otro archivo.",
        "The original audio volume must be between -60 and +12 dB." => "El volumen original debe estar entre -60 y +12 dB.",
        "The output adjustment must be between -30 and +18 dB." => "El ajuste final debe estar entre -30 y +18 dB.",
        "The output cannot overwrite the added audio file." => "El destino no puede reemplazar el archivo de audio añadido.",
        "The output filename must end in .wav or .mp3." => "El archivo de salida debe terminar en .wav o .mp3.",
        "The output filename must end in .webm." => "El destino debe terminar en .webm.",
        "The playlist must use UTF-8 encoding. Save it as M3U8." => "La lista debe estar en UTF-8. Guárdala como M3U8.",
        "The portable bundle index is invalid." => "El índice del paquete portable no es válido.",
        "The portable bundle is incomplete. Copy the executable again." => "El paquete portable está incompleto. Copia de nuevo el ejecutable.",
        "The portable bundle is truncated." => "El paquete portable está truncado.",
        "The portable trim check failed." => "La prueba de corte portable falló.",
        "The selected range is invalid." => "El intervalo no es válido.",
        "The selection or volume has changed. Measure again to update the values." => "Cambió el corte o el volumen. Vuelve a medir para actualizar los valores.",
        "The track does not exist. Check the path in the M3U playlist." => "La pista no existe. Comprueba la ruta de la lista M3U.",
        "The track is shorter, so its full duration has been selected." => "La pista es más corta: se seleccionó su duración completa.",
        "The trim range is invalid." => "El intervalo de corte no es válido.",
        "The video dimensions are invalid." => "El video tiene dimensiones no válidas.",
        "Themes" => "Temas",
        "This clip is silent or too short to measure perceived loudness." => "El corte es silencioso o demasiado corto para medir volumen percibido.",
        "This clip will replace all original video audio." => "Este fragmento sustituirá todo el sonido original del video.",
        "This executable does not contain the embedded bundle." => "Este ejecutable no contiene el paquete integrado.",
        "This increase may activate peak protection. Preview the result and measure again." => "Ese aumento puede activar la protección de picos. Escucha el resultado y vuelve a medir.",
        "This version supports M3U playlists with local tracks. For YouTube, use the link field." => "Esta versión admite M3U con pistas locales. Para YouTube, utiliza el campo de enlace.",
        "This video has no original audio." => "Este video no tiene sonido original.",
        "TRACK / {}" => "PISTA / {}",
        "Trim and save…" => "Cortar y guardar…",
        "Trim audio for the video" => "Recortar audio para el video",
        "Trim this audio…" => "Recortar este audio…",
        "TWITCH ALERT" => "ALERTA DE TWITCH",
        "Twitch alert volume" => "Volumen de alertas de Twitch",
        "Twitch alerts, ringtones, and notifications in one place." => "Alertas de Twitch, ringtones y notificaciones en un mismo lugar.",
        "Use the same target to compare your alerts. This is a personal reference, not a Twitch requirement." => "Elige el mismo objetivo para comparar tus alertas. Es una referencia personal, no un requisito de Twitch.",
        "Use this audio clip in the video" => "Usar este fragmento en el video",
        "Using {} to {} / {:.3} seconds" => "Se usará {} hasta {} / {:.3} segundos",
        "Video cuts align to frames; millisecond steps are still available." => "Los cortes de video se ajustan a sus fotogramas; los pasos de milisegundos siguen disponibles.",
        "Video preview" => "Vista previa de video",
        "Video: open a local file or download from YouTube." => "Video: abre un archivo local o descarga desde YouTube.",
        "Videos and GIFs" => "Videos y GIF",
        "WebM audio" => "Audio del WebM",
        "WebM audio / source and volume" => "Audio del WebM / mezcla y volumen",
        "WebM VP9 uses efficient compression. File size depends on motion, resolution, and duration; higher quality usually means a larger file." => "WebM VP9 usa compresión eficiente. El tamaño final depende del movimiento, la resolución y la duración; más calidad suele pesar más.",
        "Which audio would you like to save?" => "¿Qué sonido quieres guardar?",
        "With audio" => "Con audio",
        "Without audio" => "Sin audio",
        "You control the adjustment. Measure, adjust the volume, and preview before saving." => "Tú controlas el ajuste. Mide, cambia el volumen y escucha antes de guardar.",
        "Your library" => "Tu biblioteca",
        "Your next alert starts here" => "Tu próxima alerta empieza aquí",
        "Your preferences are saved for the next time you open the app." => "Tu elección se guarda para la próxima vez que abras el programa.",
        "Zoom in +" => "Acercar +",
        "Zoom out -" => "Alejar -",
        _ => english,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn detects_spanish_regional_locales_and_english_fallback() {
        for locale in ["es-MX", "es-ES", "es_US.UTF-8", "ES", "es:en"] { assert_eq!(from_locale(locale), Language::Spanish); }
        for locale in ["en-US", "fr-FR", "", "C"] { assert_eq!(from_locale(locale), Language::English); }
    }
    #[test] fn localized_settings_roundtrip_preserves_explicit_choice() {
        let settings = Settings { language: Some(Language::English) };
        let decoded: Settings = serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert_eq!(decoded.language, Some(Language::English));
        assert_eq!(serde_json::from_str::<Settings>("{}").unwrap().language, None);
    }
    #[test] fn format_segments_keep_paths_and_numeric_precision() {
        assert_eq!(segments("Audio: {} to {} / {:.3} s"), vec!["Audio: ", " to ", " / ", " s"]);
        assert_eq!(spanish("Clip saved to {}"), "Corte guardado en {}");
        assert_eq!(formatted_spanish("Clip saved to {}", "Clip saved to C:\\clips\\alert.wav".into()), "Corte guardado en C:\\clips\\alert.wav");
        assert_eq!(formatted_spanish("Audio: {} to {} / {:.3} s", "Audio: 00:01.000 to 00:02.125 / 1.125 s".into()), "Audio: 00:01.000 hasta 00:02.125 / 1.125 s");
    }
}
