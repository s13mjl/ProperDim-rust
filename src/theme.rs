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
    /// 对应 XAML 滑块 Thumb：`Ellipse Fill="#B8B8B8" Stroke="#7E7E7E" StrokeThickness="2"`。
    pub const THUMB: COLORREF = rgb(0xB8, 0xB8, 0xB8);
    pub const THUMB_STROKE: COLORREF = rgb(0x7E, 0x7E, 0x7E);
}

// ---------------------------------------------------------------------------
// 绘制工具
// ---------------------------------------------------------------------------

/// GDI 对象缓存。
///
/// 原来是「每次 WM_PAINT 新建 Painter → 重建 3 个字体 + 每控件一个画刷/画笔 + 新建后备位图」，
/// 亮度动画 60fps 时每秒上千次 GDI 对象创建/销毁，是拖动与动画卡顿的主因。
/// 这里把字体/画刷/画笔/后备缓冲全部按窗口线程缓存，尺寸变化时才重建。
struct GdiCache {
    fonts: HashMap<i32, HFONT>,
    brushes: HashMap<u32, HBRUSH>,
    pens: HashMap<(u32, i32), HPEN>,
    back: Option<(HDC, HBITMAP, isize, i32, i32)>,
}
thread_local! {
    static CACHE: std::cell::RefCell<GdiCache> = std::cell::RefCell::new(GdiCache {
        fonts: HashMap::new(), brushes: HashMap::new(), pens: HashMap::new(), back: None,
    });
}

fn cached_font(px: i32, weight: i32) -> HFONT {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let key = px * 1000 + weight;
        if let Some(&f) = c.fonts.get(&key) { return f; }
        let face = wide("Segoe UI");
        let h = unsafe {
            CreateFontW(-px.max(1), 0, 0, 0, weight, 0, 0, 0, DEFAULT_CHARSET,
                OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                DEFAULT_PITCH | (FF_DONTCARE << 4), face.as_ptr())
        };
        c.fonts.insert(key, h);
        h
    })
}

fn cached_brush(color: COLORREF) -> HBRUSH {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let key = color as u32;
        if let Some(&b) = c.brushes.get(&key) { return b; }
        let b = unsafe { CreateSolidBrush(color) };
        c.brushes.insert(key, b);
        b
    })
}

fn cached_pen(color: COLORREF, width: i32) -> HPEN {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let key = (color as u32, width);
        if let Some(&p) = c.pens.get(&key) { return p; }
        let p = unsafe { CreatePen(PS_SOLID, width.max(1), color) };
        c.pens.insert(key, p);
        p
    })
}

/// 取（必要时重建）与窗口同尺寸的后备缓冲 DC。
fn backbuffer(hdc: HDC, w: i32, h: i32) -> HDC {
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        let (w, h) = (w.max(1), h.max(1));
        // 只增不减：多个不同尺寸的窗口共用一块够大的缓冲，避免每次切换窗口都重建 DIB。
        // 只 BitBlt 左上角 w×h，缓冲多出来的部分永远不会被呈现。
        if let Some((mem, _bmp, _old, bw, bh)) = c.back {
            if bw >= w && bh >= h { return mem; }
            unsafe {
                SelectObject(mem, _old as HGDIOBJ);
                DeleteObject(_bmp as HGDIOBJ);
                DeleteDC(mem);
            }
        }
        let (nw, nh) = (w.max(
            if let Some((_, _, _, bw, _)) = c.back { bw } else { 0 }), h.max(
            if let Some((_, _, _, _, bh)) = c.back { bh } else { 0 }));
        let mem = unsafe { CreateCompatibleDC(hdc) };
        let bmp = unsafe { CreateCompatibleBitmap(hdc, nw, nh) };
        let old = unsafe { SelectObject(mem, bmp as HGDIOBJ) } as isize;
        c.back = Some((mem, bmp, old, nw, nh));
        mem
    })
}

