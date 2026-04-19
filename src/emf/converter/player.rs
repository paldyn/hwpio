//! EMF Player — 레코드 시퀀스를 순회하며 DC/ObjectTable을 갱신하고 SVG 노드를 발행한다.
//!
//! 단계 12 범위: 드로잉(선/사각형/타원/호/패스/폴리라인16). 텍스트·비트맵은 단계 13.

use std::fmt::Write;

use crate::emf::Error;
use crate::emf::parser::objects::{Header, LogBrush, LogPen, PointL, RectL};
use crate::emf::parser::records::Record;

use super::device_context::{DcStack, GraphicsObject, ObjectTable};
use super::svg::{colorref_to_rgb, SvgBuilder};

pub struct Player {
    pub dc_stack:    DcStack,
    pub objects:     ObjectTable,
    pub svg:         SvgBuilder,
    pub render_rect: (f32, f32, f32, f32),
    pub header:      Option<Header>,

    // 패스 상태
    path_active: bool,
    path_d:      String,
}

impl Player {
    #[must_use]
    pub fn new(render_rect: (f32, f32, f32, f32)) -> Self {
        Self {
            dc_stack: DcStack::new(),
            objects:  ObjectTable::new(),
            svg:      SvgBuilder::new(),
            render_rect,
            header:   None,
            path_active: false,
            path_d: String::new(),
        }
    }

    /// 레코드 시퀀스 전체 재생.
    pub fn play(&mut self, records: &[Record]) -> Result<(), Error> {
        // 먼저 헤더를 찾아 매핑 행렬을 세운다.
        if let Some(Record::Header(h)) = records.iter().find(|r| matches!(r, Record::Header(_))) {
            self.header = Some(h.clone());
        }
        self.open_root_group();

        for rec in records {
            self.exec(rec);
        }

        self.svg.close_group();
        Ok(())
    }

    fn open_root_group(&mut self) {
        // Bounds → render_rect 매핑. Bounds가 비어 있으면 identity.
        let (rx, ry, rw, rh) = self.render_rect;
        let m = if let Some(h) = &self.header {
            let w = (h.bounds.right  - h.bounds.left)  as f32;
            let hh = (h.bounds.bottom - h.bounds.top)   as f32;
            if w > 0.0 && hh > 0.0 {
                let sx = rw / w;
                let sy = rh / hh;
                let tx = rx - h.bounds.left as f32 * sx;
                let ty = ry - h.bounds.top  as f32 * sy;
                [sx, 0.0, 0.0, sy, tx, ty]
            } else {
                [1.0, 0.0, 0.0, 1.0, rx, ry]
            }
        } else {
            [1.0, 0.0, 0.0, 1.0, rx, ry]
        };
        self.svg.open_group_matrix(m);
    }

