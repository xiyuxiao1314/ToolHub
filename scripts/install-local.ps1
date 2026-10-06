[CmdletBinding(PositionalBinding=$false)]
param([switch]$VerifyOnly, [switch]$UpdateCodexMcp)
$ErrorActionPreference = 'Stop'
$sourceRoot = [IO.Path]::GetFullPath($PSScriptRoot)
$manifest = Get-Content -LiteralPath (Join-Path $sourceRoot 'package.json') -Raw | ConvertFrom-Json
if ($manifest.schema -ne 'toolhub.package/v1' -or $manifest.build_id -notmatch '^[a-zA-Z0-9][a-zA-Z0-9-]{1,63}$') { throw 'Invalid ToolHub package manifest' }
$checksums = Get-Content -LiteralPath (Join-Path $sourceRoot 'SHA256SUMS.txt')
$verifiedFiles = @()
foreach ($line in $checksums) {
  if ($line -notmatch '^([0-9a-f]{64})  (.+)$') { throw 'Invalid checksum entry' }
  $expectedHash = $Matches[1]
  $relativePath = $Matches[2]
  $filePath = [IO.Path]::GetFullPath((Join-Path $sourceRoot $relativePath))
  if (-not $filePath.StartsWith($sourceRoot + '\',[StringComparison]::OrdinalIgnoreCase)) { throw 'Package path escapes root' }
  if ((Get-FileHash -LiteralPath $filePath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $expectedHash) { throw ('Checksum mismatch: ' + $relativePath) }
  $verifiedFiles += $relativePath
}
foreach ($binaryName in @('toolhub.exe','toolhubd.exe','toolhub-desktop.exe','package.json','install-local.ps1')) { if ($binaryName -notin $verifiedFiles) { throw ('Unchecked required file: ' + $binaryName) } }
if ($VerifyOnly) { Write-Output ('Verified ' + $verifiedFiles.Count + ' package files'); return }
$installationRoot = Join-Path $env:LOCALAPPDATA 'ToolHub'
$versionRoot = Join-Path $installationRoot 'versions'
$versionPath = Join-Path $versionRoot $manifest.build_id
if (Test-Path -LiteralPath $versionPath) { throw 'This version already exists; never overwrite a running installation' }
New-Item -ItemType Directory -Path $versionPath -Force | Out-Null
foreach ($relativePath in $verifiedFiles) {
  $destinationPath = Join-Path $versionPath $relativePath
  New-Item -ItemType Directory -Path (Split-Path $destinationPath -Parent) -Force | Out-Null
  Copy-Item -LiteralPath (Join-Path $sourceRoot $relativePath) -Destination $destinationPath
}
Copy-Item -LiteralPath (Join-Path $sourceRoot 'SHA256SUMS.txt') -Destination $versionPath
$desktopPath = Join-Path $versionPath 'toolhub-desktop.exe'
$cliPath = Join-Path $versionPath 'toolhub.exe'
$userSid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$taskName = 'ToolHub-Login-' + $userSid
$startupTask = Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue
$originalActions = if ($startupTask) { $startupTask.Actions } else { $null }
$configPath = Join-Path ([Environment]::GetFolderPath('UserProfile')) '.codex\config.toml'
$originalConfig = $null
try {
  if ($startupTask) {
    $action = New-ScheduledTaskAction -Execute $desktopPath -Argument '--autostart' -WorkingDirectory $versionPath
    Set-ScheduledTask -TaskName $taskName -Action $action | Out-Null
    $registered = Get-ScheduledTask -TaskName $taskName
    if ($registered.Actions[0].Execute -ne $desktopPath -or $registered.Actions[0].WorkingDirectory -ne $versionPath) { throw 'Startup path verification failed' }
  }
  if ($UpdateCodexMcp) {
    if (-not (Test-Path -LiteralPath $configPath)) { throw 'Existing Codex configuration unavailable' }
    $originalConfig = [IO.File]::ReadAllText($configPath)
    $sectionPattern = '(?ms)^\[mcp_servers\.toolhub\]\r?\n(?<body>.*?)(?=^\[|\z)'
    $sectionMatch = [regex]::Match($originalConfig,$sectionPattern)
    $commandLine = 'command = ' + (ConvertTo-Json -InputObject $cliPath -Compress)
    if ($sectionMatch.Success) {
      $newBody = $sectionMatch.Groups['body'].Value
      if ($newBody -match '(?m)^command\s*=') { $newBody = [regex]::Replace($newBody,'(?m)^command\s*=.*$',$commandLine) } else { $newBody = $commandLine + "`n" + $newBody }
      if ($newBody -match '(?m)^args\s*=') { $newBody = [regex]::Replace($newBody,'(?m)^args\s*=.*$','args = ["mcp", "serve"]') } else { $newBody += "`nargs = [""mcp"", ""serve""]`n" }
      $newSection = "[mcp_servers.toolhub]`n" + $newBody
      $newConfig = $originalConfig.Substring(0,$sectionMatch.Index) + $newSection + $originalConfig.Substring($sectionMatch.Index + $sectionMatch.Length)
    } else { $newConfig = $originalConfig + "`n[mcp_servers.toolhub]`n" + $commandLine + "`nargs = [""mcp"", ""serve""]`n" }
    Copy-Item -LiteralPath $configPath -Destination (Join-Path $versionPath 'codex-config.before.toml')
    [IO.File]::WriteAllText($configPath,$newConfig,[Text.UTF8Encoding]::new($false))
  }
  $shortcutPath = Join-Path ([Environment]::GetFolderPath('Desktop')) 'ToolHub.lnk'
  if (Test-Path -LiteralPath $shortcutPath) { Copy-Item -LiteralPath $shortcutPath -Destination (Join-Path $versionPath 'shortcut.before.lnk') }
  $shellObject = New-Object -ComObject WScript.Shell
  $shortcutObject = $shellObject.CreateShortcut($shortcutPath)
  $shortcutObject.TargetPath = $desktopPath
  $shortcutObject.WorkingDirectory = $versionPath
  $shortcutObject.IconLocation = $desktopPath
  $shortcutObject.Save()
  $currentFile = Join-Path $installationRoot 'current.json'
  if (Test-Path -LiteralPath $currentFile) { Copy-Item -LiteralPath $currentFile -Destination (Join-Path $versionPath 'current.before.json') }
  @{build_id=$manifest.build_id;path=$versionPath;desktop=$desktopPath;cli=$cliPath;activated_at=(Get-Date).ToUniversalTime().ToString('o')} | ConvertTo-Json | Set-Content -LiteralPath $currentFile -Encoding utf8NoBOM
  Write-Output $desktopPath
} catch {
  if ($startupTask -and $originalActions) { Set-ScheduledTask -TaskName $taskName -Action $originalActions | Out-Null }
  if ($null -ne $originalConfig) { [IO.File]::WriteAllText($configPath,$originalConfig,[Text.UTF8Encoding]::new($false)) }
  throw
}
