# SPECTRA 작업표시줄 사용량 스트립 (B안) 설계

> 작성: 2026-09-12 · 기준 커밋 `e623254` · 상태: Task 1~6 구현·검증 완료(미커밋; Task 6 대기 스트립 평균 33.6 MB, 프로세스 1개 PASS)
> 목표: 창을 열지 않고도 Codex·Claude 잔여량을 **작업표시줄에서 상시** 확인한다. Phase 4에서 확보한 대기 상태 31.5 MB를 훼손하지 않는다.

## 0. 왜 트레이 아이콘으로는 안 되는가

현재 트레이 배지(`tray_badge.rs`)는 32×32 **정사각형** 이미지 한 장이다. 셸 알림 영역 아이콘은 정사각 슬롯이므로 `Codex 55% · Claude 59%` 같은 가로 스트립을 표현할 수 없다. 두 공급자를 동시에 보이려면 아이콘을 2개로 늘려야 하고(A안), 그러면 숫자가 16 px 슬롯 안에 갇혀 판독성이 떨어진다.

→ 별도의 **가로 표시 표면**이 필요하다.

## 1. 성공 기준 (체크 가능 형식)

| 축 | 기준 | 측정 방법 |
|---|---|---|
| 표시 | 스트립 켬 상태에서 작업표시줄 알림 영역 왼쪽에 `[Codex CI] NN% · [Claude CI] NN%`가 창 없이 보인다 | 스크린샷 또는 사용자 육안 확인 |
| 위치 | 스트립 오른쪽 끝이 `TrayNotifyWnd.left`에 접하고, 셸 요소와 겹치는 픽셀이 0이다 | 스크린샷 + `SPECTRA_TIMING_LOG=1` 좌표 로그 |
| 메모리 | 대기(standby) 상태 총 작업 집합 ≤ **35 MB** (Phase 4 기준 31.5 MB + 스트립) | `scripts/measure-memory.ps1 -Scenario standby-strip` |
| 메모리 | 대기 상태 프로세스 수가 **1**로 유지된다(WebView2 0개) | 동일 스크립트 `processCount` 열 |
| 폴링 | 스트립 갱신이 `provider_usage_snapshot` 완료 시점에만 일어난다. 데이터 목적 타이머 0개 | `grep -n "SetTimer" src-tauri/src/` 결과가 위치 재확인 용도 외 0건 + 코드 리뷰 |
| 테마 | 작업표시줄 라이트↔다크 전환 시 10초 안에 스트립 색이 따라간다 | 수동 전환 + 스크린샷 2장 |
| 기본값 | `UiPrefs.strip == false`(기본)에서 스트립 창이 생성되지 않고 현행 동작과 동일하다 | `ui-prefs.json` 없는 상태로 기동 + 창 목록 확인 |
| 복원 | explorer.exe 재시작 후 10초 안에 스트립이 같은 위치로 돌아온다 | 수동 재시작 + 스크린샷 |
| 의존성 | `Cargo.lock` 변경이 `spectra-native` 의존 목록의 `"windows-sys 0.61.2",` 엣지 한 줄뿐이다 — 새 `[[package]]` 0개, 버전 변경 0개 | `git diff -- Cargo.lock` 육안 + `grep -c '^+\[\[package\]\]'` = 0 |
| 회귀 | 기본·`native-oauth` 두 구성 모두 경고 0, 기존 테스트 전수 통과 | `cargo build --release` · `cargo test --lib` |

### 1-1. 최종 판정 (2026-09-12)

표시·위치·테마·기본값·Explorer 재시작·전체화면/자동숨김 전환은 Task 2~5 자동 검사와 사용자 육안 확인으로 PASS했다. 대기 상태 메모리는 Task 6에서 61개 표본 평균 33.6 MB, 최대 34.1 MB, 프로세스 수 1개로 PASS했다. 데이터 목적 폴링은 없고 `Cargo.lock`의 새 package는 없다. 별도 D3D 전체화면, 물리 다중 모니터, 30분 가려짐 관찰은 이 판정에 포함하지 않았다.

## 2. 실측 근거 (2026-09-12, windows-main)

DPI-unaware PowerShell 프로브(`FindWindowExW` + `SHAppBarMessage`) 결과:

```
Shell_TrayWnd     L=0    T=816  R=1536  W=1536 H=48   (논리)
  TrayNotifyWnd   L=1140 T=816  R=1536 W=396  H=48   (논리)  ← 트레이~시계 묶음
  ReBarWindow32   L=198  T=816  R=906   W=708  H=48   (논리)
ABM_GETTASKBARPOS edge=BOTTOM  L=0 T=1020 R=1920 B=1080  H=60  (물리)
ABM_GETSTATE      = 0  (autohide=false, alwaysontop=false)
SystemUsesLightTheme = 1 · AppsUseLightTheme = 1
Shell_SecondaryTrayWnd 없음(단일 작업표시줄)
```

