//! Issue #1143: external image bytes injection and PageLayerTree cache invalidation.
//!
//! Injection must share the renderer's binDataId index-first lookup semantics and
//! must invalidate cached page trees when image bytes become available after the
//! tree has already been built.

use rhwp::model::bin_data::BinDataContent;
use rhwp::wasm_api::HwpDocument;
use serde::Deserialize;
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExternalImageReference {
    key: String,
    bin_data_id: u16,
    original_path: String,
    basename: String,
    extension: String,
    loaded: bool,
}

fn load_doc(rel_path: &str) -> HwpDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
    HwpDocument::from_bytes(&bytes).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn read_sample(rel_path: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e))
}

fn external_image_refs(doc: &HwpDocument) -> Vec<ExternalImageReference> {
    let json = doc.get_external_image_references();
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse refs JSON {json}: {e}"))
}

fn external_image_basenames(doc: &HwpDocument) -> Vec<String> {
    let json = doc.get_external_image_basenames();
    serde_json::from_str(&json).unwrap_or_else(|e| panic!("parse basename JSON {json}: {e}"))
}

fn collect_image_ops<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if value.get("type").and_then(Value::as_str) == Some("image") {
                out.push(value);
            }
            for child in map.values() {
                collect_image_ops(child, out);
            }
        }
        Value::Array(items) => {
            for child in items {
                collect_image_ops(child, out);
            }
        }
        _ => {}
    }
}

fn page_layer_image_payload_count(doc: &HwpDocument, page: u32) -> usize {
    let json = doc
        .get_page_layer_tree_native(page)
        .unwrap_or_else(|e| panic!("page layer tree {page}: {e}"));
    let parsed: Value =
        serde_json::from_str(&json).unwrap_or_else(|e| panic!("PageLayerTree JSON: {e}"));
    let mut images = Vec::new();
    collect_image_ops(&parsed, &mut images);
    images
        .iter()
        .filter(|image| {
            image.get("mime").is_some()
                && image
                    .get("base64")
                    .and_then(Value::as_str)
                    .is_some_and(|data| !data.is_empty())
        })
        .count()
}

#[test]
fn issue_1143_basename_injection_marks_reference_loaded() {
    let mut doc = load_doc("samples/hwp3-sample10.hwp");
    let oracle = read_sample("samples/oracle.gif");

    let before = external_image_refs(&doc);
    let oracle_ref = before
        .iter()
        .find(|reference| reference.bin_data_id == 1)
        .expect("binDataId 1 ref");
    assert_eq!(oracle_ref.key, "binData:1");
    assert_eq!(oracle_ref.basename, "oracle.gif");
    assert_eq!(oracle_ref.extension, "gif");
    assert!(oracle_ref.original_path.ends_with("oracle.gif"));
    assert!(!oracle_ref.loaded);

    let injected = doc.inject_external_image("oracle.gif", &oracle, "/tmp/oracle.gif");
    assert_eq!(injected, 1);

    let after = external_image_refs(&doc);
    let oracle_ref = after
        .iter()
        .find(|reference| reference.bin_data_id == 1)
        .expect("binDataId 1 ref");
    assert!(oracle_ref.loaded);

    let basenames = external_image_basenames(&doc);
    assert!(
        !basenames.iter().any(|name| name == "oracle.gif"),
        "injected basename should be hidden from the legacy missing-image API"
    );
    assert!(
        basenames.iter().any(|name| name == "rdb02.gif")
            && basenames.iter().any(|name| name == "s1.jpg"),
        "still-missing image basenames should remain visible"
    );
}

#[test]
fn issue_1143_basename_injection_respects_index_first_loaded_state() {
    let mut doc = load_doc("samples/hwp3-sample10.hwp");

    doc.document_mut().bin_data_content[0] = BinDataContent {
        id: 99,
        data: vec![b'G', b'I', b'F'],
        extension: "gif".to_string(),
    };
    let before = doc.document().bin_data_content[0].clone();

    let injected = doc.inject_external_image("oracle.gif", &read_sample("samples/oracle.gif"), "");
    assert_eq!(
        injected, 0,
        "already-loaded detection must follow bin_data_content[binDataId - 1]"
    );
    assert_eq!(doc.document().bin_data_content[0].id, before.id);
    assert_eq!(doc.document().bin_data_content[0].data, before.data);
    assert_eq!(
        doc.document().bin_data_content[0].extension,
        before.extension
    );
}

#[test]
fn issue_1143_wasm_injection_invalidates_cached_page_layer_tree() {
    let mut doc = load_doc("samples/hwp3-sample10.hwp");

    let before = page_layer_image_payload_count(&doc, 0);
    assert_eq!(
        before, 0,
        "sample10 page 1 external images should have no payload before injection"
    );

    for (basename, sample_path) in [
        ("oracle.gif", "samples/oracle.gif"),
        ("rdb02.gif", "samples/rdb02.gif"),
        ("s1.jpg", "samples/s1.jpg"),
    ] {
        let data = read_sample(sample_path);
        assert_eq!(doc.inject_external_image(basename, &data, ""), 1);
    }

    let after = page_layer_image_payload_count(&doc, 0);
    assert!(
        after > before,
        "PageLayerTree cache should be rebuilt with injected image payloads"
    );
}

#[test]
fn issue_1143_native_dir_population_invalidates_cached_page_layer_tree() {
    let mut doc = load_doc("samples/hwp3-sample10.hwp");

    let before = page_layer_image_payload_count(&doc, 0);
    assert_eq!(
        before, 0,
        "sample10 page 1 external images should have no payload before directory population"
    );

    let sample_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("samples");
    let loaded = doc.populate_external_images_from_dir(&sample_dir);
    assert_eq!(loaded, 3);

    assert!(
        external_image_refs(&doc)
            .iter()
            .all(|reference| reference.loaded),
        "all discovered sample10 external image refs should be loaded"
    );

    let after = page_layer_image_payload_count(&doc, 0);
    assert!(
        after > before,
        "native directory population should rebuild cached PageLayerTrees"
    );
}
