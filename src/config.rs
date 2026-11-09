//! 1:1 对应 `ConfigManager.cs` / `AppSettings` / `DimSchedule.cs`。
//! 自带极小 JSON 解析/序列化器，保持与 System.Text.Json 相同的字段名与值格式，
//! 因此可直接读取原 C# 版生成的 settings.json。
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// 极简 JSON
// ---------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub enum J {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}
impl J {
    pub fn get(&self, k: &str) -> Option<&J> {
        match self { J::Obj(v) => v.iter().find(|(n, _)| n == k).map(|(_, v)| v), _ => None }
    }
    pub fn as_bool(&self) -> Option<bool> { if let J::Bool(b) = self { Some(*b) } else { None } }
    pub fn as_f64(&self) -> Option<f64> { match self { J::Num(n) => Some(*n), J::Str(s) => s.parse().ok(), _ => None } }
    pub fn as_str(&self) -> Option<&str> { if let J::Str(s) = self { Some(s) } else { None } }
    pub fn as_arr(&self) -> Option<&[J]> { if let J::Arr(a) = self { Some(a) } else { None } }
}

struct Parser<'a> { b: &'a [u8], i: usize }
impl<'a> Parser<'a> {
    fn ws(&mut self) { while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() { self.i += 1; } }
    fn val(&mut self) -> Result<J, String> {
        self.ws();
        if self.i >= self.b.len() { return Err("eof".into()); }
        match self.b[self.i] {
            b'{' => self.obj(),
            b'[' => self.arr(),
            b'"' => Ok(J::Str(self.string()?)),
            b't' => { self.i += 4; Ok(J::Bool(true)) }
            b'f' => { self.i += 5; Ok(J::Bool(false)) }
            b'n' => { self.i += 4; Ok(J::Null) }
            _ => self.num(),
        }
    }
    fn obj(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            if self.i >= self.b.len() { return Err("eof in object".into()); }
            if self.b[self.i] == b'}' { self.i += 1; break; }
            let k = self.string()?;
            self.ws();
            if self.i >= self.b.len() || self.b[self.i] != b':' { return Err("expected :".into()); }
            self.i += 1;
            let val = self.val()?;
            v.push((k, val));
            self.ws();
            if self.i < self.b.len() && self.b[self.i] == b',' { self.i += 1; }
        }
        Ok(J::Obj(v))
    }
    fn arr(&mut self) -> Result<J, String> {
        self.i += 1;
        let mut v = Vec::new();
        loop {
            self.ws();
            if self.i >= self.b.len() { return Err("eof in array".into()); }
            if self.b[self.i] == b']' { self.i += 1; break; }
            v.push(self.val()?);
            self.ws();
            if self.i < self.b.len() && self.b[self.i] == b',' { self.i += 1; }
        }
        Ok(J::Arr(v))
    }
    fn string(&mut self) -> Result<String, String> {
        self.ws();
        if self.i >= self.b.len() || self.b[self.i] != b'"' { return Err("expected string".into()); }
        self.i += 1;
        let mut bytes: Vec<u8> = Vec::new();
        while self.i < self.b.len() {
            let c = self.b[self.i]; self.i += 1;
            match c {
                b'"' => return Ok(String::from_utf8_lossy(&bytes).into_owned()),
                b'\\' => {
                    if self.i >= self.b.len() { return Err("bad escape".into()); }
                    let e = self.b[self.i]; self.i += 1;
                    match e {
                        b'"' => bytes.push(b'"'), b'\\' => bytes.push(b'\\'), b'/' => bytes.push(b'/'),
                        b'n' => bytes.push(b'\n'), b'r' => bytes.push(b'\r'), b't' => bytes.push(b'\t'),
                        b'b' => bytes.push(8), b'f' => bytes.push(12),
                        b'u' => {
                            if self.i + 4 > self.b.len() { return Err("bad unicode".into()); }
                            let h = std::str::from_utf8(&self.b[self.i..self.i + 4]).unwrap_or("0000");
                            self.i += 4;
                            if let Ok(n) = u32::from_str_radix(h, 16) {
                                if let Some(ch) = char::from_u32(n) {
                                    let mut buf = [0u8; 4];
                                    bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                                }
                            }
                        }
                        _ => bytes.push(e),
                    }
                }
                _ => bytes.push(c),
            }
        }
        Err("unterminated string".into())
    }
    fn num(&mut self) -> Result<J, String> {
        let st = self.i;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E' => self.i += 1,
                _ => break,
            }
        }
        let s = std::str::from_utf8(&self.b[st..self.i]).map_err(|_| "bad number")?;
        s.parse::<f64>().map(J::Num).map_err(|_| "bad number".into())
    }
}
pub fn parse_json(s: &str) -> Result<J, String> { Parser { b: s.as_bytes(), i: 0 }.val() }

