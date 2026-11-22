# Windows PowerShell 5.1 compatible. Executed from the embedded, trusted updater helper.
param([Parameter(Mandatory=$true)][string]$Request,[switch]$Rollback,[switch]$FilesOnly)
$ErrorActionPreference='Stop'
$plan=Get-Content -LiteralPath $Request -Raw -Encoding UTF8|ConvertFrom-Json
$installRoot=[IO.Path]::GetFullPath([string]$plan.root)
$targetRoot=[IO.Path]::GetFullPath([string]$plan.target)
$stageRoot=[IO.Path]::GetFullPath([string]$plan.stage)
$expectedRoot=Join-Path $env:LOCALAPPDATA 'ToolHub'
if($installRoot -ne [IO.Path]::GetFullPath($expectedRoot) -or -not $targetRoot.StartsWith((Join-Path $installRoot 'versions')+'\',[StringComparison]::OrdinalIgnoreCase) -or -not $stageRoot.StartsWith((Join-Path $installRoot 'updates')+'\',[StringComparison]::OrdinalIgnoreCase)){throw 'Update installation boundary mismatch'}
$sourceExe=([string]$plan.source).Replace('\\?\','')
$newExe=Join-Path $targetRoot 'toolhub-desktop.exe'
$currentFile=Join-Path $installRoot 'current.json'
$backupFile=Join-Path $stageRoot 'activation-backup.json'
$desktopDirectory=[Environment]::GetFolderPath('Desktop')
$canShortcut=-not $FilesOnly -and -not [string]::IsNullOrWhiteSpace($desktopDirectory)
$shortcut=if($canShortcut){Join-Path $desktopDirectory 'ToolHub.lnk'}else{Join-Path $stageRoot 'fixture-shortcut.lnk'}
$runKey='HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$taskName='ToolHub-Login-'+[Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$utf8=New-Object Text.UTF8Encoding($false)
function Atomic-Write([string]$Path,[byte[]]$Bytes){
  $temp=$Path+'.update-'+[Guid]::NewGuid().ToString('N')
  [IO.File]::WriteAllBytes($temp,$Bytes)
  if([IO.File]::Exists($Path)){$replacementBackup=$Path+'.previous-'+[Guid]::NewGuid().ToString('N');[IO.File]::Replace($temp,$Path,$replacementBackup);[IO.File]::Delete($replacementBackup)}else{[IO.File]::Move($temp,$Path)}
}
function Save-Backup($Value){Atomic-Write $backupFile ($utf8.GetBytes(($Value|ConvertTo-Json -Depth 12)))}
function File-Hash([string]$Path){
  $stream=[IO.File]::OpenRead($Path);$hash=[Security.Cryptography.SHA256]::Create()
  try{return ([BitConverter]::ToString($hash.ComputeHash($stream))).Replace('-','').ToLowerInvariant()}finally{$stream.Dispose();$hash.Dispose()}
}
if($Rollback){
  if(-not(Test-Path -LiteralPath $backupFile)){return}
  $before=Get-Content -LiteralPath $backupFile -Raw -Encoding UTF8|ConvertFrom-Json
  foreach($file in @($before.files)){
    if((Test-Path -LiteralPath $file.path)-and(File-Hash $file.path)-eq $file.after){Atomic-Write $file.path ([Convert]::FromBase64String($file.before))}
  }
  if($before.shortcut -and (Test-Path -LiteralPath (Join-Path $stageRoot 'shortcut.before.lnk'))){Copy-Item -LiteralPath (Join-Path $stageRoot 'shortcut.before.lnk') -Destination $shortcut -Force}
  elseif($before.shortcut_changed -and (Test-Path -LiteralPath $shortcut)){Remove-Item -LiteralPath $shortcut}
  if($before.task_xml){Register-ScheduledTask -TaskName $taskName -Xml $before.task_xml -Force|Out-Null}
  if($before.run_changed){Set-ItemProperty -LiteralPath $runKey -Name ToolHub -Value $before.run_value}
  if($before.current){Atomic-Write $currentFile ([Convert]::FromBase64String($before.current))}
  elseif(Test-Path -LiteralPath $currentFile){Remove-Item -LiteralPath $currentFile}
  return
}
if(Test-Path -LiteralPath $backupFile){throw 'Update activation already attempted'}
$before=[ordered]@{current=$null;shortcut=$false;shortcut_changed=$false;task_xml=$null;run_value=$null;run_changed=$false;files=@()}
if(Test-Path -LiteralPath $currentFile){$before.current=[Convert]::ToBase64String([IO.File]::ReadAllBytes($currentFile))}
if($canShortcut -and (Test-Path -LiteralPath $shortcut)){Copy-Item -LiteralPath $shortcut -Destination (Join-Path $stageRoot 'shortcut.before.lnk');$before.shortcut=$true}
$task=if($FilesOnly){$null}else{Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue}
if($task){
  if($task.Description -ne [string]$plan.task_marker){throw 'Existing startup task has a different owner'}
  $before.task_xml=Export-ScheduledTask -TaskName $taskName
}
$run=if($FilesOnly){$null}else{(Get-ItemProperty -LiteralPath $runKey -Name ToolHub -ErrorAction SilentlyContinue).ToolHub}
if($run){$before.run_value=[string]$run}
Save-Backup $before
try{
  $targetCli=Join-Path $targetRoot 'toolhub.exe'
  $bin=Join-Path $installRoot 'bin'
  New-Item -ItemType Directory -Path $bin -Force|Out-Null
  if((Get-Item -LiteralPath $bin).Attributes -band [IO.FileAttributes]::ReparsePoint){throw 'Stable CLI directory cannot be a reparse point'}
  $stable=Join-Path $bin 'toolhub.exe'
  $marker=Join-Path $bin 'launcher.json'
  if(-not(Test-Path -LiteralPath $stable)){
    Copy-Item -LiteralPath $targetCli -Destination $stable
    [IO.File]::WriteAllText($marker,(@{schema='toolhub.forwarder/v1';sha256=(File-Hash $stable)}|ConvertTo-Json),$utf8)
  }
  $validStable=$false
  if((Test-Path -LiteralPath $marker)-and(Test-Path -LiteralPath $stable)){
    $info=Get-Content -LiteralPath $marker -Raw -Encoding UTF8|ConvertFrom-Json
    $validStable=$info.schema -eq 'toolhub.forwarder/v1' -and $info.sha256 -eq (File-Hash $stable)
  }
  $connectionCli=if($validStable){$stable}else{$targetCli}
  $sourceCli=Join-Path (Split-Path -Parent $sourceExe) 'toolhub.exe'
  $configPaths=@(
    (Join-Path $env:USERPROFILE '.codex\config.toml'),
    (Join-Path $env:USERPROFILE '.config\mimocode\mimocode.jsonc'),
    (Join-Path $env:USERPROFILE '.config\mimocode\mimocode.json'),
    (Join-Path $env:USERPROFILE '.config\opencode\opencode.json'),
    (Join-Path $env:USERPROFILE '.cursor\mcp.json'),
    (Join-Path $env:USERPROFILE '.claude.json'),
    (Join-Path $env:USERPROFILE '.gemini\settings.json'),
    (Join-Path $env:APPDATA 'Claude\claude_desktop_config.json'))
  foreach($configPath in $configPaths){
    if(-not(Test-Path -LiteralPath $configPath -PathType Leaf)){continue}
    $item=Get-Item -LiteralPath $configPath
    if($item.Length -gt 2097152 -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)){continue}
    $bytes=[IO.File]::ReadAllBytes($configPath);$text=$utf8.GetString($bytes)
    $updated=$text.Replace((ConvertTo-Json -InputObject $sourceCli -Compress),(ConvertTo-Json -InputObject $connectionCli -Compress)).Replace("'"+$sourceCli+"'","'"+$connectionCli+"'")
    $updated=$updated.Replace((ConvertTo-Json -InputObject $sourceCli.Replace('\','/') -Compress),(ConvertTo-Json -InputObject $connectionCli.Replace('\','/') -Compress))
    # JSON/TOML basic strings can encode non-ASCII paths as \uXXXX.
    $updated=[regex]::Replace($updated,'"(?:[^"\\]|\\.)*"',[Text.RegularExpressions.MatchEvaluator]{
      param($match)
      try{
        $decoded=ConvertFrom-Json -InputObject $match.Value
        if($decoded -ceq $sourceCli -or $decoded -ceq $sourceCli.Replace('\','/')){return (ConvertTo-Json -InputObject $connectionCli -Compress)}
      }catch{}
      return $match.Value
    })
    if($updated -eq $text){continue}
    # Compare again so a concurrent host edit is preserved.
    $beforeHash=([BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($bytes))).Replace('-','').ToLowerInvariant()
    if((File-Hash $configPath)-ne $beforeHash){continue}
    $row=@{path=$configPath;before=[Convert]::ToBase64String($bytes);after=([BitConverter]::ToString([Security.Cryptography.SHA256]::Create().ComputeHash($utf8.GetBytes($updated)))).Replace('-','').ToLowerInvariant()}
    $before.files+=@($row);Save-Backup $before
    Atomic-Write $configPath ($utf8.GetBytes($updated))
  }
  if($task){
    $action=New-ScheduledTaskAction -Execute $newExe -Argument '--autostart' -WorkingDirectory $targetRoot
    Set-ScheduledTask -TaskName $taskName -Action $action|Out-Null
  }
  if($run -and $run.Contains($sourceExe)){
    $before.run_changed=$true;Save-Backup $before
    Set-ItemProperty -LiteralPath $runKey -Name ToolHub -Value $run.Replace($sourceExe,$newExe)
  }
  if($canShortcut){
  $shell=New-Object -ComObject WScript.Shell
  $link=$shell.CreateShortcut($shortcut)
  if(-not $before.shortcut -or $link.TargetPath -eq $sourceExe -or $link.TargetPath.StartsWith((Join-Path $installRoot 'versions')+'\',[StringComparison]::OrdinalIgnoreCase)){
    $before.shortcut_changed=$true;Save-Backup $before
    $link.TargetPath=$newExe;$link.WorkingDirectory=$targetRoot;$link.IconLocation=$newExe;$link.Save()
  }
  }
  $value=@{build_id=(Split-Path -Leaf $targetRoot);path=$targetRoot;desktop=$newExe;cli=$targetCli;activated_at=[DateTime]::UtcNow.ToString('o')}
  Atomic-Write $currentFile ($utf8.GetBytes(($value|ConvertTo-Json)))
}catch{
  # The helper invokes rollback before relaunching the old application.
  throw $_.Exception.Message
}
