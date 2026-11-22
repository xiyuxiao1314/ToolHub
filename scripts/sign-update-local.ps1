param([ValidateSet('windows-x64','macos-arm64','macos-x64')][string]$Platform='windows-x64')
$ErrorActionPreference='Stop'
if(-not $IsWindows){throw 'The local DPAPI signing key requires its original Windows user'}
$projectRoot=Split-Path -Parent $PSScriptRoot
$keyPath=Join-Path $env:LOCALAPPDATA 'ToolHub\release-signing\update-key.dpapi'
$seed=[Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes($keyPath),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
$previousSigningKey=$env:TOOLHUB_UPDATE_SIGNING_KEY
try{
  $env:TOOLHUB_UPDATE_SIGNING_KEY=([BitConverter]::ToString($seed)).Replace('-','').ToLowerInvariant()
  $manifest=Join-Path $projectRoot ('dist\release\'+$Platform+'\ToolHub-update-'+$Platform+'.json')
  $signer=Join-Path $projectRoot 'target\release\toolhub-update-sign.exe'
  & $signer $manifest ($manifest+'.sig')
  if($LASTEXITCODE -ne 0){throw 'Update metadata signing failed'}
  & $signer verify $manifest ($manifest+'.sig')
  if($LASTEXITCODE -ne 0){throw 'Update signature verification failed'}
}finally{
  $env:TOOLHUB_UPDATE_SIGNING_KEY=$previousSigningKey
  [Array]::Clear($seed,0,$seed.Length)
}
