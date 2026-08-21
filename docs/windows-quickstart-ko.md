# Windows 테스트 실행 가이드

현재 V0.2.1은 Windows 10/11 x64를 지원합니다.

## 1. 준비물

- Node.js 22 이상
- Rust stable
- Visual Studio Build Tools 2022의 **Desktop development with C++** 워크로드
- WebView2 Runtime (최신 Windows 10/11에는 일반적으로 설치되어 있음)

## 2. 소스 실행

PowerShell에서 프로젝트 폴더로 이동한 뒤:

```powershell
npm install
npm run validate
npm run tauri:dev
```

## 3. Windows 설치파일 만들기

```powershell
npm run tauri --workspace @5level/desktop -- build --bundles msi,nsis
```

결과는 보통 다음 경로에 생성됩니다.

```text
apps/desktop/src-tauri/target/release/bundle/msi/
apps/desktop/src-tauri/target/release/bundle/nsis/
```

## 4. 주의사항

현재 공개 테스트 빌드는 코드 서명이 없습니다. Windows SmartScreen 경고가 나올 수 있습니다. 실제 배포 전에는 Authenticode 코드 서명을 추가해야 합니다.

처음 테스트할 때는 실제 업무폴더 전체가 아니라 복사본 Workspace를 권장합니다.
