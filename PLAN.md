# SPECTRA 제품 계획

## 구현 진행

- [x] 1단계: A+C 기준선, 기준 스크린샷, Figma 비의존성 검사
- [x] 2단계: 공용 디자인 토큰 분리 (`styles/tokens.css`)
- [x] 3단계: A+C 제품 화면을 React/TypeScript 구조로 재작성
- [x] 3단계 확장: `PlanQuota` 잔여량 모델과 데스크톱·모바일 계정 연결 UX 반영
- [x] 3단계 확장 2: 공급자 capability matrix, 네이티브 연결 계약, OS 자격 증명 저장 경계 추가
- [x] 4단계 기반: Tauri OAuth callback·OS Credential Vault 경계와 Apple Swift 인증 모듈 스캐폴드
- [x] Windows 제품 셸: Tauri WebView2 실구동, 트레이 상주, 미니/대시보드 전환, 단일 실행 복원, NSIS 패키징
- [x] 4단계 Windows 데이터 연결: Codex App Server와 Claude Code status line을 통한 개인 요금제 잔여량
- [x] 4단계 계정 UX: 공식 CLI 로그인 시작, Codex 로그인 확인, Claude opt-in 브리지 설치·복원
- [x] 4단계 Windows 패키지 QA: 0.2 NSIS·로컬 업그레이드·트레이·Codex 실제 한도 검증
- [ ] 4단계 Claude 수용 QA: opt-in 브리지 설치·첫 실제 5시간/7일 값·제거 복원 검증
- [ ] 5단계: macOS 메뉴 막대 앱과 iOS SwiftUI/WidgetKit 동기화

## 제품이 바로 답해야 할 것

화면을 열면 다음 세 가지가 바로 보여야 합니다.

1. 오늘 사용할 수 있는 양이 얼마나 남았는가?
2. 어느 서비스가 한도나 초기화 시점에 가장 가까운가?
3. 다음 작업은 어느 서비스에서 하는 편이 좋은가?

## 3단계 제품 화면

- Vite가 `src/main.tsx`에서 React 앱을 시작합니다.
- 데스크톱에서는 A 대시보드, 820px 이하에서는 C 모바일 스트림만 마운트합니다.
- 기존 `app.js`와 B 시안은 비교·회귀 기준선으로 보존하고 제품 런타임에서는 로드하지 않습니다.
- 메모리 예산과 정적 구조 검증은 [`docs/performance/memory-budget.md`](./docs/performance/memory-budget.md)에 기록합니다.
- 잔여량 요약은 `planName`, `windows`, `remainingPercent`, `resetLabel`, `connectionState`, `source`, `confidence`로 표현합니다.
- 공식 범위 카드는 로그인·권한·보관 경계를 설명하고, 실제 한도 창이 없으면 네이티브 앱에서 예시 숫자를 숨깁니다.
- 공급자별 인증 방식·쿼터 범위·개인 요금제 잔여량 검증 여부는 [`docs/integrations/provider-capability-matrix.md`](./docs/integrations/provider-capability-matrix.md)와 `src/integrations/provider-capabilities.ts`에서 분리 관리합니다.

## 프로토타입 단계

- 구조가 뚜렷하게 다른 세 시안을 비교합니다.
- 데스크톱과 아이폰 크기에서 글자, 정보 밀도, 가로 넘침을 확인합니다.
- API 연결 전에 예시 데이터로 화면의 우선순위를 먼저 정합니다.
- 앱 껍데기나 로그인을 붙이기 전에 한 시안을 고릅니다.

## 시안 선택 뒤 제품 구성

### 데스크톱

- React/TypeScript 화면을 Tauri 앱 안에서 공유합니다.
- macOS는 메뉴 막대 팝오버, Windows는 트레이와 작은 고정 창을 지원합니다.
- Windows는 430×720 미니 창으로 시작하고, 트레이 메뉴에서 1280×860 대시보드로 전환합니다. 닫기 버튼은 프로세스를 종료하지 않고 트레이로 숨깁니다.
- 서비스 연결은 기기 안에서 처리하고, 비밀값은 Codex CLI·Claude Code의 공식 자격 증명 저장 경계에 둡니다.

### iOS

- SwiftUI 앱과 WidgetKit 확장을 함께 만듭니다.
- 위젯은 계속 실행되는 화면이 아니라 일정 시점의 요약 정보를 표시합니다.
- 데스크톱 또는 소형 집계 서버와의 암호화 동기화는 선택 사항으로 둡니다.

### 데이터 구조

- 서비스별 요금제를 `PlanQuota`로 맞추고, `windows` 배열 안에 한도 종류·사용률·잔여률·초기화 시각을 둡니다.
- `source`와 `confidence`를 이용해 Codex App Server·Claude status line 확인값, 브라우저 예시 snapshot, 실제 데이터 대기 상태를 구분합니다.
- 서비스가 추정치만 제공하거나 공식 잔여량 경로가 없으면 정확한 수치처럼 보이지 않게 합니다.
- Windows의 실제 연결은 provider 공식 CLI에 위임하고, web prototype에는 `demo-only` 데이터만 둡니다.
- 네이티브 callback·OS vault 코드는 향후 별도 OAuth provider와 Apple 플랫폼을 위한 경계로 보존하며 현재 provider token은 복제하지 않습니다.

## 품질 기준

- 투명도를 꺼도 글자와 정보 위계가 유지됩니다.
- 상태를 색만으로 구분하지 않습니다.
- 데스크톱 미니 위젯에서 다음 행동을 2초 안에 파악할 수 있습니다.
- iOS 위젯 갱신이 늦어도 마지막 확인 시각과 데이터 상태를 알 수 있습니다.
- 자격 증명과 원본 세션 데이터는 기본적으로 기기 밖으로 나가지 않습니다.

## 한국어 디자인 원칙

- 기본 글꼴은 Pretendard이며, 맥과 Windows의 한글 시스템 글꼴로 자연스럽게 대체됩니다.
- 짧은 명사형 라벨과 평서문을 쓰고, 영문 슬로건과 번역투를 피합니다.
- 본문은 따뜻한 먹색 계열, 상태색은 낮은 채도의 7색으로 구성합니다.
- 유리 효과는 화면 전체가 아니라 시안 전환, 내비게이션, 위젯 표면에만 사용합니다.