fn esc(s: &str) -> String {
    let mut o = String::new();
    for ch in s.chars() {
        match ch {
            '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"), '\r' => o.push_str("\\r"), '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04X}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
fn numstr(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 { format!("{}", v.round() as i64) } else { format!("{}", v) }
}

// ---------------------------------------------------------------------------
// 数据模型
// ---------------------------------------------------------------------------
#[derive(Clone, Debug)]
pub struct DimSchedule {
    pub time: (u32, u32),      // TimeSpan -> (hour, minute)
    pub brightness: f64,
    pub days: String,
    pub last_triggered: String,
}
impl DimSchedule {
    pub fn display_time(&self, military: bool) -> String {
        let (h, m) = self.time;
        if military { format!("{:02}:{:02}", h, m) }
        else {
            let pm = h >= 12;
            let hh = match h % 12 { 0 => 12, x => x };
            format!("{}:{:02} {}", hh, m, if pm { "PM" } else { "AM" })
        }
    }
    pub fn display(&self) -> String { format!("{}%", (self.brightness * 100.0).round() as i32) }
    pub fn short_days() -> [&'static str; 7] { ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"] }
    pub fn full_days() -> [&'static str; 7] {
        ["Sunday", "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday"]
    }
}

#[derive(Clone, Debug)]
pub struct AppSettings {
    pub show_on_startup: bool,
    pub close_to_tray: bool,
    pub hotkeys_enabled: bool,
    pub increase_hotkey: String,
    pub decrease_hotkey: String,
    pub schedule_toggle: bool,
    pub control_panel_left: f64,
    pub control_panel_top: f64,
    pub control_panel_width: f64,
    pub control_panel_height: f64,
    pub schedule_dialog_width: f64,
    pub schedule_dialog_height: f64,
    pub app_info_window_width: f64,
    pub app_info_window_height: f64,
    pub warning_message_width: f64,
    pub warning_message_height: f64,
    pub min_brightness_dialog_width: f64,
    pub min_brightness_dialog_height: f64,
    pub last_opacity: f64,
    pub saved_schedules: Vec<DimSchedule>,
    pub use_military_time: bool,
    pub last_schedule_days: String,
    pub swap_tray_icon_clicks: bool,
    pub global_minimum: f64,
}
impl Default for AppSettings {
    fn default() -> Self {
        AppSettings {
            show_on_startup: true, close_to_tray: true, hotkeys_enabled: false,
            increase_hotkey: String::new(), decrease_hotkey: String::new(), schedule_toggle: false,
            control_panel_left: -1.0, control_panel_top: -1.0,
            control_panel_width: 350.0, control_panel_height: 200.0,
            schedule_dialog_width: 320.0, schedule_dialog_height: 280.0,
            app_info_window_width: 320.0, app_info_window_height: 394.0,
            warning_message_width: 300.0, warning_message_height: 150.0,
            min_brightness_dialog_width: 300.0, min_brightness_dialog_height: 260.0,
            last_opacity: 1.0, saved_schedules: Vec::new(), use_military_time: false,
            last_schedule_days: "Mo,Tu,We,Th,Fr,Sa,Su".into(),
            swap_tray_icon_clicks: false, global_minimum: 0.30,
        }
    }
}

fn gb(j: &J, k: &str, d: bool) -> bool { j.get(k).and_then(J::as_bool).unwrap_or(d) }
fn gf(j: &J, k: &str, d: f64) -> f64 { j.get(k).and_then(J::as_f64).unwrap_or(d) }
fn gs(j: &J, k: &str, d: &str) -> String { j.get(k).and_then(J::as_str).unwrap_or(d).to_string() }

impl AppSettings {
    pub fn from_json(j: &J) -> Self {
        let d = AppSettings::default();
        let schedules = j.get("SavedSchedules").and_then(J::as_arr).unwrap_or(&[])
            .iter().map(|s| {
                let t = s.get("Time").and_then(J::as_str).unwrap_or("00:00:00");
                let parts: Vec<u32> = t.split(':').filter_map(|x| x.parse().ok()).collect();
                DimSchedule {
                    time: (*parts.first().unwrap_or(&0), *parts.get(1).unwrap_or(&0)),
                    brightness: gf(s, "Brightness", 1.0),
                    days: gs(s, "Days", ""),
                    last_triggered: gs(s, "LastTriggered", "0001-01-01T00:00:00"),
                }
            }).collect();
        AppSettings {
            show_on_startup: gb(j, "ShowOnStartup", d.show_on_startup),
            close_to_tray: gb(j, "CloseToTray", d.close_to_tray),
            hotkeys_enabled: gb(j, "HotkeysEnabled", d.hotkeys_enabled),
            increase_hotkey: gs(j, "IncreaseHotkey", ""),
            decrease_hotkey: gs(j, "DecreaseHotkey", ""),
            schedule_toggle: gb(j, "ScheduleToggle", d.schedule_toggle),
            control_panel_left: gf(j, "ControlPanelLeft", d.control_panel_left),
            control_panel_top: gf(j, "ControlPanelTop", d.control_panel_top),
            control_panel_width: gf(j, "ControlPanelWidth", d.control_panel_width),
            control_panel_height: gf(j, "ControlPanelHeight", d.control_panel_height),
            schedule_dialog_width: gf(j, "ScheduleDialogWidth", d.schedule_dialog_width),
            schedule_dialog_height: gf(j, "ScheduleDialogHeight", d.schedule_dialog_height),
            app_info_window_width: gf(j, "AppInfoWindowWidth", d.app_info_window_width),
            app_info_window_height: gf(j, "AppInfoWindowHeight", d.app_info_window_height),
            warning_message_width: gf(j, "WarningMessageWidth", d.warning_message_width),
            warning_message_height: gf(j, "WarningMessageHeight", d.warning_message_height),
            min_brightness_dialog_width: gf(j, "MinBrightnessDialogWidth", d.min_brightness_dialog_width),
            min_brightness_dialog_height: gf(j, "MinBrightnessDialogHeight", d.min_brightness_dialog_height),
            last_opacity: gf(j, "LastOpacity", d.last_opacity),
            saved_schedules: schedules,
            use_military_time: gb(j, "UseMilitaryTime", d.use_military_time),
            last_schedule_days: gs(j, "LastScheduleDays", &d.last_schedule_days),
            swap_tray_icon_clicks: gb(j, "SwapTrayIconClicks", d.swap_tray_icon_clicks),
            global_minimum: gf(j, "GlobalMinimum", d.global_minimum),
        }
    }
    pub fn to_json(&self) -> String {
        let mut o = String::from("{\n");
        let b = |v: bool| -> String { if v { "true".into() } else { "false".into() } };
        let s = |v: &str| -> String { format!("\"{}\"", esc(v)) };
        fn field(o: &mut String, key: &str, val: String, comma: bool) {
            o.push_str(&format!("  \"{}\": {}{}\n", key, val, if comma { "," } else { "" }));
        }
        field(&mut o, "ShowOnStartup", b(self.show_on_startup), true);
        field(&mut o, "CloseToTray", b(self.close_to_tray), true);
        field(&mut o, "HotkeysEnabled", b(self.hotkeys_enabled), true);
        field(&mut o, "IncreaseHotkey", s(&self.increase_hotkey), true);
        field(&mut o, "DecreaseHotkey", s(&self.decrease_hotkey), true);
        field(&mut o, "ScheduleToggle", b(self.schedule_toggle), true);
        field(&mut o, "ControlPanelLeft", numstr(self.control_panel_left), true);
        field(&mut o, "ControlPanelTop", numstr(self.control_panel_top), true);
        field(&mut o, "ControlPanelWidth", numstr(self.control_panel_width), true);
        field(&mut o, "ControlPanelHeight", numstr(self.control_panel_height), true);
        field(&mut o, "ScheduleDialogWidth", numstr(self.schedule_dialog_width), true);
        field(&mut o, "ScheduleDialogHeight", numstr(self.schedule_dialog_height), true);
        field(&mut o, "AppInfoWindowWidth", numstr(self.app_info_window_width), true);
        field(&mut o, "AppInfoWindowHeight", numstr(self.app_info_window_height), true);
        field(&mut o, "WarningMessageWidth", numstr(self.warning_message_width), true);
        field(&mut o, "WarningMessageHeight", numstr(self.warning_message_height), true);
        field(&mut o, "MinBrightnessDialogWidth", numstr(self.min_brightness_dialog_width), true);
        field(&mut o, "MinBrightnessDialogHeight", numstr(self.min_brightness_dialog_height), true);
        field(&mut o, "LastOpacity", numstr(self.last_opacity), true);
        o.push_str("  \"SavedSchedules\": [");
        if !self.saved_schedules.is_empty() { o.push('\n'); }
        for (i, sc) in self.saved_schedules.iter().enumerate() {
            o.push_str("    {\n");
            o.push_str(&format!("      \"Time\": \"{:02}:{:02}:00\",\n", sc.time.0, sc.time.1));
            o.push_str(&format!("      \"Brightness\": {},\n", numstr(sc.brightness)));
            o.push_str(&format!("      \"Days\": {},\n", s(&sc.days)));
            o.push_str(&format!("      \"LastTriggered\": {}\n", s(&sc.last_triggered)));
            o.push_str(&format!("    }}{}\n", if i + 1 < self.saved_schedules.len() { "," } else { "" }));
        }
        if !self.saved_schedules.is_empty() { o.push_str("  ],\n"); } else { o.push_str("],\n"); }
        field(&mut o, "UseMilitaryTime", b(self.use_military_time), true);
        field(&mut o, "LastScheduleDays", s(&self.last_schedule_days), true);
        field(&mut o, "SwapTrayIconClicks", b(self.swap_tray_icon_clicks), true);
        field(&mut o, "GlobalMinimum", numstr(self.global_minimum), false);
        o.push('}');
        o
    }
}

/// 对应 `ConfigManager.GetSettingsFilePath()`：优先 exe 同目录，否则 %LOCALAPPDATA%\ProperDim。
pub fn settings_path() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("__properdim_write_test__.tmp");
            if fs::write(&p, "test").is_ok() {
                let _ = fs::remove_file(&p);
                return dir.join("settings.json");
            }
        }
    }
    let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    PathBuf::from(base).join("ProperDim").join("settings.json")
}

pub fn load() -> AppSettings {
    let p = settings_path();
    match fs::read_to_string(&p).ok().and_then(|s| parse_json(&s).ok()) {
        Some(j) => AppSettings::from_json(&j),
        None => AppSettings::default(),
    }
}

pub fn save(s: &AppSettings) {
    let p = settings_path();
    if let Some(d) = p.parent() { let _ = fs::create_dir_all(d); }
    let _ = fs::write(&p, s.to_json());
}
