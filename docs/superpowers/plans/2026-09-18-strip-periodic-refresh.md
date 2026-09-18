# 작업표시줄 스트립 주기 갱신 (A안) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 작업표시줄 표시가 켜진 동안 네이티브 루프가 Codex·Claude를 2분 주기로 재조회하고, Claude Code가 자격 증명 파일을 다시 쓰면 즉시 재조회하며, 실패는 백오프·토큰 만료는 보류로 흡수하고, 갱신 결과를 WebView에도 이벤트로 전달한다.

**Architecture:** 기존 `spectra-claude-reset-refresh` 스레드(`desktop_shell.rs`)를 공급자 공통 루프로 일반화한다. 판단 규칙은 I/O 없는 새 모듈 `usage_refresh.rs`(`Scheduler`)에 두어 전부 단위 테스트하고, 파일 stamp 조회·프로세스 실행·이벤트 발행은 루프와 `lib.rs`의 공통 함수 `refresh_provider`가 맡는다. 실패 신호는 스냅샷의 새 필드 `live_failure`(직렬화 `liveFailure`) 하나로 스케줄러·스트립 툴팁·창 라벨이 함께 읽는다. 실시간 조회 코드는 바꾸지 않는다.

**Tech Stack:** Rust 1.96 · Tauri 2.11.5(`Emitter::emit`) · React 19 + TypeScript · Node 24 `node:test`

**Spec:** `docs/superpowers/specs/2026-09-18-strip-periodic-refresh.md` — §1 성공 기준, §2-3 갱신 규칙, §2-4 실패 코드 표, §2-5 이벤트

## Global Constraints