판정 3가지:

1. **Windows 11 26200에서도 `TrayNotifyWnd`가 살아 있고 rect가 정상이다.** 스트립을 `TrayNotifyWnd.left - 스트립폭`에 놓으면 목업 B의 `^` 왼쪽 자리가 그대로 나온다.
2. **좌표계가 섞여 있다.** `SHAppBarMessage`는 물리(1920×1080), DPI-unaware 프로세스의 `GetWindowRect`는 논리(1536×864)를 돌려줬다 — 배율 125%. 우리 코드는 DPI-aware이므로 `GetWindowRect`도 물리를 받는다. **두 API를 같은 좌표계로 쓰되, 논리 좌표를 가정한 상수를 두지 않는다.**
3. **여백이 충분하다.** `ReBarWindow32`가 논리 906에서 끝나고 `TrayNotifyWnd`가 1140에서 시작하므로, 폭 ~160 논리 px 스트립을 오른쪽 정렬해도(980~1140) 셸 요소와 겹치지 않는다.

> 프로브 스크립트의 `FindWindowExW(parent, NULL, NULL, NULL)` 전수 열거 구간은 PowerShell의 `$null` 마셜링 문제로 빈 결과를 냈다. 클래스명 지정 조회는 정상 동작했고 위 수치는 그쪽에서 얻은 것이다. Rust 구현에서는 이 문제가 없다.

## 3. 설계 결정

### 3-1. 표시 표면: 네이티브 Win32 레이어드 창 (두 번째 WebView 창 아님)

스트립은 앱이 떠 있는 동안 **항상** 보여야 한다. 두 번째 Tauri `WebviewWindow`를 쓰면 대기 모드가 `main`을 파기해도 WebView2 브라우저·렌더러·GPU 프로세스가 살아남아, Phase 4가 확보한 31.5 MB·프로세스 1개가 상시 세 자릿수로 되돌아간다. 한 단계를 통째로 들여 얻은 수치이므로 교환하지 않는다.

→ `WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST` 창 1개를 전용 스레드에서 만들고 `UpdateLayeredWindow`로 RGBA 버퍼를 올린다. 추가 비용은 스레드 1개 + 수백 KB 버퍼.

### 3-2. z-order 경로: ① 독립 topmost → 실패 시 ② `SetParent`

작업표시줄도 topmost라 두 topmost 창이 경합한다. ①을 기본으로 하고, 실전에서 가려짐이 재현되면 ②(`SetParent`로 `Shell_TrayWnd`에 삽입)로 내린다.

| | ① 독립 topmost (채택) | ② `SetParent` 삽입 (예비) |
|---|---|---|
| 실패 양상 | 위치가 어긋나거나 일시적으로 가려짐 — 되돌리기 쉬움 | 재부착 실패 시 스트립 소실 |
| 자동숨김·전체화면 | 직접 처리 필요(`ABM_GETSTATE`·`SHQueryUserNotificationState`) | 셸이 처리 |
| explorer 재시작 | 위치만 다시 계산 | 재부착 필수 |

판정 기준: ①로 구현 후 30분 사용 중 **가려짐이 2회 이상 재현되면** ②로 전환한다.

### 3-3. 공급자별 표시 값: 그 공급자 창들의 **최솟값**

`tray_badge::min_remaining_percent`는 `rolling` 우선 단일 값이라 스트립의 2칸 표시에 그대로 못 쓴다. 스트립은 공급자마다 **자기 한도 창 중 최소 잔여율**(= 실제로 먼저 막히는 창)을 보여준다. 임의 규칙이 없고 "얼마 남았나"라는 질문에 곧장 답한다.

| 상태 | 표시 |
|---|---|
| `connected`·`stale` + 창 1개 이상 | 최소 잔여율 `NN%` |
| 그 외(로그아웃·미설치·창 없음) | `—` (README의 미연결 표기 규약) |
| 두 공급자 모두 미연결 | 스트립 창을 숨김(빈 막대를 띄우지 않음) |

툴팁에는 창별 상세(5시간·7일·주간 + 초기화 시각)를 넣는다.

### 3-4. 라벨·색

- 2026-09-12 사용자 승인으로 전체 이름을 **16 DIP CI 로고**로 대체한다. 표시 순서는 Codex → Claude이며 잔여율·구분점은 유지한다. `StripModel`의 공급자 이름과 툴팁은 전체 이름을 유지한다.
- 위치는 기존 `TrayNotifyWnd.left`, 즉 `^` 왼쪽을 유지한다. 와이파이 아이콘 앞으로 이동하지 않는다.
- CI 원본·출처·재생성 절차는 `src-tauri/assets/provider-ci/README.md`를 따른다. 64×64 알파 마스크 2개를 내장하며, 런타임 의존성·네트워크 조회를 추가하지 않는다.
- CI는 GDI 텍스트의 알파 복원 **이후** premultiplied BGRA로 합성해 검정 로고와 반투명 가장자리를 보존한다. 텍스트의 순색 알파 복원을 CI에 다시 적용하지 않는다.
- 글꼴은 `SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS)`의 `lfStatusFont`를 사용한다. Segoe UI 하드코딩은 한국어 폴백을 깨뜨린다.
- 값 색은 기존 임계(≥50 / ≥20 / 그 외)를 따르되, **작업표시줄 배경에 맞는 두 벌**을 둔다. `tray_badge`의 색은 어두운 배지 위 기준이라 라이트 작업표시줄에서 대비가 부족하다.

