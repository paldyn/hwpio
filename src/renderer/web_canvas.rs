//! Web Canvas 2D 렌더러 (WASM 전용)
//!
//! 브라우저의 Canvas 2D API를 사용하여 HWP 페이지를 렌더링한다.
//! web-sys를 통해 CanvasRenderingContext2d에 직접 그린다.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlImageElement};
#[cfg(target_arch = "wasm32")]
use base64::Engine;

use super::{Renderer, TextStyle, ShapeStyle, LineStyle, PathCommand, StrokeDash, GradientFillInfo, PatternFillInfo};
use crate::model::style::UnderlineType;
use crate::model::style::ImageFillMode;
use super::render_tree::{BoundingBox, FormObjectNode, PageRenderTree, RenderNode, RenderNodeType, ShapeTransform};
use super::composer::{CharOverlapInfo, pua_to_display_text, decode_pua_overlap_number};
use crate::model::control::FormType;
#[cfg(target_arch = "wasm32")]
use super::layout::{compute_char_positions, split_into_clusters};

// 이미지 캐시: data 해시 → HtmlImageElement
// WASM 단일 스레드이므로 thread_local 안전
#[cfg(target_arch = "wasm32")]
thread_local! {
    static IMAGE_CACHE: std::cell::RefCell<std::collections::HashMap<u64, HtmlImageElement>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// 빠른 해시 (FNV-1a 64비트)
#[cfg(target_arch = "wasm32")]
fn hash_bytes(data: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 이미지 MIME 타입 감지
#[cfg(target_arch = "wasm32")]
fn detect_image_mime_type(data: &[u8]) -> &'static str {
    if data.len() >= 8 && &data[0..8] == b"\x89PNG\r\n\x1a\n" {
        "image/png"
    } else if data.len() >= 2 && data[0] == 0xFF && data[1] == 0xD8 {
        "image/jpeg"
    } else if data.len() >= 6 && (&data[0..6] == b"GIF87a" || &data[0..6] == b"GIF89a") {
        "image/gif"
    } else if data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        "image/webp"
    } else if data.len() >= 4 && &data[0..4] == b"\x00\x00\x01\x00" {
        "image/x-icon"
    } else if data.len() >= 2 && &data[0..2] == b"BM" {
        "image/bmp"
    } else if data.len() >= 4 && (data.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A]) || data.starts_with(&[0x01, 0x00, 0x09, 0x00])) {
        "image/x-wmf"
    } else {
        "application/octet-stream"
    }
}

/// 이미지 데이터에서 픽셀 크기(width, height)를 파싱한다.
fn parse_image_dimensions_canvas(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 24 {
        return None;
    }

    // PNG
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        return Some((w, h));
    }

    // JPEG
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let mut i = 2;
        while i + 9 < data.len() {
            if data[i] != 0xFF { i += 1; continue; }
            let marker = data[i + 1];
            if (marker >= 0xC0 && marker <= 0xCF) && marker != 0xC4 && marker != 0xC8 && marker != 0xCC {
                let h = u16::from_be_bytes([data[i + 5], data[i + 6]]) as u32;
                let w = u16::from_be_bytes([data[i + 7], data[i + 8]]) as u32;
                if w > 0 && h > 0 { return Some((w, h)); }
            }
            let seg_len = u16::from_be_bytes([data[i + 2], data[i + 3]]) as usize;
            i += 2 + seg_len;
        }
        return None;
    }

    // GIF
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        let w = u16::from_le_bytes([data[6], data[7]]) as u32;
        let h = u16::from_le_bytes([data[8], data[9]]) as u32;
        return Some((w, h));
    }

    // BMP
    if data.starts_with(&[0x42, 0x4D]) && data.len() >= 26 {
        let w = u32::from_le_bytes([data[18], data[19], data[20], data[21]]);
        let h = i32::from_le_bytes([data[22], data[23], data[24], data[25]]);
        return Some((w, h.unsigned_abs()));
    }

    None
}

/// Web Canvas 2D 렌더러
///
/// web-sys의 CanvasRenderingContext2d를 사용하여 실제 브라우저 Canvas에 렌더링한다.
/// WASM 환경에서만 컴파일된다.
#[cfg(target_arch = "wasm32")]
pub struct WebCanvasRenderer {
    /// Canvas 2D 컨텍스트
    ctx: CanvasRenderingContext2d,
    /// 페이지 폭 (px)
    width: f64,
    /// 페이지 높이 (px)
    height: f64,
    /// 문단부호(¶) 표시 여부
    pub show_paragraph_marks: bool,
    /// 조판부호 표시 여부
    pub show_control_codes: bool,
    /// 줌 스케일 (1.0 = 100%)
    scale: f64,
}

#[cfg(target_arch = "wasm32")]
impl WebCanvasRenderer {
    /// HtmlCanvasElement로부터 렌더러 생성
    pub fn new(canvas: &HtmlCanvasElement) -> Result<Self, JsValue> {
        let ctx = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Failed to get 2d context"))?
            .dyn_into::<CanvasRenderingContext2d>()?;

        Ok(Self {
            ctx,
            width: canvas.width() as f64,
            height: canvas.height() as f64,
            show_paragraph_marks: false,
            show_control_codes: false,
            scale: 1.0,
        })
    }

    /// 줌 스케일 설정 (1.0 = 100%, 2.0 = 200%)
    pub fn set_scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    /// 렌더 트리를 Canvas에 렌더링
    pub fn render_tree(&mut self, tree: &PageRenderTree) {
        self.render_node(&tree.root);
    }

