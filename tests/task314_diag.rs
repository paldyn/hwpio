//! Task #314 진단 테스트: HWPX 직접 vs 어댑터 변환 후 IR 비교
//! 임시 진단 도구. 작업 완료 후 제거.

use rhwp::document_core::DocumentCore;

fn load_sample(name: &str) -> Vec<u8> {
    let path = format!("samples/hwpx/{}", name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("샘플 로드 실패 {}: {}", path, e))
}

#[test]
#[ignore = "diagnostic tool"]
fn diag_hwpx_h_02_page_items() {
    let bytes = load_sample("hwpx-h-02.hwpx");
    let direct = DocumentCore::from_bytes(&bytes).expect("HWPX");
    let mut a = DocumentCore::from_bytes(&bytes).expect("HWPX2");
    let hwp_bytes = a.export_hwp_with_adapter().expect("export");
    let reloaded = DocumentCore::from_bytes(&hwp_bytes).expect("reload");

    let dd = direct.dump_page_items(None);
    let rd = reloaded.dump_page_items(None);

    // 페이지별 첫/마지막 paragraph index 추출
    fn extract_page_summary(s: &str) -> Vec<String> {
        let mut result = Vec::new();
        let mut current_page = String::new();
        let mut last_pi = String::new();
        let mut first_pi = String::new();
        let mut item_count = 0u32;
        for line in s.lines() {
            if line.starts_with("=== 페이지") {
                if !current_page.is_empty() {
                    result.push(format!("{}: items={} first={} last={}", current_page, item_count, first_pi, last_pi));
                }
                current_page = line.to_string();
                first_pi.clear();
                last_pi.clear();
                item_count = 0;
            } else if line.contains("pi=") {
                item_count += 1;
                if let Some(start) = line.find("pi=") {
                    let rest = &line[start..];
                    let end = rest.find(' ').unwrap_or(rest.len());
                    let pi = &rest[..end];
                    if first_pi.is_empty() { first_pi = pi.to_string(); }
                    last_pi = pi.to_string();
                }
            }
        }
        if !current_page.is_empty() {
            result.push(format!("{}: items={} first={} last={}", current_page, item_count, first_pi, last_pi));
        }
        result
    }

    let direct_summary = extract_page_summary(&dd);
    let reloaded_summary = extract_page_summary(&rd);

    eprintln!("=== Page summary comparison ===");
    let n = direct_summary.len().max(reloaded_summary.len());
    for i in 0..n {
        let d = direct_summary.get(i).map(|s| s.as_str()).unwrap_or("(none)");
        let r = reloaded_summary.get(i).map(|s| s.as_str()).unwrap_or("(none)");
        let mark = if d != r { " <<<" } else { "" };
        eprintln!("D: {}", d);
        eprintln!("R: {}{}", r, mark);
        eprintln!();
    }

    // 페이지 3 직접 비교
    eprintln!("\n=== 페이지 3 (global_idx=2) direct vs reloaded ===");
    let dd_p3 = direct.dump_page_items(Some(2));
    let rd_p3 = reloaded.dump_page_items(Some(2));
    eprintln!("--- direct ---\n{}", dd_p3);
    eprintln!("--- reloaded ---\n{}", rd_p3);
}

