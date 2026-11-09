//! 对应原项目的 `App.xaml` 全局样式 + 各 XAML 中的 ControlTemplate / Style。
//! 用 GDI owner-draw 复刻暗色主题、圆角窗口、自定义滑块、复选框、Tab、按钮与超链接。
#![allow(dead_code)]

use crate::native::*;
use std::collections::HashMap;

pub mod c {
    use crate::native::{rgb, COLORREF};
    pub const BG: COLORREF = rgb(0x2D, 0x2D, 0x2D);
    pub const INFO_BG: COLORREF = rgb(0x25, 0x25, 0x25);
    pub const TITLE_BG: COLORREF = rgb(0x1E, 0x1E, 0x1E);
    pub const PANEL: COLORREF = rgb(0x3E, 0x3E, 0x42);
    pub const BORDER: COLORREF = rgb(0x3F, 0x3F, 0x46);
    pub const TRACK: COLORREF = rgb(0x3E, 0x3E, 0x42);
    pub const TRACK_LINE: COLORREF = rgb(0x55, 0x55, 0x55);
    pub const WHITE: COLORREF = rgb(0xFF, 0xFF, 0xFF);
    pub const TITLE_TEXT: COLORREF = rgb(0xAA, 0xAA, 0xAA);
    pub const MUTED: COLORREF = rgb(0x90, 0x90, 0x90);
    pub const DIM: COLORREF = rgb(0x80, 0x80, 0x80);
    pub const LIGHT: COLORREF = rgb(0xD3, 0xD3, 0xD3);
    pub const PALE: COLORREF = rgb(0xCC, 0xCC, 0xCC);
    pub const LINK: COLORREF = rgb(0x00, 0x7A, 0xCC);
    pub const LINK_HOVER: COLORREF = rgb(0x33, 0x99, 0xFF);
    pub const RED: COLORREF = rgb(0x99, 0x33, 0x33);
    pub const RED_HOVER: COLORREF = rgb(0xB0, 0x3F, 0x3F);
    pub const DELETE_RED: COLORREF = rgb(0xFF, 0x55, 0x55);
    pub const PREVIEW_BLUE: COLORREF = rgb(0x57, 0x77, 0xA3);
    pub const HOVER: COLORREF = rgb(0x50, 0x50, 0x55);
}

// ---------------------------------------------------------------------------
// 绘制工具
// ---------------------------------------------------------------------------

