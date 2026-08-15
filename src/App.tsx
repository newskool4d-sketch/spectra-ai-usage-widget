import { memo, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Icon, type IconName } from "./components/Icon";
import { Sparkline } from "./components/Sparkline";
import { metricLabels, planQuotas, providers, rangeLabels, usageBars, type AuthMethod, type Metric, type PlanQuota, type Provider, type ProviderId, type QuotaWindow, type UsageRange } from "./data/providers";
import { getOAuthAdapter, type OAuthStartResult } from "./integrations/oauth-adapter";
import { providerCapabilities } from "./integrations/provider-capabilities";
import { listenNativeOAuth } from "./integrations/tauri-native-bridge";

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

const isProviderId = (value: string): value is ProviderId => providers.some(provider => provider.id === value);

function connectionLabel(quota: PlanQuota) {
  if (quota.connectionState === "unsupported") return "OAuth 지원 확인 필요";
  if (quota.connectionState === "error") return "연결 상태 확인 필요";
  if (quota.connectionState === "connected") return "OAuth 흐름 완료 · 예시 snapshot";
  return "OAuth 연결 필요";
}

function sourceLabel(quota: PlanQuota) {
  if (quota.source === "oauth") return "OAuth 흐름 완료 · 예시 snapshot";
  if (quota.source === "unavailable") return "공식 잔여량 경로 확인 중";
  return "예시 snapshot · 실제 연결 후 갱신";
}

function authMethodLabel(method: AuthMethod) {
  if (method === "oauth-pkce") return "OAuth · PKCE 흐름";
  if (method === "provider-delegated") return "공급자 delegated auth";
  return "공식 OAuth 경로 미확인";
}

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
  return <div className="bar-chart" aria-label="시간별 한도 소진 추이">{usageBars.map((height, index) => <i key={`bar-${index}`} className={index === 12 ? "peak" : ""} style={{ "--h": `${height}%`, "--delay": `${index * 35}ms` } as CSSProperties} />)}</div>;
});

const QuotaWindowRow = memo(function QuotaWindowRow({ window, compact = false }: Readonly<{ window: QuotaWindow; compact?: boolean }>) {
  return <div className={`quota-window ${compact ? "compact" : ""}`}>
    <div className="quota-window-copy"><span>{window.label}</span><strong>{window.remainingPercent}% 남음</strong></div>
    <div className="quota-window-meta"><span>{window.kindLabel}</span><span>{window.resetLabel}</span></div>
    <div className="quota-window-meter" aria-label={`${window.label} ${window.usedPercent}% 사용`}><i style={{ width: `${window.usedPercent}%` }} /></div>
  </div>;
});

const ProviderRow = memo(function ProviderRow({ provider, active, onSelect }: Readonly<{ provider: Provider; active: boolean; onSelect: (id: ProviderId) => void }>) {
  const quota = planQuotas[provider.id];
  const primary = quota.windows[0];
  return <button type="button" className={`provider-row ${active ? "active" : ""}`} style={providerStyle(provider.color)} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" />
    <span className="provider-copy"><strong>{provider.name}</strong><small>{quota.planName}</small></span>
    <span className="provider-meter"><i style={{ width: `${primary.remainingPercent}%` }} /></span>
    <span className="provider-value">{primary.remainingPercent}%</span>
  </button>;
});

const ProviderChip = memo(function ProviderChip({ provider, active, onSelect }: Readonly<{ provider: Provider; active: boolean; onSelect: (id: ProviderId) => void }>) {
  const primary = planQuotas[provider.id].windows[0];
  return <button type="button" className={`provider-chip ${active ? "active" : ""}`} style={providerStyle(provider.color)} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" /><span><strong>{provider.name}</strong><small>{primary.remainingPercent}% 남음</small></span>
  </button>;
});

