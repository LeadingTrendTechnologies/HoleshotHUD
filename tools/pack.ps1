#Requires -Version 5.1
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root

$cargo = Get-Content (Join-Path $Root "overlay\Cargo.toml") -Raw
if ($cargo -notmatch 'version\s*=\s*"([^"]+)"') { throw "Could not read version from overlay/Cargo.toml" }
$Version = $Matches[1]
if ($env:MXBO_VERSION) { $Version = $env:MXBO_VERSION }

$plugin = Join-Path $Root "out\Release\mxbo.dlo"
$overlay = Join-Path $Root "overlay\target\release\mxbo-overlay.exe"
if (-not (Test-Path $plugin) -or -not (Test-Path $overlay)) {
    Write-Host "Binaries missing - building first..."
    & (Join-Path $Root "build.bat")
    if ($LASTEXITCODE -ne 0) { throw "build.bat failed" }
}
if (-not (Test-Path $plugin)) { throw "Missing $plugin" }
if (-not (Test-Path $overlay)) { throw "Missing $overlay" }

$name = "MXBO-Overlay-$Version-windows-x64"
$dist = Join-Path $Root "dist"
$stage = Join-Path $dist $name
if (Test-Path $dist) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $overlay (Join-Path $stage "MXBO Overlay.exe")
Copy-Item $plugin (Join-Path $stage "mxbo.dlo")
Copy-Item (Join-Path $Root "installer\install.ps1") (Join-Path $stage "Install.ps1")
Copy-Item (Join-Path $Root "installer\install.bat") (Join-Path $stage "Install.bat")
Copy-Item (Join-Path $Root "installer\uninstall.ps1") (Join-Path $stage "Uninstall.ps1")
Copy-Item (Join-Path $Root "installer\uninstall.bat") (Join-Path $stage "Uninstall.bat")
Copy-Item (Join-Path $Root "installer\README.txt") (Join-Path $stage "README.txt")

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -Force
Write-Host "Packed $zip"
