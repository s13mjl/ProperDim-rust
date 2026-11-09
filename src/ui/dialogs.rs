//! 1:1 对应 `ScheduleDialog` / `MinBrightnessDialog` / `AppInfoWindow` / `WarningMessage`。
#![allow(dead_code, static_mut_refs)]

use crate::app;
use crate::config::{self, DimSchedule};
use crate::native::*;
use crate::theme::{c, Event, Kind};
use crate::ui::{self, s, mk, WindowCtx, WindowKind};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum WarningAction { #[default] None, ClearSchedule, Resync }

// ---- 控件 ID ----
const ID_CLOSE: u32 = 10;
const ID_SCH_HOUR: u32 = 20;
const ID_SCH_MINUTE: u32 = 21;
const ID_SCH_AM: u32 = 22;
const ID_SCH_PM: u32 = 23;
const ID_SCH_24H: u32 = 24;
const ID_SCH_SLIDER: u32 = 25;
const ID_SCH_PREVIEW: u32 = 26;
const ID_SCH_DAY_BASE: u32 = 30;
const ID_SCH_ADD: u32 = 40;
const ID_MIN_SLIDER: u32 = 50;
const ID_MIN_PREVIEW: u32 = 51;
const ID_MIN_OK: u32 = 52;
const ID_MIN_CANCEL: u32 = 53;
const ID_INFO_LINK_BASE: u32 = 60;
const ID_INFO_COOL: u32 = 70;
const ID_INFO_HELP: u32 = 71;
const ID_WARN_YES: u32 = 80;
const ID_WARN_NO: u32 = 81;

const SCH_W: i32 = 320;
const SCH_H: i32 = 288;
const MIN_W: i32 = 300;
const MIN_H: i32 = 260;
const INFO_W: i32 = 320;
const INFO_H: i32 = 394;
const WARN_W: i32 = 300;
const WARN_H: i32 = 150;

static mut SCH_TIME: (u32, u32) = (12, 0);
static mut SCH_PM: bool = false;
static mut SCH_24H: bool = false;
static mut SCH_BRIGHTNESS: f64 = 1.0;
/// 对应 `DialogDimmerSlider.Minimum = ConfigManager.Settings.GlobalMinimum`。
static mut SCH_MIN: f64 = 0.30;
static mut SCH_DAYS: [bool; 7] = [true; 7];
/// UI 顺序 M,T,W,Th,F,S,Su → C# 的短日名（对应 `_shortDays[DayOfWeek]`）。
const UI_SHORT: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
static mut MIN_VALUE: f64 = 0.30;
static mut WARN_TITLE: String = String::new();
static mut WARN_MESSAGE: String = String::new();
static mut WARN_OK_ONLY: bool = false;
static mut WARN_CUSTOM: String = String::new();

fn center_window(hwnd: HWND, w: i32, h: i32) {
    unsafe {
        // 对应 WindowStartupLocation=CenterOwner：以控制面板所在显示器的工作区居中
        let owner = app::app().hwnd_panel;
        let (mut mx, mut my, mut mw, mut mh) = (0, 0,
            GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        if owner != NULL {
            let mon = MonitorFromWindow(owner, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFOEXW::default();
            mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
            if mon != NULL && GetMonitorInfoW(mon, &mut mi) != 0 {
                let r = mi.rcWork;
                mx = r.left; my = r.top; mw = r.right - r.left; mh = r.bottom - r.top;
            }
        }
        SetWindowPos(hwnd, HWND_TOP, mx + (mw - w) / 2, my + (mh - h) / 2,
            w, h, SWP_NOACTIVATE);
    }
}
/// 对应各窗口的 `ApplySavedSize()`：只有宽高都 >= MinWidth/MinHeight 才采用，
/// 采用时宽度取 min(保存宽度, 上限)，高度按固定宽高比回算（上游不允许自由拉伸）。
fn saved_size(kind: WindowKind) -> (i32, i32) {
    let st = &app::app().settings;
    let (sw, sh, min_w, min_h, ratio, cap): (f64, f64, f64, f64, f64, f64) = match kind {
        WindowKind::ScheduleDialog => (st.schedule_dialog_width, st.schedule_dialog_height, 320.0, 280.0, 320.0 / 290.0, 500.0),
        WindowKind::MinDialog => (st.min_brightness_dialog_width, st.min_brightness_dialog_height, 300.0, 260.0, 300.0 / 260.0, 450.0),
        WindowKind::InfoWindow => (st.app_info_window_width, st.app_info_window_height, 320.0, 394.0, 320.0 / 394.0, 480.0),
        WindowKind::Warning => (st.warning_message_width, st.warning_message_height, 300.0, 150.0, 300.0 / 150.0, 450.0),
        _ => (320.0, 280.0, 320.0, 280.0, 320.0 / 290.0, 500.0),
    };
    if sw >= min_w && sh >= min_h {
        let cw = sw.min(cap);
        (cw.round() as i32, (cw / ratio).round() as i32)
    } else {
        (min_w as i32, (min_w / ratio).round() as i32)
    }
}

fn show_or_create(slot: &mut HWND, cls_name: &str, title: &str, cw: i32, ch: i32, make: impl FnOnce() -> WindowCtx) {
    let cls = ui::register_class(cls_name);
    if *slot == NULL {
        *slot = ui::create_window(&cls, title, 0, 0, s(cw), s(ch), WS_EX_TOOLWINDOW, make(), false);
    }
    let h = *slot;
    if h == NULL { return; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(h, &mut cr); }
    if let Some(ctx) = unsafe { (ui::get_ctx(h) as *mut WindowCtx).as_mut() } {
        ui::relayout_public(ctx, cr.width(), cr.height());
    }
    center_window(h, s(cw), s(ch));
    app::begin_modal(); // 对应 ShowDialog()
    unsafe { ShowWindow(h, SW_SHOW); SetForegroundWindow(h); InvalidateRect(h, std::ptr::null(), FALSE); }
}

// ---------------------------------------------------------------------------
// ScheduleDialog / Event Scheduler
// ---------------------------------------------------------------------------
pub fn show_schedule(edit: Option<usize>) {
    let a = app::app();
    a.edit_schedule_index = edit;
    let st = a.settings.clone();
    unsafe {
        SCH_24H = st.use_military_time;
        SCH_MIN = st.global_minimum.clamp(0.01, 1.0);
        if let Some(i) = edit {
            if let Some(sc) = st.saved_schedules.get(i) {
                SCH_TIME = sc.time;
                SCH_PM = sc.time.0 >= 12;
                SCH_BRIGHTNESS = sc.brightness.max(SCH_MIN).min(1.0);
                // 对应 SetDayCheckboxes：兼容 "Mo" / "M" / "Monday" 等形式，按 ',' 与 ' ' 分割
                let parts: Vec<&str> = sc.days.split(|c| c == ',' || c == ' ')
                    .filter(|s| !s.is_empty()).collect();
                let aliases = [("Mo", "M", "Monday"), ("Tu", "T", "Tuesday"),
                    ("We", "W", "Wednesday"), ("Th", "", "Thursday"),
                    ("Fr", "F", "Friday"), ("Sa", "S", "Saturday"),
                    ("Su", "", "Sunday")];
                for k in 0..7 {
                    let (a1, a2, full) = aliases[k];
                    SCH_DAYS[k] = parts.iter().any(|p| *p == a1 || *p == full || (!a2.is_empty() && *p == a2));
                }
            }
        } else {
            // 对应 Add 模式默认值：当前时间 + Max(Minimum, LastOpacity)
            let mut now = SYSTEMTIME::default();
            GetLocalTime(&mut now);
            SCH_TIME = (now.wHour as u32, now.wMinute as u32);
            SCH_PM = SCH_TIME.0 >= 12;
            SCH_BRIGHTNESS = st.last_opacity.max(SCH_MIN).min(1.0);
            // 对应 string.IsNullOrEmpty(savedDays) → 全选
            let days_src = if st.last_schedule_days.trim().is_empty() {
                "Mo, Tu, We, Th, Fr, Sa, Su".to_string()
            } else { st.last_schedule_days.clone() };
            for (k, d) in UI_SHORT.iter().enumerate() {
                SCH_DAYS[k] = days_src.contains(d);
            }
        }
    }
    let hwnd = &mut app::app().hwnd_schedule as *mut HWND;
    let (cw, ch) = saved_size(WindowKind::ScheduleDialog);
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustSchedule", "日程设置", cw, ch,
        || { let mut c = WindowCtx::new(WindowKind::ScheduleDialog, "日程设置", true); c.bg = c::BG; c.resizable = true; c.min_w = s(320); c.min_h = s(280); c.max_w = s(500); c.max_h = s(453); c });
}

pub fn layout_schedule(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let (hour, minute) = unsafe { SCH_TIME };
    let (pm, is24, brightness, days) = unsafe { (SCH_PM, SCH_24H, SCH_BRIGHTNESS, SCH_DAYS) };
    let editing = app::app().edit_schedule_index.is_some();
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 32, 0, 32, 32, Kind::CloseButton("✕".into())));
    let mut t = mk(1, 0, 38, w, 18, Kind::Label("时间".into(), 12, c::LIGHT));
    t.tag = "center".into();
    ctx.ui.widgets.push(t);

    let hh = if is24 { hour } else { let x = hour % 12; if x == 0 { 12 } else { x } };
    ctx.ui.widgets.push(mk(ID_SCH_HOUR, 62, 58, 65, 24, Kind::Combo(format!("{:02}", hh))));
    let mut colon = mk(2, 127, 58, 12, 24, Kind::Label(":".into(), 13, c::WHITE));
    colon.tag = "center".into();
    ctx.ui.widgets.push(colon);
    ctx.ui.widgets.push(mk(ID_SCH_MINUTE, 139, 58, 65, 24, Kind::Combo(format!("{:02}", minute))));
    ctx.ui.widgets.push(mk(ID_SCH_AM, 210, 58, 42, 24, Kind::Radio("上午".into(), !pm && !is24)));
    ctx.ui.widgets.push(mk(ID_SCH_PM, 254, 58, 42, 24, Kind::Radio("下午".into(), pm && !is24)));
    if is24 {
        ctx.ui.set_visible(ID_SCH_AM, false);
        ctx.ui.set_visible(ID_SCH_PM, false);
    }

    ctx.ui.widgets.push(mk(ID_SCH_24H, 20, 88, 220, 20,
        Kind::CheckBox("使用 24 小时制".into(), is24)));

    let pct = format!("{}%", (brightness * 100.0).round() as i32);
    ctx.ui.widgets.push(mk(3, 66, 112, 108, 18, Kind::Label("亮度：".into(), 12, c::WHITE)));
    ctx.ui.widgets.push(mk(4, 172, 112, 46, 18, Kind::Label(pct, 12, c::WHITE)));
    let mut sld = mk(ID_SCH_SLIDER, 20, 134, 240, 24,
        Kind::Slider { min: unsafe { SCH_MIN }, max: 1.0, value: brightness, vertical: false });
    sld.tag = "fine".into(); // 对应 DialogDimmerSlider_PreviewMouseWheel：固定 ±1%
    ctx.ui.widgets.push(sld);
    ctx.ui.widgets.push(mk(ID_SCH_PREVIEW, 266, 136, 24, 20, Kind::PreviewButton));
    if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "preview".into(); }

    let mut days_label = mk(5, 0, 162, w, 18, Kind::Label("生效日".into(), 12, c::LIGHT));
    days_label.tag = "center".into();
    ctx.ui.widgets.push(days_label);
    let labels = ["一", "二", "三", "四", "五", "六", "日"];
    for i in 0..7 {
        ctx.ui.widgets.push(mk(ID_SCH_DAY_BASE + i, 10 + i as i32 * 43, 184, 40, 20,
            Kind::DayBox(labels[i as usize].into(), days[i as usize])));
    }
    // 对应 ValidateTime：时段冲突时把 Add 按钮改为提示文案并禁用（不是弹窗）
    let actual_hour = if is24 { hour } else if pm { (hour % 12) + 12 } else { hour % 12 };
    let cur_days: Vec<&str> = UI_SHORT.iter().enumerate()
        .filter(|(k, _)| days[*k]).map(|(_, d)| *d).collect();
    let conflict = app::app().settings.saved_schedules.iter().enumerate().any(|(i, sc)| {
        if sc.time != (actual_hour, minute) || Some(i) == app::app().edit_schedule_index { return false; }
        // 上游：时间相同且「星期集合有交集」才算冲突
        let parts: Vec<&str> = sc.days.split(|c| c == ',' || c == ' ')
            .filter(|s| !s.is_empty()).collect();
        parts.iter().any(|p| cur_days.contains(p))
    });
    let label = if conflict { "该时段已有日程".to_string() }
        else if editing { "更新日程".into() } else { "添加日程".into() };
    let bw = if conflict { 200 } else { 130 };
    let mut btn = mk(ID_SCH_ADD, (w - bw) / 2, 218, bw, 25, Kind::Button(label));
    btn.enabled = !conflict;
    ctx.ui.widgets.push(btn);
    ctx.ui.widgets.push(mk(999, w - 14, 274, 12, 12, Kind::ResizeGrip));
}

