# SPECTRA v0.3.1 검증 기록

대상: Windows 작업표시줄 Antigravity 표시 후속 개선
전체 판정: **PASS**

구현 및 테스트 224건(프런트엔드 90 + 네이티브 125 + Windows 전용 9), 독립 검토, v0.3.1 공개 릴리즈·소스 리비전·자산 해시·업데이터 서명, 로컬 설치·버전·실행파일 해시 기록 및 실제 실행을 확인했습니다. 정식 설치본의 native 작업표시줄에서 Codex·Claude·Antigravity와 잔여율이 함께 표시되는 것을 직접 확인했습니다.

## 범위와 승인

- 사용자가 기존 수정·커밋·푸시·릴리즈·로컬 설치 목표에 작업표시줄 표시를 추가했습니다.
- 이번 범위는 작업표시줄의 Codex·Claude 표시 유지, Antigravity 요약 추가, 모델군별 툴팁, 오래된 CLI 데이터 표시, 좁은 폭 대응과 v0.3.1 배포·설치 확인입니다.
- 기존 v0.3.0 검증 문서를 변경하거나 v0.3.1 실행 증거로 대체하지 않습니다. 이전 기능의 증거는 해당 범위의 회귀 판단에만 사용합니다.
- 실제 계정 식별정보, 사용량 숫자 원문, CLI 입력 전문과 인증정보는 이 문서에 복제하지 않습니다.
- 최종 소스·태그 커밋은 `7a30f005902bbcd627856bfbb9824e858a6bba35`입니다. 아래 공개·설치 증거와 대응합니다.

## 요구사항과 완료 기준

| 요구사항 | 완료 기준 | 현재 상태 |
|---|---|---|
| Codex·Claude 유지 | 기존 두 서비스의 한도 표시가 회귀하지 않음 | PASS: 회귀 검사 및 정식 설치본 실제 표시 |
| Antigravity 요약 | 모델군의 유효한 한도 창 중 잔여 비율이 가장 낮은 한도를 표시 | PASS: 최소값 검사 및 정식 설치본 실제 표시 |
| 모델군 구분 | 툴팁 모델에서 Gemini, Claude·GPT, Argon의 한도와 상태를 구분 | PASS: 모델 및 실제 HWND 자동 검사. 수동 hover는 아래 경계 참조 |
| 오래된 데이터 | 5분 경과·초기화 경과·결측을 최신 값과 구분 | PASS: 상태 검사 및 오래된 데이터의 회색 픽셀 검사 |
| 좁은 폭 | 좁은 폭에서 3서비스 요약으로 압축하고 넓어지면 기간별 표시를 복원 | PASS: 실제 HWND 자동 검사 |
| 유지 모듈 | 기존 지원 버전을 유지하며 0.3.1 지원 추가; 스크립트 구문 정상 | PASS(소스·구문). 선택형 유지 모듈을 새로 설치하거나 예약작업을 변경하지 않음 |
| 릴리즈 | 버전·후보 리비전·테스트·CI·공개 자산·서명·해시 대응 | PASS: stable v0.3.1 공개 및 자산 검증 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 작업표시줄 동작 확인 | PASS: 설치 종료 0, 버전·해시·정식 실행 경로·실제 화면 확인 |

## 검사 증거

