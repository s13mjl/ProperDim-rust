# ProperDim 1.4.6 — Rust 移植版

原项目：`../ProperDim`（C# / .NET 10 / WPF）
本目录：**Rust + 纯 Win32 / GDI + 原生弹出菜单**，零第三方 crate，只依赖 Rust 标准库与 Windows 系统 DLL。

> 第一轮：前端 6 个窗口 + 后端主流程。
> 第二轮：差异 2/3/6/7/8/9 与 `NativeMethods.cs` 暴露的后端缺口。
> 第三轮：按上游 **198 个方法 / 64 个 XAML 事件绑定** 逐项核对，补齐交互逻辑与文案，并修正前两轮 README 中过满的「已 1:1」表述。
> 第四轮：逐段读方法体，补齐数据格式、事件订阅、模态窗口、启动自修复、崩溃恢复、滚轮步进等剩余行为差异。

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
`ProperDim_README.txt` 已内嵌，首次运行写到 exe 同目录（对应 csproj 的 `CopyToOutputDirectory=Always`）。

## 实测结果（本机，Windows 11 / Rust 1.98 MSVC）

| 指标 | 原 C# / WPF 版 | Rust 版（demo，含控制面板窗口） |
|---|---|---|
| 进程工作集 | ~90 MB | **13,200 KB ≈ 12.9 MB** |
| 可执行文件 | 64.1 MB（Portable 自包含单文件） | **278 KB** |
| 外部依赖 | .NET 10 Desktop Runtime + 3 个 NuGet 包 | 无（仅系统 DLL） |

最终构建 `tasklist` 实测输出：`"ProperDim.exe","25944","Console","1","13,200 K"`。
进程正常运行、无崩溃；界面预览见 `preview/frontend.png` 与 `preview/dialogs.png`。

## 目录与映射

| Rust 文件 | 对应 C# 源文件 | 内容 |
|---|---|---|
| `src/native.rs` | `NativeMethods.cs` + `app.manifest` | 全部 Win32 常量/结构体/DllImport；Magnification / dwmapi / shcore / wtsapi32 / MSIX 动态加载；结构体布局断言 |
| `src/theme.rs` | `App.xaml` 全局样式 + 各 ControlTemplate | 暗色主题、双缓冲 GDI、圆角窗口、自绘控件、滚动/选中状态 |
| `src/config.rs` | `ConfigManager.cs` / `AppSettings` / `DimSchedule.cs` | 全部 26 个字段、TimeSpan/Days 格式、手写 JSON、MSIX/可写性路径 |
| `src/backend.rs` | `HardwareGammaService.cs` / `GlobalHotkeyService.cs` / `MainWindow.RefreshMonitors()` | gamma LUT、watchdog、Magnification、显示器枚举、虚拟屏黑名单、`QueryDisplayConfig` 友好名/HDR、热键解析、`StubBackend` |
| `src/app.rs` | `App.xaml.cs` / `MainWindow.xaml.cs` | 单实例、托盘与原生菜单、亮度映射与动画、热键、计划、错过补偿、会话/电源、鼠标钩子、溢出窗口、崩溃日志 |
| `src/ui/mod.rs` | WPF `Window` / `WindowChrome` / `Dispatcher` | 统一窗口类与 WndProc、消息路由、标题栏拖动、DPI、键盘、尺寸位置持久化 |
| `src/ui/panels.rs` | `ControlPanel.xaml(.cs)` / `TrayMenuWindow.xaml(.cs)` | 控制面板 4 个 Tab、热键录制、QuickDim、日程排序/滚动/编辑、HKCU 自启动 |
| `src/ui/dialogs.rs` | 4 个对话框 | 完整控件、时段/星期、预览、校验、确认动作、尺寸持久化 |
| `src/main.rs` | `App.OnStartup` | 入口、参数、单实例、DPI、控制器窗口、定时器、会话通知、消息循环 |

## 前两轮已完成的对齐

- **差异 2/3/6/7/8/9**：Combo 0–59、热键录制并即时注册、WTS 会话/电源恢复与关机复位、错过计划逐条比 `LastTriggered`、窗口尺寸位置持久化、`--reset` 顺序与强制色效复位/帮助文件/CrashLogs 格式。
- **后端审计补齐**：`WH_MOUSE_LL` 全局滚轮钩子、`Shell_NotifyIconGetRect` 托盘定位与任务栏方位、`CheckAndCloseOverflow`、DWM 窗口属性与主题检测、DisplayConfig 友好名/HDR、MSIX 生效、托盘 GUID、键盘 Apps/Shift+F10/Enter/Space、`WS_EX_LAYERED`。
- 前端配色/尺寸/控件/滑块/预览按钮/resize grip/圆角/标题栏图标。

