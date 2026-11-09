//! 1:1 对应 `ControlPanel.xaml(.cs)` 与 `TrayMenuWindow.xaml(.cs)`。
#![allow(dead_code, static_mut_refs)]

use crate::app;
use crate::config;
use crate::native::*;
use crate::theme::{c, Event, Kind};
use crate::ui::{self, dialogs, s, mk, WindowCtx, WindowKind};

// ---- 控件 ID（对应 XAML 的 x:Name / 事件名） ----
pub const ID_MINIMIZE: u32 = 20;
pub const ID_CLOSE: u32 = 21;
pub const ID_TAB_CONTROLS: u32 = 10;
pub const ID_TAB_SCHEDULE: u32 = 11;
pub const ID_TAB_HOTKEYS: u32 = 12;
pub const ID_TAB_OPTIONS: u32 = 13;
pub const ID_BRIGHTNESS_TEXT: u32 = 31;
pub const ID_BRIGHTNESS_VALUE: u32 = 32;
pub const ID_SLIDER: u32 = 30;
pub const ID_QUICK_BASE: u32 = 40;
pub const ID_MIN_LINK: u32 = 46;
pub const ID_SCHEDULE_CHECK: u32 = 50;
pub const ID_ADD_SCHEDULE: u32 = 51;
pub const ID_NO_SCHEDULES: u32 = 52;
pub const ID_ROW_BASE: u32 = 200;
pub const ID_HOTKEY_CHECK: u32 = 60;
pub const ID_INC_TEXT: u32 = 61;
pub const ID_DEC_TEXT: u32 = 62;
pub const ID_STARTUP_CHECK: u32 = 70;
pub const ID_TRAY_CHECK: u32 = 71;
pub const ID_SHOWSTARTUP_CHECK: u32 = 72;
pub const ID_SWAP_CHECK: u32 = 73;
pub const ID_CLEAR: u32 = 74;
pub const ID_RESYNC: u32 = 75;
pub const ID_INFO_LINK: u32 = 76;

pub const ID_MENU_SLIDER: u32 = 300;
pub const ID_MENU_VALUE: u32 = 301;
pub const ID_MENU_CONTROLS: u32 = 302;
pub const ID_MENU_EXIT: u32 = 303;

static mut CURRENT_TAB: u32 = 0;
static PANEL_CLASS: &str = "ProperDimRustControlPanel";
static MENU_CLASS: &str = "ProperDimRustTrayMenu";

/// 对应 `RegistryServices.cs`：HKCU\...\Run 下的 ProperDim 自启动项。
pub fn auto_start() -> bool {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = wide("ProperDim");
        let mut hk: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut hk) != 0 { return false; }
        let mut size: u32 = 0;
        let r = RegQueryValueExW(hk, name.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null_mut(), &mut size);
        RegCloseKey(hk);
        r == 0 && size > 0
    }
}
pub fn set_auto_start(v: bool) {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let name = wide("ProperDim");
        let mut hk: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_SET_VALUE, &mut hk) != 0 { return; }
        if v {
            if let Ok(exe) = std::env::current_exe() {
                let cmd = wide(&format!("\"{}\"", exe.display()));
                RegSetValueExW(hk, name.as_ptr(), 0, REG_SZ, cmd.as_ptr() as *const u8,
                    (cmd.len() * 2) as u32);
            }
        } else { RegDeleteValueW(hk, name.as_ptr()); }
        RegCloseKey(hk);
    }
}

fn register(cls: &str) -> Vec<u16> { ui::register_class(cls) }

// ---------------------------------------------------------------------------
// ControlPanel
// ---------------------------------------------------------------------------
pub fn create_control_panel(_instance: HINSTANCE) -> HWND {
    let cls = register(PANEL_CLASS);
    let mut ctx = WindowCtx::new(WindowKind::ControlPanel, "ProperDim", true);
    ctx.show_title = true;
    ctx.bg = c::BG;
    ctx.min_w = s(350);
    ctx.min_h = s(200);
    ctx.resizable = true;
    let a = app::app();
    let (lw, lh) = (a.settings.control_panel_width as i32, a.settings.control_panel_height as i32);
    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let (x, y) = if a.settings.control_panel_left >= 0.0 && a.settings.control_panel_top >= 0.0 {
        (a.settings.control_panel_left as i32, a.settings.control_panel_top as i32)
    } else { (sw / 2 - s(lw) / 2, sh / 2 - s(lh) / 2) };
    ui::create_window(&cls, "ProperDim Control Panel", x, y, s(lw), s(lh),
        WS_EX_TOOLWINDOW, ctx, false)
}

