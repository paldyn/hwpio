//! Contents/section{N}.xml — Section 본문 직렬화 (Stage 1: 최소 형태)
//!
//! Stage 1에서는 파서가 파싱 에러 없이 `Section::default()`와 동등한 결과를 내는
//! 최소 뼈대만 출력한다. Stage 2에서 문단/텍스트/lineSegArray를 추가한다.

use std::io::Cursor;

use quick_xml::Writer;

use crate::model::document::{Document, Section};

use super::utils::{end_tag, start_tag_attrs, write_xml_decl};
use super::SerializeError;

/// section{index}.xml을 생성한다.
pub fn write_section(
    _section: &Section,
    _doc: &Document,
    _index: usize,
) -> Result<Vec<u8>, SerializeError> {
    let buf = Cursor::new(Vec::new());
    let mut w = Writer::new(buf);

    write_xml_decl(&mut w)?;
    start_tag_attrs(
        &mut w,
        "hs:sec",
        &[
            ("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"),
            ("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"),
        ],
    )?;
    // Stage 1: 문단 없음
    end_tag(&mut w, "hs:sec")?;

    Ok(w.into_inner().into_inner())
}
