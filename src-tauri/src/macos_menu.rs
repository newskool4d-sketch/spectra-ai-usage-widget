//! Platform-independent presentation rules for the macOS menu bar.
//! Native AppKit calls remain in desktop_shell; no credentials are needed here.
use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

const PROVIDERS: [(&str, &str, &str); 3] = [
    ("codex", "CX", "Codex"),
    ("claude", "CL", "Claude"),
    ("antigravity", "AG", "Antigravity"),
];
pub const ICON_SIZE: u32 = 22;

#[derive(Debug, PartialEq)]
pub struct MenuBarModel {
    pub title: String,
    pub tooltip: String,
}

fn verified(snapshot: &ProviderUsageSnapshot) -> bool {
    matches!(snapshot.connection_state.as_str(), "connected" | "stale")
        && match snapshot.provider_id.as_str() {
            "codex" => snapshot.source.as_deref() == Some("codex-app-server"),
            "claude" => matches!(snapshot.source.as_deref(), Some("claude-usage-api" | "claude-statusline")),
            "antigravity" => snapshot.source.as_deref() == Some("antigravity-statusline"),
            _ => false,
        }
}

fn percent(window: &ProviderQuotaWindow, now: u64) -> Option<u8> {
    (window.remaining_percent.is_finite()
        && (0.0..=100.0).contains(&window.remaining_percent)
        && !window.resets_at.is_some_and(|reset| reset <= now))
        .then(|| window.remaining_percent.round() as u8)
}

fn stale(snapshot: &ProviderUsageSnapshot, now: u64) -> bool {
    snapshot.connection_state != "connected"
        || snapshot.live_failure.is_some()
        || !snapshot.last_synced_at.is_some_and(|captured| captured > 0 && captured <= now && now - captured < 300)
}

/// Antigravity's title follows the Gemini group alone. Other groups stay in the tooltip.
fn counts_toward_title(snapshot: &ProviderUsageSnapshot, window: &ProviderQuotaWindow) -> bool {
    snapshot.provider_id != "antigravity" || window.model_group_id.as_deref() == Some("gemini")
}

pub fn build_model(snapshots: &[ProviderUsageSnapshot], now: u64) -> MenuBarModel {
    let mut parts = Vec::new();
    let mut details = vec!["SPECTRA · 서비스별 최소 잔여율 (Antigravity는 Gemini 기준)".to_string()];
    for (id, short, name) in PROVIDERS {
        let snapshot = snapshots.iter().find(|snapshot| snapshot.provider_id == id);
        let usable = snapshot.filter(|snapshot| verified(snapshot));
        let remaining = usable.and_then(|snapshot| snapshot.windows.iter()
            .filter(|window| counts_toward_title(snapshot, window))
            .filter_map(|window| percent(window, now)).min());
        let is_stale = usable.is_some_and(|snapshot| stale(snapshot, now));
        let value = remaining.map(|value| format!("{value}%{}", if is_stale { "·" } else { "" })).unwrap_or_else(|| "—".into());
        parts.push(format!("{short}{value}"));
        if let Some(snapshot) = usable {
            let windows = snapshot.windows.iter().map(|window| {
                let value = percent(window, now).map(|value| format!("{value}%")).unwrap_or_else(|| "— (초기화 후 갱신 대기)".into());
                let group = window.model_group_label.as_deref().or(window.model_group_id.as_deref());
                let label = group.map(|group| format!("{group} / {}", window.label)).unwrap_or_else(|| window.label.clone());
                format!("{label}: {value}")
            }).collect::<Vec<_>>().join(" · ");
            details.push(format!("{name}: {windows}"));
            if is_stale { details.push(format!("{name}: 이전 수집값 · 갱신 대기")); }
        } else {
            let status = match snapshot.map(|snapshot| snapshot.connection_state.as_str()) {
                Some("signed-out") => "로그인 필요",
                Some("not-installed") => "공식 CLI 설치 필요",
                Some("error") => "사용량 확인 지연",
                _ => "사용량 연결 대기",
            };
            details.push(format!("{name}: {status}"));
        }
    }
    details.push("· 표시는 이전 수집값 · 클릭: 미니 창 · 우클릭: 메뉴".into());
    MenuBarModel { title: parts.join("  "), tooltip: details.join("\n") }
}

