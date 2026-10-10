$ErrorActionPreference = 'Stop'
$installed = Join-Path $env:LOCALAPPDATA 'SPECTRA\spectra-native.exe'
$keeper = Join-Path $PSScriptRoot 'keep-spectra-taskbar.ps1'
$expectedKeeperHash = '03CC3025F1294C7F240845382ABEF56BC5EE9A632E4509164C2106C7F442A3BB'
$log = Join-Path $PSScriptRoot 'startup.log'
$mutex = [Threading.Mutex]::new($false, 'Local\SPECTRA_TaskbarKeeper_Startup')
$ownsMutex = $false
try {
    try { $ownsMutex = $mutex.WaitOne(0) }
    catch [Threading.AbandonedMutexException] { $ownsMutex = $true }
    if (-not $ownsMutex) { exit 0 }
    if ((Get-FileHash -LiteralPath $keeper -Algorithm SHA256).Hash -ne $expectedKeeperHash) {
        throw 'The frozen keeper file has changed.'
    }
    if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) {
        throw 'The installed SPECTRA executable is missing.'
    }
    $app = @(Get-Process -Name 'spectra-native' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $installed })
    if ($app.Count -eq 0) {
        Start-Process -FilePath $installed -WindowStyle Hidden | Out-Null
        Add-Content -LiteralPath $log -Encoding utf8 -Value ('{0:o} SPECTRA_STARTED' -f [DateTime]::Now)
    } else {
        Add-Content -LiteralPath $log -Encoding utf8 -Value ('{0:o} EXISTING_SPECTRA_PRESERVED pid={1}' -f [DateTime]::Now, $app[0].Id)
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
                    Add-Content -LiteralPath $log -Encoding utf8 -Value ('{0:o} FROZEN_KEEPER_STARTED' -f [DateTime]::Now)
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
            Add-Content -LiteralPath $log -Encoding utf8 -Value ('{0:o} {1}' -f [DateTime]::Now, $state)
            $lastState = $state
        }
        Start-Sleep -Seconds 2
    }
} finally {
    if ($ownsMutex) { $mutex.ReleaseMutex() }
    $mutex.Dispose()
}
