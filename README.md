# ProperDim 1.4.6 — Rust 移植版

原项目：`../ProperDim`（C# / .NET 10 / WPF）
本目录：**Rust + 纯 Win32 / GDI + 原生弹出菜单**，零第三方 crate，只依赖 Rust 标准库与 Windows 系统 DLL。

> 第一轮：前端 6 个窗口 + 后端主流程。
> 第二轮：差异 2/3/6/7/8/9 与 `NativeMethods.cs` 暴露的后端缺口。
> 第三轮：按上游 **198 个方法 / 64 个 XAML 事件绑定** 逐项核对，补齐交互逻辑与文案，并修正前两轮 README 中过满的「已 1:1」表述。
> 第四轮：逐段读方法体，补齐数据格式、事件订阅、模态窗口、启动自修复、崩溃恢复、滚轮步进等剩余行为差异。
> 第五轮：**真正逐行通读上游全部 14 个 `.cs`（5246 行）与 Rust 全部 9 个 `.rs`**，逐语句对照——抓到了前四轮「按方法名/清单抽样」结构性漏掉的一批 bug，包括**亮度动画实际只走第一帧**、**日程列表滚动完全失效**、**AM/PM 单选按钮无效**。

## 快速开始

```bash
cargo build --release --offline          # 零依赖，无需联网
cargo test --offline                     # 14 个单元测试
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
| 进程工作集 | ~90 MB | **13,220 KB ≈ 12.9 MB** |
| 可执行文件 | 64.1 MB（Portable 自包含单文件） | **278 KB** |
| 外部依赖 | .NET 10 Desktop Runtime + 3 个 NuGet 包 | 无（仅系统 DLL） |

最终构建 `tasklist` 实测输出：`"ProperDim.exe","41696","Console","1","13,220 K"`。
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

## 第五轮：逐行通读后的补齐（本轮最重要）

前四轮的核对方式是「列方法名 → 挑可疑的抽方法体 → 凭经验推断」，这必然只能发现**我已经想到的**差异。本轮改成把上游 14 个 `.cs` 从头到尾读一遍、再读全部 Rust，逐语句比对。结果抓到 27 处，其中 3 处是会让功能直接失效的硬 bug：

### 三个硬 bug

1. **亮度动画只走第一帧**。`apply_brightness` 的非动画分支无条件执行 `a.anim.active = false`，而动画每帧正是通过这条路径写硬件——第一帧之后动画就被自己关掉了；`TIMER_ANIM` 还在跑，但 `anim_tick` 直接 return，且因为没有走到 `done` 分支，定时器也永不 `KillTimer`。表现是拖动/点击滑块后亮度几乎不变、停在起点附近。修法是补上上游的 `_isUpdatingFromAnimator` 语义（`anim_updating` 标志），并加回归测试 `brightness_pipeline` 断言动画必须收敛到目标值。
2. **日程列表滚动完全失效**。`Ui::clear()` 在每次重建控件树时把 `scroll_offset` / `selected_row` 清零，而布局正是「先 clear 再读 scroll_offset」——拖动滚动条和滚轮改的值立刻被抹掉。滚动/选中属于视图状态，不能跟着控件树清空。
3. **AM/PM 单选按钮点了不生效**。`Kind::Radio` 落在 CheckBox 分支返回 `Event::Check`，而日程对话框的 `Event::Check` 只处理 24 小时开关和星期——点击只改了控件自身的高亮，`SCH_PM` 从未变化。改为互斥组语义（点击即置位 + 走 `Event::Click`）。

### 其余修正

| 上游逻辑 | 本轮修正 |
|---|---|
| `ApplyBrightness` 第 1 步 | 不忽略最小值时逻辑亮度必须 `Max(0.01, ...)`；之前 0.0 会直接传给后端、算出低于 floor 的硬件值 |
| 动画起点 | 用「当前**硬件**亮度折算回新逻辑坐标系」作为起点（`0.01 + (last-floor)/(1-floor)*0.99`），改 GlobalMinimum 后不再闪跳 |
| TrayIcon.ToolTipText | 只在「非动画帧」且「QuickDim 不可见」时刷新；之前每 16ms 刷一次，且菜单打开时也在刷 |
| `DimmerSlider_MouseWheel` | 改为 5% 网格吸附公式（`((cur+4)/5)*5`，向上且恰为网格点时 +5），不是简单 ±0.05 |
| `DialogDimmerSlider` / `MinSlider` 滚轮 | 固定 ±1%（与滑块总跨度无关），用 `tag="fine"` 区分 |
| `TabsArea_MouseWheel` | 方向搞反了：`Delta>0` 应是**下一个** Tab |
| `ScheduleList_MouseDoubleClick` | 单行单击只选中；打开编辑器必须双击或 Enter（之前单击就弹窗） |
| `ScheduleCheckbox_Checked` | 勾选后立即 `EvaluateMissedSchedules(animate, 1500)` |
| `HandleClosure` / 各对话框 `PreviewKeyDown` | Esc 走 `HandleClosure`（受 `CloseToTray` 影响）；对话框 Esc = 关闭，Warning 记为 `No` |
| `Minimize_Click` | 真正 `WindowState.Minimized`：控制面板去掉 `WS_EX_TOOLWINDOW`、加 `WS_MINIMIZEBOX`，才有任务栏按钮可最小化 |
| `DialogDimmerSlider.Minimum` | 日程对话框滑块下限是 `GlobalMinimum`，不是 0.01 |
| Add 模式默认值 | 「当前时间」+ `Max(Minimum, LastOpacity)`；之前写死 12:00 / 100% |
| `Add_Click` 的 `Days` | 永远写显式列表（`"Mo, Tu, ..., Su"`），上游**从不写空串**——写空串会让 `RefreshList` 的 `Days.Contains("Mo")` 为 false，把「每天」的计划排到列表最末 |
| `ActiveSchedules` 顺序 | 不再按时间重排：上游保持插入顺序，显示顺序由 `RefreshList` 的排序副本决定 |
| `ValidateTime` | 重复校验也必须带星期交集，否则同时间不同星期会被误挡 |
| `DayCheck_Changed` | 勾选星期后立即重算冲突状态（Add 按钮文案/禁用） |
| `ApplySavedSize` | 4 个对话框实现真正的「最小尺寸校验 + 宽度回算高度 + 500/450/480 上限」；日程对话框默认高度应是 **290**（320/(320/290)），之前是 280 |
| `WindowStartupLocation=CenterOwner` | 对话框居中改为跟随控制面板所在显示器，不再固定主屏 |
| `DialogSlider_ValueChanged` / `MinSlider_PreviewMouseMove` | 拖动时实时预览（`ApplyPreview(v, false)`）；松手按**当前** `LastOpacity` 还原，预览期间用热键调过亮度也能被尊重 |
| `HotkeyTextBox_PreviewKeyDown` | 录制支持 OEM / 小键盘 / 媒体键，并与解析表一一对应；之前录出 `;`、`NumPad5` 之类字符串无法注册成热键 |
| `RegisterClassHandler` | 键盘响应改为跟随**焦点**（之前跟随鼠标悬停），并补上 Tab / Shift+Tab 焦点遍历与虚线焦点框 |
| `TrayMenuWindow.Window_IsVisibleChanged` | 补上「图标在溢出浮出窗口」分支（贴光标弹出）、12px 任务栏间距、`Activate()`+`Focus()`（失去焦点才会自动收起） |
| `TrayMenuWindow.PreviewKeyDown` | 补上上下键调亮度（Ctrl ±1%，否则 5% 网格，走动画） |
| `ResizeThumb_DragDelta` | 面板 525、对话框 500/450/480 的拖拽上限（`WM_GETMINMAXINFO`） |
| `AppSettings` 默认值 | `AppInfoWindowHeight` 改回上游的 384（其 `MinHeight` 是 394，所以首次运行本就不套用保存尺寸） |
| `_parenthesesRegex` | 取第一个 `(` 之后的**第一个** `)`；之前用 `rfind` 会取到最后一个 |

另外**删掉**一处我上一轮自己加的、上游根本没有的行为：`Delete` 键删除日程（`ScheduleList_PreviewKeyDown` 只处理 Enter/Space）。

## 剩余差异（诚实清单）

1. **托盘右键仍是原生弹出菜单**（你指定的技术选型）：上游右键直接弹自定义 QuickDim 滑块窗口。把 `handle_tray_msg` 的右键分支改成 `toggle_tray_menu()` 即可 1:1。
2. **WPF 特有的 GPU 观感机制无法逐行对应**：`DropShadowEffect`（像素着色器）、`VisualBrush` + `OpacityMask`（逐像素 alpha 裁剪）、`AllowsTransparency` 分层窗口、`ApplyThemeShadow` / `PrepareWindowForOS` / `ApplyWindows10Shadow`。圆角形状已用 `SetWindowRgn` 等价实现，阴影未复刻；点击/功能不受影响。
3. **动画帧同步**：`CompositionTarget.Rendering` → 16ms `SetTimer`，仅顺滑度差异。
4. **滚动条为近似实现**：用自绘 Slider 样式代替上游的完整 `ScrollBar` ControlTemplate，缺少 `ScrollViewerBehavior` 的平滑滚动动画与 `UpdateScheduleScrollFromMouse` 的精确拖动换算。
5. **QuickDim 几何**：方位判定、溢出浮出窗口、12px 间距均已按上游视觉结果复刻，但本版窗口没有 WPF 的 28px 阴影边距，因此内部坐标是「视觉尺寸」而非「视觉+阴影」。
6. **焦点细节**：Tab 顺序按控件创建顺序（WPF 按视觉树 + `FocusScope`）；`Dispatcher.BeginInvoke` 异步更新改为同步（单线程消息循环下等价）。
7. **ResizeThumb**：上游锁定宽高比、只允许横向拖动并实时 `SetWindowPos`；本版用系统 `HTBOTTOMRIGHT` 自由拖拽，仅用 `WM_GETMINMAXINFO` 限制在 min/max 之间（高度不会自动按比例回算）。
8. **ComboBox**：改值用原生弹出菜单，替代 WPF 下拉框与 `DropDownScrollViewer` 的 150ms 平滑滚动。
9. **框架特有**：`DispatcherUnhandledException` 的 `COMException` 错误码白名单、`InvalidOperationException("UpdateToolTip")` 抑制、`NativeLogs.txt`（isNative 分支）、`App.OnStartup` 的 try/catch MessageBox，Rust 无对应概念。
10. **未移植的死代码/无等价物**：`SetCursorPos`（上游未调用）、`MonitorsChanged`（无订阅者）、MSIX `StartupTask`（WinRT API）、`TrayMenuWindow.Window_Loaded`（空实现）。
11. 原 `README.md` 里面向 AI 爬虫的诱导性 HTML 注释未移植（有意）。

## 验证

- `cargo build --release --offline` **零警告**；`cargo test --offline` **14 项全通过**（含 `DISPLAYCONFIG_PATH_INFO=72`、`MODE_INFO=80`、`TARGET_DEVICE_NAME=420`、`NOTIFYICONIDENTIFIER=40`、`RAMP=1536` 的 SDK 布局断言、热键解析、日期换算、JSON round-trip、读取原版 settings.json、亮度映射、滚轮 5%/1% 吸附、OEM 热键解析、**亮度动画必须收敛到目标**）。
- `--demo` 冒烟稳定、无崩溃；8 张界面回归截图由 `tools/capture_preview.py` 生成。

## 构建说明

- 目标 `x86_64-pc-windows-msvc`；本机 MSVC BuildTools 的 `link.exe` + Rust 自带导入库已验证可用（无需单独装 Windows SDK）。
- `profile.release`：`opt-level = "z"` + LTO + `panic = "abort"` + strip。
- `app.manifest` 未嵌入（运行时用 `SetProcessDpiAwarenessContext(-4)`）。如需嵌入：
  `mt.exe -manifest app.manifest -outputresource:target\release\ProperDim.exe;1`

## 许可

沿用原项目 GPL-3.0-or-later。原作者 Kevin Stanislawski（crustyoldhuman）。
