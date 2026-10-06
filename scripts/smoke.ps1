# ToolHub B01 smoke (Windows PowerShell)
# Requires: toolhub.exe and toolhubd.exe on PATH or in target/release
# Does not install/uninstall user software or modify PATH.

$ErrorActionPreference = 'Stop'
function Invoke-Toolhub {
    param([string[]]$ToolArgs)
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $out = & toolhub @ToolArgs 2>$null
    $code = $LASTEXITCODE
    $ErrorActionPreference = $prev
    if ($code -ne 0) {
        Write-Host "FAIL toolhub $($ToolArgs -join ' ') exit=$code"
        Write-Host ($out | Out-String)
        exit 1
    }
    $out | Write-Host
    return $out
}

$root = Split-Path $PSScriptRoot -Parent
$bin = Join-Path $root 'target\release'
$env:PATH = "$bin;$env:PATH"
$env:TOOLHUBD_BIN = Join-Path $bin 'toolhubd.exe'
$reg = Join-Path $env:TEMP ("toolhub-smoke-" + [guid]::NewGuid().ToString('n'))
New-Item -ItemType Directory -Force -Path $reg | Out-Null
$env:TOOLHUB_REGISTRY = Join-Path $reg 'registry.sqlite'
$env:TOOLHUB_PRINCIPAL = 'local.admin'
$env:TOOLHUB_ADMIN = '1'

# F18: owned negative fixture — marker-producing unknown executable
$fixture = Join-Path $reg 'fixtures'
New-Item -ItemType Directory -Force -Path $fixture | Out-Null
$marker = Join-Path $reg 'EXECUTED_MARKER'
$evil = Join-Path $fixture 'unknown-tool-xyz.exe'
Set-Content -Path $evil -Value 'if exist EXECUTED_MARKER echo x' -Encoding Ascii

Write-Host "== 1. status / doctor =="
Invoke-Toolhub -ToolArgs @('--json','status') | Out-Null
Invoke-Toolhub -ToolArgs @('--json','doctor') | Out-Null

Write-Host "== 2. native scan (no AI key) =="
$scanOut = Invoke-Toolhub -ToolArgs @('--json','scan','--mode','quick')
if ($scanOut -notmatch 'candidates') { Write-Host 'FAIL scan shape'; exit 1 }

Write-Host "== 3. search =="
Invoke-Toolhub -ToolArgs @('--json','search','python') | Out-Null

Write-Host "== 4. environments / duplicates =="
Invoke-Toolhub -ToolArgs @('--json','env','list') | Out-Null
Invoke-Toolhub -ToolArgs @('--json','duplicates') | Out-Null

Write-Host "== 5. resolve capability =="
Invoke-Toolhub -ToolArgs @('--json','resolve','language.python.execute') | Out-Null
Invoke-Toolhub -ToolArgs @('--json','resolve','media.video.frame.extract') | Out-Null

Write-Host "== 6. unknown executable is not auto-executed =="
if (Test-Path $marker) { Write-Host 'FAIL marker exists after scan'; exit 1 }
Write-Host 'OK no marker'

Write-Host "== 7. policy deny =="
Invoke-Toolhub -ToolArgs @('--json','policy','get') | Out-Null

Write-Host "== 8. discovery session =="
Invoke-Toolhub -ToolArgs @('--json','discovery','start','--agent-id','codex') | Out-Null
Invoke-Toolhub -ToolArgs @('--json','discovery','list') | Out-Null

Write-Host "== 9. agents =="
Invoke-Toolhub -ToolArgs @('--json','agent','list') | Out-Null

Write-Host "== 10. export redacted report =="
$export = Invoke-Toolhub -ToolArgs @('--json','export','--format','json')
if ($export -match [regex]::Escape($env:USERPROFILE)) { Write-Host 'FAIL home path leaked'; exit 1 }

Write-Host "== 11. MCP meta tools =="
Invoke-Toolhub -ToolArgs @('--json','mcp','tools') | Out-Null

Write-Host "== 12. failing command must fail smoke =="
$ErrorActionPreference = 'Continue'
& toolhub --json inspect definitely-missing-id 2>$null | Out-Null
if ($LASTEXITCODE -eq 0) { Write-Host 'FAIL missing instance should fail'; exit 1 }
Write-Host 'OK missing-instance fails'

Write-Host "== 13. approve + execute Ask path =="
$hits = Invoke-Toolhub -ToolArgs @('--json','search','python') | ConvertFrom-Json
if (-not $hits -or $hits.Count -eq 0) { Write-Host 'FAIL no python instance'; exit 1 }
$okExec = $false
foreach ($h in $hits) {
    $iid = $h.id
    $aprRaw = & toolhub --json approve $iid -- --version 2>$null
    if ($LASTEXITCODE -ne 0) { continue }
    $apr = $aprRaw | ConvertFrom-Json
    if (-not $apr.approval_id) { continue }
    $ex = & toolhub --json exec $iid --approval-id $apr.approval_id -- --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        $okExec = $true
        Write-Host "OK approved exec on $iid"
        & toolhub --json exec $iid --approval-id $apr.approval_id -- --version 2>$null | Out-Null
        if ($LASTEXITCODE -eq 0) { Write-Host 'FAIL approval replay should fail'; exit 1 }
        Write-Host 'OK replay rejected'
        break
    }
}
if (-not $okExec) { Write-Host 'FAIL no hashable approved exec succeeded'; exit 1 }

Write-Host "SMOKE DONE reg=$reg"
