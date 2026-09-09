use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Runtime, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::desktop_shell::{WindowMode, MAIN_WINDOW_LABEL};
use crate::provider_usage::{app_data_dir, ProviderUsageSnapshot};

pub const WINDOW_TITLE: &str = "SPECTRA AI 사용 현황";
const PREFS_FILE: &str = "ui-prefs.json";
const TIMING_LOG_FILE: &str = "standby-timing.log";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiPrefs {
    pub theme: String,
    pub solid: bool,
    pub standby: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self { theme: "dark".to_string(), solid: false, standby: false }
    }
}

pub fn normalize_theme(raw: &str) -> &'static str {
    if raw == "light" { "light" } else { "dark" }
}

pub fn prefs_path() -> Option<PathBuf> {
    app_data_dir().map(|dir| dir.join(PREFS_FILE))
}

pub fn load_prefs_from(path: &Path) -> UiPrefs {
    fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<UiPrefs>(&bytes).ok())
        .map(|mut prefs| {
            prefs.theme = normalize_theme(&prefs.theme).to_string();
            prefs
        })
        .unwrap_or_default()
}

pub fn save_prefs_to(path: &Path, prefs: &UiPrefs) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, serde_json::to_vec_pretty(prefs)?)
}

pub fn load_prefs() -> UiPrefs {
    prefs_path().map(|path| load_prefs_from(&path)).unwrap_or_default()
}

pub fn save_prefs(prefs: &UiPrefs) -> std::io::Result<()> {
    match prefs_path() {
        Some(path) => save_prefs_to(&path, prefs),
        None => Ok(()),
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootState {
    pub theme: String,
    pub solid: bool,
    pub standby: bool,
    pub mode: &'static str,
    pub snapshots: Vec<ProviderUsageSnapshot>,
}

pub fn boot_script(boot: &BootState) -> String {
    let json = serde_json::to_string(boot).unwrap_or_else(|_| "{}".to_string());
    format!("window.__SPECTRA_BOOT__={json};window.__SPECTRA_MODE__='{}';", boot.mode)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CloseAction {
    Hide,
    Destroy,
}

pub fn close_action(standby: bool) -> CloseAction {
    if standby { CloseAction::Destroy } else { CloseAction::Hide }
}

pub fn create_main_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
    boot: &BootState,
) -> tauri::Result<WebviewWindow<R>> {
    let profile = mode.profile();
    let started = std::time::Instant::now();
    let window = WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
        .title(WINDOW_TITLE)
        .inner_size(profile.width, profile.height)
        .min_inner_size(380.0, 600.0)
        .resizable(true)
        .center()
        .always_on_top(profile.always_on_top)
        .initialization_script(&boot_script(boot))
        .build()?;
    log_timing("create_main_window", started.elapsed());
    Ok(window)
}

pub fn log_timing(label: &str, elapsed: Duration) {
    if std::env::var_os("SPECTRA_TIMING_LOG").is_none() {
        return;
    }
    let Some(dir) = app_data_dir() else { return };
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let epoch_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(dir.join(TIMING_LOG_FILE)) {
        let _ = writeln!(file, "{epoch_ms},{label},{}", elapsed.as_millis());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::ProviderUsageSnapshot;

    fn temp_prefs_path(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("spectra-prefs-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("ui-prefs.json")
    }

    #[test]
    fn default_prefs_keep_current_behaviour() {
        let prefs = UiPrefs::default();
        assert_eq!(prefs.theme, "dark");
        assert!(!prefs.solid);
        assert!(!prefs.standby);
        assert_eq!(close_action(prefs.standby), CloseAction::Hide);
    }

    #[test]
    fn standby_closes_by_destroying() {
        assert_eq!(close_action(true), CloseAction::Destroy);
    }

    #[test]
    fn normalize_theme_accepts_only_known_values() {
        assert_eq!(normalize_theme("light"), "light");
        assert_eq!(normalize_theme("dark"), "dark");
        assert_eq!(normalize_theme("neon"), "dark");
    }

    #[test]
    fn prefs_round_trip_through_file() {
        let path = temp_prefs_path("roundtrip");
        let prefs = UiPrefs { theme: "light".into(), solid: true, standby: true };
        save_prefs_to(&path, &prefs).unwrap();
        assert_eq!(load_prefs_from(&path), prefs);
    }

    #[test]
    fn missing_or_corrupt_prefs_fall_back_to_default() {
        let missing = temp_prefs_path("missing");
        assert_eq!(load_prefs_from(&missing), UiPrefs::default());
        let corrupt = temp_prefs_path("corrupt");
        std::fs::write(&corrupt, b"{not json").unwrap();
        assert_eq!(load_prefs_from(&corrupt), UiPrefs::default());
    }

    #[test]
    fn boot_script_embeds_json_and_mode() {
        let boot = BootState { theme: "light".into(), solid: false, standby: true, mode: "dashboard", snapshots: Vec::<ProviderUsageSnapshot>::new() };
        let script = boot_script(&boot);
        assert!(script.starts_with("window.__SPECTRA_BOOT__={"));
        assert!(script.ends_with("window.__SPECTRA_MODE__='dashboard';"));
        let json_part = script.trim_start_matches("window.__SPECTRA_BOOT__=").split(";window.__SPECTRA_MODE__").next().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(json_part).unwrap();
        assert_eq!(parsed["theme"], "light");
        assert_eq!(parsed["standby"], true);
        assert_eq!(parsed["mode"], "dashboard");
        assert!(parsed["snapshots"].as_array().unwrap().is_empty());
    }
}
