//! 1:1 对应 `HardwareGammaService.cs` / `GlobalHotkeyService.cs` /
//! `MainWindow.RefreshMonitors()` / `MainWindow.ApplyBrightness()` 的后端实现。
//! 默认走真实 Win32；加 `--demo` 参数（或 PROPERDIM_DEMO=1）时切换到 StubBackend，
//! 只演示前端、不改屏幕亮度，便于安全预览界面。
#![allow(dead_code, non_snake_case)]

use crate::native::*;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};

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
    /// 卸载器复位：无条件下发 gamma=1.0 与单位颜色矩阵（绕过 early-return）。
    fn force_reset(&mut self);
    fn is_demo(&self) -> bool;
}

// ---------------------------------------------------------------------------
// gamma 工作线程
//
// 实测：`SetDeviceGammaRamp` 单次阻塞约 11ms（驱动在等垂直同步），
// 而亮度动画每帧只有 16ms 预算。把它放在 UI 线程上必然导致帧间隔 16/31ms 抖动。
// 这里用一个专用线程 + 「只保留最新值」的合并槽：UI 线程只投递，永不阻塞。
// ---------------------------------------------------------------------------
#[derive(Default)]
struct GammaJob {
    /// (设备名, 目标硬件亮度)
    batch: Vec<(String, f64)>,
    force: bool,
    clear: bool,
}