## 第三轮：交互逻辑与文案

| 上游逻辑 | 本轮实现 |
|---|---|
| `HandleClosure` | `CloseToTray=false` 时真正退出（原先永远 Hide）；关闭时 Tab 归零、列表滚回顶部 |
| `RefreshList` | 按「未来 7 天内下一次触发时间」排序（未来在上、无后续排末尾）；新增可拖动滚动条、滚轮滚动 |
| `ScheduleList_MouseDoubleClick` / `PreviewKeyDown` | 双击行编辑、上下键选中、Enter 编辑、Delete 删除 |
| `ToggleScheduleUI` | 未启用计划时置灰并禁用 `+` 按钮与列表交互 |
| `ApplySavedSize` / `Window_SizeChanged` | 4 个对话框读取并回写各自宽高；`LastScheduleDays` 新建计划后持久化 |
| `RegisterClassHandler` / `MainTabs_PreviewKeyDown` / `TabsArea_MouseWheel` | 全局 CheckBox(Enter) / Hyperlink(Enter+Space)；左右键与滚轮切 Tab |
| `ApplyPreview` / `EndPreview` | `_isPreviewActive` 隔离（预览时阻止滑块被回写覆盖）、500ms 用户交互防抖；预览按钮键盘按住、松开/移出恢复 |
| `Hyperlink_RequestNavigate` / `HelpLink_Click` | 打开失败弹 `Unable to open link: ` / `Unable to open help file: ` 提示 |
| 托盘 ToolTip | 菜单显示时清空、隐藏时恢复 |
| `ValidateTime` | 时段冲突时把 Add 按钮改为 `Timeslot already scheduled` 并禁用（**上游不是弹窗**，前两轮实现错了） |

**文案逐条校正**（前两轮因只抓了 `Text=`/`Content=`、漏了 `<Run>`/Inlines，部分说明文字是我自己编的）：

- `MinBrightnessDialog`：*"Adjust the slider below to set the absolute minimum brightness."* / *"TIP: Enable hotkeys before saving changes here so you can't accidentally blackout your screen."*
- `ControlPanel` 链接：`Adjust Minimum`（原先误写 `Adjust Minimum Brightness`）
- Clear Schedule：*"You are about to completely wipe all of your scheduled events. Is this what you truly desire?"*
- Reset Brightness：*"You are about to reset your brightness settings. Your schedule will remain intact. Is this what you truly desire?"*
- 未选星期提示按钮文案 `Oops`；链接/帮助文件错误提示文案。

同时纠正两处此前的错误结论：`GlobalBrightnessChanged` 在上游**只订阅 ControlPanel / TrayMenuWindow**（ScheduleDialog / MinDialog 本就不联动，不是缺口）；`UpdateDimmerOpacity` 是 `ApplyBrightnessAnimated` 的包装，不是「滑块透明度随亮度变化」。

## 第四轮：按方法体逐段核对后的补齐

| 上游逻辑 | 本轮实现 |
|---|---|
| `Add_Click` | 新建计划 `LastTriggered = DateTime.Now`（之前误用 MinValue）、`Days` 用 `", "` 连接（之前无空格）、保存 `UseMilitaryTime` |
| `ValidateTime` | 冲突条件补上**星期集合交集**判断（之前只比时间）；按钮原文改为 `Add Event` / `Update Event` / `Timeslot already scheduled` |
| `SetDayCheckboxes` | 兼容 `Mo` / `M` / `Monday` 等别名，按 `,` 与空格分割 |
| `Is24hCheck_Changed` | 切回 12 小时制时同步 AM/PM（之前会把 20:00 显示成 08 AM），并隐藏 AM/PM 控件 |
| `ScheduleEvaluated` / `ScheduleTriggered` | 两个事件在上游都订阅 `RefreshList`：10 秒评估无条件重排、触发后重排（之前只重绘不排序） |
| `ControlPanel_IsVisibleChanged` | 面板隐藏时销毁所有子窗口（之前子窗口会残留） |
| `ShowDialog()` | 4 个对话框改为模态：禁用主窗口 + 嵌套深度计数，关闭时恢复 |
| `OnActivated` / `Window_Loaded` | 显示时重排列表；位置合法性校验（`VirtualScreen - 50`）、350:200 宽高比、宽度上限 525 |
| `Window_Loaded` SELF-REPAIR | 启动时把注册表自启动项重写为当前 exe 路径，清除旧版本残留 |
| `UnhandledException` | panic hook 增加 `EmergencyReset`：恢复 gamma / Magnification（之前只写日志，崩溃后屏幕可能保持暗） |
| `Slider_ValueChanged` / `MouseWheel` | 滑块点击（非拖动）走 200ms 动画；滚轮 ±5%，Ctrl/右键 ±1%，小范围滑块 ±1%；空白处也能滚动列表 |
| `TrayMenuWindow.OnGlobalBrightnessChanged` | 300ms 用户交互防抖；隐藏时补 `lastTrayMenuCloseTime` / ToolTip 恢复 / 溢出窗口关闭 |
| `HotkeysTab_MouseDown` | 点击空白处取消 TextBox 焦点 |
| `ResetBrightness_Click` | 确认后强制把主滑块同步到 100% |

