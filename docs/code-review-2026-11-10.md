# SPECTRA 코드 리뷰·개선 계획 — 2026-11-10

- 대상: `spectra-ai-usage-widget` v0.3.4, 커밋 `5b4f046`(브랜치 `claude/code-review-claude-usage-limit-ee1339`)
- 범위: `src/`, `src-tauri/src/`, `scripts/`(taskbar-keeper 제외), `tests/`, `.github/workflows/`, 루트 설정·문서
- 방법: 전체 정독(약 11,700행) + 기준 게이트 실행 + Claude 사용량 응답 실측 1회(2026-11-10 09:42 KST, 값·초기화 시각만 확인)
- 순서: ① 리뷰 → ② 개선 계획 → ③ 이번 작업(Claude 모델별 한도 추가). ③ 외 항목은 이 문서에서 계획만 세우고 코드는 바꾸지 않음

## 1. 기준 상태 (변경 전)

| 검사 | 결과 |
|---|---|
| `npm test` | 90 통과 / 0 실패 |
| `npm run build` | PASS · JS 239,992 B · CSS 23,636 B |
| `verify:baseline` · `verify:tokens` · `verify:memory` · `verify:release` | 모두 PASS (JS 예산 250,000 B) |
| `cargo test --lib --locked` | 130 통과 / 0 실패 / 9 ignored |

## 2. 발견 사항

심각도: **높음** = 사용자 데이터·표시값이 틀리거나 손상될 수 있음 / **중간** = 특정 조건에서 오동작·혼란 / **낮음** = 유지보수·정리.

### 2.1 데이터 수집 (`src-tauri/src/provider_usage.rs`)

| # | 심각도 | 위치 | 내용 | 조치 방향 |
|---|---|---|---|---|
| D1 | 중간 | `parse_claude_usage_response` (734행) | 응답의 `five_hour`·`seven_day`만 읽음. 실제 응답에는 `limits` 배열이 있고 모델 범위 한도(`weekly_scoped`, 모델 `Fable`)가 들어 있으나 버려짐 → 앱 어디에서도 Fable 한도를 볼 수 없음 | **이번 작업(§4)** |
| D2 | 중간 | `write_json` (786행) → `install_claude_bridge_at`·`remove_claude_bridge_at` | 사용자의 `~/.claude/settings.json`을 `fs::write`로 직접 덮어씀. 쓰는 도중 종료되면 Claude Code 설정 파일이 잘린 채 남음. `usage_history::save_to`·`standby::save_prefs`는 임시 파일 + rename인데 여기만 다름 | 임시 파일 + rename으로 통일 |
| D3 | 낮음 | `write_json` → `claude-usage.json` | 앱(2분 주기)과 상태선 브리지 프로세스가 같은 파일을 비원자적으로 씀. 폴백 읽기가 쓰는 순간과 겹치면 파싱 실패 → 잠깐 "사용량 갱신 대기" | D2와 같은 수정으로 해소 |
| D4 | 낮음 | `fetch_claude_usage_live` (744행) | 2분마다 `reqwest::Client`를 새로 만듦(TLS 설정 재구성) | `OnceLock<Client>` 재사용 |
| D5 | 낮음 | `parse_rfc3339_seconds` (664행) | 월별 일수 검증 없음(`02-31` 허용) | 서버 값만 받으므로 실해 없음. 손댈 때 함께 보강 |
| D6 | 낮음 | `run_statusline_bridge` (1505·1527행) | `extract_claude_cache`를 같은 입력에 두 번 호출 | 한 번만 계산해 재사용 |
| D7 | 낮음 | `start_login` (1256행) | 로그인 프로세스 `Child`를 회수하지 않고 버림(앱 종료 때까지 핸들 유지) | 분리 스레드에서 `wait` |
| D8 | 정책 위험 | `CLAUDE_USAGE_URL` | `api.anthropic.com/api/oauth/usage`는 공개 문서화된 제3자용 경로가 아님. 응답 스키마가 예고 없이 바뀔 수 있음(이번에 `limits`·`weekly_scoped_shares` 등 신규 필드 확인) | 알 수 없는 필드는 무시하고 아는 값만 표시하는 현 구조 유지. 스키마 표본을 테스트 픽스처로 보관(§4에서 추가) |

