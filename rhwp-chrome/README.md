# rhwp — HWP 문서 뷰어 & 에디터 (Chrome/Edge 확장)

브라우저에서 HWP/HWPX 파일을 바로 열고 편집할 수 있는 확장 프로그램입니다.

## 특징

- **설치 없이 열기** — 확장 설치 한 번이면 HWP 파일을 브라우저에서 바로 열람
- **편집 지원** — 텍스트 입력/수정, 표 편집, 서식 변경
- **인쇄** — Ctrl+P로 인쇄, PDF 저장 가능
- **자동 감지** — 웹페이지의 HWP 링크를 자동 감지하고 아이콘 표시
- **개인정보 보호** — 파일이 서버로 전송되지 않음, 모든 처리는 브라우저 내 WASM
- **무료** — MIT 라이선스, 개인/기업 무료
- **광고 없음**

## 설치

- [Chrome Web Store](#) (준비 중)
- [Microsoft Edge Add-ons](#) (준비 중)

## 사용 방법

### HWP 파일 열기

1. **웹 다운로드**: HWP 파일 다운로드 시 자동으로 뷰어 탭에서 열림
2. **드래그 & 드롭**: 확장 아이콘 클릭 → 빈 뷰어 탭 → 파일 드래그
3. **우클릭 메뉴**: HWP 링크 우클릭 → "rhwp로 열기"
4. **배지 클릭**: 웹페이지의 HWP 링크 옆 파란색 H 배지 클릭

### 인쇄

- 파일 메뉴 → 인쇄 또는 **Ctrl+P**
- 인쇄 미리보기 창에서 [인쇄] 버튼 클릭

### 저장

- **Ctrl+S** 또는 파일 메뉴 → 저장
- HWP 형식으로 저장

## 웹사이트 개발자

공공 웹사이트에 `data-hwp-*` 속성을 추가하면 사용자 경험이 향상됩니다.
자세한 내용은 [개발자 가이드](DEVELOPER_GUIDE.md)를 참조하세요.

```html
<a href="/files/공문.hwp" data-hwp="true" data-hwp-title="공문" data-hwp-pages="5">
  공문.hwp
</a>
```

## 빌드

```bash
cd rhwp-chrome
npm install
npm run build
```

빌드 결과물은 `dist/` 폴더에 생성됩니다.

### 개발 모드 설치

1. `chrome://extensions` (또는 `edge://extensions`)
2. 개발자 모드 활성화
3. "압축 해제된 확장 프로그램을 로드합니다" → `rhwp-chrome/dist/` 선택

## 라이선스

MIT License — Copyright (c) 2026 Edward Kim

## 개인정보 처리방침

[PRIVACY.md](PRIVACY.md) 참조
