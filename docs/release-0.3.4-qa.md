# SPECTRA v0.3.4 검증 기록

대상: 대시보드 Antigravity 모델군 표시 순서 고정
전체 판정: **PASS** (Windows 설치형 범위)

## 범위와 승인

- 사용자 요청: 대시보드의 Antigravity 모델군 순서를 Gemini → Gemini 4 Argon → Claude · GPT로 고정하고, 권장안(공개 릴리스 v0.3.4)대로 실행 프로그램에 반영.
- 작업표시줄·트레이의 AG 계산 기준(`gemini` 모델군만)과 처음 선택되는 모델군 로직은 v0.3.3 그대로입니다.
- 실제 계정 식별정보, 인증정보, 개인 사용량 수치는 이 문서에 복제하지 않습니다.

## 요구사항과 완료 기준

| 요구사항 | 완료 기준 | 현재 상태 |
|---|---|---|
| 모델군 순서 | 입력 순서와 무관하게 Gemini, Gemini 4 Argon, Claude · GPT 순으로 정렬되고 알려지지 않은 모델군은 뒤에 옴 | PASS: 모델 검사(`tests/focus-dock.test.ts`, 입력을 뒤집은 경우 포함) |
| 화면 적용 | 모델군 탭·선택 목록, "다른 모델군", 구독 모델 패널이 같은 순서 | PASS: 브라우저 데모(예시 데이터)에서 탭과 "다른 모델군" 순서 확인, 정식 설치본 대시보드는 사용자 육안 확인 |
| 번들 예산 | JS 번들 240,000바이트 이하 | PASS: 정렬 보조 함수 축소 후 239,992바이트(축소 전 240,018바이트로 FAIL) |
| 릴리스 | 버전·태그·CI·공개 자산·서명·해시 대응 | PASS: stable v0.3.4 공개 및 자산 검증 |
| 로컬 설치 | 공개 NSIS 설치 후 버전·실행 경로·실제 실행 확인 | PASS: 설치 종료 0, 버전·해시·레지스트리·실행 프로세스 확인. 아래 설치 중 관찰 참조 |

## 검사 증거

| 검사 | 상태 | 범위 |
|---|---|---|
| 버전 정합성 | PASS | `node scripts/verify-release.mjs`: v0.3.4 source preflight |
| `npm test` | PASS | 90 passed, 0 failed |
| `npm run build` | PASS | TypeScript 검사 및 production 프런트엔드 빌드 |
| `npm run verify:baseline` | PASS | 디자인 기준 검사 |
| `npm run verify:tokens` | PASS | 토큰 검사 |
| `npm run verify:memory` | PASS | JS 239,992바이트, CSS 23,636바이트 |
| 네이티브 일반 검사 | PASS | `cargo test --lib --locked`: 130 passed, 0 failed, 9 ignored(첫 실행에서 통과) |
| Windows 전용 검사(`--ignored`) | 미실행 | 네이티브 코드 변경이 없어 생략. CI의 네이티브 테스트 단계는 통과 |
| 유지 모듈 지원 목록 | PASS(소스) | 0.3.4 추가. 설치된 유지 모듈과 예약작업은 변경하지 않음 |

## 공개 릴리스 증거

