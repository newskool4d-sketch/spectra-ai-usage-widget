use crate::provider_usage::ProviderUsageSnapshot;

pub const BADGE_SIZE: u32 = 32;
const SCALE: u32 = 2;
const GLYPH_W: u32 = 3;
const GLYPH_H: u32 = 5;
const GAP: u32 = 1;
const BACKGROUND: [u8; 4] = [16, 17, 15, 255];
const BORDER: [u8; 4] = [69, 74, 103, 255];
pub const TEXT_HIGH: [u8; 3] = [86, 183, 176];  // --color-cyan
pub const TEXT_MID: [u8; 3] = [199, 168, 82];   // --color-amber
pub const TEXT_LOW: [u8; 3] = [216, 132, 99];   // --color-coral

fn glyph(ch: char) -> Option<[u8; 5]> {
    Some(match ch {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b001, 0b001, 0b001],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '%' => [0b101, 0b001, 0b010, 0b100, 0b101],
        _ => return None,
    })
}

pub fn badge_text(percent: u8) -> String {
    format!("{}%", percent.min(99))
}

fn text_color(percent: u8) -> [u8; 3] {
    if percent >= 50 { TEXT_HIGH } else if percent >= 20 { TEXT_MID } else { TEXT_LOW }
}

fn usable(snapshot: &ProviderUsageSnapshot) -> bool {
    matches!(snapshot.connection_state.as_str(), "connected" | "stale") && !snapshot.windows.is_empty()
}

pub fn min_remaining_percent(snapshots: &[ProviderUsageSnapshot]) -> Option<(String, u8)> {
    snapshots
        .iter()
        .filter(|snapshot| usable(snapshot))
        .filter_map(|snapshot| {
            let window = snapshot.windows.iter().find(|w| w.id == "rolling").or_else(|| snapshot.windows.first())?;
            let percent = window.remaining_percent.round().clamp(0.0, 100.0) as u8;
            Some((snapshot.provider_id.clone(), percent))
        })
        .min_by_key(|(_, percent)| *percent)
}

fn put(pixels: &mut [u8], x: u32, y: u32, rgba: [u8; 4]) {
    if x >= BADGE_SIZE || y >= BADGE_SIZE {
        return;
    }
    let offset = ((y * BADGE_SIZE + x) * 4) as usize;
    pixels[offset..offset + 4].copy_from_slice(&rgba);
}

pub fn render(percent: u8) -> Vec<u8> {
    let mut pixels = vec![0_u8; (BADGE_SIZE * BADGE_SIZE * 4) as usize];
    for y in 0..BADGE_SIZE {
        for x in 0..BADGE_SIZE {
            let edge = x < 2 || y < 2 || x >= BADGE_SIZE - 2 || y >= BADGE_SIZE - 2;
            let corner = (x < 3 || x >= BADGE_SIZE - 3) && (y < 3 || y >= BADGE_SIZE - 3);
            if corner {
                continue;
            }
            put(&mut pixels, x, y, if edge { BORDER } else { BACKGROUND });
        }
    }

    let text = badge_text(percent);
    let glyph_count = text.chars().count() as u32;
    let text_width = glyph_count * GLYPH_W * SCALE + (glyph_count - 1) * GAP * SCALE;
    let origin_x = (BADGE_SIZE - text_width) / 2;
    let origin_y = (BADGE_SIZE - GLYPH_H * SCALE) / 2;
    let color = text_color(percent);
    let rgba = [color[0], color[1], color[2], 255];

    for (index, ch) in text.chars().enumerate() {
        let Some(rows) = glyph(ch) else { continue };
        let glyph_x = origin_x + index as u32 * (GLYPH_W + GAP) * SCALE;
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..GLYPH_W {
                if bits & (0b100 >> col) == 0 {
                    continue;
                }
                for dy in 0..SCALE {
                    for dx in 0..SCALE {
                        put(&mut pixels, glyph_x + col * SCALE + dx, origin_y + row as u32 * SCALE + dy, rgba);
                    }
                }
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

    fn snapshot(provider: &str, state: &str, windows: Vec<(&str, f64)>) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot {
            provider_id: provider.to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: state.to_string(),
            auth_method: None,
            plan_type: None,
            source: Some("codex-app-server".to_string()),
            last_synced_at: None,
            bridge_installed: false,
            windows: windows.into_iter().map(|(id, remaining)| ProviderQuotaWindow {
                id: id.to_string(), label: id.to_string(), used_percent: 100.0 - remaining, remaining_percent: remaining, resets_at: None, window_duration_mins: None,
            }).collect(),
            message: String::new(),
        }
    }

    #[test]
    fn every_badge_character_has_a_glyph() {
        for ch in "0123456789%".chars() {
            assert!(glyph(ch).is_some(), "missing glyph for {ch}");
        }
        assert!(glyph('x').is_none());
    }

    #[test]
    fn render_produces_a_full_rgba_buffer_with_visible_text() {
        let pixels = render(10); // 10% → TEXT_LOW 색으로 그려진다
        assert_eq!(pixels.len(), (BADGE_SIZE * BADGE_SIZE * 4) as usize);
        let text_pixels = pixels.chunks(4).filter(|px| px[3] == 255 && px[0] == TEXT_LOW[0] && px[1] == TEXT_LOW[1] && px[2] == TEXT_LOW[2]).count();
        assert!(text_pixels > 20, "expected glyph pixels, got {text_pixels}");
    }

    #[test]
    fn badge_text_clamps_to_two_digits() {
        assert_eq!(badge_text(7), "7%");
        assert_eq!(badge_text(24), "24%");
        assert_eq!(badge_text(100), "99%");
    }

    #[test]
    fn min_remaining_prefers_rolling_window_of_connected_providers() {
        let snapshots = vec![
            snapshot("codex", "connected", vec![("weekly", 80.0), ("rolling", 24.4)]),
            snapshot("claude", "stale", vec![("rolling", 32.0)]),
            snapshot("gemini", "signed-out", vec![("rolling", 1.0)]),
        ];
        assert_eq!(min_remaining_percent(&snapshots), Some(("codex".to_string(), 24)));
    }

    #[test]
    fn min_remaining_is_none_without_usable_windows() {
        assert_eq!(min_remaining_percent(&[]), None);
        assert_eq!(min_remaining_percent(&[snapshot("codex", "not-installed", vec![])]), None);
    }

    #[test]
    fn text_color_follows_thresholds() {
        assert_eq!(text_color(60), TEXT_HIGH);
        assert_eq!(text_color(30), TEXT_MID);
        assert_eq!(text_color(10), TEXT_LOW);
    }
}
