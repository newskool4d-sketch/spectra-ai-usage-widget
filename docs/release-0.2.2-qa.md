# SPECTRA v0.2.2 검증

대상: `feat/taskbar-usage-strip`, Windows x64 NSIS, 개선 항목 1(배포 연결)·3(Claude 최신성 표시).

## 코드 검증 결과 (2026-09-14)

| 검사 | 결과 |
|---|---|
| `npm test` | 47개 통과 |
| `npm run build` | 통과 |
| `npm run verify:release` | 버전·잠금 파일·릴리즈 노트 통과 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked` | 70개 통과, GUI 8개 별도 실행 |
| 위 명령에 `--features native-oauth` 추가 | 72개 통과 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked taskbar_window::tests -- --ignored --test-threads=1` | Windows GUI 8개 통과 |
| `git diff --check` | 통과 |

Windows 테스트는 툴팁 컨트롤 문자열 소비·갱신·hover 메시지 연결·숨김·UTF-16 버퍼 수명·창 10회 생성/파기를 검증합니다. 실제 마우스 hover 육안 확인과 앱 내 업그레이드는 잠금 화면으로 미검증입니다.

## 검증 경계

- 기존 공개 v0.2.1은 updater가 없으므로 수동 설치가 필요합니다.
- 현재 PC의 별도 재빌드 v0.2.1은 updater를 포함하고 있어 앱 내 업그레이드 시험 대상으로 유지합니다.
- GitHub Actions 서명 키는 기존 로컬 키와 동일한 공개키를 사용하는 키입니다. 개인키 내용은 저장소나 검증 문서에 기록하지 않습니다.
- 릴리즈는 버전·테스트·서명 파일·manifest·업로드 SHA-256 일치 검사 후 초안에서 공개로 전환됩니다.
- `.exe.sig`는 Tauri updater 검증용입니다. Windows Authenticode 인증서는 별개이며 현재 제공하지 않습니다.

## Claude 표시 기준

- 실제 데이터 수집 시각을 마지막 동기화로 표시합니다. 실패한 조회 시도는 동기화 시각을 새로 만들지 않습니다.
- 직접 조회 성공, 캐시 대기, 초기화 시각 경과, 로그인 필요, 설치 필요, 연결 오류를 구분합니다.
- 캐시 값을 유지하되 직접 조회 실패 시 갱신 대기로 표시합니다. 토큰 만료·401 안내는 실제 CLI 로그인 상태와 구분합니다.
- 창이 없어도 기존 15초 루프에서 툴팁의 경과 시간·초기화 상태가 갱신됩니다. 이번 작업은 사용량 폴링 주기를 추가하지 않습니다.

## 실제 업그레이드 확인 절차

1. 설치된 updater 포함 v0.2.1에서 창을 열어 v0.2.2 알림을 확인합니다.
2. `지금 설치` 후 서명 검증·다운로드·NSIS 설치·자동 재시작을 확인합니다.
3. 설치 경로의 제품 버전 0.2.2, 프로세스 경로, 미니 창, 스트립 툴팁을 확인합니다.
4. `ui-prefs.json`과 `claude-statusline-bridge.json`의 전후 해시를 비교합니다. 실제 사용량 캐시는 조회 결과에 따라 정상적으로 변할 수 있습니다.
5. v0.2.2에서 다시 확인하면 같은 버전을 재설치하도록 안내하지 않는지 확인합니다.

코드 테스트·서명 검사·공개 다운로드 성공은 이 실제 앱 내 업그레이드 시험을 대신하지 않습니다. 잠금 화면에서는 UI 입력을 중단하고 실제 화면 검증은 미검증으로 남깁니다.
