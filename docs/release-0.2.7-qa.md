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

## 검증 경계

- 실제 설치된 WebView의 네트워크 오류 화면과 설치 프로그램 실패 후 복구는 미검증이다. 브라우저 모의 응답 결과로 이를 대신하지 않는다.
- 물리적인 다중 모니터/DPI 전환, 실제 Explorer 알림 영역 변화, 장시간 standby 후 복원 화면은 미검증이다.
- Unix 실구동은 이번 작업에서 제외했다.
- 공개 release와 로컬 설치본 반영을 별도로 판정한다. 로컬 검증 EXE의 해시를 CI 설치 파일의 해시로 사용하지 않는다.
