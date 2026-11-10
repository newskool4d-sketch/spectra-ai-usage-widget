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

pub fn min_remaining_percent(snapshots: &[ProviderUsageSnapshot], now: u64) -> Option<(String, u8)> {
    snapshots
        .iter()
        .filter(|snapshot| usable(snapshot))
        .filter_map(|snapshot| {
            // A carried value whose window already reset is not shown as a percentage.
            let mut current = snapshot.windows.iter()
                .filter(|w| !crate::provider_usage::stale_window_reset_passed(snapshot, w, now))
                // Per-model limits (Claude's Fable allowance) never stand in for the service value.
                .filter(|w| if snapshot.provider_id == "antigravity" { crate::antigravity::is_summary_window(w) } else { w.model_group_id.is_none() });
            let window = if snapshot.provider_id == "antigravity" {
                current.min_by(|a, b| a.remaining_percent.total_cmp(&b.remaining_percent))
            } else {
                current.clone().find(|w| w.id == "rolling").or_else(|| current.next())
            }?;
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
                id: id.to_string(), label: id.to_string(), used_percent: 100.0 - remaining, remaining_percent: remaining, resets_at: None, window_duration_mins: None, model_group_id: None, model_group_label: None,
            }).collect(),
            message: String::new(),
            live_failure: None,
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
        assert_eq!(min_remaining_percent(&snapshots, 0), Some(("codex".to_string(), 24)));
    }

    #[test]
    fn min_remaining_is_none_without_usable_windows() {
        assert_eq!(min_remaining_percent(&[], 0), None);
        assert_eq!(min_remaining_percent(&[snapshot("codex", "not-installed", vec![])], 0), None);
    }

    #[test]
    fn min_remaining_skips_a_carried_window_that_already_reset() {
        let mut claude = snapshot("claude", "stale", vec![("rolling", 9.0), ("weekly", 40.0)]);
        claude.windows[0].resets_at = Some(1_000);
        assert_eq!(min_remaining_percent(&[claude.clone()], 999), Some(("claude".to_string(), 9)));
        assert_eq!(min_remaining_percent(&[claude.clone()], 1_000), Some(("claude".to_string(), 40)),
            "the pre-reset 9% is not today's value");
        claude.windows[1].resets_at = Some(1_000);
        assert_eq!(min_remaining_percent(&[claude.clone()], 1_000), None);
        claude.connection_state = "connected".to_string();
        assert_eq!(min_remaining_percent(&[claude], 1_000), Some(("claude".to_string(), 9)), "fresh values stay");
    }

    #[test]
    fn claude_per_model_limits_never_become_the_badge_value() {
        let mut claude = snapshot("claude", "connected", vec![("rolling", 81.0), ("weekly", 74.0), ("weekly", 3.0), ("rolling", 1.0)]);
        for window in &mut claude.windows[2..] {
            window.model_group_id = Some("fable".to_string());
        }
        assert_eq!(min_remaining_percent(&[claude.clone()], 0), Some(("claude".to_string(), 81)));
        // Only per-model windows left: the provider has no badge value rather than a Fable one.
        claude.windows.drain(..2);
        assert_eq!(min_remaining_percent(&[claude], 0), None);
    }

    fn antigravity(windows: Vec<(&str, &str, f64)>) -> ProviderUsageSnapshot {
        let mut result = snapshot("antigravity", "connected", windows.iter().map(|(_, id, remaining)| (*id, *remaining)).collect());
        for (window, (group, ..)) in result.windows.iter_mut().zip(&windows) {
            window.model_group_id = Some(group.to_string());
        }
        result
    }

    #[test]
    fn antigravity_counts_only_the_gemini_group_like_the_strip() {
        let ag = antigravity(vec![("claude-gpt", "weekly", 31.0), ("gemini-4-argon", "weekly", 20.0),
            ("gemini", "rolling", 100.0), ("gemini", "weekly", 83.0)]);
        assert_eq!(min_remaining_percent(&[ag.clone()], 0), Some(("antigravity".to_string(), 83)));
        let codex = snapshot("codex", "connected", vec![("weekly", 50.0)]);
        assert_eq!(min_remaining_percent(&[ag, codex.clone()], 0), Some(("codex".to_string(), 50)));

        let no_gemini = antigravity(vec![("claude-gpt", "weekly", 31.0), ("gemini-4-argon", "weekly", 20.0)]);
        assert_eq!(min_remaining_percent(&[no_gemini.clone()], 0), None, "no other group stands in for Gemini");
        assert_eq!(min_remaining_percent(&[no_gemini, codex], 0), Some(("codex".to_string(), 50)));
    }

    #[test]
    fn antigravity_skips_a_carried_gemini_window_that_already_reset() {
        let mut ag = antigravity(vec![("gemini", "rolling", 9.0), ("gemini", "weekly", 40.0), ("claude-gpt", "weekly", 5.0)]);
        ag.connection_state = "stale".to_string();
        ag.windows[0].resets_at = Some(1_000);
        assert_eq!(min_remaining_percent(&[ag.clone()], 999), Some(("antigravity".to_string(), 9)));
        assert_eq!(min_remaining_percent(&[ag], 1_000), Some(("antigravity".to_string(), 40)));
    }

    #[test]
    fn text_color_follows_thresholds() {
        assert_eq!(text_color(60), TEXT_HIGH);
        assert_eq!(text_color(30), TEXT_MID);
        assert_eq!(text_color(10), TEXT_LOW);
    }
}
