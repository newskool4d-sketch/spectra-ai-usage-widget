# SPECTRA 쿼터 보드 & 타이포그래피 재설계

> 작성일: 2026-08-16
> 관련 이슈: 사용자 피드백 — "터미널 자동 실행 점검", "더 개쩌는 디자인·가독성 높은 폰트", "Claude·Codex 5시간/주간 한도를 한 번에"

## 배경

정밀 코드 검토(2026-08-16) 직후 사용자가 세 가지를 요청했다.

1. 앱 실행 시 터미널(콘솔 창)이 함께 뜨는 문제 점검
2. 더 인상적인 디자인, 가독성 높은 폰트·크기
3. Claude·Codex의 5시간 한도·주간 한도 4개를 한 화면에서 동시에 보기

1번은 원인이 이미 확정되어(릴리스 바이너리가 Windows GUI가 아닌 CUI 서브시스템으로 빌드됨) 이 설계와 분리해 즉시 수정·검증했다. 이 문서는 2·3번, 즉 시각 시스템 재설계를 다룬다.

## 현황 진단

- **타이포그래피**: `styles.css` 367줄 중 폰트 크기 지정이 7~11px에 집중(10px 27곳·9px 26곳·8px 10곳·7px 3곳). 430×720 미니 창에서 가독성이 떨어진다.
- **폰트**: `styles/tokens.css`의 `--font-ui`는 `"Pretendard Variable", Pretendard, "Noto Sans KR", ...`이지만 Pretendard가 시스템에 번들되어 있지 않다. 이 기기는 Noto Sans KR(VF)로 대체 렌더링되고, Noto Sans KR도 없는 Windows 기기는 맑은 고딕으로 더 떨어진다.
- **정보 구조**: 데스크톱 히어로(`.hero-signal`)와 모바일 히어로는 `activeQuota.windows[0]`(선택된 공급자의 주 한도 1개)만 보여준다. 나머지 3개(선택 공급자의 주간 한도, 미선택 공급자의 5시간·주간 한도)는 탭을 전환하거나 추이 화면으로 들어가야 확인 가능하다. 데이터 자체는 이미 `quotas.claude.windows`/`quotas.codex.windows`에 rolling(5시간)·weekly(주간) 둘 다 존재한다 — 프레젠테이션 계층만의 문제다.
- **라이트 테마 부채(연계)**: 직전 코드 검토에서 확인된 활성 탭 글자·게이지 트랙의 흰색 하드코딩(`var(--color-white)`, `rgba(255,255,255,…)`)이 라이트 모드에서 텍스트·바를 안 보이게 만든다. 이번 재설계가 어차피 같은 CSS 영역을 다시 쓰므로 함께 해소한다.

## 목표

- 미니 창(430px)에서도 편하게 읽히는 타이포 스케일 확립, 최소 크기 바닥을 12px로 올린다.
- 시스템 미설치 여부와 무관하게 동일한 서체(Pretendard)로 렌더링되도록 폰트를 로컬 번들한다.
- 공급자 선택 없이 Claude·Codex × 5시간·주간 = 4개 한도를 overview 화면 진입 즉시 확인할 수 있게 한다.
- 라이트/다크 두 테마 모두에서 텍스트·게이지가 정상적으로 보이게 한다.

## 비목표

- 백엔드·데이터 모델 변경 없음(Rust `provider_usage.rs`, `PlanQuota` 타입 불변).
- 서비스/추이/알림 탭의 공급자 선택(`activeProviderId`) 동작 방식 변경 없음 — overview 화면의 히어로 영역만 바뀐다.
- 새 npm/cargo 의존성 추가 없음(폰트는 정적 자산으로만 추가).

## 설계

### 1. 타이포그래피 스케일

`styles/tokens.css`에 크기 토큰을 추가한다(기존 `--tracking-*`/`--leading-*` 옆에 배치).

| 토큰 | 값 | 대체 대상 |
|---|---|---|
| `--text-2xs` | 12px | 기존 7~8px 최소 크기 전부 |
| `--text-xs` | 13px | 기존 9~10px 보조 라벨 |
| `--text-sm` | 14px | 기존 10~11px 본문·버튼 |
| `--text-md` | 16px | 공급자명·소제목 신규 계층 |
| `--text-lg` | 20px | 기존 `.card-heading h3`(19px) |
| `--text-2xl` | 40px | 쿼터 보드 카드 숫자(신규) |
| `--text-3xl` | 64px | 기존 `.hero-signal h2`(66px, 유지 목적 근접치) |

`styles.css`의 모든 `font-size` 리터럴을 위 토큰으로 치환한다(값을 올림 방향으로만 조정 — 기존보다 작아지는 곳은 없음). 폰트 두께 리터럴(400/650/700/800)은 그대로 둔다 — 정적 폰트 4종만 번들해도 브라우저가 표준 CSS 폰트 매칭 규칙에 따라 가장 가까운 번들 두께로 자동 대체하므로(650→600, 800→700) 소스 값을 바꿀 필요가 없다. 아바타 이니셜·작은 배지처럼 800을 쓰는 몇 곳은 700으로 약간 얇게 보이는 정도이며 육안 회귀는 없다.

### 2. 폰트 번들