    /// 개별 노드 렌더링
    fn render_node(&mut self, node: &RenderNode) {
        if !node.visible {
            return;
        }

        match &node.node_type {
            RenderNodeType::Page(page) => {
                self.begin_page(page.width, page.height);
            }
            RenderNodeType::PageBackground(bg) => {
                // 배경색
                if let Some(color) = bg.background_color {
                    self.ctx.set_fill_style_str(&color_to_css(color));
                    self.ctx.fill_rect(
                        node.bbox.x, node.bbox.y,
                        node.bbox.width, node.bbox.height,
                    );
                }
                // 그라데이션
                if let Some(grad) = &bg.gradient {
                    if self.apply_gradient_fill(grad, node.bbox.x, node.bbox.y, node.bbox.width, node.bbox.height) {
                        self.ctx.fill_rect(
                            node.bbox.x, node.bbox.y,
                            node.bbox.width, node.bbox.height,
                        );
                    }
                }
                // 이미지 배경
                if let Some(img) = &bg.image {
                    self.draw_image(&img.data, node.bbox.x, node.bbox.y, node.bbox.width, node.bbox.height);
                }
            }
            RenderNodeType::TextRun(run) => {
                // 글자겹침(CharOverlap): 도형 + 텍스트를 Canvas로 렌더링
                if let Some(ref overlap) = run.char_overlap {
                    self.draw_char_overlap(
                        &run.text, &run.style, overlap,
                        node.bbox.x, node.bbox.y, node.bbox.width, node.bbox.height,
                    );
                } else if run.rotation != 0.0 {
                    // 회전 텍스트: bbox 중앙 기준으로 중앙 정렬 후 회전
                    let cx = node.bbox.x + node.bbox.width / 2.0;
                    let cy = node.bbox.y + node.bbox.height / 2.0;
                    // 폰트 설정
                    let font_weight = if run.style.bold { "bold " } else { "" };
                    let font_style_str = if run.style.italic { "italic " } else { "" };
                    let font_size = if run.style.font_size > 0.0 { run.style.font_size } else { 12.0 };
                    let font_family = if run.style.font_family.is_empty() {
                        "sans-serif".to_string()
                    } else {
                        let fallback = super::generic_fallback(&run.style.font_family);
                        format!("\"{}\" , {}", run.style.font_family, fallback)
                    };
                    let font = format!("{}{}{:.3}px {}", font_style_str, font_weight, font_size, font_family);
                    self.ctx.set_font(&font);
                    self.ctx.set_fill_style_str(&color_to_css(run.style.color));
                    self.ctx.save();
                    let _ = self.ctx.translate(cx, cy);
                    let _ = self.ctx.rotate(run.rotation * std::f64::consts::PI / 180.0);
                    // 중앙 정렬로 글리프를 원점에 배치 → 회전 후 bbox 중앙에 위치
                    self.ctx.set_text_align("center");
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&run.text, 0.0, 0.0);
                    self.ctx.restore();
                } else {
                    self.draw_text(
                        &run.text,
                        node.bbox.x,
                        node.bbox.y + run.baseline,
                        &run.style,
                    );
                }
                if self.show_paragraph_marks || self.show_control_codes {
                    let is_marker = !matches!(run.field_marker, crate::renderer::render_tree::FieldMarkerType::None);
                    let font_size = if run.style.font_size > 0.0 { run.style.font_size } else { 12.0 };
                    // 공백·탭 기호 (조판부호 마커는 건너뜀)
                    if !run.text.is_empty() && !is_marker {
                        let char_positions = compute_char_positions(&run.text, &run.style);
                        let mark_font_size = font_size * 0.5;
                        self.ctx.set_fill_style_str("#4A90D9");
                        self.ctx.set_font(&format!("{:.3}px sans-serif", mark_font_size));
                        for (i, c) in run.text.chars().enumerate() {
                            if c == ' ' {
                                let cx = node.bbox.x + char_positions[i];
                                let next_x = if i + 1 < char_positions.len() {
                                    node.bbox.x + char_positions[i + 1]
                                } else {
                                    node.bbox.x + node.bbox.width
                                };
                                let mid_x = (cx + next_x) / 2.0 - mark_font_size * 0.25;
                                let _ = self.ctx.fill_text("\u{2228}", mid_x, node.bbox.y + run.baseline);
                            } else if c == '\t' {
                                let cx = node.bbox.x + char_positions[i];
                                let _ = self.ctx.fill_text("\u{2192}", cx, node.bbox.y + run.baseline);
                            }
                        }
                    }
                    // 하드 리턴·강제 줄바꿈 기호
                    if run.is_para_end || run.is_line_break_end {
                        self.ctx.set_fill_style_str("#4A90D9");
                        self.ctx.set_font(&format!("{:.3}px sans-serif", font_size));
                        if run.is_vertical {
                            let mark_x = node.bbox.x + (node.bbox.width - font_size * 0.5) / 2.0;
                            let mark_y = node.bbox.y + run.baseline + font_size;
                            let cx = mark_x + font_size * 0.25;
                            let cy = mark_y - font_size * 0.5;
                            self.ctx.save();
                            let _ = self.ctx.translate(cx, cy);
                            let _ = self.ctx.rotate(90.0 * std::f64::consts::PI / 180.0);
                            let _ = self.ctx.translate(-cx, -cy);
                            let mark = if run.is_line_break_end { "\u{2193}" } else { "\u{21B5}" };
                            let _ = self.ctx.fill_text(mark, mark_x, mark_y);
                            self.ctx.restore();
                        } else {
                            let mark_x = if run.text.is_empty() { node.bbox.x } else { node.bbox.x + node.bbox.width };
                            let mark_y = node.bbox.y + run.baseline;
                            let mark = if run.is_line_break_end { "\u{2193}" } else { "\u{21B5}" };
                            let _ = self.ctx.fill_text(mark, mark_x, mark_y);
                        }
                    }
                }
            }
            RenderNodeType::Rectangle(rect) => {
                self.open_shape_transform(&rect.transform, &node.bbox);
                self.draw_rect_with_gradient(
                    node.bbox.x, node.bbox.y,
                    node.bbox.width, node.bbox.height,
                    rect.corner_radius,
                    &rect.style,
                    rect.gradient.as_deref(),
                );
            }
            RenderNodeType::Line(line) => {
                self.open_shape_transform(&line.transform, &node.bbox);
                self.draw_line(line.x1, line.y1, line.x2, line.y2, &line.style);
            }
            RenderNodeType::Ellipse(ellipse) => {
                self.open_shape_transform(&ellipse.transform, &node.bbox);
                let cx = node.bbox.x + node.bbox.width / 2.0;
                let cy = node.bbox.y + node.bbox.height / 2.0;
                self.draw_ellipse_with_gradient(
                    cx, cy,
                    node.bbox.width / 2.0, node.bbox.height / 2.0,
                    &ellipse.style,
                    ellipse.gradient.as_deref(),
                );
            }
            RenderNodeType::Image(img) => {
                self.open_shape_transform(&img.transform, &node.bbox);
                if let Some(ref data) = img.data {
                    self.draw_image_with_fill_mode(
                        data, &node.bbox, img.fill_mode, img.original_size, img.crop,
                    );
                }
            }
            RenderNodeType::Path(path) => {
                self.open_shape_transform(&path.transform, &node.bbox);
                self.draw_path_with_gradient(&path.commands, &path.style, path.gradient.as_deref());
                // 연결선 화살표: 경로의 시작/끝 접선 방향 사용
                if let (Some(ref ls), Some((x1, y1, x2, y2))) = (&path.line_style, path.connector_endpoints) {
                    let color = color_to_css(ls.color);
                    let width = ls.width;
                    let cmds = &path.commands;
                    let len = ((x2-x1)*(x2-x1) + (y2-y1)*(y2-y1)).sqrt().max(1.0);
                    // 시작 화살표: 시작점과 다른 첫 번째 점 방향
                    if ls.start_arrow != super::ArrowStyle::None {
                        let (dx, dy) = {
                            let mut found = (x1 - x2, y1 - y2);
                            for cmd in cmds.iter().skip(1) {
                                let (px, py) = match cmd {
                                    super::PathCommand::LineTo(px, py) => (*px, *py),
                                    super::PathCommand::CurveTo(cx, cy, _, _, _, _) => (*cx, *cy),
                                    _ => continue,
                                };
                                if (x1 - px).abs() > 0.5 || (y1 - py).abs() > 0.5 {
                                    found = (x1 - px, y1 - py);
                                    break;
                                }
                            }
                            found
                        };
                        let d = (dx*dx + dy*dy).sqrt().max(0.001);
                        let (aw, ah) = calc_arrow_dims(width, len, ls.start_arrow_size);
                        draw_arrow_head(&self.ctx, x1, y1, dx/d, dy/d, aw, ah, &ls.start_arrow, &color, width);
                    }
                    // 끝 화살표: 끝점과 다른 마지막 점 → 끝점 방향
                    if ls.end_arrow != super::ArrowStyle::None {
                        let (dx, dy) = {
                            let mut pts: Vec<(f64, f64)> = Vec::new();
                            for cmd in cmds.iter() {
                                match cmd {
                                    super::PathCommand::MoveTo(px, py) |
                                    super::PathCommand::LineTo(px, py) => { pts.push((*px, *py)); }
                                    super::PathCommand::CurveTo(_, _, cx, cy, ex, ey) => {
                                        pts.push((*cx, *cy));
                                        pts.push((*ex, *ey));
                                    }
                                    _ => {}
                                }
                            }
                            // 끝점과 다른 점을 역순으로 찾음
                            let mut found = (x2 - x1, y2 - y1);
                            for i in (0..pts.len()).rev() {
                                let ddx = x2 - pts[i].0;
                                let ddy = y2 - pts[i].1;
                                if ddx.abs() > 0.5 || ddy.abs() > 0.5 {
                                    found = (x2 - pts[i].0, y2 - pts[i].1);
                                    break;
                                }
                            }
                            found
                        };
                        let d = (dx*dx + dy*dy).sqrt().max(0.001);
                        let (aw, ah) = calc_arrow_dims(width, len, ls.end_arrow_size);
                        draw_arrow_head(&self.ctx, x2, y2, dx/d, dy/d, aw, ah, &ls.end_arrow, &color, width);
                    }
                }
            }
            RenderNodeType::Body { clip_rect: Some(cr) } => {
                self.ctx.save();
                self.ctx.begin_path();
                // 우측 여유: 레이아웃 메트릭과 브라우저 글리프 폭 차이 흡수
                self.ctx.rect(cr.x, cr.y, cr.width + 4.0, cr.height);
                self.ctx.clip();
            }
            RenderNodeType::TableCell(ref tc) if tc.clip => {
                self.ctx.save();
                self.ctx.begin_path();
                // 셀 우측 여유: 레이아웃 반올림 오차로 마지막 글리프 잘림 방지
                self.ctx.rect(node.bbox.x, node.bbox.y, node.bbox.width + 4.0, node.bbox.height);
                self.ctx.clip();
            }
            RenderNodeType::Equation(eq) => {
                self.ctx.save();
                super::equation::canvas_render::render_equation_canvas(
                    &self.ctx,
                    &eq.layout_box,
                    node.bbox.x,
                    node.bbox.y,
                    &eq.color_str,
                    eq.font_size,
                );
                self.ctx.restore();
            }
            RenderNodeType::FormObject(form) => {
                self.render_form_object(form, &node.bbox);
            }
            RenderNodeType::FootnoteMarker(marker) => {
                // 위첨자 렌더링: 작은 글씨 + 위로 올림
                let sup_size = (marker.base_font_size * 0.55).max(7.0);
                let font = format!("{:.1}px {}", sup_size, marker.font_family);
                self.ctx.set_font(&font);
                self.ctx.set_fill_style_str(&color_to_css(marker.color));
                // 위첨자 y: bbox 상단 + baseline의 40% (일반 텍스트 ~80%보다 높음)
                let y = node.bbox.y + node.bbox.height * 0.4;
                let _ = self.ctx.fill_text(&marker.text, node.bbox.x, y);
            }
            RenderNodeType::Placeholder(ph) => {
                // 차트/OLE placeholder — svg.rs 와 동등 출력 (점선 테두리 + 중앙 라벨)
                let x = node.bbox.x;
                let y = node.bbox.y;
                let w = node.bbox.width;
                let h = node.bbox.height;
                // 배경 rect
                self.ctx.set_fill_style_str(&color_to_css(ph.fill_color));
                self.ctx.fill_rect(x, y, w, h);
                // 점선 테두리 (6 3)
                self.set_line_dash(&StrokeDash::Dash);
                self.ctx.set_stroke_style_str(&color_to_css(ph.stroke_color));
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(x, y, w, h);
                let _ = self.ctx.set_line_dash(&js_sys::Array::new());
                // 중앙 라벨 (svg.rs 와 동일한 font_size 공식)
                let font_size = (w.min(h) * 0.06).clamp(12.0, 28.0);
                self.ctx.set_font(&format!("{:.1}px sans-serif", font_size));
                self.ctx.set_fill_style_str(&color_to_css(ph.stroke_color));
                self.ctx.set_text_align("center");
                self.ctx.set_text_baseline("middle");
                let _ = self.ctx.fill_text(&ph.label, x + w / 2.0, y + h / 2.0);
                // 텍스트 정렬 기본값 복원 (다른 노드에 영향 주지 않도록)
                self.ctx.set_text_align("start");
                self.ctx.set_text_baseline("alphabetic");
            }
            _ => {
                // 구조 노드(Header, Footer, Column 등)는 자식만 렌더링
            }
        }

        // 자식 노드 재귀 렌더링
        for child in &node.children {
            self.render_node(child);
        }

        // 도형 변환 상태 복원
        self.close_shape_transform(&node.node_type);

        // 조판부호 개체 마커 (붉은색 대괄호)
        if self.show_control_codes {
            let label = match &node.node_type {
                RenderNodeType::Table(_) => Some("[표]"),
                RenderNodeType::Image(_) => Some("[그림]"),
                RenderNodeType::TextBox => Some("[글상자]"),
                RenderNodeType::Equation(_) => Some("[수식]"),
                RenderNodeType::Header => Some("[머리말]"),
                RenderNodeType::Footer => Some("[꼬리말]"),
                RenderNodeType::FootnoteArea => Some("[각주]"),
                _ => None,
            };
            if let Some(label) = label {
                let fs = 10.0;
                self.ctx.set_fill_style_str("#CC3333");
                self.ctx.set_font(&format!("{:.3}px sans-serif", fs));
                let _ = self.ctx.fill_text(label, node.bbox.x, node.bbox.y + fs);
            }
        }

        // 셀 클리핑 상태 복원
        if matches!(&node.node_type, RenderNodeType::TableCell(tc) if tc.clip) {
            self.ctx.restore();
        }

