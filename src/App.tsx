import { memo, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Icon, type IconName } from "./components/Icon";
import { Sparkline } from "./components/Sparkline";
import { metricLabels, planQuotas, providers, rangeLabels, usageBars, type AuthMethod, type Metric, type PlanQuota, type Provider, type ProviderId, type QuotaWindow, type UsageRange } from "./data/providers";
import { getOAuthAdapter, type OAuthStartResult } from "./integrations/oauth-adapter";
import { providerCapabilities, type ProviderCapability } from "./integrations/provider-capabilities";
import { getNativeCredentialStatus, isTauriRuntime, listenNativeOAuth, listenNativeOAuthRejected } from "./integrations/tauri-native-bridge";

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
  if (quota.connectionState === "connected") return "자격 증명 보관 완료 · 잔여량 조회 경로 별도";
  if (quota.authMethod === "not-published") return "개인 OAuth 미공개 · 공식 사용량 범위만";
  return "공식 연결 필요";
}

function sourceLabel(quota: PlanQuota) {
  if (quota.source === "oauth") return "OAuth 확인 완료 · 예시 수치";
  if (quota.source === "unavailable") return "공식 잔여량 경로 확인 중";
  return "예시 수치 · 공식 조회 경로 확인 후 갱신";
}

function authMethodLabel(method: AuthMethod) {
  if (method === "oauth-pkce") return "OAuth · PKCE 흐름";
  if (method === "provider-delegated") return "공급자 delegated auth";
  return "개인 구독 OAuth 미공개";
}

function connectionCapabilityLabel(status: ProviderCapability["connectionStatus"]) {
  if (status === "supported") return "공식 조회 경로 연결 가능";
  if (status === "client-id-required") return "앱 등록 client ID 필요";
  if (status === "api-key-only") return "API 키 전용";
  return "공식 OAuth 경로 미공개";
}

function quotaEndpointStatusLabel(status: ProviderCapability["quotaEndpointStatus"]) {
  if (status === "remaining-published") return "요금제 잔여량 조회 경로 확인";
  if (status === "limit-metadata-only") return "프로젝트 한도 메타데이터만";
  if (status === "organization-usage-only") return "조직 사용량만";
  if (status === "api-key-only") return "API 키 범위만";
  return "개인 잔여량 조회 경로 미공개";
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
  return <button type="button" className={`provider-row ${active ? "active" : ""}`} style={providerStyle(provider.color)} aria-pressed={active} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" />
    <span className="provider-copy"><strong>{provider.name}</strong><small>{quota.planName}</small></span>
    <span className="provider-meter"><i style={{ width: `${primary.remainingPercent}%` }} /></span>
    <span className="provider-value">{primary.remainingPercent}%</span>
  </button>;
});

const ProviderChip = memo(function ProviderChip({ provider, active, onSelect }: Readonly<{ provider: Provider; active: boolean; onSelect: (id: ProviderId) => void }>) {
  const primary = planQuotas[provider.id].windows[0];
  return <button type="button" className={`provider-chip ${active ? "active" : ""}`} style={providerStyle(provider.color)} aria-pressed={active} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" /><span><strong>{provider.name}</strong><small>{primary.remainingPercent}% 남음</small></span>
  </button>;
});

const QuotaSummaryCard = memo(function QuotaSummaryCard({ provider, quota }: Readonly<{ provider: Provider; quota: PlanQuota }>) {
  const [primary, secondary] = quota.windows;
  const valueCopy = quota.source === "unavailable" ? "현재 숫자는 예시 수치입니다." : `${primary.label}의 ${primary.remainingPercent}%가 남아 있습니다.`;
  return <article className="glass-card runway-card quota-card span-2" style={providerStyle(provider.color)}>
    <div className="card-heading"><div><span className="eyebrow">요금제 잔여량</span><h2>{primary.remainingPercent}<span>%</span></h2></div><span className={`quota-status ${quota.connectionState}`}><i />{connectionLabel(quota)}</span></div>
    <div className="runway-content"><div className="runway-ring quota-ring" style={{ "--value": primary.remainingPercent, "--provider": provider.color } as CSSProperties}><div><strong>{primary.remainingPercent}%</strong><span>잔여</span></div></div><div className="runway-copy"><p><strong>{quota.planName}</strong> · {valueCopy} <span className="quota-source">{sourceLabel(quota)}</span></p><div className="rainbow-track"><i style={{ width: `${primary.remainingPercent}%` }} /></div><div className="track-labels"><span>{primary.kindLabel}</span><span>초기화 · {primary.resetLabel}</span></div>{secondary ? <div className="quota-window-list"><QuotaWindowRow window={secondary} compact /></div> : null}</div></div>
  </article>;
});