| 검사 | 상태 | 범위 |
|---|---|---|
| 버전 정합성 | PASS | `node scripts/verify-release.mjs`: v0.3.1 source preflight. package.json·package-lock.json·Cargo.toml·Cargo.lock·tauri.conf.json 일치 |
| `npm test` | PASS | 90 passed, 0 failed |
| `npm run build` | PASS | TypeScript 검사 및 production 프런트엔드 빌드 통과 |
| `npm run verify:baseline` | PASS | 승인 디자인 기준·시안 해시 검사 통과 |
| `npm run verify:tokens` | PASS | 토큰·색상·효과 제한 검사 통과 |
| `npm run verify:memory` | PASS | JS 239,922바이트, CSS 23,636바이트. 기존 예산 유지 |
| 네이티브 일반 검사 | PASS | `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 125 passed, 0 failed |
| Windows 전용 검사 | PASS | `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked -- --ignored --test-threads=1`: HWND·GDI 관련 9 passed, 0 failed |
| 작업표시줄 집중 검사 | PASS | 34건 통과. 위 네이티브 검사에 포함되므로 총계에 중복 가산하지 않음 |
| 독립 검토 | PASS | 모델·렌더러 검토에서 새로 확정된 P1/P2 지적 없음 |
| 유지 모듈 지원 목록 | PASS(소스) | 0.2.8·0.2.9·0.3.0 보존, 0.3.1 추가. 예약작업을 실행·변경하지 않음 |
| PowerShell 구문 | PASS | 설치 스크립트 PowerShell Parser 오류 0. 실제 예약작업 설치·변경과 구분 |
| 실제 작업표시줄 | PASS(정식 설치본) | CUA 화면에서 별도 native 작업표시줄 영역의 Codex·Claude 5시간/7일·보라색 Antigravity와 잔여율 표시 확인. 개발 실행에서도 확인 |
| 공개 자산 | PASS | stable 릴리즈·CI 성공, 원격 digest·manifest 대응, minisign 정상 서명 및 변조 payload 거부 확인 |
| 설치 및 실행 | PASS | 공개 NSIS `/S` 종료 0, ProductVersion·레지스트리 0.3.1, 설치 EXE 해시·실제 설치 경로 실행·미니창/대시보드·작업표시줄 확인 |
| Windows 게시자 서명 | 확인됨: NotSigned | 설치파일과 설치된 EXE의 Authenticode 상태. 업데이터 서명과 별개 |

위 테스트·실화면 결과는 통합 담당자가 실행한 검사와 직접 관찰 증거를 회수해 기록했습니다. 문서 편집을 별도의 독립 실행 증거로 계산하지 않습니다.

## 공개 릴리즈 및 설치 증거

- 소스·태그 커밋: `7a30f005902bbcd627856bfbb9824e858a6bba35` (`v0.3.1`). 후속 QA 문서 커밋은 이 바이너리의 빌드 소스와 구분합니다.
- [GitHub Actions 41453443646](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41453443646): success.
- [공개 릴리즈 v0.3.1](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.3.1): stable 공개, 게시 시각 `2026-11-02T13:39:37Z` (KST 2026-11-02 22:39:37).
- 설치파일·서명·manifest를 원격 자산 digest와 대조했고 manifest의 버전·설치파일·서명 대응을 확인했습니다. 실제 설치파일 payload 및 trusted comment의 minisign 암호학적 검증이 통과했으며, 변조 payload는 거부됐습니다.
- 설치 파일의 Windows Authenticode 상태는 `NotSigned`입니다. 검증된 업데이터 서명과 구분합니다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.3.1_x64-setup.exe` | 2,866,398 | `66edabb18a2fc758c8ed9d2a0893a3c5a428217d35e861608e93f146b0f52652` |
| `SPECTRA_0.3.1_x64-setup.exe.sig` | 416 | `9f67c0ea3e6d96e3e105537f1f3be995443d5d859ff9fb8af398e3e581f43a50` |
| `latest.json` | 1,382 | `620ab666b79cd20298fff49401efccdec5b288580cea3f9a40236a3aa1334032` |
| `SHA256SUMS.txt` | 270 | 위 설치파일·서명·manifest의 SHA-256 목록 |

검증한 공개 NSIS를 `/S`로 실행했고 설치 종료 코드는 `0`입니다. 설치된 `%LOCALAPPDATA%/SPECTRA/spectra-native.exe`의 ProductVersion과 제거 레지스트리의 표시 버전은 모두 `0.3.1`입니다. 설치 EXE는 **6,349,312바이트**, SHA-256 `5D50010715B28100E57EEBADDA4054759EC8E4C9AE67CB097347C14EEC033F10`, Authenticode `NotSigned`입니다.