- **신규 의존성 0**: 크레이트·npm 패키지를 추가하지 않는다. `git diff -- src-tauri/Cargo.lock package-lock.json`이 비어 있어야 한다
- **실시간 경로 현행 유지**: `read_claude_oauth_token_at`·`fetch_claude_usage_live`·`claude_snapshot`의 조회 순서(auth status → live → 캐시 폴백)와 문구는 바꾸지 않는다. `claude auth status` 생략은 채택하지 않았다(스펙 §2-7)
- **갱신 규칙 상수(스펙 §2-3)**: tick 15초 · `INTERVAL_SECS` 120 · `RESET_RETRY_SECS` 60 · 백오프 `min(120 × 2^(n−1), 1800)` · `TOKEN_HOLD_SECS` 1800 · 순서 Claude → Codex 순차
- **실패 코드(스펙 §2-4)**: `"runtime-unavailable"` · `"claude-auth-status-failed"` · `"claude-signed-out"` · Claude live 오류 코드 그대로 · Codex `reason` 그대로 · `"codex-signed-out"` · `"codex-api-key-auth"` · 성공은 `None`. `message` 문자열을 파싱하지 않는다
- **이벤트 이름**: `provider-usage-updated`, 페이로드 = `ProviderUsageSnapshot`(camelCase)
- **라벨**: 토큰 만료 = `"로그인 갱신 필요"`(Rust `claude_status`·TS `claudeFreshness` 동일 문자열)
- **프론트 금지 패턴 유지**: `localStorage`·`sessionStorage`·새 `setInterval`·`requestAnimationFrame` 금지. 창 갱신은 이벤트 수신으로만
- **테스트 기준선(2026-09-18 실측)**: `cargo test --lib --offline --locked` 기본 **70 passed, 8 ignored** · `npm test` **48 passed**. Expected는 이 수에 신규분을 더해 적는다. `native-oauth` 구성은 기본보다 2개 많다
- **두 feature 구성 모두 통과해야 커밋**: `cargo build --release --offline --locked`(기본)·`--features native-oauth`, `cargo test --lib --offline --locked` 양쪽. 경고 0
- **프론트 게이트(프론트 변경 시)**: `npm run build` · `npm run verify:memory`(JS ≤ 240,000 B) · `npm run verify:tokens` · `npm run verify:baseline` · `npm test`
- **Rust 테스트는 `SPECTRA_DATA_DIR`·`SPECTRA_CLAUDE_CREDENTIALS`를 변경하지 않는다**(병렬 실행 오염). 파일 IO는 경로를 인자로 받는 함수로 두고 임시 디렉터리로 테스트
- **실제 앱 확인용 빌드**: `node node_modules/@tauri-apps/cli/tauri.js build --no-bundle -- --offline --locked`로 프론트를 내장한 EXE를 만든다(일반 `cargo build --release`는 `devUrl`을 참조해 실제 확인에 못 쓴다). 설치 패키지는 `npm run desktop:build`
- **실행 중 앱 교체·설치·push는 사용자 승인 후**: 설치본(`%LOCALAPPDATA%\SPECTRA\spectra-native.exe`)이 트레이에 떠 있다. 새 EXE 실행 전 트레이 메뉴 "SPECTRA 종료"로 끝내야 하며(단일 실행 플러그인이 기존 인스턴스로 전달), 이 단계는 Task 7에서 사용자 승인 후 진행한다. Task별 로컬 커밋은 진행한다
- 파일 삭제 금지(빌드 산출물 제외). 작업 트리의 미추적 `docs/prompts/`·`output/`은 스테이징 금지
- 커밋 메시지 말미: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`

## File Structure

| 파일 | 역할 | 작업 |
|---|---|---|
| `src-tauri/src/provider_usage.rs` | 스냅샷 구조체 `live_failure` 필드·실패 코드 채움, `CLAUDE_TOKEN_EXPIRED` 상수, 파일 stamp 조회 `modified_stamp`/`claude_credentials_modified` | 수정 |
| `src-tauri/src/usage_refresh.rs` | **I/O 없음.** `Scheduler`(간격·초기화 경계·자격 증명 변경·백오프·보류), `Outcome`, `outcome_for`, `backoff_secs` | 신규 |
| `src-tauri/src/lib.rs` | `mod usage_refresh`, `PROVIDER_USAGE_EVENT`, 공통 `refresh_provider`(캐시·배지·스트립·이벤트), 커맨드 위임, 루프 진입점 이름 변경 | 수정 |
| `src-tauri/src/desktop_shell.rs` | 루프 `ensure_usage_refresh_loop`: tick마다 스트립 재그리기 + `Scheduler.plan` → 순차 `refresh_provider` → `record` + 타이밍 로그 | 수정 |
| `src-tauri/src/taskbar_strip.rs` | `claude_status`에 토큰 만료 라벨 | 수정 |
| `src-tauri/src/tray_badge.rs` · `lib.rs` 테스트 · `desktop_shell.rs` 테스트 | 테스트 헬퍼에 `live_failure: None` | 수정 |
| `src/integrations/tauri-native-bridge.ts` | `liveFailure` 타입, `listenNativeProviderUsage`, `TauriUnlisten` export | 수정 |
| `src/data/providers.ts` | `PlanQuota.liveFailure` | 수정 |
| `src/data/usage-freshness.ts` | `CLAUDE_TOKEN_EXPIRED`, 토큰 만료 라벨, `autoRefreshLabel` | 수정 |
| `src/App.tsx` | `quotaFromSnapshot`에 `liveFailure`, 이벤트 수신 effect | 수정 |
| `tests/usage-freshness.test.ts` | 라벨·`autoRefreshLabel` 테스트 | 수정 |
| 문서 5종 | 스펙 2종·plan·memory-budget·README | 수정 |

---

### Task 1: 스냅샷 `live_failure` 필드와 실패 코드

**Files:**
- Modify: `src-tauri/src/provider_usage.rs:37-49` (구조체), `:152-165` (`unavailable_snapshot`), `:348-480` (`codex_snapshot` 반환 6곳), `:575-596` (`read_claude_oauth_token_at`), `:803-818` (`claude_live_failure_hint`), `:820-961` (`claude_snapshot` 반환 3곳)
- Modify: `src-tauri/src/desktop_shell.rs:418-436`, `src-tauri/src/lib.rs:435-449`, `src-tauri/src/taskbar_strip.rs:236-249`, `src-tauri/src/tray_badge.rs:108-124` (테스트 헬퍼)
- Modify: `src/integrations/tauri-native-bridge.ts:61-73`, `src/data/providers.ts:29-42`, `src/App.tsx:110-140` (`quotaFromSnapshot`)
- Test: `src-tauri/src/provider_usage.rs` tests 모듈

**Interfaces:**
- Produces: `ProviderUsageSnapshot.live_failure: Option<String>` (serde `liveFailure`), `pub const CLAUDE_TOKEN_EXPIRED: &str = "claude-oauth-token-expired"` (provider_usage), TS `NativeProviderUsageSnapshot.liveFailure?: string | null`, `PlanQuota.liveFailure?: string | null`

- [ ] **Step 1: 실패 테스트 작성** — `provider_usage.rs` tests 모듈(`fn parses_codex_primary_and_secondary_windows` 바로 위)에 추가

```rust
    #[test]
    fn unavailable_snapshot_reports_a_machine_readable_failure() {
        let snapshot = unavailable_snapshot("codex", "Codex CLI를 찾지 못했습니다.");
        assert_eq!(snapshot.live_failure.as_deref(), Some("runtime-unavailable"));
        assert_eq!(snapshot.connection_state, "not-installed");
        assert_eq!(CLAUDE_TOKEN_EXPIRED, "claude-oauth-token-expired");
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked unavailable_snapshot_reports -- --nocapture`
Expected: 컴파일 오류 `no field live_failure` / `cannot find value CLAUDE_TOKEN_EXPIRED`

- [ ] **Step 3: 구조체·상수·구성 지점 수정**

구조체(`pub message: String,` 뒤):

```rust
    pub message: String,
    /// Machine-readable reason the newest lookup could not deliver fresh windows
    /// (`None` on success). Read by the refresh scheduler and the freshness labels.
    pub live_failure: Option<String>,
```

상수(`const CLAUDE_USAGE_TIMEOUT` 아래):

```rust
/// Live lookup failure code for an expired Claude Code access token. The scheduler holds
/// Claude until the credentials file changes, and both freshness labels turn it into
/// "로그인 갱신 필요".
pub const CLAUDE_TOKEN_EXPIRED: &str = "claude-oauth-token-expired";
```

`read_claude_oauth_token_at`의 `return Err("claude-oauth-token-expired".to_string());` → `return Err(CLAUDE_TOKEN_EXPIRED.to_string());`
`claude_live_failure_hint`의 `Some("claude-oauth-token-expired") =>` → `Some(CLAUDE_TOKEN_EXPIRED) =>`

각 구성 지점의 `message: …,` 바로 뒤에 한 줄 추가:

| 위치 | 추가 줄 |
|---|---|
| `unavailable_snapshot` | `live_failure: Some("runtime-unavailable".to_string()),` |
| `codex_snapshot` App Server 시작 실패(`CodexRpc::start` Err) | `live_failure: Some(reason),` (`format!("… ({reason})")`가 참조만 하므로 뒤에서 이동 가능) |
| `codex_snapshot` `account/read` 실패 | `live_failure: Some(reason),` |
| `codex_snapshot` 계정 없음(로그아웃) | `live_failure: Some("codex-signed-out".to_string()),` |
| `codex_snapshot` `!subscription_auth` | `live_failure: Some("codex-api-key-auth".to_string()),` |
| `codex_snapshot` `account/rateLimits/read` 실패 | `live_failure: Some(reason),` |
| `codex_snapshot` 최종 반환 | `live_failure: None,` |
| `claude_snapshot` `auth status` 실행 실패 | `live_failure: Some("claude-auth-status-failed".to_string()),` |
| `claude_snapshot` 로그아웃 | `live_failure: Some("claude-signed-out".to_string()),` |
| `claude_snapshot` 최종 반환 | `live_failure: live_error,` (기존 `let live_error = live_result.as_ref().err().cloned();`를 그대로 이동) |

테스트 헬퍼 4곳(`desktop_shell.rs` `claude_reset_refresh_starts_only_after_a_window_reset`, `lib.rs` `make_snapshot`, `taskbar_strip.rs` `snapshot`, `tray_badge.rs` `snapshot`)의 `message: …,` 뒤에 `live_failure: None,` 추가.

- [ ] **Step 4: TS 타입·매핑**

`tauri-native-bridge.ts` `NativeProviderUsageSnapshot`의 `message: string;` 뒤:

```ts
  liveFailure?: string | null; // Newest lookup failure code, e.g. "claude-oauth-token-expired"; absent on success.
```

`providers.ts` `PlanQuota`의 `statusMessage: string;` 뒤:

```ts
  liveFailure?: string | null; // Native lookup failure code; undefined for browser demo data.
```

`App.tsx` `quotaFromSnapshot` 반환 객체의 `statusMessage: snapshot.message,` 뒤:

```ts
    liveFailure: snapshot.liveFailure ?? null,
```

- [ ] **Step 5: 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked && cargo test --lib --offline --locked --features native-oauth && cargo build --release --offline --locked 2>&1 | grep -c warning`
Expected: `71 passed; 0 failed; 8 ignored` / `73 passed` / 경고 `0`
Run: `npm run build && npm test`
Expected: tsc 오류 없음, `ℹ pass 48`

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/provider_usage.rs src-tauri/src/desktop_shell.rs src-tauri/src/lib.rs src-tauri/src/taskbar_strip.rs src-tauri/src/tray_badge.rs src/integrations/tauri-native-bridge.ts src/data/providers.ts src/App.tsx
git commit -m "feat(usage): expose the newest lookup failure code on provider snapshots

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: 순수 스케줄러 `usage_refresh.rs`

**Files:**
- Create: `src-tauri/src/usage_refresh.rs`
- Modify: `src-tauri/src/lib.rs:10-14` (`mod` 선언)
- Test: 같은 파일 `tests` 모듈

**Interfaces:**
- Consumes: `crate::provider_usage::{ProviderUsageSnapshot, CLAUDE_TOKEN_EXPIRED}` (Task 1)
- Produces: `pub const TICK: Duration`, `pub const INTERVAL_SECS/RESET_RETRY_SECS/BACKOFF_MAX_SECS/TOKEN_HOLD_SECS: u64`, `pub enum Provider { Claude, Codex }` + `fn id(self) -> &'static str`, `pub enum Outcome { Ok, TokenExpired, Failed }` + `fn label(self) -> &'static str`, `pub fn outcome_for(&ProviderUsageSnapshot) -> Outcome`, `pub struct Inputs { now, strip_enabled, claude_reset_due, credentials_modified: Option<u64> }`, `pub struct Scheduler` + `fn new(now: u64)`, `fn plan(&mut self, Inputs) -> Vec<Provider>`, `fn record(&mut self, Provider, now: u64, Outcome)`, `pub fn backoff_secs(u32) -> u64`

- [ ] **Step 1: 모듈 선언** — `lib.rs`의 `mod tray_badge;` 뒤

```rust
#[cfg(any(target_os = "windows", test))]
#[allow(dead_code)] // Task 3 wires the loop; the allow is removed there.
mod usage_refresh;
```

- [ ] **Step 2: 실패 테스트 작성** — 새 파일 `usage_refresh.rs`를 테스트만 담아 생성

```rust
//! Pure scheduling rules for the taskbar strip's periodic provider refresh (spec
//! 2026-09-18 §2-3). No I/O here: the loop in `desktop_shell.rs` feeds clock,
//! preference and credential observations in and runs the lookups it is told to.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::ProviderUsageSnapshot;

    fn input(now: u64) -> Inputs {
        Inputs { now, strip_enabled: true, claude_reset_due: false, credentials_modified: Some(1) }
    }

    #[test]
    fn first_periodic_refresh_waits_one_full_interval_after_start() {
        let mut scheduler = Scheduler::new(1_000);
        assert!(scheduler.plan(input(1_000 + INTERVAL_SECS - 1)).is_empty());
        assert_eq!(scheduler.plan(input(1_000 + INTERVAL_SECS)), vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn disabled_strip_never_plans_a_lookup() {
        let mut scheduler = Scheduler::new(0);
        let mut off = input(10_000);
        off.strip_enabled = false;
        off.claude_reset_due = true;
        assert!(scheduler.plan(off).is_empty());
        assert_eq!(scheduler.plan(input(10_000)), vec![Provider::Claude, Provider::Codex]);
    }

    #[test]
    fn claude_is_planned_before_codex_and_each_follows_its_own_clock() {
        let mut scheduler = Scheduler::new(0);
        scheduler.record(Provider::Codex, 100, Outcome::Ok);
        assert_eq!(scheduler.plan(input(120)), vec![Provider::Claude]);
        scheduler.record(Provider::Claude, 120, Outcome::Ok);
        assert_eq!(scheduler.plan(input(220)), vec![Provider::Codex]);
    }

    #[test]
    fn reset_boundary_retries_claude_after_the_minimum_gap_only() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.record(Provider::Claude, 1_000, Outcome::Ok);
        let mut due = input(1_000 + RESET_RETRY_SECS - 1);
        due.claude_reset_due = true;
        assert!(scheduler.plan(due).is_empty());
        due.now = 1_000 + RESET_RETRY_SECS;
        assert_eq!(scheduler.plan(due), vec![Provider::Claude]);
    }

    #[test]
    fn expired_token_holds_claude_until_the_safety_cap() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        scheduler.record(Provider::Claude, 1_000, Outcome::TokenExpired);
        let mut tick = input(1_000 + INTERVAL_SECS);
        tick.claude_reset_due = true;
        assert_eq!(scheduler.plan(tick), vec![Provider::Codex], "a held Claude is skipped even at a reset boundary");
        scheduler.record(Provider::Codex, tick.now, Outcome::Ok);
        tick.now = 1_000 + TOKEN_HOLD_SECS - 1;
        assert!(!scheduler.plan(tick).contains(&Provider::Claude));
        tick.now = 1_000 + TOKEN_HOLD_SECS;
        assert!(scheduler.plan(tick).contains(&Provider::Claude), "the hold expires after 30 minutes as a safety net");
    }

    #[test]
    fn a_credentials_rewrite_refreshes_claude_immediately_and_clears_the_hold() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        scheduler.record(Provider::Claude, 1_000, Outcome::TokenExpired);
        let mut changed = input(1_015);
        changed.credentials_modified = Some(2);
        assert_eq!(scheduler.plan(changed), vec![Provider::Claude]);
        scheduler.record(Provider::Claude, 1_015, Outcome::Ok);
        let mut same = input(1_030);
        same.credentials_modified = Some(2);
        assert!(scheduler.plan(same).is_empty(), "the same stamp does not retrigger");
    }

    #[test]
    fn the_first_credentials_observation_never_triggers_a_refresh() {
        let mut scheduler = Scheduler::new(1_000);
        let mut first = input(1_015);
        first.credentials_modified = Some(42);
        assert!(scheduler.plan(first).is_empty());
    }

    #[test]
    fn credential_changes_seen_while_the_strip_is_off_do_not_fire_later() {
        let mut scheduler = Scheduler::new(1_000);
        scheduler.plan(input(1_000));
        let mut off = input(1_015);
        off.strip_enabled = false;
        off.credentials_modified = Some(2);
        assert!(scheduler.plan(off).is_empty());
        let mut on = input(1_030);
        on.credentials_modified = Some(2);
        assert!(scheduler.plan(on).is_empty(), "the change was consumed while off; the regular interval takes over");
    }

    #[test]
    fn failures_back_off_exponentially_up_to_thirty_minutes() {
        assert_eq!([1, 2, 3, 4, 5, 6, 40].map(backoff_secs), [120, 240, 480, 960, 1_800, 1_800, 1_800]);
        let mut scheduler = Scheduler::new(0);
        scheduler.record(Provider::Codex, 1_000, Outcome::Failed);
        scheduler.record(Provider::Codex, 1_120, Outcome::Failed);
        assert!(!scheduler.plan(input(1_120 + 239)).contains(&Provider::Codex));
        assert!(scheduler.plan(input(1_120 + 240)).contains(&Provider::Codex));
        scheduler.record(Provider::Codex, 1_360, Outcome::Ok);
        assert!(!scheduler.plan(input(1_360 + 119)).contains(&Provider::Codex));
        assert!(scheduler.plan(input(1_360 + 120)).contains(&Provider::Codex), "success resets the backoff to the regular interval");
    }

    #[test]
    fn outcome_follows_the_snapshot_failure_code() {
        let mut snapshot = ProviderUsageSnapshot {
            provider_id: "claude".to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: "connected".to_string(),
            auth_method: None,
            plan_type: None,
            source: Some("claude-usage-api".to_string()),
            last_synced_at: Some(1),
            bridge_installed: false,
            windows: Vec::new(),
            message: String::new(),
            live_failure: None,
        };
        assert_eq!(outcome_for(&snapshot), Outcome::Ok);
        snapshot.live_failure = Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED.to_string());
        assert_eq!(outcome_for(&snapshot), Outcome::TokenExpired);
        snapshot.live_failure = Some("claude-usage-http-429".to_string());
        assert_eq!(outcome_for(&snapshot), Outcome::Failed);
        assert_eq!([Outcome::Ok.label(), Outcome::TokenExpired.label(), Outcome::Failed.label()], ["ok", "token-expired", "failed"]);
        assert_eq!([Provider::Claude.id(), Provider::Codex.id()], ["claude", "codex"]);
    }
}
```

- [ ] **Step 3: 실패 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked usage_refresh`
Expected: 컴파일 오류 `cannot find type Scheduler` 등

- [ ] **Step 4: 구현** — 파일 상단(`//!` 주석과 `#[cfg(test)]` 사이)에 삽입

```rust
use std::time::Duration;

use crate::provider_usage::{ProviderUsageSnapshot, CLAUDE_TOKEN_EXPIRED};

/// Sleep between loop iterations; the strip is redrawn on every tick so age labels advance.
pub const TICK: Duration = Duration::from_secs(15);
/// Regular refresh interval per provider.
pub const INTERVAL_SECS: u64 = 120;
/// Minimum gap before a Claude reset boundary triggers another attempt.
pub const RESET_RETRY_SECS: u64 = 60;
/// Upper bound for the exponential failure backoff.
pub const BACKOFF_MAX_SECS: u64 = 1_800;
/// Safety cap on the expired-token hold when no credential change is ever observed.
pub const TOKEN_HOLD_SECS: u64 = 1_800;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Ok,
    TokenExpired,
    Failed,
}

impl Outcome {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::TokenExpired => "token-expired",
            Self::Failed => "failed",
        }
    }
}

/// Classifies a finished lookup from the snapshot's `live_failure` code.
pub fn outcome_for(snapshot: &ProviderUsageSnapshot) -> Outcome {
    match snapshot.live_failure.as_deref() {
        None => Outcome::Ok,
        Some(CLAUDE_TOKEN_EXPIRED) => Outcome::TokenExpired,
        Some(_) => Outcome::Failed,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Inputs {
    pub now: u64,
    pub strip_enabled: bool,
    pub claude_reset_due: bool,
    /// Modification stamp of Claude Code's credentials file, `None` when unreadable.
    pub credentials_modified: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
struct Slot {
    last_attempt: u64,
    next_allowed: u64,
    failures: u32,
}

impl Slot {
    fn new(now: u64) -> Self {
        Self { last_attempt: now, next_allowed: 0, failures: 0 }
    }

    fn elapsed(&self, now: u64) -> u64 {
        now.saturating_sub(self.last_attempt)
    }

    fn allowed(&self, now: u64) -> bool {
        now >= self.next_allowed
    }

    fn interval_due(&self, now: u64) -> bool {
        self.allowed(now) && self.elapsed(now) >= INTERVAL_SECS
    }

    fn clear(&mut self) {
        self.next_allowed = 0;
        self.failures = 0;
    }
}

#[derive(Debug)]
pub struct Scheduler {
    claude: Slot,
    codex: Slot,
    credentials_seen: Option<u64>,
}

impl Scheduler {
    /// `now` counts as the first attempt: the WebView boots with its own refresh, so the
    /// loop must not repeat it within the first tick.
    pub fn new(now: u64) -> Self {
        Self { claude: Slot::new(now), codex: Slot::new(now), credentials_seen: None }
    }

    /// Providers to refresh on this tick, Claude first. The credentials stamp is tracked
    /// even while the strip is off so a change consumed then never fires later; the first
    /// observation only records the stamp.
    pub fn plan(&mut self, input: Inputs) -> Vec<Provider> {
        let credentials_changed = matches!(
            (self.credentials_seen, input.credentials_modified),
            (Some(seen), Some(current)) if seen != current
        );
        if input.credentials_modified.is_some() {
            self.credentials_seen = input.credentials_modified;
        }
        if !input.strip_enabled {
            return Vec::new();
        }
        let now = input.now;
        let mut due = Vec::with_capacity(2);
        if credentials_changed {
            self.claude.clear();
            due.push(Provider::Claude);
        } else if self.claude.interval_due(now)
            || (input.claude_reset_due && self.claude.allowed(now) && self.claude.elapsed(now) >= RESET_RETRY_SECS)
        {
            due.push(Provider::Claude);
        }
        if self.codex.interval_due(now) {
            due.push(Provider::Codex);
        }
        due
    }

    pub fn record(&mut self, provider: Provider, now: u64, outcome: Outcome) {
        let slot = match provider {
            Provider::Claude => &mut self.claude,
            Provider::Codex => &mut self.codex,
        };
        slot.last_attempt = now;
        match outcome {
            Outcome::Ok => slot.clear(),
            Outcome::TokenExpired => {
                slot.failures = 0;
                slot.next_allowed = now + TOKEN_HOLD_SECS;
            }
            Outcome::Failed => {
                slot.failures = slot.failures.saturating_add(1);
                slot.next_allowed = now + backoff_secs(slot.failures);
            }
        }
    }
}

/// 120·2^(n−1) seconds capped at 30 minutes: 120, 240, 480, 960, 1800, 1800…
pub fn backoff_secs(consecutive_failures: u32) -> u64 {
    let exponent = consecutive_failures.saturating_sub(1).min(16);
    (INTERVAL_SECS << exponent).min(BACKOFF_MAX_SECS)
}
```

- [ ] **Step 5: 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked && cargo test --lib --offline --locked --features native-oauth && cargo build --release --offline --locked 2>&1 | grep -c warning`
Expected: `81 passed; 0 failed; 8 ignored` / `83 passed` / `0`

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/usage_refresh.rs src-tauri/src/lib.rs
git commit -m "feat(strip): add the pure refresh scheduler with backoff and token hold

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: 공통 갱신 경로·이벤트·루프 교체·자격 증명 stamp·로그

**Files:**
- Modify: `src-tauri/src/provider_usage.rs` (`claude_credentials_path` 아래에 stamp 함수 2개 + 테스트)
- Modify: `src-tauri/src/lib.rs:10-29` (import·mod), `:245-264` (커맨드), `:320-329` (`set_strip`), `:385-389` (setup)
- Modify: `src-tauri/src/desktop_shell.rs:5-6` (import), `:291-347` (루프 전면 교체)
- Test: `provider_usage.rs` tests 모듈

**Interfaces:**
- Consumes: `usage_refresh::{Scheduler, Inputs, Outcome, TICK, outcome_for}` (Task 2)
- Produces: `provider_usage::modified_stamp(&Path) -> Option<u64>`, `provider_usage::claude_credentials_modified() -> Option<u64>`, `lib::PROVIDER_USAGE_EVENT`, `lib::refresh_provider(AppHandle, String) -> Result<ProviderUsageSnapshot, String>`(async), `desktop_shell::ensure_usage_refresh_loop(&AppHandle)`

- [ ] **Step 1: 실패 테스트 작성** — `provider_usage.rs` tests 모듈에 추가

```rust
    #[test]
    fn modified_stamp_tracks_rewrites_and_missing_files() {
        let dir = temp_dir("stamp");
        let path = dir.join("credentials.json");
        assert_eq!(modified_stamp(&path), None);
        fs::write(&path, b"{}").unwrap();
        let first = modified_stamp(&path).expect("a written file has a stamp");
        let later = SystemTime::now() + Duration::from_secs(5);
        fs::File::options().write(true).open(&path).unwrap().set_modified(later).unwrap();
        assert!(modified_stamp(&path).unwrap() > first);
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked modified_stamp`
Expected: 컴파일 오류 `cannot find function modified_stamp`

- [ ] **Step 3: stamp 함수 구현** — `provider_usage.rs`의 `fn claude_credentials_path()` 바로 아래

```rust
/// Modification stamp (unix milliseconds) of the file at `path`, `None` when unreadable.
pub(crate) fn modified_stamp(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_millis() as u64)
}

/// Claude Code rewrites its credentials file whenever a session refreshes the OAuth token;
/// the refresh loop watches this stamp to retry right away instead of waiting out a hold.
pub(crate) fn claude_credentials_modified() -> Option<u64> {
    claude_credentials_path().and_then(|path| modified_stamp(&path))
}
```

- [ ] **Step 4: `lib.rs` 공통 경로·이벤트**

import 블록을 다음으로 교체(`Emitter`를 feature 밖으로):

```rust
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
#[cfg(feature = "native-oauth")]
use tauri::Runtime;
```

`mod usage_refresh;` 선언에서 `#[allow(dead_code)]` 줄을 삭제.

커맨드 `provider_usage_snapshot` 전체를 다음으로 교체:

```rust
/// Event carrying a fresh `ProviderUsageSnapshot` to the WebView. It also fires for
/// command-initiated lookups; the frontend applies whichever snapshot arrives last.
pub(crate) const PROVIDER_USAGE_EVENT: &str = "provider-usage-updated";

/// Shared by the `provider_usage_snapshot` command and the native refresh loop.
pub(crate) async fn refresh_provider(
    app: AppHandle,
    provider_id: String,
) -> Result<provider_usage::ProviderUsageSnapshot, String> {
    let ticket = app.state::<AppState>().begin_snapshot_request(&provider_id);
    let worker_provider_id = provider_id.clone();
    let snapshot = tauri::async_runtime::spawn_blocking(move || provider_usage::snapshot(&worker_provider_id))
        .await
        .map_err(|_| "provider usage worker failed".to_string())?;
    // A newer request for the same provider may have finished first; the standby cache, the
    // tray badge, the strip and the WebView follow the newest request only.
    let state = app.state::<AppState>();
    if state.snapshot_request_is_current(&provider_id, ticket) {
        state.remember_snapshot(&snapshot);
        desktop_shell::update_tray_badge(&app);
        desktop_shell::update_taskbar_strip(&app);
        if let Err(error) = app.emit(PROVIDER_USAGE_EVENT, &snapshot) {
            eprintln!("spectra: provider usage event was not delivered: {error}");
        }
    }
    Ok(snapshot)
}

#[tauri::command]
async fn provider_usage_snapshot(
    app: AppHandle,
    provider_id: String,
) -> Result<provider_usage::ProviderUsageSnapshot, String> {
    refresh_provider(app, provider_id).await
}
```

`set_strip`와 `setup`의 `desktop_shell::ensure_taskbar_refresh_loop(` 2곳 → `desktop_shell::ensure_usage_refresh_loop(`.

- [ ] **Step 5: `desktop_shell.rs` 루프 교체**

import: `use std::time::{Duration, SystemTime, UNIX_EPOCH};` → `use std::time::{SystemTime, UNIX_EPOCH};`

`ensure_taskbar_refresh_loop` 함수 전체(doc 주석 포함)를 다음으로 교체. `claude_reset_due`와 그 테스트는 그대로 둔다:

```rust
/// Periodic provider refresh while the strip is visible: two-minute cadence, the Claude reset
/// boundary, an immediate retry when Claude Code rewrites its credentials, exponential backoff
/// on failures and a hold while the OAuth token is expired (spec 2026-09-18 §2-3).
#[cfg(target_os = "windows")]
pub(crate) fn ensure_usage_refresh_loop(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    if !state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false) {
        return;
    }
    if state
        .strip_refresh_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let worker = app.clone();
    let spawned = std::thread::Builder::new()
        .name("spectra-usage-refresh".to_string())
        .spawn(move || {
            let mut scheduler = crate::usage_refresh::Scheduler::new(unix_now());
            loop {
                std::thread::sleep(crate::usage_refresh::TICK);
                let state = worker.state::<crate::AppState>();
                let enabled = state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false);
                let snapshots = state.last_snapshots.lock().map(|list| list.clone()).unwrap_or_default();
                let now = unix_now();
                if enabled {
                    // Age/reset labels must advance even with no new response; this performs no lookup.
                    update_taskbar_strip(&worker);
                }
                let due = scheduler.plan(crate::usage_refresh::Inputs {
                    now,
                    strip_enabled: enabled,
                    claude_reset_due: claude_reset_due(&snapshots, now),
                    credentials_modified: crate::provider_usage::claude_credentials_modified(),
                });
                // Sequential on purpose: never spawn both provider CLIs at once.
                for provider in due {
                    let started = std::time::Instant::now();
                    let result = tauri::async_runtime::block_on(crate::refresh_provider(worker.clone(), provider.id().to_string()));
                    let outcome = match &result {
                        Ok(snapshot) => crate::usage_refresh::outcome_for(snapshot),
                        Err(error) => {
                            eprintln!("spectra: scheduled {} refresh failed: {error}", provider.id());
                            crate::usage_refresh::Outcome::Failed
                        }
                    };
                    scheduler.record(provider, unix_now(), outcome);
                    crate::standby::log_timing(&format!("usage_refresh:{}:{}", provider.id(), outcome.label()), started.elapsed());
                }
            }
        });
    if let Err(error) = spawned {
        state.strip_refresh_started.store(false, Ordering::Release);
        eprintln!("spectra: usage refresh loop is unavailable: {error}");
    }
}

#[cfg(target_os = "windows")]
fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}
```

- [ ] **Step 6: 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked && cargo test --lib --offline --locked --features native-oauth && cargo build --release --offline --locked 2>&1 | grep -c warning && cargo build --release --offline --locked --features native-oauth 2>&1 | grep -c warning`
Expected: `82 passed; 0 failed; 8 ignored` / `84 passed` / `0` / `0`

- [ ] **Step 7: 커밋**

```bash
git add src-tauri/src/provider_usage.rs src-tauri/src/lib.rs src-tauri/src/desktop_shell.rs
git commit -m "feat(strip): refresh both providers every two minutes and notify the WebView

Replace the Claude reset-only watcher with the scheduler: credentials rewrites retry at once, failures back off, an expired token holds instead of retrying every minute, and every accepted snapshot is emitted as provider-usage-updated.

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: 토큰 만료 라벨 (스트립 툴팁 + 창)

**Files:**
- Modify: `src-tauri/src/taskbar_strip.rs:86-108` (`claude_status`)
- Modify: `src/data/usage-freshness.ts`
- Test: `src-tauri/src/taskbar_strip.rs` tests, `tests/usage-freshness.test.ts`

**Interfaces:**
- Consumes: `ProviderUsageSnapshot.live_failure`, `provider_usage::CLAUDE_TOKEN_EXPIRED`, `PlanQuota.liveFailure` (Task 1)
- Produces: TS `export const CLAUDE_TOKEN_EXPIRED = "claude-oauth-token-expired"`, 라벨 `"로그인 갱신 필요"`

- [ ] **Step 1: Rust 실패 테스트** — `taskbar_strip.rs` tests의 `claude_signed_in_cache_still_surfaces_usage_login_failures` 뒤

```rust
    #[test]
    fn claude_tooltip_asks_for_a_login_refresh_when_the_token_expired() {
        let mut claude = snapshot("claude", "stale", vec![window("rolling", 72.0)]);
        claude.source = Some("claude-statusline".into());
        claude.last_synced_at = Some(1_000);
        claude.live_failure = Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED.into());
        claude.message = "Claude Code 로그인 갱신이 필요합니다. 토큰이 만료되어 Claude Code를 한 번 실행해 주세요. 마지막 동기화 값을 표시합니다.".into();
        assert_eq!(claude_status(&claude, 1_300), "로그인 갱신 필요");
        let model = build_model_at(&[claude], StripTheme::Dark, 1_300).unwrap();
        assert!(model.tooltip.contains("Claude · 로그인 갱신 필요"));
        assert!(model.tooltip.contains("토큰이 만료되어"));
        assert_eq!(model.segments[1].value, "72%", "cached usage stays visible");
    }
```

- [ ] **Step 2: TS 실패 테스트** — `tests/usage-freshness.test.ts`: import를 `import { CLAUDE_TOKEN_EXPIRED, claudeFreshness } from "../src/data/usage-freshness.ts";`로 바꾸고 마지막 `it` 뒤에 추가

```ts
  it("asks for a login refresh when the Claude Code token expired, keeping the cached usage", () => {
    const expired = claudeFreshness({ ...quota, connectionState: "stale", source: "claude-statusline", liveFailure: CLAUDE_TOKEN_EXPIRED, statusMessage: "Claude Code 로그인 갱신이 필요합니다. 토큰이 만료되어 Claude Code를 한 번 실행해 주세요." }, captured);
    assert.equal(expired.label, "로그인 갱신 필요");
    assert.ok(expired.tooltip.includes("토큰이 만료되어"));
    assert.equal(claudeFreshness({ ...quota, liveFailure: null }, captured + 30_000).label, "동기화됨");
  });
```

- [ ] **Step 3: 실패 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked claude_tooltip_asks` → Expected: 실패(`"갱신 대기 (캐시)"`가 나옴)
Run: `npm test` → Expected: `fail 1`(`CLAUDE_TOKEN_EXPIRED` undefined)

- [ ] **Step 4: 구현**

`taskbar_strip.rs` `claude_status`의 `return "로그인 필요";` 블록 뒤에:

```rust
    if snapshot.live_failure.as_deref() == Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED) {
        return "로그인 갱신 필요";
    }
