#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$env:CARGO_TARGET_DIR = $null
rustup target add wasm32-unknown-unknown | Out-Null
if (-not (Get-Command wasm-bindgen -ErrorAction SilentlyContinue)) {
    cargo install wasm-bindgen-cli --locked --version 0.2.127
}

cargo build --release --target wasm32-unknown-unknown --manifest-path web-preview/Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "wasm build failed" }

$wasm = Join-Path $Root "web-preview\target\wasm32-unknown-unknown\release\mxbo_web_preview.wasm"
if (-not (Test-Path $wasm)) {
    $wasm = Join-Path $Root "target\wasm32-unknown-unknown\release\mxbo_web_preview.wasm"
}
if (-not (Test-Path $wasm)) { throw "Missing wasm artifact" }

$out = Join-Path $Root "web\pkg"
New-Item -ItemType Directory -Force -Path $out | Out-Null
wasm-bindgen --target web --out-dir $out $wasm
Write-Host "Wrote $out"