#[test]
#[ignore = "diagnostic tool"]
fn diag_hwpx_h_02_full_field_diff() {
    let bytes = load_sample("hwpx-h-02.hwpx");
    let direct = DocumentCore::from_bytes(&bytes).expect("HWPX");
    let mut a = DocumentCore::from_bytes(&bytes).expect("HWPX2");
    let hwp_bytes = a.export_hwp_with_adapter().expect("export");
    let reloaded = DocumentCore::from_bytes(&hwp_bytes).expect("reload");

    let mut counts: std::collections::BTreeMap<&str, u32> = Default::default();
    let mut samples: std::collections::BTreeMap<&str, Vec<(usize, usize)>> = Default::default();

    for (sec_idx, (ds, rs)) in direct.document().sections.iter()
        .zip(reloaded.document().sections.iter()).enumerate()
    {
        let pn = ds.paragraphs.len().min(rs.paragraphs.len());
        for pi in 0..pn {
            let d = &ds.paragraphs[pi];
            let r = &rs.paragraphs[pi];
            macro_rules! check {
                ($field:ident, $name:expr) => {
                    if d.$field != r.$field {
                        *counts.entry($name).or_insert(0) += 1;
                        samples.entry($name).or_default().push((sec_idx, pi));
                    }
                };
            }
            check!(char_count, "char_count");
            check!(control_mask, "control_mask");
            check!(para_shape_id, "para_shape_id");
            check!(style_id, "style_id");
            check!(column_type, "column_type");
            check!(raw_break_type, "raw_break_type");
            check!(text, "text");
            check!(char_offsets, "char_offsets");
            if d.range_tags.len() != r.range_tags.len() {
                *counts.entry("range_tags_len").or_insert(0) += 1;
            }
            if d.field_ranges.len() != r.field_ranges.len() {
                *counts.entry("field_ranges_len").or_insert(0) += 1;
            }
            check!(char_count_msb, "char_count_msb");
            check!(raw_header_extra, "raw_header_extra");
            check!(has_para_text, "has_para_text");
            check!(tab_extended, "tab_extended");
            // controls는 어댑터가 의도적으로 추가
            if d.controls.len() != r.controls.len() {
                *counts.entry("controls_len").or_insert(0) += 1;
            }
            if d.char_shapes.len() != r.char_shapes.len() {
                *counts.entry("char_shapes_len").or_insert(0) += 1;
            }
            if d.line_segs.len() != r.line_segs.len() {
                *counts.entry("line_segs_len").or_insert(0) += 1;
            }
        }
    }

    eprintln!("=== Field diff counts ===");
    for (k, v) in &counts {
        eprintln!("  {}: {}", k, v);
    }
    eprintln!("\n=== Sample (first 3) ===");
    for (k, vs) in &samples {
        eprintln!("  {}: {:?}", k, vs.iter().take(3).collect::<Vec<_>>());
    }

    // raw_break_type 상세
    eprintln!("\n=== raw_break_type 차이 상세 ===");
    for (sec_idx, (ds, rs)) in direct.document().sections.iter()
        .zip(reloaded.document().sections.iter()).enumerate()
    {
        let pn = ds.paragraphs.len().min(rs.paragraphs.len());
        for pi in 0..pn {
            let d = &ds.paragraphs[pi];
            let r = &rs.paragraphs[pi];
            if d.raw_break_type != r.raw_break_type || d.column_type != r.column_type {
                eprintln!("  {}.{}: raw d=0x{:02X} r=0x{:02X}, type d={:?} r={:?}",
                    sec_idx, pi, d.raw_break_type, r.raw_break_type, d.column_type, r.column_type);
            }
        }
    }

    // control_mask 상세 (처음 5건)
    eprintln!("\n=== control_mask 차이 상세 (처음 5건) ===");
    let mut shown = 0;
    for (sec_idx, (ds, rs)) in direct.document().sections.iter()
        .zip(reloaded.document().sections.iter()).enumerate()
    {
        if shown >= 5 { break; }
        let pn = ds.paragraphs.len().min(rs.paragraphs.len());
        for pi in 0..pn {
            if shown >= 5 { break; }
            let d = &ds.paragraphs[pi];
            let r = &rs.paragraphs[pi];
            if d.control_mask != r.control_mask {
                eprintln!("  {}.{}: cm d=0x{:08X} r=0x{:08X}, controls.len d={} r={}",
                    sec_idx, pi, d.control_mask, r.control_mask,
                    d.controls.len(), r.controls.len());
                shown += 1;
            }
        }
    }
}

