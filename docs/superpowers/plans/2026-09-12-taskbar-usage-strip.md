# 작업표시줄 사용량 스트립 (B안) 구현 계획

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 설정에서 "작업표시줄 표시"를 켜면 Windows 작업표시줄 알림 영역 왼쪽에 `Codex NN% · Claude NN%`를 창 없이 상시 표시하고, 대기(standby) 상태 총 작업 집합을 35 MB 이하·프로세스 1개로 유지한다. 기본값은 꺼짐이다.

**Architecture:** 두 번째 WebView 창을 만들지 않는다 — WebView2 프로세스가 대기 상태에서도 살아남아 Phase 4의 31.5 MB가 무너지기 때문이다. 대신 `windows-sys`로 `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST` 창 1개를 전용 스레드에서 만들고 `UpdateLayeredWindow`로 GDI 렌더 결과를 올린다. 순수 계산(공급자별 최소 잔여율 선택·색·배치 산술)은 `taskbar_strip.rs`에 Win32 없이 두어 전부 단위 테스트하고, Win32는 `taskbar_window.rs`에 격리한다. 값 갱신은 기존 `update_tray_badge` 호출 지점에 얹어 폴링 없이 스냅샷 완료 시점에만 일어난다.

**Tech Stack:** Rust 1.96 · Tauri 2.11.5 · `windows-sys` 0.61.2(신규 직접 의존, 트리에 기존재) · GDI(`DrawTextW`·DIB section) · React 19 + TypeScript · Node 24 `node:test`

**Spec:** `docs/superpowers/specs/2026-09-12-taskbar-usage-strip.md` — 전체(§1 성공 기준 표, §3 설계 결정, §4 의존성)

## Global Constraints

- **의존성**: `windows-sys = { version = "0.61.2", ... }`를 `[target.'cfg(target_os = "windows")'.dependencies]`에만 추가한다. 다른 신규 크레이트 금지. 추가 후 `Cargo.lock`이 **바이트 동일**해야 한다(features는 lock에 기록되지 않음) — Task 2에서 SHA-256으로 검증
- **기본값은 현행 동작**: `UiPrefs::default().strip == false` → 스트립 창을 아예 만들지 않는다
- **폴링 금지**: 데이터 목적 `SetTimer`/스레드 sleep 루프 0개. 값 갱신은 `provider_usage_snapshot` 완료 시점에만. 셸 이벤트(`WM_SETTINGCHANGE`·`WM_DISPLAYCHANGE`·`TaskbarCreated`)로만 위치·테마를 재확인
- **테마 출처**: 스트립은 `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Themes\Personalize\SystemUsesLightTheme`를 따른다. `UiPrefs.theme`와 **연결 금지**
- **좌표계**: 프로세스가 DPI-aware이므로 `GetWindowRect`·`SHAppBarMessage` 모두 물리 픽셀이다. 논리 픽셀을 가정한 상수를 두지 않는다
- **범위**: 주 작업표시줄·가로(TOP/BOTTOM)만. `Shell_SecondaryTrayWnd`·세로 작업표시줄은 스트립을 띄우지 않고 조용히 건너뛴다
- **프론트 금지 패턴 유지**: `localStorage`·`sessionStorage`·`setInterval`·`requestAnimationFrame` 금지. 선호 영속은 Rust `ui-prefs.json`만
- **테스트 기준선(2026-09-12 실측)**: `cargo test --lib` 기본 **44 passed** · `npm test` **32 passed**. 각 Task의 Expected는 이 수에 신규분을 더해 적는다
- **두 feature 구성 모두 통과해야 커밋**: `cargo build --release`(기본)·`--features native-oauth`, `cargo test --lib` 양쪽. 경고 0
- **프론트 게이트(프론트 변경 시)**: `npm run build` · `npm run verify:tokens` · `npm run verify:memory` · `npm run verify:baseline` · `npm test`
- **Rust 테스트는 `SPECTRA_DATA_DIR`를 변경하지 않는다**(병렬 실행 오염). 파일 IO는 경로를 인자로 받는 형태로 두고 임시 디렉터리로 테스트
- **Win32 단위 테스트를 가장하지 않는다**: 창 생성·GDI·셸 조회는 단위 테스트 대상이 아니다. 해당 Task의 게이트는 명시된 **육안 검증(스크린샷)**이며, 통과 전 완료 선언 금지(CLAUDE.md "완료 선언 전 실물 검증 필수")
- dev server·watch·30분 측정은 실행자가 직접 띄우지 않는다(명령 제시 + 사용자 협조). 릴리스 빌드·`cargo test`·릴리스 exe 1회 실행(사용자 고지 후)은 허용
- 파일 삭제 금지(빌드 산출물 제외)
- 커밋 메시지 말미: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`

## File Structure

| 파일 | 역할 | 작업 |
|---|---|---|
| `src-tauri/src/taskbar_strip.rs` | **Win32 없음.** 공급자별 최소 잔여율 선택, 세그먼트 모델, 팔레트, 배치 산술, 툴팁 문구 | 신규 |
| `src-tauri/src/taskbar_window.rs` | **Win32 전용.** 창 클래스·전용 스레드·메시지 루프·레이어드 창·GDI 텍스트·작업표시줄 조회·셸 이벤트 | 신규 |
| `src-tauri/Cargo.toml` | `windows-sys` 직접 의존(Windows 타깃 한정) | 수정 |
| `src-tauri/src/lib.rs` | `mod` 선언 2개, `AppState.strip` 핸들, `set_strip` 커맨드, 스냅샷 후 스트립 갱신 | 수정 |
| `src-tauri/src/standby.rs` | `UiPrefs.strip` 필드, `BootState.strip` | 수정 |
| `src-tauri/src/desktop_shell.rs` | `update_tray_badge` 옆 `update_taskbar_strip`, 스트립 클릭 → 창 토글 | 수정 |
| `src/integrations/boot-state.ts` | `NativeUiPrefs.strip`(없으면 `false`) | 수정 |
| `src/integrations/tauri-native-bridge.ts` | `setNativeStrip` | 수정 |
| `src/App.tsx` | `strip` 상태·토글·설정 행 | 수정 |
| `tests/boot-state.test.ts` | `strip` 파싱 테스트 | 수정 |
| `scripts/verify-memory-budget.mjs` | 스캔 대상 유지 확인 | 수정(필요 시) |
| `docs/performance/memory-budget.md` · `README.md` · 스펙 상태 줄 | 측정·검증 기록 | 수정 |

---

### Task 1: `taskbar_strip.rs` — 순수 모델·팔레트·배치 산술

Win32를 전혀 쓰지 않는 계층. 이 Task만으로 스트립의 "무엇을 어디에 그릴지"가 전부 결정되고 테스트된다. 의존성 추가는 Task 2에서 한다.

**Files:**
- Create: `src-tauri/src/taskbar_strip.rs`
- Modify: `src-tauri/src/lib.rs` — `mod taskbar_strip;` 선언만(배선은 Task 4)

**Interfaces:**
- Consumes: `provider_usage::ProviderUsageSnapshot`(필드 `provider_id`·`connection_state`·`windows`), `provider_usage::ProviderQuotaWindow`(필드 `id`·`label`·`remaining_percent`·`resets_at`)
- Produces (Task 2·3·4가 사용):
  ```rust
  #[derive(Clone, Copy, Debug, PartialEq)] pub enum StripTheme { Light, Dark }
  #[derive(Clone, Copy, Debug, PartialEq)] pub enum TaskbarEdge { Left, Top, Right, Bottom }
  #[derive(Clone, Copy, Debug, Default, PartialEq)] pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }

  #[derive(Clone, Copy, Debug, PartialEq)]
  pub struct Palette { pub high: [u8;3], pub mid: [u8;3], pub low: [u8;3], pub label: [u8;3], pub separator: [u8;3] }
  pub fn palette(theme: StripTheme) -> Palette;

  /// 그 공급자의 한도 창 중 최소 잔여율. 미연결이거나 창이 없으면 None.
  pub fn provider_remaining(snapshot: &ProviderUsageSnapshot) -> Option<u8>;

  #[derive(Clone, Debug, PartialEq)]
  pub struct StripSegment { pub label: String, pub value: String, pub value_color: [u8;3] }

  #[derive(Clone, Debug, PartialEq)]
  pub struct StripModel { pub segments: Vec<StripSegment>, pub tooltip: String }

  /// 고정 순서(codex → claude). 모든 공급자가 미연결이면 None(=스트립 숨김).
  pub fn build_model(snapshots: &[ProviderUsageSnapshot], theme: StripTheme) -> Option<StripModel>;

  /// 스트립 왼쪽 위 좌표(물리 px). 세로 작업표시줄이면 None.
  pub fn place_strip(taskbar: Rect, edge: TaskbarEdge, tray_left: i32, size: (i32, i32)) -> Option<(i32, i32)>;
  ```

- [ ] **Step 1: 실패하는 테스트 작성**

`src-tauri/src/taskbar_strip.rs`를 만들고 **테스트 모듈만** 먼저 넣는다(상단에 `use` 선언 포함, 구현은 Step 3).

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_usage::{ProviderQuotaWindow, ProviderUsageSnapshot};

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
```

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cd src-tauri && cargo test --lib --offline taskbar_strip`
Expected: 컴파일 실패 — `cannot find function provider_remaining`, `cannot find type StripTheme` 등

- [ ] **Step 3: 최소 구현 작성**

`src-tauri/src/taskbar_strip.rs`의 테스트 모듈 **위에** 넣는다.

```rust
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
```

`src-tauri/src/lib.rs`의 `mod` 목록에 알파벳 순서를 유지해 추가한다(`mod standby;` 다음 줄):

```rust
mod taskbar_strip;
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline 2>&1 | tail -3`
Expected: `57 passed; 0 failed` (기준선 44 + 신규 13)

- [ ] **Step 5: 경고 0 확인**

Run: `cd src-tauri && cargo build --offline 2>&1 | grep -c warning`
Expected: `0`

`mod taskbar_strip;`만 선언하고 아직 아무도 쓰지 않으므로 dead_code 경고가 날 수 있다. 그 경우 모듈 선언 위에 `#[allow(dead_code)] // Task 4에서 배선` 을 붙이고, **Task 4 Step 5에서 이 attribute를 제거**한다.

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/taskbar_strip.rs src-tauri/src/lib.rs
git commit -m "feat(strip): add the pure taskbar-strip model, palette and placement maths

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: 의존성 추가 + 레이어드 창 띄우기 (단색)

