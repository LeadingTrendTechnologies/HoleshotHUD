#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$metadataLines = & cargo metadata --manifest-path overlay/Cargo.toml --format-version 1 --locked --filter-platform x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo metadata failed" }
$json = $metadataLines -join "`n"

Add-Type -AssemblyName System.Web.Extensions
$serializer = New-Object System.Web.Script.Serialization.JavaScriptSerializer
$serializer.MaxJsonLength = [int]::MaxValue
$serializer.RecursionLimit = 256
$meta = $serializer.DeserializeObject($json)

$used = @{}
foreach ($node in $meta["resolve"]["nodes"]) {
    $used[$node["id"]] = $true
}
$lines = New-Object System.Collections.Generic.List[string]
foreach ($pkg in $meta["packages"]) {
    if (-not $pkg["source"]) { continue }
    if (-not $used.ContainsKey($pkg["id"])) { continue }
    $license = $pkg["license"]
    if (-not $license) { $license = "license not specified" }
    $lines.Add(("{0} {1} ({2})" -f $pkg["name"], $pkg["version"], $license))
}
$sorted = @($lines | Sort-Object)
$oflPath = Join-Path $Root "overlay\hud\fonts\OFL.txt"
$ofl = Get-Content -LiteralPath $oflPath -Raw
$preambleAt = $ofl.IndexOf("SIL OPEN FONT LICENSE")
if ($preambleAt -lt 0) { throw "OFL.txt is missing its licence heading" }
$fontNote = $ofl.Substring(0, $preambleAt).Trim()
$header = @(
    "Third-party Rust crates built into Holeshot HUD."
    "Each line is the crate name, version, and SPDX license identifier."
    "Font Awesome Free icons: https://fontawesome.com/license/free"
    ""
)
$fonts = @(
    ""
    $fontNote
    ""
    "The full font licence is OFL.txt, beside this file."
)
$out = Join-Path $Root "installer\THIRD-PARTY.txt"
($header + $sorted + $fonts) -join "`r`n" | Set-Content -LiteralPath $out -Encoding utf8
Write-Host "Wrote $out ($($sorted.Count) crates)"
