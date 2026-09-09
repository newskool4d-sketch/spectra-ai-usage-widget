# Phase 4 — 선택형 저메모리 대기 모드 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 사용자가 설정에서 "메모리 절약"을 켜면 창을 닫을 때 WebView 창을 파기해 트레이 상주 시 작업 집합을 ≤ 60 MB로 낮추고, 트레이 클릭 시 테마·창 모드·마지막 스냅샷을 복원해 재생성한다. 기본값은 현행(숨기기·즉시 재표시)이다.

**Architecture:** Rust `AppState`가 UI 선호(`theme`·`solid`·`standby`)·창 모드·공급자별 마지막 스냅샷을 보관하고, 선호는 `<앱 데이터 폴더>/ui-prefs.json`에 Rust가 영속한다(프론트 `localStorage` 금지 원칙 유지). 메인 창은 더 이상 `tauri.conf.json`이 아니라 `setup`에서 `WebviewWindowBuilder`로 만들며, 매 생성 시 `initialization_script`로 `window.__SPECTRA_BOOT__`(선호·모드·스냅샷)를 주입한다 — 스펙의 "초기 URL 쿼리"와 같은 목적이지만 스냅샷 JSON 크기 제한과 이스케이프 문제가 없어 이 경로를 쓴다. 창 닫기 요청은 `standby`에 따라 숨기기(기본) 또는 `destroy`로 분기하고, 마지막 창이 사라져도 `RunEvent::ExitRequested{code: None}`를 막아 트레이만 남긴다. 트레이 클릭·메뉴는 창이 없으면 저장된 모드로 재생성한다. 트레이 아이콘에는 연결된 공급자의 최소 잔여율을 숫자로 그려 넣고(시작·수동 새로고침 시에만 갱신) 재표시 지연은 `SPECTRA_TIMING_LOG=1`일 때만 로그 파일로 기록해 측정한다.

**Tech Stack:** Rust 1.96 · Tauri 2.11(`WebviewWindowBuilder`, `tauri::image::Image`, `tray_by_id`) · serde_json(기존) · React 19 + TypeScript · Node 24 `node:test`

**Spec:** `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — "Phase 4 — 선택형 저메모리 대기 모드" 절(§4) 및 §1 성공 기준 표 19행("(선택) 트레이 대기 모드에서 전체 작업 집합 ≤ 60 MB")

## Global Constraints

- 새 의존성 추가 금지. `serde`·`serde_json`·`tauri`(기존 `tray-icon` feature)만 사용. `tauri::image::Image::new_owned`는 feature 없이 사용 가능(디코더 feature 불필요)
- 프론트 `localStorage`·`sessionStorage`·`setInterval`·`requestAnimationFrame` 금지 유지 — 선호 영속은 Rust 파일(`ui-prefs.json`)로만
- **기본값은 현행 동작**: `UiPrefs::default().standby == false` → 닫기 = 숨기기, 트레이 클릭 = 즉시 재표시. 대기 모드는 설정에서 명시적으로 켠 경우에만
- 폴링 금지: 트레이 배지·스냅샷 갱신은 앱 시작 시와 사용자 새로고침 시(`provider_usage_snapshot` 호출 시점)에만
- Tauri ACL `src-tauri/capabilities/default.json`의 `"windows": ["main"]` — 런타임 생성 창도 라벨 `main`을 유지해야 권한이 적용됨
- 두 feature 구성 모두 통과해야 커밋: `cargo build --release`(기본)·`--features native-oauth`, `cargo test --lib`(기본)·`--features native-oauth`. 경고 0. 현재 테스트 수 기본 24 / native-oauth 26 — 각 Task의 "Expected"에 신규 테스트 수를 더해 기록
- 프론트 게이트(변경 시): `npm run build`·`npm run verify:tokens`·`npm run verify:memory`·`npm run verify:baseline`·`npm test`(현재 24/24)
- Rust 테스트는 `SPECTRA_DATA_DIR` 환경변수를 바꾸지 않는다(테스트 병렬 실행 시 오염). 파일 IO 함수는 경로를 인자로 받는 형태로 두고 임시 디렉터리로 테스트
- 기준 수치(Phase 3, 2026-09-08): 실행 파일 5,809,664 B · 프로세스 7 · mini-idle 30분 평균 422.6 MB · 호스트 단독 작업 집합 ≈ 30.5 MB
- dev server·watch·30분 측정은 실행자가 직접 띄우지 않음(명령 제시·사용자 협조). 릴리스 빌드·`cargo test`·릴리스 exe 1회 실행(사용자 고지 후)은 허용
- 파일 삭제 금지(빌드 산출물 제외)
- 커밋 메시지 말미: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`

## File Structure

| 파일 | 역할 | 작업 |
|---|---|---|
| `src-tauri/src/standby.rs` | `UiPrefs` 모델·영속, `BootState`·`boot_script`, `CloseAction`, 메인 창 생성(`create_main_window`), 타이밍 로그 | 신규 |
| `src-tauri/src/tray_badge.rs` | 3×5 비트맵 숫자 폰트, 32×32 RGBA 렌더, 최소 잔여율 선택 | 신규 |
| `src-tauri/src/lib.rs` | `AppState` 확장, 창 닫기 분기, `ExitRequested` 방지, 선호 커맨드 3개, 스냅샷 보관 | 수정 |
| `src-tauri/src/desktop_shell.rs` | `WindowMode` Default·`pub(crate)` 프로필, 창 없으면 재생성, 저장된 모드로 토글, `dismiss`, 트레이 배지 갱신 | 수정 |
| `src-tauri/src/provider_usage.rs` | `app_data_dir`을 `pub(crate)`로 | 수정(1줄) |
| `src-tauri/tauri.conf.json` | `app.windows`를 빈 배열로(창은 setup에서 생성) | 수정 |
| `src/integrations/boot-state.ts` | `parseBootState` 순수 검증 함수 | 신규 |
| `src/integrations/tauri-native-bridge.ts` | `readBootState`·`getNativeUiPrefs`·`setNativeUiPrefs`·`setNativeStandby` | 수정 |
| `src/App.tsx` | 부트 상태로 초기화, 선호 동기화, 설정 토글, 부트 스냅샷으로 초기 쿼터 | 수정 |
| `tests/boot-state.test.ts` | `parseBootState` 테스트 | 신규 |
| `scripts/verify-memory-budget.mjs` | 스캔 대상에 브리지·boot-state 추가 | 수정 |
| `docs/performance/memory-budget.md` · 스펙 상태 줄·§4 Phase 4 절 | 측정·검증 확정 기록 | 수정 |

---

### Task 1: `standby.rs` — 선호 모델·영속·부트 스크립트·닫기 판정

**Files:**
- Create: `src-tauri/src/standby.rs`
- Modify: `src-tauri/src/provider_usage.rs` — `fn app_data_dir()`을 `pub(crate) fn app_data_dir()`로
- Modify: `src-tauri/src/lib.rs` — `mod standby;` 선언(모듈만; 배선은 Task 2)
- Modify: `src-tauri/src/desktop_shell.rs` — `WindowMode`에 `Default`(Mini), `WindowProfile`·`profile()`을 `pub(crate)`로

