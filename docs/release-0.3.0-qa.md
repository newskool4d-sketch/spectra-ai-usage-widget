# SPECTRA v0.3.0 검증 기록

확인 기준: 2026-11-02 작업 회차
전체 판정: **PASS**. 구현 및 테스트 220건(프런트엔드 90 + 네이티브 121 + Windows 전용 9), 독립 검토, v0.3.0 공개 릴리즈·자산 해시·업데이터 서명, 로컬 설치·설치본 버전 및 실행파일 해시 기록·실제 실행을 확인했습니다. 정식 설치본에서 설정·구독 모델·기간 조회·다크/라이트 및 재실행 후 테마 유지·Antigravity 상태선 재연결과 실제 최신 데이터 전달까지 검증했습니다.

## 범위와 승인

- 대상: SPECTRA Windows 설치형, `codex/focus-dock-antigravity` 작업트리.
- 릴리즈 소스·태그 커밋: `a984f92855eb96dcddd3488236d2a2a40027c2c4` (`v0.3.0`). 이후 검증 기록 추가는 문서 변경이며 이 릴리즈 바이너리의 소스 리비전과 구분합니다.
- 사용자 승인: 포커스 도크 대시보드·미니창, 미니창 다크/라이트, 비용 대신 구독 모델, Antigravity 항목, 오늘/7일/30일 이력, 수정 후 커밋·푸시·릴리즈·로컬 설치.
- 기존 별도 작업트리의 사용자 수정은 이 작업에 포함하지 않습니다.
- 과거 기록을 합성하거나 수집 횟수를 AI 요청수·비용으로 환산하지 않습니다. 이력은 실제 수집 시점부터 시작합니다.

## 검사 증거

| 항목 | 상태 | 확인 결과와 범위 |
|---|---|---|
| 버전 정합성 | PASS | package.json·package-lock.json·Cargo.toml·Cargo.lock·tauri.conf.json 모두 0.3.0, release preflight 통과 |
| `npm test` | PASS | 최종 프런트엔드 회차 90 passed, 0 failed |
| `npm run build` | PASS | TypeScript 검사 및 production 프런트엔드 빌드 통과 |
| `npm run verify:memory` | PASS | JS 239,922바이트, CSS 23,636바이트. 기존 JS 240,000/CSS 34,000바이트 예산 유지 |
| `npm run verify:tokens` | PASS | 토큰·색상·효과 제한 검사 통과 |
| `npm run verify:baseline` | PASS | 승인 포커스 도크 2시안 해시, 다크·라이트 기준 및 Figma 런타임 비의존성 확인 |
| `npm run verify:release` | PASS | v0.3.0 source preflight 및 공개 자산 검사 통과. 설치파일·서명·manifest SHA-256과 GitHub digest 대조, manifest 버전·설치 자산·서명 일치 확인 |
| 네이티브 일반 검사 | PASS | 최종 `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked`: 121 passed, 0 failed, Windows 전용 9 ignored. Windows 명령 인용 및 실제 quota 버킷 alias 수정 포함 |
| Windows 전용 검사 | PASS | `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked -- --ignored --test-threads=1`: GDI·트레이·작업표시줄 9 passed, 0 failed. 최종 quota alias 추가 전에 실행했으며 이후 해당 Windows 코드 변경이 없어 이 증거를 재사용 |
| 네이티브 독립 검토 | PASS | N-001~N-003 수정 후 재검토에서 해소 확인. Windows 상태선 래퍼 최종 재검토에서 새 P1/P2 발견 없음 |
| 프런트엔드 독립 검토 | PASS | F-001~F-004 모두 수정 후 재검토에서 해소 확인 |
| 최종 설치형 패키지·리비전 | PASS | `a984f92855eb96dcddd3488236d2a2a40027c2c4` 소스·태그의 Windows CI 빌드 성공 및 v0.3.0 설치 자산 확인 |
| 공개 릴리즈·업데이터 서명·자산 해시 | PASS | stable v0.3.0 공개 자산 4개 확인. 설치파일 실제 payload·trusted comment의 minisign 암호학적 검증과 첫 바이트 변조본 거부 확인 |
| Windows Authenticode | 확인됨: NotSigned | 업데이터 minisign 서명과 별개. Windows 게시자 코드 서명은 없음 |
| 로컬 설치·실행 | PASS | 검증된 NSIS 실행 종료 코드 0, 설치 EXE/ProductVersion 0.3.0, 설치 경로 프로세스 실행 및 실제 핵심 동작 확인. EXE 해시는 아래 기록. 패키지 내부 payload 별도 추출 대조는 미실시 |

프런트엔드 수치와 아래 실화면 결과는 통합 담당자가 실행한 최신 검사·직접 관찰 결과를 회수해 기록했습니다. 이 문서 편집 자체를 별도의 독립 실행 증거로 계산하지 않습니다.

## 공개 릴리즈 증거