fn pick_number(hwnd: HWND, values: &[u32], current: u32) -> u32 {
    unsafe {
        let mut pt = POINT::default();
        GetCursorPos(&mut pt);
        let m = CreatePopupMenu();
        if m == NULL { return current; }
        for (i, v) in values.iter().enumerate() {
            let t = wide(&format!("{:02}", v));
            AppendMenuW(m, MF_STRING, i + 1, t.as_ptr());
        }
        let cmd = TrackPopupMenuEx(m, TPM_RIGHTALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
            pt.x, pt.y, hwnd, std::ptr::null());
        DestroyMenu(m);
        if cmd == 0 || cmd as usize > values.len() { current } else { values[cmd as usize - 1] }
    }
}

pub fn on_schedule(ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    match ev {
        Event::Click(id) => {
            match id {
                ID_CLOSE => { unsafe { ShowWindow(hwnd, SW_HIDE); } app::trigger_save(); app::end_modal(); },
                ID_SCH_HOUR => {
                    let (h, m) = unsafe { SCH_TIME };
                    let is24 = unsafe { SCH_24H };
                    let values: Vec<u32> = if is24 { (0..24).collect() } else { (1..=12).collect() };
                    let cur = if is24 { h } else { let x = h % 12; if x == 0 { 12 } else { x } };
                    let picked = pick_number(hwnd, &values, cur);
                    unsafe {
                        SCH_TIME = (if is24 { picked } else {
                            let pm = SCH_PM;
                            let mut hh = picked % 12;
                            if pm { hh += 12; }
                            if hh == 12 && !pm { hh = 0; }
                            hh
                        }, m);
                    }
                    relayout_hwnd(ctx, hwnd);
                }
                ID_SCH_MINUTE => {
                    let (h, _) = unsafe { SCH_TIME };
                    let values: Vec<u32> = (0..60).collect();
                    let m = pick_number(hwnd, &values, unsafe { SCH_TIME.1 });
                    unsafe { SCH_TIME = (h, m); }
                    relayout_hwnd(ctx, hwnd);
                }
                ID_SCH_AM | ID_SCH_PM => { unsafe { SCH_PM = id == ID_SCH_PM; } relayout_hwnd(ctx, hwnd); }
                ID_SCH_PREVIEW => app::start_preview(unsafe { SCH_BRIGHTNESS }),
                ID_SCH_ADD => add_schedule(ctx, hwnd),
                i if (ID_SCH_DAY_BASE..ID_SCH_DAY_BASE + 7).contains(&i) => {
                    let k = (i - ID_SCH_DAY_BASE) as usize;
                    unsafe { SCH_DAYS[k] = !SCH_DAYS[k]; }
                    relayout_hwnd(ctx, hwnd);
                }
                _ => {}
            }
        }
        Event::Check(id, v) => {
            if (ID_SCH_DAY_BASE..ID_SCH_DAY_BASE + 7).contains(&id) {
                // 对应 DayCheck_Changed -> ValidateTime()：即时刷新按钮的冲突状态
                relayout_hwnd(ctx, hwnd);
                return;
            }
            if id == ID_SCH_24H {
                unsafe {
                    // 对应 Is24hCheck_Changed：切回 12 小时制时同步 AM/PM，并隐藏 AM/PM 控件
                    if !v { SCH_PM = SCH_TIME.0 >= 12; }
                    SCH_24H = v;
                }
                app::app().settings.use_military_time = v;
                app::trigger_save();
                relayout_hwnd(ctx, hwnd);
            }
        }
        Event::Value(id, v) => {
            if id == ID_SCH_SLIDER {
                unsafe { SCH_BRIGHTNESS = v; }
                // 对应 DialogSlider_ValueChanged：仅拖动时实时预览（滚轮不预览）
                if ctx.ui.capture == Some(ID_SCH_SLIDER) {
                    if ctx.ui.slider_click { app::start_preview(v); } else { app::preview_value(v); }
                }
                relayout_hwnd(ctx, hwnd);
            }
        }
        Event::Key(_, vk) if vk == VK_RETURN || vk == VK_SPACE => {
            app::start_preview(ctx.ui.value(ID_SCH_SLIDER));
        }
        _ => {}
    }
}