        // Body 클리핑 상태 복원 + 오버플로우 컨트롤 재렌더링
        if matches!(node.node_type, RenderNodeType::Body { clip_rect: Some(_) }) {
            self.ctx.restore();
            // 편집 모드: 여백을 벗어난 도형/이미지/표를 재렌더링 (좌우 넘침 허용)
            if let RenderNodeType::Body { clip_rect: Some(ref cr) } = node.node_type {
                self.render_overflow_controls(node, cr);
            }
        }
    }

    /// 도형 변환(회전/대칭)이 있으면 ctx.save() + translate/rotate/scale을 적용한다.
    fn open_shape_transform(&self, transform: &ShapeTransform, bbox: &BoundingBox) {
        if !transform.has_transform() {
            return;
        }
        let cx = bbox.x + bbox.width / 2.0;
        let cy = bbox.y + bbox.height / 2.0;
        self.ctx.save();
        // 중심으로 이동 → 대칭 → 회전 → 원래 위치로
        let _ = self.ctx.translate(cx, cy);
        let sx = if transform.horz_flip { -1.0 } else { 1.0 };
        let sy = if transform.vert_flip { -1.0 } else { 1.0 };
        let _ = self.ctx.scale(sx, sy);
        if transform.rotation != 0.0 {
            let _ = self.ctx.rotate(transform.rotation * std::f64::consts::PI / 180.0);
        }
        let _ = self.ctx.translate(-cx, -cy);
    }

    /// 도형 변환 상태를 복원한다 (open_shape_transform에 대응).
    fn close_shape_transform(&self, node_type: &RenderNodeType) {
        let transform = match node_type {
            RenderNodeType::Rectangle(r) => &r.transform,
            RenderNodeType::Line(l) => &l.transform,
            RenderNodeType::Ellipse(e) => &e.transform,
            RenderNodeType::Image(i) => &i.transform,
            RenderNodeType::Path(p) => &p.transform,
            _ => return,
        };
        if transform.has_transform() {
            self.ctx.restore();
        }
    }

    /// 본문 영역(body_area)을 좌우로 벗어나는 도형/이미지/표를 재렌더링한다.
    /// 편집 모드에서 여백 바깥 컨트롤이 보이도록 하되, 텍스트는 여백 내부로 유지한다.
    fn render_overflow_controls(&mut self, body_node: &RenderNode, body_clip: &BoundingBox) {
        let body_left = body_clip.x;
        let body_right = body_clip.x + body_clip.width;

        // 오버플로우 컨트롤 존재 여부 빠른 확인
        let has_overflow = body_node.children.iter().any(|col| {
            col.children.iter().any(|child| {
                Self::is_overflow_control(child, body_left, body_right)
            })
        });
        if !has_overflow { return; }

        // 상하만 본문 영역 클리핑 (좌우 전폭)
        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.rect(0.0, body_clip.y, self.width, body_clip.height);
        self.ctx.clip();

        for col in &body_node.children {
            for child in &col.children {
                if Self::is_overflow_control(child, body_left, body_right) {
                    self.render_node(child);
                }
            }
        }

        self.ctx.restore();
    }

    /// 본문 영역을 좌우로 벗어나는 컨트롤(비-텍스트)인지 판별한다.
    fn is_overflow_control(node: &RenderNode, body_left: f64, body_right: f64) -> bool {
        // 텍스트 라인·구조 노드는 제외
        match node.node_type {
            RenderNodeType::TextLine(_)
            | RenderNodeType::Column(_)
            | RenderNodeType::FootnoteArea
            | RenderNodeType::Header
            | RenderNodeType::Footer
            | RenderNodeType::MasterPage
            | RenderNodeType::Page(_)
            | RenderNodeType::Body { .. } => return false,
            _ => {}
        }
        // 본문 영역 좌우 경계를 벗어나는지 확인
        node.bbox.x < body_left || node.bbox.x + node.bbox.width > body_right
    }

    /// 선 대시 패턴 설정
    fn set_line_dash(&self, dash: &StrokeDash) {
        let pattern: js_sys::Array = match dash {
            StrokeDash::Solid => js_sys::Array::new(),
            StrokeDash::Dash => {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_f64(6.0));
                arr.push(&JsValue::from_f64(3.0));
                arr
            }
            StrokeDash::Dot => {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_f64(2.0));
                arr.push(&JsValue::from_f64(2.0));
                arr
            }
            StrokeDash::DashDot => {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_f64(6.0));
                arr.push(&JsValue::from_f64(3.0));
                arr.push(&JsValue::from_f64(2.0));
                arr.push(&JsValue::from_f64(3.0));
                arr
            }
            StrokeDash::DashDotDot => {
                let arr = js_sys::Array::new();
                arr.push(&JsValue::from_f64(6.0));
                arr.push(&JsValue::from_f64(3.0));
                arr.push(&JsValue::from_f64(2.0));
                arr.push(&JsValue::from_f64(3.0));
                arr.push(&JsValue::from_f64(2.0));
                arr.push(&JsValue::from_f64(3.0));
                arr
            }
        };
        let _ = self.ctx.set_line_dash(&pattern);
    }

    /// HWP 각도(도) → Canvas linearGradient 좌표 변환
    /// 사각형 (x, y, w, h) 기준으로 (x0, y0, x1, y1) 반환
    fn angle_to_canvas_coords(angle: i16, x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
        let a = ((angle % 360 + 360) % 360) as f64;
        match a as i32 {
            0 => (x, y, x, y + h),
            45 => (x, y, x + w, y + h),
            90 => (x, y, x + w, y),
            135 => (x, y + h, x + w, y),
            180 => (x, y + h, x, y),
            225 => (x + w, y + h, x, y),
            270 => (x + w, y, x, y),
            315 => (x + w, y, x, y + h),
            _ => {
                let rad = a.to_radians();
                let sin_a = rad.sin();
                let cos_a = rad.cos();
                let cx = x + w / 2.0;
                let cy = y + h / 2.0;
                (cx - sin_a * w / 2.0, cy - cos_a * h / 2.0,
                 cx + sin_a * w / 2.0, cy + cos_a * h / 2.0)
            }
        }
    }

    /// PatternFillInfo → Canvas createPattern으로 패턴 채우기 적용
    /// 오프스크린 캔버스에 6×6 타일 생성 후 반복 패턴으로 설정
    /// 반환값: true이면 패턴이 적용됨
    fn apply_pattern_fill(&self, info: &PatternFillInfo) -> bool {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return false,
        };
        let document = match window.document() {
            Some(d) => d,
            None => return false,
        };

        // 오프스크린 캔버스 생성 (6×6 타일)
        let tile_canvas = match document.create_element("canvas") {
            Ok(el) => match el.dyn_into::<HtmlCanvasElement>() {
                Ok(c) => c,
                Err(_) => return false,
            },
            Err(_) => return false,
        };
        let sz: u32 = 6;
        tile_canvas.set_width(sz);
        tile_canvas.set_height(sz);

        let tile_ctx = match tile_canvas.get_context("2d") {
            Ok(Some(ctx)) => match ctx.dyn_into::<CanvasRenderingContext2d>() {
                Ok(c) => c,
                Err(_) => return false,
            },
            _ => return false,
        };

        let bg = color_to_css(info.background_color);
        let fg = color_to_css(info.pattern_color);
        let s = sz as f64;

        // 배경 채우기
        tile_ctx.set_fill_style_str(&bg);
        tile_ctx.fill_rect(0.0, 0.0, s, s);

        // 패턴 선 그리기
        tile_ctx.set_stroke_style_str(&fg);
        tile_ctx.set_line_width(1.0);

        match info.pattern_type {
            0 => {
                // 가로줄 (- - - -)
                tile_ctx.begin_path();
                tile_ctx.move_to(0.0, 3.0);
                tile_ctx.line_to(s, 3.0);
                tile_ctx.stroke();
            }
            1 => {
                // 세로줄 (|||||)
                tile_ctx.begin_path();
                tile_ctx.move_to(3.0, 0.0);
                tile_ctx.line_to(3.0, s);
                tile_ctx.stroke();
            }
            2 => {
                // 대각선 (/////)
                tile_ctx.begin_path();
                tile_ctx.move_to(s, 0.0);
                tile_ctx.line_to(0.0, s);
                tile_ctx.stroke();
            }
            3 => {
                // 역대각선 (\\\\\)
                tile_ctx.begin_path();
                tile_ctx.move_to(0.0, 0.0);
                tile_ctx.line_to(s, s);
                tile_ctx.stroke();
            }
            4 => {
                // 십자 (+++++)
                tile_ctx.begin_path();
                tile_ctx.move_to(3.0, 0.0);
                tile_ctx.line_to(3.0, s);
                tile_ctx.stroke();
                tile_ctx.begin_path();
                tile_ctx.move_to(0.0, 3.0);
                tile_ctx.line_to(s, 3.0);
                tile_ctx.stroke();
            }
            5 => {
                // 격자 (xxxxx)
                tile_ctx.begin_path();
                tile_ctx.move_to(0.0, 0.0);
                tile_ctx.line_to(s, s);
                tile_ctx.stroke();
                tile_ctx.begin_path();
                tile_ctx.move_to(s, 0.0);
                tile_ctx.line_to(0.0, s);
                tile_ctx.stroke();
            }
            _ => {
                // 알 수 없는 패턴: 배경색만 (이미 채움)
            }
        }

        // createPattern으로 반복 패턴 생성
        match self.ctx.create_pattern_with_html_canvas_element(&tile_canvas, "repeat") {
            Ok(Some(pattern)) => {
                self.ctx.set_fill_style_canvas_pattern(&pattern);
                true
            }
            _ => false,
        }
    }

    /// GradientFillInfo → Canvas CanvasGradient 생성 및 fillStyle 설정
    /// 반환값: true이면 gradient가 적용됨
    fn apply_gradient_fill(&self, grad: &GradientFillInfo, x: f64, y: f64, w: f64, h: f64) -> bool {
        if grad.colors.len() < 2 {
            return false;
        }

        let canvas_grad = match grad.gradient_type {
            2 | 3 | 4 => {
                // Radial / Conical / Square → radialGradient
                let cx = x + w * (grad.center_x as f64 / 100.0);
                let cy = y + h * (grad.center_y as f64 / 100.0);
                let r = w.max(h) / 2.0;
                match self.ctx.create_radial_gradient(cx, cy, 0.0, cx, cy, r) {
                    Ok(g) => g,
                    Err(_) => return false,
                }
            }
            _ => {
                // Linear (1 또는 기본값)
                let (x0, y0, x1, y1) = Self::angle_to_canvas_coords(grad.angle, x, y, w, h);
                self.ctx.create_linear_gradient(x0, y0, x1, y1)
            }
        };

        // 색상 스톱 추가 (positions는 이미 0.0~1.0으로 정규화됨)
        for (i, &color) in grad.colors.iter().enumerate() {
            let offset = if i < grad.positions.len() {
                grad.positions[i] as f32
            } else {
                i as f32 / (grad.colors.len().max(2) - 1).max(1) as f32
            };
            let _ = canvas_grad.add_color_stop(offset, &color_to_css(color));
        }

        self.ctx.set_fill_style_canvas_gradient(&canvas_grad);
        true
    }

    /// 그라데이션을 포함한 사각형 그리기
    fn draw_rect_with_gradient(&mut self, x: f64, y: f64, w: f64, h: f64, corner_radius: f64, style: &ShapeStyle, gradient: Option<&GradientFillInfo>) {
        let need_opacity = style.opacity < 1.0;
        if need_opacity {
            self.ctx.save();
            self.ctx.set_global_alpha(style.opacity);
        }
        // 그림자는 fill에만 적용 (stroke 전에 해제)
        self.apply_shadow(style);

        if corner_radius > 0.0 {
            self.ctx.begin_path();
            let r = corner_radius.min(w / 2.0).min(h / 2.0);
            self.ctx.move_to(x + r, y);
            self.ctx.line_to(x + w - r, y);
            self.ctx.arc_to(x + w, y, x + w, y + r, r).ok();
            self.ctx.line_to(x + w, y + h - r);
            self.ctx.arc_to(x + w, y + h, x + w - r, y + h, r).ok();
            self.ctx.line_to(x + r, y + h);
            self.ctx.arc_to(x, y + h, x, y + h - r, r).ok();
            self.ctx.line_to(x, y + r);
            self.ctx.arc_to(x, y, x + r, y, r).ok();
            self.ctx.close_path();
            if let Some(grad) = gradient {
                if !self.apply_gradient_fill(grad, x, y, w, h) {
                    if let Some(fill) = style.fill_color {
                        self.ctx.set_fill_style_str(&color_to_css(fill));
                    }
                }
                self.ctx.fill();
            } else if let Some(ref pat) = style.pattern {
                if !self.apply_pattern_fill(pat) {
                    if let Some(fill) = style.fill_color {
                        self.ctx.set_fill_style_str(&color_to_css(fill));
                    }
                }
                self.ctx.fill();
            } else if let Some(fill) = style.fill_color {
                self.ctx.set_fill_style_str(&color_to_css(fill));
                self.ctx.fill();
            } else if style.shadow.is_some() {
                // 채우기 없어도 그림자용 투명 fill
                self.ctx.set_fill_style_str("rgba(255,255,255,0.01)");
                self.ctx.fill();
            }
            self.clear_shadow(style); // stroke 전에 그림자 해제
            if let Some(stroke) = style.stroke_color {
                self.ctx.set_stroke_style_str(&color_to_css(stroke));
                self.ctx.set_line_width(style.stroke_width.max(0.5));
                self.set_line_dash(&style.stroke_dash);
                self.ctx.stroke();
                let _ = self.ctx.set_line_dash(&js_sys::Array::new());
            }
        } else {
            if let Some(grad) = gradient {
                if self.apply_gradient_fill(grad, x, y, w, h) {
                    self.ctx.fill_rect(x, y, w, h);
                }
            } else if let Some(ref pat) = style.pattern {
                if self.apply_pattern_fill(pat) {
                    self.ctx.fill_rect(x, y, w, h);
                }
            } else if let Some(fill) = style.fill_color {
                self.ctx.set_fill_style_str(&color_to_css(fill));
                self.ctx.fill_rect(x, y, w, h);
            } else if style.shadow.is_some() {
                // 채우기 없어도 그림자용 투명 fill
                self.ctx.set_fill_style_str("rgba(255,255,255,0.01)");
                self.ctx.fill_rect(x, y, w, h);
            }
            self.clear_shadow(style); // stroke 전에 그림자 해제
            if let Some(stroke) = style.stroke_color {
                self.ctx.set_stroke_style_str(&color_to_css(stroke));
                self.ctx.set_line_width(style.stroke_width.max(0.5));
                self.set_line_dash(&style.stroke_dash);
                self.ctx.stroke_rect(x, y, w, h);
                let _ = self.ctx.set_line_dash(&js_sys::Array::new());
            }
        }

        if need_opacity {
            self.ctx.restore();
        }
    }

    /// 그라데이션을 포함한 타원 그리기
    fn draw_ellipse_with_gradient(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, style: &ShapeStyle, gradient: Option<&GradientFillInfo>) {
        self.apply_shadow(style);
        self.ctx.begin_path();
        let _ = self.ctx.ellipse(cx, cy, rx.abs(), ry.abs(), 0.0, 0.0, std::f64::consts::TAU);

        if let Some(grad) = gradient {
            let x = cx - rx;
            let y = cy - ry;
            if !self.apply_gradient_fill(grad, x, y, rx * 2.0, ry * 2.0) {
                if let Some(fill) = style.fill_color {
                    self.ctx.set_fill_style_str(&color_to_css(fill));
                }
            }
            self.ctx.fill();
        } else if let Some(ref pat) = style.pattern {
            if !self.apply_pattern_fill(pat) {
                if let Some(fill) = style.fill_color {
                    self.ctx.set_fill_style_str(&color_to_css(fill));
                }
            }
            self.ctx.fill();
        } else if let Some(fill) = style.fill_color {
            self.ctx.set_fill_style_str(&color_to_css(fill));
            self.ctx.fill();
        }

        if let Some(stroke) = style.stroke_color {
            self.ctx.set_stroke_style_str(&color_to_css(stroke));
            self.ctx.set_line_width(style.stroke_width.max(0.5));
            self.set_line_dash(&style.stroke_dash);
            self.ctx.stroke();
            let _ = self.ctx.set_line_dash(&js_sys::Array::new());
        }
        self.clear_shadow(style);
    }

    /// 그라데이션을 포함한 패스 그리기
    fn draw_path_with_gradient(&mut self, commands: &[PathCommand], style: &ShapeStyle, gradient: Option<&GradientFillInfo>) {
        self.apply_shadow(style);
        self.ctx.begin_path();
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        // ArcTo 변환을 위해 현재 경로 위치 추적
        let mut cur_x = 0.0_f64;
        let mut cur_y = 0.0_f64;

        for cmd in commands {
            match cmd {
                PathCommand::MoveTo(x, y) => {
                    self.ctx.move_to(*x, *y);
                    cur_x = *x; cur_y = *y;
                    min_x = min_x.min(*x); min_y = min_y.min(*y);
                    max_x = max_x.max(*x); max_y = max_y.max(*y);
                }
                PathCommand::LineTo(x, y) => {
                    self.ctx.line_to(*x, *y);
                    cur_x = *x; cur_y = *y;
                    min_x = min_x.min(*x); min_y = min_y.min(*y);
                    max_x = max_x.max(*x); max_y = max_y.max(*y);
                }
                PathCommand::CurveTo(cp1x, cp1y, cp2x, cp2y, x, y) => {
                    self.ctx.bezier_curve_to(*cp1x, *cp1y, *cp2x, *cp2y, *x, *y);
                    cur_x = *x; cur_y = *y;
                    min_x = min_x.min(*x); min_y = min_y.min(*y);
                    max_x = max_x.max(*x); max_y = max_y.max(*y);
                }
                PathCommand::ArcTo(rx, ry, x_rot, large_arc, sweep, x, y) => {
                    // SVG arc → cubic bezier 변환
                    let beziers = super::svg_arc_to_beziers(
                        cur_x, cur_y, *rx, *ry, *x_rot,
                        *large_arc, *sweep, *x, *y,
                    );
                    for bcmd in &beziers {
                        if let PathCommand::CurveTo(cp1x, cp1y, cp2x, cp2y, ex, ey) = bcmd {
                            self.ctx.bezier_curve_to(*cp1x, *cp1y, *cp2x, *cp2y, *ex, *ey);
                            min_x = min_x.min(*ex); min_y = min_y.min(*ey);
                            max_x = max_x.max(*ex); max_y = max_y.max(*ey);
                        } else if let PathCommand::LineTo(lx, ly) = bcmd {
                            self.ctx.line_to(*lx, *ly);
                            min_x = min_x.min(*lx); min_y = min_y.min(*ly);
                            max_x = max_x.max(*lx); max_y = max_y.max(*ly);
                        }
                    }
                    cur_x = *x; cur_y = *y;
                    min_x = min_x.min(*x); min_y = min_y.min(*y);
                    max_x = max_x.max(*x); max_y = max_y.max(*y);
                }
                PathCommand::ClosePath => {
                    self.ctx.close_path();
                }
            }
        }

        if let Some(grad) = gradient {
            let bx = if min_x.is_finite() { min_x } else { 0.0 };
            let by = if min_y.is_finite() { min_y } else { 0.0 };
            let bw = if max_x.is_finite() && min_x.is_finite() { max_x - min_x } else { 100.0 };
            let bh = if max_y.is_finite() && min_y.is_finite() { max_y - min_y } else { 100.0 };
            if !self.apply_gradient_fill(grad, bx, by, bw, bh) {
                if let Some(fill) = style.fill_color {
                    self.ctx.set_fill_style_str(&color_to_css(fill));
                }
            }
            self.ctx.fill();
        } else if let Some(ref pat) = style.pattern {
            if !self.apply_pattern_fill(pat) {
                if let Some(fill) = style.fill_color {
                    self.ctx.set_fill_style_str(&color_to_css(fill));
                }
            }
            self.ctx.fill();
        } else if let Some(fill) = style.fill_color {
            self.ctx.set_fill_style_str(&color_to_css(fill));
            self.ctx.fill();
        }

        if let Some(stroke) = style.stroke_color {
            self.ctx.set_stroke_style_str(&color_to_css(stroke));
            self.ctx.set_line_width(style.stroke_width.max(0.5));
            self.set_line_dash(&style.stroke_dash);
            self.ctx.stroke();
            let _ = self.ctx.set_line_dash(&js_sys::Array::new());
        }

        // 그림자 해제
        if style.shadow.is_some() {
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_offset_x(0.0);
            self.ctx.set_shadow_offset_y(0.0);
            self.ctx.set_shadow_blur(0.0);
        }
    }

    /// 도형 그림자 적용
    fn apply_shadow(&self, style: &ShapeStyle) {
        if let Some(ref shadow) = style.shadow {
            let opacity = if shadow.alpha > 0 { 1.0 - (shadow.alpha as f64 / 255.0) } else { 1.0 };
            let r = (shadow.color >> 0) & 0xFF;
            let g = (shadow.color >> 8) & 0xFF;
            let b = (shadow.color >> 16) & 0xFF;
            let color = format!("rgba({},{},{},{:.2})", r, g, b, opacity);
            self.ctx.set_shadow_color(&color);
            self.ctx.set_shadow_offset_x(shadow.offset_x);
            self.ctx.set_shadow_offset_y(shadow.offset_y);
            self.ctx.set_shadow_blur(2.0);
        }
    }

    /// 도형 그림자 해제
    fn clear_shadow(&self, style: &ShapeStyle) {
        if style.shadow.is_some() {
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_offset_x(0.0);
            self.ctx.set_shadow_offset_y(0.0);
            self.ctx.set_shadow_blur(0.0);
        }
    }

    fn render_form_object(&self, form: &FormObjectNode, bbox: &super::render_tree::BoundingBox) {
        let x = bbox.x;
        let y = bbox.y;
        let w = bbox.width;
        let h = bbox.height;

        match form.form_type {
            FormType::PushButton => {
                // 명령 단추 (웹 환경 비활성 — 회색 스타일)
                self.ctx.set_fill_style_str("#d0d0d0");
                self.ctx.fill_rect(x, y, w, h);
                self.ctx.set_stroke_style_str("#a0a0a0");
                self.ctx.set_line_width(0.5);
                self.ctx.stroke_rect(x, y, w, h);
                // 캡션 텍스트 (회색)
                if !form.caption.is_empty() {
                    let font_size = (h * 0.5).min(12.0).max(8.0);
                    self.ctx.set_font(&format!("{}px sans-serif", font_size));
                    self.ctx.set_fill_style_str("#808080");
                    self.ctx.set_text_align("center");
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&form.caption, x + w / 2.0, y + h / 2.0);
                    self.ctx.set_text_align("left");
                    self.ctx.set_text_baseline("alphabetic");
                }
            }
            FormType::CheckBox => {
                let box_size = h.min(14.0);
                let box_y = y + (h - box_size) / 2.0;
                // 체크박스 사각형
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_rect(x, box_y, box_size, box_size);
                self.ctx.set_stroke_style_str("#000000");
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(x, box_y, box_size, box_size);
                // 체크 표시
                if form.value != 0 {
                    self.ctx.set_stroke_style_str("#000000");
                    self.ctx.set_line_width(2.0);
                    self.ctx.begin_path();
                    self.ctx.move_to(x + 2.0, box_y + box_size / 2.0);
                    self.ctx.line_to(x + box_size / 3.0, box_y + box_size - 3.0);
                    self.ctx.line_to(x + box_size - 2.0, box_y + 2.0);
                    self.ctx.stroke();
                    self.ctx.set_line_width(1.0);
                }
                // 캡션
                if !form.caption.is_empty() {
                    let font_size = (h * 0.7).min(12.0).max(8.0);
                    self.ctx.set_font(&format!("{}px sans-serif", font_size));
                    self.ctx.set_fill_style_str(&form.fore_color);
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&form.caption, x + box_size + 4.0, y + h / 2.0);
                    self.ctx.set_text_baseline("alphabetic");
                }
            }
            FormType::RadioButton => {
                let r = h.min(14.0) / 2.0;
                let cx = x + r;
                let cy = y + h / 2.0;
                // 원형 배경
                self.ctx.begin_path();
                let _ = self.ctx.arc(cx, cy, r, 0.0, std::f64::consts::TAU);
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill();
                self.ctx.set_stroke_style_str("#000000");
                self.ctx.set_line_width(1.0);
                self.ctx.stroke();
                // 선택 표시
                if form.value != 0 {
                    self.ctx.begin_path();
                    let _ = self.ctx.arc(cx, cy, r * 0.5, 0.0, std::f64::consts::TAU);
                    self.ctx.set_fill_style_str("#000000");
                    self.ctx.fill();
                }
                // 캡션
                if !form.caption.is_empty() {
                    let font_size = (h * 0.7).min(12.0).max(8.0);
                    self.ctx.set_font(&format!("{}px sans-serif", font_size));
                    self.ctx.set_fill_style_str(&form.fore_color);
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&form.caption, x + r * 2.0 + 4.0, y + h / 2.0);
                    self.ctx.set_text_baseline("alphabetic");
                }
            }
            FormType::ComboBox => {
                let btn_w = h.min(20.0);
                // 입력 영역
                self.ctx.set_fill_style_str("#ffffff");
                self.ctx.fill_rect(x, y, w - btn_w, h);
                self.ctx.set_stroke_style_str("#808080");
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(x, y, w - btn_w, h);
                // 텍스트
                if !form.text.is_empty() {
                    let font_size = (h * 0.6).min(12.0).max(8.0);
                    self.ctx.set_font(&format!("{}px sans-serif", font_size));
                    self.ctx.set_fill_style_str(&form.fore_color);
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&form.text, x + 2.0, y + h / 2.0);
                    self.ctx.set_text_baseline("alphabetic");
                }
                // 드롭다운 버튼
                let bx = x + w - btn_w;
                self.ctx.set_fill_style_str("#c0c0c0");
                self.ctx.fill_rect(bx, y, btn_w, h);
                self.ctx.set_stroke_style_str("#808080");
                self.ctx.stroke_rect(bx, y, btn_w, h);
                // ▼ 삼각형
                self.ctx.begin_path();
                let tri_cx = bx + btn_w / 2.0;
                let tri_cy = y + h / 2.0;
                let tri_s = btn_w * 0.3;
                self.ctx.move_to(tri_cx - tri_s, tri_cy - tri_s / 2.0);
                self.ctx.line_to(tri_cx + tri_s, tri_cy - tri_s / 2.0);
                self.ctx.line_to(tri_cx, tri_cy + tri_s / 2.0);
                self.ctx.close_path();
                self.ctx.set_fill_style_str("#000000");
                self.ctx.fill();
            }
            FormType::Edit => {
                // 입력 영역
                self.ctx.set_fill_style_str(&form.back_color);
                self.ctx.fill_rect(x, y, w, h);
                self.ctx.set_stroke_style_str("#808080");
                self.ctx.set_line_width(1.0);
                self.ctx.stroke_rect(x, y, w, h);
                // 텍스트
                if !form.text.is_empty() {
                    let font_size = (h * 0.6).min(12.0).max(8.0);
                    self.ctx.set_font(&format!("{}px sans-serif", font_size));
                    self.ctx.set_fill_style_str(&form.fore_color);
                    self.ctx.set_text_baseline("middle");
                    let _ = self.ctx.fill_text(&form.text, x + 2.0, y + h / 2.0);
                    self.ctx.set_text_baseline("alphabetic");
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl Renderer for WebCanvasRenderer {
    fn begin_page(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
        // 줌 스케일 적용: 렌더트리 좌표(문서 단위)를 캔버스 해상도에 맞게 확대
        if self.scale != 1.0 {
            let _ = self.ctx.scale(self.scale, self.scale);
        }
        // 캔버스 초기화 (흰색 배경)
        self.ctx.set_fill_style_str("#ffffff");
        self.ctx.fill_rect(0.0, 0.0, width, height);
    }

    fn end_page(&mut self) {
        // Canvas는 특별한 종료 처리 없음
    }

    fn draw_text(&mut self, text: &str, x: f64, y: f64, style: &TextStyle) {
        // PUA 문자(U+F000~F0FF, Wingdings 등 심볼 폰트)를 유니코드 표준 문자로 변환
        let text = &text.chars().map(|ch| {
            crate::renderer::layout::map_pua_bullet_char(ch)
        }).collect::<String>();

        // 글꼴 설정
        let font_weight = if style.bold { "bold " } else { "" };
        let font_style = if style.italic { "italic " } else { "" };
        let base_font_size = if style.font_size > 0.0 { style.font_size } else { 12.0 };

        // 위첨자/아래첨자: 글꼴 크기 축소 + y좌표 조정
        let (font_size, y) = if style.superscript {
            (base_font_size * 0.7, y - base_font_size * 0.3)
        } else if style.subscript {
            (base_font_size * 0.7, y + base_font_size * 0.15)
        } else {
            (base_font_size, y)
        };

        let font_family = if style.font_family.is_empty() {
            "sans-serif".to_string()
        } else {
            let fallback = super::generic_fallback(&style.font_family);
            format!("\"{}\", {}", style.font_family, fallback)
        };

        let font = format!("{}{}{:.3}px {}", font_style, font_weight, font_size, font_family);
        self.ctx.set_font(&font);

        // 장평 적용
        let ratio = if style.ratio > 0.0 { style.ratio } else { 1.0 };
        let has_ratio = (ratio - 1.0).abs() > 0.01;

        // 클러스터 분할
        let clusters = split_into_clusters(text);

        // 레이아웃 메트릭 기준으로 글자 위치 계산 (줄바꿈 결정과 동일한 메트릭 사용)
        let char_positions = compute_char_positions(text, style);

        // 형광펜 배경 (CharShape.shade_color 기반 — 편집기에서 적용한 형광펜)
        let shade_rgb = style.shade_color & 0x00FFFFFF;
        if shade_rgb != 0x00FFFFFF && shade_rgb != 0 {
            let text_width = *char_positions.last().unwrap_or(&0.0);
            if text_width > 0.0 {
                self.ctx.set_fill_style_str(&color_to_css(style.shade_color));
                self.ctx.fill_rect(x, y - font_size, text_width, font_size * 1.2);
            }
        }

        let has_effect = style.outline_type > 0 || style.shadow_type > 0
            || style.emboss || style.engrave;

        if has_effect {
            self.draw_text_with_effects(
                &clusters, &char_positions, x, y, style, font_size, ratio, has_ratio,
            );
        } else {
            // 기본 렌더링 (효과 없음)
            self.ctx.set_fill_style_str(&color_to_css(style.color));
            for (char_idx, cluster_str) in &clusters {
                if cluster_str == " " || cluster_str == "\t" || cluster_str == "\u{2007}" { continue; }
                // XML/HTML 무효 제어문자 건너뜀 (SVG의 escape_xml과 동일)
                if cluster_str.starts_with(|c: char| c < '\u{0020}' && !matches!(c, '\t' | '\n' | '\r')) { continue; }
                let char_x = x + char_positions[*char_idx];

                let ch = cluster_str.chars().next().unwrap_or(' ');

                // 통화 기호 등 글리프 미포함 문자: 폴백 폰트로 임시 전환
                let needs_font_fallback = matches!(ch,
                    '\u{20A9}' | '\u{20AC}' | '\u{00A3}' | '\u{00A5}' // ₩€£¥
                );
                if needs_font_fallback {
                    self.ctx.save();
                    let fallback_font = format!("{}{}{:.3}px 'Malgun Gothic','맑은 고딕',sans-serif",
                        if style.italic { "italic " } else { "" },
                        if style.bold { "bold " } else { "" },
                        font_size);
                    self.ctx.set_font(&fallback_font);
                    let _ = self.ctx.fill_text(cluster_str, char_x, y);
                    self.ctx.restore();
                    self.ctx.set_font(&font); // 원래 폰트 복원
                    continue;
                }

                // 반각 강제 구두점: 폰트 글리프가 전각이지만 반각 공간에 배치
                let needs_halfwidth_scale = matches!(ch,
                    '\u{2018}'..='\u{2027}' | '\u{00B7}'
                ) && !has_ratio;

                if needs_halfwidth_scale {
                    self.ctx.save();
                    self.ctx.translate(char_x, y).unwrap_or(());
                    self.ctx.scale(0.5, 1.0).unwrap_or(());
                    let _ = self.ctx.fill_text(cluster_str, 0.0, 0.0);
                    self.ctx.restore();
                } else if has_ratio {
                    self.ctx.save();
                    self.ctx.translate(char_x, y).unwrap_or(());
                    self.ctx.scale(ratio, 1.0).unwrap_or(());
                    let _ = self.ctx.fill_text(cluster_str, 0.0, 0.0);
                    self.ctx.restore();
                } else {
                    let _ = self.ctx.fill_text(cluster_str, char_x, y);
                }
            }
        }

        // 밑줄 처리
        if !matches!(style.underline, UnderlineType::None) {
            let text_width = *char_positions.last().unwrap_or(&0.0);
            let ul_color = if style.underline_color != 0 {
                color_to_css(style.underline_color)
            } else {
                color_to_css(style.color)
            };
            let ul_y = match style.underline {
                UnderlineType::Top => y - font_size + 1.0,
                _ => y + 2.0,
            };
            self.draw_line_shape_canvas(x, ul_y, x + text_width, ul_y, &ul_color, style.underline_shape);
        }

        // 취소선 처리
        if style.strikethrough {
            let text_width = *char_positions.last().unwrap_or(&0.0);
            let strike_y = y - font_size * 0.3;
            let st_color = if style.strike_color != 0 {
                color_to_css(style.strike_color)
            } else {
                color_to_css(style.color)
            };
            self.draw_line_shape_canvas(x, strike_y, x + text_width, strike_y, &st_color, style.strike_shape);
        }

        // 강조점 처리
        if style.emphasis_dot > 0 {
            let dot_char = match style.emphasis_dot {
                1 => "●", 2 => "○", 3 => "ˇ", 4 => "˜", 5 => "･", 6 => "˸", _ => "",
            };
            if !dot_char.is_empty() {
                let dot_size = font_size * 0.3;
                let dot_y = y - font_size * 1.05;
                self.ctx.save();
                self.ctx.set_font(&format!("{}px sans-serif", dot_size));
                self.ctx.set_text_align("center");
                self.ctx.set_fill_style_str(&color_to_css(style.color));
                for &cx in &char_positions[..char_positions.len().saturating_sub(1)] {
                    let dot_x = x + cx + (font_size * style.ratio * 0.5);
                    self.ctx.fill_text(dot_char, dot_x, dot_y).ok();
                }
                self.ctx.restore();
            }
        }

        // 탭 리더(채울 모양) 렌더링 — 12종
        // 0=없음, 1=실선, 2=파선, 3=점선, 4=일점쇄선, 5=이점쇄선,
        // 6=긴파선, 7=원형점선, 8=이중실선, 9=얇고굵은이중선,
        // 10=굵고얇은이중선, 11=얇고굵고얇은삼중선
        for leader in &style.tab_leaders {
            if leader.fill_type == 0 { continue; }
            let lx1 = x + leader.start_x;
            let lx2 = x + leader.end_x;
            let ly = y - font_size * 0.35; // 글자 세로 중앙
            let stroke_color = color_to_css(style.color);

            let draw_line = |ctx: &web_sys::CanvasRenderingContext2d, y: f64, width: f64, dash: &[f64]| {
                let arr = js_sys::Array::new();
                for &d in dash { arr.push(&JsValue::from(d)); }
                let _ = ctx.set_line_dash(&arr);
                ctx.set_line_width(width);
                ctx.begin_path();
                ctx.move_to(lx1, y);
                ctx.line_to(lx2, y);
                ctx.stroke();
            };

            self.ctx.set_stroke_style_str(&stroke_color);
            match leader.fill_type {
                1 => draw_line(&self.ctx, ly, 0.5, &[]),                     // 실선
                2 => draw_line(&self.ctx, ly, 0.5, &[3.0, 3.0]),             // 파선
                3 => draw_line(&self.ctx, ly, 0.5, &[1.0, 2.0]),             // 점선
                4 => draw_line(&self.ctx, ly, 0.5, &[6.0, 2.0, 1.0, 2.0]),   // 일점쇄선
                5 => draw_line(&self.ctx, ly, 0.5, &[6.0, 2.0, 1.0, 2.0, 1.0, 2.0]), // 이점쇄선
                6 => draw_line(&self.ctx, ly, 0.5, &[8.0, 4.0]),             // 긴파선
                7 => {
                    // 원형점선 ●●●
                    self.ctx.set_line_cap("round");
                    draw_line(&self.ctx, ly, 0.7, &[0.1, 2.5]);
                    self.ctx.set_line_cap("butt");
                }
                8 => {
                    // 이중실선
                    draw_line(&self.ctx, ly - 1.0, 0.3, &[]);
                    draw_line(&self.ctx, ly + 1.0, 0.3, &[]);
                }
                9 => {
                    // 얇고 굵은 이중선
                    draw_line(&self.ctx, ly - 1.2, 0.3, &[]);
                    draw_line(&self.ctx, ly + 0.8, 0.8, &[]);
                }
                10 => {
                    // 굵고 얇은 이중선
                    draw_line(&self.ctx, ly - 0.8, 0.8, &[]);
                    draw_line(&self.ctx, ly + 1.2, 0.3, &[]);
                }
                11 => {
                    // 얇고 굵고 얇은 삼중선
                    draw_line(&self.ctx, ly - 2.0, 0.3, &[]);
                    draw_line(&self.ctx, ly, 0.8, &[]);
                    draw_line(&self.ctx, ly + 2.0, 0.3, &[]);
                }
                _ => draw_line(&self.ctx, ly, 0.5, &[1.0, 2.0]),             // 폴백: 점선
            }
            let _ = self.ctx.set_line_dash(&js_sys::Array::new());
        }
    }

    fn draw_rect(&mut self, x: f64, y: f64, w: f64, h: f64, corner_radius: f64, style: &ShapeStyle) {
        self.draw_rect_with_gradient(x, y, w, h, corner_radius, style, None);
    }

    fn draw_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, style: &LineStyle) {
        let color = color_to_css(style.color);
        let width = style.width.max(0.5);
        let dx = x2 - x1;
        let dy = y2 - y1;
        let line_len = (dx * dx + dy * dy).sqrt();

        let mut lx1 = x1;
        let mut ly1 = y1;
        let mut lx2 = x2;
        let mut ly2 = y2;

        if line_len > 0.0 {
            let ux = dx / line_len;
            let uy = dy / line_len;

            if style.start_arrow != super::ArrowStyle::None {
                let (arrow_w, arrow_h) = calc_arrow_dims(width, line_len, style.start_arrow_size);
                draw_arrow_head(&self.ctx, x1, y1, -ux, -uy, arrow_w, arrow_h, &style.start_arrow, &color, width);
                lx1 += ux * arrow_w;
                ly1 += uy * arrow_w;
            }
            if style.end_arrow != super::ArrowStyle::None {
                let (arrow_w, arrow_h) = calc_arrow_dims(width, line_len, style.end_arrow_size);
                draw_arrow_head(&self.ctx, x2, y2, ux, uy, arrow_w, arrow_h, &style.end_arrow, &color, width);
                lx2 -= ux * arrow_w;
                ly2 -= uy * arrow_w;
            }
        }

        // 그림자
        if let Some(ref shadow) = style.shadow {
            let opacity = if shadow.alpha > 0 { 1.0 - (shadow.alpha as f64 / 255.0) } else { 1.0 };
            let r = (shadow.color >> 0) & 0xFF;
            let g = (shadow.color >> 8) & 0xFF;
            let b = (shadow.color >> 16) & 0xFF;
            self.ctx.set_shadow_color(&format!("rgba({},{},{},{:.2})", r, g, b, opacity));
            self.ctx.set_shadow_offset_x(shadow.offset_x);
            self.ctx.set_shadow_offset_y(shadow.offset_y);
            self.ctx.set_shadow_blur(2.0);
        }

        self.ctx.set_stroke_style_str(&color);
        self.set_line_dash(&style.dash);

        // 이중선/삼중선: SVG draw_multi_line과 동일한 오프셋 비율 방식
        // (width_ratio, offset_ratio) — offset은 선 중심으로부터의 거리 비율
        match style.line_type {
            super::LineRenderType::Double |
            super::LineRenderType::ThickThinDouble |
            super::LineRenderType::ThinThickDouble |
            super::LineRenderType::ThinThickThinTriple => {
                let lines: Vec<(f64, f64)> = match style.line_type {
                    super::LineRenderType::Double => {
                        vec![(0.30, -0.35), (0.30, 0.35)]
                    }
                    super::LineRenderType::ThickThinDouble => {
                        // 굵은선(위)-얇은선(아래)
                        vec![(0.4, -0.30), (0.2, 0.40)]
                    }
                    super::LineRenderType::ThinThickDouble => {
                        // 얇은선(위)-굵은선(아래)
                        vec![(0.2, -0.40), (0.4, 0.30)]
                    }
                    super::LineRenderType::ThinThickThinTriple => {
                        vec![(0.15, -0.425), (0.30, 0.0), (0.15, 0.425)]
                    }
                    _ => vec![],
                };

                let (nx, ny) = if line_len > 0.0 {
                    (-dy / line_len, dx / line_len)
                } else {
                    (0.0, 1.0)
                };

                for (width_ratio, offset_ratio) in &lines {
                    let lw = (width * width_ratio).max(0.3);
                    let off = width * offset_ratio;
                    let ox = nx * off;
                    let oy = ny * off;
                    self.ctx.set_line_width(lw);
                    self.ctx.begin_path();
                    self.ctx.move_to(lx1 + ox, ly1 + oy);
                    self.ctx.line_to(lx2 + ox, ly2 + oy);
                    self.ctx.stroke();
                }
            }
            _ => {
                // Single line
                self.ctx.set_line_width(width);
                self.ctx.begin_path();
                self.ctx.move_to(lx1, ly1);
                self.ctx.line_to(lx2, ly2);
                self.ctx.stroke();
            }
        }

        let _ = self.ctx.set_line_dash(&js_sys::Array::new());

        // 그림자 해제
        if style.shadow.is_some() {
            self.ctx.set_shadow_color("transparent");
            self.ctx.set_shadow_offset_x(0.0);
            self.ctx.set_shadow_offset_y(0.0);
            self.ctx.set_shadow_blur(0.0);
        }
    }

    fn draw_ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, style: &ShapeStyle) {
        self.draw_ellipse_with_gradient(cx, cy, rx, ry, style, None);
    }

    fn draw_image(&mut self, data: &[u8], x: f64, y: f64, w: f64, h: f64) {
        let key = hash_bytes(data);

        // 캐시에서 이미 로드된 이미지를 찾는다
        let cached = IMAGE_CACHE.with(|cache| {
            let c = cache.borrow();
            c.get(&key).cloned()
        });

        if let Some(img) = cached {
            if img.complete() && img.natural_width() > 0 {
                let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                    &img, x, y, w, h,
                );
                return;
            }
        }

        // 캐시 미스: 새 HtmlImageElement 생성
        let mime_type = detect_image_mime_type(data);

        // WMF → SVG 변환 (브라우저는 WMF를 렌더링할 수 없으므로 SVG로 변환)
        let (render_data, render_mime): (std::borrow::Cow<[u8]>, &str) = if mime_type == "image/x-wmf" {
            match crate::renderer::svg::convert_wmf_to_svg(data) {
                Some(svg_bytes) => (std::borrow::Cow::Owned(svg_bytes), "image/svg+xml"),
                None => (std::borrow::Cow::Borrowed(data), mime_type),
            }
        } else {
            (std::borrow::Cow::Borrowed(data), mime_type)
        };

        // Base64 인코딩 및 data URL 생성
        let base64_data = base64::engine::general_purpose::STANDARD.encode(&*render_data);
        let data_url = format!("data:{};base64,{}", render_mime, base64_data);

        if let Ok(img) = HtmlImageElement::new() {
            img.set_src(&data_url);

            // 캐시에 저장 (로드 전이라도 저장 — 다음 렌더링에서 재사용)
            IMAGE_CACHE.with(|cache| {
                let mut c = cache.borrow_mut();
                // 캐시 크기 제한 (최대 200개)
                if c.len() > 200 {
                    c.clear();
                }
                c.insert(key, img.clone());
            });

            // 이미지가 즉시 사용 가능하면 그리기
            if img.complete() && img.natural_width() > 0 {
                let _ = self.ctx.draw_image_with_html_image_element_and_dw_and_dh(
                    &img, x, y, w, h,
                );
            }
            // 아직 로드되지 않은 경우: 캐시에 저장되었으므로
            // 재렌더링 시 캐시에서 로드 완료된 이미지를 즉시 사용한다.
        } else {
            // Image 생성 실패 시 플레이스홀더
            self.ctx.set_fill_style_str("#eeeeee");
            self.ctx.fill_rect(x, y, w, h);
            self.ctx.set_stroke_style_str("#cccccc");
            self.ctx.stroke_rect(x, y, w, h);
        }
    }

    fn draw_path(&mut self, commands: &[PathCommand], style: &ShapeStyle) {
        self.draw_path_with_gradient(commands, style, None);
    }
}

