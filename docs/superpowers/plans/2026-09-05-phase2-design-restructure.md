# Phase 2 — 디자인 재구성 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 네이티브 앱의 첫 화면을 "답부터 보여주는" 구조로 재편하고, 예시 데이터·과도한 유리 효과·하드코딩 화이트를 제거한다.

**Architecture:** 장식 헤드라인 → 스냅샷만으로 계산한 "다음 행동" 스트립(순수 함수 `computeNextAction`), 쿼터 셀에 시간 진행 두 번째 트랙(`resetsAt − windowDurationMins`로 프론트에서 파생), 예시 막대 차트 → 창 요약 정적 SVG, 배경 오브·중첩 `backdrop-filter` 정리, Tauri 창 모드(`mini`/`dashboard`)를 `window.__SPECTRA_MODE__`로 전달해 미니 전용 레이아웃, 라이트 테마 토큰 마무리. 백엔드(`provider_usage.rs`) 수정 없음.

**Tech Stack:** React 19 · TypeScript · Vite 8 · Tauri 2 (Rust 1.77.2+) · Node 24 `node:test`

**Spec:** `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` — "Phase 2 — 디자인 재구성" 절(2-A ~ 2-F, 성공 기준)

## Global Constraints

- 새 의존성 추가 금지 — 표준 라이브러리·기존 모듈만 사용
- 메모리 예산 금지 항목(소스·번들 모두 검사됨): `setInterval`, `requestAnimationFrame`, `localStorage`, `sessionStorage`, 차트 라이브러리
- 번들 예산: JS ≤ 500,000 bytes · CSS ≤ 34,000 bytes · 글꼴 ≤ 820,000 bytes / 3파일
- 시간 진행률은 렌더 시점의 `Date.now()`로 1회 계산 — 타이머·폴링으로 갱신하지 않음(현재 스냅샷 원칙)
- 상태는 색만으로 구분하지 않음 — 반드시 텍스트 라벨 병기
- 프론트 테스트: `npm test` = `node --test "tests/**/*.test.ts"` (Node 24 type-stripping, import 경로에 `.ts` 확장자 명시)
- 각 커밋 전 `npm run build` · `npm run verify:tokens` · `npm run verify:memory` · `npm run verify:baseline` · `npm test` 모두 PASS
- 프로토타입 규칙(`orbit-*`, `prototype-switcher`, `widget-lab`)은 `styles/prototype.css`에만 존재. `prototype.css`는 `--glass-app-bg/blur`·`--glass-widget-bg/blur`·`--shadow-widget`을 소비하므로 이 토큰들은 삭제 금지
- 파일 삭제 금지(빌드 산출물 제외) — `src/components/Sparkline.tsx`는 데모 전용으로 유지
- 커밋 메시지 말미: `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`
- dev server·preview 프로세스는 실행자가 직접 띄우지 않음 — 명령어를 사용자에게 제시하고 실행 결과를 받아 진행

## File Structure

| 파일 | 역할 | 작업 |
|---|---|---|
| `src/data/next-action.ts` | 다음 행동 판정 · 시간 진행률 · 페이스 라벨(순수 함수) | 신규 |
| `tests/next-action.test.ts` | `computeNextAction` 테스트 | 신규 |
| `tests/time-progress.test.ts` | `computeTimeProgress` 테스트 | 신규 |
| `src/data/providers.ts` | `QuotaWindow`에 `resetsAt`·`windowDurationMins` 옵셔널 필드, `usageBars` 삭제 | 수정 |
| `src/App.tsx` | `NextActionStrip`·`WindowSummary`·`MiniLayout`·`useWindowMode` 추가, `ChartBars` 삭제, `QuotaCell` 시간 트랙, 헤더 분기 | 수정 |
| `src-tauri/src/desktop_shell.rs` | `WindowMode::slug`, `mode_script`, `show_main_window`에서 `eval` 주입 | 수정 |
| `index.html` | `.orb` 요소 3개 제거 | 수정 |
| `styles.css` | 표면·배경 정리, 새 컴포넌트 규칙, 하드코딩 화이트 토큰화, `.bar-chart` 삭제 | 수정 |
| `styles/tokens.css` | 그림자 축소, `--glass-card-*` 삭제, `--color-surface-subtle` 추가 | 수정 |
| `scripts/verify-design-tokens.mjs` | 필수 토큰 교체, 하드코딩 화이트 검사 | 수정 |
| `scripts/verify-memory-budget.mjs` | 예시 차트 잔존 검사, `next-action.ts` 스캔 대상 추가 | 수정 |
| `docs/design-baseline/baseline.json` · `README.md` · `desktop-a.png` · `mobile-c.png` | 기준선 재촬영·해시 갱신 | 수정 |
| `docs/performance/memory-budget.md` · 스펙 상태 줄 | Phase 2 측정·상태 기록 | 수정 |

---

### Task 1: "다음 행동" 판정 함수 (2-A 로직)

**Files:**
- Create: `src/data/next-action.ts`
- Create: `tests/next-action.test.ts`
- Modify: `src/data/providers.ts` — `QuotaWindow` 타입(18행 부근)

**Interfaces:**
- Consumes: `PlanQuota`, `ProviderId`, `QuotaWindow` (from `src/data/providers.ts`)
- Produces (Task 2·4·6이 사용):
  ```typescript
  export const providerNames: Readonly<Record<ProviderId, string>>;      // { codex: "Codex", claude: "Claude" }
  export function hasVerifiedUsage(quota: PlanQuota): boolean;           // confidence === "verified" && windows.length > 0
  export function primaryWindow(quota: PlanQuota): QuotaWindow | undefined; // id === "rolling" 우선, 없으면 windows[0]
  export type NextAction = Readonly<{ recommendedProvider: ProviderId | null; headline: string; chips: readonly string[] }>;
  export function computeNextAction(quotas: Readonly<Record<ProviderId, PlanQuota>>): NextAction;
  ```

- [ ] **Step 1: `QuotaWindow` 타입에 시간 필드 추가**

`src/data/providers.ts`의 `QuotaWindow`를 다음으로 교체한다(두 필드 추가, 기존 `planQuotas` 데모 데이터는 수정하지 않음):

```typescript
export type QuotaWindow = Readonly<{
  id: QuotaWindowId;
  label: string;
  usedPercent: number;
  remainingPercent: number;
  resetLabel: string;
  kindLabel: string;
  resetsAt?: number | null;
  windowDurationMins?: number | null;
}>;
```

- [ ] **Step 2: 테스트 작성**

