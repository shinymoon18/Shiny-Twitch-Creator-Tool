$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
$mediaDir = Join-Path $projectRoot '.tools\media'
New-Item -ItemType Directory -Force -Path $mediaDir | Out-Null
$archivePath = Join-Path $projectRoot '.tools\ffmpeg.zip'
$extractDir = Join-Path $projectRoot '.tools\ffmpeg-unpacked'
if (-not (Test-Path (Join-Path $mediaDir 'ffmpeg.exe')) -or -not (Test-Path (Join-Path $mediaDir 'ffprobe.exe')) -or -not (Test-Path (Join-Path $mediaDir 'ffplay.exe'))) {
    Write-Host 'Downloading FFmpeg (Gyan build linked from ffmpeg.org)...'
    & curl.exe -L --fail --retry 2 --output $archivePath 'https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip'
    if ($LASTEXITCODE -ne 0) { throw 'Could not download FFmpeg.' }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $extractDir -Force
    foreach ($name in @('ffmpeg.exe', 'ffprobe.exe', 'ffplay.exe')) {
        $file = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter $name | Select-Object -First 1
        if (-not $file) { throw "Missing $name in the download." }
        Copy-Item -LiteralPath $file.FullName -Destination (Join-Path $mediaDir $name) -Force
    }
    $license = Get-ChildItem -LiteralPath $extractDir -Recurse -Filter LICENSE | Select-Object -First 1
    if ($license) { Copy-Item -LiteralPath $license.FullName -Destination (Join-Path $mediaDir 'FFmpeg-LICENSE.txt') -Force }
}
Write-Host 'Downloading yt-dlp from its official repository...'
& curl.exe -L --fail --retry 2 --output (Join-Path $mediaDir 'yt-dlp.exe') 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe'
if ($LASTEXITCODE -ne 0) { throw 'Could not download yt-dlp.' }
& curl.exe -L --fail --retry 2 --output (Join-Path $mediaDir 'yt-dlp-LICENSE.txt') 'https://raw.githubusercontent.com/yt-dlp/yt-dlp/master/LICENSE'
if ($LASTEXITCODE -ne 0) { throw 'Could not download the yt-dlp license.' }
if (-not (Test-Path (Join-Path $mediaDir 'deno.exe'))) {
    Write-Host 'Downloading Deno for yt-dlp YouTube support...'
    $denoArchive = Join-Path $projectRoot '.tools\deno.zip'
    & curl.exe -L --fail --retry 2 --output $denoArchive 'https://github.com/denoland/deno/releases/latest/download/deno-x86_64-pc-windows-msvc.zip'
    if ($LASTEXITCODE -ne 0) { throw 'Could not download Deno.' }
    Expand-Archive -LiteralPath $denoArchive -DestinationPath $mediaDir -Force
    & curl.exe -L --fail --retry 2 --output (Join-Path $mediaDir 'Deno-LICENSE.txt') 'https://raw.githubusercontent.com/denoland/deno/main/LICENSE.md'
    if ($LASTEXITCODE -ne 0) { throw 'Could not download the Deno license.' }
}
Write-Host "Tools ready in $mediaDir"
