# SPECTRA v0.2.8 검증

대상: `main`, 수정 커밋 `609f2e5`, Windows x64 NSIS.

## 원인 점검 (2026-10-03 KST)

- 증상: 로그인 상태에서 작업표시줄·창의 Claude 값이 `—`로 바뀌었다가 몇 분 뒤 돌아옴. 사용자 제보 "현재 실행 파일 클로드 사용량 안나타나는 중"(10:3x).
- 원인: 2분마다 `claude auth status` 실행(제한 5초) → 시간 초과 시 디스크 캐시를 읽지 않고 `error`·창 0개 스냅샷 반환 → `remember_snapshot`이 직전 정상값 교체 → 스트립·트레이 배지·창이 동시에 빈 값, 백오프 120→240초 동안 유지.
- 실측(설치본 v0.2.7, 10:15~10:33, CPU 100%): `claude auth status` 5회 5,545 · 4,519 · 4,160 · 5,536 · 3,549 ms. Claude 캐시 기록 누락 4회(10:21 · 10:25 · 10:30 · 10:32). 10:28:18 성공 뒤 다음 시도 10:30:18, 스트립 Claude 값이 `—`로 바뀐 시각 10:30:23(시도 +5초).
- 측정 오염: 10:19 수동 조회, 10:25 직전 `auth status` 5회 측정, PowerShell 감시 프로세스가 부하를 더함. 평상시 부하의 실패 빈도는 측정하지 않음.
- 저부하 대조: 11:51 CPU 30~84%에서 설치본 격리 실행 Claude 5회·Codex 3회 모두 성공(1.6~3.3초). 11:54~12:19 감시 297회(5초 간격): 스트립 상시 표시, 두 공급자 값 유지, 작업표시줄 위 z-order, Claude 13회 연속 2분 간격 성공.
- 10:22 무렵 스트립 창 전체 숨김 1회 관찰. 원인 미확정(두 공급자 동시 빈 값 또는 전체화면 숨김 조건). 설치본은 로그를 남기지 않아 판정 불가.

## 수정 검증: 같은 시나리오, 변경 전후

가짜 `claude` 명령과 격리 데이터 폴더(`SPECTRA_DATA_DIR`)로 실행. 설치본 캐시 미사용, 실제 사용량 API 호출은 사례 A의 수정 빌드 1회.

| 사례 | 조건 | v0.2.7 설치본 | 수정 빌드 |
|---|---|---|---|
| A | CLI 응답 6초, 실제 유효 자격 증명 | 5,248 ms `error`, 창 0 | 1,770 ms `connected`, 창 2, 요금제 max, CLI 미실행 |
| A2 | CLI 응답 17초, 자격 증명 없음 | 5,221 ms `error` | 15,193 ms `error`, 새 문구 |
| A3 | CLI 응답 6초, 자격 증명 없음 | 5,207 ms `error` | 6,435 ms `stale`, 캐시 창 2 |
| C | CLI 로그아웃 응답 | 452 ms `signed-out` | 329 ms `signed-out` |
| D | CLI 응답 6초, 만료 토큰(가짜 파일) | 5,196 ms `error` | 210 ms `stale`, 캐시 창 2, 로그인 갱신 안내, 요금제 pro(파일 값) |
| B | CLI 즉시 응답, 자격 증명 없음 | 318 ms `stale` | 400 ms `stale` |

- CLI 모드(`--provider-snapshot`)는 `refresh_provider`를 거치지 않음 → 직전 값 유지(`keep_last_good`)는 이 표로 검증되지 않음. 단위 테스트와 아래 실앱 검증으로 확인.

## v0.2.8 후보 검증 (2026-10-03 KST)

| 검사 | 결과 |
|---|---|
| `npm test` | 78 passed, 0 failed |
| `npm run build` | TypeScript 검사·Vite 빌드 PASS |
| `npm run verify:release` | Release preflight PASS: v0.2.8 |
| `cargo test --lib --locked`(CI 동일) | 104 passed, 0 failed, 9 ignored |
| `cargo test --lib --features native-oauth` | 106 passed, 0 failed |
| 전체 Rust `--include-ignored --test-threads=1` | 114 passed(라이브러리 113 + HTTP 통합 1), 0 failed, 0 ignored — 버전 올림 전 같은 코드 |
| `tauri build --no-bundle` | 경고 0, 실행 파일 6,317,568 B — 버전 올림 전 같은 코드 |
| `git diff --check` | PASS |
| `npm run verify:memory` | FAIL, 변경 전부터 동일: JS 247,058 B → 247,293 B(+235 B), 기존 15초 타이머의 `setInterval` 패턴 |
| 버전 정합성 | package.json · package-lock.json · Cargo.toml · Cargo.lock · tauri.conf.json 모두 0.2.8 |

## 로컬 설치 검증

설치 후 기록.

## 검증 경계

- 10:22 스트립 전체 숨김 원인 미확정. 평상시 부하의 실패 빈도 미측정.
- 초기화 경계 `—` 표시는 작업표시줄·트레이 배지·미니 보드만 적용. 대시보드 서비스 목록·칩·집중 확인 카드는 이어받은 수치를 그대로 표시. 트레이 배지는 다음 조회 때 반영.
- 로컬 0.2.8 설치 후 공개 0.2.8이 나와도 업데이터는 같은 버전으로 판단 → 이전 릴리스의 앱 내 업그레이드 검증은 이번에 반복 불가. 공개 설치 파일 재설치로 대체 가능.
- 공개 배포(push + `v0.2.8` 태그) 미실시.
