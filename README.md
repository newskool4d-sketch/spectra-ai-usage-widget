# SPECTRA — AI 사용량 위젯

> A+C 방향을 제품 화면으로 옮긴 React/TypeScript 코드 퍼스트 프로젝트입니다. 이전 세 시안과 `app.js`는 시각 회귀 기준선으로만 보존합니다.

여러 AI 서비스의 사용량을 맥·윈도우·iOS에서 가장 빠르게 읽을 수 있는 구조를 찾습니다.

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

기준선 파일과 Figma 비의존성은 다음 명령으로 확인합니다.

```powershell
npm run verify:baseline
```

공용 디자인 토큰은 `styles/tokens.css`에 있으며, 토큰 구조는 다음 명령으로 확인합니다.

```powershell
npm run verify:tokens
```

브라우저에서 `http://127.0.0.1:4173`을 엽니다. 화면 폭에 따라 데스크톱 A 또는 모바일 C가 자동으로 선택됩니다.

기존 비교 기준선이 필요하면 `node server.mjs`로 별도 정적 서버를 실행한 뒤 다음 URL을 사용합니다.

- `?variant=A` — 한눈에 보기: 데스크톱용 요약 대시보드
- `?variant=B` — 서비스 비교: 서비스를 원형으로 놓고 한도를 비교하는 화면
- `?variant=C` — 모바일 알림: iOS 중심 알림 목록과 기기별 위젯

제품 화면의 수치는 예시 snapshot입니다. 서비스 선택, 단위·기간 선택, 새로고침, 유리·불투명 모드와 OAuth 연결 데모 흐름은 실제로 작동합니다. `OAuth로 연결`을 완료해도 공급자 계정에 접속하거나 실제 잔여량을 가져오지는 않으며, 연결 후 상태도 `예시 snapshot`으로 표시됩니다. 공식 OAuth와 요금제 잔여량 어댑터, 플랫폼 패키징은 다음 단계입니다.

## 요금제 잔여량과 OAuth UX

`src/data/providers.ts`의 `PlanQuota` 모델은 서비스마다 다른 요금제 한도를 다음 공통 필드로 표현합니다.

- `planName`, `accountLabel`: 요금제와 계정 표시명
- `windows`: 롤링·일일·주간·월간 등 한도 창, 사용률, 잔여률, 초기화 시각
- `connectionState`, `source`, `confidence`, `lastSyncedAt`: 연결 상태와 데이터 신뢰도
- `authMethod`: 현재 제품 UX가 가정하는 `oauth-pkce` 인증 방식

데스크톱 A에는 잔여량 요약·서비스별 잔여량·연결 카드가, 모바일 C에는 잔여량 히어로·OAuth 연결 카드·한도 알림이 들어갑니다. 공급자별 공식 잔여량 경로가 확인되기 전까지는 계정 토큰이나 비공개 세션을 읽지 않습니다.

공급자별 인증과 쿼터 범위는 [`docs/integrations/provider-capability-matrix.md`](./docs/integrations/provider-capability-matrix.md)에 기록합니다. `src/integrations/oauth-adapter.ts`와 `src/integrations/credential-vault.ts`는 Tauri·Swift 네이티브 구현을 위한 계약이며, 브라우저 제품 화면에서는 네트워크 호출과 토큰 교환을 하지 않습니다.

네이티브 경계 스캐폴드는 [`docs/integrations/native-oauth.md`](./docs/integrations/native-oauth.md)에 있습니다. 데스크톱은 `src-tauri/`의 `spectra://oauth/callback`과 OS Credential Manager/Keychain 경계를 사용하고, iOS/macOS 공유 Swift 코드는 `native/apple/SpectraAuth/`에 둡니다. 현재는 callback 검증과 보관 경계만 준비했으며 공급자별 authorize URL, client ID, token 교환, 요금제 잔여량 API 연결은 다음 승인 단계입니다.

## 디자인 방향

- 제품명: **SPECTRA**
- 글꼴: `Pretendard Variable` → `Pretendard` → `Noto Sans KR` → `Apple SD Gothic Neo` → `Malgun Gothic` 순서
- 재질: 내비게이션과 핵심 컨트롤에만 유리 효과를 쓰고, 내용 영역은 불투명도를 높여 읽기 편하게 구성
- 색상: 청록·파랑·보라·분홍·초록·겨자·테라코타를 낮은 채도로 사용
- 표현: 영문 대문자 라벨, 네온 후광, 과한 그라데이션을 줄이고 실제 운영 도구처럼 짧고 담백한 한국어를 사용
- 플랫폼: Apple의 반투명 재질과 Windows의 Mica/Acrylic 특성을 같은 디자인 토큰으로 조정

## 이번 시안에서 다루지 않는 것

- 실제 서비스 로그인과 잔여량 API 연결(현재는 OAuth 데모 UX만 포함)
- 데이터 저장, 앱 패키징, 분석 도구, 운영용 아키텍처
- 모든 AI 서비스가 공식 한도 API를 제공한다는 전제

선택된 A+C 화면만 제품 런타임에 포함하고, B와 시안 전환 UI는 기준선으로 별도 보관합니다.
