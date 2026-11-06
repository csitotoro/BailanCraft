# BailanCraft: regenerate the app icons on Windows from the master vector
# assets/app-icon/pdfcraft.svg (the palm-leaf bundle). Same outputs and file names as
# packaging/icons.sh, so no code changes when the artwork changes; it needs only resvg
# (cargo install resvg), not python3. The macOS .icns is not regenerated here.
#
#   powershell -ExecutionPolicy Bypass -File packaging/bailancraft-icons.ps1
#
# Afterwards update the sha256 values in ATTRIBUTION.toml for the changed files.
$ErrorActionPreference = 'Stop'
$Root = Split-Path -Parent $PSScriptRoot
$Dir = Join-Path $Root 'assets\app-icon'
$Svg = Join-Path $Dir 'pdfcraft.svg'
$Id = 'ai.storyteller.pdfcraft'
$Resvg = (Get-Command resvg -ErrorAction SilentlyContinue).Source
if (-not $Resvg) { $Resvg = Join-Path $env:USERPROFILE '.cargo\bin\resvg.exe' }
if (-not (Test-Path $Resvg)) { throw 'resvg not found: cargo install resvg' }
$Tmp = Join-Path ([IO.Path]::GetTempPath()) ("bailancraft-icons-" + [guid]::NewGuid())
New-Item -ItemType Directory -Force $Tmp | Out-Null

function Render([string]$src, [int]$size, [string]$out) {
    & $Resvg -w $size -h $size $src $out
    if ($LASTEXITCODE -ne 0) { throw "resvg failed for $out" }
}

# Windows .ico with PNG-compressed entries (Vista+), first entry is the default.
function Write-Ico([string]$out, [string[]]$pngs) {
    $images = $pngs | ForEach-Object { , [IO.File]::ReadAllBytes($_) }
    $ms = New-Object IO.MemoryStream
    $w = New-Object IO.BinaryWriter($ms)
    $w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$images.Count)
    $offset = 6 + 16 * $images.Count
    foreach ($png in $images) {
        # PNG IHDR width/height are big-endian at bytes 16..23.
        $width = ($png[16] -shl 24) -bor ($png[17] -shl 16) -bor ($png[18] -shl 8) -bor $png[19]
        $height = ($png[20] -shl 24) -bor ($png[21] -shl 16) -bor ($png[22] -shl 8) -bor $png[23]
        $w.Write([byte]($width % 256)); $w.Write([byte]($height % 256))
        $w.Write([byte]0); $w.Write([byte]0)
        $w.Write([UInt16]1); $w.Write([UInt16]32)
        $w.Write([UInt32]$png.Length); $w.Write([UInt32]$offset)
        $offset += $png.Length
    }
    foreach ($png in $images) { $w.Write($png) }
    $w.Flush()
    [IO.File]::WriteAllBytes($out, $ms.ToArray())
}

try {
    # 1024 px on Apple's grid: an 824 px body centred on a transparent canvas (also the runtime icon).
    # The master is nested as an inner <svg> (resvg won't load an <image> by file: URL).
    $mac = Join-Path $Tmp 'macos.svg'
    $master = [IO.File]::ReadAllText($Svg)
    $inner = [regex]::Replace($master, '^\s*<svg\b', '<svg x="100" y="100" width="824" height="824"', 1)
    if ($inner -eq $master) { throw 'pdfcraft.svg must start with <svg' }
    [IO.File]::WriteAllText($mac, '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">' + $inner + '</svg>')
    Render $mac 1024 (Join-Path $Dir 'pdfcraft-1024.png')
    foreach ($s in 16, 24, 32, 48, 64, 128, 256, 512) {
        $d = Join-Path $Dir "hicolor\${s}x${s}\apps"
        New-Item -ItemType Directory -Force $d | Out-Null
        Render $Svg $s (Join-Path $d "$Id.png")
    }
    Copy-Item -Force $Svg (Join-Path $Dir "hicolor\scalable\apps\$Id.svg")
    Copy-Item -Force $Svg (Join-Path $Dir 'pdfcraft-small.svg')
    $pngs = foreach ($s in 256, 128, 64, 48, 40, 32, 24, 20, 16) {
        $p = Join-Path $Tmp "app-$s.png"; Render $Svg $s $p; $p
    }
    Write-Ico (Join-Path $Dir 'pdfcraft.ico') $pngs
    Write-Output 'icons regenerated'
} finally {
    Remove-Item -Recurse -Force $Tmp
}
