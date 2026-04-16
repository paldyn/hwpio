//! HWPX 빈 문서에 필요한 정적 보일러플레이트 파일
//!
//! 한컴 오피스가 HWPX를 열 때 요구하는 고정 메타 파일들을 인라인으로 보관한다.
//! Stage 1 범위: 빈 문서 기준. Stage 2+에서 실제 IR 기반으로 대체/확장될 수 있다.

/// version.xml — HCF 버전 선언
/// 한컴 스펙의 `tagetApplication` 오타(=target)는 의도적으로 유지한다.
pub const VERSION_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
    r#"<hv:HCFVersion xmlns:hv="http://www.hancom.co.kr/hwpml/2011/version""#,
    r#" tagetApplication="WORDPROCESSOR" major="5" minor="1" micro="0""#,
    r#" buildNumber="0" os="1" xmlVersion="1.2""#,
    r#" application="rhwp" appVersion="0.7.2"/>"#,
);

/// META-INF/container.xml — OCF 루트 엔트리 (한컴은 3개 rootfile 요구)
pub const META_INF_CONTAINER_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
    r#"<ocf:container xmlns:ocf="urn:oasis:names:tc:opendocument:xmlns:container""#,
    r#" xmlns:hpf="http://www.hancom.co.kr/schema/2011/hpf">"#,
    r#"<ocf:rootfiles>"#,
    r#"<ocf:rootfile full-path="Contents/content.hpf" media-type="application/hwpml-package+xml"/>"#,
    r#"<ocf:rootfile full-path="Preview/PrvText.txt" media-type="text/plain"/>"#,
    r#"<ocf:rootfile full-path="META-INF/container.rdf" media-type="application/rdf+xml"/>"#,
    r#"</ocf:rootfiles>"#,
    r#"</ocf:container>"#,
);

/// META-INF/container.rdf — 패키지 내 파일 역할 RDF 선언
pub const META_INF_CONTAINER_RDF: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
    r#"<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">"#,
    r#"<rdf:Description rdf:about="">"#,
    r#"<ns0:hasPart xmlns:ns0="http://www.hancom.co.kr/hwpml/2016/meta/pkg#" rdf:resource="Contents/header.xml"/>"#,
    r#"</rdf:Description>"#,
    r#"<rdf:Description rdf:about="Contents/header.xml">"#,
    r#"<rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#HeaderFile"/>"#,
    r#"</rdf:Description>"#,
    r#"<rdf:Description rdf:about="">"#,
    r#"<ns0:hasPart xmlns:ns0="http://www.hancom.co.kr/hwpml/2016/meta/pkg#" rdf:resource="Contents/section0.xml"/>"#,
    r#"</rdf:Description>"#,
    r#"<rdf:Description rdf:about="Contents/section0.xml">"#,
    r#"<rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#SectionFile"/>"#,
    r#"</rdf:Description>"#,
    r#"<rdf:Description rdf:about="">"#,
    r#"<rdf:type rdf:resource="http://www.hancom.co.kr/hwpml/2016/meta/pkg#Document"/>"#,
    r#"</rdf:Description>"#,
    r#"</rdf:RDF>"#,
);

/// META-INF/manifest.xml — 빈 ODF manifest (Hancom 관례상 필수)
pub const META_INF_MANIFEST_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
    r#"<odf:manifest xmlns:odf="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0"/>"#,
);

/// settings.xml — 애플리케이션 설정 (캐럿 위치)
pub const SETTINGS_XML: &str = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes" ?>"#,
    r#"<ha:HWPApplicationSetting xmlns:ha="http://www.hancom.co.kr/hwpml/2011/app""#,
    r#" xmlns:config="urn:oasis:names:tc:opendocument:xmlns:config:1.0">"#,
    r#"<ha:CaretPosition listIDRef="0" paraIDRef="0" pos="16"/>"#,
    r#"</ha:HWPApplicationSetting>"#,
);

/// Preview/PrvText.txt — 미리보기 텍스트 (빈 문서는 CRLF만)
pub const PRV_TEXT: &[u8] = b"\r\n";

/// Preview/PrvImage.png — 1x1 투명 PNG (67바이트, 한컴 호환 최소 썸네일)
///
/// 표준 base64 인코딩:
/// `iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=`
pub const PRV_IMAGE_PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
    0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
    0x08, 0x04, 0x00, 0x00, 0x00, 0xB5, 0x1C, 0x0C, // 8-bit GA, CRC
    0x02, 0x00, 0x00, 0x00, 0x0B, 0x49, 0x44, 0x41, // IDAT length=11 + "IDA"
    0x54, 0x78, 0x9C, 0x63, 0x64, 0x60, 0x00, 0x00, // "T" + zlib header + data
    0x00, 0x05, 0x00, 0x01, 0x6F, 0x68, 0x67, 0xBC, // IDAT CRC
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND
    0xAE, 0x42, 0x60, 0x82,                         // IEND CRC
];