struct GammaSink {
    slot: Arc<(Mutex<Option<GammaJob>>, Condvar)>,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

/// 在 worker 线程上执行一次真实的硬件写入（对应 `HardwareGammaService.ApplyGamma`）。
fn write_gamma(dcs: &mut HashMap<String, HDC>, applied: &mut HashMap<String, f64>,
               ramp: &mut RAMP, device: &str, target: f64, force: bool) {
    let hardware = target.clamp(0.5, 1.0);
    if !force {
        if let Some(&last) = applied.get(device) {
            if (hardware - last).abs() < 0.001 { return; }
        }
    }
    let dc = match dcs.get(device).copied() {
        Some(d) => d,
        None => {
            let name = wide(device);
            let d = unsafe { CreateDCW(std::ptr::null(), name.as_ptr(), std::ptr::null(), std::ptr::null()) };
            if d == NULL { return; }
            dcs.insert(device.to_string(), d);
            d
        }
    };
    unsafe {
        if force {
            let mut cur = RAMP::default();
            if GetDeviceGammaRamp(dc, &mut cur) != 0 {
                let expected = (hardware * 65535.0) as u16;
                if ((cur.red[255] as i32) - (expected as i32)).abs() < 256 {
                    applied.insert(device.to_string(), hardware);
                    return;
                }
            }
        }
        for i in 0..256 {
            let v = ((i as f64 / 255.0) * hardware * 65535.0) as u16;
            ramp.red[i] = v; ramp.green[i] = v; ramp.blue[i] = v;
        }
        if SetDeviceGammaRamp(dc, ramp) != 0 {
            applied.insert(device.to_string(), hardware);
        }
    }
}

impl GammaSink {
    fn new() -> Self {
        let slot = Arc::new((Mutex::new(None::<GammaJob>), Condvar::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let s2 = Arc::clone(&slot);
        let st2 = Arc::clone(&stop);
        let join = std::thread::Builder::new()
            .name("properdim-gamma".into())
            .spawn(move || {
                let mut dcs: HashMap<String, HDC> = HashMap::new();
                let mut applied: HashMap<String, f64> = HashMap::new();
                let mut ramp = RAMP::default();
                loop {
                    let job = {
                        let (m, cv) = &*s2;
                        let mut g = m.lock().unwrap();
                        while g.is_none() && !st2.load(Ordering::Acquire) {
                            g = cv.wait(g).unwrap();
                        }
                        g.take()
                    };
                    match job {
                        Some(j) => {
                            if j.clear {
                                for (_, d) in dcs.drain() {
                                    if d != NULL { unsafe { DeleteDC(d); } }
                                }
                                applied.clear();
                            }
                            for (dev, v) in j.batch {
                                write_gamma(&mut dcs, &mut applied, &mut ramp, &dev, v, j.force);
                            }
                        }
                        None => { if st2.load(Ordering::Acquire) { break; } }
                    }
                }
                for (_, d) in dcs.drain() {
                    if d != NULL { unsafe { DeleteDC(d); } }
                }
            })
            .ok();
        GammaSink { slot, stop, join }
    }

    /// 投递一批目标值；若上一批还没被取走，就地合并（同设备只留最新值）。
    fn submit(&self, batch: Vec<(String, f64)>, force: bool) {
        if batch.is_empty() && !force { return; }
        let (m, cv) = &*self.slot;
        let mut g = m.lock().unwrap();
        match g.as_mut() {
            Some(j) => {
                for (dev, v) in batch {
                    if let Some(e) = j.batch.iter_mut().find(|(d, _)| *d == dev) { e.1 = v; }
                    else { j.batch.push((dev, v)); }
                }
                j.force |= force;
            }
            None => *g = Some(GammaJob { batch, force, clear: false }),
        }
        cv.notify_one();
    }

    /// 显示拓扑变化：让 worker 丢弃旧的 DC。
    fn clear(&self) {
        let (m, cv) = &*self.slot;
        let mut g = m.lock().unwrap();
        match g.as_mut() {
            Some(j) => j.clear = true,
            None => *g = Some(GammaJob { batch: Vec::new(), force: false, clear: true }),
        }
        cv.notify_one();
    }

    /// 停止并等待（会先把待处理的写入做完，保证退出时亮度被复位）。
    fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let (m, cv) = &*self.slot;
        { let _g = m.lock().unwrap(); }
        cv.notify_all();
        if let Some(j) = self.join.take() { let _ = j.join(); }
    }
}

// ---------------------------------------------------------------------------
// 真实 Win32 后端
// ---------------------------------------------------------------------------
pub struct Win32Backend {
    targets: HashMap<String, f64>,
    /// gamma 硬件写入全部丢给它（UI 线程不再直接调 SetDeviceGammaRamp）
    sink: GammaSink,
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
            targets: HashMap::new(), sink: GammaSink::new(),
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

// ---------------------------------------------------------------------------
// DisplayConfig：1:1 对应 MainWindow.GetMonitorFriendlyName / IsMonitorHdr
// ---------------------------------------------------------------------------
pub struct DisplayConfig {
    pub paths: Vec<DISPLAYCONFIG_PATH_INFO>,
}

impl DisplayConfig {
    pub fn query() -> Self {
        let mut paths: Vec<DISPLAYCONFIG_PATH_INFO> = Vec::new();
        unsafe {
            let mut path_count = 0u32;
            let mut mode_count = 0u32;
            if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count) != 0 {
                return DisplayConfig { paths };
            }
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];
            paths.resize(path_count as usize, DISPLAYCONFIG_PATH_INFO::default());
            let ok = QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut path_count,
                paths.as_mut_ptr(), &mut mode_count, modes.as_mut_ptr(), std::ptr::null_mut());
            if ok != 0 { paths.clear(); }
            else { paths.truncate(path_count as usize); }
        }
        DisplayConfig { paths }
    }

