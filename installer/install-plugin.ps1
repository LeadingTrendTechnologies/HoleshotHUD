#Requires -Version 5.1
param(
    [Parameter(Mandatory = $true)][string]$PluginSrc,
    [string]$GameDir = ""
)

$ErrorActionPreference = "Stop"

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

function Test-MxBikesDir([string]$dir) {
    (Test-Path (Join-Path $dir "mxbikes.exe")) -or (Test-Path (Join-Path $dir "plugins"))
}

if (-not (Test-Path -LiteralPath $PluginSrc)) {
    exit 3
}

$game = $GameDir
if (-not $game) {
    $game = Find-MxBikes
}
if (-not $game) {
    exit 1
}
if (-not (Test-MxBikesDir $game)) {
    exit 3
}

$plugins = Join-Path $game "plugins"
New-Item -ItemType Directory -Force -Path $plugins | Out-Null
try {
    Copy-Item -LiteralPath $PluginSrc -Destination (Join-Path $plugins "mxbo.dlo") -Force
} catch {
    exit 2
}
$app = Join-Path $env:LOCALAPPDATA "Holeshot HUD"
New-Item -ItemType Directory -Force -Path $app | Out-Null
Set-Content -LiteralPath (Join-Path $app "gamedir.txt") -Value $game -Encoding UTF8
exit 0