pub struct Painter {
    pub dc: HDC,
    fonts: HashMap<i32, HFONT>,
    pub scale: f64,
}
impl Painter {
    pub fn new(dc: HDC, scale: f64) -> Self { Painter { dc, fonts: HashMap::new(), scale } }
    pub fn px(&self, v: i32) -> i32 { (v as f64 * self.scale).round() as i32 }
    fn font(&mut self, size: i32, weight: i32) -> HFONT {
        let key = size * 1000 + weight;
        if let Some(&f) = self.fonts.get(&key) { return f; }
        let face = wide("Segoe UI");
        let h = unsafe {
            CreateFontW(-self.px(size), 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                DEFAULT_PITCH | (FF_DONTCARE << 4), face.as_ptr())
        };
        self.fonts.insert(key, h);
        h
    }
    pub fn fill(&self, r: RECT, color: COLORREF) {
        unsafe {
            let br = CreateSolidBrush(color);
            FillRect(self.dc, &r, br);
            DeleteObject(br);
        }
    }
    pub fn frame(&self, r: RECT, color: COLORREF) {
        unsafe {
            let br = CreateSolidBrush(color);
            let rr = r.deflate(0);
            frame_with_brush(self.dc, &rr, br);
            DeleteObject(br);
        }
    }
    pub fn round(&self, r: RECT, radius: i32, fill: COLORREF, border: Option<COLORREF>) {
        unsafe {
            let br = CreateSolidBrush(fill);
            let pen = CreatePen(PS_SOLID, 1, border.unwrap_or(fill));
            let old_br = SelectObject(self.dc, br);
            let old_pen = SelectObject(self.dc, pen);
            RoundRect(self.dc, r.left, r.top, r.right, r.bottom, self.px(radius * 2), self.px(radius * 2));
            SelectObject(self.dc, old_br);
            SelectObject(self.dc, old_pen);
            DeleteObject(br);
            DeleteObject(pen);
        }
    }
    pub fn rect(&self, r: RECT, fill: COLORREF) {
        unsafe {
            let br = CreateSolidBrush(fill);
            let old = SelectObject(self.dc, br);
            Rectangle(self.dc, r.left, r.top, r.right, r.bottom);
            SelectObject(self.dc, old);
            DeleteObject(br);
        }
    }
    pub fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32, color: COLORREF, width: i32) {
        unsafe {
            let pen = CreatePen(PS_SOLID, width.max(1), color);
            let old = SelectObject(self.dc, pen);
            MoveToEx(self.dc, x1, y1, std::ptr::null_mut());
            LineTo(self.dc, x2, y2);
            SelectObject(self.dc, old);
            DeleteObject(pen);
        }
    }
    pub fn text(&mut self, s: &str, r: RECT, color: COLORREF, size: i32, flags: u32) {
        unsafe {
            let f = self.font(size, FW_NORMAL);
            let old = SelectObject(self.dc, f);
            SetBkMode(self.dc, TRANSPARENT);
            SetTextColor(self.dc, color);
            let w = wide(s);
            let mut rr = r;
            DrawTextW(self.dc, w.as_ptr(), -1, &mut rr, flags | DT_NOPREFIX);
            SelectObject(self.dc, old);
        }
    }
    pub fn text_bold(&mut self, s: &str, r: RECT, color: COLORREF, size: i32, flags: u32) {
        unsafe {
            let f = self.font(size, FW_MEDIUM);
            let old = SelectObject(self.dc, f);
            SetBkMode(self.dc, TRANSPARENT);
            SetTextColor(self.dc, color);
            let w = wide(s);
            let mut rr = r;
            DrawTextW(self.dc, w.as_ptr(), -1, &mut rr, flags | DT_NOPREFIX);
            SelectObject(self.dc, old);
        }
    }
    pub fn text_width(&mut self, s: &str, size: i32) -> i32 {
        unsafe {
            let f = self.font(size, FW_NORMAL);
            let old = SelectObject(self.dc, f);
            let w = wide(s);
            let mut sz = SIZE { cx: 0, cy: 0 };
            GetTextExtentPoint32W(self.dc, w.as_ptr(), (w.len() - 1) as i32, &mut sz);
            SelectObject(self.dc, old);
            sz.cx
        }
    }
}
impl Drop for Painter {
    fn drop(&mut self) {
        unsafe { for (_, f) in self.fonts.drain() { DeleteObject(f); } }
    }
}

/// GDI 没有 FrameRect 的独立声明，这里用四个 1px 矩形实现描边。
fn frame_with_brush(dc: HDC, r: &RECT, br: HBRUSH) {
    unsafe {
        let t = RECT::new(r.left, r.top, r.right, r.top + 1);
        let b = RECT::new(r.left, r.bottom - 1, r.right, r.bottom);
        let l = RECT::new(r.left, r.top, r.left + 1, r.bottom);
        let rr = RECT::new(r.right - 1, r.top, r.right, r.bottom);
        FillRect(dc, &t, br); FillRect(dc, &b, br);
        FillRect(dc, &l, br); FillRect(dc, &rr, br);
    }
}

// ---------------------------------------------------------------------------
// 控件模型（每个 XAML 元素对应一个 Widget）
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub enum Kind {
    Label(String, i32, COLORREF),
    BoldLabel(String, i32, COLORREF),
    Button(String),
    RedButton(String),
    CloseButton(String),
    CheckBox(String, bool),
    DayBox(String, bool),
    Radio(String, bool),
    Slider { min: f64, max: f64, value: f64, vertical: bool },
    Tab(String, bool),
    Link(String, COLORREF),
    TextBox(String),
    Combo(String),
    IconBox,
    PreviewButton,
    Separator,
    ResizeGrip,
}

