export type OAuthPrepareFailure = Readonly<{
  status: "not-available";
  message: string;
}>;

// Must match the Err string of the disabled oauth_prepare stub in src-tauri/src/lib.rs.
const disabledMarker = "native-oauth-disabled";

export function describePrepareFailure(error: unknown): OAuthPrepareFailure {
  const text = error instanceof Error ? error.message : String(error);
  if (text.includes(disabledMarker)) {
    return {
      status: "not-available",
      message: "네이티브 OAuth 연결은 준비 중입니다. 지금은 Codex App Server와 Claude Code 공식 도구로 사용량을 확인합니다."
    };
  }
  return {
    status: "not-available",
    message: "이 기기에서 네이티브 OAuth 준비를 완료하지 못했습니다."
  };
}