pub fn layout_control_panel(ctx: &mut WindowCtx, w: i32, h: i32) {
    let tab = unsafe { CURRENT_TAB };
    let a = app::app();
    let brightness = a.current_brightness;
    let st = a.settings.clone();
    ctx.ui.clear();

    // 标题栏右侧（对应 TitleBar 的 − / ✕）
    ctx.ui.widgets.push(mk(ID_MINIMIZE, w - 66, 0, 32, 30, Kind::CloseButton("−".into())));
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 34, 0, 32, 30, Kind::CloseButton("✕".into())));

    // Tab 行（对应 MainTabs / TabItem Header）
    let tabs = [("Controls", ID_TAB_CONTROLS), ("Schedule", ID_TAB_SCHEDULE),
                ("Hotkeys", ID_TAB_HOTKEYS), ("Options", ID_TAB_OPTIONS)];
    let tw = (w - 10) / 4;
    for (i, (label, id)) in tabs.iter().enumerate() {
        ctx.ui.widgets.push(mk(*id, 5 + i as i32 * tw, 31, tw - 2, 24,
            Kind::Tab((*label).into(), tab == i as u32)));
    }

    match tab {
        0 => {
            let pct = format!("{}%", (brightness * 100.0).round() as i32);
            let label_w = 118;
            ctx.ui.widgets.push(mk(ID_BRIGHTNESS_TEXT, (w - label_w - 46) / 2, 60, label_w, 20,
                Kind::Label("Brightness Level: ".into(), 14, c::WHITE)));
            ctx.ui.widgets.push(mk(ID_BRIGHTNESS_VALUE, (w - label_w - 46) / 2 + label_w, 60, 46, 20,
                Kind::Label(pct, 14, c::WHITE)));
            ctx.ui.widgets.push(mk(ID_SLIDER, 20, 82, w - 40, 20,
                Kind::Slider { min: 0.01, max: 1.0, value: brightness, vertical: false }));
            let quick = [("20%", 0.20), ("33%", 0.33), ("50%", 0.50),
                         ("66%", 0.66), ("75%", 0.75), ("100%", 1.00)];
            let bw = (w - 44) / 6;
            for (i, (label, _)) in quick.iter().enumerate() {
                ctx.ui.widgets.push(mk(ID_QUICK_BASE + i as u32, 22 + i as i32 * bw, 106, bw - 4, 24,
                    Kind::Button((*label).into())));
            }
            ctx.ui.widgets.push(mk(ID_MIN_LINK, (w - 190) / 2, 136, 190, 18,
                Kind::Link("Adjust Minimum Brightness".into(), c::MUTED)));
        }
        1 => {
            ctx.ui.widgets.push(mk(ID_SCHEDULE_CHECK, 10, 60, 170, 22,
                Kind::CheckBox("Enable Schedule".into(), st.schedule_toggle)));
            ctx.ui.widgets.push(mk(ID_ADD_SCHEDULE, w - 34, 60, 24, 24, Kind::Button("+".into())));
            if st.saved_schedules.is_empty() {
                ctx.ui.widgets.push(mk(ID_NO_SCHEDULES, 10, 90, w - 20, h - 100,
                    Kind::Label("No Schedules".into(), 11, c::DIM)));
                ctx.ui.widgets.last_mut().unwrap().tag = "center".into();
            } else {
                let row_h = 20;
                let max_rows = ((h - 92) / row_h).max(1).min(st.saved_schedules.len() as i32);
                for i in 0..max_rows {
                    let sc = &st.saved_schedules[i as usize];
                    let y = 88 + i as i32 * row_h;
                    let base = ID_ROW_BASE + i as u32 * 10;
                    ctx.ui.widgets.push(mk(base, 10, y, 18, 18, Kind::Button("✕".into())));
                    let del = ctx.ui.widgets.last_mut().unwrap();
                    del.tag = "delete".into();
                    ctx.ui.widgets.push(mk(base + 1, 32, y, 60, 18,
                        Kind::Label(sc.display_time(st.use_military_time), 11, c::LIGHT)));
                    ctx.ui.widgets.push(mk(base + 2, 96, y, 45, 18,
                        Kind::Label(sc.display(), 11, c::LIGHT)));
                    let days = if sc.days.is_empty() { "Every day".to_string() } else { sc.days.clone() };
                    ctx.ui.widgets.push(mk(base + 3, 145, y, w - 150, 18,
                        Kind::Label(days, 10, c::LIGHT)));
                    let row = ctx.ui.widgets.last_mut().unwrap();
                    row.tag = format!("row{}", i);
                }
            }
        }
        2 => {
            ctx.ui.widgets.push(mk(ID_HOTKEY_CHECK, 10, 60, 170, 22,
                Kind::CheckBox("Enable Hotkeys".into(), st.hotkeys_enabled)));
            ctx.ui.widgets.push(mk(100, 20, 88, 180, 18,
                Kind::Label("Increase Brightness:".into(), 11, c::WHITE)));
            ctx.ui.widgets.push(mk(ID_INC_TEXT, w - 140, 86, 130, 22,
                Kind::TextBox(st.increase_hotkey.clone())));
            if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "hotkey".into(); }
            ctx.ui.widgets.push(mk(101, 20, 116, 180, 18,
                Kind::Label("Decrease Brightness:".into(), 11, c::WHITE)));
            ctx.ui.widgets.push(mk(ID_DEC_TEXT, w - 140, 114, 130, 22,
                Kind::TextBox(st.decrease_hotkey.clone())));
            if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "hotkey".into(); }
            ctx.ui.widgets.push(mk(102, 20, 142, w - 40, 18,
                Kind::Label("Click a field, then press desired keys".into(), 11, c::MUTED)));
            if !st.hotkeys_enabled {
                for id in [ID_INC_TEXT, ID_DEC_TEXT] { ctx.ui.set_enabled(id, false); }
            }
        }
        _ => {
            let checks = [
                (ID_STARTUP_CHECK, "Run ProperDim when Windows starts", auto_start()),
                (ID_TRAY_CHECK, "Close button minimizes to system tray", st.close_to_tray),
                (ID_SHOWSTARTUP_CHECK, "Show Controls window on app startup", st.show_on_startup),
                (ID_SWAP_CHECK, "Swap tray icon left/right click buttons", st.swap_tray_icon_clicks),
            ];
            for (i, (id, label, checked)) in checks.iter().enumerate() {
                ctx.ui.widgets.push(mk(*id, 34, 60 + i as i32 * 21, w - 40, 20,
                    Kind::CheckBox((*label).into(), *checked)));
            }
            let by = h - 32;
            let bw = 110;
            ctx.ui.widgets.push(mk(ID_CLEAR, (w - bw * 2 - 20) / 2, by, bw, 24,
                Kind::RedButton("Clear Schedule".into())));
            ctx.ui.widgets.push(mk(ID_RESYNC, (w - bw * 2 - 20) / 2 + bw + 10, by, bw, 24,
                Kind::RedButton("Resync Displays".into())));
            ctx.ui.widgets.push(mk(ID_INFO_LINK, w - 58, h - 54, 50, 16,
                Kind::Link("Info...".into(), c::DIM)));
        }
    }
    ctx.ui.widgets.push(mk(999, w - 14, h - 14, 12, 12, Kind::ResizeGrip));
}

