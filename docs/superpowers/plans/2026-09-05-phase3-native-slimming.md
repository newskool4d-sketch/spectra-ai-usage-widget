# Phase 3 — 네이티브 슬림화 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 미사용 OAuth 경로를 cargo feature 뒤로 옮겨 기본 빌드에서 `keyring`·`uuid`·`sha2`·`base64` 직접 의존을 제거한다. WebView2 렌더러 제한은 전후 측정으로 효과를 확인한 경우에만 채택한다. 바이너리 크기와 상주 메모리 개선은 별도로 판정한다.

**Architecture:** `Cargo.toml`에 기본 off인 `native-oauth` feature를 추가하고 OAuth 모듈·커맨드 구현·런타임 딥링크 등록/콜백을 cfg로 감싼다. 기본 빌드에서도 OAuth 커맨드 3개를 스텁으로 등록하고, 어댑터는 독립 순수 모듈의 메시지 매핑을 사용한다. WebView2 렌더러 제한은 미적용/적용 각 2회 비교 후 채택하거나 원복한다. thin/fat LTO는 각 2회의 새 target 빌드로 비교한다. 백엔드 사용량 조회(`provider_usage.rs`, `reqwest`)는 유지하며, 프론트 변경은 오류 매핑과 그에 따른 번들 변화에 한정한다.

**Tech Stack:** Rust 1.77.2+ · Tauri 2.11 · cargo features · WebView2 · TypeScript(어댑터 + 순수 메시지 모듈) · Node 24 `node:test`

**Spec:** `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — "Phase 3 — 네이티브 슬림화" 절(§4) 및 §1 성공 기준 표의 용량 항목

## Global Constraints

- 새 의존성 추가 금지. `uuid`를 `[dev-dependencies]`에 추가하는 것은 이미 쓰는 크레이트를 테스트 전용으로 옮기는 것이므로 허용(신규 크레이트 아님)
- `reqwest`는 `provider_usage.rs`의 Claude 실시간 조회(`fetch_claude_usage_live`)에 사용되므로 게이트·제거 대상이 아님
- `withGlobalTauri: true`, 딥링크 스킴 `spectra`, `tauri-plugin-deep-link`·`tauri-plugin-single-instance` 유지(스펙 §4 Phase 3)
- 프론트 브리지 계약 유지: `oauth_prepare`·`credential_status`·`credential_remove` 커맨드는 feature 유무와 무관하게 항상 등록(프론트가 "command not found" 예외를 만나지 않도록)
- Rust 변경 커밋 전 두 구성의 release 빌드·lib 테스트 통과. feature on 빌드가 같은 출력 경로를 덮어쓰므로, 최종 QA·측정 직전에는 반드시 기본 feature의 `npm run desktop:build`로 배포 산출물을 다시 만든다.
- 프론트 게이트(변경 시): `npm run build`·`npm run verify:tokens`·`npm run verify:memory`·`npm run verify:baseline`·`npm test`(현재 21/21)
- 성공 기준: 기본 feature의 정상 종료한 `cargo tree -e normal --depth 1`에 네 직접 의존 없음, 전체 normal 트리에 `keyring` 없음. 다른 세 크레이트는 전이 의존으로 남을 수 있으므로 실패로 보지 않는다. 실제 경로는 Task 1에서 확인하고 실행 파일 크기는 별도로 측정한다.
- 기준 수치(2026-09-05 19:11 Phase 2 릴리스 빌드): 실행 파일 7,302,144 B · NSIS 2,936,525 B · 프로세스 7개(호스트 1 + WebView2 6) · mini-idle 30분 평균 작업 집합 410.8 MB
- Rust 수동 테스트 시 `--claude-statusline-bridge`는 프로덕션 캐시(`%LOCALAPPDATA%\SPECTRA\claude-usage.json`)에 기록하므로 픽스처를 쓸 땐 `SPECTRA_DATA_DIR`로 격리
- dev server·watch·30분 측정은 명령을 사용자에게 제시하고 결과를 받아 진행한다. 릴리스 빌드·짧은 프로세스 비교·CLI 스냅샷은 실행 가능. Task 6도 이 경계를 따른다.
- 파일 삭제 금지(빌드 산출물 제외) — 게이트되는 3개 모듈은 삭제하지 않고 `cfg`로만 제외
- 커밋 메시지 말미: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`

### 실행·판정 공통 규칙

- 아래 실행 명령은 저장소 루트의 PowerShell 기준이다. Rust 명령에는 `--manifest-path src-tauri/Cargo.toml`을 지정한다. 빌드·테스트·QA 출력을 `grep`·`head`·`tail`로 잘라 성공을 판정하지 않는다.
- 외부 명령 직후 `$LASTEXITCODE`를 확인한다. 0이 아니면 후속 단계·커밋을 중단한다. 의도된 red 테스트만 실패 원인을 확인하고 다음 구현 단계로 진행한다.
- 시작 시 `git status --short`와 변경 대상 diff를 확인해 기존 사용자 변경을 구분한다. 되돌림은 이번 실험에서 바꾼 설정값에 한정한다. 전체 저장소가 깨끗해야 한다는 조건을 두지 않는다.
- 실제 구현 전 기존 릴리스의 해시·크기·공급자별 연결 QA 요약을 확보한다. 과거 Phase 2 수치는 참고 기준이며, 파일의 해시·빌드 조건을 확인하지 못하면 현재 A/B 기준으로 간주하지 않는다. 개인 데이터 원문은 커밋하지 않는다.
- 테스트 개수는 참고값이다. 종료 코드 0, 실패·예상 밖 skip 없음, 의도한 테스트 이름이 실행됐다는 근거가 필수다. 이번 작업에서 생긴 Rust 경고는 해결하고 기존 경고가 있으면 구분해 기록한다.
- 완료 판정은 Task 6의 표를 따른다. `cargo tree`는 의존 그래프 증거이며 최종 바이너리의 링크 내용을 직접 증명하지 않는다. 파일 크기 감소를 메모리 감소로 해석하지 않는다.

