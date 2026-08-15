import { memo, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Icon, type IconName } from "./components/Icon";
import { Sparkline } from "./components/Sparkline";
import { metricLabels, providers, rangeLabels, usageBars, type Metric, type Provider, type ProviderId, type UsageRange } from "./data/providers";

const mobileBreakpoint = "(max-width: 820px)";

function useIsMobile() {
  const [isMobile, setIsMobile] = useState(() => typeof window !== "undefined" && window.matchMedia(mobileBreakpoint).matches);

  useEffect(() => {
    const media = window.matchMedia(mobileBreakpoint);
    const handleChange = (event: MediaQueryListEvent) => setIsMobile(event.matches);
    setIsMobile(media.matches);
    media.addEventListener("change", handleChange);
    return () => media.removeEventListener("change", handleChange);
  }, []);

  return isMobile;
}

const providerStyle = (color: string) => ({ "--provider": color } as CSSProperties);

const Brand = memo(function Brand() {
  return <div className="brand-lockup"><span className="brand-mark" aria-hidden="true"><i /><i /><i /></span><div><strong>SPECTRA</strong><small>AI 사용 현황</small></div></div>;
});

const ProviderLogo = memo(function ProviderLogo({ provider, size = "md" }: Readonly<{ provider: Provider; size?: "xs" | "sm" | "md" | "lg" | "xl" }>) {
  return <span className={`provider-logo ${size}`} style={providerStyle(provider.color)} aria-hidden="true">{provider.short}</span>;
});

const TopActions = memo(function TopActions({ refreshedAt, refreshing, solid, onRefresh, onSolid }: Readonly<{
  refreshedAt: string;
  refreshing: boolean;
  solid: boolean;
  onRefresh: () => void;
  onSolid: () => void;
}>) {
  return <div className="top-actions">
    <span className="sync-state"><i /> 동기화 · {refreshedAt}</span>
    <button type="button" className={`icon-button ${refreshing ? "spinning" : ""}`} onClick={onRefresh} aria-label="데이터 새로고침"><Icon name="refresh" size={17} /></button>
    <button type="button" className="icon-button" onClick={onSolid} aria-label="투명도 전환" title={solid ? "유리 모드" : "가독성용 불투명 모드"}><Icon name="eye" size={17} /></button>
    <button type="button" className="avatar" aria-label="프로필">HJ</button>
  </div>;
});

const NavRail = memo(function NavRail() {
  const items: readonly Readonly<{ name: IconName; label: string; notice?: boolean }>[] = [
    { name: "grid", label: "개요" },
    { name: "pulse", label: "사용 추이" },
    { name: "spark", label: "예상 사용량" },
    { name: "bell", label: "알림", notice: true }
  ];
  return <aside className="nav-rail" aria-label="주요 메뉴">
    <div className="rail-mark"><span className="brand-mark compact"><i /><i /><i /></span></div>
    <nav>{items.map((item, index) => <button type="button" className={index === 0 ? "active" : ""} aria-label={item.label} key={item.name}><Icon name={item.name} />{item.notice ? <span className="notification-dot" /> : null}</button>)}</nav>
    <button type="button" aria-label="설정"><Icon name="settings" /></button>
  </aside>;
});

const MetricTabs = memo(function MetricTabs({ metric, onMetric }: Readonly<{ metric: Metric; onMetric: (metric: Metric) => void }>) {
  return <div className="segmented" role="tablist" aria-label="사용량 단위">
    {(Object.keys(metricLabels) as Metric[]).map(value => <button type="button" className={metric === value ? "active" : ""} role="tab" aria-selected={metric === value} key={value} onClick={() => onMetric(value)}>{metricLabels[value]}</button>)}
  </div>;
});

const RangeTabs = memo(function RangeTabs({ range, onRange }: Readonly<{ range: UsageRange; onRange: (range: UsageRange) => void }>) {
  return <div className="range-tabs" role="tablist" aria-label="조회 기간">
    {(Object.keys(rangeLabels) as UsageRange[]).map(value => <button type="button" className={range === value ? "active" : ""} role="tab" aria-selected={range === value} key={value} onClick={() => onRange(value)}>{rangeLabels[value]}</button>)}
  </div>;
});

const ChartBars = memo(function ChartBars() {
  return <div className="bar-chart" aria-label="시간별 토큰 사용량">{usageBars.map((height, index) => <i key={`bar-${index}`} className={index === 12 ? "peak" : ""} style={{ "--h": `${height}%`, "--delay": `${index * 35}ms` } as CSSProperties} />)}</div>;
});

