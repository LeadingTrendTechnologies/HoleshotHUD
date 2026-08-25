#Requires -Version 5.1
param([switch]$Silent)
$ErrorActionPreference = "Continue"

$AppName = "Holeshot HUD"
$InstallDir = Join-Path $env:LOCALAPPDATA $AppName
$LegacyDir = Join-Path $env:LOCALAPPDATA "MXBO Overlay"
# Same path the overlay uses for layout / options (HudConfig::ini_path).
$SettingsIni = Join-Path $env:USERPROFILE "Documents\PiBoSo\MX Bikes\Holeshot-HUD.ini"
$LegacySettingsIni = Join-Path $env:USERPROFILE "Documents\PiBoSo\MX Bikes\mxbo.ini"

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

function Remove-SavedSettings {
    foreach ($ini in @($SettingsIni, $LegacySettingsIni)) {
        if (Test-Path -LiteralPath $ini) {
            Remove-Item -LiteralPath $ini -Force -ErrorAction SilentlyContinue
            Write-Host "Removed $ini"
        }
    }
    foreach ($dir in @($InstallDir, $LegacyDir)) {
        if (-not (Test-Path -LiteralPath $dir)) { continue }
        foreach ($name in @("Holeshot-HUD.ini", "mxbo.ini", "gamedir.txt")) {
            $p = Join-Path $dir $name
            if (Test-Path -LiteralPath $p) {
                Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue
            }
        }
        $logs = Join-Path $dir "logs"
        if (Test-Path -LiteralPath $logs) {
            Remove-Item -LiteralPath $logs -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

function Remove-PluginFromGame([string]$game) {
    if (-not $game) { return }
    $plugins = Join-Path $game "plugins"
    foreach ($name in @("Holeshot-HUD.dlo", "mxbo.dlo")) {
        $plugin = Join-Path $plugins $name
        if (Test-Path -LiteralPath $plugin) {
            Remove-Item -LiteralPath $plugin -Force -ErrorAction SilentlyContinue
            Write-Host "Removed $plugin"
        }
    }
}

Get-Process -Name "Holeshot-HUD", "mxbo-overlay", "MXBO Overlay", "Holeshot HUD" -ErrorAction SilentlyContinue | Stop-Process -Force
Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Holeshot HUD' -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name 'Holeshot HUD game' -Force -ErrorAction SilentlyContinue

$saved = Join-Path $InstallDir "gamedir.txt"
if (Test-Path $saved) {
    $game = (Get-Content -LiteralPath $saved -Raw).Trim()
    Remove-PluginFromGame $game
}

foreach ($lib in Get-SteamLibraries) {
    Remove-PluginFromGame (Join-Path $lib "steamapps\common\MX Bikes")
}

$desktop = [Environment]::GetFolderPath("Desktop")
$start = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs"
foreach ($name in @($AppName, "MXBO Overlay")) {
    foreach ($lnk in @((Join-Path $desktop "$name.lnk"), (Join-Path $start "$name.lnk"))) {
        if (Test-Path $lnk) { Remove-Item -LiteralPath $lnk -Force }
    }
}

# Always drop layout / options and AppData leftovers so a reinstall is brand new.
# During Inno -Silent, leave the install folder itself for the uninstaller to finish;
# [UninstallDelete] then removes {app}. Standalone uninstall wipes the folders below.
Remove-SavedSettings

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