fn relayout_hwnd(ctx: &mut WindowCtx, hwnd: HWND) {
    let mut cr = RECT::default();
    unsafe { GetClientRect(hwnd, &mut cr); }
    ui::relayout_public(ctx, cr.width(), cr.height());
}

fn add_schedule(_ctx: &mut WindowCtx, hwnd: HWND) {
    let (time, pm, is24, brightness, days) = unsafe { (SCH_TIME, SCH_PM, SCH_24H, SCH_BRIGHTNESS, SCH_DAYS) };
    if !days.iter().any(|b| *b) {
        show_warning("缺少日程信息", "你还没有选择生效的星期！", true, "知道了", WarningAction::None);
        return;
    }
    let mut hour = time.0;
    if !is24 {
        hour = if pm { (hour % 12) + 12 } else { hour % 12 };
    }
    let time = (hour, time.1);
    let short = UI_SHORT;
    let mut day_str = String::new();
    for (i, d) in short.iter().enumerate() {
        if days[i] { if !day_str.is_empty() { day_str.push_str(", "); } day_str.push_str(d); }
    }
    let a = app::app();
    // 对应 ValidateTime：时间相同且星期集合有交集才算冲突
    let duplicate = a.settings.saved_schedules.iter().enumerate().any(|(i, sc)| {
        if sc.time != time || Some(i) == a.edit_schedule_index { return false; }
        let parts: Vec<&str> = sc.days.split(|c| c == ',' || c == ' ')
            .filter(|x| !x.is_empty()).collect();
        parts.iter().any(|p| day_str.split(", ").any(|d| d == *p))
    });
    // 上游通过 ValidateTime 禁用 Add 按钮；这里保留一道静默防线
    if duplicate { return; }
    let mut st = SYSTEMTIME::default();
    unsafe { GetLocalTime(&mut st); }
    let now_stamp = format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
    // 对应 Add_Click：编辑时沿用旧 LastTriggered，新建时为 DateTime.Now（不是 MinValue）
    let last_triggered = a.edit_schedule_index
        .and_then(|i| a.settings.saved_schedules.get(i).map(|s| s.last_triggered.clone()))
        .unwrap_or_else(|| now_stamp.clone());
    // 上游 Add_Click 用 string.Join(", ", selectedDays)，即使全选也写显式列表
    let sc = DimSchedule { time, brightness, days: day_str.clone(), last_triggered };
    if let Some(i) = a.edit_schedule_index {
        if i < a.settings.saved_schedules.len() { a.settings.saved_schedules[i] = sc; }
    } else {
        a.settings.saved_schedules.push(sc);
    }
    // 上游不重排存储顺序（显示顺序由 RefreshList 的排序副本决定）
    // 对应 Add_Click：保存本次选择的星期
    a.settings.last_schedule_days = day_str;
    a.edit_schedule_index = None;
    config::save(&a.settings);
    unsafe { ShowWindow(hwnd, SW_HIDE); }
    app::end_modal();
    refresh_panel();
}

