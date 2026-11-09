<#
.SYNOPSIS
  BailanCraft: sign files with Authenticode, when a certificate is configured.

.DESCRIPTION
  Signs each file with signtool and an RFC 3161 timestamp, using one of:
    BAILANCRAFT_SIGN_THUMBPRINT  SHA-1 thumbprint of a certificate in the Windows certificate
                                 store (an EV/OV token or a cloud HSM that exposes one)
    BAILANCRAFT_SIGN_PFX + BAILANCRAFT_SIGN_PFX_PASSWORD   a .pfx file and its password
  BAILANCRAFT_SIGN_TIMESTAMP overrides the timestamp server (default DigiCert's).
  Without either, it warns and leaves the files unsigned: Windows SmartScreen then shows
  "Windows protected your PC" on first run. See docs/bailancraft-code-signing.md.
#>
param([Parameter(ValueFromRemainingArguments = $true)] [string[]] $Files)
$ErrorActionPreference = 'Stop'
$thumb = $env:BAILANCRAFT_SIGN_THUMBPRINT
$pfx = $env:BAILANCRAFT_SIGN_PFX
if (-not $thumb -and -not $pfx) {
  Write-Warning "not signed (set BAILANCRAFT_SIGN_THUMBPRINT or BAILANCRAFT_SIGN_PFX): $($Files -join ', ')"
  return
}
$kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
$signtool = Get-ChildItem $kits -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue | Where-Object { $_.FullName -match '\\x64\\' } | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (-not $signtool) { throw 'signtool.exe not found (Windows SDK)' }
$ts = if ($env:BAILANCRAFT_SIGN_TIMESTAMP) { $env:BAILANCRAFT_SIGN_TIMESTAMP } else { 'http://timestamp.digicert.com' }
foreach ($f in $Files) {
  $id = if ($thumb) { @('/sha1', $thumb) } else { @('/f', $pfx, '/p', $env:BAILANCRAFT_SIGN_PFX_PASSWORD) }
  & $signtool sign @id /fd sha256 /tr $ts /td sha256 /d BailanCraft $f
  if ($LASTEXITCODE -ne 0) { throw "signing $f failed" }
}
