# SPECTRA 정기 점검 보고 — 2026-10-29

- 대상: `spectra-ai-usage-widget` v0.2.8, 커밋 `5557be3`(`origin/main` 동일), Windows 11 설치본 `%LOCALAPPDATA%\SPECTRA`
- 성격: 읽기 전용 점검. 코드·문서 수정 없음(개선은 별도 진행)
- 도구: Node 24.14.1 · npm 11.11.0 · cargo/rustc 1.96.0 · PowerShell 7.6.9
- 점검 시각: 15:08~15:27 KST

## 1. 종합 판정

| 영역 | 판정 | 요지 |
|---|---|---|
| 저장소·브랜치 | 양호 | 작업 트리 clean, 원격과 동일. 병합 완료 지역 브랜치 5개·stash 2건 잔존 |
| 빌드·테스트 | 양호 | `npm test` 78/78, `cargo test --lib --locked` 104 통과·9 ignored, 컴파일 경고 0 |
| 검증 게이트 | 1건 FAIL(기존) | baseline·tokens·release PASS, `verify:memory` FAIL(a87d1a5부터 동일, QA 문서에 기록) |
| 의존성·보안 | 주의 | 런타임 npm 취약점 0, 개발 의존성 high 2(vite·source-map-js). Rust 취약점 점검 도구 없음. 저장소 Dependabot·secret scanning 비활성 |
| 설치본·실행 | 양호 | 0.2.8 로컬 빌드 실행 중(08:19 시작), 스트립 창 표시·작업표시줄 소유 정상 |
| 작업표시줄 유지 모듈 | **결함** | 유지 모듈 프로세스 0개. 로그온 작업 결과 0xC000013A(외부 종료), 재시작 기록 없음 |
| 공급자 연결 | 양호 | Codex·Claude 모두 connected, 격리 폴더 진단 각 3.6초·0.7초 |
| 자동 갱신 주기 | 확인 필요 | Claude 캐시 기록 간격 246~317초 관측(설계 120초). 타이밍 로그 꺼져 있어 원인 미확정 |
| 공개 릴리스·CI | 양호 | v0.2.8 공개·`latest.json` 비인증 조회 정상, 최근 발행 3회 모두 성공. CI 액션 버전 구형 |
| 문서 정합 | 드리프트 4건 | README·PLAN·memory-budget의 서술이 현재 구현과 불일치 |

## 2. 발견 사항(우선순위)

| # | 구분 | 내용 | 근거 | 조치 후보(별도) |
|---|---|---|---|---|
| F1 | 결함 | `SPECTRA Taskbar Keeper` 작업이 Ready 상태, 마지막 결과 `0xC000013A`(콘솔 제어 종료). 유지 모듈 pwsh 프로세스 0개. `startup.log` 마지막 기록 08:19:18 `FROZEN_KEEPER_STARTED`, 이후 재시작 기록 없음 | 작업 스케줄러 조회, Win32_Process 명령줄, `taskbar-keeper\startup.log` | 종료 원인 기록(launcher `finally` 로그), 작업 스케줄러 이력 활성화, 재시작 조건 재검토 |
| F2 | 위험 | 공개 배포 `install-taskbar-keeper.ps1`이 앱 실행 파일 해시를 `A67DC7BA…`로 고정. 이 값은 **이 PC의 로컬 빌드** 해시(10-03 15:26 빌드). 공개 CI 설치 파일(2,839,202 B)은 로컬 설치 파일(2,845,762 B)과 다른 빌드이므로 공개 설치본 사용자는 `verified SPECTRA v0.2.8 executable` 오류로 설치 실패 가능 | 설치 스크립트 6행, `docs/release-0.2.8-qa.md` 공개 배포 검증 절 | 공개 설치본 exe 해시 확인 후 허용 해시 추가 또는 해시 고정 방식 변경. 다운로드·추출 검증은 미실시 |
| F3 | 확인 필요 | Claude 캐시 `claude-usage.json` 기록 시각 15:12:35 → 15:17:52 → 15:22:04 → 15:26:10(간격 317·252·246초). 성공 조회마다 무조건 기록하는 코드이므로 성공 조회가 약 4분 간격. 설계는 120초. 앞 두 구간은 제 Rust 컴파일(CPU 100%)과 겹침, 마지막 구간은 CPU 0%에서도 246초 | `provider_usage.rs` 1064행 쓰기 경로, `usage_refresh.rs` 상수 | `SPECTRA_TIMING_LOG=1`로 1세션 실측 후 판정 |
| F4 | 기존 FAIL | `npm run verify:memory` FAIL: JS 번들 247,293 B > 240,000 B, `setInterval` 패턴 1건 | 게이트 출력, QA 문서 | 예산 재설정 또는 15초 타이머 구조 변경(기존 보류 항목) |
| F5 | 보안(개발) | `npm audit`: 런타임 0건, 개발 의존성 high 2건 — `vite` 8.2.1(복수 권고, Windows launch-editor 포함), `source-map-js`(DoS). 배포 산출물에는 포함되지 않음 | `npm audit` | `vite` 8.3.5 등 개발 의존성 상향 |
| F6 | 보안(저장소) | 공개 저장소에 Dependabot alerts·secret scanning·push protection 모두 비활성. Rust용 `cargo audit` 미설치로 RUSTSEC 점검 미실시 | `gh api repos/...` security_and_analysis | 저장소 설정에서 활성화(공개 저장소 무료), `cargo audit` 도입은 설치 승인 필요 |
| F7 | CI | `actions/checkout@v4`·`actions/setup-node@v4` 사용(최신 v7.0.1·v7.1.0). 10-03 실행에 Node 20 지원 종료 안내 1건. `tauri-action`은 SHA 고정 = `action-v1.0.0`(최신). PR·push 시 테스트 CI 없음(태그 발행 때만) | `.github/workflows/publish-tauri.yml`, GitHub API | 액션 메이저 상향, PR 검증 워크플로 추가 검토 |
| F8 | 문서 드리프트 | ① README 116행 "아직 다루지 않는 것"에 `자동 업데이트` 잔존(v0.2.2부터 구현) ② README 95행·`memory-budget.md` 10행 "`claude auth status`로 확인" — v0.2.8부터 실패·401/403 때만 실행 ③ `memory-budget.md` 35행 "thin LTO" vs `Cargo.toml` `lto = "fat"`(같은 문서 86행은 fat 채택 기록) ④ PLAN.md 18행 "코드 서명·자동 업데이트" 미체크 — 업데이터는 완료, 코드 서명만 미완 | 각 파일 행 번호 | 문구 정정 |
| F9 | 의존성 중복 | `reqwest` 0.12.28(직접) + 0.13.4(`tauri-plugin-updater` 경유) 두 HTTP 스택 공존. `windows-sys` 5개 버전은 전이 의존(통상적) | `cargo tree --duplicates` | 직접 의존을 0.13으로 맞춰 중복 제거 검토(실행 파일 크기) |
| F10 | 저장소 정리 | 병합 완료·0 ahead 지역 브랜치 5개(`prototype/a-b-c-baseline`·`stage2/design-tokens`·`stage3-react-product` 08-15, `worktree-quota-board-redesign` 08-16, `feat/taskbar-usage-strip` 09-18). stash 2건(`stash@{0}` 미추적 `.claude/launch.json`만, `stash@{1}` 08-16 `main.rs` +24행 대체됨) | `git branch --merged`, `git stash show` | 삭제는 명시 승인 후 |

