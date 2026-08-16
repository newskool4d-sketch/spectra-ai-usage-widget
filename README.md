# SPECTRA — AI 사용량 위젯

> A+C 방향을 제품 화면으로 옮긴 React/TypeScript + Tauri 코드 퍼스트 프로젝트입니다. 이전 세 시안과 `app.js`는 시각 회귀 기준선으로만 보존합니다.

Claude와 OpenAI Codex의 **개인 요금제 잔여량**을 Windows 트레이에서 빠르게 확인합니다. macOS와 iOS는 후속 플랫폼입니다.

## 현재 구현

- Windows 미니 창·대시보드·트레이 숨김/복원·단일 실행
- Codex 공식 App Server의 ChatGPT 계정 및 rate limit 조회
- Claude 공식 로그인 상태와 사용자가 동의해 설치하는 status line 한도 브리지
- 브라우저에서는 디자인 확인용 예시 수치, 네이티브 앱에서는 실제 값이 없을 때 `—` 표시
- provider 토큰·이메일·세션 원문을 React 또는 SPECTRA 사용량 캐시에 복제하지 않음

## 확정된 방향

데스크톱 A와 모바일 C를 합친 구성을 제품 기준으로 확정했습니다. Figma가 아니라 [코드 퍼스트 디자인 기준선](./docs/design-baseline/README.md)과 검증된 스크린샷이 디자인 원본입니다. B와 화면 아래 시안 전환 막대는 비교 실험으로만 보존합니다.

## 실행

```powershell
npm run dev
```

제품 빌드와 메모리 경로 검증:

```powershell
npm run build
npm run verify:memory
```

Windows Tauri 실구동과 NSIS 패키징:

```powershell
npm run desktop:dev
npm run desktop:build
```

## Windows v0.2.1 공개 릴리스 주의

현재 Windows 설치 파일은 `Authenticode: NotSigned` 상태입니다. 따라서 Windows SmartScreen에서 게시자를 확인할 수 없다는 경고가 표시될 수 있습니다.

- 공개 릴리스: [SPECTRA v0.2.1](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/releases/tag/v0.2.1)
- 설치 파일 SHA-256: `2D3AD0AD78823222CC3C1B59D9E68611FBC4776DE90EFF10120BF4C991475809`
- 설치 전 릴리스 페이지의 SHA-256과 로컬 파일을 대조하세요.
- 공개용 Authenticode 인증서를 확보하면 서명된 설치 파일로 교체할 예정입니다.

데스크톱 앱은 430×720 미니 창으로 시작합니다. 닫기 버튼은 앱을 종료하지 않고 트레이로 숨기며, 트레이 왼쪽 클릭 또는 두 번째 앱 실행으로 미니 창을 다시 엽니다. 트레이 메뉴에서는 미니 창, 1280×860 대시보드, 숨기기, 종료를 선택할 수 있습니다. 패키징 결과는 `src-tauri/target/release/bundle/nsis/`에 생성됩니다.

개인 정보가 제거된 네이티브 연결 진단:

```powershell
src-tauri\target\release\spectra-native.exe --provider-snapshot codex
src-tauri\target\release\spectra-native.exe --provider-snapshot claude
```

기준선 파일과 Figma 비의존성은 다음 명령으로 확인합니다.

```powershell
npm run verify:baseline
```

공용 디자인 토큰은 `styles/tokens.css`에 있으며, 토큰 구조는 다음 명령으로 확인합니다.

```powershell
npm run verify:tokens
```

브라우저 개발 서버는 `http://127.0.0.1:5173`을 사용합니다. 화면 폭에 따라 데스크톱 A 또는 모바일 C가 자동으로 선택됩니다.

기존 비교 기준선이 필요하면 `node server.mjs`로 별도 정적 서버를 실행한 뒤 다음 URL을 사용합니다.

- `?variant=A` — 한눈에 보기: 데스크톱용 요약 대시보드
- `?variant=B` — 서비스 비교: 서비스를 원형으로 놓고 한도를 비교하는 화면
- `?variant=C` — 모바일 알림: iOS 중심 알림 목록과 기기별 위젯