const ProviderRow = memo(function ProviderRow({ provider, active, onSelect }: Readonly<{ provider: Provider; active: boolean; onSelect: (id: ProviderId) => void }>) {
  return <button type="button" className={`provider-row ${active ? "active" : ""}`} style={providerStyle(provider.color)} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" />
    <span className="provider-copy"><strong>{provider.name}</strong><small>{provider.tokens} 토큰</small></span>
    <span className="provider-meter"><i style={{ width: `${provider.used}%` }} /></span>
    <span className="provider-value">{provider.used}%</span>
  </button>;
});

const ProviderChip = memo(function ProviderChip({ provider, active, onSelect }: Readonly<{ provider: Provider; active: boolean; onSelect: (id: ProviderId) => void }>) {
  return <button type="button" className={`provider-chip ${active ? "active" : ""}`} style={providerStyle(provider.color)} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" /><span><strong>{provider.name}</strong><small>{provider.used}% 사용</small></span>
  </button>;
});

type LayoutActions = Readonly<{
  refreshedAt: string;
  refreshing: boolean;
  solid: boolean;
  onRefresh: () => void;
  onSolid: () => void;
}>;

type SharedViewProps = LayoutActions & Readonly<{
  activeProvider: Provider;
  activeProviderId: ProviderId;
  metric: Metric;
  range: UsageRange;
  onProvider: (id: ProviderId) => void;
  onMetric: (metric: Metric) => void;
  onRange: (range: UsageRange) => void;
}>;

