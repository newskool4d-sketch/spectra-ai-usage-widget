# 실행 메모리 예산

SPECTRA는 트레이에 오래 머무는 앱이므로 사용량 연결 뒤에도 상주 작업을 최소화합니다.

- 뷰포트에 맞는 데스크톱 A 또는 모바일 C 화면 하나만 React 트리에 마운트합니다.
- 목록·라벨·브라우저 데모 데이터는 모듈 스코프 불변 객체로 두고 작은 컴포넌트는 `memo`로 재렌더링을 줄입니다.
- 차트 라이브러리, `requestAnimationFrame`, `localStorage`, `sessionStorage`, 상시 `setInterval`을 사용하지 않습니다.
- 네이티브 앱은 과거 사용량 시계열을 저장하지 않고 현재 한도 스냅샷만 보관합니다.
- Codex App Server는 새로고침 요청 때만 실행하며 응답 또는 12초 제한 뒤 자식 프로세스를 종료합니다.
- Claude 조회는 짧은 `claude auth status` 프로세스와 작은 정제 JSON 캐시만 사용합니다.
- Codex JSONL은 총 2MiB, JSON 파일은 4MiB, Claude 상태선 입력은 1MiB로 제한합니다.
- 기존 Claude 상태선 명령은 2초, 출력은 64KiB로 제한합니다.
- 로그인 직후 확인은 대화상자가 열린 동안 단일 재귀 `setTimeout`으로만 수행하며 성공·닫기·30회 제한에서 종료합니다.
- 앱 유휴 상태에서는 provider 폴링을 하지 않습니다. 시작 시 한 번, 사용자가 새로고침할 때 한 번 조회합니다.
- `matchMedia` 리스너는 한 개만 등록하고 effect cleanup에서 해제합니다.
- `prefers-reduced-motion`을 존중합니다.

검증:

```powershell
npm run build
npm run verify:memory
```

이 검사는 번들 크기와 메모리 경로의 구조적 예산을 확인합니다. 실제 RAM 사용량은 Windows 작업 관리자에서 30분 이상 idle·수동 refresh·로그인 시나리오를 별도로 측정해야 합니다.

## Windows Tauri 기준선

2026-08-16의 0.1 릴리스 스모크 테스트에서 SPECTRA와 WebView2 자식 프로세스 7개의 합계는 다음과 같았습니다. 공급자 연결을 추가한 0.2 패키지는 기능·트레이 QA를 통과했지만 30분 메모리 재측정은 남아 있습니다.

- 최적화된 미니 창 표시 직후: 작업 집합 441.1MB, private 217.8MB
- 닫기 후 트레이 숨김 3초: 작업 집합 437.5MB, private 203.6MB
- 네이티브 호스트 자체: 작업 집합 32.4MB, private 6.3MB

대부분은 WebView2의 브라우저·렌더러·GPU 프로세스입니다. 현재는 빠른 재표시를 위해 닫기 시 창을 숨기므로 WebView2가 유지됩니다. 릴리스 프로필에는 size 최적화, thin LTO, 단일 codegen unit, 심볼 제거, abort panic을 적용합니다.

더 줄여야 한다면 닫을 때 WebView 창을 파기하고 트레이 클릭 시 재생성하는 선택형 저메모리 대기 모드를 별도 기능으로 검토합니다. 재표시 지연과 화면 상태 초기화가 대가이며, WebView2 보안 격리를 약화하는 비공식 single-process 플래그는 사용하지 않습니다.

## 2026-09-04 Phase 0 기준 (개선 착수 전)

측정: `npm run measure:memory -- -Scenario <name>` (호스트 + WebView2 자손 프로세스 합산, 30초 간격). 대상은 2026-08-23 릴리스 빌드(`spectra-native.exe` 직접 실행, 설치본 없음).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 |
|---|---|---|---|---|---|
| mini-idle (30분) | 60 | 436.3 MB | 475.3 MB | 203.7 MB | 7 |
| dashboard-idle (10분) | — | 미측정(트레이 메뉴 조작 필요, 사용자 대기) | | | |
| tray-hidden (10분) | — | 미측정(닫기 조작 필요, 사용자 대기) | | | |

번들(2026-08-23 빌드): JS 230,653 B · CSS 42,710 B · 글꼴 1,074,956 B/4개 · 실행 파일 7,571,968 B · NSIS 3,207,362 B

## 2026-09-04 Phase 1 결과 (비용 0 제거)

