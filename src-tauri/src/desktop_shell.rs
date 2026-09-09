use tauri::menu::MenuBuilder;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, LogicalSize, Manager, Runtime, Size, WebviewWindow};
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

pub(crate) fn show_main_window<R: Runtime>(
    app: &AppHandle<R>,
    mode: WindowMode,
) -> tauri::Result<()> {
    store_mode(app, mode);
    let started = std::time::Instant::now();
    let window = match app.get_webview_window(MAIN_WINDOW_LABEL) {
        Some(window) => window,
        None => {
            let boot = app.state::<crate::AppState>().boot_state(mode);
            crate::standby::create_main_window(app, mode, &boot)?
        }
    };

    let profile = mode.profile();
    window.set_size(Size::Logical(LogicalSize::new(profile.width, profile.height)))?;
    window.set_always_on_top(profile.always_on_top)?;

    if window.is_minimized()? {
        window.unminimize()?;
    }

    window.center()?;
    window.show()?;
    window.set_focus()?;
    window.eval(&mode_script(mode))?;
    crate::standby::log_timing("show_main_window", started.elapsed());
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

#[cfg(test)]
mod tests {
    use super::{mode_script, WindowMode, MENU_HIDE, MENU_OPEN_DASHBOARD, MENU_OPEN_MINI, MENU_QUIT};

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
}
