import { memo, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Icon, type IconName } from "./components/Icon";
import { Sparkline } from "./components/Sparkline";
import { metricLabels, planQuotas, providers, rangeLabels, usageBars, type AuthMethod, type Metric, type PlanQuota, type Provider, type ProviderId, type QuotaWindow, type QuotaWindowId, type UsageRange } from "./data/providers";
import { providerCapabilities, type ProviderCapability } from "./integrations/provider-capabilities";
import { getNativeProviderUsage, installNativeProviderBridge, isTauriRuntime, removeNativeProviderBridge, startNativeProviderLogin, type NativeProviderActionResult, type NativeProviderUsageSnapshot } from "./integrations/tauri-native-bridge";

const mobileBreakpoint = "(max-width: 820px)";
const mobileClockFormatter = new Intl.DateTimeFormat("ko-KR", { hour: "2-digit", minute: "2-digit", hour12: false });
const mobileDateFormatter = new Intl.DateTimeFormat("ko-KR", { month: "long", day: "numeric" });

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

type ProviderActionFeedback = Readonly<{
  status: NativeProviderActionResult["status"] | "refreshed" | "demo-only";
  message: string;
}>;

type QuotaRecord = Readonly<Record<ProviderId, PlanQuota>>;
type ProductView = "overview" | "services" | "trend" | "alerts" | "settings";
type ThemeMode = "dark" | "light";

const hasDisplayValue = (quota: PlanQuota) => quota.confidence !== "unavailable";
const displayPercent = (quota: PlanQuota, window = quota.windows[0]) => hasDisplayValue(quota) ? `${Math.round(window.remainingPercent)}%` : "—";
const meterPercent = (quota: PlanQuota, window = quota.windows[0]) => hasDisplayValue(quota) ? window.remainingPercent : 0;

function formatReset(epochSeconds: number | null) {
  if (!epochSeconds) return "초기화 시각 확인 중";
  const remainingMs = (epochSeconds * 1000) - Date.now();
  if (remainingMs <= 0) return "초기화 정보 갱신 필요";
  const minutes = Math.ceil(remainingMs / 60_000);
  if (minutes < 60) return `${minutes}분 후`;
  const hours = Math.floor(minutes / 60);
  const restMinutes = minutes % 60;
  if (hours < 24) return restMinutes ? `${hours}시간 ${restMinutes}분 후` : `${hours}시간 후`;
  const days = Math.floor(hours / 24);
  const restHours = hours % 24;
  return restHours ? `${days}일 ${restHours}시간 후` : `${days}일 후`;
}

function formatSyncedAt(epochSeconds: number | null) {
  if (!epochSeconds) return null;
  return new Intl.DateTimeFormat("ko-KR", { hour: "2-digit", minute: "2-digit" }).format(new Date(epochSeconds * 1000));
}

function planName(providerId: ProviderId, planType: string | null) {
  if (!planType) return providerId === "codex" ? "ChatGPT · Codex" : "Claude";
  const normalized = planType.toLowerCase();
  const knownPlans: Readonly<Record<string, string>> = {
    free: "Free",
    go: "Go",
    plus: "Plus",
    pro: "Pro",
    prolite: "Pro",
    max: "Max",
    team: "Team",
    business: "Business",
    enterprise: "Enterprise",
    edu: "Edu"
  };
  const title = knownPlans[normalized] ?? planType;
  return providerId === "codex" ? `ChatGPT ${title} · Codex` : `Claude ${title}`;
}

function quotaFromSnapshot(snapshot: NativeProviderUsageSnapshot, fallback: PlanQuota): PlanQuota {
  const connectionState: PlanQuota["connectionState"] = snapshot.connectionState === "not-installed"
    ? "not_installed"
    : snapshot.connectionState === "signed-out"
      ? "signed_out"
      : snapshot.connectionState === "waiting-for-usage"
        ? "waiting"
        : snapshot.connectionState;
  const verified = snapshot.windows.length > 0 && (snapshot.source === "codex-app-server" || snapshot.source === "claude-statusline");
  const windows = snapshot.windows.length > 0
    ? snapshot.windows.map(window => ({
        id: window.id,
        label: window.label,
        usedPercent: window.usedPercent,
        remainingPercent: window.remainingPercent,
        resetLabel: formatReset(window.resetsAt),
        kindLabel: snapshot.source === "codex-app-server" ? "Codex 요금제 사용량" : "Claude.ai 공유 사용량"
      }))
    : fallback.windows.map(window => ({ ...window, usedPercent: 0, remainingPercent: 0, resetLabel: "연결 후 표시", kindLabel: "실제 데이터 대기" }));
  return {
    ...fallback,
    planName: planName(snapshot.providerId, snapshot.planType),
    accountLabel: snapshot.authState === "signed-in" ? "공식 계정 연결 확인" : snapshot.runtimeAvailable ? "계정 연결 필요" : "공식 CLI 설치 필요",
    authMethod: "provider-delegated",
    connectionState,
    source: verified ? snapshot.source! : "unavailable",
    confidence: verified ? "verified" : "unavailable",
    lastSyncedAt: formatSyncedAt(snapshot.lastSyncedAt),
    bridgeInstalled: snapshot.bridgeInstalled,
    statusMessage: snapshot.message,
    windows
  };
}

