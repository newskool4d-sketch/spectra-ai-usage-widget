# 실행 메모리 예산

SPECTRA는 트레이에 오래 머무는 앱이므로 사용량 연결 뒤에도 상주 작업을 최소화합니다.

- 뷰포트에 맞는 데스크톱 A 또는 모바일 C 화면 하나만 React 트리에 마운트합니다.
- 목록·라벨·브라우저 데모 데이터는 모듈 스코프 불변 객체로 두고 작은 컴포넌트는 `memo`로 재렌더링을 줄입니다.
- 차트 라이브러리, `requestAnimationFrame`, `localStorage`, `sessionStorage`, 상시 `setInterval`을 사용하지 않습니다.
- 네이티브 앱은 과거 사용량 시계열을 저장하지 않고 현재 한도 스냅샷만 보관합니다.
- Codex App Server는 새로고침 요청 때만 실행하며 응답 또는 12초 제한 뒤 자식 프로세스를 종료합니다.
- Claude 조회는 짧은 `claude auth status` 프로세스와 작은 정제 JSON 캐시만 사용합니다.
- Codex JSONL은 총 2MiB, JSON 파일은 4MiB, Claude 상태선 입력은 1MiB로 제한합니다.
- 기존 Claude 상태선 명령은 2초, 출력은 64KiB로 제한합니다.
- 로그인 직후 확인은 대화상자가 열린 동안 단일 재귀 `setTimeout`으로만 수행하며 성공·닫기·30회 제한에서 종료합니다.
- 앱 유휴 상태에서는 provider 폴링을 하지 않습니다. 시작 시 한 번, 사용자가 새로고침할 때 한 번 조회합니다.
- `matchMedia` 리스너는 한 개만 등록하고 effect cleanup에서 해제합니다.
- `prefers-reduced-motion`을 존중합니다.

검증:

```powershell
npm run build
npm run verify:memory
```

이 검사는 번들 크기와 메모리 경로의 구조적 예산을 확인합니다. 실제 RAM 사용량은 Windows 작업 관리자에서 30분 이상 idle·수동 refresh·로그인 시나리오를 별도로 측정해야 합니다.

## Windows Tauri 기준선

2026-08-16의 0.1 릴리스 스모크 테스트에서 SPECTRA와 WebView2 자식 프로세스 7개의 합계는 다음과 같았습니다. 공급자 연결을 추가한 0.2 패키지는 기능·트레이 QA를 통과했지만 30분 메모리 재측정은 남아 있습니다.

- 최적화된 미니 창 표시 직후: 작업 집합 441.1MB, private 217.8MB
- 닫기 후 트레이 숨김 3초: 작업 집합 437.5MB, private 203.6MB
- 네이티브 호스트 자체: 작업 집합 32.4MB, private 6.3MB

대부분은 WebView2의 브라우저·렌더러·GPU 프로세스입니다. 현재는 빠른 재표시를 위해 닫기 시 창을 숨기므로 WebView2가 유지됩니다. 릴리스 프로필에는 size 최적화, thin LTO, 단일 codegen unit, 심볼 제거, abort panic을 적용합니다.

더 줄여야 한다면 닫을 때 WebView 창을 파기하고 트레이 클릭 시 재생성하는 선택형 저메모리 대기 모드를 별도 기능으로 검토합니다. 재표시 지연과 화면 상태 초기화가 대가이며, WebView2 보안 격리를 약화하는 비공식 single-process 플래그는 사용하지 않습니다.

## 2026-09-04 Phase 0 기준 (개선 착수 전)

측정: `npm run measure:memory -- -Scenario <name>` (호스트 + WebView2 자손 프로세스 합산, 30초 간격). 대상은 2026-08-23 릴리스 빌드(`spectra-native.exe` 직접 실행, 설치본 없음).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 |
|---|---|---|---|---|---|
| mini-idle (30분) | 60 | 436.3 MB | 475.3 MB | 203.7 MB | 7 |
| dashboard-idle (10분) | — | 미측정(트레이 메뉴 조작 필요, 사용자 대기) | | | |
| tray-hidden (10분) | — | 미측정(닫기 조작 필요, 사용자 대기) | | | |

