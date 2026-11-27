[CmdletBinding()]
param([string]$Makensis, [switch]$SkipBuild)

$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
$desktop = Join-Path $root 'apps/desktop'
$source = Join-Path $desktop 'src-tauri'
$mpv = Join-Path $root 'vendor/native/mpv/windows-x64'
$version = (Get-Content -Raw -LiteralPath (Join-Path $desktop 'package.json') | ConvertFrom-Json).version
if ($version -notmatch '^\d+\.\d+\.\d+$') { throw "Invalid version $version" }

if (-not $Makensis) {
    $command = Get-Command makensis -ErrorAction SilentlyContinue
    $Makensis = @(
        $command.Source
        (Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'NSIS/makensis.exe')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
}
if (-not $Makensis) { throw 'Install NSIS or pass its makensis.exe path with -Makensis.' }

Push-Location -LiteralPath $root
try {
    if (-not $SkipBuild) {
        $rustflags = $env:RUSTFLAGS
        $encodedRustflags = $env:CARGO_ENCODED_RUSTFLAGS
        $staticCrt = '-Ctarget-feature=+crt-static'
        try {
            # An installed player must start on Windows without a separate Visual C++ runtime.
            if ($null -ne $encodedRustflags) {
                $env:CARGO_ENCODED_RUSTFLAGS = if ($encodedRustflags) {
                    $encodedRustflags + [char]0x1F + $staticCrt
                } else {
                    $staticCrt
                }
            } else {
                $env:RUSTFLAGS = "$rustflags $staticCrt".Trim()
            }
            cargo build --locked --release -p openplayer-desktop --bin openplayer-desktop
            if ($LASTEXITCODE -ne 0) { throw "Cargo build failed with exit code $LASTEXITCODE" }
        } finally {
            $env:RUSTFLAGS = $rustflags
            $env:CARGO_ENCODED_RUSTFLAGS = $encodedRustflags
        }
    }
    $target = if ($env:CARGO_TARGET_DIR) { (Resolve-Path -LiteralPath $env:CARGO_TARGET_DIR).Path } else { Join-Path $root 'target' }
    $app = Join-Path $target 'release/openplayer-desktop.exe'
    $bundle = (New-Item -ItemType Directory -Force -Path (Join-Path $target 'release/bundle/nsis')).FullName
    $installer = Join-Path $bundle "OpenPlayer_$($version)_x64-setup.exe"

    # Generate both registration lists from the Rust media catalog.
    $catalog = Get-Content -Raw -LiteralPath (Join-Path $source 'src/media_paths/extensions.rs')
    $list = [regex]::Match($catalog, '(?s)const SUPPORTED_MEDIA_EXTENSIONS:[^=]+=\s*&\[(?<exts>.*?)\];')
    $extensions = [regex]::Matches($list.Groups['exts'].Value, '"([a-z0-9-]+)"') |
        ForEach-Object { $_.Groups[1].Value }
    if (-not $extensions) { throw 'Unable to read the OpenPlayer media extension catalog.' }
    $associations = Join-Path $bundle 'media-associations.nsh'
    $lines = @('!macro RegisterMediaExtensions')
    $lines += $extensions | ForEach-Object { '    !insertmacro RegisterMediaExtension "' + $_ + '"' }
    $lines += '!macroend'
    $lines += '!macro UnregisterMediaExtensions'
    $lines += $extensions | ForEach-Object { '    !insertmacro UnregisterMediaExtension "' + $_ + '"' }
    $lines += '!macroend'
    $lines | Set-Content -LiteralPath $associations -Encoding ascii

    $arguments = @(
        "/DPRODUCT_VERSION=$version"
        "/DPLAYER_EXE=$app"
        "/DMPV_DLL=$(Join-Path $mpv 'libmpv-2.dll')"
        "/DMPV_LICENSE=$(Join-Path $mpv 'LICENSE.LGPL')"
        "/DAPP_ICON=$(Join-Path $source 'icons/icon.ico')"
        "/DMEDIA_ASSOCIATIONS=$associations"
        "/DINSTALLER=$installer"
        (Join-Path $PSScriptRoot 'openplayer-windows.nsi')
    )
    & $Makensis @arguments
    if ($LASTEXITCODE -ne 0) { throw "NSIS failed with exit code $LASTEXITCODE" }
    Get-Item -LiteralPath $installer
} finally {
    Pop-Location
}