pub struct Painter {
    pub dc: HDC,
    pub scale: f64,
}
impl Painter {
    pub fn new(dc: HDC, scale: f64) -> Self { Painter { dc, scale } }
    pub fn px(&self, v: i32) -> i32 { (v as f64 * self.scale).round() as i32 }
    fn font(&self, size: i32, weight: i32) -> HFONT { cached_font(self.px(size), weight) }
    pub fn fill(&self, r: RECT, color: COLORREF) {
        unsafe { FillRect(self.dc, &r, cached_brush(color)); }
    }
    pub fn frame(&self, r: RECT, color: COLORREF) {
        frame_with_brush(self.dc, &r, cached_brush(color));
    }
    pub fn round(&self, r: RECT, radius: i32, fill: COLORREF, border: Option<COLORREF>) {
        unsafe {
            let br = cached_brush(fill);
            let pen = cached_pen(border.unwrap_or(fill), 1);
            let old_br = SelectObject(self.dc, br as HGDIOBJ);
            let old_pen = SelectObject(self.dc, pen as HGDIOBJ);
            RoundRect(self.dc, r.left, r.top, r.right, r.bottom, self.px(radius * 2), self.px(radius * 2));
            SelectObject(self.dc, old_br);
            SelectObject(self.dc, old_pen);
        }
    }
    pub fn rect(&self, r: RECT, fill: COLORREF) {
        unsafe {
            let br = cached_brush(fill);
            let old = SelectObject(self.dc, br as HGDIOBJ);
            Rectangle(self.dc, r.left, r.top, r.right, r.bottom);
            SelectObject(self.dc, old);
        }
    }
    pub fn line(&self, x1: i32, y1: i32, x2: i32, y2: i32, color: COLORREF, width: i32) {
        unsafe {
            let pen = cached_pen(color, width);
            let old = SelectObject(self.dc, pen as HGDIOBJ);
            MoveToEx(self.dc, x1, y1, std::ptr::null_mut());
            LineTo(self.dc, x2, y2);
            SelectObject(self.dc, old);
        }
    }
    pub fn text(&mut self, s: &str, r: RECT, color: COLORREF, size: i32, flags: u32) {
        unsafe {
            let f = self.font(size, FW_NORMAL);
            let old = SelectObject(self.dc, f as HGDIOBJ);
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
            let old = SelectObject(self.dc, f as HGDIOBJ);
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
            let old = SelectObject(self.dc, f as HGDIOBJ);
            let w = wide(s);
            let mut sz = SIZE { cx: 0, cy: 0 };
            GetTextExtentPoint32W(self.dc, w.as_ptr(), (w.len() - 1) as i32, &mut sz);
            SelectObject(self.dc, old);
            sz.cx
        }
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
    /// 鼠标按在按钮上但还没松开（对应 WPF 的 IsPressed）
    pub pressed: bool,
    pub tag: String,
}
impl Widget {
    pub fn new(id: u32, rect: RECT, kind: Kind) -> Self {
        Widget { id, rect, kind, enabled: true, visible: true, hover: false,
            pressed: false, tag: String::new() }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Click(u32),
    Check(u32, bool),
    Value(u32, f64),
    Text(u32, String),
    Wheel(u32, i32),
    DoubleClick(u32),
    Key(u32, i32),
    None,
}

pub struct Ui {
    pub widgets: Vec<Widget>,
    pub capture: Option<u32>,
    pub hover: Option<u32>,
    pub focus: Option<u32>,
    /// 已被按下、等待松开才触发 Click 的控件（对应 WPF 按钮在 MouseUp 时触发 Click）
    pub pressed: Option<u32>,
    pub scroll_offset: i32,
    pub selected_row: Option<usize>,
    /// true = 滑块点击（对应 Slider_ValueChanged 的非拖动分支，需要动画）；false = 拖动中。
    pub slider_click: bool,
}
impl Ui {
    pub fn new() -> Self {
        Ui { widgets: Vec::new(), capture: None, hover: None, focus: None, pressed: None,
            scroll_offset: 0, selected_row: None, slider_click: false }
    }
    pub fn clear(&mut self) {
        // 注意：scroll_offset / selected_row 是「视图状态」而不是控件状态，
        // 重建控件树时不能清零，否则每次重排都会把列表滚回顶部（上游 ListBox 不会）。
        self.widgets.clear(); self.hover = None; self.focus = None;
        self.slider_click = false;
    }
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
            if Some(w.id) == self.focus {
                // 对应 WPF 的 FocusVisualStyle：虚线焦点框
                let r = RECT::new(w.rect.left + 1, w.rect.top + 1, w.rect.right - 1, w.rect.bottom - 1);
                unsafe { DrawFocusRect(p.dc, &r); }
            }
        }
    }

