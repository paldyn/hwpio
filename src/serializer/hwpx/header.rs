//! Contents/header.xml — DocInfo 리소스 테이블 직렬화
//!
//! Stage 1: 한컴2020 레퍼런스(ref_empty.hwpx)의 header.xml을 그대로 사용한다.
//! Stage 2+: Document.doc_info IR 기반으로 동적 생성하도록 교체한다.

use crate::model::document::Document;
use super::SerializeError;

const EMPTY_HEADER_XML: &str = include_str!("templates/empty_header.xml");

pub fn write_header(_doc: &Document) -> Result<Vec<u8>, SerializeError> {
    Ok(EMPTY_HEADER_XML.as_bytes().to_vec())
}
