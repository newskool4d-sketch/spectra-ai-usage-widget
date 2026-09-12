use crate::provider_usage::ProviderUsageSnapshot;

/// 표시 순서와 사람이 읽는 이름. 스냅샷 도착 순서와 무관하게 이 순서로 그린다.
const PROVIDERS: [(&str, &str); 2] = [("codex", "Codex"), ("claude", "Claude")];
const NO_VALUE: &str = "—";

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StripTheme { Light, Dark }

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TaskbarEdge { Left, Top, Right, Bottom }

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub high: [u8; 3],
    pub mid: [u8; 3],
    pub low: [u8; 3],
    pub label: [u8; 3],
    pub separator: [u8; 3],
}

/// 작업표시줄 배경 위 대비를 맞춘 두 벌. tray_badge의 색은 어두운 배지 위 기준이라
/// 라이트 작업표시줄에서 대비가 부족해 그대로 쓰지 않는다(스펙 §3-4).
pub fn palette(theme: StripTheme) -> Palette {
    match theme {
        StripTheme::Light => Palette {
            high: [0x1F, 0x7F, 0x78],
            mid: [0x9A, 0x6B, 0x00],
            low: [0xB4, 0x47, 0x2B],
            label: [0x5F, 0x5F, 0x5F],
            separator: [0x9A, 0x9A, 0x9A],
        },
        StripTheme::Dark => Palette {
            high: [0x56, 0xB7, 0xB0],
            mid: [0xC7, 0xA8, 0x52],
            low: [0xD8, 0x84, 0x63],
            label: [0xA0, 0xA0, 0xA0],
            separator: [0x6B, 0x6B, 0x6B],
        },
    }
}

fn value_color(percent: u8, palette: Palette) -> [u8; 3] {
    if percent >= 50 { palette.high } else if percent >= 20 { palette.mid } else { palette.low }
}

fn usable(snapshot: &ProviderUsageSnapshot) -> bool {
    matches!(snapshot.connection_state.as_str(), "connected" | "stale") && !snapshot.windows.is_empty()
}

/// 그 공급자가 실제로 먼저 막히는 창의 잔여율. tray_badge의 rolling 우선 규칙과 달리
/// 임의 선택 없이 최솟값을 쓴다(스펙 §3-3).
pub fn provider_remaining(snapshot: &ProviderUsageSnapshot) -> Option<u8> {
    if !usable(snapshot) {
        return None;
    }
    snapshot
        .windows
        .iter()
        .map(|w| w.remaining_percent.round().clamp(0.0, 100.0) as u8)
        .min()
}

#[derive(Clone, Debug, PartialEq)]
pub struct StripSegment { pub label: String, pub value: String, pub value_color: [u8; 3] }

#[derive(Clone, Debug, PartialEq)]
pub struct StripModel { pub segments: Vec<StripSegment>, pub tooltip: String }

/// 조회 없이 캐시된 표시 값에 현재 작업표시줄 팔레트를 적용한다.
pub fn retheme_model(model: &mut StripModel, theme: StripTheme) {
    let colors = palette(theme);
    for segment in &mut model.segments {
        segment.value_color = segment.value.strip_suffix('%')
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|percent| *percent <= 100)
            .map(|percent| value_color(percent, colors))
            .unwrap_or(colors.label);
    }
}

pub fn build_model(snapshots: &[ProviderUsageSnapshot], theme: StripTheme) -> Option<StripModel> {
    let colors = palette(theme);
    let mut segments = Vec::with_capacity(PROVIDERS.len());
    let mut tooltip_lines = Vec::new();
    let mut any_value = false;

    for (id, name) in PROVIDERS {
        let snapshot = snapshots.iter().find(|s| s.provider_id == id);
        let remaining = snapshot.and_then(provider_remaining);
        match remaining {
            Some(percent) => {
                any_value = true;
                segments.push(StripSegment {
                    label: name.to_string(),
                    value: format!("{percent}%"),
                    value_color: value_color(percent, colors),
                });
                let detail = snapshot
                    .map(|s| {
                        s.windows
                            .iter()
                            .map(|w| format!("{} {}%", w.label, w.remaining_percent.round().clamp(0.0, 100.0) as u8))
                            .collect::<Vec<_>>()
                            .join(" · ")
                    })
                    .unwrap_or_default();
                tooltip_lines.push(format!("{name}  {detail}"));
            }
            None => {
                segments.push(StripSegment {
                    label: name.to_string(),
                    value: NO_VALUE.to_string(),
                    value_color: colors.label,
                });
                tooltip_lines.push(format!("{name}  연결되지 않음"));
            }
        }
    }

    // 두 공급자 모두 값이 없으면 빈 막대를 띄우지 않는다.
    if !any_value {
        return None;
    }
    Some(StripModel {
        segments,
        tooltip: format!("SPECTRA · 개인 요금제 잔여\n{}", tooltip_lines.join("\n")),
    })
}