fn refresh_panel() {
    let h = app::app().hwnd_panel;
    if h == NULL { return; }
    let mut cr = RECT::default();
    unsafe { GetClientRect(h, &mut cr); }
    if let Some(ctx) = unsafe { (ui::get_ctx(h) as *mut WindowCtx).as_mut() } {
        ui::relayout_public(ctx, cr.width(), cr.height());
        unsafe { InvalidateRect(h, std::ptr::null(), FALSE); }
    }
}

// ---------------------------------------------------------------------------
// MinBrightnessDialog
// ---------------------------------------------------------------------------
pub fn show_min() {
    unsafe { MIN_VALUE = app::app().settings.global_minimum; }
    let hwnd = &mut app::app().hwnd_min as *mut HWND;
    let (cw, ch) = saved_size(WindowKind::MinDialog);
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustMin", "ProperDim", cw, ch,
        || { let mut c = WindowCtx::new(WindowKind::MinDialog, "调整最低亮度", true); c.bg = c::BG; c.resizable = true; c.min_w = s(300); c.min_h = s(260); c.max_w = s(450); c.max_h = s(390); c });
}

pub fn layout_min(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let v = unsafe { MIN_VALUE };
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 45, 0, 45, 30, Kind::CloseButton("✕".into())));
    let mut intro = mk(1, 15, 34, w - 30, 34,
        Kind::Label("拖动下方滑块，设定屏幕能达到的最低亮度。".into(), 11, c::MUTED));
    intro.tag = "wrap_center".into();
    ctx.ui.widgets.push(intro);
    ctx.ui.widgets.push(mk(2, 40, 76, 150, 20,
        Kind::Label("最低亮度：".into(), 14, c::WHITE)));
    ctx.ui.widgets.push(mk(3, 190, 76, 46, 20,
        Kind::Label(format!("{}%", (v * 100.0).round() as i32), 14, c::WHITE)));
    let mut sld = mk(ID_MIN_SLIDER, 15, 102, 230, 24,
        Kind::Slider { min: 0.01, max: 0.40, value: v, vertical: false });
    sld.tag = "fine".into(); // 对应 MinSlider_MouseWheel：固定 ±1%
    ctx.ui.widgets.push(sld);
    ctx.ui.widgets.push(mk(ID_MIN_PREVIEW, 252, 104, 24, 20, Kind::PreviewButton));
    if let Some(t) = ctx.ui.widgets.last_mut() { t.tag = "preview".into(); }
    let mut note = mk(4, 15, 132, w - 30, 52,
        Kind::Label("提示：建议先启用热键，再调整这里。".into(), 11, c::MUTED));
    note.tag = "wrap_center".into();
    ctx.ui.widgets.push(note);
    ctx.ui.widgets.push(mk(ID_MIN_OK, 40, 198, 100, 25, Kind::Button("就这样".into())));
    ctx.ui.widgets.push(mk(ID_MIN_CANCEL, 160, 198, 100, 25, Kind::Button("算了".into())));
    ctx.ui.widgets.push(mk(999, w - 14, 246, 12, 12, Kind::ResizeGrip));
}

