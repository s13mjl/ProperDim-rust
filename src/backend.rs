//! 1:1 对应 `HardwareGammaService.cs` / `GlobalHotkeyService.cs` /
//! `MainWindow.RefreshMonitors()` / `MainWindow.ApplyBrightness()` 的后端实现。
//! 默认走真实 Win32；加 `--demo` 参数（或 PROPERDIM_DEMO=1）时切换到 StubBackend，
//! 只演示前端、不改屏幕亮度，便于安全预览界面。
#![allow(dead_code, non_snake_case)]

use crate::native::*;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub handle: isize,
    pub device_name: String,
    pub friendly_name: String,
    pub gamma_supported: bool,
    pub is_primary: bool,
    pub is_hdr: bool,
}

pub trait Backend {
    fn apply_brightness(&mut self, logical: f64, global_min: f64, ignore_minimum: bool);
    fn set_preview(&mut self, value: f64) { self.apply_brightness(value, 0.0, true); }
    fn restore_preview(&mut self, value: f64, global_min: f64) { self.apply_brightness(value, global_min, false); }
    fn refresh_monitors(&mut self) -> Vec<MonitorInfo>;
    fn resync_displays(&mut self);
    fn register_hotkeys(&mut self, hwnd: HWND, enabled: bool, inc: &str, dec: &str);
    fn watchdog_tick(&mut self);
    fn shutdown(&mut self);
    fn is_demo(&self) -> bool;
}

// ---------------------------------------------------------------------------
// 真实 Win32 后端
// ---------------------------------------------------------------------------
pub struct Win32Backend {
    targets: HashMap<String, f64>,
    applied: HashMap<String, f64>,
    ramp: RAMP,
    mag: Magnification,
    mag_ready: bool,
    current_mag_factor: f64,
    monitors: Vec<MonitorInfo>,
    hotkey_hwnd: HWND,
}

const VIRTUAL_KEYWORDS: &[&str] = &[
    "indirect", "virtual", "mirror", "spacedesk", "displaylink", "iddsample",
    "superdisplay", "parsec", "duet", "splashtop", "airdisplay", "idisplay",
    "twomon", "wired xdisplay", "luna", "amyuni", "spacedisplay", "secondscreen",
    "miracast", "project", "rdp", "citrix", "vmware", "vnc", "remotedisplay",
    "usb", "widi", "wifi", "root", "umb",
];

impl Win32Backend {
    pub fn new() -> Self {
        let mag = Magnification::load();
        let mut b = Win32Backend {
            targets: HashMap::new(), applied: HashMap::new(), ramp: RAMP::default(),
            mag, mag_ready: false, current_mag_factor: 1.0,
            monitors: Vec::new(), hotkey_hwnd: NULL,
        };
        unsafe {
            if let Some(init) = b.mag.init { b.mag_ready = init() != 0; }
        }
        b
    }

    /// 对应 `MainWindow.ApplyBrightness()` 的逻辑亮度 → 硬件亮度映射。
    fn to_hardware(logical: f64, global_min: f64, ignore_minimum: bool) -> f64 {
        if ignore_minimum { logical.clamp(0.0, 1.0) }
        else {
            let l = logical.clamp(0.01, 1.0);
            let floor = global_min.clamp(0.0, 1.0);
            floor + ((l - 0.01) / 0.99) * (1.0 - floor)
        }
    }

    /// 对应 `HardwareGammaService.ApplyGamma()`。
    fn apply_gamma(&mut self, device: &str, target: f64, force: bool) {
        let hardware = target.clamp(0.5, 1.0);
        if !force {
            if let Some(&last) = self.applied.get(device) {
                if (hardware - last).abs() < 0.001 { return; }
            }
        }
        unsafe {
            let name = wide(device);
            let dc = CreateDCW(std::ptr::null(), name.as_ptr(), std::ptr::null(), std::ptr::null());
            if dc == NULL { return; }
            if force {
                let mut cur = RAMP::default();
                if GetDeviceGammaRamp(dc, &mut cur) != 0 {
                    let expected = (hardware * 65535.0) as u16;
                    if ((cur.red[255] as i32) - (expected as i32)).abs() < 256 {
                        self.applied.insert(device.to_string(), hardware);
                        DeleteDC(dc);
                        return;
                    }
                }
            }
            for i in 0..256 {
                let v = ((i as f64 / 255.0) * hardware * 65535.0) as u16;
                self.ramp.red[i] = v; self.ramp.green[i] = v; self.ramp.blue[i] = v;
            }
            if SetDeviceGammaRamp(dc, &self.ramp) != 0 {
                self.applied.insert(device.to_string(), hardware);
            }
            DeleteDC(dc);
        }
    }