텍스트 없이 **팔레트 배경색 사각형**만 올려서 "창이 올바른 자리에 뜨고, 포커스를 뺏지 않고, 작업표시줄 위에 남는가"를 먼저 확정한다. 텍스트는 Task 3.

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/taskbar_window.rs`
- Modify: `src-tauri/src/lib.rs` — `mod taskbar_window;`(Windows 한정)

**Interfaces:**
- Consumes: `taskbar_strip::{Rect, StripModel, StripTheme, TaskbarEdge, palette, place_strip}`
- Produces (Task 3·4·5가 사용):
  ```rust
  /// 스트립 스레드로 가는 손잡이. HWND는 isize로 보관해 Send를 만족시킨다.
  pub struct StripHandle { /* private */ }

  /// 스레드와 창을 만들고 HWND가 준비된 뒤 반환한다. 작업표시줄을 찾지 못하거나
  /// 세로 작업표시줄이면 Err.
  pub fn spawn(model: StripModel, on_click: Box<dyn Fn() + Send + 'static>) -> Result<StripHandle, String>;

  impl StripHandle {
      pub fn update(&self, model: Option<StripModel>);  // None이면 창을 숨긴다
      pub fn shutdown(&self);
  }

  /// 작업표시줄 테마(UiPrefs.theme 아님 — 스펙 §3-5).
  pub fn current_theme() -> StripTheme;
  ```

- [ ] **Step 1: 의존성 추가 전 `Cargo.lock` 해시 기록**

Run:
```bash
cd src-tauri && sha256sum Cargo.lock
```
Expected: 해시 문자열 1줄. 이 값을 메모해 Step 3에서 대조한다.

- [ ] **Step 2: `Cargo.toml`에 의존성 추가**

`[target.'cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))'.dependencies]` 절 **아래**에 새 절을 추가한다.

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows-sys = { version = "0.61.2", features = [
    "Win32_Foundation",
    "Win32_Graphics_Gdi",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_HiDpi",
    "Win32_UI_Shell",
    "Win32_UI_WindowsAndMessaging",
] }
```

- [ ] **Step 3: `Cargo.lock`이 바뀌지 않았는지 확인**

Run:
```bash
cd src-tauri && cargo metadata --offline --format-version 1 > /dev/null && sha256sum Cargo.lock
```
Expected: Step 1과 **동일한 해시**. 달라지면 버전 지정이 트리의 해석 버전과 어긋난 것이므로, `cargo tree -i windows-sys@0.61.2`로 실제 버전을 다시 확인하고 `Cargo.toml`을 맞춘 뒤 `git checkout Cargo.lock`으로 되돌린다.

- [ ] **Step 4: `taskbar_window.rs` 작성**

```rust
#![cfg(target_os = "windows")]

use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BLENDFUNCTION, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HDC,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Shell::{SHAppBarMessage, ABM_GETTASKBARPOS, APPBARDATA};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use crate::taskbar_strip::{palette, place_strip, Rect as StripRect, StripModel, StripTheme, TaskbarEdge};

const CLASS_NAME: &str = "SpectraTaskbarStrip";
const WM_STRIP_REDRAW: u32 = WM_APP + 1;
const STRIP_HEIGHT: i32 = 24;
const STRIP_WIDTH: i32 = 200; // Task 3에서 실제 텍스트 폭으로 대체

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 작업표시줄 테마. UiPrefs.theme와 연결하지 않는다(스펙 §3-5).
pub fn current_theme() -> StripTheme {
    let sub = wide("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let name = wide("SystemUsesLightTheme");
    let mut value: u32 = 0;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            sub.as_ptr(),
            name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut value as *mut u32 as *mut _,
            &mut size,
        )
    };
    // 값이 없으면 Windows 기본값(다크 작업표시줄)을 따른다.
    if status == 0 && value == 1 { StripTheme::Light } else { StripTheme::Dark }
}

/// 작업표시줄 사각형과 붙은 모서리.
fn taskbar_rect() -> Option<(StripRect, TaskbarEdge)> {
    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    if unsafe { SHAppBarMessage(ABM_GETTASKBARPOS, &mut data) } == 0 {
        return None;
    }
    let edge = match data.uEdge {
        0 => TaskbarEdge::Left,
        1 => TaskbarEdge::Top,
        2 => TaskbarEdge::Right,
        _ => TaskbarEdge::Bottom,
    };
    let rc = data.rc;
    Some((StripRect { left: rc.left, top: rc.top, right: rc.right, bottom: rc.bottom }, edge))
}

/// 알림 영역(트레이~시계)의 왼쪽 가장자리. 실측상 Windows 11 26200에서도 존재한다.
fn tray_left() -> Option<i32> {
    let tray = unsafe { FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null()) };
    if tray.is_null() {
        return None;
    }
    let notify = unsafe { FindWindowExW(tray, std::ptr::null_mut(), wide("TrayNotifyWnd").as_ptr(), std::ptr::null()) };
    if notify.is_null() {
        return None;
    }
    let mut rc: RECT = unsafe { std::mem::zeroed() };
    if unsafe { GetWindowRect(notify, &mut rc) } == 0 {
        return None;
    }
    Some(rc.left)
}

struct StripState {
    model: Arc<Mutex<Option<StripModel>>>,
    on_click: Box<dyn Fn() + Send + 'static>,
}

pub struct StripHandle {
    hwnd: isize,
    model: Arc<Mutex<Option<StripModel>>>,
}

// HWND를 isize로 들고 PostMessage로만 건드리므로 스레드 간 이동이 안전하다.
unsafe impl Send for StripHandle {}
unsafe impl Sync for StripHandle {}

impl StripHandle {
    pub fn update(&self, model: Option<StripModel>) {
        if let Ok(mut slot) = self.model.lock() {
            *slot = model;
        }
        unsafe { PostMessageW(self.hwnd as HWND, WM_STRIP_REDRAW, 0, 0) };
    }

    pub fn shutdown(&self) {
        unsafe { PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0) };
    }
}

pub fn spawn(model: StripModel, on_click: Box<dyn Fn() + Send + 'static>) -> Result<StripHandle, String> {
    let shared = Arc::new(Mutex::new(Some(model)));
    let thread_model = Arc::clone(&shared);
    let (tx, rx) = std::sync::mpsc::channel::<Result<isize, String>>();

    std::thread::Builder::new()
        .name("spectra-taskbar-strip".to_string())
        .spawn(move || {
            let hwnd = match create_window(StripState { model: thread_model, on_click }) {
                Ok(hwnd) => {
                    let _ = tx.send(Ok(hwnd as isize));
                    hwnd
                }
                Err(error) => {
                    let _ = tx.send(Err(error));
                    return;
                }
            };
            redraw(hwnd);
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) } > 0 {
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        })
        .map_err(|error| format!("strip thread could not start: {error}"))?;

    let hwnd = rx.recv().map_err(|_| "strip thread ended before reporting".to_string())??;
    Ok(StripHandle { hwnd, model: shared })
}

fn create_window(state: StripState) -> Result<HWND, String> {
    let class = wide(CLASS_NAME);
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let wc = WNDCLASSW {
        style: 0,
        lpfnWndProc: Some(wnd_proc),
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: instance as _,
        hIcon: std::ptr::null_mut(),
        hCursor: unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) },
        hbrBackground: std::ptr::null_mut(),
        lpszMenuName: std::ptr::null(),
        lpszClassName: class.as_ptr(),
    };
    // 이미 등록돼 있으면 0이 돌아오지만 그대로 진행해도 된다(재생성 경로).
    unsafe { RegisterClassW(&wc) };

    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
            class.as_ptr(),
            wide("SPECTRA").as_ptr(),
            WS_POPUP,
            0,
            0,
            STRIP_WIDTH,
            STRIP_HEIGHT,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance as _,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        return Err("strip window could not be created".to_string());
    }
    let boxed = Box::into_raw(Box::new(state));
    unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, boxed as isize) };
    Ok(hwnd)
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_STRIP_REDRAW => {
            redraw(hwnd);
            0
        }
        WM_LBUTTONUP => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState;
            if let Some(state) = ptr.as_ref() {
                (state.on_click)();
            }
            0
        }
        WM_DESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StripState;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Task 2에서는 팔레트 배경색 사각형만 올린다. 텍스트는 Task 3.
fn redraw(hwnd: HWND) {
    let state = unsafe { (GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState).as_ref() };
    let Some(state) = state else { return };
    let model = state.model.lock().ok().and_then(|slot| slot.clone());
    let Some(_model) = model else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return;
    };
    let (Some((taskbar, edge)), Some(tray)) = (taskbar_rect(), tray_left()) else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return;
    };
    let Some((x, y)) = place_strip(taskbar, edge, tray, (STRIP_WIDTH, STRIP_HEIGHT)) else {
        unsafe { ShowWindow(hwnd, SW_HIDE) };
        return;
    };

    let colors = palette(current_theme());
    let mut pixels: *mut std::ffi::c_void = std::ptr::null_mut();
    let header = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: STRIP_WIDTH,
        biHeight: -STRIP_HEIGHT, // top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB as u32,
        biSizeImage: 0,
        biXPelsPerMeter: 0,
        biYPelsPerMeter: 0,
        biClrUsed: 0,
        biClrImportant: 0,
    };
    let info = BITMAPINFO { bmiHeader: header, bmiColors: unsafe { std::mem::zeroed() } };

    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let dc = CreateCompatibleDC(screen);
        let bitmap = CreateDIBSection(dc, &info, DIB_RGB_COLORS, &mut pixels, std::ptr::null_mut(), 0);
        let old = SelectObject(dc, bitmap as _);

        // 확인용 반투명 채움: BGRA premultiplied
        let buffer = std::slice::from_raw_parts_mut(pixels as *mut u8, (STRIP_WIDTH * STRIP_HEIGHT * 4) as usize);
        for px in buffer.chunks_exact_mut(4) {
            px[0] = colors.high[2] / 2;
            px[1] = colors.high[1] / 2;
            px[2] = colors.high[0] / 2;
            px[3] = 128;
        }

        let mut position = POINT { x, y };
        let mut size = windows_sys::Win32::Foundation::SIZE { cx: STRIP_WIDTH, cy: STRIP_HEIGHT };
        let mut source = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
        UpdateLayeredWindow(hwnd, screen, &mut position, &mut size, dc, &mut source, 0 as COLORREF, &blend, ULW_ALPHA);
        SetWindowPos(hwnd, HWND_TOPMOST, x, y, STRIP_WIDTH, STRIP_HEIGHT, SWP_NOACTIVATE | SWP_SHOWWINDOW);

        SelectObject(dc, old);
        DeleteObject(bitmap as _);
        DeleteDC(dc);
        ReleaseDC(std::ptr::null_mut(), screen);
    }
}
```

`src-tauri/src/lib.rs`의 `mod taskbar_strip;` 다음 줄에 추가한다:

```rust
#[cfg(target_os = "windows")]
mod taskbar_window;
```

- [ ] **Step 5: 빌드·경고 0 확인**

Run: `cd src-tauri && cargo build --offline 2>&1 | tail -20`
Expected: `Finished`, 경고 0. `unused` 계열 경고는 Task 4 배선 전이라 날 수 있으므로 `#[allow(dead_code)]`를 모듈 선언에 붙이고 Task 4 Step 5에서 제거한다.

- [ ] **Step 6: 기존 테스트 회귀 없음 확인**

Run: `cd src-tauri && cargo test --lib --offline 2>&1 | tail -3`
Expected: `57 passed; 0 failed` (Task 1과 동일 — 이 Task는 테스트를 추가하지 않는다)

- [ ] **Step 7: 육안 검증 — 임시 진입점으로 창 띄우기**

Win32 창은 단위 테스트로 확인할 수 없으므로 실제로 띄워 본다. `src-tauri/src/main.rs`의 CLI 분기 옆에 **임시** 플래그를 넣는다(Task 5 종료 시 제거).

```rust
// TEMP(Task 2 육안 검증): `spectra-native.exe --probe-strip`
#[cfg(target_os = "windows")]
if std::env::args().any(|arg| arg == "--probe-strip") {
    spectra_lib::probe_strip_for_20s();
    return;
}
```

`lib.rs`에 대응 헬퍼를 추가한다(Task 5에서 제거). **`StripHandle`을 반환하면 안 된다** — `taskbar_window`가 비공개 모듈이라 `pub fn`의 시그니처에 그 타입이 나타나면 E0446(private type in public interface)로 컴파일이 막힌다. spawn·대기·종료를 함수 안에서 끝내고 `()`를 돌려준다.

```rust
/// TEMP(Task 2·3 육안 검증) — Task 5 Step 3에서 제거한다.
#[cfg(target_os = "windows")]
#[doc(hidden)]
pub fn probe_strip_for_20s() {
    let model = taskbar_strip::StripModel {
        segments: vec![
            taskbar_strip::StripSegment { label: "Codex".into(), value: "55%".into(), value_color: taskbar_strip::palette(taskbar_window::current_theme()).high },
            taskbar_strip::StripSegment { label: "Claude".into(), value: "59%".into(), value_color: taskbar_strip::palette(taskbar_window::current_theme()).high },
        ],
        tooltip: "probe".to_string(),
    };
    match taskbar_window::spawn(model, Box::new(|| eprintln!("strip clicked"))) {
        Ok(handle) => {
            std::thread::sleep(std::time::Duration::from_secs(20));
            handle.shutdown();
        }
        Err(error) => eprintln!("spectra: strip probe failed: {error}"),
    }
}
```

Run: `cargo build --release --offline && ./target/release/spectra-native.exe --probe-strip`

Expected — 사용자에게 스크린샷 요청. 다음 4가지를 확인한다:
1. 작업표시줄 알림 영역(`^`) **바로 왼쪽**에 반투명 청록 막대가 20초간 보인다
2. 막대를 클릭해도 **다른 창의 포커스가 빼앗기지 않는다**
3. 다른 창을 최대화해도 막대가 작업표시줄 위에 **남아 있다**
4. 20초 뒤 스스로 사라진다

4가지 중 하나라도 실패하면 진행하지 말고 사용자에게 보고한다. 특히 3번이 실패하면 스펙 §3-2의 경로 ②(`SetParent`)로 전환하는 판단이 필요하다.

- [ ] **Step 8: 커밋**

```bash
git add src-tauri/Cargo.toml src-tauri/src/taskbar_window.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat(strip): create the layered taskbar window and place it beside the tray

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: GDI 텍스트 렌더링과 실제 폭 계산

**Files:**
- Modify: `src-tauri/src/taskbar_window.rs`

**Interfaces:**
- Consumes: Task 1의 `StripModel`·`Palette`, Task 2의 `redraw`·`current_theme`·`StripState`
- Produces: `redraw`가 실제 텍스트를 그린다. 창 크기가 `STRIP_WIDTH`/`STRIP_HEIGHT` 상수가 아니라 **측정된 텍스트 크기**가 되고(두 상수는 삭제), `StripState`가 `font: HFONT`·`font_dpi: u32`를 소유한다.

- [ ] **Step 1: 셸 글꼴 획득 함수 추가 (DPI 반영 + 안티앨리어싱 끄기)**

Segoe UI 하드코딩은 한국어 폴백을 깨뜨리므로 셸이 알려주는 글꼴을 쓴다(스펙 §3-4). 두 가지를 반드시 함께 처리한다.

**(1) DPI.** 이 기기는 125%다. `SystemParametersInfoW`는 96-DPI 기준 `lfStatusFont`를 돌려주므로 그대로 쓰면 글자가 작업표시줄 시계보다 눈에 띄게 작아져 Step 5 검증 2번이 실패한다. 창의 실제 DPI로 `SystemParametersInfoForDpi`를 쓴다.

**(2) 안티앨리어싱.** `DrawTextW`는 글자 가장자리를 **DIB 배경(전부 0 = 검정)과 블렌딩**한다. 뒤에서 알파를 복원하면 그 어두운 경계 픽셀까지 불투명해져 라이트 작업표시줄에서 글자마다 검은 후광이 생긴다. 배경을 불투명하게 채우는 통상적 회피책은 Step 5 검증 5번이 금지하고, Win11 아크릴 색을 맞추는 것도 현실적이지 않다. `lfQuality = NONANTIALIASED_QUALITY`로 두면 모든 픽셀이 정확히 `SetTextColor` 값이 되어 알파 복원이 정확해지고 투명도가 유지된다.

```rust
use windows_sys::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT, LOGFONTW, NONANTIALIASED_QUALITY};
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, SystemParametersInfoForDpi};
use windows_sys::Win32::UI::WindowsAndMessaging::{NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};

