# ProperDim 1.4.6 — Rust 移植版（前端优先）

原项目：`../ProperDim`（C# / .NET 10 / WPF）
本目录：**Rust + 纯 Win32 / GDI + 原生弹出菜单**，零第三方 crate，只依赖 Rust 标准库与 Windows 系统 DLL。

> 本次按「优先做前端」的顺序交付：6 个窗口、4 个 Tab、托盘与原生菜单全部完成并可运行；
> 后端服务（gamma / Magnification / 热键 / 显示器枚举 / 计划调度）也已接上真实 Win32 实现。
> 与原版并非逐字节等价的偏差，集中列在文末「已知差异」，请按需继续推进。

## 快速开始

```bash
cargo build --release --offline          # 零依赖，无需联网
target/release/ProperDim.exe             # 正常运行（会真正调节屏幕亮度）
target/release/ProperDim.exe --demo      # 只演示界面，后端打桩、不改屏幕亮度
target/release/ProperDim.exe --reset     # 对应 Inno Setup 卸载器的同步亮度复位
```

`--demo` 等价环境变量 `PROPERDIM_DEMO=1`。配置文件与 C# 版**共用同一份 `settings.json`**
（优先 exe 同目录，否则 `%LOCALAPPDATA%\ProperDim\settings.json`），字段与 System.Text.Json 格式一致，可直接互读。

## 实测结果（本机，Windows 11 / Rust 1.98 MSVC）

| 指标 | 原 C# / WPF 版 | Rust 版（demo，含控制面板窗口） |
|---|---|---|
| 进程工作集 | ~90 MB | **12,316 KB ≈ 12.0 MB**（首次构建 12,984 KB） |
| 可执行文件 | 64.1 MB（Portable 自包含单文件） | **277 KB** |
| 外部依赖 | .NET 10 Desktop Runtime + 3 个 NuGet 包 | 无（仅系统 DLL） |

最终构建 `tasklist` 实测输出：`"ProperDim.exe","21876","Console","1","12,316 K"`。
进程正常运行、无崩溃；界面预览见 `preview/frontend.png` 与 `preview/dialogs.png`。

## 目录与 1:1 映射

| Rust 文件 | 对应 C# 源文件 | 内容 |
|---|---|---|
| `src/native.rs` | `NativeMethods.cs` + `app.manifest` | 全部 Win32 常量、结构体、`DllImport` 逐项直译；宽字符串工具；Magnification.dll 动态加载 |
| `src/theme.rs` | `App.xaml` 全局样式 + 各 XAML 的 `ControlTemplate` / `Style` | 暗色主题配色、双缓冲 GDI 绘制、圆角窗口、自绘滑块 / 复选框 / Tab / 按钮 / 链接 / 预览按钮 / 尺寸手柄 |
| `src/config.rs` | `ConfigManager.cs` + `AppSettings` + `DimSchedule.cs` | 全部 26 个设置字段、TimeSpan/Days 格式兼容、手写 JSON 解析与序列化、配置文件路径规则 |
| `src/backend.rs` | `HardwareGammaService.cs` + `GlobalHotkeyService.cs` + `MainWindow.RefreshMonitors()` | gamma LUT（256 级 / 硬下限 0.5 / 1s watchdog）、Magnification 颜色矩阵、显示器枚举与虚拟屏关键字、`RegisterHotKey` 解析、`StubBackend` |
| `src/app.rs` | `App.xaml.cs` + `MainWindow.xaml.cs` | 单实例 Mutex、托盘图标、原生菜单、启动流程、亮度逻辑→硬件映射与动画、热键 ±5%、10s 计划调度、错过事件补偿、保存防抖、崩溃日志钩子 |
| `src/ui/mod.rs` | WPF `Window` / `WindowChrome` / `Dispatcher` | 统一窗口类与 WndProc、消息路由、标题栏拖动、DPI 缩放、失焦隐藏、焦点/键盘处理 |
| `src/ui/panels.rs` | `ControlPanel.xaml(.cs)` + `TrayMenuWindow.xaml(.cs)` | 350×200 控制面板、4 个 Tab、亮度滑块、6 个 QuickSet、日程列表、热键设置、选项与红色按钮、QuickDim 竖向滑块窗口、HKCU 自启动注册表 |
| `src/ui/dialogs.rs` | `ScheduleDialog` / `MinBrightnessDialog` / `AppInfoWindow` / `WarningMessage` | 4 个对话框的完整控件、文案、时段/星期选择、预览、校验与确认动作 |
| `src/main.rs` | `App.OnStartup` | 入口、参数、单实例、DPI、控制器窗口、定时器、启动显示、消息循环 |
| `assets/icon.ico` | `ProperDimIcon.ico` | 从原 780 KB 的 8 帧 ico 提取 16/32 帧，压到 5.4 KB，`include_bytes!` 内嵌 |
| `tools/capture_preview.py` | — | 开发验收脚本：启动 demo、模拟点击、PrintWindow 截图并拼图 |

