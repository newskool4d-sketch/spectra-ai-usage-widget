# SPECTRA v0.2.2 검증

대상: `feat/taskbar-usage-strip`, Windows x64 NSIS, 개선 항목 1(배포 연결)·3(Claude 최신성 표시).

## 코드 검증 결과 (2026-09-14)

| 검사 | 결과 |
|---|---|
| `npm test` | 태그 코드 47개 통과, 초안 조회 회귀 테스트 추가 후 48개 통과 |
| `npm run build` | 통과 |
| `npm run verify:release` | 버전·잠금 파일·릴리즈 노트 통과 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked` | 70개 통과, GUI 8개 별도 실행 |
| 위 명령에 `--features native-oauth` 추가 | 72개 통과 |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --locked taskbar_window::tests -- --ignored --test-threads=1` | Windows GUI 8개 통과 |
| `git diff --check` | 통과 |

Windows 테스트는 툴팁 컨트롤 문자열 소비·갱신·hover 메시지 연결·숨김·UTF-16 버퍼 수명·창 10회 생성/파기를 검증합니다. 최초 검증에서는 잠금 화면으로 실제 UI 확인을 보류했습니다. 잠금 해제 후 앱 내 업그레이드와 새 버전 화면은 확인했으며, 작업 표시줄의 실제 마우스 hover 육안 확인은 남아 있습니다.

## 로컬 패키지 및 실제 조회

- 릴리즈 코드 커밋: `dd94c179a6e9c35ab318bc66215c812deb9c3efe`, 태그 `v0.2.2`.
- `npm run desktop:build -- --ci -- --locked`: 통과.
- 로컬 NSIS: `SPECTRA_0.2.2_x64-setup.exe`, 2,841,628 bytes.
- 로컬 설치 파일 SHA-256: `7E566A71B72610808B36149830F7612AFBA04E51B6AB41FA0C054F99D6FCB2A5`.
- 같은 Tauri updater 버전이 사용하는 `minisign-verify`로 로컬 패키지와 `.sig`를 설정 공개키에 대조: 통과.
- 새 실행 파일의 실제 Claude 진단: `signed-in`, `connected`, `claude-usage-api`, 한도 창 2개. 실패한 요청이 아닌 실제 직접 조회 성공을 확인했습니다. 계정 식별자·토큰·잔여 수치는 이 문서에 기록하지 않습니다.
- Windows Authenticode: `NotSigned`. Tauri 서명 성공을 Windows 게시자 인증으로 표현하지 않습니다.

로컬 빌드와 GitHub 빌드는 도구 체인·패키징 시각이 달라 파일 해시가 다를 수 있습니다. 공개 배포본은 별도로 다운로드해 검증합니다.

## 공개 배포 검증

- [GitHub 실행 34807046946](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/34807046946): 프런트엔드·Rust 테스트, NSIS 빌드와 서명 업로드 통과. 마지막 검증 단계는 초안을 `releases/tags/<tag>`로 조회해 404로 실패했습니다. **전체 CI 성공으로 표시하지 않습니다.**
- `gh release view --json databaseId`로 초안을 식별한 후 `releases/<id>`를 조회하도록 검증 스크립트를 수정했습니다. 회귀 테스트 및 실제 초안의 파일·manifest·서명·업로드 digest 검사를 통과했습니다.
- 기존 CI 산출물을 교체하지 않고 체크섬 파일을 추가한 뒤 수동으로 공개했습니다. 태그를 이동하거나 CI를 재실행해 공개 설치 파일을 덮어쓰지 않았습니다. 수정된 스크립트의 전체 태그 CI 재실행은 다음 릴리즈에서 확인해야 합니다.
- [공개 v0.2.2](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.2): 2026-09-14 13:56 KST, draft=false, prerelease=false, 최신 릴리즈.
- 공개 자산: NSIS `.exe`, `.exe.sig`, `latest.json`, `SHA256SUMS.txt`.
- 공개 설치 파일: 2,834,147 bytes, SHA-256 `EE8C68DD680579EEDCC6A4C6847D66D2320DB73C743AF9D1A746AC1E887CFDD8`.
- 인증 헤더 없이 `releases/latest/download/latest.json` 조회 HTTP 200 및 버전 0.2.2 확인.
- manifest의 설치 파일 URL을 updater와 같은 `Accept: application/octet-stream`으로 비인증 다운로드하고 GitHub digest·공개 체크섬·설정 공개키 서명을 모두 대조: 통과.