fn shell_font(dpi: u32) -> HFONT {
    let mut metrics: NONCLIENTMETRICSW = unsafe { std::mem::zeroed() };
    metrics.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
    let ok = unsafe {
        SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS,
            metrics.cbSize,
            &mut metrics as *mut _ as *mut _,
            0,
            dpi,
        )
    };
    let mut lf: LOGFONTW = if ok != 0 { metrics.lfStatusFont } else { unsafe { std::mem::zeroed() } };
    if ok == 0 {
        // 폴백도 DPI를 반영한다. 96 고정은 125%·150% 기기에서 작게 보인다.
        lf.lfHeight = -((12 * dpi as i32) / 96);
    }
    // 알파 복원이 정확하려면 글자 픽셀이 순색이어야 한다(위 (2)).
    lf.lfQuality = NONANTIALIASED_QUALITY;
    unsafe { CreateFontIndirectW(&lf) }
}
```

> import 경로는 windows-sys 0.61.2에서 실측 확인됨(2026-09-12): `GetDpiForWindow`·`SystemParametersInfoForDpi` 모두 `src/Windows/Win32/UI/HiDpi/mod.rs`(feature `Win32_UI_HiDpi`), `NONANTIALIASED_QUALITY`는 `Graphics/Gdi`에 `FONT_QUALITY = 3u8`로 정의돼 있어 `lfQuality`(u8)에 캐스트 없이 그대로 대입한다. `SystemParametersInfoForDpi`는 `BOOL`(i32)을 돌려주므로 `ok != 0`으로 판정한다.

**HFONT 수명**: `redraw`마다 `CreateFontIndirectW`를 부르면 GDI 핸들이 샌다 — `WM_SETTINGCHANGE`는 테마 외에도 자주 브로드캐스트되므로 누적이 빠르다. `StripState`에 `font: HFONT`와 `font_dpi: u32`를 두고, `redraw` 진입 시 `GetDpiForWindow(hwnd)`가 `font_dpi`와 다를 때만 이전 핸들을 `DeleteObject`하고 새로 만든다. `WM_DESTROY`에서도 `DeleteObject`한다.

- [ ] **Step 2: 텍스트 폭 측정 + 그리기로 `redraw` 교체**

`redraw`의 "확인용 반투명 채움" 블록을 아래로 바꾼다. 세그먼트를 `라벨 값 · 라벨 값` 순서로 이어 그리고, 각 조각의 색을 따로 준다.

```rust
/// 한 조각을 그리고 오른쪽 끝 x를 돌려준다. measure_only면 그리지 않고 폭만 잰다.
/// 호출 전에 반드시 글꼴이 dc에 SelectObject 되어 있어야 한다 — 안 그러면 GDI 기본
/// System 글꼴로 그려져 작업표시줄과 이질적으로 보인다.
unsafe fn draw_run(dc: HDC, text: &str, color: [u8; 3], x: i32, height: i32, measure_only: bool) -> i32 {
    let mut wide_text = wide(text);
    let mut rc = RECT { left: x, top: 0, right: x + 4096, bottom: height };
    let flags = DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | if measure_only { DT_CALCRECT } else { 0 };
    SetTextColor(dc, (color[0] as u32) | ((color[1] as u32) << 8) | ((color[2] as u32) << 16));
    SetBkMode(dc, TRANSPARENT as i32);
    DrawTextW(dc, wide_text.as_mut_ptr(), -1, &mut rc, flags);
    rc.right
}
```

`redraw`는 **2패스**로 동작한다.

① **측정 패스** — 1×1 DIB로 메모리 DC를 만들고 `SelectObject(dc, font)` 한 뒤, 모든 조각을 `measure_only = true`로 훑어 **전체 폭과 높이**를 얻는다. `DT_CALCRECT`가 돌려주는 `rc.bottom`이 글자 높이이므로, 여기에 상하 여백(각 4px × DPI/96)을 더해 **창 높이를 정한다**. `STRIP_HEIGHT`·`STRIP_WIDTH` 상수는 96-DPI 값이라 125%·150% 기기에서 어긋나므로 이 패스의 결과로 대체하고 상수는 삭제한다.

② **그리기 패스** — 측정된 크기로 DIB를 만들고 같은 글꼴을 `SelectObject` 한 뒤 `measure_only = false`로 실제로 그린다.

두 패스 모두 끝에 `SelectObject(dc, old_font)`로 원래 객체를 되돌린 뒤 `DeleteDC` 한다(글꼴 자체는 `StripState`가 소유하므로 여기서 `DeleteObject` 하지 않는다).

> **GDI 알파 처리**: `DrawTextW`는 알파 채널을 0으로 남긴다. `UpdateLayeredWindow`의 `AC_SRC_ALPHA`는 premultiplied BGRA를 요구하므로 그린 뒤 **버퍼를 훑어 알파를 복원**해야 글자가 보인다. Step 1에서 `NONANTIALIASED_QUALITY`를 켰기 때문에 글자 픽셀은 정확히 `SetTextColor` 값이고 나머지는 0이다 — 따라서 다음 복원이 **정확하다**(회색 경계가 없으므로 후광이 생기지 않는다):
>
> ```rust
> for px in buffer.chunks_exact_mut(4) {
>     if px[0] != 0 || px[1] != 0 || px[2] != 0 {
>         px[3] = 255; // 이미 불투명 순색이므로 premultiplied 값과 동일
>     }
> }
> ```
>
> 이 방식은 순수 검정 글자를 투명하게 만든다. 팔레트에 `#000000`이 없으므로(가장 어두운 값이 `#1F7F78`) 안전하다. **팔레트에 검정을 추가하려면 이 로직부터 바꿔야 한다.**

