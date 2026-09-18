use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, LogicalSize, Manager, Runtime, Size, WebviewWindow};
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "windows")]
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::image::Image;

pub(crate) const MAIN_WINDOW_LABEL: &str = "main";

const MENU_OPEN_MINI: &str = "open-mini";
const MENU_OPEN_DASHBOARD: &str = "open-dashboard";
const MENU_HIDE: &str = "hide";
const MENU_QUIT: &str = "quit";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) enum WindowMode {
    #[default]
    Mini,
    Dashboard,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct WindowProfile {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) always_on_top: bool,
}

impl WindowMode {
    pub(crate) fn profile(self) -> WindowProfile {
        match self {
            Self::Mini => WindowProfile {
                width: 430.0,
                height: 720.0,
                always_on_top: true,
            },
            Self::Dashboard => WindowProfile {
                width: 1280.0,
                height: 860.0,
                always_on_top: false,
            },
        }
    }

    pub(crate) fn slug(self) -> &'static str {
        match self {
            Self::Mini => "mini",
            Self::Dashboard => "dashboard",
        }
    }
}

fn mode_script(mode: WindowMode) -> String {
    let slug = mode.slug();
    format!("window.__SPECTRA_MODE__='{slug}';window.dispatchEvent(new CustomEvent('spectra-mode',{{detail:'{slug}'}}));")
}

fn stored_mode<R: Runtime>(app: &AppHandle<R>) -> WindowMode {
    app.state::<crate::AppState>().window_mode.lock().map(|mode| *mode).unwrap_or_default()
}

fn store_mode<R: Runtime>(app: &AppHandle<R>, mode: WindowMode) {
    if let Ok(mut slot) = app.state::<crate::AppState>().window_mode.lock() {
        *slot = mode;
    }
}

fn standby_enabled<R: Runtime>(app: &AppHandle<R>) -> bool {
    app.state::<crate::AppState>().ui.lock().map(|prefs| prefs.standby).unwrap_or(false)
}

pub(crate) fn dismiss<R: Runtime>(window: &WebviewWindow<R>, standby: bool) -> tauri::Result<()> {
    match crate::standby::close_action(standby) {
        crate::standby::CloseAction::Hide => window.hide(),
        crate::standby::CloseAction::Destroy => window.destroy(),
    }
}

fn present<R: Runtime>(window: &WebviewWindow<R>, mode: WindowMode) -> tauri::Result<()> {
    let profile = mode.profile();
    window.set_size(Size::Logical(LogicalSize::new(profile.width, profile.height)))?;
    window.set_always_on_top(profile.always_on_top)?;

    if window.is_minimized()? {
        window.unminimize()?;
    }

    window.center()?;
    window.show()?;
    window.set_focus()?;
    window.eval(&mode_script(mode))
}

fn recreate_and_present<R: Runtime>(app: &AppHandle<R>, mode: WindowMode) -> tauri::Result<()> {
    let started = std::time::Instant::now();
    let boot = app.state::<crate::AppState>().boot_state(mode);
    let window = crate::standby::create_main_window(app, mode, &boot)?;
    present(&window, mode)?;
    crate::standby::log_timing("show_main_window", started.elapsed());
    Ok(())
}

/// Creates and shows the main window synchronously. Only for the `setup` hook, which runs
/// before the event loop; event callbacks must use `show_main_window`, which defers creation.
pub(crate) fn create_initial_window<R: Runtime>(app: &AppHandle<R>, mode: WindowMode) -> tauri::Result<()> {
    store_mode(app, mode);
    recreate_and_present(app, mode)
}

/// Claims the single recreation slot; `false` means a recreation is already in flight.
fn try_begin_recreate(flag: &AtomicBool) -> bool {
    flag.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_ok()
}

/// Releases the recreation slot when dropped, on every exit path of the worker.
struct RecreateGuard<'a>(&'a AtomicBool);

impl Drop for RecreateGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub(crate) fn show_main_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
) -> tauri::Result<()> {
    store_mode(app, mode);
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let started = std::time::Instant::now();
        present(&window, mode)?;
        crate::standby::log_timing("show_main_window", started.elapsed());
        return Ok(());
    }

    // On Windows, building a webview inside a synchronous event handler (tray click, tray
    // menu, single-instance callback) can deadlock — a documented `WebviewWindowBuilder`
    // known issue — so the recreation runs on a worker thread and this callback returns first.
    // Only one recreation may be in flight: Tauri checks the label in `prepare_window` but
    // registers the window only after it is built, so two concurrent builders would both
    // succeed and leave an unreachable duplicate window behind.
    if !try_begin_recreate(&app.state::<crate::AppState>().recreating) {
        return Ok(());
    }
    let worker = app.clone();
    let spawned = std::thread::Builder::new()
        .name("spectra-window-recreate".to_string())
        .spawn(move || {
            let state = worker.state::<crate::AppState>();
            let _release = RecreateGuard(&state.recreating);
            let mode = stored_mode(&worker);
            let result = match worker.get_webview_window(MAIN_WINDOW_LABEL) {
                Some(window) => {
                    let started = std::time::Instant::now();
                    present(&window, mode)
                        .map(|()| crate::standby::log_timing("show_main_window", started.elapsed()))
                }
                None => recreate_and_present(&worker, mode),
            };
            if let Err(error) = result {
                eprintln!("spectra: main window could not be recreated: {error}");
            }
        });
    if let Err(error) = spawned {
        app.state::<crate::AppState>().recreating.store(false, Ordering::Release);
        return Err(error.into());
    }
    Ok(())
}

fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        dismiss(&window, standby_enabled(app))?;
    }
    Ok(())
}

fn toggle_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return show_main_window(app, stored_mode(app));
    };

    if window.is_visible()? && !window.is_minimized()? {
        dismiss(&window, standby_enabled(app))
    } else {
        show_main_window(app, stored_mode(app))
    }
}

pub(crate) fn install<R: Runtime>(app: &mut App<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text(MENU_OPEN_MINI, "미니 창 열기")
        .text(MENU_OPEN_DASHBOARD, "대시보드 열기")
        .separator()
        .text(MENU_HIDE, "숨기기")
        .separator()
        .text(MENU_QUIT, "SPECTRA 종료")
        .build()?;

    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip(TRAY_TOOLTIP)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_MINI => {
                let _ = show_main_window(app, WindowMode::Mini);
            }
            MENU_OPEN_DASHBOARD => {
                let _ = show_main_window(app, WindowMode::Dashboard);
            }
            MENU_HIDE => {
                let _ = hide_main_window(app);
            }
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let _ = toggle_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        tray = tray.icon(icon);
    }

    let _ = tray.build(app)?;
    Ok(())
}

const TRAY_ID: &str = "spectra-main";
const TRAY_TOOLTIP: &str = "SPECTRA · 개인 요금제 사용 현황";

pub(crate) fn update_tray_badge<R: Runtime>(app: &AppHandle<R>) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let snapshots = app
        .state::<crate::AppState>()
        .last_snapshots
        .lock()
        .map(|list| list.clone())
        .unwrap_or_default();
    match crate::tray_badge::min_remaining_percent(&snapshots) {
        Some((provider_id, percent)) => {
            let image = Image::new_owned(crate::tray_badge::render(percent), crate::tray_badge::BADGE_SIZE, crate::tray_badge::BADGE_SIZE);
            let _ = tray.set_icon(Some(image));
            let _ = tray.set_tooltip(Some(format!("{TRAY_TOOLTIP} · 최소 잔여 {percent}% ({provider_id})")));
        }
        None => {
            let _ = tray.set_icon(app.default_window_icon().cloned());
            let _ = tray.set_tooltip(Some(TRAY_TOOLTIP));
        }
    }
}

/// Uses only the latest cached snapshots. Closing the WebView leaves this native window alive.
#[cfg(target_os = "windows")]
pub(crate) fn update_taskbar_strip<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<crate::AppState>();
    // Serialize reconciliation before reading preferences: an older snapshot callback must
    // not recreate a strip after a newer toggle has disabled it.
    let Ok(mut slot) = state.strip.lock() else { return };
    let enabled = state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false);
    if !enabled {
        if let Some(handle) = slot.take() {
            handle.shutdown();
        }
        return;
    }
    let snapshots = state.last_snapshots.lock().map(|list| list.clone()).unwrap_or_default();
    let model = crate::taskbar_strip::build_model(&snapshots, crate::taskbar_window::current_theme());
    if let Some(handle) = slot.as_ref() {
        // None hides a running strip when both providers become unavailable.
        handle.update(model);
    } else if let Some(model) = model {
        let click_app = app.clone();
        match crate::taskbar_window::spawn(model, Box::new(move || {
            let _ = toggle_main_window(&click_app);
        })) {
            Ok(handle) => *slot = Some(handle),
            Err(error) => eprintln!("spectra: taskbar strip is unavailable: {error}"),
        }
    }
}

