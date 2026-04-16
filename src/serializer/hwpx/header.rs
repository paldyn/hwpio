//! Contents/header.xml — DocInfo 리소스 테이블 직렬화
//!
//! Stage 1: 한컴2020이 문서 열람을 거부하지 않을 최소 골격을 출력한다.
//! - 7개 언어 fontfaces (각 1개 폰트)
//! - 1개 charPr / 1개 paraPr / 1개 borderFill / 1개 style / 1개 tabPr
//! - compatibleDocument / docOption / beginNum
//!
//! Stage 2+에서 Document.doc_info 실제 값(폰트 목록, char/para shape 전체) 직렬화로 대체한다.

use std::io::{Cursor, Write};

use quick_xml::Writer;

use crate::model::document::Document;

use super::utils::{empty_tag, end_tag, start_tag_attrs, write_xml_decl};
use super::SerializeError;

/// 언어별 기본 폰트 이름 (라이선스 중립 — 시스템 기본)
const DEFAULT_FONTS: [(&str, &str); 7] = [
    ("HANGUL", "함초롬바탕"),
    ("LATIN", "Times New Roman"),
    ("HANJA", "함초롬바탕"),
    ("JAPANESE", "함초롬바탕"),
    ("OTHER", "함초롬바탕"),
    ("SYMBOL", "함초롬바탕"),
    ("USER", "함초롬바탕"),
];

/// header.xml 생성 (Stage 1 최소 골격)
pub fn write_header(_doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let buf = Cursor::new(Vec::new());
    let mut w = Writer::new(buf);

    write_xml_decl(&mut w)?;

    start_tag_attrs(
        &mut w,
        "hh:head",
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
            ("version", "1.2"),
            ("secCnt", "1"),
        ],
    )?;

    // beginNum
    empty_tag(
        &mut w,
        "hh:beginNum",
        &[
            ("page", "1"),
            ("footnote", "1"),
            ("endnote", "1"),
            ("pic", "1"),
            ("tbl", "1"),
            ("equation", "1"),
        ],
    )?;

    start_tag_attrs(&mut w, "hh:refList", &[])?;

    // fontfaces (7개 언어)
    start_tag_attrs(&mut w, "hh:fontfaces", &[("itemCnt", "7")])?;
    for (lang, font) in DEFAULT_FONTS.iter() {
        start_tag_attrs(
            &mut w,
            "hh:fontface",
            &[("lang", lang), ("fontCnt", "1")],
        )?;
        empty_tag(
            &mut w,
            "hh:font",
            &[("id", "0"), ("face", font), ("type", "TTF"), ("isEmbedded", "0")],
        )?;
        end_tag(&mut w, "hh:fontface")?;
    }
    end_tag(&mut w, "hh:fontfaces")?;

    // borderFills (1개 기본 "없음")
    start_tag_attrs(&mut w, "hh:borderFills", &[("itemCnt", "1")])?;
    write_default_border_fill(&mut w, 1)?;
    end_tag(&mut w, "hh:borderFills")?;

    // charProperties (1개 기본)
    start_tag_attrs(&mut w, "hh:charProperties", &[("itemCnt", "1")])?;
    write_default_char_pr(&mut w, 0)?;
    end_tag(&mut w, "hh:charProperties")?;

    // tabProperties (1개 기본)
    start_tag_attrs(&mut w, "hh:tabProperties", &[("itemCnt", "1")])?;
    write_default_tab_pr(&mut w, 0)?;
    end_tag(&mut w, "hh:tabProperties")?;

    // numberings (0개) — Hancom accepts itemCnt=0
    empty_tag(&mut w, "hh:numberings", &[("itemCnt", "0")])?;

    // paraProperties (1개 기본)
    start_tag_attrs(&mut w, "hh:paraProperties", &[("itemCnt", "1")])?;
    write_default_para_pr(&mut w, 0)?;
    end_tag(&mut w, "hh:paraProperties")?;

    // styles (1개 본문 스타일)
    start_tag_attrs(&mut w, "hh:styles", &[("itemCnt", "1")])?;
    empty_tag(
        &mut w,
        "hh:style",
        &[
            ("id", "0"),
            ("type", "PARA"),
            ("name", "바탕글"),
            ("engName", "Normal"),
            ("paraPrIDRef", "0"),
            ("charPrIDRef", "0"),
            ("nextStyleIDRef", "0"),
            ("langID", "1042"),
            ("lockForm", "0"),
        ],
    )?;
    end_tag(&mut w, "hh:styles")?;

    end_tag(&mut w, "hh:refList")?;

    // compatibleDocument
    start_tag_attrs(&mut w, "hh:compatibleDocument", &[("targetProgram", "HWP")])?;
    empty_tag(
        &mut w,
        "hh:layoutCompatibility",
        &[
            ("char", "0"),
            ("paragraph", "0"),
            ("section", "0"),
            ("object", "0"),
            ("field", "0"),
        ],
    )?;
    end_tag(&mut w, "hh:compatibleDocument")?;

    // docOption
    start_tag_attrs(&mut w, "hh:docOption", &[])?;
    empty_tag(&mut w, "hh:linkinfo", &[("path", ""), ("pageInherit", "0"), ("footnoteInherit", "0")])?;
    end_tag(&mut w, "hh:docOption")?;

    end_tag(&mut w, "hh:head")?;

    Ok(w.into_inner().into_inner())
}