| 역할 | 라이트 | 다크 |
|---|---|---|
| 값 ≥50% | `#1F7F78` | `#56B7B0` |
| 값 ≥20% | `#9A6B00` | `#C7A852` |
| 값 <20% | `#B4472B` | `#D88463` |
| Codex CI | `#000000` | `#FFFFFF` |
| Claude CI | `#D97757` | `#D97757` |
| 미연결 값 `—`·알 수 없는 공급자 이름 | `#5F5F5F` | `#A0A0A0` |
| 구분점 `·` | `#9A9A9A` | `#6B6B6B` |

### 3-5. 테마 출처

스트립은 **작업표시줄 테마**를 따른다 — `HKCU\...\Themes\Personalize\SystemUsesLightTheme`. `UiPrefs.theme`(기본 `dark`)는 SPECTRA 자기 창의 설정이므로 **연결하지 않는다**. 두 값을 묶으면 조용한 오색 버그가 된다.

### 3-6. 갱신 시점 (폴링 금지 유지)

| 트리거 | 하는 일 |
|---|---|
| `provider_usage_snapshot` 완료(기존 `update_tray_badge` 지점) | 값 갱신 + 다시 그림 |
| `WM_SETTINGCHANGE` | 테마·글꼴·작업표시줄 위치 재확인 |
| `WM_DISPLAYCHANGE`·`WM_DPICHANGED` | 위치·배율 재계산 |
| `TaskbarCreated` 등록 메시지 | explorer 재시작 후 복귀 |

데이터를 얻기 위한 타이머는 두지 않는다(`docs/performance/memory-budget.md`의 "앱 유휴 상태에서는 provider 폴링을 하지 않습니다" 불변식 유지). 위치 재확인용 저빈도 타이머는 위 메시지로 해결되지 않는 경우에 한해, 사유를 문서에 적고 추가한다.

### 3-7. 기본값과 범위

- `UiPrefs.strip` 신설, **기본 `false`**. Phase 4의 "기본값은 현행 동작" 선례를 따른다.
- v1은 **주 작업표시줄만**. `Shell_SecondaryTrayWnd`(보조 모니터)는 범위 밖 — 실측상 이 기기에 없다.
- 세로 작업표시줄(edge LEFT/RIGHT)은 범위 밖. `ABM_GETTASKBARPOS`의 edge가 `BOTTOM`/`TOP`이 아니면 스트립을 띄우지 않는다.
- 스트립 클릭 = 트레이 왼쪽 클릭과 동일(창 토글).

## 4. 의존성

`windows-sys` 0.61.2를 직접 의존으로 추가한다. 실측(`cargo tree -e normal -i`) 결과 이미 트리에 있다:

```
windows-sys v0.61.2 ← dirs-sys ← dirs ← tauri ← spectra-native
windows     v0.61.3 ← tao ← tauri-runtime-wry ← tauri
```

해석된 버전에 고정하므로 **새 크레이트 0개**이다. `Cargo.lock`에는 `spectra-native` 자신의 의존 목록에 `"windows-sys 0.61.2",` 엣지 한 줄이 추가될 뿐이며(직접 의존 추가는 크레이트가 이미 트리에 있어도 이 한 줄을 반드시 남긴다 — 2026-09-12 실측 정정), 새 `[[package]]` 블록과 버전 변경은 없다(features는 lock에 기록되지 않는다). 필요한 feature: `Win32_Foundation`, `Win32_UI_WindowsAndMessaging`, `Win32_UI_Shell`, `Win32_UI_HiDpi`, `Win32_Graphics_Gdi`, `Win32_System_LibraryLoader`, `Win32_System_Registry`.

Windows 전용이므로 `[target.'cfg(target_os = "windows")'.dependencies]`에 넣는다.

## 5. 범위 밖

- macOS 메뉴 막대·Linux 패널 대응
- 보조 모니터 작업표시줄, 세로 작업표시줄
- 스트립에서의 상세 조작(새로고침 버튼·드래그 이동) — v1은 표시와 클릭 토글만
- 사용량 추정·과거 추이

## 6. 문서 갱신 대상

- `docs/performance/memory-budget.md` — 대기 상태 재측정 결과 절 추가
- `README.md` — 스트립 설정 한 줄
- 이 파일 상태 줄 — 완료 시 측정값과 판정 기록
