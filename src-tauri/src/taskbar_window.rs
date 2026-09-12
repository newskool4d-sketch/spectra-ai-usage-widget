#![cfg(target_os = "windows")]

use std::cell::Cell;
use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, DeleteDC, DeleteObject, DrawTextW,
    GdiFlush, GetDC, GetMonitorInfoW, MonitorFromWindow, ReleaseDC, SelectObject, SetBkMode, SetTextColor,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, BI_RGB, BITMAPINFO, BITMAPINFOHEADER,
    CLR_INVALID, DIB_RGB_COLORS, DT_CALCRECT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
    HBITMAP, HDC, HFONT, HGDIOBJ, LOGFONTW, MONITORINFO, MONITOR_DEFAULTTONULL,
    NONANTIALIASED_QUALITY, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HWINEVENTHOOK};
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetThreadDpiAwarenessContext, SystemParametersInfoForDpi,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Shell::{
    SHAppBarMessage, SHQueryUserNotificationState, ABM_GETSTATE, ABM_GETTASKBARPOS, ABS_AUTOHIDE,
    APPBARDATA, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::taskbar_strip::{palette, place_strip, retheme_model, Rect as StripRect, StripModel, StripTheme, TaskbarEdge};

const CLASS_NAME: &str = "SpectraTaskbarStrip";
const WM_STRIP_REDRAW: u32 = WM_APP + 1;
const WM_STRIP_SHELL_CHANGED: u32 = WM_APP + 2;

// OUTOFCONTEXT 콜백은 hook 등록 스레드에서 전달된다. 상태 포인터·앱 잠금을 공유하지 않는다.
thread_local! {
    static SHELL_TARGET: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    static SHELL_PENDING: Cell<bool> = const { Cell::new(false) };
}

fn request_shell_redraw() {
    let hwnd = SHELL_TARGET.with(Cell::get);
    if hwnd.is_null() { return; }
    SHELL_PENDING.with(|pending| {
        if !pending.replace(true) && unsafe { PostMessageW(hwnd, WM_STRIP_SHELL_CHANGED, 0, 0) } == 0 {
            pending.set(false); // 실패한 요청이 다음 정상 이벤트를 막지 않게 한다.
        }
    });
}

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK, event: u32, hwnd: HWND, object: i32, child: i32,
    _thread: u32, _time: u32,
) {
    let target = SHELL_TARGET.with(Cell::get);
    if target.is_null() || hwnd == target { return; }
    match event {
        EVENT_SYSTEM_FOREGROUND => request_shell_redraw(), // NULL 전경도 재확인한다.
        EVENT_OBJECT_LOCATIONCHANGE
            if !hwnd.is_null() && object == OBJID_WINDOW && child == CHILDID_SELF as i32
                && hwnd == GetForegroundWindow() => request_shell_redraw(),
        _ => {}
    }
}

fn register_win_event(event: u32) -> Result<HWINEVENTHOOK, String> {
    // SPECTRA 창으로 돌아오는 전경 이벤트도 필요하므로 SKIPOWNPROCESS를 쓰지 않는다.
    let hook = unsafe {
        SetWinEventHook(event, event, std::ptr::null_mut(), Some(on_win_event), 0, 0, WINEVENT_OUTOFCONTEXT)
    };
    if hook.is_null() { Err(win32_error("SetWinEventHook")) } else { Ok(hook) }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 작업표시줄 테마. UiPrefs.theme와 연결하지 않는다(스펙 §3-5).
pub fn current_theme() -> StripTheme {
    let sub = wide("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let name = wide("SystemUsesLightTheme");
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };
    // 값이 없으면 Windows 기본값(다크 작업표시줄)을 따른다.
    if status == 0 && value == 1 { StripTheme::Light } else { StripTheme::Dark }
}

/// 작업표시줄 사각형과 붙은 모서리.
fn taskbar_rect() -> Option<(StripRect, TaskbarEdge)> {
    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    if unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) } == 0 {
        return None;
    }
    let edge = match data.uEdge {
        0 => TaskbarEdge::Left,
        1 => TaskbarEdge::Top,
        2 => TaskbarEdge::Right,
        _ => TaskbarEdge::Bottom,
    };
    let rc = data.rc;
    Some((StripRect { left: rc.left, top: rc.top, right: rc.right, bottom: rc.bottom }, edge))
}

/// 알림 영역(트레이~시계)의 왼쪽 가장자리. 실측상 Windows 11 26200에서도 존재한다.
fn tray_left() -> Option<i32> {
    let tray = unsafe { FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null()) };
    if tray.is_null() {
        return None;
    }
    let notify = unsafe { FindWindowExW(tray, std::ptr::null_mut(), wide("TrayNotifyWnd").as_ptr(), std::ptr::null()) };
    if notify.is_null() {
        return None;
    }
    let mut rc: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(notify, &mut rc) } == 0 {
        return None;
    }
    Some(rc.left)
}

struct StripState {
    model: Arc<Mutex<Option<StripModel>>>,
    on_click: Box<dyn Fn() + Send + 'static>,
    font: Cell<HFONT>,
    font_dpi: Cell<u32>,
    taskbar_created: u32,
    hooks: [HWINEVENTHOOK; 2],
}

impl StripState {
    fn new(model: Arc<Mutex<Option<StripModel>>>, on_click: Box<dyn Fn() + Send + 'static>) -> Self {
        Self {
            model, on_click, font: Cell::new(std::ptr::null_mut()), font_dpi: Cell::new(0),
            taskbar_created: 0, hooks: [std::ptr::null_mut(); 2],
        }
    }

    fn font_for_dpi(&self, dpi: u32) -> Result<HFONT, String> {
        if self.font.get().is_null() || self.font_dpi.get() != dpi {
            // 새 글꼴 생성이 성공해야 기존 캐시를 교체한다. 선택 중인 DC는 없어야 한다.
            let font = shell_font(dpi)?;
            let old = self.font.replace(font);
            self.font_dpi.set(dpi);
            if !old.is_null() {
                let released = unsafe { DeleteObject(old as _) };
                if released == 0 { eprintln!("spectra: old strip font could not be deleted"); }
            }
        }
        Ok(self.font.get())
    }
}