fn write_default_border_fill<W: Write>(
    w: &mut Writer<W>,
    id: u32,
) -> Result<(), SerializeError> {
    let id_s = id.to_string();
    start_tag_attrs(
        w,
        "hh:borderFill",
        &[
            ("id", id_s.as_str()),
            ("threeD", "0"),
            ("shadow", "0"),
            ("centerLine", "NONE"),
            ("breakCellSeparateLine", "0"),
        ],
    )?;
    // 4변 + 대각선: 실선 0.12mm 검정
    for side in &["hh:leftBorder", "hh:rightBorder", "hh:topBorder", "hh:bottomBorder"] {
        empty_tag(w, side, &[("type", "SOLID"), ("width", "0.12 mm"), ("color", "#000000")])?;
    }
    empty_tag(w, "hh:diagonal", &[("type", "SLASH"), ("width", "0.12 mm"), ("color", "#000000")])?;
    empty_tag(w, "hh:backSlash", &[])?;
    end_tag(w, "hh:borderFill")?;
    Ok(())
}

fn write_default_char_pr<W: Write>(w: &mut Writer<W>, id: u32) -> Result<(), SerializeError> {
    let id_s = id.to_string();
    start_tag_attrs(
        w,
        "hh:charPr",
        &[
            ("id", id_s.as_str()),
            ("height", "1000"),
            ("textColor", "#000000"),
            ("shadeColor", "none"),
            ("useFontSpace", "0"),
            ("useKerning", "0"),
            ("symMark", "NONE"),
            ("borderFillIDRef", "0"),
        ],
    )?;
    // fontRef / ratio / spacing / relSz / offset — 7개 언어 각각
    for element in &["hh:fontRef", "hh:ratio", "hh:spacing", "hh:relSz", "hh:offset"] {
        start_tag_attrs(w, element, &[])?;
        for lang in &["hangul", "latin", "hanja", "japanese", "other", "symbol", "user"] {
            // 각 언어별 기본값 (간단화: 빈 속성)
            let attrs: &[(&str, &str)] = match *element {
                "hh:fontRef" => &[],
                "hh:ratio" => &[],
                "hh:spacing" => &[],
                "hh:relSz" => &[],
                "hh:offset" => &[],
                _ => &[],
            };
            // 언어 속성 포함
            let mut full_attrs: Vec<(&str, &str)> = attrs.to_vec();
            full_attrs.push(("hangul", "0"));
            let _ = full_attrs;
            let default_v = match *element {
                "hh:fontRef" => "0",
                "hh:ratio" => "100",
                "hh:spacing" => "0",
                "hh:relSz" => "100",
                "hh:offset" => "0",
                _ => "0",
            };
            empty_tag(w, lang, &[(match *element {
                "hh:fontRef" => "fontIDRef",
                "hh:ratio" => "val",
                "hh:spacing" => "val",
                "hh:relSz" => "val",
                "hh:offset" => "val",
                _ => "val",
            }, default_v)])?;
        }
        end_tag(w, element)?;
    }
    empty_tag(w, "hh:italic", &[])?;
    empty_tag(w, "hh:bold", &[])?;
    empty_tag(w, "hh:underline", &[("type", "NONE"), ("shape", "SOLID"), ("color", "#000000")])?;
    empty_tag(w, "hh:strikeout", &[("shape", "NONE"), ("color", "#000000")])?;
    empty_tag(w, "hh:outline", &[("type", "NONE")])?;
    empty_tag(w, "hh:shadow", &[("type", "NONE"), ("color", "#B2B2B2"), ("offsetX", "10"), ("offsetY", "10")])?;
    empty_tag(w, "hh:emboss", &[])?;
    empty_tag(w, "hh:engrave", &[])?;
    empty_tag(w, "hh:supscript", &[])?;
    empty_tag(w, "hh:subscript", &[])?;
    end_tag(w, "hh:charPr")?;
    Ok(())
}

