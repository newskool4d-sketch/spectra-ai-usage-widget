const providers = [
  { id: "openai", name: "OpenAI", short: "OA", color: "#56B7B0", used: 68, tokens: "3.8M", cost: "$18.40", reset: "3시간 42분", trend: [28, 34, 31, 48, 44, 63, 68] },
  { id: "claude", name: "Claude", short: "CL", color: "#D88463", used: 76, tokens: "2.6M", cost: "$14.90", reset: "1시간 18분", trend: [40, 42, 51, 49, 64, 71, 76] },
  { id: "gemini", name: "Gemini", short: "GM", color: "#8879C6", used: 41, tokens: "1.2M", cost: "$6.20", reset: "8시간", trend: [22, 33, 28, 37, 44, 39, 41] },
  { id: "cursor", name: "Cursor", short: "CU", color: "#8CB66B", used: 54, tokens: "892K", cost: "$4.80", reset: "12일", trend: [18, 26, 31, 38, 36, 49, 54] },
  { id: "copilot", name: "Copilot", short: "CP", color: "#C77799", used: 29, tokens: "440K", cost: "$2.10", reset: "18일", trend: [12, 18, 21, 17, 26, 24, 29] },
  { id: "perplexity", name: "Perplexity", short: "PX", color: "#C7A852", used: 18, tokens: "318K", cost: "$1.70", reset: "18일", trend: [9, 12, 15, 11, 16, 14, 18] }
];

const state = {
  variant: ["A", "B", "C"].includes(new URLSearchParams(location.search).get("variant"))
    ? new URLSearchParams(location.search).get("variant")
    : "A",
  metric: "tokens",
  activeProvider: "openai",
  solid: false,
  range: "7D",
  refreshedAt: "방금 전"
};

const labels = {
  A: "한눈에 보기",
  B: "서비스 비교",
  C: "모바일 알림"
};

const metricLabels = { tokens: "토큰", cost: "비용", requests: "요청" };

const iconPaths = {
  grid: '<rect x="3" y="3" width="7" height="7" rx="2"/><rect x="14" y="3" width="7" height="7" rx="2"/><rect x="3" y="14" width="7" height="7" rx="2"/><rect x="14" y="14" width="7" height="7" rx="2"/>',
  pulse: '<path d="M3 12h4l2.2-6 4.1 12 2.2-6H21"/>',
  spark: '<path d="m4 17 5-5 3 3 7-8"/><path d="M15 7h4v4"/>',
  bell: '<path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9"/><path d="M10 21h4"/>',
  settings: '<path d="M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7Z"/><path d="M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06A1.7 1.7 0 0 0 15 19.4a1.7 1.7 0 0 0-1 .6 1.7 1.7 0 0 0-.4 1.1V21h-4v-.09A1.7 1.7 0 0 0 8.6 19.4a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.6 15a1.7 1.7 0 0 0-.6-1 1.7 1.7 0 0 0-1.1-.4H3v-4h.09A1.7 1.7 0 0 0 4.6 8.6a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.6a1.7 1.7 0 0 0 1-.6 1.7 1.7 0 0 0 .4-1.1V3h4v.09A1.7 1.7 0 0 0 15.4 4.6a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.4 9c.14.37.35.7.6 1 .3.3.7.48 1.1.4H21v4h-.09a1.7 1.7 0 0 0-1.51.6Z"/>',
  refresh: '<path d="M20 11a8 8 0 1 0-2.34 5.66"/><path d="M20 4v7h-7"/>',
  chevron: '<path d="m9 18 6-6-6-6"/>',
  shield: '<path d="M12 22s8-3.8 8-10V5l-8-3-8 3v7c0 6.2 8 10 8 10Z"/><path d="m9 12 2 2 4-4"/>',
  command: '<rect x="4" y="4" width="16" height="16" rx="5"/><path d="M9 8v8M15 8v8M8 9h8M8 15h8"/>',
  arrowLeft: '<path d="m15 18-6-6 6-6"/>',
  arrowRight: '<path d="m9 18 6-6-6-6"/>',
  phone: '<rect x="7" y="2" width="10" height="20" rx="3"/><path d="M11 18h2"/>',
  monitor: '<rect x="3" y="4" width="18" height="13" rx="3"/><path d="M8 21h8M12 17v4"/>',
  eye: '<path d="M2 12s3.5-6 10-6 10 6 10 6-3.5 6-10 6S2 12 2 12Z"/><circle cx="12" cy="12" r="2.5"/>'
};