impl Drop for StripState {
    fn drop(&mut self) {
        // WM_DESTROY가 콜백 대상을 먼저 비운 뒤, 등록한 바로 그 스레드에서 해제한다.
        for hook in &mut self.hooks {
            if !hook.is_null() {
                if unsafe { UnhookWinEvent(*hook) } == 0 {
                    eprintln!("spectra: strip event hook could not be released");
                }
                *hook = std::ptr::null_mut();
            }
        }
        if !self.font.get().is_null() {
            if unsafe { DeleteObject(self.font.get() as _) } == 0 {
                eprintln!("spectra: strip font could not be deleted during shutdown");
            }
        }
    }
}

pub struct StripHandle {
    hwnd: isize,
    model: Arc<Mutex<Option<StripModel>>>,
}

// HWND를 isize로 들고 PostMessage로만 건드리므로 스레드 간 이동이 안전하다.
unsafe impl Send for StripHandle {}
unsafe impl Sync for StripHandle {}

impl StripHandle {
    pub fn update(&self, model: Option<StripModel>) {
        if let Ok(mut slot) = self.model.lock() {
            *slot = model;
        }
        unsafe { PostMessageW(self.hwnd as HWND, WM_STRIP_REDRAW, 0, 0) };
    }

    pub fn shutdown(&self) {
        unsafe { PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0) };
    }
}

pub fn spawn(model: StripModel, on_click: Box<dyn Fn() + Send + 'static>) -> Result<StripHandle, String> {
    let shared = Arc::new(Mutex::new(Some(model)));
    let thread_model = Arc::clone(&shared);
    let (tx, rx) = std::sync::mpsc::channel::<Result<isize, String>>();

    std::thread::Builder::new()
        .name("spectra-taskbar-strip".to_string())
        .spawn(move || {
            let hwnd = match create_window(StripState::new(thread_model, on_click)) {
                Ok(hwnd) => hwnd,
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            };
            if let Err(error) = redraw(hwnd) {
                unsafe { DestroyWindow(hwnd) };
                let _ = tx.send(Err(error));
                return;
            }
            let _ = tx.send(Ok(hwnd as isize));
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            // GetMessage 실패/외부 quit에서도 창·hook 소유권을 남기지 않는다.
            if unsafe { IsWindow(hwnd) } != 0 { unsafe { DestroyWindow(hwnd) }; }
        })
        .map_err(|error| format!("strip thread could not start: {error}"))?;

    let hwnd = rx.recv().map_err(|_| "strip thread ended before reporting".to_string())??;
    Ok(StripHandle { hwnd, model: shared })
}

fn create_window(state: StripState) -> Result<HWND, String> {
    create_window_with_hooks(state, register_win_event)
}

