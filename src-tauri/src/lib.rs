#[cfg(feature = "native-oauth")]
mod credential_vault;
mod desktop_shell;
#[cfg(feature = "native-oauth")]
mod oauth_callback;
#[cfg(feature = "native-oauth")]
mod provider_connection;
mod provider_usage;
mod standby;
mod tray_badge;

#[cfg(feature = "native-oauth")]
use std::io::{Read, Write};
#[cfg(feature = "native-oauth")]
use std::net::TcpListener;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
#[cfg(feature = "native-oauth")]
use tauri::{Emitter, Runtime};
#[cfg(feature = "native-oauth")]
use tauri_plugin_deep_link::DeepLinkExt;
#[cfg(feature = "native-oauth")]
use url::Url;

#[derive(Default)]
pub struct AppState {
    #[cfg(feature = "native-oauth")]
    pending_oauth: Mutex<Option<oauth_callback::PendingOAuth>>,
    #[cfg(feature = "native-oauth")]
    loopback_listener: Mutex<Option<TcpListener>>,
    pub(crate) ui: Mutex<standby::UiPrefs>,
    pub(crate) window_mode: Mutex<desktop_shell::WindowMode>,
    pub(crate) last_snapshots: Mutex<Vec<provider_usage::ProviderUsageSnapshot>>,
}

impl AppState {
    fn with_prefs(prefs: standby::UiPrefs) -> Self {
        Self { ui: Mutex::new(prefs), ..Self::default() }
    }

    pub(crate) fn boot_state(&self, mode: desktop_shell::WindowMode) -> standby::BootState {
        let prefs = self.ui.lock().map(|p| p.clone()).unwrap_or_default();
        let snapshots = self.last_snapshots.lock().map(|s| s.clone()).unwrap_or_default();
        standby::BootState { theme: prefs.theme, solid: prefs.solid, standby: prefs.standby, mode: mode.slug(), snapshots }
    }

    pub(crate) fn remember_snapshot(&self, snapshot: &provider_usage::ProviderUsageSnapshot) {
        if let Ok(mut list) = self.last_snapshots.lock() {
            list.retain(|s| s.provider_id != snapshot.provider_id);
            list.push(snapshot.clone());
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialStatus {
    provider_id: String,
    available: bool,
    present: bool,
}

#[cfg(feature = "native-oauth")]
fn process_callback_urls<R: Runtime + 'static>(app: &AppHandle<R>, urls: Vec<url::Url>) {
    for url in urls {
        let result = {
            let state = app.state::<AppState>();
            oauth_callback::take_exchange(&state.pending_oauth, &url)
        };

        match result {
            Ok(exchange) => {
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    match oauth_callback::exchange_and_store(exchange).await {
                        Ok(event) => {
                            let _ = app_handle.emit("oauth-callback", event);
                        }
                        Err(error) => {
                            let _ = app_handle.emit(
                                "oauth-callback-rejected",
                                oauth_callback::OAuthCallbackRejected {
                                    reason: error.safe_code(),
                                },
                            );
                        }
                    }
                });
            }
            Err(error) => {
                let _ = app.emit(
                    "oauth-callback-rejected",
                    oauth_callback::OAuthCallbackRejected {
                        reason: error.safe_code(),
                    },
                );
            }
        }
    }
}

#[cfg(feature = "native-oauth")]
fn spawn_loopback_listener<R: Runtime + 'static>(app: AppHandle<R>, listener: TcpListener) {
    let _ = std::thread::Builder::new()
        .name("spectra-oauth-loopback".to_string())
        .spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };

            let mut buffer = [0_u8; 8 * 1024];
            let Ok(read) = stream.read(&mut buffer) else {
                return;
            };
            let request = String::from_utf8_lossy(&buffer[..read]);
            let target = request
                .lines()
                .next()
                .and_then(|line| {
                    let mut parts = line.split_whitespace();
                    (parts.next() == Some("GET")).then(|| parts.next().unwrap_or_default())
                })
                .unwrap_or_default();
            if target.is_empty() || !target.starts_with(oauth_callback::CALLBACK_PATH) {
                let _ = stream.write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                return;
            }

            let Ok(callback_url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
                let _ = stream.write_all(
                    b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                return;
            };
            let body = "SPECTRA 로그인 완료. 이 창을 닫아도 됩니다.";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
            process_callback_urls(&app, vec![callback_url]);
        });
}

