//! HWPX 라운드트립 통합 테스트.
//!
//! 각 Stage의 "완료 기준" = 이 파일의 해당 Stage 테스트가 IrDiff 0으로 통과.
//! **누적만 가능, 삭제·완화 금지**. Stage 5 완료 시 모든 샘플이 한 번에 통과해야 한다.
//!
//! Stage 0 (현재): blank_hwpx.hwpx 의 뼈대 필드(섹션 수·리소스 카운트) 유지 검증
//! Stage 1 예정: ref_empty.hwpx / ref_text.hwpx
//! Stage 2 예정: 다문단·run 분할
//! Stage 3 예정: ref_table.hwpx / hwp_table_test.hwp
//! Stage 4 예정: pic-in-head-01.hwp / pic-crop-01.hwp
//! Stage 5 예정: 대형 실문서 3건

use rhwp::serializer::hwpx::roundtrip::roundtrip_ir_diff;

#[test]
fn stage0_blank_hwpx_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/blank_hwpx.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("roundtrip must succeed");
    assert!(
        diff.is_empty(),
        "blank_hwpx.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

// ---------- Stage 1 ---------------------------------------------------------
// header.xml IR 기반 동적 생성 — 샘플 parse → serialize → parse 시 리소스 카운트가 보존돼야 함.

#[test]
fn stage1_ref_empty_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_empty.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_empty roundtrip");
    assert!(
        diff.is_empty(),
        "ref_empty.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

#[test]
fn stage1_ref_text_roundtrip() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_text.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_text roundtrip");
    assert!(
        diff.is_empty(),
        "ref_text.hwpx IR roundtrip must have no diff, got: {:#?}",
        diff
    );
}

// ---------- Stage 1 탐색용 진단 ----------------------------------------------
// 다음 두 샘플은 Stage 2/3 범위의 요소(run 분할·table)를 포함하므로 현재 Stage 1
// 수준에서는 diff가 없거나 일부 허용될 수 있다. 통과 여부로 Stage 1 header.xml 범위
// 내 회귀를 탐지한다 (section/table/run 차이는 다른 테스트가 커버).

#[test]
fn stage1_ref_mixed_header_level_regression_probe() {
    let bytes = include_bytes!("../samples/hwpx/ref/ref_mixed.hwpx");
    let diff = roundtrip_ir_diff(bytes).expect("ref_mixed roundtrip");
    // 현재 Stage 1 에서는 IrDiff 0 이어야 함 — section 문단 수도 뼈대 비교 대상
    // 문제가 있으면 panic. 추후 Stage 2에서 run 비교가 추가되며 조건 강화.
    if !diff.is_empty() {
        eprintln!("ref_mixed.hwpx diffs: {:#?}", diff);
    }
    assert!(diff.is_empty(), "ref_mixed header-level regression");
}

