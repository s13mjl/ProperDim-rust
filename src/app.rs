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
    /// 大尺寸图标（对应 AppInfoWindow 里 65×65 的 Image）。
    pub icon_large: HICON,
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
    /// 对应 ControlPanel._isPreviewActive：预览期间阻止滑块被全局亮度回写覆盖。
    pub preview_isolated: bool,
    /// 对应 ControlPanel._lastUserInteraction（500ms 内不强制同步滑块）。
    pub last_user_interaction: u64,
    pub save_pending: bool,
    pub edit_schedule_index: Option<usize>,
    pub warning_action: WarningAction,
    pub warning_result: Option<bool>,
    pub anim: AnimState,
    /// 对应 `MainWindow._isUpdatingFromAnimator`：为 true 时表示本次 ApplyBrightness 来自动画帧。
    pub anim_updating: bool,
    pub monitors: Vec<MonitorInfo>,
    pub tray_added: bool,
    pub extra: ExtraApis,
    pub hook: isize,
    pub wts_registered: bool,
    /// 模态对话框嵌套计数（对应 ShowDialog）：>0 时禁用主控制面板。
    pub modal_depth: u32,
}

#[derive(Default)]
pub struct AnimState {
    pub active: bool,
    pub from: f64,
    pub to: f64,
    /// 高精度起始时刻（QPC）。用 GetTickCount64 只有 ~15.6ms 分辨率，
    /// 相邻两帧会算出同一个进度，导致动画一半的帧是「重复值」（肉眼看着一顿一顿）。
    pub start: i64,
    pub duration_ms: u64,
    pub linear: bool,
    pub ignore_minimum: bool,
}

static mut APP: *mut App = std::ptr::null_mut();

pub fn init(a: App) { unsafe { APP = Box::into_raw(Box::new(a)); } }
pub fn app() -> &'static mut App { unsafe { &mut *APP } }
pub fn has_app() -> bool { unsafe { !APP.is_null() } }

pub fn now_ms() -> u64 { unsafe { GetTickCount64() } }

/// 从 exe 自身的 RT_GROUP_ICON（由 build.rs 生成）取最接近 `px` 的一帧。
/// 对应 csproj 的 `<ApplicationIcon>ProperDimIcon.ico</ApplicationIcon>`。
pub fn load_icon(px: i32) -> HICON {
    let hinst = unsafe { GetModuleHandleW(std::ptr::null()) };
    let h = unsafe { LoadImageW(hinst, 1 as *const u16, IMAGE_ICON, px, px, 0) };
    if h != NULL { return h as HICON; }
    // 资源缺失（例如未跑 build.rs）时退回内嵌 ico 的第一帧
    make_icon()
}

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
    tray_tip(a.hwnd_controller, &format!("亮度：{}%", (a.current_brightness * 100.0).round() as i32));
}

