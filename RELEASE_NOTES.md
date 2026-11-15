# Shiny Twitch Creator Tool 1.1

Herramienta portable para Windows 10/11 de 64 bits para preparar alertas de Twitch con audio, video y GIF.

## Cambios de esta versión

- El audio local sustituye completamente el sonido original del video.
- Editor del audio añadido con onda, zoom, reproducción, marcadores, inicio/final y pasos de precisión.
- Solo se incluye el intervalo elegido del audio; el video conserva su duración.
- Se mantienen las opciones de conservar el audio original o exportar sin sonido.

## Descargar y usar

Descarga `Shiny Twitch Creator Tool Portable 1.1.exe` y ábrelo desde una carpeta donde tengas permiso de escritura. No necesitas instalar Rust, FFmpeg ni yt-dlp.

La primera apertura prepara las herramientas y crea `Audio Twitch Cut Datos` junto al ejecutable. Para llevar el programa a otro PC, copia el exe. Para llevar también historial y preferencias, copia además esa carpeta. Las funciones de YouTube necesitan conexión.

## Verificación

Las 21 pruebas del proyecto pasan, incluidas comprobaciones reales de exportación y de sustitución y recorte del audio añadido. Los componentes y avisos de terceros están documentados en `THIRD-PARTY.md` y se incluyen en el paquete portable.
