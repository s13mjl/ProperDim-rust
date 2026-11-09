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
pub const ID_SCHEDULE_SCROLL: u32 = 55;
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
    // 对应 Window_Loaded：宽度上限 525、强制 350/200 宽高比
    let cw = a.settings.control_panel_width.min(525.0) as i32;
    let ch = (cw as f64 / (350.0 / 200.0)).round() as i32;
    let (sw, sh) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let (vx, vy, vw, vh) = unsafe {
        (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN),
         GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN))
    };
    let (left, top) = (a.settings.control_panel_left, a.settings.control_panel_top);
    // 对应 Window_Loaded 的位置合法性校验（防止窗口跑到屏幕外）
    let valid = left >= 0.0 && top >= 0.0
        && left < (if vw > 0 { vw } else { sw }) as f64 - 50.0
        && top < (if vh > 0 { vh } else { sh }) as f64 - 50.0;
    let (x, y) = if valid { (left as i32, top as i32) }
        else { (vx + sw / 2 - s(cw) / 2, vy + sh / 2 - s(ch) / 2) };
    ui::create_window(&cls, "ProperDim Control Panel", x, y, s(cw), s(ch),
        WS_EX_TOOLWINDOW, ctx, false)
}

/// 对应 `ControlPanel.RefreshList()`：计算未来 7 天内下次触发时间并据此排序。
fn sorted_schedule_indices(list: &[config::DimSchedule]) -> Vec<usize> {
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st); }
    let now_m = (st.wHour as i64) * 60 + st.wMinute as i64;
    let dow = st.wDayOfWeek as usize;
    let mut keyed: Vec<(i64, usize)> = list.iter().enumerate().map(|(i, sc)| {
        let mut key = i64::MAX;
        for back in 0..=7i32 {
            let d = ((dow as i32 + back).rem_euclid(7)) as usize;
            if sc.days.contains(config::DimSchedule::short_days()[d]) {
                let cand = back as i64 * 1440 + (sc.time.0 as i64) * 60 + sc.time.1 as i64;
                if cand > now_m { key = cand; break; }
            }
        }
        (key, i)
    }).collect();
    keyed.sort_by_key(|&(k, i)| (k, i));
    keyed.into_iter().map(|(_, i)| i).collect()
}

/// 对应 `HandleClosure`：关闭时把 Tab 归零、日程列表滚回顶部。
pub fn reset_panel_state(hwnd: HWND) {
    if hwnd == NULL { return; }
    unsafe { CURRENT_TAB = 0; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(hwnd, &mut cr); }
    if let Some(ctx) = unsafe { (ui::get_ctx(hwnd) as *mut WindowCtx).as_mut() } {
        ctx.ui.scroll_offset = 0;
        ctx.ui.selected_row = None;
        ui::relayout_public(ctx, cr.width(), cr.height());
        unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE); }
    }
}

/// 对应 ScheduleEvaluated / ScheduleTriggered 订阅的 `RefreshList()`。
pub fn refresh_schedule_list(hwnd: HWND) {
    if hwnd == NULL { return; }
    if unsafe { CURRENT_TAB } != 1 { return; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(hwnd, &mut cr); }
    if let Some(ctx) = unsafe { (ui::get_ctx(hwnd) as *mut WindowCtx).as_mut() } {
        let (scroll, sel, focus, capture) =
            (ctx.ui.scroll_offset, ctx.ui.selected_row, ctx.ui.focus, ctx.ui.capture);
        ui::relayout_public(ctx, cr.width(), cr.height());
        ctx.ui.scroll_offset = scroll;
        ctx.ui.selected_row = sel;
        ctx.ui.focus = focus;
        ctx.ui.capture = capture;
        unsafe { InvalidateRect(hwnd, std::ptr::null(), FALSE); }
    }
}

fn set_tab(tab: u32, ctx: &mut WindowCtx, hwnd: HWND) {
    unsafe { CURRENT_TAB = tab; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(hwnd, &mut cr); }
    ui::relayout_public(ctx, cr.width(), cr.height());
}

fn row_real_index(ctx: &WindowCtx, id: u32) -> Option<usize> {
    if id < ID_ROW_BASE { return None; }
    let base = ID_ROW_BASE + ((id - ID_ROW_BASE) / 10) * 10;
    ctx.ui.widgets.iter().find(|w| w.id == base)
        .and_then(|w| w.tag.strip_prefix("row"))
        .and_then(|s| s.parse::<usize>().ok())
}