    fn exec(&mut self, rec: &Record) {
        match rec {
            Record::Header(_) | Record::Eof => {}

            // 객체
            Record::CreatePen { handle, pen } =>
                self.objects.insert(*handle, GraphicsObject::Pen(*pen)),
            Record::CreateBrushIndirect { handle, brush } =>
                self.objects.insert(*handle, GraphicsObject::Brush(*brush)),
            Record::ExtCreateFontIndirectW { handle, font } =>
                self.objects.insert(*handle, GraphicsObject::Font(font.clone())),
            Record::SelectObject { handle } => self.select_object(*handle),
            Record::DeleteObject { handle } => { self.objects.remove(*handle); }

            // 상태 — DC
            Record::SaveDC => self.dc_stack.save(),
            Record::RestoreDC { relative } => { self.dc_stack.restore(*relative); }
            Record::SetWorldTransform(_) | Record::ModifyWorldTransform { .. } => {
                // 단계 12에서는 WorldTransform을 DC에 저장만 하고 출력 적용은 생략.
                // 단계 13~14에서 개별 도형에 transform 적용.
            }

            // 좌표계/색상
            Record::SetMapMode(m)        => self.dc_stack.current_mut().map_mode = *m,
            Record::SetWindowExtEx(s)    => self.dc_stack.current_mut().window_ext   = (s.cx, s.cy),
            Record::SetWindowOrgEx(p)    => self.dc_stack.current_mut().window_org   = (p.x, p.y),
            Record::SetViewportExtEx(s)  => self.dc_stack.current_mut().viewport_ext = (s.cx, s.cy),
            Record::SetViewportOrgEx(p)  => self.dc_stack.current_mut().viewport_org = (p.x, p.y),
            Record::SetBkMode(v)         => self.dc_stack.current_mut().bk_mode    = *v,
            Record::SetTextAlign(v)      => self.dc_stack.current_mut().text_align = *v,
            Record::SetTextColor(v)      => self.dc_stack.current_mut().text_color = *v,
            Record::SetBkColor(v)        => self.dc_stack.current_mut().bk_color   = *v,

            // 드로잉
            Record::MoveToEx(p) => self.dc_stack.current_mut().current_pos = (p.x, p.y),
            Record::LineTo(p)   => self.emit_line_to(p),
            Record::Rectangle(r) => self.emit_rect(r, None),
            Record::RoundRect { rect, corner_w, corner_h } =>
                self.emit_rect(rect, Some((*corner_w, *corner_h))),
            Record::Ellipse(r)  => self.emit_ellipse(r),
            Record::Arc   { rect, start, end } => self.emit_arc_like(rect, start, end, ArcKind::Arc),
            Record::Chord { rect, start, end } => self.emit_arc_like(rect, start, end, ArcKind::Chord),
            Record::Pie   { rect, start, end } => self.emit_arc_like(rect, start, end, ArcKind::Pie),
            Record::Polyline16   { points, .. } => self.emit_polyline16(points, false),
            Record::Polygon16    { points, .. } => self.emit_polyline16(points, true),
            Record::PolyBezier16 { points, .. } => self.emit_polybezier16(points),

            // 패스
            Record::BeginPath => {
                self.path_active = true;
                self.path_d.clear();
            }
            Record::EndPath => {
                self.path_active = false;
            }
            Record::CloseFigure => {
                if !self.path_d.is_empty() { self.path_d.push_str(" Z"); }
            }
            Record::FillPath(_) => {
                let (fill, stroke) = (self.fill_spec(), None);
                self.emit_path(fill, stroke);
            }
            Record::StrokePath(_) => {
                self.emit_path(None, Some(self.stroke_spec()));
            }
            Record::StrokeAndFillPath(_) => {
                self.emit_path(self.fill_spec(), Some(self.stroke_spec()));
            }

            Record::Unknown { .. } => {}
        }
    }

    fn select_object(&mut self, handle: u32) {
        let Some(obj) = self.objects.get(handle) else { return; };
        match obj {
            GraphicsObject::Pen(p)   => self.dc_stack.current_mut().pen   = Some(*p),
            GraphicsObject::Brush(b) => self.dc_stack.current_mut().brush = Some(*b),
            GraphicsObject::Font(f)  => self.dc_stack.current_mut().font  = Some(f.clone()),
        }
    }

    fn stroke_spec(&self) -> StrokeSpec {
        if let Some(p) = self.dc_stack.current().pen {
            // PS_NULL(5) → 스트로크 없음
            let is_null = (p.style & 0x0F) == 5;
            if is_null {
                StrokeSpec { color: None, width: 0.0 }
            } else {
                StrokeSpec {
                    color: Some(colorref_to_rgb(p.color)),
                    width: p.width.max(1) as f32,
                }
            }
        } else {
            StrokeSpec { color: Some("black".into()), width: 1.0 }
        }
    }

    fn fill_spec(&self) -> Option<String> {
        if let Some(b) = self.dc_stack.current().brush {
            if b.style == 1 { None }                       // BS_NULL
            else            { Some(colorref_to_rgb(b.color)) }
        } else {
            Some("none".into())
        }
    }

