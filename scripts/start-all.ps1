#!/usr/bin/env pwsh
# Start the entire Edda Network stack in separate windows.
$ROOT   = Split-Path $PSScriptRoot -Parent
$BIN    = "$ROOT\target\release\edda-node.exe"
if (-not (Test-Path $BIN)) { $BIN = "$ROOT\target\debug\edda-node.exe" }

Write-Host "Starting Edda Network..." -ForegroundColor Cyan

# Node 1
Start-Process powershell -ArgumentList @(
    "-NoExit", "-Command",
    "& '$BIN' --rpc-port 8899 --p2p-port 7000 --data-dir '$ROOT\data\node1'"
) -WindowStyle Normal

Start-Sleep -Milliseconds 2000

# Node 2
Start-Process powershell -ArgumentList @(
    "-NoExit", "-Command",
    "& '$BIN' --rpc-port 8900 --p2p-port 7001 --data-dir '$ROOT\data\node2' --peer '/ip4/127.0.0.1/tcp/7000'"
) -WindowStyle Normal

# Explorer
Start-Process powershell -ArgumentList @(
    "-NoExit", "-Command",
    "Set-Location '$ROOT\explorer'; npm run dev"
) -WindowStyle Normal

# Wallet
Start-Process powershell -ArgumentList @(
    "-NoExit", "-Command",
    "Set-Location '$ROOT\wallet'; npm run dev"
) -WindowStyle Normal

Write-Host ""
Write-Host "Edda Network started!" -ForegroundColor Green
Write-Host "  Node 1 RPC:  http://localhost:8899" -ForegroundColor White
Write-Host "  Node 2 RPC:  http://localhost:8900" -ForegroundColor White
Write-Host "  Explorer:    http://localhost:3000"  -ForegroundColor White
Write-Host "  Wallet:      http://localhost:3001"  -ForegroundColor White
