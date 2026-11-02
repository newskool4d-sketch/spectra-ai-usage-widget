$ErrorActionPreference = 'Stop'
$taskName = 'SPECTRA Taskbar Keeper'
$installed = Join-Path $env:LOCALAPPDATA 'SPECTRA\spectra-native.exe'
$destination = Join-Path $env:LOCALAPPDATA 'SPECTRA\taskbar-keeper'
$pwsh = Join-Path $env:ProgramFiles 'PowerShell\7\pwsh.exe'
# Every public CI build hashes differently from the local build the keeper was verified
# against (A67DC7BA...), and the executable carries no Authenticode signature, so the check
# reads the product version instead. v0.3.0 and v0.3.1 retain the installed path and strip class.
$supportedVersions = @('0.2.8', '0.2.9', '0.3.0', '0.3.1')
$expectedKeeperHash = '03CC3025F1294C7F240845382ABEF56BC5EE9A632E4509164C2106C7F442A3BB'

# Existing working installations are never overwritten or restarted.
if (Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue) {
    throw 'A SPECTRA keeper task already exists. Existing configuration was preserved.'
}
if (Test-Path -LiteralPath $destination) {
    throw 'The keeper directory already exists. Existing files were preserved.'
}
if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) { throw 'Install SPECTRA v0.2.8 or later first.' }
$installedVersion = (Get-Item -LiteralPath $installed).VersionInfo.ProductVersion
if ($installedVersion -notin $supportedVersions) {
    throw ('This add-on supports SPECTRA ' + ($supportedVersions -join ', ') + "; found '" + $installedVersion + "'.")
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
# RestartCount only covers launches that fail. A launcher that is closed or killed is not
# restarted (verified 2026-10-29: exit code 0xC000013A, no restart). Repeating the trigger
# brings a lost launcher back within 5 minutes while IgnoreNew leaves a running one alone.
$trigger.Repetition = (New-ScheduledTaskTrigger -Once -At (Get-Date) -RepetitionInterval (New-TimeSpan -Minutes 5)).Repetition
$principal = New-ScheduledTaskPrincipal -UserId $sid -LogonType Interactive -RunLevel Limited
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)
Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings -Description 'Start the SPECTRA taskbar keeper at Windows logon and bring it back within 5 minutes if it stops.' | Out-Null
Start-ScheduledTask -TaskName $taskName
Start-Sleep -Seconds 4
if ((Get-ScheduledTask -TaskName $taskName).State -ne 'Running') { throw 'The keeper task did not start.' }
Write-Output 'SPECTRA Taskbar Keeper is running and will start at Windows logon.'
