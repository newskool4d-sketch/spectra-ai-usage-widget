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

1. `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`의 버전을 같은 SemVer로 올립니다.
2. 해당 커밋에 `v<version>` 태그를 푸시합니다.
3. `.github/workflows/publish-tauri.yml`이 NSIS 설치 파일, updater 서명 파일, `latest.json`을 GitHub Release에 업로드합니다.
4. 설치된 앱은 시작 후 백그라운드에서 확인하고, 설정 화면에서 수동 확인·설치를 할 수 있습니다.

기존 `v0.2.1` 설치본에는 updater 코드가 없으므로, 자동 업데이트를 시작하는 첫 공개 버전은
updater가 포함된 다음 버전(예: `v0.2.2`)이어야 합니다.