/// 1:1 对应 `ControlPanel.HotkeyTextBox_PreviewKeyDown()`：录制组合键并立即重新注册。
pub fn record_hotkey(ctx: &mut WindowCtx, id: u32, vk: u32, lparam: LPARAM) {
    let mut parts: Vec<String> = Vec::new();
    unsafe {
        if GetAsyncKeyState(VK_CONTROL) != 0 { parts.push("Ctrl".into()); }
        if GetAsyncKeyState(VK_MENU) != 0 { parts.push("Alt".into()); }
        if GetAsyncKeyState(VK_SHIFT) != 0 { parts.push("Shift".into()); }
        if GetAsyncKeyState(VK_LWIN) != 0 { parts.push("Win".into()); }
    }
    let raw_name = unsafe {
        let mut buf = [0u16; 64];
        let n = GetKeyNameTextW(lparam, buf.as_mut_ptr(), 64);
        if n > 0 { from_wide_buf(&buf) } else { String::new() }
    };
    let key = wpf_key_name(vk, &raw_name);
    if !key.is_empty() { parts.push(key); }
    let text = parts.join(" + ");
    if text.is_empty() { return; }
    ctx.ui.set_text(id, &text);
    let a = app::app();
    if id == ID_INC_TEXT { a.settings.increase_hotkey = text; }
    else if id == ID_DEC_TEXT { a.settings.decrease_hotkey = text; }
    config::save(&a.settings);
    let (inc, dec, en, hwnd) = (a.settings.increase_hotkey.clone(),
        a.settings.decrease_hotkey.clone(), a.settings.hotkeys_enabled, a.hwnd_controller);
    a.backend.register_hotkeys(hwnd, en, &inc, &dec);
    unsafe { InvalidateRect(a.hwnd_panel, std::ptr::null(), FALSE); }
}

