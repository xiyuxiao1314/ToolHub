# Reproducible local package script (Windows)
# Produces dist/toolhub-<version>/ with binaries, schemas, docs, checksums.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$ver = '0.2.0'
$dist = Join-Path $root "dist\toolhub-$ver"
Remove-Item $dist -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $dist | Out-Null

Copy-Item (Join-Path $root 'target\release\toolhubd.exe') $dist
Copy-Item (Join-Path $root 'target\release\toolhub.exe') $dist
Copy-Item (Join-Path $root 'target\release\toolhub-desktop.exe') $dist
Copy-Item (Join-Path $root 'schemas') (Join-Path $dist 'schemas') -Recurse
Copy-Item (Join-Path $root 'docs\development\PACKAGE.md') (Join-Path $dist 'README.md')
Copy-Item (Join-Path $root 'docs\development\UNINSTALL.md') $dist

$sums = Join-Path $dist 'SHA256SUMS.txt'
Get-ChildItem $dist -File | ForEach-Object {
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$h  $($_.Name)" | Add-Content $sums
}
Write-Host "Packaged $dist"
Get-Content $sums