**Interfaces:**
- Consumes: `provider_usage::{app_data_dir, ProviderUsageSnapshot}`(Serialize·Clone), `desktop_shell::WindowMode`
- Produces (Task 2·3·5가 사용):
  ```rust
  pub struct UiPrefs { pub theme: String, pub solid: bool, pub standby: bool }   // Default: "dark", false, false
  pub fn normalize_theme(raw: &str) -> &'static str;                             // "light" → "light", 그 외 → "dark"
  pub fn prefs_path() -> Option<PathBuf>;                                        // <app_data_dir>/ui-prefs.json
  pub fn load_prefs_from(path: &Path) -> UiPrefs;                                // 없거나 깨지면 Default
  pub fn save_prefs_to(path: &Path, prefs: &UiPrefs) -> std::io::Result<()>;
  pub fn load_prefs() -> UiPrefs;  pub fn save_prefs(prefs: &UiPrefs) -> std::io::Result<()>;
  pub struct BootState { pub theme: String, pub solid: bool, pub standby: bool, pub mode: &'static str, pub snapshots: Vec<ProviderUsageSnapshot> }
  pub fn boot_script(boot: &BootState) -> String;   // "window.__SPECTRA_BOOT__={json};window.__SPECTRA_MODE__='mini';"
  pub enum CloseAction { Hide, Destroy }  pub fn close_action(standby: bool) -> CloseAction;
  pub fn create_main_window<R: Runtime>(app: &AppHandle<R>, mode: WindowMode, boot: &BootState) -> tauri::Result<WebviewWindow<R>>;
  pub fn log_timing(label: &str, elapsed: Duration);   // SPECTRA_TIMING_LOG=1 일 때만 <app_data_dir>/standby-timing.log 에 "epoch_ms,label,ms"
  ```

- [ ] **Step 1: 준비 편집(모듈 선언·가시성)**

`src-tauri/src/provider_usage.rs`: `fn app_data_dir() -> Option<PathBuf> {` → `pub(crate) fn app_data_dir() -> Option<PathBuf> {`.