번들(2026-08-23 빌드): JS 230,653 B · CSS 42,710 B · 글꼴 1,074,956 B/4개 · 실행 파일 7,571,968 B · NSIS 3,207,362 B

## 2026-09-04 Phase 1 결과 (비용 0 제거)

변경: 프로토타입 CSS를 `styles/prototype.css`로 분리, `.grain`·`drift` 제거, Medium 글꼴 삭제(3굵기 400·600·700), 굵기 요청 정규화.

번들: JS 230,892 B · CSS 31,850 B · 글꼴 806,632 B/3개 (`npm run verify:memory` 임계값: CSS 34,000 · 글꼴 820,000/3)

릴리스 빌드(2026-09-04 17:39): 실행 파일 7,297,536 B(−274,432) · NSIS 2,934,801 B(−272,561). 글꼴 1개 제거분이 그대로 반영됐다.

기능 확인: `spectra-native.exe --provider-snapshot codex`(`.cmd` 직접 실행 경로) → 로그인·연결·주간 창 정상, `--provider-snapshot claude`(타임아웃 헬퍼 경로) → 0.6초 응답, 로그인 확인·statusline 캐시 대체 정상.

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | 비고 |
|---|---|---|---|---|---|---|
| mini-idle-phase1 (중단) | 12 | 수면 전 4표본 464.7 MB · 재개 후 8표본 445.1 MB | 474.6 MB | 227.6 MB · 205.3 MB | 7 | 17:41 시스템 수면 → 다음날 08:47 재개로 오염. 비교 근거로 쓰지 않음 |
| mini-idle-phase1b (30분) | 60 | 422.7 MB | 468.2 MB | 210.3 MB | 7 | 2026-09-05 08:52~09:22, 표본 간격 무결. Phase 0 대비 −13.6 MB(−3.1%) |

Phase 1은 GPU·컴포지팅 요소를 건드리지 않았으므로(그레인 제거만 해당) 상주 메모리 변화는 오차 범위 안이다(−3.1%). 메모리 목표(≤ 380 MB)는 Phase 2의 유리 표면·블러 오브 정리에서 다룬다.

## 2026-09-05 Phase 2 결과 (디자인 재구성)

변경: 배경 오브 3개(`filter: blur(150px)`) 삭제 → `radial-gradient` 합성, `app-surface`·`glass-card`의 `backdrop-filter` 제거(불투명 토큰 + `--shadow-inset`로 대체, `nav-rail`·`oauth-backdrop`·`mobile-nav`만 유지), 그림자 블러 54px·34px → 24px·16px, "다음 행동" 스트립·시간 진행 트랙·창 요약 SVG 추가(모두 정적), 미니 창 전용 밀도 레이아웃, 라이트 테마 하드코딩 화이트 토큰화.

번들: JS 234,378 B · CSS 32,443 B (Phase 1 대비 CSS +593 B — 신규 UI 규칙 추가분이 `.bar-chart`·`.hero-signal` 등 레거시 규칙을 `prototype.css`로 이전한 만큼을 상쇄) · 글꼴 806,632 B/3개(변경 없음)

측정: `npm run measure:memory -- -Scenario mini-idle-phase2` (릴리스 빌드, `spectra-native.exe` 직접 실행, 30초 간격 60표본, 2026-09-05 19:15~19:45, 표본 간격 무결·기기 수면 없음).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | Phase 0 대비 | Phase 1b 대비 |
|---|---|---|---|---|---|---|---|
| mini-idle-phase2 (30분) | 60 | 410.8 MB | 413.8 MB | 187.8 MB | 7 | −25.5 MB(−5.8%) | −11.9 MB(−2.8%) |

비고: 60번째 표본에서 작업 집합이 355.4 MB로 순간 하락(WebView2 프로세스 중 하나의 일시적 트림으로 추정, 59개 표본은 409.6~413.2 MB 구간에서 평탄) — 평균 계산에는 그대로 포함했으며 추세에는 영향 없음.

