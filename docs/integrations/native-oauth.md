# 네이티브 인증 callback과 OS Credential Vault

검토 기준일: 2026-08-16

현재 활성 공급자는 Claude와 Codex입니다. 두 공급자 모두 개인 구독의 잔여량을 반환하는 공개 OAuth/API endpoint가 확인되지 않았으므로, SPECTRA는 비공개 웹 세션·쿠키·내부 endpoint를 연결하지 않습니다. 아래 네이티브 경계는 향후 공식 endpoint가 공개될 때만 활성화할 수 있도록 안전하게 보존합니다.

## 현재 제품 경계

```text
Claude / Codex 공식 사용량 페이지 또는 조직 Admin API
        │  개인 잔여량과 조직 API 사용량을 별도 분류
        ▼
React 화면
        │  공식 endpoint가 검증되지 않으면 예시 snapshot 유지
        ▼
OS Credential Manager / macOS Keychain / iOS Keychain
        │  credential 상태만 반환
        ▼
SPECTRA 위젯
```

authorization code, access token, refresh token은 React 상태·`localStorage`·`sessionStorage`·Tauri 이벤트 payload에 넣지 않습니다.

## Tauri 데스크톱

- `src-tauri/src/oauth_callback.rs`는 state·만료·scheme·host·path·fragment·중복 query를 검증하는 callback 경계를 유지합니다.
- `oauth_prepare`는 현재 Codex/Claude에 대해 `api-key-only`와 `callbackMode=none`을 반환하며 pending OAuth transaction을 만들지 않습니다.
- `credential_status`, `credential_remove`만 웹뷰에 상태를 반환합니다. token·authorization code·refresh token은 이벤트 payload에 넣지 않습니다.
- 공개 OAuth endpoint가 추가되기 전에는 시스템 브라우저를 열거나 client ID를 요구하지 않습니다.

## iOS/macOS Swift

`native/apple/SpectraAuth`의 callback parser, `ASWebAuthenticationSession` 경계, Keychain vault는 향후 공식 OAuth를 위한 공통 코드입니다.

- `OAuthProviderConfiguration`은 Codex/Claude의 현재 조직 사용량 endpoint와 `organization-usage-only` 상태를 선언합니다.
- 두 provider의 개인 구독용 `authorizeURL`·`tokenURL`은 의도적으로 `nil`입니다.
- `OAuthTokenExchange`는 token endpoint가 없으면 교환하지 않고 종료합니다.
- `SpectraKeychainCredentialVault`는 `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`를 사용합니다.

## 조직 API 사용량 모드

조직 사용량을 별도 기능으로 추가할 경우 다음 원칙을 지킵니다.

1. OpenAI Usage API 또는 Anthropic Admin Usage Report의 Admin API 키를 OS vault/보안 런타임에만 주입합니다.
2. 조직 API 사용량을 ChatGPT/Codex 또는 Claude Pro/Max 개인 잔여량으로 표시하지 않습니다.
3. `source=official-usage`, `scope=organization`처럼 데이터 범위를 모델에 명시합니다.
4. API 키·응답 원문·토큰을 React 이벤트나 로그에 남기지 않습니다.

## 다음 활성화 조건

1. 공급자가 개인 구독용 공식 OAuth authorize/token endpoint를 공개합니다.
2. 같은 계정 범위의 잔여량·reset 시각 endpoint가 문서화됩니다.
3. client ID·redirect URI 정책과 native 앱 배포 조건을 검증합니다.
4. 실제 계정으로 Windows Credential Manager, macOS Keychain, iOS Keychain 저장·삭제·만료 갱신을 검증합니다.

## 검증 상태

- React/Vite build: 변경 후 실행
- Tauri Rust: `cargo fmt --check`, `cargo test`, `cargo check` 실행
- Swift: 현재 작업 환경(Windows)에 Swift/Xcode가 없어 `NOT_RUN`
- 실제 공급자 로그인·토큰 교환: 공개 개인 구독 endpoint가 없어 `NOT_RUN`
- 개인 요금제 잔여량 endpoint: Codex·Claude 모두 `NOT_PUBLISHED`
