# SPECTRA 작업표시줄 스트립 주기 갱신 (A안) 설계

> 작성: 2026-09-18 · 기준 커밋 `a87d1a5`(브랜치 `feat/taskbar-usage-strip`, 설치본 0.2.3) · 상태: 사용자 승인(2026-09-18 "A안 선택, 실시간 경로 현행 유지 + 백오프") · 구현 대기
> 목표: 작업표시줄 표시가 켜진 동안 Codex·Claude 잔여율이 사용자 조작 없이 **2분 이내** 최신값을 따라가고, Claude Code 세션이 토큰을 갱신하면 **수 초 안에** "동기화됨"으로 복귀한다. 2026-09-12 스펙이 확보한 대기 프로세스 1개는 유지한다.

## 0. 배경 — 2026-09-18 진단 요약

증상 두 가지를 코드·실측으로 추적한 결과다. 상세 근거는 세션 보고에 있고, 여기에는 설계에 필요한 결론만 적는다.

| # | 증상 | 원인 | 근거 |
|---|---|---|---|
| 1 | 스트립이 자동 갱신되지 않음 | 2026-09-12 스펙 §3-6 "무조건 폴링 금지" 설계. 값은 `provider_usage_snapshot` 완료 시점에만 바뀌고, 그 호출은 창의 부팅·수동 새로고침뿐. Codex는 자동 경로가 없고 Claude만 초기화 시각 경과 감시(15초 확인·60초 재시도) | `desktop_shell.rs` `ensure_taskbar_refresh_loop` |
| 1-a | 재시도 폭주 | 초기화가 지났는데 토큰 만료로 새 값을 못 받으면 60초마다 `claude auth status` 프로세스를 띄우며 무한 반복 | `desktop_shell.rs:330`, `provider_usage.rs:823` |
| 1-b | 창과 스트립 불일치 | 네이티브→WebView 이벤트가 없어 루프가 갱신해도 숨겨진 창은 수동 새로고침 전까지 옛 값 | `lib.rs` emit은 OAuth 2종뿐 |
| 2 | Claude가 CLI 실행 때만 동기화됨 | 실시간 조회는 `~/.claude/.credentials.json`의 access token(수명 8시간, 실측 mtime+8h)에 의존. SPECTRA·`claude auth status` 모두 갱신하지 않고, Claude Code 세션 시작(터미널·데스크톱 앱 모두)만 갱신한다 | `provider_usage.rs:575·866` |
| 2-a | 데스크톱 앱 세션은 브리지 미발동 | status line 브리지는 터미널 UI 기능. 데스크톱 앱(Code 탭) 세션이 도구 20회를 실행하는 동안 캐시 mtime 불변(20:59:41→21:01:27) | 실험 |
| 2-b | 라벨 | 브리지 값은 `!source_is_live`면 항상 stale → 몇 초 전 값도 "갱신 대기 (캐시)". 토큰 만료 상태도 같은 라벨이라 조치(Claude Code 실행)를 알 수 없음 | `provider_usage.rs:502` |

비용 실측(설치본 0.2.3, 2026-09-18 21:00 KST): Claude 조회 1.1초(`claude auth status` + HTTPS 1회), Codex 조회 1.3초(`codex app-server` + RPC 2회).