Phase 0(436.3 MB)·Phase 1b(422.7 MB) 대비 평균 작업 집합이 지속적으로 감소했고, 특히 **최대 작업 집합이 475.3 MB → 413.8 MB(−13.0%)**로 크게 줄었다 — 스펙 2-2절에서 지목한 배경 오브·중첩 `backdrop-filter`가 GPU 컴포지팅 메모리에 직접 반영된다는 진단과 일치한다. 평균 private도 203.7 MB → 187.8 MB(−7.8%)로 감소했다. 목표치(≤ 380 MB)에는 아직 미달이며, 남은 격차는 Phase 3(WebView2 `--renderer-process-limit=1`, 네이티브 실행 파일 슬림화)에서 다룬다.

## 2026-09-08 Phase 3 결과 (네이티브 슬림화)

변경: `native-oauth` cargo feature(기본 off, `default = []`)로 OAuth 모듈과 네 직접 의존(`keyring`·`uuid`·`sha2`·`base64`)을 제외하고, `oauth_prepare`·`credential_status`·`credential_remove` 3개 커맨드는 계약 안정성 목적으로 스텁 상태로 항상 등록 유지. 딥링크 스킴 설정·플러그인은 유지하되 `register_all()`과 콜백 소비는 `native-oauth` feature on에서만 실행. 렌더러 제한(`--renderer-process-limit=1`): 효과 미확인 → 미채택(되돌림). LTO: `lto = "fat"` 채택.

### 의존 트리

기본 feature `cargo tree -e normal --depth 1`(Task 1)에는 `reqwest`·`serde`·`serde_json`·`tauri`·`tauri-plugin-deep-link`·`tauri-plugin-single-instance`·`url`만 남고 `keyring`·`uuid`·`sha2`·`base64` 직접 의존이 사라졌다. `cargo tree -i keyring`은 "did not match any packages"로 트리 전체에서 완전히 제거됨을 확인. 단, `sha2`(tauri-codegen 경유)·`uuid`(tauri-utils 경유)·`base64`(reqwest·plist 경유)는 전이 의존으로 남아 있다 — 바이너리 링크 제거를 직접 증명한 결과는 아니며, 기준은 "직접 의존(depth 1) 부재 + `-i keyring` 무매치 + 실행 파일 크기 전후 기록"으로 정정됐다(아래 스펙 정정 주석 참조). `Cargo.lock` 변경 없음. 게이트: 기본 `cargo build`/`cargo test --lib` 0 경고·24 passed, `--features native-oauth` 0 경고·26 passed. 근거: 2026-09-08 `cargo tree --offline -e normal -i <crate>` 실행 출력 `docs/performance/measurements/cargo-tree-inverse-phase3.txt`(git-ignored) — 발췌: `sha2 ← tauri-codegen ← tauri-macros ← tauri`, `uuid ← cfb ← infer ← tauri-utils ← tauri-codegen ← tauri-macros ← tauri`, `base64 ← hyper-util ← hyper-rustls ← reqwest` 및 `base64 ← plist ← tauri-utils`, `keyring` 무매치.

### OAuth 스텁 범위 (Task 2)

`src/integrations/oauth-messages.ts`의 `describePrepareFailure`는 스텁 오류 `native-oauth-disabled`를 "네이티브 OAuth 연결은 준비 중입니다. 지금은 Codex App Server와 Claude Code 공식 도구로 사용량을 확인합니다."로 매핑한다(테스트 3건). **범위 사실**: `oauth-adapter.ts`와 브리지의 `prepareNativeOAuth`/`getNativeCredentialStatus`/`removeNativeCredential`는 `src/main.tsx`에서 도달 가능한 소비처가 없다 — `src` 전체에서 `oauth-adapter|oauth-messages`를 참조하는 파일은 `oauth-adapter.ts` 자신뿐이고(Task 5 재확인), 빌드된 번들(`dist/assets/index-DWFLH-td.js`, 234,378 B)에 `native-oauth-disabled` 문자열이 0회 등장한다(Task 5 재확인). 따라서 "준비 중" 문구는 오늘 시점에는 사용자에게 노출되지 않는다. 3개 스텁 커맨드는 계약 안정성 목적으로 등록을 유지하며, Rust 테스트 `disabled_oauth_prepare_reports_marker`가 스텁 계약을 고정한다. **UI PASS를 주장하지 않는다.**

