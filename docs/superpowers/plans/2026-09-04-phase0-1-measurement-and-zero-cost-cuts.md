# Phase 0·1 — 측정 기반 + 결함 3건 + 비용 0 제거 구현 계획

> 진행 상태(2026-09-05): Task 1~8 완료·커밋. Task 6의 대시보드·트레이 시나리오와 Task 9의 설치·30분 재측정(phase1b 진행 중)·push 확인만 남음.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 메모리·번들 측정 도구를 갖추고, 보류 중이던 결함 3건을 고친 뒤, 디자인 변화 없이 제품 번들에서 프로토타입 CSS·미사용 글꼴·죽은 효과를 제거한다.

**Architecture:** 측정은 PowerShell(프로세스 트리 합산)과 Node 스크립트(번들 크기)로 분리한다. Rust 결함은 `provider_usage.rs` 안에서 `std::process::Command` 사용 방식만 바꾼다. 프론트 레이스는 순수 TS 모듈(`src/data/refresh-sequence.ts`)로 분리해 `node --test`로 검증한다. CSS 분리는 `styles.css`(제품) / `styles/prototype.css`(기준선 전용)로 나눈다.

**Tech Stack:** Node 24(`node --test`, 타입 스트리핑 내장 — 새 의존성 없음), Rust 1.96(`cargo test`), PowerShell 5+, Vite 8.

**Spec:** `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` (Phase 0·1, 선택지 4)

## Global Constraints