#[cfg(target_arch = "wasm32")]
impl WebCanvasRenderer {
    /// crop 영역만 표시하는 drawImage (9인자 버전)
    fn draw_image_cropped(&mut self, data: &[u8],
        sx: f64, sy: f64, sw: f64, sh: f64,
        dx: f64, dy: f64, dw: f64, dh: f64,
    ) {
        let key = hash_bytes(data);

        let cached = IMAGE_CACHE.with(|cache| {
            let c = cache.borrow();
            c.get(&key).cloned()
        });

        if let Some(img) = cached {
            if img.complete() && img.natural_width() > 0 {
                let _ = self.ctx.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img, sx, sy, sw, sh, dx, dy, dw, dh,
                );
                return;
            }
        }

        // 캐시 미스: draw_image로 로드 시작 (다음 렌더에서 crop 적용)
        self.draw_image(data, dx, dy, dw, dh);
    }

    /// 텍스트 변형 효과 렌더링 (외곽선/그림자/양각/음각)
    fn draw_text_with_effects(
        &self,
        clusters: &[(usize, String)],
        char_positions: &[f64],
        x: f64, y: f64,
        style: &TextStyle,
        font_size: f64,
        ratio: f64,
        has_ratio: bool,
    ) {
        let text_color_css = color_to_css(style.color);

        // 클러스터 단위로 fill/stroke 하는 헬퍼 클로저
        let render_pass = |ctx: &web_sys::CanvasRenderingContext2d,
                           dx: f64, dy: f64,
                           fill_color: &str,
                           stroke: bool, stroke_color: &str, line_width: f64| {
            ctx.set_fill_style_str(fill_color);
            if stroke {
                ctx.set_stroke_style_str(stroke_color);
                ctx.set_line_width(line_width);
            }
            for (char_idx, cluster_str) in clusters {
                let cs: &str = cluster_str;
                if cs == " " || cs == "\t" || cs == "\u{2007}" { continue; }
                if cs.starts_with(|c: char| c < '\u{0020}' && !matches!(c, '\t' | '\n' | '\r')) { continue; }
                let char_x = x + char_positions[*char_idx] + dx;
                let char_y = y + dy;

                if has_ratio {
                    ctx.save();
                    ctx.translate(char_x, char_y).unwrap_or(());
                    ctx.scale(ratio, 1.0).unwrap_or(());
                    let _ = ctx.fill_text(cs, 0.0, 0.0);
                    if stroke { let _ = ctx.stroke_text(cs, 0.0, 0.0); }
                    ctx.restore();
                } else {
                    let _ = ctx.fill_text(cs, char_x, char_y);
                    if stroke { let _ = ctx.stroke_text(cs, char_x, char_y); }
                }
            }
        };

        // 양각/음각 (상호 배타적, 다른 효과보다 우선)
        if style.emboss || style.engrave {
            let offset = (font_size / 20.0).max(1.0);
            // 양각: ↗밝은색 → ↘어두운색 → 원본
            // 음각: ↗어두운색 → ↘밝은색 → 원본
            let (first_color, second_color) = if style.emboss {
                ("#ffffff", "#808080")
            } else {
                ("#808080", "#ffffff")
            };
            render_pass(&self.ctx, -offset, -offset, first_color, false, "", 0.0);
            render_pass(&self.ctx, offset, offset, second_color, false, "", 0.0);
            render_pass(&self.ctx, 0.0, 0.0, &text_color_css, false, "", 0.0);
            return;
        }

        // 그림자 (원본 아래에 그림자색으로 오프셋 렌더)
        if style.shadow_type > 0 {
            let shadow_css = color_to_css(style.shadow_color);
            let dx = style.shadow_offset_x;
            let dy = style.shadow_offset_y;
            render_pass(&self.ctx, dx, dy, &shadow_css, false, "", 0.0);
        }

        // 외곽선 (fillText(흰색) + strokeText(글자색))
        if style.outline_type > 0 {
            let line_width = (font_size / 25.0).max(0.5);
            render_pass(&self.ctx, 0.0, 0.0, "#ffffff", true, &text_color_css, line_width);
        } else {
            // 일반 텍스트 (그림자 위에 원본)
            render_pass(&self.ctx, 0.0, 0.0, &text_color_css, false, "", 0.0);
        }
    }

    /// 글자겹침(CharOverlap)을 Canvas 2D로 렌더링한다.
    fn draw_char_overlap(
        &mut self, text: &str, style: &TextStyle, overlap: &CharOverlapInfo,
        bbox_x: f64, bbox_y: f64, bbox_w: f64, bbox_h: f64,
    ) {
        let font_size = if style.font_size > 0.0 { style.font_size } else { 12.0 };
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() { return; }

        // PUA 다자리 숫자 디코딩 시도
        if let Some(number_str) = decode_pua_overlap_number(&chars) {
            self.draw_char_overlap_combined(style, overlap, &number_str, bbox_x, bbox_y, bbox_w, bbox_h);
            return;
        }

        // Canvas 상태 보존
        self.ctx.save();

        let box_size = font_size;
        let char_advance = if chars.len() > 1 { bbox_w / chars.len() as f64 } else { box_size };

        let is_reversed = overlap.border_type == 2 || overlap.border_type == 4;
        let is_circle = overlap.border_type == 1 || overlap.border_type == 2;
        let is_rect = overlap.border_type == 3 || overlap.border_type == 4;

        let size_ratio = if overlap.inner_char_size > 0 {
            overlap.inner_char_size as f64 / 100.0
        } else {
            1.0
        };
        let inner_font_size = font_size * size_ratio;

        let fill_color = if is_reversed { "#000000" } else { "none" };
        let stroke_color = "#000000";
        let text_color = if is_reversed {
            "#FFFFFF".to_string()
        } else {
            color_to_css(style.color)
        };

        let font_family = if style.font_family.is_empty() {
            "sans-serif".to_string()
        } else {
            let fallback = super::generic_fallback(&style.font_family);
            format!("\"{}\" , {}", style.font_family, fallback)
        };
        let font_weight = if style.bold { "bold " } else { "" };
        let font_style_str = if style.italic { "italic " } else { "" };
        let font = format!("{}{}{:.3}px {}", font_style_str, font_weight, inner_font_size, font_family);

        for (i, ch) in chars.iter().enumerate() {
            let display_str = {
                let cp = *ch as u32;
                if (0x2460..=0x2473).contains(&cp) {
                    format!("{}", cp - 0x2460 + 1)
                } else if let Some(s) = pua_to_display_text(*ch) {
                    s
                } else {
                    ch.to_string()
                }
            };

            let cx = bbox_x + i as f64 * char_advance + box_size / 2.0;
            let cy = bbox_y + bbox_h - box_size / 2.0;

            if is_circle {
                let r = box_size / 2.0;
                self.ctx.begin_path();
                let _ = self.ctx.arc(cx, cy, r, 0.0, std::f64::consts::PI * 2.0);
                if is_reversed {
                    self.ctx.set_fill_style_str(fill_color);
                    self.ctx.fill();
                }
                self.ctx.set_stroke_style_str(stroke_color);
                self.ctx.set_line_width(0.8);
                self.ctx.stroke();
            } else if is_rect {
                let rx = cx - box_size / 2.0;
                let ry = cy - box_size / 2.0;
                if is_reversed {
                    self.ctx.set_fill_style_str(fill_color);
                    self.ctx.fill_rect(rx, ry, box_size, box_size);
                }
                self.ctx.set_stroke_style_str(stroke_color);
                self.ctx.set_line_width(0.8);
                self.ctx.stroke_rect(rx, ry, box_size, box_size);
            }

            self.ctx.set_font(&font);
            self.ctx.set_fill_style_str(&text_color);
            self.ctx.set_text_align("center");
            self.ctx.set_text_baseline("middle");
            let _ = self.ctx.fill_text(&display_str, cx, cy);
        }

        self.ctx.restore();
    }

    /// PUA 다자리 숫자를 하나의 도형 안에 합쳐서 Canvas 렌더링
    fn draw_char_overlap_combined(
        &mut self, style: &TextStyle, overlap: &CharOverlapInfo,
        number_str: &str, bbox_x: f64, bbox_y: f64, bbox_w: f64, bbox_h: f64,
    ) {
        let font_size = if style.font_size > 0.0 { style.font_size } else { 12.0 };
        let box_size = font_size;

        self.ctx.save();

        let effective_border = if overlap.border_type == 0 { 1u8 } else { overlap.border_type };
        let is_reversed = effective_border == 2 || effective_border == 4;
        let is_circle = effective_border == 1 || effective_border == 2;
        let is_rect = effective_border == 3 || effective_border == 4;

        let size_ratio = if overlap.inner_char_size > 0 {
            overlap.inner_char_size as f64 / 100.0
        } else {
            1.0
        };
        let inner_font_size = font_size * size_ratio;

        let fill_color = if is_reversed { "#000000" } else { "none" };
        let stroke_color = "#000000";
        let text_color = if is_reversed {
            "#FFFFFF".to_string()
        } else {
            color_to_css(style.color)
        };

        let font_family = if style.font_family.is_empty() {
            "sans-serif".to_string()
        } else {
            let fallback = super::generic_fallback(&style.font_family);
            format!("\"{}\" , {}", style.font_family, fallback)
        };

        let cx = bbox_x + box_size / 2.0;
        let cy = bbox_y + bbox_h - box_size / 2.0;

        // 도형 렌더링
        if is_circle {
            let r = box_size / 2.0;
            self.ctx.begin_path();
            let _ = self.ctx.arc(cx, cy, r, 0.0, std::f64::consts::PI * 2.0);
            if is_reversed {
                self.ctx.set_fill_style_str(fill_color);
                self.ctx.fill();
            }
            self.ctx.set_stroke_style_str(stroke_color);
            self.ctx.set_line_width(0.8);
            self.ctx.stroke();
        } else if is_rect {
            let rx = cx - box_size / 2.0;
            let ry = cy - box_size / 2.0;
            if is_reversed {
                self.ctx.set_fill_style_str(fill_color);
                self.ctx.fill_rect(rx, ry, box_size, box_size);
            }
            self.ctx.set_stroke_style_str(stroke_color);
            self.ctx.set_line_width(0.8);
            self.ctx.stroke_rect(rx, ry, box_size, box_size);
        }

        // 장평 조절: 숫자 자릿수에 따라 scaleX로 폭 압축
        let digit_count = number_str.len();
        let scale_x = if digit_count > 1 { 0.7 / digit_count as f64 * 2.0 } else { 1.0 };

        let font_weight = if style.bold { "bold " } else { "" };
        let font_style_str = if style.italic { "italic " } else { "" };
        let font = format!("{}{}{:.3}px {}", font_style_str, font_weight, inner_font_size, font_family);

        self.ctx.set_font(&font);
        self.ctx.set_fill_style_str(&text_color);
        self.ctx.set_text_align("center");
        self.ctx.set_text_baseline("middle");

        // 다자리 숫자는 baseline을 살짝 올려 시각적 중앙 맞춤
        let text_y = cy - font_size * 0.08;
        if scale_x < 1.0 {
            self.ctx.save();
            let _ = self.ctx.translate(cx, text_y);
            let _ = self.ctx.scale(scale_x, 1.0);
            let _ = self.ctx.fill_text(number_str, 0.0, 0.0);
            self.ctx.restore();
        } else {
            let _ = self.ctx.fill_text(number_str, cx, text_y);
        }

        self.ctx.restore();
    }

    /// 선 모양(shape)에 따라 Canvas 라인을 그린다.
    fn draw_line_shape_canvas(&self, x1: f64, y1: f64, x2: f64, y2: f64, color: &str, shape: u8) {
        match shape {
            7 => {
                // 이중선
                self.draw_single_canvas_line(x1, y1 - 1.0, x2, y2 - 1.0, color, 0.7, &[]);
                self.draw_single_canvas_line(x1, y1 + 1.0, x2, y2 + 1.0, color, 0.7, &[]);
            }
            8 => {
                // 가는+굵은 이중선
                self.draw_single_canvas_line(x1, y1 - 1.2, x2, y2 - 1.2, color, 0.5, &[]);
                self.draw_single_canvas_line(x1, y1 + 0.8, x2, y2 + 0.8, color, 1.2, &[]);
            }
            9 => {
                // 굵은+가는 이중선
                self.draw_single_canvas_line(x1, y1 - 0.8, x2, y2 - 0.8, color, 1.2, &[]);
                self.draw_single_canvas_line(x1, y1 + 1.2, x2, y2 + 1.2, color, 0.5, &[]);
            }
            10 => {
                // 삼중선
                self.draw_single_canvas_line(x1, y1 - 1.5, x2, y2 - 1.5, color, 0.5, &[]);
                self.draw_single_canvas_line(x1, y1, x2, y2, color, 0.5, &[]);
                self.draw_single_canvas_line(x1, y1 + 1.5, x2, y2 + 1.5, color, 0.5, &[]);
            }
            11 => {
                // 물결선
                self.draw_wave_canvas(x1, y1, x2, color, 0.7, 1.5, 6.0);
            }
            12 => {
                // 이중물결선
                self.draw_wave_canvas(x1, y1 - 1.0, x2, color, 0.5, 1.2, 6.0);
                self.draw_wave_canvas(x1, y1 + 1.0, x2, color, 0.5, 1.2, 6.0);
            }
            _ => {
                // 0=실선, 1=파선, 2=점선, 3=일점쇄선, 4=이점쇄선, 5=긴파선, 6=원형점선
                let dash: &[f64] = match shape {
                    1 => &[3.0, 3.0],
                    2 => &[1.0, 2.0],
                    3 => &[6.0, 2.0, 1.0, 2.0],
                    4 => &[6.0, 2.0, 1.0, 2.0, 1.0, 2.0],
                    5 => &[8.0, 4.0],
                    6 => &[0.1, 2.5],
                    _ => &[],
                };
                if shape == 6 {
                    self.ctx.set_line_cap("round");
                }
                self.draw_single_canvas_line(x1, y1, x2, y2, color, 1.0, dash);
                if shape == 6 {
                    self.ctx.set_line_cap("butt");
                }
            }
        }
    }

    fn draw_wave_canvas(&self, x1: f64, y1: f64, x2: f64, color: &str, width: f64, wave_h: f64, wave_w: f64) {
        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.move_to(x1, y1);
        let mut cx = x1;
        let mut up = true;
        while cx < x2 {
            let next = (cx + wave_w).min(x2);
            let cy = if up { y1 - wave_h } else { y1 + wave_h };
            let _ = self.ctx.quadratic_curve_to((cx + next) / 2.0, cy, next, y1);
            cx = next;
            up = !up;
        }
        self.ctx.set_stroke_style_str(color);
        self.ctx.set_line_width(width);
        self.ctx.stroke();
        self.ctx.restore();
    }

    fn draw_single_canvas_line(&self, x1: f64, y1: f64, x2: f64, y2: f64, color: &str, width: f64, dash: &[f64]) {
        self.ctx.save();
        self.ctx.begin_path();
        self.ctx.move_to(x1, y1);
        self.ctx.line_to(x2, y2);
        self.ctx.set_stroke_style_str(color);
        self.ctx.set_line_width(width);
        if !dash.is_empty() {
            let arr = js_sys::Array::new();
            for &d in dash {
                arr.push(&JsValue::from(d));
            }
            self.ctx.set_line_dash(&arr).ok();
        }
        self.ctx.stroke();
        self.ctx.restore();
    }
}

