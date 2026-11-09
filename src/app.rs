//! 对应 `App.xaml.cs` + `MainWindow.xaml.cs` 的应用级状态机：
//! 单实例、启动流程、托盘、原生菜单、定时器、亮度动画、计划调度、崩溃恢复。
#![allow(dead_code, static_mut_refs)]

use crate::backend::{Backend, MonitorInfo};
use crate::config::{self, AppSettings, DimSchedule};
use crate::native::*;
use crate::ui;
use crate::ui::dialogs::WarningAction;
use std::ffi::c_void;

static ICON_ICO: &[u8] = include_bytes!("../assets/icon.ico");
/// 对应 C# csproj 的 `Content Include="ProperDim_README.txt" CopyToOutputDirectory=Always`。
static README_TXT: &[u8] = include_bytes!("../ProperDim_README.txt");

pub const TIMER_SCHEDULE: usize = 1;
pub const TIMER_SAVE: usize = 2;
pub const TIMER_DEBOUNCE: usize = 3;
pub const TIMER_WATCHDOG: usize = 4;
pub const TIMER_ANIM: usize = 5;
pub const TIMER_SESSION_UNLOCK: usize = 6;
pub const TIMER_POWER_RESUME: usize = 7;

pub const MENU_CONTROLS: usize = 1001;
pub const MENU_QUICKDIM: usize = 1002;
pub const MENU_INFO: usize = 1003;
pub const MENU_EXIT: usize = 1004;

pub struct App {
    pub settings: AppSettings,
    pub backend: Box<dyn Backend>,
    pub demo: bool,
    pub icon: HICON,
    pub scale: f64,
    pub instance: HINSTANCE,
    pub hwnd_controller: HWND,
    pub hwnd_panel: HWND,
    pub hwnd_tray_menu: HWND,
    pub hwnd_schedule: HWND,
    pub hwnd_min: HWND,
    pub hwnd_info: HWND,
    pub hwnd_warning: HWND,
    pub current_brightness: f64,
    pub last_hardware: f64,
    pub locked_hardware: f64,
    pub last_triggered: (i32, i32),
    pub last_hotkey_tick: u64,
    pub last_toggle_tick: u64,
    pub last_menu_close_tick: u64,
    pub is_shutting_down: bool,
    pub preview_active: bool,
    pub preview_restore: f64,
    pub save_pending: bool,
    pub edit_schedule_index: Option<usize>,
    pub warning_action: WarningAction,
    pub warning_result: Option<bool>,
    pub anim: AnimState,
    pub monitors: Vec<MonitorInfo>,
    pub tray_added: bool,
    pub extra: ExtraApis,
    pub hook: isize,
    pub wts_registered: bool,
}

#[derive(Default)]
pub struct AnimState {
    pub active: bool,
    pub from: f64,
    pub to: f64,
    pub start: u64,
    pub duration_ms: u64,
    pub linear: bool,
    pub ignore_minimum: bool,
}

static mut APP: *mut App = std::ptr::null_mut();

pub fn init(a: App) { unsafe { APP = Box::into_raw(Box::new(a)); } }
pub fn app() -> &'static mut App { unsafe { &mut *APP } }
pub fn has_app() -> bool { unsafe { !APP.is_null() } }

pub fn now_ms() -> u64 { unsafe { GetTickCount64() } }

/// 从嵌入的 ico 中取第一帧（16×16 32bpp DIB）创建 HICON。
pub fn make_icon() -> HICON {
    if ICON_ICO.len() < 22 { return NULL; }
    let n = u16::from_le_bytes([ICON_ICO[4], ICON_ICO[5]]) as usize;
    if n == 0 { return NULL; }
    let e = 6usize;
    let sz = u32::from_le_bytes([ICON_ICO[e + 8], ICON_ICO[e + 9], ICON_ICO[e + 10], ICON_ICO[e + 11]]) as usize;
    let off = u32::from_le_bytes([ICON_ICO[e + 12], ICON_ICO[e + 13], ICON_ICO[e + 14], ICON_ICO[e + 15]]) as usize;
    if off + sz > ICON_ICO.len() { return NULL; }
    unsafe { CreateIconFromResourceEx(ICON_ICO[off..].as_ptr(), sz as u32, TRUE, 0x00030000, 16, 16, 0) }
}

