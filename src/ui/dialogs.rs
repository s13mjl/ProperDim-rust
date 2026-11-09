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
        let (sw, sh) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r);
        SetWindowPos(hwnd, HWND_TOP, (sw - r.width()) / 2, (sh - r.height()) / 2,
            w, h, SWP_NOACTIVATE);
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
        if let Some(i) = edit {
            if let Some(sc) = st.saved_schedules.get(i) {
                SCH_TIME = sc.time;
                SCH_PM = sc.time.0 >= 12;
                SCH_BRIGHTNESS = sc.brightness;
                for (k, d) in UI_SHORT.iter().enumerate() {
                    let full = DimSchedule::full_days()[(k + 1) % 7];
                    SCH_DAYS[k] = sc.days.is_empty() || sc.days.contains(d) || sc.days.contains(full);
                }
            }
        } else {
            SCH_TIME = (12, 0);
            SCH_PM = false;
            SCH_BRIGHTNESS = 1.0;
            for (k, d) in UI_SHORT.iter().enumerate() {
                SCH_DAYS[k] = st.last_schedule_days.contains(d);
            }
        }
    }
    let hwnd = &mut app::app().hwnd_schedule as *mut HWND;
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustSchedule", "Event Scheduler", SCH_W, SCH_H,
        || { let mut c = WindowCtx::new(WindowKind::ScheduleDialog, "Event Scheduler", true); c.bg = c::BG; c.resizable = true; c.min_w = s(320); c.min_h = s(280); c });
}

pub fn layout_schedule(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let (hour, minute) = unsafe { SCH_TIME };
    let (pm, is24, brightness, days) = unsafe { (SCH_PM, SCH_24H, SCH_BRIGHTNESS, SCH_DAYS) };
    let editing = app::app().edit_schedule_index.is_some();
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 32, 0, 32, 32, Kind::CloseButton("✕".into())));
    let mut t = mk(1, 0, 38, w, 18, Kind::Label("Time".into(), 12, c::LIGHT));
    t.tag = "center".into();
    ctx.ui.widgets.push(t);

    let hh = if is24 { hour } else { let x = hour % 12; if x == 0 { 12 } else { x } };
    ctx.ui.widgets.push(mk(ID_SCH_HOUR, 62, 58, 65, 24, Kind::Combo(format!("{:02}", hh))));
    let mut colon = mk(2, 127, 58, 12, 24, Kind::Label(":".into(), 13, c::WHITE));
    colon.tag = "center".into();
    ctx.ui.widgets.push(colon);
    ctx.ui.widgets.push(mk(ID_SCH_MINUTE, 139, 58, 65, 24, Kind::Combo(format!("{:02}", minute))));
    ctx.ui.widgets.push(mk(ID_SCH_AM, 210, 58, 42, 24, Kind::Radio("AM".into(), !pm && !is24)));
    ctx.ui.widgets.push(mk(ID_SCH_PM, 254, 58, 42, 24, Kind::Radio("PM".into(), pm && !is24)));
    if is24 { ctx.ui.set_enabled(ID_SCH_AM, false); ctx.ui.set_enabled(ID_SCH_PM, false); }

    ctx.ui.widgets.push(mk(ID_SCH_24H, 20, 88, 220, 20,
        Kind::CheckBox("Use 24-hour clock".into(), is24)));

    let pct = format!("{}%", (brightness * 100.0).round() as i32);
    ctx.ui.widgets.push(mk(3, 66, 112, 108, 18, Kind::Label("Brightness: ".into(), 12, c::WHITE)));
    ctx.ui.widgets.push(mk(4, 172, 112, 46, 18, Kind::Label(pct, 12, c::WHITE)));
    ctx.ui.widgets.push(mk(ID_SCH_SLIDER, 20, 134, 240, 24,
        Kind::Slider { min: 0.01, max: 1.0, value: brightness, vertical: false }));
    ctx.ui.widgets.push(mk(ID_SCH_PREVIEW, 266, 136, 24, 20, Kind::PreviewButton));

    let mut days_label = mk(5, 0, 162, w, 18, Kind::Label("Days Active".into(), 12, c::LIGHT));
    days_label.tag = "center".into();
    ctx.ui.widgets.push(days_label);
    let labels = ["M", "T", "W", "Th", "F", "S", "Su"];
    for i in 0..7 {
        ctx.ui.widgets.push(mk(ID_SCH_DAY_BASE + i, 10 + i as i32 * 43, 184, 40, 20,
            Kind::DayBox(labels[i as usize].into(), days[i as usize])));
    }
    ctx.ui.widgets.push(mk(ID_SCH_ADD, 95, 218, 130, 25,
        Kind::Button(if editing { "Update Event".into() } else { "Add to Schedule".into() })));
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
                ID_CLOSE => unsafe { ShowWindow(hwnd, SW_HIDE); },
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
            if id == ID_SCH_24H {
                unsafe { SCH_24H = v; }
                app::app().settings.use_military_time = v;
                app::trigger_save();
                relayout_hwnd(ctx, hwnd);
            }
        }
        Event::Value(id, v) => {
            if id == ID_SCH_SLIDER { unsafe { SCH_BRIGHTNESS = v; } relayout_hwnd(ctx, hwnd); }
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
        show_warning("Missing Event Information", "You forgot to select the event days!", true, "OK", WarningAction::None);
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
        if days[i] { if !day_str.is_empty() { day_str.push(','); } day_str.push_str(d); }
    }
    let all = day_str == "Mo,Tu,We,Th,Fr,Sa,Su";
    let a = app::app();
    let duplicate = a.settings.saved_schedules.iter().enumerate().any(|(i, sc)| {
        sc.time == time && Some(i) != a.edit_schedule_index
    });
    if duplicate {
        show_warning("Timeslot already scheduled", "Timeslot already scheduled", true, "OK", WarningAction::None);
        return;
    }
    let sc = DimSchedule {
        time, brightness, days: if all { String::new() } else { day_str },
        last_triggered: "0001-01-01T00:00:00".into(),
    };
    if let Some(i) = a.edit_schedule_index {
        if i < a.settings.saved_schedules.len() { a.settings.saved_schedules[i] = sc; }
    } else {
        a.settings.saved_schedules.push(sc);
    }
    a.settings.saved_schedules.sort_by_key(|x| (x.time.0, x.time.1));
    a.edit_schedule_index = None;
    config::save(&a.settings);
    unsafe { ShowWindow(hwnd, SW_HIDE); }
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
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustMin", "ProperDim", MIN_W, MIN_H,
        || { let mut c = WindowCtx::new(WindowKind::MinDialog, "Adjust Minimum Brightness", true); c.bg = c::BG; c.resizable = true; c.min_w = s(300); c.min_h = s(260); c });
}

