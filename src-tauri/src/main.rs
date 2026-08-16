#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Release builds run under the Windows GUI subsystem so the tray app never
// flashes a console window.
fn main() {
    if spectra_lib::run_cli_mode() {
        return;
    }
    spectra_lib::run();
}
