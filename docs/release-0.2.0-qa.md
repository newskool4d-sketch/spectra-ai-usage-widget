# SPECTRA 0.2.0 Windows QA

검증일: 2026-08-16  
판정: **PASS — Windows 설치·Codex·Claude 실데이터**

## 빌드 산출물

- NSIS: `src-tauri/target/release/bundle/nsis/SPECTRA_0.2.0_x64-setup.exe`
- 크기: 2,116,295 bytes
- SHA-256: `2D3AD0AD78823222CC3C1B59D9E68611FBC4776DE90EFF10120BF4C991475809`
- Authenticode: `NotSigned`
- 로컬 설치: `%LOCALAPPDATA%\SPECTRA\spectra-native.exe`
- 설치 버전: `0.2.0`
- 제거 프로그램: 확인

## 자동 검증

- `npm run build`: PASS
- `npm run verify:baseline`: PASS
- `npm run verify:tokens`: PASS
- `npm run verify:memory`: PASS — JS 225,166 bytes, 1 asset
- `cargo test`: PASS — 14 passed
- `cargo clippy --all-targets -- -D warnings`: PASS
- NSIS production build: PASS

## 실제 계정·화면 검증

### Codex

- 설치된 릴리스 진단: `runtimeAvailable=true`, `authState=signed-in`, `authMethod=chatgpt`
- `source=codex-app-server`, 주간 한도 창·사용률·초기화 epoch 수신
- 미니 창에서 실제 잔여량·초기화까지의 시간을 표시
- 연결 대화상자의 수동 새로고침 정상
- 이메일·token·계정 ID는 진단·React payload에 없음

### Claude

- 설치된 릴리스 진단: `runtimeAvailable=true`, `authState=signed-in`, `planType=max`
- SPECTRA 연결 대화상자에서 사용자가 요청한 opt-in 브리지를 설치하고 `~/.claude/settings.json`에 status line 명령을 등록
- 기존 상태선 백업 파일 생성 및 기존 상태선 출력 보존 확인
- 대화형 Claude Code 실제 응답 후 status line: `5h 1% · 7d 47%` 사용
- 설치된 SPECTRA 진단: 5시간 `used=1%, remaining=99%`, 주간 `used=47%, remaining=53%`
- 5시간 초기화 시각과 주간 초기화 시각 epoch 수신, SPECTRA 화면에 5시간 잔여 `99%` 표시
- 이메일·token·세션 원문은 SPECTRA 캐시와 진단에 없음

### Windows 셸

- 430×720 미니 창: PASS
- 현재 날짜·시간 표시: PASS
- 연결 CTA와 하단 내비게이션 겹침 수정: PASS
- 닫기 후 프로세스 트레이 상주: PASS
- 두 번째 실행으로 창 복원: PASS
- 복원 후 `spectra-native.exe` 단일 프로세스: PASS
- 로컬 0.2.0 silent upgrade와 설치 후 GUI: PASS
- Claude status line 설치·실제 응답·5시간/7일 동기화: PASS

## 보안·메모리 경계

- Codex App Server JSONL 총 입력: 2MiB 상한
- JSON 설정·캐시 파일: 4MiB 상한
- Claude status line stdin: 1MiB 상한
- 기존 status line stdout: 64KiB 상한, 실행 시간 2초 제한
- Claude 캐시: rate limit 사용률·초기화 시각·캡처 시각만 저장
- 기존 Claude 상태선이 설치 후 바뀌면 제거 과정에서 덮어쓰지 않음

## 남은 위험

- Claude 사용률과 초기화 시각은 공급자 응답에 따라 변동하므로, 장기 사용 후 새로고침 회귀 QA가 필요함.
- 설치 파일은 코드 서명되지 않아 SmartScreen 경고가 날 수 있음.
- Codex Security 워크벤치는 한글 사용자 경로를 CP949로 디코딩하는 플러그인 오류로 시작하지 못함. 대신 이번 diff를 수동 검토해 입력 상한과 timeout을 보강했지만, 정식 SARIF 보고서는 없음.
- GitHub `main` push 및 저장소 Public 전환: PASS
- `v0.2.0` Release·설치 파일 asset 업로드: 공개용 Code Signing 인증서 확보 후 진행
- macOS/iOS는 현재 Windows 환경에서 NOT_RUN.