    /// 对应 WPF 的 Tab / Shift+Tab 焦点遍历（按控件创建顺序）。
    pub fn focus_next(&mut self, back: bool) {
        let ids: Vec<u32> = self.widgets.iter()
            .filter(|w| w.visible && w.enabled && focusable(&w.kind))
            .map(|w| w.id).collect();
        if ids.is_empty() { return; }
        let cur = self.focus.and_then(|f| ids.iter().position(|&i| i == f));
        let next = match cur {
            None => if back { ids.len() - 1 } else { 0 },
            Some(p) => if back { (p + ids.len() - 1) % ids.len() } else { (p + 1) % ids.len() },
        };
        self.focus = Some(ids[next]);
    }

    pub fn mouse_down(&mut self, x: i32, y: i32) -> Event {
        let Some(i) = self.hit(x, y) else {
            // 对应 HotkeysTab_MouseDown：点击空白处夺走 TextBox 焦点
            self.focus = None;
            return Event::None;
        };
        let id = self.widgets[i].id;
        let kind = self.widgets[i].kind.clone();
        if focusable(&kind) { self.focus = Some(id); }
        // 对应 WPF：Button / Hyperlink 在 MouseUp 时才 Click。
        // 原来按下就触发，导致「点 Cool 时窗口在还没点完就消失」。
        if matches!(kind, Kind::Button(_) | Kind::RedButton(_) | Kind::CloseButton(_) | Kind::Link(..)) {
            self.pressed = Some(id);
            for w in &mut self.widgets { w.pressed = Some(w.id) == self.pressed; }
            return Event::None;
        }
        match kind {
            Kind::Slider { min, max, vertical, .. } => {
                self.capture = Some(id);
                self.slider_click = true;
                let r = self.widgets[i].rect;
                let v = slider_value(r, x, y, min, max, vertical);
                self.set_value(id, v);
                Event::Value(id, v)
            }
            Kind::CheckBox(_, b) | Kind::DayBox(_, b) => {
                self.set_checked(id, !b);
                Event::Check(id, !b)
            }
            // 单选按钮（AM/PM）不自行反选：由窗口层维护互斥组（对应 RadioButton GroupName 行为）
            Kind::Radio(..) => {
                self.set_checked(id, true);
                Event::Click(id)
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
            self.slider_click = false;
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
    pub fn mouse_up(&mut self, x: i32, y: i32) -> Event {
        self.capture = None;
        // 对应 _isDragging = false：松手后滚轮/键盘改值恢复为动画分支
        self.slider_click = false;
        let pressed = self.pressed.take();
        for w in &mut self.widgets { w.pressed = false; }
        if let Some(pid) = pressed {
            if self.hit(x, y).map(|i| self.widgets[i].id) == Some(pid) {
                return Event::Click(pid);
            }
        }
        Event::None
    }
    pub fn wheel(&mut self, x: i32, y: i32, delta: i32) -> Event {
        let Some(i) = self.hit(x, y) else { return Event::None; };
        let id = self.widgets[i].id;
        // 对应 DimmerSlider_MouseWheel / MenuSlider 全局滚轮：
        //   Ctrl 或右键按住 → ±1%；否则按 5% 网格吸附（((cur+4)/5)*5，向上恰好等于当前值时 +5）
        // 对应 ScheduleDialog/MinDialog：固定 ±1%（以 tag="fine" 标记）
        let fine_key = unsafe { GetAsyncKeyState(VK_CONTROL) != 0 || GetAsyncKeyState(0x02) != 0 };
        let (min, max, value, tag) = match &self.widgets[i].kind {
            Kind::Slider { min, max, value, .. } => (*min, *max, *value, self.widgets[i].tag.clone()),
            _ => return Event::Wheel(id, delta),
        };
        let v = if tag == "fine" {
            let step = if delta > 0 { 0.01 } else { -0.01 };
            (value + step).clamp(min, max)
        } else {
            let cur = (value * 100.0).round() as i32;
            let mut pct = if fine_key {
                if delta > 0 { cur + 1 } else { cur - 1 }
            } else if delta > 0 {
                let n = ((cur + 4) / 5) * 5;
                if n == cur { n + 5 } else { n }
            } else {
                ((cur - 1) / 5) * 5
            };
            let lo = (min * 100.0).round() as i32;
            let hi = (max * 100.0).round() as i32;
            pct = pct.clamp(lo, hi);
            pct as f64 / 100.0
        };
        self.set_value(id, v);
        Event::Value(id, v)
    }
}

/// 对应 WPF 中可接受键盘焦点的元素（Label / 装饰元素不可聚焦）。
fn focusable(k: &Kind) -> bool {
    !matches!(k, Kind::Label(..) | Kind::BoldLabel(..) | Kind::IconBox
        | Kind::Separator | Kind::ResizeGrip)
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
            let fill = if w.pressed {
                match w.kind {
                    Kind::RedButton(_) => rgb(0x7A, 0x28, 0x28),
                    Kind::CloseButton(_) => c::RED_HOVER,
                    _ => rgb(0x2A, 0x2A, 0x2E),
                }
            } else if w.hover {
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
                let fill = if dim { c::DIM } else { c::THUMB };
                if !gp::fill_circle(p.dc, cx, cy, d, fill, Some((c::THUMB_STROKE, p.px(2)))) {
                    p.round(RECT::new(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2), d / 2, fill, Some(c::THUMB_STROKE));
                }
            } else {
                let cy = (r.top + r.bottom) / 2;
                let th = p.px(6);
                let track = RECT::new(r.left + p.px(5), cy - th / 2, r.right - p.px(5), cy + th / 2);
                p.round(track, 3, c::TRACK, None);
                let cx = track.left + (t.clamp(0.0, 1.0) * track.width() as f64).round() as i32;
                let d = p.px(18);
                let fill = if dim { c::DIM } else { c::THUMB };
                // 上游是 Ellipse（矢量圆），GDI 的 RoundRect 会有锯齿 → 用 GDI+ 抗锯齿绘制
                if !gp::fill_circle(p.dc, cx, cy, d, fill, Some((c::THUMB_STROKE, p.px(2)))) {
                    p.round(RECT::new(cx - d / 2, cy - d / 2, cx + d / 2, cy + d / 2), d / 2, fill, Some(c::THUMB_STROKE));
                }
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
            let right = w.tag.contains("right");
            // 上游这些链接都在 HorizontalAlignment=Center 的 TextBlock 里：
            // 文字必须居中，不能按控件左边界左对齐（否则整体偏左）
            let flags = if right { DT_RIGHT } else { DT_CENTER };
            p.text(s, r, col, 11, flags | DT_VCENTER | DT_SINGLELINE);
            if !w.tag.contains("nounderline") {
                let tw = p.text_width(s, 11).min(r.width());
                let x0 = if right { r.right - tw } else { r.left + (r.width() - tw) / 2 };
                let y = (r.top + r.bottom) / 2 + p.px(7);
                p.line(x0, y, x0 + tw, y, col, 1);
            }
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
pub fn paint_window(hwnd: HWND, ui: &Ui, bg: COLORREF, title: Option<(RECT, &str)>,
                    icon: HICON, icon_large: HICON, scale: f64) {
    unsafe {
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let hdc = BeginPaint(hwnd, &mut ps);
        if hdc == NULL { return; }
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr);
        // 复用后备缓冲（尺寸不变时不再每次分配 DIB）
        let mem = backbuffer(hdc, cr.width(), cr.height());

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
                    let big = if icon_large != NULL { icon_large } else { icon };
                    if big != NULL {
                        DrawIconEx(mem, wi.rect.left, wi.rect.top, big,
                            wi.rect.width(), wi.rect.height(), 0, 0, 0x0003);
                    }
                }
            }
        }

        BitBlt(hdc, 0, 0, cr.width(), cr.height(), mem, 0, 0, SRCCOPY);
        EndPaint(hwnd, &ps);
    }
}

pub fn apply_round_region(hwnd: HWND, w: i32, h: i32, radius: i32) {
    unsafe {
        let rgn = CreateRoundRectRgn(0, 0, w, h, radius * 2, radius * 2);
        if rgn != NULL { SetWindowRgn(hwnd, rgn, TRUE); }
    }
}

/// GDI+ 动态封装：只用于画抗锯齿圆（对应 WPF 的 `Ellipse`）。
/// 本机没有 Windows SDK 的 `gdiplus.lib`，所以全部走 `LoadLibrary` + `GetProcAddress`，
/// 失败时调用方退回 GDI 的 `RoundRect`，不影响功能。
pub mod gp {
    use crate::native::*;
    use std::sync::OnceLock;

