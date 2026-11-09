//! 窗口框架：统一窗口类、WndProc 分发、双缓冲绘制、标题栏拖动、消息路由。
//! 对应 C# 里 `Window` / `WindowChrome` / `Dispatcher` 提供的公共行为。
#![allow(dead_code, static_mut_refs)]

pub mod dialogs;
pub mod panels;

use crate::app;
use crate::native::*;
use crate::theme::{self, c, paint_window, Event, Kind, Ui, Widget};
use std::ffi::c_void;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowKind {
    Controller,
    ControlPanel,
    TrayMenu,
    ScheduleDialog,
    MinDialog,
    InfoWindow,
    Warning,
}

pub struct WindowCtx {
    pub kind: WindowKind,
    pub ui: Ui,
    pub title: String,
    pub show_title: bool,
    pub bg: COLORREF,
    pub min_w: i32,
    pub min_h: i32,
    pub resizable: bool,
    pub hide_on_deactivate: bool,
    /// 窗口样式（默认 WS_POPUP）；控制面板需要 WS_MINIMIZEBOX 才能最小化到任务栏。
    pub win_style: u32,
    /// 拖拽拉伸上限（对应上游 ResizeThumb 的 150% 限制），0 表示不限制。
    pub max_w: i32,
    pub max_h: i32,
}

impl WindowCtx {
    pub fn new(kind: WindowKind, title: &str, show_title: bool) -> Self {
        WindowCtx {
            kind, ui: Ui::new(), title: title.to_string(), show_title,
            bg: c::BG, min_w: 0, min_h: 0, resizable: false, hide_on_deactivate: false,
            win_style: WS_POPUP, max_w: 0, max_h: 0,
        }
    }
    pub fn title_rect(&self, w: i32, _h: i32) -> Option<RECT> {
        if !self.show_title { None } else { Some(RECT::new(1, 1, w - 1, 1 + s(30))) }
    }
}

/// 逻辑像素 → 物理像素（对应 XAML 的 DIP）。
pub fn s(v: i32) -> i32 { (v as f64 * app::app().scale).round() as i32 }

pub const CLASS_PREFIX: &str = "ProperDimRust";

pub fn register_class(name: &str) -> Vec<u16> {
    let cls = wide(name);
    unsafe {
        let mut wc: WNDCLASSW = std::mem::zeroed();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = app::app().instance;
        wc.hIcon = app::app().icon;
        wc.hCursor = LoadCursorW(NULL as HINSTANCE, IDC_ARROW);
        wc.hbrBackground = 0;
        wc.lpszClassName = cls.as_ptr();
        RegisterClassW(&wc);
    }
    cls
}

pub fn create_window(
    cls: &[u16], title: &str, x: i32, y: i32, w: i32, h: i32,
    ex_style: u32, ctx: WindowCtx, visible: bool,
) -> HWND {
    let t = wide(title);
    let a = app::app();
    let style = ctx.win_style | if visible { WS_VISIBLE } else { 0 };
    let boxed = Box::into_raw(Box::new(ctx));
    let hwnd = unsafe {
        CreateWindowExW(
            ex_style, cls.as_ptr(), t.as_ptr(), style,
            x, y, w, h, NULL, NULL, a.instance, boxed as *const c_void,
        )
    };
    if hwnd == NULL { unsafe { drop(Box::from_raw(boxed)); } return NULL; }
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, boxed as isize);
        let icon = app::app().icon;
        if icon != NULL {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG, icon as isize);
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL, icon as isize);
        }
        theme::apply_round_region(hwnd, w, h, s(8));
        app::app().extra.apply_window_attributes(hwnd);
        relayout(&mut *boxed, w, h);
    }
    hwnd
}

pub fn get_ctx(hwnd: HWND) -> *mut WindowCtx {
    unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowCtx }
}

pub fn refresh(hwnd: HWND) {
    unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE); }
}

