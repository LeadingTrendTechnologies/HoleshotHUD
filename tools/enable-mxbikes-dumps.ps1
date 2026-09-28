#Requires -Version 5.1
# WER LocalDumps for mxbikes.exe (minidump). Needs an elevated prompt.
$ErrorActionPreference = "Stop"

$exe = "mxbikes.exe"
$key = "HKLM:\SOFTWARE\Microsoft\Windows\Windows Error Reporting\LocalDumps\$exe"
$folder = Join-Path $env:LOCALAPPDATA "CrashDumps"

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $here = $MyInvocation.MyCommand.Path
    Start-Process -FilePath "powershell.exe" -Verb RunAs -Wait -ArgumentList @(
        "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $here
    )
    if (-not (Test-Path $key)) { throw "LocalDumps key was not created. Approve the elevation prompt." }
    exit 0
}

New-Item -ItemType Directory -Force -Path $folder | Out-Null
New-Item -Path $key -Force | Out-Null
Set-ItemProperty -Path $key -Name DumpFolder -Type ExpandString -Value $folder
Set-ItemProperty -Path $key -Name DumpCount -Type DWord -Value 8
Set-ItemProperty -Path $key -Name DumpType -Type DWord -Value 1
Write-Host "LocalDumps on for $exe -> $folder (mini, keep 8)"
