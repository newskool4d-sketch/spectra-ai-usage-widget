export type UpdatePhase = "idle" | "checking" | "available" | "installing" | "up-to-date" | "unsupported" | "error";
export type UpdateState = Readonly<{ phase: UpdatePhase; version: string | null; message: string }>;

type UpdateNoticeProps = Readonly<{
  state: UpdateState;
  visible: boolean;
  onInstall: () => void;
  onRetry: () => void;
  onDismiss: () => void;
}>;

export function UpdateNotice({ state, visible, onInstall, onRetry, onDismiss }: UpdateNoticeProps) {
  if (!visible || !["available", "checking", "installing", "error"].includes(state.phase)) return null;
  const busy = state.phase === "checking" || state.phase === "installing";
  const available = state.phase === "available";
  const failed = state.phase === "error";
  const title = failed ? "SPECTRA 업데이트 실패" : available ? `SPECTRA 새 버전 ${state.version}` : "SPECTRA 업데이트";
  const actionLabel = available ? "지금 설치" : state.phase === "installing" ? "설치 중" : state.phase === "checking" ? "확인 중" : "다시 확인";
  return <aside className="update-notice" role={failed ? "alert" : "status"} aria-live={failed ? "assertive" : "polite"} aria-busy={busy}>
    <div className="update-notice-copy"><strong>{title}</strong><span>{state.message}</span></div>
    <div className="update-notice-actions"><button type="button" className="primary-action" onClick={available ? onInstall : onRetry} disabled={busy}>{actionLabel}</button><button type="button" className="text-button" onClick={onDismiss} disabled={busy}>나중에</button></div>
  </aside>;
}
