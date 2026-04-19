//! 단계 10 단위 테스트 — EMR_HEADER 파싱 + 레코드 시퀀스 읽기.

use super::*;
use super::parser::records::Record;

/// 최소 88바이트 EMR_HEADER + EMR_EOF(0x14바이트) 조합. ext 없음.
fn fixture_minimal_header_eof() -> Vec<u8> {
    let mut b = Vec::with_capacity(88 + 20);

    // EMR_HEADER (88바이트)
    b.extend_from_slice(&1u32.to_le_bytes());             // Type=1
    b.extend_from_slice(&88u32.to_le_bytes());            // Size=88
    // Bounds RECTL (16B)
    b.extend_from_slice(&0i32.to_le_bytes());
    b.extend_from_slice(&0i32.to_le_bytes());
    b.extend_from_slice(&1000i32.to_le_bytes());
    b.extend_from_slice(&500i32.to_le_bytes());
    // Frame RECTL (16B) — 0.01mm 단위
    b.extend_from_slice(&0i32.to_le_bytes());
    b.extend_from_slice(&0i32.to_le_bytes());
    b.extend_from_slice(&10000i32.to_le_bytes());
    b.extend_from_slice(&5000i32.to_le_bytes());
    // Signature " EMF"
    b.extend_from_slice(&0x464D4520u32.to_le_bytes());
    // Version
    b.extend_from_slice(&0x00010000u32.to_le_bytes());
    // Bytes (전체 파일 크기) / Records / Handles / Reserved
    b.extend_from_slice(&108u32.to_le_bytes());           // 88 + 20 EOF
    b.extend_from_slice(&2u32.to_le_bytes());             // Records
    b.extend_from_slice(&1u16.to_le_bytes());             // Handles
    b.extend_from_slice(&0u16.to_le_bytes());             // Reserved
    // nDescription / offDescription / nPalEntries
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    // Device SIZEL (8B)
    b.extend_from_slice(&1920i32.to_le_bytes());
    b.extend_from_slice(&1080i32.to_le_bytes());
    // Millimeters SIZEL (8B)
    b.extend_from_slice(&508i32.to_le_bytes());
    b.extend_from_slice(&286i32.to_le_bytes());
    assert_eq!(b.len(), 88);

    // EMR_EOF (최소 20바이트: type+size + nPalEntries(4) + offPalEntries(4) + SizeLast(4))
    b.extend_from_slice(&14u32.to_le_bytes());            // Type=14
    b.extend_from_slice(&20u32.to_le_bytes());            // Size=20
    b.extend_from_slice(&0u32.to_le_bytes());             // nPalEntries
    b.extend_from_slice(&0u32.to_le_bytes());             // offPalEntries
    b.extend_from_slice(&20u32.to_le_bytes());            // SizeLast

    b
}

#[test]
fn parses_minimal_header_and_eof() {
    let bytes = fixture_minimal_header_eof();
    let records = parse_emf(&bytes).expect("parse");
    assert_eq!(records.len(), 2);

    match &records[0] {
        Record::Header(h) => {
            assert_eq!(h.signature, 0x464D4520);
            assert_eq!(h.bounds.right, 1000);
            assert_eq!(h.bounds.bottom, 500);
            assert_eq!(h.device.cx, 1920);
            assert_eq!(h.handles, 1);
            assert!(h.ext1.is_none());
            assert!(h.ext2.is_none());
        }
        other => panic!("first record not Header: {other:?}"),
    }
    matches!(records[1], Record::Eof);
}

#[test]
fn rejects_bad_signature() {
    let mut bytes = fixture_minimal_header_eof();
    // signature는 EMR_HEADER 시작 + 40 = offset 40에 위치. Type/Size 제외하면 [40..44].
    bytes[40] = 0xAA;
    bytes[41] = 0xBB;
    bytes[42] = 0xCC;
    bytes[43] = 0xDD;
    match parse_emf(&bytes) {
        Err(Error::InvalidSignature { got }) => {
            assert_eq!(got, 0xDDCCBBAA);
        }
        other => panic!("expected InvalidSignature, got {other:?}"),
    }
}