const QuotaSummaryCard = memo(function QuotaSummaryCard({ provider, quota }: Readonly<{ provider: Provider; quota: PlanQuota }>) {
  const [primary, secondary] = quota.windows;
  return <article className="glass-card runway-card quota-card span-2" style={providerStyle(provider.color)}>
    <div className="card-heading"><div><span className="eyebrow">요금제 잔여량</span><h2>{primary.remainingPercent}<span>%</span></h2></div><span className={`quota-status ${quota.connectionState}`}><i />{connectionLabel(quota)}</span></div>
    <div className="runway-content"><div className="runway-ring quota-ring" style={{ "--value": primary.remainingPercent, "--provider": provider.color } as CSSProperties}><div><strong>{primary.remainingPercent}%</strong><span>잔여</span></div></div><div className="runway-copy"><p><strong>{quota.planName}</strong> · {primary.label}의 {primary.remainingPercent}%가 남아 있습니다. <span className="quota-source">{sourceLabel(quota)}</span></p><div className="rainbow-track"><i style={{ width: `${primary.remainingPercent}%` }} /></div><div className="track-labels"><span>{primary.kindLabel}</span><span>초기화 · {primary.resetLabel}</span></div>{secondary ? <div className="quota-window-list"><QuotaWindowRow window={secondary} compact /></div> : null}</div></div>
  </article>;
});

const OAuthConnectCard = memo(function OAuthConnectCard({ provider, quota, compact = false, onOpen }: Readonly<{ provider: Provider; quota: PlanQuota; compact?: boolean; onOpen: (id: ProviderId) => void }>) {
  const unsupported = quota.connectionState === "unsupported";
  const connected = quota.connectionState === "connected";
  const capability = providerCapabilities[provider.id];
  return <article className={`glass-card oauth-card ${compact ? "compact" : ""}`} style={providerStyle(provider.color)}>
    <div className="oauth-icon"><Icon name={unsupported ? "shield" : connected ? "check" : "link"} size={18} /></div>
    <div className="oauth-copy"><span className="eyebrow">계정 연결</span><h3>{provider.name} 요금제 연결</h3><p>OAuth로 연결하면 요금제 잔여량과 초기화 시각을 제품 화면에서 확인합니다.</p><div className={`oauth-status ${unsupported ? "unsupported" : connected ? "connected" : ""}`}><i />{connectionLabel(quota)}</div><div className="oauth-capability"><span>공식 인증</span><strong>{capability.oauthLabel}</strong><span>잔여량 범위</span><strong>{capability.quotaLabel}</strong></div></div>
    <button type="button" className="oauth-button" disabled={unsupported} onClick={() => onOpen(provider.id)}>{unsupported ? "지원 확인 필요" : connected ? "연결 흐름 다시 보기" : "OAuth로 연결"}</button>
    <div className="oauth-security"><Icon name="shield" size={13} /><span>실제 토큰은 화면이나 브라우저 저장소에 표시하지 않습니다.</span></div>
  </article>;
});