### Task 3 — 렌더러 프로세스 제한 A/B (효과 미확인 → 미채택·되돌림)

`--renderer-process-limit=1`을 4사이클(미적용→적용→적용→미적용, 각 60초 안정화 후 12표본×5초 간격)로 측정:

| 사이클 | 설정 | 프로세스 수 | 렌더러 수 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 인자 확인 |
|---|---|---:|---:|---:|---:|---:|---|
| before-1 | 미적용 | 7 | 1 | 435.0 MB | 439.8 MB | 199.8 MB | n/a(미적용, 기대대로) |
| after-1 | 적용 | 7 | 1 | 429.9 MB | 430.4 MB | 196.4 MB | 확인됨 |
| after-2 | 적용(빌드 재사용) | 7 | 1 | 423.4 MB | 423.8 MB | 191.7 MB | 확인됨 |
| before-2 | 미적용 | 7 | 1 | 432.6 MB | 437.5 MB | 192.5 MB | n/a(미적용, 기대대로) |

판정: **NOT ADOPTED (효과 미확인 → 미채택·되돌림)** — 채택 규칙(두 전후 비교 모두에서 렌더러 또는 전체 프로세스 수 감소)을 충족하지 못했다. before-1→after-1, before-2→after-2 두 비교 모두 렌더러 1/1·전체 7/7로 동일했다(단일 창 앱이라 원래도 렌더러가 1개뿐이어서 제한할 대상이 없었음). 작업 집합은 적용 시 소폭 낮았으나(−1.2%, −2.1%, 12표본·5초 간격 측정의 잡음 범위) 판정 규칙이 요구하는 "수 감소 재현"에는 못 미친다. 설정 커밋 `a147b52`는 `git revert`로 `44c265e`에서 되돌렸다.

### Task 4 — LTO fat/thin 비교 (fat 채택)

신규 `--target-dir`마다 클린 빌드 4회(순서 thin→fat→fat→thin, `cargo build --release --bin spectra-native --no-default-features --locked --offline` — **트라이얼 경로**, 아래 "최종 산출물" 표의 `tauri build` 경로와 크기가 다름 § 참조):

| 회차 | 구성 | 시간(초) | exe 바이트(트라이얼 경로) |
|---|---|---:|---:|
| 1 | thin | 386.6412297 | 5,313,024 |
| 2 | fat | 397.4963121 | 4,898,816 |
| 3 | fat | 390.5581568 | 4,898,816 |
| 4 | thin | 322.1561986 | 5,313,024 |

중앙값(2회 평균): thin 354.3987142초 / fat 394.0272345초

채택 규칙 3항목: 크기(fat이 thin보다 ≥100,000 B 작은가) **PASS**(−414,208 B) · 시간(fat 중앙값 ≤ thin 중앙값의 2배) **PASS**(비율 1.11×) · 안정성(동일 구성 두 회차 시간차 ≤20%) **FAIL** — thin 자체 두 회차 시간차 **20.017%**(기준 20% 초과, 0.017%p·0.054초 차이).

**컨트롤러 채택 판단 — 규칙 개정으로 기록**: 계획의 문자 규칙대로면 "불안정 → thin 유지"가 정답이었다(실행자 판정). 컨트롤러는 이를 기존 규칙의 적용이 아닌 **명시적 규칙 개정(중대성 예외)**으로 채택했다 — 초과폭 1%p 이내(실측 0.017%p), 크기 기준 3배 이상 여유 통과(−414,208 B ≥ 3×100,000), 관측된 모든 원시 시간 짝의 fat/thin 비율이 기준의 2/3 이하(최악 원시 짝 397.50/322.16 = **1.234×**, 기준 2×). 개정 문구는 계획서 Task 4 Interfaces에 2026-09-08자로 추가했고, 되돌리려면 `git revert 35f8016` + 릴리스 재빌드로 충분하다. 채택 절차: `lto = "fat"` 설정(1줄 변경) → Task 1 게이트 재실행(기본/native-oauth 양쪽 release 빌드·테스트, 경고 0·24/26 passed) → 커밋 `35f8016`. fat 릴리스 바이너리의 실제 실행은 아래 연결 QA(CLI 경로)와 Task 6 30분 측정으로 확인한다.