- 새 npm·cargo 의존성 추가 금지 (스펙 선택지 1: React 유지).
- `localStorage`·`sessionStorage`·`setInterval`·`requestAnimationFrame` 사용 금지 (`docs/performance/memory-budget.md`).
- 파일 쓰기는 UTF-8. PowerShell 스크립트는 UTF-8 with BOM으로 저장(한글 주석 깨짐 방지).
- `docs/design-baseline/*.png`와 `baseline.json` 해시는 이 계획에서 바꾸지 않는다(디자인 변화 없음).
- 커밋 메시지 끝: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`. push는 사용자 확인 후.
- dev server(`npm run dev`·`preview`)는 에이전트가 직접 실행하지 않는다. 시각 확인이 필요한 단계는 명령어를 사용자에게 제시한다.
- 테스트 파일은 `tests/` 아래에 둔다(`tsconfig.json`의 `include`가 `src`뿐이라 `@types/node` 없이도 `tsc --noEmit`이 통과함).

---

### Task 1: 프로세스 트리 메모리 측정 스크립트

**Files:**
- Create: `scripts/measure-memory.ps1`
- Modify: `package.json` (scripts에 `measure:memory` 추가)

**Interfaces:**
- Produces: `scripts/measure-memory.ps1 -Scenario <name> [-ProcessName spectra-native] [-RootPid <int>] [-IntervalSeconds 30] [-Samples 60] [-OutFile docs/performance/measurements/<scenario>.csv]`. CSV 열: `timestamp,scenario,sample,processCount,workingSetMB,privateMB`. 마지막 줄에 요약을 stdout으로 출력.

- [x] **Step 1: 스크립트 작성**

```powershell
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
    if (-not $children.ContainsKey([int]$p.ParentProcessId)) { $children[[int]$p.ParentProcessId] = New-Object System.Collections.ArrayList }
    [void]$children[[int]$p.ParentProcessId].Add([int]$p.ProcessId)
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
  $procs = Get-Process -Id $pids -ErrorAction SilentlyContinue
  if (-not $procs) { Write-Error "루트 프로세스 $RootPid 가 종료되었습니다."; break }
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
```

파일은 UTF-8 with BOM으로 저장한다(Python으로 쓸 때 `encoding="utf-8-sig"`).

- [x] **Step 2: 스크립트 동작 검증 (자기 자신 대상 스모크)**

Run:
```bash
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1 -Scenario smoke -RootPid $$ -IntervalSeconds 1 -Samples 2 -OutFile "$TMP/spectra-smoke.csv"
```
(Git Bash에서 `$$`는 bash 자신의 PID. PowerShell 직접 실행 시 `-RootPid $PID`.)
Expected: `SUMMARY scenario=smoke samples=2 ...` 출력, CSV에 헤더 + 2행.

- [x] **Step 3: `.gitignore`에 측정 원본 제외, npm 스크립트 추가**

`.gitignore`에 추가:
```
docs/performance/measurements/
```
`package.json` scripts에 추가:
```json
"measure:memory": "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-memory.ps1"
```

- [x] **Step 4: Commit**

```bash
git add scripts/measure-memory.ps1 package.json .gitignore
git commit -m "chore: add process-tree memory measurement script"
```

---

### Task 2: 번들 예산 검사에 CSS·글꼴 항목 추가

**Files:**
- Modify: `scripts/verify-memory-budget.mjs:38-52`

**Interfaces:**
- Produces: `npm run verify:memory` 출력에 `css=<bytes> fonts=<bytes>/<count>` 포함. 상수 `maxCssBytes`, `maxFontBytes`, `maxFontFiles`(Task 8에서 하향).

- [x] **Step 1: 실패 조건 먼저 확인 (현재 값 출력 없음)**

Run: `npm run build && npm run verify:memory`
Expected: `MEMORY BUDGET: PASS (js=... assets=1, ...)` — CSS·글꼴 수치가 출력되지 않음(이것이 red 상태).

- [x] **Step 2: 검사 확장**

`scripts/verify-memory-budget.mjs`의 `let jsBytes = 0; let jsFiles = 0;` 아래를 다음으로 바꾼다.

```js
let jsBytes = 0;
let jsFiles = 0;
let cssBytes = 0;
let fontBytes = 0;
let fontFiles = 0;
if (existsSync(distAssets)) {
  for (const file of readdirSync(distAssets)) {
    const path = join(distAssets, file);
    if (!statSync(path).isFile()) continue;
    const size = statSync(path).size;
    if (file.endsWith(".js")) {
      jsBytes += size;
      jsFiles += 1;
      const bundle = readFileSync(path, "utf8");
      if (/setInterval\s*\(|requestAnimationFrame\s*\(|localStorage|sessionStorage/.test(bundle)) failures.push(`forbidden runtime pattern in ${file}`);
    } else if (file.endsWith(".css")) {
      cssBytes += size;
      const css = readFileSync(path, "utf8");
      for (const forbidden of ["orbit-", "prototype-switcher", "widget-lab", "@keyframes drift"]) {
        if (css.includes(forbidden)) failures.push(`prototype-only CSS shipped in ${file}: ${forbidden}`);
      }
    } else if (file.endsWith(".woff2")) {
      fontBytes += size;
      fontFiles += 1;
    }
  }
}

const maxJsBytes = 500_000;
const maxCssBytes = 45_000;
const maxFontBytes = 1_100_000;
const maxFontFiles = 4;
if (jsBytes > maxJsBytes) failures.push(`JavaScript bundle ${jsBytes} bytes exceeds ${maxJsBytes} byte structural budget`);
if (jsFiles > 3) failures.push(`JavaScript asset count ${jsFiles} exceeds 3`);
if (cssBytes > maxCssBytes) failures.push(`CSS bundle ${cssBytes} bytes exceeds ${maxCssBytes}`);
if (fontBytes > maxFontBytes) failures.push(`font payload ${fontBytes} bytes exceeds ${maxFontBytes}`);
if (fontFiles > maxFontFiles) failures.push(`font file count ${fontFiles} exceeds ${maxFontFiles}`);
```

성공 메시지를 다음으로 바꾼다.
```js
console.log(`MEMORY BUDGET: PASS (js=${jsBytes} bytes, css=${cssBytes} bytes, fonts=${fontBytes} bytes/${fontFiles} files, assets=${jsFiles}, one-layout mount, no polling/persistence)`);
```

- [x] **Step 3: 현재 번들은 프로토타입 CSS 검사에서 실패해야 함**

Run: `npm run verify:memory`
Expected: `MEMORY BUDGET: FAIL` + `prototype-only CSS shipped in index-*.css: orbit-` 등. (Task 7 이후 PASS로 바뀐다.) 이 실패는 의도된 red 상태이므로 커밋한다 — 단, 커밋 메시지에 명시.

- [x] **Step 4: Commit**

```bash
git add scripts/verify-memory-budget.mjs
git commit -m "chore: verify CSS and font budgets; flag prototype CSS in product bundle (red until Task 7)"
```

---

### Task 3: `.cmd` 실행 시 공백 경로 인용 결함 수정 (Rust)

**Files:**
- Modify: `src-tauri/src/provider_usage.rs:70-90` (`CommandSpec::command`)
- Test: `src-tauri/src/provider_usage.rs` 하단 `mod tests`

배경: 현재 `cmd.exe /D /S /C <path> <args>`로 실행한다. `/S`는 `/C` 뒤 문자열의 첫·끝 큰따옴표를 벗기므로, 경로에 공백이 있어 Rust가 `"C:\Program Files\...\codex.cmd"`로 감싸면 따옴표가 깨진다. Rust 1.77.2부터 `std::process::Command`는 프로그램 경로가 `.bat`/`.cmd`이면 자체적으로 `cmd.exe /d /c`와 안전한 인용을 적용한다(BatBadBut 대응). 따라서 직접 실행으로 바꾸면 된다.

- [x] **Step 1: 실패 테스트 작성**

`mod tests` 안에 추가:
```rust
#[cfg(target_os = "windows")]
#[test]
fn cmd_scripts_run_directly_so_std_handles_quoting() {
    let spec = CommandSpec {
        path: PathBuf::from(r"C:\Program Files\Codex Tools\codex.cmd"),
        kind: CommandKind::Cmd,
    };
    let command = spec.command(&["app-server", "--stdio"]);
    assert_eq!(command.get_program(), spec.path.as_os_str());
    let args: Vec<_> = command.get_args().collect();
    assert_eq!(args, [OsStr::new("app-server"), OsStr::new("--stdio")]);
}
```
파일 상단 `use std::ffi::OsString;`을 `use std::ffi::{OsStr, OsString};`으로 바꾼다(테스트 모듈에서 `use super::*;`로 가져옴).

- [x] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test cmd_scripts_run_directly -- --nocapture`
Expected: FAIL — `get_program()`이 `cmd.exe`.

- [x] **Step 3: 구현**

`CommandSpec::command`의 match를 다음으로 바꾼다.
```rust
let mut command = match self.kind {
    // Rust 1.77.2+ spawns .bat/.cmd through cmd.exe itself with safe quoting
    // (BatBadBut mitigation), so paths with spaces work. Wrapping in our own
    // `cmd.exe /S /C` broke the quotes for such paths.
    CommandKind::Direct | CommandKind::Cmd => Command::new(&self.path),
    CommandKind::PowerShell => {
        let mut command = Command::new("powershell.exe");
        command
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&self.path);
        command
    }
};
```

- [x] **Step 4: 통과 확인 + 전체 테스트**

Run: `cd src-tauri && cargo test`
Expected: 모두 PASS.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/provider_usage.rs
git commit -m "fix: run .cmd providers directly so paths with spaces work"
```

---

### Task 4: 외부 명령 실행에 타임아웃 적용 (Rust)

**Files:**
- Modify: `src-tauri/src/provider_usage.rs` — `command_output`(≈738행), `run_previous_statusline`(≈1127행), 상수 영역(15행 부근)
- Test: 같은 파일 `mod tests`

배경: `claude auth status`가 `.output()`으로 무한 대기한다. 자식 stdout을 별도 스레드로 읽고, `try_wait` 폴링으로 데드라인을 강제하는 헬퍼 하나를 만들어 두 호출처가 함께 쓴다.

- [x] **Step 1: 실패 테스트 작성**

```rust
#[cfg(target_os = "windows")]
#[test]
fn output_with_timeout_kills_slow_child() {
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", "Start-Sleep -Seconds 5"]);
    let started = Instant::now();
    let result = output_with_timeout(&mut command, Duration::from_millis(300), 1024);
    assert_eq!(result.unwrap_err(), "provider-command-timeout");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[cfg(target_os = "windows")]
#[test]
fn output_with_timeout_returns_stdout() {
    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/C", "echo spectra"]);
    let output = output_with_timeout(&mut command, Duration::from_secs(5), 1024).unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "spectra");
}

#[cfg(target_os = "windows")]
#[test]
fn output_with_timeout_rejects_oversized_stdout() {
    let mut command = Command::new("cmd.exe");
    command.args(["/D", "/C", "echo 0123456789abcdef"]);
    let result = output_with_timeout(&mut command, Duration::from_secs(5), 4);
    assert_eq!(result.unwrap_err(), "provider-command-output-too-large");
}
```

- [x] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test output_with_timeout`
Expected: 컴파일 오류 — `output_with_timeout` 미정의.

- [x] **Step 3: 헬퍼 구현**

상수 영역에 추가:
```rust
const CLAUDE_AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_COMMAND_OUTPUT_BYTES: u64 = 256 * 1024;
```

`command_output` 위에 헬퍼를 추가한다.
```rust
/// Spawns `command`, drains stdout on a helper thread (so a chatty child
/// cannot dead-lock on a full pipe) and kills it once `timeout` passes.
fn output_with_timeout(command: &mut Command, timeout: Duration, max_stdout: u64) -> Result<Output, String> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "provider-command-failed".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "provider-command-failed".to_string())?;
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(max_stdout + 1).read_to_end(&mut buffer);
        buffer
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("provider-command-timeout".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => return Err("provider-command-failed".to_string()),
        }
    };
    let stdout = reader.join().map_err(|_| "provider-command-failed".to_string())?;
    if stdout.len() as u64 > max_stdout {
        return Err("provider-command-output-too-large".to_string());
    }
    Ok(Output { status, stdout, stderr: Vec::new() })
}

fn command_output(spec: &CommandSpec, args: &[&str]) -> Result<Output, String> {
    let mut command = spec.command(args);
    command.stdin(Stdio::null());
    output_with_timeout(&mut command, CLAUDE_AUTH_TIMEOUT, MAX_COMMAND_OUTPUT_BYTES)
}
```

`run_previous_statusline`는 stdin에 입력을 쓴 뒤 같은 헬퍼를 쓰도록 바꾼다. 단, 헬퍼가 spawn을 담당하므로 stdin 쓰기는 spawn 이후여야 한다 → 헬퍼에 `stdin_payload: Option<&[u8]>` 인자를 추가하는 대신, 다음처럼 `run_previous_statusline` 본문을 교체한다.

```rust
fn run_previous_statusline(command: &str, input: &[u8]) -> Option<String> {
    if command.contains(CLAUDE_BRIDGE_FLAG) {
        return None;
    }
    #[cfg(target_os = "windows")]
    let mut shell = {
        let mut shell = Command::new("cmd.exe");
        shell.args(["/D", "/S", "/C", command]);
        shell
    };
    #[cfg(not(target_os = "windows"))]
    let mut shell = {
        let mut shell = Command::new("sh");
        shell.args(["-c", command]);
        shell
    };
    let mut child = shell
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(input).ok()?;
    let output = wait_with_timeout(child, STATUSLINE_COMMAND_TIMEOUT, MAX_STATUSLINE_OUTPUT_BYTES).ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim_end().to_string())
}
```

그리고 `output_with_timeout`의 spawn 이후 부분을 `wait_with_timeout(child: Child, timeout, max_stdout) -> Result<Output, String>`으로 분리해 두 함수가 공유한다:

```rust
fn output_with_timeout(command: &mut Command, timeout: Duration, max_stdout: u64) -> Result<Output, String> {
    let child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "provider-command-failed".to_string())?;
    wait_with_timeout(child, timeout, max_stdout)
}

fn wait_with_timeout(mut child: Child, timeout: Duration, max_stdout: u64) -> Result<Output, String> {
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "provider-command-failed".to_string())?;
    let reader = std::thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = stdout.take(max_stdout + 1).read_to_end(&mut buffer);
        buffer
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("provider-command-timeout".to_string());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(15)),
            Err(_) => return Err("provider-command-failed".to_string()),
        }
    };
    let stdout = reader.join().map_err(|_| "provider-command-failed".to_string())?;
    if stdout.len() as u64 > max_stdout {
        return Err("provider-command-output-too-large".to_string());
    }
    Ok(Output { status, stdout, stderr: Vec::new() })
}
```

`Child` import는 이미 있다. `CLAUDE_USAGE_TIMEOUT`(reqwest용)은 그대로 둔다.

- [x] **Step 4: 통과 확인 + 전체 테스트 + clippy**

Run: `cd src-tauri && cargo test && cargo clippy --all-targets -- -D warnings`
Expected: PASS, 경고 0. (기존에 clippy 경고가 있으면 이번 변경분만 해결하고 나머지는 보고.)

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/provider_usage.rs
git commit -m "fix: bound provider command execution with a timeout and stdout cap"
```

---

### Task 5: 동시 새로고침 레이스 수정 (프론트)

**Files:**
- Create: `src/data/refresh-sequence.ts`
- Create: `tests/refresh-sequence.test.ts`
- Modify: `src/App.tsx:503-527` (`refreshProvider`)
- Modify: `package.json` (scripts에 `test` 추가)

**Interfaces:**
- Produces: `createRefreshSequencer<K extends string>(): { begin(key: K): number; isCurrent(key: K, ticket: number): boolean }`

- [x] **Step 1: 실패 테스트 작성**

`tests/refresh-sequence.test.ts`:
```ts
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRefreshSequencer } from "../src/data/refresh-sequence.ts";

test("only the latest ticket per key is current", () => {
  const sequencer = createRefreshSequencer<"codex" | "claude">();
  const first = sequencer.begin("codex");
  const second = sequencer.begin("codex");
  assert.equal(sequencer.isCurrent("codex", first), false);
  assert.equal(sequencer.isCurrent("codex", second), true);
});

test("keys are tracked independently", () => {
  const sequencer = createRefreshSequencer<"codex" | "claude">();
  const codex = sequencer.begin("codex");
  sequencer.begin("claude");
  assert.equal(sequencer.isCurrent("codex", codex), true);
});

test("unknown ticket is never current", () => {
  const sequencer = createRefreshSequencer<"codex">();
  assert.equal(sequencer.isCurrent("codex", 1), false);
});
```

`package.json` scripts에 추가:
```json
"test": "node --test tests/"
```

- [x] **Step 2: 실패 확인**

Run: `npm test`
Expected: FAIL — 모듈을 찾을 수 없음.

- [x] **Step 3: 구현**

`src/data/refresh-sequence.ts`:
```ts
/**
 * Hands out a monotonically increasing ticket per key so an async refresh can
 * tell whether a newer request superseded it before it writes state.
 */
export function createRefreshSequencer<K extends string>() {
  const latest = new Map<K, number>();
  return {
    begin(key: K): number {
      const ticket = (latest.get(key) ?? 0) + 1;
      latest.set(key, ticket);
      return ticket;
    },
    isCurrent(key: K, ticket: number): boolean {
      return latest.get(key) === ticket;
    }
  };
}
```

- [x] **Step 4: 통과 확인**

Run: `npm test`
Expected: 3 pass.

- [x] **Step 5: App.tsx 연결**

`src/App.tsx` import에 `import { createRefreshSequencer } from "./data/refresh-sequence";` 추가.
`App` 컴포넌트 안(`initialRefreshStarted` ref 근처)에 다음을 추가:
```ts
const refreshSequence = useRef(createRefreshSequencer<ProviderId>()).current;
```
`refreshProvider`를 다음으로 바꾼다(변경점: 티켓 발급 + 응답 적용 전 최신 여부 확인, catch에서도 동일).
```ts
const refreshProvider = useCallback(async (id: ProviderId) => {
  const ticket = refreshSequence.begin(id);
  try {
    const snapshot = await getNativeProviderUsage(id);
    if (!snapshot) return null;
    if (!refreshSequence.isCurrent(id, ticket)) return snapshot;
    setQuotas(current => ({ ...current, [id]: quotaFromSnapshot(snapshot, planQuotas[id]) }));
    return snapshot;
  } catch (error) {
    if (!refreshSequence.isCurrent(id, ticket)) return null;
    const message = error instanceof Error ? error.message : "공식 사용량을 불러오지 못했습니다.";
    setQuotas(current => {
      const previous = current[id];
      const hadVerifiedData = previous.confidence === "verified";
      return {
        ...current,
        [id]: {
          ...previous,
          connectionState: hadVerifiedData ? "stale" : "error",
          source: hadVerifiedData ? previous.source : "unavailable",
          confidence: hadVerifiedData ? "verified" : "unavailable",
          statusMessage: message
        }
      };
    });
    return null;
  }
}, [refreshSequence]);
```

- [x] **Step 6: 타입·빌드·예산 확인**

Run: `npm run build && npm test`
Expected: tsc 통과, 빌드 성공, 테스트 3 pass. (`verify:memory`는 Task 2의 의도된 red가 남아 있음.)

- [x] **Step 7: Commit**

```bash
git add src/data/refresh-sequence.ts tests/refresh-sequence.test.ts src/App.tsx package.json
git commit -m "fix: ignore stale provider responses when refreshes overlap"
```

---

### Task 6: Phase 0 기준 메모리 측정·기록

**Files:**
- Modify: `docs/performance/memory-budget.md` (「Windows Tauri 기준선」 아래에 「2026-09-04 Phase 0 기준」 절 추가)

전제: 실행 대상은 현재 설치된 SPECTRA(0.2.1) 또는 `src-tauri/target/release/spectra-native.exe`(2026-08-23 빌드, 코드 변경 전 상태와 동일). 결함 수정(Task 3~5)은 메모리와 무관하므로 기준값으로 유효하다.

- [x] **Step 1: 미니 창 idle 30분 측정 (자동)**

앱 실행(설치본 우선, 없으면 `src-tauri/target/release/spectra-native.exe`). 창이 미니 모드로 표시되면:
```bash
npm run measure:memory -- -Scenario mini-idle -IntervalSeconds 30 -Samples 60
```
백그라운드로 실행하고 종료 시 SUMMARY를 기록한다. 30분 동안 창을 건드리지 않는다.

- [ ] **Step 2: 대시보드·트레이 숨김 10분 측정 (사용자 조작 필요)**

트레이 메뉴에서 대시보드로 전환 후:
```bash
npm run measure:memory -- -Scenario dashboard-idle -IntervalSeconds 30 -Samples 20
```
닫기 버튼으로 트레이 숨김 후:
```bash
npm run measure:memory -- -Scenario tray-hidden -IntervalSeconds 30 -Samples 20
```
사용자가 자리에 없으면 이 두 시나리오는 "미측정"으로 표에 남기고 다음 태스크로 진행한다.

- [x] **Step 3: 문서 기록**

`docs/performance/memory-budget.md` 끝에 추가:
```markdown
## 2026-09-04 Phase 0 기준 (개선 착수 전)

측정: `npm run measure:memory -- -Scenario <name>` (프로세스 트리 합산, 30초 간격)

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 |
|---|---|---|---|---|---|
| mini-idle (30분) | 60 | … MB | … MB | … MB | … |
| dashboard-idle (10분) | 20 | … MB | … MB | … MB | … |
| tray-hidden (10분) | 20 | … MB | … MB | … MB | … |

번들(2026-08-23 빌드): JS 230,653 B · CSS 42,710 B · 글꼴 1,074,956 B/4개 · 실행 파일 7,571,968 B · NSIS 3,207,362 B
```
`…` 자리는 실제 SUMMARY 값으로 채운다. 미측정 시나리오는 "미측정(사용자 조작 대기)"로 적는다.

- [x] **Step 4: Commit**

```bash
git add docs/performance/memory-budget.md
git commit -m "docs: record Phase 0 memory and bundle baseline"
```

---

### Task 7: 프로토타입 CSS 분리 + 죽은 효과 제거

**Files:**
- Create: `styles/prototype.css`
- Modify: `styles.css` (B 시안·전환기·위젯랩 규칙 이동, `drift`·`.grain` 삭제)
- Modify: `index.html:16-19` (`.grain` 요소 삭제)
- Modify: `scripts/verify-design-tokens.mjs` (prototype.css에 `:root` 없음 확인 추가)
- Modify: `docs/design-baseline/README.md` (B 시안 CSS 위치 안내)

- [x] **Step 1: 이동 대상 목록 확정**

다음 셀렉터를 포함한 규칙 블록을 `styles.css`에서 잘라 `styles/prototype.css`로 옮긴다. 제품(`src/App.tsx`)이 쓰는 `.stream-app`·`.stream-layout`·`.mobile-*`·`.app-surface`는 남긴다.

| 이동 | 근거 |
|---|---|
| `.prototype-shell` (18행의 `.prototype-shell, .product-shell` 결합 셀렉터에서 `.prototype-shell`만 분리) | app.js 전용 |
| 200~241행 `/* Variant B */` 블록 전체 (`.orbit-*`) | App.tsx 사용 0건 |
| `.stream-header`, `.widget-lab*`, `.widget-window`, `.ios-card`, `.device-label`, `.brand-mini` (254, 290~305행) | App.tsx 사용 0건 |
| `.prototype-switcher*` (309~311행) | 기준선 제외 항목 |
| `@media (max-width: 1180px)` 안 `.orbit-layout`, `.orbit-detail` (316~317행) | B 전용 |
| `@media (max-width: 820px)` 안 `.orbit-header … .orbit-node` (354~355행), `.stream-header`, `.widget-lab`, `.prototype-switcher` (357~358행 해당 부분) | B·전환기 전용 |
| 19~20행의 결합 셀렉터에서 `.orbit-app` 제거 (`.app-surface, .stream-app { … }`로) | B 전용 |

삭제(이동 없이):
- 11행 `.grain { … }` 규칙, 12행 `@keyframes drift`
- `index.html`의 `<span class="grain"></span>`

`styles/prototype.css` 머리말:
```css
/* 기준선 비교용 B 시안·시안 전환기·기기 미리보기 규칙.
   제품 번들(index.html → styles.css)에는 포함하지 않는다. app.js 기준선을 볼 때만 로드한다. */
```
`prototype.css`는 `styles.css`의 토큰·공용 규칙 위에 얹는 파일이므로 자체 `:root`를 두지 않는다.

- [x] **Step 2: 이동·삭제 수행**

수동 편집 대신 Python 스크립트로 행 범위를 잘라 붙인다(한글 주석 인코딩 보존, `encoding="utf-8"`). 작업 후 `grep -c "orbit-\|prototype-\|widget-lab\|drift\|\.grain" styles.css` 결과가 0이어야 한다.

- [x] **Step 3: 토큰 검사 확장**

`scripts/verify-design-tokens.mjs`에 `prototype.css` 읽기와 검사를 추가:
```js
const prototypePath = path.join(projectRoot, "styles", "prototype.css");
const prototype = await readFile(prototypePath, "utf8");
if (/:root\s*\{/.test(prototype)) failures.push("prototype.css에 토큰 선언 블록이 있습니다.");
if (!/orbit-app/.test(prototype)) failures.push("B 시안 규칙이 prototype.css에 없습니다.");
if (/orbit-|prototype-switcher|widget-lab/.test(styles)) failures.push("제품 styles.css에 프로토타입 전용 규칙이 남아 있습니다.");
```

- [x] **Step 4: 검증**

Run: `npm run build && npm run verify:memory && npm run verify:tokens && npm run verify:baseline`
Expected: 세 검사 모두 PASS. `verify:memory` 출력의 `css=` 값이 34,000 미만.

- [x] **Step 5: README 갱신**

`docs/design-baseline/README.md`의 "선택한 구조" 절 끝에 한 줄 추가:
```markdown
- B 시안·시안 전환기·기기 미리보기의 CSS는 [`styles/prototype.css`](../../styles/prototype.css)에 분리해 두며 제품 번들에는 포함하지 않습니다.
```

- [x] **Step 6: Commit**

```bash
git add styles.css styles/prototype.css index.html scripts/verify-design-tokens.mjs docs/design-baseline/README.md
git commit -m "perf: move prototype-only CSS out of the product bundle; drop grain and dead drift keyframes"
```

---

### Task 8: 글꼴 3굵기 정규화 + Medium 제거 + 예산 하향

**Files:**
- Modify: `styles/tokens.css:8-13` (500 `@font-face` 삭제)
- Delete: `styles/fonts/Pretendard-Medium.subset.woff2` (스펙 Phase 1 승인 항목 — `git rm`)
- Modify: `styles.css` (font-weight 650→600, 750·800·900→700)
- Modify: `scripts/verify-memory-budget.mjs` (임계값 하향)
- Modify: `docs/performance/memory-budget.md`, `styles/tokens.css` 주석

- [x] **Step 1: 사용처 확인 (red 조건)**

Run: `grep -rn "font-weight: *500\|fontWeight: *500\|font-weight:500" src styles.css styles/prototype.css`
Expected: 0건. (있으면 600으로 치환 후 진행.)

- [x] **Step 2: 굵기 치환**

Python으로 `styles.css`와 `styles/prototype.css`에서 정규식 치환(`encoding="utf-8"`):
- `font-weight:\s*650` → `font-weight: 600`
- `font-weight:\s*(750|800|900)` → `font-weight: 700`
치환 후 `grep -o "font-weight: *[0-9]*" styles.css | sort -u` 결과가 600·700만 남아야 한다.

- [x] **Step 3: Medium 제거**

`styles/tokens.css`에서 `font-weight: 500` `@font-face` 블록(8~13행)을 삭제하고, 주석을 "3굵기(400·600·700)"로 고친다.
```bash
git rm styles/fonts/Pretendard-Medium.subset.woff2
```

- [x] **Step 4: 예산 하향**

`scripts/verify-memory-budget.mjs`:
```js
const maxCssBytes = 34_000;
const maxFontBytes = 820_000;
const maxFontFiles = 3;
```

- [x] **Step 5: 검증**

Run: `npm run build && npm run verify:memory && npm run verify:tokens && npm run verify:baseline && npm test`
Expected: 모두 PASS. `fonts=` 값 ≈ 806,632 bytes / 3 files.

- [x] **Step 6: 문서 갱신**

`docs/performance/memory-budget.md`에 Phase 1 결과 행 추가(번들 수치: JS·CSS·글꼴 실제 값). `styles/tokens.css`의 글꼴 주석을 "4굵기" → "3굵기"로 수정.

- [x] **Step 7: Commit**

```bash
git add styles/tokens.css styles/fonts styles.css styles/prototype.css scripts/verify-memory-budget.mjs docs/performance/memory-budget.md
git commit -m "perf: ship three font weights and normalize weight requests"
```

---

### Task 9: 릴리스 빌드·설치·Phase 1 재측정

**Files:**
- Modify: `docs/performance/memory-budget.md` (Phase 1 측정 행)

- [x] **Step 1: 릴리스 빌드**

Run: `npm run desktop:build` (5~6분)
Expected: `src-tauri/target/release/bundle/nsis/SPECTRA_0.2.1_x64-setup.exe` 갱신. 실행 파일·설치 파일 크기를 기록.

- [ ] **Step 2: 설치 (사용자 명시 승인 필요)**

설치 프로그램 실행은 매번 별도 승인. 승인 없으면 `src-tauri/target/release/spectra-native.exe`를 직접 실행해 측정한다.

- [ ] **Step 3: mini-idle 30분 재측정**

```bash
npm run measure:memory -- -Scenario mini-idle-phase1 -IntervalSeconds 30 -Samples 60
```

- [x] **Step 4: 기능 스모크**

미니 창에서 새로고침 → Codex·Claude 값이 갱신되는지 확인(Task 3·4 회귀). 새로고침 버튼 연타 → 마지막 값이 유지되는지(Task 5).

- [ ] **Step 5: 문서·커밋**

Phase 0 표 아래에 Phase 1 행을 추가하고 커밋:
```bash
git add docs/performance/memory-budget.md
git commit -m "docs: record Phase 1 memory measurements"
```

- [ ] **Step 6: push 여부 사용자 확인**

파일 목록과 커밋 목록을 제시하고 `git push origin main` 승인을 받는다.

---

## Self-review

- 스펙 Phase 0 항목: 측정 스크립트(Task 1), 번들 리포트(Task 2), 기준 수치(Task 6) — 모두 대응. 선택지 4의 결함 3건: Task 3·4·5.
- 스펙 Phase 1 항목: CSS 분리·drift·그레인(Task 7), Medium 제거·굵기 정규화(Task 8), 측정(Task 9).
- 타입 일관성: `createRefreshSequencer`·`begin`·`isCurrent`(Task 5), `output_with_timeout`·`wait_with_timeout`(Task 4), `maxCssBytes`·`maxFontBytes`·`maxFontFiles`(Task 2→8) 이름 일치 확인.
- 의도된 red: Task 2 커밋 시점에 `verify:memory`가 실패 상태로 남고 Task 7에서 해소된다. 중간 커밋을 원치 않으면 Task 2 커밋을 Task 7과 합친다.
