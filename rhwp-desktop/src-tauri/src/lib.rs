// HanPage 데스크톱 앱 엔트리.
//
// 1단계(Task #1)는 기존 rhwp-studio 빌드 산출물 + WASM을 OS 웹뷰에 그대로 로드한다.
// 엔진(WASM)은 수정하지 않는다.
//
// Stage 2: 네이티브 열기/저장 dialog 를 Rust 측 app command 로 제공한다. dialog 와
// 파일 IO 를 전부 Rust 에서 처리하므로 프런트(JS)에는 dialog/fs 플러그인 권한을 주지
// 않는다(앱 자체 command 는 ACL 권한 대상이 아니다). 프런트의 web-inert 브리지
// (`rhwp-studio/src/core/desktop-bridge.ts`)가 `window.__TAURI__.core.invoke` 로 호출한다.

use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

/// 열기 dialog 로 선택·읽은 문서. `data` 는 파일 바이트(현재 JSON 배열 직렬화 —
/// 일반 HWP 문서는 수십 KB~수 MB 라 1단계 검증에는 충분. 대용량 최적화는 후속).
#[derive(Serialize)]
struct OpenedFile {
    name: String,
    path: String,
    data: Vec<u8>,
}

/// 저장 dialog 결과. 프런트에서 `status` 로 분기한다(saved/cancelled).
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum SaveOutcome {
    Saved { path: String, name: String },
    Cancelled,
}

/// 네이티브 열기 dialog → 선택 파일 바이트 반환. 취소 시 `Ok(None)`.
///
/// async command 는 메인 스레드가 아닌 async 런타임 워커에서 실행되므로 여기서
/// `blocking_pick_file()`(내부적으로 dialog 를 메인 스레드에 디스패치)을 호출해도
/// 데드락이 없다. 메인 스레드에서 직접 호출하면 안 된다.
#[tauri::command]
async fn cmd_open_document(app: tauri::AppHandle) -> Result<Option<OpenedFile>, String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("한글 문서 (HWP/HWPX)", &["hwp", "hwpx"])
        .blocking_pick_file();

    let Some(file_path) = picked else {
        return Ok(None); // 사용자 취소
    };
    let path = file_path.into_path().map_err(|e| e.to_string())?;
    let data = std::fs::read(&path).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document")
        .to_string();
    Ok(Some(OpenedFile {
        name,
        path: path.to_string_lossy().into_owned(),
        data,
    }))
}

/// 네이티브 저장 dialog → 선택 경로에 바이트 기록. 취소 시 `SaveOutcome::Cancelled`.
#[tauri::command]
async fn cmd_save_document(
    app: tauri::AppHandle,
    suggested_name: String,
    data: Vec<u8>,
) -> Result<SaveOutcome, String> {
    let picked = app
        .dialog()
        .file()
        .set_file_name(&suggested_name)
        .add_filter("한글 문서 (HWP)", &["hwp"])
        .blocking_save_file();

    let Some(file_path) = picked else {
        return Ok(SaveOutcome::Cancelled); // 사용자 취소
    };
    let path = file_path.into_path().map_err(|e| e.to_string())?;
    std::fs::write(&path, &data).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.hwp")
        .to_string();
    Ok(SaveOutcome::Saved {
        path: path.to_string_lossy().into_owned(),
        name,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![cmd_open_document, cmd_save_document])
        .run(tauri::generate_context!())
        .expect("error while running HanPage desktop application");
}
