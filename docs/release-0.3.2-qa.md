# SPECTRA v0.3.2 검증 기록

대상: 작업표시줄 AG Gemini 기준 표시, Claude·Antigravity 갱신 필요 안내 완화
전체 판정: **PASS** (Windows 설치형 범위. 절전 해제 재현과 수동 hover는 아래 경계 참조)

## 범위와 승인

- 사용자 요청: 작업표시줄 AG 표시를 Gemini 기준으로 변경, Claude·Antigravity의 CLI 실행 후 갱신 필요 안내가 자주 뜨는 부분 점검, 커밋·푸시·릴리스·로컬 설치까지 완료.
- 사용자 선택: Gemini 기준은 `gemini` 모델군만 사용하고 Gemini 4 Argon은 제외.
- 실제 계정 식별정보, 인증정보는 이 문서에 복제하지 않습니다.

## 진단

| 대상 | 원인 | 근거 |
|---|---|---|
| Antigravity | 값은 CLI 실행 때만 들어오는데 수집 5분 뒤 곧바로 갱신 필요로 표시 | `FRESH_SECS = 300`, 2026-11-02~04 수집 시각 11건, 대부분 15분 이상 간격 |
| Claude | 만료 토큰 갱신이 실패하면 30분 동안 재조회 없이 "Claude Code를 한 번 실행해 주세요" 유지 | `TOKEN_HOLD_SECS = 1800`. `standby-timing.log`의 2026-10-30 기록에서 Claude `token-expired` 직후 Codex `failed`가 함께 발생해 네트워크 미준비 상태로 추정 |

Claude 원인은 위 로그 1건과 코드 경로로 판단한 것이며, 절전 해제 상황을 재현해 확인하지는 않았습니다.

## 요구사항과 완료 기준

| 요구사항 | 완료 기준 | 현재 상태 |
|---|---|---|
| AG Gemini 기준 | AG 값이 `gemini` 모델군의 5시간·주간 중 낮은 값을 표시하고, 값이 없으면 다른 모델군으로 대체하지 않음 | PASS: 모델 검사 |
| 다른 모델군 유지 | Argon·Claude·GPT가 툴팁에 남음 | PASS: 모델 검사. 수동 hover는 미수행 |
| AG 갱신 안내 | 초기화 경과 또는 수집 후 24시간 경과 때만 갱신 필요 표시 | PASS: Rust·TypeScript 경계 검사 |
| Claude 토큰 재시도 | 토큰 만료 후 120초부터 백오프 재시도, 성공 시 정상 주기 복귀 | PASS: 스케줄러 검사. 실제 절전 해제 재현은 미수행 |
| 릴리스 | 버전·태그·CI·공개 자산·서명·해시 대응 | PASS: stable v0.3.2 공개 및 자산 검증 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 작업표시줄 동작 확인 | PASS: 설치 종료 0, 버전·해시·실행 경로·실제 화면 확인 |

## 검사 증거

| 검사 | 상태 | 범위 |
|---|---|---|
| 버전 정합성 | PASS | `node scripts/verify-release.mjs`: v0.3.2 source preflight |
| `npm test` | PASS | 90 passed, 0 failed |
| `npm run build` | PASS | TypeScript 검사 및 production 프런트엔드 빌드 |
| `npm run verify:baseline` | PASS | 디자인 기준 검사 |
| `npm run verify:tokens` | PASS | 토큰 검사 |
| `npm run verify:memory` | PASS | JS 239,914바이트, CSS 23,636바이트 |
| 네이티브 일반 검사 | PASS | `cargo test --lib --locked`: 128 passed, 0 failed |
| Windows 전용 검사 | PASS | `--ignored --test-threads=1`: 9 passed, 0 failed |
| 유지 모듈 지원 목록 | PASS(소스) | 0.3.2 추가, PowerShell Parser 오류 0. 설치된 유지 모듈과 예약작업은 변경하지 않음 |
| 실제 작업표시줄 | PASS(로컬 release 빌드) | 실제 캐시로 실행해 화면 캡처 확인. AG는 수집 46분 뒤에도 회색이 아니고 Gemini 값으로 표시됨. 수정 전 표시값은 화면으로 확인하지 못했고(첫 캡처 실패), 같은 캐시로 계산하면 Claude·GPT 주간 값임 |
| 자동 갱신 | PASS(로컬 release 빌드) | `standby-timing.log`: `usage_refresh` Claude·Codex·Antigravity `ok` |