    #[repr(C)]
    struct StartupInput {
        version: u32,
        callback: *const std::ffi::c_void,
        suppress_bg: i32,
        suppress_codecs: i32,
    }

    type FnStartup = unsafe extern "system" fn(*mut usize, *const StartupInput, *mut std::ffi::c_void) -> i32;
    type FnCreateFromHdc = unsafe extern "system" fn(HDC, *mut *mut std::ffi::c_void) -> i32;
    type FnSetSmoothing = unsafe extern "system" fn(*mut std::ffi::c_void, i32) -> i32;
    type FnSolidFill = unsafe extern "system" fn(u32, *mut *mut std::ffi::c_void) -> i32;
    type FnPen1 = unsafe extern "system" fn(u32, f32, i32, *mut *mut std::ffi::c_void) -> i32;
    type FnEllipse = unsafe extern "system" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, f32, f32, f32, f32) -> i32;
    type FnDelete = unsafe extern "system" fn(*mut std::ffi::c_void) -> i32;

    struct Apis {
        create_from_hdc: FnCreateFromHdc,
        set_smoothing: FnSetSmoothing,
        solid_fill: FnSolidFill,
        pen1: FnPen1,
        fill_ellipse: FnEllipse,
        draw_ellipse: FnEllipse,
        delete_brush: FnDelete,
        delete_pen: FnDelete,
        delete_graphics: FnDelete,
    }