#[test]
fn rejects_non_header_first_record() {
    // type=14(EOF)를 선두에 둔 손상된 EMF.
    let mut b = Vec::new();
    b.extend_from_slice(&14u32.to_le_bytes());
    b.extend_from_slice(&20u32.to_le_bytes());
    b.extend_from_slice(&[0u8; 12]);
    match parse_emf(&b) {
        Err(Error::InvalidFirstRecord { got }) => assert_eq!(got, 14),
        other => panic!("expected InvalidFirstRecord, got {other:?}"),
    }
}

#[test]
fn rejects_misaligned_record() {
    let mut b = Vec::new();
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&87u32.to_le_bytes()); // size not multiple of 4
    b.extend_from_slice(&[0u8; 79]);
    match parse_emf(&b) {
        Err(Error::MisalignedRecord { size, .. }) => assert_eq!(size, 87),
        other => panic!("expected MisalignedRecord, got {other:?}"),
    }
}

#[test]
fn parses_header_with_extensions() {
    // Size=108 (ext1+ext2 포함)로 빌드.
    let mut b = Vec::new();
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&108u32.to_le_bytes()); // Size=108
    // 80B 본체
    for _ in 0..20 {
        b.extend_from_slice(&0u32.to_le_bytes());
    }
    // signature는 offset 40에 있어야 함 → b[40..44]. 지금까지 8(type/size)+80 = 88.
    // 이미 0으로 채워져 있으니 offset 40의 4바이트를 signature로 교체.
    b[40..44].copy_from_slice(&0x464D4520u32.to_le_bytes());

    // ext1 (12B): cbPixelFormat, offPixelFormat, bOpenGL
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    // ext2 (8B)
    b.extend_from_slice(&12345u32.to_le_bytes());
    b.extend_from_slice(&6789u32.to_le_bytes());
    assert_eq!(b.len(), 108);

    // EOF
    b.extend_from_slice(&14u32.to_le_bytes());
    b.extend_from_slice(&20u32.to_le_bytes());
    b.extend_from_slice(&[0u8; 12]);

    let records = parse_emf(&b).expect("parse");
    let Record::Header(h) = &records[0] else { panic!() };
    assert_eq!(h.ext1.unwrap().b_open_gl, 1);
    assert_eq!(h.ext2.unwrap().micrometers_x, 12345);
    assert_eq!(h.ext2.unwrap().micrometers_y, 6789);
}

/// 단계 11 — 객체/상태 레코드 파싱 테스트용 픽스처 빌더.
fn header_prefix() -> Vec<u8> {
    // 최소 88B 헤더. fixture_minimal_header_eof에서 EOF 제외.
    let mut b = fixture_minimal_header_eof();
    b.truncate(88);
    b
}

fn push_record(b: &mut Vec<u8>, rt: u32, payload: &[u8]) {
    let size = 8u32 + payload.len() as u32;
    assert!(size % 4 == 0, "record size must be 4-aligned");
    b.extend_from_slice(&rt.to_le_bytes());
    b.extend_from_slice(&size.to_le_bytes());
    b.extend_from_slice(payload);
}

fn push_eof(b: &mut Vec<u8>) {
    push_record(b, 14, &[0u8; 12]);
}

#[test]
fn parses_create_pen_and_select_and_delete() {
    let mut b = header_prefix();

    // EMR_CREATEPEN (0x26): handle=1, style=0(PS_SOLID), width=2, reserved=0, color=0x00FF0000
    let mut pen_payload = Vec::new();
    pen_payload.extend_from_slice(&1u32.to_le_bytes());       // handle
    pen_payload.extend_from_slice(&0u32.to_le_bytes());       // style
    pen_payload.extend_from_slice(&2i32.to_le_bytes());       // width.x
    pen_payload.extend_from_slice(&0i32.to_le_bytes());       // width.y
    pen_payload.extend_from_slice(&0x00FF0000u32.to_le_bytes()); // color
    push_record(&mut b, 0x26, &pen_payload);

    // EMR_SELECTOBJECT (0x25): handle=1
    push_record(&mut b, 0x25, &1u32.to_le_bytes());

    // EMR_DELETEOBJECT (0x28): handle=1
    push_record(&mut b, 0x28, &1u32.to_le_bytes());

    push_eof(&mut b);

    let recs = parse_emf(&b).expect("parse");
    assert_eq!(recs.len(), 5);

    match &recs[1] {
        Record::CreatePen { handle, pen } => {
            assert_eq!(*handle, 1);
            assert_eq!(pen.style, 0);
            assert_eq!(pen.width, 2);
            assert_eq!(pen.color, 0x00FF0000);
        }
        other => panic!("expected CreatePen, got {other:?}"),
    }
    assert!(matches!(recs[2], Record::SelectObject { handle: 1 }));
    assert!(matches!(recs[3], Record::DeleteObject { handle: 1 }));
    assert!(matches!(recs[4], Record::Eof));
}