fn write_default_para_pr<W: Write>(w: &mut Writer<W>, id: u32) -> Result<(), SerializeError> {
    let id_s = id.to_string();
    start_tag_attrs(
        w,
        "hh:paraPr",
        &[
            ("id", id_s.as_str()),
            ("tabPrIDRef", "0"),
            ("condense", "0"),
            ("fontLineHeight", "0"),
            ("snapToGrid", "1"),
            ("suppressLineNumbers", "0"),
            ("checked", "0"),
        ],
    )?;
    empty_tag(w, "hh:align", &[
        ("horizontal", "JUSTIFY"),
        ("vertical", "BASELINE"),
    ])?;
    empty_tag(w, "hh:heading", &[("type", "NONE"), ("idRef", "0"), ("level", "0")])?;
    empty_tag(w, "hh:breakSetting", &[
        ("breakLatinWord", "KEEP_WORD"),
        ("breakNonLatinWord", "KEEP_WORD"),
        ("widowOrphan", "0"),
        ("keepWithNext", "0"),
        ("keepLines", "0"),
        ("pageBreakBefore", "0"),
        ("lineWrap", "BREAK"),
    ])?;
    empty_tag(w, "hh:margin", &[
        ("intent", "0"),
    ])?;
    empty_tag(w, "hh:lineSpacing", &[
        ("type", "PERCENT"),
        ("value", "160"),
        ("unit", "HWPUNIT"),
    ])?;
    empty_tag(w, "hh:border", &[
        ("borderFillIDRef", "0"),
        ("offsetLeft", "0"),
        ("offsetRight", "0"),
        ("offsetTop", "0"),
        ("offsetBottom", "0"),
        ("connect", "0"),
        ("ignoreMargin", "0"),
    ])?;
    empty_tag(w, "hh:autoSpacing", &[
        ("eAsianEng", "0"),
        ("eAsianNum", "0"),
    ])?;
    end_tag(w, "hh:paraPr")?;
    Ok(())
}

fn write_default_tab_pr<W: Write>(w: &mut Writer<W>, id: u32) -> Result<(), SerializeError> {
    let id_s = id.to_string();
    empty_tag(
        w,
        "hh:tabPr",
        &[
            ("id", id_s.as_str()),
            ("autoTabLeft", "1"),
            ("autoTabRight", "1"),
            ("itemCnt", "0"),
        ],
    )?;
    Ok(())
}