#[test]
#[ignore = "diagnostic tool"]
fn diag_hwpx_h_02_pre_serialize() {
    let bytes = load_sample("hwpx-h-02.hwpx");

    // 어댑터 적용 후 (직렬화 전) IR
    let mut a = DocumentCore::from_bytes(&bytes).expect("HWPX 로드");
    let _hwp_bytes = a.export_hwp_with_adapter().expect("어댑터 export");

    // 같은 a를 (이미 어댑터 적용됨) 재로드한 R 와 비교
    // export_hwp_with_adapter는 IR을 변경하므로 a.document() 가 어댑터 후 IR
    let mut empty_count = 0;
    for sec in &a.document().sections {
        for p in &sec.paragraphs {
            if p.char_shapes.is_empty() {
                empty_count += 1;
            }
        }
    }
    eprintln!("어댑터 적용 후 (직렬화 전) char_shapes 빈 paragraph 수: {}", empty_count);

    // 같은 bytes 다시 로드 (어댑터 미적용)
    let direct = DocumentCore::from_bytes(&bytes).expect("HWPX 직접 로드");
    let mut empty_direct = 0;
    for sec in direct.document().sections.iter() {
        for p in &sec.paragraphs {
            if p.char_shapes.is_empty() {
                empty_direct += 1;
            }
        }
    }
    eprintln!("HWPX 직접 로드 char_shapes 빈 paragraph 수: {}", empty_direct);

    // 재로드 후 IR
    let reloaded = DocumentCore::from_bytes(&_hwp_bytes).expect("HWP 재로드");
    let mut empty_reload = 0;
    for sec in reloaded.document().sections.iter() {
        for p in &sec.paragraphs {
            if p.char_shapes.is_empty() {
                empty_reload += 1;
            }
        }
    }
    eprintln!("HWP 재로드 후 char_shapes 빈 paragraph 수: {}", empty_reload);
}

