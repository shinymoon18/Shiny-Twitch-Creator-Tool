$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot

# Embed the C runtime so the editor does not need Visual C++ installed.
$previousFlags = $env:RUSTFLAGS
try {
    $env:RUSTFLAGS = "$previousFlags -C target-feature=+crt-static".Trim()
    & (Join-Path $PSScriptRoot 'build.ps1') -OutputFolder 'Shiny Twitch Creator Tool 1.2.1'
} finally { $env:RUSTFLAGS = $previousFlags }

$partsDir = Join-Path $projectRoot '.tools\portable-parts'
New-Item -ItemType Directory -Force -Path $partsDir | Out-Null
$inputs = @(Get-ChildItem -LiteralPath (Join-Path $projectRoot '.tools\media') -File)
foreach ($name in @('ffmpeg.exe', 'ffprobe.exe', 'ffplay.exe', 'yt-dlp.exe', 'deno.exe')) {
    if (-not ($inputs | Where-Object Name -eq $name)) { throw "Missing tool $name. Run setup-media.ps1." }
}
$inputs += @(Get-Item -LiteralPath (Join-Path $projectRoot 'README.md'), (Join-Path $projectRoot 'THIRD-PARTY.md'), (Join-Path $projectRoot 'VALIDATION.md'))
$entries = @()
foreach ($item in $inputs) {
    $packedPath = Join-Path $partsDir ($item.Name + '.gz')
    Write-Host "Bundling $($item.Name)..."
    $source = [System.IO.File]::OpenRead($item.FullName)
    try {
        $output = [System.IO.File]::Create($packedPath)
        try {
            $gzip = New-Object System.IO.Compression.GZipStream($output, [System.IO.Compression.CompressionLevel]::Optimal, $true)
            try { $source.CopyTo($gzip) } finally { $gzip.Dispose() }
        } finally { $output.Dispose() }
    } finally { $source.Dispose() }
    $entries += [ordered]@{ name = $item.Name; size = $item.Length; packed = (Get-Item -LiteralPath $packedPath).Length }
}
$manifest = [System.Text.Encoding]::UTF8.GetBytes((@{ files = @($entries) } | ConvertTo-Json -Compress -Depth 5))
$payloadPath = Join-Path $projectRoot '.tools\portable-payload.bin'
$payload = [System.IO.File]::Create($payloadPath)
try {
    $sizeBytes = [BitConverter]::GetBytes([UInt64]$manifest.Length)
    $payload.Write($sizeBytes, 0, $sizeBytes.Length)
    $payload.Write($manifest, 0, $manifest.Length)
    foreach ($entry in $entries) {
        $part = [System.IO.File]::OpenRead((Join-Path $partsDir ($entry.name + '.gz')))
        try { $part.CopyTo($payload) } finally { $part.Dispose() }
    }
} finally { $payload.Dispose() }

$hash = (Get-FileHash -LiteralPath $payloadPath -Algorithm SHA256).Hash
$hashBytes = New-Object byte[] 32
for ($index = 0; $index -lt 32; $index++) { $hashBytes[$index] = [Convert]::ToByte($hash.Substring($index * 2, 2), 16) }
$destination = Join-Path $projectRoot 'dist\Shiny-Twitch-Creator-Tool-Portable-1.2.1.exe'
Copy-Item -LiteralPath (Join-Path $projectRoot 'target\release\audio-twitch-cut.exe') -Destination $destination -Force
$exe = [System.IO.File]::Open($destination, [System.IO.FileMode]::Append, [System.IO.FileAccess]::Write)
try {
    $payload = [System.IO.File]::OpenRead($payloadPath)
    try {
        $payload.CopyTo($exe)
        $sizeBytes = [BitConverter]::GetBytes([UInt64]$payload.Length)
        $exe.Write($sizeBytes, 0, $sizeBytes.Length)
    } finally { $payload.Dispose() }
    $exe.Write($hashBytes, 0, $hashBytes.Length)
    $magic = [System.Text.Encoding]::ASCII.GetBytes('ATCUT_BUNDLE_001')
    $exe.Write($magic, 0, $magic.Length)
} finally { $exe.Dispose() }
Write-Host "Single-file executable ready: $destination"
Get-Item -LiteralPath $destination | Select-Object FullName, Length