    static APIS: OnceLock<Option<Apis>> = OnceLock::new();

    /// COLORREF(0x00BBGGRR) → GDI+ ARGB(0xAARRGGBB)
    fn argb(c: COLORREF) -> u32 {
        0xFF00_0000 | ((c & 0xFF) << 16) | (c & 0x00FF_00) | ((c >> 16) & 0xFF)
    }

    fn load() -> Option<Apis> {
        unsafe {
            let lib = wide("gdiplus.dll");
            let h = LoadLibraryW(lib.as_ptr());
            if h == 0 { return None; }
            unsafe fn sym(h: HMODULE, name: &str) -> *const std::ffi::c_void {
                let mut v = name.as_bytes().to_vec();
                v.push(0);
                GetProcAddress(h, v.as_ptr())
            }
            macro_rules! sym_as {
                ($name:literal, $t:ty) => {{
                    let p = sym(h, $name);
                    if p.is_null() { return None; }
                    std::mem::transmute::<*const std::ffi::c_void, $t>(p)
                }};
            }
            let startup = sym_as!("GdiplusStartup", FnStartup);
            let create_from_hdc = sym_as!("GdipCreateFromHDC", FnCreateFromHdc);
            let set_smoothing = sym_as!("GdipSetSmoothingMode", FnSetSmoothing);
            let solid_fill = sym_as!("GdipCreateSolidFill", FnSolidFill);
            let pen1 = sym_as!("GdipCreatePen1", FnPen1);
            let fill_ellipse = sym_as!("GdipFillEllipse", FnEllipse);
            let draw_ellipse = sym_as!("GdipDrawEllipse", FnEllipse);
            let delete_brush = sym_as!("GdipDeleteBrush", FnDelete);
            let delete_pen = sym_as!("GdipDeletePen", FnDelete);
            let delete_graphics = sym_as!("GdipDeleteGraphics", FnDelete);

            let mut token: usize = 0;
            let input = StartupInput {
                version: 1, callback: std::ptr::null(), suppress_bg: 0, suppress_codecs: 0,
            };
            if startup(&mut token, &input, std::ptr::null_mut()) != 0 { return None; }

            Some(Apis {
                create_from_hdc, set_smoothing, solid_fill, pen1,
                fill_ellipse, draw_ellipse, delete_brush, delete_pen, delete_graphics,
            })
        }
    }

