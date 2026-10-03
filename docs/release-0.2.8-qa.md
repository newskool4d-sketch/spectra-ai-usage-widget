# SPECTRA v0.2.8 검증

대상: `main`, 수정 커밋 `609f2e5`, Windows x64 NSIS.

## 원인 점검 (2026-10-03 KST)

- 증상: 로그인 상태에서 작업표시줄·창의 Claude 값이 `—`로 바뀌었다가 몇 분 뒤 돌아옴. 사용자 제보 "현재 실행 파일 클로드 사용량 안나타나는 중"(10:3x).
- 원인: 2분마다 `claude auth status` 실행(제한 5초) → 시간 초과 시 디스크 캐시를 읽지 않고 `error`·창 0개 스냅샷 반환 → `remember_snapshot`이 직전 정상값 교체 → 스트립·트레이 배지·창이 동시에 빈 값, 백오프 120→240초 동안 유지.
- 실측(설치본 v0.2.7, 10:15~10:33, CPU 100%): `claude auth status` 5회 5,545 · 4,519 · 4,160 · 5,536 · 3,549 ms. Claude 캐시 기록 누락 4회(10:21 · 10:25 · 10:30 · 10:32). 10:28:18 성공 뒤 다음 시도 10:30:18, 스트립 Claude 값이 `—`로 바뀐 시각 10:30:23(시도 +5초).
- 측정 오염: 10:19 수동 조회, 10:25 직전 `auth status` 5회 측정, PowerShell 감시 프로세스가 부하를 더함. 평상시 부하의 실패 빈도는 측정하지 않음.
- 저부하 대조: 11:51 CPU 30~84%에서 설치본 격리 실행 Claude 5회·Codex 3회 모두 성공(1.6~3.3초). 11:54~12:19 감시 297회(5초 간격): 스트립 상시 표시, 두 공급자 값 유지, 작업표시줄 위 z-order, Claude 13회 연속 2분 간격 성공.
- 10:22 무렵 스트립 창 전체 숨김 1회 관찰. 원인 미확정(두 공급자 동시 빈 값 또는 전체화면 숨김 조건). 설치본은 로그를 남기지 않아 판정 불가.

## 수정 검증: 같은 시나리오, 변경 전후

가짜 `claude` 명령과 격리 데이터 폴더(`SPECTRA_DATA_DIR`)로 실행. 설치본 캐시 미사용, 실제 사용량 API 호출은 사례 A의 수정 빌드 1회.

| 사례 | 조건 | v0.2.7 설치본 | 수정 빌드 |
|---|---|---|---|
| A | CLI 응답 6초, 실제 유효 자격 증명 | 5,248 ms `error`, 창 0 | 1,770 ms `connected`, 창 2, 요금제 max, CLI 미실행 |
| A2 | CLI 응답 17초, 자격 증명 없음 | 5,221 ms `error` | 15,193 ms `error`, 새 문구 |
| A3 | CLI 응답 6초, 자격 증명 없음 | 5,207 ms `error` | 6,435 ms `stale`, 캐시 창 2 |
| C | CLI 로그아웃 응답 | 452 ms `signed-out` | 329 ms `signed-out` |
| D | CLI 응답 6초, 만료 토큰(가짜 파일) | 5,196 ms `error` | 210 ms `stale`, 캐시 창 2, 로그인 갱신 안내, 요금제 pro(파일 값) |
| B | CLI 즉시 응답, 자격 증명 없음 | 318 ms `stale` | 400 ms `stale` |

- CLI 모드(`--provider-snapshot`)는 `refresh_provider`를 거치지 않음 → 직전 값 유지(`keep_last_good`)는 이 표로 검증되지 않음. 단위 테스트와 아래 실앱 검증으로 확인.

## v0.2.8 후보 검증 (2026-10-03 KST)

