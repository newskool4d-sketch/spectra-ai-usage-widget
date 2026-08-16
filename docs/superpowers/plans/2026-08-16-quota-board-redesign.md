# SPECTRA 쿼터 보드 & 타이포그래피 재설계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Raise SPECTRA's minimum readable text size, self-host Pretendard so the UI font is consistent regardless of what's installed on the user's machine, and show all four quota numbers (Claude/Codex × 5시간/주간) at once instead of one at a time.

**Architecture:** Pure frontend/CSS change. No Rust or data-model changes. A new `QuotaBoard`/`QuotaCell` component pair reads the already-existing `quotas.claude`/`quotas.codex` state and replaces the single-provider hero elements on both desktop and mobile. Typography gets four new size tokens applied only where text is currently below the 12px readability floor — sizes already ≥12px are left untouched (no regression risk). Four Pretendard static-weight `.woff2` files are vendored and registered via `@font-face` under the same family name already first in the `--font-ui` stack, so no font-stack change is needed.

**Tech Stack:** React 19 + TypeScript (`src/App.tsx`), plain CSS custom properties (`styles/tokens.css`, `styles.css`), Vite build, no test framework (verification uses `tsc --noEmit`, the project's `scripts/verify-*.mjs` checks, and live browser checks via the Claude Browser MCP tool).

## Global Constraints

- No new npm or cargo dependencies (spec 비목표). Fonts are vendored as static build assets only.
- Never make any font-size smaller than it currently renders (spec 목표: 최소 크기 바닥 12px).
- `activeProviderId`-driven views (services/trend/alerts) and their components (`ProviderChip`, `ProviderRow`, `QuotaWindowRow`, `MetricTabs`, `RangeTabs`, `ChartBars`) are unaffected — only the overview-screen hero element changes on each variant.
- Backend/data model unchanged: `PlanQuota`, `QuotaWindow`, `provider_usage.rs` are not touched.
- `src-tauri/src/main.rs` console-subsystem fix is already done and verified (separate track, not part of this plan) — do not re-touch it.
- Every CSS/TSX edit must leave `npm run verify:tokens` and `npx tsc --noEmit` passing. Do not reintroduce a `:root {}` block in `styles.css` (verify:tokens rejects it).
- `styles.css` lines 205–255 (`/* Variant B — spatial orbit */`) and lines 295–317 (`.widget-lab`/`.device-*`/`.mini-providers`/`.brand-mini`/`.ios-card`/`.platform-note`/`.prototype-switcher`) plus the media-query line for `.orbit-layout` inside `@media (max-width: 820px)` are **legacy prototype CSS with zero live consumers in `src/App.tsx`** (verified: `grep -c` for every one of these class names against `src/App.tsx` returns 0). Do not edit these ranges in this plan — touching them is unrelated cleanup out of scope for this feature. The one exception is noted explicitly in Task 2 (a `.primary-action` rule that happens to sit inside the Variant B line range but also matches a live element via CSS cascade).

---

## File Structure

- `styles/fonts/` (new directory) — 4 vendored Pretendard static-subset `.woff2` files + `LICENSE`. Pure static assets, no logic.
- `styles/tokens.css` — add typography-scale tokens, `@font-face` block, and two light-theme tokens (`--color-track` under `[data-theme="light"]`, new `--color-on-control-active`).
- `styles.css` — apply typography tokens to currently-sub-12px live rules, replace hardcoded white with the new tokens, add `.quota-board`/`.quota-cell` rules, remove the CSS exclusive to the deleted `QuotaSummaryCard`.
- `src/App.tsx` — add `QuotaCell` and `QuotaBoard` components; remove `QuotaSummaryCard`; wire `QuotaBoard` into `VariantADesktop` and `VariantCMobile`.
- `docs/design-baseline/desktop-a.png`, `docs/design-baseline/mobile-c.png`, `docs/design-baseline/baseline.json` — refreshed screenshots + bumped `baselineRevision`/hashes. `desktop-c-context.png` is **not** touched (see Task 6 note — it documents the retired app.js/B-variant reference material per `docs/design-baseline/README.md`, not the live product).

---

## Task 1: Vendor Pretendard static fonts

**Files:**
- Create: `styles/fonts/Pretendard-Regular.subset.woff2`
- Create: `styles/fonts/Pretendard-Medium.subset.woff2`
- Create: `styles/fonts/Pretendard-SemiBold.subset.woff2`
- Create: `styles/fonts/Pretendard-Bold.subset.woff2`
- Create: `styles/fonts/LICENSE`
- Modify: `styles/tokens.css` (add `@font-face` block after the `:root {` palette/typography section, before line 59's `/* Typography */` comment or directly after it)

**Interfaces:**
- Produces: a font-family named `"Pretendard Variable"` available at weights 400/500/600/700, matching the first entry of the existing `--font-ui` stack in `styles/tokens.css:60` — no other file needs to change to consume it.

- [ ] **Step 1: Download the four static-subset weight files and the license**

```bash
mkdir -p styles/fonts
curl -sS -L -o styles/fonts/Pretendard-Regular.subset.woff2 "https://cdn.jsdelivr.net/npm/pretendard@1.3.9/dist/web/static/woff2-subset/Pretendard-Regular.subset.woff2"
curl -sS -L -o styles/fonts/Pretendard-Medium.subset.woff2 "https://cdn.jsdelivr.net/npm/pretendard@1.3.9/dist/web/static/woff2-subset/Pretendard-Medium.subset.woff2"
curl -sS -L -o styles/fonts/Pretendard-SemiBold.subset.woff2 "https://cdn.jsdelivr.net/npm/pretendard@1.3.9/dist/web/static/woff2-subset/Pretendard-SemiBold.subset.woff2"
curl -sS -L -o styles/fonts/Pretendard-Bold.subset.woff2 "https://cdn.jsdelivr.net/npm/pretendard@1.3.9/dist/web/static/woff2-subset/Pretendard-Bold.subset.woff2"
curl -sS -L -o styles/fonts/LICENSE "https://raw.githubusercontent.com/orioncactus/pretendard/main/LICENSE"
```

- [ ] **Step 2: Verify the downloads are real font files, not error pages**

```bash
ls -la styles/fonts/
for f in styles/fonts/*.woff2; do file "$f"; done
```

Expected: each `.woff2` file is 260,000–271,000 bytes (Regular ≈267096, Medium ≈268324, SemiBold ≈268752, Bold ≈270784 bytes) and `file` reports `Web Open Font Format (Version 2)` for each — not an HTML/JSON error body. `LICENSE` should start with `Copyright (c) 2021` and mention "SIL Open Font License".

- [ ] **Step 3: Register the four faces in `styles/tokens.css`**

Insert this block immediately after line 57 (`--coral: var(--color-coral);`) and before the `/* Typography */` comment on line 59:

```css
  /* Fonts — self-hosted so the UI font stays consistent regardless of what's
     installed on the user's machine. Static subset (Korean + Latin only,
     not the full variable font) at the 4 weights this stylesheet uses. */
```

Then, above the `:root {` block (i.e. as the very first thing in the file, before line 1), add:

```css
@font-face {
  font-family: "Pretendard Variable";
  font-weight: 400;
  font-style: normal;
  font-display: swap;
  src: url("./fonts/Pretendard-Regular.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard Variable";
  font-weight: 500;
  font-style: normal;
  font-display: swap;
  src: url("./fonts/Pretendard-Medium.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard Variable";
  font-weight: 600;
  font-style: normal;
  font-display: swap;
  src: url("./fonts/Pretendard-SemiBold.subset.woff2") format("woff2");
}
@font-face {
  font-family: "Pretendard Variable";
  font-weight: 700;
  font-style: normal;
  font-display: swap;
  src: url("./fonts/Pretendard-Bold.subset.woff2") format("woff2");
}

:root {
```

(The existing `:root {` opening line moves down to follow the 4 new `@font-face` blocks; everything currently inside `:root { ... }` stays exactly as-is.) Do not change `--font-ui` on line 60 — it already lists `"Pretendard Variable"` first, so it now resolves to these local files instead of a system font.

- [ ] **Step 4: Verify the font actually loads in the running app**

```bash
npm run dev
```

Then, using the Claude Browser MCP tool: navigate to `http://127.0.0.1:5173`, and run via `javascript_tool`:

```javascript
await document.fonts.ready;
const loaded = [...document.fonts].filter(f => f.family.includes("Pretendard"));
JSON.stringify(loaded.map(f => ({ weight: f.weight, status: f.status })));
```

Expected: an array with 4 entries, each `status: "loaded"`, weights `"400"`, `"500"`, `"600"`, `"700"`. Also confirm via `read_network_requests` (filter `urlPattern: "woff2"`) that all 4 files returned HTTP 200.

- [ ] **Step 5: Confirm existing checks still pass and commit**

```bash
npm run verify:tokens
npx tsc --noEmit
```

Expected: both PASS (this task only adds CSS/assets, no token removed).

```bash
git add styles/fonts/ styles/tokens.css
git commit -m "feat: self-host Pretendard static subset fonts"
```

---

## Task 2: Typography scale — raise the sub-12px floor

**Files:**
- Modify: `styles/tokens.css` (add 4 size tokens)
- Modify: `styles.css` (apply tokens to every currently-live declaration below 12px)

**Interfaces:**
- Produces: `--text-2xs` (12px), `--text-xs` (13px), `--text-sm` (14px), `--text-2xl` (40px) custom properties in `styles/tokens.css`, consumed by `styles.css` in this task and by the new `.quota-cell` rules in Task 4.

**Rule** (deviates from the original design doc's 7-token table, which turned out to double-count: most existing sizes ≥12px are already fine and don't need touching — expanding the token set to retrofit them would be pure churn with no visible benefit). Every **live** `font-size` declaration currently **below 12px** maps by its exact current value, preserving relative ordering so nothing that was bigger becomes smaller-or-equal to something that was smaller:

| Current value | New token | Value |
|---|---|---|
| 7px, 8px | `--text-2xs` | 12px |
| 9px, 10px | `--text-xs` | 13px |
| 11px | `--text-sm` | 14px |

Declarations already ≥12px are left untouched. `--text-2xl` (40px) is not applied by this task — it's consumed by the new `QuotaCell` component in Task 4.

"Live" means the selector matches something actually rendered by `src/App.tsx` today. Verified dead (0 matches when grepped against `src/App.tsx`): everything in `styles.css` lines 205–255, lines 295–317, the `.orbit-layout` line inside the `@media (max-width: 820px)` block (line 359), and `.alert-card`/`.signal-timeline`/`.summary-stats`/`.detail-metrics` wherever they appear. Also skip lines 90, 91, 99, 100, 103, 110, 112, 115 (`.card-heading h2`, `.card-heading h2 span`, `.runway-ring strong/span`, `.quota-status`, `.runway-copy p`, `.quota-source`, `.track-labels`) — these are exclusive to `QuotaSummaryCard`, which Task 5 deletes; leave them for Task 5 to remove entirely rather than tokenizing code about to be deleted.

- [ ] **Step 1: Add the tokens to `styles/tokens.css`**

In the `/* Typography */` section (currently lines 59–65), add after `--leading-copy: 1.6;`:

```css
  --text-2xs: 12px;
  --text-xs: 13px;
  --text-sm: 14px;
  --text-2xl: 40px;
```

- [ ] **Step 2: Verify the tokens exist**

```bash
grep -c -- "--text-2xs:\|--text-xs:\|--text-sm:\|--text-2xl:" styles/tokens.css
```

Expected: `4`.

- [ ] **Step 3: Apply the mapping to every live declaration in `styles.css`**

Edit each of the following lines (selector shown for identification; replace only the `font-size` value, leave every other property on the line untouched):

```
L21  .eyebrow                                          11px → var(--text-sm)
L29  .brand-lockup small                                10px → var(--text-xs)
L37  .sync-state                                        11px → var(--text-sm)
L41  .avatar                                            11px → var(--text-sm)
L44  .provider-logo.xs                                   7px → var(--text-2xs)
L45  .provider-logo.sm                                   8px → var(--text-2xs)
L46  .provider-logo.md                                  10px → var(--text-xs)
L47  .provider-logo.lg                                  11px → var(--text-sm)
L65  .segmented button, .range-tabs button              11px → var(--text-sm)
L70  .view-description                                  11px → var(--text-sm)
L76  .alert-row strong                                  11px → var(--text-sm)
L77  .alert-row span                                     9px → var(--text-xs)
L82  .settings-row strong                                11px → var(--text-sm)
L83  .settings-row span                                   9px → var(--text-xs)
L84  .settings-note                                       9px → var(--text-xs)
L93  .card-heading h3 small, .budget-card h3 small       11px → var(--text-sm)
L94  .trend-badge, .live-pill                             9px → var(--text-xs)
L119 .quota-window-copy span                             10px → var(--text-xs)
L120 .quota-window-copy strong                           10px → var(--text-xs)
L121 .quota-window-meta                                   8px → var(--text-2xs)
L125 .quota-window.compact .quota-window-meta              7px → var(--text-2xs)
L133 .chart-empty strong                                 11px → var(--text-sm)
L134 .chart-empty span                                    9px → var(--text-xs)
L139 .text-button                                        10px → var(--text-xs)
L144 .provider-copy strong                                11px → var(--text-sm)
L145 .provider-copy small                                  9px → var(--text-xs)
L148 .provider-value                                       9px → var(--text-xs)
L150 .focus-card p                                        11px → var(--text-sm)
L152 .focus-status                                        10px → var(--text-xs)
L156 .budget-card > p                                     10px → var(--text-xs)
L158 .micro-stat span                                     10px → var(--text-xs)
L164 .alert-card button, .primary-action                  10px → var(--text-xs)
L167 .oauth-copy h3 (14px, already ≥12 — SKIP)
L168 .oauth-copy p                                        10px → var(--text-xs)
L169 .oauth-status                                          9px → var(--text-xs)
L172 .oauth-capability                                      8px → var(--text-2xs)
L173 .oauth-capability strong                                8px → var(--text-2xs)
L174 .oauth-button, .secondary-action, .danger-action     10px → var(--text-xs)
L176 .oauth-security                                        8px → var(--text-2xs)
L178 .plan-card > p                                         9px → var(--text-xs)
L188 .oauth-provider span                                   9px → var(--text-xs)
L189 .oauth-capability-note                                  8px → var(--text-2xs)
L190 .oauth-capability-note strong                           9px → var(--text-xs)
L193 .oauth-steps li > span                                  8px → var(--text-2xs)
L196 .oauth-steps small                                      9px → var(--text-xs)
L197 .oauth-demo-note                                        9px → var(--text-xs)
L201 .privacy-strip                                         10px → var(--text-xs)
L247 .primary-action (Variant-B-labeled block, but this
     rule also matches the live settings-panel refresh
     button via CSS cascade — see note below)             10px → var(--text-xs)
L262 .mobile-top                                            11px → var(--text-sm)
L272 .hero-signal > p (deleted in Task 5 along with the
     rest of .hero-signal — SKIP here, handled there)
L275 .signal-foot                                           10px → var(--text-xs)
L280 .provider-chip strong                                  10px → var(--text-xs)
L280 .provider-chip small                                    9px → var(--text-xs)
L283 .section-title h3 (18px, already ≥12 — SKIP)
L284 .section-title button                                    9px → var(--text-xs)
L288 .feed-time                                               9px → var(--text-xs)
L289 .feed-item h4 (12px, already ≥12 — SKIP)
L290 .feed-item p                                            10px → var(--text-xs)
L291 .feed-item > strong                                     11px → var(--text-sm)
L293 .mobile-nav button                                        9px → var(--text-xs)
L332 (inside @media max-width:820px) — convert only .quota-window-copy
     span/strong 9px → var(--text-xs) and .quota-window-meta 7px →
     var(--text-2xs). Do NOT touch the .quota-status/.quota-source portions
     of this same line — they're QuotaSummaryCard-exclusive and Task 5
     deletes them outright; tokenizing them here would be wasted work.
L334 .oauth-copy h3 (13px, already ≥12px — SKIP, do not round up; the rule
     is "leave ≥12px alone", not "make every media-query override match
     its desktop counterpart")
L335 .oauth-copy p                                            10px → var(--text-xs)
L336 .oauth-capability                                          9px → var(--text-xs)
L337 .oauth-capability strong                                   9px → var(--text-xs)
L351 .oauth-capability-note, .oauth-capability-note strong     10px → var(--text-xs)
L354 .oauth-steps small                                        10px → var(--text-xs)
L355 .oauth-demo-note                                            10px → var(--text-xs)
```

**Note on L247:** `styles.css` has two separate `.primary-action { }` rules — one at L164 (grouped with `.alert-card button`) and one at L247 (grouped with the Variant B orbit layout, `display:flex;width:100%;...`). Because CSS cascade applies by property regardless of which "section" a rule visually sits in, and both selectors match the live `<button className="primary-action">` in the settings panel (`src/App.tsx` line 404), **both** rules' `font-size` must be updated or the L247 declaration (later in the file) will silently override the L164 one and the token change will have no visible effect on that button. This is the one required edit inside the "Variant B" line range; do not change anything else in that range.

Do not touch `L110 .runway-copy p` (already 12px, but it's `QuotaSummaryCard`-exclusive — Task 5 deletes the whole rule).

- [ ] **Step 4: Verify no live element renders below 12px**

```bash
grep -nE "font-size:\s*[0-9]px" styles.css
```

Expected: every remaining match with a value below 12px is inside lines 205–255, 295–317, or line 359 (the excluded legacy ranges), or is `.alert-card`/`.signal-timeline`/`.summary-*`/`.detail-metrics`. If a match outside those ranges still shows a raw sub-12 `px` value, it was missed — go back and convert it.

Then, with the dev server running, use the Claude Browser MCP tool to spot-check the cascade-sensitive case from the note above:

```javascript
const btn = document.querySelector(".settings-row .primary-action");
getComputedStyle(btn).fontSize;
```

Expected: `"13px"` (i.e. `var(--text-xs)`, not `"10px"`).

- [ ] **Step 5: Full regression check and commit**

```bash
npx tsc --noEmit
npm run verify:tokens
npm run verify:memory
```

Expected: all PASS.

```bash
git add styles/tokens.css styles.css
git commit -m "feat: raise minimum readable font size to 12px"
```

---

## Task 3: Light theme token fixes

**Files:**
- Modify: `styles/tokens.css` (add `--color-track` under `[data-theme="light"]`, add new `--color-on-control-active` to both the dark `:root` block and the light block)
- Modify: `styles.css` (replace hardcoded white with the two tokens above)

**Interfaces:**
- Produces: `--color-on-control-active` and a light-mode value for `--color-track`, consumed by existing selectors in this task and by the new `.quota-cell-meter` rule in Task 4.

This closes a gap found during the prior code review: `--color-track` (styles/tokens.css:24) is defined only in the dark `:root` block; the `[data-theme="light"]` block never redefines it, and several selectors hardcode `var(--color-white)`/`rgba(255,255,255,…)` instead of a themeable token, so active tab labels and meter tracks are invisible in light mode.

- [ ] **Step 1: Add `--color-on-control-active` to the dark palette**

In `styles/tokens.css`, immediately after line 25 (`--color-white: #fff;`), add:

```css
  --color-on-control-active: var(--color-white);
```

- [ ] **Step 2: Add the light-mode overrides**

In the `[data-theme="light"]` block (starts at `styles/tokens.css:115`), immediately after the line `--color-track: rgba(32, 35, 31, .09);` (already present at what is currently line 24's dark counterpart — confirm the light block has its own `--color-track` line; if it's missing, add it), ensure these two lines exist:

```css
  --color-track: rgba(32, 35, 31, .09);
  --color-on-control-active: #20231f;
```

(`--color-track` may already exist in the light block from an earlier revision — check with `grep -n -- "--color-track" styles/tokens.css` first; if it prints two lines, one per theme block, only add `--color-on-control-active`. If it prints one line, add both.)

- [ ] **Step 3: Replace hardcoded white in `styles.css`**

Replace these exact occurrences:

```
L55  .nav-rail button:hover, .nav-rail button.active { color: var(--color-white); ...
     → color: var(--color-on-control-active);
L66  .segmented button.active, .range-tabs button.active { color: var(--color-white); ...
     → color: var(--color-on-control-active);
L111 .runway-copy p strong { color: var(--color-white); }
     → DELETE this rule (QuotaSummaryCard-exclusive, removed in Task 5 — do not tokenize dead-soon code)
L120 .quota-window-copy strong { color: var(--color-white); font-size: ...}
     → color: var(--color-on-control-active); (keep — QuotaWindowRow is still live via the trend view)
L212 .orbit-nav button.active { color: var(--color-white); ... }
     → SKIP (Variant B, dead)
L238 .orbit-node:hover, .orbit-node.active { ...color: var(--color-white); ...}
     → SKIP (Variant B, dead)
L312 .platform-note strong{color:#fff;font-size:11px}
     → SKIP (widget-lab family, dead)
```

Then replace the meter-track hardcodes that belong to **live** selectors only:

```
L122 .quota-window-meter { height: 3px; margin-top: 8px; overflow: hidden; border-radius: 4px; background: rgba(255,255,255,.08); }
     → background: var(--color-track);
L146 .provider-meter { height: 3px; overflow: hidden; border-radius: 3px; background: rgba(255,255,255,.07); }
     → background: var(--color-track);
L264 .mobile-top i { width: 3px; border-radius: 2px; background: #fff; }...
     → background: var(--color-on-control-active); (the rest of that compact line — the `:nth-child` height rules — is unchanged)
L273 .spectrum-line { height: 4px; margin-top: 24px; overflow: hidden; border-radius: 8px; background: rgba(255,255,255,.07); }
     → background: var(--color-track);
```

Leave `L96`/`L101` (`.runway-ring`/`.quota-ring` conic-gradient `rgba(255,255,255,.07)` stops) and `L225`/`L227`/`L229` (Variant B orbit rings) untouched — `.runway-ring`/`.quota-ring` are deleted in Task 5 along with the rest of `QuotaSummaryCard`, and the orbit ones are dead code out of scope.

- [ ] **Step 4: Verify in both themes**

With the dev server running, use the Claude Browser MCP tool: navigate to `http://127.0.0.1:5173`, resize to desktop width (≥820px), and run:

```javascript
document.querySelector(".nav-rail button.active")?.click();
getComputedStyle(document.querySelector(".range-tabs button.active")).color;
```

Take a screenshot in dark mode (default), then click the theme toggle (`button[aria-label="테마 전환"]`) and screenshot again. Expected: the active tab label is clearly visible (dark ink on light background) in the light screenshot, not white-on-light.

- [ ] **Step 5: Verify and commit**

```bash
npx tsc --noEmit
npm run verify:tokens
```

Expected: PASS.

```bash
git add styles/tokens.css styles.css
git commit -m "fix: theme meter tracks and active-tab text with tokens instead of hardcoded white"
```

---

## Task 4: `QuotaCell` and `QuotaBoard` components

**Files:**
- Modify: `src/App.tsx` (add two new components, after `QuotaWindowRow` at line 242 and before `ProviderRow` at line 244)
- Modify: `styles.css` (add `.quota-board`/`.quota-cell*` rules, after the `.span-2` rule at line 88)

**Interfaces:**
- Consumes: `QuotaRecord` (`Readonly<Record<ProviderId, PlanQuota>>`, already defined at `src/App.tsx:33`), `providers` (from `src/data/providers.ts`), `Provider`/`PlanQuota`/`QuotaWindow`/`ProviderId` types (from `src/data/providers.ts`), `providerStyle`/`hasDisplayValue` helpers (`src/App.tsx:26,37`).
- Produces: `QuotaBoard` component with signature `({ quotas }: Readonly<{ quotas: QuotaRecord }>) => JSX.Element`, consumed by Task 5.

`PlanQuota.windows` is expected to contain exactly one `"rolling"` and one `"weekly"` entry (see `src/data/providers.ts:70-73,86-89` and `nativePendingQuota` at `src/App.tsx:113-131`, both always producing 2), but the real native snapshot path (`src-tauri/src/provider_usage.rs`) can return 0, 1, or 2 windows depending on what Codex/Claude's own APIs report at that moment — so `QuotaBoard` must not assume `windows.length === 2` or a fixed order. It looks up each window by `id` and synthesizes a placeholder cell if that id is missing, so the grid always shows exactly 4 cells.

- [ ] **Step 1: Add `QuotaCell` and `QuotaBoard` to `src/App.tsx`**

Insert immediately after the closing `});` of `QuotaWindowRow` (line 242):

```tsx
const emptyQuotaWindow = (id: QuotaWindowId): QuotaWindow => ({
  id,
  label: id === "rolling" ? "5시간 한도" : "주간 한도",
  usedPercent: 0,
  remainingPercent: 0,
  resetLabel: "연결 후 표시",
  kindLabel: "실제 데이터 대기"
});

const QuotaCell = memo(function QuotaCell({ provider, quota, window }: Readonly<{ provider: Provider; quota: PlanQuota; window: QuotaWindow }>) {
  const available = hasDisplayValue(quota);
  return <div className="quota-cell" style={providerStyle(provider.color)}>
    <div className="quota-cell-top"><span className="quota-cell-pill">{provider.name}</span><span className="quota-cell-window">{window.label}</span></div>
    <div className="quota-cell-value">{available ? Math.round(window.remainingPercent) : "—"}{available ? <span>%</span> : null}</div>
    <div className="quota-cell-meta">{available ? `초기화 · ${window.resetLabel}` : "연결 후 표시"}</div>
    <div className="quota-cell-meter" aria-label={available ? `${provider.name} ${window.label} ${Math.round(window.remainingPercent)}% 남음` : `${provider.name} ${window.label} 데이터 대기`}><i style={{ width: `${available ? window.remainingPercent : 0}%` }} /></div>
  </div>;
});

const QuotaBoard = memo(function QuotaBoard({ quotas }: Readonly<{ quotas: QuotaRecord }>) {
  const windowIds: readonly QuotaWindowId[] = ["rolling", "weekly"];
  return <div className="quota-board span-2" role="group" aria-label="공급자별 한도 현황">
    {providers.flatMap(provider => windowIds.map(id => {
      const quota = quotas[provider.id];
      const window = quota.windows.find(candidate => candidate.id === id) ?? emptyQuotaWindow(id);
      return <QuotaCell key={`${provider.id}-${id}`} provider={provider} quota={quota} window={window} />;
    }))}
  </div>;
});
```

The `span-2` class (`styles.css:88`, `grid-column: span 2`) is included unconditionally — it's what desktop's `bento-grid` needs to give the board 2 of its 4 columns (Task 5), and it's inert on mobile since `.mobile-stream` is a block container, not a grid, so `grid-column` has no effect there.

This requires `QuotaWindowId` to be imported from `src/data/providers.ts` — add it to the existing import on `src/App.tsx:4`:

```tsx
import { metricLabels, planQuotas, providers, rangeLabels, usageBars, type AuthMethod, type Metric, type PlanQuota, type Provider, type ProviderId, type QuotaWindow, type QuotaWindowId, type UsageRange } from "./data/providers";
```

- [ ] **Step 2: Add the grid CSS**

Insert immediately after `styles.css:88` (`.span-2 { grid-column: span 2; }`):

```css
.quota-board { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); }
.quota-cell { position: relative; padding: var(--space-5); overflow: hidden; border: 1px solid color-mix(in srgb, var(--provider), transparent 68%); border-radius: var(--radius-lg); background: color-mix(in srgb, var(--provider), var(--color-surface-inner) 95%); box-shadow: var(--shadow-inset); }
.quota-cell-top { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); }
.quota-cell-pill { padding: 3px 9px; border-radius: var(--radius-pill); color: var(--provider); background: color-mix(in srgb, var(--provider), transparent 85%); font-size: var(--text-xs); font-weight: 700; }
.quota-cell-window { color: var(--subtle); font-size: var(--text-xs); }
.quota-cell-value { margin-top: 10px; font-size: var(--text-2xl); line-height: .95; letter-spacing: -.04em; }
.quota-cell-value span { margin-left: 2px; color: var(--provider); font-size: var(--text-sm); }
.quota-cell-meta { margin-top: 6px; color: var(--muted); font-size: var(--text-xs); }
.quota-cell-meter { height: 4px; margin-top: 12px; overflow: hidden; border-radius: 4px; background: var(--color-track); }
.quota-cell-meter i { display: block; height: 100%; background: var(--provider); }
```

- [ ] **Step 3: Type-check**

```bash
npx tsc --noEmit
```

Expected: PASS. `QuotaBoard`/`QuotaCell` are defined but not yet referenced anywhere (Task 5 wires them in) — this is fine, TypeScript does not flag unused top-level component declarations as errors the way it would an unused local variable.

- [ ] **Step 4: Commit**

```bash
git add src/App.tsx styles.css
git commit -m "feat: add QuotaBoard component showing all provider windows at once"
```

---

## Task 5: Wire `QuotaBoard` into desktop and mobile, remove `QuotaSummaryCard`

**Files:**
- Modify: `src/App.tsx` (`VariantADesktop`, `VariantCMobile`; delete `QuotaSummaryCard`)
- Modify: `styles.css` (remove the CSS exclusive to `QuotaSummaryCard`)

**Interfaces:**
- Consumes: `QuotaBoard` from Task 4.

- [ ] **Step 1: Delete the `QuotaSummaryCard` component**

Task 4 inserted new code before this point in the file, so the original line numbers (261–269) have shifted — run `grep -n "const QuotaSummaryCard" src/App.tsx` to find its current location, then remove the whole `const QuotaSummaryCard = memo(function QuotaSummaryCard(...) { ... });` block (it ends at the first `});` that closes the `memo(function QuotaSummaryCard...` call). Nothing else references `Math.round`/`meterPercent` in a way that breaks — those helpers are used elsewhere too and are untouched.

- [ ] **Step 2: Replace it on desktop**

In `VariantADesktop` (find the line `<QuotaSummaryCard provider={activeProvider} quota={activeQuota} />` inside the `bento-grid`), replace with:

```tsx
        <QuotaBoard quotas={quotas} />
```

`QuotaBoard` already carries its own `span-2` class (Task 4), so no extra wrapping is needed here — it takes the same 2-of-4 grid columns `QuotaSummaryCard` used to.

`VariantADesktop` already destructures `quotas` from its props (`SharedViewProps` includes it — other desktop code like `ProviderRow` in the `providers-card` already reads `quotas[provider.id]`), so it's already in scope; no signature change needed.

- [ ] **Step 3: Replace it on mobile**

In `VariantCMobile`, replace the entire `<article className="hero-signal" ...>...</article>` block (the one starting right after `{view === "overview" ? <>`) with:

```tsx
    {view === "overview" ? <><QuotaBoard quotas={quotas} />
```

(keeping everything after it — `<div className="chip-scroll">...`, `<OAuthConnectCard .../>`, `<section className="stream-feed">...` — unchanged, only the opening fragment content changes).

Then remove the now-unused locals at the top of `VariantCMobile`:

```tsx
  const primary = activeQuota.windows[0];
  const available = hasDisplayValue(activeQuota);
```

(Confirm before deleting: `primary` and `available` are not referenced anywhere else in `VariantCMobile` after the hero block is removed — `grep -n "primary\|available" ` within the function body should show zero remaining uses once the hero markup is gone. `now`, `codex`, `claude`, `codexQuota`, `claudeQuota` stay, they're used by the mobile-top clock and the stream-feed section.)

- [ ] **Step 4: Remove the CSS exclusive to `QuotaSummaryCard`**

Delete these rules from `styles.css` entirely (all confirmed to have no other consumer after Step 1):

```
L90  .card-heading h2 { ... }
L91  .card-heading h2 span { ... }
L99  .runway-ring strong { ... }
L100 .runway-ring span { ... }
L103 .quota-status { ... }
L110 .runway-copy p { ... }
L111 .runway-copy p strong { ... }  (already flagged for deletion in Task 3, remove here if not already gone)
L112 .quota-source { ... }
L115 .track-labels { ... }
```

And the `.runway-content`/`.runway-ring`/`.quota-ring`/`.rainbow-track`/`.quota-card`/`.runway-card` rules in the same neighborhood (lines ~95–118) — read the current file first (line numbers will have shifted from earlier tasks' edits) and remove every selector containing `runway-`, `.quota-card`, `.quota-ring`, `.rainbow-track`, or `.quota-status` (but **not** `.quota-window-*`, `.quota-cell*`, or `.quota-board` — those are shared/new and must stay).

Also delete the mobile override at what was `styles.css:332` (inside `@media max-width:820px`): the `.quota-status { max-width: 142px; font-size: ...}.quota-source { font-size: ...}` portion of that combined line — but **keep** the `.quota-window-list`/`.quota-window-copy`/`.quota-window-meta` portions of the same line, since those still apply to the live trend-view `QuotaWindowRow`.

- [ ] **Step 5: Manual verification in the browser**

```bash
npm run dev
```

Using the Claude Browser MCP tool:
1. Navigate to `http://127.0.0.1:5173`, resize to 1440×1000 (desktop). Screenshot. Expected: 4 cells (Claude 5h, Claude weekly, Codex 5h, Codex weekly) visible near the top of the overview screen, each showing a percentage, a reset label, and a meter bar.
2. Resize to 390×844 (mobile). Screenshot. Expected: the same 4-cell board at the top of the mobile stream, 2 columns, readable without overflow.
3. Toggle theme (light) at both sizes, screenshot again. Expected: cells remain fully legible (this depends on Task 3's fixes already being in place).
4. Switch to the "서비스"/"추이"/"알림" tabs on both variants. Expected: unaffected — `ProviderRow`/`QuotaWindowRow`/alert rows render exactly as before.

- [ ] **Step 6: Full regression check and commit**

```bash
npx tsc --noEmit
npm run verify:tokens
npm run verify:memory
```

Expected: all PASS. (`verify:memory` re-asserts the JS bundle is ≤500KB and ≤3 files — run `npm run build` first since that check reads `dist/assets`.)

```bash
npm run build
npm run verify:memory
```

```bash
git add src/App.tsx styles.css
git commit -m "feat: show all four provider quota windows at once on overview"
```

---

## Task 6: Refresh the design baseline

**Files:**
- Modify: `docs/design-baseline/desktop-a.png` (replace)
- Modify: `docs/design-baseline/mobile-c.png` (replace)
- Modify: `docs/design-baseline/baseline.json` (bump `baselineRevision`, update the two asset hashes)

Do **not** touch `docs/design-baseline/desktop-c-context.png` — inspecting it shows it documents the retired multi-provider app.js prototype with a device-preview panel and a prototype-switcher bar (6 providers: OpenAI/Claude/Gemini/Cursor/Copilot/Perplexity; none of this exists in `src/App.tsx`, which only has Codex + Claude). Per `docs/design-baseline/README.md`, "B와 화면 아래 시안 전환 막대는 실험 기록입니다" — that file is reference material for the retired B variant, not a snapshot of the shipping product, so this feature doesn't change what it should depict.

**Known pre-existing gap, not fixed by this task:** `desktop-a.png` and `mobile-c.png` themselves currently *also* depict the old app.js prototype (6 providers, donut chart, monthly-cost card, prototype-switcher bar) rather than the current React product — they were captured before the React rewrite (commit `d09478f`, before `2379e88 feat: add A+C React product shell`) and never refreshed since. This plan brings them in sync with the real product for the first time as a side effect of doing an intentional baseline update anyway; a full audit of whether `scripts/verify-design-baseline.mjs`'s design (which never compares screenshot pixels to live rendered output, only to its own recorded hash) is the right verification strategy going forward is out of scope here.

- [ ] **Step 1: Build and preview the app**

```bash
npm run build
npm run preview
```

- [ ] **Step 2: Capture the desktop screenshot**

Using the Claude Browser MCP tool: navigate to `http://127.0.0.1:4173`, resize to 1440×1000, wait for the overview screen to render with real quota-board cells visible, then use `computer` (`screenshot` action) or the browser's screenshot capability to save a PNG. Save it to `docs/design-baseline/desktop-a.png`, overwriting the existing file.

- [ ] **Step 3: Capture the mobile screenshot**

Resize to 390×844. Screenshot, save to `docs/design-baseline/mobile-c.png`, overwriting the existing file.

- [ ] **Step 4: Recompute hashes and update the manifest**

```bash
python -c "
import hashlib
for name in ['desktop-a.png', 'mobile-c.png']:
    path = f'docs/design-baseline/{name}'
    with open(path, 'rb') as f:
        print(name, hashlib.sha256(f.read()).hexdigest().upper())
"
```

In `docs/design-baseline/baseline.json`: set `"baselineRevision": 2`, and update the two matching `sha256` fields under `assets` (for `desktop-a.png` and `mobile-c.png` only — leave the `desktop-c-context.png` entry's hash unchanged since that file isn't touched) to the values just printed.

- [ ] **Step 5: Verify**

```bash
npm run verify:baseline
```

Expected: `DESIGN BASELINE: PASS`.

- [ ] **Step 6: Commit**

```bash
git add docs/design-baseline/desktop-a.png docs/design-baseline/mobile-c.png docs/design-baseline/baseline.json
git commit -m "docs: refresh design baseline for quota board and typography redesign"
```

---

## Task 7: Final full verification pass

**Files:** none (verification only).

- [ ] **Step 1: Full build and static checks**

```bash
npx tsc --noEmit
npm run build
npm run verify:tokens
npm run verify:baseline
npm run verify:memory
```

Expected: all PASS.

- [ ] **Step 2: Rust sanity check (confirm nothing in this plan touched the Rust side)**

```bash
cd src-tauri && cargo check --message-format short
```

Expected: PASS, no changes (this plan is frontend-only; the console-subsystem fix in `src-tauri/src/main.rs` was already committed separately before this plan started).

- [ ] **Step 3: Four-combination manual browser sweep**

Using the Claude Browser MCP tool against `npm run preview` (port 4173):
1. Desktop (1440×1000) × dark — screenshot, confirm quota board legible, no white-on-dark regressions.
2. Desktop (1440×1000) × light — screenshot, confirm quota board and active tabs legible (Task 3 fix holds).
3. Mobile (390×844) × dark — screenshot.
4. Mobile (390×844) × light — screenshot.

For each, also check `document.fonts` reports the 4 Pretendard weights as `"loaded"` (Task 1's check, re-run once more here as a final regression guard).

- [ ] **Step 4: Report to the user**

Summarize: font now self-hosted (list the 4 weights), minimum text size raised to 12px, quota board shows all 4 numbers at once on both desktop and mobile, light theme text/meter visibility fixed, baseline screenshots refreshed (with the pre-existing-staleness note from Task 6 surfaced, not hidden). Remind the user that a new NSIS installer build (`npm run desktop:build`) and reinstall are still needed to see this in the installed app — building/installing was intentionally left out of this plan (it's a separate, explicit-consent action per the earlier conversation).
