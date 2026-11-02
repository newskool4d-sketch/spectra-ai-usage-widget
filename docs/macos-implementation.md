# macOS 포팅 작업 및 검증

기준: v0.3.1 (`2588389`), 2026-11-03. Mac 시험판 소스: `803ba25a70ff0cae0f422d69b4de01e9331000de`. 상태: 양쪽 아키텍처의 설치 패키지 생성·검증 완료, 실기기 GUI 미검증.

## 범위와 완료 기준

- 기존 Focus Dock 대시보드와 미니창, 다크·라이트 테마 및 저장 설정을 공통 코드로 유지한다.
- macOS 상단 메뉴 막대에서 Codex·Claude·Antigravity 잔여량을 확인하고 미니창·대시보드를 연다. 데이터 없음·오래된 값은 현재 수치처럼 표시하지 않는다.
- 창을 닫거나 메모리 절약 대기를 사용해도 메뉴 막대와 공급자 갱신은 유지한다.
- Finder 실행 시에도 일반적인 CLI 설치 경로를 탐색한다. 계정 비밀이나 대화 원문은 새로 수집하지 않는다.
- Apple Silicon과 Intel용 macOS 앱·DMG 빌드 경로를 준비하고 가능한 환경에서 빌드·테스트한다.
- Windows 동작을 회귀 검증한다. Mac 시험판 자동 업데이트는 별도 배포 피드가 준비될 때까지 명시적으로 비활성화한다.

## 가정 및 배포 범위

- Apple Silicon·Intel 모두 대상이며 최소 macOS 13.0으로 준비한다.
- 현재 작업 호스트는 Windows이다. CI의 macOS 빌드·실행과 사용자의 Mac 화면 검증을 구분한다.
- Apple Developer ID 인증서·공증은 제공되지 않았다. ad-hoc 서명 패키지는 공증된 정식 배포본으로 표시하지 않는다.
- 기존 Windows 공개 릴리즈는 유지하며 이번 패키지를 시험판으로 검증한다.

## 검증 기록

- 프론트엔드 테스트: 92/92 통과. 플랫폼·테마 복원과 Mac에서 Windows 업데이트 피드를 조회하지 않는 경로 포함.
- TypeScript 및 Vite 빌드: 통과. 디자인 기준·132개 디자인 토큰 검사 통과.
- 번들 크기: JS 239,973 bytes / CSS 23,636 bytes. 240,000-byte JS 상한 유지. 사용하지 않는 아이콘 정의 4개를 제거하여 Mac 분기 추가분을 상쇄했다.
- 공급자 테스트: Windows에서 provider_usage 31/31, Antigravity 12/12 통과. 초기 subprocess 시간 제한 실패 2건은 직렬 재실행 통과. Antigravity Windows wrapper는 샌드박스 PowerShell 초기화 경고가 stderr에 섞여 실패했으며 동일 합성 입력 테스트를 일반 실행 환경에서 통과했다.
- Windows 네이티브 전체 테스트: 134 통과 / 0 실패 / 기존 GDI 통합 테스트 9개 미실행. 직렬 실행으로 subprocess 검사를 격리했으며 macOS 메뉴 모델·좌표 테스트 6개를 포함한다.
- 독립검토: 프론트엔드·패키징 P1/P2 없음. 네이티브 RV-001(Retina 좌표 단위 혼용), RV-002(비동기 창 크기 변경 직후 이전 크기로 앵커 계산)를 수용하여 논리좌표·목표 창 크기 기반으로 수정하고 재검토했다. 남은 P1/P2 없음. 혼합 DPI 모니터가 모호하면 중앙 표시로 복원한다.
- [Mac CI 실행 41531866262](https://github.com/newskool4d-sketch/spectra-ai-usage-widget/actions/runs/41531866262): 최종 성공. Intel·Apple Silicon에서 네이티브 테스트, 앱·DMG 빌드, 아키텍처·번들 메타데이터·ad-hoc 서명·DMG 무결성, 패키징된 CLI 실행 검사 통과. 네이티브 테스트는 각 129/129 통과했다. 프론트엔드 테스트 92개 및 크기 검사도 각 환경에서 통과했다.
- 실행 환경: Intel macOS 15.7.9, Apple Silicon macOS 15.8.2. macOS 13 최소 버전 실기기 검증은 하지 않았다.
- 다운로드 후 DMG와 app.zip 각각의 SHA-256을 CI 산출값과 재대조하여 4개 모두 일치했다.

## 설치 패키지

로컬 위치: 작업트리의 `src-tauri/target/macos-artifacts/arm64/` 및 `x86_64/`. 바이너리는 Git에 넣지 않는다.

| DMG | SHA-256 |
|---|---|
| `SPECTRA_0.3.1_arm64.dmg` | `602cdf384122c69f16263b231e0d72eae6dc7bb9ac7bcf1e34f0e784a2d7c9cc` |
| `SPECTRA_0.3.1_x86_64.dmg` | `153dda6fedd180d096094957a8bfb8f74138fed86118bf9f5a466cada6406e12` |

## 남은 검증

- 실제 Mac 로그인 계정으로 공급자 연결·메뉴 막대 표시·클릭·창 위치·다크/라이트 설정 유지 확인.
- Apple Developer ID 서명·공증, Mac 자동 업데이트 피드. 현재 ad-hoc 서명 시험판이며 Windows 정식 릴리즈는 변경하지 않았다.
- 실제 Mac 대화형 검증을 CI의 컴파일·단위 테스트·CLI 실행 성공으로 대체하지 않는다. 종합 상태는 시험판 구현 및 패키지 검증 완료 / GUI 검증 PARTIAL이다.
