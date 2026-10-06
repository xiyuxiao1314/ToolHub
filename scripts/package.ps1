param([string]$BuildId = ('agent-ready-' + (Get-Date -Format 'yyyyMMdd-HHmmss')), [ValidateSet('release','debug')][string]$Configuration = 'release')
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
if ($BuildId -notmatch '^[a-zA-Z0-9][a-zA-Z0-9-]{1,63}$') { throw 'Invalid build identifier' }
$outputRoot = Join-Path $projectRoot 'dist'
$packagePath = Join-Path $outputRoot ('toolhub-' + $BuildId)
if (Test-Path -LiteralPath $packagePath) { throw 'Package already exists; choose a fresh build identifier' }
$binaryRoot = Join-Path $projectRoot ('target\' + $Configuration)
foreach ($binaryName in @('toolhub.exe','toolhubd.exe','toolhub-desktop.exe')) {
  if (-not (Test-Path -LiteralPath (Join-Path $binaryRoot $binaryName) -PathType Leaf)) { throw ('Missing binary: ' + $binaryName) }
}
New-Item -ItemType Directory -Path $packagePath -Force | Out-Null
foreach ($binaryName in @('toolhub.exe','toolhubd.exe','toolhub-desktop.exe')) { Copy-Item -LiteralPath (Join-Path $binaryRoot $binaryName) -Destination $packagePath }
foreach ($assetName in @('schemas','resources','skills')) { if (Test-Path -LiteralPath (Join-Path $projectRoot $assetName)) { Copy-Item -LiteralPath (Join-Path $projectRoot $assetName) -Destination (Join-Path $packagePath $assetName) -Recurse } }
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-local.ps1') -Destination $packagePath
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\development\PACKAGE.md') -Destination (Join-Path $packagePath 'README.md')
$versionOutput = & (Join-Path $packagePath 'toolhub.exe') '--version'
if ($LASTEXITCODE -ne 0) { throw 'CLI version check failed' }
@{schema='toolhub.package/v1';build_id=$BuildId;version=$versionOutput;configuration=$Configuration;created_at=(Get-Date).ToUniversalTime().ToString('o');unsigned=$true;binaries=@('toolhub.exe','toolhubd.exe','toolhub-desktop.exe')} | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $packagePath 'package.json') -Encoding utf8NoBOM
$checksumLines = Get-ChildItem -LiteralPath $packagePath -Recurse -File | Sort-Object FullName | ForEach-Object {
  $relativePath = $_.FullName.Substring($packagePath.Length + 1).Replace('\','/')
  ((Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $relativePath)
}
$checksumLines | Set-Content -LiteralPath (Join-Path $packagePath 'SHA256SUMS.txt') -Encoding utf8NoBOM
Write-Output $packagePath