function nativePendingQuota(fallback: PlanQuota): PlanQuota {
  return {
    ...fallback,
    accountLabel: "공식 계정 상태 확인 중",
    connectionState: "not_connected",
    source: "unavailable",
    confidence: "unavailable",
    lastSyncedAt: null,
    bridgeInstalled: false,
    statusMessage: "공식 도구의 계정 및 사용량 상태를 확인하고 있습니다.",
    windows: fallback.windows.map(window => ({
      ...window,
      usedPercent: 0,
      remainingPercent: 0,
      resetLabel: "연결 후 표시",
      kindLabel: "실제 데이터 대기"
    }))
  };
}

function initialQuotaRecord(): QuotaRecord {
  if (!isTauriRuntime()) return planQuotas;
  return {
    codex: nativePendingQuota(planQuotas.codex),
    claude: nativePendingQuota(planQuotas.claude)
  };
}

function connectionLabel(quota: PlanQuota) {
  if (quota.connectionState === "not_installed") return "공식 CLI 설치 필요";
  if (quota.connectionState === "signed_out") return "계정 로그인 필요";
  if (quota.connectionState === "waiting") return quota.bridgeInstalled ? "로그인됨 · 첫 사용량 대기" : "로그인됨 · 동기화 설정 필요";
  if (quota.connectionState === "stale") return "연결됨 · 데이터 갱신 필요";
  if (quota.connectionState === "unsupported") return "OAuth 지원 확인 필요";
  if (quota.connectionState === "error") return "연결 상태 확인 필요";
  if (quota.connectionState === "connected") return "공식 요금제 사용량 연결됨";
  return "공식 계정 연결 필요";
}

function sourceLabel(quota: PlanQuota) {
  if (quota.source === "codex-app-server") return "Codex 공식 App Server";
  if (quota.source === "claude-statusline") return "Claude Code 공식 상태선";
  if (quota.source === "unavailable") return "실제 사용량 데이터 대기";
  return "브라우저 데모 수치";
}

function authMethodLabel(method: AuthMethod) {
  if (method === "oauth-pkce") return "OAuth · PKCE 흐름";
  if (method === "provider-delegated") return "공급자 공식 도구 관리";
  return "공식 로그인 경로 확인 필요";
}

function connectionCapabilityLabel(status: ProviderCapability["connectionStatus"]) {
  if (status === "supported") return "공식 계정 연결 가능";
  if (status === "client-id-required") return "앱 등록 client ID 필요";
  if (status === "api-key-only") return "API 키 전용";
  return "공식 연결 경로 미공개";
}

