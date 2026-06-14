#!/usr/bin/env pwsh
# Start Edda validator node 1 (primary)
$env:RUST_LOG = "info"
$BIN = "$PSScriptRoot\..\target\release\edda-node.exe"
if (-not (Test-Path $BIN)) {
    Write-Host "Release binary not found — using debug build"
    $BIN = "$PSScriptRoot\..\target\debug\edda-node.exe"
}
& $BIN --rpc-port 8899 --p2p-port 7000 --data-dir "$PSScriptRoot\..\data\node1"
