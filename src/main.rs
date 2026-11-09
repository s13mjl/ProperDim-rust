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
        // 对应 App.xaml.cs 的 UnhandledException -> LogCrash + EmergencyReset
        if app::has_app() { app::app().backend.shutdown(); }
    }));

    let args: Vec<String> = std::env::args().collect();
    let demo = args.iter().any(|a| a.eq_ignore_ascii_case("--demo"))
        || std::env::var("PROPERDIM_DEMO").map(|v| v == "1").unwrap_or(false);
    let is_reset = args.iter().any(|a| a.to_lowercase().contains("reset"));
    let bench = args.iter().any(|a| a.eq_ignore_ascii_case("--bench"));

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
    // 此时还不知道 DPI，先占位；算出 scale 后再按物理像素加载真正的图标
    let icon = NULL;
    let extra_apis = ExtraApis::load();
    config::set_msix(extra_apis.is_msix());
    // 亮度动画用 16ms 定时器；默认 15.6ms 的时钟粒度会让帧间隔忽长忽短（观感一卡一卡）
    if let Some(f) = extra_apis.time_begin { unsafe { f(1); } }

    // 4. 控制器窗口（对应隐藏的 MainWindow / TaskbarIcon 宿主）
    let cls = {
        let mut app_stub = App {
            settings: config::load(),
            backend: backend::create_backend(demo),
            demo, icon, icon_large: NULL, scale: 1.0, instance,
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
            anim_updating: false,
            monitors: Vec::new(), tray_added: false,
            extra: extra_apis, hook: NULL, wts_registered: false, modal_depth: 0,
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
        // 按实际 DPI 取对应尺寸的图标资源（16px 给托盘/标题栏，64px 给 Info 窗口大图）
        a.icon = app::load_icon(ui::s(16));
        a.icon_large = app::load_icon(ui::s(64));
    }
    let icon = app::app().icon;

    // 对应 ControlPanel.Window_Loaded 的 SELF-REPAIR：确保注册表自启动项指向当前 exe
    if !app::app().extra.is_msix() {
        let en = ui::panels::auto_start();
        ui::panels::set_auto_start(en);
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

    // 性能实测模式：测完直接退出（结果写到 exe 同目录 bench.txt）
    if bench {
        app::show_control_panel();
        app::bench();
        // 必须把 gamma 复位，否则测完屏幕会留在暗的状态
        app::app().backend.shutdown();
        app::tray_remove(controller);
        if let Some(f) = app::app().extra.time_end { unsafe { f(1); } }
        unsafe { ExitProcess(0); }
    }

    // 6. 恢复亮度 / 补偿错过的计划（对应 EvaluateMissedSchedules）
    app::evaluate_missed_schedules(true);

    // 7. 按配置显示控制面板（对应 ShowOnStartup）
    if app::app().settings.show_on_startup { app::show_control_panel(); }

    // 8. 消息循环（对应 WPF Dispatcher）
    //
    // 动画帧由本循环按截止时间驱动，不再用 WM_TIMER：WM_TIMER 是低优先级消息，
    // 队列里还有其它消息时会被系统合并，实测会让帧间隔变成 16/31ms 交替（肉眼一卡一卡）。
    let mut msg: MSG = unsafe { std::mem::zeroed() };
    let mut last_tick = qpc_now();
    loop {
        let wait = if app::anim_active() { 8 } else { INFINITE };
        unsafe {
            MsgWaitForMultipleObjectsEx(0, std::ptr::null(), wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE);
        }
        let mut quit = false;
        unsafe {
            while PeekMessageW(&mut msg, NULL, 0, 0, PM_REMOVE) != 0 {
                if msg.message == WM_QUIT { quit = true; break; }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if quit || (app::has_app() && app::app().is_shutting_down) { break; }
        if app::anim_active() && qpc_ms(qpc_now() - last_tick) >= 15.0 {
            app::anim_tick();
            last_tick = qpc_now();
        }
    }

    app::unregister_session_notification(controller);
    app::remove_mouse_hook();
    if let Some(f) = app::app().extra.time_end { unsafe { f(1); } }
    app::shutdown_app();
}

fn synchronous_reset() {
    let mut b = backend::create_backend(false);
    b.refresh_monitors();
    b.force_reset();
    b.shutdown();
}
