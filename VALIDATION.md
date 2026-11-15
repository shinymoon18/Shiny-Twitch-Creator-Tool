# Verificación de Shiny Twitch Creator Tool

## Actualización 1.1 / Sustitución y recorte del audio local

- 21 pruebas aprobadas, sin omisiones; compilación release y paquete de ejecutable único actualizados.
- La prueba de sustitución utiliza un video con tono de 440 Hz y un audio local con tres secciones de 660, 880 y 1100 Hz. Selecciona únicamente un fragmento de la sección de 880 Hz y comprueba la ausencia del tono original y de las secciones no elegidas. También comprueba el silencio después del final del fragmento, sin acortar el video.
- La exportación y la reproducción comparten la misma cadena de recorte y volumen. Se elimina el modo de sumar el original con el archivo local; los tres modos disponibles son Original, Sin audio y Reemplazar.
- Nueva ventana independiente de edición del audio añadido con onda, zoom, reproducción, marcadores, inicio/final/duración, presets y pasos. Aplicar guarda el intervalo para combinarlo con el video; cancelar conserva el archivo anterior.

## Actualización 1.0.1 / Nuevo nombre

Se actualizan el título de la ventana, la cabecera, los avisos y el ejecutable portable a Shiny Twitch Creator Tool. Se mantienen la carpeta de datos y la identidad interna anteriores para conservar la compatibilidad con historial y preferencias. Cambio verificado mediante compilación release y empaquetado portable.

## Actualización 1.0 / Ejecutable único portable

- 21 pruebas aprobadas, sin omisiones.
- Prueba del exe final copiado solo a una carpeta aislada y con PATH limitado a Windows: salida correcta, cinco herramientas detectadas dentro del paquete, datos junto al exe y corte WAV real de nueve segundos con ajuste de −6 dB. Primera preparación y verificación completas en aproximadamente 7.6 segundos en este equipo.
- Inspección de dependencias del editor: runtime C estático; las dependencias externas detectadas corresponden a DLL del sistema de Windows. Se inspeccionaron también las cinco herramientas incluidas.
- Paquete comprimido anexado al ejecutable, con manifest de archivos, tamaños y versión de caché. No descarga dependencias al abrirse.
- Nuevas pruebas de extracción y reutilización de caché, rechazo de rutas que salgan de la carpeta de herramientas y detección de contenido comprimido dañado.
- Historial, preferencias y descargas se guardan junto al exe en la variante integrada. La distribución sin paquete integrado conserva su ubicación anterior.
- El comando de verificación `--portable-check` comprueba las versiones de las cinco herramientas desde el paquete y realiza un corte WAV de nueve segundos con ajuste de volumen.

## Actualización 0.9 / Normalización manual e interfaz simplificada

- 19 pruebas aprobadas, sin omisiones.
- Nueva prueba con una alerta WAV real: mide volumen percibido y pico antes y después de un ajuste manual de +6 dB y comprueba la variación esperada. Exporta ese ajuste y vuelve a medir el WAV resultante para comprobar que coincide con el nivel solicitado.
- La prueba de modos de sonido mantiene un archivo local seleccionado al exportar Sin audio y Mantener original. Verifica que no se incluya su señal en ninguno de esos dos casos.
- El ajuste final de volumen se aplica tanto en la pestaña Audio como en Video, y se conserva en la previsualización y la exportación. El usuario controla la ganancia; la referencia LUFS solo propone una diferencia que se aplica al pulsar el botón correspondiente.
- Nueva ventana con medición, barras de volumen/pico, control manual grande, botones −1 / +1 dB, referencia editable y controles de cada fuente en un apartado avanzado. Una medición queda invalidada cuando cambia el corte o sus ajustes.

## Actualización 0.8 / Silencio y mezcla de audio

- 18 pruebas aprobadas, sin omisiones.
- Nueva prueba de exportación: crea un video con un tono de 440 Hz y un WAV externo de 880 Hz; verifica ausencia de pista de audio con las opciones predeterminadas, sustitución por el WAV sin fuga del tono original, relleno de silencio cuando termina el audio añadido y duración del video sin recortes adicionales.
- La misma prueba mezcla ambas pistas y mide sus amplitudes para comprobar la reducción relativa de −6 dB del original y −12 dB del archivo externo.
- Nueva prueba de reproducción integrada: reproduce un GIF con un audio local, verifica que se presentan varios fotogramas, que finaliza la reproducción y que el reloj conserva el desplazamiento absoluto del video.
- La previsualización y exportación comparten los filtros de volumen, mezcla y limitador. El sonido original del video está desactivado por defecto; la pestaña Audio conserva su funcionamiento.

## Actualización 0.7 / Reproducción integrada