/// Monochrome SPECTRA slashes. Template images are recolored by macOS for both appearances.
pub fn template_icon() -> Vec<u8> {
    let mut rgba = vec![0; (ICON_SIZE * ICON_SIZE * 4) as usize];
    for y in 3..19 {
        for x in 2..20 {
            let slash = x as i32 + y as i32 / 2;
            if (8..=10).contains(&slash) || (14..=16).contains(&slash) || (20..=22).contains(&slash) {
                rgba[((y * ICON_SIZE + x) * 4 + 3) as usize] = 255;
            }
        }
    }
    rgba
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }

/// macOS expresses tray/monitor rectangles in physical pixels scaled per display.
/// Convert to points before positioning a window; mixed-DPI absolute rectangles can overlap,
/// so ambiguous monitor matches deliberately fall back to normal window centering.
pub fn logical_anchor(physical: ScreenRect, monitors: &[(ScreenRect, f64)]) -> Option<(usize, ScreenRect)> {
    let mut selected = None;
    for (index, (bounds, scale)) in monitors.iter().enumerate() {
        if !scale.is_finite() || *scale <= 0.0 { continue; }
        let anchor = ScreenRect { x: physical.x / scale, y: physical.y / scale, width: physical.width / scale, height: physical.height / scale };
        let x = anchor.x + anchor.width / 2.0;
        let y = anchor.y + anchor.height / 2.0;
        if x >= bounds.x / scale && x < (bounds.x + bounds.width) / scale
            && y >= bounds.y / scale && y < (bounds.y + bounds.height) / scale {
            if selected.is_some() { return None; }
            selected = Some((index, anchor));
        }
    }
    selected
}

