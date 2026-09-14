# SPECTRA 자동 업데이트

SPECTRA는 Tauri updater를 사용해 GitHub Release의 `latest.json`을 확인하고,
새 버전이 있으면 사용자의 승인 후 NSIS 업데이트 파일을 내려받아 재시작합니다.

## 저장소 설정

저장소에는 updater 공개키만 `src-tauri/tauri.conf.json`에 포함합니다.
개인키는 절대 커밋하지 않고 GitHub 저장소의 Actions secret에 등록합니다.

- `TAURI_SIGNING_PRIVATE_KEY`: `%USERPROFILE%\.tauri\spectra-updater.key`의 전체 내용
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 키에 비밀번호를 설정한 경우에만 등록

현재 생성된 개인키는 로컬 `%USERPROFILE%\.tauri\spectra-updater.key`에 보관되어
있습니다. GitHub Actions에 등록하기 전까지 공개 Release를 만들면 안 됩니다.

## Release 흐름

1. `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`과 두 잠금 파일의 앱 버전을 같은 SemVer로 올립니다.
2. `docs/releases/v<version>.md`에 변경사항과 설치 주의사항을 기록합니다. `npm run verify:release`가 버전·잠금 파일·릴리스 노트를 확인합니다.
3. 검증한 변경을 커밋하고 해당 커밋에 `v<version>` 태그를 푸시합니다.
4. `.github/workflows/publish-tauri.yml`이 태그와 버전 일치, 서명 키 존재, 프런트엔드·네이티브 테스트를 확인한 뒤 NSIS 설치 파일, updater 서명 파일, `latest.json`을 **초안** Release에 업로드합니다.
5. 업로드한 설치 파일·서명·manifest의 일치 여부를 확인하고 `SHA256SUMS.txt`를 추가한 후 공개합니다. 이 검사는 앱 updater의 실제 암호학적 서명 검증을 대신하지 않습니다. 실패한 초안은 자동으로 최신 공개 릴리스가 되지 않습니다.
6. 설치된 앱은 **미니 창/대시보드 생성 후 2.5초 뒤** 업데이트를 확인합니다. 새 버전 안내의 `지금 설치`, 대시보드 설정의 수동 확인·설치를 사용할 수 있습니다. 창이 없는 상태의 정기 확인이나 Windows 알림은 아직 지원하지 않습니다.

기존 **공개 `v0.2.1`** 설치본에는 updater 코드가 없으므로 `v0.2.2`를 한 번 수동 설치해야 합니다.
updater를 넣어 별도로 재빌드·설치한 로컬 `v0.2.1`은 이번 `v0.2.2`로 앱 내 업데이트할 수 있습니다.
그 이후에는 현재 버전보다 높은 SemVer만 업데이트 대상으로 안내합니다.

`.exe.sig`는 Tauri updater 검증용이며 Windows Authenticode 서명이 아닙니다.
현재 Windows 게시자 인증서는 없으므로 SmartScreen 경고는 별개입니다.