同时新增两个只读审计工具：`tools/dump_methods.py`（按方法名抽取上游方法体）与 `tools/audit_literals.py`（字面量/常量比对），便于后续继续核对。

## 剩余差异（诚实清单）

1. **托盘右键仍是原生弹出菜单**（你指定的技术选型）：上游右键直接弹自定义 QuickDim 滑块窗口。把 `handle_tray_msg` 的右键分支改成 `toggle_tray_menu()` 即可 1:1。
2. **WPF 特有的 GPU 观感机制无法逐行对应**：`DropShadowEffect`（像素着色器）、`VisualBrush` + `OpacityMask`（逐像素 alpha 裁剪）、`AllowsTransparency` 分层窗口、`ApplyThemeShadow` / `PrepareWindowForOS` / `ApplyWindows10Shadow`。圆角形状已用 `SetWindowRgn` 等价实现，阴影未复刻；点击/功能不受影响。
3. **动画帧同步**：`CompositionTarget.Rendering` → 16ms `SetTimer`，仅顺滑度差异。
4. **滚动条为近似实现**：用自绘 Slider 样式代替上游的完整 `ScrollBar` ControlTemplate，缺少 `ScrollViewerBehavior` 的平滑滚动动画与 `UpdateScheduleScrollFromMouse` 的精确拖动换算。
5. **QuickDim 定位**：上游用 DIP + `GetDpiForMonitor` 换算工作区边界；本版全程用物理像素（PerMonitorV2 下 `GetMonitorInfo` 返回的就是物理值），结果等价但实现路径不同，且未复刻 28px 阴影边距。
6. **焦点与激活细节**：`OnActivated` / `Window_Deactivated` 的部分语义、`Dispatcher.BeginInvoke` 异步更新改为同步（单线程消息循环下等价）。
7. **ResizeThumb 宽高比**：上游拖动时按 320/290 比例缩放，本版用系统 `HTBOTTOMRIGHT` 自由调整。
8. **框架特有**：`DispatcherUnhandledException` 的 `COMException` 错误码白名单、`InvalidOperationException("UpdateToolTip")` 抑制、`NativeLogs.txt`（isNative 分支）、`App.OnStartup` 的 try/catch MessageBox，Rust 无对应概念。
9. **未移植的死代码/无等价物**：`SetCursorPos`（上游未调用）、`MonitorsChanged`（无订阅者）、`FriendlyName`/`IsHdrEnabled`/`GammaSupported`（上游只赋值不读取，但已按 1:1 补齐计算）、MSIX `StartupTask`（WinRT API）、`Window_Loaded` 里空实现的 `TrayMenuWindow`。
10. 原 `README.md` 里面向 AI 爬虫的诱导性 HTML 注释未移植（有意）。

## 验证

- `cargo build --release --offline` **零警告**；`cargo test --offline` **9 项全通过**（含 `DISPLAYCONFIG_PATH_INFO=72`、`MODE_INFO=80`、`TARGET_DEVICE_NAME=420`、`NOTIFYICONIDENTIFIER=40`、`RAMP=1536` 的 SDK 布局断言、热键解析、日期换算、JSON round-trip、读取原版 settings.json）。
- `--demo` 冒烟稳定、无崩溃；8 张界面回归截图由 `tools/capture_preview.py` 生成。

## 构建说明

- 目标 `x86_64-pc-windows-msvc`；本机 MSVC BuildTools 的 `link.exe` + Rust 自带导入库已验证可用（无需单独装 Windows SDK）。
- `profile.release`：`opt-level = "z"` + LTO + `panic = "abort"` + strip。
- `app.manifest` 未嵌入（运行时用 `SetProcessDpiAwarenessContext(-4)`）。如需嵌入：
  `mt.exe -manifest app.manifest -outputresource:target\release\ProperDim.exe;1`

## 许可

沿用原项目 GPL-3.0-or-later。原作者 Kevin Stanislawski（crustyoldhuman）。