```

`usage-freshness.ts` 전체를 다음으로 교체:

```ts
import type { PlanQuota } from "./providers.ts";

/** Native lookup failure code for an expired Claude Code access token (provider_usage::CLAUDE_TOKEN_EXPIRED). */
export const CLAUDE_TOKEN_EXPIRED = "claude-oauth-token-expired";

// Keep the labels and expiry boundary aligned with taskbar_strip::claude_status.
export function claudeFreshness(quota: PlanQuota, now: number): Readonly<{ label: string; tooltip: string }> {
  let label: string;
  const captured = quota.lastSyncedAtMs;
  const hasUsage = quota.confidence === "verified" && quota.windows.length > 0;
  const expired = quota.windows.some(window => window.resetsAt != null && window.resetsAt <= now);
  const capturedRecently = captured != null && Number.isFinite(captured) && captured <= now && now - captured <= 86_400_000;

  if (quota.source === "example") label = "브라우저 데모";
  else if (quota.connectionState === "not_installed") label = "Claude Code 설치 필요";
  else if (quota.connectionState === "signed_out") label = "로그인 필요";
  else if (quota.liveFailure === CLAUDE_TOKEN_EXPIRED) label = "로그인 갱신 필요";
  else if (quota.connectionState === "error" && !hasUsage) label = "연결 상태 확인 필요";
  else if (!hasUsage) label = "사용량 갱신 대기";
  else if (quota.connectionState !== "connected" || expired || !capturedRecently || quota.source !== "claude-usage-api") label = "갱신 대기 (캐시)";
  else label = "동기화됨";

  const synced = quota.source === "example" ? "데모 데이터" : quota.lastSyncedAt ?? "기록 없음";
  const lines = [`Claude · ${label}`, `마지막 동기화: ${synced} (데이터 수집 기준)`];
  if (quota.connectionState !== "connected" && quota.statusMessage) lines.push(quota.statusMessage);
  return { label, tooltip: lines.join("\n") };
}
```

- [ ] **Step 5: 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline --locked && cargo test --lib --offline --locked --features native-oauth`
Expected: `83 passed; 0 failed; 8 ignored` / `85 passed`
Run: `npm test && npm run build`
Expected: `ℹ pass 49`, tsc 오류 없음

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/taskbar_strip.rs src/data/usage-freshness.ts tests/usage-freshness.test.ts
git commit -m "feat(usage): label an expired Claude Code token as needing a login refresh

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: 창이 네이티브 갱신 이벤트를 반영