#[cfg(target_arch = "wasm32")]
impl WebCanvasRenderer {
    /// 이미지를 fill_mode에 따라 렌더링한다.
    fn draw_image_with_fill_mode(
        &mut self,
        data: &[u8],
        bbox: &super::render_tree::BoundingBox,
        fill_mode: Option<ImageFillMode>,
        original_size: Option<(f64, f64)>,
        crop: Option<(i32, i32, i32, i32)>,
    ) {
        let mode = fill_mode.unwrap_or(ImageFillMode::FitToSize);
        match mode {
            ImageFillMode::FitToSize | ImageFillMode::None => {
                // crop이 있으면 source rect 기반 drawImage 사용
                if let Some((cl, ct, cr, cb)) = crop {
                    if let Some((img_w, img_h)) = parse_image_dimensions_canvas(data) {
                        let img_w = img_w as f64;
                        let img_h = img_h as f64;
                        let scale_x = cr as f64 / img_w;
                        let src_x = cl as f64 / scale_x;
                        let src_y = ct as f64 / scale_x;
                        let src_w = (cr - cl) as f64 / scale_x;
                        let src_h = (cb - ct) as f64 / scale_x;
                        let is_cropped = src_x > 0.5 || src_y > 0.5
                            || (src_w - img_w).abs() > 1.0 || (src_h - img_h).abs() > 1.0;
                        if is_cropped {
                            self.draw_image_cropped(data, src_x, src_y, src_w, src_h,
                                bbox.x, bbox.y, bbox.width, bbox.height);
                            return;
                        }
                    }
                }
                self.draw_image(data, bbox.x, bbox.y, bbox.width, bbox.height);
            }
            _ => {
                // 원본 크기: HWP shape_attr 기반(우선) 또는 이미지 픽셀 크기(폴백)
                let (img_width, img_height) = if let Some((ow, oh)) = original_size {
                    (ow, oh)
                } else {
                    match parse_image_dimensions_canvas(data) {
                        Some((w, h)) => (w as f64, h as f64),
                        None => {
                            // 크기 파싱 실패 시 전체 채우기로 폴백
                            self.draw_image(data, bbox.x, bbox.y, bbox.width, bbox.height);
                            return;
                        }
                    }
                };

                let (ix, iy) = match mode {
                    ImageFillMode::LeftTop => (bbox.x, bbox.y),
                    ImageFillMode::CenterTop => (bbox.x + (bbox.width - img_width) / 2.0, bbox.y),
                    ImageFillMode::RightTop => (bbox.x + bbox.width - img_width, bbox.y),
                    ImageFillMode::LeftCenter => (bbox.x, bbox.y + (bbox.height - img_height) / 2.0),
                    ImageFillMode::Center => (bbox.x + (bbox.width - img_width) / 2.0, bbox.y + (bbox.height - img_height) / 2.0),
                    ImageFillMode::RightCenter => (bbox.x + bbox.width - img_width, bbox.y + (bbox.height - img_height) / 2.0),
                    ImageFillMode::LeftBottom => (bbox.x, bbox.y + bbox.height - img_height),
                    ImageFillMode::CenterBottom => (bbox.x + (bbox.width - img_width) / 2.0, bbox.y + bbox.height - img_height),
                    ImageFillMode::RightBottom => (bbox.x + bbox.width - img_width, bbox.y + bbox.height - img_height),
                    ImageFillMode::TileAll | ImageFillMode::TileHorzTop | ImageFillMode::TileHorzBottom
                    | ImageFillMode::TileVertLeft | ImageFillMode::TileVertRight => (bbox.x, bbox.y),
                    _ => (bbox.x, bbox.y),
                };

                // Canvas에서 클리핑 적용
                self.ctx.save();
                self.ctx.begin_path();
                self.ctx.rect(bbox.x, bbox.y, bbox.width, bbox.height);
                self.ctx.clip();

                match mode {
                    ImageFillMode::TileAll => {
                        // 바둑판식으로-모두: 전체 타일링
                        let mut ty = bbox.y;
                        while ty < bbox.y + bbox.height {
                            let mut tx = bbox.x;
                            while tx < bbox.x + bbox.width {
                                self.draw_image(data, tx, ty, img_width, img_height);
                                tx += img_width;
                            }
                            ty += img_height;
                        }
                    }
                    ImageFillMode::TileHorzTop | ImageFillMode::TileHorzBottom => {
                        let ty = if mode == ImageFillMode::TileHorzTop { bbox.y } else { bbox.y + bbox.height - img_height };
                        let mut tx = bbox.x;
                        while tx < bbox.x + bbox.width {
                            self.draw_image(data, tx, ty, img_width, img_height);
                            tx += img_width;
                        }
                    }
                    ImageFillMode::TileVertLeft | ImageFillMode::TileVertRight => {
                        let tx = if mode == ImageFillMode::TileVertLeft { bbox.x } else { bbox.x + bbox.width - img_width };
                        let mut ty = bbox.y;
                        while ty < bbox.y + bbox.height {
                            self.draw_image(data, tx, ty, img_width, img_height);
                            ty += img_height;
                        }
                    }
                    _ => {
                        // 배치 모드: 원본 크기로 지정 위치에 배치
                        self.draw_image(data, ix, iy, img_width, img_height);
                    }
                }

                self.ctx.restore();
            }
        }
    }
}

