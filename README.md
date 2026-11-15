# Shiny Twitch Creator Tool

Editor de escritorio para Windows, escrito en Rust, para crear alertas de Twitch, ringtones y notificaciones a partir de audio, video y GIF.

## Funciones principales

- Audio local, videos, GIF animados y descarga de audio/video de YouTube.
- Presets de 9 y 11 segundos, ringtone de 30 segundos y notificaciones de 1, 1.5, 3, 4 y 5 segundos.
- Cortes libres con onda, zoom, pasos de milisegundos y marcadores durante reproducción.
- Reproducción de audio y video dentro del editor.
- Audio del video: conservar el original, quitarlo o sustituirlo por un fragmento de un archivo local.
- Editor independiente para recortar el audio añadido antes de combinarlo con la imagen.
- Medición de volumen y picos, ajuste manual en dB y referencia LUFS para igualar alertas.
- Exportación WAV/MP3 y WebM VP9, con resolución y calidad configurables.
- Historial, selección de carpeta de salida y temas oscuros personalizables.
- Distribución portable mediante un solo ejecutable con las herramientas incluidas.

## Código y ejecutable

Este repositorio contiene el código fuente y los scripts de compilación. El ejecutable portable se distribuye como archivo adjunto de una versión en **Releases**; los directorios `dist`, `target`, `.tools` y los datos personales no forman parte del código publicado.

Consulta [THIRD-PARTY.md](THIRD-PARTY.md) para los componentes de terceros y [VALIDATION.md](VALIDATION.md) para las verificaciones realizadas.

## Recortar el audio añadido y reemplazar el original (versión 1.1)

Al elegir **Reemplazar con archivo local**, el sonido original del video se descarta completamente. Se conservan las opciones independientes **Mantener audio original** y **Cortar sin audio**; ya no se suman el original y el archivo local.

1. En **Video / GIF → Audio del WebM**, selecciona **Reemplazar con archivo local** y elige tu archivo.
2. Se abre **Recortar audio para el video**, con la onda del audio y controles de zoom.
3. Ajusta **Inicio**, **Final** y **Duración**, arrastra los extremos o el cuerpo de la selección, usa 9/11 segundos o elige **Todo el audio**. También puedes mover por pasos de 1 ms, 5 ms, 100 ms, 500 ms o 1 minuto, y sumar o quitar esa cantidad a la duración.
4. Usa **Escuchar fragmento**, **Escuchar desde aquí** y los marcadores de inicio/final para localizar lo que quieres.
5. Pulsa **Usar este fragmento en el video**. Para modificarlo después, pulsa **Recortar este audio…** junto a su nombre.

El audio elegido se alinea con el inicio del corte de video. Solo se utiliza su intervalo seleccionado: si termina antes, el resto queda en silencio; si dura más, se limita a la duración del corte de video. Esto se aplica tanto a la reproducción integrada como al WebM exportado. El archivo de audio original no se modifica.

El programa se llamaba anteriormente Audio Twitch Cut. La versión 1.0.1 actualiza el título, la cabecera y el nombre del ejecutable. Se mantiene la carpeta **Audio Twitch Cut Datos** y la identidad interna anterior para reutilizar el historial y las preferencias existentes.

Aplicación de escritorio para Windows, escrita en Rust, para preparar alertas de Twitch.

## Un solo ejecutable portable (versión 1.0)

El archivo **Shiny Twitch Creator Tool Portable 1.1.exe** contiene el editor, FFmpeg, ffprobe, ffplay, yt-dlp, Deno y sus avisos de licencia. Solo necesitas copiar ese `.exe` a tu PC o memoria USB. No requiere instalar Rust ni descargar herramientas al abrirlo. La descarga de YouTube sigue necesitando conexión a Internet.

En la primera apertura prepara automáticamente sus herramientas comprimidas dentro de **Audio Twitch Cut Datos**, junto al ejecutable. Puede tardar unos segundos y requiere espacio libre para las herramientas descomprimidas, además del propio exe. Las aperturas siguientes reutilizan esa copia. Es necesario colocarlo en una carpeta donde tengas permiso de escritura; no hace falta ejecutarlo como administrador.

En esa misma carpeta quedan el historial, los temas y las descargas. Para llevar **solo el programa**, copia únicamente el exe: preparará todo de nuevo en su nueva ubicación. Para llevar también tus preferencias e historial, copia además **Audio Twitch Cut Datos**. Las pistas originales y los cortes guardados fuera de esa carpeta deben copiarse por separado; las entradas del historial conservan sus rutas originales.