fn create_window_with_hooks(
    state: StripState,
    mut register: impl FnMut(u32) -> Result<HWINEVENTHOOK, String>,
) -> Result<HWND, String> {
    if !SHELL_TARGET.with(Cell::get).is_null() {
        return Err("a strip window already owns this thread".into());
    }
    // 전용 스레드에서 창을 만들기 전에 물리 좌표계를 설정하고 종료까지 유지한다.
    if unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) }.is_null() {
        return Err(win32_error("SetThreadDpiAwarenessContext"));
    }
    let class = wide(CLASS_NAME);
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance as _,
        hIcon: std::ptr::null_mut(),
        hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class.as_ptr(),
    };
    // 이미 등록돼 있으면 0이 돌아오지만 그대로 진행해도 된다(재생성 경로).
    unsafe { RegisterClassW(&wc) };

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            class.as_ptr(),
            wide("SPECTRA").as_ptr(),
            WS_POPUP,
            0,
            0,
            1, // 숨겨진 초기 크기. 최초 redraw에서 실제 텍스트 크기로 교체한다.
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance as _,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err("strip window could not be created".to_string());
    }
    let boxed = Box::into_raw(Box::new(state));
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, boxed as isize) };
    let initialized = (|| -> Result<(), String> {
        let message = unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) };
        if message == 0 { return Err(win32_error("RegisterWindowMessageW TaskbarCreated")); }
        unsafe { (*boxed).taskbar_created = message; }
        SHELL_PENDING.with(|pending| pending.set(false));
        SHELL_TARGET.with(|target| target.set(hwnd));
        for (index, event) in [EVENT_SYSTEM_FOREGROUND, EVENT_OBJECT_LOCATIONCHANGE].into_iter().enumerate() {
            let hook = register(event)?;
            unsafe { (*boxed).hooks[index] = hook; }
        }
        Ok(())
    })();
    if let Err(error) = initialized {
        // 일부 hook만 등록됐어도 WM_DESTROY가 대상 초기화·hook 해제·상태 해제를 수행한다.
        unsafe { DestroyWindow(hwnd) };
        return Err(error);
    }
    Ok(hwnd)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_THEMECHANGED => {
            if matches!(msg, WM_SETTINGCHANGE | WM_DPICHANGED | WM_THEMECHANGED) {
                let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState;
                if let Some(state) = ptr.as_ref() {
                    state.font_dpi.set(0); // 같은 DPI에서 바뀐 시스템 글꼴도 다시 읽는다.
                }
            }
            // UpdateLayeredWindow/SetWindowPos 중 DPI 메시지가 재진입해도 렌더링을 중첩하지 않는다.
            request_shell_redraw();
            0
        }
        WM_STRIP_REDRAW | WM_STRIP_SHELL_CHANGED => {
            if msg == WM_STRIP_SHELL_CHANGED { SHELL_PENDING.with(|pending| pending.set(false)); }
            if let Err(error) = redraw(hwnd) {
                ShowWindow(hwnd, SW_HIDE);
                eprintln!("spectra: strip redraw failed: {error}");
            }
            0
        }
        WM_LBUTTONUP => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState;
            if let Some(state) = ptr.as_ref() {
                (state.on_click)();
            }
            0
        }
        WM_DESTROY => {
            if SHELL_TARGET.with(Cell::get) == hwnd {
                SHELL_TARGET.with(|target| target.set(std::ptr::null_mut()));
                SHELL_PENDING.with(|pending| pending.set(false));
            }
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StripState;
            if !ptr.is_null() {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(ptr));
            }
            PostQuitMessage(0);
            0
        }
        _ => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState;
            if let Some(state) = ptr.as_ref() {
                if state.taskbar_created != 0 && msg == state.taskbar_created {
                    request_shell_redraw(); // Explorer HWND/위치를 캐시하지 않고 다시 찾는다.
                    return 0;
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

fn win32_error(operation: &str) -> String {
    format!("{operation} failed: {}", std::io::Error::last_os_error())
}

fn shell_font(dpi: u32) -> Result<HFONT, String> {
    if dpi == 0 {
        return Err("strip DPI is unavailable".to_string());
    }
    let mut metrics: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
    metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
    let ok = unsafe {
        SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, metrics.cbSize, &mut metrics as *mut _ as *mut _, 0, dpi)
    };
    let mut font: LOGFONTW = if ok != 0 { metrics.lfStatusFont } else { unsafe { std::mem::zeroed() } };
    if ok == 0 {
        let height = dpi.checked_mul(12).and_then(|value| i32::try_from(value / 96).ok())
            .filter(|height| *height > 0).ok_or("strip fallback font size is invalid")?;
        font.lfHeight = -height;
    }
    // 검은 DIB와 섞인 안티앨리어싱 픽셀이 알파 복원 후 후광으로 남지 않게 한다.
    font.lfQuality = NONANTIALIASED_QUALITY;
    let handle = unsafe { CreateFontIndirectW(&font) };
    if handle.is_null() { Err(win32_error("CreateFontIndirectW")) } else { Ok(handle) }
}

fn bitmap_byte_len(width: i32, height: i32) -> Result<usize, String> {
    if width <= 0 || height <= 0 {
        return Err("strip bitmap dimensions must be positive".to_string());
    }
    (width as usize).checked_mul(height as usize).and_then(|size| size.checked_mul(4))
        .filter(|size| *size <= isize::MAX as usize)
        .ok_or_else(|| "strip bitmap dimensions overflow".to_string())
}

/// 부분 생성에 실패해도 확보한 GDI 객체를 생성 스레드에서 정리한다.
struct DibSurface {
    screen: HDC,
    dc: HDC,
    bitmap: HBITMAP,
    old_bitmap: HGDIOBJ,
    pixels: *mut std::ffi::c_void,
}

impl DibSurface {
    fn for_size(width: i32, height: i32) -> Result<Self, String> {
        bitmap_byte_len(width, height)?;
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        info.bmiHeader.biHeight = -height; // top-down 32bpp
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        Self::new(&info)
    }

    fn new(info: &BITMAPINFO) -> Result<Self, String> {
        let mut surface = Self {
            screen: std::ptr::null_mut(),
            dc: std::ptr::null_mut(),
            bitmap: std::ptr::null_mut(),
            old_bitmap: std::ptr::null_mut(),
            pixels: std::ptr::null_mut(),
        };
        unsafe {
            surface.screen = GetDC(std::ptr::null_mut());
            if surface.screen.is_null() {
                return Err(win32_error("GetDC"));
            }
            surface.dc = CreateCompatibleDC(surface.screen);
            if surface.dc.is_null() {
                return Err(win32_error("CreateCompatibleDC"));
            }
            surface.bitmap = CreateDIBSection(surface.dc, info, DIB_RGB_COLORS, &mut surface.pixels, std::ptr::null_mut(), 0);
            if surface.bitmap.is_null() || surface.pixels.is_null() {
                return Err(win32_error("CreateDIBSection"));
            }
            surface.old_bitmap = SelectObject(surface.dc, surface.bitmap as _);
            if surface.old_bitmap.is_null() {
                return Err(win32_error("SelectObject"));
            }
        }
        Ok(surface)
    }

    fn select_font(&self, font: HFONT) -> Result<SelectedFont<'_>, String> {
        let previous = unsafe { SelectObject(self.dc, font as _) };
        if previous.is_null() {
            Err(win32_error("SelectObject font"))
        } else {
            Ok(SelectedFont { surface: self, previous })
        }
    }
}

struct SelectedFont<'a> {
    surface: &'a DibSurface,
    previous: HGDIOBJ,
}

impl Drop for SelectedFont<'_> {
    fn drop(&mut self) {
        unsafe { SelectObject(self.surface.dc, self.previous) };
    }
}

impl Drop for DibSurface {
    fn drop(&mut self) {
        unsafe {
            if !self.old_bitmap.is_null() {
                SelectObject(self.dc, self.old_bitmap);
            }
            // 복원에 실패해도 비트맵이 선택된 채 남지 않도록 DC를 먼저 해제한다.
            if !self.dc.is_null() {
                DeleteDC(self.dc);
            }
            if !self.bitmap.is_null() {
                DeleteObject(self.bitmap as _);
            }
            if !self.screen.is_null() {
                ReleaseDC(std::ptr::null_mut(), self.screen);
            }
        }
    }
}

const CI_MASK_SIDE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProviderCi { Codex, Claude }

impl ProviderCi {
    fn for_label(label: &str) -> Option<Self> {
        match label { "Codex" => Some(Self::Codex), "Claude" => Some(Self::Claude), _ => None }
    }

