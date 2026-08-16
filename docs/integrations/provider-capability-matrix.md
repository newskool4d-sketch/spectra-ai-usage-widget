# 공급자 연결 capability matrix

검토일: 2026-08-16

SPECTRA 0.2의 활성 공급자는 **OpenAI Codex**와 **Claude** 두 가지입니다. 데스크톱 앱은 공급자가 공식 문서로 제공하는 로컬 인터페이스만 사용하며, 웹 세션·쿠키·비공개 endpoint를 읽지 않습니다. 브라우저 미리보기는 예시 수치를 표시하지만 Tauri 앱은 실제 데이터가 없으면 퍼센트를 `—`로 표시합니다.

| 공급자 | 계정 연결 | 개인 요금제 잔여량 | SPECTRA 구현 |
| --- | --- | --- | --- |
| Codex | `codex login`의 ChatGPT 브라우저 로그인. 토큰 저장·갱신은 Codex가 관리 | App Server `account/rateLimits/read`의 `usedPercent`, `windowDurationMins`, `resetsAt` | 로컬 stdio App Server를 요청 때만 실행하고 응답 후 즉시 종료 |
| Claude | `claude auth login`의 Claude.ai 로그인. 토큰은 Claude Code가 관리 | 공식 status line 입력의 `rate_limits.five_hour`·`seven_day` | 사용자가 동의하면 상태선 브리지를 설치하고 한도 필드만 정제해 로컬 캐시 |

## 구현 경계

- `ProviderCapability`는 인증 방식과 쿼터 범위를 `PlanQuota` 화면 모델과 분리합니다.
- 실제 한도 창이 한 개 이상 확인된 경우에만 `confidence=verified`와 숫자를 표시합니다.
- Codex App Server 연결은 `initialize` → `initialized` 핸드셰이크 뒤 `account/read`와 `account/rateLimits/read`만 호출합니다.
- Claude 브리지는 `rate_limits.*.used_percentage`, `rate_limits.*.resets_at`, 캡처 시각만 저장합니다. 세션 ID, 작업 경로, 대화 기록, 토큰 수, 비용은 저장하지 않습니다.
- Claude의 기존 `statusLine` 설정은 설치 전에 보존하고 브리지 제거 시 복원합니다. 기존 명령이 있으면 그 출력도 이어서 표시합니다.
- React 상태·브라우저 저장소·Tauri 이벤트에는 provider access token이나 refresh token을 넣지 않습니다.
- 조직 API 사용량은 이번 개인 구독 잔여량 기능에 포함하지 않습니다.

## 호환성 메모

- Codex CLI 또는 Claude Code가 설치되지 않았으면 해당 공급자는 `공식 CLI 설치 필요`로 표시됩니다.
- API 키로 로그인한 Codex는 ChatGPT 구독 한도로 표시하지 않습니다.
- Claude `rate_limits`는 Pro/Max 구독자가 세션에서 첫 API 응답을 받은 뒤 제공됩니다.
- Claude 상태선 명령은 로컬 셸에서 실행되므로 Claude Code의 trust 승인이 적용될 수 있습니다.
- 공급자 CLI와 응답 스키마가 바뀔 수 있으므로 릴리스 전 실제 계정 회귀 검증이 필요합니다.

## 공식 참고

- [Codex App Server](https://developers.openai.com/codex/app-server)
- [Codex authentication](https://developers.openai.com/codex/auth)
- [Claude Code CLI usage](https://code.claude.com/docs/en/cli-usage)
- [Claude Code status line](https://code.claude.com/docs/en/statusline)
- [Claude Code with Pro or Max](https://support.claude.com/en/articles/11145838-use-claude-code-with-your-pro-or-max-plan)
