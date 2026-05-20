//! [Task #852] HWPX → HWP OLE contract 스트림 변환
//!
//! 한컴 HWP 5.0 정답지는 다음 9 스트림 contract 를 요구한다:
//!
//! ```text
//! FileHeader, DocInfo, BodyText/Section0,        // 본문 (rhwp 기본 작성)
//! HwpSummaryInformation, DocOptions/_LinkDoc,    // 메타 (본 모듈)
//! Scripts/DefaultJScript, Scripts/JScriptVersion, // 스크립트 (본 모듈)
//! PrvImage, PrvText                              // 미리보기 (본 모듈)
//! ```
//!
//! HWPX 컨테이너에 동등 데이터가 있으면 (Preview, Scripts) 변환·passthrough,
//! 없으면 (HwpSummary, DocOptions/_LinkDoc, Scripts/JScriptVersion) 정적
//! fallback (`saved/blank2010.hwp` 추출) 사용.
//!
//! Stage 2.1 (본 모듈) = HWPX 컨테이너 → extra_streams (정공법).
//! Stage 2.2 (별도) = blank2010.hwp fallback.

use super::reader::HwpxReader;

/// HWPX 컨테이너 → HWP OLE 스트림 매핑 결과
pub(super) struct ContractStreams {
    /// `Vec<(path, data)>` 형태 — Document::extra_streams 에 그대로 주입 가능
    pub streams: Vec<(String, Vec<u8>)>,
}

/// HWPX ZIP reader 로부터 contract 스트림 4 개를 추출/변환.
///
/// - `Preview/PrvText.txt` (UTF-8) → `/PrvText` (UTF-16 LE)
/// - `Preview/PrvImage.png` → `/PrvImage` (passthrough)
/// - `Scripts/sourceScripts` → `/Scripts/DefaultJScript` (zlib deflate)
/// - `Scripts/headerScripts` → 정적 fallback 사용 (Stage 2.2)
///
/// HWPX 에 동등 파일이 없으면 해당 스트림은 생략. cfb_writer 가 한컴 정답지
/// 와 비교하여 추가 fallback (HwpSummary / DocOptions/_LinkDoc) 가 필요.
pub(super) fn extract_contract_streams(reader: &mut HwpxReader) -> ContractStreams {
    let mut streams = Vec::new();

    // PrvText.txt (UTF-8) → /PrvText (UTF-16 LE, HWP5 spec)
    if let Ok(prv_text_utf8) = reader.read_file("Preview/PrvText.txt") {
        let utf16_bytes: Vec<u8> = prv_text_utf8
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        streams.push(("/PrvText".to_string(), utf16_bytes));
    }

    // PrvImage.png → /PrvImage (PNG passthrough)
    if let Ok(prv_image_bytes) = reader.read_file_bytes("Preview/PrvImage.png") {
        streams.push(("/PrvImage".to_string(), prv_image_bytes));
    }

    // Scripts/sourceScripts → /Scripts/DefaultJScript (zlib deflate)
    if let Ok(source_scripts_bytes) = reader.read_file_bytes("Scripts/sourceScripts") {
        if let Some(compressed) = zlib_deflate(&source_scripts_bytes) {
            streams.push(("/Scripts/DefaultJScript".to_string(), compressed));
        }
    }

    ContractStreams { streams }
}

/// 단순 zlib deflate 헬퍼. 실패 시 None.
fn zlib_deflate(input: &[u8]) -> Option<Vec<u8>> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(input).ok()?;
    encoder.finish().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlib_deflate_roundtrip() {
        use flate2::read::ZlibDecoder;
        use std::io::Read;

        let input = b"hello rhwp scripts test";
        let compressed = zlib_deflate(input).expect("zlib deflate failed");
        let mut decoder = ZlibDecoder::new(&compressed[..]);
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .expect("zlib inflate failed");
        assert_eq!(decoded, input);
    }
}
