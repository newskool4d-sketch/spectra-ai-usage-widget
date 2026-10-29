# SPECTRA v0.2.9 검증

대상: `main`, 준비 커밋 `c266e9b`, 주석 태그 `v0.2.9`, Windows x64 NSIS. 앱 코드 변경이 없는 문서·배포물 정리 릴리스이며, 기능 검증은 `docs/release-0.2.8-qa.md`가 그대로 유효합니다.

## 릴리스 준비 (2026-10-29 KST)

| 검사 | 결과 |
|---|---|
| 버전 정합성 | package.json · package-lock.json · Cargo.toml · Cargo.lock · tauri.conf.json 모두 0.2.9 |
| `npm run verify:release` | Release preflight PASS: v0.2.9 |
| `npm test` | 78 passed, 0 failed |
| `git diff --cached --check` | PASS |
| `cargo check --locked --lib` | PASS, 잠금 파일 변경 없음 |
| 같은 날 정기 점검(`docs/inspection-2026-10-29.md`) | `cargo test --lib --locked` 104 passed · 9 ignored, `verify:memory` FAIL(a87d1a5부터 동일) |

- 변경 파일: 버전 5파일과 `docs/releases/v0.2.9.md`. 소스 코드 변경 없음.
- `main`과 주석 태그 `v0.2.9`를 `git push --atomic`으로 함께 푸시(16:18 KST). 태그 커밋과 릴리스 `targetCommitish`가 `c266e9b`로 일치.

## 공개 배포 검증 (2026-10-29 KST)

- [GitHub Actions 40834612709](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/40834612709): `completed / success`, 16:18:46~16:29:01(10분 15초). 버전·서명 키 존재·프런트엔드·네이티브 테스트·NSIS 빌드·자산 검증·공개 단계 모두 success. 안내 주석 1건: `actions/checkout@v4`·`actions/setup-node@v4`의 Node.js 20 지원 종료(Node 24로 강제 실행), 실패 아님 — 점검 보고 F7 후속.
- [공개 release v0.2.9](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.9): release ID `427569403`, `draft=false`, `prerelease=false`, 공개 16:28:54, `releases/latest`가 같은 ID를 가리킴. 본문은 `docs/releases/v0.2.9.md`.
- 공개 자산 4개를 인증 없이 공개 URL로 격리 폴더에 내려받아 바이트 수와 SHA-256을 GitHub 자산 메타데이터의 `digest`와 대조. `SHA256SUMS.txt` 3행이 설치 파일·서명·manifest 해시와 일치.
- 업데이터 엔드포인트 `releases/latest/download/latest.json`을 인증 없이 조회한 내용이 자산 `latest.json`과 같은 해시. manifest: version `0.2.9`, pub_date `2026-10-29T07:28:23.994Z`, Windows 두 플랫폼 항목이 설치 파일 자산 API URL(ID 687729806)과 `.sig` 내용을 담음.
- 앱 공개키(키 ID `2b2190fae1b3f640`)로 `.sig`를 검증: Ed25519 + BLAKE2b-512 프리해시 서명 PASS, trusted comment(`timestamp:1793258900 file:SPECTRA_0.2.9_x64-setup.exe`) 전역 서명 PASS, 첫 바이트를 변조한 사본은 거부.
- 내려받은 자산에 `node scripts/verify-release.mjs <dir>`를 적용해 PASS. Windows Authenticode는 `NotSigned`로, 업데이터 서명 검증과 별개.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.2.9_x64-setup.exe` | 2,838,964 | `47674bd72c872d52a7590c8812cb6e5366425672dff485746d3783ea65b4c184` |
| `SPECTRA_0.2.9_x64-setup.exe.sig` | 416 | `1999977008331b5b9535dbd1543a2e227998e0e3d0f2a933b1550c2721bfeabb` |
| `latest.json` | 1,382 | `27273a743c8f08e25f1bb21dfb0c4a44df9b83b41876ff93792f5d7718081566` |
| `SHA256SUMS.txt` | 270 | `46373b67a880d5d82802edacfa345d30d1a59a6b0e73c78e897902ae87b59cd8` |

## 검증 경계

- 설치 파일 실행과 앱 내 업그레이드는 하지 않음. 이 PC의 설치본은 v0.2.8 로컬 빌드로, 다음 미니 창·대시보드 생성 때 v0.2.9 안내가 뜰 것으로 예상하며 실제 업그레이드는 사용자 선택.
- 공개 설치 파일 안의 `spectra-native.exe` 해시는 추출하지 않음(7-Zip 미설치). 작업표시줄 유지 모듈 설치 스크립트의 실행 파일 해시 고정(점검 보고 F2)이 v0.2.9 설치본에 미치는 영향은 미확인이며, 릴리스 노트에 "유지 모듈 추가 설치는 v0.2.8 zip 전용"으로 명시.
- 앱 기능 변경이 없어 기능·메모리·연결 QA는 반복하지 않음.
