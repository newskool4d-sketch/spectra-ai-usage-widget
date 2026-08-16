# 실행 메모리 예산

SPECTRA는 트레이에 오래 머무는 앱이므로 사용량 연결 뒤에도 상주 작업을 최소화합니다.

- 뷰포트에 맞는 데스크톱 A 또는 모바일 C 화면 하나만 React 트리에 마운트합니다.
- 목록·라벨·브라우저 데모 데이터는 모듈 스코프 불변 객체로 두고 작은 컴포넌트는 `memo`로 재렌더링을 줄입니다.
- 차트 라이브러리, `requestAnimationFrame`, `localStorage`, `sessionStorage`, 상시 `setInterval`을 사용하지 않습니다.
- 네이티브 앱은 과거 사용량 시계열을 저장하지 않고 현재 한도 스냅샷만 보관합니다.
- Codex App Server는 새로고침 요청 때만 실행하며 응답 또는 12초 제한 뒤 자식 프로세스를 종료합니다.
- Claude 조회는 짧은 `claude auth status` 프로세스와 작은 정제 JSON 캐시만 사용합니다.
- Codex JSONL은 총 2MiB, JSON 파일은 4MiB, Claude 상태선 입력은 1MiB로 제한합니다.
- 기존 Claude 상태선 명령은 2초, 출력은 64KiB로 제한합니다.
- 로그인 직후 확인은 대화상자가 열린 동안 단일 재귀 `setTimeout`으로만 수행하며 성공·닫기·30회 제한에서 종료합니다.
- 앱 유휴 상태에서는 provider 폴링을 하지 않습니다. 시작 시 한 번, 사용자가 새로고침할 때 한 번 조회합니다.
- `matchMedia` 리스너는 한 개만 등록하고 effect cleanup에서 해제합니다.
- `prefers-reduced-motion`을 존중합니다.

검증:

```powershell
npm run build
npm run verify:memory
```

이 검사는 번들 크기와 메모리 경로의 구조적 예산을 확인합니다. 실제 RAM 사용량은 Windows 작업 관리자에서 30분 이상 idle·수동 refresh·로그인 시나리오를 별도로 측정해야 합니다.

## Windows Tauri 기준선

2026-08-16의 0.1 릴리스 스모크 테스트에서 SPECTRA와 WebView2 자식 프로세스 7개의 합계는 다음과 같았습니다. 공급자 연결을 추가한 0.2 패키지는 기능·트레이 QA를 통과했지만 30분 메모리 재측정은 남아 있습니다.

- 최적화된 미니 창 표시 직후: 작업 집합 441.1MB, private 217.8MB
- 닫기 후 트레이 숨김 3초: 작업 집합 437.5MB, private 203.6MB
- 네이티브 호스트 자체: 작업 집합 32.4MB, private 6.3MB

대부분은 WebView2의 브라우저·렌더러·GPU 프로세스입니다. 현재는 빠른 재표시를 위해 닫기 시 창을 숨기므로 WebView2가 유지됩니다. 릴리스 프로필에는 size 최적화, thin LTO, 단일 codegen unit, 심볼 제거, abort panic을 적용합니다.

더 줄여야 한다면 닫을 때 WebView 창을 파기하고 트레이 클릭 시 재생성하는 선택형 저메모리 대기 모드를 별도 기능으로 검토합니다. 재표시 지연과 화면 상태 초기화가 대가이며, WebView2 보안 격리를 약화하는 비공식 single-process 플래그는 사용하지 않습니다.