const OAuthConnectCard = memo(function OAuthConnectCard({ provider, quota, compact = false, onOpen }: Readonly<{ provider: Provider; quota: PlanQuota; compact?: boolean; onOpen: (id: ProviderId) => void }>) {
  const unsupported = quota.connectionState === "unsupported";
  const connected = quota.connectionState === "connected";
  const capability = providerCapabilities[provider.id];
  return <article className={`glass-card oauth-card ${compact ? "compact" : ""}`} style={providerStyle(provider.color)}>
    <div className="oauth-icon"><Icon name={unsupported ? "shield" : connected ? "check" : "link"} size={18} /></div>
    <div className="oauth-copy"><span className="eyebrow">공식 범위</span><h3>{provider.name} 사용량 연결</h3><p>개인 구독 잔여량은 공식 조회 경로가 공개된 경우에만 숫자로 표시합니다.</p><div className={`oauth-status ${unsupported ? "unsupported" : connected ? "connected" : ""}`}><i />{connectionLabel(quota)}</div><div className="oauth-capability"><span>공식 인증</span><strong>{capability.oauthLabel}</strong><span>연결 준비</span><strong>{connectionCapabilityLabel(capability.connectionStatus)}</strong><span>조회 범위</span><strong>{quotaEndpointStatusLabel(capability.quotaEndpointStatus)}</strong></div></div>
    <button type="button" className="oauth-button" onClick={() => onOpen(provider.id)}>{unsupported ? "지원 확인 필요" : connected ? "공식 범위 다시 보기" : "공식 범위 확인"}</button>
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
  const dialogRef = useRef<HTMLDivElement>(null);
  const closeButtonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!open) return;
    const previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    const previousOverflow = document.body.style.overflow;
    const focusTimer = window.setTimeout(() => closeButtonRef.current?.focus(), 0);
    document.body.style.overflow = "hidden";

    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose();
        return;
      }
      if (event.key !== "Tab" || !dialogRef.current) return;
      const focusable = Array.from(dialogRef.current.querySelectorAll<HTMLElement>("button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])"));
      if (focusable.length === 0) {
        event.preventDefault();
        dialogRef.current.focus();
        return;
      }
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      window.clearTimeout(focusTimer);
      document.removeEventListener("keydown", handleKeyDown);
      document.body.style.overflow = previousOverflow;
      previousFocus?.focus();
    };
  }, [onClose, open]);

  if (!open) return null;
  const unsupported = quota.connectionState === "unsupported";
  const connected = quota.connectionState === "connected";
  const capability = providerCapabilities[provider.id];
  const nativeReady = startResult?.status === "authorize-ready";
  const configurationRequired = startResult?.status === "configuration-required";
  const nativeFailed = startResult?.status === "not-available" || startResult?.status === "not-supported";
  const quotaEndpointUnavailable = capability.quotaEndpointStatus !== "remaining-published";
  const loginUnavailable = capability.connectionStatus === "api-key-only" || capability.connectionStatus === "not-published";
  return <div className="oauth-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}><div ref={dialogRef} className="oauth-dialog" role="dialog" aria-modal="true" aria-labelledby="oauth-dialog-title" aria-describedby="oauth-dialog-description" tabIndex={-1}>
    <div className="oauth-dialog-header"><div><span className="eyebrow">공식 연결 경계</span><h2 id="oauth-dialog-title">{provider.name} 사용량 범위</h2></div><button ref={closeButtonRef} type="button" className="icon-button oauth-close" onClick={onClose} aria-label="연결 창 닫기"><Icon name="x" size={17} /></button></div>
    <div className="oauth-provider"><ProviderLogo provider={provider} size="lg" /><div><strong>{quota.planName}</strong><span>{authMethodLabel(quota.authMethod)}</span></div></div>
    <div className="oauth-capability-note"><span>공식 인증 경로</span><strong>{capability.oauthLabel}</strong><span>연결 준비</span><strong>{connectionCapabilityLabel(capability.connectionStatus)}</strong><span>요금제 잔여량</span><strong>{capability.quotaLabel}</strong><span>조회 경로 상태</span><strong>{quotaEndpointStatusLabel(capability.quotaEndpointStatus)}</strong></div>
    <ol className="oauth-steps"><li><span>01</span><div><strong>공식 로그인·사용량 범위 확인</strong><small>공급자가 제공하는 계정·조직 범위만 확인합니다.</small></div></li><li><span>02</span><div><strong>개인 잔여량 공개 여부 판정</strong><small>공식 조회 경로가 없으면 숫자를 임의로 계산하지 않습니다.</small></div></li><li><span>03</span><div><strong>필요할 때만 자격 증명 보관</strong><small>토큰은 UI·브라우저 저장소에 노출하지 않는 구조로 연결합니다.</small></div></li></ol>
    <div id="oauth-dialog-description" className="oauth-demo-note"><Icon name="shield" size={14} /><span>{startResult?.message ?? (quotaEndpointUnavailable ? "현재 개인 구독 잔여량 조회 경로는 공개되지 않았습니다. 화면의 숫자는 예시 수치입니다." : "공식 조회 범위를 확인할 수 있습니다.")}</span></div>
    <div className="oauth-dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>닫기</button>{connected ? <button type="button" className="danger-action" onClick={() => { onDisconnect(provider.id); onClose(); }}>연결 해제</button> : <button type="button" className="primary-action" disabled={unsupported || loginUnavailable || nativeReady || nativeFailed || configurationRequired} onClick={() => { onConnect(provider.id); onClose(); }}>{unsupported ? "공식 경로 확인 필요" : loginUnavailable ? "공식 OAuth 미공개" : nativeReady ? "브라우저 로그인 진행 중" : configurationRequired ? "client ID 설정 필요" : nativeFailed ? "공식 연결 불가" : "연결 완료"}</button>}</div>
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
        <article className="glass-card focus-card"><div className="card-heading"><div><ProviderLogo provider={activeProvider} size="lg" /><span className="eyebrow">집중 확인</span></div><span className="trend-badge">{primary.remainingPercent}% 남음</span></div><h3>{activeProvider.name}</h3><p>{primary.label}는 {primary.resetLabel} 초기화됩니다. <strong>{activeQuota.planName}</strong> 기준 예시 수치입니다.</p><Sparkline values={activeProvider.trend} color={activeProvider.color} width={220} height={62} /></article>
        <article className="glass-card budget-card plan-card"><span className="eyebrow">연결된 요금제</span><h3>{activeQuota.planName}</h3><p><span>{activeQuota.accountLabel}</span><span className={activeQuota.connectionState === "connected" ? "positive" : ""}>{activeQuota.lastSyncedAt ? `마지막 확인 · ${activeQuota.lastSyncedAt}` : "연결 전 · 예시 수치"}</span></p><div className="micro-stat"><span>인증 방식</span><strong>{authMethodLabel(activeQuota.authMethod)}</strong></div></article>
        <OAuthConnectCard provider={activeProvider} quota={activeQuota} onOpen={onOpenOAuth} />
      </div>
      <footer className="privacy-strip"><Icon name="shield" size={15} /><span>잔여량 숫자는 공식 조회 경로가 확인된 경우에만 갱신됩니다.</span><i /><span>{sourceLabel(activeQuota)}</span></footer>
    </section>
  </div>;
});

