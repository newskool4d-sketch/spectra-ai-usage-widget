# SPECTRA v0.2.0 — Windows unsigned prerelease

## 포함 내용

- Windows Tauri 미니 창·트레이·단일 실행 복원
- ChatGPT 개인 요금제 Codex App Server 잔여량
- Claude Max 개인 요금제 status line 브리지
- 5시간·7일 한도 모델과 연결 상태 UX

## 설치 파일

- 파일: `SPECTRA_0.2.0_x64-setup.exe`
- SHA-256: `2D3AD0AD78823222CC3C1B59D9E68611FBC4776DE90EFF10120BF4C991475809`
- Authenticode: `NotSigned`

## Windows 주의

이 prerelease의 설치 파일은 공개용 Authenticode 인증서로 서명되지 않았습니다. Windows SmartScreen에서 게시자를 확인할 수 없다는 경고가 표시될 수 있습니다. 설치 전 위 SHA-256을 확인하고, 출처가 이 Release인지 확인하세요.

공개용 코드서명 인증서를 확보하면 서명된 설치 파일로 교체할 예정입니다.

## 검증

- React/Vite production build: PASS
- Rust unit tests: PASS — 14 passed
- Rust Clippy: PASS
- Windows 설치·트레이·Codex 실데이터·Claude 5시간/7일 실데이터: PASS
- 상세 기록: `docs/release-0.2.0-qa.md`