/// 把虚拟键映射成原版 WPF `Key.ToString()` 的字符串（parse_hotkey 认识的形式）。
fn wpf_key_name(vk: u32, raw: &str) -> String {
    match vk {
        0x08 => "Backspace".into(), 0x09 => "Tab".into(), 0x0D => "Enter".into(),
        0x1B => "Escape".into(), 0x20 => "Space".into(),
        0x21 => "PageUp".into(), 0x22 => "PageDown".into(),
        0x23 => "End".into(), 0x24 => "Home".into(),
        0x25 => "Left".into(), 0x26 => "Up".into(), 0x27 => "Right".into(), 0x28 => "Down".into(),
        0x2D => "Insert".into(), 0x2E => "Delete".into(),
        0x70..=0x7B => format!("F{}", vk - 0x6F),
        0x41..=0x5A => ((vk as u8) as char).to_string(),
        0x30..=0x39 => format!("D{}", (vk as u8 as char)),
        0x60..=0x69 => format!("NumPad{}", vk - 0x60),
        0x10 | 0x11 | 0x12 | 0x5B | 0x5C => String::new(), // 纯修饰键不记
        _ => raw.trim().replace(' ', ""),
    }
}

pub fn on_control_panel(ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    match ev {
        Event::Click(id) => {
            match id {
                ID_MINIMIZE => unsafe { ShowWindow(hwnd, SW_HIDE); },
                ID_CLOSE => app::hide_control_panel(true),
                ID_TAB_CONTROLS | ID_TAB_SCHEDULE | ID_TAB_HOTKEYS | ID_TAB_OPTIONS => {
                    unsafe { CURRENT_TAB = id - ID_TAB_CONTROLS; }
                    let mut cr = RECT::default();
                    unsafe { GetClientRect(hwnd, &mut cr); }
                    ui::relayout_public(ctx, cr.width(), cr.height());
                }
                ID_MIN_LINK => dialogs::show_min(),
                ID_ADD_SCHEDULE => dialogs::show_schedule(None),
                ID_CLEAR => dialogs::show_warning("Clear Schedule", "Clear all scheduled events?", false, "", dialogs::WarningAction::ClearSchedule),
                ID_RESYNC => dialogs::show_warning("Reset Brightness", "Resync all displays and reset brightness?", false, "", dialogs::WarningAction::Resync),
                ID_INFO_LINK => dialogs::show_info(),
                ID_INC_TEXT | ID_DEC_TEXT => { /* 焦点由 Ui 维护；热键录入在 WM_KEYDOWN 中处理 */ }
                i if (ID_QUICK_BASE..ID_QUICK_BASE + 6).contains(&i) => {
                    let vals = [0.20, 0.33, 0.50, 0.66, 0.75, 1.00];
                    app::quick_set(vals[(i - ID_QUICK_BASE) as usize]);
                }
                i if i >= ID_ROW_BASE => {
                    let row = (i - ID_ROW_BASE) / 10;
                    let sub = (i - ID_ROW_BASE) % 10;
                    let a = app::app();
                    if sub == 0 && (row as usize) < a.settings.saved_schedules.len() {
                        a.settings.saved_schedules.remove(row as usize);
                        app::trigger_save();
                        let mut cr = RECT::default();
                        unsafe { GetClientRect(hwnd, &mut cr); }
                        ui::relayout_public(ctx, cr.width(), cr.height());
                    } else if (row as usize) < a.settings.saved_schedules.len() {
                        dialogs::show_schedule(Some(row as usize));
                    }
                }
                _ => {}
            }
        }
        Event::Check(id, v) => {
            let a = app::app();
            match id {
                ID_SCHEDULE_CHECK => a.settings.schedule_toggle = v,
                ID_HOTKEY_CHECK => a.settings.hotkeys_enabled = v,
                ID_STARTUP_CHECK => { set_auto_start(v); return; }
                ID_SHOWSTARTUP_CHECK => a.settings.show_on_startup = v,
                ID_TRAY_CHECK => a.settings.close_to_tray = v,
                ID_SWAP_CHECK => a.settings.swap_tray_icon_clicks = v,
                _ => return,
            }
            app::trigger_save();
            if id == ID_HOTKEY_CHECK {
                let a = app::app();
                let (inc, dec, en) = (a.settings.increase_hotkey.clone(), a.settings.decrease_hotkey.clone(), v);
                a.backend.register_hotkeys(a.hwnd_controller, en, &inc, &dec);
            }
            let mut cr = RECT::default();
            unsafe { GetClientRect(hwnd, &mut cr); }
            ui::relayout_public(ctx, cr.width(), cr.height());
        }
        Event::Value(id, v) => {
            if id == ID_SLIDER {
                app::app().settings.last_opacity = v;
                app::trigger_save();
                app::apply_brightness(v, false, false, 0, false);
            }
        }
        Event::Wheel(id, _d) => {
            if id == ID_SLIDER {
                let v = ctx.ui.value(ID_SLIDER);
                app::apply_brightness(v, false, false, 0, false);
            }
        }
        Event::Text(..) | Event::None => {}
    }
}