const VariantADesktop = memo(function VariantADesktop({ activeProvider, activeProviderId, metric, range, onProvider, onMetric, onRange, ...actions }: SharedViewProps) {
  return <div className="product-shell variant-a">
    <NavRail />
    <section className="app-surface command-center">
      <header className="app-header">
        <div><span className="eyebrow">개요 · 오늘</span><h1>오늘 쓸 수 있는 양을<br /><em>한눈에 봅니다.</em></h1></div>
        <TopActions {...actions} />
      </header>
      <div className="dashboard-toolbar"><MetricTabs metric={metric} onMetric={onMetric} /><RangeTabs range={range} onRange={onRange} /></div>
      <div className="bento-grid">
        <article className="glass-card runway-card span-2">
          <div className="card-heading"><div><span className="eyebrow">전체 사용 여유</span><h2>82<span>%</span></h2></div><span className="trend-badge positive">지난주보다 6.4% 여유</span></div>
          <div className="runway-content"><div className="runway-ring" style={{ "--value": 82 } as CSSProperties}><div><strong>18시간</strong><span>예상</span></div></div><div className="runway-copy"><p>지금 속도라면 오늘 작업을 마친 뒤에도 약 <strong>18시간</strong>분이 남습니다.</p><div className="rainbow-track"><i /></div><div className="track-labels"><span>서비스 6개</span><span>다음 초기화 · {activeProvider.reset}</span></div></div></div>
        </article>
        <article className="glass-card live-card span-2"><div className="card-heading"><div><span className="eyebrow">실시간 사용</span><h3>8.42M <small>{metricLabels[metric]}</small></h3></div><span className="live-pill"><i />집계 중</span></div><ChartBars /><div className="chart-axis"><span>09:00</span><span>12:00</span><span>15:00</span><span>지금</span></div></article>
        <article className="glass-card providers-card span-2"><div className="card-heading"><div><span className="eyebrow">서비스</span><h3>서비스별 현황</h3></div><button type="button" className="text-button">모두 보기 <Icon name="chevron" size={14} /></button></div><div className="provider-list">{providers.map(provider => <ProviderRow key={provider.id} provider={provider} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div></article>
        <article className="glass-card focus-card"><div className="card-heading"><div><ProviderLogo provider={activeProvider} size="lg" /><span className="eyebrow">집중 확인</span></div><span className="trend-badge">초기화까지 {activeProvider.reset}</span></div><h3>{activeProvider.name}</h3><p>7일 평균보다 <strong>12% 빠른</strong> 속도로 사용 중입니다.</p><Sparkline values={activeProvider.trend} color={activeProvider.color} width={220} height={62} /></article>
        <article className="glass-card budget-card"><span className="eyebrow">이번 달 예상</span><h3>$48.10 <small>/ $75</small></h3><div className="budget-gauge"><i style={{ width: "64%" }} /></div><p><span>64% 사용</span><span className="positive">$9.80 절약</span></p><div className="micro-stat"><span>예상 결제액</span><strong>$69.20</strong></div></article>
        <article className="glass-card alert-card span-2"><div className="alert-icon"><Icon name="bell" size={18} /></div><div><span className="eyebrow">확인이 필요한 항목</span><h3>Claude 5시간 한도의 76%를 사용했습니다.</h3><p>다음 35분은 OpenAI로 돌리면 초기화 전 병목을 줄일 수 있습니다.</p></div><button type="button" onClick={() => onProvider("openai")}>OpenAI로 전환</button></article>
      </div>
      <footer className="privacy-strip"><Icon name="shield" size={15} /><span>사용량 정보는 이 기기에만 저장됩니다.</span><i /><span>예시 데이터</span></footer>
    </section>
  </div>;
});

const MobileNav = memo(function MobileNav() {
  const items: readonly Readonly<{ name: IconName; label: string }>[] = [{ name: "pulse", label: "현황" }, { name: "grid", label: "서비스" }, { name: "spark", label: "추이" }, { name: "settings", label: "설정" }];
  return <nav className="mobile-nav">{items.map((item, index) => <button type="button" className={index === 0 ? "active" : ""} key={item.name}><Icon name={item.name} size={19} /><span>{item.label}</span></button>)}</nav>;
});

const VariantCMobile = memo(function VariantCMobile({ activeProvider, activeProviderId, onProvider, ...actions }: SharedViewProps) {
  const claude = providers.find(provider => provider.id === "claude") ?? providers[1];
  const cursor = providers.find(provider => provider.id === "cursor") ?? providers[3];
  const gemini = providers.find(provider => provider.id === "gemini") ?? providers[2];
  return <div className="product-shell variant-c"><section className="stream-app"><div className="stream-layout"><section className="mobile-stream">
    <div className="mobile-top"><span>9:41</span><div><i /><i /><i /></div></div>
    <div className="mobile-title"><div><span className="eyebrow">8월 15일</span><h1>사용 현황</h1></div><button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
    <article className="hero-signal" style={providerStyle(activeProvider.color)}><div className="hero-signal-top"><span className="signal-orb"><ProviderLogo provider={activeProvider} size="md" /></span><span className="trend-badge positive">여유 있음</span></div><span className="eyebrow">전체 사용 여유</span><h2>82<span>%</span></h2><p>오늘 작업을 마치고도 여유가 있습니다.</p><div className="spectrum-line"><i /></div><div className="signal-foot"><span>8.42M 토큰</span><span>약 18시간</span></div></article>
    <div className="chip-scroll" role="list">{providers.map(provider => <ProviderChip key={provider.id} provider={provider} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div>
    <section className="stream-feed"><div className="section-title"><div><span className="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><button type="button">전체</button></div>
      <article className="feed-item priority"><span className="feed-line" style={providerStyle(claude.color)} /><ProviderLogo provider={claude} size="sm" /><div><span className="feed-time">지금 · CLAUDE</span><h4>5시간 한도에 가까워졌습니다.</h4><p>24% 남았습니다. 다음 큰 작업은 OpenAI로 옮겨 주세요.</p></div><strong>76%</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(cursor.color)} /><ProviderLogo provider={cursor} size="sm" /><div><span className="feed-time">14분 전 · CURSOR</span><h4>사용 효율이 좋아졌습니다.</h4><p>캐시된 대화 맥락 덕분에 토큰 사용이 18% 줄었습니다.</p></div><strong className="positive">+18%</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(gemini.color)} /><ProviderLogo provider={gemini} size="sm" /><div><span className="feed-time">42분 전 · GEMINI</span><h4>일일 한도가 초기화됐습니다.</h4><p>전체 컨텍스트 창을 다시 사용할 수 있습니다.</p></div><strong>100%</strong></article>
    </section>
    <MobileNav />
  </section></div></section></div>;
});

export function App() {
  const isMobile = useIsMobile();
  const [activeProviderId, setActiveProviderId] = useState<ProviderId>("openai");
  const [metric, setMetric] = useState<Metric>("tokens");
  const [range, setRange] = useState<UsageRange>("7D");
  const [solid, setSolid] = useState(false);
  const [refreshedAt, setRefreshedAt] = useState("방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const refreshTimer = useRef<number | null>(null);
  const activeProvider = useMemo(() => providers.find(provider => provider.id === activeProviderId) ?? providers[0], [activeProviderId]);

  const refresh = useCallback(() => {
    if (refreshTimer.current !== null) window.clearTimeout(refreshTimer.current);
    setRefreshing(true);
    refreshTimer.current = window.setTimeout(() => {
      setRefreshedAt("지금");
      setRefreshing(false);
      refreshTimer.current = null;
    }, 420);
  }, []);

  useEffect(() => () => {
    if (refreshTimer.current !== null) window.clearTimeout(refreshTimer.current);
  }, []);

  const sharedProps: SharedViewProps = {
    activeProvider,
    activeProviderId,
    metric,
    range,
    onProvider: setActiveProviderId,
    onMetric: setMetric,
    onRange: setRange,
    refreshedAt,
    refreshing,
    solid,
    onRefresh: refresh,
    onSolid: () => setSolid(value => !value)
  };

  return <div className={solid ? "solid-mode" : ""}>{isMobile ? <VariantCMobile {...sharedProps} /> : <VariantADesktop {...sharedProps} />}</div>;
}