| 검사 | 결과 |
|---|---|
| `npm test` | 78 passed, 0 failed |
| `npm run build` | TypeScript 검사·Vite 빌드 PASS |
| `npm run verify:release` | Release preflight PASS: v0.2.8 |
| `cargo test --lib --locked`(CI 동일) | 104 passed, 0 failed, 9 ignored |
| `cargo test --lib --features native-oauth` | 106 passed, 0 failed |
| 전체 Rust `--include-ignored --test-threads=1` | 114 passed(라이브러리 113 + HTTP 통합 1), 0 failed, 0 ignored — 버전 올림 전 같은 코드 |
| `tauri build --no-bundle` | 경고 0, 실행 파일 6,317,568 B — 버전 올림 전 같은 코드 |
| `git diff --check` | PASS |
| `npm run verify:memory` | FAIL, 변경 전부터 동일: JS 247,058 B → 247,293 B(+235 B), 기존 15초 타이머의 `setInterval` 패턴 |
| 버전 정합성 | package.json · package-lock.json · Cargo.toml · Cargo.lock · tauri.conf.json 모두 0.2.8 |

## 로컬 설치 검증 (2026-10-03 KST)

| 항목 | 결과 |
|---|---|
| 설치 파일 | `SPECTRA_0.2.8_x64-setup.exe` 2,845,762 B, SHA-256 `B2CDCE5FA77D9918ACAC6EEE74ACFE332031763AC9EE27786A476441BF206A6E` |
| 빌드 | 커밋 `7b07baa`, `tauri build --bundles nsis`, 로컬에 서명 키가 없어 `createUpdaterArtifacts=false`로 업데이터 서명 생략, 경고 0 |
| 설치 | `/S`, 종료 코드 0, 15:27:06~15:27:11(5.4초), 실행 중이던 v0.2.7은 설치 프로그램이 종료 |
| 설치본 | ProductVersion 0.2.8, 실행 파일 6,317,568 B, HKCU 제거 항목 `SPECTRA` |
| 설정 보존 | `ui-prefs.json`·`claude-statusline-bridge.json` SHA-256 설치 전후 동일 |
| 재실행 | `explorer.exe` 경유, 프로세스 1개, 응답 정상 |

### 실앱 직전 값 유지 검증 (15:27~15:32)

설치본 0.2.8을 `SPECTRA_TIMING_LOG=1`과 Codex 래퍼로 실행. 래퍼는 `fail.flag`가 있으면 즉시 실패하고, 없으면 실제 Codex CLI로 전달.

| 시각 | 사건 | 스트립(물리 px) |
|---|---|---|
| 15:27:29 | 테스트 인스턴스 시작, 부팅 조회 | 15:27:39 253×33, 두 값 표시 |
| 15:27:57 | `fail.flag` 생성 | 253×33 |
| 15:29:31.262 | `usage_refresh:claude:ok` 487 ms, CLI 프로브 없음 | 253×33 |
| 15:29:31.382 | `usage_refresh:codex:failed` 116 ms | 15:29:34·15:29:39 253×33 유지 |
| 15:29:56 | `fail.flag` 삭제 | 253×33 |
| 15:31:31.822 | `usage_refresh:claude:ok` 428 ms | 253×33 |
| 15:31:33.445 | `usage_refresh:codex:ok` 1,620 ms, 백오프 120초 뒤 | 253×33 |
| 15:32:01 | 테스트 인스턴스 종료, `explorer.exe`로 평소 인스턴스 재실행 | — |

- 표본 52회(5초 간격) 모두 표시·253×33·작업표시줄 위. Codex 실패 직후에도 Codex 값 유지. 변경 전이라면 Codex 칸이 `—`로 바뀌어 폭이 줄어듦(평소 인스턴스 부팅 중 Codex 첫 조회 전 236×33 관찰로 확인).
- 실패 후 백오프·회복은 기존 스케줄러대로 동작(실패 120초 뒤 재시도 성공).
- 툴팁 문구(`Codex · 갱신 대기 (캐시)`)는 실앱에서 직접 읽지 않음. 단위 테스트로만 확인.

### 평소 인스턴스 관찰 (15:32~15:36)

- `explorer.exe`로 재실행한 설치본(환경 변수 변경 없음)을 5초 간격 54회 관찰: 스트립 상시 표시, 작업표시줄 위 z-order.
- Claude 캐시 기록 15:32:03(부팅) → 15:34:03 → 15:36:06, 약 2분 간격 연속 성공.
- 스트립 폭은 부팅 직후 1회만 236×33(Codex 첫 조회 완료 전 `—`), 15:32:10부터 253×33.

## 검증 경계

