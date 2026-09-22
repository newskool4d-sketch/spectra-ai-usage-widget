# SPECTRA v0.2.5 검증

대상: `main`, 태그 `v0.2.5`, Windows x64 NSIS. Claude 작업표시줄 잔여량을 `5h NN% · 7d NN%`로 고정 표시한다.

## 배포 전 검증 (2026-09-23)

| 검사 | 결과 |
|---|---|
| `npm test` | 50 passed |
| `npm run build` | PASS |
| `npm run verify:release` | `Release preflight PASS: v0.2.5` |
| Rust 기본 라이브러리 회귀 | 85 passed, 8 ignored (동일 구현, 버전 올림 전) |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --features native-oauth --offline --locked` | 87 passed, 8 ignored (v0.2.5) |
| 위 구성에서 `taskbar_window::tests -- --ignored --test-threads=1` | Windows 전용 8개 모두 PASS |
| GDI 렌더링 | 라이트·다크 × 96·120·144 DPI × 정상·0/100%·부분 누락·Claude 미연결 5종, 총 30개 조합 PASS |
| 버전 정합성 | package.json·package-lock.json·Cargo.toml·Cargo.lock·tauri.conf.json 모두 0.2.5 |
| 신규 의존성 | 없음 |
| `git diff --check` | PASS |

회귀 테스트는 잔여량 역전, 수집 순서 역전, 캐시 상태, 한쪽 창 누락, 로그아웃, 기간 라벨 유지, 독립 경고 색상을 확인한다. 실제 GDI 픽셀 검증은 기간·값·색상·CI 개수·투명 여백·측정 폭 내부 배치를 포함한다.

## 로컬 설치 파일 검증

- `node node_modules/@tauri-apps/cli/tauri.js build --bundles nsis -- --offline --locked`: PASS. 릴리스 컴파일 5분 55초, NSIS 설치 파일과 updater 서명 생성.
- `SPECTRA_0.2.5_x64-setup.exe`: 2,843,238 bytes, SHA-256 `F54CD42AD4C98B7A25A91C4A8B38DE3337FC0F63A0E48E29E65ABFDEFD07C141`.
- 앱에 내장한 공개키로 설치 파일의 Ed25519/BLAKE2b-512 서명, trusted comment 서명, 키 ID 일치와 변조 파일 거부를 확인했다.
- NSIS ProductVersion 및 앱 실행 파일 ProductVersion: `0.2.5`.
- Windows Authenticode: `NotSigned` (기존 배포 방식과 동일).

## 공개 배포 검증 (2026-09-23 KST)

- 릴리스 커밋: `b92a23ef3858766396999b9ca859a3d5dde4a66d`, 주석 태그 `v0.2.5`.
- [GitHub Actions 35795795854](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/35795795854): 8분 52초, 테스트·설치 파일 생성·서명·초안 자산 검증·체크섬 업로드·공개 전환 전 단계 PASS.
- [공개 v0.2.5](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.5): `draft=false`, `prerelease=false`, `releases/latest`가 v0.2.5를 가리킨다. 공개 시각은 08:15:32 KST.
- 공개 자산: 설치 파일 2,836,931 bytes(asset 582449896), updater 서명 416 bytes, `latest.json` 1,382 bytes, `SHA256SUMS.txt` 270 bytes.
- 공개 설치 파일 SHA-256: `A85C280B7325EDA8CFE35F7B96BBCB13423005CD7D8A841B4D0B1B138DF979C6`.
- 공개 자산 다운로드 후 `scripts/verify-release.mjs`로 manifest 버전·설치 파일 URL/asset ID·내장 서명·GitHub digest를 확인했고, 재생성한 체크섬 파일도 공개 체크섬과 바이트 단위로 일치했다.
- 앱 공개키로 공개 설치 파일의 서명·trusted comment 서명·키 ID 일치와 변조 거부를 검증했다. Windows Authenticode는 `NotSigned`다.
- 비인증 `releases/latest/download/latest.json` 응답 HTTP 200, 버전 `0.2.5`, Windows 대상 URL은 위 설치 파일의 asset ID와 일치했다.

## 실제 앱 내 업그레이드 (08:18 KST)

- 기존 설치본 0.2.4를 재시작해 실제 창의 `SPECTRA 새 버전 0.2.5` 안내를 확인했다. `지금 설치`를 실행한 뒤 updater가 공개 설치 파일을 받아 설치하고 자동 재시작했다.
- 설치 경로 `%LOCALAPPDATA%\SPECTRA\spectra-native.exe`: ProductVersion `0.2.5`, 6,308,352 bytes, 수정 시각 08:15:16 KST(CI 빌드 시각).
- 설치 실행 파일 SHA-256: `59B28E672884EBE9233CF4700EE11899FEB6CBF907A02EC9B1F43D15755805F6`.
- 설치 경로의 실행 프로세스 1개, 재시작 시각 08:18:10 KST. 기존 `strip=true` 유지.
- `ui-prefs.json`과 `claude-statusline-bridge.json`의 설치 전후 SHA-256이 각각 일치했다.
- Windows Computer Use로 재시작한 실제 미니 창을 캡처하고, Codex·Claude 사용량 카드와 동기화 상태가 정상 렌더링되며 새 버전 설치 안내가 사라진 것을 확인했다.
- 작업표시줄 스트립은 Computer Use의 선택 가능한 창 목록에 노출되지 않았다. 실제 두 값 표시와 주변 아이콘 겹침 여부는 사용자 확인 요청 상태이며, 확인 전에는 실물 PASS로 판정하지 않는다.

## 판정

| 요구사항 | 상태 | 근거 |
|---|---|---|
| Claude 5시간·주간 고정 표시 구현 | PASS | 순서·누락·잔여량 역전 테스트 및 실제 GDI 렌더링 30개 조합 |
| 서명된 공개 릴리스와 updater manifest | PASS | Actions 전 단계, 공개 자산·서명·digest·체크섬 검증 |
| 실제 설치본 0.2.4 → 0.2.5 업데이트 | PASS | 실제 앱 내 설치, 새 버전 파일, 자동 재시작 및 미니 창 렌더링 |
| 사용자 설정 보존 | PASS | 두 설정 파일의 설치 전후 해시 일치 |
| 작업표시줄 실물 표시·주변 아이콘 간격 | NOT_RUN | 사용자 육안 확인 요청 중 |

전체 판정: **PARTIAL** — 빌드·공개 배포·실제 앱 업데이트는 완료했다. 작업표시줄 실물 표시·간격의 육안 확인만 남았다.

## 검증 경계

- GDI 미리보기는 실제 작업표시줄 화면 캡처가 아니다.
- Tauri updater 서명은 Windows Authenticode 게시자 인증과 별개다.
- 기존 미추적 문서·로컬 검증 출력은 커밋 대상에서 제외한다.
- 기존 설치본의 실제 앱 내 업그레이드를 검증했으며, 별도 신규 사용자 프로필의 클린 설치는 수행하지 않았다.