#[test]
fn parses_create_brush_indirect() {
    let mut b = header_prefix();
    // EMR_CREATEBRUSHINDIRECT (0x27): handle=2, style=0, color=0x00112233, hatch=0
    let mut p = Vec::new();
    p.extend_from_slice(&2u32.to_le_bytes());
    p.extend_from_slice(&0u32.to_le_bytes());
    p.extend_from_slice(&0x00112233u32.to_le_bytes());
    p.extend_from_slice(&0u32.to_le_bytes());
    push_record(&mut b, 0x27, &p);
    push_eof(&mut b);

    let recs = parse_emf(&b).expect("parse");
    match &recs[1] {
        Record::CreateBrushIndirect { handle, brush } => {
            assert_eq!(*handle, 2);
            assert_eq!(brush.color, 0x00112233);
        }
        other => panic!("expected CreateBrushIndirect, got {other:?}"),
    }
}

#[test]
fn parses_ext_create_font_indirect_w() {
    let mut b = header_prefix();
    // EMR_EXTCREATEFONTINDIRECTW (0x52): handle(4) + LogFontW(92) = 96B payload.
    let mut p = Vec::new();
    p.extend_from_slice(&3u32.to_le_bytes());           // handle
    p.extend_from_slice(&(-12i32).to_le_bytes());       // height
    p.extend_from_slice(&0i32.to_le_bytes());           // width
    p.extend_from_slice(&0i32.to_le_bytes());           // escapement
    p.extend_from_slice(&0i32.to_le_bytes());           // orientation
    p.extend_from_slice(&700i32.to_le_bytes());         // weight (bold)
    p.extend_from_slice(&[1u8, 0, 0, 1, 0, 0, 0, 0]);   // italic/underline/strikeout/charset + precisions
    // FaceName "Arial" + null padding
    let face: Vec<u16> = "Arial".encode_utf16().collect();
    for w in &face {
        p.extend_from_slice(&w.to_le_bytes());
    }
    for _ in face.len()..32 {
        p.extend_from_slice(&0u16.to_le_bytes());
    }
    assert_eq!(p.len(), 96);
    push_record(&mut b, 0x52, &p);
    push_eof(&mut b);

    let recs = parse_emf(&b).expect("parse");
    match &recs[1] {
        Record::ExtCreateFontIndirectW { handle, font } => {
            assert_eq!(*handle, 3);
            assert_eq!(font.weight, 700);
            assert_eq!(font.italic, 1);
            assert_eq!(font.face_name, "Arial");
            assert_eq!(font.height, -12);
        }
        other => panic!("expected ExtCreateFontIndirectW, got {other:?}"),
    }
}

#[test]
fn parses_dc_stack_and_world_transform() {
    let mut b = header_prefix();
    push_record(&mut b, 0x21, &[]);                      // EMR_SAVEDC
    // EMR_SETWORLDTRANSFORM (0x23): XForm(24B)
    let mut p = Vec::new();
    for v in [2.0_f32, 0.0, 0.0, 3.0, 10.0, 20.0] {
        p.extend_from_slice(&v.to_le_bytes());
    }
    push_record(&mut b, 0x23, &p);
    // EMR_RESTOREDC (0x22): relative=-1
    push_record(&mut b, 0x22, &(-1i32).to_le_bytes());
    push_eof(&mut b);

    let recs = parse_emf(&b).expect("parse");
    assert!(matches!(recs[1], Record::SaveDC));
    match &recs[2] {
        Record::SetWorldTransform(x) => {
            assert!((x.m11 - 2.0).abs() < 1e-6);
            assert!((x.m22 - 3.0).abs() < 1e-6);
            assert!((x.dx  - 10.0).abs() < 1e-6);
            assert!((x.dy  - 20.0).abs() < 1e-6);
        }
        other => panic!("expected SetWorldTransform, got {other:?}"),
    }
    assert!(matches!(recs[3], Record::RestoreDC { relative: -1 }));
}