    fn apis() -> Option<&'static Apis> { APIS.get_or_init(load).as_ref() }

    /// 画一个抗锯齿圆（可选描边）。返回 false 表示 GDI+ 不可用，调用方需自行回退。
    pub fn fill_circle(dc: HDC, cx: i32, cy: i32, d: i32,
                       fill: COLORREF, stroke: Option<(COLORREF, i32)>) -> bool {
        if d <= 0 { return false; }
        let Some(a) = apis() else { return false; };
        // 描边居中落在路径上：内缩半个笔宽，保证整体不超出 d×d
        let inset = stroke.map(|(_, w)| (w.max(1) / 2) as f32).unwrap_or(0.0);
        let x = cx as f32 - d as f32 / 2.0 + inset;
        let y = cy as f32 - d as f32 / 2.0 + inset;
        let s = d as f32 - inset * 2.0;
        unsafe {
            let mut g: *mut std::ffi::c_void = std::ptr::null_mut();
            if (a.create_from_hdc)(dc, &mut g) != 0 || g.is_null() { return false; }
            (a.set_smoothing)(g, 4); // SmoothingModeAntiAlias
            let mut br: *mut std::ffi::c_void = std::ptr::null_mut();
            if (a.solid_fill)(argb(fill), &mut br) == 0 && !br.is_null() {
                (a.fill_ellipse)(g, br, x, y, s, s);
                (a.delete_brush)(br);
            }
            if let Some((col, w)) = stroke {
                let mut pen: *mut std::ffi::c_void = std::ptr::null_mut();
                if (a.pen1)(argb(col), w.max(1) as f32, 2 /* UnitPixel */, &mut pen) == 0 && !pen.is_null() {
                    (a.draw_ellipse)(g, pen, x, y, s, s);
                    (a.delete_pen)(pen);
                }
            }
            (a.delete_graphics)(g);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 亮度动画每 16ms 重绘一次；绘制路径必须是「查缓存 + 画」，不能每次重建 GDI 对象。
    /// 这里用真实窗口尺寸 + 近似控件数量做绝对上限回归（原来每次重绘要新建字体/画刷/位图）。
    #[test]
    fn repaint_is_cheap_enough_for_60fps() {
        unsafe {
            let dc = CreateCompatibleDC(NULL);
            let bmp = CreateCompatibleBitmap(dc, 525, 300);
            let old = SelectObject(dc, bmp as HGDIOBJ);

            let mut ui = Ui::new();
            let mut y = 0;
            while y < 260 {
                ui.widgets.push(Widget::new(1000 + y as u32, RECT::new(10, y, 200, y + 18),
                    Kind::Label("Brightness Level: ".into(), 14, c::WHITE)));
                ui.widgets.push(Widget::new(2000 + y as u32, RECT::new(210, y, 500, y + 18),
                    Kind::Button("20%".into())));
                y += 20;
            }
            ui.widgets.push(Widget::new(1, RECT::new(20, 82, 505, 102),
                Kind::Slider { min: 0.01, max: 1.0, value: 0.62, vertical: false }));
            ui.widgets.push(Widget::new(2, RECT::new(100, 130, 300, 148),
                Kind::Link("Adjust Minimum".into(), c::MUTED)));

            let frames = 200;
            let t0 = GetTickCount64();
            for _ in 0..frames {
                let mut p = Painter::new(dc, 1.0);
                p.fill(RECT::new(0, 0, 525, 300), c::BG);
                ui.draw(&mut p);
            }
            let total = GetTickCount64() - t0;
            let per = total as f64 / frames as f64;

            SelectObject(dc, old);
            DeleteObject(bmp as HGDIOBJ);
            DeleteDC(dc);

            // 16ms 一帧的预算里，绘制应占用很小一部分；超过 4ms 说明又退回了逐对象创建
            assert!(per < 4.0, "单帧绘制 {:.2}ms，超过 4ms 预算（{} 帧共 {}ms）", per, frames, total);
        }
    }

    #[test]
    fn thumb_uses_antialiased_gdiplus() {
        // 圆点必须能走 GDI+ 抗锯齿路径（失败会回退成锯齿的 RoundRect）
        unsafe {
            let dc = CreateCompatibleDC(NULL);
            let bmp = CreateCompatibleBitmap(dc, 32, 32);
            let old = SelectObject(dc, bmp as HGDIOBJ);
            let ok = gp::fill_circle(dc, 16, 16, 18, c::THUMB, Some((c::THUMB_STROKE, 2)));
            SelectObject(dc, old);
            DeleteObject(bmp as HGDIOBJ);
            DeleteDC(dc);
            assert!(ok, "GDI+ 未加载，圆点会退回带锯齿的 GDI 实现");
        }
    }


    fn slider(min: f64, max: f64, value: f64, tag: &str) -> Ui {
        let mut ui = Ui::new();
        let mut w = Widget::new(1, RECT::new(0, 0, 100, 20),
            Kind::Slider { min, max, value, vertical: false });
        w.tag = tag.to_string();
        ui.widgets.push(w);
        ui
    }

    #[test]
    fn wheel_snaps_to_five_percent_grid() {
        // 对应 DimmerSlider_MouseWheel：((cur+4)/5)*5，47 → 50
        let mut ui = slider(0.01, 1.0, 0.47, "");
        match ui.wheel(50, 10, 120) {
            Event::Value(_, v) => assert!((v - 0.50).abs() < 1e-9, "{}", v),
            other => panic!("{:?}", other),
        }
        // 向下：47 → 45
        ui.set_value(1, 0.47);
        match ui.wheel(50, 10, -120) {
            Event::Value(_, v) => assert!((v - 0.45).abs() < 1e-9, "{}", v),
            other => panic!("{:?}", other),
        }
        // 已是网格点时向上必须前进一格：50 → 55
        ui.set_value(1, 0.50);
        match ui.wheel(50, 10, 120) {
            Event::Value(_, v) => assert!((v - 0.55).abs() < 1e-9, "{}", v),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn fine_slider_uses_one_percent() {
        // 对应 ScheduleDialog/MinDialog：滚轮固定 ±1%，与总跨度无关
        let mut ui = slider(0.01, 0.40, 0.30, "fine");
        match ui.wheel(50, 10, 120) {
            Event::Value(_, v) => assert!((v - 0.31).abs() < 1e-9, "{}", v),
            other => panic!("{:?}", other),
        }
    }
}