- [ ] **Step 3: 빌드·경고 0 확인**

Run: `cd src-tauri && cargo build --offline 2>&1 | tail -20`
Expected: `Finished`, 경고 0

- [ ] **Step 4: 기존 테스트 회귀 없음 확인**

Run: `cd src-tauri && cargo test --lib --offline 2>&1 | tail -3`
Expected: `57 passed; 0 failed`

- [ ] **Step 5: 육안 검증**

Run: `cargo build --release --offline && ./target/release/spectra-native.exe --probe-strip`

Expected — 사용자에게 스크린샷 요청. 확인 항목:
1. `Codex 55% · Claude 59%` 형태의 텍스트가 읽힌다(값은 임시 모델 값)
2. 글꼴이 작업표시줄 시계와 **같은 계열**로 보인다
3. 값 색이 팔레트대로다(라이트 테마에서 청록/황갈/적갈)
4. 텍스트가 잘리거나 넘치지 않고, 막대 폭이 텍스트에 맞다
5. 배경이 작업표시줄과 자연스럽게 섞인다(불투명 사각형이 보이지 않는다)

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src/taskbar_window.rs
git commit -m "feat(strip): draw the usage text with the shell status font

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: 배선 — 선호·커맨드·스냅샷 갱신·프론트 토글

**Files:**
- Modify: `src-tauri/src/standby.rs` — `UiPrefs.strip`, `BootState.strip`
- Modify: `src-tauri/src/lib.rs` — `AppState.strip`, `set_strip` 커맨드, 스냅샷 후 갱신
- Modify: `src-tauri/src/desktop_shell.rs` — `update_taskbar_strip`
- Modify: `src/integrations/boot-state.ts`, `src/integrations/tauri-native-bridge.ts`, `src/App.tsx`
- Modify: `tests/boot-state.test.ts`

