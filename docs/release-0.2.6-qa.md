# SPECTRA v0.2.6 검증

대상: `main`, 태그 `v0.2.6`, Windows x64 NSIS. Codex 응답에 5시간 창이 있으면(ChatGPT Plus 등) 작업표시줄에 `5h NN% · 7d NN%`를 표시하고, 주간 창만 있으면(ChatGPT Pro 등) 기존 단일 값을 유지한다.

## 배포 전 검증 (2026-09-24)

| 검사 | 결과 |
|---|---|
| 신규 테스트 red 확인 | `codex_with_a_short_window_shows_both_periods_like_claude` 구현 전 FAIL(세그먼트 3 ≠ 4) |
| `cargo test --lib` | 87 passed, 8 ignored |
| `cargo test --lib --features native-oauth` | 89 passed, 8 ignored |
| `taskbar_window -- --ignored --test-threads=1` | Windows GDI 8개 PASS |
| `cargo check` 경고 | 0 |
| `npm test` | 50 passed |
| `npm run verify:release` | `Release preflight PASS: v0.2.6` |
| 버전 정합성 | package.json·package-lock.json·Cargo.toml·Cargo.lock·tauri.conf.json 모두 0.2.6 |
| 신규 의존성 | 없음 |

## 공개 배포 검증 (2026-09-24 KST)

- 커밋: 기능 `e9fe95f`, 릴리스 `c18ba38`, 주석 태그 `v0.2.6`.
- [GitHub Actions 35943465070](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/35943465070): 10:33:42→10:43:35 KST(9분 53초), 전 단계 PASS.
- [공개 v0.2.6](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.6): `draft=false`, `prerelease=false`, 공개 10:43:29 KST, `releases/latest` = v0.2.6.
- 공개 자산: 설치 파일 2,837,471 bytes(asset 584943376), updater 서명 416 bytes, `latest.json` 1,382 bytes, `SHA256SUMS.txt` 270 bytes.
- 공개 설치 파일 SHA-256: `C0627BBE9E3AF20D2E4637DF9B9AFE8914E27A64CD341BE4C4A1D98118FEF541`.
- 다운로드한 공개 자산에 `scripts/verify-release.mjs <dir>` PASS, `sha256sum -c SHA256SUMS.txt` 3개 모두 OK.
- 비인증 `releases/latest/download/latest.json`: 버전 `0.2.6`, Windows 대상 URL이 asset 584943376과 일치.

## 실제 앱 내 업그레이드 (10:56 KST)

- 꺼져 있던 설치본 0.2.5(6,308,352 bytes)를 10:56:27 실행 → `SPECTRA 새 버전 0.2.6` 안내에서 사용자가 `지금 설치` → updater가 공개 설치 파일로 설치 후 자동 재시작(PID 30128, 10:56:34).
- 설치 경로 `%LOCALAPPDATA%\SPECTRA\spectra-native.exe`: ProductVersion `0.2.6`, 6,308,864 bytes, 수정 시각 10:42:54 KST(CI 빌드 시각), SHA-256 `C7A0846801B63C8F074394D521BAF309B44DA7E30C8F76FFCF12F3F4517D4B8F`.
- 설치 전후 SHA-256 일치: `ui-prefs.json` `B1B2A9C7…520E6BDB`, `claude-statusline-bridge.json` `A4785440…5B4DF318`.
- 재시작 2초 뒤 `claude-usage.json` 갱신(10:56:36) — 새 버전의 사용량 조회 동작 확인.

## 판정

| 요구사항 | 상태 | 근거 |
|---|---|---|
| Codex 5시간 창 있을 때 5h·7d 고정 표시 | PASS | 단위 테스트(connected·stale × 두 창/5시간만) |
| 주간 창만 있는 요금제의 단일 값 유지 | PASS | 단위 테스트 + 기존 회귀 테스트 |
| 서명된 공개 릴리스와 updater manifest | PASS | Actions 전 단계, 공개 자산·digest·체크섬 검증 |
| 실제 앱 내 업그레이드 0.2.5 → 0.2.6 | PASS | 앱 내 설치, 새 버전 파일, 자동 재시작 |
| 사용자 설정 보존 | PASS | 두 설정 파일의 설치 전후 해시 일치 |
| Plus 계정 실물 표시 | 미검증 | 검증자 계정이 Pro(주간 창만 제공) — 사용자 제보로 확인 예정 |

전체 판정: **PARTIAL** — 구현·공개 배포·앱 내 업그레이드·설정 보존 PASS, Plus 계정 실물 표시만 미검증.

## 검증 경계

- Codex 4칸 스트립(`Codex 5h·7d` + `Claude 5h·7d`)의 GDI 렌더링은 전용 테스트가 없다. 폭은 측정 기반이라 칸 수와 무관하게 계산되지만 실물 화면은 확인하지 않았다.
- Tauri updater 서명은 Windows Authenticode 게시자 인증과 별개다(`NotSigned`).