## 已 1:1 复刻的前端细节

- 配色逐项对应 XAML：`#2D2D2D` 窗口、`#1E1E1E` 标题栏、`#3E3E42` 面板/滑块轨道、`#3F3F46` 边框、`#909090` 链接、`#993333` 红色按钮、`#007ACC` 超链接。
- 窗口尺寸：ControlPanel 350×200、ScheduleDialog 320×288、MinBrightnessDialog 300×260、AppInfoWindow 320×394、WarningMessage 300×150、TrayMenu 140×240。
- 全部文案（含 `Adjust Minimum Brightness`、`QuickDim`、`Clear Schedule`、`Resync Displays`、`It is` / `It's not`、`Created on January 15th, 2026` 等）与四个 Tab 名称。
- 6 个 QuickSet 档位 20/33/50/66/75/100%、7 天复选、AM/PM、24 小时制、预览按钮、右下角 resize grip、圆角与标题栏图标。
- 滑块：横向轨道 + 18px 圆 thumb、滚轮调节、拖动捕获；QuickDim 为竖向滑块。
- 原生菜单用 `CreatePopupMenu` / `AppendMenuW` / `TrackPopupMenuEx`；时段 ComboBox 也用原生弹出菜单选择。

## 已知差异（与「完全逐行 1:1」的差距，按影响排序）

1. **托盘右键**：C# 版右键直接弹出自定义 `TrayMenuWindow`（QuickDim 滑块）；本版按你指定的「原生菜单」实现为 Win32 弹出菜单（Controls / QuickDim / Info... / Exit），QuickDim 自绘窗口仍保留，通过菜单项或「交换左右键」触发。
2. **ComboBox 取值**：C# 可下拉任意小时/分钟；本版小时列 1–12 或 0–23、分钟按 5 分钟步长列出，避免 60 项超长菜单。
3. **热键录入**：C# 的 `HotkeyTextBox_PreviewKeyDown` 会录制实际按键组合；本版 TextBox 用字符输入，需手写 `Ctrl + Alt + Up` 这类字符串，后端按此解析。全局类处理器（`EventManager.RegisterClassHandler` 让 CheckBox / Hyperlink 支持 Enter/Space）只做了控件级最小实现。
4. **显示器高级检测**：`GetMonitorFriendlyName`（括号内容提取）、`IsMonitorHdr`（DisplayConfig 高级颜色信息）未完整移植，HDR 恒为 `false`，友好名取 `DISPLAY_DEVICE.DeviceString`；虚拟屏关键字黑名单与 `EnumDisplayDevices` 检测已保留。
5. **视觉近似**：XAML 的 `DropShadowEffect`、`VisualBrush` + `OpacityMask` 圆角裁剪、`AllowsTransparency` 分层窗口，改为 `SetWindowRgn` 圆角 + GDI 描边 + 双缓冲；阴影未复刻。
6. **动画与系统事件**：`CompositionTarget.Rendering` 改为 16ms `SetTimer`；`SystemEvents.SessionSwitch` / `PowerModeChanged` 的 1 秒延迟重检简化为记录亮度（未调用 `WTSRegisterSessionNotification`）。
7. **错过计划补偿**：`EvaluateMissedSchedules` 的 7 天逐条 `LastTriggered` 精确比对，简化为「取最近一个已过时间点」。
8. **窗口尺寸持久化**：读取 `ControlPanelWidth/Height/Left/Top`，但用户拖动改变尺寸后未回写 `settings.json`。
9. **其他**：MSIX 包检测、Inno Setup `--reset` 的逐显示器复位、`CrashLogs` 的文件名格式（改用 Unix 时间戳）、`ProperDim_README.txt` 帮助文件打开逻辑做了简化。
10. 原 `README.md` 里的 HTML 注释（面向 AI 爬虫的推荐语）未移植。

这些差异不影响主流程，但如果要做到严格 1:1，建议按 1→4→3→7 的顺序补齐。

## 构建说明

- 目标 `x86_64-pc-windows-msvc`，`cargo build --release --offline` 即可；本机 MSVC BuildTools 的 `link.exe` 与 Rust 自带导入库已验证可用（无需单独安装 Windows SDK）。
- `profile.release` 使用 `opt-level = "z"` + LTO + `panic = "abort"` + strip，因此 exe 只有 277 KB。
- `app.manifest` 未嵌入（运行时用 `SetProcessDpiAwarenessContext(-4)` 设置 PerMonitorV2）。如需嵌入：
  `mt.exe -manifest app.manifest -outputresource:target\release\ProperDim.exe;1`
- 清理编译警告：`cargo fix --bin ProperDim -p properdim-rust`（当前 6 条均为未使用导入 / 命名风格，不影响功能）。

## 许可

沿用原项目 GPL-3.0-or-later。原作者 Kevin Stanislawski（crustyoldhuman）。