`tests/next-action.test.ts`:

```typescript
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { computeNextAction } from "../src/data/next-action.ts";
import type { PlanQuota, ProviderId, QuotaWindow } from "../src/data/providers.ts";

const base = 1_700_000_000_000;
const hour = 60 * 60 * 1000;

function window(overrides: Partial<QuotaWindow> = {}): QuotaWindow {
  return { id: "rolling", label: "5시간 한도", usedPercent: 60, remainingPercent: 40, resetLabel: "2시간 후", kindLabel: "", resetsAt: base + 2 * hour, windowDurationMins: 300, ...overrides };
}

function connected(providerId: ProviderId, windows: readonly QuotaWindow[] = [window()]): PlanQuota {
  return { providerId, planName: "Test", accountLabel: "", authMethod: "provider-delegated", connectionState: "connected", source: "claude-statusline", confidence: "verified", lastSyncedAt: null, bridgeInstalled: true, statusMessage: "", windows };
}

function pending(providerId: ProviderId): PlanQuota {
  return { ...connected(providerId), connectionState: "not_connected", source: "unavailable", confidence: "unavailable", windows: [window({ usedPercent: 0, remainingPercent: 0, resetLabel: "연결 후 표시", resetsAt: null, windowDurationMins: null })] };
}

describe("computeNextAction", () => {
  it("recommends the provider whose primary window keeps more remaining percent", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ usedPercent: 76, remainingPercent: 24, resetLabel: "1시간 18분 후" })]),
      claude: connected("claude", [window({ usedPercent: 68, remainingPercent: 32, resetLabel: "3시간 후" })])
    });
    assert.equal(result.recommendedProvider, "claude");
    assert.equal(result.headline, "지금은 Claude에서 작업");
    assert.deepEqual(result.chips, ["Codex 5시간 한도 24% 남음 · 1시간 18분 후 초기화", "Claude 5시간 한도 32% 남음 · 3시간 후 초기화"]);
  });

  it("breaks a tie by the later reset time", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 50, resetsAt: base + 3 * hour })]),
      claude: connected("claude", [window({ remainingPercent: 50, resetsAt: base + 1 * hour })])
    });
    assert.equal(result.recommendedProvider, "codex");
  });

  it("keeps provider order when tied and reset time is unknown", () => {
    const result = computeNextAction({
      codex: connected("codex", [window({ remainingPercent: 50, resetsAt: null })]),
      claude: connected("claude", [window({ remainingPercent: 50, resetsAt: null })])
    });
    assert.equal(result.recommendedProvider, "codex");
  });

  it("uses the rolling window even when it is not first", () => {
    const weekly = window({ id: "weekly", label: "주간 한도", remainingPercent: 90, windowDurationMins: 10080 });
    const result = computeNextAction({
      codex: connected("codex", [weekly, window({ remainingPercent: 10 })]),
      claude: connected("claude", [window({ remainingPercent: 30 })])
    });
    assert.equal(result.recommendedProvider, "claude");
  });

  it("asks to connect when no provider has verified usage", () => {
    const result = computeNextAction({ codex: pending("codex"), claude: pending("claude") });
    assert.equal(result.recommendedProvider, null);
    assert.equal(result.headline, "Codex·Claude 연결 후 권장 서비스를 알려 드립니다");
    assert.deepEqual(result.chips, ["Codex 연결 필요", "Claude 연결 필요"]);
  });

  it("recommends the only connected provider and marks the other as needing connection", () => {
    const result = computeNextAction({ codex: pending("codex"), claude: connected("claude") });
    assert.equal(result.recommendedProvider, "claude");
    assert.equal(result.chips[0], "Codex 연결 필요");
  });
});
```

- [ ] **Step 3: 테스트 실행 — 실패 확인**

```bash
npm test
```

Expected: FAIL — `Cannot find module '.../src/data/next-action.ts'`.

- [ ] **Step 4: `next-action.ts` 구현**

`src/data/next-action.ts`:

```typescript
import type { PlanQuota, ProviderId, QuotaWindow } from "./providers";

export const providerNames: Readonly<Record<ProviderId, string>> = Object.freeze({ codex: "Codex", claude: "Claude" });
const providerIds: readonly ProviderId[] = ["codex", "claude"];

export type NextAction = Readonly<{
  recommendedProvider: ProviderId | null;
  headline: string;
  chips: readonly string[];
}>;

export function hasVerifiedUsage(quota: PlanQuota): boolean {
  return quota.confidence === "verified" && quota.windows.length > 0;
}

export function primaryWindow(quota: PlanQuota): QuotaWindow | undefined {
  return quota.windows.find(window => window.id === "rolling") ?? quota.windows[0];
}

function chipText(id: ProviderId, quota: PlanQuota): string {
  const window = hasVerifiedUsage(quota) ? primaryWindow(quota) : undefined;
  if (!window) return `${providerNames[id]} 연결 필요`;
  return `${providerNames[id]} ${window.label} ${Math.round(window.remainingPercent)}% 남음 · ${window.resetLabel} 초기화`;
}

function beats(candidate: QuotaWindow, current: QuotaWindow): boolean {
  if (candidate.remainingPercent !== current.remainingPercent) return candidate.remainingPercent > current.remainingPercent;
  if (candidate.resetsAt == null || current.resetsAt == null) return false;
  return candidate.resetsAt > current.resetsAt;
}

export function computeNextAction(quotas: Readonly<Record<ProviderId, PlanQuota>>): NextAction {
  const chips = providerIds.map(id => chipText(id, quotas[id]));
  const connected = providerIds.filter(id => hasVerifiedUsage(quotas[id]));
  if (connected.length === 0) {
    return { recommendedProvider: null, headline: `${providerIds.map(id => providerNames[id]).join("·")} 연결 후 권장 서비스를 알려 드립니다`, chips };
  }
  let best = connected[0];
  for (const id of connected.slice(1)) {
    const candidate = primaryWindow(quotas[id]);
    const current = primaryWindow(quotas[best]);
    if (candidate && current && beats(candidate, current)) best = id;
  }
  return { recommendedProvider: best, headline: `지금은 ${providerNames[best]}에서 작업`, chips };
}
```

- [ ] **Step 5: 테스트 실행 — 통과 확인**

```bash
npm test
```

Expected: 기존 refresh-sequence 테스트 + 신규 6개 모두 PASS.

- [ ] **Step 6: 커밋**

