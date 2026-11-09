<#
.SYNOPSIS
  BailanCraft: build the Windows x64 installer (.msi) and a portable .zip.

.DESCRIPTION
  Produces, in dist\bailancraft:
    bailancraft-<version>-windows-x64.msi            per-machine installer (WiX v5)
    bailancraft-<version>-windows-x64-portable.zip   the same files, run from any folder

  Both hold BailanCraft.exe and bailancraft-cli.exe (the pdfcraft binaries, renamed), the OCR
  models, and Tesseract with English and Thai language data for Thai OCR. The binaries link the C
  runtime statically, so no Visual C++ redistributable is needed.

  Needs: Rust (MSVC), Visual Studio Build Tools (dumpbin), WiX v5 (dotnet tool install --global
  wix --version 5.0.2), CRAFT_FONTS_DIR and CRAFT_I18N_DIR (the Thai font and UI translations),
  Tesseract installed (winget install UB-Mannheim.TesseractOCR), and tha/eng.traineddata from
  tesseract-ocr/tessdata_best in -TessdataDir.

.EXAMPLE
  $env:CRAFT_FONTS_DIR = '..\craft-fonts'; $env:CRAFT_I18N_DIR = '..\craft-i18n'
  powershell -ExecutionPolicy Bypass -File packaging\windows\bailancraft-package.ps1 -Manufacturer 'My Company'
#>
param(
  [switch] $SkipBuild,
  # Shown as the publisher in Apps & features and in the MSI's properties.
  [string] $Manufacturer = 'LANNAFORGE',
  [string] $TesseractDir = 'C:\Program Files\Tesseract-OCR',
  [string] $TessdataDir,
  # The ocrs models (text-detection.rten, text-recognition.rten, their licences, ATTRIBUTION.txt).
  # Default: an installed PdfCraft's models folder; else run `cargo xtask models <dir>` first.
  [string] $ModelsDir
)
$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
if (-not $TessdataDir) { $TessdataDir = Join-Path (Split-Path $Root -Parent) 'BailanCraft-deps\tessdata' }
if (-not $ModelsDir) { $ModelsDir = 'C:\Program Files\PdfCraft\models' }
foreach ($need in @(@('CRAFT_FONTS_DIR', 'the Thai font'), @('CRAFT_I18N_DIR', 'the Thai interface'))) {
  if (-not [Environment]::GetEnvironmentVariable($need[0])) { throw "$($need[0]) is not set: without it the build has no $($need[1])" }
}
$Wix = (Get-Command wix -ErrorAction SilentlyContinue).Source
if (-not $Wix) { $Wix = Join-Path $env:USERPROFILE '.dotnet\tools\wix.exe' }
if (-not (Test-Path $Wix)) { throw 'WiX not found: dotnet tool install --global wix --version 5.0.2' }
if (-not (Test-Path (Join-Path $TesseractDir 'tesseract.exe'))) { throw "no tesseract.exe in $TesseractDir" }
foreach ($l in 'tha', 'eng') {
  if (-not (Test-Path (Join-Path $TessdataDir "$l.traineddata"))) { throw "no $l.traineddata in $TessdataDir (tesseract-ocr/tessdata_best)" }
}
if (-not (Test-Path (Join-Path $ModelsDir 'ATTRIBUTION.txt'))) { throw "no OCR models in $ModelsDir (cargo xtask models <dir>, then -ModelsDir <dir>)" }

function Invoke-Native([string] $What, [scriptblock] $Block) {
  Write-Output "==> $What"
  & $Block
  if ($LASTEXITCODE -ne 0) { throw "$What failed with exit code $LASTEXITCODE" }
}

# The version: [workspace.package] version in Cargo.toml (MSI versions are numeric).
$Version = $null
$inPkg = $false
foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
  if ($line -match '^\s*\[') { $inPkg = ($line.Trim() -eq '[workspace.package]'); continue }
  if ($inPkg -and $line -match '^\s*version\s*=\s*"([^"]+)"') { $Version = $Matches[1]; break }
}
if (-not $Version) { throw 'could not read [workspace.package] version from Cargo.toml' }
$MsiVersion = ($Version -split '-')[0]
$Target = 'x86_64-pc-windows-msvc'
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
$Dist = Join-Path $Root 'dist\bailancraft'
New-Item -ItemType Directory -Force -Path $Dist | Out-Null
Write-Output "BailanCraft $Version for Windows x64"

