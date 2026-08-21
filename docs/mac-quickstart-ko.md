# macOS 테스트 실행 가이드

현재 V0.2.1은 macOS 12+를 지원하며 Apple Silicon/Intel 구조를 대상으로 합니다.

## 권장: 소스에서 직접 실행

다운로드한 미서명 `.app`/`.dmg`는 Gatekeeper가 차단할 수 있으므로 개발 단계에서는 소스 실행을 권장합니다.

필수 환경:

- Xcode Command Line Tools
- Node.js 22 이상
- Rust stable

확인:

```bash
xcode-select -p
node -v
rustc --version
```

프로젝트 폴더에서:

```bash
npm install
npm run validate
npm run tauri:dev
```

## 로컬 macOS 앱 빌드

현재 Mac 아키텍처용 앱:

```bash
npm run tauri --workspace @5level/desktop -- build --bundles app,dmg
```

GitHub Actions에서는 Apple Silicon + Intel Universal 빌드를 생성하도록 설정되어 있습니다.

## Gatekeeper

공개 다운로드용 macOS 앱을 정상 배포하려면 Apple Developer ID 서명과 Notarization이 필요합니다. V0.2.1 개발 빌드는 아직 해당 절차가 적용되지 않았습니다.