// ---------------------------------------------------------------------------
// TrayMenuWindow / QuickDim
// ---------------------------------------------------------------------------
pub fn create_tray_menu() -> HWND {
    let cls = register(MENU_CLASS);
    let mut ctx = WindowCtx::new(WindowKind::TrayMenu, "", false);
    ctx.bg = c::BG;
    ctx.hide_on_deactivate = true;
    let (w, h) = (s(140), s(240));
    let h2 = ui::create_window(&cls, "ProperDim Menu", 0, 0, w, h,
        WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED, ctx, false);
    h2
}

pub fn layout_tray_menu(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let v = app::app().current_brightness;
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_MENU_VALUE - 100, 0, 6, w, 22,
        Kind::BoldLabel("QuickDim".into(), 12, c::WHITE)));
    if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "center".into(); }
    // 用 DT_CENTER 的 Label 由主题绘制；这里保持与 XAML 一致的文本
    ctx.ui.widgets.push(mk(ID_MENU_SLIDER, (w - 60) / 2, 30, 60, 120,
        Kind::Slider { min: 0.01, max: 1.0, value: v, vertical: true }));
    ctx.ui.widgets.push(mk(ID_MENU_VALUE, (w - 60) / 2, 152, 60, 20,
        Kind::Label(format!("{}%", (v * 100.0).round() as i32), 14, c::WHITE)));
    if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "center".into(); }
    ctx.ui.widgets.push(mk(310, 8, 178, w - 16, 1, Kind::Separator));
    ctx.ui.widgets.push(mk(ID_MENU_CONTROLS, 4, 182, w - 8, 26, Kind::Button("Controls".into())));
    ctx.ui.widgets.push(mk(ID_MENU_EXIT, 4, 210, w - 8, 26, Kind::Button("Exit".into())));
}