### 2.2 프론트엔드 (`src/`)

| # | 심각도 | 위치 | 내용 | 조치 방향 |
|---|---|---|---|---|
| U1 | 중간 | `App.tsx` `connectOAuth`·`disconnectOAuth` (733~770행) | `startNativeProviderLogin`·`installNativeProviderBridge`·`removeNativeProviderBridge`의 거부(reject)를 잡지 않음. 네이티브 명령이 실패하면 연결 창에 아무 안내 없이 멈춤 | `try/catch` 후 `actionFeedback`에 실패 문구 |
| U2 | 낮음 | `App.tsx` 470·791행 `refreshedAt` | 상태를 갱신만 하고 화면 어디에도 쓰지 않음. 자동 갱신마다 불필요한 재렌더 | 상태·`LayoutActions` 필드·`autoRefreshLabel` 제거 또는 실제 표시 |
| U3 | 낮음 | `src/data/next-action.ts`, `src/components/Sparkline.tsx` | 런타임(`main.tsx` → `App.tsx`)에서 import하지 않음. 테스트(`tests/next-action.test.ts`·`time-progress.test.ts`)와 `verify-memory-budget.mjs`의 파일 목록에만 남음 | 사용 계획이 없으면 모듈·테스트·게이트 목록 함께 정리 |
| U4 | 낮음 | `App.tsx` `ModelSelector` 미니(234행) | 모델군이 1개인 서비스(Codex 등)는 요금제 이름 아래에 같은 이름 한 개만 든 선택 상자가 나옴 | 모델군 1개면 선택 상자 대신 이름만 표시 |
| U5 | 낮음 | `App.tsx` `quotaFromSnapshot`(112행)·`nativePendingQuota`(141행) | 미연결 시 예시 창을 0%로 바꿔 자리표시로 씀. 예시에 모델군 창이 들어가면 계정에 없는 모델군이 자리표시로 보일 수 있음 | **이번 작업(§4)**: 자리표시는 모델군 없는 창만 |
| U6 | 낮음 | `role="tablist"`/`role="tab"` (216·220·228·235·236행) | `tabpanel`·방향키 이동 없이 역할만 선언 | 버튼 그룹(`aria-pressed`)으로 바꾸거나 키보드 처리 추가 |
| U7 | 낮음 | `App.tsx` 186행 `ProviderLogo` | 쓰지 않는 `size` prop | 제거 |

### 2.3 네이티브 셸·작업표시줄·기록 (`src-tauri/src/` 나머지)