const MobileNav = memo(function MobileNav() {
  const items: readonly Readonly<{ name: IconName; label: string }>[] = [{ name: "pulse", label: "현황" }, { name: "grid", label: "서비스" }, { name: "spark", label: "추이" }, { name: "settings", label: "설정" }];
  return <nav className="mobile-nav" aria-label="하단 메뉴">{items.map((item, index) => <button type="button" className={index === 0 ? "active" : ""} aria-current={index === 0 ? "page" : undefined} key={item.name}><Icon name={item.name} size={19} /><span>{item.label}</span></button>)}</nav>;
});

const VariantCMobile = memo(function VariantCMobile({ activeProvider, activeProviderId, activeQuota, onProvider, onOpenOAuth }: SharedViewProps) {
  const codex = providers.find(provider => provider.id === "codex") ?? providers[0];
  const claude = providers.find(provider => provider.id === "claude") ?? providers[1];
  const codexQuota = planQuotas.codex;
  const claudeQuota = planQuotas.claude;
  const primary = activeQuota.windows[0];
  return <div className="product-shell variant-c"><section className="stream-app"><div className="stream-layout"><section className="mobile-stream">
    <div className="mobile-top"><span>9:41</span><div><i /><i /><i /></div></div>
    <div className="mobile-title"><div><span className="eyebrow">8월 15일</span><h1>사용 현황</h1></div><button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
    <article className="hero-signal" style={providerStyle(activeProvider.color)}><div className="hero-signal-top"><span className="signal-orb"><ProviderLogo provider={activeProvider} size="md" /></span><span className={`trend-badge ${activeQuota.connectionState === "connected" ? "positive" : ""}`}>{activeQuota.connectionState === "connected" ? "연결됨" : "예시 수치"}</span></div><span className="eyebrow">{activeQuota.planName} · {primary.label}</span><h2>{primary.remainingPercent}<span>%</span></h2><p>초기화 전까지 남은 요금제 여유입니다.</p><div className="spectrum-line"><i style={{ width: `${primary.remainingPercent}%` }} /></div><div className="signal-foot"><span>남은 한도 {primary.remainingPercent}%</span><span>초기화 {primary.resetLabel}</span></div></article>
    <div className="chip-scroll" role="group" aria-label="서비스 선택">{providers.map(provider => <ProviderChip key={provider.id} provider={provider} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div>
    <OAuthConnectCard provider={activeProvider} quota={activeQuota} compact onOpen={onOpenOAuth} />
    <section className="stream-feed"><div className="section-title"><div><span className="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><button type="button">전체</button></div>
      <article className="feed-item priority"><span className="feed-line" style={providerStyle(claude.color)} /><ProviderLogo provider={claude} size="sm" /><div><span className="feed-time">지금 · CLAUDE</span><h4>개인 잔여량 조회 경로를 확인할 수 없습니다.</h4><p>{claudeQuota.windows[0].remainingPercent}%는 예시 수치입니다. 공식 사용량 범위만 연결합니다.</p></div><strong>{claudeQuota.windows[0].remainingPercent}%</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(codex.color)} /><ProviderLogo provider={codex} size="sm" /><div><span className="feed-time">14분 전 · CODEX</span><h4>ChatGPT 요금제 사용량 페이지를 확인하세요.</h4><p>{codexQuota.windows[0].remainingPercent}%는 예시 수치이며, 개인 잔여량 API는 미공개입니다.</p></div><strong>{codexQuota.windows[0].remainingPercent}%</strong></article>
    </section>
    <MobileNav />
  </section></div></section></div>;
});

export function App() {
  const isMobile = useIsMobile();
  const [activeProviderId, setActiveProviderId] = useState<ProviderId>("codex");
  const [metric, setMetric] = useState<Metric>("remaining");
  const [range, setRange] = useState<UsageRange>("7D");
  const [solid, setSolid] = useState(false);
  const [refreshedAt, setRefreshedAt] = useState("방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const [connectedProviderIds, setConnectedProviderIds] = useState<readonly ProviderId[]>([]);
  const [oauthProviderId, setOauthProviderId] = useState<ProviderId>("codex");
  const [oauthOpen, setOauthOpen] = useState(false);
  const [oauthStartResult, setOauthStartResult] = useState<OAuthStartResult | null>(null);
  const refreshTimer = useRef<number | null>(null);
  const activeProvider = useMemo(() => providers.find(provider => provider.id === activeProviderId) ?? providers[0], [activeProviderId]);
  const activeQuota = useMemo(() => {
    const baseQuota = planQuotas[activeProviderId];
    if (baseQuota.connectionState === "unsupported" || !connectedProviderIds.includes(activeProviderId)) return baseQuota;
    return { ...baseQuota, accountLabel: "OAuth 자격 증명 · OS 보관 완료", connectionState: "connected" as const, source: "unavailable" as const, confidence: "unavailable" as const, lastSyncedAt: null };
  }, [activeProviderId, connectedProviderIds]);
  const oauthProvider = useMemo(() => providers.find(provider => provider.id === oauthProviderId) ?? providers[0], [oauthProviderId]);
  const oauthQuota = useMemo(() => {
    const baseQuota = planQuotas[oauthProviderId];
    if (baseQuota.connectionState === "unsupported" || !connectedProviderIds.includes(oauthProviderId)) return baseQuota;
    return { ...baseQuota, accountLabel: "OAuth 자격 증명 · OS 보관 완료", connectionState: "connected" as const, source: "unavailable" as const, confidence: "unavailable" as const, lastSyncedAt: null };
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
    if (!isTauriRuntime()) return;
    let disposed = false;
    void Promise.all(providers.map(provider => getNativeCredentialStatus(provider.id)))
      .then(statuses => {
        if (disposed) return;
        setConnectedProviderIds(statuses.flatMap(status => status?.present ? [status.providerId] : []));
      })
      .catch(() => undefined);
    return () => {
      disposed = true;
    };
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlistenOAuth: (() => void | Promise<void>) | null = null;
    let unlistenRejected: (() => void | Promise<void>) | null = null;
    void listenNativeOAuth(event => {
      if (disposed || !isProviderId(event.providerId)) return;
      setOauthProviderId(event.providerId);
      setOauthOpen(true);
      if (event.status === "credential-stored") {
        setConnectedProviderIds(current => current.includes(event.providerId as ProviderId) ? current : [...current, event.providerId as ProviderId]);
        setOauthStartResult({
          status: "authorize-ready",
          message: "공식 토큰 교환을 완료했고 자격 증명은 OS 보관소에 저장했습니다. 잔여량 조회 경로는 별도로 확인합니다."
        });
      }
    }).then(listener => {
      if (disposed) {
        void listener?.();
      } else {
        unlistenOAuth = listener;
      }
    });
    void listenNativeOAuthRejected(event => {
      if (disposed) return;
      setOauthStartResult({
        status: "not-available",
        message: `공식 토큰 교환을 완료하지 못했습니다 (${event.reason}). 토큰은 보관되지 않았습니다.`
      });
    }).then(listener => {
      if (disposed) {
        void listener?.();
      } else {
        unlistenRejected = listener;
      }
    });
    return () => {
      disposed = true;
      void unlistenOAuth?.();
      void unlistenRejected?.();
    };
  }, []);

  const openOAuth = useCallback((id: ProviderId) => {
    setOauthProviderId(id);
    setOauthStartResult(null);
    setOauthOpen(true);
    void getOAuthAdapter(id).start().then(setOauthStartResult);
  }, []);
  const closeOAuth = useCallback(() => {
    setOauthOpen(false);
    setOauthStartResult(null);
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

  return <><div className={solid ? "solid-mode" : ""}>{isMobile ? <VariantCMobile {...sharedProps} /> : <VariantADesktop {...sharedProps} />}</div><OAuthDialog open={oauthOpen} provider={oauthProvider} quota={oauthQuota} startResult={oauthStartResult} onClose={closeOAuth} onConnect={connectOAuth} onDisconnect={disconnectOAuth} /></>;
}