if (-not $SkipBuild) {
  # Static C runtime: no Visual C++ redistributable on the users' machines.
  $env:CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS = '-C target-feature=+crt-static'
  $env:PDFCRAFT_REQUIRE_WINRES = '1'
  $env:CRAFT_FONTS_REQUIRED = '1'
  $env:CRAFT_I18N_REQUIRED = '1'
  if (-not $env:PDFCRAFT_BUILD_SHA) { $env:PDFCRAFT_BUILD_SHA = (git -C $Root rev-parse HEAD 2>$null) }
  if (-not $env:PDFCRAFT_BUILD_DATE) { $env:PDFCRAFT_BUILD_DATE = (Get-Date).ToUniversalTime().ToString('yyyy-MM-dd') }
  Invoke-Native "cargo build ($Target)" { cargo build --release --locked -p pdfcraft -p pdfcraft-cli --target $Target -j 8 }
}
$Bin = Join-Path $TargetDir "$Target\release"

# Stage everything the MSI and the zip hold.
$Stage = Join-Path $TargetDir 'bailancraft-package\stage'
Remove-Item -Recurse -Force $Stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $Stage | Out-Null
Copy-Item (Join-Path $Bin 'pdfcraft.exe') (Join-Path $Stage 'BailanCraft.exe')
Copy-Item (Join-Path $Bin 'pdfcraft-cli.exe') (Join-Path $Stage 'bailancraft-cli.exe')
foreach ($f in 'LICENSE-MIT', 'LICENSE-APACHE', 'NOTICE', 'README.md') { Copy-Item (Join-Path $Root $f) $Stage }
Copy-Item (Join-Path $Root 'assets\app-icon\LICENSE-document.txt') $Stage
& (Join-Path $PSScriptRoot 'bailancraft-sign.ps1') (Join-Path $Stage 'BailanCraft.exe') (Join-Path $Stage 'bailancraft-cli.exe')

$Models = Join-Path $Stage 'models'
New-Item -ItemType Directory -Force -Path $Models | Out-Null
Copy-Item (Join-Path $ModelsDir '*') $Models -Exclude '*.part'

# The OFL notices of the craft-fonts faces the build embeds (Noto Sans Thai and the rest).
$Docs = Join-Path $Stage 'font-licences'
New-Item -ItemType Directory -Force -Path $Docs | Out-Null
. (Join-Path $PSScriptRoot 'font-notices.ps1')
$HasCraftFontLicences = 0
foreach ($ofl in @(Get-CraftFontNotices $env:CRAFT_FONTS_DIR $false)) {
  Copy-Item -LiteralPath $ofl.FullName -Destination (Join-Path $Docs "OFL-$($ofl.Directory.Name).txt")
  $HasCraftFontLicences = 1
}

