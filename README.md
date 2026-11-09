# ProperDim 1.4.6 — Rust 移植版

原项目：`../ProperDim`（C# / .NET 10 / WPF）
本目录：**Rust + 纯 Win32 / GDI + 原生弹出菜单**，零第三方 crate，只依赖 Rust 标准库与 Windows 系统 DLL。

> 第一轮完成前端 6 个窗口与后端主流程；第二轮把差异 2/3/6/7/8/9 与后端审计出的缺口全部与上游 1:1 对齐。
> 目前除「原生菜单选型」与「WPF 特有的 GPU 观感机制」外，功能与后端能力已对齐。

## 快速开始

```bash
cargo build --release --offline          # 零依赖，无需联网
cargo test --offline                     # 9 个单元测试
target/release/ProperDim.exe             # 正常运行（会真正调节屏幕亮度）
target/release/ProperDim.exe --demo      # 只演示界面，后端打桩、不改屏幕亮度
target/release/ProperDim.exe --reset     # 对应 Inno Setup 卸载器的同步亮度复位
```

`--demo` 等价环境变量 `PROPERDIM_DEMO=1`。配置文件与 C# 版**共用同一份 `settings.json`**
（MSIX 包内用 `%LOCALAPPDATA%\ProperDim\`，否则优先 exe 同目录），字段与 System.Text.Json 格式一致，可直接互读。
`ProperDim_README.txt` 已内嵌，首次运行会写到 exe 同目录（对应 csproj 的 `CopyToOutputDirectory=Always`）。

## 实测结果（本机，Windows 11 / Rust 1.98 MSVC）

| 指标 | 原 C# / WPF 版 | Rust 版（demo，含控制面板窗口） |
|---|---|---|
| 进程工作集 | ~90 MB | **13,188 KB ≈ 12.9 MB** |
| 可执行文件 | 64.1 MB（Portable 自包含单文件） | **278 KB** |
| 外部依赖 | .NET 10 Desktop Runtime + 3 个 NuGet 包 | 无（仅系统 DLL） |

最终构建 `tasklist` 实测输出：`"ProperDim.exe","13900","Console","1","13,188 K"`。
进程正常运行、无崩溃；界面预览见 `preview/frontend.png` 与 `preview/dialogs.png`。

## 目录与 1:1 映射

| Rust 文件 | 对应 C# 源文件 | 内容 |
|---|---|---|
| `src/native.rs` | `NativeMethods.cs` + `app.manifest` | 全部 Win32 常量、结构体、`DllImport` 逐项直译；Magnification / dwmapi / shcore / wtsapi32 / MSIX 动态加载；宽字符串工具；结构体布局断言测试 |
| `src/theme.rs` | `App.xaml` 全局样式 + 各 XAML 的 `ControlTemplate` / `Style` | 暗色主题、双缓冲 GDI 绘制、圆角窗口、自绘控件 |
| `src/config.rs` | `ConfigManager.cs` + `AppSettings` + `DimSchedule.cs` | 全部设置字段、TimeSpan/Days 格式兼容、手写 JSON 解析与序列化、MSIX/可写性路径规则 |
| `src/backend.rs` | `HardwareGammaService.cs` + `GlobalHotkeyService.cs` + `MainWindow.RefreshMonitors()` + DisplayConfig | gamma LUT / watchdog、Magnification、显示器枚举、虚拟屏黑名单、`QueryDisplayConfig` 友好名与 HDR、`RegisterHotKey` 解析、`StubBackend` |
| `src/app.rs` | `App.xaml.cs` + `MainWindow.xaml.cs` | 单实例、托盘与原生菜单、启动流程、亮度映射与动画、热键、10s 计划、错过补偿、会话/电源事件、全局鼠标钩子、溢出窗口、崩溃日志 |
| `src/ui/mod.rs` | WPF `Window` / `WindowChrome` / `Dispatcher` | 统一窗口类与 WndProc、消息路由、标题栏拖动、DPI、焦点/键盘、尺寸位置持久化 |
| `src/ui/panels.rs` | `ControlPanel.xaml(.cs)` + `TrayMenuWindow.xaml(.cs)` | 350×200 控制面板、4 个 Tab、热键录制、QuickDim 竖向滑块、HKCU 自启动注册表 |
| `src/ui/dialogs.rs` | `ScheduleDialog` / `MinBrightnessDialog` / `AppInfoWindow` / `WarningMessage` | 4 个对话框的完整控件、文案、时段/星期选择、预览、校验与确认动作 |
| `src/main.rs` | `App.OnStartup` | 入口、参数、单实例、DPI、控制器窗口、定时器、会话通知、消息循环 |
| `assets/icon.ico` | `ProperDimIcon.ico` | 从原 780 KB 的 8 帧 ico 提取 16/32 帧压到 5.4 KB，`include_bytes!` 内嵌 |
| `tools/capture_preview.py` | — | 开发验收：启动 demo、模拟点击、PrintWindow 截图拼图 |

## 已 1:1 复刻的前端细节

- 配色逐项对应 XAML：`#2D2D2D` 窗口、`#1E1E1E` 标题栏、`#3E3E42` 面板/滑块轨道、`#3F3F46` 边框、`#909090` 链接、`#993333` 红色按钮、`#007ACC` 超链接。
- 窗口尺寸：ControlPanel 350×200、ScheduleDialog 320×288、MinBrightnessDialog 300×260、AppInfoWindow 320×394、WarningMessage 300×150、TrayMenu 140×240。
- 全部文案（含 `Adjust Minimum Brightness`、`QuickDim`、`Clear Schedule`、`Resync Displays`、`It is` / `It's not`、`Created on January 15th, 2026` 等）与四个 Tab 名称。
- 6 个 QuickSet、7 天复选、AM/PM、24 小时制、预览按钮、resize grip、圆角与标题栏图标。
- 滑块横向轨道 + 18px 圆 thumb、滚轮调节、拖动捕获；QuickDim 为竖向滑块。
- 原生菜单用 `CreatePopupMenu` / `AppendMenuW` / `TrackPopupMenuEx`；时段 ComboBox 也用原生弹出菜单。