## File Structure

| 파일 | 역할 | 작업 |
|---|---|---|
| `src-tauri/Cargo.toml` | `native-oauth` feature, optional 의존, dev-dependency `uuid` | 수정 |
| `src-tauri/src/lib.rs` | 모듈·커맨드·딥링크 콜백 cfg 게이트, 스텁 커맨드·동작 테스트 | 수정 |
| `src-tauri/src/credential_vault.rs` · `oauth_callback.rs` · `provider_connection.rs` | 변경 없음(lib.rs의 `#[cfg]` mod 선언으로 제외) | 유지 |
| `src-tauri/src/provider_usage.rs` | 테스트 헬퍼의 `uuid` 사용은 dev-dependency로 충족 — 변경 없음 | 유지 |
| `src-tauri/tauri.conf.json` | 메인 창 `additionalBrowserArgs` | 수정 |
| `src/integrations/oauth-adapter.ts` | 순수 메시지 함수를 import해 catch에서 사용 | 수정 |
| `src/integrations/oauth-messages.ts` | 런타임 import 없는 오류 메시지 순수 함수 | 신규 |
| `tests/oauth-adapter.test.ts` | 매핑 함수 테스트 | 신규 |
| `docs/performance/memory-budget.md` | Phase 3 결과(크기·`cargo tree`·프로세스 수·LTO 비교·30분 측정) | 수정 |
| `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` | Phase 3 성공 기준 정정 주석, 상태 줄 | 수정 |

---

### Task 1: `native-oauth` feature 게이트 (Cargo + lib.rs)

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/lib.rs` (1-5행 `mod`, 7-20행 `use`·`AppState`, 30-154행 OAuth 함수·커맨드, 210-227행 `setup`)

**Interfaces:**
- Consumes: 기존 모듈 공개 API — `oauth_callback::{PendingOAuth, OAuthPrepareResult, OAuthCallbackRejected, CALLBACK_PATH, prepare, take_exchange, exchange_and_store}`, `credential_vault::{exists, remove, VaultError}`
- Produces: feature off 빌드에서 동일 이름의 스텁 커맨드 — `oauth_prepare` → `Err("native-oauth-disabled")`, `credential_status` → `Ok(CredentialStatus { available: false, present: false })`, `credential_remove` → `Ok(())`. Task 2의 프론트가 `"native-oauth-disabled"` 문자열을 소비한다.

- [ ] **Step 1: 현재 의존 트리 기록(전/후 비교용)**

```powershell
New-Item -ItemType Directory -Force -Path 'docs/performance/measurements' | Out-Null
cargo tree --manifest-path src-tauri/Cargo.toml --offline -e normal --depth 1 > docs/performance/measurements/cargo-tree-depth1-before.txt
if ($LASTEXITCODE -ne 0) { throw 'before dependency tree failed' }
cargo tree --manifest-path src-tauri/Cargo.toml --offline -e normal -i keyring
if ($LASTEXITCODE -ne 0) { throw 'before keyring tree failed' }
```

Expected: depth-1 목록에 `base64`·`keyring`·`sha2`·`uuid`가 보인다(현재 상태). `docs/performance/measurements/`는 `.gitignore` 대상이므로 커밋되지 않는다 — 수치는 Task 5에서 문서에 옮긴다.

- [ ] **Step 2: Rust 테스트 작성(스텁 동작)**

`src-tauri/src/lib.rs` 파일 끝에 추가:

```rust
#[cfg(all(test, not(feature = "native-oauth")))]
mod tests {
    use super::*;

    #[test]
    fn disabled_oauth_prepare_reports_marker() {
        assert_eq!(oauth_prepare("codex".to_string()).unwrap_err(), "native-oauth-disabled");
    }

    #[cfg(not(feature = "native-oauth"))]
    #[test]
    fn disabled_credential_status_reports_unavailable() {
        let status = credential_status("codex".to_string()).expect("stub never fails");
        assert_eq!(status.provider_id, "codex");
        assert!(!status.available);
        assert!(!status.present);
    }

    #[cfg(not(feature = "native-oauth"))]
    #[test]
    fn disabled_credential_remove_is_a_noop() {
        assert!(credential_remove("claude".to_string()).is_ok());
    }
}
```

- [ ] **Step 3: 테스트 실행 — 실패 확인**

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --lib -- tests::
if ($LASTEXITCODE -eq 0) { throw 'expected red test failure; inspect test selection' }
```

Expected: 기존 `oauth_prepare`는 AppHandle·State를 요구하므로 새 스텁 테스트 호출에서 컴파일 실패. 다른 원인(의존 다운로드·환경 오류)이면 red 테스트 성공으로 간주하지 않는다. feature 선언 전의 cfg 경고는 Step 4 이후 해소돼야 한다.

- [ ] **Step 4: Cargo.toml — feature와 optional 의존**

`[dependencies]`의 네 줄을 교체하고 `[features]`·`[dev-dependencies]`를 추가한다:

```toml
[features]
default = []
native-oauth = ["dep:keyring", "dep:uuid", "dep:sha2", "dep:base64"]

[dependencies]
base64 = { version = "0.22", optional = true }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
sha2 = { version = "0.10", optional = true }
tauri = { version = "2.11.5", features = ["tray-icon"] }
tauri-plugin-deep-link = "2.4.9"
reqwest = { version = "0.12.28", default-features = false, features = ["json", "rustls-tls"] }
url = "2"
uuid = { version = "1", features = ["v4"], optional = true }

[dev-dependencies]
uuid = { version = "1", features = ["v4"] }

[target.'cfg(any(target_os = "windows", target_os = "macos"))'.dependencies]
keyring = { version = "4.1.6", default-features = true, optional = true }
```

