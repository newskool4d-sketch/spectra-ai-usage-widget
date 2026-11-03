# SPECTRA v0.3.3 검증 기록

대상: 트레이 배지·트레이 툴팁의 AG Gemini 기준 표시
전체 판정: **PARTIAL — 공개·설치 검증 대기**

## 범위와 승인

- 사용자 요청: v0.3.2에서 범위 밖으로 둔 트레이 배지의 AG 값을 작업표시줄과 같은 Gemini 기준으로 맞추고 배포.
- Gemini 기준은 v0.3.2에서 사용자가 선택한 대로 `gemini` 모델군만 사용하고 Gemini 4 Argon은 제외합니다.
- 실제 계정 식별정보, 인증정보, 개인 사용량 수치는 이 문서에 복제하지 않습니다.

## 요구사항과 완료 기준

| 요구사항 | 완료 기준 | 현재 상태 |
|---|---|---|
| 배지 AG 기준 | AG가 `gemini` 모델군의 낮은 값으로만 계산되고 다른 모델군이 값을 대신하지 않음 | PASS: 모델 검사 |
| 서비스 간 최솟값 유지 | 다른 서비스의 잔여율이 더 낮으면 그 서비스를 표시 | PASS: 모델 검사 |
| 작업표시줄 기준 일치 | 작업표시줄과 트레이가 같은 판정 함수 사용 | PASS: `antigravity::is_summary_window` 공용 |
| 릴리스 | 버전·태그·CI·공개 자산·서명·해시 대응 | 미검증: 공개 단계 대기 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 실행 확인 | 미검증: 설치 단계 대기 |

## 검사 증거

| 검사 | 상태 | 범위 |
|---|---|---|
| 버전 정합성 | PASS | `node scripts/verify-release.mjs`: v0.3.3 source preflight |
| `npm test` | PASS | 90 passed, 0 failed |
| `npm run build` | PASS | TypeScript 검사 및 production 프런트엔드 빌드 |
| `npm run verify:baseline` | PASS | 디자인 기준 검사 |
| `npm run verify:tokens` | PASS | 토큰 검사 |
| `npm run verify:memory` | PASS | JS 239,914바이트, CSS 23,636바이트 |
| 네이티브 일반 검사 | PASS(재실행 기준) | `cargo test --lib --locked`: 130 passed, 0 failed. 아래 실패 기록 참조 |
| Windows 전용 검사 | PASS | `--ignored --test-threads=1`: 9 passed, 0 failed |
| 유지 모듈 지원 목록 | PASS(소스) | 0.3.3 추가, PowerShell Parser 오류 0. 설치된 유지 모듈과 예약작업은 변경하지 않음 |

버전 변경 후 재빌드 직후 첫 네이티브 일반 검사에서 시간 제한 테스트 2건(`output_with_timeout_kills_slow_child`, `previous_statusline_times_out_when_stdin_is_not_consumed`)이 실패했습니다(128 passed, 2 failed). 같은 실행 파일로 다시 실행한 4회는 모두 130건 통과했습니다. v0.3.2 작업에서도 같은 2건이 같은 방식으로 실패한 적이 있으며, 트레이 변경 직후 재빌드 첫 실행은 통과해 재현이 일정하지 않습니다. 두 테스트는 하위 프로세스 실행 시간을 벽시계로 재며 이번 변경이 건드리지 않은 코드입니다. 실제 원인은 확인하지 않았습니다.
