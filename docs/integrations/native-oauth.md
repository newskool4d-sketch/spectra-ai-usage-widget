# 네이티브 계정 연결과 자격 증명 경계

검토 기준일: 2026-08-16

SPECTRA 0.2의 Codex·Claude 연결은 앱 자체 OAuth client ID나 토큰 교환을 사용하지 않습니다. 사용자가 이미 신뢰하는 공식 CLI가 로그인·토큰 갱신·자격 증명 보관을 담당하고, SPECTRA는 계정 상태와 개인 요금제 한도 필드만 읽습니다.

```text
SPECTRA React 화면
        │ provider id만 Tauri command로 전달
        ▼
Rust provider_usage
        ├─ Codex CLI App Server ─ account/read + account/rateLimits/read
        └─ Claude Code ─ auth status + opt-in statusLine bridge
        │ 토큰·이메일·세션 원문 제외
        ▼
PlanQuota: 사용률·잔여률·초기화 시각·최신성
```

## Codex

1. 설치 여부를 로컬 `PATH`에서 확인합니다.
2. 로그아웃 상태에서는 `codex login`을 시작해 공식 브라우저 로그인을 사용합니다.
3. 조회 때마다 `codex app-server --stdio`를 한 번 실행합니다.
4. `initialize` 응답을 받은 뒤 `initialized`를 보내고 `account/read`를 호출합니다.
5. ChatGPT 계정이면 `account/rateLimits/read`의 한도 창만 파싱합니다.
6. 응답 또는 12초 제한 뒤 App Server 자식 프로세스를 종료합니다.

SPECTRA는 Codex의 `email`, access token, refresh token, 계정 ID를 직렬화하거나 화면에 전달하지 않습니다. Codex 자격 증명은 Codex 설정에 따라 OS keyring 또는 Codex 전용 로컬 저장소에 남습니다.

## Claude

1. `claude auth status`로 설치·로그인·구독 종류만 확인합니다.
2. 사용자가 화면에서 동의한 경우에만 `~/.claude/settings.json`의 `statusLine.command`를 SPECTRA 브리지로 바꿉니다.
3. 기존 상태선 객체와 명령은 `%LOCALAPPDATA%\SPECTRA\claude-statusline-bridge.json`에 보존합니다.
4. 브리지는 stdin 원문에서 `rate_limits.five_hour`·`seven_day`만 추출해 `%LOCALAPPDATA%\SPECTRA\claude-usage.json`에 저장합니다.
5. 기존 상태선 명령이 있으면 같은 stdin을 넘기고 기존 출력을 그대로 사용합니다.
6. 제거 시 기존 `statusLine` 객체를 복원합니다.

Claude 상태선 입력에는 작업 경로와 세션 정보도 포함될 수 있지만, SPECTRA 캐시에는 한도 사용률·초기화 epoch·캡처 시각만 남습니다. 브리지는 첫 Claude 응답 이후 값을 받으며, 24시간이 지났거나 초기화 시각을 넘긴 캐시는 `stale`로 표시합니다.

## 기존 OAuth callback·OS vault 코드

`src-tauri/src/oauth_callback.rs`, `credential_vault.rs`, `provider_connection.rs`와 `native/apple/SpectraAuth`는 향후 독립 OAuth provider 또는 iOS 동기화를 위한 안전 경계로 남겨 둡니다. 현재 Codex·Claude 개인 요금제 흐름에서는 이 vault에 provider token을 복제하지 않습니다.

## 검증 상태

- React/Vite production build: `PASS`
- Rust 단위 테스트: `PASS` — 14개, App Server 응답 파싱·Claude 필드 정제·상태선 복원·입력 상한 포함
- Windows 0.2.0 설치·미니 창·트레이 복원·Codex 실제 한도: `PASS`
- Claude Max 로그인·미연결 숫자 숨김: `PASS`
- Claude 브리지 설치·첫 실제 5시간/7일 값·SPECTRA 화면 동기화: `PASS`
- Claude 브리지 제거·기존 상태선 복원: `NOT_RUN` — 현재 사용을 위해 브리지를 유지
- macOS/iOS Swift·WidgetKit: Windows 환경에서 `NOT_RUN`
- 코드 서명·자동 업데이트: `NOT_CONFIGURED`