/// 스트립을 트레이 왼쪽 가장자리에 오른쪽 정렬하고 작업표시줄 안에서 세로 중앙에 놓는다.
/// 세로 작업표시줄은 v1 범위 밖이라 None(=띄우지 않음).
pub fn place_strip(taskbar: Rect, edge: TaskbarEdge, tray_left: i32, size: (i32, i32)) -> Option<(i32, i32)> {
    if matches!(edge, TaskbarEdge::Left | TaskbarEdge::Right) {
        return None;
    }
    let (width, height) = size;
    let x = (tray_left - width).max(taskbar.left);
    let y = taskbar.top + ((taskbar.bottom - taskbar.top) - height) / 2;
    Some((x, y))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

    #[test]
    fn theme_changes_recolor_cached_values_without_changing_text_or_tooltip() {
        let mut model = StripModel {
            segments: ["0%", "19%", "20%", "49%", "50%", "100%", "—"].into_iter()
                .map(|value| StripSegment { label: "Codex".into(), value: value.into(), value_color: [0; 3] })
                .collect(),
            tooltip: "SPECTRA · 캐시된 실제 잔여량".into(),
        };
        let text = model.segments.iter().map(|s| (s.label.clone(), s.value.clone())).collect::<Vec<_>>();
        let tooltip = model.tooltip.clone();
        for theme in [StripTheme::Light, StripTheme::Dark, StripTheme::Light] {
            retheme_model(&mut model, theme);
            let colors = palette(theme);
            assert_eq!(model.segments.iter().map(|s| s.value_color).collect::<Vec<_>>(),
                [colors.low, colors.low, colors.mid, colors.mid, colors.high, colors.high, colors.label]);
            assert_eq!(model.segments.iter().map(|s| (s.label.clone(), s.value.clone())).collect::<Vec<_>>(), text);
            assert_eq!(model.tooltip, tooltip);
        }
    }

    fn window(id: &str, remaining: f64) -> ProviderQuotaWindow {
        ProviderQuotaWindow {
            id: id.to_string(),
            label: id.to_string(),
            used_percent: 100.0 - remaining,
            remaining_percent: remaining,
            resets_at: None,
            window_duration_mins: None,
        }
    }

    fn snapshot(provider: &str, state: &str, windows: Vec<ProviderQuotaWindow>) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot {
            provider_id: provider.to_string(),
            runtime_available: true,
            auth_state: "signed-in".to_string(),
            connection_state: state.to_string(),
            auth_method: None,
            plan_type: None,
            source: None,
            last_synced_at: None,
            bridge_installed: false,
            windows,
            message: String::new(),
        }
    }

    #[test]
    fn provider_remaining_takes_the_binding_window_not_the_first() {
        let s = snapshot("claude", "connected", vec![window("rolling", 59.0), window("weekly", 31.4)]);
        assert_eq!(provider_remaining(&s), Some(31));
    }

    #[test]
    fn provider_remaining_accepts_stale_but_not_disconnected() {
        assert_eq!(provider_remaining(&snapshot("codex", "stale", vec![window("weekly", 55.0)])), Some(55));
        assert_eq!(provider_remaining(&snapshot("codex", "signed-out", vec![window("weekly", 55.0)])), None);
        assert_eq!(provider_remaining(&snapshot("codex", "connected", vec![])), None);
    }

    #[test]
    fn build_model_orders_codex_before_claude_regardless_of_input_order() {
        let model = build_model(
            &[
                snapshot("claude", "connected", vec![window("rolling", 59.0)]),
                snapshot("codex", "connected", vec![window("weekly", 55.0)]),
            ],
            StripTheme::Light,
        )
        .expect("both providers are usable");
        assert_eq!(model.segments.len(), 2);
        assert_eq!(model.segments[0].label, "Codex");
        assert_eq!(model.segments[0].value, "55%");
        assert_eq!(model.segments[1].label, "Claude");
        assert_eq!(model.segments[1].value, "59%");
    }

    #[test]
    fn build_model_marks_a_disconnected_provider_with_an_em_dash() {
        let model = build_model(
            &[
                snapshot("codex", "connected", vec![window("weekly", 8.0)]),
                snapshot("claude", "not-installed", vec![]),
            ],
            StripTheme::Light,
        )
        .expect("one usable provider still shows");
        assert_eq!(model.segments[0].value, "8%");
        assert_eq!(model.segments[0].value_color, palette(StripTheme::Light).low);
        assert_eq!(model.segments[1].value, "—");
        assert_eq!(model.segments[1].value_color, palette(StripTheme::Light).label);
    }

    #[test]
    fn build_model_is_none_when_no_provider_has_a_value() {
        assert!(build_model(&[], StripTheme::Dark).is_none());
        assert!(build_model(&[snapshot("codex", "signed-out", vec![])], StripTheme::Dark).is_none());
    }

    #[test]
    fn value_colour_follows_the_same_thresholds_as_the_tray_badge() {
        let light = palette(StripTheme::Light);
        let model = build_model(
            &[
                snapshot("codex", "connected", vec![window("weekly", 50.0)]),
                snapshot("claude", "connected", vec![window("rolling", 19.0)]),
            ],
            StripTheme::Light,
        )
        .unwrap();
        assert_eq!(model.segments[0].value_color, light.high);
        assert_eq!(model.segments[1].value_color, light.low);
    }

    #[test]
    fn light_and_dark_palettes_differ_on_every_role() {
        let (l, d) = (palette(StripTheme::Light), palette(StripTheme::Dark));
        assert_ne!(l.high, d.high);
        assert_ne!(l.mid, d.mid);
        assert_ne!(l.low, d.low);
        assert_ne!(l.label, d.label);
    }

    #[test]
    fn tooltip_lists_each_window_of_each_connected_provider() {
        let model = build_model(
            &[snapshot("codex", "connected", vec![window("rolling", 72.0), window("weekly", 55.0)])],
            StripTheme::Dark,
        )
        .unwrap();
        assert!(model.tooltip.contains("Codex"));
        assert!(model.tooltip.contains("72%"));
        assert!(model.tooltip.contains("55%"));
    }

    #[test]
    fn place_strip_right_aligns_to_the_tray_and_centres_vertically() {
        // 실측 기준(2026-09-12): 1920×1080 물리, 하단 작업표시줄 높이 60, TrayNotifyWnd.left = 1425
        let taskbar = Rect { left: 0, top: 1020, right: 1920, bottom: 1080 };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, 1425, (200, 24)), Some((1225, 1038)));
    }

    #[test]
    fn place_strip_handles_a_top_taskbar() {
        let taskbar = Rect { left: 0, top: 0, right: 1920, bottom: 60 };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Top, 1425, (200, 24)), Some((1225, 18)));
    }

    #[test]
    fn place_strip_centres_inside_a_150_percent_taskbar() {
        // 배율 150% → 작업표시줄 높이 90 물리 px. 96-DPI 상수를 쓰면 여기서 깨진다.
        let taskbar = Rect { left: 0, top: 990, right: 1920, bottom: 1080 };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, 1425, (260, 32)), Some((1165, 1019)));
    }

    #[test]
    fn place_strip_refuses_a_vertical_taskbar() {
        let taskbar = Rect { left: 0, top: 0, right: 60, bottom: 1080 };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Left, 60, (200, 24)), None);
        assert_eq!(place_strip(taskbar, TaskbarEdge::Right, 60, (200, 24)), None);
    }

    #[test]
    fn place_strip_clamps_to_the_taskbar_left_edge_when_the_tray_is_huge() {
        let taskbar = Rect { left: 0, top: 1020, right: 1920, bottom: 1080 };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, 120, (200, 24)), Some((0, 1038)));
    }
}