/// Claude's reset timestamp is the one provider signal that can become stale while the
/// WebView is destroyed. Keep the native strip self-healing without polling until a reset.
#[cfg(target_os = "windows")]
pub(crate) fn ensure_taskbar_refresh_loop(app: &AppHandle) {
    let state = app.state::<crate::AppState>();
    if !state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false) {
        return;
    }
    if state
        .strip_refresh_started
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return;
    }

    let worker = app.clone();
    let spawned = std::thread::Builder::new()
        .name("spectra-claude-reset-refresh".to_string())
        .spawn(move || {
            let mut last_attempt = 0_u64;
            loop {
                std::thread::sleep(Duration::from_secs(15));
                let state = worker.state::<crate::AppState>();
                let enabled = state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false);
                if !enabled {
                    last_attempt = 0;
                    continue;
                }

                let snapshots = state.last_snapshots.lock().map(|list| list.clone()).unwrap_or_default();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let reset_due = claude_reset_due(&snapshots, now);
                // Age/reset status must advance even with no WebView or new response.
                // This reuses the existing wake-up and performs no provider lookup.
                update_taskbar_strip(&worker);
                if !reset_due || now.saturating_sub(last_attempt) < 60 {
                    continue;
                }

                last_attempt = now;
                let refresh_app = worker.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = crate::provider_usage_snapshot(refresh_app, "claude".to_string()).await {
                        eprintln!("spectra: scheduled Claude reset refresh failed: {error}");
                    }
                });
            }
        });
    if let Err(error) = spawned {
        state.strip_refresh_started.store(false, Ordering::Release);
        eprintln!("spectra: Claude reset refresh loop is unavailable: {error}");
    }
}

#[cfg(target_os = "windows")]
fn claude_reset_due(snapshots: &[crate::provider_usage::ProviderUsageSnapshot], now: u64) -> bool {
    snapshots
        .iter()
        .find(|snapshot| snapshot.provider_id == "claude")
        .is_some_and(|snapshot| snapshot.windows.iter().any(|window| window.resets_at.is_some_and(|reset| reset <= now)))
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn update_taskbar_strip<R: Runtime>(_app: &AppHandle<R>) {}

#[cfg(test)]
mod tests {
    use super::{mode_script, try_begin_recreate, RecreateGuard, WindowMode, MENU_HIDE, MENU_OPEN_DASHBOARD, MENU_OPEN_MINI, MENU_QUIT};
    use std::sync::atomic::AtomicBool;

    #[test]
    fn window_mode_slug_matches_frontend_contract() {
        assert_eq!(WindowMode::Mini.slug(), "mini");
        assert_eq!(WindowMode::Dashboard.slug(), "dashboard");
    }

    #[test]
    fn window_mode_defaults_to_mini() {
        assert_eq!(WindowMode::default(), WindowMode::Mini);
    }

    #[test]
    fn mode_script_sets_global_and_dispatches_event() {
        assert_eq!(mode_script(WindowMode::Mini), "window.__SPECTRA_MODE__='mini';window.dispatchEvent(new CustomEvent('spectra-mode',{detail:'mini'}));");
        assert_eq!(mode_script(WindowMode::Dashboard), "window.__SPECTRA_MODE__='dashboard';window.dispatchEvent(new CustomEvent('spectra-mode',{detail:'dashboard'}));");
    }

    #[test]
    fn mini_profile_is_compact_and_pinned() {
        let profile = WindowMode::Mini.profile();
        assert_eq!(profile.width, 430.0);
        assert_eq!(profile.height, 720.0);
        assert!(profile.always_on_top);
    }

    #[test]
    fn dashboard_profile_restores_full_workspace() {
        let profile = WindowMode::Dashboard.profile();
        assert_eq!(profile.width, 1280.0);
        assert_eq!(profile.height, 860.0);
        assert!(!profile.always_on_top);
    }

    #[test]
    fn tray_menu_ids_are_stable() {
        assert_eq!(
            [MENU_OPEN_MINI, MENU_OPEN_DASHBOARD, MENU_HIDE, MENU_QUIT],
            ["open-mini", "open-dashboard", "hide", "quit"]
        );
    }

    #[test]
    fn recreate_slot_is_exclusive_until_the_guard_drops() {
        let flag = AtomicBool::new(false);
        assert!(try_begin_recreate(&flag));
        assert!(!try_begin_recreate(&flag));
        drop(RecreateGuard(&flag));
        assert!(try_begin_recreate(&flag));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn claude_reset_refresh_starts_only_after_a_window_reset() {
        let snapshot = crate::provider_usage::ProviderUsageSnapshot {
            provider_id: "claude".into(),
            runtime_available: true,
            auth_state: "signed-in".into(),
            connection_state: "connected".into(),
            auth_method: None,
            plan_type: None,
            source: Some("claude-usage-api".into()),
            last_synced_at: Some(100),
            bridge_installed: false,
            windows: vec![crate::provider_usage::ProviderQuotaWindow {
                id: "rolling".into(),
                label: "5시간 한도".into(),
                used_percent: 20.0,
                remaining_percent: 80.0,
                resets_at: Some(120),
                window_duration_mins: Some(300),
            }],
            message: String::new(),
            live_failure: None,
        };
        assert!(!super::claude_reset_due(&[snapshot.clone()], 119));
        assert!(super::claude_reset_due(&[snapshot], 120));
    }
}