function quotaEndpointStatusLabel(status: ProviderCapability["quotaEndpointStatus"]) {
  if (status === "remaining-published") return "요금제 사용률·초기화 조회 가능";
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

const TopActions = memo(function TopActions({ refreshedAt, refreshing, solid, theme, onRefresh, onSolid, onTheme }: Readonly<{
  refreshedAt: string;
  refreshing: boolean;
  solid: boolean;
  theme: ThemeMode;
  onRefresh: () => void;
  onSolid: () => void;
  onTheme: () => void;
}>) {
  return <div className="top-actions">
    <span className="sync-state"><i /> 동기화 · {refreshedAt}</span>
    <button type="button" className={`icon-button ${refreshing ? "spinning" : ""}`} onClick={onRefresh} aria-label="데이터 새로고침"><Icon name="refresh" size={17} /></button>
    <button type="button" className="icon-button" onClick={onSolid} aria-label="투명도 전환" title={solid ? "유리 모드" : "가독성용 불투명 모드"}><Icon name="eye" size={17} /></button>
    <button type="button" className="icon-button" onClick={onTheme} aria-label="테마 전환" title={theme === "dark" ? "일반 모드" : "다크 모드"}><Icon name="spark" size={17} /></button>
    <button type="button" className="avatar" aria-label="프로필">HJ</button>
  </div>;
});

const NavRail = memo(function NavRail({ view, onView }: Readonly<{ view: ProductView; onView: (view: ProductView) => void }>) {
  const items: readonly Readonly<{ name: IconName; label: string; view: ProductView; notice?: boolean }>[] = [
    { name: "grid", label: "개요", view: "overview" },
    { name: "grid", label: "서비스", view: "services" },
    { name: "pulse", label: "추이", view: "trend" },
    { name: "bell", label: "알림", view: "alerts", notice: true }
  ];
  return <aside className="nav-rail" aria-label="주요 메뉴">
    <div className="rail-mark"><span className="brand-mark compact"><i /><i /><i /></span></div>
    <nav>{items.map(item => <button type="button" className={view === item.view ? "active" : ""} aria-label={item.label} aria-current={view === item.view ? "page" : undefined} key={item.view} onClick={() => onView(item.view)}><Icon name={item.name} />{item.notice ? <span className="notification-dot" /> : null}</button>)}</nav>
    <button type="button" className={view === "settings" ? "active" : ""} aria-label="설정" aria-current={view === "settings" ? "page" : undefined} onClick={() => onView("settings")}><Icon name="settings" /></button>
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

const QuotaWindowRow = memo(function QuotaWindowRow({ window, unavailable = false, compact = false }: Readonly<{ window: QuotaWindow; unavailable?: boolean; compact?: boolean }>) {
  return <div className={`quota-window ${compact ? "compact" : ""}`}>
    <div className="quota-window-copy"><span>{window.label}</span><strong>{unavailable ? "—" : `${Math.round(window.remainingPercent)}% 남음`}</strong></div>
    <div className="quota-window-meta"><span>{window.kindLabel}</span><span>{window.resetLabel}</span></div>
    <div className="quota-window-meter" aria-label={unavailable ? `${window.label} 데이터 대기` : `${window.label} ${window.usedPercent}% 사용`}><i style={{ width: `${unavailable ? 0 : window.usedPercent}%` }} /></div>
  </div>;
});

const emptyQuotaWindow = (id: QuotaWindowId, providerId: ProviderId): QuotaWindow => ({
  id,
  label: planQuotas[providerId].windows.find(candidate => candidate.id === id)?.label ?? (id === "rolling" ? "5시간 한도" : "주간 한도"),
  usedPercent: 0,
  remainingPercent: 0,
  resetLabel: "연결 후 표시",
  kindLabel: "실제 데이터 대기"
});

const QuotaCell = memo(function QuotaCell({ provider, window, available }: Readonly<{ provider: Provider; window: QuotaWindow; available: boolean }>) {
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
      const found = quota.windows.find(candidate => candidate.id === id);
      const window = found ?? emptyQuotaWindow(id, provider.id);
      const available = hasDisplayValue(quota) && found !== undefined;
      return <QuotaCell key={`${provider.id}-${id}`} provider={provider} window={window} available={available} />;
    }))}
  </div>;
});

const ProviderRow = memo(function ProviderRow({ provider, quota, active, onSelect }: Readonly<{ provider: Provider; quota: PlanQuota; active: boolean; onSelect: (id: ProviderId) => void }>) {
  const primary = quota.windows[0];
  return <button type="button" className={`provider-row ${active ? "active" : ""}`} style={providerStyle(provider.color)} aria-pressed={active} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" />
    <span className="provider-copy"><strong>{provider.name}</strong><small>{quota.planName}</small></span>
    <span className="provider-meter"><i style={{ width: `${meterPercent(quota, primary)}%` }} /></span>
    <span className="provider-value">{displayPercent(quota, primary)}</span>
  </button>;
});

const ProviderChip = memo(function ProviderChip({ provider, quota, active, onSelect }: Readonly<{ provider: Provider; quota: PlanQuota; active: boolean; onSelect: (id: ProviderId) => void }>) {
  const primary = quota.windows[0];
  return <button type="button" className={`provider-chip ${active ? "active" : ""}`} style={providerStyle(provider.color)} aria-pressed={active} onClick={() => onSelect(provider.id)}>
    <ProviderLogo provider={provider} size="sm" /><span><strong>{provider.name}</strong><small>{hasDisplayValue(quota) ? `${Math.round(primary.remainingPercent)}% 남음` : connectionLabel(quota)}</small></span>
  </button>;
});

const OAuthConnectCard = memo(function OAuthConnectCard({ provider, quota, compact = false, onOpen }: Readonly<{ provider: Provider; quota: PlanQuota; compact?: boolean; onOpen: (id: ProviderId) => void }>) {
  const unsupported = quota.connectionState === "unsupported";
  const connected = quota.connectionState === "connected";
  const capability = providerCapabilities[provider.id];
  return <article className={`glass-card oauth-card ${compact ? "compact" : ""}`} style={providerStyle(provider.color)}>
    <div className="oauth-icon"><Icon name={unsupported ? "shield" : connected ? "check" : "link"} size={18} /></div>
    <div className="oauth-copy"><span className="eyebrow">계정 연결</span><h3>{provider.name} 사용량 연결</h3><p>공급자의 공식 로그인과 사용량 인터페이스에서 개인 요금제 잔여량을 확인합니다.</p><div className={`oauth-status ${quota.connectionState}`}><i />{connectionLabel(quota)}</div><div className="oauth-capability"><span>공식 인증</span><strong>{capability.oauthLabel}</strong><span>연결 준비</span><strong>{connectionCapabilityLabel(capability.connectionStatus)}</strong><span>조회 범위</span><strong>{quotaEndpointStatusLabel(capability.quotaEndpointStatus)}</strong></div></div>
    <button type="button" className="oauth-button" onClick={() => onOpen(provider.id)}>{quota.connectionState === "not_installed" ? "설치 안내" : quota.connectionState === "signed_out" ? "계정 로그인" : connected || quota.connectionState === "stale" ? "계정·동기화 관리" : "연결 설정"}</button>
    <div className="oauth-security"><Icon name="shield" size={13} /><span>공급자 토큰은 SPECTRA에 복제하지 않습니다.</span></div>
  </article>;
});