- Compilación release completada y 16 pruebas aprobadas, sin omisiones.
- Nueva prueba real de reproducción integrada: genera un MP4 con audio y un GIF sin audio, reproduce un segundo desde 0.25 s y verifica que se reciben al menos tres fotogramas dentro del editor, que la imagen cambia, que el reloj está en el intervalo esperado y que la reproducción no finaliza antes de presentar los fotogramas.
- FFmpeg decodifica la imagen para una textura del editor. ffplay se limita al sonido, con las opciones sin ventana y sin video. Al detener o cambiar de pestaña, los procesos y la lectura de fotogramas se cancelan.
- La vista previa no modifica los parámetros de resolución ni calidad de los WebM exportados.
- La revisión visual de la versión 0.7 se interrumpió por el usuario con Escape; no se continuó el control de la ventana. La reproducción integrada se comprobó con fotogramas reales mediante la prueba automatizada anterior.

## Actualización 0.6 / Video y temas

- 15 pruebas aprobadas, sin omisiones, incluyendo las funciones de audio anteriores.
- La nueva prueba crea un MP4 con imagen y sonido, carga sus metadatos y un fotograma, exporta cortes WebM de 9 y 11 segundos desde 1.125 s y comprueba duración, resolución elegida y presencia de audio. También crea un GIF animado sin sonido, lo convierte completo y comprueba duración y ausencia de pista de audio.
- Prueba real de YouTube: descarga de video y audio del video público `jNQXAC9IVRw`, combinados en un MKV local para editar.
- Audio y Video disponen de espacios de trabajo separados dentro de la misma ventana. El historial antiguo es compatible: las entradas sin campo de tipo se interpretan como audio.
- Los cinco temas son oscuros; se incluye personalización del acento y escala de interfaz, guardadas en la carpeta de datos de la aplicación.
- Revisión visual de la versión release 0.6: cambio entre Audio y Video / GIF, apertura de Temas, cambio real a Océano y restauración a Noche; carga del MKV descargado de YouTube con fotograma y onda de audio; apertura del diálogo WebM con resolución, tamaño resultante, calidad, límite de fps y opción de audio.

## Actualización 0.5 / Night Studio

- Compilación release completada y 14 pruebas aprobadas, sin pruebas omitidas.
- Nueva prueba de marcadores: inicio y final con tiempo absoluto, duración resultante correcta, rechazo de final anterior al inicio y límites al acercarse al final de la pista.
- Nueva prueba de reproducción real y silenciosa con ffplay: tras buscar a 2.125 s, el reproductor comunica una posición absoluta válida y avanza. El reloj deja de depender del tiempo transcurrido desde que se crea el proceso.
- Se conservan las 12 pruebas anteriores del motor.
- Revisión visual de la aplicación release con MP3 cargado: tema oscuro, tarjetas, onda, panel lateral con biblioteca/presets/historial y barra inferior con reloj, reproducción, marcadores y exportación visibles.
- La prueba de pulsar los marcadores desde la interfaz se interrumpió con Escape por el usuario; no se continuó la automatización de la ventana. Su cálculo y el reloj de audio sí se verificaron mediante las pruebas automatizadas.

## Actualización 0.4

Se añadieron Ringtone (30 s) y Notificación (1, 1.5, 3, 4 y 5 s). Todos los botones utilizan la misma lógica de selección y límites que los presets anteriores. La actualización se verifica mediante compilación release; las pruebas del motor descritas a continuación corresponden a la versión 0.3.

## Verificación del motor 0.3

Verificado el 15 de noviembre de 2026 en Windows x64.

- Compilación release completada con Rust 1.100.0 y Cargo.lock.
- Doce pruebas automatizadas aprobadas, sin pruebas omitidas:
  - conservar 11 segundos al seleccionar el final de la pista;
  - manejar pistas más cortas que el preset;
  - resolver rutas relativas, espacios y BOM en M3U;
  - rechazar listas de red y dominios que suplantan a YouTube;
  - guardar, reemplazar y leer el historial;
  - generar audio real con FFmpeg, exportar WAV de 9, 11 y 13.125 segundos y un fragmento de 5 ms desde un punto fraccional y comprobar su duración con tolerancia de una muestra, reemplazar el destino, exportar MP3 y proteger el original;
  - mover la selección hacia ambos lados con pasos de 1 ms, 5 ms, 100 ms, 500 ms y un minuto;
  - ampliar y acortar los extremos por separado, conservando el extremo opuesto;
  - sumar y quitar directamente 1 ms, 5 ms, 100 ms, 500 ms y 1 minuto a cortes de 9 y 11 segundos manteniendo el inicio, y comprobar que varios clics acumulan la duración;
  - impedir cruces de los extremos y movimientos fuera de la pista;
  - comprobar el anclaje, los límites y el seguimiento de la vista con zoom;
  - distinguir un pico de un milisegundo tanto en la vista de detalle como en la general.
- Prueba de descarga con yt-dlp, FFmpeg y Deno: audio MP3 extraído correctamente del video público `jNQXAC9IVRw` de YouTube. El video de prueba antiguo de yt-dlp `BaW_jenozKc` estaba no disponible.
- Revisión visual de la aplicación release 0.2: pista MP3 cargada, onda con zoom 2x y tiempos de la ventana visible, extremos INICIO y FIN, campos de inicio/fin/duración y todos los botones de precisión presentes.…4834 tokens truncated…f.start,self.length)=editor::nudge(self.start,self.length,sign*self.step,total,Target::End); } }
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
