#Requires -Version 5.1
param([switch]$Silent)
$ErrorActionPreference = "Continue"

$AppName = "Holeshot HUD"
$InstallDir = Join-Path $env:LOCALAPPDATA $AppName
$LegacyDir = Join-Path $env:LOCALAPPDATA "MXBO Overlay"

function Get-SteamLibraries {
    $roots = @()
    foreach ($key in @(
            "HKCU:\Software\Valve\Steam",
            "HKLM:\SOFTWARE\WOW6432Node\Valve\Steam",
            "HKLM:\SOFTWARE\Valve\Steam"
        )) {
        try {
            $p = (Get-ItemProperty -Path $key -ErrorAction Stop).InstallPath
            if ($p) { $roots += $p }
        } catch {}
    }
    $libs = New-Object System.Collections.Generic.List[string]
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        [void]$libs.Add($root)
        $vdf = Join-Path $root "steamapps\libraryfolders.vdf"
        if (-not (Test-Path $vdf)) { continue }
        foreach ($line in Get-Content -LiteralPath $vdf) {
            if ($line -match '"path"\s+"([^"]+)"') {
                $lib = $Matches[1] -replace '\\\\', '\'
                if (Test-Path $lib) { [void]$libs.Add($lib) }
            }
        }
    }
    $libs | Select-Object -Unique
}

Get-Process -Name "Holeshot-HUD", "mxbo-overlay", "MXBO Overlay", "Holeshot HUD" -ErrorAction SilentlyContinue | Stop-Process -Force

$saved = Join-Path $InstallDir "gamedir.txt"
if (Test-Path $saved) {
    $game = (Get-Content -LiteralPath $saved -Raw).Trim()
    $plugin = Join-Path $game "plugins\mxbo.dlo"
    if ($game -and (Test-Path $plugin)) {
        Remove-Item -LiteralPath $plugin -Force
        Write-Host "Removed $plugin"
    }
}

foreach ($lib in Get-SteamLibraries) {
    $plugin = Join-Path $lib "steamapps\common\MX Bikes\plugins\mxbo.dlo"
    if (Test-Path $plugin) {
        Remove-Item -LiteralPath $plugin -Force
        Write-Host "Removed $plugin"
    }
}

$desktop = [Environment]::GetFolderPath("Desktop")
$start = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs"
foreach ($name in @($AppName, "MXBO Overlay")) {
    foreach ($lnk in @((Join-Path $desktop "$name.lnk"), (Join-Path $start "$name.lnk"))) {
        if (Test-Path $lnk) { Remove-Item -LiteralPath $lnk -Force }
    }
}

if ($Silent) {
    exit 0
}

foreach ($dir in @($InstallDir, $LegacyDir)) {
    if (Test-Path $dir) {
        Start-Sleep -Milliseconds 400
        Remove-Item -LiteralPath $dir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "Holeshot HUD has been removed."
Read-Host "Press Enter to close"
