# KM-P6 RGB Tool: undo install_shutdown.ps1 - removes the shutdown entry, the logon task and
# "C:\Program Files\kmp6_rgbtool". Other shutdown scripts are kept. Needs administrator rights (asks for them).
param([switch]$Elevated)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'gpo_scripts.ps1')

if (-not (Test-Admin)) {
    if ($Elevated) { throw 'Administrator rights are required.' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-Elevated')
    exit
}

try {
    if (Test-Path $ScriptsIni) {
        $ini = Read-ScriptsIni $ScriptsIni
        Set-Entries $ini 'Shutdown' @(Get-Entries $ini 'Shutdown' | Where-Object { $_.CmdLine -notlike '*kmp6_rgbtool.exe' })
        Write-ScriptsIni $ScriptsIni $ini
        Update-GptIni
        Write-Host 'Shutdown entry removed.' -ForegroundColor Green
    }
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        Write-Host "Logon task '$TaskName' removed." -ForegroundColor Green
    }
    if (Test-Path $InstallDir) {
        Remove-Item -Recurse -Force $InstallDir
        Write-Host "$InstallDir removed." -ForegroundColor Green
    }
    gpupdate /target:computer /force | Out-Null
    Write-Host 'Done.' -ForegroundColor Cyan
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}
Write-Host 'This window closes in 8 seconds...'
Start-Sleep -Seconds 8