`[target.'cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))'.dependencies]`의 `tauri-plugin-single-instance`와 `[profile.release]`는 그대로 둔다. `uuid`가 `[dependencies]`(optional)와 `[dev-dependencies]`(항상)에 동시에 있는 것은 의도된 구성 — `provider_usage.rs`의 테스트 헬퍼 `temp_dir()`가 feature 없이도 `uuid::Uuid::new_v4()`를 쓸 수 있게 한다.

- [ ] **Step 5: lib.rs — 모듈·상태·함수 게이트와 스텁**

1-20행을 교체:

```rust
#[cfg(feature = "native-oauth")]
mod credential_vault;
mod desktop_shell;
#[cfg(feature = "native-oauth")]
mod oauth_callback;
#[cfg(feature = "native-oauth")]
mod provider_connection;
mod provider_usage;

#[cfg(feature = "native-oauth")]
use std::io::{Read, Write};
#[cfg(feature = "native-oauth")]
use std::net::TcpListener;
#[cfg(feature = "native-oauth")]
use std::sync::Mutex;

use serde::Serialize;
#[cfg(feature = "native-oauth")]
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
#[cfg(feature = "native-oauth")]
use tauri_plugin_deep_link::DeepLinkExt;
#[cfg(feature = "native-oauth")]
use url::Url;

#[derive(Default)]
pub struct AppState {
    #[cfg(feature = "native-oauth")]
    pending_oauth: Mutex<Option<oauth_callback::PendingOAuth>>,
    #[cfg(feature = "native-oauth")]
    loopback_listener: Mutex<Option<TcpListener>>,
}
```

`CredentialStatus` 구조체(22-28행)는 그대로 둔다. `process_callback_urls`(30행)와 `spawn_loopback_listener`(68행) 두 함수 정의 바로 위에 각각 `#[cfg(feature = "native-oauth")]`를 붙인다.

`oauth_prepare`·`credential_status`·`credential_remove`(113-154행)를 다음으로 교체 — 실제 구현은 feature on, 스텁은 feature off:

```rust
#[cfg(feature = "native-oauth")]
#[tauri::command]
fn oauth_prepare(
    app: AppHandle,
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<oauth_callback::OAuthPrepareResult, String> {
    let result =
        oauth_callback::prepare(&state.pending_oauth, &state.loopback_listener, provider_id)?;
    if result.callback_mode == "loopback" {
        let listener = state
            .loopback_listener
            .lock()
            .map_err(|_| "oauth loopback state unavailable".to_string())?
            .take();
        if let Some(listener) = listener {
            spawn_loopback_listener(app, listener);
        }
    }
    Ok(result)
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn oauth_prepare(provider_id: String) -> Result<serde_json::Value, String> {
    let _ = provider_id;
    Err("native-oauth-disabled".to_string())
}

#[cfg(feature = "native-oauth")]
#[tauri::command]
fn credential_status(provider_id: String) -> Result<CredentialStatus, String> {
    match credential_vault::exists(&provider_id) {
        Ok(present) => Ok(CredentialStatus {
            provider_id,
            available: true,
            present,
        }),
        Err(credential_vault::VaultError::UnsupportedPlatform) => Ok(CredentialStatus {
            provider_id,
            available: false,
            present: false,
        }),
        Err(_) => Err("credential vault unavailable".to_string()),
    }
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn credential_status(provider_id: String) -> Result<CredentialStatus, String> {
    Ok(CredentialStatus {
        provider_id,
        available: false,
        present: false,
    })
}

#[cfg(feature = "native-oauth")]
#[tauri::command]
fn credential_remove(provider_id: String) -> Result<(), String> {
    credential_vault::remove(&provider_id).map_err(|_| "credential removal failed".to_string())
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn credential_remove(provider_id: String) -> Result<(), String> {
    let _ = provider_id;
    Ok(())
}
```

`setup` 클로저(210-227행)를 교체 — 딥링크 플러그인(`.plugin(tauri_plugin_deep_link::init())`)과 `tauri.conf.json`의 `spectra` 스킴 설정은 그대로 두고, 런타임 스킴 등록·콜백 소비는 OAuth 없이는 쓸 곳이 없으므로 함께 게이트한다(`Manager`·`DeepLinkExt` import도 위에서 게이트했으므로 feature off 빌드에 미사용 경고가 남지 않는다):

```rust
        .setup(|app| {
            desktop_shell::install(app)?;

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
```

`invoke_handler`의 `generate_handler![...]` 7개 목록은 변경하지 않는다(스텁이 같은 이름으로 존재).

- [ ] **Step 6: 두 구성 빌드·테스트**

```powershell
cargo build --manifest-path src-tauri/Cargo.toml --release --no-default-features
if ($LASTEXITCODE -ne 0) { throw 'default release build failed' }
cargo test --manifest-path src-tauri/Cargo.toml --lib --no-default-features
if ($LASTEXITCODE -ne 0) { throw 'default tests failed' }
cargo build --manifest-path src-tauri/Cargo.toml --release --no-default-features --features native-oauth
if ($LASTEXITCODE -ne 0) { throw 'native-oauth release build failed' }
cargo test --manifest-path src-tauri/Cargo.toml --lib --no-default-features --features native-oauth
if ($LASTEXITCODE -ne 0) { throw 'native-oauth tests failed' }
```

Expected: 네 명령 모두 종료 코드 0, 신규 경고 0건. 기존 26개 기준 기본 feature는 24개(26 − OAuth 모듈 테스트 5 + 스텁 테스트 3), feature on은 기존 26개다. 기본 구성에서는 `disabled_oauth_prepare_reports_marker`·status·remove 테스트 3개, feature on에서는 기존 OAuth 모듈 테스트 5개가 실제 실행됐는지 확인한다. cfg 자체를 반복 확인하는 함수·테스트는 추가하지 않는다. 경고가 나면 실제 사용처를 확인해 import 범위 등을 수정하며, 무조건 feature 게이트를 더해 숨기지 않는다.