    /// 对应 `HardwareGammaService.SetGlobalMagnification()`。
    fn set_magnification(&mut self, hardware: f64) {
        let visual_factor = if hardware < 0.5 {
            let normalized = hardware.max(0.0) / 0.5;
            let eased = normalized.powf(1.5);
            let linear = 0.05 + eased * 0.95;
            linear.powf(2.2)
        } else { 1.0 };
        if (self.current_mag_factor - visual_factor).abs() < 0.001 { return; }
        self.current_mag_factor = visual_factor;
        unsafe {
            if let Some(set) = self.mag.set_effect {
                if self.mag_ready {
                    let f = visual_factor as f32;
                    let mut e = MAGCOLOREFFECT::default();
                    e.transform = [
                        f, 0.0, 0.0, 0.0, 0.0,
                        0.0, f, 0.0, 0.0, 0.0,
                        0.0, 0.0, f, 0.0, 0.0,
                        0.0, 0.0, 0.0, 1.0, 0.0,
                        0.0, 0.0, 0.0, 0.0, 1.0,
                    ];
                    set(&e);
                }
            }
        }
    }
}

unsafe extern "system" fn enum_monitor_proc(hmon: HWND, _hdc: HDC, _r: *mut RECT, data: LPARAM) -> BOOL {
    let out = &mut *(data as *mut Vec<MonitorInfo>);
    let mut mi = MONITORINFOEXW::default();
    mi.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(hmon, &mut mi) == 0 { return TRUE; }
    let device = from_wide_buf(&mi.szDevice);
    let primary = (mi.dwFlags & MONITORINFOF_PRIMARY) != 0;

    let dn = wide(&device);
    let mut adapter = DISPLAY_DEVICEW::default();
    adapter.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
    let mut adapter_string = String::new();
    let mut adapter_id = String::new();
    let mut monitor_string = String::new();
    let mut monitor_id = String::new();
    if EnumDisplayDevicesW(dn.as_ptr(), 0, &mut adapter, 0) != 0 {
        adapter_string = from_wide_buf(&adapter.device_string);
        adapter_id = from_wide_buf(&adapter.device_id);
        let an = wide(&from_wide_buf(&adapter.device_name));
        let mut md = DISPLAY_DEVICEW::default();
        md.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        if EnumDisplayDevicesW(an.as_ptr(), 0, &mut md, 0) != 0 {
            monitor_string = from_wide_buf(&md.device_string);
            monitor_id = from_wide_buf(&md.device_id);
        }
    }
    let mut is_virtual = false;
    for s in [&device, &adapter_string, &adapter_id, &monitor_string, &monitor_id] {
        let low = s.to_lowercase();
        if VIRTUAL_KEYWORDS.iter().any(|k| low.contains(k)) { is_virtual = true; break; }
    }
    let low_id = adapter_id.to_lowercase() + &monitor_id.to_lowercase();
    if low_id.contains("root") || low_id.contains("citrix") || low_id.contains("usb") { is_virtual = true; }

    // 对应 TestGammaSupport：尝试读取 LUT 判断驱动是否支持
    let mut gamma_supported = false;
    let dcn = wide(&device);
    let d = CreateDCW(std::ptr::null(), dcn.as_ptr(), std::ptr::null(), std::ptr::null());
    if d != NULL {
        let mut test = RAMP::default();
        gamma_supported = GetDeviceGammaRamp(d, &mut test) != 0;
        DeleteDC(d);
    }
    out.push(MonitorInfo {
        handle: hmon, device_name: device.clone(), friendly_name: friendly_name(&device, &adapter_string),
        gamma_supported: gamma_supported && !is_virtual, is_primary: primary, is_hdr: false,
    });
    TRUE
}

fn friendly_name(device: &str, adapter: &str) -> String {
    if !adapter.is_empty() { return adapter.to_string(); }
    if !device.is_empty() { return device.to_string(); }
    "Display".into()
}

/// 对应 C# 的 `"Ctrl + Alt + Up"` 热键字符串解析。
fn parse_hotkey(s: &str, id: i32) -> Option<(u32, u32)> {
    let mut mods = 0u32;
    let mut key = 0u32;
    for part in s.split('+').map(str::trim) {
        match part.to_lowercase().as_str() {
            "ctrl" | "control" => mods |= MOD_CONTROL,
            "alt" => mods |= MOD_ALT,
            "shift" => mods |= MOD_SHIFT,
            "win" => mods |= MOD_WIN,
            "" => {}
            k => {
                key = match k {
                    "up" => 0x26, "down" => 0x28, "left" => 0x25, "right" => 0x27,
                    "space" => 0x20, "enter" => 0x0D, "tab" => 0x09, "escape" | "esc" => 0x1B,
                    "home" => 0x24, "end" => 0x23, "pageup" => 0x21, "pagedown" => 0x22,
                    "insert" => 0x2D, "delete" => 0x2E, "backspace" => 0x08,
                    x if x.len() == 1 && x.as_bytes()[0].is_ascii_alphabetic() => x.as_bytes()[0].to_ascii_uppercase() as u32,
                    x if x.len() == 1 && x.as_bytes()[0].is_ascii_digit() => x.as_bytes()[0] as u32,
                    x if x.starts_with('f') && x.len() > 1 => {
                        if let Ok(n) = x[1..].parse::<u32>() { 0x6F + n } else { 0 }
                    }
                    _ => 0,
                };
                let _ = id;
            }
        }
    }
    if key == 0 { None } else { Some((mods, key)) }
}

