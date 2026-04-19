//! HWPX → HWP IR 어댑터 통합 테스트 (#178)
//!
//! Stage 1: 베이스라인 측정 (페이지 폭주 + 영역별 차이 인벤토리).
//!         아직 어댑터 본체가 동작하지 않으므로 회복 검증 없음 — 측정만.

use rhwp::document_core::DocumentCore;
use rhwp::document_core::converters::diagnostics::diff_hwpx_vs_serializer_assumptions;
use rhwp::document_core::converters::hwpx_to_hwp::{
    convert_if_hwpx_source, convert_hwpx_to_hwp_ir,
};

fn load_sample(name: &str) -> Vec<u8> {
    let path = format!("samples/hwpx/{}", name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("샘플 로드 실패 {}: {}", path, e))
}

fn page_count_after_hwp_export(hwpx_bytes: &[u8]) -> (u32, u32) {
    let core = DocumentCore::from_bytes(hwpx_bytes).expect("HWPX 로드 실패");
    let original_pages = core.page_count();

    let hwp_bytes = core.export_hwp_native().expect("HWP 직렬화 실패");

    let reloaded = DocumentCore::from_bytes(&hwp_bytes).expect("HWP 재로드 실패");
    let reloaded_pages = reloaded.page_count();

    (original_pages, reloaded_pages)
}

/// 베이스라인 측정: 현 단계는 페이지 폭주 (reloaded > orig) 가 발생하는 것이 "정상".
/// 어댑터 영역별 매핑이 누적되면서 폭주 비율이 줄고, Stage 5 완료 시점에는
/// reloaded == orig 가 되도록 게이트가 강화된다.
fn assert_explosion_baseline(name: &str, bytes: &[u8]) {
    let (orig, reloaded) = page_count_after_hwp_export(bytes);
    eprintln!("[#178 baseline] {}: orig={}, reloaded={}", name, orig, reloaded);
    assert!(orig >= 1, "{}: 원본 페이지 수 측정 실패", name);
    assert!(
        reloaded > orig,
        "{}: 현 단계는 폭주가 발생해야 정상 (어댑터 미적용). orig={}, reloaded={}",
        name,
        orig,
        reloaded
    );
}

#[test]
fn baseline_page_count_explosion_hwpx_h_01() {
    assert_explosion_baseline("hwpx-h-01", &load_sample("hwpx-h-01.hwpx"));
}

#[test]
fn baseline_page_count_explosion_hwpx_h_02() {
    assert_explosion_baseline("hwpx-h-02", &load_sample("hwpx-h-02.hwpx"));
}

#[test]
fn baseline_page_count_explosion_hwpx_h_03() {
    let bytes = load_sample("hwpx-h-03.hwpx");
    let (orig, reloaded) = page_count_after_hwp_export(&bytes);
    eprintln!("[#178 baseline] hwpx-h-03: orig={}, reloaded={}", orig, reloaded);
    // hwpx-h-03 은 폭주 여부 자체가 미확정 — 측정만 기록.
    assert!(orig >= 1);
    assert!(reloaded >= 1);
}

#[test]
fn baseline_diff_inventory_hwpx_h_01() {
    let bytes = load_sample("hwpx-h-01.hwpx");
    let core = DocumentCore::from_bytes(&bytes).expect("HWPX 로드 실패");
    let summary = diff_hwpx_vs_serializer_assumptions(core.document());
    eprintln!("[#178 inventory] hwpx-h-01:\n{}", summary.human_report());
    // 영역별 카운트는 측정만. assert 는 의미있는 영역이 1개 이상 검출됐는지.
    let counts = summary.counts_by_area();
    let interesting = counts.iter().any(|(a, c)| {
        *c > 0
            && (*a == "table.raw_ctrl_data"
                || *a == "paragraph.line_seg.vertical_pos"
                || *a == "cell.list_attr.bit16")
    });
    assert!(
        interesting,
        "hwpx-h-01 에서 위반 영역이 검출돼야 함 (페이지 폭주가 발생하므로). counts={:?}",
        counts
    );
}

#[test]
fn adapter_idempotent_no_op_in_stage_1() {
    let bytes = load_sample("hwpx-h-01.hwpx");
    let core = DocumentCore::from_bytes(&bytes).expect("HWPX 로드 실패");

    // Stage 1 본체는 no-op 이므로 두 번 호출해도 같은 결과.
    let mut doc1 = core.document().clone();
    let mut doc2 = core.document().clone();

    let r1 = convert_hwpx_to_hwp_ir(&mut doc1);
    let r2 = convert_hwpx_to_hwp_ir(&mut doc2);
    assert_eq!(r1, r2);
    assert!(!r1.changed_anything());
}

#[test]
fn adapter_skips_hwp_source() {
    let mut doc = rhwp::model::document::Document::default();
    let report = convert_if_hwpx_source(&mut doc, rhwp::parser::FileFormat::Hwp);
    assert_eq!(report.skipped_reason.as_deref(), Some("source_format != Hwpx"));
}