- 소스·태그 커밋: `4e396c47bb21107a7a6f9eb4299cfb98f9838e22` (`v0.3.4`, `origin/main`). 이 QA 문서 커밋은 바이너리의 빌드 소스와 구분합니다.
- [GitHub Actions 42351494274](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/42351494274): success(07:29:32Z 시작, 07:39:00Z 종료). 워크플로가 자산 검증·`SHA256SUMS.txt` 업로드·공개 전환까지 수행했습니다.
- [공개 릴리스 v0.3.4](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.3.4): stable 공개, 게시 시각 `2026-11-07T07:38:53Z` (KST 2026-11-07 16:38:53). `releases/latest` = v0.3.4.
- 내려받은 자산으로 `sha256sum -c SHA256SUMS.txt` OK, GitHub 자산 digest와 일치. `node scripts/verify-release.mjs <자산 폴더>` PASS(manifest 버전·서명·설치 파일 asset ID 719997112 대응).
- 앱 설정 공개키(minisign 키 ID `40f6b3e1fa90212b`, v0.2.4 기록의 `2b2190fae1b3f640`과 바이트 순서만 다른 같은 키)로 `.exe.sig`를 Node 표준 `crypto`(Ed25519 + BLAKE2b-512 prehash)로 검증: 파일 서명·전역 서명 유효, trusted comment `file:SPECTRA_0.3.4_x64-setup.exe`, 변조 payload 거부, manifest가 같은 서명을 포함.
- 설치 파일의 Windows Authenticode 상태는 `NotSigned`입니다. 업데이터 서명과 별개입니다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.3.4_x64-setup.exe` | 2,865,478 | `8c14da250279a09acbd3fbc40ce5a2b577e5164cf61c05b83ba9782480efdef1` |
| `SPECTRA_0.3.4_x64-setup.exe.sig` | 416 | `2b884a4945d99df5a269f1803b94bc2c583ddf9ff362ee1b7337134c9f42aef1` |
| `latest.json` | 1,382 | `32e5a7e991fa18f6e0b04cb7069c297bc53228913aac1f6b48fcf40b8b088f52` |
| `SHA256SUMS.txt` | 270 | `98c5045093ec49bd353b3799a66b624f31a1d516bdc4474e9fc1a4613d881a94` |

## 로컬 설치

설치 전 상태: `%LOCALAPPDATA%/SPECTRA/spectra-native.exe`는 v0.3.3 설치본(6,349,824바이트, 2026-11-04)이고 앱 프로세스 1개가 실행 중이었습니다. 사용자 승인(2026-11-07) 후 검증한 공개 NSIS를 `/S`로 실행했고 종료 코드는 `0`입니다.

설치된 `spectra-native.exe`의 ProductVersion과 제거 레지스트리(`DisplayVersion`)는 모두 `0.3.4`이고, **6,349,824바이트**, SHA-256 `56bd2adb031d73f04722a1de22d4cceb782d96a421a5ef3ac0f61eec4b1c8335`, 파일 시각은 CI 빌드 시각(16:38:38 KST), Authenticode `NotSigned`입니다. `ui-prefs.json` 해시는 설치 전후 동일합니다. 설치 직후 앱이 정식 설치 경로에서 자동으로 다시 실행됐고(`SPECTRA` 창 보유 프로세스 1개) 설치 폴더의 사용량 캐시·기록 파일과 유지 모듈 폴더는 그대로 남아 있습니다.

### 설치 중 관찰

앱이 실행 중인 상태에서 `/S` 설치를 시작하자 설치 프로그램이 기존 앱을 종료하고 이전 버전 제거 프로그램(`SPECTRA Uninstall`, v0.3.3 `uninstall.exe`)을 호출했는데, 제거 프로그램이 실행 파일과 레지스트리 항목을 지운 뒤 약 10분 넘게 종료되지 않았고 설치 프로그램은 `Not Responding` 상태로 대기했습니다. 제거 프로그램이 멈춘 원인(대화상자 여부 포함)은 화면을 확인하지 못해 밝히지 못했습니다. 강제 종료를 시도한 시점에는 제거 프로그램이 이미 스스로 끝나 있었고, 그 직후 설치가 정상 완료됐습니다. v0.3.3 설치 때는 보고되지 않은 현상이며, 재현되면 설치 전에 앱을 먼저 종료하는 절차를 검토할 필요가 있습니다.

## 데이터와 검증 경계

모델군 정렬은 모델 검사로, 화면 반영은 브라우저 데모(예시 데이터)로 확인했습니다. 예시 데이터는 이미 정렬된 순서라 뒤섞인 입력의 정렬은 모델 검사에만 의존합니다. 정식 설치본의 Tauri 대시보드 화면은 화면 캡처 도구가 앱 창을 대상으로 잡지 못해 자동 확인하지 못했습니다. 설치 직후 사용자가 2026-11-07 대시보드를 열어 모델군이 Gemini → Gemini 4 Argon → Claude · GPT 순으로 표시되는 것을 육안으로 확인했습니다.

비Windows 플랫폼 빌드는 확인하지 않았습니다. NSIS 내부 payload를 별도로 추출해 설치 EXE와 대조하는 검사도 수행하지 않았습니다.