## 第二轮：已消除的差异

| # | 原差异 | 对齐方式 |
|---|---|---|
| 2 | Combo 分钟只有 5 分钟步长 | 改为 0–59 共 60 项，与 `PopulateTimeBoxes` 一致 |
| 3 | 热键需手打字符串、改完不生效 | 新增 `record_hotkey()`：`WM_KEYDOWN` / `WM_SYSKEYDOWN` 录制 Ctrl/Alt/Shift/Win + 虚拟键，映射成 WPF `Key` 枚举名（含 `D0–D9`、`F1–F12`、方向键），写配置并**立即**重新注册；`WM_CHAR` 不再追加字符 |
| 6 | 会话/电源恢复整块缺失 | 补 `WTSRegisterSessionNotification`；`WM_WTSSESSION_CHANGE` / `WM_POWERBROADCAST` 延迟 1 秒重枚举显示器、恢复亮度、补触发计划、重建托盘图标；补 `WM_QUERYENDSESSION` / `WM_ENDSESSION` 应急复位 |
| 7 | 错过计划补偿会覆盖用户亮度 | 完整重写：ISO 时间→总分钟、`LastTriggered` 逐条比对、7 天回溯、`DateTime.MinValue` 自修复，与 `EvaluateMissedSchedules` 同构 |
| 8 | 窗口尺寸/位置未回写 | `WM_SIZE` / `WM_MOVE` 写回 `ControlPanelWidth/Height/Left/Top` 并防抖保存 |
| 9 | reset 顺序、色效复位、帮助文件、日志格式 | `--reset` 移到单实例检查之前；新增 `force_reset()` 无条件下发 gamma=1.0 与单位颜色矩阵；内嵌帮助文件首次落地，打开失败弹 `WarningMessage`；`CrashLogs` 改为 `Crash_yyyy-MM-dd_HH-mm-ss.txt` |
| 4 | DisplayConfig 未移植 | 补齐 `GetDisplayConfigBufferSizes` / `QueryDisplayConfig` / `DisplayConfigGetDeviceInfo`：括号名提取、`Generic PnP Monitor` 清理、HDR bit1 检测（上游这些字段只赋值不读取，但按 1:1 要求仍补齐） |

## 后端全量审计后补齐