# Tesseract: tesseract.exe and the DLLs it loads (found with dumpbin, transitively), its data and
# configs, and the licences.
$Tess = Join-Path $Stage 'tesseract'
New-Item -ItemType Directory -Force -Path (Join-Path $Tess 'tessdata'), (Join-Path $Tess 'licences') | Out-Null
$vs = & "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe" -products * -property installationPath | Select-Object -First 1
$Dumpbin = Get-ChildItem "$vs\VC\Tools\MSVC" -Recurse -Filter dumpbin.exe | Where-Object { $_.FullName -match 'Hostx64\\x64' } | Select-Object -First 1 -ExpandProperty FullName
if (-not $Dumpbin) { throw 'dumpbin.exe not found (Visual Studio Build Tools, C++)' }
$local = @{}
Get-ChildItem (Join-Path $TesseractDir '*.dll') | ForEach-Object { $local[$_.Name.ToLowerInvariant()] = $_.Name }
$need = New-Object 'System.Collections.Generic.HashSet[string]'
$queue = New-Object System.Collections.Queue
$queue.Enqueue('tesseract.exe')
while ($queue.Count -gt 0) {
  $f = $queue.Dequeue()
  $deps = & $Dumpbin /nologo /dependents (Join-Path $TesseractDir $f) | ForEach-Object { $_.Trim().ToLowerInvariant() } | Where-Object { $_ -like '*.dll' }
  foreach ($d in $deps) { if ($local.ContainsKey($d) -and $need.Add($local[$d])) { $queue.Enqueue($local[$d]) } }
}
Copy-Item (Join-Path $TesseractDir 'tesseract.exe') $Tess
foreach ($dll in $need) { Copy-Item (Join-Path $TesseractDir $dll) $Tess }
Write-Output "tesseract: $($need.Count) DLLs"
foreach ($dir in 'configs', 'tessconfigs') { Copy-Item -Recurse (Join-Path $TesseractDir "tessdata\$dir") (Join-Path $Tess 'tessdata') }
Copy-Item (Join-Path $TesseractDir 'tessdata\osd.traineddata') (Join-Path $Tess 'tessdata')
foreach ($l in 'tha', 'eng') { Copy-Item (Join-Path $TessdataDir "$l.traineddata") (Join-Path $Tess 'tessdata') }
Copy-Item (Join-Path $TesseractDir 'doc\LICENSE') (Join-Path $Tess 'licences\Tesseract-LICENSE')
Copy-Item (Join-Path $TesseractDir 'doc\AUTHORS') (Join-Path $Tess 'licences\Tesseract-AUTHORS')
if (Test-Path (Join-Path $TessdataDir 'LICENSE')) { Copy-Item (Join-Path $TessdataDir 'LICENSE') (Join-Path $Tess 'licences\tessdata-LICENSE') }
Copy-Item (Join-Path $PSScriptRoot 'tesseract-THIRD-PARTY.txt') (Join-Path $Tess 'THIRD-PARTY.txt')
# The bundled Tesseract must read Thai before it ships.
$probe = & (Join-Path $Tess 'tesseract.exe') --tessdata-dir (Join-Path $Tess 'tessdata') --list-langs 2>&1 | Out-String
if ($probe -notmatch '(?m)^tha\s*$') { throw "the staged tesseract does not list tha: $probe" }

$Msi = Join-Path $Dist "bailancraft-$Version-windows-x64.msi"
Invoke-Native 'wix build' {
  & $Wix build (Join-Path $PSScriptRoot 'bailancraft.wxs') (Join-Path $PSScriptRoot 'bailancraft-ui.wxs') -arch x64 `
    -d "Version=$MsiVersion" -d "Manufacturer=$Manufacturer" -d "BinDir=$Stage" -d "ModelsDir=$Models" -d "TesseractDir=$Tess" `
    -d "IconPath=$(Join-Path $Root 'assets\app-icon\pdfcraft.ico')" -d "DocsDir=$Docs" -d "HasCraftFontLicences=$HasCraftFontLicences" `
    -o $Msi
}
Remove-Item -Force -ErrorAction SilentlyContinue ([IO.Path]::ChangeExtension($Msi, '.wixpdb'))
& (Join-Path $PSScriptRoot 'bailancraft-sign.ps1') $Msi

# Portable: the same files plus portable.txt, which keeps the settings in BailanCraftData beside
# BailanCraft.exe instead of the user profile (the MSI has no marker).
$Portable = Join-Path $TargetDir "bailancraft-package\BailanCraft-$Version-portable"
Remove-Item -Recurse -Force $Portable -ErrorAction SilentlyContinue
Copy-Item -Recurse $Stage $Portable
Copy-Item (Join-Path $PSScriptRoot 'bailancraft-portable.txt') (Join-Path $Portable 'portable.txt')
$Zip = Join-Path $Dist "bailancraft-$Version-windows-x64-portable.zip"
Remove-Item -Force $Zip -ErrorAction SilentlyContinue
Compress-Archive -Path $Portable -DestinationPath $Zip
Get-Item $Msi, $Zip | ForEach-Object { Write-Output ("{0}  {1:N1} MB" -f $_.Name, ($_.Length / 1MB)) }
