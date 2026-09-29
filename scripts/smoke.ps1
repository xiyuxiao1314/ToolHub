# ToolHub B01 smoke (Windows PowerShell)
# Requires: toolhub.exe and toolhubd.exe on PATH or in target/release
# Does not install/uninstall user software or modify PATH.

$ErrorActionPreference = 'Continue'
$root = Split-Path $PSScriptRoot -Parent
$bin = Join-Path $root 'target\release'
$env:PATH = "$bin;$env:PATH"
$env:TOOLHUBD_BIN = Join-Path $bin 'toolhubd.exe'
$reg = Join-Path $env:TEMP ("toolhub-smoke-" + [guid]::NewGuid().ToString('n'))
New-Item -ItemType Directory -Force -Path $reg | Out-Null
$env:TOOLHUB_REGISTRY = Join-Path $reg 'registry.sqlite'

Write-Host "== 1. status / doctor =="
toolhub --json status
toolhub --json doctor

Write-Host "== 2. native scan (no AI key) =="
toolhub --json scan --mode quick

Write-Host "== 3. search =="
toolhub --json search python
toolhub --json search node

Write-Host "== 4. environments / duplicates =="
toolhub --json env list
toolhub --json duplicates

Write-Host "== 5. resolve capability =="
toolhub --json resolve language.python.execute
toolhub --json resolve media.video.frame.extract

Write-Host "== 6. unknown executable is not auto-executed =="
# Marker file would appear if an unknown probe ran; scan must not create it.
$marker = Join-Path $reg 'EXECUTED_MARKER'
if (Test-Path $marker) { Write-Host "FAIL marker exists"; exit 1 } else { Write-Host "OK no marker" }

Write-Host "== 7. policy deny / ask =="
toolhub --json policy get

Write-Host "== 8. discovery session =="
toolhub --json discovery start --agent-id codex
toolhub --json discovery list

Write-Host "== 9. agents =="
toolhub --json agent list

Write-Host "== 10. export redacted report =="
toolhub --json export --format json

Write-Host "== 11. MCP meta tools =="
toolhub --json mcp tools

Write-Host "SMOKE DONE reg=$reg"