- [ ] **Step 7: 의존 트리 확인(성공 기준)**

```powershell
cargo tree --manifest-path src-tauri/Cargo.toml --offline --no-default-features -e normal --depth 1
if ($LASTEXITCODE -ne 0) { throw 'default direct dependency tree failed' }
cargo tree --manifest-path src-tauri/Cargo.toml --offline --no-default-features -e normal
if ($LASTEXITCODE -ne 0) { throw 'default full dependency tree failed' }
cargo tree --manifest-path src-tauri/Cargo.toml --offline --no-default-features -e normal --depth 1 --features native-oauth
if ($LASTEXITCODE -ne 0) { throw 'native-oauth dependency tree failed' }
```

Expected: 기본 depth-1 목록에 네 직접 의존 없음, 기본 전체 normal 트리에 `keyring` 패키지 없음, feature on의 depth-1에는 네 크레이트 모두 표시. `cargo tree ... -i keyring`은 보조 진단으로만 사용한다. 비활성 패키지 조회의 경고·종료 코드 차이를 성공 기준으로 삼지 않고, 정상 종료한 전체 트리에서 부재를 확인한다.

- [ ] **Step 8: 커밋**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/lib.rs
git commit -m "feat(phase3): gate native OAuth path behind the native-oauth cargo feature

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

`Cargo.lock`이 실제 갱신됐을 때만 의도한 의존 그래프 변경인지 diff 내용을 확인해 함께 커밋한다. optional 전환만으로 lockfile이 반드시 바뀐다고 가정하지 않는다.

---

### Task 2: 프론트 "준비 중" 상태 매핑

**Files:**
- Modify: `src/integrations/oauth-adapter.ts` — `createStart`의 `catch` 분기
- Create: `src/integrations/oauth-messages.ts` — 런타임 import 없는 순수 모듈
- Create: `tests/oauth-adapter.test.ts`

**Interfaces:**
- Consumes: Task 1 스텁의 오류 문자열 `"native-oauth-disabled"` (Tauri invoke 실패 시 `catch (error)`의 `error`는 문자열 그대로 전달됨)
- Produces: `describePrepareFailure(error: unknown)` — `OAuthStartResult`에 대입 가능한 `{ status: "not-available", message: string }` 반환. disabled marker이면 준비 중 메시지, 그 외에는 기존 일반 오류 메시지 유지.

- [ ] **Step 1: 테스트 작성**

`tests/oauth-adapter.test.ts`:

```typescript
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { describePrepareFailure } from "../src/integrations/oauth-messages.ts";

describe("describePrepareFailure", () => {
  it("maps the disabled-feature stub to a preparing message", () => {
    const result = describePrepareFailure("native-oauth-disabled");
    assert.equal(result.status, "not-available");
    assert.equal(result.message, "네이티브 OAuth 연결은 준비 중입니다. 지금은 Codex App Server와 Claude Code 공식 도구로 사용량을 확인합니다.");
  });

  it("recognises the stub code inside an Error object", () => {
    const result = describePrepareFailure(new Error("invoke failed: native-oauth-disabled"));
    assert.ok(result.message.startsWith("네이티브 OAuth 연결은 준비 중입니다."));
  });

  it("keeps the generic failure message for other errors", () => {
    const result = describePrepareFailure(new Error("loopback bind failed"));
    assert.equal(result.status, "not-available");
    assert.equal(result.message, "이 기기에서 네이티브 OAuth 준비를 완료하지 못했습니다.");
  });
});
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
npm test
```

Expected: 종료 코드가 0이 아니고 원인이 신규 `oauth-messages.ts` 부재다. 기존 어댑터를 Node에서 직접 import하면 확장자 없는 `provider-capabilities` 경로에서 `ERR_MODULE_NOT_FOUND`가 발생하는 것을 계획 검토 때 재현했다. 따라서 처음부터 순수 모듈을 테스트하며, Vite의 모듈 해석이나 `window`에 의존하지 않는다.

- [ ] **Step 3: 구현**

`src/integrations/oauth-messages.ts`를 생성한다. 반환 타입은 구조적으로 `OAuthStartResult`에 대입 가능하며 어댑터를 역으로 import하지 않는다:

```typescript
const disabledMarker = "native-oauth-disabled";

export function describePrepareFailure(error: unknown): Readonly<{ status: "not-available"; message: string }> {
  const text = error instanceof Error ? error.message : String(error);
  if (text.includes(disabledMarker)) {
    return {
      status: "not-available",
      message: "네이티브 OAuth 연결은 준비 중입니다. 지금은 Codex App Server와 Claude Code 공식 도구로 사용량을 확인합니다."
    };
  }
  return {
    status: "not-available",
    message: "이 기기에서 네이티브 OAuth 준비를 완료하지 못했습니다."
  };
}
```

어댑터 상단에 `import { describePrepareFailure } from "./oauth-messages";`를 추가하고 `createStart`의 `catch {` 블록을 교체:

```typescript
  } catch (error) {
    return describePrepareFailure(error);
  }
```

- [ ] **Step 4: 테스트·게이트 실행**

```powershell
npm test
if ($LASTEXITCODE -ne 0) { throw 'frontend tests failed' }
foreach ($gate in @('build', 'verify:tokens', 'verify:memory', 'verify:baseline')) {
    npm run $gate
    if ($LASTEXITCODE -ne 0) { throw "frontend gate failed: $gate" }
}
```

Expected: 기존 21개 기준 총 24개, 매핑 테스트 3건 실행 및 모든 게이트 종료 코드 0. JS 크기와 증감을 기록하고 기존 번들 예산을 통과해야 한다. 순수 함수 테스트는 실제 invoke 연결을 증명하지 않으므로 Task 5의 네이티브 커맨드·앱 QA도 필수다.

- [ ] **Step 5: 커밋**

