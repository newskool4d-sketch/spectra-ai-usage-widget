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
