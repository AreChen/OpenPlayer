param([string] $ArchivePath)

$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$manifest = Get-Content -LiteralPath (Join-Path $repoRoot 'docs/native-deps/mpv-windows-x64.json') | ConvertFrom-Json
$targetRoot = Join-Path $repoRoot 'vendor/native/mpv/windows-x64'
$runtimePath = Join-Path $targetRoot 'libmpv-2.dll'
$missing = $manifest.requiredFiles | Where-Object { -not (Test-Path -LiteralPath (Join-Path $targetRoot $_) -PathType Leaf) }
if (-not $missing -and (Get-FileHash -Algorithm SHA256 -LiteralPath $runtimePath).Hash -eq $manifest.runtimeSha256) { return }

$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('openplayer-mpv-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
try {
    $archive = $ArchivePath
    if (-not $archive) {
        $archive = Join-Path $tempRoot $manifest.asset
        Invoke-WebRequest -Uri $manifest.url -OutFile $archive
    }
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash
    if ($hash -ne $manifest.sha256) { throw "mpv archive SHA256 mismatch for $($manifest.release)" }

    $extractRoot = Join-Path $tempRoot 'extracted'
    Expand-Archive -LiteralPath $archive -DestinationPath $extractRoot
    foreach ($name in $manifest.requiredFiles) {
        if (-not (Test-Path -LiteralPath (Join-Path $extractRoot $name) -PathType Leaf)) { throw "mpv archive missing $name" }
    }
    $newRuntime = Join-Path $extractRoot 'libmpv-2.dll'
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $newRuntime).Hash -ne $manifest.runtimeSha256) { throw 'mpv runtime DLL SHA256 mismatch' }

    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $extractRoot 'include') -Destination $targetRoot -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $extractRoot 'libmpv.dll.a'), (Join-Path $extractRoot 'LICENSE.LGPL') -Destination $targetRoot -Force
    $stagedRuntime = Join-Path $targetRoot ('.libmpv-' + [guid]::NewGuid() + '.dll')
    try {
        Copy-Item -LiteralPath $newRuntime -Destination $stagedRuntime
        [IO.File]::Move($stagedRuntime, $runtimePath, $true)
    } finally {
        if (Test-Path -LiteralPath $stagedRuntime) { Remove-Item -LiteralPath $stagedRuntime -Force }
    }
    Write-Output "Restored Windows libmpv $($manifest.release)"
} finally {
    $resolvedTemp = [System.IO.Path]::GetFullPath($tempRoot)
    $expectedParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedTemp.StartsWith($expectedParent, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected temporary directory' }
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
}