**Files:**
- Modify: `src/integrations/tauri-native-bridge.ts:4` (`TauriUnlisten` export), 파일 끝 (`listenNativeProviderUsage`)
- Modify: `src/data/usage-freshness.ts` (`autoRefreshLabel`)
- Modify: `src/App.tsx:1-11` (import), `:746-757` 뒤 (effect)
- Test: `tests/usage-freshness.test.ts`

**Interfaces:**
- Consumes: `PROVIDER_USAGE_EVENT = "provider-usage-updated"` (Task 3), `refreshSequence`(기존 `createRefreshSequencer`), `quotaFromSnapshot`, `planQuotas`
- Produces: `listenNativeProviderUsage(onEvent: (snapshot: NativeProviderUsageSnapshot) => void): Promise<TauriUnlisten | null>`, `autoRefreshLabel(at: Date): string` → `"자동 확인 HH:MM"`

- [ ] **Step 1: 실패 테스트** — `tests/usage-freshness.test.ts` import에 `autoRefreshLabel` 추가, 파일 끝 `describe` 블록 안 마지막에

```ts
  it("labels a native auto refresh with the wall-clock time", () => {
    assert.equal(autoRefreshLabel(new Date(2027, 0, 15, 9, 5)), "자동 확인 09:05");
    assert.equal(autoRefreshLabel(new Date(2027, 0, 15, 21, 40)), "자동 확인 21:40");
  });
```

