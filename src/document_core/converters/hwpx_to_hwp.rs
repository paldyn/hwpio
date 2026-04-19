//! HWPX → HWP IR 매핑 어댑터
//!
//! HWPX 파서가 채운 IR 을 HWP 직렬화기가 받아들이는 형태로 정규화한다.
//!
//! ## 핵심 원칙
//!
//! - **HWP 직렬화기 0줄 수정**: `serializer/cfb_writer.rs`, `body_text.rs`,
//!   `control.rs` 등은 변경하지 않는다.
//! - **IR 만 만진다**: 진입점은 `&mut Document` 이며, 출력은 IR 필드 갱신뿐.
//! - **idempotent**: 같은 IR 에 두 번 호출해도 같은 결과.
//! - **HWP 출처 보호**: `source_format == Hwpx` 일 때만 동작. HWP 출처는 no-op.
//!
//! ## 매핑 명세서
//!
//! HWP 직렬화기가 IR 에서 무엇을 읽는지가 단 하나의 명세서 (구현계획서 §1.3 참조).
//!
//! Stage 1 (현재): 진입점만 노출. 영역별 매핑은 Stage 2~ 에서 추가.

use crate::model::document::Document;
use crate::parser::FileFormat;

/// 어댑터 실행 보고서.
///
/// 각 영역별로 변환된 항목 수를 누적한다. 진단 도구와 단계별 회귀 측정에 사용.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct AdapterReport {
    /// 변환을 건너뛴 사유 (HWP 출처 등). None 이면 정상 적용.
    pub skipped_reason: Option<String>,
    /// `table.raw_ctrl_data` 합성 횟수 (Stage 2)
    pub tables_ctrl_data_synthesized: u32,
    /// `table.attr` 재구성 횟수 (Stage 2)
    pub tables_attr_packed: u32,
    /// `cell.list_attr bit 16` 보강 횟수 (Stage 3)
    pub cells_list_attr_bit16_set: u32,
    /// 문단 break_type 보정 횟수 (Stage 4)
    pub paragraphs_break_type_set: u32,
    /// lineseg vpos 사전계산 적용 문단 수 (Stage 4)
    pub paragraphs_vpos_precomputed: u32,
}

impl AdapterReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn no_op(mut self, reason: impl Into<String>) -> Self {
        self.skipped_reason = Some(reason.into());
        self
    }

    /// 어댑터가 실제로 무언가를 변경했는지 여부.
    pub fn changed_anything(&self) -> bool {
        self.skipped_reason.is_none()
            && (self.tables_ctrl_data_synthesized
                + self.tables_attr_packed
                + self.cells_list_attr_bit16_set
                + self.paragraphs_break_type_set
                + self.paragraphs_vpos_precomputed)
                > 0
    }
}

/// HWPX 출처 IR 을 HWP 직렬화기가 기대하는 형태로 정규화한다.
///
/// HWP 출처에는 no-op (idempotent + 보호).
///
/// Stage 1: 진입점만 노출 (영역별 매핑 미구현, source_format 분기만 동작).
pub fn convert_hwpx_to_hwp_ir(doc: &mut Document) -> AdapterReport {
    let report = AdapterReport::new();

    // source_format 검사: Document 자체에는 source_format 이 없으므로
    // 호출자(DocumentCore) 가 책임진다. 현재 단계는 받은 doc 을 그대로 처리.
    // Stage 5 통합 진입점에서 DocumentCore 측 분기와 결합.
    let _ = doc; // Stage 1 은 변경 없음
    report
}

/// `source_format` 검사 후 어댑터를 호출하는 보조 함수.
///
/// 호출자: `DocumentCore::export_hwp_with_adapter()` (Stage 5 에서 추가).
pub fn convert_if_hwpx_source(doc: &mut Document, source_format: FileFormat) -> AdapterReport {
    if source_format != FileFormat::Hwpx {
        return AdapterReport::new().no_op("source_format != Hwpx");
    }
    convert_hwpx_to_hwp_ir(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage1_entry_point_returns_default_report() {
        let mut doc = Document::default();
        let report = convert_hwpx_to_hwp_ir(&mut doc);
        assert!(!report.changed_anything());
        assert!(report.skipped_reason.is_none());
    }

    #[test]
    fn stage1_hwp_source_no_op() {
        let mut doc = Document::default();
        let report = convert_if_hwpx_source(&mut doc, FileFormat::Hwp);
        assert_eq!(report.skipped_reason.as_deref(), Some("source_format != Hwpx"));
    }

    #[test]
    fn stage1_hwpx_source_runs_adapter() {
        let mut doc = Document::default();
        let report = convert_if_hwpx_source(&mut doc, FileFormat::Hwpx);
        // Stage 1 본체는 no-op 이므로 changed_anything() = false 이지만
        // skipped_reason 도 None (어댑터가 실행되긴 했음).
        assert!(report.skipped_reason.is_none());
        assert!(!report.changed_anything());
    }
}
