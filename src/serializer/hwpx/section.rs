//! Contents/section{N}.xml — Section 본문 직렬화
//!
//! Stage 1: 한컴2020 오픈 가능한 최소 완전 섹션을 출력한다.
//! - 1개 문단(`<hp:p>`) + 1개 런(`<hp:run>`) with full `<hp:secPr>`
//! - `<hp:linesegarray>` 1개 lineseg
//!
//! Stage 2+에서 Document IR의 실제 문단·런을 직렬화하도록 확장한다.

use std::io::Cursor;

use quick_xml::Writer;

use crate::model::document::{Document, Section};

use super::utils::{empty_tag, end_tag, start_tag_attrs, write_xml_decl};
use super::SerializeError;

/// A4 용지 기본값 (template/empty.hwp 기준, HWPUNIT)
const PAGE_WIDTH: u32 = 59528;
const PAGE_HEIGHT: u32 = 84188;
const MARGIN_LEFT: u32 = 8504;
const MARGIN_RIGHT: u32 = 8504;
const MARGIN_TOP: u32 = 5668;
const MARGIN_BOTTOM: u32 = 4252;
const MARGIN_HEADER: u32 = 4252;
const MARGIN_FOOTER: u32 = 4252;
const BODY_WIDTH: u32 = 42520; // = page - left - right