fn relayout(ctx: &mut WindowCtx, hwnd: HWND) {
    let mut cr = RECT::default();
    unsafe { GetClientRect(hwnd, &mut cr); }
    ui::relayout_public(ctx, cr.width(), cr.height());
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
                Kind::Link("Adjust Minimum".into(), c::MUTED)));
        }
        1 => {
            let enabled = st.schedule_toggle;
            ctx.ui.widgets.push(mk(ID_SCHEDULE_CHECK, 10, 60, 170, 22,
                Kind::CheckBox("Enable Schedule".into(), enabled)));
            let mut add = mk(ID_ADD_SCHEDULE, w - 34, 60, 24, 24, Kind::Button("+".into()));
            add.enabled = enabled; // ToggleScheduleUI: 未启用时禁止交互
            ctx.ui.widgets.push(add);
            if st.saved_schedules.is_empty() {
                let mut t = mk(ID_NO_SCHEDULES, 10, 90, w - 20, h - 100,
                    Kind::Label("No Schedules".into(), 11, c::DIM));
                t.tag = "center".into();
                t.enabled = enabled;
                ctx.ui.widgets.push(t);
            } else {
                // 对应 RefreshList：按「下一次触发时间」排序，未来在上、无后续的排末尾
                let order = sorted_schedule_indices(&st.saved_schedules);
                let row_h = 20;
                let visible = ((h - 96) / row_h).max(1) as usize;
                let max_start = order.len().saturating_sub(visible);
                let start = (ctx.ui.scroll_offset as usize).min(max_start);
                ctx.ui.scroll_offset = start as i32;
                for (vi, &si) in order.iter().skip(start).take(visible).enumerate() {
                    let sc = &st.saved_schedules[si];
                    let y = 88 + vi as i32 * row_h;
                    let base = ID_ROW_BASE + vi as u32 * 10;
                    let tag = format!("row{}", si);
                    let mut del = mk(base, 10, y, 18, 18, Kind::Button("✕".into()));
                    del.tag = tag.clone(); del.enabled = enabled;
                    ctx.ui.widgets.push(del);
                    for (sub, text, x, cw) in [
                        (1u32, sc.display_time(st.use_military_time), 32i32, 60i32),
                        (2, sc.display(), 96, 45),
                        (3, if sc.days.is_empty() { "Every day".into() } else { sc.days.clone() }, 145, w - 150),
                    ] {
                        let mut item = mk(base + sub, x, y, cw, 18, Kind::Label(text, 11, c::LIGHT));
                        item.tag = tag.clone(); item.enabled = enabled;
                        ctx.ui.widgets.push(item);
                    }
                }
                // 对应 ScheduleScrollBar_PreviewMouse*：可拖动的列表滚动条
                if max_start > 0 {
                    let mut sb = mk(ID_SCHEDULE_SCROLL, w - 14, 88, 10, h - 98,
                        Kind::Slider { min: 0.0, max: max_start as f64,
                            value: start as f64, vertical: true });
                    sb.enabled = enabled;
                    ctx.ui.widgets.push(sb);
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
                ID_TAB_CONTROLS | ID_TAB_SCHEDULE | ID_TAB_HOTKEYS | ID_TAB_OPTIONS =>
                    set_tab(id - ID_TAB_CONTROLS, ctx, hwnd),
                ID_MIN_LINK => dialogs::show_min(),
                ID_ADD_SCHEDULE => dialogs::show_schedule(None),
                ID_CLEAR => dialogs::show_warning("Clear Schedule",
                    "You are about to completely wipe all of your scheduled events. Is this what you truly desire?",
                    false, "", dialogs::WarningAction::ClearSchedule),
                ID_RESYNC => dialogs::show_warning("Reset Brightness",
                    "You are about to reset your brightness settings. Your schedule will remain intact. Is this what you truly desire?",
                    false, "", dialogs::WarningAction::Resync),
                ID_INFO_LINK => dialogs::show_info(),
                ID_INC_TEXT | ID_DEC_TEXT => { /* 焦点由 Ui 维护；热键录入在 WM_KEYDOWN 中处理 */ }
                i if (ID_QUICK_BASE..ID_QUICK_BASE + 6).contains(&i) => {
                    let vals = [0.20, 0.33, 0.50, 0.66, 0.75, 1.00];
                    app::quick_set(vals[(i - ID_QUICK_BASE) as usize]);
                }
                i if i >= ID_ROW_BASE => {
                    let Some(real) = row_real_index(ctx, i) else { return; };
                    let sub = i - (ID_ROW_BASE + ((i - ID_ROW_BASE) / 10) * 10);
                    let len = app::app().settings.saved_schedules.len();
                    if real >= len { return; }
                    ctx.ui.selected_row = Some(real);
                    if sub == 0 {
                        app::app().settings.saved_schedules.remove(real);
                        app::trigger_save();
                        relayout(ctx, hwnd);
                    } else {
                        dialogs::show_schedule(Some(real));
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
                app::app().last_user_interaction = app::now_ms();
                app::app().settings.last_opacity = v;
                app::trigger_save();
                if ctx.ui.slider_click {
                    // 对应 Slider_ValueChanged 的非拖动分支：ApplyBrightnessAnimated(200ms)
                    app::apply_brightness(v, true, false, 200, false);
                } else {
                    app::apply_brightness(v, false, false, 0, false);
                }
            } else if id == ID_SCHEDULE_SCROLL {
                ctx.ui.scroll_offset = v.round() as i32;
                relayout(ctx, hwnd);
            }
        }
        Event::Wheel(id, delta) => {
            if (ID_TAB_CONTROLS..=ID_TAB_OPTIONS).contains(&id) {
                // 对应 TabsArea_MouseWheel：滚轮切换 Tab
                let cur = unsafe { CURRENT_TAB };
                let next = if delta > 0 { (cur + 4 - 1) % 4 } else { (cur + 1) % 4 };
                set_tab(next, ctx, hwnd);
            } else if id == ID_SLIDER {
                app::app().last_user_interaction = app::now_ms();
                let v = ctx.ui.value(ID_SLIDER);
                app::apply_brightness(v, false, false, 0, false);
            } else if unsafe { CURRENT_TAB } == 1 {
                // 对应 ScheduleList_PreviewMouseWheel：滚动日程列表
                let len = app::app().settings.saved_schedules.len() as i32;
                ctx.ui.scroll_offset = (ctx.ui.scroll_offset - delta.signum()).clamp(0, (len - 1).max(0));
                relayout(ctx, hwnd);
            }
        }
        Event::DoubleClick(id) => {
            if let Some(real) = row_real_index(ctx, id) {
                ctx.ui.selected_row = Some(real);
                dialogs::show_schedule(Some(real));
            }
        }
        Event::Key(_, vk) => {
            match vk {
                VK_LEFT | VK_RIGHT => {
                    // 对应 MainTabs_PreviewKeyDown
                    let cur = unsafe { CURRENT_TAB };
                    let next = if vk == VK_LEFT { (cur + 4 - 1) % 4 } else { (cur + 1) % 4 };
                    set_tab(next, ctx, hwnd);
                }
                VK_UP | VK_DOWN => {
                    let len = app::app().settings.saved_schedules.len();
                    if len > 0 {
                        let cur = ctx.ui.selected_row.unwrap_or(0) as i32;
                        ctx.ui.selected_row = Some((cur + if vk == VK_DOWN { 1 } else { -1 })
                            .clamp(0, len as i32 - 1) as usize);
                    }
                }
                VK_DELETE => {
                    if let Some(real) = ctx.ui.selected_row {
                        if real < app::app().settings.saved_schedules.len() {
                            app::app().settings.saved_schedules.remove(real);
                            app::trigger_save();
                            ctx.ui.selected_row = None;
                            relayout(ctx, hwnd);
                        }
                    }
                }
                VK_RETURN | VK_SPACE => {
                    if let Some(real) = ctx.ui.selected_row { dialogs::show_schedule(Some(real)); }
                }
                _ => {}
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
                app::app().last_user_interaction = app::now_ms();
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
    // 对应 HandleTrayMenuToggle：菜单显示时清空 ToolTip，隐藏时恢复
    app::tray_tip(app::app().hwnd_controller, "");
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
                // 对应 TrayMenuWindow.OnGlobalBrightnessChanged 的 300ms 防抖
                if app::now_ms() - app::app().last_user_interaction >= 300 {
                    ctx.ui.set_text(ID_MENU_VALUE, &pct);
                    ctx.ui.set_value(ID_MENU_SLIDER, v);
                }
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
        // 对应 IsVisibleChanged 隐藏分支
        app::app().last_menu_close_tick = app::now_ms();
        app::set_tooltip_brightness();
        app::check_and_close_overflow();
    } else { show_tray_menu(); }
}
