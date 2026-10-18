# KM-P6 RGB Tool: switch the mouse pad LEDs off when Windows shuts down.
#
# Copies kmp6_rgbtool.exe to "C:\Program Files\kmp6_rgbtool" and registers "kmp6_rgbtool.exe off" as a machine
# shutdown script of the local group policy (scripts.ini). Existing shutdown scripts are kept; an entry of the old
# PowerShell version (mauspad_off.ps1 / mousepad_off.ps1 in psscripts.ini) is removed.
#
#   -LogonOn   also send the stored setting again at every logon ("kmp6_rgbtool.exe on", scheduled task)
#
# Undo with uninstall_shutdown.ps1. Needs administrator rights (asks for them).
param([switch]$LogonOn, [switch]$Elevated)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'gpo_scripts.ps1')

if (-not (Test-Admin)) {
    if ($Elevated) { throw 'Administrator rights are required.' }
    $argList = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-Elevated')
    if ($LogonOn) { $argList += '-LogonOn' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $argList
    exit
}

try {
    # the program next to this script (release package) or the release build of the repository
    $candidates = @(
        (Join-Path $PSScriptRoot 'kmp6_rgbtool.exe'),
        (Join-Path $PSScriptRoot '..\..\target\release\kmp6_rgbtool.exe')
    )
    $source = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $source) { throw "kmp6_rgbtool.exe not found next to this script." }

    New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
    Copy-Item -Path $source -Destination $InstallExe -Force
    Write-Host "Copied to $InstallExe" -ForegroundColor Green

    # 1. shutdown script (the exe directly, so no PowerShell is needed at shutdown)
    $ini = Read-ScriptsIni $ScriptsIni
    $kept = @(Get-Entries $ini 'Shutdown' | Where-Object { $_.CmdLine -notlike '*kmp6_rgbtool.exe' })
    Set-Entries $ini 'Shutdown' ($kept + @([pscustomobject]@{ CmdLine = $InstallExe; Parameters = 'off' }))
    Write-ScriptsIni $ScriptsIni $ini
    Write-Host "Shutdown script registered in $ScriptsIni" -ForegroundColor Green

    # 2. old PowerShell version of this tool
    if (Test-Path $PsScriptsIni) {
        $ps = Read-ScriptsIni $PsScriptsIni
        $old = @(Get-Entries $ps 'Shutdown')
        $rest = @($old | Where-Object { $_.CmdLine -notmatch '(mauspad|mousepad)_off\.ps1$' })
        if ($rest.Count -ne $old.Count) {
            Set-Entries $ps 'Shutdown' $rest
            Write-ScriptsIni $PsScriptsIni $ps
            Write-Host 'Removed the old mauspad_off.ps1 shutdown entry (the file in C:\Skripte stays).' -ForegroundColor Yellow
        }
    }

    # 3. group policy: scripts extension listed, version raised so the change is picked up
    Update-GptIni
    if ($LogonOn) {
        Register-LogonTask
        Write-Host "Logon task '$TaskName' registered (kmp6_rgbtool.exe on)." -ForegroundColor Green
    }

    Write-Host 'Refreshing group policy...' -ForegroundColor Yellow
    gpupdate /target:computer /force | Out-Null
    Write-Host ''
    Write-Host 'Done: the mouse pad LEDs now switch off when Windows shuts down.' -ForegroundColor Cyan
} catch {
    Write-Host "Error: $_" -ForegroundColor Red
}
Write-Host 'This window closes in 8 seconds...'
Start-Sleep -Seconds 8
