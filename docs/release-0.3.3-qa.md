# SPECTRA v0.3.3 검증 기록

대상: 트레이 배지·트레이 툴팁의 AG Gemini 기준 표시
전체 판정: **PASS** (Windows 설치형 범위. 실제 트레이 툴팁 값은 미확인, 아래 경계 참조)

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
| 릴리스 | 버전·태그·CI·공개 자산·서명·해시 대응 | PASS: stable v0.3.3 공개 및 자산 검증 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 실행 확인 | PASS: 설치 종료 0, 버전·해시·실행 경로·작업표시줄 화면 확인 |

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

## 공개 릴리스 및 설치 증거

- 소스·태그 커밋: `42804568d97fec12f09070d7a1b29f513d0740f7` (`v0.3.3`). 이 QA 문서 커밋은 바이너리의 빌드 소스와 구분합니다.
- [GitHub Actions 41735320911](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41735320911): success. CI의 네이티브 테스트 단계도 통과했습니다.
- [공개 릴리스 v0.3.3](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.3.3): stable 공개, 게시 시각 `2026-11-03T23:07:35Z` (KST 2026-11-04 08:07:35). `releases/latest` = v0.3.3.
- `SHA256SUMS.txt` 대조 OK, manifest 버전·서명이 공개 자산과 일치. 앱 설정 공개키로 설치파일 서명과 trusted comment를 직접 검증해 통과했고, 변조 payload는 거부됐습니다.
- 설치 파일의 Windows Authenticode 상태는 `NotSigned`입니다. 업데이터 서명과 별개입니다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.3.3_x64-setup.exe` | 2,865,907 | `2d2c0fb157152685156e697eeaca57a9bd5a6faca6bed72fc2b3435bac49e87b` |
| `SPECTRA_0.3.3_x64-setup.exe.sig` | 416 | `da2a7a37904a0481621e4b333600effe5a615fdd3493c4f89bb0bf505076b773` |
| `latest.json` | 1,382 | `d7f7b7fa89ac640df8e63f18542b32d3024dd9ea207dfdcf951835c32989f2e0` |
| `SHA256SUMS.txt` | 270 | `f5846064fe0eb16d220d3e4597aed7ddedf8444ef1e1c5140f1a3610edca3f6b` |

검증한 공개 NSIS를 `/S`로 실행했고 종료 코드는 `0`입니다. 설치된 `%LOCALAPPDATA%/SPECTRA/spectra-native.exe`의 ProductVersion과 제거 레지스트리는 모두 `0.3.3`이고, **6,349,824바이트**, SHA-256 `7909499283920E6DEB6A1B1691BB046335E891A16A2EB26912A64B1534532CDD`, Authenticode `NotSigned`입니다. `ui-prefs.json` 해시는 설치 전후 동일합니다. 앱은 Explorer로 실행해 정식 설치 경로에서 동작하는 것을 확인했고, 작업표시줄의 세 서비스 표시를 화면 캡처로 확인했습니다.

## 데이터와 검증 경계

트레이 배지의 AG 계산은 모델 검사로 확인했습니다. 트레이 아이콘이 숨겨진 아이콘 영역에 있어 화면을 조작하지 않는 UI Automation으로는 툴팁을 읽지 못했고, 실제 배지·툴팁 값은 확인하지 않았습니다.

비Windows 플랫폼 빌드는 확인하지 않았습니다. NSIS 내부 payload를 별도로 추출해 설치 EXE와 대조하는 검사도 수행하지 않았습니다. 선택 설치형 유지 모듈 예약작업은 변경하지 않았습니다.
