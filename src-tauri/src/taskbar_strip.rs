use crate::provider_usage::ProviderUsageSnapshot;
use std::time::{SystemTime, UNIX_EPOCH};

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

/// Codex의 기존 단일 값은 먼저 막히는 창의 잔여율을 유지한다.
/// Claude와 5시간 창이 있는 Codex는 build_model_at에서 5시간·주간을 각각 고정 표시한다(스펙 §3-3).
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
pub struct StripSegment {
    pub label: String,
    pub window_label: Option<&'static str>,
    pub value: String,
    pub value_color: [u8; 3],
}

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

fn claude_status(snapshot: &ProviderUsageSnapshot, now: u64) -> &'static str {
    if snapshot.connection_state == "not-installed" {
        return "Claude Code 설치 필요";
    }
    if snapshot.auth_state == "signed-out" || snapshot.connection_state == "signed-out" {
        return "로그인 필요";
    }
    if snapshot.live_failure.as_deref() == Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED) {
        return "로그인 갱신 필요";
    }
    if snapshot.connection_state == "error" && snapshot.windows.is_empty() {
        return "연결 상태 확인 필요";
    }
    if snapshot.windows.is_empty() {
        return "사용량 갱신 대기";
    }
    let expired = snapshot.windows.iter().any(|window| window.resets_at.is_some_and(|reset| reset <= now));
    let captured_recently = snapshot.last_synced_at
        .is_some_and(|captured| captured <= now && now - captured <= 24 * 60 * 60);
    if snapshot.connection_state != "connected" || expired || !captured_recently
        || snapshot.source.as_deref() != Some("claude-usage-api") {
        "갱신 대기 (캐시)"
    } else {
        "동기화됨"
    }
}

fn last_sync_label(captured_at: Option<u64>, now: u64) -> String {
    let age = match captured_at {
        None => return "기록 없음".to_string(),
        Some(captured) if captured > now => return "시각 확인 필요".to_string(),
        Some(captured) => now - captured,
    };
    match age {
        0..=59 => "1분 이내".to_string(),
        60..=3599 => format!("{}분 전", age / 60),
        3600..=86399 => format!("{}시간 {}분 전", age / 3600, age % 3600 / 60),
        _ => format!("{}일 {}시간 전", age / 86400, age % 86400 / 3600),
    }
}

pub fn build_model(snapshots: &[ProviderUsageSnapshot], theme: StripTheme) -> Option<StripModel> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    build_model_at(snapshots, theme, now)
}

