# 실행 메모리 예산

3단계 제품 화면은 실제 서비스 API를 붙이기 전에도 메모리 사용 경로가 불어나지 않도록 다음 제약을 코드로 고정합니다.

- 뷰포트에 맞는 A 또는 C 화면 하나만 React 트리에 마운트합니다. 숨겨진 두 번째 화면을 렌더링하지 않습니다.
- 서비스 목록·차트 막대·라벨은 모듈 스코프의 불변 배열로 한 번만 만들고, 작은 잎 컴포넌트는 `memo`로 재렌더링을 줄입니다.
- 차트 라이브러리, 폴링 `setInterval`, `requestAnimationFrame`, `localStorage`/`sessionStorage` 캐시를 사용하지 않습니다.
- OAuth UX는 연결 상태만 React 메모리에 유지하는 데모입니다. 실제 토큰을 붙일 때는 브라우저 저장소가 아니라 운영체제 자격 증명 저장소(Windows Credential Manager·macOS Keychain 등) 경계를 별도로 둡니다.
- capability matrix와 demo adapter는 모듈 스코프 불변 객체이며, OAuth callback·토큰 갱신·백그라운드 폴링을 제품 화면 번들에 넣지 않습니다.
- 새로고침은 한 번의 짧은 타이머만 사용하며, 새 화면으로 이동하거나 언마운트될 때 `clearTimeout`으로 정리합니다.
- `matchMedia` 리스너는 한 개만 등록하고 effect cleanup에서 해제합니다.
- `prefers-reduced-motion`을 존중해 저사양 환경의 애니메이션 비용을 줄입니다.

검증:

```powershell
npm run build
npm run verify:memory
```

이 검사는 번들 크기와 메모리 경로의 구조적 예산을 확인합니다. 실제 RAM 사용량은 연결할 API, Tauri/WebView 런타임, 네이티브 iOS 위젯에 따라 달라지므로, 실제 데이터 연결 이후에는 Windows 작업 관리자·macOS Activity Monitor에서 30분 이상 idle/refresh 시나리오를 별도로 측정해야 합니다.

## Windows Tauri 실측 기준선

2026-08-16 Windows 릴리스 스모크 테스트에서 SPECTRA와 자식 WebView2 프로세스 7개의 합계는 다음과 같았습니다. 이 값은 한 기기의 짧은 기준선이며 일반적인 보장값이 아닙니다.

- 최적화된 미니 창 표시 직후: 작업 집합 441.1MB, private 217.8MB
- 닫기 후 트레이 숨김 3초: 작업 집합 437.5MB, private 203.6MB
- 네이티브 호스트 자체: 작업 집합 32.4MB, private 6.3MB

대부분은 WebView2의 브라우저·렌더러·GPU 다중 프로세스가 차지합니다. 현재는 빠른 재표시를 위해 닫기 시 창을 숨기므로 WebView2가 유지됩니다. 릴리스 프로필에는 `opt-level="s"`, thin LTO, 단일 codegen unit, 심볼 제거, abort panic을 적용해 네이티브 바이너리와 매핑 비용을 줄입니다.

추가 절감이 필요하면 다음 단계에서 선택형 **저메모리 대기 모드**를 구현합니다. 이 모드는 닫을 때 WebView 창을 파기하고 트레이 클릭 시 새로 생성하므로 재표시 지연과 화면 상태 초기화를 감수해야 합니다. WebView2의 보안·프로세스 격리를 약화하는 비공식 single-process 플래그는 사용하지 않습니다.
