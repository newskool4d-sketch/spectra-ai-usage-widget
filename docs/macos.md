# SPECTRA for macOS

Windows 포커스 도크·대시보드와 같은 React 화면을 사용하며 미니 창의 다크/라이트 설정을 유지합니다. 상단 메뉴 막대에서 수치를 확인하고 미니 창·대시보드를 열 수 있도록 macOS 전용 처리를 추가했습니다.

2026-11-07 갱신: Windows 공개 v0.3.4까지의 변경(작업표시줄·트레이·메뉴 막대의 AG 값 Gemini 기준, Antigravity 캐시 하루 유지, 대시보드 모델군 순서 고정)을 Mac 구현에 병합해 버전을 v0.3.4로 맞췄습니다. 이 커밋의 Mac CI 실행 결과와 설치파일 해시는 [검증 기록](./macos-implementation.md)에 기록합니다. 실행 전까지는 v0.3.4 Mac 패키지가 없습니다.

2026-11-13 수정: Mac에서도 Claude 로그인 정보를 로그인 키체인(`Claude Code-credentials`)에서 읽어 공식 OAuth 사용량 endpoint를 직접 조회합니다. 이전에는 Mac에서 상태선 브리지만 사용해 `Fable` 같은 모델별 한도가 표시되지 않았고, 예전 버전이 남긴 만료된 `~/.claude/.credentials.json`이 있으면 Claude 값이 갱신되지 않았습니다. 상세 경계는 [자격 증명 경계](./integrations/native-oauth.md)를 참고합니다.

2026-11-03 시험판(`803ba25`, v0.3.1): [Mac 빌드 실행](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41531866262)에서 Apple Silicon·Intel의 앱/DMG 생성 및 패키지 검사가 통과했습니다. 각 설치파일과 해시는 [검증 기록](./macos-implementation.md)에 있습니다. 실제 Mac 화면·계정 연결 검증과 Apple 공증은 아직 완료하지 않았습니다.

## 빌드 대상과 현재 검증 범위

| 항목 | 대상 / 상태 |
|---|---|
| Apple Silicon | `aarch64-apple-darwin`, arm64 `.app`·`.dmg` |
| Intel Mac | `x86_64-apple-darwin`, x86_64 `.app`·`.dmg` |
| 최소 OS 설정 | macOS 13.0; 최소 버전 실기기 호환성은 별도 확인 필요 |
| CI 실행 환경 | macOS 15, 각 아키텍처에서 네이티브 테스트·패키징 |
| 서명 | ad-hoc 서명. Apple Developer ID 인증서 서명이 아님 |
| Apple 공증 | 미적용 |
| 자동 업데이트 | Mac에서는 비활성화; Windows용 `latest.json`을 조회하지 않음 |
| Mac 실기기 화면 확인 | 미검증. 메뉴 막대·다중 모니터·테마는 실제 Mac에서 확인 필요 |

워크플로우가 존재한다는 사실만으로 Mac 빌드가 통과한 것은 아닙니다. 해당 커밋의 **Build SPECTRA for macOS** 실행 결과가 성공했는지 확인하세요. CI는 공개 릴리스를 만들거나 Windows 최신 릴리스를 수정하지 않으며, 검증을 통과한 설치파일을 Actions 아티팩트로 14일간 보관합니다.

## Mac에서 직접 빌드

Xcode Command Line Tools, Node.js 24, Rust stable이 필요합니다. SPECTRA가 읽는 Codex·Claude·Antigravity CLI의 설치와 로그인은 각 공급자의 공식 도구에서 진행합니다.

```sh
npm ci
npm test
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
npm run desktop:build:mac -- -- --locked
```

이 명령은 현재 Mac의 아키텍처로 빌드합니다. 대상 아키텍처를 명시하려면 다음 중 하나를 사용합니다.

