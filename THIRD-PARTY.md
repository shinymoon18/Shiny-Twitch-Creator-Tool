# Componentes de terceros

La versión de ejecutable único incluye estos componentes comprimidos y prepara las herramientas y sus licencias junto a los datos portables durante la primera ejecución.

Shiny Twitch Creator Tool utiliza bibliotecas Rust cuyas versiones quedan registradas en Cargo.lock y herramientas externas.

- egui / eframe: MIT o Apache-2.0. https://github.com/emilk/egui
- rfd: MIT. https://github.com/PolyMeilex/rfd
- serde / serde_json: MIT o Apache-2.0. https://github.com/serde-rs
- directories: MIT o Apache-2.0. https://github.com/dirs-dev/directories-rs
- tempfile: MIT o Apache-2.0. https://github.com/Stebalien/tempfile
- chrono: MIT o Apache-2.0. https://github.com/chronotope/chrono
- flate2: MIT o Apache-2.0. https://github.com/rust-lang/flate2-rs. Descompresión del paquete portable.
- Deno: MIT y avisos de terceros incluidos en su licencia. https://github.com/denoland/deno. Se incluye como motor JavaScript requerido por yt-dlp para YouTube.

El script descarga FFmpeg de Gyan, una distribución enlazada en https://ffmpeg.org/download.html. Las compilaciones essentials incluyen componentes GPL: consulta la licencia incluida junto a las herramientas y los detalles de la distribución en https://www.gyan.dev/ffmpeg/builds/. Código fuente de FFmpeg: https://ffmpeg.org/download.html. FFmpeg se ejecuta como proceso separado.

yt-dlp se descarga de https://github.com/yt-dlp/yt-dlp/releases. Su código principal se distribuye bajo Unlicense; los ejecutables empaquetados contienen componentes con sus propias licencias. Consulta https://github.com/yt-dlp/yt-dlp#license y el archivo de licencia incluido en tools.

Antes de redistribuir las herramientas binarias, revisa sus condiciones, avisos y obligaciones de entrega de código fuente correspondientes a la versión concreta distribuida.