/// 화살표 크기 계산 (SVG 렌더러와 동일 로직)
#[cfg(target_arch = "wasm32")]
fn calc_arrow_dims(stroke_width: f64, line_len: f64, arrow_size: u8) -> (f64, f64) {
    let width_level = arrow_size / 3;
    let length_level = arrow_size % 3;
    let width_mult = match width_level {
        0 => 1.5,
        1 => 2.5,
        _ => 3.5,
    };
    let length_mult = match length_level {
        0 => 1.0,
        1 => 1.5,
        _ => 2.0,
    };
    let arrow_h = (stroke_width * width_mult).max(3.0);
    let arrow_w = (arrow_h * length_mult).min(line_len * 0.3);
    (arrow_w, arrow_h)
}

/// Canvas 2D에 화살표 머리 그리기
///
/// (tip_x, tip_y): 화살표 끝점 (선의 시작/끝 좌표)
/// (dir_x, dir_y): 선이 향하는 방향의 단위벡터 (tip에서 선 바깥쪽을 향함)
/// arrow_w: 화살표 길이, arrow_h: 화살표 높이(폭)
#[cfg(target_arch = "wasm32")]
fn draw_arrow_head(
    ctx: &web_sys::CanvasRenderingContext2d,
    tip_x: f64, tip_y: f64,
    dir_x: f64, dir_y: f64,
    arrow_w: f64, arrow_h: f64,
    arrow_style: &super::ArrowStyle,
    color: &str,
    stroke_width: f64,
) {
    use super::ArrowStyle;

    // 화살표 로컬 좌표 → 월드 좌표 변환
    // along: 선 방향 (tip → base), perp: 수직 방향
    let along_x = -dir_x; // tip에서 base 방향
    let along_y = -dir_y;
    let perp_x = dir_y;   // 90도 회전 (오른쪽)
    let perp_y = -dir_x;

    let half_h = arrow_h / 2.0;

    // 로컬(along, perp) → 월드(x, y) 변환
    let to_world = |along: f64, perp: f64| -> (f64, f64) {
        (
            tip_x + along * along_x + perp * perp_x,
            tip_y + along * along_y + perp * perp_y,
        )
    };

    match arrow_style {
        ArrowStyle::Arrow => {
            // 삼각형: tip → 좌하 → 우하
            let (bx1, by1) = to_world(arrow_w, -half_h);
            let (bx2, by2) = to_world(arrow_w, half_h);
            ctx.begin_path();
            ctx.move_to(tip_x, tip_y);
            ctx.line_to(bx1, by1);
            ctx.line_to(bx2, by2);
            ctx.close_path();
            ctx.set_fill_style_str(color);
            ctx.fill();
        }
        ArrowStyle::ConcaveArrow => {
            let concave = arrow_w * 0.3;
            let (bx1, by1) = to_world(arrow_w, -half_h);
            let (bx2, by2) = to_world(arrow_w, half_h);
            let (cx, cy) = to_world(arrow_w - concave, 0.0);
            ctx.begin_path();
            ctx.move_to(tip_x, tip_y);
            ctx.line_to(bx1, by1);
            ctx.line_to(cx, cy);
            ctx.line_to(bx2, by2);
            ctx.close_path();
            ctx.set_fill_style_str(color);
            ctx.fill();
        }
        ArrowStyle::Diamond | ArrowStyle::OpenDiamond => {
            let half_w = arrow_w / 2.0;
            let (px1, py1) = to_world(0.0, 0.0);       // 앞 꼭짓점 (tip 쪽)
            let (px2, py2) = to_world(half_w, -half_h); // 좌
            let (px3, py3) = to_world(arrow_w, 0.0);    // 뒤 꼭짓점
            let (px4, py4) = to_world(half_w, half_h);  // 우
            ctx.begin_path();
            ctx.move_to(px1, py1);
            ctx.line_to(px2, py2);
            ctx.line_to(px3, py3);
            ctx.line_to(px4, py4);
            ctx.close_path();
            if *arrow_style == ArrowStyle::Diamond {
                ctx.set_fill_style_str(color);
                ctx.fill();
            } else {
                ctx.set_fill_style_str("white");
                ctx.fill();
                ctx.set_stroke_style_str(color);
                ctx.set_line_width((stroke_width * 0.3).max(0.5));
                ctx.stroke();
            }
        }
        ArrowStyle::Circle | ArrowStyle::OpenCircle => {
            let half_w = arrow_w / 2.0;
            let (cx, cy) = to_world(half_w, 0.0);
            let rx = half_w * 0.8;
            let ry = half_h * 0.8;
            ctx.begin_path();
            let _ = ctx.ellipse(cx, cy, rx, ry, 0.0, 0.0, std::f64::consts::TAU);
            if *arrow_style == ArrowStyle::Circle {
                ctx.set_fill_style_str(color);
                ctx.fill();
            } else {
                ctx.set_fill_style_str("white");
                ctx.fill();
                ctx.set_stroke_style_str(color);
                ctx.set_line_width((stroke_width * 0.3).max(0.5));
                ctx.stroke();
            }
        }
        ArrowStyle::Square | ArrowStyle::OpenSquare => {
            let (px1, py1) = to_world(0.0, -half_h);
            let (px2, py2) = to_world(arrow_w, -half_h);
            let (px3, py3) = to_world(arrow_w, half_h);
            let (px4, py4) = to_world(0.0, half_h);
            ctx.begin_path();
            ctx.move_to(px1, py1);
            ctx.line_to(px2, py2);
            ctx.line_to(px3, py3);
            ctx.line_to(px4, py4);
            ctx.close_path();
            if *arrow_style == ArrowStyle::Square {
                ctx.set_fill_style_str(color);
                ctx.fill();
            } else {
                ctx.set_fill_style_str("white");
                ctx.fill();
                ctx.set_stroke_style_str(color);
                ctx.set_line_width((stroke_width * 0.3).max(0.5));
                ctx.stroke();
            }
        }
        ArrowStyle::None => {}
    }
}

/// COLORREF (BGR) → CSS 색상 문자열 변환
///
/// HWP의 COLORREF는 BGR 순서 (0x00BBGGRR)이므로
/// CSS RGB 형식으로 변환한다.
fn color_to_css(color: u32) -> String {
    let b = (color >> 16) & 0xFF;
    let g = (color >> 8) & 0xFF;
    let r = color & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_to_css() {
        // HWP COLORREF: 0x00BBGGRR (BGR)
        assert_eq!(color_to_css(0x000000FF), "#ff0000"); // 빨강
        assert_eq!(color_to_css(0x0000FF00), "#00ff00"); // 초록
        assert_eq!(color_to_css(0x00FF0000), "#0000ff"); // 파랑
        assert_eq!(color_to_css(0x00FFFFFF), "#ffffff"); // 흰색
        assert_eq!(color_to_css(0x00000000), "#000000"); // 검정
    }
}
