# scripts/measure-memory.ps1
# SPECTRA 호스트와 그 자손(WebView2 브라우저·렌더러·GPU) 프로세스의 메모리를 합산해 CSV로 남긴다.
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)][string]$Scenario,
  [string]$ProcessName = "spectra-native",
  [int]$RootPid = 0,
  [int]$IntervalSeconds = 30,
  [int]$Samples = 60,
  [string]$OutFile = ""
)

$ErrorActionPreference = "Stop"

function Get-DescendantPids([int]$root) {
  $all = Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId
  $children = @{}
  foreach ($p in $all) {
    $parent = [int]$p.ParentProcessId
    if (-not $children.ContainsKey($parent)) { $children[$parent] = New-Object System.Collections.ArrayList }
    [void]$children[$parent].Add([int]$p.ProcessId)
  }
  $result = New-Object System.Collections.ArrayList
  $queue = New-Object System.Collections.Queue
  $queue.Enqueue($root)
  while ($queue.Count -gt 0) {
    $current = $queue.Dequeue()
    [void]$result.Add($current)
    if ($children.ContainsKey($current)) {
      foreach ($child in $children[$current]) { $queue.Enqueue($child) }
    }
  }
  return $result
}

if ($RootPid -eq 0) {
  $root = Get-Process -Name $ProcessName -ErrorAction SilentlyContinue | Select-Object -First 1
  if (-not $root) { Write-Error "프로세스를 찾지 못했습니다: $ProcessName"; exit 1 }
  $RootPid = $root.Id
}

if ($OutFile -eq "") {
  $dir = Join-Path $PSScriptRoot "..\docs\performance\measurements"
  New-Item -ItemType Directory -Force -Path $dir | Out-Null
  $OutFile = Join-Path $dir ("{0}-{1}.csv" -f $Scenario, (Get-Date -Format "yyyyMMdd-HHmm"))
}

"timestamp,scenario,sample,processCount,workingSetMB,privateMB" | Out-File -FilePath $OutFile -Encoding utf8
$peakWs = 0.0; $sumWs = 0.0; $sumPrivate = 0.0; $taken = 0

for ($i = 1; $i -le $Samples; $i++) {
  $pids = Get-DescendantPids $RootPid
  $procs = @(Get-Process -Id $pids -ErrorAction SilentlyContinue)
  if ($procs.Count -eq 0) { Write-Error "루트 프로세스 $RootPid 가 종료되었습니다."; break }
  $ws = [math]::Round(($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB, 1)
  $priv = [math]::Round(($procs | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB, 1)
  $line = "{0},{1},{2},{3},{4},{5}" -f (Get-Date -Format "s"), $Scenario, $i, $procs.Count, $ws, $priv
  $line | Out-File -FilePath $OutFile -Encoding utf8 -Append
  Write-Host $line
  $taken++; $sumWs += $ws; $sumPrivate += $priv; if ($ws -gt $peakWs) { $peakWs = $ws }
  if ($i -lt $Samples) { Start-Sleep -Seconds $IntervalSeconds }
}

if ($taken -gt 0) {
  Write-Host ("SUMMARY scenario={0} samples={1} avgWorkingSetMB={2} peakWorkingSetMB={3} avgPrivateMB={4} file={5}" -f $Scenario, $taken, [math]::Round($sumWs / $taken, 1), $peakWs, [math]::Round($sumPrivate / $taken, 1), $OutFile)
}
