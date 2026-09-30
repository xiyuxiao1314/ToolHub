# ToolHub B02/R3 local Windows package (unsigned RC)
# Produces dist/toolhub-<version>/ with binaries, UI dist, schemas, resources, docs, recursive checksums.

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
if (Test-Path (Join-Path $root 'resources')) {
  Copy-Item (Join-Path $root 'resources') (Join-Path $dist 'resources') -Recurse
}
if (Test-Path (Join-Path $root 'apps\desktop\ui\dist')) {
  Copy-Item (Join-Path $root 'apps\desktop\ui\dist') (Join-Path $dist 'ui-dist') -Recurse
}
if (Test-Path (Join-Path $root 'packages\sdk-typescript')) {
  Copy-Item (Join-Path $root 'packages\sdk-typescript') (Join-Path $dist 'sdk-typescript') -Recurse
}
Copy-Item (Join-Path $root 'docs\development\PACKAGE.md') (Join-Path $dist 'README.md')
Copy-Item (Join-Path $root 'docs\development\UNINSTALL.md') $dist

# Recursive checksums
$sums = Join-Path $dist 'SHA256SUMS.txt'
if (Test-Path $sums) { Remove-Item $sums }
Get-ChildItem $dist -Recurse -File | ForEach-Object {
  $rel = $_.FullName.Substring($dist.Length + 1).Replace('\', '/')
  $h = (Get-FileHash $_.FullName -Algorithm SHA256).Hash.ToLower()
  "$h  $rel" | Add-Content $sums
}
Write-Host "Packaged $dist"
Get-Content $sums | Select-Object -First 12
