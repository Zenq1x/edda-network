#!/usr/bin/env pwsh
# Run all Rust tests including WASM gas metering
$env:PATH += ";$env:USERPROFILE\.cargo\bin"
Set-Location "$PSScriptRoot\.."
Write-Host "Running Edda test suite..." -ForegroundColor Cyan
cargo test 2>&1
