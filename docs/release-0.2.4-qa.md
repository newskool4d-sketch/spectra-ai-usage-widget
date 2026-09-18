# SPECTRA v0.2.4 검증

대상: `feat/taskbar-usage-strip`, Windows x64 NSIS, 작업 표시줄 스트립 2분 주기 갱신(A안, [설계 문서](./superpowers/specs/2026-09-18-strip-periodic-refresh.md)).

## 코드 검증 결과 (2026-09-18)

| 검사 | 결과 |
|---|---|
| `npm test` | 50개 통과(기준 48 + 신규 2) |
| `npm run build` | 통과 |
| `npm run verify:release` | `Release preflight PASS: v0.2.4` |
| `npm run verify:tokens` · `npm run verify:baseline` | 통과 |
| `npm run verify:memory` | **FAIL — 기존 문제.** 기준 커밋 `a87d1a5`에서 이미 JS 번들 244,894 B > 240,000 B였고 `setInterval` 패턴은 `dd94c17`(0.2.2)에서 추가됐다. 이번 변경은 +650 B. [memory-budget.md](./performance/memory-budget.md) 2026-09-18 절 참조 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked` | 83개 통과(기준 70 + 신규 13), GUI 8개 별도 실행 대상 |
| 위 명령에 `--features native-oauth` 추가 | 85개 통과 |
| `cargo check --lib --locked` 기본·`native-oauth` | 경고 0 |
| 릴리스 빌드 기본(`tauri build --no-bundle`)·`--features native-oauth` | 경고 0 |
| `git diff --check` | 통과 |
| `git diff -- src-tauri/Cargo.lock package-lock.json`(기준 커밋 대비, 버전 올림 전) | 변경 없음(신규 의존성 0) |

신규 테스트는 순수 스케줄러(`usage_refresh.rs`) 10개, 스냅샷 실패 코드 1개, 자격 증명 stamp 1개, 스트립 툴팁 라벨 1개, 프런트 라벨 2개다.

## 로컬 패키지 및 실물 검증 (버전 올림 전, 같은 코드)

- 검증 코드 커밋: `7c357d4`(구현 `01677fa`…`7917c88`, 문서 `12fc47d`·`a1363ad`·`7c357d4`). 태그 `v0.2.4`(`62c99ea`)는 여기에 버전 올림과 릴리즈 노트만 더한 것이다.
- `node node_modules/@tauri-apps/cli/tauri.js build --no-bundle -- --offline --locked`: 통과(실행 파일 6,313,984 B).
- `SPECTRA_TIMING_LOG=1`로 실행한 실행 파일의 `standby-timing.log`: `usage_refresh:claude:ok` 22:04:59(시작 +122초)·22:07:01(+123초), `usage_refresh:codex:ok` 22:05:01·22:07:03(+122초). 한 tick 안에서 Claude → Codex 순차 실행, 조회 소요 Claude 843~966 ms·Codex 1.5~1.9초.
- 같은 시각에 `claude-usage.json`의 `captured_at`이 갱신됐고, 작업 표시줄 스트립 값이 사용자 조작 없이 48% → 45%로 바뀌었다(Win32 창 열거로 `SpectraTaskbarStrip` 표시 유지 확인).
- 사용자가 미니 창을 열어 창의 값이 스트립과 같고 2분 뒤 스스로 바뀌는 것을 육안 확인했다(2026-09-18).
- 로컬 NSIS(버전 0.2.3 표기, 같은 코드): `SPECTRA_0.2.3_x64-setup.exe` 2,842,485 bytes, SHA-256 `55664A3BAB6F4688E2A6A47938D0E2A0A8209B544ECCD5F2479CEA841F080FC2`. 이 세션에는 updater 서명 키를 두지 않아 `.sig`가 없으며, 로컬 `/S` 설치(종료 코드 0) 후 설치본에서도 환경변수 없이 캐시 갱신 22:19:11 → 22:21:11 → 22:23:16 → 22:25:19(+120~125초)를 확인했다.
- Windows Authenticode: `NotSigned`. Tauri 서명 성공을 Windows 게시자 인증으로 표현하지 않는다.

## 공개 배포 검증 (2026-09-18 22:41~23:00 KST)

- 태그 `v0.2.4`(주석 태그, 커밋 `62c99ea`)를 푸시해 [GitHub Actions 실행 35351718923](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/35351718923)을 시작했다. `Verify release version`, 서명 키 설정 확인, 프런트 테스트, Rust 기본 테스트, NSIS 초안 빌드, 초안 자산 검증, 체크섬 업로드, 공개 전환이 모두 통과했다. 전체 실행 시간은 10분 54초다. 주석 1건은 `actions/checkout@v4`·`actions/setup-node@v4`의 Node.js 20 지원 종료 안내이며 결과에 영향이 없다.
- [공개 v0.2.4](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.4): `draft=false`, `prerelease=false`, 공개 시각 `2026-09-18 22:52:34 KST`, `releases/latest`가 `v0.2.4`를 가리킨다.
- 공개 자산 4개: `SPECTRA_0.2.4_x64-setup.exe`(2,836,431 bytes, asset 572727000), `.exe.sig`(416 bytes), `latest.json`(1,382 bytes), `SHA256SUMS.txt`(270 bytes).
- `gh release download`로 내려받은 자산에 `node scripts/verify-release.mjs <dir>`를 실행해 manifest 버전·설치 파일 URL(asset ID 결합)·내장 서명·업로드 digest 대조를 통과했고, 재생성한 체크섬이 공개 `SHA256SUMS.txt`와 바이트 단위로 같다.
- 공개 설치 파일 SHA-256: `0F2F1166E189A46D3368D977B1DFB1933C9817D9360830CFD0EC10BC72A27E49`.
- 인증 헤더 없이 `releases/latest/download/latest.json` 조회 HTTP 200, 버전 `0.2.4`, `pub_date 2026-09-18T13:52:17Z`. manifest의 설치 파일 URL을 updater와 같은 `Accept: application/octet-stream`으로 비인증 다운로드해 위 해시와 일치함을 확인했다.
- 설정 공개키(minisign 키 ID `2b2190fae1b3f640`)로 `.exe.sig`를 직접 검증(Node 표준 `crypto`의 Ed25519 + BLAKE2b-512 prehash): 파일 서명·전역 서명 모두 유효, 키 ID 일치. 즉 Actions 서명 키는 앱에 내장된 공개키와 같은 쌍이다.

## 최종 판정: PASS

| 요구사항 | 상태 | 근거 |
|---|---|---|
| 작업 표시줄 스트립 2분 주기 갱신 | PASS | 스케줄러 단위 테스트, 타이밍 로그 간격 122~123초, 설치본 캐시 갱신 120~125초 |
| Claude 토큰 갱신 즉시 반영·만료 보류·백오프 | PASS | 단위 테스트(자격 증명 stamp·보류·백오프), 라벨 테스트 Rust·TS |
| 창·스트립 값 일치 | PASS | `provider-usage-updated` 이벤트 경로, 사용자 육안 확인 |
| 서명된 공개 릴리즈·updater manifest | PASS | Actions 전 단계 통과, 서명·digest·체크섬·비인증 다운로드 검증 |
| 앱 내 업그레이드(0.2.3 → 0.2.4) | 미실시 | 설치본은 로컬 0.2.3 빌드이며 앱 내 업데이트는 사용자 승인 후 진행 |

## 검증 경계

- 공개 설치 파일은 GitHub Actions에서 빌드·서명되며 로컬 빌드와 해시가 다르다. 공개본은 별도로 내려받아 서명·manifest·digest를 대조한다.
- `.exe.sig`는 Tauri updater 검증용이다. Windows Authenticode 인증서는 별개이며 현재 제공하지 않는다.
- standby-strip 30분 메모리 재측정은 이번 검증 범위에 포함하지 않았다(2026-09-12 실측 33.6 MB는 주기 갱신 이전 값).