- [ ] **Step 2: 실패 확인**

Run: `npm test` → Expected: `fail 1`(`autoRefreshLabel` is not a function)

- [ ] **Step 3: 구현**

`usage-freshness.ts` 끝에 추가:

```ts
const clockFormatter = new Intl.DateTimeFormat("ko-KR", { hour: "2-digit", minute: "2-digit", hour12: false });

/** "조회 시도" label for a refresh the native loop ran on its own. */
export function autoRefreshLabel(at: Date): string {
  return `자동 확인 ${clockFormatter.format(at)}`;
}
```

`tauri-native-bridge.ts`: `type TauriUnlisten = () => void | Promise<void>;` → `export type TauriUnlisten = () => void | Promise<void>;`, 파일 끝에 추가:

```ts
/** Snapshots the native refresh loop (and every command lookup) accepted as newest. */
export async function listenNativeProviderUsage(
  onEvent: (snapshot: NativeProviderUsageSnapshot) => void,
): Promise<TauriUnlisten | null> {
  const listener = nativeGlobal()?.event?.listen;
  if (!listener) return null;
  return listener<NativeProviderUsageSnapshot>("provider-usage-updated", event => onEvent(event.payload));
}
```

`App.tsx` import 수정: `import { claudeFreshness } from "./data/usage-freshness";` → `import { autoRefreshLabel, claudeFreshness } from "./data/usage-freshness";`, 브리지 import 목록에 `listenNativeProviderUsage`와 `type TauriUnlisten` 추가.

