//! HWPX(ZIP+XML) 직렬화 모듈 — `parser::hwpx`의 역방향.
//!
//! Document IR을 HWPX 파일(ZIP 컨테이너)로 변환한다.
//!
//! ## 단계
//! - Stage 1 (현재): 모듈 스켈레톤 + 빈 HWPX 생성 (parse_hwpx 라운드트립)
//! - Stage 2: 본문 문단·텍스트·lineSegArray
//! - Stage 3: 표(Table)
//! - Stage 4: 그림(Picture) + BinData
//! - Stage 5: 라운드트립 테스트 + CLI

pub mod content;
pub mod header;
pub mod section;
pub mod utils;
pub mod writer;

use crate::model::document::Document;

use super::SerializeError;
use content::write_container_xml;
use writer::HwpxZipWriter;

/// Document IR을 HWPX(ZIP+XML) 바이트로 직렬화한다.
pub fn serialize_hwpx(doc: &Document) -> Result<Vec<u8>, SerializeError> {
    let mut z = HwpxZipWriter::new();

    // 1. mimetype (반드시 최초 엔트리, STORED)
    z.write_stored("mimetype", b"application/hwp+zip")?;

    // 2. META-INF/container.xml (OCF — 한컴 호환용)
    let container_xml = write_container_xml()?;
    z.write_deflated("META-INF/container.xml", &container_xml)?;

    // 3. Contents/header.xml
    let header_xml = header::write_header(doc)?;
    z.write_deflated("Contents/header.xml", &header_xml)?;

    // 4. Contents/section{N}.xml (섹션 수만큼)
    let section_hrefs: Vec<String> = (0..doc.sections.len())
        .map(|i| format!("Contents/section{}.xml", i))
        .collect();
    for (i, sec) in doc.sections.iter().enumerate() {
        let xml = section::write_section(sec, doc, i)?;
        z.write_deflated(&section_hrefs[i], &xml)?;
    }

    // 5. Contents/content.hpf (OPF 매니페스트 — 마지막에 써도 무방)
    let content_hpf = content::write_content_hpf(&section_hrefs, &[])?;
    z.write_deflated("Contents/content.hpf", &content_hpf)?;

    z.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::hwpx::parse_hwpx;

    #[test]
    fn serialize_empty_doc_parses_back() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize empty");
        let parsed = parse_hwpx(&bytes).expect("parse back");
        assert_eq!(parsed.sections.len(), 0, "empty doc should have 0 sections");
        assert!(parsed.bin_data_content.is_empty());
    }

    #[test]
    fn serialize_with_one_section_parses_back() {
        let mut doc = Document::default();
        doc.sections.push(crate::model::document::Section::default());
        let bytes = serialize_hwpx(&doc).expect("serialize one-section");
        let parsed = parse_hwpx(&bytes).expect("parse back");
        assert_eq!(parsed.sections.len(), 1);
    }

    #[test]
    fn mimetype_is_first_entry() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize");
        // ZIP local file header 시그니처 PK\x03\x04 이후 30바이트 뒤에 파일명이 온다.
        // 이름 길이는 offset 26-27 (LE u16).
        assert_eq!(&bytes[0..4], b"PK\x03\x04", "ZIP signature");
        let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let name = &bytes[30..30 + name_len];
        assert_eq!(name, b"mimetype");
    }

    #[test]
    fn mimetype_stored_not_deflated() {
        let doc = Document::default();
        let bytes = serialize_hwpx(&doc).expect("serialize");
        // compression method 필드는 offset 8-9 (LE u16). STORED=0.
        let method = u16::from_le_bytes([bytes[8], bytes[9]]);
        assert_eq!(method, 0, "mimetype must be STORED (method=0)");
    }
}
