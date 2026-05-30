// HanPage 데스크톱 앱 엔트리.
//
// 1단계(Task #1)는 기존 rhwp-studio 빌드 산출물 + WASM을 OS 웹뷰에 그대로 로드만
// 한다. 네이티브 기능(열기/저장 dialog, 파일 연결, 메뉴, 최근문서, 윈도우 상태)은
// 후속 Stage에서 플러그인과 함께 추가한다.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running HanPage desktop application");
}