`App.tsx`의 부팅 조회 effect(`providersToRefreshAfterBoot` 사용) 바로 뒤에 추가:

```tsx
  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: TauriUnlisten | null = null;
    void listenNativeProviderUsage(snapshot => {
      const id = snapshot.providerId;
      if (id !== "codex" && id !== "claude") return;
      // Claim a fresh ticket so an older in-flight request cannot overwrite this newer snapshot.
      refreshSequence.begin(id);
      setQuotas(current => ({ ...current, [id]: quotaFromSnapshot(snapshot, planQuotas[id]) }));
      setRefreshedAt(autoRefreshLabel(new Date()));
    }).then(handle => {
      if (disposed) void handle?.();
      else unlisten = handle;
    }).catch((error: unknown) => console.warn("SPECTRA: provider usage events are unavailable", error));
    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [refreshSequence]);
```

- [ ] **Step 4: 통과 확인**

Run: `npm test && npm run build && npm run verify:memory && npm run verify:tokens && npm run verify:baseline`
Expected: `ℹ pass 50`, tsc 오류 없음, `verify:memory` JS ≤ 240,000 B PASS(기준선 236,604 B + 수백 B), 나머지 PASS

- [ ] **Step 5: 커밋**

```bash
git add src/integrations/tauri-native-bridge.ts src/data/usage-freshness.ts src/App.tsx tests/usage-freshness.test.ts
git commit -m "feat(app): apply native provider usage events so the window matches the strip

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: 문서 정정

**Files:**
- Modify: `docs/superpowers/specs/2026-09-12-taskbar-usage-strip.md` (§1 갱신 행, §1-1, §3-6)
- Modify: `docs/superpowers/plans/2026-09-12-taskbar-usage-strip.md` (전역 제약, 갱신 트리거, 대응표)
- Modify: `docs/performance/memory-budget.md:14`, 2026-09-12 절, 새 절
- Modify: `README.md:49`, `:99`

- [ ] **Step 1: 2026-09-12 스펙**

§1 표의 `| 갱신 | 스트립 갱신은 \`provider_usage_snapshot\` 완료 시점에 일어나며, Claude 초기화 시각이 지난 경우에만 네이티브 감시가 재조회를 시작한다(15초 확인 간격·재시도 최소 60초) | 코드 리뷰 + Claude 초기화 경계 테스트 |` 행을 다음으로 교체:

```markdown
| 갱신 | 스트립 갱신은 `provider_usage_snapshot` 완료 시점에 일어나며, 표시 중에는 네이티브 루프가 2분 주기로 두 공급자를 재조회한다(2026-09-18 스펙 §2-3: 자격 증명 변경 즉시·실패 백오프·토큰 만료 보류) | 코드 리뷰 + `usage_refresh` 스케줄러 단위 테스트 |
```

§1-1의 `무조건 폴링은 없고 Claude 초기화 경계만 제한적으로 감시하며 \`Cargo.lock\`의 새 package는 없다.` → `갱신 정책은 2026-09-18 스펙(2분 주기 갱신)으로 대체됐고 \`Cargo.lock\`의 새 package는 없다.`

§3-6 제목 `### 3-6. 갱신 시점 (무조건 폴링 금지, Claude 초기화 경계만 감시)` → `### 3-6. 갱신 시점 (2026-09-18 개정: 표시 중 2분 주기 갱신)`. 표의 마지막 행 `| 작업표시줄 표시 상태에서 Claude 초기화 시각 경과 | 15초 간격으로 경계를 확인하고, 경과 시 \`provider_usage_snapshot\`을 재호출(재시도 최소 60초) |`을 다음으로 교체:

```markdown
| 작업표시줄 표시 상태의 네이티브 루프(15초 tick) | Claude → Codex 순으로 120초마다 `provider_usage_snapshot` 재호출. Claude 초기화 경과(60초 간격)·`.credentials.json` 변경은 즉시, 실패는 120→1800초 백오프, 토큰 만료는 자격 증명 변경까지 보류 — 규칙 전체는 [2026-09-18 스펙 §2-3](./2026-09-18-strip-periodic-refresh.md) |
```

표 아래 문단 `일반 공급자 폴링은 두지 않는다. 단, … 예외다.` → `2026-09-12 판정의 "무조건 폴링 금지"는 2026-09-18 진단(스트립이 사용자 조작 없이는 갱신되지 않음)으로 폐기했다. 표시 중에만 주기 갱신하며, 표시가 꺼지면 현행대로 조회하지 않는다.`

- [ ] **Step 2: 2026-09-12 plan**

전역 제약의 `- **무조건 폴링 금지**: …` 항목 맨 앞에 `(2026-09-18 대체 — 현행 규칙은 \`docs/superpowers/plans/2026-09-18-strip-periodic-refresh.md\`) ` 를 붙인다. Task 6 코드 블록 안의 `갱신 트리거: \`provider_usage_snapshot\` 완료 · \`WM_SETTINGCHANGE\` · \`WM_DISPLAYCHANGE\` · \`TaskbarCreated\` · Claude 초기화 시각 경과(15초 확인 간격, 재시도 최소 60초).` → `갱신 트리거: \`provider_usage_snapshot\` 완료 · \`WM_SETTINGCHANGE\` · \`WM_DISPLAYCHANGE\` · \`TaskbarCreated\` · 표시 중 2분 주기 네이티브 갱신(2026-09-18).` 대응표의 `| §1 무조건 폴링 없음·Claude 초기화 경계 갱신 | Task 4(스냅샷 지점 배선)·5(셸 이벤트·초기화 감시) |` → `| §1 갱신(2026-09-18 개정) | 2026-09-18 plan Task 2·3 |`

- [ ] **Step 3: memory-budget.md**

14행 `- 일반 유휴 상태에서는 provider 폴링을 하지 않습니다. 시작·수동 새로고침 때 조회하며, 작업표시줄 표시가 켜진 경우에만 Claude 초기화 시각 경과를 제한적으로 감시해 재조회합니다.` → `- 작업표시줄 표시가 꺼진 유휴 상태에서는 provider 폴링을 하지 않습니다. 표시가 켜진 동안에만 네이티브 루프가 2분 주기로 Claude → Codex를 순차 재조회하며, 실패는 최대 30분까지 백오프하고 토큰 만료 중에는 조회하지 않습니다(2026-09-18).`