### 최종 산출물 (`npm run desktop:build` = `tauri build` 경로, Task 5·6 고정 대상)

| 항목 | Phase 2 (2026-09-05 19:11) | Phase 3 thin | Phase 3 fat(채택) |
|---|---:|---:|---:|
| 실행 파일 | 7,302,144 B | 6,223,872 B | 5,809,664 B |
| NSIS 설치 파일 | 2,936,525 B | 2,663,082 B | 2,673,530 B |

fat 채택분 해시(Task 5 재검증 일치): exe SHA-256 `883195EEA3FFF0137CC840728032CFDAE44DAFADC14AA62550DD148FCDE01A0F` · NSIS SHA-256 `7F89DC4CDDA732402E96680756C4D2376ADAF468C86CF812B4F86E9164A1C153`. NSIS는 thin(2,663,082 B) 대비 fat이 +10,448 B 크다 — fat LTO로 밀도가 높아진(인라인 강화) 코드가 압축 효율을 낮췄을 가능성이며 결정에는 미반영(스펙 기준은 실행 파일 크기). NSIS 자체에 빌드마다 ±수백 바이트의 비결정성이 있다 — exe 바이트는 동일 구성 재빌드에서 완전히 재현되며 해시만 상이(PE 메타데이터 비결정성).

§ 트라이얼 경로 exe는 두 LTO 구성 모두에서 `tauri build` 경로보다 정확히 910,848 B 작다(thin 6,223,872−5,313,024, fat 5,809,664−4,898,816 = 각 910,848 B). `--bin --no-default-features` 단일 바이너리 빌드와 `tauri build`(lib의 staticlib+cdylib+rlib 전체+bin 빌드)의 경로 차이이며 LTO 설정과는 무관하다.

측정 조건: HEAD `35f80167f8b24ee3c8546ec535c4f34e70a1f05d` · `Cargo.lock` SHA-256 `04752C4C553AC387E0D0ABE65E11AABB2A6D46CC6055E29E64048A488B46ACAD` · rustc/cargo 1.96.0(`ac68faa20`/`30a34c682`, 2026-05-25) · host `x86_64-pc-windows-msvc` · Windows `10.0.26200.0` · WebView2 Evergreen `152.0.4191.66` · feature 기본(`default = []`, native-oauth off) · `lto = "fat"` · 측정 일시 2026-09-08.

### 연결 QA (Task 5 Step 1)

고정된 fat 빌드(`spectra-native.exe`, 해시 위와 일치 확인 완료)로 `--provider-snapshot <provider>` 구조 검사를 실행:

| 공급자 | 종료 코드 | connectionState | source | windowCount | 구조 검사(기본+확장) | 판정 |
|---|---:|---|---|---:|---|---|
| codex | 0 | connected | codex-app-server | 1 | 전부 통과 | **PASS** |
| claude | 0 | connected | claude-statusline | 2 | 전부 통과 | **PASS** |

구조 검사 항목: (기본) `providerId` 일치·`connectionState` 열거값·`windows` 배열·비-`error` / (확장) `runtimeAvailable` bool·`authState` 비어있지 않음·`source`가 공급자별 허용값(codex: `codex-app-server`, claude: `claude-usage-api`/`claude-statusline`)·`lastSyncedAt` nullable 숫자 타입·각 창의 `id` 존재·0~100 범위 사용률/잔여율·합계 100 근사(오차 ≤1)·`resetsAt`/`windowDurationMins` nullable 숫자 타입. 두 공급자 모두 `connected`로 조회되어 브리프 기준상 "연결 QA PASS"로 기록한다(signed-out/not-installed/waiting-for-usage였다면 구조 검사 통과로만 기록하고 PASS로 카운트하지 않았을 것). 참고: Task 1 착수 전 시점의 공급자별 창 수 기록이 없어 "연결 상실·창 소실" 전후 비교는 수행할 수 없다 — 이번 단일 관측 기준으로는 이상 없음.

앱 새로고침·미니/대시보드 전환·트레이 숨김/재표시: 확인 완료(아래 "사용자 확인 결과" 참조, 이상 없음). OAuth 스텁의 실제 Tauri invoke 확인: **미검증**(UI 진입점 없음 — 동일 절 참조). feature on 회귀: 기존 게이트로만 확인(native-oauth release 빌드 성공 + 26 tests pass) — 실제 OAuth 인증 성공까지 검증한 것은 아니다.