#[cfg(feature = "native-oauth")]
#[tauri::command]
fn oauth_prepare(
    app: AppHandle,
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<oauth_callback::OAuthPrepareResult, String> {
    let result =
        oauth_callback::prepare(&state.pending_oauth, &state.loopback_listener, provider_id)?;
    if result.callback_mode == "loopback" {
        let listener = state
            .loopback_listener
            .lock()
            .map_err(|_| "oauth loopback state unavailable".to_string())?
            .take();
        if let Some(listener) = listener {
            spawn_loopback_listener(app, listener);
        }
    }
    Ok(result)
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn oauth_prepare(provider_id: String) -> Result<serde_json::Value, String> {
    let _ = provider_id;
    // Marker consumed by src/integrations/oauth-messages.ts (describePrepareFailure).
    Err("native-oauth-disabled".to_string())
}

#[cfg(feature = "native-oauth")]
#[tauri::command]
fn credential_status(provider_id: String) -> Result<CredentialStatus, String> {
    match credential_vault::exists(&provider_id) {
        Ok(present) => Ok(CredentialStatus {
            provider_id,
            available: true,
            present,
        }),
        Err(credential_vault::VaultError::UnsupportedPlatform) => Ok(CredentialStatus {
            provider_id,
            available: false,
            present: false,
        }),
        Err(_) => Err("credential vault unavailable".to_string()),
    }
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn credential_status(provider_id: String) -> Result<CredentialStatus, String> {
    Ok(CredentialStatus { provider_id, available: false, present: false })
}

#[cfg(feature = "native-oauth")]
#[tauri::command]
fn credential_remove(provider_id: String) -> Result<(), String> {
    credential_vault::remove(&provider_id).map_err(|_| "credential removal failed".to_string())
}

#[cfg(not(feature = "native-oauth"))]
#[tauri::command]
fn credential_remove(provider_id: String) -> Result<(), String> {
    let _ = provider_id;
    Ok(())
}
#[tauri::command]
async fn provider_usage_snapshot(
    app: AppHandle,
    provider_id: String,
) -> Result<provider_usage::ProviderUsageSnapshot, String> {
    let snapshot = tauri::async_runtime::spawn_blocking(move || provider_usage::snapshot(&provider_id))
        .await
        .map_err(|_| "provider usage worker failed".to_string())?;
    app.state::<AppState>().remember_snapshot(&snapshot);
    desktop_shell::update_tray_badge(&app);
    Ok(snapshot)
}

#[tauri::command]
fn provider_start_login(provider_id: String) -> provider_usage::ProviderActionResult {
    provider_usage::start_login(&provider_id)
}

#[tauri::command]
fn provider_install_bridge(provider_id: String) -> provider_usage::ProviderActionResult {
    provider_usage::install_bridge(&provider_id)
}

#[tauri::command]
fn provider_remove_bridge(provider_id: String) -> provider_usage::ProviderActionResult {
    provider_usage::remove_bridge(&provider_id)
}

fn locked_prefs<'a>(state: &'a State<'_, AppState>) -> Result<std::sync::MutexGuard<'a, standby::UiPrefs>, String> {
    state.ui.lock().map_err(|_| "ui preferences unavailable".to_string())
}

#[tauri::command]
fn get_ui_prefs(state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    locked_prefs(&state).map(|prefs| prefs.clone())
}

fn apply_prefs(
    state: &State<'_, AppState>,
    update: impl FnOnce(&mut standby::UiPrefs),
) -> Result<standby::UiPrefs, String> {
    let updated = {
        let mut prefs = locked_prefs(state)?;
        update(&mut prefs);
        prefs.clone()
    };
    // The change takes effect immediately; persistence is best-effort so a disk
    // error never blocks theme or standby for the running session.
    if let Err(error) = standby::save_prefs(&updated) {
        eprintln!("spectra: ui preferences were not saved: {error}");
    }
    Ok(updated)
}

#[tauri::command]
fn set_ui_prefs(theme: String, solid: bool, state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    apply_prefs(&state, |prefs| {
        prefs.theme = standby::normalize_theme(&theme).to_string();
        prefs.solid = solid;
    })
}

#[tauri::command]
fn set_standby(enabled: bool, state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    apply_prefs(&state, |prefs| prefs.standby = enabled)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_deep_link::init());

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = desktop_shell::show_main_window(app, desktop_shell::WindowMode::Mini);
        }));
    }

    let app = builder
        .on_window_event(|window, event| {
            if window.label() != desktop_shell::MAIN_WINDOW_LABEL {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let standby = window
                    .app_handle()
                    .state::<AppState>()
                    .ui
                    .lock()
                    .map(|prefs| prefs.standby)
                    .unwrap_or(false);
                if let Some(webview) = window.app_handle().get_webview_window(desktop_shell::MAIN_WINDOW_LABEL) {
                    let _ = desktop_shell::dismiss(&webview, standby);
                }
            }
        })
        .manage(AppState::with_prefs(standby::load_prefs()))
        .invoke_handler(tauri::generate_handler![
            oauth_prepare,
            credential_status,
            credential_remove,
            provider_usage_snapshot,
            provider_start_login,
            provider_install_bridge,
            provider_remove_bridge,
            get_ui_prefs,
            set_ui_prefs,
            set_standby
        ])
        .setup(|app| {
            desktop_shell::install(app)?;
            if let Err(error) = desktop_shell::show_main_window(app.handle(), desktop_shell::WindowMode::Mini) {
                // The tray is already installed, so a failed first show must not abort
                // startup; the user can reopen the window from the tray.
                eprintln!("spectra: main window could not be shown at startup: {error}");
            }

            #[cfg(feature = "native-oauth")]
            {
                #[cfg(any(target_os = "windows", target_os = "linux"))]
                app.deep_link().register_all()?;

                let app_handle = app.handle().clone();
                if let Some(urls) = app.deep_link().get_current()? {
                    process_callback_urls(&app_handle, urls);
                }

                let event_app = app.handle().clone();
                app.deep_link().on_open_url(move |event| {
                    process_callback_urls(&event_app, event.urls());
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building SPECTRA native shell");

    app.run(|_app, event| {
        if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
            api.prevent_exit();
        }
    });
}

pub fn run_cli_mode() -> bool {
    provider_usage::run_statusline_bridge() || provider_usage::run_provider_snapshot_cli()
}

#[cfg(test)]
mod app_state_tests {
    use super::*;

    fn make_snapshot(provider_id: &str, message: &str) -> provider_usage::ProviderUsageSnapshot {
        provider_usage::ProviderUsageSnapshot {
            provider_id: provider_id.to_string(),
            runtime_available: true,
            auth_state: "authenticated".to_string(),
            connection_state: "connected".to_string(),
            auth_method: Some("oauth".to_string()),
            plan_type: Some("pro".to_string()),
            source: Some("bridge".to_string()),
            last_synced_at: Some(0),
            bridge_installed: true,
            windows: Vec::new(),
            message: message.to_string(),
        }
    }

    #[test]
    fn remember_snapshot_replaces_same_provider_with_newer_payload() {
        let state = AppState::default();
        state.remember_snapshot(&make_snapshot("codex", "first sync"));
        state.remember_snapshot(&make_snapshot("codex", "second sync"));

        let snapshots = state.last_snapshots.lock().unwrap();
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].message, "second sync");
    }

    #[test]
    fn boot_state_carries_mode_prefs_and_snapshots() {
        let state = AppState::default();
        *state.ui.lock().unwrap() = standby::UiPrefs {
            theme: "light".to_string(),
            solid: true,
            standby: true,
        };
        state.remember_snapshot(&make_snapshot("codex", "codex snapshot"));

        let boot = state.boot_state(desktop_shell::WindowMode::Dashboard);

        assert_eq!(boot.mode, "dashboard");
        assert_eq!(boot.theme, "light");
        assert!(boot.solid);
        assert!(boot.standby);
        assert_eq!(boot.snapshots.len(), 1);
        assert_eq!(boot.snapshots[0].provider_id, "codex");
    }
}

#[cfg(all(test, not(feature = "native-oauth")))]
mod tests {
    use super::*;

    #[test]
    fn disabled_oauth_prepare_reports_marker() {
        assert_eq!(oauth_prepare("codex".to_string()).unwrap_err(), "native-oauth-disabled");
    }

    #[cfg(not(feature = "native-oauth"))]
    #[test]
    fn disabled_credential_status_reports_unavailable() {
        let status = credential_status("codex".to_string()).expect("stub never fails");
        assert_eq!(status.provider_id, "codex");
        assert!(!status.available);
        assert!(!status.present);
    }

    #[cfg(not(feature = "native-oauth"))]
    #[test]
    fn disabled_credential_remove_is_a_noop() {
        assert!(credential_remove("claude".to_string()).is_ok());
    }
}
