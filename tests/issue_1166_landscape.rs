//! Issue #1166: HWPX 가로 편집 용지가 세로로 렌더링되는 결함.
//!
//! OWPML pagePr `landscape` 값: WIDELY=세로(Portrait), NARROWLY=가로(Landscape).
//! (hwplib ForSecPr: Portrait→WIDELY, Landscape→NARROWLY.)
//! 종전 HWPX 파서는 landscape 를 무시(false 고정)해 가로 용지가 세로로 렌더됐다.
//!
//! width/height 는 HWP 바이너리와 동일하게 짧은변=width/긴변=height 로 저장되고,
//! landscape=true 일 때 렌더러가 swap 한다.

use std::fs;
use std::path::Path;

fn load(rel: &str) -> rhwp::wasm_api::HwpDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    let bytes = fs::read(&path).unwrap_or_else(|e| panic!("read {}: {}", rel, e));
    rhwp::wasm_api::HwpDocument::from_bytes(&bytes).expect("parse")
}

fn page_def_landscape(doc: &rhwp::wasm_api::HwpDocument) -> bool {
    let json = doc.get_page_def_native(0).expect("page def");
    let key = "\"landscape\":";
    let s = json.find(key).expect("landscape key") + key.len();
    let rest = &json[s..];
    let end = rest.find(|c: char| c == ',' || c == '}').expect("delim");
    rest[..end].trim() == "true"
}

#[test]
fn landscape_001_hwpx_is_landscape() {
    // landscape-001.hwpx 는 가로 편집 용지 (pagePr landscape="NARROWLY").
    // HWPX 파서가 NARROWLY → landscape=true 로 정합해야 한다.
    let doc = load("samples/hwpx/landscape-001.hwpx");
    assert!(
        page_def_landscape(&doc),
        "가로 편집 용지 HWPX(landscape=NARROWLY)가 landscape=true 로 파싱되어야 함"
    );
}

#[test]
fn portrait_hwpx_stays_portrait() {
    // para-001.hwpx 는 세로 용지 (pagePr landscape="WIDELY") — 회귀 가드.
    let doc = load("samples/hwpx/para-001.hwpx");
    assert!(
        !page_def_landscape(&doc),
        "세로 용지 HWPX(landscape=WIDELY)는 landscape=false 여야 함"
    );
}

#[test]
fn landscape_hwp5_matches_hwpx() {
    // 동일 문서의 HWP5(가로)도 landscape=true (HWP5 는 이미 정상, 정합 확인).
    let doc = load("samples/landscape-001.hwp");
    assert!(
        page_def_landscape(&doc),
        "가로 편집 용지 HWP5 는 landscape=true (HWPX 와 정합)"
    );
}