    /// 对应 `_parenthesesRegex` 与 `Generic PnP Monitor` 清理。
    pub fn friendly_name(&self, device: &str) -> (String, String) {
        let mut result = "Display".to_string();
        let mut device_path = String::new();
        for p in &self.paths {
            unsafe {
                let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                src.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                src.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                src.header.adapter_id = p.source_info.adapter_id;
                src.header.id = p.source_info.id;
                if DisplayConfigGetDeviceInfo(&mut src as *mut _ as *mut c_void) != 0 { continue; }
                if from_wide_buf(&src.view_gdi_device_name) != device { continue; }

                let mut tgt = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                tgt.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                tgt.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                tgt.header.adapter_id = p.target_info.adapter_id;
                tgt.header.id = p.target_info.id;
                if DisplayConfigGetDeviceInfo(&mut tgt as *mut _ as *mut c_void) == 0 {
                    device_path = from_wide_buf(&tgt.monitor_device_path);
                    let raw = from_wide_buf(&tgt.monitor_friendly_device_name);
                    let close = raw.find('(').and_then(|a| raw[a..].find(')').map(|i| a + i));
                    if let (Some(a), Some(b)) = (raw.find('('), close) {
                        if b > a + 1 {
                            let inner = raw[a + 1..b].trim().to_string();
                            if !inner.is_empty() { result = inner; }
                        }
                    } else {
                        let clean = raw.replace("Generic PnP Monitor", "").replace("Generic Monitor", "").trim().to_string();
                        result = if clean.is_empty() { raw.clone() } else { clean };
                    }
                }
                break;
            }
        }
        (result, device_path)
    }

    /// 对应 `IsMonitorHdr`：advancedColorInfo.value 的 bit 1。
    pub fn is_hdr(&self, device: &str) -> bool {
        for p in &self.paths {
            unsafe {
                let mut src = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                src.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                src.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                src.header.adapter_id = p.source_info.adapter_id;
                src.header.id = p.source_info.id;
                if DisplayConfigGetDeviceInfo(&mut src as *mut _ as *mut c_void) != 0 { continue; }
                if from_wide_buf(&src.view_gdi_device_name) != device { continue; }
                let mut info = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO::default();
                info.header.kind = DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO;
                info.header.size = std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32;
                info.header.adapter_id = p.target_info.adapter_id;
                info.header.id = p.target_info.id;
                if DisplayConfigGetDeviceInfo(&mut info as *mut _ as *mut c_void) == 0 {
                    return (info.value & 2) != 0;
                }
                break;
            }
        }
        false
    }
}

pub struct EnumCtx {
    pub out: Vec<MonitorInfo>,
    pub cfg: DisplayConfig,
}