La primera apertura portable copia el historial y las preferencias de versiones anteriores de este mismo equipo si aún no existen en la carpeta portable. No los elimina de su ubicación anterior.

Para generar el ejecutable único desde el proyecto:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/build-single.ps1
```

El resultado es `dist/Shiny Twitch Creator Tool Portable 1.1.exe`. Las herramientas, licencias y esta documentación se extraen dentro de la carpeta `herramientas` de los datos portables. El editor se compila con el runtime de C estático.

## Volumen manual para alertas de Twitch (versión 0.9)

En la barra inferior pulsa **Normalizar volumen…**. Se abre una ventana independiente dentro del editor, disponible en Audio y Video, para igualar manualmente tus alertas:

1. Selecciona el fragmento que quieres guardar y pulsa **Medir este corte**. La medición no cambia el archivo: muestra su volumen percibido en LUFS y su pico más alto en dBTP, con barras visuales.
2. Mueve el control grande **Ajuste / dB**, usa **−1 dB / +1 dB** o escribe el valor. Un valor negativo baja el volumen, uno positivo lo sube y **Restablecer** vuelve a 0 dB. El intervalo es de −30 a +18 dB.
3. Pulsa **Escuchar resultado**. Si te gusta, cierra la ventana y guarda normalmente. El mismo ajuste se aplica a la previsualización y al WAV, MP3 o WebM exportado.
4. Para comparar varias alertas, elige tu referencia personal en **Objetivo / LUFS**. Después de medir, se muestra la diferencia en dB para acercarte a ella. **Usar esta diferencia como ajuste manual** es opcional: solo cambia el volumen cuando tú lo pulsas. No se aplica un perfil automático ni una compresión dinámica para igualar niveles.
5. Vuelve a medir después de cambiar el corte, las fuentes o los volúmenes. Las mediciones anteriores quedan señaladas como desactualizadas y no se usan para proponer un ajuste nuevo.

El objetivo inicial de −18 LUFS es una referencia editable, no una exigencia de Twitch. Un fragmento muy corto o silencioso puede no producir una medición de volumen percibido válida. Si se necesita subir demasiado, la protección de picos puede limitar el resultado: escucha y mide de nuevo. La medición usa los valores de entrada de [loudnorm de FFmpeg](https://ffmpeg.org/ffmpeg-filters.html#loudnorm); el audio analizado que produciría ese filtro no se guarda.

### Elegir el sonido del video

En **Audio del WebM** elige una opción:

- **Mantener audio original**: conserva únicamente el sonido del video. Está disponible si el video tiene pista de audio.
- **Cortar sin audio**: genera el WebM sin pista de audio, aunque haya un archivo local seleccionado.
- **Reemplazar con archivo local**: descarta el sonido del video y utiliza el archivo que elijas. Puedes indicar desde qué segundo tomarlo.

Los controles de decibelios de cada fuente están dentro de **Volumen de cada fuente (avanzado)**, en la ventana de normalización. Solo se escucha la fuente elegida: original o archivo local.

## Video silencioso y mezcla de audio (versión 0.8)

Los WebM se exportan **sin sonido por defecto**, tanto al cortar como al convertir el archivo completo. En la pestaña Video / GIF, el panel **Audio del WebM** permite:

- **Agregar audio local…**: elige WAV, MP3, FLAC, OGG, M4A, AAC, Opus o WebM con audio. El sonido añadido se alinea con el inicio del corte, sin cambiar su duración.
- **Archivo local / dB** (en los controles avanzados de la nueva ventana): ajusta el volumen del archivo elegido entre −60 y +12 dB. En 0 dB conserva su nivel; valores negativos lo reducen y positivos lo aumentan.
- **Inicio del audio (s)**: elige desde qué segundo del archivo local se toma el sonido. Si el audio restante es más largo que el corte, se recorta; si es más corto, esa pista se completa con silencio. No se repite en bucle.
- **Mantener audio original**: conserva solo el sonido del video; se ignora cualquier archivo local que hayas seleccionado previamente.
- **Cortar sin audio**: desactiva todas las fuentes para el WebM. Puedes conservar el archivo local seleccionado y volver a usarlo después, sin que se cuele en una exportación silenciosa o con solo el original.

Los mismos controles aparecen en el diálogo de exportación. **Reproducir selección** permite ver y escuchar la mezcla integrada antes de guardar. Después de cambiar la selección, el archivo añadido o los volúmenes, reproduce otra vez para escuchar el resultado actualizado; las modificaciones de volumen detienen la reproducción que estuviera en curso. La pestaña Audio mantiene su comportamiento anterior.

La previsualización y el WebM usan los mismos ajustes de volumen y limitador de FFmpeg. El limitador reduce los picos elevados. Los decibelios son un ajuste relativo al archivo, no una medición de presión sonora física. Se genera una pista Opus con la fuente elegida cuando se incluye sonido.

## Reproducción integrada (versión 0.7)

Los videos y GIF se reproducen en movimiento dentro de **Vista previa de video**, en la misma ventana del editor. Ya no se abre una ventana externa. **Reproducir selección**, **Reproducir desde aquí**, **Detener**, **Marcar inicio** y **Marcar final** actúan sobre esta reproducción integrada. Desde la versión 0.8, el sonido depende de las fuentes que actives en **Audio del WebM**; la vista previa silenciosa no necesita un dispositivo de audio para mostrar la imagen.

La vista previa usa hasta 30 fps y una resolución reducida para mantenerla fluida. Esta reducción solo afecta a la previsualización; la exportación sigue utilizando el original y la resolución/calidad que elijas. Los fotogramas se sincronizan con el reloj de audio cuando existe, y con un reloj de reproducción propio cuando no hay sonido.

## Video, GIF y temas (versión 0.6)

Las pestañas **Audio** y **Video / GIF** aparecen en la cabecera. Cada una conserva su archivo, selección, zoom y controles mientras alternas entre ellas. El historial sigue siendo común y distingue Audio de Video. Los presets de 9 y 11 segundos, la duración libre, los pasos de precisión, sumar/quitar tiempo y los marcadores durante reproducción funcionan en ambos apartados.

### Editar video y convertir a WebM

1. Selecciona **Video / GIF** y pulsa **Abrir video o GIF**. Se admiten MP4, GIF animado, MKV, WebM, MOV, AVI y otros formatos que FFmpeg pueda decodificar. El filtro **Todos los archivos** permite elegir otros formatos compatibles.
2. Como alternativa, pega un enlace de YouTube y pulsa **Descargar video de YouTube**. Se descarga el video con su audio disponible y se carga para editarlo; los originales se conservan en `videos` dentro de la carpeta de datos de la aplicación.
3. La vista central muestra un fotograma del inicio de tu corte. Al mover el inicio, se actualiza. Los botones **Reproducir selección** y **Reproducir desde aquí** muestran el video en movimiento dentro de esa misma vista, con sonido cuando exista. Puedes usar **Marcar inicio** y **Marcar final** mientras se reproduce.
4. Aplica 9 u 11 segundos, cualquier otro preset o una duración personalizada. Los videos sin sonido y los GIF muestran una línea de tiempo sin onda de audio.
5. Pulsa **Guardar corte WebM…** para exportar la selección. Para convertir toda la animación o película, pulsa **Convertir todo a WebM…**.
6. Antes de elegir carpeta, selecciona la resolución: original, 2160p, 1440p, 1080p, 720p, 480p o 360p. Es una altura máxima: se conserva la proporción y no se amplía un original más pequeño. Las dimensiones se redondean a números pares para el codificador.
7. Elige **Más ligero**, **Equilibrado** o **Más calidad**. También puedes limitar la salida a 30 fps y conservar o quitar el sonido. Después pulsa **Elegir carpeta y guardar WebM…**.

El valor inicial es 720p máximo, calidad equilibrada, hasta 30 fps y sin audio. La salida usa [VP9](https://www.ffmpeg.org/ffmpeg-codecs.html#libvpx) con calidad constante (CRF 37 / 32 / 26 según el perfil) y, cuando añades o conservas sonido, Opus (64 / 96 / 128 kbit/s). Mayor resolución, movimiento, duración o calidad pueden aumentar el tamaño: no se garantiza un peso fijo ni un mínimo universal. La codificación de clips grandes puede tardar; **Cancelar operación** permite detenerla.

Los límites de video quedan condicionados por sus fotogramas: un paso de 1 ms permite ajustar el intervalo solicitado, pero no crea un fotograma entre dos existentes. La duración WebM puede variar ligeramente por el tiempo de fotograma y el retardo del audio. Los GIF se convierten por una vuelta de su animación, sin audio. Cuando se detecta transparencia, la exportación intenta conservarla en WebM VP9; su visualización depende del reproductor de destino.

### Personalizar el aspecto

Pulsa **Temas** en la cabecera. Puedes elegir **Noche**, **OLED**, **Océano**, **Rosa** o **Bosque**, personalizar el color de acento y ajustar el tamaño de la interfaz. Todos son oscuros. Las preferencias se guardan en `apariencia.json` junto al historial y se restauran al abrir el programa.

## Night Studio y marcadores en vivo (versión 0.5)

La interfaz usa un tema azul noche con tarjetas oscuras y acentos violeta y menta. La biblioteca, los presets y el historial están en el panel lateral derecho. La onda y los ajustes ocupan el centro. La barra inferior permanece visible con reproducción, reloj, marcadores y exportación. Los controles para mover por pasos se encuentran dentro de **Mover por pasos / inicio, final o selección**, en la tarjeta de ajuste fino.

Para seleccionar un fragmento mientras escuchas:

1. Elige el punto desde el que quieres buscar y pulsa **Escuchar desde aquí**. Este modo sigue hasta el final de la pista; **Escuchar selección** reproduce únicamente el intervalo seleccionado al pulsarlo.
2. Cuando llegue el momento deseado, pulsa **Marcar inicio**. Se coloca el inicio en la posición actual de reproducción y se conserva la duración que tenías, limitada por el final de la pista.
3. Cuando termine el fragmento que quieres, pulsa **Marcar final**. El final debe quedar después del inicio.
4. Los dos marcadores mantienen la reproducción en marcha. Pulsa **Detener** y después **Escuchar selección** para revisar el resultado. Puedes afinarlo con el zoom, los campos y los botones de milisegundos.

Los marcadores se habilitan cuando el reproductor entrega una posición válida. El reloj usa las estadísticas de audio de [ffplay](https://www.ffmpeg.org/ffplay.html), con actualización e interpolación de la posición; la latencia del dispositivo y la reacción al pulsar pueden introducir pequeñas diferencias. Para un ajuste de milisegundos, usa los controles de precisión después de marcar.

## Uso

1. Abre `Shiny Twitch Creator Tool Portable 1.1.exe`. Si usas la distribución anterior por carpeta, abre `Shiny Twitch Creator Tool.exe` y conserva su carpeta `tools`.
2. Pulsa **Abrir audio o M3U** y elige una pista WAV o MP3. También admite FLAC, OGG y M4A. Si abres una lista M3U/M3U8, elige su pista en el desplegable.
3. También puedes pegar un enlace de YouTube y pulsar **Descargar audio**. La aplicación descarga solo audio y lo carga automáticamente.
4. Pulsa **Alerta de Twitch · 9 segundos** o **Alerta de Twitch · 11 segundos** para aplicar una duración rápida. Puedes modificar libremente el inicio, el fin y la duración con los campos en segundos (tres decimales). Ya no estás limitado a esos dos tiempos.
5. Pulsa **Escuchar selección** para probar la alerta o **Escuchar desde aquí** para buscar el momento que quieres. **Detener** para la reproducción.
6. Elige WAV o MP3 y pulsa **Cortar y guardar…**. Selecciona carpeta y nombre en el diálogo de Windows.
7. Al terminar, la ruta aparece en el mensaje de estado (pasa el cursor para verla completa) y en el botón **Mostrar último corte**, que abre su carpeta. El historial conserva la pista original, el intervalo, la fecha y el destino entre sesiones.

La selección mantiene la duración elegida al acercarse al final de la pista. Si la pista completa dura menos que 9 u 11 segundos, la aplicación avisa y exporta su duración real, sin rellenar con silencio. El corte se recodifica para permitir inicios precisos; MP3 puede incluir el pequeño retardo propio de ese formato. WAV es la opción para duración exacta por muestras.

## Duraciones rápidas (versión 0.4)

- **Alerta de Twitch:** 9 u 11 segundos.
- **Ringtone:** 30 segundos.
- **Notificación:** 1, 1.5, 3, 4 o 5 segundos.

Cada botón aplica su duración al corte actual. Después puedes modificarla con los campos, arrastrando los extremos o con los botones para sumar y quitar milisegundos. Si el fragmento no cabe al final de la pista, se ajusta el inicio para conservar su duración; si la pista completa es más corta, se usa su duración real y aparece un aviso.

## Sumar y quitar tiempo al corte

En **Sumar o quitar tiempo al corte**, pulsa directamente **+1 ms**, **+5 ms**, **+100 ms**, **+500 ms** o **+1 min** para ampliar la duración. Los botones negativos la reducen. Cada clic conserva el inicio del fragmento y mueve únicamente su final, independientemente del objetivo seleccionado en los controles de pasos.

Por ejemplo, un corte de 9 segundos pasa a **9.001 s** con **+1 ms**, a **9.005 s** con **+5 ms**, a **9.100 s** con **+100 ms** o a **9.500 s** con **+500 ms**. También funciona con 11 segundos y con cualquier duración personalizada. Los clics se acumulan. Si se alcanza el final de la pista o la duración mínima, el programa lo indica y aplica solo el ajuste posible.

## Zoom y ajuste preciso

- **Acercar +** y **Alejar -** cambian el zoom de la onda hasta una ventana mínima de 50 ms. **Ver selección** enfoca el corte y **Ver toda la pista** recupera la vista general. La onda contiene picos por milisegundo y muestra una regla con los tiempos de la zona visible.
- **Explorar pista**, **Vista anterior** y **Vista siguiente** desplazan la vista sin alterar el corte.
- Arrastra las barras **INICIO** y **FIN** de la onda para acortar o ampliar la selección; arrastra su interior para moverla conservando su duración. Un clic coloca el inicio del corte.
- Selecciona un paso de **1 ms**, **5 ms**, **100 ms**, **500 ms** o **1 min** (60 segundos). Después elige **Inicio**, **Fin** o **Selección completa** y usa **Retroceder** o **Avanzar**.
- Ajustar Inicio conserva el fin; ajustar Fin conserva el inicio; mover la selección conserva su duración. Los límites no pueden cruzarse ni salir de la pista. La duración mínima es 1 ms, salvo pistas completas más cortas.
- Cambiar los extremos o la duración activa la duración personalizada. Los botones de 9 y 11 segundos siguen disponibles para volver a aplicar esos tiempos.

## Datos y límites

- En el ejecutable único, historial y descargas quedan en **Audio Twitch Cut Datos**, junto al exe. En la distribución por carpeta anterior se utiliza `%LOCALAPPDATA%`. Pasa el cursor sobre **Datos y descargas locales** en el panel lateral para ver la ruta exacta.
- Las descargas de audio de YouTube se conservan en `descargas`; las de video, en `videos`. Los cortes se guardan donde tú elijas.
- M3U es una lista, no un formato de audio. Se admiten listas UTF-8 con pistas locales y rutas absolutas o relativas a la lista. Las listas con enlaces de red no están admitidas en esta versión.
- Archivos de hasta cuatro horas. Las descargas tienen un tiempo máximo de 15 minutos y un límite solicitado de 500 MB para audio o 2 GB para video cuando el servidor informa el tamaño. Cada stream descargado puede tener su propio tamaño; el límite no equivale a un límite garantizado del total combinado.
- YouTube requiere conexión. Videos privados, protegidos, restringidos, transmisiones en directo o cambios del servicio pueden impedir una descarga. No se incluyen cookies ni acceso a cuentas. Actualiza yt-dlp si el servicio cambia.
- Usa **Cancelar operación** para detener cargas, descargas o exportaciones. Si intentas cerrar mientras hay una operación, se solicita su cancelación; vuelve a cerrar cuando termine.

## Compilar

Requisitos: Windows 10/11 de 64 bits, Rust estable con Cargo y Visual Studio Build Tools con C++ y Windows SDK.

```powershell
powershell -ExecutionPolicy Bypass -File scripts/setup-media.ps1
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Check
powershell -ExecutionPolicy Bypass -File scripts/build.ps1 -Test
powershell -ExecutionPolicy Bypass -File scripts/build.ps1
```

El resultado portable queda en `dist/Shiny Twitch Creator Tool/`. Copia **toda esa carpeta** para usarlo en otro equipo. No necesita Rust instalado para ejecutarse.

Durante el desarrollo, las herramientas se buscan en `.tools/media`, junto al ejecutable, en `tools` junto al ejecutable o en PATH. El script de compilación también reconoce un Rust local en `.tools/cargo` y `.tools/rustup`.

## Tecnología

- Interfaz nativa: [eframe/egui](https://docs.rs/eframe/0.29.1/eframe/).
- Análisis, onda y exportación: [FFmpeg](https://ffmpeg.org/ffmpeg.html), ffprobe y ffplay.
- Descarga de audio: [yt-dlp](https://github.com/yt-dlp/yt-dlp).

Consulta `THIRD-PARTY.md` para los componentes de terceros.