| # | 심각도 | 위치 | 내용 | 조치 방향 |
|---|---|---|---|---|
| N1 | 중간 | `provider_usage.rs` `parse_claude_reset` ↔ `usage_history.rs` 70행, `src/data/focus-dock.ts` `historySegments` | 서버의 `resets_at`이 호출마다 ±1초 흔들림(예: `…14:00:00.0158` → 1794751200, 다른 호출은 1794751199). 기록은 초기화 시각이 같을 때만 같은 버킷으로 합치고, 그래프는 초기화 시각이 다르면 선을 끊음. **실측(설치본 기록 1,857점, 2026-11-10)**: Claude 주간 인접 485쌍 중 163쌍이 2초 이내 차이, 385개 버킷 중 101개에 중복 점 | Claude 초기화 시각을 분 단위로 반올림해 저장(수집 단계 한 곳) |
| N2 | 중간 | `lib.rs` 296~300행, `usage_history.rs` 72·94~107행 | 성공한 조회마다 `captured_at`이 바뀌어 기록 전체를 다시 직렬화·`sync_all`. 그동안 `history` 잠금을 쥐고 있어 `get_usage_history`가 대기. 31일 누적 시 2분마다 수 MB 재기록(추정) | 값·버킷이 바뀔 때만 저장, 쓰기는 복제본으로 잠금 밖에서 |
| N3 | 중간 | `lib.rs` 465행, `desktop_shell.rs` 297~299·319~322행 | 자동 갱신 루프는 작업표시줄 표시가 켜져 있을 때만 시작. 메모리 절약 대기 켬 + 작업표시줄 끔 상태로 창을 닫으면 갱신 주체가 없어 트레이 배지가 마지막 값에 고정 | WebView가 없으면 설정과 무관하게 루프 가동 또는 만료 시 배지 초기화 |
| N4 | 중간 | `tray_badge.rs` 55행 ↔ `taskbar_strip.rs` 73~83행 | 배지는 5시간 창 우선, 작업표시줄 간결 표시는 최소값. Claude 5시간 90%·주간 3%면 배지는 90%(테스트가 현 동작을 고정 — 의도 여부 확인 필요) | 창 선택 규칙을 한 모듈로 모으고 기준 결정 |
| N5 | 중간 | `antigravity.rs` 264~268·296·303~311행 | 백업 파일(`antigravity-statusline-bridge.json`)이 없거나 깨지면 브리지 재설치·제거가 모두 실패하고 화면에는 같은 문구만 나옴 | 백업이 없으면 이전 설정 없음으로 간주해 복구 |
| N6 | 중간(보안) | `tauri.conf.json` 12~15행, `capabilities/default.json` | CSP 미설정 + `withGlobalTauri: true`. WebView에 스크립트가 주입되면 브리지 설치·로그인 시작 명령을 제한 없이 호출 가능(현재 원격 리소스 로드는 없음) | `default-src 'self'` 기반 CSP 추가 |
| N7 | 낮음 | `taskbar_window.rs` 68~89·472~479행 | 전역 위치 변경 훅에 디바운스 없음 — 창 이동마다 작업표시줄 전체 다시 그림 | 타이머 디바운스, 변화 없으면 생략 |
| N8 | 낮음 | `taskbar_window.rs` 334~376행, `desktop_shell.rs` 277~279행 | 작업표시줄 창이 외부 요인으로 닫혀도 핸들이 남아 설정을 껐다 켜기 전까지 복구되지 않음 | 스레드 종료 플래그, 게시 실패 시 핸들 비우기 |
| N9 | 낮음 | `usage_refresh.rs` 159~162행 | 실시간 조회가 안 되는 Claude 계정(API 키 등)은 캐시 폴백이 항상 실패로 집계되어 30분 백오프 → 브리지가 방금 쓴 값도 최대 30분 늦게 반영 | 캐시 파일 수정 시각 감시 또는 폴백을 실패 누적에서 제외 |
| N10 | 낮음 | `usage_history.rs` 14·85·91행 | `deny_unknown_fields` + 전체 파싱 실패 시 빈 목록 → 상위 버전에서 내려오거나 한 점이 깨지면 31일 기록이 조용히 사라짐 | 점 단위 관대 파싱, 실패 파일 보존 |
| N11 | 낮음 | `desktop_shell.rs` 82~89행 | 창을 다시 표시할 때마다 크기·항상 위 설정을 기본값으로 되돌림 — 사용자가 끈 "항상 위"가 재표시 때 풀림(프론트 재적용 여부 미확인) | 선택값 유지 |
| N12 | 낮음 | `tray_badge.rs` 39행 ↔ `taskbar_strip.rs` 54행, `antigravity.rs` 349행 ↔ `taskbar_strip.rs` 130행 | `usable` 2벌(조건 다름), 신선도 경계 1초 불일치(`>` vs `<`), `unix_now` 3벌, `url` 의존성은 `native-oauth` 없이는 미사용 | 창 선택·신선도 규칙 단일 모듈화 |

확인했으나 발견 없음: 잠금 순서 역전·교착, GDI·핸들 누수, 외부 입력에 의한 패닉, 시간 연산 넘침, 명령 주입, 계정 정보 기록.

### 2.4 프론트엔드 — 화면 동작·접근성·미사용 코드