    fn mask(self) -> &'static [u8; CI_MASK_SIDE * CI_MASK_SIDE] {
        match self {
            Self::Codex => include_bytes!("../assets/provider-ci/codex.alpha"),
            Self::Claude => include_bytes!("../assets/provider-ci/claude.alpha"),
        }
    }

    fn color(self, theme: StripTheme) -> [u8; 3] {
        match (self, theme) {
            (Self::Codex, StripTheme::Light) => [0, 0, 0],
            (Self::Codex, StripTheme::Dark) => [255, 255, 255],
            (Self::Claude, _) => [0xD9, 0x77, 0x57],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum RunContent { Text(String), Ci(ProviderCi) }

struct TextRun {
    content: RunContent,
    color: [u8; 3],
    width: i32,
}

struct TextLayout {
    runs: Vec<TextRun>,
    width: i32,
    height: i32,
}

fn measure_run(dc: HDC, text: &str) -> Result<(i32, i32), String> {
    if text.is_empty() { return Ok((0, 0)); }
    let text = wide(text);
    let mut rc = RECT { left: 0, top: 0, right: 4096, bottom: 0 };
    if unsafe { DrawTextW(dc, text.as_ptr(), -1, &mut rc, DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT) } == 0 {
        return Err(win32_error("DrawTextW measurement"));
    }
    Ok((rc.right, rc.bottom))
}

fn measure_layout(model: &StripModel, theme: StripTheme, font: HFONT, dpi: u32) -> Result<TextLayout, String> {
    let colors = palette(theme);
    let icon_size = dpi.checked_mul(16).and_then(|value| i32::try_from(value / 96).ok())
        .filter(|size| *size > 0).ok_or("strip CI size is invalid")?;
    let measure = DibSurface::for_size(1, 1)?;
    let _font = measure.select_font(font)?;
    let mut runs = Vec::new();
    let mut width: i32 = 0;
    let mut height: i32 = 0;
    let mut append = |content: RunContent, color| -> Result<(), String> {
        let (run_width, run_height) = match &content {
            RunContent::Text(text) => measure_run(measure.dc, text)?,
            RunContent::Ci(_) => (icon_size, icon_size),
        };
        if run_width < 0 || run_height < 0 {
            return Err("strip text measurement is invalid".to_string());
        }
        width = width.checked_add(run_width).ok_or("strip text width overflow")?;
        height = height.max(run_height);
        runs.push(TextRun { content, color, width: run_width });
        Ok(())
    };
    for (index, segment) in model.segments.iter().enumerate() {
        if index > 0 { append(RunContent::Text(" · ".into()), colors.separator)?; }
        if let Some(ci) = ProviderCi::for_label(&segment.label) {
            append(RunContent::Ci(ci), ci.color(theme))?;
        } else {
            append(RunContent::Text(segment.label.clone()), colors.label)?;
        }
        append(RunContent::Text(" ".into()), colors.label)?;
        append(RunContent::Text(segment.value.clone()), segment.value_color)?;
    }
    let padding = dpi.checked_mul(4).and_then(|value| i32::try_from(value / 96).ok())
        .and_then(|value| value.checked_mul(2)).ok_or("strip text padding overflow")?;
    height = height.checked_add(padding).ok_or("strip text height overflow")?;
    bitmap_byte_len(width, height)?;
    Ok(TextLayout { runs, width, height })
}

fn draw_run(dc: HDC, run: &TextRun, x: i32, height: i32) -> Result<(), String> {
    let RunContent::Text(text) = &run.content else { return Ok(()) };
    if text.is_empty() { return Ok(()); }
    let text = wide(text);
    let right = x.checked_add(run.width).ok_or("strip text position overflow")?;
    let mut rc = RECT { left: x, top: 0, right, bottom: height };
    let color = (run.color[0] as u32) | ((run.color[1] as u32) << 8) | ((run.color[2] as u32) << 16);
    unsafe {
        if SetTextColor(dc, color) == CLR_INVALID { return Err(win32_error("SetTextColor")); }
        if SetBkMode(dc, TRANSPARENT as i32) == 0 { return Err(win32_error("SetBkMode")); }
        if DrawTextW(dc, text.as_ptr(), -1, &mut rc, DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX) == 0 {
            return Err(win32_error("DrawTextW drawing"));
        }
    }
    Ok(())
}

/// CI의 가장자리 알파를 보존한다. GDI 텍스트 알파 복원이 끝난 뒤 빈 CI 영역에 합성한다.
fn draw_ci(buffer: &mut [u8], stride: usize, x: usize, height: usize, size: usize, ci: ProviderCi, rgb: [u8; 3]) {
    let mask = ci.mask();
    let top = (height - size) / 2;
    for dy in 0..size {
        let sy = ((dy as f64 + 0.5) * CI_MASK_SIDE as f64 / size as f64 - 0.5)
            .clamp(0.0, (CI_MASK_SIDE - 1) as f64);
        let y0 = sy.floor() as usize;
        let y1 = (y0 + 1).min(CI_MASK_SIDE - 1);
        let fy = sy - y0 as f64;
        for dx in 0..size {
            let sx = ((dx as f64 + 0.5) * CI_MASK_SIDE as f64 / size as f64 - 0.5)
                .clamp(0.0, (CI_MASK_SIDE - 1) as f64);
            let x0 = sx.floor() as usize;
            let x1 = (x0 + 1).min(CI_MASK_SIDE - 1);
            let fx = sx - x0 as f64;
            let upper = mask[y0 * CI_MASK_SIDE + x0] as f64 * (1.0 - fx) + mask[y0 * CI_MASK_SIDE + x1] as f64 * fx;
            let lower = mask[y1 * CI_MASK_SIDE + x0] as f64 * (1.0 - fx) + mask[y1 * CI_MASK_SIDE + x1] as f64 * fx;
            let alpha = (upper * (1.0 - fy) + lower * fy).round() as u8;
            let pixel = &mut buffer[((top + dy) * stride + x + dx) * 4..][..4];
            for (channel, color) in pixel[..3].iter_mut().zip(rgb.into_iter().rev()) {
                *channel = ((color as u16 * alpha as u16 + 127) / 255) as u8;
            }
            pixel[3] = alpha;
        }
    }
}

struct RenderedStrip {
    surface: DibSurface,
    layout: TextLayout,
}

fn render_model(model: &StripModel, theme: StripTheme, font: HFONT, dpi: u32) -> Result<RenderedStrip, String> {
    let layout = measure_layout(model, theme, font, dpi)?;
    let byte_len = bitmap_byte_len(layout.width, layout.height)?;
    let surface = DibSurface::for_size(layout.width, layout.height)?;
    // 투명 배경을 초기화하고 GDI가 그리는 동안 Rust 가변 슬라이스를 유지하지 않는다.
    unsafe { std::ptr::write_bytes(surface.pixels as *mut u8, 0, byte_len) };
    {
        let _font = surface.select_font(font)?;
        let mut x: i32 = 0;
        for run in &layout.runs {
            draw_run(surface.dc, run, x, layout.height)?;
            x = x.checked_add(run.width).ok_or("strip text position overflow")?;
        }
        debug_assert_eq!(x, layout.width);
        if unsafe { GdiFlush() } == 0 { return Err(win32_error("GdiFlush")); }
    } // 창 API의 동기 재진입 전에 셸 글꼴 선택을 복원한다.
    unsafe {
        let buffer = std::slice::from_raw_parts_mut(surface.pixels as *mut u8, byte_len);
        for pixel in buffer.chunks_exact_mut(4) {
            pixel[3] = if pixel[..3].iter().any(|value| *value != 0) { 255 } else { 0 };
        }
        let mut x = 0;
        for run in &layout.runs {
            if let RunContent::Ci(ci) = &run.content {
                draw_ci(buffer, layout.width as usize, x, layout.height as usize, run.width as usize, *ci, run.color);
            }
            x += run.width as usize;
        }
    }
    Ok(RenderedStrip { surface, layout })
}

fn rect_covers_monitor(window: RECT, monitor: RECT) -> bool {
    monitor.right > monitor.left && monitor.bottom > monitor.top
        && window.left <= monitor.left && window.top <= monitor.top
        && window.right >= monitor.right && window.bottom >= monitor.bottom
}

fn foreground_covers_taskbar_monitor(strip: HWND) -> bool {
    unsafe {
        let foreground = GetForegroundWindow();
        let taskbar = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        if foreground.is_null() || taskbar.is_null() || foreground == strip || foreground == taskbar
            || foreground == GetDesktopWindow() || foreground == GetShellWindow()
            || IsWindowVisible(foreground) == 0 || IsIconic(foreground) != 0 {
            return false;
        }
        // 바탕 화면 클릭 시 전경이 되는 보조 shell 창도 전체화면 앱으로 오인하지 않는다.
        let mut class = [0u16; 64];
        let len = GetClassNameW(foreground, class.as_mut_ptr(), class.len() as i32);
        if len > 0 && matches!(String::from_utf16_lossy(&class[..len as usize]).as_str(),
            "Progman" | "WorkerW" | "Shell_SecondaryTrayWnd") {
            return false;
        }
        let monitor = MonitorFromWindow(taskbar, MONITOR_DEFAULTTONULL);
        if monitor.is_null() || MonitorFromWindow(foreground, MONITOR_DEFAULTTONULL) != monitor {
            return false; // 다른 모니터의 일반 borderless/F11 전체화면은 제외한다.
        }
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        let mut window: RECT = std::mem::zeroed();
        if GetMonitorInfoW(monitor, &mut info) == 0 || GetWindowRect(foreground, &mut window) == 0 {
            return false;
        }
        // rcWork가 아닌 전체 모니터와 비교한다. 일반 최대화의 보이지 않는 테두리도
        // 작업표시줄 높이를 덮지는 않으므로 전체화면으로 오인하지 않는다.
        rect_covers_monitor(window, info.rcMonitor)
    }
}

fn should_hide(strip: HWND) -> bool {
    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    if unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) } & ABS_AUTOHIDE as usize != 0 {
        return true; // v1: 자동숨김 옵션이 켜진 동안 계속 숨긴다.
    }
    let mut notification = 0;
    if unsafe { SHQueryUserNotificationState(&mut notification) } == 0
        && matches!(notification, QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE) {
        return true;
    }
    foreground_covers_taskbar_monitor(strip)
}