**Interfaces:**
- Consumes: Task 2의 `taskbar_window::{spawn, current_theme, StripHandle}`, Task 1의 `build_model`
- Produces:
  ```rust
  // desktop_shell.rs
  pub(crate) fn update_taskbar_strip<R: Runtime>(app: &AppHandle<R>);
  // lib.rs 커맨드
  fn set_strip(enabled: bool, app: AppHandle, state: State<'_, AppState>) -> Result<standby::UiPrefs, String>;
  ```
  ```ts
  // boot-state.ts
  export type NativeUiPrefs = Readonly<{ theme: "dark" | "light"; solid: boolean; standby: boolean; strip: boolean }>;
  // tauri-native-bridge.ts
  export async function setNativeStrip(enabled: boolean): Promise<NativeUiPrefs | null>;
  ```

- [ ] **Step 1: 실패하는 테스트 작성 (Rust + 프론트)**

`src-tauri/src/standby.rs`의 테스트 모듈에 추가:

```rust
#[test]
fn strip_defaults_to_off_and_round_trips() {
    assert!(!UiPrefs::default().strip);
    let path = temp_prefs_path("strip");
    let prefs = UiPrefs { theme: "light".into(), solid: false, standby: false, strip: true };
    save_prefs_to(&path, &prefs).unwrap();
    assert_eq!(load_prefs_from(&path), prefs);
}

#[test]
fn prefs_written_before_the_strip_field_still_load() {
    let path = temp_prefs_path("strip-legacy");
    std::fs::write(&path, br#"{"theme":"light","solid":true,"standby":true}"#).unwrap();
    let loaded = load_prefs_from(&path);
    assert!(loaded.standby);
    assert!(!loaded.strip); // serde default
}
```