`src-tauri/src/desktop_shell.rs`: `#[derive(Clone, Copy, Debug, PartialEq)] pub(crate) enum WindowMode { Mini, Dashboard }` → 아래로 교체:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum WindowMode {
    #[default]
    Mini,
    Dashboard,
}
```

그리고 `struct WindowProfile {` → `pub(crate) struct WindowProfile {`, 필드 3개를 `pub(crate) width: f64, pub(crate) height: f64, pub(crate) always_on_top: bool,`로, `fn profile(self)` → `pub(crate) fn profile(self)`.

`src-tauri/src/lib.rs` 모듈 선언부에 `mod standby;` 추가(`mod provider_usage;` 다음 줄).

- [ ] **Step 2: 테스트 작성(`standby.rs` 하단)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::ProviderUsageSnapshot;

    fn temp_prefs_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spectra-prefs-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("ui-prefs.json")
    }

    #[test]
    fn default_prefs_keep_current_behaviour() {
        let prefs = UiPrefs::default();
        assert_eq!(prefs.theme, "dark");
        assert!(!prefs.solid);
        assert!(!prefs.standby);
        assert_eq!(close_action(prefs.standby), CloseAction::Hide);
    }

    #[test]
    fn standby_closes_by_destroying() {
        assert_eq!(close_action(true), CloseAction::Destroy);
    }

    #[test]
    fn normalize_theme_accepts_only_known_values() {
        assert_eq!(normalize_theme("light"), "light");
        assert_eq!(normalize_theme("dark"), "dark");
        assert_eq!(normalize_theme("neon"), "dark");
    }

    #[test]
    fn prefs_round_trip_through_file() {
        let path = temp_prefs_path("roundtrip");
        let prefs = UiPrefs { theme: "light".into(), solid: true, standby: true };
        save_prefs_to(&path, &prefs).unwrap();
        assert_eq!(load_prefs_from(&path), prefs);
    }

    #[test]
    fn missing_or_corrupt_prefs_fall_back_to_default() {
        let missing = temp_prefs_path("missing");
        assert_eq!(load_prefs_from(&missing), UiPrefs::default());
        let corrupt = temp_prefs_path("corrupt");
        std::fs::write(&corrupt, b"{not json").unwrap();
        assert_eq!(load_prefs_from(&corrupt), UiPrefs::default());
    }

    #[test]
    fn boot_script_embeds_json_and_mode() {
        let boot = BootState { theme: "light".into(), solid: false, standby: true, mode: "dashboard", snapshots: Vec::<ProviderUsageSnapshot>::new() };
        let script = boot_script(&boot);
        assert!(script.starts_with("window.__SPECTRA_BOOT__={"));
        assert!(script.ends_with("window.__SPECTRA_MODE__='dashboard';"));
        let json_part = script.trim_start_matches("window.__SPECTRA_BOOT__=").split(";window.__SPECTRA_MODE__").next().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(parsed["theme"], "light");
        assert_eq!(parsed["standby"], true);
        assert_eq!(parsed["mode"], "dashboard");
        assert!(parsed["snapshots"].as_array().unwrap().is_empty());
    }
}
```

- [ ] **Step 3: 테스트 실행 — 실패 확인**

```bash
cd src-tauri && cargo test --lib standby:: 2>&1 | tail -8; cd ..
```

Expected: 컴파일 실패(`standby` 모듈 본문 없음).

- [ ] **Step 4: `standby.rs` 구현(테스트 블록 위에)**

```rust
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::desktop_shell::{WindowMode, MAIN_WINDOW_LABEL};
use crate::provider_usage::{app_data_dir, ProviderUsageSnapshot};

pub const WINDOW_TITLE: &str = "SPECTRA AI 사용 현황";
const PREFS_FILE: &str = "ui-prefs.json";
const TIMING_LOG_FILE: &str = "standby-timing.log";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiPrefs {
    pub theme: String,
    pub solid: bool,
    pub standby: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self { theme: "dark".to_string(), solid: false, standby: false }
    }
}

pub fn normalize_theme(raw: &str) -> &'static str {
    if raw == "light" { "light" } else { "dark" }
}

pub fn prefs_path() -> Option<PathBuf> {
    app_data_dir().map(|dir| dir.join(PREFS_FILE))
}

pub fn load_prefs_from(path: &Path) -> UiPrefs {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<UiPrefs>(&bytes).ok())
        .map(|mut prefs| {
            prefs.theme = normalize_theme(&prefs.theme).to_string();
            prefs
        })
        .unwrap_or_default()
}

pub fn save_prefs_to(path: &Path, prefs: &UiPrefs) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_vec_pretty(prefs)?)
}

pub fn load_prefs() -> UiPrefs {
    prefs_path().map(|path| load_prefs_from(&path)).unwrap_or_default()
}

pub fn save_prefs(prefs: &UiPrefs) -> std::io::Result<()> {
    match prefs_path() {
        Some(path) => save_prefs_to(&path, prefs),
        None => Ok(()),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootState {
    pub theme: String,
    pub solid: bool,
    pub standby: bool,
    pub mode: &'static str,
    pub snapshots: Vec<ProviderUsageSnapshot>,
}

pub fn boot_script(boot: &BootState) -> String {
    let json = serde_json::to_string(boot).unwrap_or_else(|_| "{}".to_string());
    format!("window.__SPECTRA_BOOT__={json};window.__SPECTRA_MODE__='{}';", boot.mode)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CloseAction {
    Hide,
    Destroy,
}

pub fn close_action(standby: bool) -> CloseAction {
    if standby { CloseAction::Destroy } else { CloseAction::Hide }
}

pub fn create_main_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
    boot: &BootState,
) -> tauri::Result<WebviewWindow<R>> {
    let profile = mode.profile();
    let started = std::time::Instant::now();
    let window = WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
        .title(WINDOW_TITLE)
        .inner_size(profile.width, profile.height)
        .min_inner_size(380.0, 600.0)
        .resizable(true)
        .center()
        .always_on_top(profile.always_on_top)
        .initialization_script(&boot_script(boot))
        .build()?;
    log_timing("create_main_window", started.elapsed());
    Ok(window)
}

pub fn log_timing(label: &str, elapsed: Duration) {
    if std::env::var_os("SPECTRA_TIMING_LOG").is_none() {
        return;
    }
    let Some(dir) = app_data_dir() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let epoch_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(dir.join(TIMING_LOG_FILE)) {
        let _ = writeln!(file, "{epoch_ms},{label},{}", elapsed.as_millis());
    }
}
```

`serde_json::Error` → `std::io::Error` 변환(`?`)은 표준 `From` 구현이 있다. `uuid`는 `[dev-dependencies]`에 있어 테스트에서만 쓴다.

- [ ] **Step 5: 두 구성 빌드·테스트**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib 2>&1 | grep -E "test result|error|panicked" && cargo build --features native-oauth 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib --features native-oauth 2>&1 | grep -E "test result|error|panicked"; cd ..
```

Expected: 경고 0(모듈이 아직 호출되지 않아 `dead_code` 경고가 나면 `standby.rs` 상단에 `#![allow(dead_code)]`를 임시로 두고 Task 2에서 제거한다 — 보고서에 기록). 기본 `30 passed`(24 + 6), native-oauth `32 passed`(26 + 6).

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/standby.rs src-tauri/src/lib.rs src-tauri/src/desktop_shell.rs src-tauri/src/provider_usage.rs
git commit -m "feat(phase4): add standby preferences model, boot script, and close decision

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: 창 생명주기 — setup 생성·닫기 분기·재생성·종료 방지·선호 커맨드

**Files:**
- Modify: `src-tauri/src/lib.rs` — `use` 블록, `AppState`, 커맨드 3개 추가, `provider_usage_snapshot` 시그니처, `run()`
- Modify: `src-tauri/src/desktop_shell.rs` — `show_main_window`·`toggle_main_window`·`hide_main_window`, `dismiss`
- Modify: `src-tauri/tauri.conf.json` — `app.windows`를 `[]`로

**Interfaces:**
- Consumes: Task 1 전부, `crate::AppState`
- Produces:
  - `AppState { ui: Mutex<UiPrefs>, window_mode: Mutex<WindowMode>, last_snapshots: Mutex<Vec<ProviderUsageSnapshot>> }` (+ 기존 OAuth 필드, cfg)
  - 커맨드: `get_ui_prefs() -> UiPrefs`, `set_ui_prefs(theme: String, solid: bool) -> UiPrefs`, `set_standby(enabled: bool) -> UiPrefs` (Task 3 프론트가 호출; camelCase 인자 `theme`·`solid`·`enabled`)
  - `desktop_shell::show_main_window`: 창이 없으면 저장 상태로 재생성 후 표시; 호출 시 `window_mode` 저장
  - `desktop_shell::dismiss<R>(window: &WebviewWindow<R>, standby: bool)`: Hide → `hide()`, Destroy → `destroy()`
  - 마지막 창이 파기돼도 앱은 종료되지 않음(트레이 유지); 트레이 "SPECTRA 종료"는 종료됨

- [ ] **Step 1: Rust 테스트 추가(`desktop_shell.rs` tests)**

```rust
    #[test]
    fn window_mode_defaults_to_mini() {
        assert_eq!(WindowMode::default(), WindowMode::Mini);
    }
```

(파일 상단 `use super::{...}`에 이미 `WindowMode`가 있다.)

- [ ] **Step 2: `lib.rs` — import·AppState·커맨드**

`use` 블록에서 `std::sync::Mutex`, `tauri::{AppHandle, Manager, State}`의 cfg 게이트를 해제한다(이제 항상 사용). 결과:

```rust
#[cfg(feature = "native-oauth")]
use std::io::{Read, Write};
#[cfg(feature = "native-oauth")]
use std::net::TcpListener;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
#[cfg(feature = "native-oauth")]
use tauri::{Emitter, Runtime};
#[cfg(feature = "native-oauth")]
use tauri_plugin_deep_link::DeepLinkExt;
#[cfg(feature = "native-oauth")]
use url::Url;
```

`AppState`를 교체:

```rust
#[derive(Default)]
pub struct AppState {
    #[cfg(feature = "native-oauth")]
    pending_oauth: Mutex<Option<oauth_callback::PendingOAuth>>,
    #[cfg(feature = "native-oauth")]
    loopback_listener: Mutex<Option<TcpListener>>,
    pub(crate) ui: Mutex<standby::UiPrefs>,
    pub(crate) window_mode: Mutex<desktop_shell::WindowMode>,
    pub(crate) last_snapshots: Mutex<Vec<provider_usage::ProviderUsageSnapshot>>,
}

impl AppState {
    fn with_prefs(prefs: standby::UiPrefs) -> Self {
        Self { ui: Mutex::new(prefs), ..Self::default() }
    }

    pub(crate) fn boot_state(&self, mode: desktop_shell::WindowMode) -> standby::BootState {
        let prefs = self.ui.lock().map(|p| p.clone()).unwrap_or_default();
        let snapshots = self.last_snapshots.lock().map(|s| s.clone()).unwrap_or_default();
        standby::BootState { theme: prefs.theme, solid: prefs.solid, standby: prefs.standby, mode: mode.slug(), snapshots }
    }

    pub(crate) fn remember_snapshot(&self, snapshot: &provider_usage::ProviderUsageSnapshot) {
        if let Ok(mut list) = self.last_snapshots.lock() {
            list.retain(|s| s.provider_id != snapshot.provider_id);
            list.push(snapshot.clone());
        }
    }
}
```

`desktop_shell::WindowMode::slug`를 `pub(crate) fn slug`로 바꾼다.

`provider_usage_snapshot` 커맨드를 교체(`State`는 `.await`를 넘길 수 없으므로 `AppHandle`로 상태 접근):

```rust
#[tauri::command]
async fn provider_usage_snapshot(
    app: AppHandle,
    provider_id: String,
) -> Result<provider_usage::ProviderUsageSnapshot, String> {
    let snapshot = tauri::async_runtime::spawn_blocking(move || provider_usage::snapshot(&provider_id))
        .await
        .map_err(|_| "provider usage worker failed".to_string())?;
    app.state::<AppState>().remember_snapshot(&snapshot);
    Ok(snapshot)
}
```

선호 커맨드 3개 추가(`provider_remove_bridge` 아래):

```rust
fn locked_prefs<'a>(state: &'a State<'_, AppState>) -> Result<std::sync::MutexGuard<'a, standby::UiPrefs>, String> {
    state.ui.lock().map_err(|_| "ui preferences unavailable".to_string())
}

#[tauri::command]
fn get_ui_prefs(state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    locked_prefs(&state).map(|prefs| prefs.clone())
}

#[tauri::command]
fn set_ui_prefs(theme: String, solid: bool, state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    let updated = {
        let mut prefs = locked_prefs(&state)?;
        prefs.theme = standby::normalize_theme(&theme).to_string();
        prefs.solid = solid;
        prefs.clone()
    };
    standby::save_prefs(&updated).map_err(|error| error.to_string())?;
    Ok(updated)
}

#[tauri::command]
fn set_standby(enabled: bool, state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    let updated = {
        let mut prefs = locked_prefs(&state)?;
        prefs.standby = enabled;
        prefs.clone()
    };
    standby::save_prefs(&updated).map_err(|error| error.to_string())?;
    Ok(updated)
}
```

`generate_handler![...]`에 `get_ui_prefs, set_ui_prefs, set_standby` 3개를 추가한다(기존 7개 유지).

- [ ] **Step 3: `lib.rs` — `run()` 교체(닫기 분기·창 생성·종료 방지)**

`run()` 본문 중 `.on_window_event(...)`부터 끝까지를 교체:

```rust
    let app = builder
        .on_window_event(|window, event| {
            if window.label() != desktop_shell::MAIN_WINDOW_LABEL {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let standby = window
                    .app_handle()
                    .state::<AppState>()
                    .ui
                    .lock()
                    .map(|prefs| prefs.standby)
                    .unwrap_or(false);
                if let Some(webview) = window.app_handle().get_webview_window(desktop_shell::MAIN_WINDOW_LABEL) {
                    let _ = desktop_shell::dismiss(&webview, standby);
                }
            }
        })
        .manage(AppState::with_prefs(standby::load_prefs()))
        .invoke_handler(tauri::generate_handler![
            oauth_prepare,
            credential_status,
            credential_remove,
            provider_usage_snapshot,
            provider_start_login,
            provider_install_bridge,
            provider_remove_bridge,
            get_ui_prefs,
            set_ui_prefs,
            set_standby
        ])
        .setup(|app| {
            desktop_shell::install(app)?;
            desktop_shell::show_main_window(app.handle(), desktop_shell::WindowMode::Mini)?;

            #[cfg(feature = "native-oauth")]
            {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                app.deep_link().register_all()?;

                let app_handle = app.handle().clone();
                if let Some(urls) = app.deep_link().get_current()? {
                    process_callback_urls(&app_handle, urls);
                }

                let event_app = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    process_callback_urls(&event_app, event.urls());
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building SPECTRA native shell");

    app.run(|_app, event| {
        if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
```

`code: None`은 "모든 창이 닫혀서" 발생한 종료 요청이고, 트레이 메뉴의 `app.exit(0)`은 `code: Some(0)`이라 그대로 종료된다. `show_main_window`가 setup에서 창을 처음 생성하므로 `tauri.conf.json`의 창 정의를 제거한다.

`CloseRequested` 핸들러 안에서 `destroy()`를 호출했을 때 창이 남거나 패닉이 나면(이벤트 처리 중 재진입), 아래처럼 메인 스레드 큐로 지연 실행한다 — 이 경우 보고서에 기록한다:

```rust
let handle = window.app_handle().clone();
let _ = window.app_handle().run_on_main_thread(move || {
    if let Some(webview) = handle.get_webview_window(desktop_shell::MAIN_WINDOW_LABEL) {
        let _ = webview.destroy();
    }
});
```

- [ ] **Step 4: `tauri.conf.json` — 창 정의 제거**

`"app"` 블록을 교체:

```json
  "app": {
    "withGlobalTauri": true,
    "windows": []
  },
```

- [ ] **Step 5: `desktop_shell.rs` — 재생성·저장 모드·dismiss**

`use tauri::{App, AppHandle, LogicalSize, Manager, Runtime, Size};` → `use tauri::{App, AppHandle, LogicalSize, Manager, Runtime, Size, WebviewWindow};`

`show_main_window`·`hide_main_window`·`toggle_main_window`를 교체:

```rust
fn stored_mode<R: Runtime>(app: &AppHandle<R>) -> WindowMode {
    app.state::<crate::AppState>().window_mode.lock().map(|mode| *mode).unwrap_or_default()
}

fn store_mode<R: Runtime>(app: &AppHandle<R>, mode: WindowMode) {
    if let Ok(mut slot) = app.state::<crate::AppState>().window_mode.lock() {
        *slot = mode;
    }
}

fn standby_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<crate::AppState>().ui.lock().map(|prefs| prefs.standby).unwrap_or(false)
}

pub(crate) fn dismiss<R: Runtime>(window: &WebviewWindow<R>, standby: bool) -> tauri::Result<()> {
    match crate::standby::close_action(standby) {
        crate::standby::CloseAction::Hide => window.hide(),
        crate::standby::CloseAction::Destroy => window.destroy(),
    }
}

pub(crate) fn show_main_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
) -> tauri::Result<()> {
    store_mode(app, mode);
    let started = std::time::Instant::now();
    let window = match app.get_webview_window(MAIN_WINDOW_LABEL) {
        Some(window) => window,
        None => {
            let boot = app.state::<crate::AppState>().boot_state(mode);
            crate::standby::create_main_window(app, mode, &boot)?
        }
    };

    let profile = mode.profile();
    window.set_size(Size::Logical(LogicalSize::new(profile.width, profile.height)))?;
    window.set_always_on_top(profile.always_on_top)?;

    if window.is_minimized()? {
        window.unminimize()?;
    }

    window.center()?;
    window.show()?;
    window.set_focus()?;
    window.eval(&mode_script(mode))?;
    crate::standby::log_timing("show_main_window", started.elapsed());
    Ok(())
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        dismiss(&window, standby_enabled(app))?;
    }
    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return show_main_window(app, stored_mode(app));
    };

    if window.is_visible()? && !window.is_minimized()? {
        dismiss(&window, standby_enabled(app))
    } else {
        show_main_window(app, stored_mode(app))
    }
}
```

`WindowMode::slug`가 `pub(crate)`가 됐는지 확인(Step 2). Task 1에서 임시로 둔 `#![allow(dead_code)]`가 있으면 제거한다.

- [ ] **Step 6: 두 구성 빌드·테스트**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib 2>&1 | grep -E "test result|error|panicked" && cargo build --features native-oauth 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib --features native-oauth 2>&1 | grep -E "test result|error|panicked"; cd ..
```

Expected: 경고 0, 기본 `31 passed`(30 + 1), native-oauth `33 passed`.

- [ ] **Step 7: 네이티브 동작 확인(사용자 협조)**

```bash
npm run desktop:build
```

사용자에게 릴리스 exe 실행을 안내하고 확인 항목: ① 기동 시 미니 창이 뜬다(setup 생성 경로) ② 창 닫기 → 숨김, 트레이 클릭 → 즉시 재표시(기본값 유지) ③ 트레이 "SPECTRA 종료" → 프로세스 종료. 대기 모드 동작은 Task 3의 설정 토글이 생긴 뒤 확인한다. 실행자는 `tasklist | findstr /I spectra-native`로 ③의 종료를 확인한다.

- [ ] **Step 8: 커밋**

```bash
git add src-tauri/src/lib.rs src-tauri/src/desktop_shell.rs src-tauri/tauri.conf.json
git commit -m "feat(phase4): create the main window at setup, destroy on close in standby, and keep the tray alive

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: 프론트 — 부트 상태 복원·선호 동기화·설정 토글

**Files:**
- Create: `src/integrations/boot-state.ts`
- Create: `tests/boot-state.test.ts`
- Modify: `src/integrations/tauri-native-bridge.ts` — 타입·함수 4개
- Modify: `src/App.tsx` — `initialQuotaRecord`, `App()` 초기 상태·효과·설정 토글, `LayoutActions`, 설정 뷰
- Modify: `scripts/verify-memory-budget.mjs` — `sourceFiles`에 두 파일 추가

**Interfaces:**
- Consumes: Task 2 커맨드(`get_ui_prefs`·`set_ui_prefs {theme, solid}`·`set_standby {enabled}`), `window.__SPECTRA_BOOT__`(Task 1 `boot_script`), 기존 `quotaFromSnapshot`
- Produces:
  ```typescript
  // src/integrations/boot-state.ts
  export type NativeUiPrefs = Readonly<{ theme: "dark" | "light"; solid: boolean; standby: boolean }>;
  export type NativeBootState = NativeUiPrefs & Readonly<{ mode: "mini" | "dashboard"; snapshots: readonly NativeProviderUsageSnapshot[] }>;
  export function parseBootState(raw: unknown): NativeBootState | null;   // 형식이 어긋나면 null(부분 복원 없음)
  // src/integrations/tauri-native-bridge.ts
  export function readBootState(): NativeBootState | null;                 // parseBootState(window.__SPECTRA_BOOT__)
  export async function getNativeUiPrefs(): Promise<NativeUiPrefs | null>;
  export async function setNativeUiPrefs(prefs: { theme: "dark" | "light"; solid: boolean }): Promise<NativeUiPrefs | null>;
  export async function setNativeStandby(enabled: boolean): Promise<NativeUiPrefs | null>;
  ```

- [ ] **Step 1: 테스트 작성**

`tests/boot-state.test.ts`:

```typescript
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { parseBootState } from "../src/integrations/boot-state.ts";

const snapshot = {
  providerId: "codex", runtimeAvailable: true, authState: "signed-in", connectionState: "connected",
  authMethod: null, planType: "pro", source: "codex-app-server", lastSyncedAt: 1_700_000_000, bridgeInstalled: false,
  windows: [{ id: "rolling", label: "5시간 한도", usedPercent: 40, remainingPercent: 60, resetsAt: 1_700_003_600, windowDurationMins: 300 }],
  message: ""
};

describe("parseBootState", () => {
  it("accepts a well-formed boot payload", () => {
    const boot = parseBootState({ theme: "light", solid: true, standby: true, mode: "dashboard", snapshots: [snapshot] });
    assert.ok(boot);
    assert.equal(boot.theme, "light");
    assert.equal(boot.mode, "dashboard");
    assert.equal(boot.snapshots.length, 1);
    assert.equal(boot.snapshots[0].providerId, "codex");
  });

  it("rejects unknown theme or mode values", () => {
    assert.equal(parseBootState({ theme: "neon", solid: false, standby: false, mode: "mini", snapshots: [] }), null);
    assert.equal(parseBootState({ theme: "dark", solid: false, standby: false, mode: "phone", snapshots: [] }), null);
  });

  it("drops snapshots for unknown providers but keeps the rest", () => {
    const boot = parseBootState({ theme: "dark", solid: false, standby: false, mode: "mini", snapshots: [snapshot, { ...snapshot, providerId: "gemini" }] });
    assert.ok(boot);
    assert.deepEqual(boot.snapshots.map(s => s.providerId), ["codex"]);
  });

  it("returns null for non-objects and missing fields", () => {
    assert.equal(parseBootState(undefined), null);
    assert.equal(parseBootState("dark"), null);
    assert.equal(parseBootState({ theme: "dark" }), null);
  });
});
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
npm test
```

Expected: FAIL — `boot-state.ts` 없음.

- [ ] **Step 3: `boot-state.ts` 구현**

```typescript
import type { NativeProviderUsageSnapshot } from "./tauri-native-bridge";

export type NativeUiPrefs = Readonly<{ theme: "dark" | "light"; solid: boolean; standby: boolean }>;
export type NativeBootState = NativeUiPrefs & Readonly<{ mode: "mini" | "dashboard"; snapshots: readonly NativeProviderUsageSnapshot[] }>;

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === "object" && value !== null;
const knownProviders = new Set(["codex", "claude"]);

function isSnapshot(value: unknown): value is NativeProviderUsageSnapshot {
  return isRecord(value) && typeof value.providerId === "string" && typeof value.connectionState === "string" && Array.isArray(value.windows);
}

export function parseBootState(raw: unknown): NativeBootState | null {
  if (!isRecord(raw)) return null;
  const { theme, solid, standby, mode, snapshots } = raw;
  if (theme !== "dark" && theme !== "light") return null;
  if (mode !== "mini" && mode !== "dashboard") return null;
  if (typeof solid !== "boolean" || typeof standby !== "boolean" || !Array.isArray(snapshots)) return null;
  return {
    theme,
    solid,
    standby,
    mode,
    snapshots: snapshots.filter(isSnapshot).filter(snapshot => knownProviders.has(snapshot.providerId))
  };
}
```

`tauri-native-bridge.ts`는 타입만 import하므로(`import type`) 순환 import가 런타임에 생기지 않는다.

- [ ] **Step 4: 브리지 함수 추가**

`src/integrations/tauri-native-bridge.ts`에 추가(파일 상단 import에 `import { parseBootState, type NativeBootState, type NativeUiPrefs } from "./boot-state";`, `declare global`의 `Window`에 `__SPECTRA_BOOT__?: unknown;` 추가):

```typescript
export function readBootState(): NativeBootState | null {
  if (typeof window === "undefined") return null;
  return parseBootState(window.__SPECTRA_BOOT__);
}

export async function getNativeUiPrefs(): Promise<NativeUiPrefs | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeUiPrefs>("get_ui_prefs");
}

export async function setNativeUiPrefs(prefs: Readonly<{ theme: "dark" | "light"; solid: boolean }>): Promise<NativeUiPrefs | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeUiPrefs>("set_ui_prefs", { theme: prefs.theme, solid: prefs.solid });
}

export async function setNativeStandby(enabled: boolean): Promise<NativeUiPrefs | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeUiPrefs>("set_standby", { enabled });
}
```

- [ ] **Step 5: `App.tsx` — 부트 상태로 초기화**

임포트 갱신(8행): `getNativeProviderUsage, installNativeProviderBridge, isTauriRuntime, readBootState, removeNativeProviderBridge, setNativeStandby, setNativeUiPrefs, startNativeProviderLogin, type NativeProviderActionResult, type NativeProviderUsageSnapshot`.

`initialQuotaRecord`를 교체:

```typescript
function initialQuotaRecord(snapshots: readonly NativeProviderUsageSnapshot[] = []): QuotaRecord {
  if (!isTauriRuntime()) return planQuotas;
  const restored = new Map(snapshots.map(snapshot => [snapshot.providerId, snapshot] as const));
  const codex = restored.get("codex");
  const claude = restored.get("claude");
  return {
    codex: codex ? quotaFromSnapshot(codex, planQuotas.codex) : nativePendingQuota(planQuotas.codex),
    claude: claude ? quotaFromSnapshot(claude, planQuotas.claude) : nativePendingQuota(planQuotas.claude)
  };
}
```

`App()` 안에서 상태 선언을 수정(`const isMobile = useIsMobile();` 바로 아래에 `const boot = useRef(readBootState()).current;` 추가):

```typescript
  const [solid, setSolid] = useState(boot?.solid ?? false);
  const [theme, setTheme] = useState<ThemeMode>(boot?.theme ?? "dark");
  const [standby, setStandby] = useState(boot?.standby ?? false);
  const [refreshedAt, setRefreshedAt] = useState(boot && boot.snapshots.length > 0 ? "이전 표시 복원" : "방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const [quotas, setQuotas] = useState<QuotaRecord>(() => initialQuotaRecord(boot?.snapshots ?? []));
```

`initialRefreshStarted`는 부트 스냅샷이 있으면 자동 새로고침을 건너뛴다(대기 모드 재표시마다 Codex App Server를 띄우지 않기 위함 — 사용자가 새로고침 버튼으로 갱신):

```typescript
  const initialRefreshStarted = useRef(Boolean(boot && boot.snapshots.length > 0));
```

테마 효과 아래에 선호 동기화 효과를 추가(첫 렌더는 건너뛰어 기동마다 파일을 쓰지 않음):

```typescript
  const prefsSyncArmed = useRef(false);
  useEffect(() => {
    if (!isTauriRuntime()) return;
    if (!prefsSyncArmed.current) {
      prefsSyncArmed.current = true;
      return;
    }
    void setNativeUiPrefs({ theme, solid });
  }, [theme, solid]);

  const toggleStandby = useCallback(async () => {
    const next = !standby;
    setStandby(next);
    if (!isTauriRuntime()) return;
    const saved = await setNativeStandby(next);
    if (saved) setStandby(saved.standby);
  }, [standby]);
```

`LayoutActions` 타입에 `standby: boolean; onStandby: () => void;` 두 줄을 추가하고, `sharedProps`에 `standby, onStandby: () => void toggleStandby(),`를 추가한다. `TopActions`는 spread로 추가 prop을 받지만 사용하지 않으므로 변경하지 않는다(spread 속성은 초과 속성 검사 대상이 아님).

- [ ] **Step 6: 설정 뷰에 대기 모드 행 추가**

`DesktopViewPanel`의 인자 목록에 `standby, onStandby`를 추가하고, 설정 뷰의 "가독성용 불투명 모드" 행 바로 뒤에 추가:

```tsx
<div className="settings-row"><div><strong>메모리 절약 대기</strong><span>{standby ? "창을 닫으면 WebView를 종료하고 트레이만 남깁니다. 다시 열 때 약 0.5초 걸립니다." : "창을 닫으면 숨기기만 해 즉시 다시 표시됩니다(기본)."}</span></div><button type="button" className="secondary-action" onClick={onStandby}>{standby ? "빠른 재표시" : "메모리 절약"}</button></div>
```

- [ ] **Step 7: `verify-memory-budget.mjs` 스캔 대상 추가**

`sourceFiles` 배열에 `"src/integrations/tauri-native-bridge.ts"`, `"src/integrations/boot-state.ts"`를 추가한다(금지 패턴 스캔 확대; 두 파일에 `matchMedia` 등 필수 문자열 검사는 `source` 결합 문자열 기준이라 영향 없음).

- [ ] **Step 8: 게이트 실행**

```bash
npm test && npm run build && npm run verify:tokens && npm run verify:memory && npm run verify:baseline
```

Expected: `npm test` 28/28(24 + 4), 나머지 PASS. `verify:memory`의 `js=` 값이 Phase 3(234,378 B)보다 1~2 KB 늘 수 있다(예산 500,000 B).

- [ ] **Step 9: 네이티브 확인(사용자 협조)**

```bash
npm run desktop:build
```

사용자 확인 항목: ① 설정 → "메모리 절약" 켬 → 창 닫기 → 작업 관리자/`tasklist`에서 `msedgewebview2.exe` 자손이 사라지고 `spectra-native.exe`만 남는지(실행자가 `tasklist | findstr /I "spectra-native msedgewebview2"`로 확인) ② 트레이 클릭 → 창 재생성, 테마(라이트로 바꿔 둔 경우)·창 모드(대시보드로 열어 둔 경우)·수치가 유지되는지, 상단 동기화 라벨이 "이전 표시 복원"인지 ③ 새로고침 → 수치 갱신 ④ 설정 → "빠른 재표시"로 되돌리고 닫기 → 숨김만 되는지 ⑤ 앱 재기동 후 테마·대기 설정이 유지되는지(`ui-prefs.json`).

- [ ] **Step 10: 커밋**

```bash
git add src/integrations/boot-state.ts src/integrations/tauri-native-bridge.ts src/App.tsx tests/boot-state.test.ts scripts/verify-memory-budget.mjs
git commit -m "feat(phase4): restore theme, mode, and snapshots from boot state and add the standby toggle

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: 트레이 배지 — 최소 잔여율 숫자 아이콘

**Files:**
- Create: `src-tauri/src/tray_badge.rs`
- Modify: `src-tauri/src/lib.rs` — `mod tray_badge;`, `provider_usage_snapshot`에서 배지 갱신 호출
- Modify: `src-tauri/src/desktop_shell.rs` — `update_tray_badge`

**Interfaces:**
- Consumes: `AppState::last_snapshots`, `tauri::image::Image`, `Manager::tray_by_id("spectra-main")`
- 스펙 "`build.rs`의 `png` 인코더 재사용"은 런타임에 불가능하다 — `png`는 `[build-dependencies]`에만 있어 앱 바이너리에서 쓸 수 없고, 런타임 의존으로 옮기면 새 의존성 추가가 된다. 대신 RGBA 버퍼를 `Image::new_owned`로 트레이에 직접 전달한다(PNG 인코딩·디코딩 불필요, 스펙 의도인 "숫자를 그려 넣기"는 동일).
- Produces:
  ```rust
  pub const BADGE_SIZE: u32 = 32;
  pub fn min_remaining_percent(snapshots: &[ProviderUsageSnapshot]) -> Option<(String, u8)>; // (provider_id, %) — connected/stale 공급자의 rolling 창 기준
  pub fn badge_text(percent: u8) -> String;          // "24%" (100 이상은 "99%"로 표시)
  pub fn render(percent: u8) -> Vec<u8>;             // 32×32 RGBA
  pub(crate) fn desktop_shell::update_tray_badge<R: Runtime>(app: &AppHandle<R>);
  ```

- [ ] **Step 1: 테스트 작성(`tray_badge.rs` 하단)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

    fn snapshot(provider: &str, state: &str, windows: Vec<(&str, f64)>) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot {
            provider_id: provider.to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: state.to_string(),
            auth_method: None,
            plan_type: None,
            source: Some("codex-app-server".to_string()),
            last_synced_at: None,
            bridge_installed: false,
            windows: windows.into_iter().map(|(id, remaining)| ProviderQuotaWindow {
                id: id.to_string(), label: id.to_string(), used_percent: 100.0 - remaining, remaining_percent: remaining, resets_at: None, window_duration_mins: None,
            }).collect(),
            message: String::new(),
        }
    }

    #[test]
    fn every_badge_character_has_a_glyph() {
        for ch in "0123456789%".chars() {
            assert!(glyph(ch).is_some(), "missing glyph for {ch}");
        }
        assert!(glyph('x').is_none());
    }

    #[test]
    fn render_produces_a_full_rgba_buffer_with_visible_text() {
        let pixels = render(10); // 10% → TEXT_LOW 색으로 그려진다
        assert_eq!(pixels.len(), (BADGE_SIZE * BADGE_SIZE * 4) as usize);
        let text_pixels = pixels.chunks(4).filter(|px| px[3] == 255 && px[0] == TEXT_LOW[0] && px[1] == TEXT_LOW[1] && px[2] == TEXT_LOW[2]).count();
        assert!(text_pixels > 20, "expected glyph pixels, got {text_pixels}");
    }

    #[test]
    fn badge_text_clamps_to_two_digits() {
        assert_eq!(badge_text(7), "7%");
        assert_eq!(badge_text(24), "24%");
        assert_eq!(badge_text(100), "99%");
    }

    #[test]
    fn min_remaining_prefers_rolling_window_of_connected_providers() {
        let snapshots = vec![
            snapshot("codex", "connected", vec![("weekly", 80.0), ("rolling", 24.4)]),
            snapshot("claude", "stale", vec![("rolling", 32.0)]),
            snapshot("gemini", "signed-out", vec![("rolling", 1.0)]),
        ];
        assert_eq!(min_remaining_percent(&snapshots), Some(("codex".to_string(), 24)));
    }

    #[test]
    fn min_remaining_is_none_without_usable_windows() {
        assert_eq!(min_remaining_percent(&[]), None);
        assert_eq!(min_remaining_percent(&[snapshot("codex", "not-installed", vec![])]), None);
    }

    #[test]
    fn text_color_follows_thresholds() {
        assert_eq!(text_color(60), TEXT_HIGH);
        assert_eq!(text_color(30), TEXT_MID);
        assert_eq!(text_color(10), TEXT_LOW);
    }
}
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
cd src-tauri && cargo test --lib tray_badge:: 2>&1 | tail -6; cd ..
```

Expected: 컴파일 실패(모듈 없음). `lib.rs`에 `mod tray_badge;`를 먼저 추가한다.

- [ ] **Step 3: `tray_badge.rs` 구현**

```rust
use crate::provider_usage::ProviderUsageSnapshot;

pub const BADGE_SIZE: u32 = 32;
const SCALE: u32 = 2;
const GLYPH_W: u32 = 3;
const GLYPH_H: u32 = 5;
const GAP: u32 = 1;
const BACKGROUND: [u8; 4] = [16, 17, 15, 255];
const BORDER: [u8; 4] = [69, 74, 103, 255];
pub const TEXT_HIGH: [u8; 3] = [86, 183, 176];  // --color-cyan
pub const TEXT_MID: [u8; 3] = [199, 168, 82];   // --color-amber
pub const TEXT_LOW: [u8; 3] = [216, 132, 99];   // --color-coral

fn glyph(ch: char) -> Option<[u8; 5]> {
    Some(match ch {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        _ => return None,
    })
}

pub fn badge_text(percent: u8) -> String {
    format!("{}%", percent.min(99))
}

fn text_color(percent: u8) -> [u8; 3] {
    if percent >= 50 { TEXT_HIGH } else if percent >= 20 { TEXT_MID } else { TEXT_LOW }
}

fn usable(snapshot: &ProviderUsageSnapshot) -> bool {
    matches!(snapshot.connection_state.as_str(), "connected" | "stale") && !snapshot.windows.is_empty()
}

pub fn min_remaining_percent(snapshots: &[ProviderUsageSnapshot]) -> Option<(String, u8)> {
    snapshots
        .iter()
        .filter(|snapshot| usable(snapshot))
        .filter_map(|snapshot| {
            let window = snapshot.windows.iter().find(|w| w.id == "rolling").or_else(|| snapshot.windows.first())?;
            let percent = window.remaining_percent.round().clamp(0.0, 100.0) as u8;
            Some((snapshot.provider_id.clone(), percent))
        })
        .min_by_key(|(_, percent)| *percent)
}

fn put(pixels: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    if x >= BADGE_SIZE || y >= BADGE_SIZE {
        return;
    }
    let offset = ((y * BADGE_SIZE + x) * 4) as usize;
    pixels[offset..offset + 4].copy_from_slice(&rgba);
}

pub fn render(percent: u8) -> Vec<u8> {
    let mut pixels = vec![0_u8; (BADGE_SIZE * BADGE_SIZE * 4) as usize];
    for y in 0..BADGE_SIZE {
        for x in 0..BADGE_SIZE {
            let edge = x < 2 || y < 2 || x >= BADGE_SIZE - 2 || y >= BADGE_SIZE - 2;
            let corner = (x < 3 || x >= BADGE_SIZE - 3) && (y < 3 || y >= BADGE_SIZE - 3);
            if corner {
                continue;
            }
            put(&mut pixels, x, y, if edge { BORDER } else { BACKGROUND });
        }
    }

    let text = badge_text(percent);
    let glyph_count = text.chars().count() as u32;
    let text_width = glyph_count * GLYPH_W * SCALE + (glyph_count - 1) * GAP * SCALE;
    let origin_x = (BADGE_SIZE - text_width) / 2;
    let origin_y = (BADGE_SIZE - GLYPH_H * SCALE) / 2;
    let color = text_color(percent);
    let rgba = [color[0], color[1], color[2], 255];

    for (index, ch) in text.chars().enumerate() {
        let Some(rows) = glyph(ch) else { continue };
        let glyph_x = origin_x + index as u32 * (GLYPH_W + GAP) * SCALE;
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits & (0b100 >> col) == 0 {
                    continue;
                }
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        put(&mut pixels, glyph_x + col * SCALE + dx, origin_y + row as u32 * SCALE + dy, rgba);
                    }
                }
            }
        }
    }
    pixels
}
```

- [ ] **Step 4: `desktop_shell::update_tray_badge` + 호출 지점**

`desktop_shell.rs` import에 `use tauri::image::Image;` 추가, `install` 아래에 추가:

```rust
const TRAY_ID: &str = "spectra-main";
const TRAY_TOOLTIP: &str = "SPECTRA · 개인 요금제 사용 현황";

pub(crate) fn update_tray_badge<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let snapshots = app
        .state::<crate::AppState>()
        .last_snapshots
        .lock()
        .map(|list| list.clone())
        .unwrap_or_default();
    match crate::tray_badge::min_remaining_percent(&snapshots) {
        Some((provider_id, percent)) => {
            let image = Image::new_owned(crate::tray_badge::render(percent), crate::tray_badge::BADGE_SIZE, crate::tray_badge::BADGE_SIZE);
            let _ = tray.set_icon(Some(image));
            let _ = tray.set_tooltip(Some(format!("{TRAY_TOOLTIP} · 최소 잔여 {percent}% ({provider_id})")));
        }
        None => {
            let _ = tray.set_icon(app.default_window_icon().cloned());
            let _ = tray.set_tooltip(Some(TRAY_TOOLTIP));
        }
    }
}
```

`install`의 `TrayIconBuilder::with_id("spectra-main")`과 `.tooltip("SPECTRA · 개인 요금제 사용 현황")`을 상수 `TRAY_ID`·`TRAY_TOOLTIP`으로 바꾼다. `lib.rs`의 `provider_usage_snapshot`에서 `app.state::<AppState>().remember_snapshot(&snapshot);` 다음 줄에 `desktop_shell::update_tray_badge(&app);`를 추가한다(시작 시 1회 + 사용자 새로고침 시에만 호출되는 경로 — 폴링 없음).

- [ ] **Step 5: 두 구성 빌드·테스트**

```bash
cd src-tauri && cargo build 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib 2>&1 | grep -E "test result|error|panicked" && cargo build --features native-oauth 2>&1 | grep -E "^(warning|error)|Finished" && cargo test --lib --features native-oauth 2>&1 | grep -E "test result|error|panicked"; cd ..
```

Expected: 경고 0, 기본 `37 passed`(31 + 6), native-oauth `39 passed`.

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/tray_badge.rs src-tauri/src/desktop_shell.rs src-tauri/src/lib.rs
git commit -m "feat(phase4): draw the minimum remaining percentage on the tray icon

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: 측정 · 문서 · 마무리

**Files:**
- Modify: `docs/performance/memory-budget.md` — "## 2026-09-09 Phase 4 결과 (선택형 저메모리 대기 모드)" 절
- Modify: `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — §4 Phase 4 절에 `- 검증 확정(날짜)` 불릿, 상태 줄

**Interfaces:**
- Consumes: Task 1~4의 최종 릴리스 빌드(`npm run desktop:build`, 기본 feature), `scripts/measure-memory.ps1`, `SPECTRA_TIMING_LOG`
- Produces: 성공 기준 판정 — 대기 모드 30분 작업 집합 ≤ 60 MB(PASS/FAIL), 재표시 후 테마·모드 유지(사용자 확인), 기본값 현행(사용자 확인), 재표시 지연 중앙값(ms)

- [ ] **Step 1: 최종 빌드·해시 고정**

```bash
npm run desktop:build 2>&1 | grep -E "Finished|error" && ls -l src-tauri/target/release/spectra-native.exe src-tauri/target/release/bundle/nsis/SPECTRA_0.2.1_x64-setup.exe | awk '{print $5, $9}'
```

`Get-FileHash`로 두 파일의 SHA-256을 기록한다. Phase 3(5,809,664 B) 대비 증가분(신규 모듈 2개)을 기록한다.

- [ ] **Step 2: 기본값 회귀 확인(사용자)**

새 `ui-prefs.json`이 없는 상태(또는 `standby: false`)에서 사용자가 확인: 닫기 → 숨김, 트레이 클릭 → 즉시 재표시. 트레이 아이콘에 최소 잔여율 숫자가 보이는지, 툴팁에 "최소 잔여 N% (provider)"가 보이는지도 함께 확인.

- [ ] **Step 3: 대기 모드 30분 측정(사용자 협조)**

사용자가 설정에서 "메모리 절약"을 켠 뒤 창을 닫는다(트레이만 남음). 실행자가 호스트 PID로 측정을 시작한다:

```bash
tasklist | findstr /I "spectra-native msedgewebview2"
```

Expected: `spectra-native.exe` 1개, `msedgewebview2.exe` 중 SPECTRA 자손 없음(다른 앱의 WebView2는 남을 수 있음 — 측정 스크립트는 호스트 자손만 합산).

```powershell
npm run measure:memory -- -Scenario standby-idle-phase4 -RootPid <호스트PID> -Samples 61 -IntervalSeconds 30
```

Expected: 61표본, `processCount` 1, 평균 작업 집합 ≤ 60 MB(호스트 단독 ≈ 30 MB 예상). 60 MB 초과면 자손 프로세스 잔존 여부를 CSV `processCount`로 확인하고 원인을 기록한다(예: WebView2 브라우저 프로세스가 destroy 후에도 남는 경우 → `WebviewWindow::destroy` 뒤 남는 프로세스 이름을 보고).

- [ ] **Step 4: 재표시 지연 측정**

앱을 종료하고 `SPECTRA_TIMING_LOG=1` 환경변수로 재실행한다(PowerShell):

```powershell
$env:SPECTRA_TIMING_LOG = "1"; Start-Process -FilePath "src-tauri\target\release\spectra-native.exe"
```

대기 모드가 켜진 상태에서 사용자가 "창 닫기 → 트레이 클릭"을 5회 반복한 뒤, 실행자가 `%LOCALAPPDATA%\SPECTRA\standby-timing.log`의 `show_main_window` 행 5개(창 생성 포함 경로)의 중앙값을 계산해 기록한다(스펙 예상 300~600 ms). 측정 후 환경변수 없이 정상 재실행하도록 안내한다.

- [ ] **Step 5: 문서화**

`docs/performance/memory-budget.md`에 절 추가:

```markdown
## 2026-09-09 Phase 4 결과 (선택형 저메모리 대기 모드)

변경: 설정 "메모리 절약 대기"(기본 off). 켜면 창 닫기 시 WebView 창을 `destroy`하고 트레이만 남기며, 트레이 클릭 시 `WebviewWindowBuilder`로 재생성해 테마·창 모드·마지막 스냅샷을 `initialization_script`(`window.__SPECTRA_BOOT__`)로 복원한다. 선호는 Rust가 `<앱 데이터 폴더>/ui-prefs.json`에 저장(`localStorage` 미사용). 트레이 아이콘에 연결된 공급자의 최소 잔여율을 표시(시작·수동 새로고침 시에만 갱신).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | 목표(≤ 60 MB) |
|---|---|---|---|---|---|---|
| standby-idle-phase4 (30분, 창 파기 상태) | 61 | … MB | … MB | … MB | … | PASS/FAIL |

재표시 지연(`SPECTRA_TIMING_LOG=1`, `show_main_window` 5회 중앙값): … ms (스펙 예상 300~600 ms).
기본값 회귀: 닫기=숨김·즉시 재표시 유지(사용자 확인 …). 재표시 후 테마·모드 유지(사용자 확인 …).
실행 파일: 5,809,664 B → … B (신규 모듈 2개).
```

스펙 §4 Phase 4 절 끝에 `- 검증 확정(2026-09-09): …` 불릿으로 위 수치·판정을 한 줄로 기록하고, 상태 줄을 `· Phase 4 완료(2026-09-09, standby 30분 평균 … MB · 재표시 … ms · 기본값 현행) · 전체 스펙 목표 {PASS|PARTIAL}`로 갱신한다(대기 모드 목표 PASS이고 메모리 목표(≤ 380 MB)는 대기 모드로만 달성되면 "PARTIAL(활성 표시 상태 380 MB 미달, 대기 상태 PASS)"로 정직하게 표기).

- [ ] **Step 6: 커밋·푸시(푸시 전 사용자 확인)**

```bash
git add docs/performance/memory-budget.md docs/superpowers/specs/2026-09-04-design-footprint-improvement.md
git commit -m "docs(phase4): record standby measurement, reshow latency, and close the phase

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git log --oneline origin/main..HEAD
```

커밋 목록과 대상(`origin main`)을 제시하고 확인을 받은 뒤 `git push origin main`(`--all` 금지).
