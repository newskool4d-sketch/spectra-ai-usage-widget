use std::env;
use std::fs;
use std::path::PathBuf;

const ICON_SIZE: u32 = 256;

fn rounded_square_contains(x: u32, y: u32, radius: u32) -> bool {
    let max = ICON_SIZE - 1;
    let corner_x = if x < radius {
        radius - x
    } else if x > max - radius {
        x - (max - radius)
    } else {
        0
    };
    let corner_y = if y < radius {
        radius - y
    } else if y > max - radius {
        y - (max - radius)
    } else {
        0
    };

    corner_x * corner_x + corner_y * corner_y <= radius * radius
}

fn generated_icon_png() -> Vec<u8> {
    let mut pixels = vec![0_u8; (ICON_SIZE * ICON_SIZE * 4) as usize];
    let spectral = [
        [108_u8, 92_u8, 231_u8],
        [71_u8, 145_u8, 255_u8],
        [35_u8, 203_u8, 180_u8],
        [246_u8, 190_u8, 64_u8],
        [239_u8, 99_u8, 141_u8],
    ];

    for y in 0..ICON_SIZE {
        for x in 0..ICON_SIZE {
            let offset = ((y * ICON_SIZE + x) * 4) as usize;
            if !rounded_square_contains(x, y, 46) {
                continue;
            }

            let glow = ((x + (ICON_SIZE - y)) / 32) as u8;
            pixels[offset] = 10 + glow.min(8);
            pixels[offset + 1] = 12 + glow.min(10);
            pixels[offset + 2] = 25 + glow.min(13);
            pixels[offset + 3] = 255;

            let border = x < 4 || y < 4 || x >= ICON_SIZE - 4 || y >= ICON_SIZE - 4;
            if border {
                pixels[offset] = 69;
                pixels[offset + 1] = 74;
                pixels[offset + 2] = 103;
            }

            if (45..=211).contains(&x) {
                for (band, color) in spectral.iter().enumerate() {
                    let center = 58_i32 + band as i32 * 35 - ((x as i32 - 45) * 22 / 166);
                    if (y as i32 - center).abs() <= 8 {
                        pixels[offset] = color[0];
                        pixels[offset + 1] = color[1];
                        pixels[offset + 2] = color[2];
                    }
                }
            }
        }
    }

    let mut png_data = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_data, ICON_SIZE, ICON_SIZE);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .expect("PNG header could not be written");
        writer
            .write_image_data(&pixels)
            .expect("PNG pixels could not be written");
    }
    png_data
}

fn generated_icon_ico(png_data: &[u8]) -> Vec<u8> {
    let mut ico = Vec::with_capacity(22 + png_data.len());
    ico.extend_from_slice(&0_u16.to_le_bytes());
    ico.extend_from_slice(&1_u16.to_le_bytes());
    ico.extend_from_slice(&1_u16.to_le_bytes());
    ico.extend_from_slice(&[0, 0, 0, 0]);
    ico.extend_from_slice(&1_u16.to_le_bytes());
    ico.extend_from_slice(&32_u16.to_le_bytes());
    ico.extend_from_slice(&(png_data.len() as u32).to_le_bytes());
    ico.extend_from_slice(&22_u32.to_le_bytes());
    ico.extend_from_slice(png_data);
    ico
}

fn main() {
    let manifest_dir =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir is required"));
    let icons_dir = manifest_dir.join("icons");
    fs::create_dir_all(&icons_dir).expect("icon directory could not be created");
    let png_data = generated_icon_png();
    let png_path = icons_dir.join("icon.png");
    let ico_path = icons_dir.join("icon.ico");
    fs::write(&png_path, &png_data).expect("generated PNG could not be written");
    fs::write(&ico_path, generated_icon_ico(&png_data))
        .expect("generated ICO could not be written");

    if env::var("CARGO_CFG_TARGET_OS").is_ok_and(|target| target == "windows") {
        let windows = tauri_build::WindowsAttributes::new().window_icon_path(ico_path);
        let attributes = tauri_build::Attributes::new().windows_attributes(windows);
        tauri_build::try_build(attributes).expect("tauri build setup failed");
    } else {
        tauri_build::build();
    }
}