| # | 심각도 | 위치 | 내용 | 조치 방향 |
|---|---|---|---|---|
| U8 | 중간 | `App.tsx` 419·486행, `focus-dock.ts` `selectedModelGroup` | 한도 창 선택(`selectedWindowId`)이 모든 서비스 공용이고 기본값이 주간. Codex 5시간 24%·주간 58%면 사이드바는 24%, 큰 숫자는 58%. Codex에서 5시간을 고르면 다른 서비스도 바뀜 | 서비스별로 분리, 미선택 시 요약 창을 따름 |
| U9 | 중간 | `App.tsx` 427~436·574·594·684행 | 기록 전체(최대 수만 점)를 갱신 이벤트·30초 폴링마다 다시 받음. 기록 화면이 없는 미니창에서도 받음 | 추이·잔여량 화면에서만, 이벤트 경로로 일원화 |
| U10 | 중간 | `App.tsx` `nativePendingQuota` | 연결 확인 전 예시 요금제 이름("Claude Max" 등)이 실제 요금제처럼 보임 | `planName(id, null)` 사용 |
| U11 | 중간 | `styles.css` 140행 | 미니창 모델군 선택 상자가 승인된 기준 디자인(전체 폭)과 달리 내용 폭으로 그려짐(Claude 94px 실측). `.model-controls`의 `justify-content: space-between`이 미니 grid에 상속 | **이번 작업에서 수정**(`justify-content: stretch`, 394px 확인) |
| U12 | 중간 | `index.html` 16행 | 앱 루트 전체가 `aria-live="polite"` — 30초 틱·탭 전환마다 화면 낭독 대상 | 제거 |
| U13 | 중간 | `styles.css` 84행, `tokens.css` | 라이트 테마에서 `.freshness-note` 대비 약 2.3:1(계산값) | 라이트용 amber 재정의 |
| U14 | 낮음 | `App.tsx` 544·623·645행 | `error instanceof Error`만 처리 — Tauri는 Rust `Err(String)`을 문자열로 거부하므로 네이티브 오류 문구가 항상 일반 문구로 바뀜(런타임 미확인) | 문자열 거부도 처리 |
| U15 | 낮음 | `App.tsx` 733~761행 | 연결 버튼에 진행 중 가드 없음 — 두 번 누르면 로그인·브리지 설치 2회 | 진행 중 비활성화 |
| U16 | 낮음 | 미사용 코드 | `credential-vault.ts`·`oauth-adapter.ts`·`provider-capabilities.ts`(런타임 import 0), `App.tsx` 45~47행 `hasDisplayValue`·`displayPercent`·`meterPercent`, `getNativeUiPrefs`, `.oauth-steps` CSS, 루트 `app.js`·`server.mjs`(README 실행 안내는 동작 불가) | 보존 목적을 문서에 남기거나 정리 |

### 2.5 테스트·게이트·CI·문서

| # | 심각도 | 위치 | 내용 | 조치 방향 |
|---|---|---|---|---|
| G1 | 중간 | `scripts/verify-memory-budget.mjs` 6~24행 | 검사 목록에 미사용 파일 2개가 있고 실제 런타임 파일 5개가 빠짐. `Object.freeze`·`clearTimeout` 검사는 한 번만 나와도 통과 | 진입점에서 실제 import 그래프 기준으로 |
| G2 | 중간 | `scripts/verify-design-tokens.mjs` 48~52행 | 토큰이 1회만 쓰여도 통과, `.glass-card` 규칙은 없어 항상 통과 | 실효 있는 조건으로 교체 또는 삭제 |
| G3 | 중간 | `tsconfig.json` | `tests/`가 타입 검사 대상이 아님 — 필드 이름이 바뀌어도 테스트 타입 오류를 못 잡음 | 테스트용 `tsc` 설정 추가 |
| G4 | 중간 | `publish-tauri.yml` 3~6·14~15·41~45·75행 | 어느 브랜치든 `v*` 태그만 올리면 최신 릴리스로 공개되어 자동 업데이트로 배포. 작업 전체가 `contents: write`, `checkout`·`setup-node`는 가변 태그, `verify:memory`·`verify:tokens`·`verify:baseline` 미실행 | `main` 조상 검증, 권한 최소화, SHA 고정, 게이트 추가 |
| G5 | 낮음 | `.gitignore` | `.claude/`(작업 트리·실행 설정)·`.playwright-mcp/`·`.env*` 없음 | 추가 |
| G6 | 낮음 | 문서 | README 74~78행(`node server.mjs`·`?variant=` 동작 불가), 5·120행(Antigravity 누락), 102·116행("시계열 미저장·폴링 없음" — 31일 기록·30초 폴링 있음), PLAN.md 28·33행 | 문구 정정 |