pub fn on_min(ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    match ev {
        Event::Click(id) => match id {
            ID_CLOSE | ID_MIN_CANCEL => { unsafe { ShowWindow(hwnd, SW_HIDE); } app::trigger_save(); app::end_modal(); },
            ID_MIN_PREVIEW => app::start_preview(unsafe { MIN_VALUE }),
            ID_MIN_OK => {
                let v = unsafe { MIN_VALUE };
                let a = app::app();
                a.settings.global_minimum = v;
                config::save(&a.settings);
                let cur = a.current_brightness;
                app::apply_brightness(cur, false, false, 0, false);
                unsafe { ShowWindow(hwnd, SW_HIDE); }
                app::end_modal();
            }
            _ => {}
        },
        Event::Value(id, v) => {
            if id == ID_MIN_SLIDER {
                unsafe { MIN_VALUE = v; }
                // 对应 MinSlider_PreviewMouseMove：拖动时实时预览最小亮度的实际效果
                if ctx.ui.capture == Some(ID_MIN_SLIDER) {
                    if ctx.ui.slider_click { app::start_preview(v); } else { app::preview_value(v); }
                }
                relayout_hwnd(ctx, hwnd);
            }
        }
        Event::Key(_, vk) if vk == VK_RETURN || vk == VK_SPACE => {
            app::start_preview(unsafe { MIN_VALUE });
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// AppInfoWindow
// ---------------------------------------------------------------------------
/// 只保留两个仓库地址：上游项目 + 本移植版仓库。
const INFO_URLS: &[&str] = &[
    "https://github.com/crustyoldhuman/ProperDim",
    "https://github.com/s13mjl/ProperDim-rust",
];

pub fn show_info() {
    // 对应上游 `AppInfoWindow { Owner = this }` + ShowDialog：Info 是控制面板的模态子窗口，
    // 所以打开它时面板必须已经可见。这样点 Cool「返回」时只是隐藏 Info，
    // 面板不需要重新 ShowWindow —— 从根源上避免「返回时主窗口闪一下」。
    let panel_visible = {
        let a = app::app();
        a.hwnd_panel != NULL && unsafe { IsWindowVisible(a.hwnd_panel) } != 0
    };
    if !panel_visible { app::show_control_panel(); }
    let hwnd = &mut app::app().hwnd_info as *mut HWND;
    let (cw, ch) = saved_size(WindowKind::InfoWindow);
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustInfo", "关于 ProperDim", cw, ch,
        || { let mut c = WindowCtx::new(WindowKind::InfoWindow, "关于 ProperDim", true); c.bg = c::INFO_BG; c.resizable = true; c.min_w = s(320); c.min_h = s(394); c.max_w = s(480); c.max_h = s(591); c });
}

pub fn layout_info(ctx: &mut WindowCtx, w: i32, _h: i32) {
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 40, 0, 40, 30, Kind::CloseButton("✕".into())));
    // 图标（对应 AppInfoWindow 里 65×65 的 Image）
    ctx.ui.widgets.push(mk(1, (w - 65) / 2, 80, 65, 65, Kind::IconBox));
    let mut name = mk(2, 0, 152, w, 32, Kind::BoldLabel("ProperDim".into(), 24, c::WHITE));
    name.tag = "center".into();
    ctx.ui.widgets.push(name);
    let mut ver = mk(3, 0, 186, w, 18, Kind::Label("版本 1.4.6".into(), 12, c::DIM));
    ver.tag = "center".into();
    ctx.ui.widgets.push(ver);

    // 仅两个仓库链接：上游 + 本移植版
    for (i, url) in INFO_URLS.iter().enumerate() {
        let mut l = mk(ID_INFO_LINK_BASE + i as u32, 20, 232 + i as i32 * 26, w - 40, 20,
            Kind::Link((*url).into(), c::LINK));
        l.tag = "link".into();
        ctx.ui.widgets.push(l);
    }

    ctx.ui.widgets.push(mk(ID_INFO_COOL, (w - 110) / 2, 300, 110, 26, Kind::Button("好的".into())));
    // 对应 XAML：HorizontalAlignment="Right" 且 TextDecorations="None"
    let mut help = mk(ID_INFO_HELP, w - 70, 34, 62, 16, Kind::Link("查看帮助".into(), c::DIM));
    help.tag = "right nounderline".into();
    ctx.ui.widgets.push(help);
    ctx.ui.widgets.push(mk(999, w - 14, 380, 12, 12, Kind::ResizeGrip));
}