`tests/boot-state.test.ts`에 추가:

```ts
test("boot state defaults strip to false when the field is absent", () => {
  const parsed = parseBootState({ theme: "dark", solid: false, standby: false, mode: "mini", snapshots: [] });
  assert.equal(parsed?.strip, false);
});

test("boot state carries an explicit strip flag", () => {
  const parsed = parseBootState({ theme: "dark", solid: false, standby: false, strip: true, mode: "mini", snapshots: [] });
  assert.equal(parsed?.strip, true);
});
```

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cd src-tauri && cargo test --lib --offline standby 2>&1 | tail -5`
Expected: 컴파일 실패 — `struct UiPrefs has no field named strip`

Run: `npm test 2>&1 | tail -5`
Expected: `fail 2` — `parsed?.strip`이 `undefined`

- [ ] **Step 3: 구현**

`standby.rs` — `UiPrefs`에 필드 추가(`#[serde(default)]`가 구조체 수준에 이미 있으므로 누락된 필드는 `false`가 된다):

```rust
pub struct UiPrefs {
    pub theme: String,
    pub solid: bool,
    pub standby: bool,
    pub strip: bool,
}
```
`Default` 구현에 `strip: false` 추가. `BootState`에 `pub strip: bool` 추가하고 `AppState::boot_state`에서 `strip: prefs.strip` 채운다. 기존 `boot_script_embeds_json_and_mode` 테스트의 `BootState` 리터럴에도 `strip: false`를 넣는다.

`lib.rs`:

```rust
#[derive(Default)]
pub struct AppState {
    // ... 기존 필드 ...
    #[cfg(target_os = "windows")]
    pub(crate) strip: Mutex<Option<taskbar_window::StripHandle>>,
}

#[tauri::command]
fn set_strip(enabled: bool, app: AppHandle, state: State<'_, AppState>) -> Result<standby::UiPrefs, String> {
    let prefs = apply_prefs(&state, |prefs| prefs.strip = enabled)?;
    desktop_shell::update_taskbar_strip(&app);
    Ok(prefs)
}
```
`invoke_handler`의 `set_standby` 다음에 `set_strip`을 추가한다.

`provider_usage_snapshot`의 `desktop_shell::update_tray_badge(&app);` **바로 다음 줄**에 추가:

```rust
desktop_shell::update_taskbar_strip(&app);
```

`setup`에서 최초 창 생성 뒤에도 한 번 호출해 켜진 상태로 기동했을 때 스트립이 뜨게 한다.

`desktop_shell.rs`:

```rust
/// 스냅샷·선호가 바뀔 때만 불린다. 타이머 없음(스펙 §3-6).
#[cfg(target_os = "windows")]
pub(crate) fn update_taskbar_strip<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<crate::AppState>();
    let enabled = state.ui.lock().map(|prefs| prefs.strip).unwrap_or(false);
    let snapshots = state.last_snapshots.lock().map(|list| list.clone()).unwrap_or_default();
    let model = if enabled {
        crate::taskbar_strip::build_model(&snapshots, crate::taskbar_window::current_theme())
    } else {
        None
    };

    let Ok(mut slot) = state.strip.lock() else { return };
    match (&*slot, enabled) {
        (Some(handle), true) => handle.update(model),
        (Some(handle), false) => {
            handle.shutdown();
            *slot = None;
        }
        (None, true) => {
            let Some(model) = model else { return };
            let click_app = app.clone();
            match crate::taskbar_window::spawn(model, Box::new(move || {
                let _ = toggle_main_window(&click_app);
            })) {
                Ok(handle) => *slot = Some(handle),
                // 스트립은 부가 기능이므로 실패해도 앱을 막지 않는다.
                Err(error) => eprintln!("spectra: taskbar strip is unavailable: {error}"),
            }
        }
        (None, false) => {}
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn update_taskbar_strip<R: Runtime>(_app: &AppHandle<R>) {}
```

프론트 3파일:

```ts
// boot-state.ts — NativeUiPrefs에 strip 추가, parseBootState 안에서
const stripFlag = typeof strip === "boolean" ? strip : false;
// 반환 객체에 strip: stripFlag

// tauri-native-bridge.ts
export async function setNativeStrip(enabled: boolean): Promise<NativeUiPrefs | null> {
  const command = invoke();
  if (!command) return null;
  return command<NativeUiPrefs>("set_strip", { enabled });
}
```

`App.tsx` — `standby` 패턴을 그대로 따른다: `const [strip, setStrip] = useState(boot?.strip ?? false);`, `toggleStrip` 콜백(`toggleStandby` 복사 후 `setNativeStrip` 사용), `SharedViewProps`에 `strip`·`onStrip` 추가, 설정 패널의 "메모리 절약 대기" 행 **다음에** 행 추가:

```tsx
<div className="settings-row"><div><strong>작업표시줄 표시</strong><span>{strip ? "작업표시줄 알림 영역 왼쪽에 잔여량을 상시 표시합니다." : "작업표시줄에 표시하지 않습니다(기본)."}</span></div><button type="button" className="secondary-action" onClick={onStrip}>{strip ? "표시 끄기" : "표시 켜기"}</button></div>
```

- [ ] **Step 4: 테스트 통과 확인**

Run: `cd src-tauri && cargo test --lib --offline 2>&1 | tail -3`
Expected: `59 passed; 0 failed` (57 + 신규 2)

Run: `npm test 2>&1 | tail -5`
Expected: `pass 34` (32 + 신규 2)