```bash
git add src/integrations/oauth-adapter.ts src/integrations/oauth-messages.ts tests/oauth-adapter.test.ts
git commit -m "feat(phase3): show a preparing state when native OAuth is compiled out

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: WebView2 렌더러 프로세스 제한

**Files:**
- Modify: `src-tauri/tauri.conf.json` — `app.windows[0]`

**Interfaces:**
- Consumes: Task 1·2 이후 동일 소스·thin LTO의 기본 feature 빌드
- Produces: 인자 미적용/적용 비교와 채택 또는 되돌림 결정. 7 → 5~6은 가설이며 합격 조건이 아니다.

- [ ] **Step 0: 인자 미적용 기준 확보**

`npm run desktop:build` 종료 코드 0을 확인한 뒤 exe 경로·SHA-256·바이트 크기, Windows·WebView2 버전, 창 모드와 연결 상태를 기록한다. 미니 창을 실행하고 60초 안정화 후 기존 측정 스크립트로 5초 간격 12표본을 수집한다. 기존 앱이 실행 중이면 임의 종료하거나 중복 실행하지 말고 측정을 분리한다.

앱 실행은 `Start-Process -FilePath <검증한 exe 절대경로> -WindowStyle Hidden -PassThru`로 PID를 받는다. 앱의 미니 창이 실제 보이는지 확인한다. `Get-CimInstance Win32_Process`에서 이 PID의 자손을 재귀로 추적하고 `--type=renderer`, `--type=gpu-process`, `--type=utility`, 브라우저 및 기타 역할별 개수를 기록한다. 전체 명령줄을 보고서에 복제하지 않는다. 측정 명령의 `-RootPid`에는 검증한 호스트 PID를 명시한다.

```powershell
npm run measure:memory -- -Scenario phase3-renderer-before -RootPid <호스트PID> -Samples 12 -IntervalSeconds 5
if ($LASTEXITCODE -ne 0) { throw 'renderer baseline measurement failed' }
```

`<호스트PID>`는 실행 직전 실제 정수로 대체하는 자리표시자다. 측정 후 트레이 종료로 이 앱을 종료하고 해당 호스트·자손의 종료를 확인한다. 이미지 이름 전체를 대상으로 강제 종료하지 않는다.

- [ ] **Step 1: 설정 추가**

`app.windows[0]` 객체에 `"resizable": true` 다음 줄로 추가:

```json
        "resizable": true,
        "additionalBrowserArgs": "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --renderer-process-limit=1"
```

이 키를 지정하면 Tauri가 기본으로 넣던 `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`이 **대체**되므로 기본값을 그대로 포함해야 한다(스펙 §4 "유지"). `--disable-gpu`·`--single-process`는 사용하지 않는다(기존 결정).

- [ ] **Step 2: 설정 검증 빌드**

```powershell
npm run desktop:build
if ($LASTEXITCODE -ne 0) { throw 'renderer candidate release build failed' }
```

Expected: `Finished`. `tauri.conf.json` 스키마 오류(알 수 없는 키)가 나면 키 이름을 `https://schema.tauri.app/config/2`의 `WindowConfig`에서 확인해 보고한다(Tauri 2에서는 `additionalBrowserArgs`).

- [ ] **Step 3: 즉시 확인(프로세스 수)**

Step 0과 같은 실행·안정화·역할 분류 절차를 적용한다. `additionalBrowserArgs`가 실제 실행 인자에 전달됐는지도 확인한다.

```powershell
npm run measure:memory -- -Scenario phase3-renderer-after -RootPid <호스트PID> -Samples 12 -IntervalSeconds 5
if ($LASTEXITCODE -ne 0) { throw 'renderer candidate measurement failed' }
```

각 구성 2회 재시작 측정을 수행한다(미적용 → 적용 → 적용 → 미적용 순서, 설정 변경마다 재빌드). 두 비교에서 렌더러 또는 전체 프로세스 수 감소가 재현되고 평균 작업 집합이 악화되지 않으며 미니/대시보드 전환·새로고침·트레이 숨김/재표시가 정상인 경우에만 채택한다. 수 감소가 없거나 결과가 일관되지 않으면 효과 미확인으로 기록하고 이번에 추가한 인자 설정을 되돌린다. 기능 회귀가 있으면 즉시 되돌린다. 어느 쪽이든 최종 설정으로 다시 빌드한다. 이는 짧은 비교이며 30분 결과를 대신하지 않는다.

- [ ] **Step 4: 커밋**

채택한 경우에만 아래 설정 커밋을 만든다. 미채택은 Task 5에 비교 결과와 되돌림을 기록한다.