审计方式：把 `NativeMethods.cs` 的 34 个 extern 与 `native.rs` 逐项比对，并核对 `RegistryServices.cs`、`TrayMenuWindow.xaml.cs`、`App.xaml.cs`、`MainWindow.xaml.cs` 的所有调用点。

| 上游能力 | Rust 实现 |
|---|---|
| `SetWindowsHookEx(WH_MOUSE_LL)` 全局滚轮 | `mouse_hook`：QuickDim 显示时全局滚轮 ±5%（Ctrl/右键 ±1%）并吞掉消息，隐藏时卸载钩子 |
| `Shell_NotifyIconGetRect` 托盘精确定位 | 固定 GUID + uID/GUID/Empty 三组重试；`MonitorFromPoint` + `rcWork/rcMonitor` 推断任务栏方位贴边摆放 QuickDim；失败回退光标 |
| `CheckAndCloseOverflow` | 菜单隐藏时检查并关闭 `NotifyIconOverflowWindow` 等溢出窗口 |
| `GetDpiForMonitor` / `DwmSetWindowAttribute` / `AppsUseLightTheme` | 动态加载；每个窗口应用 DWM 圆角偏好与沉浸式暗色；读取主题注册表 |
| `GetCurrentPackageFullName` / `IsRunningAsMsix` | `ExtraApis::is_msix()` + `config::set_msix()`，MSIX 下直接用 LocalAppData |
| 托盘键盘事件 | Apps / Shift+F10 / Enter / Space 按 `SwapTrayIconClicks` 触发菜单或控制面板 |
| `WS_EX_TRANSPARENT / TOOLWINDOW / LAYERED` | 控制器窗口补齐 `LAYERED` |
| `SystemParameters.DropShadow` | API 已声明；阴影本身属观感层（见下） |

`SetCursorPos` 在上游是未被调用的声明，未移植；MSIX `StartupTask` 是 WinRT API，纯 Win32 无等价物，非打包版不需要。

## 剩余差异

1. **托盘右键仍是原生弹出菜单**（你指定的技术选型）：原版右键直接弹自定义 QuickDim 滑块窗口；本版 QuickDim 自绘窗口仍在，通过菜单项或「交换左右键」打开。若想严格 1:1，把 `handle_tray_msg` 的右键分支改成 `toggle_tray_menu()` 即可。
2. **WPF 特有的 GPU 观感机制无法逐行对应**：`DropShadowEffect`（像素着色器）、`VisualBrush` + `OpacityMask`（逐像素 alpha 裁剪）、`AllowsTransparency` 分层窗口。圆角形状已用 `SetWindowRgn` 等价实现，阴影未复刻；功能与点击行为不受影响。
3. **动画帧同步**：`CompositionTarget.Rendering` 改为 16ms `SetTimer`，仅顺滑度差异。
4. 原 `README.md` 里面向 AI 爬虫的诱导性 HTML 注释未移植（有意）。

## 验证

- `cargo build --release --offline` 零警告；`cargo test --offline` **9 个测试全部通过**，含 DisplayConfig 结构体大小与 Windows SDK 的一致性断言（`PATH_INFO=72`、`MODE_INFO=80`、`TARGET_DEVICE_NAME=420`、`NOTIFYICONIDENTIFIER=40`、`RAMP=1536`）、热键解析、日期换算、JSON round-trip 与读取原版 settings.json。
- `--demo` 冒烟通过，进程稳定、无崩溃；8 张界面回归截图由 `tools/capture_preview.py` 生成。

## 构建说明

- 目标 `x86_64-pc-windows-msvc`；本机 MSVC BuildTools 的 `link.exe` + Rust 自带导入库已验证可用（无需单独安装 Windows SDK）。
- `profile.release`：`opt-level = "z"` + LTO + `panic = "abort"` + strip。
- `app.manifest` 未嵌入（运行时用 `SetProcessDpiAwarenessContext(-4)` 设置 PerMonitorV2）。如需嵌入：
  `mt.exe -manifest app.manifest -outputresource:target\release\ProperDim.exe;1`

## 许可

沿用原项目 GPL-3.0-or-later。原作者 Kevin Stanislawski（crustyoldhuman）。
