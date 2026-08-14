#Requires -Version 5.1
$ErrorActionPreference = "Continue"

$AppName = "MXBO Overlay"
$InstallDir = Join-Path $env:LOCALAPPDATA $AppName

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

Get-Process -Name "mxbo-overlay", "MXBO Overlay" -ErrorAction SilentlyContinue | Stop-Process -Force

foreach ($lib in Get-SteamLibraries) {
    $plugin = Join-Path $lib "steamapps\common\MX Bikes\plugins\mxbo.dlo"
    if (Test-Path $plugin) {
        Remove-Item -LiteralPath $plugin -Force
        Write-Host "Removed $plugin"
    }
}

$desktop = Join-Path ([Environment]::GetFolderPath("Desktop")) "$AppName.lnk"
$start = Join-Path ([Environment]::GetFolderPath("StartMenu")) "Programs\$AppName.lnk"
foreach ($lnk in @($desktop, $start)) {
    if (Test-Path $lnk) { Remove-Item -LiteralPath $lnk -Force }
}

if (Test-Path $InstallDir) {
    Start-Sleep -Milliseconds 400
    Remove-Item -LiteralPath $InstallDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "MXBO Overlay has been removed."
Read-Host "Press Enter to close"
