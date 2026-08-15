mod credential_vault;
mod oauth_callback;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_deep_link::DeepLinkExt;

#[derive(Default)]
pub struct AppState {
    pending_oauth: Mutex<Option<oauth_callback::PendingOAuth>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CredentialStatus {
    provider_id: String,
    available: bool,
    present: bool,
}

fn process_callback_urls<R: Runtime>(app: &AppHandle<R>, urls: Vec<url::Url>) {
    for url in urls {
        let result = {
            let state = app.state::<AppState>();
            oauth_callback::accept(&state.pending_oauth, &url)
        };

        match result {
            Ok(event) => {
                let _ = app.emit("oauth-callback", event);
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

#[tauri::command]
fn oauth_prepare(
    provider_id: String,
    state: State<'_, AppState>,
) -> Result<oauth_callback::OAuthPrepareResult, String> {
    oauth_callback::prepare(&state.pending_oauth, provider_id)
}

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

#[tauri::command]
fn credential_remove(provider_id: String) -> Result<(), String> {
    credential_vault::remove(&provider_id).map_err(|_| "credential removal failed".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default().plugin(tauri_plugin_deep_link::init());

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}));
    }

    builder
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            oauth_prepare,
            credential_status,
            credential_remove
        ])
        .setup(|app| {
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

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running SPECTRA native shell");
}
