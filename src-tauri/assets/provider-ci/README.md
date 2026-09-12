# 작업표시줄 공급자 CI

2026-09-12 사용자 승인: 공급자 전체 이름 대신 CI와 잔여율을 표시한다.
Codex → Claude 순서와 `^` 왼쪽 배치를 유지한다. 이 자산은 서비스 식별용이며
SPECTRA 자체 아이콘을 대체하지 않는다.

SVG 출처는 [Lobe Icons](https://github.com/lobehub/lobe-icons)이며 원본 파일을 보존했다.
배포 코드의 MIT 고지는 `LICENSE.lobe-icons.txt`에 포함한다. Codex와 Claude의
상표·로고 권리는 각각 OpenAI와 Anthropic에 있다.

| 로컬 파일 | 원본 | 원본 SHA-256 |
|---|---|---|
| `codex.svg` | [Codex](https://raw.githubusercontent.com/lobehub/lobe-icons/master/packages/static-svg/icons/codex.svg) | `d08b4e824cd6727e89617f59e4737273ff53e44c1da849d016dd64dfd08b33e1` |
| `claude.svg` | [Claude color](https://raw.githubusercontent.com/lobehub/lobe-icons/master/packages/static-svg/icons/claude-color.svg) | `a3101f3047a119aa11825ad9369510f0c472428c8c52d420e31bc62db44a8364` |

`*.alpha`는 64×64, 행 우선, 픽셀당 알파 1바이트인 렌더링 자산이다(각 4,096바이트).
SVG 윤곽과 내부 투명 영역을 보존한다. 앱은 이를 `include_bytes!`로 내장하고
16 DIP 크기로 보간한 뒤 premultiplied BGRA로 합성한다. Codex는 라이트에서 검정,
다크에서 흰색이며, Claude는 원본의 `#D97757`을 사용한다. 잔여율 색상은 별도다.

재생성은 저장소 루트에서 다음 명령을 사용한다. 검증 환경은 PyMuPDF 1.27.2.3이다.

```powershell
python scripts/generate-provider-ci-masks.py
```

PyMuPDF는 자산 재생성 시에만 필요하다. Rust 빌드·앱 실행에서는 SVG 파서,
이미지 디코더, 네트워크 조회, 추가 Cargo 의존성을 사용하지 않는다.

| 생성물 | SHA-256 |
|---|---|
| `codex.alpha` | `0787e60d7898876da11bca8733ad0d2e520b865e925e00b0f40854328c76ab9a` |
| `claude.alpha` | `82620188e86d4b15da1dc180656998d1e0644229fdf1bfe28e11fd3b9c48c52c` |