/// CI와 셸 글꼴로 측정한 크기의 투명 스트립을 작업표시줄 위에 올린다.
fn redraw(hwnd: HWND) -> Result<(), String> {
    let state = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState).as_ref() };
    let Some(state) = state else { return Ok(()) };
    let model = state.model.lock().ok().and_then(|slot| slot.clone());
    let Some(mut model) = model else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return Ok(());
    };
    let (Some((taskbar, edge)), Some(tray)) = (taskbar_rect(), tray_left()) else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return Ok(());
    };
    // 지원하지 않는 세로 작업표시줄에서는 렌더링 자체를 건너뛴다.
    if matches!(edge, TaskbarEdge::Left | TaskbarEdge::Right) || should_hide(hwnd) {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return Ok(());
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let font = state.font_for_dpi(dpi)?;
    let theme = current_theme();
    retheme_model(&mut model, theme);
    let rendered = render_model(&model, theme, font, dpi)?;
    let (width, height) = (rendered.layout.width, rendered.layout.height);
    let Some((x, y)) = place_strip(taskbar, edge, tray, (width, height)) else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return Ok(());
    };
    let surface = &rendered.surface;
    unsafe {
        let mut position = POINT { x, y };
        let mut size = windows_sys::Win32::Foundation::SIZE { cx: width, cy: height };
        let mut source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        if UpdateLayeredWindow(hwnd, surface.screen, &mut position, &mut size, surface.dc, &mut source, 0 as COLORREF, &blend, ULW_ALPHA) == 0 {
            return Err(win32_error("UpdateLayeredWindow"));
        }
        if SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, SWP_NOACTIVATE | SWP_SHOWWINDOW) == 0 {
            return Err(win32_error("SetWindowPos"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskbar_strip::StripSegment;
    use windows_sys::Win32::Graphics::Gdi::{GetCurrentObject, GetObjectW, OBJ_BITMAP, OBJ_FONT};
    use windows_sys::Win32::UI::HiDpi::{
        AreDpiAwarenessContextsEqual, GetWindowDpiAwarenessContext, DPI_AWARENESS_CONTEXT_UNAWARE,
    };

    #[test]
    fn fullscreen_geometry_distinguishes_f11_from_normal_maximization() {
        let monitor = RECT { left: 0, top: 0, right: 1920, bottom: 1080 };
        assert!(rect_covers_monitor(monitor, monitor), "F11 has no non-client frame");
        assert!(rect_covers_monitor(RECT { left: -8, top: -8, right: 1928, bottom: 1088 }, monitor));
        assert!(!rect_covers_monitor(RECT { left: -8, top: -8, right: 1928, bottom: 1040 }, monitor),
            "ordinary maximization includes invisible borders but leaves the taskbar free");
        assert!(!rect_covers_monitor(RECT { left: -8, top: 40, right: 1928, bottom: 1088 }, monitor),
            "ordinary maximization also leaves a top taskbar free");
        assert!(!rect_covers_monitor(RECT { left: 0, top: 0, right: 1919, bottom: 1080 }, monitor));
    }

    #[test]
    fn fullscreen_geometry_handles_negative_coordinates_and_other_monitors() {
        let monitor = RECT { left: -2560, top: -400, right: 0, bottom: 1040 };
        assert!(rect_covers_monitor(monitor, monitor));
        assert!(!rect_covers_monitor(RECT { left: 0, top: 0, right: 1920, bottom: 1080 }, monitor));
        assert!(!rect_covers_monitor(RECT { left: -2560, top: -400, right: 0, bottom: 980 }, monitor));
        assert!(!rect_covers_monitor(monitor, RECT { left: 0, top: 0, right: 0, bottom: 0 }));
    }

    unsafe fn discard_quit_message() {
        let mut message: MSG = std::mem::zeroed();
        PeekMessageW(&mut message, std::ptr::null_mut(), WM_QUIT, WM_QUIT, PM_REMOVE);
    }

    #[test]
    #[ignore = "requires a Windows desktop; run with --ignored --test-threads=1"]
    fn shell_hooks_release_after_ten_window_lifecycles() {
        std::thread::spawn(|| unsafe {
            for cycle in 1..=10 {
                let hwnd = create_window(StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {}))).unwrap();
                let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState);
                let hooks = state.hooks;
                assert!(hooks.iter().all(|hook| !hook.is_null()));
                assert_eq!(SHELL_TARGET.with(Cell::get), hwnd);
                assert!(state.taskbar_created >= 0xc000);
                request_shell_redraw();
                assert!(SHELL_PENDING.with(Cell::get));
                assert_ne!(DestroyWindow(hwnd), 0);
                assert_eq!(IsWindow(hwnd), 0);
                assert!(SHELL_TARGET.with(Cell::get).is_null());
                assert!(!SHELL_PENDING.with(Cell::get));
                for hook in hooks {
                    assert_eq!(UnhookWinEvent(hook), 0, "hook must already be released in cycle {cycle}");
                }
                discard_quit_message();
            }
            eprintln!("10 strip HWND cycles: both hooks released and callback targets cleared");
        }).join().unwrap();
    }

    #[test]
    #[ignore = "requires a Windows desktop; run with --ignored --test-threads=1"]
    fn shell_hook_registration_failures_destroy_partial_state_and_allow_retry() {
        std::thread::spawn(|| unsafe {
            for fail_at in [0, 1] {
                let mut registered = Vec::new();
                let mut attempted_hwnd = std::ptr::null_mut();
                let result = create_window_with_hooks(
                    StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {})),
                    |event| {
                        attempted_hwnd = SHELL_TARGET.with(Cell::get);
                        if registered.len() == fail_at { return Err("injected hook registration failure".into()); }
                        let hook = register_win_event(event)?;
                        registered.push(hook);
                        Ok(hook)
                    },
                );
                assert_eq!(result.unwrap_err(), "injected hook registration failure");
                assert!(!attempted_hwnd.is_null());
                assert_eq!(IsWindow(attempted_hwnd), 0);
                assert!(SHELL_TARGET.with(Cell::get).is_null());
                assert!(!SHELL_PENDING.with(Cell::get));
                for hook in registered { assert_eq!(UnhookWinEvent(hook), 0, "release partial registration"); }
                discard_quit_message();
            }
            let hwnd = create_window(StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {}))).unwrap();
            assert_ne!(DestroyWindow(hwnd), 0, "retry succeeds after failed setup");
            eprintln!("first/second hook failure injection: HWND and hooks released; real registration retry succeeded");
        }).join().unwrap();
    }

    #[test]
    #[ignore = "requires a Windows desktop; run with --ignored --test-threads=1"]
    fn shell_callbacks_filter_coalesce_and_recover_from_a_failed_post() {
        std::thread::spawn(|| unsafe {
            let model = Arc::new(Mutex::new(None));
            let hwnd = create_window(StripState::new(Arc::clone(&model), Box::new(|| {}))).unwrap();
            let invoke = |event, window, object, child| {
                on_win_event(std::ptr::null_mut(), event, window, object, child, 0, 0);
            };
            // 콜백은 앱/모델 잠금을 잡지 않는다. NULL 전경 전환도 한 번만 큐에 넣는다.
            {
                let _locked = model.lock().unwrap();
                invoke(EVENT_SYSTEM_FOREGROUND, hwnd, OBJID_WINDOW, 0);
                invoke(EVENT_OBJECT_LOCATIONCHANGE, GetForegroundWindow(), OBJID_CLIENT, 0);
                invoke(EVENT_OBJECT_LOCATIONCHANGE, GetForegroundWindow(), OBJID_WINDOW, 1);
                invoke(EVENT_OBJECT_LOCATIONCHANGE, std::ptr::null_mut(), OBJID_WINDOW, 0);
                assert!(!SHELL_PENDING.with(Cell::get), "ignore own/child/non-window location events");
                for _ in 0..100 { invoke(EVENT_SYSTEM_FOREGROUND, std::ptr::null_mut(), OBJID_WINDOW, 0); }
                assert!(SHELL_PENDING.with(Cell::get));
            }
            let mut message: MSG = std::mem::zeroed();
            assert_ne!(PeekMessageW(&mut message, hwnd, WM_STRIP_SHELL_CHANGED, WM_STRIP_SHELL_CHANGED, PM_REMOVE), 0);
            assert_eq!(message.message, WM_STRIP_SHELL_CHANGED);
            let mut duplicate: MSG = std::mem::zeroed();
            assert_eq!(PeekMessageW(&mut duplicate, hwnd, WM_STRIP_SHELL_CHANGED, WM_STRIP_SHELL_CHANGED, PM_REMOVE), 0);
            DispatchMessageW(&message);
            assert!(!SHELL_PENDING.with(Cell::get), "message handling permits the next event");
            assert_eq!(IsWindowVisible(hwnd), 0, "empty-cache redraw stays hidden with hooks alive");
            let foreground = GetForegroundWindow();
            assert!(!foreground.is_null());
            invoke(EVENT_OBJECT_LOCATIONCHANGE, foreground, OBJID_WINDOW, 0);
            assert!(SHELL_PENDING.with(Cell::get), "foreground resize is observed while hidden");
            assert_ne!(PeekMessageW(&mut message, hwnd, WM_STRIP_SHELL_CHANGED, WM_STRIP_SHELL_CHANGED, PM_REMOVE), 0);
            DispatchMessageW(&message);
            let state = &*(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState);
            let font = state.font_for_dpi(96).unwrap();
            SendMessageW(hwnd, WM_SETTINGCHANGE, 0, 0);
            assert_eq!(state.font_dpi.get(), 0, "settings invalidate the font without deleting a selected object");
            assert_eq!(state.font.get(), font);
            SendMessageW(hwnd, WM_DISPLAYCHANGE, 0, 0);
            SendMessageW(hwnd, WM_DPICHANGED, 0, 0);
            SendMessageW(hwnd, state.taskbar_created, 0, 0);
            assert_ne!(PeekMessageW(&mut message, hwnd, WM_STRIP_SHELL_CHANGED, WM_STRIP_SHELL_CHANGED, PM_REMOVE), 0);
            assert_eq!(PeekMessageW(&mut duplicate, hwnd, WM_STRIP_SHELL_CHANGED, WM_STRIP_SHELL_CHANGED, PM_REMOVE), 0);
            DispatchMessageW(&message);
            assert_ne!(DestroyWindow(hwnd), 0);
            invoke(EVENT_SYSTEM_FOREGROUND, std::ptr::null_mut(), OBJID_WINDOW, 0);
            assert!(!SHELL_PENDING.with(Cell::get), "late callback after destroy does nothing");
            // 파기된 우리 HWND로 PostMessage 실패를 일으킨 뒤 pending이 풀리는지 확인한다.
            SHELL_TARGET.with(|target| target.set(hwnd));
            request_shell_redraw();
            assert!(!SHELL_PENDING.with(Cell::get), "failed post must not latch pending");
            SHELL_TARGET.with(|target| target.set(std::ptr::null_mut()));
            eprintln!("callback filters, 100-to-1 coalescing, hidden resize, shell messages, teardown and post failure: PASS");
        }).join().unwrap();
    }

    // 실제 Win32 API를 사용하는 명시 실행용 검사. 육안 배치 검증을 대신하지 않는다.
    #[test]
    #[ignore = "requires a Windows desktop; run with --ignored --test-threads=1"]
    fn strip_window_sets_dpi_before_creation_without_tauri() {
        std::thread::spawn(|| unsafe {
            assert!(!SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_UNAWARE).is_null());
            let hwnd = create_window(StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {})))
                .expect("hidden strip window");
            let matches = AreDpiAwarenessContextsEqual(
                GetWindowDpiAwarenessContext(hwnd),
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
            );
            let destroyed = DestroyWindow(hwnd);
            assert_ne!(matches, 0, "the HWND must use per-monitor-v2 DPI");
            assert_ne!(destroyed, 0);
        }).join().unwrap();
    }

    #[test]
    #[ignore = "requires a Windows desktop; run with --ignored --test-threads=1"]
    fn dib_surface_handles_failed_creation_and_a_valid_retry() {
        let mut info: BITMAPINFO = unsafe { std::mem::zeroed() };
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 0; // CreateDIBSection 실패를 실제로 발생시킨다.
        info.bmiHeader.biHeight = -2;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let error = DibSurface::new(&info).err().expect("zero-width DIB must fail");
        assert!(error.starts_with("CreateDIBSection failed:"), "{error}");

        info.bmiHeader.biWidth = 2;
        let surface = DibSurface::new(&info).expect("valid DIB after failure");
        unsafe {
            assert_eq!(GetCurrentObject(surface.dc, OBJ_BITMAP as u32), surface.bitmap as _);
            let pixels = std::slice::from_raw_parts_mut(surface.pixels as *mut u8, 2 * 2 * 4);
            pixels.fill(0x7f);
            assert_eq!(pixels, &[0x7f; 16]);
        }
    }

    #[test]
    #[ignore = "requires Windows GDI; run with --ignored --test-threads=1"]
    fn shell_font_cache_reuses_replaces_and_releases_handles() {
        let read_font = |handle: HFONT| -> Option<LOGFONTW> {
            let mut font: LOGFONTW = unsafe { std::mem::zeroed() };
            let bytes = unsafe {
                GetObjectW(handle as _, std::mem::size_of::<LOGFONTW>() as i32, &mut font as *mut _ as *mut _)
            };
            (bytes == std::mem::size_of::<LOGFONTW>() as i32).then_some(font)
        };
        let state = StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {}));
        let first = state.font_for_dpi(96).expect("96-DPI shell font");
        assert_eq!(state.font_for_dpi(96).unwrap(), first, "same DPI reuses the font");
        assert!(state.font_for_dpi(0).is_err());
        assert_eq!(state.font.get(), first, "failed replacement retains the font");
        let scaled = state.font_for_dpi(144).expect("144-DPI shell font");
        unsafe {
            assert_ne!(GdiFlush(), 0, "complete pending GDI operations before inspecting handles");
            // GetObjectType은 이 환경에서 삭제된 핸들의 종류도 반환하므로 실제 조회로 확인한다.
            assert!(read_font(first).is_none(), "old font must be released");
            let font = read_font(scaled).expect("live font properties");
            let mut metrics: NONCLIENTMETRICSW = std::mem::zeroed();
            metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
            assert_ne!(SystemParametersInfoForDpi(
                SPI_GETNONCLIENTMETRICS, metrics.cbSize, &mut metrics as *mut _ as *mut _, 0, 144,
            ), 0);
            assert_eq!(font.lfHeight, metrics.lfStatusFont.lfHeight, "use the scaled shell height");
            assert_eq!(font.lfFaceName, metrics.lfStatusFont.lfFaceName, "use the shell font family");
            assert_eq!(font.lfQuality, NONANTIALIASED_QUALITY);
        }
        state.font_dpi.set(0); // WM_SETTINGCHANGE invalidates a same-DPI font.
        let refreshed = state.font_for_dpi(144).expect("font after settings change");
        assert_ne!(unsafe { GdiFlush() }, 0);
        assert!(read_font(scaled).is_none());
        assert!(read_font(refreshed).is_some());
        drop(state);
        assert_ne!(unsafe { GdiFlush() }, 0);
        assert!(read_font(refreshed).is_none(), "state drop releases the last font");
    }

    #[test]
    #[ignore = "requires Windows GDI; run with --ignored --test-threads=1"]
    fn ci_rendering_preserves_values_colors_transparency_and_reduces_width() {
        let state = StripState::new(Arc::new(Mutex::new(None)), Box::new(|| {}));
        for theme in [StripTheme::Light, StripTheme::Dark] {
            let colors = palette(theme);
            for (values, value_colors) in [
                (["55%", "59%"], [colors.high, colors.high]),
                (["0%", "100%"], [colors.low, colors.high]),
                (["—", "31%"], [colors.label, colors.mid]),
                (["8%", "—"], [colors.low, colors.label]),
            ] {
                let model = StripModel {
                    segments: ["Codex", "Claude"].into_iter().enumerate().map(|(index, label)| StripSegment {
                        label: label.into(), value: values[index].into(), value_color: value_colors[index],
                    }).collect(),
                    tooltip: String::new(),
                };
                let mut previous_size = (0, 0);
                for dpi in [96, 120, 144] {
                    let font = state.font_for_dpi(dpi).unwrap();
                    let rendered = render_model(&model, theme, font, dpi).expect("offscreen text render");
                    assert_ne!(unsafe { GetCurrentObject(rendered.surface.dc, OBJ_FONT as u32) }, font as _, "restore the DC font before window API calls");
                    let layout = &rendered.layout;
                    let (width, height) = (layout.width as usize, layout.height as usize);
                    assert!(layout.width > previous_size.0 && layout.height > previous_size.1, "DPI must scale the text");
                    previous_size = (layout.width, layout.height);
                    let pixels = unsafe {
                        std::slice::from_raw_parts(rendered.surface.pixels as *const u8, width * height * 4)
                    };
                    let expected_runs = [
                        (RunContent::Ci(ProviderCi::Codex), ProviderCi::Codex.color(theme)),
                        (RunContent::Text(" ".into()), colors.label),
                        (RunContent::Text(values[0].into()), value_colors[0]),
                        (RunContent::Text(" · ".into()), colors.separator),
                        (RunContent::Ci(ProviderCi::Claude), ProviderCi::Claude.color(theme)),
                        (RunContent::Text(" ".into()), colors.label),
                        (RunContent::Text(values[1].into()), value_colors[1]),
                    ];
                    assert_eq!(layout.runs.len(), expected_runs.len());
                    let mut x = 0;
                    for (run, (content, rgb)) in layout.runs.iter().zip(expected_runs) {
                        assert_eq!(run.content, content);
                        let bgra = [rgb[2], rgb[1], rgb[0], 255];
                        let mut ink = 0;
                        let mut partial_alpha = 0;
                        for row in pixels.chunks_exact(width * 4) {
                            for pixel in row[x * 4..(x + run.width as usize) * 4].chunks_exact(4) {
                                if pixel[3] == 0 {
                                    assert_eq!(pixel, &[0, 0, 0, 0], "transparent pixels must be clear");
                                } else {
                                    match &content {
                                        RunContent::Text(_) => assert_eq!(pixel, &bgra, "text color changed in {content:?}"),
                                        RunContent::Ci(_) => {
                                            for (channel, color) in pixel[..3].iter().zip(rgb.into_iter().rev()) {
                                                assert_eq!(*channel, ((color as u16 * pixel[3] as u16 + 127) / 255) as u8, "CI must use premultiplied alpha");
                                            }
                                            partial_alpha += usize::from(pixel[3] < 255);
                                        }
                                    }
                                    ink += 1;
                                }
                            }
                        }
                        match &content {
                            RunContent::Text(text) => assert_eq!(ink > 0, !text.trim().is_empty(), "missing or stray text in {content:?} at {dpi} DPI"),
                            RunContent::Ci(_) => {
                                assert_eq!(run.width, (16 * dpi / 96) as i32);
                                assert!(ink > 0 && partial_alpha > 0, "CI must remain visible with smooth transparent edges");
                            }
                        }
                        x += run.width as usize;
                    }
                    assert_eq!(x, width, "all runs must fit in the measured width");
                    let padding = (4 * dpi / 96) as usize;
                    assert!(pixels[..padding * width * 4].iter().all(|byte| *byte == 0), "top padding must be transparent");
                    assert!(pixels[(height - padding) * width * 4..].iter().all(|byte| *byte == 0), "bottom padding must be transparent");
                    let legacy = format!("Codex {} · Claude {}", values[0], values[1]);
                    let measure = DibSurface::for_size(1, 1).unwrap();
                    let _font = measure.select_font(font).unwrap();
                    let legacy_width = measure_run(measure.dc, &legacy).unwrap().0;
                    assert!(layout.width < legacy_width, "CI layout must be narrower than service names");
                    eprintln!("{theme:?} {dpi} DPI: {} / {} = {width}x{height}, names={legacy_width}px", values[0], values[1]);
                    if values == ["55%", "59%"] {
                        if let Some(root) = std::env::var_os("SPECTRA_STRIP_PREVIEW_DIR") {
                            let path = std::path::PathBuf::from(root).join(format!("{theme:?}-{dpi}-{width}x{height}.bgra"));
                            std::fs::write(path, pixels).expect("offscreen preview bytes");
                        }
                    }
                }
            }
        }
    }
}
