use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir is required"));
    let icons_dir = manifest_dir.join("icons");
    fs::create_dir_all(&icons_dir).expect("icon directory could not be created");
    // A tiny valid RGBA PNG lets `generate_context!` run in a fresh checkout.
    // The final product icon should replace this generated placeholder before
    // packaging; the directory is ignored so no binary asset is committed.
    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("PNG header could not be written");
        writer
            .write_image_data(&[32, 160, 255, 255])
            .expect("PNG pixel could not be written");
    }
    fs::write(icons_dir.join("icon.png"), png_data).expect("generated PNG could not be written");

    if env::var("CARGO_CFG_TARGET_OS").is_ok_and(|target| target == "windows") {
        let icon_path = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is required"))
            .join("spectra-generated.ico");
        // A tiny valid 1x1 ICO keeps the native scaffold buildable without
        // committing a binary brand asset. The product icon can replace this
        // generated placeholder before packaging.
        const ICON: &[u8] = &[
            0, 0, 1, 0, 1, 0, 1, 1, 0, 0, 1, 0, 32, 0, 48, 0, 0, 0, 22, 0, 0, 0, 40, 0, 0, 0, 1, 0,
            0, 0, 2, 0, 0, 0, 1, 0, 32, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 32, 160, 255, 255, 0, 0, 0, 0,
        ];
        fs::write(&icon_path, ICON).expect("generated ICO could not be written");
        let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon_path);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("tauri build setup failed");
    } else {
        tauri_build::build();
    }
}