- [ ] **Step 5: `#[allow(dead_code)]` 제거 + 전체 게이트**

Task 1·2에서 임시로 붙인 `#[allow(dead_code)]`를 모두 제거한다. 이제 실제로 배선됐으므로 경고가 나지 않아야 한다.

Run:
```bash
cd src-tauri && cargo build --offline 2>&1 | grep -c warning && cargo build --offline --features native-oauth 2>&1 | grep -c warning && cargo test --lib --offline --features native-oauth 2>&1 | tail -3
```
Expected: `0`, `0`, `59 + native-oauth 전용 테스트 수`. native-oauth 전용 분은 Task 1 착수 시 `cargo test --lib --features native-oauth`로 기준선을 한 번 재어 기록해 둔다

Run: `npm run build && npm run verify:tokens && npm run verify:memory && npm run verify:baseline`
Expected: 모두 통과

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src src/integrations src/App.tsx tests/boot-state.test.ts
git commit -m "feat(strip): wire the taskbar strip to snapshots, preferences and the settings toggle

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: 셸 변화 대응 + 임시 진입점 제거

explorer 재시작·테마 전환·해상도 변경·자동숨김·전체화면에서 스트립이 올바르게 행동하게 한다. 여기까지 끝나야 "상시 표시"가 참이 된다.

**Files:**
- Modify: `src-tauri/src/taskbar_window.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/lib.rs` — Task 2의 `--probe-strip` 임시 진입점과 헬퍼 제거

**Interfaces:**
- Consumes: Task 2·3의 `wnd_proc`·`redraw`
- Produces: 없음(내부 동작만 보강)

- [ ] **Step 1: `TaskbarCreated` 등록 + 메시지 처리 추가**

창 생성 직후 등록한다:

```rust
use windows_sys::Win32::UI::WindowsAndMessaging::RegisterWindowMessageW;
use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN, QUNS_PRESENTATION_MODE, ABM_GETSTATE, ABS_AUTOHIDE};

// create_window 안, SetWindowLongPtrW 다음
let taskbar_created = unsafe { RegisterWindowMessageW(wide("TaskbarCreated").as_ptr()) };
// StripState에 taskbar_created: u32 필드를 추가해 보관
```

`wnd_proc`에 분기를 더한다:

```rust
WM_SETTINGCHANGE | WM_DISPLAYCHANGE | WM_DPICHANGED => {
    // 테마·글꼴·작업표시줄 위치가 모두 여기서 바뀔 수 있다. 값은 캐시된 모델을 그대로 쓴다.
    redraw(hwnd);
    0
}
```

그리고 `_ =>` 분기 앞에 등록 메시지 비교를 넣는다(상수가 아니라 런타임 값이라 match arm으로 쓸 수 없다):

```rust
_ => {
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const StripState;
    if let Some(state) = ptr.as_ref() {
        if msg == state.taskbar_created {
            // explorer가 재시작됐다. 작업표시줄 HWND가 새로 생겼으므로 위치를 다시 계산한다.
            redraw(hwnd);
            return 0;
        }
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}
```

- [ ] **Step 2: 자동숨김·전체화면에서 숨기기**

`redraw`의 위치 계산 직후, `UpdateLayeredWindow` 호출 **전에** 넣는다:

```rust
/// 작업표시줄이 자동숨김이거나 전체화면 앱이 떠 있으면 스트립도 비켜 준다.
fn should_hide() -> bool {
    let mut data: APPBARDATA = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
    let state = unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) };
    if state & ABS_AUTOHIDE as usize != 0 {
        return true;
    }
    let mut notification = 0;
    if unsafe { SHQueryUserNotificationState(&mut notification) } == 0 {
        return matches!(notification, QUNS_RUNNING_D3D_FULL_SCREEN | QUNS_PRESENTATION_MODE);
    }
    false
}
```

> 자동숨김은 v1에서 "숨긴다"로 처리한다. 작업표시줄이 올라올 때 따라 올라오게 하려면 `ABN_FULLSCREENAPP`·`ABN_POSCHANGED` 콜백 등록이 필요한데, 이는 스펙 §3-2의 경로 ②와 함께 재검토한다.

- [ ] **Step 3: 임시 진입점 제거**

Task 2 Step 7에서 넣은 `--probe-strip` 분기와 `lib.rs`의 `probe_*` 헬퍼 2개를 삭제한다. 이제 정식 설정 토글로 확인할 수 있다.

Run: `grep -rn "probe_strip" src-tauri/src/`
Expected: 결과 없음

- [ ] **Step 4: 빌드·테스트 전체 게이트**

Run:
```bash
cd src-tauri && cargo build --offline 2>&1 | grep -c warning && cargo test --lib --offline 2>&1 | tail -3
```
Expected: `0`, `59 passed; 0 failed`

- [ ] **Step 5: 육안 검증 — 릴리스 빌드로 5가지 시나리오**

Run: `npm run desktop:build` 후 설치 없이 `src-tauri/target/release/spectra-native.exe` 실행

사용자에게 다음을 요청한다(각 항목 스크린샷 또는 확인):
1. **기본값**: `ui-prefs.json`을 지우고 기동 → 스트립이 보이지 않는다
2. **켜기**: 설정 → "작업표시줄 표시" 켜기 → 즉시 스트립이 나타나고 값이 실제 스냅샷과 일치한다
2-1. **켠 채로 재기동**: 기동 직후에는 `last_snapshots`가 비어 `build_model`이 `None`을 돌려주므로 스트립이 **첫 새로고침이 끝난 뒤에** 나타난다. 이는 설계대로이며 실패가 아니다 — 창이 뜨자마자 보이지 않는다고 보고하지 않는다
3. **테마**: Windows 설정에서 라이트↔다크 전환 → 스트립 색이 따라간다
4. **explorer 재시작**: 작업 관리자에서 Windows 탐색기 다시 시작 → 10초 안에 스트립이 같은 자리로 돌아온다
5. **끄기**: 토글을 끄면 스트립이 사라지고, 앱을 재기동해도 꺼진 상태가 유지된다

추가로 **가려짐 관찰**: 30분 사용 중 스트립이 다른 창에 가려지는 일이 2회 이상 재현되면 스펙 §3-2의 경로 ②로 전환 판단이 필요하다 — 발생 여부를 기록한다.

- [ ] **Step 6: 커밋**

```bash
git add src-tauri/src
git commit -m "feat(strip): follow explorer restarts, theme changes and fullscreen state

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: 메모리 측정과 문서 확정

**Files:**
- Modify: `docs/performance/memory-budget.md`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-09-12-taskbar-usage-strip.md` — 상태 줄

**Interfaces:**
- Consumes: Task 5의 릴리스 빌드
- Produces: 없음(기록만)

- [ ] **Step 1: 대기 상태 30분 측정**

스트립을 **켠 채** 창을 닫고(대기 모드도 켬) 60초 안정화 후 측정한다. 실행자가 직접 30분을 기다리지 말고 사용자에게 명령을 제시하고 결과를 받는다.

