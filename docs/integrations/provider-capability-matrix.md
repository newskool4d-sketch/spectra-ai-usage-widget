# 공급자 연결 capability matrix

검토일: 2026-08-16

이 문서는 SPECTRA가 **요금제 잔여량**과 **API 프로젝트·조직 쿼터**를 혼동하지 않도록 하는 연결 기준선입니다. `src/integrations/provider-capabilities.ts`가 이 표의 제품 상태를 소비합니다.

| 공급자 | 공식 인증 근거 | 쿼터 범위 | 개인 요금제 잔여량 | 제품 상태 |
| --- | --- | --- | --- | --- |
| OpenAI | API 키 인증 문서 | 조직 Usage API | 미확인 | OAuth 데모만 허용 |
| Claude | API 키·Admin Usage 문서 | 조직/API 사용량 | 미확인 | OAuth 데모만 허용 |
| Gemini | Google OAuth 문서 | Google Cloud 프로젝트 | 미확인 | OAuth 경로는 문서화, 요금제는 별도 |
| Copilot | Microsoft Entra delegated auth | Graph/Copilot API 권한 | 미확인 | delegated 흐름만 문서화 |
| Cursor | 이번 회차 공식 경로 미확인 | 미확인 | 미확인 | 연결 버튼 비활성 |
| Perplexity | 이번 회차 공식 경로 미확인 | 미확인 | 미확인 | OAuth 데모만 허용 |

## 구현 경계

- `ProviderCapability`는 인증 방식과 쿼터 범위를 `PlanQuota` snapshot과 분리합니다.
- `planQuotaVerified`가 `true`가 되기 전에는 실제 계정 수치로 표시하지 않습니다.
- `OAuthAdapter`는 네이티브 셸의 redirect/callback 경계를 위한 계약만 제공합니다. 현재 web prototype의 adapter는 `demo-only` 결과만 반환합니다.
- `CredentialVault`는 Windows Credential Manager·macOS Keychain·iOS Keychain 구현이 들어갈 자리이며, 브라우저 저장소 fallback은 제공하지 않습니다.

## 공식 참고

- [OpenAI authentication](https://platform.openai.com/docs/api-reference/authentication)
- [OpenAI Usage API](https://platform.openai.com/docs/api-reference/usage)
- [Anthropic API access](https://support.anthropic.com/en/articles/8114521-how-can-i-access-the-anthropic-api)
- [Anthropic usage report](https://docs.anthropic.com/en/api/admin-api/usage-cost/get-messages-usage-report)
- [Gemini OAuth quickstart](https://ai.google.dev/gemini-api/docs/oauth)
- [Gemini API rate limits](https://ai.google.dev/gemini-api/docs/rate-limits)
- [Microsoft 365 Copilot API authentication](https://learn.microsoft.com/en-us/microsoft-365/copilot/extensibility/copilot-apis-security-authentication)
- [Microsoft Graph delegated access](https://learn.microsoft.com/en-us/graph/auth-v2-user)