```bash
git add src/data/next-action.ts tests/next-action.test.ts src/data/providers.ts
git commit -m "feat(phase2): add computeNextAction pure function (2-A logic)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 2: "다음 행동" 스트립 UI (2-A UI)

**Files:**
- Modify: `src/App.tsx` — 임포트(5행 부근), `viewCopy`(406행 부근), `VariantADesktop` 헤더(438행 부근), `VariantCMobile` `.mobile-title`(467행 부근), 새 `NextActionStrip` 컴포넌트
- Modify: `styles.css` — `.next-action` 규칙, `.mobile-title .next-action h1`

**Interfaces:**
- Consumes: `computeNextAction` (Task 1), `QuotaRecord` (App.tsx 34행)
- Produces: `NextActionStrip({ quotas: QuotaRecord; eyebrow?: string })` memo 컴포넌트 — Task 6의 `MiniLayout`이 재사용

- [ ] **Step 1: 임포트 및 `NextActionStrip` 컴포넌트 추가**

`src/App.tsx` 임포트 블록에 추가:

```typescript
import { computeNextAction } from "./data/next-action";
```

`QuotaBoard` 정의 바로 위(263행 부근)에 추가:

```tsx
const NextActionStrip = memo(function NextActionStrip({ quotas, eyebrow }: Readonly<{ quotas: QuotaRecord; eyebrow?: string }>) {
  const action = useMemo(() => computeNextAction(quotas), [quotas]);
  return <div className="next-action">
    {eyebrow ? <span className="eyebrow">{eyebrow}</span> : null}
    <h1>{action.headline}</h1>
    <ul className="next-action-chips" aria-label="판단 근거">{action.chips.map(chip => <li key={chip}>{chip}</li>)}</ul>
  </div>;
});
```

- [ ] **Step 2: `VariantADesktop` 헤더를 overview에서만 스트립으로 교체**

현재 `<header className="app-header">` 첫 자식:

```tsx
<div><span className="eyebrow">{copy.eyebrow}</span><h1>{copy.title.split("\n").map((line, index) => <span key={line}>{index > 0 ? <br /> : null}{index === copy.title.split("\n").length - 1 ? <em>{line}</em> : line}</span>)}</h1></div>
```

다음으로 교체(다른 뷰의 제목은 그대로 유지):

```tsx
{view === "overview"
  ? <NextActionStrip quotas={quotas} eyebrow={copy.eyebrow} />
  : <div><span className="eyebrow">{copy.eyebrow}</span><h1>{copy.title.split("\n").map((line, index) => <span key={line}>{index > 0 ? <br /> : null}{index === copy.title.split("\n").length - 1 ? <em>{line}</em> : line}</span>)}</h1></div>}
```

- [ ] **Step 3: `viewCopy`의 overview 장식 문장 제거**

`viewCopy` 마지막 return을 교체(overview 제목은 더 이상 렌더되지 않음):

```typescript
return { eyebrow: "개요 · 오늘", title: "" };
```

- [ ] **Step 4: `VariantCMobile`의 제목도 overview에서만 교체**

현재:

```tsx
<div className="mobile-title"><div><span className="eyebrow">{mobileDateFormatter.format(now)}</span><h1>사용 현황</h1></div><button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
```

교체:

```tsx
<div className="mobile-title">{view === "overview" ? <NextActionStrip quotas={quotas} eyebrow={mobileDateFormatter.format(now)} /> : <div><span className="eyebrow">{mobileDateFormatter.format(now)}</span><h1>사용 현황</h1></div>}<button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
```

- [ ] **Step 5: `styles.css`에 스트립 규칙 추가**

`.app-header h1` 규칙(59행 부근, `font-size: clamp(34px, 3.2vw, 54px)`) 바로 아래에 추가한다. `.next-action h1`은 `.app-header h1`과 명시도가 같으므로 반드시 뒤에 와야 우선한다:

```css
.next-action { display: flex; min-width: 0; flex-direction: column; gap: var(--space-2); }
.next-action h1 { max-width: none; margin-top: 0; font-size: 24px; font-weight: 700; letter-spacing: var(--tracking-tight); line-height: var(--leading-ui); }
.next-action-chips { display: flex; margin: 0; padding: 0; flex-wrap: wrap; gap: var(--space-2); list-style: none; }
.next-action-chips li { padding: 3px 10px; border-radius: var(--radius-pill); background: var(--color-surface-control); color: var(--color-muted); font-size: var(--text-xs); font-weight: 600; white-space: nowrap; }
.mobile-title .next-action h1 { margin-top: 7px; font-size: 24px; }
```

- [ ] **Step 6: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

Expected: 모두 PASS. `npm run dev` 명령을 사용자에게 제시하고, 브라우저 데모 개요에 "Codex·Claude 연결 후 권장 서비스를 알려 드립니다" + 칩 2개("Codex 연결 필요", "Claude 연결 필요")가 보이는지, 서비스·추이·설정 뷰 제목이 유지되는지 확인 결과를 받는다.

- [ ] **Step 7: 커밋**

```bash
git add src/App.tsx styles.css
git commit -m "feat(phase2): replace decorative headline with next-action strip (2-A)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 3: 쿼터 셀 시간 진행 트랙 (2-B)

**Files:**
- Create: `tests/time-progress.test.ts`
- Modify: `src/data/next-action.ts` — `computeTimeProgress`·`paceLabel` 추가
- Modify: `src/App.tsx` — `quotaFromSnapshot`(80행 부근) windows 매핑, `QuotaCell`(254행 부근)
- Modify: `styles.css` — `.quota-cell-time`·`.quota-cell-pace`

**Interfaces:**
- Consumes: `NativeProviderQuotaWindow.resetsAt`·`windowDurationMins` (`src/integrations/tauri-native-bridge.ts`), `QuotaWindow.resetsAt`·`windowDurationMins` (Task 1)
- Produces (Task 4가 사용):
  ```typescript
  export type Pace = "fast" | "steady" | "even";
  export type TimeProgress = Readonly<{ timePercent: number; pace: Pace }>;
  export function computeTimeProgress(window: Pick<QuotaWindow, "usedPercent" | "resetsAt" | "windowDurationMins">, now: number): TimeProgress | null;
  export function paceLabel(pace: Pace): string; // "빠름" | "여유" | "보통"
  ```

- [ ] **Step 1: 테스트 작성**

`tests/time-progress.test.ts`:

