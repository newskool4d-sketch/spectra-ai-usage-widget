# SPECTRA Apple OAuth boundary

이 폴더는 iOS와 macOS 앱에서 공유할 수 있는 Swift 네이티브 인증 경계입니다. 현재 활성 공급자는 Claude와 Codex이며, 개인 구독 OAuth endpoint가 공개될 때까지 실제 로그인 흐름은 비활성 상태입니다.

## 구성

- `OAuthSessionCoordinator.swift`: `ASWebAuthenticationSession`으로 시스템 브라우저 로그인을 열고 `spectra://oauth/callback`을 받습니다.
- `OAuthCallback.swift`: scheme, host, path, query 중복, `state`, fragment를 검증합니다.
- `KeychainCredentialVault.swift`: access/refresh token JSON을 `kSecClassGenericPassword`로 저장합니다. 접근성은 `kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly`입니다.

## 앱 설정

1. iOS/macOS 앱의 URL Types에 `spectra` scheme을 등록합니다.
2. `SpectraOAuthSessionCoordinator`에 현재 화면의 `UIWindow` 또는 `NSWindow`를 presentation anchor로 설정합니다.
3. 공급자별 authorize URL과 client ID는 공식 문서가 공개된 경우에만 앱 환경별 설정으로 주입하고, client secret은 앱 번들에 넣지 않습니다.
4. 현재 Codex/Claude 구성은 authorize/token URL을 비워 두고 조직 사용량 endpoint 범위만 선언합니다. 공개 endpoint가 추가될 때만 callback 뒤 code 교환과 `SpectraKeychainCredentialVault.write` 호출을 활성화합니다.

웹뷰에는 code, access token, refresh token을 전달하지 않습니다. UI에는 연결 상태와 마지막 확인 시각만 전달합니다.