impl Backend for Win32Backend {
    fn apply_brightness(&mut self, logical: f64, global_min: f64, ignore_minimum: bool) {
        let hardware = Self::to_hardware(logical, global_min, ignore_minimum);
        let names: Vec<String> = self.monitors.iter().map(|m| m.device_name.clone()).collect();
        for n in &names {
            self.targets.insert(n.clone(), hardware);
            self.apply_gamma(n, hardware, false);
        }
        self.set_magnification(hardware);
    }
    fn refresh_monitors(&mut self) -> Vec<MonitorInfo> {
        let mut list: Vec<MonitorInfo> = Vec::new();
        unsafe {
            EnumDisplayMonitors(NULL, std::ptr::null(), Some(enum_monitor_proc), &mut list as *mut _ as LPARAM);
        }
        if list.is_empty() { return self.monitors.clone(); }
        self.monitors = list.clone();
        list
    }
    fn resync_displays(&mut self) {
        let names: Vec<String> = self.monitors.iter().map(|m| m.device_name.clone()).collect();
        for n in &names { self.apply_gamma(n, 1.0, true); }
        self.set_magnification(1.0);
        self.monitors.clear();
        self.refresh_monitors();
        self.apply_brightness(1.0, 0.0, true);
    }
    fn register_hotkeys(&mut self, hwnd: HWND, enabled: bool, inc: &str, dec: &str) {
        unsafe {
            UnregisterHotKey(hwnd, HOTKEY_ID_UP);
            UnregisterHotKey(hwnd, HOTKEY_ID_DOWN);
            self.hotkey_hwnd = hwnd;
            if !enabled { return; }
            if let Some((m, k)) = parse_hotkey(inc, HOTKEY_ID_UP) { RegisterHotKey(hwnd, HOTKEY_ID_UP, m, k); }
            if let Some((m, k)) = parse_hotkey(dec, HOTKEY_ID_DOWN) { RegisterHotKey(hwnd, HOTKEY_ID_DOWN, m, k); }
        }
    }
    fn watchdog_tick(&mut self) {
        let items: Vec<(String, f64)> = self.targets.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for (k, v) in items { self.apply_gamma(&k, v, true); }
    }
    fn shutdown(&mut self) {
        let names: Vec<String> = self.monitors.iter().map(|m| m.device_name.clone()).collect();
        for n in &names { self.apply_gamma(n, 1.0, true); }
        self.set_magnification(1.0);
        unsafe {
            UnregisterHotKey(self.hotkey_hwnd, HOTKEY_ID_UP);
            UnregisterHotKey(self.hotkey_hwnd, HOTKEY_ID_DOWN);
            if self.mag_ready { if let Some(u) = self.mag.uninit { u(); } }
        }
        self.mag_ready = false;
    }
    fn is_demo(&self) -> bool { false }
}

// ---------------------------------------------------------------------------
// 演示桩：只演示前端，不触碰屏幕
// ---------------------------------------------------------------------------
pub struct StubBackend { pub last_brightness: f64, monitors: Vec<MonitorInfo> }
impl StubBackend {
    pub fn new() -> Self {
        StubBackend {
            last_brightness: 1.0,
            monitors: vec![MonitorInfo {
                handle: NULL, device_name: "DISPLAY1".into(), friendly_name: "Demo Display (stub)".into(),
                gamma_supported: true, is_primary: true, is_hdr: false,
            }],
        }
    }
}
impl Backend for StubBackend {
    fn apply_brightness(&mut self, logical: f64, _g: f64, _i: bool) { self.last_brightness = logical; }
    fn refresh_monitors(&mut self) -> Vec<MonitorInfo> { self.monitors.clone() }
    fn resync_displays(&mut self) { self.last_brightness = 1.0; }
    fn register_hotkeys(&mut self, _h: HWND, _e: bool, _i: &str, _d: &str) {}
    fn watchdog_tick(&mut self) {}
    fn shutdown(&mut self) {}
    fn is_demo(&self) -> bool { true }
}

pub fn create_backend(demo: bool) -> Box<dyn Backend> {
    if demo { Box::new(StubBackend::new()) } else { Box::new(Win32Backend::new()) }
}