확인했으나 발견 없음: `refreshProvider`·갱신 이벤트·`createRefreshSequencer` 사이 경합, 초↔밀리초 변환 오류.

## 3. 개선 계획

원칙: 표시값 정확성 → 사용자 파일 안전 → 정리 순. 각 단계는 독립 PR, 완료 기준은 체크 가능한 형식.

| 단계 | 내용 | 대상 발견 | 완료 기준 | 상태 |
|---|---|---|---|---|
| P0 | Claude 모델별 한도(Fable) 수집·표시 | D1·U5·D8 | §4 완료 기준 전부 충족 | **이번 작업** |
| P1 | 표시값 정확성 | N1·N4·U8·U10 | 같은 버킷에 Claude 점 1개, 추이 선이 초기화 때만 끊김(설치본 기록으로 재측정) / 배지·작업표시줄·창의 대표 값 기준이 문서 한 곳에 정의되고 테스트로 고정 | 계획 |
| P2 | 사용자 파일·데이터 안전 | D2·D3·N5·N10·N6 | `~/.claude/settings.json`·Antigravity 설정·캐시 쓰기가 모두 임시 파일 + rename / 백업 유실 상태에서 브리지 제거 성공 / 기록 파일 일부 손상 시 나머지 보존 / CSP 적용 후 화면·업데이터 정상 | 계획 |
| P3 | 갱신·부하 | N2·N3·N9·U9·N7 | 값 변화 없는 조회에서 기록 파일 미기록 / 대기 모드 + 작업표시줄 끔에서도 배지가 120초 주기로 갱신 / 미니창에서 기록 조회 0회 | 계획 |
| P4 | 릴리스·게이트 | G1~G4 | 태그가 `main` 조상이 아니면 발행 실패 / PR에서 `npm test`·`cargo test`·게이트 4종 실행 / 게이트가 실제 런타임 파일을 검사 | 계획 |
| P5 | 접근성·오류 안내 | U1·U6·U12~U15 | 연결 실패 시 안내 문구 표시 / 낭독 영역이 상태 문구로 한정 / 라이트 테마 대비 4.5:1 이상 | 계획 |
| P6 | 정리 | U2~U4·U7·U16·D4~D7·N8·N11·N12·G5·G6 | 미사용 모듈·상태 0건(또는 보존 사유 문서화) / README·PLAN이 현재 동작과 일치 | 계획 |

- P1의 N1은 이번에 추가한 Fable 추이에도 같은 영향을 주므로 다음 순서 1번
- P2~P6은 서로 독립이라 순서를 바꿔도 됨. 배포(버전 올림·태그)는 별도 승인 사항

## 4. 이번 작업 — Claude 모델별 한도(Fable) 추가

### 4.1 요구와 실측의 차이

- 요구: "클로드 사용량 관련 Fable 5시간 한도 추가. 미니창 기준 드롭박스 형태, 대시보드에서도 볼 수 있도록"
- 실측(2026-11-10 09:42 KST, Max 요금제): 응답 `limits` 배열은 3건

| `kind` | `group` | 범위 | 사용률 | 초기화 |
|---|---|---|---|---|
| `session` | `session` | 전체 | 19% | 5시간 창 |
| `weekly_all` | `weekly` | 전체 | 26% | 주간 |
| `weekly_scoped` | `weekly` | 모델 `Fable` | 10% | 주간 |

- **Fable 전용 한도는 5시간 창이 아니라 주간 창으로만 내려옴.** Fable 5시간(세션) 범위 한도는 현재 응답에 없음
- 결정 ①: 값을 지어내지 않고 `limits`의 모델 범위 항목을 그대로 모델군으로 읽음(`Fable · 주간 한도`)
- 결정 ②(사용자 확정, 2026-11-10): **Fable에는 주간 한도만 있음.** Fable 모델군에는 서버가 준 창만 담음. 계정 전체 5시간 한도를 Fable 아래에 "전체 모델 공용"으로 함께 보여 주던 중간 구현(커밋 `5ab1703`)은 사용자 확인에 따라 제거함
- 서버가 `group: "session"` + 모델 범위 항목을 실제로 내려주는 경우에만 같은 모델군에 `5시간 한도`가 나타남(수집 계층은 그대로 지원)