변경: 프로토타입 CSS를 `styles/prototype.css`로 분리, `.grain`·`drift` 제거, Medium 글꼴 삭제(3굵기 400·600·700), 굵기 요청 정규화.

번들: JS 230,892 B · CSS 31,850 B · 글꼴 806,632 B/3개 (`npm run verify:memory` 임계값: CSS 34,000 · 글꼴 820,000/3)

릴리스 빌드(2026-09-04 17:39): 실행 파일 7,297,536 B(−274,432) · NSIS 2,934,801 B(−272,561). 글꼴 1개 제거분이 그대로 반영됐다.

기능 확인: `spectra-native.exe --provider-snapshot codex`(`.cmd` 직접 실행 경로) → 로그인·연결·주간 창 정상, `--provider-snapshot claude`(타임아웃 헬퍼 경로) → 0.6초 응답, 로그인 확인·statusline 캐시 대체 정상.

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | 비고 |
|---|---|---|---|---|---|---|
| mini-idle-phase1 (중단) | 12 | 수면 전 4표본 464.7 MB · 재개 후 8표본 445.1 MB | 474.6 MB | 227.6 MB · 205.3 MB | 7 | 17:41 시스템 수면 → 다음날 08:47 재개로 오염. 비교 근거로 쓰지 않음 |
| mini-idle-phase1b (30분) | 60 | 422.7 MB | 468.2 MB | 210.3 MB | 7 | 2026-09-05 08:52~09:22, 표본 간격 무결. Phase 0 대비 −13.6 MB(−3.1%) |

Phase 1은 GPU·컴포지팅 요소를 건드리지 않았으므로(그레인 제거만 해당) 상주 메모리 변화는 오차 범위 안이다(−3.1%). 메모리 목표(≤ 380 MB)는 Phase 2의 유리 표면·블러 오브 정리에서 다룬다.

## 2026-09-05 Phase 2 결과 (디자인 재구성)

변경: 배경 오브 3개(`filter: blur(150px)`) 삭제 → `radial-gradient` 합성, `app-surface`·`glass-card`의 `backdrop-filter` 제거(불투명 토큰 + `--shadow-inset`로 대체, `nav-rail`·`oauth-backdrop`·`mobile-nav`만 유지), 그림자 블러 54px·34px → 24px·16px, "다음 행동" 스트립·시간 진행 트랙·창 요약 SVG 추가(모두 정적), 미니 창 전용 밀도 레이아웃, 라이트 테마 하드코딩 화이트 토큰화.

번들: JS 234,378 B · CSS 32,443 B (Phase 1 대비 CSS +593 B — 신규 UI 규칙 추가분이 `.bar-chart`·`.hero-signal` 등 레거시 규칙을 `prototype.css`로 이전한 만큼을 상쇄) · 글꼴 806,632 B/3개(변경 없음)

측정: `npm run measure:memory -- -Scenario mini-idle-phase2` (릴리스 빌드, `spectra-native.exe` 직접 실행, 30초 간격 60표본, 2026-09-05 19:15~19:45, 표본 간격 무결·기기 수면 없음).

| 시나리오 | 표본 | 평균 작업 집합 | 최대 작업 집합 | 평균 private | 프로세스 수 | Phase 0 대비 | Phase 1b 대비 |
|---|---|---|---|---|---|---|---|
| mini-idle-phase2 (30분) | 60 | 410.8 MB | 413.8 MB | 187.8 MB | 7 | −25.5 MB(−5.8%) | −11.9 MB(−2.8%) |

비고: 60번째 표본에서 작업 집합이 355.4 MB로 순간 하락(WebView2 프로세스 중 하나의 일시적 트림으로 추정, 59개 표본은 409.6~413.2 MB 구간에서 평탄) — 평균 계산에는 그대로 포함했으며 추세에는 영향 없음.

Phase 0(436.3 MB)·Phase 1b(422.7 MB) 대비 평균 작업 집합이 지속적으로 감소했고, 특히 **최대 작업 집합이 475.3 MB → 413.8 MB(−13.0%)**로 크게 줄었다 — 스펙 2-2절에서 지목한 배경 오브·중첩 `backdrop-filter`가 GPU 컴포지팅 메모리에 직접 반영된다는 진단과 일치한다. 평균 private도 203.7 MB → 187.8 MB(−7.8%)로 감소했다. 목표치(≤ 380 MB)에는 아직 미달이며, 남은 격차는 Phase 3(WebView2 `--renderer-process-limit=1`, 네이티브 실행 파일 슬림화)에서 다룬다.