pub fn layout_min(ctx: &mut WindowCtx, w: i32, _h: i32) {
    let v = unsafe { MIN_VALUE };
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 45, 0, 45, 30, Kind::CloseButton("✕".into())));
    let mut intro = mk(1, 15, 34, w - 30, 34,
        Kind::Label("Set the darkest brightness the slider can reach. Preview before you commit.".into(), 11, c::MUTED));
    intro.tag = "wrap_center".into();
    ctx.ui.widgets.push(intro);
    ctx.ui.widgets.push(mk(2, 40, 76, 150, 20,
        Kind::Label("Minimum Brightness: ".into(), 14, c::WHITE)));
    ctx.ui.widgets.push(mk(3, 190, 76, 46, 20,
        Kind::Label(format!("{}%", (v * 100.0).round() as i32), 14, c::WHITE)));
    ctx.ui.widgets.push(mk(ID_MIN_SLIDER, 15, 102, 230, 24,
        Kind::Slider { min: 0.01, max: 0.40, value: v, vertical: false }));
    ctx.ui.widgets.push(mk(ID_MIN_PREVIEW, 252, 104, 24, 20, Kind::PreviewButton));
    let mut note = mk(4, 15, 132, w - 30, 52,
        Kind::Label("Take care not to black out your screen. Enable Hotkeys so you can always raise the brightness again.".into(), 11, c::MUTED));
    note.tag = "wrap_center".into();
    ctx.ui.widgets.push(note);
    ctx.ui.widgets.push(mk(ID_MIN_OK, 40, 198, 100, 25, Kind::Button("Looks good".into())));
    ctx.ui.widgets.push(mk(ID_MIN_CANCEL, 160, 198, 100, 25, Kind::Button("Never mind".into())));
    ctx.ui.widgets.push(mk(999, w - 14, 246, 12, 12, Kind::ResizeGrip));
}

