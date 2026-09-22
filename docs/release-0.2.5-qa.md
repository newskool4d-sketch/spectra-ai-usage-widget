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

## 공개 배포 및 설치 검증

태그 자동 배포가 끝난 후 공개 자산, 서명·체크섬, 설치 버전·실행과 설정 보존 결과를 이 문서에 추가한다.

## 검증 경계

- GDI 미리보기는 실제 작업표시줄 화면 캡처가 아니다.
- Tauri updater 서명은 Windows Authenticode 게시자 인증과 별개다.
- 기존 미추적 문서·로컬 검증 출력은 커밋 대상에서 제외한다.