브라우저 제품 화면은 예시 snapshot을 사용합니다. 설치된 Tauri 앱은 공식 CLI에서 실제 한도 창을 받았을 때만 숫자를 표시합니다. 서비스 선택, 단위·기간 선택, 새로고침, 유리·불투명 모드가 실제로 작동합니다.

## 요금제 잔여량과 계정 연결

`src/data/providers.ts`의 `PlanQuota` 모델은 서비스마다 다른 요금제 한도를 다음 공통 필드로 표현합니다.

- `planName`, `accountLabel`: 요금제와 계정 표시명
- `windows`: 롤링·일일·주간·월간 등 한도 창, 사용률, 잔여률, 초기화 시각
- `connectionState`, `source`, `confidence`, `lastSyncedAt`: 연결 상태와 데이터 신뢰도
- `authMethod`: 공급자 공식 도구에 위임된 인증 경계

데스크톱 A에는 잔여량 요약·서비스별 잔여량·연결 카드가, 모바일 C에는 잔여량 히어로·계정 연결 카드·한도 알림이 들어갑니다.

Codex는 로그아웃 상태에서 `codex login`을 시작하고, 로그인 뒤 `codex app-server --stdio`의 `account/read`와 `account/rateLimits/read`를 호출합니다. App Server는 요청 때만 실행하며 응답 또는 제한 시간 뒤 종료합니다. API key 로그인은 ChatGPT 개인 요금제 한도로 취급하지 않습니다.

Claude는 `claude auth status`로 로그인 상태를 확인합니다. 사용자가 `사용량 브리지 설치`를 선택하면 공식 status line 입력의 5시간·7일 `rate_limits` 필드만 정제해 저장합니다. 첫 Claude 응답 전에는 `첫 사용량 대기`로 표시될 수 있으며, 브리지 제거 시 기존 상태선 설정을 복원합니다.

공급자별 공식 인터페이스와 호환성 범위는 [`docs/integrations/provider-capability-matrix.md`](./docs/integrations/provider-capability-matrix.md), 데이터·자격 증명 경계는 [`docs/integrations/native-oauth.md`](./docs/integrations/native-oauth.md)에 기록합니다. 현재 활성 흐름은 별도 client ID나 SPECTRA 소유 token vault를 요구하지 않습니다. 공식 CLI가 자격 증명을 계속 관리합니다.

## 메모리 정책

앱은 과거 시계열을 저장하거나 유휴 상태에서 계속 폴링하지 않습니다. 시작·수동 새로고침 때 현재 스냅샷만 읽고 Codex App Server는 즉시 종료합니다. 자세한 기준은 [`docs/performance/memory-budget.md`](./docs/performance/memory-budget.md)에 있습니다.

## 디자인 방향

- 제품명: **SPECTRA**
- 글꼴: `Pretendard Variable` → `Pretendard` → `Noto Sans KR` → `Apple SD Gothic Neo` → `Malgun Gothic` 순서
- 재질: 내비게이션과 핵심 컨트롤에만 유리 효과를 쓰고, 내용 영역은 불투명도를 높여 읽기 편하게 구성
- 색상: 청록·파랑·보라·분홍·초록·겨자·테라코타를 낮은 채도로 사용
- 표현: 영문 대문자 라벨, 네온 후광, 과한 그라데이션을 줄이고 실제 운영 도구처럼 짧고 담백한 한국어를 사용
- 플랫폼: Apple의 반투명 재질과 Windows의 Mica/Acrylic 특성을 같은 디자인 토큰으로 조정

## 아직 다루지 않는 것

- OpenAI Codex·Claude의 비공개 웹 세션·스크래핑·잔여량 추정
- 조직 API 사용량과 ChatGPT/Claude 개인 구독 잔여량의 동일시
- 과거 사용량 분석·코드 서명·자동 업데이트
- 모든 AI 서비스가 공식 한도 API를 제공한다는 전제
- macOS 메뉴 막대 앱과 iOS SwiftUI/WidgetKit 실구현

선택된 A+C 화면과 Claude/Codex provider만 제품 런타임에 포함하고, B와 시안 전환 UI는 기준선으로 별도 보관합니다.