/// 统一窗口过程：所有窗口共用，按 kind 把事件交给对应模块。
pub unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let p = get_ctx(hwnd);
    if p.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }

    // 应用级消息：无论哪个窗口收到都路由到 app
    match msg {
        WM_APP_TRAY => { app::handle_tray_msg(lparam); return 0; }
        WM_HOTKEY => { app::on_hotkey(wparam as i32); return 0; }
        WM_TIMER => { app::handle_timer(wparam); return 0; }
        WM_DISPLAYCHANGE | WM_SETTINGCHANGE => { app::debounce_refresh(); return 0; }
        WM_WTSSESSION_CHANGE => { app::handle_session_change(wparam); return 0; }
        WM_POWERBROADCAST => { app::handle_power(wparam); return 0; }
        WM_QUERYENDSESSION => { return 1; }
        WM_ENDSESSION => { app::handle_end_session(); return 0; }
        _ => {}
    }

    let ctx = &mut *p;
    match msg {
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let mut cr = RECT::default();
            GetClientRect(hwnd, &mut cr);
            let tr = ctx.title_rect(cr.width(), cr.height());
            let title = tr.as_ref().map(|_| (tr.unwrap(), ctx.title.as_str()));
            paint_window(hwnd, &ctx.ui, ctx.bg, title,
                app::app().icon, app::app().icon_large, app::app().scale);
            return 0;
        }
        WM_NCHITTEST => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            let mut r = RECT::default(); GetWindowRect(hwnd, &mut r);
            let px = x - r.left; let py = y - r.top;
            if ctx.show_title && py < s(30) && px < r.width() - s(70) { return HTCAPTION; }
            if ctx.resizable && px > r.width() - s(16) && py > r.height() - s(16) { return HTBOTTOMRIGHT; }
            return HTCLIENT;
        }
        WM_LBUTTONDOWN => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            let mut cr = RECT::default(); GetClientRect(hwnd, &mut cr);
            if ctx.show_title && y < s(30) && x < cr.width() - s(70) {
                ReleaseCapture();
                SendMessageW(hwnd, 0x00A1 /* WM_NCLBUTTONDOWN */, HTCAPTION as WPARAM, 0);
                return 0;
            }
            if ctx.resizable && x > cr.width() - s(16) && y > cr.height() - s(16) {
                ReleaseCapture();
                SendMessageW(hwnd, 0x00A1, HTBOTTOMRIGHT as WPARAM, 0);
                return 0;
            }
            let ev = ctx.ui.mouse_down(x, y);
            if ctx.ui.capture.is_some() { SetCapture(hwnd); }
            dispatch(ctx, ev, hwnd);
            return 0;
        }
        WM_LBUTTONUP => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            let ev = ctx.ui.mouse_up(x, y);
            if ctx.ui.capture.is_none() { ReleaseCapture(); }
            if app::app().preview_active { app::stop_preview(); }
            dispatch(ctx, ev, hwnd);
            return 0;
        }
        WM_LBUTTONDBLCLK => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            if let Some(i) = ctx.ui.hit(x, y) {
                let id = ctx.ui.widgets[i].id;
                dispatch(ctx, Event::DoubleClick(id), hwnd);
            }
            return 0;
        }
        WM_MOUSEMOVE => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            // 对应 PreviewButton_MouseLeave：按住预览时移出按钮即恢复
            if app::app().preview_active {
                let over = ctx.ui.widgets.iter().any(|ww| ww.tag == "preview" && ww.rect.contains(x, y));
                if !over { app::stop_preview(); }
            }
            let prev_hover = ctx.ui.hover;
            let prev_capture = ctx.ui.capture;
            let ev = ctx.ui.mouse_move(x, y);
            // 高刷新率鼠标每秒可发上千次移动事件，状态没变化时不要整窗重绘
            if ev != Event::None || prev_hover != ctx.ui.hover || prev_capture != ctx.ui.capture {
                InvalidateRect(hwnd, std::ptr::null(), FALSE);
            }
            dispatch(ctx, ev, hwnd);
            return 0;
        }
        WM_MOUSEWHEEL => {
            let x = lparam_x(lparam); let y = lparam_y(lparam);
            let mut pt = POINT { x, y };
            ScreenToClient(hwnd, &mut pt);
            let delta = ((wparam >> 16) & 0xFFFF) as u16 as i16 as i32;
            let ev = ctx.ui.wheel(pt.x, pt.y, delta);
            // 列表/Tab 空白区域也要能滚动（对应 ScheduleList_PreviewMouseWheel / TabsArea_MouseWheel）
            let ev = if ev == Event::None { Event::Wheel(0, delta) } else { ev };
            dispatch(ctx, ev, hwnd);
            return 0;
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let vk = wparam as i32;
            if msg == WM_KEYDOWN && vk == VK_ESCAPE {
                match ctx.kind {
                    WindowKind::Controller => {}
                    // 对应 ControlPanel 的 PreviewKeyDown：Esc 走 HandleClosure（受 CloseToTray 影响）
                    WindowKind::ControlPanel => app::hide_control_panel(true),
                    WindowKind::Warning => {
                        // 对应 WarningMessage：Esc = 回答 "No"
                        app::app().warning_result = Some(false);
                        ShowWindow(hwnd, SW_HIDE);
                        app::end_modal();
                    }
                    _ => {
                        ShowWindow(hwnd, SW_HIDE);
                        app::end_modal();
                    }
                }
                return 0;
            }
            // 对应 HotkeyTextBox_PreviewKeyDown：录制组合键
            if let Some(id) = ctx.ui.focus {
                let is_hotkey = ctx.ui.get(id).map(|w| w.tag.as_str()) == Some("hotkey");
                if is_hotkey {
                    panels::record_hotkey(ctx, id, wparam as u32, lparam);
                    return 0;
                }
            }
            // 对应 WPF 的 Tab 焦点遍历
            if msg == WM_KEYDOWN && vk == VK_TAB {
                let back = unsafe { GetAsyncKeyState(VK_SHIFT) != 0 };
                ctx.ui.focus_next(back);
                return 0;
            }
            // 对应 App.xaml.cs 的 RegisterClassHandler：CheckBox(Enter) / Hyperlink(Enter+Space)
            if vk == VK_RETURN || vk == VK_SPACE {
                if let Some(id) = ctx.ui.focus {
                    if let Some(k) = ctx.ui.get(id).map(|w| w.kind.clone()) {
                        match k {
                            Kind::CheckBox(_, b) | Kind::DayBox(_, b) => {
                                ctx.ui.set_checked(id, !b);
                                dispatch(ctx, Event::Check(id, !b), hwnd);
                            }
                            Kind::TextBox(_) | Kind::Combo(_) => {}
                            // 日程行是 Label，但上游 Enter/Space 是「编辑选中项」而非点击
                            Kind::Label(..) if ctx.ui.get(id).map(|w| w.tag.starts_with("row")).unwrap_or(false) => {
                                dispatch(ctx, Event::Key(0, vk), hwnd);
                            }
                            _ => { dispatch(ctx, Event::Click(id), hwnd); }
                        }
                        return 0;
                    }
                }
                // 无焦点时退化为鼠标悬停控件（保持可用性）
                if let Some(id) = ctx.ui.hover {
                    if let Some(k) = ctx.ui.get(id).map(|w| w.kind.clone()) {
                        match k {
                            Kind::CheckBox(_, b) | Kind::DayBox(_, b) => {
                                dispatch(ctx, Event::Check(id, !b), hwnd); return 0;
                            }
                            Kind::Link(..) => { dispatch(ctx, Event::Click(id), hwnd); return 0; }
                            _ => {}
                        }
                    }
                }
            }
            dispatch(ctx, Event::Key(0, vk), hwnd);
            return 0;
        }
        WM_KEYUP => {
            if app::app().preview_active { app::stop_preview(); }
            return 0;
        }
        WM_CHAR => {
            if let Some(id) = ctx.ui.focus {
                if ctx.ui.get(id).map(|w| w.tag.as_str()) == Some("hotkey") { return 0; }
                let ch = char::from_u32(wparam as u32).unwrap_or('\0');
                if ch == '\u{0008}' {
                    let old = match ctx.ui.get(id).map(|w| &w.kind) {
                        Some(Kind::TextBox(t)) | Some(Kind::Combo(t)) => t.clone(), _ => String::new(),
                    };
                    let mut n = old; n.pop(); ctx.ui.set_text(id, &n);
                } else if ch >= ' ' && ch != '\u{7f}' {
                    let old = match ctx.ui.get(id).map(|w| &w.kind) {
                        Some(Kind::TextBox(t)) | Some(Kind::Combo(t)) => t.clone(), _ => String::new(),
                    };
                    ctx.ui.set_text(id, &format!("{}{}", old, ch));
                }
                InvalidateRect(hwnd, std::ptr::null(), FALSE);
            }
            return 0;
        }
        WM_ACTIVATE => {
            if ctx.hide_on_deactivate && (wparam & 0xFFFF) == 0 {
                ShowWindow(hwnd, SW_HIDE);
                app::remove_mouse_hook();
                app::check_and_close_overflow();
                app::app().last_menu_close_tick = app::now_ms();
                app::set_tooltip_brightness();
            }
            return 0;
        }
        WM_GETMINMAXINFO => {
            let mm = &mut *(lparam as *mut MINMAXINFO);
            if ctx.min_w > 0 {
                mm.ptMinTrackSize.x = ctx.min_w;
                mm.ptMinTrackSize.y = ctx.min_h;
            }
            // 对应 ResizeThumb_DragDelta 的 min/maxWidth：用户拖拽不能超过 150%
            if ctx.max_w > 0 {
                mm.ptMaxTrackSize.x = ctx.max_w;
                mm.ptMaxTrackSize.y = ctx.max_h;
            }
            return 0;
        }
        WM_SIZE => {
            let w = loword(lparam as usize); let h = hiword(lparam as usize);
            theme::apply_round_region(hwnd, w, h, s(8));
            // 对应各窗口的 Window_SizeChanged：持久化尺寸
            let sc = app::app().scale;
            let lw = (w as f64 / sc).round();
            let lh = (h as f64 / sc).round();
            let a = app::app();
            match ctx.kind {
                WindowKind::ControlPanel => {
                    a.settings.control_panel_width = lw; a.settings.control_panel_height = lh;
                    app::trigger_save();
                }
                WindowKind::ScheduleDialog => {
                    a.settings.schedule_dialog_width = lw; a.settings.schedule_dialog_height = lh;
                    app::trigger_save();
                }
                WindowKind::MinDialog => {
                    a.settings.min_brightness_dialog_width = lw; a.settings.min_brightness_dialog_height = lh;
                    app::trigger_save();
                }
                WindowKind::InfoWindow => {
                    a.settings.app_info_window_width = lw; a.settings.app_info_window_height = lh;
                    app::trigger_save();
                }
                WindowKind::Warning => {
                    a.settings.warning_message_width = lw; a.settings.warning_message_height = lh;
                    app::trigger_save();
                }
                _ => {}
            }
            relayout(ctx, w, h);
            return 0;
        }
        WM_MOVE => {
            // 对应 Window_LocationChanged：持久化窗口位置
            if ctx.kind == WindowKind::ControlPanel {
                let mut r = RECT::default();
                GetWindowRect(hwnd, &mut r);
                let a = app::app();
                a.settings.control_panel_left = r.left as f64;
                a.settings.control_panel_top = r.top as f64;
                app::trigger_save();
            }
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        WM_CLOSE => {
            if ctx.kind == WindowKind::ControlPanel { app::hide_control_panel(true); }
            else {
                ShowWindow(hwnd, SW_HIDE);
                if matches!(ctx.kind, WindowKind::ScheduleDialog | WindowKind::MinDialog
                    | WindowKind::InfoWindow | WindowKind::Warning) { app::end_modal(); }
            }
            return 0;
        }
        WM_NCDESTROY => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(p));
            return 0;
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn relayout(ctx: &mut WindowCtx, pw: i32, ph: i32) {
    let sc = app::app().scale;
    let w = (pw as f64 / sc).round() as i32;
    let h = (ph as f64 / sc).round() as i32;
    match ctx.kind {
        WindowKind::ControlPanel => panels::layout_control_panel(ctx, w, h),
        WindowKind::TrayMenu => panels::layout_tray_menu(ctx, w, h),
        WindowKind::ScheduleDialog => dialogs::layout_schedule(ctx, w, h),
        WindowKind::MinDialog => dialogs::layout_min(ctx, w, h),
        WindowKind::InfoWindow => dialogs::layout_info(ctx, w, h),
        WindowKind::Warning => dialogs::layout_warning(ctx, w, h),
        WindowKind::Controller => {}
    }
}

