$ErrorActionPreference = 'Stop'
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
$manifest = Get-Content -LiteralPath (Join-Path $repoRoot 'docs/native-deps/mpv-windows-x64.json') | ConvertFrom-Json
$targetRoot = Join-Path $repoRoot 'vendor/native/mpv/windows-x64'
if ((Test-Path -LiteralPath (Join-Path $targetRoot 'libmpv-2.dll')) -and (Test-Path -LiteralPath (Join-Path $targetRoot 'libmpv.dll.a'))) { return }
$tempRoot = Join-Path ([System.IO.Path]::GetTempPath()) ('openplayer-mpv-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tempRoot -Force | Out-Null
try {
    $archive = Join-Path $tempRoot $manifest.asset
    Invoke-WebRequest -Uri $manifest.url -OutFile $archive
    $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    if ($hash -ne $manifest.sha256) { throw 'mpv archive SHA256 mismatch' }
    $extractRoot = Join-Path $tempRoot 'extracted'
    & 7z x $archive "-o$extractRoot" -y | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'mpv archive extraction failed' }
    New-Item -ItemType Directory -Path $targetRoot -Force | Out-Null
    foreach ($name in @('libmpv-2.dll', 'libmpv.dll.a')) {
        $file = Get-ChildItem -LiteralPath $extractRoot -Recurse -File -Filter $name | Select-Object -First 1
        if (-not $file) { throw "mpv archive missing $name" }
        Copy-Item -LiteralPath $file.FullName -Destination $targetRoot -Force
    }
    $include = Get-ChildItem -LiteralPath $extractRoot -Recurse -Directory -Filter 'include' | Select-Object -First 1
    if (-not $include) { throw 'mpv archive missing headers' }
    Copy-Item -LiteralPath $include.FullName -Destination $targetRoot -Recurse -Force
} finally {
    $resolvedTemp = [System.IO.Path]::GetFullPath($tempRoot)
    $expectedParent = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\', '/') + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolvedTemp.StartsWith($expectedParent, [System.StringComparison]::OrdinalIgnoreCase)) { throw 'Unexpected temporary directory' }
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
}
