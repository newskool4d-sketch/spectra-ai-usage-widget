$ErrorActionPreference = 'Stop'
$taskName = 'SPECTRA Taskbar Keeper'
$installed = Join-Path $env:LOCALAPPDATA 'SPECTRA\spectra-native.exe'
$destination = Join-Path $env:LOCALAPPDATA 'SPECTRA\taskbar-keeper'
$pwsh = Join-Path $env:ProgramFiles 'PowerShell\7\pwsh.exe'
$expectedExeHash = 'A67DC7BA19B5F245EC23C8450899093438F5DDCDF2A7E8FEA982A7F2007C8C20'
$expectedKeeperHash = '03CC3025F1294C7F240845382ABEF56BC5EE9A632E4509164C2106C7F442A3BB'

# Existing working installations are never overwritten or restarted.
if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
    throw 'A SPECTRA keeper task already exists. Existing configuration was preserved.'
}
if (Test-Path -LiteralPath $destination) {
    throw 'The keeper directory already exists. Existing files were preserved.'
}
if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) { throw 'Install SPECTRA v0.2.8 first.' }
if ((Get-FileHash -LiteralPath $installed -Algorithm SHA256).Hash -ne $expectedExeHash) {
    throw 'This add-on requires the verified SPECTRA v0.2.8 executable.'
}
if (-not (Test-Path -LiteralPath $pwsh -PathType Leaf)) { throw 'PowerShell 7 is required.' }
$keeper = Join-Path $PSScriptRoot 'keep-spectra-taskbar.ps1'
if ((Get-FileHash -LiteralPath $keeper -Algorithm SHA256).Hash -ne $expectedKeeperHash) {
    throw 'Keeper checksum verification failed.'
}
$launcher = Join-Path $PSScriptRoot 'start-spectra-taskbar-keeper.ps1'
$tokens = $null
$parseErrors = $null
[Management.Automation.Language.Parser]::ParseFile($launcher, [ref]$tokens, [ref]$parseErrors) | Out-Null
if ($parseErrors.Count -ne 0) { throw 'Launcher syntax verification failed.' }

New-Item -ItemType Directory -Path $destination | Out-Null
foreach ($name in @('keep-spectra-taskbar.ps1', 'start-spectra-taskbar-keeper.ps1')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $destination $name)
}
if ((Get-FileHash -LiteralPath (Join-Path $destination 'keep-spectra-taskbar.ps1') -Algorithm SHA256).Hash -ne $expectedKeeperHash) {
    throw 'Installed keeper checksum verification failed.'
}
$sid = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
$arguments = '-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "' + (Join-Path $destination 'start-spectra-taskbar-keeper.ps1') + '"'
$action = New-ScheduledTaskAction -Execute $pwsh -Argument $arguments -WorkingDirectory $destination
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $sid
$principal = New-ScheduledTaskPrincipal -UserId $sid -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Start the confirmed SPECTRA v0.2.8 taskbar keeper at Windows logon.' | Out-Null
Start-ScheduledTask -TaskName $taskName
Start-Sleep -Seconds 4
if ((Get-ScheduledTask -TaskName $taskName).State -ne 'Running') { throw 'The keeper task did not start.' }
Write-Output 'SPECTRA Taskbar Keeper is running and will start at Windows logon.'