    fn emit_line_to(&mut self, to: &PointL) {
        let (x1, y1) = self.dc_stack.current().current_pos;
        let s = self.stroke_spec();
        let color = s.color.as_deref().unwrap_or("none");
        let node = format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{color}\" stroke-width=\"{:.2}\" fill=\"none\"/>",
            x1, y1, to.x, to.y, s.width,
        );
        if self.path_active {
            if self.path_d.is_empty() {
                let _ = write!(self.path_d, "M{x1} {y1} ");
            }
            let _ = write!(self.path_d, "L{} {} ", to.x, to.y);
        } else {
            self.svg.push(&node);
        }
        self.dc_stack.current_mut().current_pos = (to.x, to.y);
    }

    fn emit_rect(&mut self, r: &RectL, corner: Option<(i32, i32)>) {
        let stroke = self.stroke_spec();
        let fill   = self.fill_spec().unwrap_or_else(|| "none".into());
        let stroke_color = stroke.color.as_deref().unwrap_or("none");
        let (rx_attr, ry_attr) = match corner {
            Some((cw, ch)) => (format!(" rx=\"{}\"", cw / 2), format!(" ry=\"{}\"", ch / 2)),
            None => (String::new(), String::new()),
        };
        let node = format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"{rx_attr}{ry_attr} fill=\"{fill}\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            r.left, r.top, r.width(), r.height(), stroke.width,
        );
        self.svg.push(&node);
    }

    fn emit_ellipse(&mut self, r: &RectL) {
        let stroke = self.stroke_spec();
        let fill   = self.fill_spec().unwrap_or_else(|| "none".into());
        let cx = (r.left + r.right) / 2;
        let cy = (r.top + r.bottom) / 2;
        let rx = (r.right - r.left).abs() / 2;
        let ry = (r.bottom - r.top).abs() / 2;
        let stroke_color = stroke.color.as_deref().unwrap_or("none");
        let node = format!(
            "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            stroke.width,
        );
        self.svg.push(&node);
    }

    fn emit_arc_like(&mut self, r: &RectL, start: &PointL, end: &PointL, kind: ArcKind) {
        // 근사: arc은 시작점→끝점 단순 선, chord는 같음, pie는 중심까지 삼각형 폐곡선.
        // 단계 12는 SVG arc path로 표현.
        let cx = (r.left + r.right) as f32 / 2.0;
        let cy = (r.top + r.bottom) as f32 / 2.0;
        let rx = (r.right - r.left).abs() as f32 / 2.0;
        let ry = (r.bottom - r.top).abs() as f32 / 2.0;
        let (s, e) = (start, end);
        let stroke = self.stroke_spec();
        let fill = match kind {
            ArcKind::Arc   => "none".to_string(),
            ArcKind::Chord | ArcKind::Pie => self.fill_spec().unwrap_or_else(|| "none".into()),
        };
        let d = match kind {
            ArcKind::Arc =>
                format!("M {} {} A {} {} 0 0 1 {} {}", s.x, s.y, rx, ry, e.x, e.y),
            ArcKind::Chord =>
                format!("M {} {} A {} {} 0 0 1 {} {} Z", s.x, s.y, rx, ry, e.x, e.y),
            ArcKind::Pie =>
                format!("M {cx} {cy} L {} {} A {} {} 0 0 1 {} {} Z", s.x, s.y, rx, ry, e.x, e.y),
        };
        let stroke_color = stroke.color.as_deref().unwrap_or("none");
        let node = format!(
            "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            stroke.width,
        );
        self.svg.push(&node);
    }

    fn emit_polyline16(&mut self, points: &[(i16, i16)], close: bool) {
        if points.is_empty() { return; }
        let pts: String = points.iter().map(|(x, y)| format!("{x},{y}")).collect::<Vec<_>>().join(" ");
        let stroke = self.stroke_spec();
        let fill = if close { self.fill_spec().unwrap_or_else(|| "none".into()) } else { "none".into() };
        let tag = if close { "polygon" } else { "polyline" };
        let stroke_color = stroke.color.as_deref().unwrap_or("none");
        let node = format!(
            "<{tag} points=\"{pts}\" fill=\"{fill}\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            stroke.width,
        );
        self.svg.push(&node);
    }

    fn emit_polybezier16(&mut self, points: &[(i16, i16)]) {
        if points.is_empty() { return; }
        let mut d = format!("M{} {}", points[0].0, points[0].1);
        // EMF PolyBezier: 첫 점은 시작점, 이후 3점씩 제어1 제어2 끝점(C 커맨드).
        let mut i = 1;
        while i + 2 < points.len() + 1 && i + 2 <= points.len() {
            let (c1x, c1y) = points[i];
            let (c2x, c2y) = points[i + 1];
            let (ex, ey)   = points[i + 2];
            let _ = write!(d, " C{c1x} {c1y} {c2x} {c2y} {ex} {ey}");
            i += 3;
        }
        let stroke = self.stroke_spec();
        let stroke_color = stroke.color.as_deref().unwrap_or("none");
        let node = format!(
            "<path d=\"{d}\" fill=\"none\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            stroke.width,
        );
        self.svg.push(&node);
    }

    fn emit_path(&mut self, fill: Option<String>, stroke: Option<StrokeSpec>) {
        if self.path_d.is_empty() { return; }
        let fill_attr = fill.as_deref().unwrap_or("none");
        let (stroke_color, stroke_width) = stroke
            .map_or(("none".into(), 0.0_f32),
                    |s| (s.color.unwrap_or_else(|| "none".into()), s.width));
        let node = format!(
            "<path d=\"{}\" fill=\"{fill_attr}\" stroke=\"{stroke_color}\" stroke-width=\"{:.2}\"/>",
            self.path_d.trim(),
            stroke_width,
        );
        self.svg.push(&node);
        self.path_d.clear();
    }
}

#[derive(Copy, Clone)]
enum ArcKind { Arc, Chord, Pie }

#[derive(Debug, Clone)]
pub struct StrokeSpec {
    pub color: Option<String>,
    pub width: f32,
}
