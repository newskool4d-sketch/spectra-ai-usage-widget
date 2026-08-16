#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Release builds run under the Windows GUI subsystem so the tray app never
// flashes a console. CLI-invoked paths (statusline bridge, --provider-snapshot)
// still need to print into whatever terminal launched them, so we reattach to
// the parent console explicitly rather than relying on subsystem defaults.
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn AttachConsole(dw_process_id: u32) -> i32;
}

#[cfg(windows)]
fn attach_parent_console() {
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
}

#[cfg(not(windows))]
fn attach_parent_console() {}

fn main() {
    attach_parent_console();
    if spectra_lib::run_cli_mode() {
        return;
    }
    spectra_lib::run();
}