```typescript
import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { computeTimeProgress, paceLabel } from "../src/data/next-action.ts";

const now = 1_700_000_000_000;
const minute = 60 * 1000;

describe("computeTimeProgress", () => {
  it("returns null without reset time or duration", () => {
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: null, windowDurationMins: 300 }, now), null);
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: now + 60 * minute, windowDurationMins: null }, now), null);
    assert.equal(computeTimeProgress({ usedPercent: 50, resetsAt: now + 60 * minute, windowDurationMins: 0 }, now), null);
  });

  it("derives elapsed share from resetsAt minus duration", () => {
    const result = computeTimeProgress({ usedPercent: 50, resetsAt: now + 150 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 50);
    assert.equal(result?.pace, "even");
  });

  it("flags fast when usage runs more than 10 points ahead of time", () => {
    const result = computeTimeProgress({ usedPercent: 50, resetsAt: now + 240 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 20);
    assert.equal(result?.pace, "fast");
  });

  it("flags steady when time runs more than 10 points ahead of usage", () => {
    const result = computeTimeProgress({ usedPercent: 20, resetsAt: now + 60 * minute, windowDurationMins: 300 }, now);
    assert.equal(result?.timePercent, 80);
    assert.equal(result?.pace, "steady");
  });

  it("clamps to the 0–100 range", () => {
    assert.equal(computeTimeProgress({ usedPercent: 0, resetsAt: now - minute, windowDurationMins: 300 }, now)?.timePercent, 100);
    assert.equal(computeTimeProgress({ usedPercent: 0, resetsAt: now + 400 * minute, windowDurationMins: 300 }, now)?.timePercent, 0);
  });

  it("labels each pace in Korean", () => {
    assert.deepEqual([paceLabel("fast"), paceLabel("steady"), paceLabel("even")], ["빠름", "여유", "보통"]);
  });
});
```

- [ ] **Step 2: 테스트 실행 — 실패 확인**

```bash
npm test
```

Expected: FAIL — `computeTimeProgress`가 export되지 않음.

- [ ] **Step 3: `next-action.ts`에 구현 추가**

파일 끝에 추가:

```typescript
export type Pace = "fast" | "steady" | "even";
export type TimeProgress = Readonly<{ timePercent: number; pace: Pace }>;

export function computeTimeProgress(
  window: Pick<QuotaWindow, "usedPercent" | "resetsAt" | "windowDurationMins">,
  now: number
): TimeProgress | null {
  if (window.resetsAt == null || window.windowDurationMins == null || window.windowDurationMins <= 0) return null;
  const durationMs = window.windowDurationMins * 60 * 1000;
  const elapsed = now - (window.resetsAt - durationMs);
  const timePercent = Math.max(0, Math.min(100, (elapsed / durationMs) * 100));
  const gap = window.usedPercent - timePercent;
  return { timePercent, pace: gap > 10 ? "fast" : gap < -10 ? "steady" : "even" };
}

export function paceLabel(pace: Pace): string {
  return pace === "fast" ? "빠름" : pace === "steady" ? "여유" : "보통";
}
```

- [ ] **Step 4: 테스트 실행 — 통과 확인**

```bash
npm test
```

- [ ] **Step 5: `quotaFromSnapshot`에서 시간 필드 전달**

`src/App.tsx` `quotaFromSnapshot`의 `snapshot.windows.map(...)` 객체에 두 줄 추가:

```typescript
? snapshot.windows.map(window => ({
    id: window.id,
    label: window.label,
    usedPercent: window.usedPercent,
    remainingPercent: window.remainingPercent,
    resetLabel: formatReset(window.resetsAt),
    kindLabel: snapshot.source === "codex-app-server" ? "Codex 요금제 사용량" : "Claude.ai 공유 사용량",
    resetsAt: window.resetsAt,
    windowDurationMins: window.windowDurationMins
  }))
```

- [ ] **Step 6: `QuotaCell`에 시간 트랙 추가**

임포트 갱신:

```typescript
import { computeNextAction, computeTimeProgress, paceLabel } from "./data/next-action";
```

`QuotaCell`을 다음으로 교체(`window` 매개변수는 기존 이름 유지 — 전역 `window`가 아님):

```tsx
const QuotaCell = memo(function QuotaCell({ provider, window, available }: Readonly<{ provider: Provider; window: QuotaWindow; available: boolean }>) {
  const progress = available ? computeTimeProgress(window, Date.now()) : null;
  return <div className="quota-cell" style={providerStyle(provider.color)}>
    <div className="quota-cell-top"><span className="quota-cell-pill">{provider.name}</span><span className="quota-cell-window">{window.label}</span></div>
    <div className="quota-cell-value">{available ? Math.round(window.remainingPercent) : "—"}{available ? <span>%</span> : null}</div>
    <div className="quota-cell-meta">{available ? `초기화 · ${window.resetLabel}` : "연결 후 표시"}</div>
    <div className="quota-cell-meter" aria-label={available ? `${provider.name} ${window.label} ${Math.round(window.remainingPercent)}% 남음` : `${provider.name} ${window.label} 데이터 대기`}><i style={{ width: `${available ? window.remainingPercent : 0}%` }} /></div>
    {progress ? <div className="quota-cell-time"><i style={{ "--progress": `${progress.timePercent}%` } as CSSProperties} aria-label={`시간 진행 ${Math.round(progress.timePercent)}%`} />{progress.pace !== "even" ? <span className={`quota-cell-pace ${progress.pace}`}>{paceLabel(progress.pace)}</span> : null}</div> : null}
  </div>;
});
```

`Date.now()`는 렌더마다 1회 계산되며 `QuotaCell`은 memo이므로 새 스냅샷이 도착할 때만 갱신된다(타이머 없음 — 의도된 동작).

- [ ] **Step 7: `styles.css`에 규칙 추가**

`.quota-cell-meter` 규칙 아래에 추가:

```css
.quota-cell-time { display: flex; margin-top: 6px; align-items: center; gap: 6px; }
.quota-cell-time i { display: block; flex: 1; height: 3px; border-radius: 2px; background: linear-gradient(90deg, var(--color-subtle) var(--progress), var(--color-track) var(--progress)); }
.quota-cell-pace { font-size: var(--text-2xs); font-weight: 700; line-height: 1; }
.quota-cell-pace.fast { color: var(--color-coral); }
.quota-cell-pace.steady { color: var(--color-cyan); }
```

- [ ] **Step 8: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

- [ ] **Step 9: 커밋**

