# SPECTRA v0.3.2 검증 기록

대상: 작업표시줄 AG Gemini 기준 표시, Claude·Antigravity 갱신 필요 안내 완화
전체 판정: **PARTIAL — 공개·설치 검증 대기**

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
| 릴리스 | 버전·태그·CI·공개 자산·서명·해시 대응 | 미검증: 공개 단계 대기 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 작업표시줄 동작 확인 | 미검증: 설치 단계 대기 |

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
| 실제 작업표시줄 | PASS(로컬 release 빌드) | 실제 캐시로 실행해 화면 캡처 확인. AG는 수집 46분 뒤에도 회색이 아니고 Gemini 값으로 표시됨. 같은 캐시에서 수정 전 표시값은 Claude·GPT 주간 값이었음 |
| 자동 갱신 | PASS(로컬 release 빌드) | `standby-timing.log`: `usage_refresh` Claude·Codex·Antigravity `ok` |

첫 네이티브 일반 검사에서 시간 제한 테스트 2건(`output_with_timeout_kills_slow_child`, `previous_statusline_times_out_when_stdin_is_not_consumed`)이 실패했습니다. 다른 빌드 작업과 겹친 실행이었고, 해당 2건 단독 3회와 전체 재실행 1회는 모두 통과했습니다. 이번 변경이 건드리지 않은 코드이며, 원인을 부하로 확정하지는 않았습니다.

## 데이터와 검증 경계

Antigravity 수치는 공식 CLI 상태선에서만 들어옵니다. 이번 변경은 안내 기준을 바꾼 것이며 CLI 없이 수치를 갱신하지 않습니다. 트레이 배지는 Antigravity 모든 모델군의 최소값을 계속 사용하므로 작업표시줄 AG 값과 다를 수 있습니다.