정책 확인: [code.claude.com legal-and-compliance](https://code.claude.com/docs/en/legal-and-compliance)는 OAuth 인증을 Claude Code·자사 앱의 통상 사용 전용으로 규정하고 사전 통보 없는 조치를 명시한다. `/api/oauth/usage` 실시간 경로는 이 토큰을 쓰므로 **차단 가능성이 있는 best-effort**다. 사용자 결정: 현행 유지하되 401·403·429는 백오프로 흡수하고 차단 시 캐시 폴백(기존)에 맡긴다.

## 1. 성공 기준 (체크 가능 형식)

| 축 | 기준 | 측정 방법 |
|---|---|---|
| 주기 | strip ON에서 Codex·Claude 각각 마지막 시도로부터 **120초(+tick 15초 이내)** 안에 `provider_usage_snapshot`이 다시 호출된다 | `SPECTRA_TIMING_LOG=1`로 실행한 `standby-timing.log`의 `usage_refresh:` 행 간격 + 스케줄러 단위 테스트 |
| 직렬 | 한 tick 안에서 Claude → Codex 순으로 **순차** 실행되며 두 공급자 프로세스를 동시에 띄우지 않는다 | 코드 리뷰(`for` 루프 + `block_on`) + 로그 시각 |
| 즉시 갱신 | `.credentials.json` mtime 변경 후 **다음 tick(≤15초)** 에 Claude를 재조회하고 보류·백오프를 해제한다 | 단위 테스트 + 로그(`usage_refresh:claude:ok`) |
| 백오프 | 실패가 연속되면 재시도 간격이 **120·240·480·960·1800초(상한)** 로 늘고 성공 시 초기화된다 | 단위 테스트 |
| 토큰 만료 보류 | `claude-oauth-token-expired`면 자격 증명 변경 또는 **1800초** 경과까지 네트워크·CLI 호출을 하지 않는다(기존 60초 재시도 폭주 제거) | 단위 테스트 |
| 창 일치 | WebView가 살아 있으면 `provider-usage-updated` 이벤트로 창 수치·"조회 시도" 라벨이 스트립과 같은 스냅샷을 따른다 | 창을 연 채 2분 대기 후 육안 확인 |
| 라벨 | 토큰 만료 상태에서 창(`claudeFreshness`)과 스트립 툴팁(`claude_status`) 모두 **"로그인 갱신 필요"** 를 표시한다 | 단위 테스트(Rust·TS) |
| 기본값 | strip OFF에서는 주기 조회가 **0회**다(현행 동작 유지) | 단위 테스트(`plan`이 빈 목록) |
| 회귀 | `cargo test --lib` 기본·`native-oauth` 통과, `npm test` 통과, `cargo build --release` 경고 0, `npm run verify:memory` JS ≤ 240,000 B | 각 명령 |
| 의존성 | 신규 크레이트 0, 신규 npm 패키지 0 | `git diff -- Cargo.lock package-lock.json` 비어 있음 |

## 2. 설계 결정

### 2-1. 갱신 위치: 네이티브 루프 (프론트 폴링 아님)

기존 `spectra-claude-reset-refresh` 스레드를 **공급자 공통 주기 갱신 루프**로 일반화한다. WebView가 파기된 대기 상태에서도 스트립을 최신화해야 하고, 프론트에는 `setInterval` 금지 패턴(2026-09-12 plan 전역 제약)이 있기 때문이다. 루프는 strip ON일 때만 시작되고(기존 `ensure_*` 진입점 유지), strip OFF 동안은 tick만 돌며 아무 조회도 하지 않는다.

### 2-2. 순수 스케줄러 + I/O 분리

판단 규칙은 I/O 없는 `src-tauri/src/usage_refresh.rs`(`Scheduler`)에 두고 전부 단위 테스트한다. 파일 mtime 조회·프로세스 spawn·이벤트 발행은 `desktop_shell.rs` 루프와 `lib.rs`가 맡는다. `taskbar_strip.rs`(순수) / `taskbar_window.rs`(Win32) 분리 선례를 따른다.

### 2-3. 갱신 규칙

| 항목 | 값 | 비고 |
|---|---|---|
| tick | 15초 | 기존 값. 매 tick에 스트립을 다시 그려 "N분 전" 라벨을 진행시킨다(조회 아님) |
| 정기 간격 `INTERVAL_SECS` | 120초 | 사용자 요구 "2분 단위" |
| 순서 | Claude → Codex | 한 tick에 순차 실행 |
| 시작 시점 | 루프 시작 시각을 첫 시도로 간주 | 창 부팅 조회와 15초 안에 겹치지 않게 함 |
| Claude 초기화 경계 | 지났으면 마지막 시도 60초 후 즉시 | 기존 동작 유지, 단 아래 보류·백오프가 우선 |
| 자격 증명 변경 | `.credentials.json` mtime이 직전 관측과 다르면 다음 tick에 Claude 즉시 조회, 보류·백오프 초기화 | 첫 관측은 기록만 한다 |
| 실패 백오프 | 연속 실패 n회 → `min(120 × 2^(n−1), 1800)`초 뒤 재시도 | 공급자별 독립. 성공 시 0으로 |
| 토큰 만료 보류 | `claude-oauth-token-expired` → 자격 증명 변경 또는 1800초까지 조회 없음 | 60초 재시도 폭주 제거 |

### 2-4. 실패 신호: 스냅샷 `liveFailure` 필드

`ProviderUsageSnapshot`에 `live_failure: Option<String>`(직렬화 `liveFailure`)를 신설한다. 값은 기존 오류 코드 문자열이며 비밀 정보를 담지 않는다.

| 공급자 | 상황 | 값 |
|---|---|---|
| 공통 | CLI를 찾지 못함(`unavailable_snapshot`) | `"runtime-unavailable"` |
| Claude | `auth status` 실행 실패 | `"claude-auth-status-failed"` |
| Claude | 로그아웃 | `"claude-signed-out"` |
| Claude | 실시간 조회 실패(캐시 폴백 여부 무관) | 기존 `live_error` 코드(`claude-oauth-token-expired`·`claude-usage-unauthorized`·`claude-usage-http-429`·`claude-usage-network-failed` 등) |
| Claude | 실시간 조회 성공 | `None` |
| Codex | App Server 시작·RPC 실패 | 기존 `reason` 코드 |
| Codex | 로그아웃 | `"codex-signed-out"` |
| Codex | API 키 로그인(요금제 아님) | `"codex-api-key-auth"` |
| Codex | 연결됐으나 창 없음 / 연결 | `None` |

스케줄러(`Outcome`), 스트립 툴팁(`claude_status`), 창 라벨(`claudeFreshness`)이 모두 이 한 필드를 읽는다. 문자열 매칭으로 `message`를 해석하지 않는다.

### 2-5. WebView 동기화: `provider-usage-updated` 이벤트

커맨드 `provider_usage_snapshot`과 루프가 공유하는 `refresh_provider`가, 해당 요청이 최신일 때(기존 `snapshot_request_is_current`) 캐시·배지·스트립 갱신에 이어 `app.emit("provider-usage-updated", &snapshot)`을 호출한다. 창이 없으면 emit은 조용히 성공한다. 프론트는 수신 시 해당 공급자의 refresh sequencer 티켓을 새로 발급한 뒤 상태를 덮어써, 진행 중이던 구 요청의 결과가 나중에 도착해도 더 오래된 값으로 되돌리지 못하게 한다. "조회 시도" 라벨은 `자동 확인 HH:MM`으로 바뀐다.

### 2-6. 실시간 경로: 현행 유지 + 백오프

토큰 읽기·`/api/oauth/usage` 호출 코드는 바꾸지 않는다. 401·403·429·네트워크 오류는 2-3의 백오프로 흡수하고, 차단되면 기존 캐시 폴백과 힌트 문구가 그대로 동작한다. SPECTRA가 refresh token으로 직접 갱신하는 방식은 채택하지 않는다(비공개 엔드포인트, 토큰 회전 시 Claude Code 로그아웃 위험, 정책 저촉).

### 2-7. 보류 항목과 사유

- `claude auth status` 생략(진단 보고의 A.5): `plan_type`·`auth_method`가 그 출력에 의존해 창의 "Claude Max" 표기가 사라진다 → 미채택. 주기당 비용 1.1초는 감수한다.
- strip OFF + 창 열림 상태의 주기 갱신: 요청 범위(작업표시줄) 밖.
- 브리지 값 신선도 라벨(B3): 터미널 사용자만 영향 → 이번 범위 밖.

### 2-8. 로그

`SPECTRA_TIMING_LOG=1`일 때 기존 `standby-timing.log`에 `usage_refresh:<provider>:<outcome>` 행(경과 ms 포함)을 남긴다. 이번 진단이 로그 부재로 추정에 그친 부분을 메운다.

## 3. 비용 근거

2분마다 Claude 1.1초 + Codex 1.3초 ≈ 2.4초의 단기 프로세스 실행이다(CPU 듀티 약 2%). 두 프로세스 모두 조회 직후 종료되므로 상주 메모리 증가는 없다. 2026-09-12의 standby-strip 30분 실측(33.6 MB)은 주기 갱신 이전 값이며, 재측정은 후속 항목으로 남긴다.

## 4. 범위 밖

- Codex 초기화 경계 감시(2분 주기로 충분)
- 실시간 경로의 토큰 갱신·대체 인증
- 세션 잠금·디스플레이 꺼짐 시 조회 중단
- macOS·Linux

## 5. 문서 갱신 대상

- `docs/superpowers/specs/2026-09-12-taskbar-usage-strip.md` §1 "갱신" 행, §1-1, §3-6 — 이 문서를 가리키도록 정정
- `docs/superpowers/plans/2026-09-12-taskbar-usage-strip.md` 전역 제약 "무조건 폴링 금지", 갱신 트리거 문장, 스펙 대응표
- `docs/performance/memory-budget.md` 폴링 불변식 문장, 2026-09-12 절 문장, 2026-09-18 절 추가
- `README.md` 작업표시줄 표시 문장, 메모리 정책 문단
- 이 파일 상태 줄 — 완료 시 게이트 결과 기록
