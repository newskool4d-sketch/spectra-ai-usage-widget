$ErrorActionPreference = 'Stop'
$installed = Join-Path $env:LOCALAPPDATA 'SPECTRA\spectra-native.exe'
$keeper = Join-Path $PSScriptRoot 'keep-spectra-taskbar.ps1'
$expectedKeeperHash = '03CC3025F1294C7F240845382ABEF56BC5EE9A632E4509164C2106C7F442A3BB'
$log = Join-Path $PSScriptRoot 'startup.log'
$mutex = [Threading.Mutex]::new($false, 'Local\SPECTRA_TaskbarKeeper_Startup')
$ownsMutex = $false
# Written by the finally block. A console close (exit code 0xC000013A) or a kill never
# reaches finally, so the next launcher records the missing exit as PREVIOUS_LAUNCHER_LOST.
$exitReason = 'INTERRUPTED'

function Write-StartupLog([string] $Message) {
    Add-Content -LiteralPath $log -Encoding utf8 -Value ('{0:o} {1}' -f [DateTime]::Now, $Message)
}

# Start of the current interactive logon, or $null when it cannot be resolved.
function Get-LogonStart {
    try {
        $logonIds = @(Get-CimInstance Win32_LoggedOnUser |
            Where-Object { $_.Antecedent.Name -eq $env:USERNAME -and $_.Antecedent.Domain -eq $env:USERDOMAIN } |
            ForEach-Object { $_.Dependent.LogonId })
        $session = Get-CimInstance Win32_LogonSession -Filter 'LogonType = 2 OR LogonType = 10 OR LogonType = 11' |
            Where-Object { $logonIds -contains $_.LogonId } | Sort-Object StartTime -Descending | Select-Object -First 1
        if ($session) { return $session.StartTime }
    } catch { }
    return $null
}

try {
    try { $ownsMutex = $mutex.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $ownsMutex = $true }
    if (-not $ownsMutex) { $exitReason = 'DUPLICATE_LAUNCHER'; exit 0 }
    $history = @(Get-Content -LiteralPath $log -Tail 400 -ErrorAction SilentlyContinue)
    $previous = @($history -match ' LAUNCHER_STARTED pid=') | Select-Object -Last 1
    if ($previous -match ' LAUNCHER_STARTED pid=(\d+) ') {
        $previousPid = $Matches[1]
        if (@($history -match (' LAUNCHER_EXIT pid=' + $previousPid + ' ')).Count -eq 0) {
            Write-StartupLog ('PREVIOUS_LAUNCHER_LOST pid=' + $previousPid)
        }
    }
    # The scheduled task repeats its trigger so a lost launcher comes back. Only the first
    # launcher of a logon starts the app; a relaunch waits instead of forcing the app back on.
    $logonStart = Get-LogonStart
    $relaunch = $false
    if ($logonStart) {
        $relaunch = @($history -match ' LAUNCHER_STARTED pid=' | Where-Object {
            $started = [DateTime]::MinValue
            [DateTime]::TryParse($_.Split(' ')[0], [Globalization.CultureInfo]::InvariantCulture, [Globalization.DateTimeStyles]::None, [ref]$started) -and $started -ge $logonStart
        }).Count -gt 0
    }
    Write-StartupLog ('LAUNCHER_STARTED pid={0} mode={1}' -f $PID, $(if ($relaunch) { 'relaunch' } else { 'logon' }))
    if ((Get-FileHash -LiteralPath $keeper -Algorithm SHA256).Hash -ne $expectedKeeperHash) {
        throw 'The frozen keeper file has changed.'
    }
    if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) {
        throw 'The installed SPECTRA executable is missing.'
    }
    $app = @(Get-Process -Name 'spectra-native' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $installed })
    if ($app.Count -eq 0 -and $relaunch) {
        Write-StartupLog 'RELAUNCH_WAITING_FOR_SPECTRA'
    } elseif ($app.Count -eq 0) {
        Start-Process -FilePath $installed -WindowStyle Hidden | Out-Null
        Write-StartupLog 'SPECTRA_STARTED'
    } else {
        Write-StartupLog ('EXISTING_SPECTRA_PRESERVED pid=' + $app[0].Id)
    }
    $lastState = ''
    while ($true) {
        try {
            $app = @(Get-Process -Name 'spectra-native' -ErrorAction SilentlyContinue |
                Where-Object { $_.Path -eq $installed })
            if ($app.Count -eq 1) {
                $existingKeeper = @(Get-CimInstance Win32_Process -Filter "Name = 'pwsh.exe' OR Name = 'powershell.exe'" |
                    Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -like '*keep-spectra-taskbar.ps1*' })
                if ($existingKeeper.Count -eq 0) {
                    Write-StartupLog 'FROZEN_KEEPER_STARTED'
                    & $keeper *> $null
                    $state = 'WAITING_FOR_SPECTRA'
                } else {
                    $state = 'EXISTING_KEEPER_PRESERVED pid=' + $existingKeeper[0].ProcessId
                }
            } else {
                $state = 'WAITING_FOR_SINGLE_SPECTRA_PROCESS'
            }
        } catch {
            $state = 'RETRY ' + $_.Exception.Message
        }
        if ($state -ne $lastState) {
            Write-StartupLog $state
            $lastState = $state
        }
        Start-Sleep -Seconds 2
    }
} catch {
    $exitReason = 'ERROR ' + $_.Exception.Message
    throw
} finally {
    Write-StartupLog ('LAUNCHER_EXIT pid={0} reason={1}' -f $PID, $exitReason)
    if ($ownsMutex) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
