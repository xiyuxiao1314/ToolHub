$ErrorActionPreference='Stop'
$repoRoot=Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$temporaryRoot=Join-Path ([IO.Path]::GetTempPath()) ('toolhub-retention-test-'+[guid]::NewGuid())
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
$temporaryRoot=[IO.Path]::GetFullPath($temporaryRoot)
if (-not $temporaryRoot.StartsWith([IO.Path]::GetFullPath([IO.Path]::GetTempPath()),[StringComparison]::OrdinalIgnoreCase)) { throw 'Test root escaped temp' }
try {
  $installerText=[IO.File]::ReadAllText((Join-Path $repoRoot 'scripts\install-local.ps1'))
  $start=$installerText.IndexOf('  $maintenancePath=')
  $end=$installerText.IndexOf('  Write-Output $desktopPath',$start)
  if ($start -lt 0 -or $end -le $start) { throw 'Installer cleanup block unavailable' }
  $hook=[scriptblock]::Create($installerText.Substring($start,$end-$start))
  $versionPath=$temporaryRoot
  $installationRoot=$temporaryRoot
  $KeepOldVersions=$false
  $helper=Join-Path $temporaryRoot 'maintenance.ps1'
  Set-Content -LiteralPath $helper -Value "throw 'simulated unavailable dependency'"
  $retentionWarnings=@(& $hook 3>&1)
  if (-not ($retentionWarnings | Where-Object { $_ -is [Management.Automation.WarningRecord] })) { throw 'Maintenance failure must be reported but remain nonfatal' }
  $marker=Join-Path $temporaryRoot 'ran.txt'
  Set-Content -LiteralPath $helper -Value "Set-Content -LiteralPath (Join-Path `$PSScriptRoot 'ran.txt') -Value 'ran'"
  $KeepOldVersions=$true
  & $hook
  if (Test-Path -LiteralPath $marker) { throw 'KeepOldVersions did not skip cleanup' }
  $KeepOldVersions=$false
  & $hook
  if (-not (Test-Path -LiteralPath $marker)) { throw 'Successful activation did not invoke retention' }
  # Execute the actual packaging hook against this isolated helper as well.
  $packager=[IO.File]::ReadAllText((Join-Path $repoRoot 'scripts\package.ps1'))
  $packageStart=$packager.IndexOf("`$maintenancePath=Join-Path `$PSScriptRoot 'maintenance.ps1'")
  $packageEnd=$packager.IndexOf('Write-Output $packagePath',$packageStart)
  if ($packageStart -lt 0 -or $packageEnd -le $packageStart) { throw 'Packaging retention hook unavailable' }
  $packageHook=[scriptblock]::Create($packager.Substring($packageStart,$packageEnd-$packageStart).Replace('$PSScriptRoot','$fixture'))
  $packageFixtureModule=New-Module -ArgumentList $temporaryRoot,$packageHook -ScriptBlock {
    param($fixture,$packageCode)
    $projectRoot=$fixture
    $KeepOldPackages=$false
    Remove-Item -LiteralPath (Join-Path $fixture 'ran.txt') -ErrorAction SilentlyContinue
    & $packageCode
    if (-not (Test-Path -LiteralPath (Join-Path $fixture 'ran.txt'))) { throw 'Packaging hook did not invoke cleanup' }
    $KeepOldPackages=$true
    Remove-Item -LiteralPath (Join-Path $fixture 'ran.txt')
    & $packageCode
    if (Test-Path -LiteralPath (Join-Path $fixture 'ran.txt')) { throw 'KeepOldPackages did not skip cleanup' }
  }
  Remove-Module $packageFixtureModule
  $beforeNativeStatus=$LASTEXITCODE
  $caught=$false
  try { & (Join-Path $repoRoot 'scripts\maintenance.ps1') -Repo $temporaryRoot -Scope build } catch { $caught=$true }
  if (-not $caught) { throw 'Non-ToolHub directory was accepted' }
  if ($LASTEXITCODE -ne $beforeNativeStatus) { throw 'Maintenance leaked native failure status into installation caller' }
  Write-Output 'PASS installer retention: nonfatal failure, opt-out, successful invocation; wrapper rejects foreign root'
} finally {
  if ([IO.Path]::GetFullPath($temporaryRoot) -notlike ([IO.Path]::GetFullPath([IO.Path]::GetTempPath())+'*')) { throw 'Unsafe test cleanup path' }
  Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
}
