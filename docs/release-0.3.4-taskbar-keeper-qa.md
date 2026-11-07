# v0.3.4 작업표시줄 유지 모듈 ZIP 배포 검증

## 배포 범위

2026-10-29 보정(실행기 종료 기록·5분 자동 재시작, 실행 파일 ProductVersion 검사)이 담긴 `scripts/taskbar-keeper/`를 공개 ZIP으로 묶어 기존 v0.3.4 릴리스에 자산으로 추가합니다. v0.3.4 태그·설치 파일·서명·업데이트 manifest·`SHA256SUMS.txt`는 바꾸지 않으며 앱 소스도 수정·재빌드하지 않습니다.

- 사용자 결정(2026-11-07): v0.2.10 대신 v0.3.4 keeper ZIP 추가. v0.2.10 준비 지시는 이미 v0.3.0~v0.3.4로 병합·배포된 상태여서 진행하지 않음.
- 기존 공개 ZIP `SPECTRA_0.2.8_taskbar-keeper.zip`(SHA-256 `1cccca12…`, 소스 커밋 `5557be3`)은 보정 이전 스크립트이며 v0.2.8 릴리스에 그대로 둡니다.

## 산출물

ZIP은 커밋된 소스(`git show <커밋>:scripts/taskbar-keeper/…`)에서 직접 만들어 작업 폴더의 줄바꿈 변환이 섞이지 않게 했습니다. 구성은 v0.2.8 ZIP과 같습니다(스크립트 3개, `README.md`, `SOURCE_COMMIT.txt`, `SHA256SUMS.txt`).

| 파일 | 크기 | SHA-256 |
|---|---|---|
| `SPECTRA_0.3.4_taskbar-keeper.zip` | 7,716 B | `eea995b7c932402b71d716bcfa1b1a97d935fc55bf3d6587d20945162803c905` |
| └ `install-taskbar-keeper.ps1` | 4,112 B | `f27c971bc458b99edcc89895fc712595cf9ff4e55b788e112e9cff50fa077521` |
| └ `keep-spectra-taskbar.ps1` | 1,918 B | `03cc3025f1294c7f240845382abef56bc5ee9a632e4509164c2106c7f442a3bb` |
| └ `start-spectra-taskbar-keeper.ps1` | 4,904 B | `58f880dd58dafbbdfa30f38b20301f1f76fa83aa37e1e66b30c0c4854934c373` |
| └ `README.md` | 4,342 B | `5b8358b685080da0bf3e4a169acffd548fd421f2165fa4a04a5470010ad50ec9` |

- 소스 커밋(`SOURCE_COMMIT.txt`): `a0bba1cd23b43e4ecce2631ff7393323c54479d1`.
- `SPECTRA_0.3.4_taskbar-keeper.zip.sha256`은 `<해시>  <파일명>` 한 줄로 v0.2.8 형식과 같습니다.
- ZIP은 릴리스의 `SHA256SUMS.txt`(설치 파일·서명·manifest 3개만 담음)에 넣지 않고 자체 `.sha256`으로 검증합니다. v0.2.8 선례와 같습니다. `scripts/verify-release.mjs`는 자산을 정확한 이름으로 찾으므로 추가 자산이 있어도 검증에 영향이 없습니다.
- `SOURCE_COMMIT.txt`의 커밋이 `main`에 그대로 남도록 이 브랜치는 merge commit으로 병합해야 합니다. squash·rebase로 병합하면 병합 뒤 `main` 커밋으로 ZIP을 다시 만들고 이 표의 해시를 갱신합니다.
- 유지 모듈 `keep-spectra-taskbar.ps1` 해시는 v0.2.8 공개 사본과 같고, 설치 스크립트의 `$expectedKeeperHash`와 일치합니다.

## 확인한 결과

| 검사 | 상태 | 범위 |
|---|---|---|
| ZIP 무결성 | PASS | `zipfile.testzip()` 오류 없음, 압축 해제 뒤 내부 `SHA256SUMS.txt` 5개 항목 모두 OK |
| 소스 일치 | PASS | 해제한 스크립트 3개가 작업 폴더 파일과 바이트 단위로 같음, 모두 LF |
| 구문 검증 | PASS | PowerShell 7.6.9 파서로 스크립트 3개 오류 0건 |
| 개인정보 | PASS | 해제한 파일에서 사용자 이름·개인 경로·계정 문자열 미검출 |
| 지원 버전 | PASS(소스) | 설치 스크립트 지원 목록 `0.2.8`~`0.3.4`, README·릴리스 노트 문구 일치 |
| `npm test` | PASS | 90 passed, 0 failed (워크트리 `npm ci` 뒤. 설치 전에는 `@tauri-apps/api` 누락으로 2개 파일 실패) |
| `cargo test --lib --locked` | PASS | 130 passed, 0 failed, 9 ignored (2회 연속). 병렬 빌드 중 첫 실행은 시간 제한 검사 2개가 5.2초로 초과해 실패. Rust 코드 변경 없음 |
| `npm run verify:release` | PASS | `Release preflight PASS: v0.3.4` |
| `git diff --check` | PASS | 공백 오류 없음 |

## 문서 변경

- `scripts/taskbar-keeper/README.md`: 제목·지원 버전을 v0.2.8~v0.3.4로, 다운로드 파일명을 v0.3.4 ZIP으로 갱신하고 v0.3.4 공개 CI 실행 파일 참고 해시(`56BD2ADB…`, v0.3.4 검증 기록의 설치본)를 추가했습니다.
- `docs/releases/v0.2.9.md`: "v0.2.9 설치 파일에는 적용되지 않습니다" 문장을 보정 완료·v0.3.4 ZIP 배포 안내로 바꾸고, 기존 설치 PC는 작업·폴더 삭제 후 재설치해야 보정이 적용된다는 안내를 유지했습니다.
- `docs/releases/v0.3.4.md`: 선택 설치형 유지 모듈 절을 추가했습니다. 공개 릴리스 본문 반영은 아래 미수행 항목입니다.

## 미수행·미검증

- 릴리스 자산 업로드, 공개 릴리스 본문 갱신, push는 사용자 승인 전이라 수행하지 않았습니다. 승인 시 순서는 브랜치 push → `main` merge commit 병합(노트의 `blob/main` 검증 기록 링크가 이때부터 유효) → v0.3.4에 ZIP·`.sha256` 2개 업로드 → v0.3.4·v0.2.9 공개 본문을 각 노트 파일로 갱신 → `gh release view`로 재확인입니다.
- 현재 PC에 설치된 앱·유지 모듈·예약 작업은 교체하거나 다시 시작하지 않았습니다. 따라서 이 ZIP의 실제 설치, 재부팅, 다른 PC의 새 설치는 검증하지 않았습니다. 보정 동작 자체는 2026-10-29 격리 폴더 검증(`docs/release-0.2.8-taskbar-keeper-qa.md`)을 근거로 합니다.
- v0.3.0~v0.3.3 공개 CI 실행 파일의 ProductVersion은 설치 파일을 풀어 확인하지 않았습니다.