```bash
git add src-tauri/tauri.conf.json
git commit -m "perf(phase3): cap WebView2 renderer processes at one

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: `lto = "fat"` 비교 측정

**Files:**
- Modify(조건부): `src-tauri/Cargo.toml` `[profile.release]`

**Interfaces:**
- Consumes: Task 1·3 이후의 릴리스 빌드
- Produces: 동일 조건의 thin/fat 크기·빌드 시간과 채택 결정. fat exe가 **100,000 B 이상** 작고 Rust clean release 빌드 시간 중앙값이 thin의 2배 이하일 때만 채택한다. 그 외에는 thin 유지. NSIS 크기는 별도 기록한다.
- 규칙 개정(2026-09-08, 컨트롤러 판단 — 실행 결과에 대한 사후 개정이며 원장·`memory-budget.md`에 근거 기록): Step 2의 안정성 조항("같은 구성의 시간 차이가 작은 값의 20%를 넘으면 불안정 → fat 미채택")에 **중대성 예외**를 둔다 — 초과폭이 1%p 이내이고, 크기 기준을 3배 이상 여유로 통과하며(실측 −414,208 B ≥ 3×100,000), 관측된 모든 원시 시간 짝의 fat/thin 비율이 기준(2×)의 2/3 이하(실측 최악 397.50/322.16 = 1.234×)이면 채택한다. 실측(thin 20.017% 초과)은 이 예외에 해당해 fat을 채택했다(커밋 `35f8016`). 예외 없이 문자 규칙을 적용하면 thin 유지가 정답이었음을 함께 기록한다.

- [ ] **Step 1: 비교 조건 고정**

같은 소스·Cargo.lock·Rust 버전·타깃·기본 feature·프론트 dist·Task 3 최종 설정을 사용한다. `npm run build`와 `cargo fetch --manifest-path src-tauri/Cargo.toml --locked`를 먼저 실행하고 각각 종료 코드를 확인한다. 프론트 빌드·다운로드·NSIS 패키징 시간은 Rust 빌드 시간에 섞지 않는다.

기존 target 캐시를 사용하는 thin 재빌드와 fat 재컴파일을 비교하지 않는다. 각 측정은 저장소 내부의 **새 비어 있는 target 디렉터리**를 `--target-dir`로 지정한다. 기존 target을 삭제하지 않는다. 경로는 `src-tauri/target/phase3-lto-<실행ID>-<구성>-<회차>`로 구분하고 존재하면 다른 실행ID를 쓴다. 디스크 여유가 부족하면 시간 비교를 미검증으로 남기고 thin을 유지한다.

- [ ] **Step 2: thin/fat 각각 2회 측정**

thin → fat → fat → thin 순으로 Cargo.toml의 `lto` 값만 바꿔 실행한다. 아래 자리표시자는 회차별 실제 절대경로로 바꾼다. OS 파일 캐시는 통제하지 못하므로 측정 조건에 명시하고, 다른 빌드는 동시에 실행하지 않는다.

```powershell
$trialTarget = '<회차별 새 target 절대경로>'
if (Test-Path -LiteralPath $trialTarget) { throw 'trial target must be fresh' }
$timer = [Diagnostics.Stopwatch]::StartNew()
cargo build --manifest-path src-tauri/Cargo.toml --release --bin spectra-native --no-default-features --locked --offline --target-dir $trialTarget
$buildExit = $LASTEXITCODE
$timer.Stop()
if ($buildExit -ne 0) { throw "LTO trial failed: $buildExit" }
$trialExe = Join-Path $trialTarget 'release/spectra-native.exe'
[pscustomobject]@{ Seconds = $timer.Elapsed.TotalSeconds; Bytes = (Get-Item -LiteralPath $trialExe).Length; SHA256 = (Get-FileHash -LiteralPath $trialExe -Algorithm SHA256).Hash }
```

각 구성의 두 시간 원값과 중앙값(두 값의 평균)을 기록한다. 같은 구성의 시간 차이가 작은 값의 20%를 넘거나 exe 크기가 다르면 비교는 불안정으로 판정해 fat을 채택하지 않는다. 빌드 캐시가 비어 있었다는 뜻의 clean 측정이며 OS 캐시까지 cold인 측정으로 표현하지 않는다.

NSIS는 각 LTO 설정에서 `npm run desktop:build`를 별도 실행하고 종료 코드·exe/설치 파일 바이트 크기·해시를 기록한다. 이 패키징 실행 시간은 위 채택 규칙에 사용하지 않는다.

- [ ] **Step 3: 결정·정리**

채택 규칙을 적용하고 Cargo.toml의 LTO 값만 최종 선택으로 맞춘다. `git diff -- src-tauri/Cargo.toml`로 다른 사용자 변경이 보존됐는지 확인한다. 설정이 바뀌었으면 Task 1 Step 6의 두 구성 release 빌드·테스트를 최종 설정에서 재실행한다. 마지막에는 항상 `npm run desktop:build`와 종료 코드 확인으로 기본 feature 배포본을 만든다. 이 exe/NSIS의 해시·크기를 Task 5·6의 대상으로 고정한다. fat 채택 시에만 다음 커밋을 만든다:

```bash
git add src-tauri/Cargo.toml
git commit -m "perf(phase3): use fat LTO for the release profile

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

어느 쪽이든 두 수치는 Task 5의 문서에 표로 남긴다.

---

### Task 5: 연결 QA 재통과 · 크기 기록 · 문서