- [공개 릴리즈 v0.3.0](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.3.0): stable, 공개 자산 4개, 게시 시각 `2026-11-02T12:53:51Z` (KST 2026-11-02 21:53:51). 최종 재조회에서 GitHub latest도 같은 stable 공개 v0.3.0을 가리킵니다.
- [GitHub Actions 41446683701](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41446683701): success. 소스·태그 커밋 `a984f92855eb96dcddd3488236d2a2a40027c2c4`.
- `minisign-verify 0.2.5`로 앱의 공개키를 사용해 실제 설치파일 payload와 trusted comment 서명을 검증했습니다. 정상 파일은 통과했고 첫 바이트를 변조한 사본은 거부했습니다.
- Windows Authenticode 상태는 `NotSigned`입니다. 업데이터 서명 검증 통과를 Windows 게시자 코드 서명으로 표현하지 않습니다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.3.0_x64-setup.exe` | 2,863,838 | `52de59fa93b19355e6c9365e818c96ef365a9f960abe6ffefd69df657b98428c` |
| `SPECTRA_0.3.0_x64-setup.exe.sig` | 416 | `78a68f423ed1d6357ce2bbaf8d032895a6d4c8107e7982e26217ce18a9edf2b1` |
| `latest.json` | 1,382 | `9619cedce2316b065717041d2dd983e8128ce118ad6e3f377e1462a94a05b260` |
| `SHA256SUMS.txt` | 270 | 설치파일·서명·manifest의 위 SHA-256 목록 |

설치파일·서명·manifest 각각을 원격 자산 digest와 대조한 뒤 검사 스크립트가 SHA256SUMS.txt를 생성했습니다. 이 파일은 검증 대상 3개 자산의 해시 목록이며 자기 자신의 해시를 완료 필수항목으로 삼지 않습니다.

## 로컬 설치 검증

- 검증한 공개 NSIS 설치 파일을 실행했고 종료 코드는 `0`입니다.
- 설치 경로는 `%LOCALAPPDATA%/SPECTRA/spectra-native.exe`입니다. 해당 경로의 프로세스만 실행 중이며 개발용 debug 실행파일은 실행 중이지 않음을 확인했습니다.
- 설치된 실행파일 ProductVersion은 `0.3.0`입니다. 기존 제거 레지스트리의 표시 버전은 `0.2.7`에서 `0.3.0`으로 갱신됐습니다. 이 레지스트리 표시값을 이전 실행파일 버전으로 해석하지 않습니다.
- 설치 실행파일은 **6,346,240바이트**, SHA-256은 `98CBEA3C75345D326E9C7221022DBD0CF0B380D6FECF5E000481154EADCD4959`입니다. Authenticode 상태는 `NotSigned`입니다.
- 정식 설치본 UI에서 Antigravity 상태선 연결을 제거·재설치해 명령 대상이 설치 경로이며 debug 경로를 참조하지 않음을 확인했습니다. 실제 CLI 재시작 후 5개 quota가 새로 수집됐고, 관찰 시 수집 후 21초인 데이터와 최신 수집 표시를 확인했습니다. 개인 사용량 숫자·원본 CLI 입력은 이 문서에 복제하지 않습니다.
- 정식 설치본에서 다크→라이트 전환 후 native prefs가 light로 저장됐으며, 프로세스를 종료·재실행한 뒤 미니창에서 라이트 모드가 유지됐습니다.
- 검증을 마친 뒤 UI로 원래 다크 모드로 복귀했고 저장된 theme=dark를 확인했습니다. 테스트용 Antigravity CLI는 종료했으며 최종 SPECTRA 실행 인스턴스는 설치본 1개(ProductVersion 0.3.0)입니다. 라이트 유지 검사 당시 상태와 최종 복귀 상태를 구분합니다.

검증한 NSIS를 직접 실행한 결과·설치 버전·설치된 EXE 해시·실제 동작으로 설치를 확인했습니다. NSIS 내부 payload를 별도로 추출해 설치 EXE와 바이트 대조하는 검사는 수행하지 않았으며, 설치 EXE의 기록 해시를 그런 독립 대조 결과로 표현하지 않습니다.

## 실제 화면·연결 확인

| 시나리오 | 상태 | 확인한 표면과 결과 |
|---|---|---|
| 다크·라이트 전환 | PASS(정식 설치본) | 설치된 앱에서 다크→라이트 전환과 native prefs light 저장 확인 |
| 설정 접근 | PASS(정식 설치본) | 설정을 고정 위치에서 열 수 있음 |
| 미니창 → 대시보드 | PASS(개발 실행) | 미니창 버튼으로 대시보드 전환 확인 |
| 구독 모델 | PASS(정식 설치본) | Antigravity Google AI Pro·모델군 3개·한도 창 5개 표시 확인. Codex 구독 화면은 개발 실행에서도 확인. 개인 계정 식별정보는 기록에 복제하지 않음 |
| 오늘 | PASS(정식 설치본) | 2026-11-02 범위로 내용이 변경됨 |
| 최근 7일 | PASS(정식 설치본) | 2026-10-27~11-02 범위로 내용이 변경됨 |
| 최근 30일 | PASS(정식 설치본) | 2026-10-04~11-02 범위로 내용이 변경됨 |
| Antigravity 공식 CLI | PASS(실연동) | 설치된 CLI 1.4.12의 로그인·usage 화면 확인 후 공식 상태선 전달까지 검증 |
| Antigravity → SPECTRA 전달 | PASS(실연동) | Windows 명령 인용 실패 재현 후 EncodedCommand 래퍼로 해결. 공식 CLI에서 Gemini 5시간/주간, Claude·GPT 5시간/주간, Argon 주간의 5개 quota 수신. 실제 `3p-*`, `gemini-argon-wtu-weekly` 버킷 alias 확인. 개인 사용량 숫자는 공개 기록에 복제하지 않음 |
| Antigravity 모델군 표시 | PASS(정식 설치본) | Google AI Pro 구독·모델군 3개·5시간/주간 전환 확인. Argon에는 주간 창만 표시됨 |
| 재실행·테마 유지 | PASS(정식 설치본) | 설치된 프로세스를 종료·재실행한 뒤 미니창 라이트 선택 유지 확인 |
| 설치 경로 상태선 연결 | PASS(정식 설치본) | UI로 제거·재설치한 연결 명령이 설치 EXE를 가리키며 debug EXE를 가리키지 않음. 실제 CLI 재시작 후 5개 quota의 새 캡처와 UI 최신 수집 표시 확인 |

## 독립 검토와 처분

| 식별자 | 지적 | 처분 |
|---|---|---|
| N-001 | 24,000개 이력 상한이 9개 한도창의 30일을 보존하지 못함 | accepted: 32,000개 상한, 31일 26,784개 회귀 검사 통과 및 재검토 해소 |
| N-002 | 기존 비활성 상태선 명령이 재실행될 수 있음 | accepted: 실행 배제, 기본 상태선 유지 및 원래 설정 복원 검사 통과, 재검토 해소 |
| N-003 | 기존 상태선 명령의 선두 `~` 홈 경로 확장 누락 | accepted: Antigravity 경로에서만 확장·인용, 인수와 원본 백업 유지 검사 통과, 재검토 해소 |
| F-001 | 첫 Antigravity 연결에서 인증 상태 unknown이 브리지 설치를 차단 | accepted: 설치 우선 분기 및 회귀 검사 반영, 독립 재검토에서 해소 확인 |
| F-002 | 좁은 화면의 사이드바 버튼 접근성 이름 소실 | accepted: 항상 접근성 이름을 지정하도록 수정, 독립 재검토에서 해소 확인 |
| F-003 | CLI 새 캐시 반영과 5분 경과 최신성 표시 누락 | accepted: 갱신 및 최신성 처리를 수정, 독립 재검토에서 해소 확인. 실제 CLI 전달 및 모델군 표시도 확인 |
| F-004 | 같은 미니 모드 재열기 시 실제 고정 상태와 버튼 상태 불일치 | accepted: 모드 이벤트와 고정 상태를 맞추도록 수정, 독립 재검토에서 해소 확인 |

## 연결 방식과 경계

Antigravity의 공식 CLI statusLine 인터페이스에서 quota·plan을 수신합니다. 공급자 계정 토큰을 복제하지 않으며 입력 전문·이메일·작업 제목·작업 경로는 캐시에 저장하지 않습니다. 기존 사용자 상태선은 활성 상태일 때만 연결해 실행하고, 해제 시 복원합니다.

GUI만 사용하는 동안에는 새 사용량을 받지 못하며, 공식 CLI가 상태선을 실행해야 값이 갱신됩니다. 실제 CLI 1.4.12의 상태선에서 전달된 버킷 이름을 확인해 모델군과 한도 창에 연결했습니다. 이번 실연동 성공은 공식 CLI 전달에 대한 증거이며 GUI 직접 조회 지원을 의미하지 않습니다. 시안 숫자나 브라우저 예시는 연결 성공 증거로 사용하지 않습니다.

## 완료 판정과 검증 경계

요청한 수정·검증·커밋·푸시·릴리즈·로컬 설치 및 핵심 동작 확인을 완료했습니다. 전체 PASS는 위에 명시한 Windows 설치형 범위에 대한 판정입니다.

- 릴리즈 바이너리의 소스·태그 커밋은 `a984f92855eb96dcddd3488236d2a2a40027c2c4`로 유지합니다. 후속 main의 keeper v0.3 지원 스크립트 2줄 변경과 QA 문서 보강은 태그의 빌드 산출물을 변경하지 않습니다.
- 업데이터 서명은 검증됐으나 Windows Authenticode 게시자 코드 서명은 없습니다.
- 설치 패키지 내부 payload의 별도 추출 대조는 미실시입니다. 검증된 NSIS 실행, 설치 버전·해시 기록, 정식 설치 경로의 프로세스와 실제 기능을 확인한 범위입니다.
- Antigravity GUI 직접 조회는 지원하지 않으며 공식 CLI 상태선 실행이 필요합니다. 기록되지 않은 과거 이력을 생성하지 않습니다.
