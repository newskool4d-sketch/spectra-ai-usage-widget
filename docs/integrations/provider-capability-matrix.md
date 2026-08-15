# 공급자 연결 capability matrix

검토일: 2026-08-16

SPECTRA의 활성 공급자는 **Claude와 OpenAI Codex** 두 가지입니다. 개인 구독 잔여량과 조직 API 사용량을 하나의 숫자로 섞지 않으며, 공식 endpoint가 확인되기 전에는 화면 숫자를 예시 snapshot으로 유지합니다.

| 공급자 | 공식 인증 경계 | 공식 데이터 범위 | 제품 상태 |
| --- | --- | --- | --- |
| Codex | ChatGPT 개인 요금제의 공개 OAuth·잔여량 endpoint 확인 불가 | OpenAI Usage API는 조직 API 사용량이며 Admin API 키 필요 | 개인 구독은 공식 사용량 페이지 안내 · 조직 API 사용량은 별도 연결 대상 |
| Claude | Claude.ai Pro/Max 개인 요금제 OAuth·잔여량 endpoint 확인 불가 | Anthropic Admin Usage Report는 조직 API 사용량이며 Admin API 키 필요 | 개인 구독은 공식 앱/사용량 경계 안내 · 조직 API 사용량은 별도 연결 대상 |

## 구현 경계

- `ProviderCapability`는 인증 방식과 쿼터 범위를 `PlanQuota` 예시 snapshot과 분리합니다.
- `planQuotaVerified`가 `true`가 되기 전에는 실제 계정 수치로 표시하지 않습니다.
- 브라우저 빌드는 데모 흐름만 제공하며, 비공개 계정 세션·쿠키·내부 endpoint를 읽지 않습니다.
- Tauri/Swift callback·OS vault 코드는 공식 OAuth endpoint가 공개될 때 사용할 안전 경계로 보존하지만, 현재 두 provider에는 authorize URL을 하드코딩하지 않습니다.
- `CredentialVault`는 Windows Credential Manager·macOS Keychain·iOS Keychain 경계를 유지하며 token을 React 상태나 이벤트 payload에 넣지 않습니다.
- 조직 API 사용량을 연결할 경우에도 Admin API 키는 OS vault 또는 보안 런타임에만 주입하고, 개인 구독 잔여량으로 표시하지 않습니다.

## 공식 참고

- [Codex and ChatGPT plan limits](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan)
- [OpenAI Usage API](https://platform.openai.com/docs/api-reference/usage)
- [Claude Code with Pro or Max](https://support.anthropic.com/en/articles/11145838-using-claude-code-with-your-pro-or-max-plan)
- [Anthropic usage report](https://docs.anthropic.com/en/api/admin-api/usage-cost/get-messages-usage-report)
- [Claude subscription and API Console are separate](https://support.anthropic.com/en/articles/9876003-i-subscribe-to-a-paid-claude-ai-plan-why-do-i-have-to-pay-separately-for-api-usage-on-console)
