# Mousepad Shutdown Script Installer
# Requires Administrator privileges
param([switch]$Elevated)

# Automatically request Administrator privileges
if (-not $Elevated -and -not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Start-Process powershell.exe -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Elevated"
    Exit
}

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$ScriptSource = "$ScriptDir\mousepad_off.ps1"
if (-not (Test-Path $ScriptSource)) {
    $ScriptSource = "$ScriptDir\mauspad_off.ps1"
}

$SafeDir = "C:\Skripte"
$SafeScriptPath = "$SafeDir\mauspad_off.ps1"

# 1. Create secure directory and copy script
if (-not (Test-Path $SafeDir)) {
    New-Item -ItemType Directory -Force -Path $SafeDir | Out-Null
}
Copy-Item -Path $ScriptSource -Destination $SafeScriptPath -Force
Write-Host "Script successfully copied to $SafeScriptPath." -ForegroundColor Green

# 2. Prepare Windows Group Policy (GPO) folder
$GpoDir = "$env:windir\System32\GroupPolicy\Machine\Scripts"
if (-not (Test-Path $GpoDir)) {
    New-Item -ItemType Directory -Force -Path $GpoDir | Out-Null
}

$IniFile = "$GpoDir\psscripts.ini"

# 3. Create / overwrite psscripts.ini (UTF-16 Unicode required by Windows GPO)
$iniContent = @"
[Shutdown]
0CmdLine=$SafeScriptPath
0Parameters=
"@

Set-Content -Path $IniFile -Value $iniContent -Encoding Unicode -Force
Write-Host "Shutdown Group Policy (psscripts.ini) updated successfully." -ForegroundColor Green

# 4. Force update group policies so Windows applies changes immediately
Write-Host "Refreshing Windows Group Policies..." -ForegroundColor Yellow
gpupdate /force

Write-Host ""
Write-Host "Done! The mousepad LEDs will now turn off automatically during Windows shutdown." -ForegroundColor Cyan
Write-Host "This window will close in 5 seconds..."
Start-Sleep -Seconds 5