const OAuthDialog = memo(function OAuthDialog({ open, provider, quota, startResult, onClose, onConnect, onDisconnect }: Readonly<{
  open: boolean;
  provider: Provider;
  quota: PlanQuota;
  startResult: ProviderActionFeedback | null;
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
  const capability = providerCapabilities[provider.id];
  const loginPending = startResult?.status === "login-started";
  const runtimeMissing = quota.connectionState === "not_installed";
  const needsLogin = quota.connectionState === "signed_out";
  const needsBridge = provider.id === "claude" && quota.connectionState === "waiting" && !quota.bridgeInstalled;
  const primaryLabel = runtimeMissing
    ? `${provider.name} CLI 설치 필요`
    : needsLogin
      ? `${provider.name} 계정 로그인`
      : needsBridge
        ? "사용량 브리지 설치"
        : "사용량 새로고침";
  return <div className="oauth-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}><div ref={dialogRef} className="oauth-dialog" role="dialog" aria-modal="true" aria-labelledby="oauth-dialog-title" aria-describedby="oauth-dialog-description" tabIndex={-1}>
    <div className="oauth-dialog-header"><div><span className="eyebrow">공식 계정 연결</span><h2 id="oauth-dialog-title">{provider.name} 계정 및 사용량</h2></div><button ref={closeButtonRef} type="button" className="icon-button oauth-close" onClick={onClose} aria-label="연결 창 닫기"><Icon name="x" size={17} /></button></div>
    <div className="oauth-provider"><ProviderLogo provider={provider} size="lg" /><div><strong>{quota.planName}</strong><span>{authMethodLabel(quota.authMethod)}</span></div></div>
    <div className="oauth-capability-note"><span>공식 인증 경로</span><strong>{capability.oauthLabel}</strong><span>연결 준비</span><strong>{connectionCapabilityLabel(capability.connectionStatus)}</strong><span>요금제 잔여량</span><strong>{capability.quotaLabel}</strong><span>조회 경로 상태</span><strong>{quotaEndpointStatusLabel(capability.quotaEndpointStatus)}</strong></div>
    <ol className="oauth-steps"><li><span>01</span><div><strong>공급자 공식 도구로 로그인</strong><small>{provider.id === "codex" ? "Codex가 ChatGPT 로그인과 토큰 갱신을 관리합니다." : "Claude Code가 Claude.ai 로그인을 관리합니다."}</small></div></li><li><span>02</span><div><strong>개인 요금제 한도만 읽기</strong><small>{provider.id === "codex" ? "App Server의 한도 창·사용률·초기화 시각을 확인합니다." : "상태선의 5시간·7일 한도 필드만 정제해 저장합니다."}</small></div></li><li><span>03</span><div><strong>토큰을 SPECTRA에 복제하지 않기</strong><small>화면에는 요금제·퍼센트·초기화 시각과 최신성만 전달합니다.</small></div></li></ol>
    <div id="oauth-dialog-description" className="oauth-demo-note"><Icon name="shield" size={14} /><span>{startResult?.message ?? quota.statusMessage}</span></div>
    <div className="oauth-dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>닫기</button>{provider.id === "claude" && quota.bridgeInstalled ? <button type="button" className="danger-action" onClick={() => onDisconnect(provider.id)}>브리지 제거</button> : null}<button type="button" className="primary-action" disabled={unsupported || runtimeMissing || loginPending} onClick={() => onConnect(provider.id)}>{loginPending ? "브라우저 로그인 진행 중" : primaryLabel}</button></div>
  </div></div>;
});

type LayoutActions = Readonly<{
  refreshedAt: string;
  refreshing: boolean;
  solid: boolean;
  theme: ThemeMode;
  onRefresh: () => void;
  onSolid: () => void;
  onTheme: () => void;
}>;

type SharedViewProps = LayoutActions & Readonly<{
  view: ProductView;
  onView: (view: ProductView) => void;
  activeProvider: Provider;
  activeProviderId: ProviderId;
  activeQuota: PlanQuota;
  quotas: QuotaRecord;
  metric: Metric;
  range: UsageRange;
  onProvider: (id: ProviderId) => void;
  onMetric: (metric: Metric) => void;
  onRange: (range: UsageRange) => void;
  onOpenOAuth: (id: ProviderId) => void;
}>;

function viewCopy(view: ProductView) {
  if (view === "services") return { eyebrow: "서비스", title: "연결된 서비스를\n비교합니다." };
  if (view === "trend") return { eyebrow: "추이", title: "현재 한도 창을\n살펴봅니다." };
  if (view === "alerts") return { eyebrow: "알림", title: "지금 확인할\n항목입니다." };
  if (view === "settings") return { eyebrow: "설정", title: "사용 환경을\n조정합니다." };
  return { eyebrow: "개요 · 오늘", title: "오늘 쓸 수 있는 양을\n한눈에 봅니다." };
}

type ViewPanelProps = Omit<SharedViewProps, "onView">;

const DesktopViewPanel = memo(function DesktopViewPanel({ view, activeProvider, activeProviderId, activeQuota, quotas, metric, range, onProvider, onMetric, onRange, onOpenOAuth, refreshedAt, refreshing, solid, theme, onRefresh, onSolid, onTheme }: ViewPanelProps) {
  if (view === "services") {
    return <div className="view-stack"><article className="glass-card providers-card"><div className="card-heading"><div><span className="eyebrow">서비스</span><h3>서비스별 잔여량</h3></div><span className="live-pill"><i />공식 조회</span></div><div className="provider-list">{providers.map(provider => <ProviderRow key={provider.id} provider={provider} quota={quotas[provider.id]} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div></article><OAuthConnectCard provider={activeProvider} quota={activeQuota} onOpen={onOpenOAuth} /></div>;
  }
  if (view === "trend") {
    return <div className="view-stack"><div className="dashboard-toolbar"><MetricTabs metric={metric} onMetric={onMetric} /><RangeTabs range={range} onRange={onRange} /></div><article className="glass-card trend-panel"><div className="card-heading"><div><span className="eyebrow">현재 한도 창</span><h3>{activeProvider.name} 사용량</h3></div><span className="live-pill"><i />{activeQuota.source === "example" ? "데모 집계" : "현재 스냅샷"}</span></div><p className="view-description">과거 사용량은 저장하지 않고, 공식 경로에서 확인한 현재 한도와 초기화 시각만 보여줍니다.</p><div className="trend-window-grid">{activeQuota.windows.map(window => <QuotaWindowRow key={window.id} window={window} unavailable={!hasDisplayValue(activeQuota)} />)}</div></article></div>;
  }
  if (view === "alerts") {
    return <div className="view-stack"><article className="glass-card alert-panel"><div className="card-heading"><div><span className="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><span className="live-pill"><i />현재 상태</span></div>{providers.map(provider => <div className="alert-row" key={provider.id} style={providerStyle(provider.color)}><ProviderLogo provider={provider} size="sm" /><div><strong>{connectionLabel(quotas[provider.id])}</strong><span>{quotas[provider.id].statusMessage}</span></div><b>{displayPercent(quotas[provider.id])}</b></div>)}</article></div>;
  }
  return <div className="view-stack"><article className="glass-card settings-panel"><div className="card-heading"><div><span className="eyebrow">설정</span><h3>사용 환경</h3></div><span className="live-pill"><i />기기 안에서만 처리</span></div><div className="settings-row"><div><strong>화면 테마</strong><span>{theme === "dark" ? "짙은 배경과 선명한 대비를 사용합니다." : "밝은 배경과 부드러운 대비를 사용합니다."}</span></div><button type="button" className="secondary-action" onClick={onTheme}>{theme === "dark" ? "일반 모드" : "다크 모드"}</button></div><div className="settings-row"><div><strong>가독성용 불투명 모드</strong><span>{solid ? "현재 불투명 표면을 사용합니다." : "현재 유리 효과를 사용합니다."}</span></div><button type="button" className="secondary-action" onClick={onSolid}>{solid ? "유리 모드" : "불투명 모드"}</button></div><div className="settings-row"><div><strong>공식 사용량 새로고침</strong><span>Codex App Server와 Claude status line 캐시를 다시 확인합니다.</span></div><button type="button" className="primary-action" onClick={onRefresh} disabled={refreshing}>{refreshing ? "확인 중" : "지금 확인"}</button></div><div className="settings-note"><Icon name="shield" size={15} /><span>토큰·이메일·세션 원문은 SPECTRA에 복제하지 않습니다. 마지막 확인 · {refreshedAt}</span></div></article></div>;
});

const VariantADesktop = memo(function VariantADesktop({ view, onView, activeProvider, activeProviderId, activeQuota, quotas, metric, range, onProvider, onMetric, onRange, onOpenOAuth, ...actions }: SharedViewProps) {
  const primary = activeQuota.windows[0];
  const available = hasDisplayValue(activeQuota);
  const demo = activeQuota.source === "example";
  const copy = viewCopy(view);
  return <div className="product-shell variant-a">
    <NavRail view={view} onView={onView} />
    <section className="app-surface command-center">
      <header className="app-header">
        <div><span className="eyebrow">{copy.eyebrow}</span><h1>{copy.title.split("\n").map((line, index) => <span key={line}>{index > 0 ? <br /> : null}{index === copy.title.split("\n").length - 1 ? <em>{line}</em> : line}</span>)}</h1></div>
        <TopActions {...actions} />
      </header>
      {view === "overview" ? <><div className="dashboard-toolbar"><MetricTabs metric={metric} onMetric={onMetric} /><RangeTabs range={range} onRange={onRange} /></div><div className="bento-grid">
        <QuotaBoard quotas={quotas} />
        <article className="glass-card live-card span-2"><div className="card-heading"><div><span className="eyebrow">한도 소진 추이</span><h3>{available ? `${Math.round(primary.usedPercent)}%` : "—"} <small>{available ? "현재 사용" : "실제 데이터 대기"}</small></h3></div><span className="live-pill"><i />{demo ? "데모 집계" : available ? "현재 스냅샷" : "연결 대기"}</span></div>{demo ? <><ChartBars /><div className="chart-axis"><span>09:00</span><span>12:00</span><span>15:00</span><span>지금</span></div></> : <div className="chart-empty"><Icon name="pulse" size={19} /><strong>과거 추이는 저장하지 않습니다.</strong><span>현재 한도 스냅샷만 읽어 메모리 사용을 줄였습니다.</span></div>}</article>
        <article className="glass-card providers-card span-2"><div className="card-heading"><div><span className="eyebrow">서비스</span><h3>서비스별 잔여량</h3></div><button type="button" className="text-button">모두 보기 <Icon name="chevron" size={14} /></button></div><div className="provider-list">{providers.map(provider => <ProviderRow key={provider.id} provider={provider} quota={quotas[provider.id]} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div></article>
        <article className="glass-card focus-card"><div className="card-heading"><div><ProviderLogo provider={activeProvider} size="lg" /><span className="eyebrow">집중 확인</span></div><span className="trend-badge">{displayPercent(activeQuota, primary)}{available ? " 남음" : ""}</span></div><h3>{activeProvider.name}</h3><p>{available ? <>{primary.label}는 {primary.resetLabel} 초기화됩니다. <strong>{activeQuota.planName}</strong> {demo ? "브라우저 데모" : "공식 조회"} 수치입니다.</> : activeQuota.statusMessage}</p>{demo ? <Sparkline values={activeProvider.trend} color={activeProvider.color} width={220} height={62} /> : <div className="focus-status"><Icon name={available ? "check" : "link"} size={17} /><span>{sourceLabel(activeQuota)}</span></div>}</article>
        <article className="glass-card budget-card plan-card"><span className="eyebrow">연결된 요금제</span><h3>{activeQuota.planName}</h3><p><span>{activeQuota.accountLabel}</span><span className={activeQuota.connectionState === "connected" ? "positive" : ""}>{activeQuota.lastSyncedAt ? `마지막 확인 · ${activeQuota.lastSyncedAt}` : connectionLabel(activeQuota)}</span></p><div className="micro-stat"><span>인증 방식</span><strong>{authMethodLabel(activeQuota.authMethod)}</strong></div></article>
        <OAuthConnectCard provider={activeProvider} quota={activeQuota} onOpen={onOpenOAuth} />
      </div></> : <DesktopViewPanel view={view} activeProvider={activeProvider} activeProviderId={activeProviderId} activeQuota={activeQuota} quotas={quotas} metric={metric} range={range} onProvider={onProvider} onMetric={onMetric} onRange={onRange} onOpenOAuth={onOpenOAuth} {...actions} />}
      <footer className="privacy-strip"><Icon name="shield" size={15} /><span>잔여량 숫자는 공식 조회 경로가 확인된 경우에만 갱신됩니다.</span><i /><span>{sourceLabel(activeQuota)}</span></footer>
    </section>
  </div>;
});

const MobileNav = memo(function MobileNav({ view, onView }: Readonly<{ view: ProductView; onView: (view: ProductView) => void }>) {
  const items: readonly Readonly<{ name: IconName; label: string; view: ProductView }>[] = [{ name: "pulse", label: "현황", view: "overview" }, { name: "grid", label: "서비스", view: "services" }, { name: "spark", label: "추이", view: "trend" }, { name: "settings", label: "설정", view: "settings" }];
  return <nav className="mobile-nav" aria-label="하단 메뉴">{items.map(item => <button type="button" className={view === item.view ? "active" : ""} aria-current={view === item.view ? "page" : undefined} key={item.view} onClick={() => onView(item.view)}><Icon name={item.name} size={19} /><span>{item.label}</span></button>)}</nav>;
});

const VariantCMobile = memo(function VariantCMobile({ view, onView, activeProvider, activeProviderId, activeQuota, quotas, onProvider, onOpenOAuth, ...actions }: SharedViewProps) {
  const codex = providers.find(provider => provider.id === "codex") ?? providers[0];
  const claude = providers.find(provider => provider.id === "claude") ?? providers[1];
  const codexQuota = quotas.codex;
  const claudeQuota = quotas.claude;
  const now = new Date();
  return <div className="product-shell variant-c"><section className="stream-app"><div className="stream-layout"><section className="mobile-stream">
    <div className="mobile-top"><span>{mobileClockFormatter.format(now)}</span><div><i /><i /><i /></div></div>
    <div className="mobile-title"><div><span className="eyebrow">{mobileDateFormatter.format(now)}</span><h1>사용 현황</h1></div><button type="button" className="icon-button" aria-label="알림"><Icon name="bell" size={18} /><span className="notification-dot" /></button></div>
    {view === "overview" ? <><QuotaBoard quotas={quotas} />
    <div className="chip-scroll" role="group" aria-label="서비스 선택">{providers.map(provider => <ProviderChip key={provider.id} provider={provider} quota={quotas[provider.id]} active={provider.id === activeProviderId} onSelect={onProvider} />)}</div>
    <OAuthConnectCard provider={activeProvider} quota={activeQuota} compact onOpen={onOpenOAuth} />
    <section className="stream-feed"><div className="section-title"><div><span className="eyebrow">알림</span><h3>지금 확인할 항목</h3></div><button type="button">전체</button></div>
      <article className="feed-item priority"><span className="feed-line" style={providerStyle(claude.color)} /><ProviderLogo provider={claude} size="sm" /><div><span className="feed-time">현재 · CLAUDE</span><h4>{connectionLabel(claudeQuota)}</h4><p>{claudeQuota.statusMessage}</p></div><strong>{displayPercent(claudeQuota)}</strong></article>
      <article className="feed-item"><span className="feed-line" style={providerStyle(codex.color)} /><ProviderLogo provider={codex} size="sm" /><div><span className="feed-time">현재 · CODEX</span><h4>{connectionLabel(codexQuota)}</h4><p>{codexQuota.statusMessage}</p></div><strong>{displayPercent(codexQuota)}</strong></article>
    </section></> : <DesktopViewPanel view={view} activeProvider={activeProvider} activeProviderId={activeProviderId} activeQuota={activeQuota} quotas={quotas} onProvider={onProvider} onOpenOAuth={onOpenOAuth} {...actions} />}
    <MobileNav view={view} onView={onView} />
  </section></div></section></div>;
});

export function App() {
  const isMobile = useIsMobile();
  const [view, setView] = useState<ProductView>("overview");
  const [activeProviderId, setActiveProviderId] = useState<ProviderId>("codex");
  const [metric, setMetric] = useState<Metric>("remaining");
  const [range, setRange] = useState<UsageRange>("7D");
  const [solid, setSolid] = useState(false);
  const [theme, setTheme] = useState<ThemeMode>("dark");
  const [refreshedAt, setRefreshedAt] = useState("방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const [quotas, setQuotas] = useState<QuotaRecord>(initialQuotaRecord);
  const [oauthProviderId, setOauthProviderId] = useState<ProviderId>("codex");
  const [oauthOpen, setOauthOpen] = useState(false);
  const [actionFeedback, setActionFeedback] = useState<ProviderActionFeedback | null>(null);
  const [loginPollingProviderId, setLoginPollingProviderId] = useState<ProviderId | null>(null);
  const initialRefreshStarted = useRef(false);
  const activeProvider = useMemo(() => providers.find(provider => provider.id === activeProviderId) ?? providers[0], [activeProviderId]);
  const activeQuota = quotas[activeProviderId];
  const oauthProvider = useMemo(() => providers.find(provider => provider.id === oauthProviderId) ?? providers[0], [oauthProviderId]);
  const oauthQuota = quotas[oauthProviderId];

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const refreshProvider = useCallback(async (id: ProviderId) => {
    try {
      const snapshot = await getNativeProviderUsage(id);
      if (!snapshot) return null;
      setQuotas(current => ({ ...current, [id]: quotaFromSnapshot(snapshot, planQuotas[id]) }));
      return snapshot;
    } catch (error) {
      const message = error instanceof Error ? error.message : "공식 사용량을 불러오지 못했습니다.";
      setQuotas(current => {
        const previous = current[id];
        const hadVerifiedData = previous.confidence === "verified";
        return {
          ...current,
          [id]: {
            ...previous,
            connectionState: hadVerifiedData ? "stale" : "error",
            source: hadVerifiedData ? previous.source : "unavailable",
            confidence: hadVerifiedData ? "verified" : "unavailable",
            statusMessage: message
          }
        };
      });
      return null;
    }
  }, []);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      if (isTauriRuntime()) {
        await Promise.all(providers.map(provider => refreshProvider(provider.id)));
      } else {
        await new Promise(resolve => window.setTimeout(resolve, 420));
      }
      setRefreshedAt("지금");
    } finally {
      setRefreshing(false);
    }
  }, [refreshProvider]);

  useEffect(() => {
    if (!isTauriRuntime() || initialRefreshStarted.current) return;
    initialRefreshStarted.current = true;
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (!loginPollingProviderId || !isTauriRuntime()) return;
    let disposed = false;
    let attempts = 0;
    let pollTimer: number | null = null;
    const poll = async () => {
      if (disposed) return;
      attempts += 1;
      const snapshot = await refreshProvider(loginPollingProviderId);
      if (disposed) return;
      if (snapshot?.authState === "signed-in") {
        setLoginPollingProviderId(null);
        setActionFeedback({ status: "refreshed", message: snapshot.message });
      } else if (attempts >= 30) {
        setLoginPollingProviderId(null);
        setActionFeedback({ status: "not-available", message: "로그인 확인 시간이 초과되었습니다. 공급자 로그인을 마친 뒤 새로고침해 주세요." });
      } else {
        pollTimer = window.setTimeout(() => void poll(), 4_000);
      }
    };
    pollTimer = window.setTimeout(() => void poll(), 1_200);
    return () => {
      disposed = true;
      if (pollTimer !== null) window.clearTimeout(pollTimer);
    };
  }, [loginPollingProviderId, refreshProvider]);

  const openOAuth = useCallback((id: ProviderId) => {
    setOauthProviderId(id);
    setActionFeedback(null);
    setOauthOpen(true);
    if (isTauriRuntime()) void refreshProvider(id);
  }, [refreshProvider]);
  const closeOAuth = useCallback(() => {
    setOauthOpen(false);
    setActionFeedback(null);
    setLoginPollingProviderId(null);
  }, []);
  const connectOAuth = useCallback(async (id: ProviderId) => {
    if (!isTauriRuntime()) {
      setActionFeedback({ status: "demo-only", message: "브라우저 미리보기에서는 예시 수치만 표시합니다. 설치된 SPECTRA 앱에서 공식 계정을 연결해 주세요." });
      return;
    }
    const snapshot = await refreshProvider(id);
    if (!snapshot) {
      setActionFeedback({ status: "not-available", message: "공식 도구의 계정 상태를 확인하지 못했습니다. 잠시 뒤 다시 시도해 주세요." });
      return;
    }
    if (!snapshot.runtimeAvailable) {
      setActionFeedback({ status: "not-installed", message: snapshot.message });
      return;
    }
    if (snapshot.authState !== "signed-in") {
      const result = await startNativeProviderLogin(id);
      setActionFeedback(result ?? { status: "not-available", message: "로그인 프로세스를 시작하지 못했습니다." });
      if (result?.status === "login-started") setLoginPollingProviderId(id);
      return;
    }
    if (id === "claude" && !snapshot.bridgeInstalled) {
      const result = await installNativeProviderBridge(id);
      setActionFeedback(result ?? { status: "not-available", message: "Claude 사용량 브리지를 설치하지 못했습니다." });
      if (result?.status === "bridge-installed") void refreshProvider(id);
      return;
    }
    setActionFeedback({ status: "refreshed", message: snapshot.message });
  }, [refreshProvider]);
  const disconnectOAuth = useCallback(async (id: ProviderId) => {
    if (!isTauriRuntime() || id !== "claude") {
      setActionFeedback({ status: "not-supported", message: "SPECTRA는 공급자 로그인 자격 증명을 삭제하지 않습니다." });
      return;
    }
    const result = await removeNativeProviderBridge(id);
    setActionFeedback(result ?? { status: "not-available", message: "Claude 사용량 브리지를 제거하지 못했습니다." });
    if (result?.status === "bridge-removed") void refreshProvider(id);
  }, [refreshProvider]);

  const sharedProps: SharedViewProps = {
    view,
    onView: setView,
    activeProvider,
    activeProviderId,
    activeQuota,
    quotas,
    metric,
    range,
    onProvider: setActiveProviderId,
    onMetric: setMetric,
    onRange: setRange,
    onOpenOAuth: openOAuth,
    refreshedAt,
    refreshing,
    solid,
    theme,
    onRefresh: refresh,
    onSolid: () => setSolid(value => !value),
    onTheme: () => setTheme(value => value === "dark" ? "light" : "dark")
  };

  return <><div className={solid ? "solid-mode" : ""}>{isMobile ? <VariantCMobile {...sharedProps} /> : <VariantADesktop {...sharedProps} />}</div><OAuthDialog open={oauthOpen} provider={oauthProvider} quota={oauthQuota} startResult={actionFeedback} onClose={closeOAuth} onConnect={connectOAuth} onDisconnect={disconnectOAuth} /></>;
}