첫 네이티브 일반 검사에서 시간 제한 테스트 2건(`output_with_timeout_kills_slow_child`, `previous_statusline_times_out_when_stdin_is_not_consumed`)이 실패했습니다. 다른 빌드 작업과 겹친 실행이었고, 해당 2건 단독 3회와 전체 재실행 1회는 모두 통과했습니다. 이번 변경이 건드리지 않은 코드이며, 원인을 부하로 확정하지는 않았습니다.

## 공개 릴리스 및 설치 증거

- 소스·태그 커밋: `c522fffa2bc04d5966169edba1f5ee022f0ef814` (`v0.3.2`). 이 QA 문서 커밋은 바이너리의 빌드 소스와 구분합니다.
- [GitHub Actions 41729737081](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41729737081): success.
- [공개 릴리스 v0.3.2](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.3.2): stable 공개, 게시 시각 `2026-11-03T22:26:00Z` (KST 2026-11-04 07:26:00). `releases/latest` = v0.3.2.
- `SHA256SUMS.txt` 대조 OK, manifest 버전·서명이 공개 자산과 일치. 앱 설정 공개키로 설치파일 서명과 trusted comment를 직접 검증해 통과했고, 변조 payload는 거부됐습니다.
- 설치 파일의 Windows Authenticode 상태는 `NotSigned`입니다. 업데이터 서명과 별개입니다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.3.2_x64-setup.exe` | 2,865,265 | `0ba558551381c375a9a4ba4aa1702aae549848795cec520dbd26013e7f15aa8d` |
| `SPECTRA_0.3.2_x64-setup.exe.sig` | 416 | `01fbc1a1db56976b65dfb2b25ad051eb05cca536f89c989e135825f0f9652151` |
| `latest.json` | 1,382 | `2693741ce5e25f7839a44639a401243dd4386d244e8a340246e093b3aae4f69c` |
| `SHA256SUMS.txt` | 270 | `54597c0b3f784c7661eb536182814e317e5ba20a0b78a7a5b16183655f6974b9` |

검증한 공개 NSIS를 `/S`로 실행했고 종료 코드는 `0`입니다(첫 실행의 출력 기록이 유실되어 같은 설치를 한 번 더 실행해 기록). 설치된 `%LOCALAPPDATA%/SPECTRA/spectra-native.exe`의 ProductVersion과 제거 레지스트리는 모두 `0.3.2`이고, **6,349,312바이트**, SHA-256 `9059D618313C4B3C5281D934349533A2A25BA30B1436CA0482598841395695F3`, Authenticode `NotSigned`입니다. `ui-prefs.json` 해시는 설치 전후 동일합니다.

정식 설치 경로로 실행한 앱의 작업표시줄을 화면 캡처로 확인했습니다. AG가 Gemini 기준 값으로 표시되고 수집 후 1시간이 지난 값도 회색이 아니었습니다. `claude-usage.json`은 설치 후 자동 갱신됐습니다.

NSIS 내부 payload를 별도로 추출해 설치 EXE와 대조하는 검사는 수행하지 않았습니다.

## 데이터와 검증 경계

Antigravity 수치는 공식 CLI 상태선에서만 들어옵니다. 이번 변경은 안내 기준을 바꾼 것이며 CLI 없이 수치를 갱신하지 않습니다. 트레이 배지는 Antigravity 모든 모델군의 최소값을 계속 사용하므로 작업표시줄 AG 값과 다를 수 있습니다.

절전 해제 직후 토큰 갱신 실패 상황을 실제로 재현하지 않았습니다. 재시도 동작은 스케줄러 자동 검사로만 확인했으므로, 안내 빈도 감소는 실사용에서 확인이 필요합니다. Antigravity는 5시간 창의 초기화 시각이 지나면 CLI를 다시 실행하기 전까지 갱신 필요 안내가 계속 표시됩니다. 실제 포인터 hover로 툴팁을 여는 수동 검증은 수행하지 않았습니다.

선택 설치형 유지 모듈 예약작업은 변경하지 않았습니다. 이 작업의 마지막 실행은 2026-10-30이며, 이번 확인 시점에 유지 모듈 프로세스는 실행 중이 아니었습니다.
