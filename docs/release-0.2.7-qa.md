# SPECTRA v0.2.7 검증

대상: `main`, `v0.2.7`, Windows x64 NSIS 및 Tauri updater manifest.

## 기능 검증 근거

- 프런트엔드 75개 테스트 통과: 공급자 추천·캐시 복원·실제 updater JS SDK·오류 배너 SSR 및 콜백.
- Windows 전체 Rust 104개 테스트 통과: 라이브러리 103개(네이티브 9개 포함)와 실제 HTTP 타임아웃 통합 1개. GDI 렌더링 60조합을 포함한다.
- Windows 최적화 EXE를 기존 설치본에 반영하고 주 화면과 Codex·Claude 실조회를 확인했다. UI·연결 설정 파일 해시를 보존했다.
- 실제 App + 합성 IPC 브라우저에서 확인 실패→재시도→최신 버전 복구, 다운로드 실패→재확인→다운로드 재시도를 확인했다. 빠른 두 번 클릭에도 확인 총 2회·다운로드 총 2회, 최대 동시 다운로드 1개·재시작 0회였다.

위 근거는 버전 변경 전 동일 기능 소스의 검증이다. v0.2.7 후보의 버전 정합성·테스트·빌드와 공개 자산 검증은 아래에 별도로 기록한다.

## v0.2.7 후보 검증 (2026-09-30 KST)

| 검사 | 결과 |
|---|---|
| `npm test` | 75 passed, 0 failed |
| `npm run build` | TypeScript 검사·Vite 빌드 PASS |
| `npm run verify:release` | Release preflight PASS: v0.2.7 |
| 전체 Rust, `--include-ignored --test-threads=1` | 104 passed, 0 failed, 0 ignored; 라이브러리 103 + HTTP 통합 1 |
| 실제 HTTP 타임아웃·재시도 | 확인 218.28ms / 다운로드 211.22ms·203.07ms, 제한 200ms, 설치 호출 0 |
| 버전 정합성 | package.json·package-lock.json·Cargo.toml·Cargo.lock·tauri.conf.json 모두 0.2.7 |
| 공백·패치 검사 | `git diff --cached --check` PASS |

서명키 `TAURI_SIGNING_PRIVATE_KEY` 등록 상태를 확인했다. 키 값은 조회하거나 기록하지 않았다. 공개 배포는 태그 푸시로 기존 GitHub Actions를 실행하며, 서명된 NSIS·manifest·체크섬 자산을 별도로 검증한다.

## 공개 배포 검증 (2026-09-30 KST)

- 배포 소스: `0927fb7c5970530f1d1d6d0055751724daa993cd`. `main`과 `v0.2.7` 태그를 원격으로 함께 푸시했고, 태그의 커밋과 Actions 실행 커밋이 일치했다.
- [GitHub Actions 36688891866](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/36688891866): `completed / success`. Windows NSIS 빌드·서명·자산 검증·공개 단계 통과.
- [공개 release v0.2.7](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.7): release ID `399847603`, `draft=false`, `prerelease=false`, 최신 release ID와 일치. 공개 시각은 2026-09-30 17:28:58 KST.
- 공개 자산 4개를 내려받아 바이트 수와 SHA-256을 GitHub 자산 메타데이터의 `digest`와 각각 대조했다. `SHA256SUMS.txt`의 설치 파일·서명·manifest 해시도 일치했다.
- 인증 헤더 없이 `releases/latest/download/latest.json`을 조회해 버전 `0.2.7` 및 다운로드한 manifest와의 내용 일치를 확인했다. 두 Windows 플랫폼 항목의 설치 파일 연결과 서명 문자열을 확인했다.
- 앱의 공개키로 설치 파일 및 trusted comment 서명을 검증했고, 설치 파일의 첫 바이트를 변조한 사본은 서명 검증에서 거부됐다. 세 검사 모두 PASS.
- 공개 자산에 `node scripts/verify-release.mjs`를 다시 적용해 PASS를 확인했다. Windows Authenticode는 `NotSigned`로, Tauri 업데이터 서명 검증과 별개다.

| 공개 자산 | 바이트 수 | SHA-256 |
|---|---:|---|
| `SPECTRA_0.2.7_x64-setup.exe` | 2,837,812 | `3380fbf74c38dd3cff077e4c35e7d62885c908abacdbee0ebdefaa51036c17a3` |
| `SPECTRA_0.2.7_x64-setup.exe.sig` | 416 | `5c9980579be9c836939df629b368c4f9fe29e7a530e13338b83838bd2495fdb3` |
| `latest.json` | 1,382 | `5e46d2cfb7f4a2051a8a5c13d8089d5a3316a96ddea40253b9dd80a92d5eaa1f` |
| `SHA256SUMS.txt` | 270 | `b5083532187c12aa36db8ee07cd7e9604a89c3f07981fd163f42d00ec6b872fb` |

공개 배포와 자산 검증은 PASS다. 이 단계에서는 로컬 앱에 v0.2.7 공개 설치 파일을 설치하지 않았으며, 아래 실환경 검증 경계는 유지한다.

## 검증 경계

- 실제 설치된 WebView의 네트워크 오류 화면과 설치 프로그램 실패 후 복구는 미검증이다. 브라우저 모의 응답 결과로 이를 대신하지 않는다.
- 물리적인 다중 모니터/DPI 전환, 실제 Explorer 알림 영역 변화, 장시간 standby 후 복원 화면은 미검증이다.
- Unix 실구동은 이번 작업에서 제외했다.
- 공개 release와 로컬 설치본 반영을 별도로 판정한다. 로컬 검증 EXE의 해시를 CI 설치 파일의 해시로 사용하지 않는다.