## 3. 상세 결과

### 3.1 저장소

| 항목 | 결과 |
|---|---|
| HEAD / origin/main | `5557be3` / `5557be3` (동일), 작업 트리 clean |
| 버전 문자열 | package.json · package-lock.json · Cargo.toml · Cargo.lock · tauri.conf.json 모두 0.2.8 |
| 태그 | v0.2.1 ~ v0.2.8, 릴리스 노트 `docs/releases/` v0.2.2 ~ v0.2.8 |
| 비밀 패턴 스캔 | 추적 파일 0건(API 키·개인키·PAT 패턴) |
| 큰 추적 파일 | Pretendard 서브셋 woff2 3개 각 약 268 KB, 기준선 PNG 2개 각 약 205 KB, `src-tauri/gen/schemas` JSON 2개 각 126 KB(생성물 추적) |
| TODO/FIXME | 0건 |

### 3.2 빌드·테스트·게이트

| 검사 | 결과 |
|---|---|
| `npm ci` | 29 패키지, 잠금 파일 기준 |
| `npm test` | 78 passed / 0 failed (9 suites) |
| `npm run build` | tsc·Vite PASS. `index-*.js` 247,293 B, CSS 33,360 B, dist 1,088,103 B |
| `npm run verify:baseline` | PASS (A+C, assets 3, Figma 의존 0) |
| `npm run verify:tokens` | PASS (토큰 129) |
| `npm run verify:release` | PASS v0.2.8 |
| `npm run verify:memory` | **FAIL** — F4 |
| `cargo test --lib --locked` | 104 passed / 0 failed / 9 ignored (CI 동일 구성, v0.2.8 QA와 같음) |
| `cargo check --locked --all-targets` | 경고 0 |

### 3.3 의존성

npm 상향 가능(동작 변경 없이 잠금 파일만 갱신되는 범위 아님 — 적용 시 재검증 필요):

| 패키지 | 현재 | 최신 |
|---|---|---|
| @tauri-apps/cli | 2.11.4 | 2.12.4 |
| @tauri-apps/plugin-process | 2.3.1 | 2.4.0 |
| @tauri-apps/plugin-updater | 2.11.0 | 2.14.1 |
| @types/react / @types/react-dom | 19.2.18 / 19.2.4 | 19.3.0 / 19.3.0 |
| @vitejs/plugin-react | 6.0.5 | 6.1.2 |
| react / react-dom | 19.2.8 | 19.3.1 |
| vite | 8.2.1 | 8.3.5 |

Rust: `tauri` 2.11.5(2.x 최신 안정, 3.0은 alpha), `tauri-plugin-updater` 2.11.0, `reqwest` 0.12.28, `rustls` 0.23.40, `windows-sys` 0.61.2(직접). `cargo audit`·`cargo outdated` 미설치.