## 실제 앱 내 업그레이드 (2026-09-14 15:37 KST)

- 사용자의 설치 승인 및 잠금 해제 후, 설치된 updater 포함 v0.2.1에서 시험했습니다. 이전 조회 상태를 해소하기 위해 정확한 설치 경로의 SPECTRA 프로세스만 종료하고 다시 실행했습니다.
- 실제 앱 화면에서 `SPECTRA 새 버전 0.2.2` 및 `지금 설치`를 확인하고 해당 버튼을 눌렀습니다. 별도의 설치 파일 실행 없이 앱 내 updater로 설치 후 자동 재시작됐습니다.
- 설치 파일 경로 `C:\Users\홍주형\AppData\Local\SPECTRA\spectra-native.exe`의 ProductVersion·FileVersion 모두 `0.2.2`를 확인했습니다.
- 재시작 프로세스: PID `26516`, 시작 시각 `2026-09-14 15:37:04 KST`, 동일 설치 경로, `Responding=True`.
- 재시작된 실제 미니 창을 스크린샷과 접근성 트리로 확인했습니다. Claude 두 한도에 `동기화됨`이 표시되고, 설명에는 `마지막 동기화: 2026. 09. 14. 15:37 (데이터 수집 기준)`이 제공됩니다. 시작 후 관찰 구간에서 같은 버전의 설치 안내는 다시 나타나지 않았습니다.
- 설정 전후 SHA-256 일치:
  - `ui-prefs.json`: `5A2026DBDEA57E57450D9D86C728AD7062900C55D96F3655634FEB38889065CF`
  - `claude-statusline-bridge.json`: `A4785440885ED91C93C4FE36C68C85489DBC1CB22437E974137EC9675B4DF318`
- 현재 네이티브 UI 도구의 창 목록에는 미니 창만 노출되고 마우스 이동/hover 기능이 제공되지 않아, 작업 표시줄 툴팁이 실제 포인터 아래 나타나는 장면은 확인하지 못했습니다. 컨트롤 테스트 통과와 실제 hover 육안 확인은 구분합니다.

## 최종 판정: PARTIAL (실제 업그레이드 PASS, 잔여 검증 2건)

| 요구사항 | 상태 | 근거/잔여 항목 |
|---|---|---|
| 1. 서명 키·공개 릴리즈·업데이트 다운로드 연결 | PASS | 실제 공개 파일·manifest·해시·서명 검증 |
| 1. 설치된 0.2.1에서 앱 내 업데이트·재시작 | PASS | 실제 설치 버튼, 0.2.2 자동 재시작·렌더링, 설정 전후 해시 일치 |
| 3. Claude 최신성·마지막 수집 시각·복구 안내 | PASS | 단위 테스트, 실제 Claude 조회, 네이티브 컨트롤 소비 테스트, 설치된 새 버전의 동기화 상태·수집 시각 표시 |
| 3. 실제 설치 화면의 마우스 hover 육안 확인 | 미검증 | 잠금은 해제됨. 현재 UI 도구에서 스트립 창·포인터 hover 동작에 접근 불가 |
| 후속 릴리즈의 전체 CI 자동 공개 | 미검증 | 초안 조회 수정은 테스트·실데이터 검사 통과, 전체 태그 CI는 미재실행 |

## 검증 경계

- 기존 공개 v0.2.1은 updater가 없으므로 수동 설치가 필요합니다.
- 현재 PC의 별도 재빌드 v0.2.1은 updater를 포함한 시험 대상이었으며, 앱 내 업데이트를 통해 공개 v0.2.2로 교체됐습니다.
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

코드 테스트·서명 검사·공개 다운로드 성공은 실제 앱 내 업그레이드 시험을 대신하지 않습니다. 이번에는 실제 업그레이드·재시작·설정 보존까지 확인했으며, 작업 표시줄 hover 육안 확인과 후속 릴리즈 전체 CI 자동 공개는 별도 잔여 항목입니다.