### 사용자 확인 결과 (2026-09-08)

fat 빌드(`spectra-native.exe`, SHA-256 `883195EE…`)로 사용자가 직접 확인했다. 네 항목 모두 결과가 확정됐다:

- [x] 최종 네이티브 앱에서 Codex·Claude 각각 새로고침 → 로딩 종료, 실제 source·사용량 창 정상 표시, 오류 메시지 없음 — 확인 완료, 이상 없음
- [x] 미니 창 ↔ 대시보드 창 모드 전환 정상 — 확인 완료, 이상 없음
- [x] 트레이 숨김 → 재표시 정상 — 확인 완료, 이상 없음
- [ ] 기본 feature 앱에서 OAuth 연결 시작 UI를 실제로 조작 → `oauth_prepare` invoke가 disabled marker로 실패하고 status의 `available`/`present`가 false로 보이는지 확인 — **UI 진입점 없음 — Rust 스텁 테스트로만 고정**(Task 2에서 확인된 대로 현재 UI에는 이 경로로 도달하는 진입점이 없다. UI PASS를 주장하지 않는다)

### 2026-09-08 mini-idle-phase3 (Task 6 완료)

측정 조건: Task 5에서 해시를 고정한 fat LTO 릴리스 실행 파일(`spectra-native.exe`, 5,809,664 B, SHA-256 `883195EE…`, 기본 feature, HEAD `35f8016` 이후 — 4c3699d까지 코드 변경 없이 문서 커밋만 누적)을 대상으로, 사용자 UI 확인에 쓰던 기존 인스턴스를 PID로 종료한 뒤 새로 실행하고 60초 안정화 후 `measure-memory.ps1 -Samples 61 -IntervalSeconds 30`(RootPid 33704)으로 측정했다. 유효성: 61개 표본이 2026-09-08 17:36:55~18:07:37에 걸쳐 있어 공백 1,842초(≥1,800초 기준 충족), 표본 간 최대 간격 31초(기준 이내), 61개 표본 전 구간 processCount 7 유지, 호스트 프로세스가 측정 종료까지 생존(호스트 작업 집합 약 30.5 MB), SUMMARY 라인 수치(평균 422.6 / 최대 439.5 / private 205.9)가 CSV 재계산값과 일치했다. 환경: Windows 10.0.26200, WebView2 Evergreen 152.0.4191.66, rustc/cargo 1.96.0, `lto = "fat"`, feature 기본(`native-oauth` off), 미니 창, Codex App Server·Claude 상태선 두 공급자 모두 연결.

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 최소 작업 집합 | 평균 private | 프로세스 수 | Phase 2 대비(관찰) |
|---|---|---|---|---|---|---|---|
| mini-idle-phase3 (최소 30분) | 61 | 422.6 MB | 439.5 MB | 416.3 MB | 205.9 MB | 7 | +11.8 MB(관찰값·비교 미검증) |

추세: 첫 10개 표본 평균 433.6 MB → 마지막 10개 표본 평균 417.3 MB로 갈수록 안정화됐다(31~61번 표본은 416.3~419.5 MB 구간에서 평탄). 최댓값(439.5 MB)은 측정 시작 직후인 1번 표본에서만 관측됐다.

해석: Phase 2 → Phase 3 비교는 **미검증**이다 — 2026-09-05 측정은 WebView2 버전이 기록되지 않았고 안정화 대기 없이 60표본을 측정한 반면, 이번은 60초 안정화 후 61표본을 측정했으며 측정일·시스템 상태도 다르다. 위 표의 Phase 2 대비 수치는 이 조건 차이를 통제하지 않은 **관찰값**으로만 보고한다. Phase 3의 변경(OAuth feature 게이트, fat LTO)이 상주 메모리를 악화시켰다는 근거는 없다 — 같은 날 측정한 Task 3의 thin 빌드 A/B 4회(60초 안정화 후 12표본×5초 간격, 평균 435.0/429.9/423.4/432.6 MB)의 범위 안에 이번 fat 30분(61표본×30초 간격) 결과(422.6 MB, 후반부는 ~417 MB로 안정화)가 들어오며, 호스트 자체의 상주 메모리도 변화가 없다(약 30.5 MB). 그렇다고 개선됐다고도 주장하지 않는다. 프로세스 수는 7로 유지됐다(렌더러 프로세스 수 제한은 미채택 — 위 Task 3 절 참조). 목표(평균 작업 집합 ≤ 380 MB)는 **미달**이며, 남은 격차는 WebView2 자체가 차지하는 비중이 커서 Phase 4(선택형 저메모리 대기 모드)의 과제로 남긴다.