```powershell
npm run measure:memory -- -Scenario standby-strip -Samples 61 -IntervalSeconds 30
```

Expected: 평균 작업 집합 ≤ 35 MB, `processCount` 전 표본 1

- [ ] **Step 2: 실행 파일 크기 기록**

Run: `ls -l src-tauri/target/release/spectra-native.exe && sha256sum src-tauri/target/release/spectra-native.exe`
Expected: Phase 4 기준 5,836,800 B 대비 증가분을 기록. `windows-sys`는 기존재 크레이트이므로 신규 feature 분량만 늘어난다.

- [ ] **Step 3: 문서 갱신**

`docs/performance/memory-budget.md`에 절 추가:

```markdown
## 2026-09-12 작업표시줄 스트립 (B안)

변경: `windows-sys` 직접 의존 추가(트리 기존재, `Cargo.lock` 불변), 네이티브 레이어드 창 1개 + 전용 스레드 1개. 두 번째 WebView 창을 쓰지 않아 대기 모드의 프로세스 1개 구성을 유지한다.

| 시나리오 | 표본 | 평균 작업 집합 | 프로세스 수 | 목표 |
|---|---|---|---|---|
| standby-strip (30분, 스트립 켬) | 61 | (측정값) MB | 1 | ≤ 35 MB |

갱신 트리거: `provider_usage_snapshot` 완료 · `WM_SETTINGCHANGE` · `WM_DISPLAYCHANGE` · `TaskbarCreated`. 데이터 목적 타이머 없음.
```

`README.md`의 대기 모드 설명 문단 끝에 한 문장 추가:

```markdown
대시보드 설정의 "작업표시줄 표시"를 켜면 작업표시줄 알림 영역 왼쪽에 `Codex NN% · Claude NN%`를 창 없이 상시 표시합니다(기본값 꺼짐, 주 작업표시줄·가로 배치만 지원).
```

스펙 상태 줄에 완료 기록과 §1 성공 기준 표의 항목별 판정(PASS/FAIL)을 적는다. 미달 항목은 미달로 남긴다.

- [ ] **Step 4: 커밋**

```bash
git add docs/ README.md
git commit -m "docs(strip): record the standby measurement and the strip settings entry

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

## Self-Review

**스펙 coverage**

| 스펙 항목 | 담당 Task |
|---|---|
| §1 표시·위치 기준 | Task 2(위치)·3(텍스트)·5(육안 검증) |
| §1 메모리 ≤35 MB·프로세스 1 | Task 2(네이티브 선택)·6(측정) |
| §1 폴링 없음 | Task 4(스냅샷 지점 배선)·5(셸 이벤트) |
| §1 테마 추종 | Task 4(`current_theme` 사용)·5(`WM_SETTINGCHANGE`) |
| §1 기본값 꺼짐 | Task 4(`UiPrefs.strip` 기본 false) |
| §1 explorer 재시작 복원 | Task 5(`TaskbarCreated`) |
| §1 `Cargo.lock` 불변 | Task 2 Step 1·3 |
| §1 회귀(두 feature·경고 0) | Task 4 Step 5 |
| §3-1 네이티브 레이어드 창 | Task 2 |
| §3-2 z-order ①→② | Task 2 Step 7(3번 항목)·Task 5 Step 5(가려짐 관찰) |
| §3-3 공급자별 최솟값·`—` | Task 1 |
| §3-4 라벨·색·셸 글꼴 | Task 1(색)·Task 3(글꼴) |
| §3-5 테마 출처 분리 | Task 2(`current_theme`)·Task 4(연결 금지) |
| §3-6 갱신 시점 | Task 4·5 |
| §3-7 기본값·범위(주 작업표시줄·가로) | Task 1(`place_strip` None)·Task 4 |
| §4 의존성·feature 목록 | Task 2 |
| §6 문서 갱신 | Task 6 |

**타입 일관성 확인**: `StripModel`·`StripSegment`·`Palette`·`Rect`·`TaskbarEdge`는 Task 1에서 정의되고 Task 2·3·4가 같은 이름으로 소비한다. `update_taskbar_strip`은 Task 4에서 정의되고 Task 4 안에서만 호출된다. `spawn`/`update`/`shutdown`/`current_theme`는 Task 2에서 정의되고 Task 4가 호출한다. `StripHandle`은 `AppState`에 `Mutex<Option<StripHandle>>`로 보관되므로 `Send`가 필요하고, Task 2에서 `unsafe impl Send`로 보장한다.

**남은 불확실성(실행 중 확인 필요)**

1. **z-order**: Task 2 Step 7의 3번 항목이 실패하면 경로 ②(`SetParent`)로 전환해야 하고, Task 5의 자동숨김 처리도 함께 바뀐다. 이 경우 계획을 갱신한 뒤 진행한다.
2. **`TrayNotifyWnd` 정확도**: 실측상 rect는 정상이지만, Win11 XAML 작업표시줄에서 **그려지는 위치와 1:1로 맞는지**는 Task 2 Step 7 스크린샷으로만 확정된다. 어긋나면 오프셋 보정값을 설정으로 빼는 것이 아니라 `place_strip`에 보정을 넣고 테스트를 추가한다.
3. **나머지 windows-sys 심볼 경로**: `GetDpiForWindow`·`SystemParametersInfoForDpi`·`NONANTIALIASED_QUALITY`는 2026-09-12 실측 확인됐다(Task 3 Step 1 주석). 그 외 심볼(`SHQueryUserNotificationState`·`RegisterWindowMessageW` 등)의 모듈 경로는 컴파일러가 잡아 주므로 빌드 단계에서 맞춘다.

**검토에서 해소된 항목(2026-09-12)** — 아래 4건은 초안에 있던 결함이며 Task 2·3 본문에 반영됐다. 실행자는 이미 반영된 내용을 다시 "발견"할 필요가 없다:

| 결함 | 증상 | 반영 위치 |
|---|---|---|
| 글꼴을 DC에 `SelectObject` 하지 않음 | GDI 기본 System 글꼴로 그려져 Task 3 검증 2번 실패 | Task 3 Step 2 `draw_run` 주석 + 2패스 설명 |
| `HFONT` 매 `redraw` 생성 | GDI 핸들 누수(`WM_SETTINGCHANGE`가 잦아 누적 빠름) | Task 3 Step 1 "HFONT 수명" |
| 안티앨리어싱 경계가 알파 복원으로 불투명화 | 라이트 작업표시줄에서 글자마다 검은 후광, Task 3 검증 5번 실패 | Task 3 Step 1 `NONANTIALIASED_QUALITY` |
| `shell_font(dpi)`의 `dpi` 출처 없음 + 96-DPI 상수 | 125% 기기에서 글자·창 크기 축소 | Task 3 Step 1 `GetDpiForWindow`/`SystemParametersInfoForDpi`, Step 2 크기 측정, Task 1 150% 배치 테스트 |
| `probe_spawn_strip`이 비공개 타입 반환 | E0446로 첫 육안 게이트 차단 | Task 2 Step 7 `probe_strip_for_20s() -> ()` |