```sh
# Apple Silicon
rustup target add aarch64-apple-darwin
npm run desktop:build:mac -- --target aarch64-apple-darwin -- --locked

# Intel
rustup target add x86_64-apple-darwin
npm run desktop:build:mac -- --target x86_64-apple-darwin -- --locked
```

기본 빌드 결과는 `src-tauri/target/release/bundle/`, 대상 지정 시 `src-tauri/target/<target>/release/bundle/`에 생성됩니다. `macos/SPECTRA.app`은 실행 앱이며 `dmg/`에는 설치용 디스크 이미지가 있습니다. Mac 빌드는 macOS에서 수행해야 합니다.

## CI 설치파일 사용

1. GitHub Actions의 **Build SPECTRA for macOS**에서 사용할 커밋과 두 아키텍처의 결과를 확인합니다.
2. `SPECTRA-macos-arm64-<commit>` 또는 `SPECTRA-macos-x86_64-<commit>` 아티팩트를 받습니다. Mac의 **이 Mac에 관하여**에서 Apple 칩이면 arm64, Intel 프로세서이면 x86_64를 선택합니다.
3. ZIP을 푼 뒤 같은 폴더에서 `shasum -a 256 -c SHA256SUMS.txt`로 파일을 대조합니다.
4. DMG를 열어 SPECTRA를 응용 프로그램 폴더에 복사하고 실행합니다. 대안인 `.app.zip`도 앱 권한과 리소스를 보존한 패키지입니다.

현재 패키지는 Developer ID 서명과 Apple 공증이 없으므로 macOS가 처음 실행을 차단할 수 있습니다. 출처와 해시를 확인한 경우에만 시스템 설정의 **개인정보 보호 및 보안 → 확인 없이 열기**로 해당 앱의 실행을 허용하세요. 조직 관리 Mac에서는 관리자 정책으로 실행이 제한될 수 있습니다. 시스템 전체의 Gatekeeper를 끄는 방식은 사용하지 않습니다.

## 검증과 배포 전 확인

CI는 각 아키텍처에서 프론트엔드·Rust 라이브러리 테스트, `.app`·`.dmg` 생성, Mach-O 아키텍처, 앱 식별자·버전, ad-hoc 서명 무결성과 DMG 무결성을 검사합니다. 패키징된 실행파일을 CLI 모드로 실행해 외부 연결 없이 응답하는지도 확인합니다. `build-info.txt`에 커밋·아키텍처·실행 OS와 검사 범위를 남깁니다. 이 검사는 사용자 계정 로그인이나 실제 메뉴 막대의 표시 품질을 검증하지 않습니다.

Mac에서 최종 확인할 항목:

- 메뉴 막대 수치와 공급자별 실제 사용량 대조, 데이터가 없는 경우 `—` 표시
- 메뉴 막대에서 미니 창·대시보드 열기, 숨기기·종료
- 미니 창 다크/라이트 전환과 재실행 후 설정 유지
- 창을 닫은 상태에서 사용량 갱신, 메모리 절약 대기 후 복원
- Retina·외부 모니터에서 창이 화면 밖으로 벗어나지 않는지 확인
- Finder에서 실행했을 때 공식 CLI를 탐색하고 연결할 수 있는지 확인

Mac 자동 업데이트를 활성화하려면 별도로 검증한 `darwin-aarch64`·`darwin-x86_64` 업데이트 아티팩트와 서명, 대상별 매니페스트를 마련해야 합니다. 현재 Windows 릴리스에 Mac 업데이트를 임의로 혼합하지 않습니다.

## 공식 근거

- [Tauri 플랫폼별 설정](https://v2.tauri.app/reference/config/#platform-specific-configuration)
- [Tauri macOS 앱 번들](https://v2.tauri.app/distribute/macos-application-bundle/)
- [Tauri DMG](https://v2.tauri.app/distribute/dmg/)
- [Tauri macOS ad-hoc 서명과 공증](https://v2.tauri.app/distribute/sign/macos/)
- [GitHub macOS 호스팅 러너](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
