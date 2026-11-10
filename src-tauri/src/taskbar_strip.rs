use crate::provider_usage::{stale_window_reset_passed, ProviderQuotaWindow, ProviderUsageSnapshot};
use std::time::{SystemTime, UNIX_EPOCH};

/// 표시 순서와 사람이 읽는 이름. 스냅샷 도착 순서와 무관하게 이 순서로 그린다.
const PROVIDERS: [(&str, &str); 3] = [("codex", "Codex"), ("claude", "Claude"), ("antigravity", "Antigravity")];
const NO_VALUE: &str = "—";
/// 이전 값이 없을 때 조회 자체가 실패한 상태(`connection_state == "error"`)의 표기.
/// 창의 claudeFreshness·connectionLabel·next-action 칩과 같은 문구를 쓴다.
const LOOKUP_DELAYED: &str = "사용량 확인 지연";

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
        && (snapshot.provider_id != "antigravity" || snapshot.source.as_deref() == Some("antigravity-statusline"))
}

/// 이어받은(stale) 값 중 초기화 시각이 지난 창은 오늘 값이 아니므로 "—"로 둔다.
fn window_percent(snapshot: &ProviderUsageSnapshot, window: &ProviderQuotaWindow, now: u64) -> Option<u8> {
    let valid = window.remaining_percent.is_finite() && (0.0..=100.0).contains(&window.remaining_percent);
    // CLI 캐시를 읽는 사이에도 초기화가 지나면 AG 값을 즉시 비운다.
    let valid_ag_window = snapshot.provider_id != "antigravity"
        || (matches!(window.id.as_str(), "rolling" | "weekly")
            && !window.resets_at.is_some_and(|reset| reset <= now));
    (valid && valid_ag_window && !stale_window_reset_passed(snapshot, window, now))
        .then(|| window.remaining_percent.round() as u8)
}

/// Codex의 기존 단일 값은 먼저 막히는 창의 잔여율을 유지한다.
/// Claude와 5시간 창이 있는 Codex는 build_model_at에서 5시간·주간을 각각 고정 표시한다(스펙 §3-3).
/// AG 값은 Gemini 모델군의 창만 보고, 다른 모델군은 tooltip에만 남긴다.
pub fn provider_remaining(snapshot: &ProviderUsageSnapshot, now: u64) -> Option<u8> {
    if !usable(snapshot) {
        return None;
    }
    snapshot
        .windows
        .iter()
        .filter(|w| if snapshot.provider_id == "antigravity" { crate::antigravity::is_summary_window(w) } else { w.model_group_id.is_none() })
        .filter_map(|w| window_percent(snapshot, w, now))
        .min()
}

#[derive(Clone, Debug, PartialEq)]
pub struct StripSegment {
    pub label: String,
    pub window_label: Option<&'static str>,
    pub value: String,
    pub value_color: [u8; 3],
    pub stale: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StripModel { pub segments: Vec<StripSegment>, pub tooltip: String }

/// 조회 없이 캐시된 표시 값에 현재 작업표시줄 팔레트를 적용한다.
pub fn retheme_model(model: &mut StripModel, theme: StripTheme) {
    let colors = palette(theme);
    for segment in &mut model.segments {
        segment.value_color = segment.value.strip_suffix('%').filter(|_| !segment.stale)
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|percent| *percent <= 100)
            .map(|percent| value_color(percent, colors))
            .unwrap_or(colors.label);
    }
}

/// 좁은 작업표시줄에서는 기간별 칸을 서비스별 최소값 하나로 줄인다.
/// 누락값은 0으로 바꾸지 않고, 상세 기간·모델군은 원래 tooltip에 남긴다.
pub fn compact_model(model: &StripModel) -> StripModel {
    let mut segments: Vec<StripSegment> = Vec::new();
    for segment in &model.segments {
        let percent = |value: &str| value.strip_suffix('%').and_then(|n| n.parse::<u8>().ok()).filter(|n| *n <= 100);
        if let Some(existing) = segments.iter_mut().find(|s| s.label == segment.label) {
            if percent(&segment.value).is_some_and(|value| percent(&existing.value).map_or(true, |old| value < old)) {
                existing.value = segment.value.clone();
                existing.value_color = segment.value_color;
            }
            existing.stale |= segment.stale;
        } else {
            segments.push(StripSegment { window_label: None, ..segment.clone() });
        }
    }
    StripModel { segments, tooltip: format!("간결 표시 · 서비스별 최소 잔여율\n{}", model.tooltip) }
}

