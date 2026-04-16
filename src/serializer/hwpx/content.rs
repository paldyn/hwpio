//! Contents/content.hpf — OPF 패키지 매니페스트 쓰기
//!
//! `parser::hwpx::content`의 역방향.
//! `<opf:manifest>`에 섹션/BinData 항목을 등록하고 `<opf:spine>`에 순서를 기록한다.

use std::io::Cursor;

use quick_xml::Writer;

use super::utils::{empty_tag, end_tag, start_tag_attrs, text, write_xml_decl};
use super::SerializeError;

/// BinData 엔트리 (manifest 등록용)
#[derive(Debug, Clone)]
pub struct BinDataEntry {
    /// manifest item id (예: "image1")
    pub id: String,
    /// ZIP 내 상대 경로 (예: "BinData/image1.png")
    pub href: String,
    /// MIME 유형
    pub media_type: String,
}

/// content.hpf XML을 생성한다.
///
/// - `section_hrefs`: `Contents/section0.xml`, `Contents/section1.xml` ... 순서
/// - `bin_data`: BinData manifest 항목
pub fn write_content_hpf(
    section_hrefs: &[String],
    bin_data: &[BinDataEntry],
) -> Result<Vec<u8>, SerializeError> {
    let buf = Cursor::new(Vec::new());
    let mut w = Writer::new(buf);

    write_xml_decl(&mut w)?;

    start_tag_attrs(
        &mut w,
        "opf:package",
        &[
            ("xmlns:opf", "http://www.idpf.org/2007/opf/"),
            ("version", "1.2"),
            ("unique-identifier", "rhwp-hwpx"),
        ],
    )?;

    // <opf:metadata> (최소)
    start_tag_attrs(
        &mut w,
        "opf:metadata",
        &[("xmlns:dc", "http://purl.org/dc/elements/1.1/")],
    )?;
    start_tag_attrs(&mut w, "opf:meta-data", &[])?;
    end_tag(&mut w, "opf:meta-data")?;
    start_tag_attrs(&mut w, "dc:title", &[])?;
    text(&mut w, "")?;
    end_tag(&mut w, "dc:title")?;
    end_tag(&mut w, "opf:metadata")?;

    // <opf:manifest>
    start_tag_attrs(&mut w, "opf:manifest", &[])?;

    empty_tag(
        &mut w,
        "opf:item",
        &[
            ("id", "header"),
            ("href", "Contents/header.xml"),
            ("media-type", "application/xml"),
        ],
    )?;

    for (i, href) in section_hrefs.iter().enumerate() {
        let id = format!("section{}", i);
        empty_tag(
            &mut w,
            "opf:item",
            &[
                ("id", id.as_str()),
                ("href", href.as_str()),
                ("media-type", "application/xml"),
            ],
        )?;
    }

    for entry in bin_data {
        empty_tag(
            &mut w,
            "opf:item",
            &[
                ("id", entry.id.as_str()),
                ("href", entry.href.as_str()),
                ("media-type", entry.media_type.as_str()),
            ],
        )?;
    }

    end_tag(&mut w, "opf:manifest")?;

    // <opf:spine>
    start_tag_attrs(&mut w, "opf:spine", &[])?;
    empty_tag(
        &mut w,
        "opf:itemref",
        &[("idref", "header"), ("linear", "yes")],
    )?;
    for i in 0..section_hrefs.len() {
        let id = format!("section{}", i);
        empty_tag(
            &mut w,
            "opf:itemref",
            &[("idref", id.as_str()), ("linear", "yes")],
        )?;
    }
    end_tag(&mut w, "opf:spine")?;

    end_tag(&mut w, "opf:package")?;

    Ok(w.into_inner().into_inner())
}

/// META-INF/container.xml — OPC 루트 엔트리 경로 선언. 한컴 호환용.
pub fn write_container_xml() -> Result<Vec<u8>, SerializeError> {
    let buf = Cursor::new(Vec::new());
    let mut w = Writer::new(buf);
    write_xml_decl(&mut w)?;
    start_tag_attrs(
        &mut w,
        "ocf:container",
        &[
            ("xmlns:ocf", "urn:oasis:names:tc:opendocument:xmlns:container"),
            ("version", "1.0"),
        ],
    )?;
    start_tag_attrs(&mut w, "ocf:rootfiles", &[])?;
    empty_tag(
        &mut w,
        "ocf:rootfile",
        &[
            ("full-path", "Contents/content.hpf"),
            ("media-type", "application/hwpml-package+xml"),
        ],
    )?;
    end_tag(&mut w, "ocf:rootfiles")?;
    end_tag(&mut w, "ocf:container")?;
    Ok(w.into_inner().into_inner())
}