function icon(name, size = 20) {
  return `<svg class="icon" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${iconPaths[name] || iconPaths.spark}</svg>`;
}

function providerLogo(provider, size = "md") {
  return `<span class="provider-logo ${size}" style="--provider:${provider.color}">${provider.short}</span>`;
}

function sparkline(values, color, width = 160, height = 54) {
  const min = Math.min(...values) - 5;
  const max = Math.max(...values) + 5;
  const points = values.map((value, index) => {
    const x = (index / (values.length - 1)) * width;
    const y = height - ((value - min) / (max - min)) * height;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const fillPoints = `0,${height} ${points} ${width},${height}`;
  return `<svg class="sparkline" viewBox="0 0 ${width} ${height}" role="img" aria-label="7일 사용 추세">
    <defs><linearGradient id="g-${color.replace("#", "")}" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="${color}" stop-opacity=".36"/><stop offset="1" stop-color="${color}" stop-opacity="0"/></linearGradient></defs>
    <polygon points="${fillPoints}" fill="url(#g-${color.replace("#", "")})"/><polyline points="${points}" fill="none" stroke="${color}" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"/>
    <circle cx="${width}" cy="${points.split(" ").at(-1).split(",")[1]}" r="3.5" fill="${color}"/>
  </svg>`;
}

function brand() {
  return `<div class="brand-lockup"><span class="brand-mark" aria-hidden="true"><i></i><i></i><i></i></span><div><strong>SPECTRA</strong><small>AI 사용 현황</small></div></div>`;
}

function metricTabs() {
  return `<div class="segmented" role="tablist" aria-label="사용량 단위">
    ${Object.entries(metricLabels).map(([value, label]) => `<button class="${state.metric === value ? "active" : ""}" data-metric="${value}" role="tab" aria-selected="${state.metric === value}">${label}</button>`).join("")}
  </div>`;
}

function topActions() {
  return `<div class="top-actions">
    <span class="sync-state"><i></i> 동기화 · ${state.refreshedAt}</span>
    <button class="icon-button" data-action="refresh" aria-label="데이터 새로고침">${icon("refresh", 17)}</button>
    <button class="icon-button" data-action="solid" aria-label="투명도 전환" title="가독성용 불투명 모드">${icon("eye", 17)}</button>
    <button class="avatar" aria-label="프로필">HJ</button>
  </div>`;
}

function navRail() {
  return `<aside class="nav-rail" aria-label="주요 메뉴">
    <div class="rail-mark"><span class="brand-mark compact"><i></i><i></i><i></i></span></div>
    <nav>
      <button class="active" aria-label="개요">${icon("grid")}</button>
      <button aria-label="사용 추이">${icon("pulse")}</button>
      <button aria-label="예상 사용량">${icon("spark")}</button>
      <button aria-label="알림">${icon("bell")}<span class="notification-dot"></span></button>
    </nav>
    <button aria-label="설정">${icon("settings")}</button>
  </aside>`;
}

function chartBars() {
  const bars = [43, 58, 49, 70, 64, 83, 74, 91, 66, 79, 88, 72, 96, 82];
  return `<div class="bar-chart" aria-label="시간별 토큰 사용량">${bars.map((h, index) => `<i style="--h:${h}%;--delay:${index * 35}ms" class="${index === 12 ? "peak" : ""}"></i>`).join("")}</div>`;
}

function providerRows(compact = false) {
  return providers.map(provider => `<button class="provider-row ${state.activeProvider === provider.id ? "active" : ""}" data-provider="${provider.id}" style="--provider:${provider.color}">
    ${providerLogo(provider, compact ? "sm" : "md")}
    <span class="provider-copy"><strong>${provider.name}</strong><small>${provider.tokens} 토큰</small></span>
    <span class="provider-meter"><i style="width:${provider.used}%"></i></span>
    <span class="provider-value">${provider.used}%</span>
  </button>`).join("");
}

function renderVariantA() {
  const active = providers.find(p => p.id === state.activeProvider);
  return `<div class="prototype-shell variant-a">
    ${navRail()}
    <section class="app-surface command-center">
      <header class="app-header">
        <div><span class="eyebrow">개요 · 오늘</span><h1>오늘 쓸 수 있는 양을<br><em>한눈에 봅니다.</em></h1></div>
        ${topActions()}
      </header>
      <div class="dashboard-toolbar">${metricTabs()}<div class="range-tabs">${[["24H", "오늘"], ["7D", "7일"], ["30D", "30일"]].map(([value, label]) => `<button class="${state.range === value ? "active" : ""}" data-range="${value}">${label}</button>`).join("")}</div></div>
      <div class="bento-grid">
        <article class="glass-card runway-card span-2">
          <div class="card-heading"><div><span class="eyebrow">전체 사용 여유</span><h2>82<span>%</span></h2></div><span class="trend-badge positive">지난주보다 6.4% 여유</span></div>
          <div class="runway-content">
            <div class="runway-ring" style="--value:82"><div><strong>18시간</strong><span>예상</span></div></div>
            <div class="runway-copy"><p>지금 속도라면 오늘 작업을 마친 뒤에도 약 <strong>18시간</strong>분이 남습니다.</p><div class="rainbow-track"><i></i></div><div class="track-labels"><span>서비스 6개</span><span>다음 초기화 · ${active.reset}</span></div></div>
          </div>
        </article>
        <article class="glass-card live-card span-2">
          <div class="card-heading"><div><span class="eyebrow">실시간 사용</span><h3>8.42M <small>토큰</small></h3></div><span class="live-pill"><i></i>집계 중</span></div>
          ${chartBars()}
          <div class="chart-axis"><span>09:00</span><span>12:00</span><span>15:00</span><span>지금</span></div>
        </article>
        <article class="glass-card providers-card span-2">
          <div class="card-heading"><div><span class="eyebrow">서비스</span><h3>서비스별 현황</h3></div><button class="text-button">모두 보기 ${icon("chevron", 14)}</button></div>
          <div class="provider-list">${providerRows(true)}</div>
        </article>
        <article class="glass-card focus-card">
          <div class="card-heading"><div>${providerLogo(active, "lg")}<span class="eyebrow">집중 확인</span></div><span class="trend-badge">초기화까지 ${active.reset}</span></div>
          <h3>${active.name}</h3><p>7일 평균보다 <strong>12% 빠른</strong> 속도로 사용 중입니다.</p>${sparkline(active.trend, active.color, 220, 62)}
        </article>
        <article class="glass-card budget-card">
          <span class="eyebrow">이번 달 예상</span><h3>$48.10 <small>/ $75</small></h3>
          <div class="budget-gauge"><i style="width:64%"></i></div><p><span>64% 사용</span><span class="positive">$9.80 절약</span></p>
          <div class="micro-stat"><span>예상 결제액</span><strong>$69.20</strong></div>
        </article>
        <article class="glass-card alert-card span-2">
          <div class="alert-icon">${icon("bell", 18)}</div><div><span class="eyebrow">확인이 필요한 항목</span><h3>Claude 5시간 한도의 76%를 사용했습니다.</h3><p>다음 35분은 OpenAI로 돌리면 초기화 전 병목을 줄일 수 있습니다.</p></div><button>OpenAI로 전환</button>
        </article>
      </div>
      <footer class="privacy-strip">${icon("shield", 15)} <span>사용량 정보는 이 기기에만 저장됩니다.</span><i></i><span>예시 데이터</span></footer>
    </section>
  </div>`;
}

function renderOrbitNodes(activeId) {
  return providers.map((provider, index) => {
    const angle = index * (360 / providers.length) - 90;
    return `<button class="orbit-node ${activeId === provider.id ? "active" : ""}" data-provider="${provider.id}" style="--angle:${angle}deg;--provider:${provider.color};--used:${provider.used}">
      ${providerLogo(provider, "sm")}<span>${provider.used}%</span>
    </button>`;
  }).join("");
}

function renderVariantB() {
  const active = providers.find(p => p.id === state.activeProvider);
  return `<div class="prototype-shell variant-b">
    <section class="orbit-app">
      <header class="orbit-header">${brand()}<div class="orbit-nav"><button class="active">지금</button><button>기록</button><button>한도</button></div>${topActions()}</header>
      <div class="orbit-layout">
        <aside class="orbit-summary">
          <span class="eyebrow">전체 현황</span><h1>아직<br><em>여유 있습니다.</em></h1>
          <p>여섯 서비스를 한 화면에서 비교합니다.</p>
          <div class="summary-number"><strong>82%</strong><span>전체 사용 여유</span></div>
          <div class="summary-stats"><div><span>시간당 사용</span><strong>482K</strong></div><div><span>다음 초기화</span><strong>${active.reset}</strong></div></div>
        </aside>
        <section class="orbit-stage" aria-label="서비스별 사용량 비교">
          <div class="orbit-grid" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
          <div class="orbit-ring ring-outer"></div><div class="orbit-ring ring-inner"></div>
          <div class="orbit-core" style="--provider:${active.color}">
            <span class="core-glow"></span>${providerLogo(active, "xl")}<small>선택한 서비스</small><strong>${active.name}</strong><div><b>${active.used}%</b> 사용</div>
          </div>
          ${renderOrbitNodes(active.id)}
        </section>
        <aside class="orbit-detail">
          <div class="detail-top"><span class="eyebrow">${active.name} 사용 내역</span><span class="status-dot">안정적</span></div>
          <h2>${active.tokens}</h2><p>오늘 사용한 토큰</p>${sparkline(active.trend, active.color, 240, 80)}
          <div class="detail-metrics"><div><span>비용</span><strong>${active.cost}</strong></div><div><span>초기화</span><strong>${active.reset}</strong></div><div><span>효율</span><strong class="positive">+12%</strong></div></div>
          <button class="primary-action">서비스 자세히 보기 ${icon("chevron", 16)}</button>
        </aside>
      </div>
      <footer class="signal-timeline">
        <div><span class="eyebrow">앞으로 12시간</span><strong>초기화 일정</strong></div>
        <div class="timeline-track"><i class="now" style="left:12%"><span>지금</span></i><i style="left:29%;--provider:#D88463"><span>Claude</span></i><i style="left:52%;--provider:#56B7B0"><span>OpenAI</span></i><i style="left:82%;--provider:#8879C6"><span>Gemini</span></i></div>
        <button class="timeline-settings">${icon("settings", 18)}</button>
      </footer>
    </section>
  </div>`;
}

function providerChips() {
  return providers.map(provider => `<button class="provider-chip ${state.activeProvider === provider.id ? "active" : ""}" data-provider="${provider.id}" style="--provider:${provider.color}">${providerLogo(provider, "sm")}<span><strong>${provider.name}</strong><small>${provider.used}% 사용</small></span></button>`).join("");
}

function renderVariantC() {
  const active = providers.find(p => p.id === state.activeProvider);
  return `<div class="prototype-shell variant-c">
    <section class="stream-app">
      <header class="stream-header">${brand()}${topActions()}</header>
      <div class="stream-layout">
        <section class="mobile-stream">
          <div class="mobile-top"><span>9:41</span><div><i></i><i></i><i></i></div></div>
          <div class="mobile-title"><div><span class="eyebrow">8월 15일</span><h1>사용 현황</h1></div><button class="icon-button" aria-label="알림">${icon("bell", 18)}<span class="notification-dot"></span></button></div>
          <article class="hero-signal" style="--provider:${active.color}">
            <div class="hero-signal-top"><span class="signal-orb">${providerLogo(active, "md")}</span><span class="trend-badge positive">여유 있음</span></div>
            <span class="eyebrow">전체 사용 여유</span><h2>82<span>%</span></h2><p>오늘 작업을 마치고도 여유가 있습니다.</p>
            <div class="spectrum-line"><i></i></div><div class="signal-foot"><span>8.42M 토큰</span><span>약 18시간</span></div>
          </article>
          <div class="chip-scroll" role="list">${providerChips()}</div>
          <section class="stream-feed">
            <div class="section-title"><div><span class="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><button>전체</button></div>
            <article class="feed-item priority"><span class="feed-line" style="--provider:#D88463"></span>${providerLogo(providers[1], "sm")}<div><span class="feed-time">지금 · CLAUDE</span><h4>5시간 한도에 가까워졌습니다.</h4><p>24% 남았습니다. 다음 큰 작업은 OpenAI로 옮겨 주세요.</p></div><strong>76%</strong></article>
            <article class="feed-item"><span class="feed-line" style="--provider:#8CB66B"></span>${providerLogo(providers[3], "sm")}<div><span class="feed-time">14분 전 · CURSOR</span><h4>사용 효율이 좋아졌습니다.</h4><p>캐시된 대화 맥락 덕분에 토큰 사용이 18% 줄었습니다.</p></div><strong class="positive">+18%</strong></article>
            <article class="feed-item"><span class="feed-line" style="--provider:#8879C6"></span>${providerLogo(providers[2], "sm")}<div><span class="feed-time">42분 전 · GEMINI</span><h4>일일 한도가 초기화됐습니다.</h4><p>전체 컨텍스트 창을 다시 사용할 수 있습니다.</p></div><strong>100%</strong></article>
          </section>
          <nav class="mobile-nav"><button class="active">${icon("pulse", 19)}<span>현황</span></button><button>${icon("grid", 19)}<span>서비스</span></button><button>${icon("spark", 19)}<span>추이</span></button><button>${icon("settings", 19)}<span>설정</span></button></nav>
        </section>
        <aside class="widget-lab">
          <div class="widget-lab-heading"><span class="eyebrow">기기별 보기</span><h2>맥·윈도우·아이폰에서<br>같은 사용량을 봅니다.</h2><p>표시 정보는 같고, 화면 크기에 맞춰 밀도만 달라집니다.</p></div>
          <div class="device-preview desktop-widget">
            <div class="device-label">${icon("monitor", 15)} 맥·윈도우 미니 위젯</div>
            <div class="widget-window"><div class="window-bar"><i></i><i></i><i></i><span>SPECTRA</span></div><div class="compact-content"><div><span>사용 여유</span><strong>82%</strong></div><div class="mini-spectrum"><i></i></div><div class="mini-providers">${providers.slice(0, 4).map(p => providerLogo(p, "xs")).join("")}<span>+2</span></div></div></div>
          </div>
          <div class="device-preview ios-widget">
            <div class="device-label">${icon("phone", 15)} iOS 중형 위젯</div>
            <div class="ios-card"><div><span class="brand-mini">SPECTRA</span><strong>82%</strong><small>약 18시간</small></div><div class="ios-bars">${providers.slice(0, 5).map(p => `<i style="height:${p.used}%;--provider:${p.color}"></i>`).join("")}</div></div>
          </div>
          <div class="platform-note">${icon("shield", 16)}<div><strong>기본은 기기 안에서만</strong><span>로컬 연결 · 암호화 동기화는 선택 사항</span></div></div>
        </aside>
      </div>
    </section>
  </div>`;
}

function prototypeSwitcher() {
  return `<div class="prototype-switcher" role="navigation" aria-label="프로토타입 변형 선택">
    <button data-cycle="-1" aria-label="이전 변형">${icon("arrowLeft", 18)}</button>
    <div><span>시안 전환</span><strong>${state.variant} — ${labels[state.variant]}</strong><small>${metricLabels[state.metric]} · ${providers.find(provider => provider.id === state.activeProvider).name} · ${state.solid ? "불투명" : "유리"}</small></div>
    <button data-cycle="1" aria-label="다음 변형">${icon("arrowRight", 18)}</button>
  </div>`;
}

function render() {
  document.body.classList.toggle("solid-mode", state.solid);
  const content = state.variant === "A" ? renderVariantA() : state.variant === "B" ? renderVariantB() : renderVariantC();
  document.querySelector("#app").innerHTML = `${content}${prototypeSwitcher()}`;
  bindEvents();
}

function setVariant(next) {
  state.variant = next;
  const url = new URL(location.href);
  url.searchParams.set("variant", next);
  history.replaceState({}, "", url);
  render();
}

function cycleVariant(direction) {
  const variants = ["A", "B", "C"];
  const nextIndex = (variants.indexOf(state.variant) + direction + variants.length) % variants.length;
  setVariant(variants[nextIndex]);
}

function bindEvents() {
  document.querySelectorAll("[data-cycle]").forEach(button => button.addEventListener("click", () => cycleVariant(Number(button.dataset.cycle))));
  document.querySelectorAll("[data-provider]").forEach(button => button.addEventListener("click", () => { state.activeProvider = button.dataset.provider; render(); }));
  document.querySelectorAll("[data-metric]").forEach(button => button.addEventListener("click", () => { state.metric = button.dataset.metric; render(); }));
  document.querySelectorAll("[data-range]").forEach(button => button.addEventListener("click", () => { state.range = button.dataset.range; render(); }));
  document.querySelectorAll('[data-action="solid"]').forEach(button => button.addEventListener("click", () => { state.solid = !state.solid; render(); }));
  document.querySelectorAll('[data-action="refresh"]').forEach(button => button.addEventListener("click", () => {
    state.refreshedAt = "지금";
    button.classList.add("spinning");
    setTimeout(render, 420);
  }));
}

addEventListener("keydown", event => {
  if (["INPUT", "TEXTAREA"].includes(document.activeElement?.tagName) || document.activeElement?.isContentEditable) return;
  if (event.key === "ArrowLeft") cycleVariant(-1);
  if (event.key === "ArrowRight") cycleVariant(1);
});

addEventListener("popstate", () => {
  const requested = new URLSearchParams(location.search).get("variant");
  if (["A", "B", "C"].includes(requested)) { state.variant = requested; render(); }
});

render();
