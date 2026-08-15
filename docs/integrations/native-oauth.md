# 네이티브 OAuth callback과 OS Credential Vault

검토 기준일: 2026-08-16

이번 단계는 실제 공급자 계정에 로그인하거나 토큰을 발급하는 단계가 아닙니다. Tauri/Swift가 맡아야 할 **callback 검증·비밀값 보관 경계**를 먼저 고정하고, 공급자별 authorize URL·client ID·token 교환은 다음 adapter 작업으로 남겼습니다.

## 경계

```text
공급자 로그인 브라우저
        │  spectra://oauth/callback?code=...&state=...
        ▼
Tauri deep-link / ASWebAuthenticationSession
        │  scheme·host·path·state·만료 검증
        ▼
네이티브 provider adapter (다음 단계)
        │  code 교환·refresh
        ▼
Windows Credential Manager / macOS Keychain / iOS Keychain
        │  상태·마지막 확인 시각만
        ▼
React 화면
```

authorization code, access token, refresh token은 React 상태·`localStorage`·`sessionStorage`·Tauri 이벤트 payload에 넣지 않습니다.

## Tauri 데스크톱

- `src-tauri/tauri.conf.json`에 `spectra` custom scheme을 등록했습니다.
- `tauri-plugin-deep-link`가 앱 시작 시의 CLI URL과 실행 중의 open-url 이벤트를 받습니다.
- Windows/Linux에서 두 번째 프로세스가 생기지 않도록 `tauri-plugin-single-instance`의 `deep-link` feature를 사용합니다.
- `src-tauri/src/oauth_callback.rs`는 `spectra://oauth/callback` 외의 scheme/host/path를 거부하고, fragment·중복 `code/state`·`error` 응답·빈 값·state 불일치를 거부합니다. pending transaction은 10분 뒤 폐기됩니다.
- `src-tauri/src/credential_vault.rs`는 `keyring`의 네이티브 backend를 사용합니다. Windows에서는 Windows Credential Manager, macOS에서는 Keychain으로 연결되며, account key는 공급자 allowlist로 제한됩니다.
- 현재 Tauri command는 `oauth_prepare`, `credential_status`, `credential_remove`만 노출합니다. 토큰을 읽거나 쓰는 함수는 Rust 내부 경계에만 있고 command로 공개하지 않았습니다.

## iOS/macOS Swift

`native/apple/SpectraAuth`의 세 파일을 앱 target 또는 Swift package에 포함합니다.

- `OAuthSessionCoordinator`는 `ASWebAuthenticationSession`을 사용합니다. authorize URL은 HTTPS만 허용하고 presentation anchor를 명시적으로 요구합니다.
- `OAuthCallback` parser는 Tauri와 같은 callback 규칙과 state 검사를 적용합니다.
- `SpectraKeychainCredentialVault`는 `kSecClassGenericPassword`와 `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`를 사용합니다.
- URL Types에 `spectra` scheme을 등록해야 합니다. universal link/App Link는 별도 도메인 검증과 signing 설정이 필요한 다음 단계입니다.

## React bridge

`src/integrations/tauri-native-bridge.ts`는 `window.__TAURI__`가 있을 때만 native command/event를 호출합니다. 일반 브라우저에서는 기존 `demo-only` adapter를 유지합니다. Tauri 런타임에서 OAuth 카드를 열면 pending state를 native에 준비하고, 브라우저로 돌아온 callback은 `callback-accepted` 상태만 UI에 전달합니다. 아직 token exchange가 없으므로 UI를 실제 연결 완료로 표시하지 않습니다.

## 다음 구현 승인 지점

1. 공급자별 공식 OAuth authorize/token endpoint와 client ID를 확인합니다.
2. PKCE code verifier를 native adapter 안에서 사용해 code 교환과 refresh를 구현합니다.
3. 공급자별 **요금제 잔여량** endpoint가 실제로 같은 계정 범위를 제공하는지 검증합니다. API project/org 사용량과 개인 구독 잔여량을 섞지 않습니다.
4. 실기기에서 Windows Credential Manager, macOS Keychain, iOS Keychain의 저장·삭제·만료 갱신을 각각 확인한 뒤에만 `connected` snapshot으로 승격합니다.

## 검증 상태

- React/Vite build: 기존 회귀 명령으로 확인 대상
- Tauri Rust: `cargo fmt --check`, `cargo test`, `cargo check` 대상
- Swift: 현재 작업 환경(Windows)에 Swift/Xcode가 없어 이 회차에서는 `NOT_RUN`으로 기록
- 실제 공급자 로그인·토큰 교환·요금제 잔여량 API: 의도적으로 `NOT_RUN`