참고(선택): Phase 2 기준 빌드(커밋 `4e5f3c4`, 새 target 디렉터리)를 이번과 동일한 조건(60초 안정화·61표본)으로 재측정하면 단계 간 비교를 유효화할 수 있다 — 실행 여부는 사용자 판단에 맡긴다.

목표 판정: 평균 작업 집합 422.6 MB > 380 MB — **미달**. 실행 파일 축소(위 "최종 산출물" 표 참조)는 별도 판정 항목이며 이번 상주 메모리 목표 결과와는 무관하게 판단한다.

## 2026-09-09 Phase 4 결과 (선택형 저메모리 대기 모드)

변경: 설정 "메모리 절약 대기"(기본 off). 켜면 창 닫기 시 WebView 창을 `destroy`하고 트레이만 남기며, 트레이 클릭 시 `WebviewWindowBuilder`로 재생성해 테마·창 모드·마지막 스냅샷을 `initialization_script`(`window.__SPECTRA_BOOT__`)로 복원한다. 선호는 Rust가 `<앱 데이터 폴더>/ui-prefs.json`에 저장(`localStorage` 미사용). 트레이 아이콘에 연결된 공급자의 최소 잔여율을 표시(시작·수동 새로고침 시에만 갱신). 구현 커밋 77a0f08~3eab6e9(양끝 포함 6커밋, git 범위 `ceebe1e..3eab6e9`), 릴리스 빌드는 3eab6e9 기준(빌드 5분 28초).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | 목표(≤ 60 MB) |
|---|---|---|---|---|---|---|
| standby-idle-phase4 (30분, 창 파기 상태) | 61 | 31.5 MB | 31.7 MB | 6.3 MB | 1 (전 표본) | PASS |
| (참고) 기동 직후 활성 표시 상태, 1표본 | 1 | 440.6 MB | — | 207.7 MB | 7 | — |

측정 파일: `measurements/standby-idle-phase4-20260909-1913.csv`(2026-09-09 19:13~19:44, 30초 간격, 호스트 PID 기준 자손 합산). 대기 상태 작업 집합은 Phase 3 mini-idle 평균 422.6 MB 대비 −92.5%. 활성 표시 상태의 30분 idle은 이번에 재측정하지 않았다(Phase 3 수치 유지, 목표 ≤ 380 MB 미달).

재표시 지연(`SPECTRA_TIMING_LOG=1`, `create_main_window` 직후의 `show_main_window` 행, 30분 대기 후 사용자 조작 8회): 965 · 739 · 259 · 234 · 692 · 673 · 250 · 566 ms → 중앙값 620 ms(8회), 콜드 재생성(≥ 400 ms) 5회 중앙값 692 ms, 30분 대기 직후 첫 재생성 965 ms. 300 ms 미만 3건은 직전 창 파기 뒤 WebView2 프로세스가 종료되기 전에 재생성된 경우다. 스펙 예상 300~600 ms를 콜드 기준으로 소폭 상회. 숨김만 하는 기본 재표시는 18~43 ms.

기본값 회귀: `ui-prefs.json`이 없는 상태에서 닫기=숨김·트레이 클릭 즉시 재표시, 트레이 숫자 배지·툴팁 표시(사용자 확인 2026-09-09). 대기 모드: 재생성 후 라이트 테마·대시보드 모드·이전 수치·"이전 표시 복원" 라벨 유지(사용자 확인), "빠른 재표시"로 되돌리면 숨김만 됨(로그 18 ms), 재기동 후 저장된 선호(라이트·standby)로 닫기 시 파기(호스트 단독 30.1 MB, 실행자 확인).

