//! ProperDim 1.4.6 — Rust + 纯 Win32/GDI + 原生菜单 1:1 移植版入口。
//! 对应 `App.xaml.cs::OnStartup` / `MainWindow` 的启动流程与消息循环。
#![allow(static_mut_refs)]
#![windows_subsystem = "windows"]

mod app;
mod backend;
mod config;
mod native;
mod theme;
mod ui;

use app::{App, AnimState};
use native::*;
use ui::dialogs::WarningAction;
use ui::WindowKind;

fn main() {
    // 崩溃日志（对应 App.LogCrash / CrashLogs）
    std::panic::set_hook(Box::new(|info| {
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        let dir = std::path::PathBuf::from(base).join("ProperDim").join("CrashLogs");
        let _ = std::fs::create_dir_all(&dir);
        let mut st = SYSTEMTIME::default();
        unsafe { GetLocalTime(&mut st); }
        let name = format!("Crash_{:04}-{:02}-{:02}_{:02}-{:02}-{:02}.txt",
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
        let _ = std::fs::write(dir.join(name), format!("{}", info));
    }));

    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a.eq_ignore_ascii_case("--demo"))
        || std::env::var("PROPERDIM_DEMO").map(|v| v == "1").unwrap_or(false);
    let is_reset = args.iter().any(|a| a.to_lowercase().contains("reset"));

    // 对应 App.xaml.cs：卸载器 reset 必须在单实例检查之前，否则主程序运行时会被 Mutex 挡掉
    if is_reset {
        synchronous_reset();
        unsafe { ExitProcess(0); }
    }

    // 1. 单实例检查（对应 App.xaml.cs 的 Mutex("ProperDim_Unique_Mutex")）
    unsafe {
        let name = wide("ProperDim_Unique_Mutex");
        let m = CreateMutexW(std::ptr::null(), FALSE, name.as_ptr());
        if m == NULL || GetLastError() == 183 /* ERROR_ALREADY_EXISTS */ {
            ExitProcess(0);
        }
    }

    // 3. DPI（对应 app.manifest 的 PerMonitorV2）
    unsafe { SetProcessDpiAwarenessContext(-4isize); }
    app::ensure_readme();

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    let icon = app::make_icon();
    let extra_apis = ExtraApis::load();
    config::set_msix(extra_apis.is_msix());

    // 4. 控制器窗口（对应隐藏的 MainWindow / TaskbarIcon 宿主）
    let cls = {
        let mut app_stub = App {
            settings: config::load(),
            backend: backend::create_backend(demo),
            demo, icon, scale: 1.0, instance,
            hwnd_controller: NULL, hwnd_panel: NULL, hwnd_tray_menu: NULL,
            hwnd_schedule: NULL, hwnd_min: NULL, hwnd_info: NULL, hwnd_warning: NULL,
            current_brightness: 1.0, last_hardware: 1.0, locked_hardware: 1.0,
            last_triggered: (-1, -1), last_hotkey_tick: 0, last_toggle_tick: 0,
            last_menu_close_tick: 0, is_shutting_down: false,
            preview_active: false, preview_restore: 1.0,
            preview_isolated: false, last_user_interaction: 0,
            save_pending: false,
            edit_schedule_index: None, warning_action: WarningAction::None,
            warning_result: None,             anim: AnimState::default(),
            monitors: Vec::new(), tray_added: false,
            extra: extra_apis, hook: NULL, wts_registered: false,
        };
        app_stub.current_brightness = app_stub.settings.last_opacity;
        app::init(app_stub);
        ui::register_class("ProperDimRustController")
    };

    let controller_ctx = ui::WindowCtx::new(WindowKind::Controller, "", false);
    let controller = ui::create_window(&cls, "ProperDim Controller", 0, 0, 0, 0,
        WS_EX_TOOLWINDOW | WS_EX_TRANSPARENT | WS_EX_LAYERED, controller_ctx, false);
    if controller == NULL {
        unsafe {
            let t = wide("Startup Error: failed to create controller window");
            let c = wide("ProperDim");
            MessageBoxW(NULL, t.as_ptr(), c.as_ptr(), 0x10);
            ExitProcess(1);
        }
    }

    // 以实际 DPI 计算缩放（对应 WPF 的 DIP → 物理像素）
    let dpi = unsafe { GetDpiForWindow(controller) };
    let scale = if dpi > 0 { dpi as f64 / 96.0 } else { 1.0 };
    {
        let a = app::app();
        a.hwnd_controller = controller;
        a.scale = scale;
    }

    // 5. 托盘、定时器、热键、显示器、会话通知（对应 MainWindow 构造函数 + SystemEvents）
    app::tray_add(controller, icon);
    app::register_session_notification(controller);
    unsafe {
        SetTimer(controller, app::TIMER_SCHEDULE, 10_000, std::ptr::null());
        SetTimer(controller, app::TIMER_WATCHDOG, 1_000, std::ptr::null());
    }
    {
        let a = app::app();
        let (inc, dec, en) = (a.settings.increase_hotkey.clone(), a.settings.decrease_hotkey.clone(),
            a.settings.hotkeys_enabled);
        a.backend.register_hotkeys(controller, en, &inc, &dec);
        a.monitors = a.backend.refresh_monitors();
    }

    // 6. 恢复亮度 / 补偿错过的计划（对应 EvaluateMissedSchedules）
    app::evaluate_missed_schedules(true);

    // 7. 按配置显示控制面板（对应 ShowOnStartup）
    if app::app().settings.show_on_startup { app::show_control_panel(); }

    // 8. 消息循环（对应 WPF Dispatcher）
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        loop {
            let r = GetMessageW(&mut msg, NULL, 0, 0);
            if r == 0 || r == -1 { break; }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    app::unregister_session_notification(controller);
    app::remove_mouse_hook();
    app::shutdown_app();
}

fn synchronous_reset() {
    let mut b = backend::create_backend(false);
    b.refresh_monitors();
    b.force_reset();
    b.shutdown();
}