/// 关闭 Info 并「返回」：Info 可能是从 QuickDim / 托盘菜单打开的，
/// 那时控制面板本来就是隐藏的，直接隐藏 Info 会让人以为整个程序被关掉了。
fn close_info(hwnd: HWND) {
    unsafe { ShowWindow(hwnd, SW_HIDE); }
    app::trigger_save();
    app::end_modal();
    let panel_visible = {
        let a = app::app();
        a.hwnd_panel != NULL && unsafe { IsWindowVisible(a.hwnd_panel) } != 0
    };
    if panel_visible {
        let h = app::app().hwnd_panel;
        unsafe { SetForegroundWindow(h); }
    } else {
        app::show_control_panel();
    }
}

pub fn on_info(_ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    if let Event::Click(id) = ev {
        match id {
            ID_CLOSE | ID_INFO_COOL => close_info(hwnd),
            ID_INFO_HELP => {
                let path = app::readme_path();
                let s = wide(&path.to_string_lossy());
                let op = wide("open");
                let r = unsafe {
                    ShellExecuteW(NULL, op.as_ptr(), s.as_ptr(), std::ptr::null(),
                        std::ptr::null(), SW_SHOWNORMAL)
                };
                if (r as isize) <= 32 {
                    show_warning("Error", &format!("无法打开帮助文件：{}", path.display()),
                        true, "OK", WarningAction::None);
                }
            }
            i if (ID_INFO_LINK_BASE..ID_INFO_LINK_BASE + 2).contains(&i) => {
                app::open_url(INFO_URLS[(i - ID_INFO_LINK_BASE) as usize]);
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// WarningMessage
// ---------------------------------------------------------------------------
pub fn show_warning(title: &str, message: &str, ok_only: bool, custom: &str, action: WarningAction) {
    unsafe {
        WARN_TITLE = title.to_string();
        WARN_MESSAGE = message.to_string();
        WARN_OK_ONLY = ok_only;
        WARN_CUSTOM = if custom.is_empty() { "OK".into() } else { custom.to_string() };
    }
    let a = app::app();
    a.warning_action = action;
    a.warning_result = None;
    let hwnd = &mut a.hwnd_warning as *mut HWND;
    let (cw, ch) = saved_size(WindowKind::Warning);
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustWarning", title, cw, ch,
        || { let mut c = WindowCtx::new(WindowKind::Warning, "", true); c.bg = c::BG; c.resizable = true; c.min_w = s(300); c.min_h = s(150); c.max_w = s(450); c.max_h = s(225); c });
}

pub fn layout_warning(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let (title, message, ok_only, custom) = unsafe {
        (WARN_TITLE.clone(), WARN_MESSAGE.clone(), WARN_OK_ONLY, WARN_CUSTOM.clone())
    };
    ctx.title = title;
    ctx.ui.clear();
    let mut msg = mk(1, 15, 36, w - 30, 58, Kind::Label(message, 11, c::WHITE));
    msg.tag = "wrap_center".into();
    ctx.ui.widgets.push(msg);
    if ok_only {
        ctx.ui.widgets.push(mk(ID_WARN_YES, (w - 75) / 2, 100, 75, 25, Kind::Button(custom)));
    } else {
        ctx.ui.widgets.push(mk(ID_WARN_YES, 60, 100, 75, 25, Kind::Button("确定".into())));
        ctx.ui.widgets.push(mk(ID_WARN_NO, 160, 100, 75, 25, Kind::Button("取消".into())));
    }
    ctx.ui.widgets.push(mk(999, w - 14, 136, 12, 12, Kind::ResizeGrip));
}

pub fn on_warning(_ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    if let Event::Click(id) = ev {
        let yes = id == ID_WARN_YES;
        let a = app::app();
        a.warning_result = Some(yes);
        let action = a.warning_action;
        unsafe { ShowWindow(hwnd, SW_HIDE); }
        app::end_modal();
        if yes {
            match action {
                WarningAction::ClearSchedule => {
                    app::app().settings.saved_schedules.clear();
                    app::trigger_save();
                    refresh_panel();
                }
                WarningAction::Resync => app::resync_displays(),
                WarningAction::None => {}
            }
        }
    }
}