정식 설치 경로의 앱을 CUA로 확인해 미니창·대시보드 정상 표시와 다크 모드 유지를 확인했습니다. 새 Antigravity CLI가 오류 없이 실행됐고 사용량 UI에 최신 값이 반영됐습니다. 정식 설치본의 별도 native 작업표시줄 영역에서 Codex, Claude 5시간/7일, 보라색 Antigravity와 잔여율이 함께 표시되는 것을 직접 확인했습니다. 실제 개인 사용량 숫자는 이 기록에 복제하지 않습니다.

NSIS 내부 payload를 별도로 추출해 설치 EXE와 대조하는 검사는 수행하지 않았습니다. 검증한 NSIS의 직접 설치 실행, 설치 버전·해시 기록, 설치 경로에서 실행되는 앱과 실제 화면을 확인한 범위입니다.

## 작업표시줄 집중 확인

- Antigravity 요약은 예측값이 아니라 **유효한 한도 창 중 잔여 비율이 가장 낮은 값**입니다. 사용 속도나 초기화 시각으로 어느 모델이 먼저 제한되는지를 예측하지 않습니다.
- 최소값 집계, 5분 경과 stale, 초기화 경과, 결측·NaN 제외 및 Antigravity의 Claude·GPT와 직접 Claude 구독 분리 검사가 통과했습니다.
- 실제 Windows HWND를 생성하는 자동 검사에서 좁은 폭의 3서비스 압축 표시, 폭 복원 시 원래 기간별 표시와 툴팁 모델 복원, Antigravity 보라색 픽셀, stale 회색 픽셀, DPI 관련 검사가 통과했습니다.
- 실제 데스크톱 화면에서 별도 native 작업표시줄 영역에 기존 서비스와 Antigravity가 함께 표시되는 것을 확인했습니다. 실제 개인 수치는 이 문서에 복제하지 않습니다.

## 데이터와 검증 경계

Antigravity는 공식 CLI 상태선에서 전달된 수치를 사용합니다. 새로운 CLI 전달 없이 예시값을 실제 사용량으로 표시하지 않습니다. 오래된 수치나 초기화 시각이 지난 창은 정상 최신 값과 구분합니다. GUI 직접 조회를 추가하지 않았습니다.

실제 포인터 hover로 툴팁을 여는 수동 검증과 현재 모니터 외의 DPI 환경 수동 검증은 수행하지 않았습니다. 툴팁 모델·폭 복원·DPI의 자동 HWND/GDI 검사를 그 수동 검증으로 표현하지 않습니다.

포커스 도크·구독 모델·테마·기간 조회는 v0.3.0 동작을 유지합니다. 해당 프런트엔드 JS/CSS 해시가 동일해 v0.3.0의 버튼·테마·이력 실검증을 해당 범위의 회귀 근거로 재사용하며, v0.3.1 정식 설치본의 미니창·대시보드와 다크 모드 유지도 직접 확인했습니다. 이번 변경의 핵심 범위는 native 작업표시줄입니다. 선택 설치형 유지 모듈의 지원 목록·구문 확인을 해당 예약작업의 신규 설치 증거로 해석하지 않습니다.

## 완료 판정

작업표시줄 표시를 포함한 수정·검증·커밋·푸시·릴리즈·로컬 설치와 정식 설치본의 핵심 표시 확인을 완료했습니다. 전체 PASS는 위에 명시한 Windows 설치형 범위의 판정입니다. 수동 hover·다른 모니터의 수동 DPI·NSIS 내부 payload 별도 추출은 수행하지 않은 경계로 보존하며, 자동 검사나 설치 실행을 해당 검사의 완료로 대체해 표현하지 않습니다.
