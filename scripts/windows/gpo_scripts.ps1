# Shared helpers of install_shutdown.ps1 / uninstall_shutdown.ps1: the scripts.ini / psscripts.ini files of the
# local group policy (UTF-16, sections [Startup]/[Shutdown] with entries 0CmdLine/0Parameters, 1CmdLine, ...).

$InstallDir = Join-Path $env:ProgramFiles 'kmp6_rgbtool'
$InstallExe = Join-Path $InstallDir 'kmp6_rgbtool.exe'
$GpoDir = Join-Path $env:windir 'System32\GroupPolicy'
$ScriptsIni = Join-Path $GpoDir 'Machine\Scripts\scripts.ini'
$PsScriptsIni = Join-Path $GpoDir 'Machine\Scripts\psscripts.ini'
$GptIni = Join-Path $GpoDir 'gpt.ini'
$TaskName = 'KM-P6 RGB Tool - LEDs on at logon'
# client side extension "Scripts" + its snap-in, needed in gpt.ini so Windows runs machine scripts
$ScriptsExtension = '[{42B5FAAE-6536-11D2-AE5A-0000F87571E3}{40B6664F-4972-11D1-A7CA-0000F87571E3}]'

function Test-Admin {
    ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
        [Security.Principal.WindowsBuiltInRole]::Administrator)
}

# ordered: section name -> ordered list of "key=value" lines
function Read-ScriptsIni([string]$Path) {
    $ini = [ordered]@{}
    if (-not (Test-Path $Path)) { return $ini }
    $section = $null
    foreach ($line in (Get-Content -Path $Path -Encoding Unicode)) {
        if ($line -match '^\s*\[(.+)\]\s*$') {
            $section = $Matches[1]
            if (-not $ini.Contains($section)) { $ini[$section] = [System.Collections.ArrayList]@() }
        } elseif ($section -and $line.Trim()) {
            [void]$ini[$section].Add($line.Trim())
        }
    }
    return $ini
}

function Write-ScriptsIni([string]$Path, $Ini) {
    $lines = foreach ($name in $Ini.Keys) {
        "[$name]"
        $Ini[$name]
        ''
    }
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    Set-Content -Path $Path -Value $lines -Encoding Unicode -Force
}

# entries of a section as objects with CmdLine and Parameters, in their order
function Get-Entries($Ini, [string]$Section) {
    if (-not $Ini.Contains($Section)) { return @() }
    $map = @{}
    foreach ($line in $Ini[$Section]) {
        if ($line -match '^(\d+)(CmdLine|Parameters)=(.*)$') {
            $i = [int]$Matches[1]
            if (-not $map.ContainsKey($i)) { $map[$i] = [pscustomobject]@{ CmdLine = ''; Parameters = '' } }
            $map[$i].($Matches[2]) = $Matches[3]
        }
    }
    $map.Keys | Sort-Object | ForEach-Object { $map[$_] }
}

# writes the entries numbered 0, 1, 2, ... (Windows stops at the first gap)
function Set-Entries($Ini, [string]$Section, $Entries) {
    $lines = [System.Collections.ArrayList]@()
    $i = 0
    foreach ($e in @($Entries)) {
        if (-not $e) { continue }
        [void]$lines.Add("${i}CmdLine=$($e.CmdLine)")
        [void]$lines.Add("${i}Parameters=$($e.Parameters)")
        $i++
    }
    if ($lines.Count) { $Ini[$Section] = $lines } elseif ($Ini.Contains($Section)) { $Ini.Remove($Section) }
}

# lists the scripts extension and raises the version, so gpupdate applies the changed scripts
function Update-GptIni {
    $lines = if (Test-Path $GptIni) { @(Get-Content -Path $GptIni) } else { @('[General]') }
    $out = [System.Collections.ArrayList]@()
    $hasExt = $false
    $hasVersion = $false
    foreach ($line in $lines) {
        if ($line -match '^gPCMachineExtensionNames=(.*)$') {
            $hasExt = $true
            # one [{extension}{snap-in}] group per extension, sorted by GUID as the group policy editor writes them
            $groups = @([regex]::Matches($Matches[1], '\[[^\]]*\]') | ForEach-Object { $_.Value })
            if ($groups -notcontains $ScriptsExtension) { $groups += $ScriptsExtension }
            [void]$out.Add('gPCMachineExtensionNames=' + (($groups | Sort-Object) -join ''))
        } elseif ($line -match '^Version=(\d+)$') {
            $hasVersion = $true
            [void]$out.Add("Version=$([int64]$Matches[1] + 1)")
        } else {
            [void]$out.Add($line)
        }
    }
    if (-not $hasExt) { [void]$out.Add("gPCMachineExtensionNames=$ScriptsExtension") }
    if (-not $hasVersion) { [void]$out.Add('Version=1') }
    Set-Content -Path $GptIni -Value $out -Encoding Ascii -Force
}

# "kmp6_rgbtool.exe on" at the logon of any user (the stored setting is per user)
function Register-LogonTask {
    $action = New-ScheduledTaskAction -Execute $InstallExe -Argument 'on'
    $trigger = New-ScheduledTaskTrigger -AtLogOn
    $principal = New-ScheduledTaskPrincipal -GroupId 'S-1-5-32-545' -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 1)
    Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal `
        -Settings $settings -Force | Out-Null
}
