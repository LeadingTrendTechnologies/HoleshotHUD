#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$AppName = "Holeshot HUD"
$InstallDir = Join-Path $env:LOCALAPPDATA $AppName
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path

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

function Find-MxBikes {
    if ($env:MXBIKES_DIR -and (Test-Path (Join-Path $env:MXBIKES_DIR "plugins"))) {
        return (Resolve-Path $env:MXBIKES_DIR).Path
    }
    foreach ($lib in Get-SteamLibraries) {
        $candidate = Join-Path $lib "steamapps\common\MX Bikes"
        if (Test-Path (Join-Path $candidate "plugins")) {
            return $candidate
        }
    }
    foreach ($fallback in @(
            "C:\Program Files (x86)\Steam\steamapps\common\MX Bikes",
            "C:\Steam\steamapps\common\MX Bikes",
            "D:\Steam\steamapps\common\MX Bikes",
            "E:\Steam\steamapps\common\MX Bikes"
        )) {
        if (Test-Path (Join-Path $fallback "plugins")) { return $fallback }
    }
    $null
}

function Choose-MxBikes {
    $found = Find-MxBikes
    if ($found) { return $found }

    Write-Host "MX Bikes was not found automatically."
    Add-Type -AssemblyName System.Windows.Forms | Out-Null
    $dlg = New-Object System.Windows.Forms.FolderBrowserDialog
    $dlg.Description = "Select the MX Bikes folder (the one that contains mxbikes.exe)"
    $dlg.ShowNewFolderButton = $false
    if ($dlg.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
        throw "Install cancelled - no MX Bikes folder selected."
    }
    $picked = $dlg.SelectedPath
    if (-not (Test-Path (Join-Path $picked "mxbikes.exe")) -and -not (Test-Path (Join-Path $picked "plugins"))) {
        throw "That folder does not look like MX Bikes (missing mxbikes.exe / plugins)."
    }
    $plugins = Join-Path $picked "plugins"
    if (-not (Test-Path $plugins)) { New-Item -ItemType Directory -Path $plugins | Out-Null }
    $picked
}

function New-Shortcut([string]$path, [string]$target) {
    $w = New-Object -ComObject WScript.Shell
    $s = $w.CreateShortcut($path)
    $s.TargetPath = $target
    $s.WorkingDirectory = (Split-Path $target)
    $s.WindowStyle = 1
    $s.Description = $AppName
    $s.IconLocation = "$target,0"
    $s.Save()
}

$pluginSrc = Join-Path $Here "mxbo.dlo"
if (-not (Test-Path $pluginSrc)) { $pluginSrc = Join-Path $Here "..\out\Release\mxbo.dlo" }
$overlaySrc = Join-Path $Here "Holeshot-HUD.exe"
if (-not (Test-Path $overlaySrc)) { $overlaySrc = Join-Path $Here "Holeshot HUD.exe" }
if (-not (Test-Path $overlaySrc)) { $overlaySrc = Join-Path $Here "MXBO Overlay.exe" }
if (-not (Test-Path $overlaySrc)) { $overlaySrc = Join-Path $Here "mxbo-overlay.exe" }
if (-not (Test-Path $overlaySrc)) { $overlaySrc = Join-Path $Here "..\overlay\target\release\Holeshot-HUD.exe" }
if (-not (Test-Path $overlaySrc)) { $overlaySrc = Join-Path $Here "..\overlay\target\release\mxbo-overlay.exe" }
if (-not (Test-Path $pluginSrc)) { throw "mxbo.dlo is missing next to the installer." }
if (-not (Test-Path $overlaySrc)) { throw "Holeshot-HUD.exe is missing next to the installer." }

Write-Host "Installing $AppName..."
$game = Choose-MxBikes
$plugins = Join-Path $game "plugins"
New-Item -ItemType Directory -Force -Path $plugins | Out-Null
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

try {
    Copy-Item -LiteralPath $pluginSrc -Destination (Join-Path $plugins "mxbo.dlo") -Force
} catch {
    throw "Could not copy mxbo.dlo into MX Bikes. Fully quit the game and run Install again."
}

Copy-Item -LiteralPath $overlaySrc -Destination (Join-Path $InstallDir "Holeshot-HUD.exe") -Force
Copy-Item -LiteralPath (Join-Path $Here "Uninstall.ps1") -Destination (Join-Path $InstallDir "Uninstall.ps1") -Force
Copy-Item -LiteralPath (Join-Path $Here "Uninstall.bat") -Destination (Join-Path $InstallDir "Uninstall.bat") -Force
Copy-Item -LiteralPath (Join-Path $Here "README.txt") -Destination (Join-Path $InstallDir "README.txt") -ErrorAction SilentlyContinue

$desktop = [Environment]::GetFolderPath("Desktop")
$start = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs"
New-Item -ItemType Directory -Force -Path $start | Out-Null
New-Shortcut (Join-Path $desktop "$AppName.lnk") (Join-Path $InstallDir "Holeshot-HUD.exe")
New-Shortcut (Join-Path $start "$AppName.lnk") (Join-Path $InstallDir "Holeshot-HUD.exe")

Write-Host ""
Write-Host "Installed."
Write-Host "  Overlay:  $InstallDir"
Write-Host "  Plugin:   $plugins\mxbo.dlo"
Write-Host ""
Write-Host "1. Set MX Bikes to borderless or windowed (not exclusive fullscreen)."
Write-Host "2. Start MX Bikes (restart it if it was already open)."
Write-Host "3. Start Holeshot HUD from the desktop shortcut."
Write-Host "4. Press F8 for settings."
Write-Host ""

$ans = Read-Host "Start Holeshot HUD now? [Y/n]"
if ($ans -notmatch '^[Nn]') {
    Start-Process -FilePath (Join-Path $InstallDir "Holeshot-HUD.exe")
}