/// Coordinates and dimensions are logical points, including negative monitor origins.
pub fn anchored_position(anchor: ScreenRect, work: ScreenRect, width: f64, height: f64, gap: f64) -> (i32, i32) {
    let rightmost = (work.x + work.width - width).max(work.x);
    let lowest = (work.y + work.height - height).max(work.y);
    let x = (anchor.x + anchor.width / 2.0 - width / 2.0).clamp(work.x, rightmost);
    let y = (anchor.y + anchor.height + gap).clamp(work.y, lowest);
    (x.round() as i32, y.round() as i32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(provider: &str, values: &[f64]) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot {
            provider_id: provider.into(), runtime_available: true, auth_state: "signed-in".into(),
            connection_state: "connected".into(), auth_method: None, plan_type: None,
            source: Some(match provider { "codex" => "codex-app-server", "claude" => "claude-usage-api", _ => "antigravity-statusline" }.into()),
            last_synced_at: Some(900), bridge_installed: true, message: String::new(), live_failure: None,
            windows: values.iter().enumerate().map(|(index, value)| ProviderQuotaWindow {
                id: if index == 0 { "rolling" } else { "weekly" }.into(), label: format!("window {index}"),
                used_percent: 100.0 - value, remaining_percent: *value, resets_at: Some(2_000),
                window_duration_mins: None, model_group_id: None, model_group_label: None,
            }).collect(),
        }
    }

    fn antigravity(groups: &[&str], values: &[f64]) -> ProviderUsageSnapshot {
        let mut result = snapshot("antigravity", values);
        for (window, group) in result.windows.iter_mut().zip(groups) {
            window.model_group_id = Some((*group).into());
        }
        result
    }

    #[test]
    fn model_uses_verified_minimum_in_stable_provider_order() {
        let model = build_model(&[antigravity(&["gemini", "gemini"], &[100.0, 83.0]), snapshot("claude", &[96.0]), snapshot("codex", &[70.0, 47.0])], 1_000);
        assert_eq!(model.title, "CX47%  CL96%  AG83%");
        assert!(model.tooltip.contains("서비스별 최소 잔여율"));
    }

    #[test]
    fn antigravity_title_follows_gemini_and_tooltip_keeps_other_groups() {
        let model = build_model(&[antigravity(&["claude-gpt", "gemini", "gemini-4-argon", "gemini"], &[52.0, 100.0, 76.0, 90.0])], 1_000);
        assert_eq!(model.title, "CX—  CL—  AG90%");
        assert!(model.tooltip.contains("Antigravity는 Gemini 기준"));
        assert!(model.tooltip.contains("claude-gpt / window 0: 52%"));
        assert!(model.tooltip.contains("gemini-4-argon / window 2: 76%"));
    }

    #[test]
    fn antigravity_title_never_substitutes_another_group_for_gemini() {
        let model = build_model(&[antigravity(&["claude-gpt", "gemini-4-argon"], &[52.0, 76.0])], 1_000);
        assert_eq!(model.title, "CX—  CL—  AG—");
    }

    #[test]
    fn absent_invalid_and_reset_values_are_never_zero() {
        let mut codex = snapshot("codex", &[f64::NAN, 40.0]);
        codex.windows[1].resets_at = Some(1_000);
        let mut claude = snapshot("claude", &[30.0]);
        claude.source = Some("unverified".into());
        assert_eq!(build_model(&[codex, claude], 1_000).title, "CX—  CL—  AG—");
    }

    #[test]
    fn old_or_failed_snapshots_have_visible_cache_markers() {
        let mut codex = snapshot("codex", &[0.0]);
        codex.connection_state = "stale".into();
        let mut claude = snapshot("claude", &[100.0]);
        claude.last_synced_at = Some(700);
        let model = build_model(&[codex, claude], 1_000);
        assert_eq!(model.title, "CX0%·  CL100%·  AG—");
        assert!(model.tooltip.contains("이전 수집값"));
    }

    #[test]
    fn template_is_transparent_monochrome_and_visible() {
        let pixels = template_icon();
        assert_eq!(pixels.len(), (ICON_SIZE * ICON_SIZE * 4) as usize);
        assert!(pixels.chunks_exact(4).all(|pixel| pixel[..3] == [0, 0, 0]));
        assert!(pixels.chunks_exact(4).filter(|pixel| pixel[3] == 255).count() > 50);
        assert_eq!(pixels[3], 0);
    }

    #[test]
    fn position_stays_inside_retina_and_negative_origin_monitors() {
        let work = ScreenRect { x: -2880.0, y: 50.0, width: 2880.0, height: 1700.0 };
        let anchor = ScreenRect { x: -60.0, y: 0.0, width: 44.0, height: 50.0 };
        assert_eq!(anchored_position(anchor, work, 860.0, 1450.0, 12.0), (-860, 62));
        let small = ScreenRect { x: 0.0, y: 24.0, width: 800.0, height: 650.0 };
        assert_eq!(anchored_position(anchor, small, 430.0, 720.0, 6.0), (0, 24));
    }

    #[test]
    fn retina_anchor_is_converted_to_points_and_ambiguous_monitors_are_rejected() {
        let tray = ScreenRect { x: 2900.0, y: 0.0, width: 100.0, height: 48.0 };
        let retina = (ScreenRect { x: 0.0, y: 0.0, width: 3024.0, height: 1964.0 }, 2.0);
        assert_eq!(logical_anchor(tray, &[retina]), Some((0, ScreenRect { x: 1450.0, y: 0.0, width: 50.0, height: 24.0 })));
        let external = (ScreenRect { x: 1512.0, y: 0.0, width: 1920.0, height: 1080.0 }, 1.0);
        assert_eq!(logical_anchor(tray, &[retina, external]), None);
    }
}