pub fn on_min(ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    match ev {
        Event::Click(id) => match id {
            ID_CLOSE | ID_MIN_CANCEL => unsafe { ShowWindow(hwnd, SW_HIDE); },
            ID_MIN_PREVIEW => app::start_preview(unsafe { MIN_VALUE }),
            ID_MIN_OK => {
                let v = unsafe { MIN_VALUE };
                let a = app::app();
                a.settings.global_minimum = v;
                config::save(&a.settings);
                let cur = a.current_brightness;
                app::apply_brightness(cur, false, false, 0, false);
                unsafe { ShowWindow(hwnd, SW_HIDE); }
            }
            _ => {}
        },
        Event::Value(id, v) => { if id == ID_MIN_SLIDER { unsafe { MIN_VALUE = v; } relayout_hwnd(ctx, hwnd); } }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// AppInfoWindow
// ---------------------------------------------------------------------------
const INFO_URLS: &[&str] = &[
    "https://github.com/crustyoldhuman/ProperDim",
    "https://www.twitch.tv/crustyoldhuman",
    "https://www.youtube.com/@crustyoldhuman",
    "https://github.com/crustyoldhuman/ProperDim",
];

pub fn show_info() {
    let hwnd = &mut app::app().hwnd_info as *mut HWND;
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustInfo", "ProperDim Info", INFO_W, INFO_H,
        || { let mut c = WindowCtx::new(WindowKind::InfoWindow, "ProperDim Info", true); c.bg = c::INFO_BG; c.resizable = true; c.min_w = s(320); c.min_h = s(394); c });
}

pub fn layout_info(ctx: &mut WindowCtx, w: i32, _h: i32) {
    ctx.ui.clear();
    ctx.ui.widgets.push(mk(ID_CLOSE, w - 40, 0, 40, 30, Kind::CloseButton("✕".into())));
    ctx.ui.widgets.push(mk(1, (w - 65) / 2, 40, 65, 65, Kind::IconBox));
    let mut name = mk(2, 0, 110, w, 32, Kind::BoldLabel("ProperDim".into(), 24, c::WHITE));
    name.tag = "center".into();
    ctx.ui.widgets.push(name);
    let mut ver = mk(3, 0, 142, w, 18, Kind::Label("Version 1.4.6".into(), 12, c::DIM));
    ver.tag = "center".into();
    ctx.ui.widgets.push(ver);
    let mut gh = mk(ID_INFO_LINK_BASE, 20, 162, w - 40, 18,
        Kind::Link("https://github.com/crustyoldhuman/ProperDim".into(), c::LINK));
    gh.tag = "link".into();
    ctx.ui.widgets.push(gh);
    let mut created = mk(4, 0, 184, w, 18, Kind::Label("Created on January 15th, 2026".into(), 11, c::PALE));
    created.tag = "center".into();
    ctx.ui.widgets.push(created);
    let mut by = mk(5, 0, 202, w, 18, Kind::Label("By Kevin Stanislawski".into(), 11, c::PALE));
    by.tag = "center".into();
    ctx.ui.widgets.push(by);
    let mut games = mk(6, 0, 224, w, 18,
        Kind::Label("Like weird games? Check out crustyoldhuman:".into(), 12, c::MUTED));
    games.tag = "center".into();
    ctx.ui.widgets.push(games);
    for (i, label) in ["twitch.tv/crustyoldhuman", "youtube.com/@crustyoldhuman"].iter().enumerate() {
        let mut l = mk(ID_INFO_LINK_BASE + 1 + i as u32, (w - 190) / 2, 242 + i as i32 * 18, 190, 18,
            Kind::Link((*label).into(), c::LINK));
        l.tag = "link".into();
        ctx.ui.widgets.push(l);
    }
    let mut like = mk(7, 0, 280, w, 18, Kind::Label("If you like ProperDim...".into(), 12, c::MUTED));
    like.tag = "center".into();
    ctx.ui.widgets.push(like);
    let mut coffee = mk(ID_INFO_LINK_BASE + 3, (w - 190) / 2, 298, 190, 18,
        Kind::Link("...maybe throw me a coffee?".into(), c::LINK));
    coffee.tag = "link".into();
    ctx.ui.widgets.push(coffee);
    ctx.ui.widgets.push(mk(ID_INFO_COOL, (w - 70) / 2, 322, 70, 25, Kind::Button("Cool".into())));
    ctx.ui.widgets.push(mk(ID_INFO_HELP, w - 70, 34, 62, 16, Kind::Link("View help".into(), c::DIM)));
    ctx.ui.widgets.push(mk(999, w - 14, 380, 12, 12, Kind::ResizeGrip));
}

pub fn on_info(_ctx: &mut WindowCtx, ev: Event, hwnd: HWND) {
    if let Event::Click(id) = ev {
        match id {
            ID_CLOSE | ID_INFO_COOL => unsafe { ShowWindow(hwnd, SW_HIDE); },
            ID_INFO_HELP => {
                let path = app::readme_path();
                let s = wide(&path.to_string_lossy());
                let op = wide("open");
                let r = unsafe {
                    ShellExecuteW(NULL, op.as_ptr(), s.as_ptr(), std::ptr::null(),
                        std::ptr::null(), SW_SHOWNORMAL)
                };
                if (r as isize) <= 32 {
                    show_warning("Error", &format!("Unable to open help file: {}", path.display()),
                        true, "OK", WarningAction::None);
                }
            }
            i if (ID_INFO_LINK_BASE..ID_INFO_LINK_BASE + 4).contains(&i) => {
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
    show_or_create(unsafe { &mut *hwnd }, "ProperDimRustWarning", title, WARN_W, WARN_H,
        || { let mut c = WindowCtx::new(WindowKind::Warning, "", true); c.bg = c::BG; c.resizable = true; c.min_w = s(300); c.min_h = s(150); c });
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
        ctx.ui.widgets.push(mk(ID_WARN_YES, 60, 100, 75, 25, Kind::Button("It is".into())));
        ctx.ui.widgets.push(mk(ID_WARN_NO, 160, 100, 75, 25, Kind::Button("It's not".into())));
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