unsafe extern "system" fn enum_monitor_proc(hmon: HWND, _hdc: HDC, _r: *mut RECT, data: LPARAM) -> BOOL {
    let ctx = &mut *(data as *mut EnumCtx);
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
    let (nice, device_path) = ctx.cfg.friendly_name(&device);
    let is_hdr = ctx.cfg.is_hdr(&device);
    let mut is_virtual = false;
    for s in [&device, &nice, &device_path, &adapter_string, &adapter_id, &monitor_string, &monitor_id] {
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
    ctx.out.push(MonitorInfo {
        handle: hmon, device_name: device.clone(),
        friendly_name: if nice.is_empty() { friendly_name(&device, &adapter_string) } else { nice },
        gamma_supported: gamma_supported && !is_virtual, is_primary: primary, is_hdr,
    });
    TRUE
}

fn friendly_name(device: &str, adapter: &str) -> String {
    if !adapter.is_empty() { return adapter.to_string(); }
    if !device.is_empty() { return device.to_string(); }
    "Display".into()
}

/// 对应 WPF `Key` 枚举里非字母数字的键名 → 虚拟键码（`KeyInterop.VirtualKeyFromKey` 的等价表）。
const NAMED_KEYS: &[(&str, u32)] = &[
    ("backspace", 0x08), ("tab", 0x09), ("clear", 0x0C), ("enter", 0x0D),
    ("pause", 0x13), ("capital", 0x14), ("capslock", 0x14), ("escape", 0x1B),
    ("space", 0x20), ("pageup", 0x21), ("pagedown", 0x22), ("end", 0x23),
    ("home", 0x24), ("left", 0x25), ("up", 0x26), ("right", 0x27), ("down", 0x28),
    ("select", 0x29), ("print", 0x2A), ("execute", 0x2B), ("printscreen", 0x2C),
    ("snapshot", 0x2C), ("insert", 0x2D), ("delete", 0x2E), ("help", 0x2F),
    ("apps", 0x5D), ("sleep", 0x5F),
    ("multiply", 0x6A), ("add", 0x6B), ("separator", 0x6C), ("subtract", 0x6D),
    ("decimal", 0x6E), ("divide", 0x6F),
    ("numlock", 0x90), ("scroll", 0x91),
    ("browserback", 0xA6), ("browserforward", 0xA7), ("browserrefresh", 0xA8),
    ("browserstop", 0xA9), ("browsersearch", 0xAA), ("browserfavorites", 0xAB),
    ("browserhome", 0xAC), ("volumemute", 0xAD), ("volumedown", 0xAE), ("volumeup", 0xAF),
    ("medianexttrack", 0xB0), ("mediaprevioustrack", 0xB1), ("mediastop", 0xB2),
    ("mediaplaypause", 0xB3),
    ("oem1", 0xBA), ("oemplus", 0xBB), ("oemcomma", 0xBC), ("oemminus", 0xBD),
    ("oemperiod", 0xBE), ("oem2", 0xBF), ("oem3", 0xC0), ("oem4", 0xDB),
    ("oem5", 0xDC), ("oem6", 0xDD), ("oem7", 0xDE), ("oem8", 0xDF), ("oem102", 0xE2),
];

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
            "return" => 0x0D, "clear" => 0x0C, "pause" => 0x13,
            "numlock" => 0x90, "scroll" => 0x91, "printscreen" => 0x2C,
            "lwin" | "rwin" | "apps" => 0,
            x if x.len() == 1 && x.as_bytes()[0].is_ascii_alphabetic() => x.as_bytes()[0].to_ascii_uppercase() as u32,
            x if x.len() == 1 && x.as_bytes()[0].is_ascii_digit() => x.as_bytes()[0] as u32,
            x if x.len() == 2 && (x.as_bytes()[0] == b'd' || x.as_bytes()[0] == b'D')
                && x.as_bytes()[1].is_ascii_digit() => x.as_bytes()[1] as u32,
            x if x.starts_with("numpad") && x.len() > 6 => {
                if let Ok(n) = x[6..].parse::<u32>() { 0x60 + n } else { 0 }
            }
                    x if x.starts_with('f') && x.len() > 1 => {
                        if let Ok(n) = x[1..].parse::<u32>() { 0x6F + n } else { 0 }
                    }
                    x => NAMED_KEYS.iter().find(|(n, _)| *n == x).map(|(_, vk)| *vk).unwrap_or(0),
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
        let mut batch = Vec::with_capacity(self.monitors.len());
        for m in &self.monitors {
            self.targets.insert(m.device_name.clone(), hardware);
            batch.push((m.device_name.clone(), hardware));
        }
        // 只投递，不阻塞 UI 线程
        self.sink.submit(batch, false);
        self.set_magnification(hardware);
    }
    fn refresh_monitors(&mut self) -> Vec<MonitorInfo> {
        // 显示拓扑可能已变，让 worker 丢弃旧的 DC
        self.sink.clear();
        let mut ctx = EnumCtx { out: Vec::new(), cfg: DisplayConfig::query() };
        unsafe {
            EnumDisplayMonitors(NULL, std::ptr::null(), Some(enum_monitor_proc), &mut ctx as *mut _ as LPARAM);
        }
        if ctx.out.is_empty() { return self.monitors.clone(); }
        self.monitors = ctx.out.clone();
        ctx.out
    }
    fn resync_displays(&mut self) {
        let batch: Vec<(String, f64)> = self.monitors.iter()
            .map(|m| (m.device_name.clone(), 1.0)).collect();
        self.sink.submit(batch, true);
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
        self.sink.submit(items, true);
    }
    fn shutdown(&mut self) {
        let batch: Vec<(String, f64)> = self.monitors.iter()
            .map(|m| (m.device_name.clone(), 1.0)).collect();
        self.sink.submit(batch, true);
        // 等待 worker 把复位写完，避免进程退出时屏幕还留在暗的状态
        self.sink.stop();
        self.set_magnification(1.0);
        unsafe {
            UnregisterHotKey(self.hotkey_hwnd, HOTKEY_ID_UP);
            UnregisterHotKey(self.hotkey_hwnd, HOTKEY_ID_DOWN);
            if self.mag_ready { if let Some(u) = self.mag.uninit { u(); } }
        }
        self.mag_ready = false;
    }
    fn force_reset(&mut self) {
        let names: Vec<String> = self.monitors.iter().map(|m| m.device_name.clone()).collect();
        let mut ramp = RAMP::default();
        for n in &names {
            unsafe {
                let dcn = wide(n);
                let dc = CreateDCW(std::ptr::null(), dcn.as_ptr(), std::ptr::null(), std::ptr::null());
                if dc != NULL {
                    for i in 0..256 {
                        let v = ((i as f64 / 255.0) * 65535.0) as u16;
                        ramp.red[i] = v; ramp.green[i] = v; ramp.blue[i] = v;
                    }
                    SetDeviceGammaRamp(dc, &ramp);
                    DeleteDC(dc);
                }
            }
        }
        unsafe {
            if self.mag_ready {
                if let Some(set) = self.mag.set_effect {
                    let mut e = MAGCOLOREFFECT::default();
                    e.transform = [1.0, 0.0, 0.0, 0.0, 0.0,
                                   0.0, 1.0, 0.0, 0.0, 0.0,
                                   0.0, 0.0, 1.0, 0.0, 0.0,
                                   0.0, 0.0, 0.0, 1.0, 0.0,
                                   0.0, 0.0, 0.0, 0.0, 1.0];
                    set(&e);
                }
            }
        }
        self.current_mag_factor = 1.0;
    }
    fn is_demo(&self) -> bool { false }
}

