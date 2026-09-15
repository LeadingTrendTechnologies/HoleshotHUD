#Requires -Version 5.1
# One-at-a-time plugin A/B. Quit game + HUD, then swap.
param(
    [ValidateSet("mrp-off", "mrp-on", "hud-off", "hud-on", "status")]
    [string]$Mode = "status"
)

$ErrorActionPreference = "Stop"
$plugins = "D:\Steam\steamapps\common\MX Bikes\plugins"
$mrp = Join-Path $plugins "mxbmrp3.dlo"
$mrpOff = Join-Path $plugins "mxbmrp3.dlo.off"
$hud = Join-Path $plugins "Holeshot-HUD.dlo"
$hudOff = Join-Path $plugins "Holeshot-HUD.dlo.off"

function Stop-GameAndHud {
    foreach ($name in @("mxbikes", "Holeshot-HUD")) {
        Get-Process -Name $name -ErrorAction SilentlyContinue | Stop-Process -Force
    }
    Start-Sleep -Milliseconds 400
}

function Show-Status {
    $m = if (Test-Path $mrp) { "on" } elseif (Test-Path $mrpOff) { "off" } else { "missing" }
    $h = if (Test-Path $hud) { "on" } elseif (Test-Path $hudOff) { "off" } else { "missing" }
    Write-Host "mxbmrp3=$m  Holeshot-HUD=$h"
}

Stop-GameAndHud
switch ($Mode) {
    "mrp-off" {
        if (Test-Path $mrp) { Rename-Item -LiteralPath $mrp -NewName "mxbmrp3.dlo.off" }
    }
    "mrp-on" {
        if (Test-Path $mrpOff) { Rename-Item -LiteralPath $mrpOff -NewName "mxbmrp3.dlo" }
    }
    "hud-off" {
        if (Test-Path $hud) { Rename-Item -LiteralPath $hud -NewName "Holeshot-HUD.dlo.off" }
    }
    "hud-on" {
        if (Test-Path $hudOff) { Rename-Item -LiteralPath $hudOff -NewName "Holeshot-HUD.dlo" }
    }
}
Show-Status