**Files:**
- Modify: `docs/performance/memory-budget.md` — "2026-09-05 Phase 3 결과" 절 추가
- Modify: `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — Phase 3 성공 기준 정정 주석

**Interfaces:**
- Consumes: Task 1의 `cargo tree` 출력, Task 4의 최종 릴리스 빌드(채택된 LTO 설정)
- Produces: 문서화된 전후 수치. 30분 측정(Task 6)이 남는다.

- [ ] **Step 1: 기존 Codex·Claude 연결 QA(스펙 성공 기준)**

Task 4에서 고정한 기본 feature exe의 해시를 확인하고 CLI 스냅샷을 실행한다. stdout 전체를 변수로 받아 JSON으로 파싱하고 종료 코드를 즉시 확인한다. 아래는 최소 구조 검사이며 상태·실사용 검증은 이어지는 체크리스트까지 수행한다:

```powershell
$releaseExe = (Resolve-Path -LiteralPath 'src-tauri/target/release/spectra-native.exe').Path
foreach ($provider in @('codex', 'claude')) {
    $raw = & $releaseExe --provider-snapshot $provider
    $snapshotExit = $LASTEXITCODE
    if ($snapshotExit -ne 0) { throw "snapshot failed: $provider ($snapshotExit)" }
    $snapshot = ($raw -join "`n") | ConvertFrom-Json -ErrorAction Stop
    if ($snapshot.providerId -ne $provider) { throw 'providerId mismatch' }
    if ($snapshot.connectionState -notin @('not-installed', 'signed-out', 'waiting-for-usage', 'connected', 'stale', 'error')) { throw 'invalid connectionState' }
    if ($null -eq $snapshot.windows -or $snapshot.windows -isnot [Array]) { throw 'windows must be an array' }
    if ($snapshot.connectionState -eq 'error') { throw "provider returned error: $provider" }
    [pscustomobject]@{ Provider = $provider; State = $snapshot.connectionState; Source = $snapshot.source; WindowCount = @($snapshot.windows).Count }
}
```

- [ ] 각 공급자의 `runtimeAvailable`·`authState`·`source`·`lastSyncedAt`를 현재 `NativeProviderUsageSnapshot` 계약과 대조한다. 사용량 창은 ID, 유한한 0~100 범위 사용률·잔여율, 합계 약 100, nullable 초기화 시각·창 길이의 타입을 확인한다. `connected`인데 창이 비거나 source가 해당 공급자의 허용값과 다르면 실패다.
- [ ] Task 1 착수 전과 같은 로그인·데이터 조건에서 두 공급자의 실제 사용량 조회를 확인한다. 사용률·초기화 시각의 시간 경과 변화는 허용하되 연결 상실·창 소실은 원인을 판별한다. signed-out/not-installed/waiting-for-usage 응답은 구조 검사만 통과한 것이며 연결 QA PASS로 기록하지 않는다. 외부 인증·서비스 문제로 조회하지 못하면 해당 QA는 미검증이다.
- [ ] 최종 네이티브 앱에서 두 공급자를 새로고침하고 로딩 종료, 실제 source·사용량 창 표시, 오류 메시지를 확인한다. 미니/대시보드 전환, 트레이 숨김/재표시도 정상이어야 한다.
- [ ] 기본 feature 앱의 실제 Tauri invoke로 `oauth_prepare`가 정확한 disabled marker로 실패하고 status가 available/present false인지 확인한다. 실제 OAuth 시작 UI가 있으면 "준비 중" 문구까지 확인한다. UI 진입점이 없다면 invoke 결과와 Task 2 매핑 테스트·빌드 검사로 범위를 명시하며 UI PASS를 주장하지 않는다. remove는 Rust no-op 테스트로 확인한다.
- [ ] feature on은 기존 OAuth 테스트·release 빌드 통과로 회귀를 확인하며, 실제 OAuth 인증 성공까지 검증했다고 표현하지 않는다. 스킴 설정·플러그인은 유지되지만 `register_all()`과 콜백 소비는 feature on에서만 실행된다는 점을 기록한다.

보고서에는 상태·검사 결과만 남기고 토큰·이메일·개인 데이터 원문은 옮기지 않는다. 시간 초과·종료 코드 실패·JSON 파싱 실패·기능 회귀가 있으면 후속 완료 판정을 중단한다.

- [ ] **Step 2: memory-budget.md에 Phase 3 절 추가**

Phase 2 절 뒤에 다음 형식으로 추가(수치는 실측값으로 채움):

```markdown
## 2026-09-05 Phase 3 결과 (네이티브 슬림화)

변경: `native-oauth` cargo feature(기본 off)로 OAuth 모듈·네 직접 의존 제외, OAuth 커맨드 3개는 스텁 유지. 딥링크 스킴 설정·플러그인은 유지하되 런타임 등록·콜백 소비는 feature on 전용. 렌더러 제한: {채택 | 효과 미확인으로 원복 | 회귀로 원복}. LTO: {thin 유지 | fat 채택}.

의존 트리: 기본 depth-1에서 네 직접 의존 부재, 기본 전체 normal 트리에서 keyring 부재 확인. 나머지 전이 의존 경로는 실측 출력에 따라 기록. 바이너리 링크 제거를 직접 증명한 결과는 아님.

| 항목 | Phase 2 (2026-09-05 19:11) | Phase 3 thin | Phase 3 fat | 채택 |
|---|---|---|---|---|
| 실행 파일 | 7,302,144 B | … | … | … |
| NSIS 설치 파일 | 2,936,525 B | … | … | … |
| Rust clean 빌드 시간(각 2회·중앙값) | 과거 2m 51s, 조건 달라 비교 제외 | … | … | |

측정 조건: 소스 식별자·lockfile 해시·Rust/Windows/WebView2 버전·타깃·feature·LTO·exe/NSIS 해시·측정 일시 기록.
렌더러 비교: 미적용/적용 각 2회 역할별 프로세스 수·작업 집합 원값과 채택 근거 기록.
연결 QA: Codex {PASS/FAIL/미검증}, Claude {PASS/FAIL/미검증}, 앱 새로고침·창/트레이 {PASS/FAIL/미검증}, OAuth 스텁 invoke {PASS/FAIL/미검증}. 검사 범위와 근거 기재.
```

- [ ] **Step 3: 스펙 성공 기준 정정 주석**

Phase 3 절뿐 아니라 §1 성공 기준 표의 "네 크레이트를 링크하지 않는다" 문구도 직접 의존·keyring 트리 부재·크기 기록 기준으로 일치시킨다. 아래 주석의 날짜는 실제 검증일을 사용한다. 이는 바이너리 링크 증명이 아니라 의존 그래프 기준 정정이다:

```
- 검증 확정({실제 검증일}): 전이 의존이 남는 크레이트는 전체 제거 대상으로 삼지 않으며, 기준은 "기본 feature에서 네 직접 의존 부재 + 정상 종료한 전체 normal 트리에서 keyring 부재 + 실행 파일 크기 전후 기록"으로 정정. 실제 전이 의존 경로는 검증 결과에 병기.
```

- [ ] **Step 4: 커밋**

```bash
git add docs/performance/memory-budget.md docs/superpowers/specs/2026-09-04-design-footprint-improvement.md
git commit -m "docs(phase3): record dependency gating, binary sizes, and LTO comparison

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: 30분 메모리 측정 · 상태 갱신 · 푸시