/// section{index}.xml 생성
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
            ("xmlns:ha", "http://www.hancom.co.kr/hwpml/2011/app"),
            ("xmlns:hp", "http://www.hancom.co.kr/hwpml/2011/paragraph"),
            ("xmlns:hp10", "http://www.hancom.co.kr/hwpml/2016/paragraph"),
            ("xmlns:hs", "http://www.hancom.co.kr/hwpml/2011/section"),
            ("xmlns:hc", "http://www.hancom.co.kr/hwpml/2011/core"),
            ("xmlns:hh", "http://www.hancom.co.kr/hwpml/2011/head"),
            ("xmlns:hhs", "http://www.hancom.co.kr/hwpml/2011/history"),
            ("xmlns:hm", "http://www.hancom.co.kr/hwpml/2011/master-page"),
            ("xmlns:hpf", "http://www.hancom.co.kr/schema/2011/hpf"),
            ("xmlns:dc", "http://purl.org/dc/elements/1.1/"),
            ("xmlns:opf", "http://www.idpf.org/2007/opf/"),
            ("xmlns:ooxmlchart", "http://www.hancom.co.kr/hwpml/2016/ooxmlchart"),
            ("xmlns:epub", "http://www.idpf.org/2007/ops"),
            ("xmlns:config", "urn:oasis:names:tc:opendocument:xmlns:config:1.0"),
        ],
    )?;

    // 단 하나의 빈 문단
    start_tag_attrs(
        &mut w,
        "hp:p",
        &[
            ("id", "0"),
            ("paraPrIDRef", "0"),
            ("styleIDRef", "0"),
            ("pageBreak", "0"),
            ("columnBreak", "0"),
            ("merged", "0"),
        ],
    )?;

    // 첫 런: SectionDef + ColumnDef 컨트롤
    start_tag_attrs(&mut w, "hp:run", &[("charPrIDRef", "0")])?;

    // <hp:secPr>: 섹션 정의 (한컴 요구 전체 블록)
    start_tag_attrs(
        &mut w,
        "hp:secPr",
        &[
            ("id", ""),
            ("textDirection", "HORIZONTAL"),
            ("spaceColumns", "1134"),
            ("tabStop", "8000"),
            ("outlineShapeIDRef", "0"),
            ("memoShapeIDRef", "0"),
            ("textVerticalWidthHead", "0"),
            ("masterPageCnt", "0"),
        ],
    )?;
    empty_tag(
        &mut w,
        "hp:grid",
        &[("lineGrid", "0"), ("charGrid", "0"), ("wonggojiFormat", "0")],
    )?;
    empty_tag(
        &mut w,
        "hp:startNum",
        &[
            ("pageStartsOn", "BOTH"),
            ("page", "0"),
            ("pic", "0"),
            ("tbl", "0"),
            ("equation", "0"),
        ],
    )?;
    empty_tag(
        &mut w,
        "hp:visibility",
        &[
            ("hideFirstHeader", "0"),
            ("hideFirstFooter", "0"),
            ("hideFirstMasterPage", "0"),
            ("border", "SHOW_ALL"),
            ("fill", "SHOW_ALL"),
            ("hideFirstPageNum", "0"),
            ("hideFirstEmptyLine", "0"),
            ("showLineNumber", "0"),
        ],
    )?;
    empty_tag(
        &mut w,
        "hp:lineNumberShape",
        &[
            ("restartType", "0"),
            ("countBy", "0"),
            ("distance", "0"),
            ("startNumber", "0"),
        ],
    )?;

    // pagePr
    let page_w = PAGE_WIDTH.to_string();
    let page_h = PAGE_HEIGHT.to_string();
    start_tag_attrs(
        &mut w,
        "hp:pagePr",
        &[
            ("landscape", "WIDELY"),
            ("width", page_w.as_str()),
            ("height", page_h.as_str()),
            ("gutterType", "LEFT_ONLY"),
        ],
    )?;
    let ml = MARGIN_LEFT.to_string();
    let mr = MARGIN_RIGHT.to_string();
    let mt = MARGIN_TOP.to_string();
    let mb = MARGIN_BOTTOM.to_string();
    let mh = MARGIN_HEADER.to_string();
    let mf = MARGIN_FOOTER.to_string();
    empty_tag(
        &mut w,
        "hp:margin",
        &[
            ("header", mh.as_str()),
            ("footer", mf.as_str()),
            ("gutter", "0"),
            ("left", ml.as_str()),
            ("right", mr.as_str()),
            ("top", mt.as_str()),
            ("bottom", mb.as_str()),
        ],
    )?;
    end_tag(&mut w, "hp:pagePr")?;

    // footNotePr / endNotePr (기본값)
    write_note_pr(&mut w, "hp:footNotePr", "EACH_COLUMN", "-1")?;
    write_note_pr(&mut w, "hp:endNotePr", "END_OF_DOCUMENT", "14692344")?;

    // pageBorderFill ×3 (BOTH/EVEN/ODD)
    for t in &["BOTH", "EVEN", "ODD"] {
        start_tag_attrs(
            &mut w,
            "hp:pageBorderFill",
            &[
                ("type", *t),
                ("borderFillIDRef", "1"),
                ("textBorder", "PAPER"),
                ("headerInside", "0"),
                ("footerInside", "0"),
                ("fillArea", "PAPER"),
            ],
        )?;
        empty_tag(
            &mut w,
            "hp:offset",
            &[("left", "1417"), ("right", "1417"), ("top", "1417"), ("bottom", "1417")],
        )?;
        end_tag(&mut w, "hp:pageBorderFill")?;
    }

    end_tag(&mut w, "hp:secPr")?;

    // <hp:ctrl><hp:colPr/>
    start_tag_attrs(&mut w, "hp:ctrl", &[])?;
    empty_tag(
        &mut w,
        "hp:colPr",
        &[
            ("id", ""),
            ("type", "NEWSPAPER"),
            ("layout", "LEFT"),
            ("colCount", "1"),
            ("sameSz", "1"),
            ("sameGap", "0"),
        ],
    )?;
    end_tag(&mut w, "hp:ctrl")?;

    end_tag(&mut w, "hp:run")?;

    // 빈 텍스트 런
    start_tag_attrs(&mut w, "hp:run", &[("charPrIDRef", "0")])?;
    empty_tag(&mut w, "hp:t", &[])?;
    end_tag(&mut w, "hp:run")?;

    // <hp:linesegarray>
    start_tag_attrs(&mut w, "hp:linesegarray", &[])?;
    let body_w = BODY_WIDTH.to_string();
    empty_tag(
        &mut w,
        "hp:lineseg",
        &[
            ("textpos", "0"),
            ("vertpos", "0"),
            ("vertsize", "1000"),
            ("textheight", "1000"),
            ("baseline", "850"),
            ("spacing", "600"),
            ("horzpos", "0"),
            ("horzsize", body_w.as_str()),
            ("flags", "393216"),
        ],
    )?;
    end_tag(&mut w, "hp:linesegarray")?;

    end_tag(&mut w, "hp:p")?;
    end_tag(&mut w, "hs:sec")?;

    Ok(w.into_inner().into_inner())
}

fn write_note_pr<W: std::io::Write>(
    w: &mut Writer<W>,
    tag: &str,
    placement: &str,
    note_line_length: &str,
) -> Result<(), SerializeError> {
    start_tag_attrs(w, tag, &[])?;
    empty_tag(
        w,
        "hp:autoNumFormat",
        &[
            ("type", "DIGIT"),
            ("userChar", ""),
            ("prefixChar", ""),
            ("suffixChar", ")"),
            ("supscript", "0"),
        ],
    )?;
    empty_tag(
        w,
        "hp:noteLine",
        &[
            ("length", note_line_length),
            ("type", "SOLID"),
            ("width", "0.12 mm"),
            ("color", "#000000"),
        ],
    )?;
    empty_tag(
        w,
        "hp:noteSpacing",
        &[
            ("betweenNotes", "283"),
            ("belowLine", "567"),
            ("aboveLine", "850"),
        ],
    )?;
    empty_tag(
        w,
        "hp:numbering",
        &[("type", "CONTINUOUS"), ("newNum", "1")],
    )?;
    empty_tag(
        w,
        "hp:placement",
        &[("place", placement), ("beneathText", "0")],
    )?;
    end_tag(w, tag)?;
    Ok(())
}
