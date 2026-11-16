param([switch]$Check, [switch]$Test, [switch]$YouTubeTest, [string]$OutputFolder = 'Shiny Twitch Creator Tool', [string]$Target = '')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $projectRoot
$localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
if (Test-Path -LiteralPath $localCargo) {
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $cargoExe = $localCargo
} else {
    $cargoExe = (Get-Command cargo -ErrorAction Stop).Source
}
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (Test-Path -LiteralPath $vswhere) {
    $vsPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsPath) {
        $vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
        $envLines = & cmd.exe /d /c "`"$vcvars`" >nul && set"
        foreach ($line in $envLines) {
            if ($line -match '^([^=]+)=(.*)$') { [Environment]::SetEnvironmentVariable($matches[1], $matches[2], 'Process') }
        }
    }
}
$localSdk = Join-Path $projectRoot '.tools\windows-sdk-x64\c'
if (Test-Path -LiteralPath $localSdk) {
    $env:LIB = "$(Join-Path $localSdk 'um\x64');$(Join-Path $localSdk 'ucrt\x64');$env:LIB"
}
if ($Target -and -not $Check) { throw 'Cross-target support in this Windows script is limited to -Check. Build Linux packages on Linux.' }
if ($Check) {
    if ($Target) { & $cargoExe check --locked --target $Target } else { & $cargoExe check --locked }
}
elseif ($YouTubeTest) { & $cargoExe test --locked live_youtube_audio_and_video_download -- --ignored }
elseif ($Test) { & $cargoExe test --locked }
else {
    & $cargoExe build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
    if ($OutputFolder -match '[\\/]' -or $OutputFolder -in @('.', '..')) { throw 'OutputFolder must be a folder name.' }
    $dist = Join-Path (Join-Path $projectRoot 'dist') $OutputFolder
    New-Item -ItemType Directory -Force -Path $dist | Out-Null
    Copy-Item -LiteralPath (Join-Path $projectRoot 'target\release\audio-twitch-cut.exe') -Destination (Join-Path $dist 'Shiny Twitch Creator Tool.exe') -Force
    Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md'), (Join-Path $projectRoot 'THIRD-PARTY.md'), (Join-Path $projectRoot 'LINUX.md'), (Join-Path $projectRoot 'RELEASE_NOTES.md') -Destination $dist -Force
    if (Test-Path -LiteralPath (Join-Path $projectRoot 'VALIDATION.md')) { Copy-Item -LiteralPath (Join-Path $projectRoot 'VALIDATION.md') -Destination $dist -Force }
    $media = Join-Path $projectRoot '.tools\media'
    if (Test-Path -LiteralPath $media) {
        $toolsDest = Join-Path $dist 'tools'
        New-Item -ItemType Directory -Force -Path $toolsDest | Out-Null
        Get-ChildItem -LiteralPath $media -File | ForEach-Object { Copy-Item -LiteralPath $_.FullName -Destination $toolsDest -Force }
    } else { Write-Warning 'Media tools are missing. Run scripts/setup-media.ps1 before using the app.' }
    Write-Host "Portable app: $dist"
}
if ($LASTEXITCODE -ne 0) { throw 'Validation failed.' }
