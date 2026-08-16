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

$name = "Holeshot-HUD-$Version-windows-x64"
$dist = Join-Path $Root "dist"
$stage = Join-Path $dist $name
if (Test-Path $dist) { Remove-Item -LiteralPath $stage -Recurse -Force -ErrorAction SilentlyContinue }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $overlay (Join-Path $stage "Holeshot HUD.exe")
Copy-Item $plugin (Join-Path $stage "mxbo.dlo")
Copy-Item (Join-Path $Root "installer\install.ps1") (Join-Path $stage "Install.ps1")
Copy-Item (Join-Path $Root "installer\install.bat") (Join-Path $stage "Install.bat")
Copy-Item (Join-Path $Root "installer\install-plugin.ps1") (Join-Path $stage "Install-Plugin.ps1")
Copy-Item (Join-Path $Root "installer\uninstall.ps1") (Join-Path $stage "Uninstall.ps1")
Copy-Item (Join-Path $Root "installer\uninstall.bat") (Join-Path $stage "Uninstall.bat")
Copy-Item (Join-Path $Root "installer\README.txt") (Join-Path $stage "README.txt")

$zip = Join-Path $dist "$name.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path (Join-Path $stage "*") -DestinationPath $zip -Force
Write-Host "Packed $zip"

$iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) {
    throw "Inno Setup 6 is not installed. Install it or run: choco install innosetup -y"
}

$setup = Join-Path $dist "HoleshotHUD-Setup.exe"
if (Test-Path $setup) { Remove-Item $setup -Force }
& $iscc "/DMyAppVersion=$Version" "/DSourceDir=$stage" "/DOutputDir=$dist" (Join-Path $Root "installer\holeshot.iss")
if ($LASTEXITCODE -ne 0) { throw "Inno Setup compile failed" }
if (-not (Test-Path $setup)) { throw "Missing $setup" }
Write-Host "Built $setup"