fn antigravity_stale(snapshot: &ProviderUsageSnapshot, now: u64) -> bool {
    snapshot.connection_state != "connected"
        || !snapshot.last_synced_at.is_some_and(|captured| captured > 0 && captured <= now && now - captured < crate::antigravity::FRESH_SECS)
        || snapshot.windows.iter().any(|w| w.resets_at.is_some_and(|reset| reset <= now))
}

fn antigravity_status(snapshot: Option<&ProviderUsageSnapshot>, now: u64) -> &'static str {
    let Some(snapshot) = snapshot else { return "CLI 첫 수집 대기" };
    if !snapshot.runtime_available || snapshot.connection_state == "not-installed" { return "Antigravity CLI 설치 필요"; }
    if !snapshot.bridge_installed { return "CLI 사용량 연결 필요"; }
    if snapshot.auth_state == "signed-out" || snapshot.connection_state == "signed-out" { return "CLI 로그인 필요"; }
    if snapshot.connection_state == "error" { return LOOKUP_DELAYED; }
    if !usable(snapshot) { return "CLI 첫 수집 대기"; }
    if antigravity_stale(snapshot, now) { "갱신 대기 (CLI 캐시)" } else { "CLI 수집값" }
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
        return LOOKUP_DELAYED;
    }
    if snapshot.windows.is_empty() {
        return "사용량 갱신 대기";
    }
    // 모델별 한도는 전체 한도의 신선도 판정에 쓰지 않는다(provider_usage::claude_cache_is_stale과 같은 기준).
    let expired = snapshot.windows.iter().filter(|window| window.model_group_id.is_none())
        .any(|window| window.resets_at.is_some_and(|reset| reset <= now));
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
        let stale = id == "antigravity" && snapshot.is_some_and(|s| antigravity_stale(s, now));
        let mut append_segment = |window_label, remaining: Option<u8>| {
            any_value |= remaining.is_some();
            segments.push(StripSegment {
                label: name.to_string(),
                window_label,
                value: remaining.map(|percent| format!("{percent}%")).unwrap_or_else(|| NO_VALUE.to_string()),
                value_color: remaining.filter(|_| !stale).map(|percent| value_color(percent, colors)).unwrap_or(colors.label),
                stale,
            });
        };
        // Codex는 5시간 창이 있는 요금제(Plus 등)일 때만 두 기간을 나눠 표시하고,
        // 주간 창만 있는 요금제(Pro 등)는 기존 단일 값을 유지한다.
        let codex_has_short_window = id == "codex" && snapshot.filter(|s| usable(s))
            .is_some_and(|s| s.windows.iter().any(|window| window.id == "rolling" && window.model_group_id.is_none()));
        if id == "claude" || codex_has_short_window {
            // 데이터 순서·잔여량 크기·누락 여부에 따라 표시 기준이 바뀌지 않는다.
            // 모델별 한도(Claude의 Fable 등)는 칸에 쓰지 않고 tooltip에만 남긴다.
            for (window_id, label) in [("rolling", "5h"), ("weekly", "7d")] {
                let remaining = snapshot.filter(|s| usable(s)).and_then(|s| {
                    s.windows.iter().find(|window| window.id == window_id && window.model_group_id.is_none())
                        .and_then(|window| window_percent(s, window, now))
                });
                append_segment(Some(label), remaining);
            }
        } else {
            append_segment(None, snapshot.and_then(|s| provider_remaining(s, now)));
        }
        if let Some(snapshot) = snapshot.filter(|s| usable(s)) {
            let detail = snapshot.windows.iter()
                .map(|w| {
                    let group = w.model_group_label.as_deref().or(w.model_group_id.as_deref());
                    let label = match group {
                        Some(group) => format!("{group} / {}", w.label),
                        None if id == "antigravity" => format!("모델군 확인 대기 / {}", w.label),
                        None => w.label.clone(),
                    };
                    match window_percent(snapshot, w, now) {
                    Some(percent) => format!("{label} {percent}%"),
                    None => format!("{label} {NO_VALUE}"),
                    }
                })
                .collect::<Vec<_>>()
                .join(" · ");
            tooltip_lines.push(format!("{name}  {detail}"));
        } else if id != "antigravity" && (id != "claude" || snapshot.is_none()) {
            // "error"는 조회 자체의 실패라 계정 연결 문제로 표기하지 않는다.
            let status = if snapshot.is_some_and(|s| s.connection_state == "error") { LOOKUP_DELAYED } else { "연결되지 않음" };
            tooltip_lines.push(format!("{name}  {status}"));
        }
        if id == "antigravity" {
            tooltip_lines.push(format!("Antigravity · Gemini 기준 · {}", antigravity_status(snapshot, now)));
            tooltip_lines.push(format!("마지막 CLI 수집: {}", last_sync_label(snapshot.and_then(|s| s.last_synced_at), now)));
            tooltip_lines.push("Antigravity CLI 실행 시 갱신 · agy /usage · 직접 Claude 구독과 별도".to_string());
        }
        // 조회 실패를 넘기며 이어받은 Codex 값에도 Claude와 같은 최신성·수집 시각을 붙인다.
        if let Some(snapshot) = snapshot.filter(|s| id == "codex" && usable(s) && s.connection_state == "stale") {
            tooltip_lines.push("Codex · 갱신 대기 (캐시)".to_string());
            tooltip_lines.push(format!("마지막 동기화: {} (데이터 수집 기준)", last_sync_label(snapshot.last_synced_at, now)));
            if !snapshot.message.is_empty() {
                tooltip_lines.push(snapshot.message.clone());
            }
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

    // 모든 공급자에 값이 없으면 빈 막대를 띄우지 않는다.
    if !any_value {
        return None;
    }
    Some(StripModel {
        segments,
        tooltip: format!("SPECTRA · 개인 요금제 잔여\n{}", tooltip_lines.join("\n")),
    })
}