#[test]
fn parses_window_viewport_and_colors() {
    let mut b = header_prefix();
    // EMR_SETWINDOWEXTEX (0x09): SizeL (cx=100, cy=200)
    let mut p = Vec::new();
    p.extend_from_slice(&100i32.to_le_bytes());
    p.extend_from_slice(&200i32.to_le_bytes());
    push_record(&mut b, 0x09, &p);
    // EMR_SETVIEWPORTORGEX (0x0C): PointL (x=5, y=6)
    let mut p = Vec::new();
    p.extend_from_slice(&5i32.to_le_bytes());
    p.extend_from_slice(&6i32.to_le_bytes());
    push_record(&mut b, 0x0C, &p);
    // EMR_SETTEXTCOLOR (0x18): 0x00ABCDEF
    push_record(&mut b, 0x18, &0x00ABCDEFu32.to_le_bytes());
    // EMR_SETBKMODE (0x12): 1=transparent
    push_record(&mut b, 0x12, &1u32.to_le_bytes());
    push_eof(&mut b);

    let recs = parse_emf(&b).expect("parse");
    match &recs[1] {
        Record::SetWindowExtEx(s) => { assert_eq!(s.cx, 100); assert_eq!(s.cy, 200); }
        other => panic!("expected SetWindowExtEx, got {other:?}"),
    }
    match &recs[2] {
        Record::SetViewportOrgEx(p) => { assert_eq!(p.x, 5); assert_eq!(p.y, 6); }
        other => panic!("expected SetViewportOrgEx, got {other:?}"),
    }
    assert!(matches!(recs[3], Record::SetTextColor(0x00ABCDEF)));
    assert!(matches!(recs[4], Record::SetBkMode(1)));
}

#[test]
fn dc_stack_save_restore_round_trip() {
    use super::converter::{DcStack};

    let mut dc = DcStack::new();
    assert_eq!(dc.depth(), 0);
    dc.current_mut().text_color = 0x111111;
    dc.save();
    assert_eq!(dc.depth(), 1);
    dc.current_mut().text_color = 0x222222;
    dc.save();
    dc.current_mut().text_color = 0x333333;
    // Pop 1 — returns to state after first save (text_color=0x222222)
    assert!(dc.restore(-1));
    assert_eq!(dc.current().text_color, 0x222222);
    assert!(dc.restore(-1));
    assert_eq!(dc.current().text_color, 0x111111);
    assert!(!dc.restore(-1));   // 스택 비었으므로 실패
}

#[test]
fn object_table_insert_get_remove() {
    use super::converter::{GraphicsObject, ObjectTable};
    use super::parser::objects::LogPen;

    let mut table = ObjectTable::new();
    table.insert(1, GraphicsObject::Pen(LogPen {
        style: 0, width: 2, _reserved: 0, color: 0x00FF0000,
    }));
    assert!(table.get(1).is_some());
    assert_eq!(table.len(), 1);
    table.remove(1);
    assert!(table.get(1).is_none());
    assert!(table.is_empty());
}

#[test]
fn preserves_unknown_records_as_payload() {
    // Header + Unknown(type=0x00000054 = ExtTextOutW, 단계 13에서 분기) + EOF
    let mut b = fixture_minimal_header_eof();
    // Insert unknown BEFORE eof. fixture_minimal_header_eof는 88 + 20 = 108 바이트.
    // EOF 구간(마지막 20바이트)을 잘라내고, Unknown(16B) + EOF(20B) 재조립.
    let eof = b.split_off(88);
    b.extend_from_slice(&0x00000054u32.to_le_bytes());    // Type=ExtTextOutW
    b.extend_from_slice(&16u32.to_le_bytes());            // Size=16
    b.extend_from_slice(&0xDEADBEEFu32.to_le_bytes());    // 페이로드
    b.extend_from_slice(&0xCAFEBABEu32.to_le_bytes());
    b.extend_from_slice(&eof);

    let records = parse_emf(&b).expect("parse");
    assert_eq!(records.len(), 3);
    match &records[1] {
        Record::Unknown { record_type, payload } => {
            assert_eq!(*record_type, 0x00000054);
            assert_eq!(payload.len(), 8);
        }
        other => panic!("expected Unknown, got {other:?}"),
    }
}
