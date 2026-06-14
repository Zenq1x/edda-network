#!/usr/bin/env pwsh
# Start Edda validator node 3
$env:RUST_LOG = "info"
$BIN = "$PSScriptRoot\..\target\release\edda-node.exe"
if (-not (Test-Path $BIN)) {
    Write-Host "Release binary not found — using debug build"
    $BIN = "$PSScriptRoot\..\target\debug\edda-node.exe"
}
& $BIN --rpc-port 8901 --p2p-port 7002 --data-dir "$PSScriptRoot\..\data\node3" `
       --peer "/ip4/127.0.0.1/tcp/7000"