/// 스트립을 트레이 왼쪽 가장자리에 오른쪽 정렬하고 작업표시줄 안에서 세로 중앙에 놓는다.
/// 세로 작업표시줄이나 전체 스트립을 담을 공간이 없으면 None(=띄우지 않음).
pub fn place_strip(taskbar: Rect, edge: TaskbarEdge, tray_left: i32, size: (i32, i32)) -> Option<(i32, i32)> {
    if matches!(edge, TaskbarEdge::Left | TaskbarEdge::Right) {
        return None;
    }
    let (width, height) = size;
    if width <= 0 || height <= 0 || taskbar.right <= taskbar.left || taskbar.bottom <= taskbar.top
        || tray_left < taskbar.left || tray_left > taskbar.right {
        return None;
    }
    // 가상 데스크톱은 음수 좌표를 포함한다. 차이는 i64로 계산해 경계값에서도 넘치지 않게 한다.
    let available_width = i64::from(tray_left) - i64::from(taskbar.left);
    let available_height = i64::from(taskbar.bottom) - i64::from(taskbar.top);
    if i64::from(width) > available_width || i64::from(height) > available_height {
        return None;
    }
    let x = i64::from(tray_left) - i64::from(width);
    let y = i64::from(taskbar.top) + (available_height - i64::from(height)) / 2;
    Some((i32::try_from(x).ok()?, i32::try_from(y).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

    #[test]
    fn theme_changes_recolor_cached_values_without_changing_text_or_tooltip() {
        let mut model = StripModel {
            segments: ["0%", "19%", "20%", "49%", "50%", "100%", "—"].into_iter()
                .map(|value| StripSegment { label: "Claude".into(), window_label: Some("5h"), value: value.into(), value_color: [0; 3], stale: false })
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
            model_group_id: None,
            model_group_label: None,
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

    fn antigravity(windows: Vec<ProviderQuotaWindow>) -> ProviderUsageSnapshot {
        ProviderUsageSnapshot {
            source: Some("antigravity-statusline".into()), last_synced_at: Some(1_000), bridge_installed: true,
            ..snapshot("antigravity", "connected", windows)
        }
    }

    fn grouped_window(group: &str, id: &str, remaining: f64) -> ProviderQuotaWindow {
        ProviderQuotaWindow { model_group_id: Some(group.into()), ..window(id, remaining) }
    }

    fn gemini_window(id: &str, remaining: f64) -> ProviderQuotaWindow {
        grouped_window("gemini", id, remaining)
    }

    fn fable_window(id: &str, remaining: f64) -> ProviderQuotaWindow {
        ProviderQuotaWindow { model_group_label: Some("Fable".into()), label: "주간 한도".into(), ..grouped_window("fable", id, remaining) }
    }

    #[test]
    fn claude_per_model_limits_stay_out_of_the_segments_and_are_named_in_the_tooltip() {
        let segments = |windows: Vec<ProviderQuotaWindow>| {
            let model = build_model_at(&[snapshot("claude", "connected", windows)], StripTheme::Dark, 1_000).unwrap();
            let values = model.segments.iter().filter(|s| s.label == "Claude").map(|s| s.value.clone()).collect::<Vec<_>>();
            (values, model.tooltip)
        };
        let (plain, _) = segments(vec![window("rolling", 81.0), window("weekly", 74.0)]);
        let (with_model, tooltip) = segments(vec![window("rolling", 81.0), window("weekly", 74.0), fable_window("weekly", 3.0), fable_window("rolling", 1.0)]);
        assert_eq!(plain, ["81%", "74%"]);
        assert_eq!(with_model, plain);
        assert!(tooltip.contains("Fable / 주간 한도 3%"), "{tooltip}");

        // A per-model window never fills a missing account-wide one, whatever the order.
        let (weekly_missing, _) = segments(vec![fable_window("weekly", 3.0), window("rolling", 81.0)]);
        assert_eq!(weekly_missing, ["81%", "—"]);

        // Its passed reset does not turn the account-wide status into "갱신 대기".
        let mut claude = ProviderUsageSnapshot { source: Some("claude-usage-api".into()), last_synced_at: Some(1_000),
            ..snapshot("claude", "connected", vec![window("rolling", 81.0), fable_window("weekly", 3.0)]) };
        claude.windows[1].resets_at = Some(900);
        assert_eq!(claude_status(&claude, 1_000), "동기화됨");
        claude.windows[0].resets_at = Some(900);
        assert_eq!(claude_status(&claude, 1_000), "갱신 대기 (캐시)");
    }

    #[test]
    fn codex_single_value_ignores_model_group_windows() {
        let codex = snapshot("codex", "connected", vec![window("weekly", 55.0), grouped_window("some-model", "weekly", 2.0)]);
        assert_eq!(provider_remaining(&codex, 0), Some(55));
    }

    #[test]
    fn antigravity_value_follows_only_the_gemini_group() {
        let others = || vec![grouped_window("claude-gpt", "rolling", 100.0), grouped_window("claude-gpt", "weekly", 52.45),
            grouped_window("gemini-4-argon", "weekly", 76.25)];
        for (gemini, expected) in [
            (vec![gemini_window("rolling", 100.0), gemini_window("weekly", 99.76)], Some(100)),
            (vec![gemini_window("rolling", 40.0), gemini_window("weekly", 90.0)], Some(40)),
            (vec![gemini_window("rolling", 90.0), gemini_window("weekly", 40.0)], Some(40)),
            (vec![gemini_window("weekly", 0.0)], Some(0)),
            (vec![], None),
        ] {
            let ag = antigravity(others().into_iter().chain(gemini).collect());
            assert_eq!(provider_remaining(&ag, 1_100), expected);
            let model = build_model_at(&[ag, snapshot("codex", "connected", vec![window("weekly", 47.0)])], StripTheme::Dark, 1_100).unwrap();
            let segment = model.segments.last().unwrap();
            assert_eq!(segment.label, "Antigravity");
            assert_eq!(segment.value, expected.map_or("—".to_string(), |percent| format!("{percent}%")), "no other group stands in for Gemini");
            assert_eq!(compact_model(&model).segments.last().unwrap().value, segment.value);
            assert!(model.tooltip.contains("Antigravity · Gemini 기준"));
            assert!(model.tooltip.contains("claude-gpt / weekly 52%"), "other groups stay in the tooltip");
            assert!(model.tooltip.contains("gemini-4-argon / weekly 76%"));
        }
    }

    #[test]
    fn antigravity_with_no_gemini_value_does_not_raise_an_empty_strip() {
        let ag = antigravity(vec![grouped_window("claude-gpt", "weekly", 52.0)]);
        assert!(build_model_at(&[ag], StripTheme::Dark, 1_100).is_none());
    }

    #[test]
    fn antigravity_keeps_model_groups_separate_from_the_direct_claude_subscription() {
        let mut gemini = gemini_window("weekly", 83.0);
        gemini.model_group_label = Some("Gemini Models".into());
        let mut bundled = grouped_window("claude-gpt", "weekly", 31.0);
        bundled.model_group_label = Some("Claude and GPT models".into());
        let model = build_model_at(&[
            antigravity(vec![gemini, bundled]),
            snapshot("claude", "connected", vec![window("rolling", 96.0), window("weekly", 95.0)]),
            snapshot("codex", "connected", vec![window("weekly", 47.0)]),
        ], StripTheme::Dark, 1_100).unwrap();
        assert_eq!(model.segments.iter().map(|s| (s.label.as_str(), s.value.as_str())).collect::<Vec<_>>(),
            [("Codex", "47%"), ("Claude", "96%"), ("Claude", "95%"), ("Antigravity", "83%")]);
        assert!(model.tooltip.contains("Gemini Models / weekly 83%"));
        assert!(model.tooltip.contains("Claude and GPT models / weekly 31%"));
        assert!(model.tooltip.contains("직접 Claude 구독과 별도"));
        assert_eq!(model.segments.last().unwrap().value_color, palette(StripTheme::Dark).high);
    }

    #[test]
    fn antigravity_cache_turns_gray_after_a_day_and_stays_gray_after_retheme() {
        let ag = antigravity(vec![gemini_window("weekly", 83.0)]);
        let fresh_secs = crate::antigravity::FRESH_SECS;
        for (now, stale) in [(1_300, false), (1_000 + fresh_secs - 1, false), (1_000 + fresh_secs, true), (999, true)] {
            let mut model = build_model_at(&[ag.clone()], StripTheme::Light, now).unwrap();
            for theme in [StripTheme::Light, StripTheme::Dark] {
                retheme_model(&mut model, theme);
                let segment = model.segments.last().unwrap();
                assert_eq!(segment.value, "83%", "cached values retain their actual number");
                assert_eq!(segment.stale, stale);
                assert_eq!(segment.value_color, if stale { palette(theme).label } else { palette(theme).high });
                assert_eq!(model.tooltip.contains("갱신 대기 (CLI 캐시)"), stale);
            }
        }
    }

    #[test]
    fn antigravity_rejects_invalid_or_expired_windows_without_inventing_zero() {
        let mut expired = gemini_window("rolling", 3.0);
        expired.resets_at = Some(1_100);
        let ag = antigravity(vec![expired, gemini_window("weekly", 83.0), gemini_window("other", 0.0),
            gemini_window("rolling", f64::NAN), gemini_window("weekly", -1.0), gemini_window("weekly", 101.0), gemini_window("weekly", f64::INFINITY)]);
        assert_eq!(provider_remaining(&ag, 1_099), Some(3));
        assert_eq!(provider_remaining(&ag, 1_100), Some(83), "reset is enforced even before the next CLI cache refresh");
        let mut model = build_model_at(&[ag.clone()], StripTheme::Light, 1_100).unwrap();
        assert_eq!(model.segments.last().unwrap().value, "83%");
        assert!(model.segments.last().unwrap().stale);
        let invalid = antigravity(ag.windows.into_iter().filter(|w| w.remaining_percent != 83.0).collect());
        assert!(build_model_at(&[invalid], StripTheme::Dark, 1_100).is_none());
        let mut unsupported = antigravity(vec![gemini_window("weekly", 0.0)]);
        unsupported.source = None;
        assert_eq!(provider_remaining(&unsupported, 1_100), None);
        model = build_model_at(&[antigravity(vec![gemini_window("weekly", 0.0)])], StripTheme::Dark, 1_100).unwrap();
        assert_eq!(model.segments.last().unwrap().value, "0%", "a real zero remains valid");
    }

    #[test]
    fn antigravity_waiting_states_and_compact_values_do_not_turn_missing_into_zero() {
        let mut pending = antigravity(vec![]);
        assert_eq!(antigravity_status(Some(&pending), 1_100), "CLI 첫 수집 대기");
        pending.bridge_installed = false;
        assert_eq!(antigravity_status(Some(&pending), 1_100), "CLI 사용량 연결 필요");
        pending.runtime_available = false;
        assert_eq!(antigravity_status(Some(&pending), 1_100), "Antigravity CLI 설치 필요");
        let model = build_model_at(&[
            snapshot("codex", "connected", vec![window("rolling", 72.0), window("weekly", 55.0)]),
            snapshot("claude", "connected", vec![window("weekly", 0.0)]), pending,
        ], StripTheme::Dark, 1_100).unwrap();
        let compact = compact_model(&model);
        assert_eq!(compact.segments.iter().map(|s| (s.label.as_str(), s.value.as_str(), s.window_label)).collect::<Vec<_>>(),
            [("Codex", "55%", None), ("Claude", "0%", None), ("Antigravity", "—", None)]);
        assert!(compact.tooltip.contains(&model.tooltip));
        assert!(compact.tooltip.starts_with("간결 표시 · 서비스별 최소 잔여율"));
    }

    #[test]
    fn provider_remaining_takes_the_binding_window_not_the_first() {
        let s = snapshot("codex", "connected", vec![window("rolling", 59.0), window("weekly", 31.4)]);
        assert_eq!(provider_remaining(&s, 0), Some(31));
    }

    #[test]
    fn provider_remaining_accepts_stale_but_not_disconnected() {
        assert_eq!(provider_remaining(&snapshot("codex", "stale", vec![window("weekly", 55.0)]), 0), Some(55));
        assert_eq!(provider_remaining(&snapshot("codex", "signed-out", vec![window("weekly", 55.0)]), 0), None);
        assert_eq!(provider_remaining(&snapshot("codex", "connected", vec![]), 0), None);
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
        assert_eq!(model.segments.len(), 4);
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
                assert_eq!(model.segments.len(), 5);
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
        assert_eq!(model.segments.len(), 4);
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
            ("error", "사용량 확인 지연"), ("not-installed", "Claude Code 설치 필요")] {
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
    fn carried_values_past_their_reset_show_a_dash_until_a_new_lookup() {
        let mut claude = snapshot("claude", "stale", vec![window("rolling", 9.0), window("weekly", 40.0)]);
        claude.windows[0].resets_at = Some(1_000);
        let mut codex = snapshot("codex", "stale", vec![window("rolling", 12.0), window("weekly", 55.0)]);
        codex.windows[0].resets_at = Some(1_000);
        let values = |model: &StripModel| model.segments.iter().map(|s| s.value.clone()).collect::<Vec<_>>();

        let before = build_model_at(&[codex.clone(), claude.clone()], StripTheme::Dark, 999).unwrap();
        assert_eq!(values(&before), ["12%", "55%", "9%", "40%", "—"]);
        let after = build_model_at(&[codex.clone(), claude.clone()], StripTheme::Dark, 1_000).unwrap();
        assert_eq!(values(&after), ["—", "55%", "—", "40%", "—"], "a pre-reset value is not today's value");
        assert_eq!(after.segments.len(), 5, "the layout does not change at the reset");
        assert!(after.tooltip.contains("Claude  rolling — · weekly 40%"));

        // A weekly-only Codex plan has a single value; it also turns into a dash.
        let mut weekly_only = snapshot("codex", "stale", vec![window("weekly", 55.0)]);
        weekly_only.windows[0].resets_at = Some(1_000);
        assert_eq!(provider_remaining(&weekly_only, 1_000), None);

        // Fresh values are refreshed at the boundary by the scheduler and stay untouched.
        claude.connection_state = "connected".into();
        let fresh = build_model_at(&[claude], StripTheme::Dark, 1_000).unwrap();
        assert_eq!(fresh.segments[1].value, "9%");
    }

    #[test]
    fn a_lookup_error_without_history_reads_as_delayed_not_disconnected() {
        let mut codex_error = snapshot("codex", "error", vec![]);
        codex_error.live_failure = Some("codex-app-server-timeout".into());
        let mut claude_error = snapshot("claude", "error", vec![]);
        claude_error.live_failure = Some("claude-auth-status-failed".into());
        let model = build_model_at(&[codex_error, snapshot("claude", "connected", vec![window("rolling", 40.0)])], StripTheme::Light, 1_000).unwrap();
        assert!(model.tooltip.contains("Codex  사용량 확인 지연"));
        assert!(!model.tooltip.contains("연결되지 않음"));
        assert_eq!(claude_status(&claude_error, 1_000), "사용량 확인 지연");
        let missing = build_model_at(&[snapshot("claude", "connected", vec![window("rolling", 40.0)])], StripTheme::Light, 1_000).unwrap();
        assert!(missing.tooltip.contains("Codex  연결되지 않음"), "a provider with no snapshot is still not connected");
    }

    #[test]
    fn carried_codex_values_show_their_capture_age_like_claude() {
        let mut codex = snapshot("codex", "stale", vec![window("weekly", 55.0)]);
        codex.last_synced_at = Some(1_000);
        codex.live_failure = Some("codex-app-server-timeout".into());
        codex.message = "Codex App Server 응답이 없어 이번 확인을 마치지 못했습니다. (codex-app-server-timeout) 마지막 동기화 값을 표시합니다.".into();
        let model = build_model_at(&[codex.clone()], StripTheme::Dark, 1_300).unwrap();
        assert_eq!(model.segments[0].value, "55%", "the carried value stays visible");
        assert!(model.tooltip.contains("Codex · 갱신 대기 (캐시)"));
        assert!(model.tooltip.contains("마지막 동기화: 5분 전 (데이터 수집 기준)"));
        assert!(model.tooltip.contains("마지막 동기화 값을 표시합니다."));
        codex.connection_state = "connected".into();
        let fresh = build_model_at(&[codex], StripTheme::Dark, 1_300).unwrap();
        assert!(!fresh.tooltip.contains("Codex · 갱신 대기"), "fresh Codex values carry no stale marker");
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
    fn place_strip_hides_when_the_tray_leaves_insufficient_space_and_recovers() {
        let taskbar = Rect { left: 0, top: 1020, right: 1920, bottom: 1080 };
        for (tray_left, expected) in [(120, None), (199, None), (200, Some((0, 1038))), (1425, Some((1225, 1038)))] {
            assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, tray_left, (200, 24)), expected);
        }
    }

    #[test]
    fn place_strip_preserves_bounds_across_dpi_and_negative_monitor_coordinates() {
        for dpi in [96, 120, 144] {
            let size = (200 * dpi / 96, 24 * dpi / 96);
            let taskbar = Rect { left: -1920, top: -60 * dpi / 96, right: 0, bottom: 0 };
            for edge in [TaskbarEdge::Top, TaskbarEdge::Bottom] {
                for tray in [taskbar.left + size.0 - 1, taskbar.left + size.0, -400, taskbar.right] {
                    let position = place_strip(taskbar, edge, tray, size);
                    if tray - taskbar.left < size.0 {
                        assert_eq!(position, None);
                    } else {
                        let (x, y) = position.unwrap();
                        assert_eq!(x + size.0, tray);
                        assert!(x >= taskbar.left && x + size.0 <= taskbar.right);
                        assert!(y >= taskbar.top && y + size.1 <= taskbar.bottom);
                        assert_eq!(y - taskbar.top, (taskbar.bottom - taskbar.top - size.1) / 2);
                    }
                }
            }
        }
    }

    #[test]
    fn place_strip_rejects_invalid_dimensions_and_tray_coordinates() {
        let taskbar = Rect { left: 0, top: 1020, right: 1920, bottom: 1080 };
        for size in [(0, 24), (-1, 24), (200, 0), (200, -1), (200, 61), (i32::MAX, 24)] {
            assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, 1425, size), None);
        }
        for tray in [-1, 0, 1921, i32::MIN, i32::MAX] {
            assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, tray, (200, 24)), None);
        }
        for invalid in [
            Rect { left: 0, top: 0, right: 0, bottom: 60 },
            Rect { left: 10, top: 0, right: 0, bottom: 60 },
            Rect { left: 0, top: 60, right: 1920, bottom: 60 },
            Rect { left: 0, top: 61, right: 1920, bottom: 60 },
        ] {
            assert_eq!(place_strip(invalid, TaskbarEdge::Bottom, 0, (200, 24)), None);
        }
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, 200, (200, 60)), Some((0, 1020)));
    }

    #[test]
    fn place_strip_avoids_overflow_at_coordinate_limits() {
        let taskbar = Rect { left: i32::MIN, top: i32::MIN, right: i32::MAX, bottom: i32::MAX };
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, i32::MIN + 199, (200, 24)), None);
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, i32::MIN + 200, (200, 24)), Some((i32::MIN, -13)));
        assert_eq!(place_strip(taskbar, TaskbarEdge::Bottom, i32::MAX, (200, 24)), Some((i32::MAX - 200, -13)));
    }
}
