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

use crate::model::control::Control;
use crate::model::document::Document;
use crate::model::paragraph::Paragraph;
use crate::model::table::Table;
use crate::parser::FileFormat;

use super::common_obj_attr_writer::serialize_common_obj_attr;

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
/// Stage 2: 표 `raw_ctrl_data` 합성 + `attr` 재구성. 기타 영역은 후속 Stage.
pub fn convert_hwpx_to_hwp_ir(doc: &mut Document) -> AdapterReport {
    let mut report = AdapterReport::new();

    for section in &mut doc.sections {
        for para in &mut section.paragraphs {
            adapt_paragraph(para, &mut report);
        }
    }

    report
}

fn adapt_paragraph(para: &mut Paragraph, report: &mut AdapterReport) {
    for ctrl in &mut para.controls {
        if let Control::Table(table) = ctrl {
            adapt_table(table, report);
        }
    }
}

fn adapt_table(table: &mut Table, report: &mut AdapterReport) {
    // 1. raw_ctrl_data 합성 (HWPX 출처는 비어있음)
    if table.raw_ctrl_data.is_empty() {
        table.raw_ctrl_data = serialize_common_obj_attr(&table.common);
        report.tables_ctrl_data_synthesized += 1;
    }

    // 2. attr 동기화: serializer/control.rs:349 가 raw_ctrl_data 를 그대로 쓰므로
    //    raw_ctrl_data 안의 attr 비트가 진실. table.attr 자체는 직렬화기 경로에서
    //    raw_ctrl_data 가 비어있을 때만 사용되므로 추가 작업 불필요.
    //    다만 IR 의 일관성을 위해 (다른 코드가 table.attr 을 읽을 가능성) 동기화.
    if table.attr == 0 && !table.raw_ctrl_data.is_empty() && table.raw_ctrl_data.len() >= 4 {
        let attr = u32::from_le_bytes([
            table.raw_ctrl_data[0],
            table.raw_ctrl_data[1],
            table.raw_ctrl_data[2],
            table.raw_ctrl_data[3],
        ]);
        if attr != 0 {
            table.attr = attr;
            report.tables_attr_packed += 1;
        }
    }

    // 셀 내부 문단 재귀 (중첩 표 대응)
    for cell in &mut table.cells {
        for cpara in &mut cell.paragraphs {
            adapt_paragraph(cpara, report);
        }
    }
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
    fn empty_doc_no_change() {
        let mut doc = Document::default();
        let report = convert_hwpx_to_hwp_ir(&mut doc);
        assert!(!report.changed_anything());
        assert!(report.skipped_reason.is_none());
    }

    #[test]
    fn hwp_source_no_op_via_filter() {
        let mut doc = Document::default();
        let report = convert_if_hwpx_source(&mut doc, FileFormat::Hwp);
        assert_eq!(report.skipped_reason.as_deref(), Some("source_format != Hwpx"));
    }

    #[test]
    fn idempotent_when_called_twice() {
        let mut doc = Document::default();
        let r1 = convert_hwpx_to_hwp_ir(&mut doc);
        let r2 = convert_hwpx_to_hwp_ir(&mut doc);
        // 두 번째 호출은 변경 없음 (이미 정규화됨).
        assert_eq!(r2.tables_ctrl_data_synthesized, 0);
        assert_eq!(r1, r2);
    }
}