/// 供各窗口在切换 Tab / 数据变化后重建控件树。
pub fn relayout_public(ctx: &mut WindowCtx, w: i32, h: i32) { relayout(ctx, w, h); }

fn dispatch(ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    if ev == Event::None { return; }
    match ctx.kind {
        WindowKind::ControlPanel => panels::on_control_panel(ctx, ev, hwnd),
        WindowKind::TrayMenu => panels::on_tray_menu(ctx, ev, hwnd),
        WindowKind::ScheduleDialog => dialogs::on_schedule(ctx, ev, hwnd),
        WindowKind::MinDialog => dialogs::on_min(ctx, ev, hwnd),
        WindowKind::InfoWindow => dialogs::on_info(ctx, ev, hwnd),
        WindowKind::Warning => dialogs::on_warning(ctx, ev, hwnd),
        WindowKind::Controller => {}
    }
    unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE); }
}

pub fn hide(hwnd: HWND) { if hwnd != NULL { unsafe { ShowWindow(hwnd, SW_HIDE); } } }
pub fn show(hwnd: HWND) { if hwnd != NULL { unsafe { ShowWindow(hwnd, SW_SHOW); SetForegroundWindow(hwnd); } } }

/// 便捷构造控件
pub fn mk(id: u32, x: i32, y: i32, cw: i32, ch: i32, kind: Kind) -> Widget {
    Widget::new(id, RECT::new(s(x), s(y), s(x + cw), s(y + ch)), kind)
}