**Files:**
- Modify: `docs/performance/memory-budget.md` — Phase 3 측정 표
- Modify: `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — 상태 줄

**Interfaces:**
- Consumes: Task 5의 해시가 고정된 최종 릴리스 빌드, `scripts/measure-memory.ps1`(명시적 RootPid, 30초 간격 61표본)
- Produces: `mini-idle-phase3` 측정 결과와 Phase 2 대비 증감, 프로세스 수 확정

- [ ] **Step 1: 측정 준비(사용자 협조)**

사용자에게 아래 절차·명령을 제시하고 결과를 받는다. 기존 앱과 중복 실행하지 않고 검증한 exe를 실행해 미니 창 표시·호스트 PID·실행 경로를 확인한다. 60초 안정화 후 30분간 조작하지 않는다. 전원 모드·창 모드·공급자 연결 상태·WebView2 버전 등 비교 조건을 기록한다. 측정 중 로그인·새로고침·창 전환을 하지 않는다.

```powershell
npm run measure:memory -- -Scenario mini-idle-phase3 -RootPid <호스트PID> -Samples 61 -IntervalSeconds 30
if ($LASTEXITCODE -ne 0) { throw 'phase3 measurement failed' }
```

Expected: CSV 61행(헤더 제외), sample 1~61 연속, 첫·마지막 타임스탬프 차이 ≥1,800초. 기존 스크립트는 첫 표본 직후부터 간격을 두므로 60표본은 명목상 29분 30초이고, 이번에는 61표본으로 최소 30분을 확보한다. 각 간격 30~40초, 호스트가 끝까지 생존, exe 해시 동일, SUMMARY와 CSV 재계산값(평균·최대·private)이 반올림 오차 0.1 이내여야 한다. 누락·수면·앱 재시작·조건 변경 시 해당 측정은 무효로 남기고 원인을 해소한 후 새 파일로 재측정한다. 부분 표본으로 완료하지 않는다.

프로세스 수는 최소/최대와 역할별 관찰값을 기록하고 감소를 강제하지 않는다. Phase 2의 60표본 수치와는 표본 수·안정화 조건 차이를 명시해 관찰된 증감으로만 설명한다. 조건이 크게 다르면 비교를 미검증으로 남긴다. 비교 기준 복원이 가능한 경우에만 같은 조건의 기준 빌드 재측정을 사용자에게 제시한다.

- [ ] **Step 2: 문서 표 추가**

Phase 3 절에 추가:

```markdown
| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | Phase 2 대비 |
|---|---|---|---|---|---|---|
| mini-idle-phase3 (최소 30분) | 61 | … MB | … MB | … MB | 최소 … / 최대 … | … MB(…%), 비교 조건 차이 … |
```

목표치(평균 작업 집합 ≤380 MB) 달성 여부와 남은 격차를 기록한다. exe 축소 여부와 별도 항목으로 판정한다. Phase 4는 숨긴 창의 대기 모드이므로 **표시 중 mini-idle 목표 미달을 해결한다고 간주하지 않는다**. 이번 측정 없이 Phase 4 예상 효과를 실측처럼 쓰지 않는다.

- [ ] **Step 3: 스펙 상태 줄 갱신**

다음 표를 먼저 채운 뒤 상태를 갱신한다. 필수 검증 FAIL은 구현 완료 불가, 미검증은 PARTIAL이다. 수치 목표 미달과 기능 구현 완료를 별도로 표시한다.

| 판정 항목 | 통과 조건 | 미충족 처리 |
|---|---|---|
| 두 feature 구성 | 최종 설정 release 빌드·lib 테스트 정상, 신규 경고 없음 | FAIL/미검증이면 Phase 3 미완료 |
| 의존 게이트 | 기본 직접 의존 4개 부재·전체 normal 트리에 keyring 부재, on 구성 유지 | 미충족이면 FAIL |
| 프론트·연결 QA | Task 2 게이트 및 Task 5의 두 공급자 실제 조회·앱·스텁 검사 통과 | 인증 등 외부 사유는 미검증, Phase 3 PARTIAL |
| 렌더러·LTO 실험 | 정해진 비교와 채택/원복 근거 확보, 최종 배포본 재빌드·해시 확인 | 효과 없어 원복한 실험은 완료 가능; 비교 미수행은 PARTIAL |
| 측정 유효성 | 최소 30분의 유효 CSV·재계산·조건·최종 exe 식별 확인 | 미검증/무효이면 Phase 3 PARTIAL |
| 바이너리 효과 | exe·NSIS 바이트 증감 기록, 감소/동일/증가를 구분 | 의존 제거만으로 크기 개선 PASS 금지; 증가 시 원인과 채택 근거 기록 |
| 메모리 목표 | 유효 측정 평균 ≤380 MB | 초과면 목표 FAIL, 표시 중 목표 미달 명시 |

필수 구현·QA·실험·측정 검증이 끝났을 때만 아래 형식을 사용한다. 실행일을 실제 날짜로 쓰며, 메모리 목표가 미달이면 전체 스펙 목표는 PARTIAL로 남긴다. 바이너리 개선 효과 역시 수치로 별도 표시한다.

```
· Phase 3 구현·검증 완료({검증일}) · 바이너리 {감소/동일/증가}: … B · mini-idle … MB · 메모리 목표 {PASS/FAIL} · 전체 스펙 목표 {PASS/PARTIAL, 다른 필수 항목도 확인} · Phase 4 승인 대기
```

필수 항목이 남으면 `Phase 3 PARTIAL — 미검증: …`로 기록하고 완료 문구를 넣지 않는다. 커밋·푸시 여부는 품질 판정과 분리한다.

- [ ] **Step 4: 커밋·푸시(푸시 전 사용자 확인)**

```bash
git add docs/performance/memory-budget.md docs/superpowers/specs/2026-09-04-design-footprint-improvement.md
git commit -m "docs(phase3): record Phase 3 memory measurement and close the phase

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git log --oneline origin/main..HEAD
```

커밋 목록과 대상(`origin main`)을 제시하고 확인을 받은 뒤 `git push origin main`.
