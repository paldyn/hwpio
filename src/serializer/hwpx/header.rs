//! Contents/header.xml — DocInfo/DocProperties 직렬화 (Stage 1: 최소 형태)
//!
//! `parser::hwpx::header`의 역방향. 이번 단계에서는 파서가 거부하지 않고
//! IR 라운드트립에 영향을 주지 않을 만큼의 최소 XML만 출력한다.
//! 다음 단계(Stage 2)에서 fonts/charShapes/paraShapes 등을 완전 직렬화한다.

use std::io::Cursor;

use quick_xml::Writer;

use crate::model::document::Document;

use super::utils::{end_tag, start_tag_attrs, write_xml_decl};
use super::SerializeError;

/// header.xml을 생성한다. Stage 1에서는 최소 뼈대만 출력.
pub fn write_header(_doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let buf = Cursor::new(Vec::new());
    let mut w = Writer::new(buf);

    write_xml_decl(&mut w)?;
    start_tag_attrs(
        &mut w,
        "hh:head",
        &[
            ("xmlns:hh", "http://www.hancom.co.kr/hwpml/2011/head"),
            ("version", "1.31"),
            ("secCnt", "0"),
        ],
    )?;
    end_tag(&mut w, "hh:head")?;

    Ok(w.into_inner().into_inner())
}