impl Drop for Win32Backend {
    fn drop(&mut self) { self.sink.stop(); }
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
    fn force_reset(&mut self) { self.last_brightness = 1.0; }
    fn is_demo(&self) -> bool { true }
}

pub fn create_backend(demo: bool) -> Box<dyn Backend> {
    if demo { Box::new(StubBackend::new()) } else { Box::new(Win32Backend::new()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hotkey_parsing() {
        assert_eq!(parse_hotkey("Ctrl + Alt + Up", 1), Some((MOD_CONTROL | MOD_ALT, 0x26)));
        assert_eq!(parse_hotkey("Ctrl + Shift + F5", 2), Some((MOD_CONTROL | MOD_SHIFT, 0x74)));
        assert_eq!(parse_hotkey("Win + D1", 1), Some((MOD_WIN, b'1' as u32)));
        assert_eq!(parse_hotkey("", 1), None);
    }
    #[test]
    fn brightness_mapping_matches_upstream() {
        // 对应 ApplyBrightness：0.01-1.0 比例映射到 activeFloor-1.0
        assert!((Win32Backend::to_hardware(0.01, 0.30, false) - 0.30).abs() < 1e-9);
        assert!((Win32Backend::to_hardware(1.0, 0.30, false) - 1.0).abs() < 1e-9);
        let mid = Win32Backend::to_hardware(0.65, 0.30, false);
        assert!((mid - (0.30 + (0.64 / 0.99) * 0.70)).abs() < 1e-9, "{}", mid);
        // ignoreMinimum 时直接透传且允许 0
        assert_eq!(Win32Backend::to_hardware(0.0, 0.30, true), 0.0);
        assert_eq!(Win32Backend::to_hardware(0.42, 0.30, true), 0.42);
    }
    #[test]
    fn oem_hotkeys_parse() {
        assert_eq!(parse_hotkey("Ctrl + OemPlus", 1), Some((MOD_CONTROL, 0xBB)));
        assert_eq!(parse_hotkey("Ctrl + NumPad5", 1), Some((MOD_CONTROL, 0x65)));
        assert_eq!(parse_hotkey("Alt + Multiply", 1), Some((MOD_ALT, 0x6A)));
    }
    #[test]
    fn virtual_keywords_present() {
        assert!(VIRTUAL_KEYWORDS.contains(&"spacedesk"));
        assert!(VIRTUAL_KEYWORDS.contains(&"iddsample"));
    }
}
