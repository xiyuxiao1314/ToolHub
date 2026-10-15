#requires -Version 7.0
[CmdletBinding(PositionalBinding=$false)]
param(
  [ValidateSet('all','build','packages','versions')][string]$Scope='all',
  [switch]$Apply,
  [ValidateRange(2,100)][int]$Keep=2,
  [string]$Repo,
  [string[]]$ReferenceFile=@(),
  [string]$Report
)
$ErrorActionPreference='Stop'
$pythonPath=$null
$pythonPrefix=@()
foreach ($name in @('python','python3','py')) {
  $candidate=Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
  if ($candidate -and $candidate.Source -notlike '*\Microsoft\WindowsApps\*') {
    $pythonPath=$candidate.Source
    if ($name -eq 'py') { $pythonPrefix=@('-3') }
    break
  }
}
if (-not $pythonPath) {
  $pythonRoot=Join-Path $env:LOCALAPPDATA 'Programs\Python'
  if (Test-Path -LiteralPath $pythonRoot) {
    $candidate=Get-ChildItem -LiteralPath $pythonRoot -Directory | Sort-Object Name -Descending | ForEach-Object { Join-Path $_.FullName 'python.exe' } | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
    if ($candidate) { $pythonPath=$candidate }
  }
}
if (-not $pythonPath) { throw 'Python 3.11+ is required for maintenance; nothing was cleaned.' }
$toolArguments=@($pythonPrefix)+@('-I','-X','utf8',(Join-Path $PSScriptRoot 'maintenance.py'),'--scope',$Scope,'--keep',([string]$Keep))
if ($Apply) { $toolArguments+='--apply' }
if ($Repo) { $toolArguments+=@('--repo',$Repo) }
foreach ($reference in $ReferenceFile) { $toolArguments+=@('--reference-file',$reference) }
if ($Report) { $toolArguments+=@('--report',$Report) }
$startInfo=[Diagnostics.ProcessStartInfo]::new()
$startInfo.FileName=$pythonPath
$startInfo.UseShellExecute=$false
$startInfo.CreateNoWindow=$true
$startInfo.RedirectStandardOutput=$true
$startInfo.RedirectStandardError=$true
$startInfo.StandardOutputEncoding=[Text.UTF8Encoding]::new($false)
$startInfo.StandardErrorEncoding=[Text.UTF8Encoding]::new($false)
$startInfo.WindowStyle=[Diagnostics.ProcessWindowStyle]::Hidden
foreach ($argument in $toolArguments) { $startInfo.ArgumentList.Add($argument) }
$maintenanceProcess=[Diagnostics.Process]::Start($startInfo)
$outputTask=$maintenanceProcess.StandardOutput.ReadToEndAsync()
$errorTask=$maintenanceProcess.StandardError.ReadToEndAsync()
$maintenanceProcess.WaitForExit()
$outputText=$outputTask.GetAwaiter().GetResult()
$errorText=$errorTask.GetAwaiter().GetResult()
if ($outputText) { Write-Output $outputText.TrimEnd() }
if ($maintenanceProcess.ExitCode -ne 0) { throw ('Storage maintenance stopped with exit code '+$maintenanceProcess.ExitCode+': '+$errorText.Trim()) }
if ($errorText) { Write-Warning $errorText.Trim() }