const OAuthDialog = memo(function OAuthDialog({ open, provider, quota, startResult, onClose, onConnect, onDisconnect }: Readonly<{
  open: boolean;
  provider: Provider;
  quota: PlanQuota;
  startResult: OAuthStartResult | null;
  onClose: () => void;
  onConnect: (id: ProviderId) => void;
  onDisconnect: (id: ProviderId) => void;
}>) {
  if (!open) return null;
  const unsupported = quota.connectionState === "unsupported";
  const connected = quota.connectionState === "connected";
  const capability = providerCapabilities[provider.id];
  const nativeReady = startResult?.status === "native-ready";
  const nativeFailed = startResult?.status === "not-available";
  return <div className="oauth-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}><div className="oauth-dialog" role="dialog" aria-modal="true" aria-labelledby="oauth-dialog-title">
    <div className="oauth-dialog-header"><div><span className="eyebrow">안전한 계정 연결</span><h2 id="oauth-dialog-title">{provider.name} 요금제 확인</h2></div><button type="button" className="icon-button oauth-close" onClick={onClose} aria-label="연결 창 닫기"><Icon name="x" size={17} /></button></div>
    <div className="oauth-provider"><ProviderLogo provider={provider} size="lg" /><div><strong>{quota.planName}</strong><span>{authMethodLabel(quota.authMethod)}</span></div></div>
    <div className="oauth-capability-note"><span>공식 인증 경로</span><strong>{capability.oauthLabel}</strong><span>요금제 잔여량</span><strong>{capability.quotaLabel}</strong></div>
    <ol className="oauth-steps"><li><span>01</span><div><strong>브라우저에서 로그인</strong><small>공급자 로그인 화면에서 계정을 직접 확인합니다.</small></div></li><li><span>02</span><div><strong>잔여량 권한 확인</strong><small>계정·요금제 정보만 읽는 범위를 먼저 보여줍니다.</small></div></li><li><span>03</span><div><strong>기기에 안전하게 보관</strong><small>토큰은 UI·브라우저 저장소에 노출하지 않는 구조로 연결합니다.</small></div></li></ol>
    <div className="oauth-demo-note"><Icon name="shield" size={14} /><span>{startResult?.message ?? "현재는 실제 공급자 호출 전 데모 흐름입니다. 완료 후에도 예시 snapshot으로 표시됩니다."}</span></div>
    <div className="oauth-dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>취소</button>{connected ? <button type="button" className="danger-action" onClick={() => { onDisconnect(provider.id); onClose(); }}>연결 해제</button> : <button type="button" className="primary-action" disabled={unsupported || nativeReady || nativeFailed} onClick={() => { onConnect(provider.id); onClose(); }}>{unsupported ? "공식 경로 확인 필요" : nativeReady ? "공급자 설정 대기" : nativeFailed ? "네이티브 준비 실패" : "데모 연결 완료"}</button>}</div>
  </div></div>;
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
  activeQuota: PlanQuota;
  metric: Metric;
  range: UsageRange;
  onProvider: (id: ProviderId) => void;
  onMetric: (metric: Metric) => void;
  onRange: (range: UsageRange) => void;
  onOpenOAuth: (id: ProviderId) => void;
}>;

const VariantADesktop = memo(function VariantADesktop({ activeProvider, activeProviderId, activeQuota, metric, range, onProvider, onMetric, onRange, onOpenOAuth, ...actions }: SharedViewProps) {
  const primary = activeQuota.windows[0];
  return <div className="product-shell variant-a">
    <NavRail />
    <section className="app-surface command-center">
      <header className="app-header">
        <div><span className="eyebrow">개요 · 오늘</span><h1>오늘 쓸 수 있는 양을<br /><em>한눈에 봅니다.</em></h1></div>
        <TopActions {...actions} />
      </header>
      <div className="dashboard-toolbar"><MetricTabs metric={metric} onMetric={onMetric} /><RangeTabs range={range} onRange={onRange} /></div>
      <div className="bento-grid">
        <QuotaSummaryCard provider={activeProvider} quota={activeQuota} />
        <article className="glass-card live-card span-2"><div className="card-heading"><div><span className="eyebrow">한도 소진 추이</span><h3>{primary.usedPercent}% <small>현재 사용</small></h3></div><span className="live-pill"><i />집계 중</span></div><ChartBars /><div className="chart-axis"><span>09:00</span><span>12:00</span><span>15:00</span><span>지금</span></div></article>
        <article className="glass-card providers-card span-2"><div className="card-heading"><div><span className="eyebrow">서비스</span><h3>서비스별 잔여량</h3></div><button type="button" className="text-button">모두 보기 <Icon name="chevron" size={14} /></button></div><div className="provider-list">{providers.map(provider => <ProviderRow key={provider.id} provider={provider} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div></article>
        <article className="glass-card focus-card"><div className="card-heading"><div><ProviderLogo provider={activeProvider} size="lg" /><span className="eyebrow">집중 확인</span></div><span className="trend-badge">{primary.remainingPercent}% 남음</span></div><h3>{activeProvider.name}</h3><p>{primary.label}이 {primary.resetLabel} 뒤 초기화됩니다. <strong>{activeQuota.planName}</strong> 기준 예시 snapshot입니다.</p><Sparkline values={activeProvider.trend} color={activeProvider.color} width={220} height={62} /></article>
        <article className="glass-card budget-card plan-card"><span className="eyebrow">연결된 요금제</span><h3>{activeQuota.planName}</h3><p><span>{activeQuota.accountLabel}</span><span className={activeQuota.connectionState === "connected" ? "positive" : ""}>{activeQuota.lastSyncedAt ? `마지막 확인 · ${activeQuota.lastSyncedAt}` : "연결 전 · 예시 snapshot"}</span></p><div className="micro-stat"><span>인증 방식</span><strong>{authMethodLabel(activeQuota.authMethod)}</strong></div></article>
        <OAuthConnectCard provider={activeProvider} quota={activeQuota} onOpen={onOpenOAuth} />
      </div>
      <footer className="privacy-strip"><Icon name="shield" size={15} /><span>잔여량은 OAuth 연결 후 공급자 정책에 맞춰 갱신됩니다.</span><i /><span>{sourceLabel(activeQuota)}</span></footer>
    </section>
  </div>;
});

const MobileNav = memo(function MobileNav() {
  const items: readonly Readonly<{ name: IconName; label: string }>[] = [{ name: "pulse", label: "현황" }, { name: "grid", label: "서비스" }, { name: "spark", label: "추이" }, { name: "settings", label: "설정" }];
  return <nav className="mobile-nav">{items.map((item, index) => <button type="button" className={index === 0 ? "active" : ""} key={item.name}><Icon name={item.name} size={19} /><span>{item.label}</span></button>)}</nav>;
});

const VariantCMobile = memo(function VariantCMobile({ activeProvider, activeProviderId, activeQuota, onProvider, onOpenOAuth }: SharedViewProps) {
  const claude = providers.find(provider => provider.id === "claude") ?? providers[1];
  const cursor = providers.find(provider => provider.id === "cursor") ?? providers[3];
  const gemini = providers.find(provider => provider.id === "gemini") ?? providers[2];
  const claudeQuota = planQuotas.claude;
  const cursorQuota = planQuotas.cursor;
  const geminiQuota = planQuotas.gemini;
  const primary = activeQuota.windows[0];
  return <div className="product-shell variant-c"><section className="stream-app"><div className="stream-layout"><section className="mobile-stream">
    <div className="mobile-top"><span>9:41</span><div><i /><i /><i /></div></div>
    <div className="mobile-title"><div><span className="eyebrow">8월 15일</span><h1>사용 현황</h1></div><button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
    <article className="hero-signal" style={providerStyle(activeProvider.color)}><div className="hero-signal-top"><span className="signal-orb"><ProviderLogo provider={activeProvider} size="md" /></span><span className={`trend-badge ${activeQuota.connectionState === "connected" ? "positive" : ""}`}>{activeQuota.connectionState === "connected" ? "연결됨" : "예시 snapshot"}</span></div><span className="eyebrow">{activeQuota.planName} · {primary.label}</span><h2>{primary.remainingPercent}<span>%</span></h2><p>초기화 전까지 남은 요금제 여유입니다.</p><div className="spectrum-line"><i style={{ width: `${primary.remainingPercent}%` }} /></div><div className="signal-foot"><span>남은 한도 {primary.remainingPercent}%</span><span>초기화 {primary.resetLabel}</span></div></article>
    <div className="chip-scroll" role="list">{providers.map(provider => <ProviderChip key={provider.id} provider={provider} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div>
    <OAuthConnectCard provider={activeProvider} quota={activeQuota} compact onOpen={onOpenOAuth} />
    <section className="stream-feed"><div className="section-title"><div><span className="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><button type="button">전체</button></div>
      <article className="feed-item priority"><span className="feed-line" style={providerStyle(claude.color)} /><ProviderLogo provider={claude} size="sm" /><div><span className="feed-time">지금 · CLAUDE</span><h4>5시간 한도에 가까워졌습니다.</h4><p>{claudeQuota.windows[0].remainingPercent}% 남았습니다. 다음 큰 작업은 다른 연결 서비스로 옮겨 주세요.</p></div><strong>{claudeQuota.windows[0].usedPercent}%</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(cursor.color)} /><ProviderLogo provider={cursor} size="sm" /><div><span className="feed-time">14분 전 · CURSOR</span><h4>공식 잔여량 연결 경로를 확인 중입니다.</h4><p>{cursorQuota.windows[0].remainingPercent}% 남음 · OAuth 지원 여부를 먼저 확인해야 합니다.</p></div><strong>{cursorQuota.windows[0].remainingPercent}%</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(gemini.color)} /><ProviderLogo provider={gemini} size="sm" /><div><span className="feed-time">42분 전 · GEMINI</span><h4>일일 한도 초기화 시각을 확인했습니다.</h4><p>다음 초기화까지 {geminiQuota.windows[0].resetLabel} · 예시 snapshot</p></div><strong className="positive">{geminiQuota.windows[0].remainingPercent}%</strong></article>
    </section>
    <MobileNav />
  </section></div></section></div>;
});

export function App() {
  const isMobile = useIsMobile();
  const [activeProviderId, setActiveProviderId] = useState<ProviderId>("openai");
  const [metric, setMetric] = useState<Metric>("remaining");
  const [range, setRange] = useState<UsageRange>("7D");
  const [solid, setSolid] = useState(false);
  const [refreshedAt, setRefreshedAt] = useState("방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const [connectedProviderIds, setConnectedProviderIds] = useState<readonly ProviderId[]>([]);
  const [oauthProviderId, setOauthProviderId] = useState<ProviderId>("openai");
  const [oauthOpen, setOauthOpen] = useState(false);
  const [oauthStartResult, setOauthStartResult] = useState<OAuthStartResult | null>(null);
  const refreshTimer = useRef<number | null>(null);
  const activeProvider = useMemo(() => providers.find(provider => provider.id === activeProviderId) ?? providers[0], [activeProviderId]);
  const activeQuota = useMemo(() => {
    const baseQuota = planQuotas[activeProviderId];
    if (baseQuota.connectionState === "unsupported" || !connectedProviderIds.includes(activeProviderId)) return baseQuota;
    return { ...baseQuota, accountLabel: "예시 계정 · OAuth 흐름 완료", connectionState: "connected" as const, source: "oauth" as const, confidence: "example" as const, lastSyncedAt: "방금 전" };
  }, [activeProviderId, connectedProviderIds]);
  const oauthProvider = useMemo(() => providers.find(provider => provider.id === oauthProviderId) ?? providers[0], [oauthProviderId]);
  const oauthQuota = useMemo(() => {
    const baseQuota = planQuotas[oauthProviderId];
    if (baseQuota.connectionState === "unsupported" || !connectedProviderIds.includes(oauthProviderId)) return baseQuota;
    return { ...baseQuota, accountLabel: "예시 계정 · OAuth 흐름 완료", connectionState: "connected" as const, source: "oauth" as const, confidence: "example" as const, lastSyncedAt: "방금 전" };
  }, [oauthProviderId, connectedProviderIds]);

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

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void | Promise<void>) | null = null;
    void listenNativeOAuth(event => {
      if (disposed || !isProviderId(event.providerId)) return;
      setOauthProviderId(event.providerId);
      setOauthOpen(true);
      setOauthStartResult({
        status: "native-ready",
        message: "네이티브 callback이 state 검증을 통과했습니다. 토큰 교환과 OS 보관은 공급자 adapter 설정 후 진행됩니다."
      });
    }).then(listener => {
      if (disposed) {
        void listener?.();
      } else {
        unlisten = listener;
      }
    });
    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, []);

  const openOAuth = useCallback((id: ProviderId) => {
    setOauthProviderId(id);
    setOauthStartResult(null);
    setOauthOpen(true);
    void getOAuthAdapter(id).start().then(setOauthStartResult);
  }, []);
  const connectOAuth = useCallback((id: ProviderId) => setConnectedProviderIds(current => current.includes(id) ? current : [...current, id]), []);
  const disconnectOAuth = useCallback((id: ProviderId) => {
    setConnectedProviderIds(current => current.filter(providerId => providerId !== id));
    void getOAuthAdapter(id).disconnect().catch(() => undefined);
  }, []);

  const sharedProps: SharedViewProps = {
    activeProvider,
    activeProviderId,
    activeQuota,
    metric,
    range,
    onProvider: setActiveProviderId,
    onMetric: setMetric,
    onRange: setRange,
    onOpenOAuth: openOAuth,
    refreshedAt,
    refreshing,
    solid,
    onRefresh: refresh,
    onSolid: () => setSolid(value => !value)
  };

  return <><div className={solid ? "solid-mode" : ""}>{isMobile ? <VariantCMobile {...sharedProps} /> : <VariantADesktop {...sharedProps} />}</div><OAuthDialog open={oauthOpen} provider={oauthProvider} quota={oauthQuota} startResult={oauthStartResult} onClose={() => { setOauthOpen(false); setOauthStartResult(null); }} onConnect={connectOAuth} onDisconnect={disconnectOAuth} /></>;
}