### 3.4 설치본·실행 상태

| 항목 | 결과 |
|---|---|
| 실행 파일 | ProductVersion 0.2.8, 6,317,568 B, 2026-10-03 15:26 빌드, SHA-256 `A67DC7BA…`(로컬 빌드, 유지 모듈 README 고정값과 일치), Authenticode NotSigned |
| 프로세스 | `spectra-native` PID 20512, 08:19:17 시작(로그온 08:18:46 직후 유지 모듈이 실행) |
| 메모리(1회 표본, 창 숨김·스트립 ON) | 호스트 11.6 MB + WebView2 6개 = 7 프로세스, 작업 집합 합계 99.7 MB |
| 설정 `ui-prefs.json` | theme dark · solid true · standby false · strip true (10-10 22:27) |
| 브리지 설정 | `claude-statusline-bridge.json` previousCommand/previousStatusLine null (09-16) |
| 스트립 창 | hwnd 67160 `SpectraTaskbarStrip` visible=True, owner 131352 = `Shell_TrayWnd`(유지 모듈 로그의 tray 값과 일치), 작업표시줄 영역 위 |
| 타이밍 로그 | `standby-timing.log` 마지막 기록 10-03(환경변수 없이 실행 중이라 비활성) |
| 충돌 이력 | Application 로그 30일 내 spectra 관련 오류 0건 |

### 3.5 작업표시줄 유지 모듈

| 항목 | 결과 |
|---|---|
| 예약 작업 | `\SPECTRA Taskbar Keeper` State=Ready, LastRun 08:18:46, LastResult 0xC000013A, 로그온 트리거 enabled, 배터리 제한 없음, 시간 제한 없음, 중복 IgnoreNew |
| 프로세스 | `start-spectra-taskbar-keeper.ps1`/`keep-spectra-taskbar.ps1` 실행 중인 pwsh 0개 |
| `startup.log` | 10-21~10-29 로그온마다 `SPECTRA_STARTED` → `FROZEN_KEEPER_STARTED` 2행씩. 10-29 08:19:17/18 이후 기록 없음(재시작·`EXISTING_*` 행 없음) |
| `spectra-taskbar-keeper.log` | 마지막 10-29 08:20:13 `visible=True`. 10-24 11:40 이후 `owner=0` 반복 구간 존재(소유 재설정 실패 이력) |
| 설치된 모듈 해시 | keep `FD767139…`(원본), launcher `FCED0A9F…` — QA 문서대로 공개 사본(`03CC3025…`, `43D10F9C…)으로 미교체 |
| 작업 스케줄러 이력 | 비활성(이벤트 없음) → 종료 시각·재시도 여부 판정 불가 |

### 3.6 공급자 연결(격리 데이터 폴더 `SPECTRA_DATA_DIR`로 실행)

| 공급자 | 상태 | 요금제 | 창 | 소요 |
|---|---|---|---|---|
| Codex | connected, chatgpt | prolite | 주간 잔여 95% | 3,608 ms |
| Claude | connected, claude.ai | max | 5h 잔여 91% · 7d 잔여 63% | 667 ms |

- CLI 진단은 데이터 폴더에 `claude-usage.json`을 기록하므로 격리 실행이 맞았음(프로덕션 캐시 미변경).

### 3.7 공개 릴리스·CI

| 항목 | 결과 |
|---|---|
| 최신 릴리스 | v0.2.8, 2026-10-03 06:52Z 공개, draft/prerelease 아님 |
| 자산 | setup.exe 2,839,202 B(다운로드 7) · .sig 416 · latest.json 1,382(다운로드 27) · SHA256SUMS 270 · keeper zip 5,538(다운로드 1) · zip.sha256 99 |
| `latest.json` | 비인증 조회 정상, version 0.2.8, windows-x86_64·windows-x86_64-nsis |
| 발행 워크플로 | v0.2.6·v0.2.7·v0.2.8 3회 모두 success(9분 53초~10분 40초) |
| 열린 PR/이슈 | 0 / 0, 마지막 push 10-10 |
| `verify-release.mjs` | keeper zip·.sha256 자산은 검증 범위 밖(설치 파일·서명·manifest만) |

## 4. 미실시·한계

- 재부팅 뒤 유지 모듈 자동 실행, 다른 PC 새 설치: 미검증(유지 모듈 README도 동일하게 명시)
- 공개 설치본 내부 exe 해시: 설치 파일 다운로드·추출을 하지 않아 미확인(F2는 크기 차이에 근거한 추정)
- RUSTSEC 취약점: 도구 미설치로 미실시
- 30분 메모리 측정, 대시보드·트레이 시나리오: 미실시(1회 표본만)
- Claude 갱신 간격 원인: 타이밍 로그 비활성으로 판정 불가(F3)
- 이번 점검으로 생성된 파일: 워크트리 `node_modules/`·`dist/`·`src-tauri/target/`·`src-tauri/icons/`(모두 .gitignore 대상), 스크래치패드 진단 폴더. 저장소 추적 파일 변경 없음