fn build_model_at(snapshots: &[ProviderUsageSnapshot], theme: StripTheme, now: u64) -> Option<StripModel> {
    let colors = palette(theme);
    let mut segments = Vec::with_capacity(PROVIDERS.len() + 1);
    let mut tooltip_lines = Vec::new();
    let mut any_value = false;

    for (id, name) in PROVIDERS {
        let snapshot = snapshots.iter().find(|s| s.provider_id == id);
        let mut append_segment = |window_label, remaining: Option<u8>| {
            any_value |= remaining.is_some();
            segments.push(StripSegment {
                label: name.to_string(),
                window_label,
                value: remaining.map(|percent| format!("{percent}%")).unwrap_or_else(|| NO_VALUE.to_string()),
                value_color: remaining.map(|percent| value_color(percent, colors)).unwrap_or(colors.label),
            });
        };
        // Codex는 5시간 창이 있는 요금제(Plus 등)일 때만 두 기간을 나눠 표시하고,
        // 주간 창만 있는 요금제(Pro 등)는 기존 단일 값을 유지한다.
        let codex_has_short_window = id == "codex" && snapshot.filter(|s| usable(s))
            .is_some_and(|s| s.windows.iter().any(|window| window.id == "rolling"));
        if id == "claude" || codex_has_short_window {
            // 데이터 순서·잔여량 크기·누락 여부에 따라 표시 기준이 바뀌지 않는다.
            for (window_id, label) in [("rolling", "5h"), ("weekly", "7d")] {
                let remaining = snapshot.filter(|s| usable(s))
                    .and_then(|s| s.windows.iter().find(|window| window.id == window_id))
                    .map(|window| window.remaining_percent.round().clamp(0.0, 100.0) as u8);
                append_segment(Some(label), remaining);
            }
        } else {
            append_segment(None, snapshot.and_then(provider_remaining));
        }
        if let Some(snapshot) = snapshot.filter(|s| usable(s)) {
            let detail = snapshot.windows.iter()
                .map(|w| format!("{} {}%", w.label, w.remaining_percent.round().clamp(0.0, 100.0) as u8))
                .collect::<Vec<_>>()
                .join(" · ");
            tooltip_lines.push(format!("{name}  {detail}"));
        } else if id != "claude" || snapshot.is_none() {
            tooltip_lines.push(format!("{name}  연결되지 않음"));
        }
        if let Some(snapshot) = snapshot.filter(|_| id == "claude") {
            let status = claude_status(snapshot, now);
            tooltip_lines.push(format!("Claude · {status}"));
            tooltip_lines.push(format!("마지막 동기화: {} (데이터 수집 기준)", last_sync_label(snapshot.last_synced_at, now)));
            if snapshot.connection_state != "connected" && !snapshot.message.is_empty() {
                tooltip_lines.push(snapshot.message.clone());
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
                .map(|value| StripSegment { label: "Claude".into(), window_label: Some("5h"), value: value.into(), value_color: [0; 3] })
                .collect(),
            tooltip: "SPECTRA · 캐시된 실제 잔여량".into(),
        };
        let text = model.segments.iter().map(|s| (s.label.clone(), s.window_label, s.value.clone())).collect::<Vec<_>>();
        let tooltip = model.tooltip.clone();
        for theme in [StripTheme::Light, StripTheme::Dark, StripTheme::Light] {
            retheme_model(&mut model, theme);
            let colors = palette(theme);
            assert_eq!(model.segments.iter().map(|s| s.value_color).collect::<Vec<_>>(),
                [colors.low, colors.low, colors.mid, colors.mid, colors.high, colors.high, colors.label]);
            assert_eq!(model.segments.iter().map(|s| (s.label.clone(), s.window_label, s.value.clone())).collect::<Vec<_>>(), text);
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
            live_failure: None,
        }
    }

    #[test]
    fn provider_remaining_takes_the_binding_window_not_the_first() {
        let s = snapshot("codex", "connected", vec![window("rolling", 59.0), window("weekly", 31.4)]);
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
        assert_eq!(model.segments.len(), 3);
        assert_eq!(model.segments[0].label, "Codex");
        assert_eq!(model.segments[0].window_label, None);
        assert_eq!(model.segments[0].value, "55%");
        assert_eq!(model.segments[1].label, "Claude");
        assert_eq!(model.segments[1].window_label, Some("5h"));
        assert_eq!(model.segments[1].value, "59%");
        assert_eq!(model.segments[2].label, "Claude");
        assert_eq!(model.segments[2].window_label, Some("7d"));
        assert_eq!(model.segments[2].value, "—");
    }

    #[test]
    fn claude_keeps_both_periods_when_the_binding_window_changes() {
        for state in ["connected", "stale"] {
            for rolling in [72.0, 12.0, 100.0] {
                let model = build_model(&[
                    snapshot("claude", state, vec![window("weekly", 31.4), window("rolling", rolling)]),
                ], StripTheme::Dark).unwrap();
                assert_eq!(model.segments[0].value, "—");
                assert_eq!(model.segments[1].window_label, Some("5h"));
                assert_eq!(model.segments[1].value, format!("{rolling:.0}%"));
                assert_eq!(model.segments[2].window_label, Some("7d"));
                assert_eq!(model.segments[2].value, "31%");
                assert_eq!(model.segments[1].value_color,
                    if rolling < 20.0 { palette(StripTheme::Dark).low } else { palette(StripTheme::Dark).high });
                assert_eq!(model.segments[2].value_color, palette(StripTheme::Dark).mid);
            }
        }
    }

    #[test]
    fn claude_missing_windows_keep_their_slots_without_falling_back() {
        for (state, windows, expected) in [
            ("connected", vec![window("rolling", 72.0)], ["72%", "—"]),
            ("connected", vec![window("weekly", 31.0)], ["—", "31%"]),
            ("connected", vec![window("other", 9.0)], ["—", "—"]),
            ("connected", vec![], ["—", "—"]),
            ("signed-out", vec![window("rolling", 72.0), window("weekly", 31.0)], ["—", "—"]),
        ] {
            let model = build_model(&[
                snapshot("codex", "connected", vec![window("weekly", 55.0)]),
                snapshot("claude", state, windows),
            ], StripTheme::Light).unwrap();
            assert_eq!(model.segments[1].window_label, Some("5h"));
            assert_eq!(model.segments[2].window_label, Some("7d"));
            for (segment, expected) in model.segments[1..].iter().zip(expected) {
                assert_eq!(segment.value, expected);
                if expected == "—" { assert_eq!(segment.value_color, palette(StripTheme::Light).label); }
            }
        }
    }

    #[test]
    fn codex_with_a_short_window_shows_both_periods_like_claude() {
        // Plus처럼 5시간 창이 있는 요금제: Codex도 5h·7d를 각각 고정 표시한다.
        for (windows, expected) in [
            (vec![window("weekly", 55.0), window("rolling", 72.0)], ["72%", "55%"]),
            (vec![window("rolling", 12.0)], ["12%", "—"]),
        ] {
            for state in ["connected", "stale"] {
                let model = build_model(&[
                    snapshot("codex", state, windows.clone()),
                    snapshot("claude", "connected", vec![window("rolling", 40.0), window("weekly", 30.0)]),
                ], StripTheme::Light).unwrap();
                assert_eq!(model.segments.len(), 4);
                let codex = &model.segments[..2];
                assert_eq!(codex.iter().map(|s| (s.label.as_str(), s.window_label)).collect::<Vec<_>>(),
                    [("Codex", Some("5h")), ("Codex", Some("7d"))]);
                assert_eq!(codex.iter().map(|s| s.value.as_str()).collect::<Vec<_>>(), expected);
                assert_eq!(model.segments[2].window_label, Some("5h"));
                assert_eq!(model.segments[2].value, "40%");
            }
        }
    }

    #[test]
    fn codex_weekly_only_plan_keeps_a_single_unlabelled_value() {
        // Pro처럼 주간 창만 오는 요금제는 빈 5h 칸을 만들지 않는다.
        let model = build_model(&[snapshot("codex", "connected", vec![window("weekly", 55.0)])], StripTheme::Dark).unwrap();
        assert_eq!(model.segments[0].label, "Codex");
        assert_eq!(model.segments[0].window_label, None);
        assert_eq!(model.segments[0].value, "55%");
        assert_eq!(model.segments.len(), 3);
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
        assert_eq!(model.segments[2].value, "—");
        assert_eq!(model.segments[2].value_color, palette(StripTheme::Light).label);
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
    fn claude_tooltip_distinguishes_captured_data_from_refresh_attempts() {
        let mut claude = snapshot("claude", "connected", vec![window("rolling", 72.0)]);
        claude.source = Some("claude-usage-api".into());
        claude.last_synced_at = Some(1_000);
        let fresh = build_model_at(&[claude.clone()], StripTheme::Dark, 1_300).unwrap();
        assert!(fresh.tooltip.contains("Claude · 동기화됨"));
        assert!(fresh.tooltip.contains("마지막 동기화: 5분 전 (데이터 수집 기준)"));

        claude.connection_state = "stale".into();
        claude.message = "서버 연결 실패. 마지막 동기화 값을 표시합니다.".into();
        let cached = build_model_at(&[claude], StripTheme::Dark, 1_600).unwrap();
        assert!(cached.tooltip.contains("Claude · 갱신 대기 (캐시)"));
        assert!(cached.tooltip.contains("마지막 동기화: 10분 전"));
        assert!(cached.tooltip.contains("서버 연결 실패"));
        assert_eq!(cached.segments[1].value, "72%");
    }

    #[test]
    fn claude_tooltip_expires_at_reset_without_waiting_for_a_response() {
        let mut claude = snapshot("claude", "connected", vec![window("rolling", 72.0)]);
        claude.source = Some("claude-usage-api".into());
        claude.last_synced_at = Some(100);
        claude.windows[0].resets_at = Some(120);
        assert_eq!(claude_status(&claude, 119), "동기화됨");
        assert_eq!(claude_status(&claude, 120), "갱신 대기 (캐시)");
        claude.windows[0].resets_at = None;
        assert_eq!(claude_status(&claude, 100 + 86401), "갱신 대기 (캐시)");
        claude.source = Some("claude-statusline".into());
        assert_eq!(claude_status(&claude, 119), "갱신 대기 (캐시)");
    }

    #[test]
    fn claude_tooltip_shows_login_waiting_and_error_without_inventing_a_sync_time() {
        for (state, label) in [("signed-out", "로그인 필요"), ("waiting-for-usage", "사용량 갱신 대기"),
            ("error", "연결 상태 확인 필요"), ("not-installed", "Claude Code 설치 필요")] {
            let model = build_model_at(&[
                snapshot("codex", "connected", vec![window("weekly", 55.0)]),
                snapshot("claude", state, vec![]),
            ], StripTheme::Light, 1_000).unwrap();
            assert!(model.tooltip.contains(&format!("Claude · {label}")));
            assert!(model.tooltip.contains("마지막 동기화: 기록 없음"));
            assert_eq!(model.segments[1].value, "—");
        }
    }

    #[test]
    fn last_sync_age_handles_missing_future_and_day_boundaries() {
        assert_eq!(last_sync_label(None, 1_000), "기록 없음");
        assert_eq!(last_sync_label(Some(1_001), 1_000), "시각 확인 필요");
        assert_eq!(last_sync_label(Some(1_000), 1_059), "1분 이내");
        assert_eq!(last_sync_label(Some(1_000), 1_060), "1분 전");
        assert_eq!(last_sync_label(Some(1_000), 4_660), "1시간 1분 전");
        assert_eq!(last_sync_label(Some(1_000), 91_000), "1일 1시간 전");
    }

    #[test]
    fn claude_signed_in_cache_still_surfaces_usage_login_failures() {
        let mut claude = snapshot("claude", "stale", vec![window("rolling", 72.0)]);
        claude.source = Some("claude-statusline".into());
        claude.last_synced_at = Some(1_000);
        for hint in ["Claude Code 로그인 갱신이 필요합니다.", "Claude 사용량 조회를 위한 로그인이 필요합니다."] {
            claude.message = hint.into();
            let model = build_model_at(&[claude.clone()], StripTheme::Dark, 1_300).unwrap();
            assert!(model.tooltip.contains(hint));
            assert!(model.tooltip.contains("갱신 대기 (캐시)"));
            assert!(model.tooltip.contains("마지막 동기화: 5분 전"));
            assert_eq!(model.segments[1].value, "72%");
        }
    }

    #[test]
    fn claude_tooltip_asks_for_a_login_refresh_when_the_token_expired() {
        let mut claude = snapshot("claude", "stale", vec![window("rolling", 72.0)]);
        claude.source = Some("claude-statusline".into());
        claude.last_synced_at = Some(1_000);
        claude.live_failure = Some(crate::provider_usage::CLAUDE_TOKEN_EXPIRED.into());
        claude.message = "Claude Code 로그인 갱신이 필요합니다. 토큰이 만료되어 Claude Code를 한 번 실행해 주세요. 마지막 동기화 값을 표시합니다.".into();
        assert_eq!(claude_status(&claude, 1_300), "로그인 갱신 필요");
        let model = build_model_at(&[claude], StripTheme::Dark, 1_300).unwrap();
        assert!(model.tooltip.contains("Claude · 로그인 갱신 필요"));
        assert!(model.tooltip.contains("토큰이 만료되어"));
        assert_eq!(model.segments[1].value, "72%", "cached usage stays visible");
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