pub fn on_tray_menu(_ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    match ev {
        Event::Click(id) => match id {
            ID_MENU_CONTROLS => { unsafe { ShowWindow(hwnd, SW_HIDE); } app::remove_mouse_hook(); app::show_control_panel(); }
            ID_MENU_EXIT => { unsafe { ShowWindow(hwnd, SW_HIDE); } app::remove_mouse_hook(); app::shutdown_app(); }
            _ => {}
        },
        Event::Value(id, v) => {
            if id == ID_MENU_SLIDER {
                app::app().settings.last_opacity = v;
                app::trigger_save();
                app::apply_brightness(v, false, false, 0, false);
            }
        }
        Event::Wheel(id, _d) => {
            if id == ID_MENU_SLIDER { let v = app::app().current_brightness; app::apply_brightness(v, false, false, 0, false); }
        }
        _ => {}
    }
}

pub fn show_tray_menu() {
    let a = app::app();
    if a.hwnd_tray_menu == NULL { a.hwnd_tray_menu = create_tray_menu(); }
    let h = a.hwnd_tray_menu;
    if h == NULL { return; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(h, &mut cr); }
    if let Some(ctx) = unsafe { (ui::get_ctx(h) as *mut WindowCtx).as_mut() } {
        ui::relayout_public(ctx, cr.width(), cr.height());
    }
    unsafe {
        // 对应 GetTrayIconScreenPosition + TrayMenuWindow 的任务栏方位定位
        let icon = app::tray_icon_position();
        let mon = MonitorFromPoint(icon, MONITOR_DEFAULTTONEAREST);
        let mut mi = MONITORINFOEXW::default();
        mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        let (work, screen) = if mon != NULL && GetMonitorInfoW(mon, &mut mi) != 0 {
            (mi.rcWork, mi.rcMonitor)
        } else {
            (RECT::new(0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)),
             RECT::new(0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)))
        };
        let (cw, ch) = (cr.width(), cr.height());
        let (mut x, mut y) = if work.top > screen.top {
            (icon.x - cw / 2, work.top)
        } else if work.left > screen.left {
            (work.left, icon.y - ch / 2)
        } else if work.right < screen.right {
            (work.right - cw, icon.y - ch / 2)
        } else {
            (icon.x - cw / 2, work.bottom - ch)
        };
        x = x.clamp(work.left, (work.right - cw).max(work.left));
        y = y.clamp(work.top, (work.bottom - ch).max(work.top));
        SetWindowPos(h, HWND_TOPMOST, x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE);
        ShowWindow(h, SW_SHOWNA);
    }
    app::install_mouse_hook();
}

/// 亮度变化时同步滑块与数值标签（对应 XAML 的 `{Binding ElementName=...Slider, Path=Value}`）。
pub fn sync_brightness(hwnd: HWND, v: f64) {
    if hwnd == NULL { return; }
    let pct = format!("{}%", (v * 100.0).round() as i32);
    if let Some(ctx) = unsafe { (ui::get_ctx(hwnd) as *mut WindowCtx).as_mut() } {
        match ctx.kind {
            WindowKind::ControlPanel => {
                ctx.ui.set_text(ID_BRIGHTNESS_VALUE, &pct);
                ctx.ui.set_value(ID_SLIDER, v);
            }
            WindowKind::TrayMenu => {
                ctx.ui.set_text(ID_MENU_VALUE, &pct);
                ctx.ui.set_value(ID_MENU_SLIDER, v);
            }
            _ => {}
        }
    }
}

pub fn toggle_tray_menu() {
    let a = app::app();
    let now = app::now_ms();
    if now - a.last_toggle_tick < 300 { return; }
    a.last_toggle_tick = now;
    if now - a.last_menu_close_tick < 250 { return; }
    if a.hwnd_tray_menu != NULL && unsafe { IsWindowVisible(a.hwnd_tray_menu) } != 0 {
        unsafe { ShowWindow(a.hwnd_tray_menu, SW_HIDE); }
        app::remove_mouse_hook();
    } else { show_tray_menu(); }
}