#[test]
#[ignore = "diagnostic tool"]
fn diag_hwpx_h_02_compare() {
    let bytes = load_sample("hwpx-h-02.hwpx");

    // (1) HWPX 직접 로드
    let direct = DocumentCore::from_bytes(&bytes).expect("HWPX 직접 로드");
    let direct_pages = direct.page_count();

    // (2) HWPX → 어댑터 → HWP → 재로드
    let mut a = DocumentCore::from_bytes(&bytes).expect("HWPX 로드");
    let hwp_bytes = a.export_hwp_with_adapter().expect("어댑터 export");
    let reloaded = DocumentCore::from_bytes(&hwp_bytes).expect("HWP 재로드");
    let reloaded_pages = reloaded.page_count();

    eprintln!("=== Page Counts ===");
    eprintln!("direct: {} pages", direct_pages);
    eprintln!("after adapter+reload: {} pages", reloaded_pages);

    // 섹션/문단/줄 메트릭 비교
    eprintln!("\n=== Section/Paragraph counts ===");
    eprintln!(
        "direct sections={}, reloaded sections={}",
        direct.document().sections.len(),
        reloaded.document().sections.len()
    );

    let mut total_para_diff = 0;
    let mut total_lineseg_diff_count = 0;
    let mut first_diff_paragraph: Option<(usize, usize)> = None;

    for (sec_idx, (ds, rs)) in direct
        .document()
        .sections
        .iter()
        .zip(reloaded.document().sections.iter())
        .enumerate()
    {
        let dp = ds.paragraphs.len();
        let rp = rs.paragraphs.len();
        eprintln!("section {}: direct paras={}, reloaded paras={}", sec_idx, dp, rp);
        if dp != rp {
            total_para_diff += (dp as i64 - rp as i64).abs() as usize;
        }

        let pn = dp.min(rp);
        for pi in 0..pn {
            let dpara = &ds.paragraphs[pi];
            let rpara = &rs.paragraphs[pi];

            if dpara.line_segs.len() != rpara.line_segs.len() {
                total_lineseg_diff_count += 1;
                if first_diff_paragraph.is_none() {
                    first_diff_paragraph = Some((sec_idx, pi));
                }
                eprintln!(
                    "  para {}.{}: line_segs direct={}, reloaded={} (text='{}')",
                    sec_idx,
                    pi,
                    dpara.line_segs.len(),
                    rpara.line_segs.len(),
                    dpara.text.chars().take(30).collect::<String>()
                );
            } else {
                // 같은 라인 수: 필드 차이 검사
                for (li, (dseg, rseg)) in dpara.line_segs.iter().zip(rpara.line_segs.iter()).enumerate() {
                    if dseg.vertical_pos != rseg.vertical_pos
                        || dseg.line_height != rseg.line_height
                        || dseg.line_spacing != rseg.line_spacing
                        || dseg.text_height != rseg.text_height
                    {
                        if first_diff_paragraph.is_none() {
                            first_diff_paragraph = Some((sec_idx, pi));
                        }
                        if total_lineseg_diff_count < 20 {
                            eprintln!(
                                "  para {}.{} line {}: vpos d={}/r={} lh d={}/r={} ls d={}/r={} th d={}/r={}",
                                sec_idx, pi, li,
                                dseg.vertical_pos, rseg.vertical_pos,
                                dseg.line_height, rseg.line_height,
                                dseg.line_spacing, rseg.line_spacing,
                                dseg.text_height, rseg.text_height,
                            );
                        }
                        total_lineseg_diff_count += 1;
                    }
                }
            }
        }
    }

    eprintln!("\n=== Summary ===");
    eprintln!("paragraph count diff total: {}", total_para_diff);
    eprintln!("line_seg diff count: {}", total_lineseg_diff_count);
    eprintln!("first diff paragraph: {:?}", first_diff_paragraph);

    // ========== 확장 비교: controls, char_shapes, text, paragraph props ==========
    let mut text_diff = 0;
    let mut controls_count_diff = 0;
    let mut char_shapes_diff = 0;
    let mut para_props_diff = 0;
    let mut first_other_diff: Option<(usize, usize, &str)> = None;

    for (sec_idx, (ds, rs)) in direct.document().sections.iter()
        .zip(reloaded.document().sections.iter()).enumerate()
    {
        let pn = ds.paragraphs.len().min(rs.paragraphs.len());
        for pi in 0..pn {
            let d = &ds.paragraphs[pi];
            let r = &rs.paragraphs[pi];
            if d.text != r.text {
                text_diff += 1;
                if first_other_diff.is_none() { first_other_diff = Some((sec_idx, pi, "text")); }
            }
            if d.controls.len() != r.controls.len() {
                controls_count_diff += 1;
                if first_other_diff.is_none() { first_other_diff = Some((sec_idx, pi, "controls_count")); }
                eprintln!("  para {}.{} controls: direct={}, reloaded={}",
                    sec_idx, pi, d.controls.len(), r.controls.len());
            }
            if d.char_shapes.len() != r.char_shapes.len() {
                char_shapes_diff += 1;
                if first_other_diff.is_none() { first_other_diff = Some((sec_idx, pi, "char_shapes")); }
            }
            if d.para_shape_id != r.para_shape_id {
                para_props_diff += 1;
                if first_other_diff.is_none() { first_other_diff = Some((sec_idx, pi, "para_shape_id")); }
            }
        }
    }
    eprintln!("\n=== Extended diff ===");
    eprintln!("text_diff: {}", text_diff);
    eprintln!("controls_count_diff: {}", controls_count_diff);
    eprintln!("char_shapes_diff: {}", char_shapes_diff);
    eprintln!("para_props_diff: {}", para_props_diff);
    eprintln!("first other diff: {:?}", first_other_diff);

    // ========== char_shapes 상세 ==========
    eprintln!("\n=== char_shapes 차이 상세 (처음 10건) ===");
    let mut shown = 0;
    for (sec_idx, (ds, rs)) in direct.document().sections.iter()
        .zip(reloaded.document().sections.iter()).enumerate()
    {
        if shown >= 10 { break; }
        let pn = ds.paragraphs.len().min(rs.paragraphs.len());
        for pi in 0..pn {
            if shown >= 10 { break; }
            let d = &ds.paragraphs[pi];
            let r = &rs.paragraphs[pi];
            if d.char_shapes.len() != r.char_shapes.len() {
                eprintln!("  {}.{}: direct n={} {:?}",
                    sec_idx, pi, d.char_shapes.len(),
                    d.char_shapes.iter().take(5).map(|c| (c.start_pos, c.char_shape_id)).collect::<Vec<_>>()
                );
                eprintln!("        reloaded n={} {:?}",
                    r.char_shapes.len(),
                    r.char_shapes.iter().take(5).map(|c| (c.start_pos, c.char_shape_id)).collect::<Vec<_>>()
                );
                shown += 1;
            }
        }
    }
}
