//! Contents/section{N}.xml — Section 본문 직렬화
//!
//! Stage 1: 한컴2020 레퍼런스(ref_empty.hwpx)의 section0.xml을 그대로 사용한다.
//! Stage 2+: Section IR의 문단·런·표·그림을 동적 직렬화하도록 교체한다.

use crate::model::document::{Document, Section};
use super::SerializeError;

const EMPTY_SECTION_XML: &str = include_str!("templates/empty_section0.xml");

pub fn write_section(
    _section: &Section,
    _doc: &Document,
    _index: usize,
) -> Result<Vec<u8>, SerializeError> {
    Ok(EMPTY_SECTION_XML.as_bytes().to_vec())
}