- Pretendard 정적 서브셋(한글+영문, 가변폰트 아님) 4중량 — Regular 400 / Medium 500 / SemiBold 600 / Bold 700 — 각 ~265~270KB, 총 ~1.05MB를 `styles/fonts/`에 vendor한다.
- 출처: `pretendard` npm 패키지(SIL OFL-1.1, 재배포 허용), 라이선스 파일을 `styles/fonts/LICENSE`로 함께 보관.
- `styles/tokens.css` 상단에 `@font-face` 4개 선언(`font-display: swap`), `--font-ui` 스택은 그대로 두어 자산이 없는 극단적 상황에서도 기존 폴백(Noto Sans KR → 맑은 고딕 → Segoe UI)이 안전망으로 동작한다.
- 가변폰트(2MB) 대신 정적 서브셋(1.05MB)을 선택한 이유: 이 앱이 실제로 쓰는 두께가 4종뿐이고, 정적 폰트가 크롬 렌더링에서 보간 없이 더 또렷하다.

### 3. 쿼터 보드 (핵심 레이아웃 변경)

새 컴포넌트 `QuotaBoard`를 `src/App.tsx`에 추가한다.

- **입력**: `quotas`(기존 `QuotaRecord` 그대로) — 새 prop 타입 불필요.
- **구조**: 2행(Claude/Codex) × 2열(5시간/주간) 그리드. 각 셀: 공급자 색상 필(공급자별 `--provider` 색 재사용) + 창 라벨 + 잔여율 큰 숫자(`--text-2xl`) + 초기화까지 라벨 + 미니 게이지.
- **빈 값 처리**: 기존 `hasDisplayValue`/`QuotaWindowRow`와 동일한 규칙을 셀 단위로 적용 — 데모/미연결 상태는 "—"와 "연결 후 표시"를 그대로 쓴다. 새 상태·새 판단 로직을 만들지 않는다.
- **배치**:
  - 데스크톱(`VariantADesktop`): 기존 `.hero-signal` 자리(overview 최상단)에 `QuotaBoard`를 배치. 기존 "집중 확인"(`focus-card`, 데모 스파크라인)은 유지하되 보조 카드로 격하한다 — 삭제하지 않는다(회귀 방지, 데모 모드에서 유일한 추이 시각화이므로).
  - 모바일(`VariantCMobile`, 430px): 동일 `QuotaBoard`를 스트림 최상단에 배치. 칸당 실사용 폭 약 180px로 확인됨(패딩 제외 ~390px÷2).
- **영향 없는 부분**: `MetricTabs`/`RangeTabs`/`ChartBars`(추이 화면), `ProviderChip`/`ProviderRow`(서비스 화면), `activeProviderId` 기반 필터링 로직 — 전부 그대로.

### 4. 라이트 테마 토큰 정리

- `--color-track`을 `[data-theme="light"]` 블록에도 정의(현재 다크에만 존재, 라이트는 누락).
- 신규 토큰 `--color-on-control-active` 추가(다크: 기존 `--color-white` 값과 동일, 라이트: 어두운 잉크색) — `.nav-rail button.active`, `.segmented/.range-tabs button.active`, `.quota-window-copy strong`, `.runway-copy p strong`, `.platform-note strong` 등의 `var(--color-white)` 하드코딩을 이 토큰으로 치환.
- 게이지 트랙 `rgba(255,255,255,.0x)` 하드코딩(`.quota-window-meter`, `.provider-meter`, `.spectrum-line`, `.mobile-top i` 등)을 `var(--color-track)`으로 치환.

### 데이터 흐름

변경 없음. `App.tsx`의 `refreshProvider`/`quotaFromSnapshot`/`quotas` 상태 관리는 그대로이며, `QuotaBoard`는 이미 존재하는 `quotas.claude`/`quotas.codex`를 읽기만 한다.

### 에러 처리

새로운 실패 모드 없음 — 기존 `PlanQuota.confidence`/`connectionState` 판정을 그대로 재사용한다.

### 테스트·검증

- `npx tsc --noEmit`
- `npm run verify:tokens` / `npm run verify:baseline`(의도적 기준선 변경이므로 `docs/design-baseline/*.png`·`baseline.json` 해시를 함께 갱신) / `npm run verify:memory`
- 브라우저 수동 확인: 뷰포트 폭으로 데스크톱/모바일 전환(`src/App.tsx`의 `useIsMobile`이 `matchMedia(max-width: 820px)` 기준이며 URL 쿼리 파라미터는 사용하지 않음 — `docs/design-baseline/README.md`의 `?variant=` 안내는 구 `app.js` 프로토타입 전용이라 현재 React 앱에는 적용되지 않음) — 820px 초과 폭(데스크톱)·430×720(모바일 미니 창) × 다크·라이트 두 테마 = 4개 조합
- `cargo build --release` 후 실제 실행 파일 실행해 콘솔 창 미표시 확인(1번 항목, 이미 완료·검증 예정)

## 영향 파일

- `styles/tokens.css` (타이포 토큰, `@font-face`, 라이트 테마 토큰 추가)
- `styles/fonts/*.woff2`, `styles/fonts/LICENSE` (신규)
- `styles.css` (크기 토큰 적용, 하드코딩 흰색 치환, `.quota-board`/`.quota-cell` 신규 규칙)
- `src/App.tsx` (`QuotaBoard` 컴포넌트, `VariantADesktop`/`VariantCMobile` 배치)
- `docs/design-baseline/*` (스크린샷·`baseline.json` 재생성 — 의도적 기준선 변경)
- `src-tauri/src/main.rs` (완료 — 콘솔 서브시스템 수정, 이 설계와 별도 트랙)

## 결정 로그

- 가변폰트 대신 정적 서브셋 4종 선택 — 총 용량을 2MB→1.05MB로 줄이고 렌더링 보간을 없앤다.
- `QuotaBoard`를 기존 히어로 자리에 배치하고 "집중 확인" 카드는 보조로 격하(삭제 아님) — 데모 모드 추이 시각화를 잃지 않기 위함.
- 새 의존성(npm/cargo) 추가 없음 — 폰트는 빌드 자산으로만 vendor.