### 4.2 설계

| 계층 | 변경 |
|---|---|
| 수집(Rust) | `limits[]` 중 `scope.model.display_name`이 있는 항목만 채택. `group`이 `session` → `rolling`(5시간), `weekly` → `weekly`(주간), 그 외는 버림. 모델군 id는 표시 이름을 소문자·영숫자·하이픈으로 정규화(`Fable` → `fable`). 최대 8건, 같은 (모델군, 창) 중복은 첫 항목만 |
| 캐시 | `ClaudeUsageCache.scoped`(기본값 빈 배열, 비면 직렬화 생략) — 기존 `claude-usage.json`과 호환. 상태선 브리지는 `five_hour`·`seven_day`만 받으므로 `scoped`를 쓰지 않음(오래된 Fable 값을 새 값처럼 섞지 않음) |
| 스냅샷 | 기존 두 창(모델군 없음) 뒤에 `modelGroupId: "fable"`, `modelGroupLabel: "Fable"` 창을 덧붙임. 기존 창의 id·순서·기록 키는 그대로 → 기록 연속성 유지. 초기화 시각이 지난 모델별 창은 새 조회 전까지 스냅샷에서 빼고, 계정 전체 한도의 stale 판정·상태 문구에는 모델별 창을 쓰지 않음 |
| 작업표시줄·트레이 | 표시 값은 종전대로 전체 모델 5시간·주간만. 모델군 창은 고르지 않도록 명시. 작업표시줄 tooltip에는 `Fable / 주간 한도 90%`로 구분 표기 |
| 기록 | 모델군 창도 15분 단위로 기록(시리즈 9 → 10). `MAX_POINTS`를 12개 시리즈 × 31일(35,712)을 담도록 36,000으로 조정 |
| 미니창 | 기존 모델군 선택 상자(드롭다운)에 `전체 모델`·`Fable`이 나옴. 기본 선택은 `전체 모델`(Fable 잔여량이 더 낮아도 자동 전환하지 않음). `Fable` 선택 시 주간 한도 잔여량·초기화 표시(한도 탭 없음), `다른 모델군` 줄에는 항상 나머지 모델군 잔여량 표시. 선택 상자 폭을 기준 디자인대로 전체 폭으로 수정(U11) |
| 대시보드 | 개요의 모델군 탭(`전체 모델`·`Fable`)과 한도 탭(`5시간 한도`·`주간 한도`), `다른 모델군` 카드, 구독 모델 화면의 모델군별 목록, 추이(모델군별 기록)에 자동 반영 |
| 브라우저 데모 | Claude 예시 데이터에 `Fable · 주간 한도` 1건 추가. 미연결 자리표시는 모델군 없는 창만 사용(U5) |

### 4.3 완료 기준

- [x] 실측 응답 표본으로 만든 Rust 테스트에서 Claude 스냅샷 창이 `rolling`·`weekly`(모델군 없음) + `weekly`(모델군 `fable`) 3건으로 나옴
- [x] `limits`가 없거나 형식이 다른 응답에서도 기존 두 창이 종전과 동일하게 나옴
- [x] 작업표시줄 5h·7d 칸과 트레이 배지 값이 모델군 창 유무와 무관하게 동일함(테스트로 고정)
- [x] 미니창 선택 상자에 `전체 모델`·`Fable`이 있고, `Fable` 선택 시 Fable 주간 한도만 표시됨(공용 5시간 제거 후 브라우저 데모로 확인, 설치본 재확인은 미실시)
- [x] 대시보드 개요·구독 모델 화면에 Fable 주간 한도가 표시됨
- [x] `npm test`·`npm run build`·검증 게이트 4종·`cargo test --lib --locked` 통과
- [x] 실제 계정으로 `--provider-snapshot claude` 실행 시 Fable 창이 포함됨