#[derive(Clone, Debug)]
pub struct Widget {
    pub id: u32,
    pub rect: RECT,
    pub kind: Kind,
    pub enabled: bool,
    pub visible: bool,
    pub hover: bool,
    pub tag: String,
}
impl Widget {
    pub fn new(id: u32, rect: RECT, kind: Kind) -> Self {
        Widget { id, rect, kind, enabled: true, visible: true, hover: false, tag: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Click(u32),
    Check(u32, bool),
    Value(u32, f64),
    Text(u32, String),
    Wheel(u32, i32),
    None,
}

pub struct Ui {
    pub widgets: Vec<Widget>,
    pub capture: Option<u32>,
    pub hover: Option<u32>,
    pub focus: Option<u32>,
}
impl Ui {
    pub fn new() -> Self { Ui { widgets: Vec::new(), capture: None, hover: None, focus: None } }
    pub fn clear(&mut self) { self.widgets.clear(); self.hover = None; self.focus = None; }
    pub fn get(&self, id: u32) -> Option<&Widget> { self.widgets.iter().find(|w| w.id == id && w.visible) }
    pub fn get_mut(&mut self, id: u32) -> Option<&mut Widget> { self.widgets.iter_mut().find(|w| w.id == id) }
    pub fn set_value(&mut self, id: u32, v: f64) {
        if let Some(w) = self.get_mut(id) {
            if let Kind::Slider { value, min, max, .. } = &mut w.kind { *value = v.clamp(*min, *max); }
        }
    }
    pub fn value(&self, id: u32) -> f64 {
        match self.get(id).map(|w| &w.kind) {
            Some(Kind::Slider { value, .. }) => *value,
            _ => 0.0,
        }
    }
    pub fn set_text(&mut self, id: u32, s: &str) {
        if let Some(w) = self.get_mut(id) {
            match &mut w.kind {
                Kind::Label(t, _, _) | Kind::BoldLabel(t, _, _) | Kind::Button(t)
                | Kind::RedButton(t) | Kind::CloseButton(t) | Kind::Link(t, _)
                | Kind::TextBox(t) | Kind::Combo(t) => *t = s.to_string(),
                _ => {}
            }
        }
    }
    pub fn set_checked(&mut self, id: u32, v: bool) {
        if let Some(w) = self.get_mut(id) {
            match &mut w.kind {
                Kind::CheckBox(_, b) | Kind::DayBox(_, b) | Kind::Radio(_, b) => *b = v,
                _ => {}
            }
        }
    }
    pub fn checked(&self, id: u32) -> bool {
        match self.get(id).map(|w| &w.kind) {
            Some(Kind::CheckBox(_, b)) | Some(Kind::DayBox(_, b)) | Some(Kind::Radio(_, b)) => *b,
            _ => false,
        }
    }
    pub fn set_enabled(&mut self, id: u32, v: bool) {
        if let Some(w) = self.get_mut(id) { w.enabled = v; }
    }
    pub fn set_visible(&mut self, id: u32, v: bool) {
        if let Some(w) = self.get_mut(id) { w.visible = v; }
    }
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        self.widgets.iter().rposition(|w| w.visible && w.enabled && w.rect.contains(x, y))
    }

    pub fn draw(&self, p: &mut Painter) {
        for w in &self.widgets {
            if !w.visible { continue; }
            draw_widget(p, w);
        }
    }

    pub fn mouse_down(&mut self, x: i32, y: i32) -> Event {
        let Some(i) = self.hit(x, y) else { return Event::None; };
        let id = self.widgets[i].id;
        let kind = self.widgets[i].kind.clone();
        match kind {
            Kind::Slider { min, max, vertical, .. } => {
                self.capture = Some(id);
                let r = self.widgets[i].rect;
                let v = slider_value(r, x, y, min, max, vertical);
                self.set_value(id, v);
                Event::Value(id, v)
            }
            Kind::CheckBox(_, b) | Kind::DayBox(_, b) | Kind::Radio(_, b) => {
                self.set_checked(id, !b);
                Event::Check(id, !b)
            }
            Kind::TextBox(_) | Kind::Combo(_) => {
                self.focus = Some(id);
                Event::Click(id)
            }
            _ => Event::Click(id),
        }
    }
    pub fn mouse_move(&mut self, x: i32, y: i32) -> Event {
        if let Some(cap) = self.capture {
            if let Some(w) = self.widgets.iter().find(|w| w.id == cap) {
                let (min, max, vertical, r) = match &w.kind {
                    Kind::Slider { min, max, vertical, .. } => (*min, *max, *vertical, w.rect),
                    _ => return Event::None,
                };
                let v = slider_value(r, x, y, min, max, vertical);
                self.set_value(cap, v);
                return Event::Value(cap, v);
            }
        }
        let h = self.hit(x, y).map(|i| self.widgets[i].id);
        if h != self.hover {
            self.hover = h;
            for w in &mut self.widgets { w.hover = Some(w.id) == h; }
            return Event::None;
        }
        Event::None
    }
    pub fn mouse_up(&mut self, _x: i32, _y: i32) -> Event {
        self.capture = None;
        Event::None
    }
    pub fn wheel(&mut self, x: i32, y: i32, delta: i32) -> Event {
        let Some(i) = self.hit(x, y) else { return Event::None; };
        let id = self.widgets[i].id;
        let step = if delta > 0 { 0.05 } else { -0.05 };
        match self.widgets[i].kind.clone() {
            Kind::Slider { min, max, value, .. } => {
                let v = (value + step).clamp(min, max);
                self.set_value(id, v);
                Event::Value(id, v)
            }
            _ => Event::Wheel(id, delta),
        }
    }
}

fn slider_value(r: RECT, x: i32, y: i32, min: f64, max: f64, vertical: bool) -> f64 {
    let pad = 9;
    let t = if vertical {
        let h = (r.height() - pad * 2).max(1);
        1.0 - ((y - r.top - pad) as f64 / h as f64)
    } else {
        let w = (r.width() - pad * 2).max(1);
        (x - r.left - pad) as f64 / w as f64
    };
    min + t.clamp(0.0, 1.0) * (max - min)
}

fn draw_widget(p: &mut Painter, w: &Widget) {
    let r = w.rect;
    let dim = !w.enabled;
    let fg = if dim { c::DIM } else { c::WHITE };
    match &w.kind {
        Kind::Label(s, size, color) => {
            let multi = w.tag == "wrap" || w.tag == "wrap_center";
            let mut flags: u32 = if w.tag == "center" || w.tag == "wrap_center" { DT_CENTER } else { DT_LEFT };
            if multi { flags |= DT_WORDBREAK; } else { flags |= DT_VCENTER | DT_SINGLELINE; }
            p.text(s, r, if dim { c::DIM } else { *color }, *size, flags);
        }
        Kind::BoldLabel(s, size, color) => {
            let flags = if w.tag == "center" { DT_CENTER } else { DT_LEFT };
            p.text_bold(s, r, if dim { c::DIM } else { *color }, *size,
                flags | DT_VCENTER | DT_SINGLELINE);
        }
        Kind::Button(s) | Kind::RedButton(s) | Kind::CloseButton(s) => {
            let base = match w.kind {
                Kind::RedButton(_) => c::RED,
                Kind::CloseButton(_) => c::TITLE_BG,
                _ => c::PANEL,
            };
            let fill = if w.hover {
                match w.kind {
                    Kind::RedButton(_) => c::RED_HOVER,
                    Kind::CloseButton(_) => c::RED,
                    _ => c::HOVER,
                }
            } else { base };
            let radius = if matches!(w.kind, Kind::CloseButton(_)) { 0 } else { 2 };
            p.round(RECT::new(r.left, r.top, r.right - 1, r.bottom - 1), radius, fill, Some(c::BORDER));
            let tr = r.deflate(0);
            p.text(s, tr, fg, 11, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
        Kind::CheckBox(s, checked) | Kind::DayBox(s, checked) => {
            let box_sz = p.px(16);
            let br = RECT::new(r.left, r.top + (r.height() - box_sz) / 2, r.left + box_sz, r.top + (r.height() + box_sz) / 2);
            p.round(RECT::new(br.left, br.top, br.right - 1, br.bottom - 1), 2, c::PANEL, Some(c::BORDER));
            if *checked {
                let cx = (br.left + br.right) / 2;
                let cy = (br.top + br.bottom) / 2;
                p.line(cx - p.px(4), cy, cx - p.px(1), cy + p.px(3), c::WHITE, p.px(2));
                p.line(cx - p.px(1), cy + p.px(3), cx + p.px(4), cy - p.px(3), c::WHITE, p.px(2));
            }
            let tr = RECT::new(br.right + p.px(5), r.top, r.right, r.bottom);
            p.text(s, tr, if dim { c::DIM } else { c::LIGHT }, 11, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        }
        Kind::Radio(s, checked) => {
            let fill = if *checked { c::PANEL } else { c::TITLE_BG };
            p.round(RECT::new(r.left, r.top, r.right - 1, r.bottom - 1), 2, fill, Some(c::BORDER));
            let col = if *checked { c::WHITE } else { c::DIM };
            p.text(s, r, col, 11, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
        Kind::Slider { min, max, value, vertical } => {
            let t = if (*max - *min).abs() < 1e-9 { 0.0 } else { (value - min) / (max - min) };
            if *vertical {
                let cx = (r.left + r.right) / 2;
                let tw = p.px(5);
                let track = RECT::new(cx - tw / 2, r.top + p.px(6), cx + tw / 2, r.bottom - p.px(6));
                p.round(track, 2, c::TRACK, None);
                let cy = track.bottom - (t.clamp(0.0, 1.0) * track.height() as f64).round() as i32;
                let d = p.px(18);
                let thumb_color = if w.hover || w.enabled { c::LIGHT } else { c::DIM };
                p.round(RECT::new(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2), 9, thumb_color, Some(c::BORDER));
            } else {
                let cy = (r.top + r.bottom) / 2;
                let th = p.px(6);
                let track = RECT::new(r.left + p.px(5), cy - th / 2, r.right - p.px(5), cy + th / 2);
                p.round(track, 3, c::TRACK, None);
                let cx = track.left + (t.clamp(0.0, 1.0) * track.width() as f64).round() as i32;
                let d = p.px(18);
                let thumb_color = if dim { c::DIM } else if w.hover { c::WHITE } else { c::LIGHT };
                p.round(RECT::new(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2), 9, thumb_color, Some(c::BORDER));
            }
        }
        Kind::Tab(s, active) => {
            let fill = if *active { c::PANEL } else { c::BG };
            p.round(RECT::new(r.left, r.top, r.right - 1, r.bottom + 2), 3, fill, Some(c::BORDER));
            if *active { p.fill(RECT::new(r.left + 1, r.bottom - 2, r.right - 1, r.bottom + 2), c::PANEL); }
            p.text(s, r, if *active { c::WHITE } else { c::MUTED }, 11, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
        }
        Kind::Link(s, color) => {
            let col = if w.hover { c::LINK_HOVER } else { *color };
            p.text(s, r, col, 11, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            let tw = p.text_width(s, 11).min(r.width());
            let y = (r.top + r.bottom) / 2 + p.px(7);
            p.line(r.left, y, r.left + tw, y, col, 1);
        }
        Kind::TextBox(s) | Kind::Combo(s) => {
            p.round(RECT::new(r.left, r.top, r.right - 1, r.bottom - 1), 2, c::PANEL, Some(c::BORDER));
            let tr = r.deflate(p.px(4));
            p.text(s, tr, fg, 11, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            if matches!(w.kind, Kind::Combo(_)) {
                let cx = r.right - p.px(12);
                let cy = (r.top + r.bottom) / 2;
                p.line(cx - p.px(3), cy - p.px(1), cx, cy + p.px(2), c::LIGHT, 1);
                p.line(cx, cy + p.px(2), cx + p.px(3), cy - p.px(1), c::LIGHT, 1);
            }
        }
        Kind::IconBox => {
            // 图标由窗口层用 DrawIconEx 绘制，这里只保留占位框
        }
        Kind::PreviewButton => {
            p.round(RECT::new(r.left, r.top, r.right - 1, r.bottom - 1), 2, c::PANEL, Some(c::BORDER));
            let cx = (r.left + r.right) / 2;
            let cy = (r.top + r.bottom) / 2;
            let d = p.px(10);
            p.round(RECT::new(cx - d, cy - d / 2, cx + d, cy + d / 2), 5, c::PREVIEW_BLUE, None);
            p.round(RECT::new(cx - 2, cy - 2, cx + 2, cy + 2), 2, c::TITLE_BG, None);
        }
        Kind::Separator => { p.line(r.left, (r.top + r.bottom) / 2, r.right, (r.top + r.bottom) / 2, c::BORDER, 1); }
        Kind::ResizeGrip => {
            for i in 0..3 {
                let o = p.px(3 + i * 3);
                p.line(r.right - o, r.bottom - p.px(2), r.right - p.px(2), r.bottom - o, c::DIM, 1);
            }
        }
    }
}

/// 双缓冲绘制入口：对应 WPF 的 composition / 离屏渲染。
pub fn paint_window(hwnd: HWND, ui: &Ui, bg: COLORREF, title: Option<(RECT, &str)>, icon: HICON, scale: f64) {
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc == NULL { return; }
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr);
        let mem = CreateCompatibleDC(hdc);
        let bmp = CreateCompatibleBitmap(hdc, cr.width().max(1), cr.height().max(1));
        let old_bmp = SelectObject(mem, bmp);

        {
            let mut p = Painter::new(mem, scale);
            p.fill(cr, bg);
            // 圆角窗口边框（对应 XAML 的最外层 Border CornerRadius=8）
            p.round(RECT::new(0, 0, cr.width() - 1, cr.height() - 1), 8, bg, Some(c::BORDER));
            if let Some((tr, text)) = title {
                p.fill(tr, c::TITLE_BG);
                let ix = tr.left + p.px(10);
                let iy = tr.top + (tr.height() - p.px(16)) / 2;
                if icon != NULL {
                    DrawIconEx(mem, ix, iy, icon, p.px(16), p.px(16), 0, 0, 0x0003);
                }
                let mut tx = tr;
                tx.left = ix + p.px(24);
                p.text(text, tx, c::TITLE_TEXT, 12, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
            }
            ui.draw(&mut p);
            // 内容区图标（对应 AppInfoWindow 的 65×65 Image）
            for wi in &ui.widgets {
                if !wi.visible { continue; }
                if let Kind::IconBox = wi.kind {
                    if icon != NULL {
                        DrawIconEx(mem, wi.rect.left, wi.rect.top, icon,
                            wi.rect.width(), wi.rect.height(), 0, 0, 0x0003);
                    }
                }
            }
        }

        BitBlt(hdc, 0, 0, cr.width(), cr.height(), mem, 0, 0, SRCCOPY);
        SelectObject(mem, old_bmp);
        DeleteObject(bmp);
        DeleteDC(mem);
        EndPaint(hwnd, &ps);
    }
}

pub fn apply_round_region(hwnd: HWND, w: i32, h: i32, radius: i32) {
    unsafe {
        let rgn = CreateRoundRectRgn(0, 0, w, h, radius * 2, radius * 2);
        if rgn != NULL { SetWindowRgn(hwnd, rgn, TRUE); }
    }
}