- 10:22 스트립 전체 숨김 원인 미확정. 평상시 부하의 실패 빈도 미측정.
- 초기화 경계 `—` 표시는 작업표시줄·트레이 배지·한도 보드(미니 창, 대시보드·모바일 개요)에 적용. 대시보드 서비스 목록·서비스 칩·집중 확인 카드·알림·추이 화면은 이어받은 수치를 그대로 표시. 트레이 배지는 다음 조회 때 반영.
- 이어받은 Codex 값의 창 표기는 `연결됨 · 데이터 갱신 필요`(기존 stale 문구), 작업표시줄 툴팁은 `Codex · 갱신 대기 (캐시)`.
- 로컬 0.2.8 설치 후 공개 0.2.8이 나와도 업데이터는 같은 버전으로 판단 → 이전 릴리스의 앱 내 업그레이드 검증은 이번에 반복 불가. 공개 설치 파일 재설치로 대체 가능.
- 설치본 실행 파일 SHA-256은 `target/release` 산출물과 다름(크기 동일, 번들 단계의 실행 파일 패치). 공개 릴리스는 CI가 같은 커밋에서 다시 빌드하므로 로컬 설치 파일 해시를 공개 자산 해시로 쓰지 않음.
- Claude 쪽 실앱 병합(CLI 프로브 실패 시 직전 값 유지)은 실앱에서 재현하지 않음. 구독 로그인이 있으면 프로브가 실행되지 않아 경로 자체가 드묾. 단위 테스트로 확인.

## 공개 배포 검증 (2026-10-03 KST)

- 배포 소스: `6b8e14c05a305bc65bed7b778d99f5f411210f4a`. `main`과 주석 태그 `v0.2.8`을 원자적으로 함께 푸시(15:41:50), 태그 커밋과 Actions 실행 커밋 일치.
- [GitHub Actions 37103901145](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/37103901145): `completed / success`, 15:41:55~15:52:19(10분 24초). 버전·서명 키 존재·프런트엔드·네이티브 테스트·NSIS 빌드·서명·자산 검증·공개 단계 통과. Node.js 20 지원 종료 안내 주석 1건(`actions/checkout@v4`·`actions/setup-node@v4`가 Node 24로 실행), 실패 아님.
- [공개 release v0.2.8](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.8): release ID `402374671`, `draft=false`, `prerelease=false`, 최신 release ID와 일치, 공개 시각 2026-10-03 15:52:11 KST. 본문은 `docs/releases/v0.2.8.md`.
- 공개 자산 4개를 내려받아 바이트 수와 SHA-256을 GitHub 자산 메타데이터의 `digest`와 각각 대조. `SHA256SUMS.txt`의 설치 파일·서명·manifest 해시도 일치.
- 인증 헤더 없이 `releases/latest/download/latest.json`을 조회해 버전 `0.2.8`, 내려받은 manifest와의 내용 일치, Windows 두 플랫폼 항목의 설치 파일 연결과 서명 문자열 확인.
- 앱의 공개키(키 ID `2b2190fae1b3f640`)로 설치 파일 및 trusted comment 서명 검증 PASS, 설치 파일 첫 바이트를 변조한 사본은 거부.
- 공개 자산에 `node scripts/verify-release.mjs`를 다시 적용해 PASS. Windows Authenticode는 `NotSigned`로, Tauri 업데이터 서명 검증과 별개.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.2.8_x64-setup.exe` | 2,839,202 | `a61e3a28ce9d208ed902340ff8b17df7ea66411c495c09e7bf50f71d7120d979` |
| `SPECTRA_0.2.8_x64-setup.exe.sig` | 416 | `745fff80fb7baa4af1a21ad595220e6ee52f22a084d36d490ce66e35e97b9f99` |
| `latest.json` | 1,382 | `6ef5473c62a302995a3e88510a1d5790a9097e8471b6368cb145a789cfe06f67` |
| `SHA256SUMS.txt` | 270 | `24819b0eab4d8fe011e6375a3c3632104152ba140d3a79c77055598c930c5273` |

- 이 PC의 설치본은 로컬 빌드 0.2.8(설치 파일 2,845,762 B)로, 공개 설치 파일과 해시가 다름. 업데이터는 같은 버전으로 판단해 안내하지 않음. 공개 빌드로 맞추려면 공개 설치 파일 재설치 필요(미실시).