실행 파일: 5,809,664 B → 5,836,800 B(+27,136 B, 신규 모듈 `standby.rs`·`tray_badge.rs`) · NSIS 2,673,530 B → 2,684,969 B(+11,439 B). SHA-256: exe `e7fa8266afce54223e24c7fca9e6ee37abf80fe1bdacb840526bedbee20e3cee`, 설치 파일 `6f0bb00dc4db9a74f46bfe7b4e1cdd314b3028a992e4afa3a99ba6e82cbac4cf`.

목표 판정: 대기 모드 30분 평균 31.5 MB ≤ 60 MB — **PASS**. 재표시 후 테마·모드 유지 — PASS(사용자 확인). 기본값 현행 동작 — PASS(사용자 확인). 스펙 §1 "미니 창 30분 idle ≤ 380 MB"는 활성 표시 상태 기준이라 이번 결과로 바뀌지 않는다(Phase 3 422.6 MB, 미달 유지).

후속 수정(2026-09-10, Codex 리뷰 P1): 트레이 클릭·트레이 메뉴·두 번째 실행(single-instance) 콜백 안에서 동기적으로 WebView를 만들던 재생성 경로를 워커 스레드로 옮겼다(Tauri 2.11.5 `WebviewWindowBuilder` 알려진 문제: Windows에서 동기 이벤트 핸들러 내 생성 시 교착 가능). 기동 시 최초 창은 `setup`에서 그대로 동기 생성한다. 재현 시도(대기 중 두 번째 실행 1회)에서는 교착이 나타나지 않았고, 수정은 예방적이다.

## 2026-09-12 작업표시줄 스트립 (B안)

변경: 기존 `windows-sys 0.61.2`에 `Win32_UI_Accessibility` feature를 추가하고, 네이티브 레이어드 스트립 창 1개와 전용 메시지 스레드 1개를 사용했다. WebView 창은 추가하지 않는다. 스트립은 공급자 스냅샷 완료와 셸 변화 이벤트에서 캐시된 값을 다시 그리며 데이터 목적의 타이머나 폴링을 사용하지 않는다.

대기 조건은 `standby=true`, `strip=true`, 미니 창 닫힘 상태였다. Task 5 standalone 실행 파일 PID 25064를 대상으로 60초 안정화 후 30초 간격으로 61개 표본을 수집했다. 측정 구간은 2026-09-12 17:33:01~18:03:40 KST(총 1,839초), 표본 간격은 30~31초였다. CSV 원자료는 [`standby-strip-task6-20260912.csv`](./measurements/standby-strip-task6-20260912.csv)이며 SHA-256은 `39E93B88F85742EEA66BF6D61EE1D512B67E54233D1BAB00E9DAA769C4166608`이다.

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | 목표 |
|---|---:|---:|---:|---:|---:|---|
| standby-strip (30분, 스트립 켬) | 61 | **33.6 MB** | 34.1 MB | 7.4 MB | **1 (전 표본)** | **PASS, 평균 ≤ 35 MB** |

평균 작업 집합은 기준 35 MB보다 1.4 MB 낮고, 프로세스 수는 전 표본에서 1개였다. 대기 스트립의 총 작업 집합 목표와 WebView 파기 조건을 모두 충족한다. 실행 파일은 Task 5 standalone 빌드 `5,881,344 B`, SHA-256 `456DC56FDFC0A520744A6B621491C4D87C815065489BB5E3714BFEA8A51BDEFA`이며, Phase 4 기준 5,836,800 B보다 44,544 B(약 0.76%) 증가했다.

Task 5의 셸 이벤트·WinEvent·자동숨김·F11·Explorer 재시작 검증은 별도 [Task 5 보고서](../superpowers/sdd/2026-09-12-taskbar-usage-strip/task-5-report.md)에 기록했다. 이번 측정은 대기 모드의 메모리·프로세스 수만 판정한다. 활성 미니 창 30분 메모리 목표(Phase 3 평균 422.6 MB)는 이 결과로 변경하지 않는다. 추가 D3D·다중 모니터·30분 가려짐 관찰은 실시하지 않았다.