```bash
git add src/data/next-action.ts tests/time-progress.test.ts src/App.tsx styles.css
git commit -m "feat(phase2): add time-progress track and pace label to quota cells (2-B)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 4: 예시 그래프 제거 · 창 요약 SVG (2-C)

**Files:**
- Modify: `src/App.tsx` — 임포트(5행), `ChartBars`(233행 부근) 삭제, `WindowSummary` 추가, `VariantADesktop` live-card 본문 교체
- Modify: `src/data/providers.ts` — `usageBars` 삭제(파일 끝 부근)
- Modify: `styles.css` — `.bar-chart*`·`.chart-axis`·`@keyframes barRise` 삭제, `.window-summary*` 추가

**Interfaces:**
- Consumes: `hasVerifiedUsage`(Task 1), `computeTimeProgress`, `paceLabel`(Task 3), `providers`(providers.ts — `provider.name`·`provider.color` 사용), `providerStyle`(App.tsx 27행)
- Produces: `WindowSummary({ quotas: QuotaRecord })` — 점 ≤ 4개 + 기준선 1개인 정적 SVG와 텍스트 범례. 네이티브에서 예시 데이터 렌더 경로 0건(`usageBars` 삭제로 구조적으로 보장)

- [ ] **Step 1: `usageBars`·`ChartBars` 삭제**

- `src/data/providers.ts`에서 `export const usageBars = Object.freeze([...]);` 줄 삭제
- `src/App.tsx` 5행 임포트에서 `usageBars` 제거
- `src/App.tsx`의 `ChartBars` 컴포넌트(`function ChartBars` 또는 `const ChartBars = memo(...)`, 233-234행) 삭제

- [ ] **Step 2: `WindowSummary` 컴포넌트 추가**

임포트 갱신:

```typescript
import { computeNextAction, computeTimeProgress, hasVerifiedUsage, paceLabel } from "./data/next-action";
```

`ChartBars`가 있던 자리에 추가:

```tsx
const WindowSummary = memo(function WindowSummary({ quotas }: Readonly<{ quotas: QuotaRecord }>) {
  const now = Date.now();
  const points = providers.flatMap(provider => {
    const quota = quotas[provider.id];
    if (!hasVerifiedUsage(quota)) return [];
    return quota.windows.flatMap(window => {
      const progress = computeTimeProgress(window, now);
      return progress ? [{ provider, window, progress }] : [];
    });
  });
  if (points.length === 0) {
    return <div className="chart-empty"><Icon name="pulse" size={19} /><strong>서비스 연결 후 창 요약을 표시합니다.</strong><span>창별 시간 진행과 사용률을 한 그림에서 비교합니다.</span></div>;
  }
  return <figure className="window-summary">
    <svg viewBox="0 0 200 100" role="img" aria-label="창별 시간 진행 대비 사용률">
      <line x1="0" y1="100" x2="200" y2="0" />
      {points.map(({ provider, window, progress }) => <circle key={`${provider.id}-${window.id}`} cx={progress.timePercent * 2} cy={100 - window.usedPercent} r="4" style={providerStyle(provider.color)} />)}
    </svg>
    <ul className="window-summary-legend">
      {points.map(({ provider, window, progress }) => <li key={`${provider.id}-${window.id}`} style={providerStyle(provider.color)}><i />{provider.name} {window.label} · 사용 {Math.round(window.usedPercent)}% / 시간 {Math.round(progress.timePercent)}% · {paceLabel(progress.pace)}</li>)}
    </ul>
  </figure>;
});
```

대각선은 "사용률 = 시간 진행률" 기준선이다. 점이 선 위쪽이면 빠름, 아래쪽이면 여유이며 범례 텍스트가 같은 정보를 문장으로 제공한다.

- [ ] **Step 3: `VariantADesktop`의 live-card 교체**

현재 live-card `<article className="glass-card live-card span-2">…</article>` 전체를 다음으로 교체:

```tsx
<article className="glass-card live-card span-2"><div className="card-heading"><div><span className="eyebrow">창 요약</span><h3>{available ? `${Math.round(primary.usedPercent)}%` : "—"} <small>{available ? "현재 사용" : "실제 데이터 대기"}</small></h3></div><span className="live-pill"><i />{demo ? "브라우저 데모" : available ? "현재 스냅샷" : "연결 대기"}</span></div><WindowSummary quotas={quotas} /></article>
```

`demo`·`available`·`primary` 변수는 focus-card에서도 계속 사용되므로 유지한다. focus-card의 `{demo ? <Sparkline …/> : <div className="focus-status">…}`는 변경하지 않는다 — `source === "example"`은 브라우저 데모(`planQuotas`)에서만 발생하며 네이티브(`initialQuotaRecord`·`quotaFromSnapshot`)는 절대 생성하지 않는다.

- [ ] **Step 4: `styles.css` 정리 및 추가**

삭제: `@keyframes barRise` (12행), `.bar-chart`, `.bar-chart i`, `.bar-chart i:nth-child(3n+2)`, `.bar-chart i.peak`, `.chart-axis` (119-123행). `.chart-empty*` 규칙은 유지.

추가(`.chart-empty span` 규칙 아래):

```css
.window-summary { display: grid; margin: var(--space-3) 0 0; grid-template-columns: minmax(0, 1fr) minmax(0, 1fr); gap: var(--space-4); align-items: center; }
.window-summary svg { width: 100%; aspect-ratio: 2 / 1; border: 1px solid var(--line); border-radius: var(--radius-sm); background: var(--color-surface-control-soft); }
.window-summary line { stroke: var(--line-strong); stroke-width: 1; stroke-dasharray: 4 4; }
.window-summary circle { fill: var(--provider); }
.window-summary-legend { display: grid; margin: 0; padding: 0; gap: 6px; list-style: none; color: var(--muted); font-size: var(--text-xs); }
.window-summary-legend li { display: flex; align-items: center; gap: 8px; }
.window-summary-legend i { width: 8px; height: 8px; flex: none; border-radius: 50%; background: var(--provider); }
```

- [ ] **Step 5: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

번들 잔존 확인(둘 다 0건이어야 함):

```bash
grep -l "43,58,49,70" dist/assets/*.js; grep -l "bar-chart" dist/assets/*.css
```

- [ ] **Step 6: 커밋**

```bash
git add src/App.tsx src/data/providers.ts styles.css
git commit -m "feat(phase2): replace example bar chart with static window summary (2-C)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 5: 표면·배경 체계 정리 (2-D)

**Files:**
- Modify: `index.html` — 15-18행 `.ambient` 내부 `<i class="orb …">` 3개 삭제
- Modify: `styles.css` — `.ambient`(6행), `.orb*`(7-10행), `.app-surface`(17행), `.solid-mode`(18행), `.glass-card`(83행)
- Modify: `styles/tokens.css` — `--shadow-app`·`--shadow-card`(다크·라이트), `--glass-card-bg`·`--glass-card-blur` 삭제
- Modify: `scripts/verify-design-tokens.mjs` — 필수 토큰(27행)·소비자 검사(48행)
- Modify: `src/App.tsx` — 설정 뷰의 불투명 모드 설명 문구

**Interfaces:**
- Consumes: 없음(CSS·마크업 변경)
- Produces: `backdrop-filter`가 `.nav-rail`(1) · `.oauth-backdrop`(2, prefix 포함) · `.mobile-nav`(1) = 4건, 3행에만 존재. 그림자 블러 54px→24px, 34px→16px, 카드 그림자 2단(inset + drop)

- [ ] **Step 1: 오브 마크업 삭제**

`index.html`의 `.ambient`를 다음으로 교체:

```html
<div class="ambient" aria-hidden="true"></div>
```

- [ ] **Step 2: `.ambient`를 라디얼 합성으로 교체, `.orb*` 삭제**

`styles.css` 6-10행(`.ambient`, `.orb`, `.orb-violet`, `.orb-cyan`, `.orb-pink`)을 다음 한 줄로 교체. 라디얼 3겹은 기존 오브 3개의 위치·색을 근사하고, 알파는 `color-mix`로 색에 내장한다(`filter`·`opacity` 없음, 기본 그라디언트는 원래 밝기 유지):

```css
.ambient { position: fixed; inset: 0; overflow: hidden; pointer-events: none; z-index: 0; background: radial-gradient(ellipse 70% 60% at 2% 0%, color-mix(in srgb, var(--violet) 4%, transparent), transparent 70%), radial-gradient(ellipse 60% 55% at 100% 22%, color-mix(in srgb, var(--cyan) 4%, transparent), transparent 70%), radial-gradient(ellipse 70% 60% at 58% 110%, color-mix(in srgb, var(--pink) 4%, transparent), transparent 70%), var(--color-ambient-gradient); }
```

- [ ] **Step 3: `.app-surface`·`.glass-card`에서 `backdrop-filter` 제거**

17행을 교체:

```css
.app-surface, .stream-app { position: relative; min-height: calc(100vh - 52px); border: 1px solid var(--line); box-shadow: var(--shadow-app); background: var(--color-surface-solid); overflow: hidden; }
```

18행을 교체(불투명 모드는 이제 카드만 평탄화):

```css
.solid-mode .glass-card { background: var(--color-surface-solid) !important; }
```

83행을 교체(`--color-surface-card`는 불투명한 앱 표면 위에서 단순 알파 합성 — 배경 샘플링 없음):

```css
.glass-card { position: relative; min-height: 170px; padding: var(--space-5); overflow: hidden; border: 1px solid var(--line); border-radius: var(--radius-lg); background: var(--color-surface-card); box-shadow: var(--shadow-card); }
```

- [ ] **Step 4: 그림자 축소**

`styles/tokens.css` `:root` 블록:

```css
--shadow-app: 0 12px 24px rgba(0, 0, 0, .22), var(--shadow-inset);
--shadow-card: var(--shadow-inset), 0 8px 16px rgba(0, 0, 0, .12);
```

`[data-theme="light"]` 블록:

```css
--shadow-app: 0 12px 24px rgba(43, 61, 48, .09), var(--shadow-inset);
--shadow-card: var(--shadow-inset), 0 8px 16px rgba(43, 61, 48, .06);
```

`--shadow-control`·`--shadow-widget`은 스펙 범위 밖이므로 그대로 둔다.

- [ ] **Step 5: `--glass-card-*` 토큰 삭제 및 검증 스크립트 교체**

`styles/tokens.css`에서 두 줄 삭제:

```css
--glass-card-bg: var(--color-surface-card);
--glass-card-blur: 14px;
```

`--glass-app-*`·`--glass-nav-*`·`--glass-widget-*`은 유지(`nav-rail`·`mobile-nav`·`prototype.css`가 소비).

`scripts/verify-design-tokens.mjs`:
- `requiredTokens` 배열의 `"--glass-card-blur"` → `"--glass-nav-blur"`
- 48행 검사를 교체:
  ```javascript
  if (!/var\(--glass-nav-blur\)/.test(styles)) failures.push("내비게이션이 --glass-nav-blur 토큰을 사용하지 않습니다.");
  if (/\.glass-card\s*\{[^}]*backdrop-filter/.test(styles)) failures.push("카드에 backdrop-filter가 남아 있습니다.");
  ```

- [ ] **Step 6: 설정 문구 갱신**

`src/App.tsx` `DesktopViewPanel` 설정 뷰의 불투명 모드 설명:

```tsx
<span>{solid ? "현재 불투명 카드를 사용합니다." : "현재 반투명 카드를 사용합니다."}</span>
```

- [ ] **Step 7: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

`backdrop-filter` 건수 확인:

```bash
grep -c "backdrop-filter" styles.css; grep -o "backdrop-filter" styles.css | wc -l
```

Expected: `3` 행 / `4` 건.

- [ ] **Step 8: 커밋**

```bash
git add index.html styles.css styles/tokens.css scripts/verify-design-tokens.mjs src/App.tsx
git commit -m "feat(phase2): flatten surfaces, replace blur orbs with gradients (2-D)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 6: 미니 창 전용 밀도 (2-E)

**Files:**
- Modify: `src-tauri/src/desktop_shell.rs` — `impl WindowMode`(25행 부근), `show_main_window`(42행 부근), `mod tests`(134행 부근)
- Modify: `src/App.tsx` — `useWindowMode` 훅, `MiniLayout`, `App()` 렌더 분기(652행 부근)
- Modify: `styles.css` — `.mini-shell`·`.mini-layout`

**Interfaces:**
- Consumes: `WindowMode`(desktop_shell.rs), `NextActionStrip`(Task 2), `QuotaBoard`(기존), `isTauriRuntime`(bridge)
- Produces: Rust → 프론트 계약 `window.__SPECTRA_MODE__: "mini" | "dashboard"` + `window` 대상 `CustomEvent("spectra-mode", { detail })`. 미니 창에서 `NextActionStrip` + `QuotaBoard` 4셀 + 새로고침 버튼만 렌더

- [ ] **Step 1: Rust 테스트 작성**

`desktop_shell.rs` `mod tests`의 `use` 줄을 교체하고 테스트 2개 추가:

```rust
use super::{mode_script, WindowMode, MENU_HIDE, MENU_OPEN_DASHBOARD, MENU_OPEN_MINI, MENU_QUIT};

#[test]
fn window_mode_slug_matches_frontend_contract() {
    assert_eq!(WindowMode::Mini.slug(), "mini");
    assert_eq!(WindowMode::Dashboard.slug(), "dashboard");
}

#[test]
fn mode_script_sets_global_and_dispatches_event() {
    let script = mode_script(WindowMode::Dashboard);
    assert!(script.contains("window.__SPECTRA_MODE__='dashboard'"));
    assert!(script.contains("new CustomEvent('spectra-mode',{detail:'dashboard'})"));
}
```

- [ ] **Step 2: Rust 테스트 실행 — 실패 확인**

```bash
cd src-tauri && cargo test --lib desktop_shell
```

Expected: 컴파일 실패 — `slug`·`mode_script` 없음.

- [ ] **Step 3: `slug`·`mode_script` 구현 및 `show_main_window` 주입**

`impl WindowMode` 블록에 추가:

```rust
fn slug(self) -> &'static str {
    match self {
        Self::Mini => "mini",
        Self::Dashboard => "dashboard",
    }
}
```

`impl WindowMode` 아래에 자유 함수 추가:

```rust
fn mode_script(mode: WindowMode) -> String {
    let slug = mode.slug();
    format!("window.__SPECTRA_MODE__='{slug}';window.dispatchEvent(new CustomEvent('spectra-mode',{{detail:'{slug}'}}));")
}
```

`show_main_window`에서 `window.set_always_on_top(profile.always_on_top)?;` 다음 줄에 추가:

```rust
window.eval(&mode_script(mode))?;
```

앱 기동 시(`lib.rs` setup의 `show_main_window(app, WindowMode::Mini)`)에는 페이지 로드 전이라 스크립트가 유실될 수 있다 — 프론트 기본값이 Tauri 런타임에서 `"mini"`이므로 결과는 동일하다. 트레이 메뉴로 모드를 바꿀 때는 페이지가 이미 로드되어 있어 이벤트로 즉시 반영된다.

- [ ] **Step 4: Rust 테스트 실행 — 통과 확인**

```bash
cd src-tauri && cargo test --lib desktop_shell
```

Expected: 5 tests PASS.

- [ ] **Step 5: `useWindowMode` 훅 추가**

`src/App.tsx`의 `useIsMobile` 아래에 추가:

```typescript
type WindowMode = "mini" | "dashboard" | "browser";

declare global {
  interface Window { __SPECTRA_MODE__?: "mini" | "dashboard" }
}

function useWindowMode(): WindowMode {
  const [mode, setMode] = useState<WindowMode>(() => window.__SPECTRA_MODE__ ?? (isTauriRuntime() ? "mini" : "browser"));
  useEffect(() => {
    const onMode = (event: Event) => {
      const detail = (event as CustomEvent<string>).detail;
      if (detail === "mini" || detail === "dashboard") setMode(detail);
    };
    window.addEventListener("spectra-mode", onMode);
    return () => window.removeEventListener("spectra-mode", onMode);
  }, []);
  return mode;
}
```

- [ ] **Step 6: `MiniLayout` 컴포넌트 추가**

`VariantCMobile` 정의 아래에 추가:

```tsx
const MiniLayout = memo(function MiniLayout({ quotas, onRefresh, refreshing }: Readonly<{ quotas: QuotaRecord; onRefresh: () => void; refreshing: boolean }>) {
  return <div className="product-shell mini-shell"><section className="app-surface mini-layout">
    <NextActionStrip quotas={quotas} eyebrow="지금" />
    <QuotaBoard quotas={quotas} />
    <button type="button" className="primary-action mini-refresh" onClick={onRefresh} disabled={refreshing}><span>{refreshing ? "확인 중" : "지금 확인"}</span></button>
  </section></div>;
});
```

- [ ] **Step 7: `App()` 렌더 분기**

`App()` 첫 줄 `const isMobile = useIsMobile();` 아래에 `const windowMode = useWindowMode();` 추가. return을 교체(`verify:memory`가 `isMobile ? <VariantCMobile`와 `: <VariantADesktop` 문자열을 검사하므로 그대로 둔다):

```tsx
return <><div className={solid ? "solid-mode" : ""}>{windowMode === "mini" ? <MiniLayout quotas={quotas} onRefresh={refresh} refreshing={refreshing} /> : isMobile ? <VariantCMobile {...sharedProps} /> : <VariantADesktop {...sharedProps} />}</div><OAuthDialog open={oauthOpen} provider={oauthProvider} quota={oauthQuota} startResult={actionFeedback} onClose={closeOAuth} onConnect={connectOAuth} onDisconnect={disconnectOAuth} /></>;
```

- [ ] **Step 8: `styles.css`에 미니 규칙 추가**

`.product-shell` 규칙 아래에 추가:

```css
.mini-shell { padding: var(--space-3); }
.mini-layout { display: flex; min-height: calc(100vh - 24px); padding: var(--space-4); flex-direction: column; gap: var(--space-4); }
.mini-layout .next-action h1 { font-size: 20px; }
.mini-layout .quota-board { grid-template-columns: repeat(2, minmax(0, 1fr)); }
.mini-refresh { margin-top: auto; }
```

- [ ] **Step 9: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

네이티브 확인은 사용자에게 `npm run desktop:dev` 실행을 요청하고 다음 3가지 결과를 받는다: ① 기동 직후 미니 창에 스트립·4셀·버튼만 보임 ② 트레이 "대시보드 열기" → A 시안 전환 ③ 트레이 "미니 창 열기" → 미니 레이아웃 복귀.

- [ ] **Step 10: 커밋**

```bash
git add src-tauri/src/desktop_shell.rs src/App.tsx styles.css
git commit -m "feat(phase2): inject window mode and add mini-density layout (2-E)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 7: 라이트 테마 하드코딩 화이트 제거 (2-F)

**Files:**
- Modify: `styles/tokens.css` — `--color-surface-subtle` 추가(다크·라이트)
- Modify: `styles.css` — `.segmented, .range-tabs` · `.quota-window` · `.oauth-capability-note` · `.feed-item` 배경
- Modify: `scripts/verify-design-tokens.mjs` — 하드코딩 화이트 검사, 필수 토큰 추가

**Interfaces:**
- Consumes: 없음
- Produces: `styles.css`에 `#fff`·`rgba(255,255,255` 직접 사용 0건, `verify:tokens`가 재발을 차단

- [ ] **Step 1: 토큰 추가**

`styles/tokens.css` `:root`의 `--color-surface-control-soft` 아래:

```css
--color-surface-subtle: rgba(255, 255, 255, .025);
```

`[data-theme="light"]`의 `--color-surface-control-soft` 아래:

```css
--color-surface-subtle: rgba(32, 35, 31, .025);
```

- [ ] **Step 2: 검사 추가 — 먼저 실패 확인**

`scripts/verify-design-tokens.mjs`:
- `requiredTokens`에 `"--color-surface-subtle"` 추가
- `tokenCount` 계산 앞에 추가:
  ```javascript
  const hardcodedWhite = /#fff(?:fff)?\b|rgba\(\s*255\s*,\s*255\s*,\s*255/i;
  const whiteLines = styles.split("\n").flatMap((line, index) => hardcodedWhite.test(line) ? [index + 1] : []);
  if (whiteLines.length > 0) failures.push(`styles.css에 하드코딩 화이트가 남아 있습니다 (lines: ${whiteLines.join(", ")})`);
  ```

```bash
npm run verify:tokens
```

Expected: FAIL — 4개 행 보고(`tokens.css`의 `rgba(255,255,255,…)`는 토큰 정의이므로 검사 대상이 아님).

- [ ] **Step 3: 4개 선택자의 배경을 토큰으로 교체**

`styles.css`에서 다음 선택자의 `background: rgba(255,255,255,.025);`(`.feed-item`은 `.024`)를 모두 `background: var(--color-surface-subtle);`로 교체:

| 선택자 | 현재 |
|---|---|
| `.segmented, .range-tabs` | `rgba(255,255,255,.025)` |
| `.quota-window` | `rgba(255,255,255,.025)` |
| `.oauth-capability-note` | `rgba(255,255,255,.025)` |
| `.feed-item` | `rgba(255,255,255,.024)` |

- [ ] **Step 4: 빌드·검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
```

Expected: 모두 PASS, `grep -c "rgba(255" styles.css` → `0`.

- [ ] **Step 5: 커밋**

```bash
git add styles.css styles/tokens.css scripts/verify-design-tokens.mjs
git commit -m "feat(phase2): tokenize remaining hardcoded whites and guard them (2-F)

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
```

---

### Task 8: 검증 강화 · 기준선 재촬영 · 측정 · 상태 갱신

**Files:**
- Modify: `scripts/verify-memory-budget.mjs` — 스캔 대상·예시 차트 잔존 검사
- Modify: `docs/design-baseline/desktop-a.png` · `mobile-c.png` · `baseline.json` · `README.md`
- Modify: `docs/performance/memory-budget.md`, `docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` 상태 줄

**Interfaces:**
- Consumes: dist 산출물, 브라우저 캡처
- Produces: Phase 2 성공 기준 4항 충족 기록 — 기준선 3장 갱신, `backdrop-filter` ≤ 4건, 예시 렌더 경로 0건, 작업 집합 측정

- [ ] **Step 1: `verify-memory-budget.mjs` 강화**

`sourceFiles`에 `"src/data/next-action.ts"` 추가. JS 번들 검사 블록에 추가:

```javascript
if (/43,58,49,70,64,83/.test(bundle)) failures.push(`example chart data shipped in ${file}`);
```

CSS 검사의 `forbidden` 배열에 `"bar-chart"`, `"barRise"` 추가.

```bash
npm run verify:memory
```

Expected: PASS.

- [ ] **Step 2: 기준선 스크린샷 재촬영**

사용자에게 다음 명령 실행을 요청한다(실행자는 서버를 직접 띄우지 않음):

```bash
npm run build && npm run preview
```

서버가 뜨면 브라우저 도구로 `http://127.0.0.1:4173/`를 캡처한다(React 앱은 `?variant=` 쿼리를 처리하지 않으며 폭으로만 A/C를 결정한다):
- `docs/design-baseline/desktop-a.png` — 뷰포트 1440×1000 (A 시안)
- `docs/design-baseline/mobile-c.png` — 뷰포트 390×844 (C 시안)
- `docs/design-baseline/desktop-c-context.png` — 프로토타입 시기의 기기 미리보기 참고 화면으로 제품이 더 이상 렌더하지 않으므로 파일·해시 유지

새 해시 계산(ASCII 경로만 사용):

```bash
node -e "const {createHash}=require('node:crypto');const {readFileSync}=require('node:fs');for(const f of ['desktop-a.png','mobile-c.png'])console.log(f,createHash('sha256').update(readFileSync('docs/design-baseline/'+f)).digest('hex').toUpperCase())"
```

`baseline.json` 갱신: 두 asset의 `sha256`, `baselineRevision: 3`, `approvedAt`을 촬영일로. `README.md` 28-30행의 `?variant=A`·`?variant=C` 안내를 "`/`를 1440×1000 / 390×844로 확인"으로 고치고 제목 "디자인 기준선 1"은 유지.

```bash
npm run verify:baseline
```

Expected: PASS.

- [ ] **Step 3: 전체 검증**

```bash
npm run build
npm run verify:tokens
npm run verify:memory
npm run verify:baseline
npm test
cd src-tauri && cargo test --lib desktop_shell && cd ..
```

Expected: 모두 PASS. 성공 기준 확인 명령:

```bash
grep -o "backdrop-filter" styles.css | wc -l
```

Expected: `4`.

- [ ] **Step 4: Phase 2 메모리 측정**

릴리스 빌드 후 30분 측정(측정 스크립트는 기존 Phase 0·1과 동일):

```bash
npm run desktop:build
npm run measure:memory -- -Scenario mini-idle-phase2
```

측정 결과를 `docs/performance/memory-budget.md` 표에 `mini-idle-phase2` 행으로 추가(평균·최대 작업 집합, 평균 private, 프로세스 수)하고 Phase 1 대비 증감을 한 줄로 기록. 설치 프로그램 실행은 사용자 승인 후에만.

- [ ] **Step 5: 스펙 상태 갱신**

`docs/superpowers/specs/2026-09-04-design-footprint-improvement.md` 상태 줄을 다음으로 교체:

```
상태: 승인(2026-09-04, 권장안 4개 채택) · Phase 0·1·2 완료 · Phase 3 대기
```

- [ ] **Step 6: 커밋 · 푸시**

푸시 전 파일 목록과 대상 저장소(`origin main`)를 사용자에게 제시하고 확인을 받는다.

```bash
git add scripts/verify-memory-budget.mjs docs/design-baseline/ docs/performance/memory-budget.md docs/superpowers/specs/2026-09-04-design-footprint-improvement.md
git commit -m "chore(phase2): tighten verification, refresh design baseline, record measurements

Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>"
git push origin main
```