// ---------------------------------------------------------------------------
// 托盘 / 原生菜单
// ---------------------------------------------------------------------------
/// 对应 H.NotifyIcon 的持久 GUID：托盘图标以 GUID 标识，Shell_NotifyIconGetRect 才能稳定定位。
pub const TRAY_GUID: GUID = GUID {
    data1: 0x9E5A_3C21, data2: 0x7B4D, data3: 0x4C8E,
    data4: [0xA1, 0x2F, 0x63, 0xD9, 0x0B, 0x44, 0x71, 0x0C],
};

pub fn tray_add(hwnd: HWND, icon: HICON) {
    unsafe {
        let mut d = NOTIFYICONDATAW::default();
        d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        d.hWnd = hwnd;
        d.uID = 1;
        d.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_GUID;
        d.guidItem = TRAY_GUID;
        d.uCallbackMessage = WM_APP_TRAY;
        d.hIcon = icon;
        let tip = wide("ProperDim");
        for (i, c) in tip.iter().take(127).enumerate() { d.szTip[i] = *c; }
        Shell_NotifyIconW(NIM_ADD, &mut d);
        Shell_NotifyIconW(NIM_SETVERSION, &mut d);
    }
    app().tray_added = true;
}
pub fn tray_remove(hwnd: HWND) {
    unsafe {
        let mut d = NOTIFYICONDATAW::default();
        d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        d.hWnd = hwnd; d.uID = 1; d.uFlags = NIF_GUID; d.guidItem = TRAY_GUID;
        Shell_NotifyIconW(NIM_DELETE, &mut d);
    }
    app().tray_added = false;
}
pub fn tray_tip(hwnd: HWND, text: &str) {
    unsafe {
        let mut d = NOTIFYICONDATAW::default();
        d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        d.hWnd = hwnd; d.uID = 1; d.uFlags = NIF_TIP | NIF_GUID; d.guidItem = TRAY_GUID;
        let t = wide(text);
        for (i, c) in t.iter().take(127).enumerate() { d.szTip[i] = *c; }
        Shell_NotifyIconW(NIM_MODIFY, &mut d);
    }
}
pub fn set_tooltip_brightness() {
    let a = app();
    tray_tip(a.hwnd_controller, &format!("Brightness: {}%", (a.current_brightness * 100.0).round() as i32));
}

/// 原生 Win32 弹出菜单（对应 C# 右键菜单中的常规项；QuickDim 滑块另用自绘窗口）。
pub fn show_native_menu(hwnd: HWND) {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt);
        let m = CreatePopupMenu();
        if m == NULL { return; }
        let items: [(usize, &str); 4] = [
            (MENU_CONTROLS, "Controls"),
            (MENU_QUICKDIM, "QuickDim"),
            (MENU_INFO, "Info..."),
            (MENU_EXIT, "Exit"),
        ];
        for (i, &(id, label)) in items.iter().enumerate() {
            if i == 3 {
                let sep = wide("");
                AppendMenuW(m, MF_SEPARATOR, 0, sep.as_ptr());
            }
            let t = wide(label);
            AppendMenuW(m, MF_STRING, id, t.as_ptr());
        }
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenuEx(m, TPM_RIGHTALIGN | TPM_RIGHTBUTTON | TPM_RETURNCMD | TPM_NONOTIFY,
            pt.x, pt.y, hwnd, std::ptr::null());
        DestroyMenu(m);
        match cmd as usize {
            MENU_CONTROLS => show_control_panel(),
            MENU_QUICKDIM => ui::panels::show_tray_menu(),
            MENU_INFO => ui::dialogs::show_info(),
            MENU_EXIT => shutdown_app(),
            _ => {}
        }
    }
}