/// 原生 Win32 弹出菜单（对应 C# 右键菜单中的常规项；QuickDim 滑块另用自绘窗口）。
pub fn show_native_menu(hwnd: HWND) {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt);
        let m = CreatePopupMenu();
        if m == NULL { return; }
        let items: [(usize, &str); 4] = [
            (MENU_CONTROLS, "控制面板"),
            (MENU_QUICKDIM, "快速调光"),
            (MENU_INFO, "关于..."),
            (MENU_EXIT, "退出"),
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
/// 对应 `ScheduleEvaluated` / `ScheduleTriggered` 事件订阅的 `RefreshList()`。
pub fn refresh_panel_list() {
    let h = app().hwnd_panel;
    if h != NULL { ui::panels::refresh_schedule_list(h); }
}

/// 对应 `WarningMessage.ShowDialog` / 各对话框的 ShowDialog：禁用主窗口。
pub fn begin_modal() {
    let a = app();
    a.modal_depth += 1;
    if a.modal_depth == 1 && a.hwnd_panel != NULL {
        unsafe { EnableWindow(a.hwnd_panel, FALSE); }
    }
}
pub fn end_modal() {
    let a = app();
    a.modal_depth = a.modal_depth.saturating_sub(1);
    if a.modal_depth == 0 && a.hwnd_panel != NULL {
        unsafe { EnableWindow(a.hwnd_panel, TRUE); }
    }
}

/// 对应 `ControlPanel_IsVisibleChanged`：面板隐藏时关闭所有子窗口。
pub fn close_child_windows() {
    let a = app();
    let mut closed = 0u32;
    for h in [a.hwnd_schedule, a.hwnd_min, a.hwnd_info, a.hwnd_warning] {
        if h != NULL { unsafe { DestroyWindow(h); } closed += 1; }
    }
    a.hwnd_schedule = NULL;
    a.hwnd_min = NULL;
    a.hwnd_info = NULL;
    a.hwnd_warning = NULL;
    while closed > 0 && a.modal_depth > 0 { end_modal(); closed -= 1; }
}

pub fn show_control_panel() {
    let a = app();
    if a.hwnd_panel == NULL { a.hwnd_panel = ui::panels::create_control_panel(a.instance); }
    let h = a.hwnd_panel;
    if h == NULL { return; }
    // 对应 ShowControlPanel：每次显示都把 Tab 归零（Dispatcher.BeginInvoke(ApplicationIdle)）
    ui::panels::reset_panel_state(h);
    // 对应 ControlPanel_IsVisibleChanged / SyncSliderWithOpacity / OnActivated
    ui::panels::sync_brightness(h, a.current_brightness);
    refresh_panel_list();
    unsafe {
        // 先在「隐藏」状态把内容画到窗口表面，再显示。
        // 否则 ShowWindow 到首次 WM_PAINT 之间会露出未绘制的内容，看起来是闪一下。
        InvalidateRect(h, std::ptr::null(), TRUE);
        UpdateWindow(h);
        ShowWindow(h, SW_SHOW);
        SetForegroundWindow(h);
    }
}
pub fn toggle_control_panel() {
    let a = app();
    let now = now_ms();
    if now - a.last_toggle_tick < 300 { return; }
    a.last_toggle_tick = now;
    if a.hwnd_panel != NULL && unsafe { IsWindowVisible(a.hwnd_panel) } != 0 {
        unsafe { ShowWindow(a.hwnd_panel, SW_HIDE); }
        close_child_windows();
    } else { show_control_panel(); }
}
pub fn hide_control_panel(handle_closure: bool) {
    let a = app();
    // 对应 ControlPanel.HandleClosure：CloseToTray=false 时真正退出并解绑事件
    if handle_closure && !a.settings.close_to_tray {
        shutdown_app();
        return;
    }
    if a.hwnd_panel != NULL {
        unsafe { ShowWindow(a.hwnd_panel, SW_HIDE); }
        close_child_windows();
        if handle_closure { ui::panels::reset_panel_state(a.hwnd_panel); }
    }
}

// ---------------------------------------------------------------------------
// 亮度 / 动画
// ---------------------------------------------------------------------------
pub fn apply_brightness(logical: f64, animate: bool, ignore_minimum: bool, duration_ms: u64, linear: bool) {
    let a = app();
    // 对应 ApplyBrightness 第 1 步：不忽略最小值时逻辑亮度下限是 0.01（Math.Max(0.01, ...)）
    let l = if ignore_minimum { logical.clamp(0.0, 1.0) } else { logical.clamp(0.01, 1.0) };
    let min = a.settings.global_minimum;
    // 第 2 步：逻辑 → 硬件映射（activeFloor 只在忽略最小值时为 0）
    let floor = if ignore_minimum { 0.0 } else { min.clamp(0.0, 1.0) };
    let target_hardware = if ignore_minimum {
        l
    } else {
        floor + ((l - 0.01) / 0.99) * (1.0 - floor)
    };
    if animate && (target_hardware - a.last_hardware).abs() > 0.001 {
        // 对应：把当前「硬件」亮度折算回新的逻辑坐标系，动画从屏幕真实位置起步
        let mut from = if ignore_minimum {
            a.last_hardware
        } else if 1.0 - floor <= 0.0 {
            1.0
        } else {
            0.01 + ((a.last_hardware - floor) / (1.0 - floor)) * 0.99
        };
        from = from.clamp(0.0, 1.0);
        a.anim = AnimState {
            active: true, from, to: l, start: qpc_now(), duration_ms: duration_ms.max(1),
            linear, ignore_minimum,
        };
        // 动画由消息循环按截止时间驱动（见 main.rs）；WM_TIMER 会被系统合并，不能当帧时钟
        return;
    }
    // 对应 if (!_isUpdatingFromAnimator) _gammaAnimator.Stop();
    // 动画每帧回调时不能停掉正在跑的动画，否则只会应用第一帧
    if !a.anim_updating { a.anim.active = false; }
    a.current_brightness = l;
    a.last_hardware = target_hardware;
    a.backend.apply_brightness(l, min, ignore_minimum);
    // 对应：动画回调期间不刷 ToolTip；QuickDim 可见时也不刷
    let menu_visible = a.hwnd_tray_menu != NULL && unsafe { IsWindowVisible(a.hwnd_tray_menu) } != 0;
    // 拖动过程中（100ms 内有用户交互）不刷托盘 ToolTip：每次 NIM_MODIFY 要 0.2~2.4ms，
    // 拖动结束时会由 WM_LBUTTONUP 补一次
    let recently_touched = now_ms().saturating_sub(a.last_user_interaction) < 100;
    if !a.anim_updating && !menu_visible && !recently_touched { set_tooltip_brightness(); }
    if a.hwnd_panel != NULL {
        if !a.preview_isolated && now_ms() - a.last_user_interaction >= 500 {
            ui::panels::sync_brightness(a.hwnd_panel, l);
        }
        unsafe { InvalidateRect(a.hwnd_panel, std::ptr::null(), FALSE); }
    }
    if a.hwnd_tray_menu != NULL {
        if !a.preview_isolated { ui::panels::sync_brightness(a.hwnd_tray_menu, l); }
        unsafe { InvalidateRect(a.hwnd_tray_menu, std::ptr::null(), FALSE); }
    }
}

/// 动画是否在跑（消息循环据此决定等待超时）。
pub fn anim_active() -> bool { has_app() && app().anim.active }

pub fn anim_tick() {
    let a = app();
    if !a.anim.active { return; }
    let elapsed = qpc_ms(qpc_now() - a.anim.start);
    let t = (elapsed / a.anim.duration_ms as f64).min(1.0);
    let e = if a.anim.linear { t } else if t < 0.5 { 2.0 * t * t } else { 1.0 - (-2.0 * t + 2.0).powi(2) / 2.0 };
    let v = a.anim.from + (a.anim.to - a.anim.from) * e;
    let ignore = a.anim.ignore_minimum;
    let done = t >= 1.0;
    if done { a.anim.active = false; }
    a.anim_updating = true;
    apply_brightness(v, false, ignore, 0, false);
    app().anim_updating = false;
    if done {
        let a = app();
        apply_brightness(a.anim.to, false, ignore, 0, false);
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
    a.preview_isolated = true;
    a.backend.set_preview(v);
}
/// 对应 `ControlPanel.ApplyPreview(value, animate:false)`：拖动预览时连续改写，不重置还原点。
pub fn preview_value(v: f64) {
    let a = app();
    if !a.preview_active {
        a.preview_restore = a.current_brightness;
        a.preview_active = true;
    }
    a.preview_isolated = true;
    a.backend.set_preview(v);
}
pub fn stop_preview() {
    let a = app();
    if !a.preview_active { return; }
    a.preview_active = false;
    a.preview_isolated = false;
    // 对应 EndPreview：动态读取当前设置，热键在预览期间调过亮度也能被尊重
    let restore = a.settings.last_opacity;
    let min = a.settings.global_minimum;
    a.backend.restore_preview(restore, min);
    let logical = restore.clamp(0.01, 1.0);
    let floor = min.clamp(0.0, 1.0);
    a.current_brightness = logical;
    a.last_hardware = floor + ((logical - 0.01) / 0.99) * (1.0 - floor);
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
    // 对应 ScheduleEvaluated?.Invoke() -> RefreshList（无条件，每 10 秒重排）
    refresh_panel_list();
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
    // 对应 ScheduleTriggered?.Invoke() -> RefreshList
    refresh_panel_list();
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
        // 对应 ScheduleTriggered?.Invoke() -> RefreshList
        refresh_panel_list();
    } else {
        let v = a.settings.last_opacity;
        apply_brightness(v, animate, false, 1500, false);
    }
}

/// `--bench`：实测「一次亮度更新」各环节的真实耗时，避免继续凭猜优化。
/// 结果写到 exe 同目录 `bench.txt`。
pub fn bench() {
    use std::fmt::Write as _;
    let mut out = String::new();
    let n = 200usize;

    fn time_it<F: FnMut(usize)>(out: &mut String, name: &str, n: usize, mut f: F) {
        use std::fmt::Write as _;
        let mut sum = 0.0f64;
        let mut max = 0.0f64;
        for i in 0..n {
            let t = qpc_now();
            f(i);
            let ms = qpc_ms(qpc_now() - t);
            sum += ms;
            if ms > max { max = ms; }
        }
        let _ = writeln!(out, "{:<30} avg {:>7.3}  max {:>7.3}", name, sum / n as f64, max);
    }

    let _ = writeln!(out, "=== 每次调用耗时（{} 次，单位 ms）===", n);

    // A. 后端：逻辑→硬件 + gamma LUT + Magnification
    time_it(&mut out, "apply_brightness(后端)", n, |i| {
        let a = app();
        let g = a.settings.global_minimum;
        let v = if i % 2 == 0 { 0.600 } else { 0.605 };
        a.backend.apply_brightness(v, g, false);
    });
    // A2. 同值（走 0.001 早退，不写硬件）→ 与 A 的差值就是 SetDeviceGammaRamp 本身
    time_it(&mut out, "apply_brightness(同值早退)", n, |_| {
        let a = app();
        let g = a.settings.global_minimum;
        let v = 0.600;
        a.backend.apply_brightness(v, g, false);
    });
    // A3. 低于 50%（会走 MagSetFullscreenColorEffect）
    time_it(&mut out, "apply_brightness(<50%,含放大)", 50, |i| {
        let a = app();
        let g = a.settings.global_minimum;
        let v = if i % 2 == 0 { 0.400 } else { 0.405 };
        a.backend.apply_brightness(v, g, false);
    });
    // B. 托盘 ToolTip（Shell_NotifyIcon NIM_MODIFY）
    time_it(&mut out, "tray ToolTip(NIM_MODIFY)", n, |i| {
        let a = app();
        tray_tip(a.hwnd_controller, &format!("Brightness: {}%", 60 + (i % 2)));
    });
    // C. 面板整窗重绘（Invalidate + UpdateWindow 强制同步 WM_PAINT）
    let panel = app().hwnd_panel;
    if panel != NULL {
        time_it(&mut out, "panel 整窗重绘", n, |_| unsafe {
            InvalidateRect(panel, std::ptr::null(), TRUE);
            UpdateWindow(panel);
        });
    }
    // D. 保存防抖定时器
    time_it(&mut out, "trigger_save(SetTimer)", n, |_| {
        let a = app();
        unsafe { SetTimer(a.hwnd_controller, TIMER_SAVE, 1000, std::ptr::null()); }
    });

    // E. 动画帧间隔：跑一次 400ms 动画，采样亮度变化时刻
    apply_brightness(0.60, false, false, 0, false);
    apply_brightness(0.62, true, false, 400, false);
    let t0 = now_ms();
    let mut last = -1.0f64;
    let mut stamps: Vec<u64> = Vec::new();
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    let mut last_tick = qpc_now();
    while now_ms() - t0 < 700 {
        unsafe {
            while PeekMessageW(&mut msg, NULL, 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        // 与正式消息循环同样的驱动节拍
        if anim_active() && qpc_ms(qpc_now() - last_tick) >= 15.0 {
            anim_tick();
            last_tick = qpc_now();
        }
        let v = app().current_brightness;
        if (v - last).abs() > 1e-12 {
            stamps.push(now_ms() - t0);
            last = v;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let _ = writeln!(out, "
=== 动画帧（400ms，共 {} 帧）===", stamps.len());
    let mut deltas: Vec<u64> = stamps.windows(2).map(|w| w[1] - w[0]).collect();
    if !deltas.is_empty() {
        let show = deltas.iter().take(40).cloned().collect::<Vec<_>>();
        deltas.sort_unstable();
        let avg: f64 = deltas.iter().sum::<u64>() as f64 / deltas.len() as f64;
        let p50 = deltas[deltas.len() / 2];
        let p95 = deltas[((deltas.len() as f64) * 0.95) as usize % deltas.len()];
        let _ = writeln!(out, "帧间隔 ms: avg {:.1}  p50 {}  p95 {}  max {}",
            avg, p50, p95, deltas[deltas.len() - 1]);
        let _ = writeln!(out, "前 40 帧: {:?}", show);
    }

    let path = readme_path().parent().map(|p| p.join("bench.txt"))
        .unwrap_or_else(|| std::path::PathBuf::from("bench.txt"));
    let _ = std::fs::write(path, out);
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
        // TIMER_ANIM 不再使用：动画由消息循环驱动
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
    // 对应 ResetBrightness_Click 的 SyncSliderWithOpacity(1.0)：强制滑块回到 100%
    if a.hwnd_panel != NULL { ui::panels::sync_brightness(a.hwnd_panel, 1.0); }
}

pub fn shutdown_app() {
    let a = app();
    if a.is_shutting_down { return; }
    a.is_shutting_down = true;
    // 对应 ShutdownApp：先关掉 QuickDim 菜单，避免隐藏时回调再触碰已销毁对象
    if a.hwnd_tray_menu != NULL {
        unsafe { ShowWindow(a.hwnd_tray_menu, SW_HIDE); }
        remove_mouse_hook();
    }
    if a.tray_added { tray_remove(a.hwnd_controller); }
    config::save(&a.settings);
    a.backend.shutdown();
    unsafe { PostQuitMessage(0); }
}

pub fn open_url(url: &str) {
    let u = wide(url);
    let op = wide("open");
    let r = unsafe { ShellExecuteW(NULL, op.as_ptr(), u.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
    // 对应 Hyperlink_RequestNavigate 的失败提示
    if (r as isize) <= 32 {
        ui::dialogs::show_warning("错误", &format!("无法打开链接：{}", url),
            true, "OK", WarningAction::None);
    }
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
    let icon = load_icon(crate::ui::s(16));
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
    use crate::backend::StubBackend;

    /// 构造一个只跑亮度逻辑的最小 App（无窗口、Stub 后端）。
    fn test_app() {
        let a = App {
            settings: AppSettings::default(),
            backend: Box::new(StubBackend::new()),
            demo: true, icon: NULL, icon_large: NULL, scale: 1.0, instance: NULL,
            hwnd_controller: NULL, hwnd_panel: NULL, hwnd_tray_menu: NULL,
            hwnd_schedule: NULL, hwnd_min: NULL, hwnd_info: NULL, hwnd_warning: NULL,
            current_brightness: 1.0, last_hardware: 1.0, locked_hardware: 1.0,
            last_triggered: (-1, -1), last_hotkey_tick: 0, last_toggle_tick: 0,
            last_menu_close_tick: 0, is_shutting_down: false,
            preview_active: false, preview_restore: 1.0,
            preview_isolated: false, last_user_interaction: 0, save_pending: false,
            edit_schedule_index: None, warning_action: WarningAction::None,
            warning_result: None, anim: AnimState::default(), anim_updating: false,
            monitors: Vec::new(), tray_added: false,
            extra: ExtraApis {
                dwm_set: None, dpi_for_monitor: None, wts_register: None,
                wts_unregister: None, package_full_name: None,
                time_begin: None, time_end: None,
            },
            hook: NULL, wts_registered: false, modal_depth: 0,
        };
        init(a);
    }

    #[test]
    fn icon_resource_embedded() {
        // 由 build.rs 生成的 RT_GROUP_ICON(id=1) 必须能被 LoadImage 取到
        let h = unsafe {
            LoadImageW(GetModuleHandleW(std::ptr::null()), 1 as *const u16,
                IMAGE_ICON, 32, 32, 0)
        };
        assert!(h != 0, "exe 缺少图标资源，资源管理器里会显示默认图标");
        unsafe { DestroyIcon(h as HICON); }

        // 用资源管理器同款 API 再确认一次（它会从 exe 里挑第一个图标组）
        let exe = std::env::current_exe().expect("current_exe");
        let path = wide(&exe.to_string_lossy());
        let mut large: isize = 0;
        let mut small: isize = 0;
        let n = unsafe { ExtractIconExW(path.as_ptr(), 0, &mut large, &mut small, 1) };
        assert!(n > 0 && large != 0, "ExtractIconEx 取不到 exe 图标（n={}）", n);
        unsafe {
            if large != 0 { DestroyIcon(large as HICON); }
            if small != 0 { DestroyIcon(small as HICON); }
        }
    }

    /// 两个场景共用全局 APP，必须放在同一个测试里串行执行。
    #[test]
    fn brightness_pipeline() {
        // 1) 动画必须真正跑到终点，而不是停在第一帧
        test_app();
        apply_brightness(0.5, true, false, 150, false);
        assert!(app().anim.active, "动画应已启动");
        let start = now_ms();
        while app().anim.active && now_ms() - start < 2000 {
            std::thread::sleep(std::time::Duration::from_millis(16));
            anim_tick();
        }
        assert!(!app().anim.active, "动画必须在超时前结束");
        assert!((app().current_brightness - 0.5).abs() < 1e-6,
            "最终亮度应为 0.5，实际 {}", app().current_brightness);

        // 2) 传入 0.0 时逻辑亮度必须收敛到 0.01，硬件亮度落在 floor 上
        test_app();
        app().settings.global_minimum = 0.40;
        apply_brightness(0.0, false, false, 0, false);
        assert!((app().current_brightness - 0.01).abs() < 1e-9);
        assert!((app().last_hardware - 0.40).abs() < 1e-9,
            "硬件应为最低亮度 0.40，实际 {}", app().last_hardware);
    }

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
