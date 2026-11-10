import { memo, useCallback, useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { Icon, type IconName } from "./components/Icon";
import { historyForWindow, historySegments, modelGroups, rangeStart, selectedModelGroup, summaryWindow, windowAvailable, type ModelGroup } from "./data/focus-dock";
import { UpdateNotice, type UpdateState } from "./components/UpdateNotice";
import { createRefreshSequencer } from "./data/refresh-sequence";
import { LOOKUP_DELAYED, autoRefreshLabel, claudeFreshness, antigravityFreshness } from "./data/usage-freshness";
import { metricLabels, planQuotas, providers, rangeLabels, type AuthMethod, type Metric, type PlanQuota, type Provider, type ProviderId, type QuotaWindow, type QuotaWindowId, type UsageRange } from "./data/providers";
import { checkForAppUpdate, macUpdateMessage, type AvailableAppUpdate } from "./integrations/app-updater";
import { providersToRefreshAfterBoot, type NativePlatform } from "./integrations/boot-state";
import { nextConnectionAction } from "./integrations/provider-connect";
import { getNativeUsageHistory, setNativeWindowMode, setNativeWindowPinned, type UsageHistoryPoint, getNativeProviderUsage, installNativeProviderBridge, isTauriRuntime, listenNativeProviderUsage, readBootState, removeNativeProviderBridge, setNativeStandby, setNativeStrip, setNativeUiPrefs, startNativeProviderLogin, type NativeProviderActionResult, type NativeProviderUsageSnapshot, type TauriUnlisten } from "./integrations/tauri-native-bridge";

type WindowMode = "mini" | "dashboard" | "browser";

declare global {
  interface Window { __SPECTRA_MODE__?: "mini" | "dashboard" }
}

function useWindowMode(): WindowMode {
  const [mode, setMode] = useState<WindowMode>(() => window.__SPECTRA_MODE__ ?? (isTauriRuntime() ? "mini" : "browser"));
  useEffect(() => {
    const current = window.__SPECTRA_MODE__;
    if (current) setMode(current);
    const onMode = (event: Event) => {
      const detail = (event as CustomEvent<string>).detail;
      if (detail === "mini" || detail === "dashboard") setMode(detail);
    };
    window.addEventListener("spectra-mode", onMode);
    return () => window.removeEventListener("spectra-mode", onMode);
  }, []);
  return mode;
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
  if (epochSeconds === null) return null;
  const date = new Date(epochSeconds * 1000);
  if (!Number.isFinite(date.getTime())) return null;
  return new Intl.DateTimeFormat("ko-KR", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit", hour12: false }).format(date);
}

function planName(providerId: ProviderId, planType: string | null) {
  if (providerId === "antigravity") return planType || "Antigravity 구독";
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

// 실제 값이 오기 전 자리표시. 계정에 없을 수 있는 모델군(예시의 Fable·Antigravity 모델군)은 만들지 않는다.
const pendingWindows = (fallback: PlanQuota): QuotaWindow[] => fallback.windows.filter(window => !window.modelGroupId)
  .map(window => ({ ...window, usedPercent: 0, remainingPercent: 0, resetLabel: "연결 후 표시", kindLabel: "실제 데이터 대기" }));

function quotaFromSnapshot(snapshot: NativeProviderUsageSnapshot, fallback: PlanQuota): PlanQuota {
  const connectionState: PlanQuota["connectionState"] = snapshot.connectionState === "not-installed"
    ? "not_installed"
    : snapshot.connectionState === "signed-out"
      ? "signed_out"
      : snapshot.connectionState === "waiting-for-usage"
        ? "waiting"
        : snapshot.connectionState;
  const verified = snapshot.windows.length > 0 && (snapshot.source === "codex-app-server" || snapshot.source === "claude-statusline" || snapshot.source === "claude-usage-api" || snapshot.source === "antigravity-statusline");
  const windows = snapshot.windows.length > 0
    ? snapshot.windows.map(window => ({
        id: window.id,
        modelGroupId: window.modelGroupId,
        modelGroupLabel: window.modelGroupLabel,
        label: window.label,
        usedPercent: window.usedPercent,
        remainingPercent: window.remainingPercent,
        resetLabel: formatReset(window.resetsAt),
        kindLabel: "구독 한도",
        resetsAt: window.resetsAt ? window.resetsAt * 1000 : null,
        windowDurationMins: window.windowDurationMins
      }))
    : pendingWindows(fallback);
  return {
    ...fallback,
    planName: planName(snapshot.providerId, snapshot.planType),
    accountLabel: snapshot.authState === "signed-in" ? "공식 계정 연결 확인" : snapshot.runtimeAvailable ? "계정 연결 필요" : "공식 CLI 설치 필요",
    authMethod: "provider-delegated",
    connectionState,
    source: verified ? snapshot.source! : "unavailable",
    confidence: verified ? "verified" : "unavailable",
    lastSyncedAt: formatSyncedAt(snapshot.lastSyncedAt),
    lastSyncedAtMs: snapshot.lastSyncedAt === null ? null : snapshot.lastSyncedAt * 1000,
    bridgeInstalled: snapshot.bridgeInstalled,
    statusMessage: snapshot.message,
    liveFailure: snapshot.liveFailure ?? null,
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
    lastSyncedAtMs: null,
    bridgeInstalled: false,
    statusMessage: "연결 상태를 확인하고 있습니다.",
    windows: pendingWindows(fallback)
  };
}

function initialQuotaRecord(snapshots: readonly NativeProviderUsageSnapshot[] = []): QuotaRecord {
  if (!isTauriRuntime()) return planQuotas;
  const restored = new Map(snapshots.map(snapshot => [snapshot.providerId, snapshot] as const));
  return Object.fromEntries(providers.map(provider => {
    const snapshot = restored.get(provider.id);
    return [provider.id, snapshot ? quotaFromSnapshot(snapshot, planQuotas[provider.id]) : nativePendingQuota(planQuotas[provider.id])];
  })) as QuotaRecord;
}

function connectionLabel(quota: PlanQuota) {
  if (quota.providerId === "antigravity") return antigravityFreshness(quota, Date.now());
  if (quota.providerId === "claude" && quota.source !== "example") return claudeFreshness(quota, Date.now()).label;
  if (quota.connectionState === "not_installed") return "공식 CLI 설치 필요";
  if (quota.connectionState === "signed_out") return "계정 로그인 필요";
  if (quota.connectionState === "waiting") return quota.bridgeInstalled ? "로그인됨 · 첫 사용량 대기" : "로그인됨 · 동기화 설정 필요";
  if (quota.connectionState === "stale") return "연결됨 · 데이터 갱신 필요";
  if (quota.connectionState === "unsupported") return "OAuth 지원 확인 필요";
  if (quota.connectionState === "error") return LOOKUP_DELAYED;
  if (quota.connectionState === "connected") return "공식 요금제 사용량 연결됨";
  return "공식 계정 연결 필요";
}

function sourceLabel(quota: PlanQuota) {
  if (quota.source === "antigravity-statusline") return "Antigravity CLI 상태선 · 실행 시 갱신";
  if (quota.source === "codex-app-server") return "Codex 공식 App Server";
  if (quota.source === "claude-statusline") return "Claude Code 공식 상태선";
  if (quota.source === "claude-usage-api") return "Claude 공식 사용량 조회";
  if (quota.source === "unavailable") return "실제 사용량 데이터 대기";
  return "브라우저 데모 수치";
}

const Brand = memo(function Brand() {
  return <div className="brand-lockup"><span className="brand-mark" aria-hidden="true"><i /><i /><i /></span><strong>SPECTRA</strong></div>;
});

const ProviderLogo = memo(function ProviderLogo({ provider }: Readonly<{ provider: Provider; size?: string }>) {
  return <span className="provider-dot" style={providerStyle(provider.color)} aria-hidden="true" />;
});

type LayoutActions = Readonly<{
  platform: NativePlatform;
  refreshedAt: string; refreshing: boolean; solid: boolean; theme: ThemeMode;
  standby: boolean; strip: boolean; stripPending: boolean;
  onRefresh: () => void; onSolid: () => void; onTheme: () => void;
  onStandby: () => void; onStrip: () => void;
  updateState: UpdateState; onCheckForUpdate: () => void; onInstallUpdate: () => void;
}>;
type SharedViewProps = LayoutActions & Readonly<{
  view: ProductView; onView: (view: ProductView) => void;
  activeProvider: Provider; activeProviderId: ProviderId; activeQuota: PlanQuota; quotas: QuotaRecord;
  metric: Metric; range: UsageRange; onProvider: (id: ProviderId) => void;
  onMetric: (metric: Metric) => void; onRange: (range: UsageRange) => void;
  onOpenOAuth: (id: ProviderId) => void;
  group: ModelGroup | undefined; onGroup: (id: string) => void;
  quotaWindow: QuotaWindow | undefined; onWindow: (id: QuotaWindowId) => void;
  history: readonly UsageHistoryPoint[]; historyState: "loading" | "ready" | "error";
  now: number; pinned: boolean; onPin: () => void;
  onOpenDashboard: (view?: ProductView) => void; onOpenMini: () => void;
  uiError: string | null;
}>;

const ThemeButton = memo(function ThemeButton({ theme, onTheme }: Pick<LayoutActions, "theme" | "onTheme">) {
  return <button type="button" className="icon-button theme-button" onClick={onTheme} aria-label={theme === "dark" ? "라이트 모드로 전환" : "다크 모드로 전환"} title={theme === "dark" ? "라이트 모드로 전환" : "다크 모드로 전환"}><Icon name={theme === "dark" ? "sun" : "moon"} size={17} /><span>테마</span></button>;
});

const MetricTabs = memo(function MetricTabs({ metric, onMetric }: Pick<SharedViewProps, "metric" | "onMetric">) {
  return <div className="segmented" role="tablist" aria-label="사용 현황 보기">{(Object.keys(metricLabels) as Metric[]).map(value => <button type="button" role="tab" aria-selected={metric === value} className={metric === value ? "active" : ""} onClick={() => onMetric(value)} key={value}>{metricLabels[value]}</button>)}</div>;
});

const RangeTabs = memo(function RangeTabs({ range, onRange }: Pick<SharedViewProps, "range" | "onRange">) {
  return <div className="range-tabs" role="tablist" aria-label="조회 기간">{(Object.keys(rangeLabels) as UsageRange[]).map(value => <button type="button" role="tab" aria-selected={range === value} className={range === value ? "active" : ""} onClick={() => onRange(value)} key={value}>{rangeLabels[value]}</button>)}</div>;
});

function percentFor(quota: PlanQuota, window: QuotaWindow | undefined, now: number) {
  return windowAvailable(quota, window, now) ? `${Math.round(window!.remainingPercent)}%` : "—";
}

const ProviderTabs = memo(function ProviderTabs({ activeProviderId, onProvider }: Pick<SharedViewProps, "activeProviderId" | "onProvider">) {
  return <div className="provider-tabs" role="tablist" aria-label="서비스 선택">{providers.map(provider => <button key={provider.id} type="button" role="tab" aria-selected={provider.id === activeProviderId} className={provider.id === activeProviderId ? "active" : ""} style={providerStyle(provider.color)} onClick={() => onProvider(provider.id)}>{provider.name}</button>)}</div>;
});

const ModelSelector = memo(function ModelSelector({ activeQuota, group, onGroup, quotaWindow, onWindow, compact = false }: Pick<SharedViewProps, "activeQuota" | "group" | "onGroup" | "quotaWindow" | "onWindow"> & Readonly<{ compact?: boolean }>) {
  const groups = modelGroups(activeQuota);
  return <div className={`model-controls ${compact ? "compact" : ""}`}>
    {compact ? <label className="model-select"><span>{activeQuota.planName}</span><select aria-label="모델군 선택" disabled={!groups.length} value={group?.id ?? ""} onChange={event => onGroup(event.target.value)}>{!groups.length ? <option value="">모델군 확인 대기</option> : null}{groups.map(item => <option value={item.id} key={item.id}>{item.label}</option>)}</select></label>
      : groups.length > 1 ? <div className="model-tabs" role="tablist" aria-label="모델군 선택">{groups.map(item => <button type="button" role="tab" aria-selected={group?.id === item.id} className={group?.id === item.id ? "active" : ""} key={item.id} onClick={() => onGroup(item.id)}>{item.label}</button>)}</div> : null}
    {(group?.windows.length ?? 0) > 1 ? <div className="window-tabs" role="tablist" aria-label="한도 창 선택">{group!.windows.map(window => <button type="button" role="tab" aria-selected={quotaWindow?.id === window.id} className={quotaWindow?.id === window.id ? "active" : ""} key={window.id} onClick={() => onWindow(window.id)}>{window.label}</button>)}</div> : null}
  </div>;
});

const QuotaHero = memo(function QuotaHero({ activeQuota, group, quotaWindow, now, compact = false }: Pick<SharedViewProps, "activeQuota" | "group" | "quotaWindow" | "now"> & Readonly<{ compact?: boolean }>) {
  const available = windowAvailable(activeQuota, quotaWindow, now);
  const percent = available ? Math.round(quotaWindow!.remainingPercent) : null;
  return <section className={`quota-hero ${compact ? "compact" : ""}`} aria-label="선택한 모델군 잔여량">
    {!compact ? <h2>{group?.label ?? activeQuota.planName} · {quotaWindow?.label ?? "한도"}</h2> : null}
    <div className="hero-reading"><div className="hero-value">{percent ?? "—"}{percent !== null ? <span>%</span> : null}</div><span>{quotaWindow?.label ?? "한도"} {available ? "남음" : "확인 대기"}</span></div>
    <div className="quota-meter" role="meter" aria-label={quotaWindow?.label ?? "잔여량"} aria-valuemin={0} aria-valuemax={100} aria-valuenow={percent ?? undefined} aria-valuetext={percent === null ? "확인된 데이터 없음" : `${percent}% 남음`}><i style={{ width: `${percent ?? 0}%` }} /></div>
    <div className="hero-reset"><Icon name="refresh" size={15} /><span>{available ? quotaWindow?.resetsAt ? `${formatReset(quotaWindow.resetsAt / 1000)} 초기화` : quotaWindow?.resetLabel : quotaWindow?.resetsAt && quotaWindow.resetsAt <= now ? "초기화 후 새 데이터 대기" : activeQuota.statusMessage}</span></div>
    {activeQuota.connectionState === "stale" || activeQuota.connectionState === "error" || (activeQuota.providerId === "antigravity" && activeQuota.confidence === "verified" && antigravityFreshness(activeQuota, now) === "CLI 실행 후 갱신 필요") ? <p className="freshness-note">{connectionLabel(activeQuota)} · 마지막 수집값입니다.</p> : null}
  </section>;
});

const OtherLimits = memo(function OtherLimits({ activeQuota, group, quotaWindow, now, compact = false }: Pick<SharedViewProps, "activeQuota" | "group" | "quotaWindow" | "now"> & Readonly<{ compact?: boolean }>) {
  const groups = modelGroups(activeQuota);
  const otherGroups = groups.filter(item => item.id !== group?.id);
  const otherWindows = group?.windows.filter(window => window.id !== quotaWindow?.id) ?? [];
  if (!otherGroups.length && !otherWindows.length) return null;
  return <section className={`other-limits ${compact ? "compact" : ""}`}><h2>{otherGroups.length ? "다른 모델군" : "다른 한도"}</h2>
    {otherGroups.map(item => <div className="other-limit-row" key={item.id}><strong>{item.label}</strong><div>{item.windows.map(window => <span key={window.id}>{window.label.replace(" 한도", "")} <b>{percentFor(activeQuota, window, now)}</b></span>)}</div></div>)}
    {otherWindows.map(window => <div className="other-limit-row" key={window.id}><strong>{window.label}</strong><div><b>{percentFor(activeQuota, window, now)}</b><span>{window.resetsAt ? formatReset(window.resetsAt / 1000) : window.resetLabel}</span></div></div>)}
  </section>;
});

const HistoryPanel = memo(function HistoryPanel({ history, historyState, activeProvider, group, quotaWindow, range, now }: Pick<SharedViewProps, "history" | "historyState" | "activeProvider" | "group" | "quotaWindow" | "range" | "now">) {
  const points = historyForWindow(history, activeProvider.id, group?.id ?? "default", quotaWindow?.id ?? "weekly", range, now);
  const start = rangeStart(range, now);
  const end = Math.max(start + 1, now);
  const dateLabel = (ms: number) => new Intl.DateTimeFormat("ko-KR", { month: "2-digit", day: "2-digit" }).format(ms);
  const x = (point: UsageHistoryPoint) => 44 + (point.capturedAt * 1000 - start) / (end - start) * 620;
  const y = (point: UsageHistoryPoint) => 150 - point.remainingPercent * 1.2;
  const stride = Math.max(1, Math.ceil(points.length / 300));
  const sampled = points.filter((_, index) => index % stride === 0 || index === points.length - 1);
  return <section className="history-panel" aria-label={`${rangeLabels[range]} 잔여량 기록`}>
    <div className="section-heading"><div><h2>{range === "24H" ? "오늘" : `최근 ${rangeLabels[range]}`} 잔여량</h2><p>{activeProvider.name} · {group?.label ?? "모델군"} · {quotaWindow?.label ?? "한도"}</p></div><span className="subtle-badge">{points.length}회 기록</span></div>
    <div className="history-period">{dateLabel(start)} — {dateLabel(now)}</div>
    {points.length ? <><svg className="history-chart" viewBox="0 0 700 184" role="img" aria-label={`${dateLabel(start)}부터 ${dateLabel(now)}까지 ${points.length}회 수집, 최신 잔여량 ${Math.round(points[points.length - 1].remainingPercent)}%`}>
      {[0, 25, 50, 75, 100].map(percent => <g key={percent}><text x="0" y={154 - percent * 1.2}>{percent}%</text><line x1="44" x2="664" y1={150 - percent * 1.2} y2={150 - percent * 1.2} /></g>)}
      {historySegments(sampled).map((segment, index) => segment.length > 1 ? <polyline className="history-line" key={index} points={segment.map(point => `${x(point)},${y(point)}`).join(" ")} /> : null)}
      {sampled.map((point, index) => <circle cx={x(point)} cy={y(point)} r="3.5" key={`${point.capturedAt}-${index}`} />)}
      <text x="44" y="179">{dateLabel(start)}</text><text x="664" y="179" textAnchor="end">{dateLabel(now)}</text>
    </svg><details className="history-details"><summary>최근 수집 기록 보기</summary><table><thead><tr><th>수집 시각</th><th>잔여량</th></tr></thead><tbody>{points.slice(-8).reverse().map((point, index) => <tr key={`${point.capturedAt}-${index}`}><td>{formatSyncedAt(point.capturedAt)}</td><td>{Math.round(point.remainingPercent)}%</td></tr>)}</tbody></table></details></>
      : <div className="empty-state"><Icon name="pulse" size={24} /><strong>{historyState === "loading" ? "기록을 불러오는 중입니다" : historyState === "error" ? "기록을 불러오지 못했습니다" : "이 기간에 수집된 기록이 없습니다"}</strong><p>{historyState === "error" ? "새로고침을 눌러 다시 확인해 주세요." : "실제 수집 시점부터 기록됩니다. 이전 기록은 없습니다."}</p></div>}
    <p className="history-note">기기 내 수집 기록입니다. 빈 구간은 미수집, 값의 상승은 초기화일 수 있습니다.</p>
  </section>;
});

const SubscriptionPanel = memo(function SubscriptionPanel({ activeProvider, activeQuota, now, onOpenOAuth }: Pick<SharedViewProps, "activeProvider" | "activeQuota" | "now" | "onOpenOAuth">) {
  const groups = modelGroups(activeQuota);
  return <section className="subscription-panel"><div className="section-heading"><div><span className="eyebrow">연결된 구독 모델</span><h2>{activeQuota.planName}</h2><p>{activeQuota.accountLabel}</p></div><button type="button" className="secondary-action" onClick={() => onOpenOAuth(activeProvider.id)}>연결 관리</button></div>
    <dl className="subscription-meta"><div><dt>서비스</dt><dd>{activeProvider.name}</dd></div><div><dt>데이터 출처</dt><dd>{sourceLabel(activeQuota)}</dd></div><div><dt>마지막 수집</dt><dd>{activeQuota.lastSyncedAt ?? "아직 수집되지 않음"}</dd></div><div><dt>연결 상태</dt><dd>{connectionLabel(activeQuota)}</dd></div></dl>
    <div className="subscription-groups">{groups.map(item => <section key={item.id}><h3>{item.label}</h3>{item.windows.map(window => <div className="subscription-window" key={window.id}><span>{window.label}</span><strong>{percentFor(activeQuota, window, now)}</strong><small>{window.resetsAt ? `${formatReset(window.resetsAt / 1000)} 초기화` : window.resetLabel}</small></div>)}</section>)}</div>
    {activeProvider.id === "antigravity" ? <p className="view-description">Claude 직접 구독과 별도 한도입니다. Antigravity CLI 실행 시 갱신됩니다.</p> : null}
  </section>;
});

const SettingsPanel = memo(function SettingsPanel(props: SharedViewProps) {
  const busy = props.updateState.phase === "checking" || props.updateState.phase === "installing";
  const updateAvailable = props.updateState.phase === "available";
  const isMac = props.platform === "macos";
  const rows = [
    { title: "화면 테마", description: "모든 창에 적용하며 재실행 후 유지합니다.", label: props.theme === "dark" ? "라이트 모드" : "다크 모드", action: props.onTheme },
    { title: "가독성용 불투명 모드", description: "배경의 투명도를 조절합니다.", label: props.solid ? "유리 모드" : "불투명 모드", action: props.onSolid },
    { title: "메모리 절약 대기", description: props.standby ? "창을 닫으면 WebView를 종료합니다." : "창을 숨겨 다음에 빠르게 엽니다.", label: props.standby ? "빠른 재표시" : "메모리 절약", action: props.onStandby },
    { title: isMac ? "메뉴 막대 잔여량" : "작업표시줄 표시", description: isMac ? "상단 메뉴 막대에 잔여량을 표시합니다. 꺼도 앱 아이콘은 유지됩니다." : "작업표시줄에 확인된 잔여량을 표시합니다.", label: props.strip ? "표시 끄기" : "표시 켜기", action: props.onStrip, disabled: props.stripPending },
    { title: "사용량 새로고침", description: "공급자 도구에서 확인 가능한 최신 수치를 조회합니다.", label: props.refreshing ? "확인 중" : "지금 확인", action: props.onRefresh, disabled: props.refreshing },
    { title: "SPECTRA 업데이트", description: props.updateState.message, label: isMac ? "수동 업데이트" : updateAvailable ? "지금 설치" : busy ? "확인 중" : "업데이트 확인", action: updateAvailable ? props.onInstallUpdate : props.onCheckForUpdate, disabled: busy || isMac }
  ];
  return <section className="settings-panel"><h2>사용 환경</h2>{rows.map(row => <div className="settings-row" key={row.title}><div><strong>{row.title}</strong><p>{row.description}</p></div><button type="button" className="secondary-action" onClick={row.action} disabled={row.disabled}>{row.label}</button></div>)}<p className="settings-note"><Icon name="shield" size={16} />계정·대화 원문 없이 수치만 기기에 기록합니다.</p></section>;
});

const Sidebar = memo(function Sidebar({ view, onView, activeProviderId, onProvider, quotas, now }: Pick<SharedViewProps, "view" | "onView" | "activeProviderId" | "onProvider" | "quotas" | "now">) {
  const nav: readonly [ProductView, IconName, string][] = [["overview", "grid", "개요"], ["services", "list", "구독 모델"], ["trend", "pulse", "추이"], ["alerts", "bell", "알림"]];
  return <aside className="focus-sidebar" aria-label="주요 메뉴"><Brand /><nav>{nav.map(([id, icon, label]) => <button type="button" key={id} aria-label={label} className={view === id ? "active" : ""} aria-current={view === id ? "page" : undefined} onClick={() => onView(id)}><Icon name={icon} /><span>{label}</span></button>)}</nav><div className="sidebar-services"><span className="eyebrow">서비스</span>{providers.map(provider => <button type="button" key={provider.id} aria-label={provider.name} style={providerStyle(provider.color)} className={provider.id === activeProviderId ? "active" : ""} aria-pressed={provider.id === activeProviderId} onClick={() => onProvider(provider.id)}><ProviderLogo provider={provider} /><span>{provider.name}</span><strong>{percentFor(quotas[provider.id], summaryWindow(quotas[provider.id], now), now)}</strong></button>)}</div><button type="button" className={`sidebar-settings ${view === "settings" ? "active" : ""}`} aria-label="설정" aria-current={view === "settings" ? "page" : undefined} onClick={() => onView("settings")}><Icon name="settings" /><span>설정</span></button></aside>;
});

const FocusDashboard = memo(function FocusDashboard(props: SharedViewProps) {
  const { activeProvider, activeQuota, view, metric } = props;
  const title = view === "settings" ? "설정" : view === "alerts" ? "알림" : activeProvider.name;
  const remainingView = view === "overview" && metric === "remaining";
  return <div className="focus-dashboard" style={providerStyle(activeProvider.color)}><Sidebar {...props} /><section className="focus-main"><header className="focus-header"><div><span className="eyebrow">{view === "settings" ? "사용 환경" : activeQuota.source === "example" ? "브라우저 데모 · 예시 데이터" : "AI 사용 현황"}</span><h1>{title}</h1><p>{view === "settings" ? "미니창과 대시보드의 사용 환경을 조정합니다." : activeQuota.planName}</p></div><div className="top-actions"><button type="button" className="icon-button" onClick={props.onRefresh} disabled={props.refreshing} aria-label="데이터 새로고침"><Icon name="refresh" size={18} /></button><ThemeButton {...props} /><button type="button" className="icon-button" onClick={props.onOpenMini} aria-label="미니창 열기" title="미니창 열기"><Icon name="phone" size={18} /></button></div></header>
    {props.uiError ? <p className="operation-error" role="alert">{props.uiError}</p> : null}
    <div className="dashboard-content">
      {view === "settings" ? <SettingsPanel {...props} /> : view === "alerts" ? <section className="alerts-panel"><h2>연결 및 수집 상태</h2>{providers.map(provider => <button type="button" className="alert-row" key={provider.id} onClick={() => props.onOpenOAuth(provider.id)} style={providerStyle(provider.color)}><ProviderLogo provider={provider} /><div><strong>{provider.name} · {connectionLabel(props.quotas[provider.id])}</strong><p>{props.quotas[provider.id].statusMessage}</p></div><Icon name="chevron" size={18} /></button>)}</section> : <>
        <div className="dashboard-toolbar">{view === "overview" ? <MetricTabs {...props} /> : <h2>{view === "trend" ? "수집 기록" : "구독 모델"}</h2>}{view === "trend" || remainingView ? <RangeTabs {...props} /> : null}</div>
        {view === "services" || (view === "overview" && metric === "subscription") ? <SubscriptionPanel {...props} /> : view === "overview" && metric === "requests" ? <section className="requests-panel"><span className="eyebrow">{activeProvider.name}</span><h2>요청 수</h2><div className="empty-state"><Icon name="list" size={26} /><strong>이 연결에서는 요청 수를 제공하지 않습니다</strong><p>구독 한도만 제공됩니다. 요청 수는 집계하지 않습니다.</p></div><button type="button" className="secondary-action" onClick={() => props.onMetric("subscription")}>구독 모델 확인</button></section> : <>
          <ModelSelector {...props} />
          {remainingView ? <div className="focus-quota-row"><QuotaHero {...props} /><OtherLimits {...props} /></div> : null}
          <HistoryPanel {...props} />
        </>}
      </>}
    </div>
    <footer className="dashboard-footer"><span className={`status-dot ${activeQuota.connectionState === "connected" ? "connected" : ""}`} /><span>{activeQuota.source === "example" ? "예시 데이터" : connectionLabel(activeQuota)}</span><span className="footer-source">{sourceLabel(activeQuota)}</span><button type="button" onClick={() => props.onOpenOAuth(activeProvider.id)}>연결 관리 <Icon name="chevron" size={13} /></button></footer>
  </section></div>;
});

const MiniLayout = memo(function MiniLayout(props: SharedViewProps) {
  const { activeProvider, activeQuota, quotas, now } = props;
  return <div className="focus-mini" style={providerStyle(activeProvider.color)}><header className="mini-header"><Brand /><div className="mini-actions"><button type="button" className="icon-button" aria-label="데이터 새로고침" onClick={props.onRefresh} disabled={props.refreshing}><Icon name="refresh" size={17} /></button><button type="button" className={`icon-button ${props.pinned ? "pressed" : ""}`} aria-label="항상 위에 표시" aria-pressed={props.pinned} onClick={props.onPin}><Icon name="pin" size={17} /></button><ThemeButton {...props} /><button type="button" className="icon-button" aria-label="설정" onClick={() => props.onOpenDashboard("settings")}><Icon name="settings" size={17} /></button></div></header>
    <div className="mini-content"><ProviderTabs {...props} />{props.uiError ? <p className="operation-error" role="alert">{props.uiError}</p> : null}<ModelSelector {...props} compact /><QuotaHero {...props} compact /><OtherLimits {...props} compact />
      {activeQuota.confidence === "unavailable" ? <button type="button" className="secondary-action mini-connect" onClick={() => props.onOpenOAuth(activeProvider.id)}>연결 관리</button> : null}
      <div className="mini-summaries">{providers.filter(provider => provider.id !== activeProvider.id).map(provider => { const quota = quotas[provider.id]; const window = summaryWindow(quota, now); return <button type="button" key={provider.id} onClick={() => props.onProvider(provider.id)} style={providerStyle(provider.color)}><div><ProviderLogo provider={provider} /><strong>{provider.name}</strong></div><span>{window?.modelGroupLabel ? `${window.modelGroupLabel} · ` : ""}{window?.label ?? "확인 대기"}</span><b>{percentFor(quota, window, now)}</b></button>; })}</div>
    </div><footer className="mini-footer"><div><span className={`status-dot ${activeQuota.connectionState === "connected" ? "connected" : ""}`} /><span title={activeQuota.lastSyncedAt ?? undefined}>{activeQuota.source === "example" ? "예시 데이터" : connectionLabel(activeQuota)}</span></div><button type="button" className="primary-action" onClick={() => props.onOpenDashboard("overview")}><Icon name="monitor" size={16} />대시보드 열기</button></footer></div>;
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
  const loginPending = startResult?.status === "login-started";
  const runtimeMissing = quota.connectionState === "not_installed";
  const needsLogin = quota.connectionState === "signed_out";
  const needsBridge = !quota.bridgeInstalled && (provider.id === "antigravity" || (provider.id === "claude" && quota.connectionState === "waiting"));
  const primaryLabel = runtimeMissing
    ? `${provider.name} CLI 설치 필요`
    : needsBridge ? "사용량 브리지 설치" : needsLogin ? `${provider.name} 계정 로그인` : "사용량 새로고침";
  return <div className="oauth-backdrop" role="presentation" onMouseDown={event => { if (event.target === event.currentTarget) onClose(); }}><div ref={dialogRef} className="oauth-dialog" role="dialog" aria-modal="true" aria-labelledby="oauth-dialog-title" aria-describedby="oauth-dialog-description" tabIndex={-1}>
    <div className="oauth-dialog-header"><div><span className="eyebrow">공식 계정 연결</span><h2 id="oauth-dialog-title">{provider.name} 계정 및 사용량</h2></div><button ref={closeButtonRef} type="button" className="icon-button oauth-close" onClick={onClose} aria-label="연결 창 닫기"><Icon name="x" size={17} /></button></div>
    <div className="oauth-provider"><ProviderLogo provider={provider} size="lg" /><div><strong>{quota.planName}</strong><span>공급자 공식 로그인</span></div></div>
    <div className="oauth-capability-note"><span>조회 출처</span><strong>{sourceLabel(quota)}</strong><span>연결 상태</span><strong>{connectionLabel(quota)}</strong></div>
    <p className="view-description">공급자가 로그인을 관리하며 SPECTRA는 한도만 읽습니다.{provider.id === "antigravity" ? " 브리지 설치 후 Antigravity CLI를 실행하면 갱신됩니다." : ""}</p>
    <div id="oauth-dialog-description" className="oauth-demo-note"><Icon name="shield" size={14} /><span>{startResult?.message ?? quota.statusMessage}</span></div>
    <div className="oauth-dialog-actions"><button type="button" className="secondary-action" onClick={onClose}>닫기</button>{(provider.id === "claude" || provider.id === "antigravity") && quota.bridgeInstalled ? <button type="button" className="danger-action" onClick={() => onDisconnect(provider.id)}>브리지 제거</button> : null}<button type="button" className="primary-action" disabled={unsupported || runtimeMissing || loginPending} onClick={() => onConnect(provider.id)}>{loginPending ? "브라우저 로그인 진행 중" : primaryLabel}</button></div>
  </div></div>;
});

export function App() {
  const boot = useRef(readBootState()).current;
  const platform = boot?.platform ?? "unknown";
  const windowMode = useWindowMode();
  const [view, setView] = useState<ProductView>("overview");
  const [selectedGroups, setSelectedGroups] = useState<Partial<Record<ProviderId, string>>>({});
  const [selectedWindowId, setSelectedWindowId] = useState<QuotaWindowId>("weekly");
  const [history, setHistory] = useState<readonly UsageHistoryPoint[]>([]);
  const [historyState, setHistoryState] = useState<"loading" | "ready" | "error">(isTauriRuntime() ? "loading" : "ready");
  const historyRequest = useRef(0);
  const [now, setNow] = useState(Date.now);
  const [pinned, setPinned] = useState(windowMode === "mini");
  const pinPending = useRef(false);
  const [uiError, setUiError] = useState<string | null>(null);
  const loadHistory = useCallback(async () => {
    const ticket = ++historyRequest.current;
    try {
      const points = await getNativeUsageHistory();
      if (ticket !== historyRequest.current) return;
      setHistory(points);
      setHistoryState("ready");
    } catch { if (ticket === historyRequest.current) setHistoryState("error"); }
  }, []);
  useEffect(() => { void loadHistory(); }, [loadHistory]);
  useEffect(() => { setPinned(windowMode === "mini"); }, [windowMode]);
  useEffect(() => {
    const syncPin = (event: Event) => setPinned((event as CustomEvent<string>).detail === "mini");
    window.addEventListener("spectra-mode", syncPin);
    return () => window.removeEventListener("spectra-mode", syncPin);
  }, []);
  useEffect(() => {
    let timer: number;
    const tick = () => { setNow(Date.now()); timer = window.setTimeout(tick, 30_000); };
    timer = window.setTimeout(tick, 30_000);
    return () => window.clearTimeout(timer);
  }, []);
  const changeWindowMode = useCallback(async (mode: "mini" | "dashboard", targetView?: ProductView) => {
    if (targetView) setView(targetView);
    try { await setNativeWindowMode(mode); setUiError(null); }
    catch { setUiError("창을 전환하지 못했습니다. 다시 시도해 주세요."); }
  }, []);
  const togglePin = useCallback(async () => {
    if (pinPending.current) return;
    pinPending.current = true;
    try { setPinned(await setNativeWindowPinned(!pinned)); setUiError(null); }
    catch { setUiError("항상 위 표시를 변경하지 못했습니다."); }
    finally { pinPending.current = false; }
  }, [pinned]);
  const [activeProviderId, setActiveProviderId] = useState<ProviderId>("codex");
  const [metric, setMetric] = useState<Metric>("remaining");
  const [range, setRange] = useState<UsageRange>("7D");
  const [solid, setSolid] = useState(boot?.solid ?? false);
  const [theme, setTheme] = useState<ThemeMode>(boot?.theme ?? "dark");
  const [standby, setStandby] = useState(boot?.standby ?? false);
  const [strip, setStrip] = useState(boot?.strip ?? false);
  const [stripPending, setStripPending] = useState(false);
  const stripToggleInFlight = useRef(false);
  const [refreshedAt, setRefreshedAt] = useState(boot && boot.snapshots.length > 0 ? "이전 표시 복원" : "방금 전");
  const [refreshing, setRefreshing] = useState(false);
  const [quotas, setQuotas] = useState<QuotaRecord>(() => initialQuotaRecord(boot?.snapshots ?? []));
  const [oauthProviderId, setOauthProviderId] = useState<ProviderId>("codex");
  const [oauthOpen, setOauthOpen] = useState(false);
  const [actionFeedback, setActionFeedback] = useState<ProviderActionFeedback | null>(null);
  const [loginPollingProviderId, setLoginPollingProviderId] = useState<ProviderId | null>(null);
  const isMac = platform === "macos";
  const [updateState, setUpdateState] = useState<UpdateState>({ phase: isMac ? "unsupported" : "idle", version: null, message: isMac ? macUpdateMessage : "업데이트 확인 전" });
  const [updateNoticeVisible, setUpdateNoticeVisible] = useState(false);
  const pendingUpdate = useRef<AvailableAppUpdate | null>(null);
  const updateOperationInFlight = useRef(false);
  const initialRefreshStarted = useRef(false);
  const refreshSequence = useRef(createRefreshSequencer<ProviderId>()).current;
  const activeProvider = useMemo(() => providers.find(provider => provider.id === activeProviderId) ?? providers[0], [activeProviderId]);
  const activeQuota = quotas[activeProviderId];
  const group = selectedModelGroup(activeQuota, selectedGroups[activeProviderId]);
  const quotaWindow = group?.windows.find(window => window.id === selectedWindowId) ?? group?.windows[0];
  const oauthProvider = useMemo(() => providers.find(provider => provider.id === oauthProviderId) ?? providers[0], [oauthProviderId]);
  const oauthQuota = quotas[oauthProviderId];

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  const prefsSyncArmed = useRef(false);
  useEffect(() => {
    if (!isTauriRuntime()) return;
    if (!prefsSyncArmed.current) {
      prefsSyncArmed.current = true;
      return;
    }
    void setNativeUiPrefs({ theme, solid }).catch(() => setUiError("설정 저장 실패. 이번 실행에만 적용됩니다."));
  }, [theme, solid]);

  const toggleStandby = useCallback(async () => {
    const next = !standby;
    setStandby(next);
    if (!isTauriRuntime()) return;
    try {
      const saved = await setNativeStandby(next);
      if (saved) setStandby(saved.standby);
    } catch {
      setStandby(!next);
    }
  }, [standby]);

  const toggleStrip = useCallback(async () => {
    if (stripToggleInFlight.current) return;
    const next = !strip;
    setStrip(next);
    if (!isTauriRuntime()) return;
    stripToggleInFlight.current = true;
    setStripPending(true);
    try {
      const saved = await setNativeStrip(next);
      setStrip(saved?.strip ?? !next);
    } catch {
      setStrip(!next);
    } finally {
      stripToggleInFlight.current = false;
      setStripPending(false);
    }
  }, [strip]);

  const refreshProvider = useCallback(async (id: ProviderId) => {
    const ticket = refreshSequence.begin(id);
    try {
      const snapshot = await getNativeProviderUsage(id);
      if (!snapshot) return null;
      if (!refreshSequence.isCurrent(id, ticket)) return snapshot;
      setQuotas(current => ({ ...current, [id]: quotaFromSnapshot(snapshot, planQuotas[id]) }));
      return snapshot;
    } catch (error) {
      if (!refreshSequence.isCurrent(id, ticket)) return null;
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
  }, [refreshSequence]);

  useEffect(() => {
    if (!isTauriRuntime() || !quotas.antigravity.bridgeInstalled) return;
    let disposed = false;
    let pending = false;
    let timer: number | undefined;
    const poll = async () => {
      if (disposed || pending) return;
      window.clearTimeout(timer);
      if (document.visibilityState === "visible") {
        pending = true;
        await refreshProvider("antigravity");
        if (!disposed) await loadHistory();
        pending = false;
      }
      if (!disposed) timer = window.setTimeout(() => void poll(), 30_000);
    };
    const activate = () => { if (document.visibilityState === "visible") void poll(); };
    void poll();
    window.addEventListener("focus", activate);
    document.addEventListener("visibilitychange", activate);
    return () => { disposed = true; window.clearTimeout(timer); window.removeEventListener("focus", activate); document.removeEventListener("visibilitychange", activate); };
  }, [quotas.antigravity.bridgeInstalled, refreshProvider, loadHistory]);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      if (isTauriRuntime()) {
        await Promise.all(providers.map(provider => refreshProvider(provider.id)));
      } else {
        await new Promise(resolve => window.setTimeout(resolve, 420));
      }
      await loadHistory();
      setNow(Date.now());
      setRefreshedAt("지금");
    } finally {
      setRefreshing(false);
    }
  }, [refreshProvider, loadHistory]);

  const checkForUpdates = useCallback(async () => {
    if (updateOperationInFlight.current) return;
    // Mac preview packages have no signed updater feed; the browser has no updater at all.
    const unsupported = isMac ? macUpdateMessage : isTauriRuntime() ? null : "설치된 앱에서만 업데이트를 확인할 수 있습니다.";
    if (unsupported) {
      setUpdateState({ phase: "unsupported", version: null, message: unsupported });
      setUpdateNoticeVisible(false);
      return;
    }
    updateOperationInFlight.current = true;
    setUpdateState({ phase: "checking", version: null, message: "새 버전을 확인하고 있습니다." });
    try {
      const update = await checkForAppUpdate();
      pendingUpdate.current = update;
      if (!update) {
        setUpdateState({ phase: "up-to-date", version: null, message: "현재 최신 버전입니다." });
        setUpdateNoticeVisible(false);
        return;
      }
      setUpdateState({ phase: "available", version: update.version, message: `새 버전 ${update.version}을 설치할 수 있습니다.` });
      setUpdateNoticeVisible(true);
    } catch (error) {
      pendingUpdate.current = null;
      const detail = error instanceof Error ? error.message : "업데이트 서버에 연결하지 못했습니다.";
      setUpdateState({ phase: "error", version: null, message: `업데이트 확인 실패: ${detail}` });
      setUpdateNoticeVisible(true);
    } finally {
      updateOperationInFlight.current = false;
    }
  }, [isMac]);

  const installUpdate = useCallback(async () => {
    if (updateOperationInFlight.current) return;
    let update = pendingUpdate.current;
    if (!update) {
      await checkForUpdates();
      update = pendingUpdate.current;
    }
    if (!update) return;
    updateOperationInFlight.current = true;
    setUpdateState({ phase: "installing", version: update.version, message: `버전 ${update.version}을 설치하고 있습니다.` });
    setUpdateNoticeVisible(true);
    try {
      await update.install();
    } catch (error) {
      const detail = error instanceof Error ? error.message : "업데이트를 설치하지 못했습니다.";
      setUpdateState({ phase: "error", version: update.version, message: `업데이트 설치 실패: ${detail}` });
      setUpdateNoticeVisible(true);
    } finally {
      updateOperationInFlight.current = false;
    }
  }, [checkForUpdates]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    const timer = window.setTimeout(() => void checkForUpdates(), 2_500);
    return () => window.clearTimeout(timer);
  }, [checkForUpdates]);

  useEffect(() => {
    if (!isTauriRuntime() || initialRefreshStarted.current) return;
    initialRefreshStarted.current = true;
    const pending = providersToRefreshAfterBoot(boot, providers.map(provider => provider.id));
    if (pending.length === providers.length) {
      void refresh();
      return;
    }
    // Recheck missing or stale snapshots after standby while reusing fresh Codex usage
    // to avoid an unnecessary App Server launch.
    for (const id of pending) void refreshProvider(id);
  }, [boot, refresh, refreshProvider]);

  useEffect(() => {
    if (!isTauriRuntime()) return;
    let disposed = false;
    let unlisten: TauriUnlisten | null = null;
    void listenNativeProviderUsage(snapshot => {
      const id = snapshot.providerId;
      if (!providers.some(provider => provider.id === id)) return;
      // Claim a fresh ticket so an older in-flight request cannot overwrite this newer snapshot.
      refreshSequence.begin(id);
      setQuotas(current => ({ ...current, [id]: quotaFromSnapshot(snapshot, planQuotas[id]) }));
      setRefreshedAt(autoRefreshLabel(new Date()));
      setNow(Date.now());
      void loadHistory();
    }).then(handle => {
      if (disposed) void handle?.();
      else unlisten = handle;
    }).catch((error: unknown) => console.warn("SPECTRA: provider usage events are unavailable", error));
    return () => {
      disposed = true;
      void unlisten?.();
    };
  }, [refreshSequence, loadHistory]);

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
        setActionFeedback({ status: "not-available", message: "로그인 확인 시간 초과. 로그인 후 새로고침해 주세요." });
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
      setActionFeedback({ status: "demo-only", message: "예시 화면입니다. 설치된 앱에서 연결해 주세요." });
      return;
    }
    const snapshot = await refreshProvider(id);
    if (!snapshot) {
      setActionFeedback({ status: "not-available", message: "계정 상태 확인 실패. 다시 시도해 주세요." });
      return;
    }
    const action = nextConnectionAction(id, snapshot);
    if (action === "install-cli") {
      setActionFeedback({ status: "not-installed", message: snapshot.message });
      return;
    }
    if (action === "login") {
      const result = await startNativeProviderLogin(id);
      setActionFeedback(result ?? { status: "not-available", message: "로그인 프로세스를 시작하지 못했습니다." });
      if (result?.status === "login-started") setLoginPollingProviderId(id);
      return;
    }
    if (action === "install-bridge") {
      const result = await installNativeProviderBridge(id);
      setActionFeedback(result ?? { status: "not-available", message: "사용량 브리지를 설치하지 못했습니다." });
      if (result?.status === "bridge-installed") void refreshProvider(id);
      return;
    }
    setActionFeedback({ status: "refreshed", message: snapshot.message });
  }, [refreshProvider]);
  const disconnectOAuth = useCallback(async (id: ProviderId) => {
    if (!isTauriRuntime() || (id !== "claude" && id !== "antigravity")) {
      setActionFeedback({ status: "not-supported", message: "SPECTRA는 공급자 로그인 자격 증명을 삭제하지 않습니다." });
      return;
    }
    const result = await removeNativeProviderBridge(id);
    setActionFeedback(result ?? { status: "not-available", message: "사용량 브리지를 제거하지 못했습니다." });
    if (result?.status === "bridge-removed") void refreshProvider(id);
  }, [refreshProvider]);

  const sharedProps: SharedViewProps = {
    platform,
    view,
    onView: setView,
    activeProvider,
    activeProviderId,
    activeQuota,
    quotas,
    metric,
    range,
    onProvider: (id) => { setActiveProviderId(id); if (view === "settings" || view === "alerts") setView("overview"); },
    onMetric: setMetric,
    onRange: setRange,
    onOpenOAuth: openOAuth,
    group, quotaWindow, history, historyState, now, pinned, uiError,
    onGroup: id => setSelectedGroups(current => ({ ...current, [activeProviderId]: id })),
    onWindow: setSelectedWindowId,
    onPin: () => void togglePin(),
    onOpenDashboard: targetView => void changeWindowMode("dashboard", targetView),
    onOpenMini: () => void changeWindowMode("mini"),
    refreshedAt,
    refreshing,
    solid,
    theme,
    standby,
    strip,
    stripPending,
    onRefresh: refresh,
    onSolid: () => setSolid(value => !value),
    onTheme: () => setTheme(value => value === "dark" ? "light" : "dark"),
    onStandby: () => void toggleStandby(),
    onStrip: () => void toggleStrip(),
    updateState,
    onCheckForUpdate: () => void checkForUpdates(),
    onInstallUpdate: () => void installUpdate()
  };

  return <><div className={solid ? "solid-mode" : ""}>{windowMode === "mini" ? <MiniLayout {...sharedProps} /> : <FocusDashboard {...sharedProps} />}</div><UpdateNotice state={updateState} visible={updateNoticeVisible} onInstall={installUpdate} onRetry={checkForUpdates} onDismiss={() => setUpdateNoticeVisible(false)} /><OAuthDialog open={oauthOpen} provider={oauthProvider} quota={oauthQuota} startResult={actionFeedback} onClose={closeOAuth} onConnect={connectOAuth} onDisconnect={disconnectOAuth} /></>;
}