pub fn handle_tray_msg(lparam: LPARAM) {
    let msg = (lparam as usize) & 0xFFFF;
    let a = app();
    let swap = a.settings.swap_tray_icon_clicks;
    match msg {
        0x0201 /* WM_LBUTTONDOWN */ | 0x0203 /* WM_LBUTTONDBLCLK */ => {
            if swap { ui::panels::toggle_tray_menu(); } else { toggle_control_panel(); }
        }
        0x0205 /* WM_RBUTTONUP */ => {
            if swap { toggle_control_panel(); } else { show_native_menu(a.hwnd_controller); }
        }
        0x0400 /* NIN_SELECT (键盘 Enter/Space) */ => {
            if swap { ui::panels::toggle_tray_menu(); } else { toggle_control_panel(); }
        }
        _ => {
            // 对应 MessageWindow.KeyboardEventReceived：Apps / Shift+F10 / Enter / Space
            unsafe {
                let apps = GetAsyncKeyState(VK_APPS);
                let shift = GetAsyncKeyState(VK_SHIFT);
                let f10 = GetAsyncKeyState(VK_F10);
                let enter = GetAsyncKeyState(VK_RETURN);
                let space = GetAsyncKeyState(VK_SPACE);
                let right = apps != 0 || (shift != 0 && f10 != 0);
                let action = enter != 0 || space != 0;
                if right {
                    if swap { toggle_control_panel(); } else { show_native_menu(a.hwnd_controller); }
                } else if action {
                    if swap { ui::panels::toggle_tray_menu(); } else { toggle_control_panel(); }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 窗口显示 / 生命周期
// ---------------------------------------------------------------------------
pub fn show_control_panel() {
    let a = app();
    if a.hwnd_panel == NULL { a.hwnd_panel = ui::panels::create_control_panel(a.instance); }
    let h = a.hwnd_panel;
    if h == NULL { return; }
    unsafe { ShowWindow(h, SW_SHOW); SetForegroundWindow(h); InvalidateRect(h, std::ptr::null(), FALSE); }
}
pub fn toggle_control_panel() {
    let a = app();
    let now = now_ms();
    if now - a.last_toggle_tick < 300 { return; }
    a.last_toggle_tick = now;
    if a.hwnd_panel != NULL && unsafe { IsWindowVisible(a.hwnd_panel) } != 0 {
        unsafe { ShowWindow(a.hwnd_panel, SW_HIDE); }
    } else { show_control_panel(); }
}
pub fn hide_control_panel(minimize_to_tray: bool) {
    let a = app();
    if a.hwnd_panel != NULL {
        unsafe { ShowWindow(a.hwnd_panel, SW_HIDE); }
        if !minimize_to_tray { /* 对应 CloseToTray=false：仍然只隐藏，由 Exit 菜单退出 */ }
    }
}

// ---------------------------------------------------------------------------
// 亮度 / 动画
// ---------------------------------------------------------------------------
pub fn apply_brightness(logical: f64, animate: bool, ignore_minimum: bool, duration_ms: u64, linear: bool) {
    let a = app();
    let l = logical.clamp(0.0, 1.0);
    if animate {
        let from = a.current_brightness;
        if (from - l).abs() > 0.001 {
            a.anim = AnimState {
                active: true, from, to: l, start: now_ms(), duration_ms: duration_ms.max(1),
                linear, ignore_minimum,
            };
            unsafe { SetTimer(a.hwnd_controller, TIMER_ANIM, 16, std::ptr::null()); }
            return;
        }
    }
    a.anim.active = false;
    a.current_brightness = l;
    let min = a.settings.global_minimum;
    a.backend.apply_brightness(l, min, ignore_minimum);
    a.last_hardware = if ignore_minimum { l } else {
        let floor = min.clamp(0.0, 1.0);
        floor + ((l.clamp(0.01, 1.0) - 0.01) / 0.99) * (1.0 - floor)
    };
    set_tooltip_brightness();
    if a.hwnd_panel != NULL {
        ui::panels::sync_brightness(a.hwnd_panel, l);
        unsafe { InvalidateRect(a.hwnd_panel, std::ptr::null(), FALSE); }
    }
    if a.hwnd_tray_menu != NULL {
        ui::panels::sync_brightness(a.hwnd_tray_menu, l);
        unsafe { InvalidateRect(a.hwnd_tray_menu, std::ptr::null(), FALSE); }
    }
}

pub fn anim_tick() {
    let a = app();
    if !a.anim.active { return; }
    let st = now_ms();
    let t = ((st - a.anim.start) as f64 / a.anim.duration_ms as f64).min(1.0);
    let e = if a.anim.linear { t } else if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
    let v = a.anim.from + (a.anim.to - a.anim.from) * e;
    let ignore = a.anim.ignore_minimum;
    let done = t >= 1.0;
    if done { a.anim.active = false; }
    apply_brightness(v, false, ignore, 0, false);
    if done {
        let a = app();
        apply_brightness(a.anim.to, false, ignore, 0, false);
        unsafe { KillTimer(a.hwnd_controller, TIMER_ANIM); }
    }
}

pub fn on_hotkey(id: i32) {
    let a = app();
    if !a.settings.hotkeys_enabled { return; }
    let now = now_ms();
    if now - a.last_hotkey_tick < 75 { return; }
    a.last_hotkey_tick = now;
    let base = if a.anim.active { a.anim.to } else { a.settings.last_opacity };
    let pct = (base * 100.0).round() as i32;
    let snapped = { let r = pct.rem_euclid(5); if r > 2 { pct + (5 - r) } else { pct - r } };
    let mut target = snapped;
    if id == HOTKEY_ID_UP { target += 5; } else if id == HOTKEY_ID_DOWN { target -= 5; }
    target = target.clamp(1, 100);
    let v = target as f64 / 100.0;
    app().settings.last_opacity = v;
    trigger_save();
    apply_brightness(v, true, false, 150, true);
}

pub fn quick_set(v: f64) {
    app().settings.last_opacity = v;
    trigger_save();
    apply_brightness(v, true, false, 200, false);
}

pub fn start_preview(v: f64) {
    let a = app();
    if !a.preview_active { a.preview_restore = a.current_brightness; }
    a.preview_active = true;
    a.backend.set_preview(v);
}
pub fn stop_preview() {
    let a = app();
    if !a.preview_active { return; }
    a.preview_active = false;
    let restore = a.preview_restore;
    let min = a.settings.global_minimum;
    a.backend.restore_preview(restore, min);
}

// ---------------------------------------------------------------------------
// 计划调度 / 保存 / 定时器
// ---------------------------------------------------------------------------
pub fn trigger_save() {
    let a = app();
    a.save_pending = true;
    unsafe { SetTimer(a.hwnd_controller, TIMER_SAVE, 1000, std::ptr::null()); }
}

fn local_now() -> SYSTEMTIME {
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st); }
    st
}

pub fn schedule_tick() {
    let a = app();
    if !a.settings.schedule_toggle { return; }
    let st = local_now();
    let (h, m) = (st.wHour as i32, st.wMinute as i32);
    if a.last_triggered == (h, m) { return; }
    let dow = st.wDayOfWeek as usize;
    let full = DimSchedule::full_days()[dow];
    let short = DimSchedule::short_days()[dow];
    let idx = a.settings.saved_schedules.iter().position(|s| {
        s.time.0 as i32 == h && s.time.1 as i32 == m &&
            (s.days.is_empty() || s.days.contains(full) || s.days.contains(short))
    });
    let Some(i) = idx else { return; };
    let a = app();
    a.last_triggered = (h, m);
    let v = a.settings.saved_schedules[i].brightness;
    a.settings.saved_schedules[i].last_triggered = format!("{:04}-{:02}-{:02}T{:02}:{:02}:00",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute);
    a.settings.last_opacity = v;
    config::save(&a.settings);
    apply_brightness(v, true, false, 200, false);
    if a.hwnd_panel != NULL {
        unsafe { InvalidateRect(a.hwnd_panel, std::ptr::null(), FALSE); }
    }
}

/// 公历日期 → 距 1970-01-01 的天数（Howard Hinnant 算法），用于比较 `DateTime`。
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = y - if m <= 2 { 1 } else { 0 };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = m as i32 + if m > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + d as i32 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    (era * 146097 + doe - 719468) as i64
}
/// 解析 C# `DateTime` 的 ISO 字符串为总分钟数；MinValue 返回 i64::MIN。
fn iso_minutes(s: &str) -> i64 {
    let n: Vec<i64> = s.split(|c: char| !c.is_ascii_digit())
        .filter(|x| !x.is_empty()).filter_map(|x| x.parse().ok()).collect();
    if n.len() < 5 { return i64::MIN; }
    if n[0] <= 1 { return i64::MIN; }
    days_from_civil(n[0] as i32, n[1] as u32, n[2] as u32) * 1440 + n[3] * 60 + n[4]
}
fn now_iso(st: &SYSTEMTIME) -> String {
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond)
}

/// 1:1 对应 `MainWindow.EvaluateMissedSchedules()`：回溯 7 天、逐条比对 LastTriggered。
pub fn evaluate_missed_schedules(animate: bool) {
    let st = local_now();
    let now_day = days_from_civil(st.wYear as i32, st.wMonth as u32, st.wDay as u32);
    let now_m = now_day * 1440 + (st.wHour as i64) * 60 + st.wMinute as i64;
    let a = app();
    if !a.settings.schedule_toggle || a.settings.saved_schedules.is_empty() {
        let v = a.settings.last_opacity;
        apply_brightness(v, animate, false, 1500, false);
        return;
    }
    let dow = st.wDayOfWeek as usize;
    let mut most_recent_target: i64 = i64::MIN;
    let mut most_recent_missed: Option<usize> = None;
    for i in 0..a.settings.saved_schedules.len() {
        let s = &mut a.settings.saved_schedules[i];
        // 自修复：防止新建 schedule 无限向后看
        if iso_minutes(&s.last_triggered) == i64::MIN { s.last_triggered = now_iso(&st); }
        if s.days.is_empty() { continue; }
        for back in 0..7i32 {
            let d = ((dow as i32 - back).rem_euclid(7)) as usize;
            if s.days.contains(DimSchedule::full_days()[d]) || s.days.contains(DimSchedule::short_days()[d]) {
                let target_m = (now_day - back as i64) * 1440
                    + (s.time.0 as i64) * 60 + s.time.1 as i64;
                if target_m <= now_m && target_m > most_recent_target {
                    most_recent_target = target_m;
                    most_recent_missed = if iso_minutes(&s.last_triggered) < target_m { Some(i) } else { None };
                }
                break;
            }
        }
    }
    let a = app();
    if let Some(i) = most_recent_missed {
        let v = a.settings.saved_schedules[i].brightness;
        a.settings.saved_schedules[i].last_triggered = now_iso(&st);
        a.settings.last_opacity = v;
        config::save(&a.settings);
        apply_brightness(v, animate, false, 1500, false);
        if a.hwnd_panel != NULL {
            unsafe { InvalidateRect(a.hwnd_panel, std::ptr::null(), FALSE); }
        }
    } else {
        let v = a.settings.last_opacity;
        apply_brightness(v, animate, false, 1500, false);
    }
}

pub fn handle_timer(id: usize) {
    match id {
        TIMER_SCHEDULE => schedule_tick(),
        TIMER_SAVE => {
            let a = app();
            if a.save_pending { a.save_pending = false; config::save(&a.settings); }
            unsafe { KillTimer(a.hwnd_controller, TIMER_SAVE); }
        }
        TIMER_DEBOUNCE => {
            let a = app();
            unsafe { KillTimer(a.hwnd_controller, TIMER_DEBOUNCE); }
            a.monitors = a.backend.refresh_monitors();
            let v = a.settings.last_opacity;
            apply_brightness(v, false, false, 0, false);
        }
        TIMER_WATCHDOG => app().backend.watchdog_tick(),
        TIMER_ANIM => anim_tick(),
        TIMER_SESSION_UNLOCK => {
            let a = app();
            unsafe { KillTimer(a.hwnd_controller, TIMER_SESSION_UNLOCK); }
            a.monitors = a.backend.refresh_monitors();
            a.last_hardware = a.locked_hardware;
            refresh_tray_icon();
            evaluate_missed_schedules(true);
        }
        TIMER_POWER_RESUME => {
            let a = app();
            unsafe { KillTimer(a.hwnd_controller, TIMER_POWER_RESUME); }
            a.monitors = a.backend.refresh_monitors();
        }
        _ => {}
    }
}
pub fn debounce_refresh() {
    let a = app();
    unsafe { SetTimer(a.hwnd_controller, TIMER_DEBOUNCE, 200, std::ptr::null()); }
}

pub fn resync_displays() {
    let a = app();
    a.backend.resync_displays();
    a.settings.last_opacity = 1.0;
    a.current_brightness = 1.0;
    config::save(&a.settings);
    apply_brightness(1.0, false, false, 0, false);
}

pub fn shutdown_app() {
    let a = app();
    if a.is_shutting_down { return; }
    a.is_shutting_down = true;
    if a.tray_added { tray_remove(a.hwnd_controller); }
    config::save(&a.settings);
    a.backend.shutdown();
    unsafe { PostQuitMessage(0); }
}

pub fn open_url(url: &str) {
    let u = wide(url);
    let op = wide("open");
    unsafe { ShellExecuteW(NULL, op.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL); }
}

// ---------------------------------------------------------------------------
// 1:1 补齐：帮助文件 / 托盘定位 / 溢出窗口 / 全局钩子 / 会话与电源
// ---------------------------------------------------------------------------
pub fn readme_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().ok();
    let dir = exe.and_then(|e| e.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    dir.join("ProperDim_README.txt")
}
/// 对应 csproj 的 `CopyToOutputDirectory=Always`：首次运行把内嵌帮助文件落到 exe 同目录。
pub fn ensure_readme() {
    let p = readme_path();
    if !p.exists() { let _ = std::fs::write(&p, README_TXT); }
}

pub fn tray_modify_icon(hwnd: HWND, icon: HICON) {
    unsafe {
        let mut d = NOTIFYICONDATAW::default();
        d.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        d.hWnd = hwnd; d.uID = 1; d.uFlags = NIF_ICON | NIF_GUID;
        d.guidItem = TRAY_GUID; d.hIcon = icon;
        Shell_NotifyIconW(NIM_MODIFY, &mut d);
    }
}
/// 对应 SessionUnlock 时重建托盘图标。
pub fn refresh_tray_icon() {
    let icon = make_icon();
    if icon == NULL { return; }
    let a = app();
    let old = a.icon;
    a.icon = icon;
    tray_modify_icon(a.hwnd_controller, icon);
    if old != NULL && old != icon { unsafe { DestroyIcon(old); } }
}

/// 对应 `MainWindow.GetTrayIconScreenPosition()`：Shell_NotifyIconGetRect + uID 重试，
/// 失败回退到光标位置。返回物理屏幕坐标。
pub fn tray_icon_position() -> POINT {
    let a = app();
    let hwnd = a.hwnd_controller;
    unsafe {
        let mut id = NOTIFYICONIDENTIFIER::default();
        id.cb_size = std::mem::size_of::<NOTIFYICONIDENTIFIER>() as u32;
        id.hwnd = hwnd; id.uid = 0; id.guid_item = TRAY_GUID;
        let mut r = RECT::default();
        let mut found = Shell_NotifyIconGetRect(&id, &mut r) == 0;
        if !found {
            // 对应原版的三组重试：uid + GUID、uid + 备用 GUID、uid + Guid.Empty
            for with_guid in [true, false] {
                for i in 0..=10u32 {
                    id.uid = i;
                    id.guid_item = if with_guid { TRAY_GUID } else { GUID::default() };
                    if Shell_NotifyIconGetRect(&id, &mut r) == 0 { found = true; break; }
                }
                if found { break; }
            }
        }
        if found {
            return POINT { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
        }
        let mut p = POINT::default();
        GetCursorPos(&mut p);
        p
    }
}

/// 对应 `CheckAndCloseOverflow`：菜单隐藏后若光标已离开托盘溢出窗口则关掉它。
pub fn check_and_close_overflow() {
    unsafe {
        let mut pos = POINT::default();
        GetCursorPos(&mut pos);
        for cls in ["NotifyIconOverflowWindow", "TopLevelWindowForOverflowDropShadow"] {
            let n = wide(cls);
            let hwnd = FindWindowW(n.as_ptr(), std::ptr::null());
            if hwnd != NULL && IsWindowVisible(hwnd) != 0 {
                let mut r = RECT::default();
                if GetWindowRect(hwnd, &mut r) != 0 {
                    if pos.x < r.left || pos.x > r.right || pos.y < r.top || pos.y > r.bottom {
                        ShowWindow(hwnd, SW_HIDE);
                    }
                }
            }
        }
    }
}

/// 1:1 对应 `TrayMenuWindow.HookCallback`：QuickDim 显示时全局滚轮调亮度并吞掉消息。
pub unsafe extern "system" fn mouse_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= HC_ACTION && wparam == WM_MOUSEWHEEL_HOOK as usize {
        let hs = &*(lparam as *const MSLLHOOKSTRUCT);
        let delta = ((hs.mouse_data >> 16) & 0xFFFF) as u16 as i16 as i32;
        let a = app();
        if a.hwnd_tray_menu != NULL && IsWindowVisible(a.hwnd_tray_menu) != 0 {
            let cur = (a.current_brightness * 100.0).round() as i32;
            let fine = GetAsyncKeyState(VK_CONTROL) != 0 || GetAsyncKeyState(0x02) != 0;
            let mut np = if fine {
                if delta > 0 { cur + 1 } else { cur - 1 }
            } else if delta > 0 {
                let n = ((cur + 4) / 5) * 5;
                if n == cur { n + 5 } else { n }
            } else { ((cur - 1) / 5) * 5 };
            np = np.clamp(1, 100);
            let v = np as f64 / 100.0;
            a.settings.last_opacity = v;
            trigger_save();
            apply_brightness(v, false, false, 0, false);
            return 1;
        }
    }
    let h = app().hook;
    CallNextHookEx(h, code, wparam, lparam)
}
pub fn install_mouse_hook() {
    let a = app();
    if a.hook != NULL { return; }
    let fp: unsafe extern "system" fn(i32, WPARAM, LPARAM) -> LRESULT = mouse_hook;
    unsafe { a.hook = SetWindowsHookExW(WH_MOUSE_LL, fp as *const c_void, NULL as HINSTANCE, 0); }
}
pub fn remove_mouse_hook() {
    let a = app();
    if a.hook != NULL {
        unsafe { UnhookWindowsHookEx(a.hook); }
        a.hook = NULL;
    }
}

pub fn register_session_notification(hwnd: HWND) {
    let a = app();
    if let Some(f) = a.extra.wts_register {
        unsafe { if f(hwnd, NOTIFY_FOR_THIS_SESSION) != 0 { a.wts_registered = true; } }
    }
}
pub fn unregister_session_notification(hwnd: HWND) {
    let a = app();
    if a.wts_registered {
        if let Some(f) = a.extra.wts_unregister { unsafe { f(hwnd); } }
        a.wts_registered = false;
    }
}

/// 对应 `SystemEvents_SessionSwitch`（Lock 记录 / Unlock 延迟 1 秒恢复）。
pub fn handle_session_change(wparam: WPARAM) {
    match wparam {
        WTS_SESSION_LOCK => { let a = app(); a.locked_hardware = a.last_hardware; }
        WTS_SESSION_UNLOCK => {
            let a = app();
            unsafe { SetTimer(a.hwnd_controller, TIMER_SESSION_UNLOCK, 1000, std::ptr::null()); }
        }
        _ => {}
    }
}
/// 对应 `SystemEvents_PowerModeChanged`。
pub fn handle_power(wparam: WPARAM) {
    match wparam {
        PBT_APMSUSPEND => { let a = app(); a.locked_hardware = a.last_hardware; }
        PBT_APMRESUMEAUTOMATIC | PBT_APMRESUMESUSPEND => {
            let a = app();
            unsafe { SetTimer(a.hwnd_controller, TIMER_POWER_RESUME, 1000, std::ptr::null()); }
        }
        _ => {}
    }
}
/// 对应 `App.OnSessionEnding` → EmergencyReset + Exit。
pub fn handle_end_session() {
    let a = app();
    config::save(&a.settings);
    a.backend.shutdown();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn civil_date_math() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(1970, 1, 2), 1);
        assert_eq!(days_from_civil(2026, 11, 9) - days_from_civil(2026, 11, 8), 1);
        assert_eq!(days_from_civil(2024, 2, 29) - days_from_civil(2024, 2, 28), 1);
    }
    #[test]
    fn iso_minutes_handles_minvalue() {
        assert_eq!(iso_minutes("0001-01-01T00:00:00"), i64::MIN);
        assert_eq!(iso_minutes("1970-01-01T00:00:00"), 0);
        assert_eq!(iso_minutes("1970-01-02T01:02:00"), 1502);
    }
}