2026-09-12 절의 `스트립은 공급자 스냅샷 완료와 셸 변화 이벤트에서 캐시된 값을 다시 그리며, 작업표시줄 표시 상태에서 Claude 초기화 시각이 지난 경우에만 별도 경량 감시 스레드가 제한적 재조회를 사용한다.` → `스트립은 공급자 스냅샷 완료와 셸 변화 이벤트에서 캐시된 값을 다시 그린다. 갱신 주기는 2026-09-18 절에서 개정했다.`

파일 끝에 절 추가:

```markdown
## 2026-09-18 스트립 주기 갱신 (A안)

변경: 작업표시줄 표시가 켜진 동안 네이티브 루프(`spectra-usage-refresh`, 15초 tick)가 Claude → Codex를 120초마다 순차 재조회한다. `.credentials.json`이 다시 쓰이면 다음 tick에 Claude를 즉시 재조회하고, 실패는 120·240·480·960·1800초로 백오프하며, 토큰 만료(`claude-oauth-token-expired`)는 자격 증명 변경 또는 30분까지 조회를 보류한다. 갱신된 스냅샷은 `provider-usage-updated` 이벤트로 WebView에도 전달된다. 규칙과 근거는 [`docs/superpowers/specs/2026-09-18-strip-periodic-refresh.md`](../superpowers/specs/2026-09-18-strip-periodic-refresh.md)에 있다.

비용(2026-09-18 실측, 설치본 0.2.3): Claude 조회 1.1초(`claude auth status` + HTTPS 1회), Codex 조회 1.3초(`codex app-server` + RPC 2회). 두 프로세스 모두 조회 직후 종료되므로 상주 메모리 증가는 없고, 2분당 약 2.4초의 단기 실행이 더해진다. 위 2026-09-12 standby-strip 30분 실측(평균 33.6 MB)은 주기 갱신 이전 값이며, 같은 시나리오 재측정은 후속 항목이다.
```

- [ ] **Step 4: README**

49행의 `…Claude 초기화 시각이 지나면 창이 닫힌 대기 상태에서도 사용량을 다시 확인합니다(기본값 꺼짐, 주 작업표시줄의 가로 배치만 지원).` → `…표시가 켜진 동안 2분마다 Codex·Claude를 다시 확인하고 Claude Code가 로그인 토큰을 갱신하면 곧바로 반영합니다(기본값 꺼짐, 주 작업표시줄의 가로 배치만 지원).`

99행의 `…작업표시줄 표시가 켜진 경우에만 Claude 초기화 시각 경과를 감시해 필요한 순간에 재조회합니다.` → `…작업표시줄 표시가 켜진 동안에만 2분 주기로 재조회하며 실패 시 최대 30분까지 간격을 늘립니다.`

- [ ] **Step 5: 커밋**

```bash
git add docs/superpowers/specs/2026-09-12-taskbar-usage-strip.md docs/superpowers/plans/2026-09-12-taskbar-usage-strip.md docs/performance/memory-budget.md README.md docs/superpowers/specs/2026-09-18-strip-periodic-refresh.md docs/superpowers/plans/2026-09-18-strip-periodic-refresh.md
git commit -m "docs(strip): replace the no-polling rule with the two-minute refresh policy

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 7: 전체 게이트와 실물 검증

**Files:**
- Modify: `docs/superpowers/specs/2026-09-18-strip-periodic-refresh.md` 상태 줄 (결과 기록)

- [ ] **Step 1: 전체 게이트**

```bash
cd src-tauri && cargo test --lib --offline --locked && cargo test --lib --offline --locked --features native-oauth && cargo build --release --offline --locked 2>&1 | grep -c warning; cargo build --release --offline --locked --features native-oauth 2>&1 | grep -c warning; cd .. && npm test && npm run build && npm run verify:memory && npm run verify:tokens && npm run verify:baseline && git diff --stat main -- src-tauri/Cargo.lock package-lock.json
```

Expected: `83 passed; 8 ignored` / `85 passed` / `0` / `0` / `ℹ pass 50` / tsc 오류 없음 / 3개 verify PASS / lock 파일 diff 없음

- [ ] **Step 2: 프론트 내장 EXE 빌드**

```bash
node node_modules/@tauri-apps/cli/tauri.js build --no-bundle -- --offline --locked
```

Expected: `src-tauri/target/release/spectra-native.exe` 갱신, 오류 없음

- [ ] **Step 3: 실물 검증 — 사용자 승인 후** (트레이 메뉴 "SPECTRA 종료"로 설치본을 끝낸 뒤)

```powershell
$env:SPECTRA_TIMING_LOG = "1"; Start-Process "src-tauri\target\release\spectra-native.exe"
```

5분 대기 후 확인:

```powershell
Select-String -Path "$env:LOCALAPPDATA\SPECTRA\standby-timing.log" -Pattern "usage_refresh:" | Select-Object -Last 8
```

판정: `usage_refresh:claude:ok`·`usage_refresh:codex:ok` 행이 각각 약 120초 간격(±15초)으로 있고 한 tick 안에서 claude 행이 codex 행보다 앞선다. 창을 연 채로 두면 "조회 시도 · 자동 확인 HH:MM"이 2분 안에 갱신되고 창 수치가 스트립 툴팁과 같다. Claude Code 터미널 세션을 새로 열면(토큰 재발급 시) 15초 안에 `usage_refresh:claude:ok` 행이 추가된다. 확인 후 새 EXE를 종료하고 설치본을 다시 실행하거나, 승인 시 `npm run desktop:build`로 설치 파일을 만든다.

- [ ] **Step 4: 스펙 상태 줄 갱신·커밋**

`2026-09-18-strip-periodic-refresh.md`의 `상태: 사용자 승인(…) · 구현 대기` → `상태: 구현 완료(Task 1~6, cargo 83/85 · npm 50 · 경고 0) · 실물 검증 <결과와 시각> · 대기 재측정 후속`

```bash
git add docs/superpowers/specs/2026-09-18-strip-periodic-refresh.md
git commit -m "docs(strip): record the periodic refresh gates and live check

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

## 스펙 대응표

| 스펙 §1 기준 | Task |
|---|---|
| 주기 120초·직렬 Claude → Codex | Task 2(`plan` 순서·`INTERVAL_SECS`)·3(`for` + `block_on`) |
| 자격 증명 변경 즉시 | Task 2(`credentials_changed`)·3(`claude_credentials_modified`) |
| 백오프·토큰 만료 보류 | Task 2(`record`·`backoff_secs`) |
| 창 일치(이벤트) | Task 3(`refresh_provider` emit)·5(수신) |
| 라벨 "로그인 갱신 필요" | Task 1(`live_failure`)·4 |
| 기본값(strip OFF 0회) | Task 2(`disabled_strip_never_plans_a_lookup`) |
| 회귀·의존성 | Task 7 |
| 로그(§2-8) | Task 3(`log_timing`) |
| 문서(§5) | Task 6·7 |