## 5. 검증 기록

| 검사 | 변경 전 | 변경 후 |
|---|---|---|
| `npm test` | 90 통과 | 94 통과 / 0 실패 |
| `npm run build` | JS 239,992 B · CSS 23,636 B | JS 240,392 B · CSS 23,660 B (예산 250,000 B) |
| 검증 게이트 4종 | PASS | PASS |
| `cargo test --lib --locked` | 130 통과 · 9 ignored | 138 통과 · 0 실패 · 9 ignored, 경고 0 |

실물 확인(2026-11-10):

- **실제 Tauri 창 + 실제 계정**: 설치본은 그대로 두고, 식별자(`com.spectra.ai-usage-widget.verify`)와 데이터 폴더(`SPECTRA_DATA_DIR`)를 분리한 임시 개발 빌드를 띄워 WebView 화면을 직접 읽고 캡처함. 확인 뒤 임시 인스턴스·WebView 캐시·데이터 폴더는 정리함

| 화면 | 선택 | 확인된 표시 |
|---|---|---|
| 미니창 431×720 | Claude · `전체 모델` · 주간 | 선택 상자 `전체 모델`/`Fable`(폭 395px), `70% 주간 한도 남음`, 다른 모델군 `Fable 주간 83%` |
| 미니창 | `Fable` · 주간 | `83% 주간 한도 남음 · 5일 12시간 후 초기화`, 다른 모델군 `전체 모델 5시간 48% · 주간 70%` |
| 미니창 | `Fable` · 5시간 | `48% 5시간 한도 남음 · 3시간 3분 후 초기화 · 전체 모델 공용` (공용 표시 제거 전 기록) |
| 대시보드 1280×860 | `Fable` · 5시간 | 제목 `Fable · 5시간 한도`, `48%`, 추이 부제 `Claude · Fable · 5시간 한도 (전체 모델 공용)` (공용 표시 제거 전 기록) |
| 대시보드 구독 모델 | — | `전체 모델`: 5시간 48%·주간 70% / `Fable`: `5시간 한도 · 전체 모델 공용` 48%·주간 83% (공용 표시 제거 전 기록) |

- 모든 화면에서 가로 넘침 없음, 상태 `동기화됨`, 출처 `Claude 공식 사용량 조회`
- 명령줄 스냅샷(`--provider-snapshot claude`, 데이터 폴더 격리): 창 3건 — `rolling`, `weekly`, `weekly`(`fable`/`Fable`). 캐시 파일에 `scoped` 1건 기록
- 브라우저 데모(예시 수치)에서도 같은 구조 확인

설치본 교체(2026-11-10, 사용자 승인): `SPECTRA_0.3.4_x64-setup.exe`(2,877,027 B, SHA-256 `9FD0BD25…9B860E19`, 커밋 `8fc1c81` 기준 빌드)를 `/S`로 설치, 종료 코드 0. 실행 파일 6,349,824 → 6,405,120 B, `ui-prefs.json` 해시 동일. 설치본 미니창에서 `Fable` 선택 시 `5시간 한도 98% · 전체 모델 공용`, `주간 한도 75%` 표시 확인. 설치본 자동 갱신 13:23 → 13:25 → 13:27 성공, 수신 모델별 한도는 `Fable · 주간`뿐. 업데이터 서명(`.sig`)은 서명 키가 없어 만들지 않음. 버전 번호가 공개본과 같은 0.3.4라 앱 내 업데이트를 받으면 공개본으로 되돌아감.

공용 5시간 표시 제거(2026-11-10, 사용자 확인 "페이블은 주간한도만 있음"): 브라우저 데모 미니창에서 `Fable` 선택 시 `77% 주간 한도 남음`만 표시, 한도 탭 없음, 다른 모델군 `전체 모델 5시간 32% · 주간 49%`. `npm test` 94 통과, 빌드 JS 240,030 B, 게이트 4종 PASS. 이 변경은 설치본에 아직 반영되지 않음.

미실시: 작업표시줄 tooltip 실물 확인(단위 테스트로만 확인), push. 버전·릴리스 노트는 바꾸지 않음. 구현 커밋은 `5ab1703`.
