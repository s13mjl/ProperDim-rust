//! 1:1 对应原 C# 项目的 `NativeMethods.cs`。
//! 零外部依赖：所有 Win32 入口以 `#[link]` 静态声明；少数无导入库的 API 走动态加载。
#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]

use std::ffi::c_void;

pub type HWND = isize;
pub type HDC = isize;
pub type HICON = isize;
pub type HBRUSH = isize;
pub type HPEN = isize;
pub type HFONT = isize;
pub type HBITMAP = isize;
pub type HGDIOBJ = isize;
pub type HMENU = isize;
pub type HRGN = isize;
pub type HINSTANCE = isize;
pub type HMODULE = isize;
pub type HANDLE = isize;
pub type WPARAM = usize;
pub type LPARAM = isize;
pub type LRESULT = isize;
pub type BOOL = i32;
pub type UINT = u32;
pub type DWORD = u32;
pub type COLORREF = u32;
pub type ATOM = u16;
pub type WNDPROC = Option<unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT>;

pub const NULL: isize = 0;
pub const TRUE: i32 = 1;
pub const FALSE: i32 = 0;
pub const INVALID_HANDLE_VALUE: isize = -1;

// ---- 窗口 / 消息 ----
pub const WM_CREATE: UINT = 0x0001;
pub const WM_DESTROY: UINT = 0x0002;
pub const WM_NCDESTROY: UINT = 0x0082;
pub const WM_SIZE: UINT = 0x0005;
pub const WM_MOVE: UINT = 0x0003;
pub const WM_PAINT: UINT = 0x000F;
pub const WM_CLOSE: UINT = 0x0010;
pub const WM_ERASEBKGND: UINT = 0x0014;
pub const WM_SETTINGCHANGE: UINT = 0x001A;
pub const WM_SETICON: UINT = 0x0080;
pub const ICON_SMALL: usize = 0;
pub const ICON_BIG: usize = 1;
pub const WM_FONTCHANGE: UINT = 0x001D;
pub const WM_KEYDOWN: UINT = 0x0100;
pub const WM_KEYUP: UINT = 0x0101;
pub const WM_CHAR: UINT = 0x0102;
pub const WM_SYSKEYDOWN: UINT = 0x0104;
pub const WM_COMMAND: UINT = 0x0111;
pub const WM_TIMER: UINT = 0x0113;
pub const WM_HSCROLL: UINT = 0x0114;
pub const WM_VSCROLL: UINT = 0x0115;
pub const WM_MOUSEMOVE: UINT = 0x0200;
pub const WM_LBUTTONDOWN: UINT = 0x0201;
pub const WM_LBUTTONUP: UINT = 0x0202;
pub const WM_LBUTTONDBLCLK: UINT = 0x0203;
pub const WM_RBUTTONUP: UINT = 0x0205;
pub const WM_MOUSEWHEEL: UINT = 0x020A;
pub const WM_DISPLAYCHANGE: UINT = 0x007E;
pub const WM_HOTKEY: UINT = 0x0312;
pub const WM_POWERBROADCAST: UINT = 0x0218;
pub const WM_WTSSESSION_CHANGE: UINT = 0x02B1;
pub const WM_DPICHANGED: UINT = 0x02E0;
pub const WM_WINDOWPOSCHANGED: UINT = 0x0047;
pub const WM_ACTIVATE: UINT = 0x0006;
pub const WM_SETFOCUS: UINT = 0x0007;
pub const WM_KILLFOCUS: UINT = 0x0008;
pub const WM_NCHITTEST: UINT = 0x0084;
pub const WM_NCLBUTTONDOWN: UINT = 0x00A1;
pub const WM_GETMINMAXINFO: UINT = 0x0024;
pub const WM_APP: UINT = 0x8000;
pub const WM_USER: UINT = 0x0400;
pub const WM_APP_TRAY: UINT = WM_APP + 1;
pub const WM_APP_APPLY: UINT = WM_APP + 2;

pub const WS_OVERLAPPED: u32 = 0x00000000;
pub const WS_POPUP: u32 = 0x80000000;
pub const WS_CHILD: u32 = 0x40000000;
pub const WS_VISIBLE: u32 = 0x10000000;
pub const WS_CAPTION: u32 = 0x00C00000;
pub const WS_SYSMENU: u32 = 0x00080000;
pub const WS_THICKFRAME: u32 = 0x00040000;
pub const WS_MINIMIZEBOX: u32 = 0x00020000;
pub const WS_MAXIMIZEBOX: u32 = 0x00010000;
pub const WS_EX_TOOLWINDOW: u32 = 0x00000080;
pub const WS_EX_LAYERED: u32 = 0x00080000;
pub const WS_EX_TRANSPARENT: u32 = 0x00000020;
pub const WS_EX_NOACTIVATE: u32 = 0x08000000;
pub const WS_EX_TOPMOST: u32 = 0x00000008;

pub const SW_HIDE: i32 = 0;
pub const SW_SHOW: i32 = 5;
pub const SW_SHOWNA: i32 = 8;
pub const SW_MINIMIZE: i32 = 6;
pub const SWP_NOSIZE: u32 = 0x0001;
pub const SWP_NOMOVE: u32 = 0x0002;
pub const SWP_NOACTIVATE: u32 = 0x0010;
pub const SWP_NOZORDER: u32 = 0x0004;
pub const HWND_TOPMOST: HWND = -1;
pub const HWND_TOP: HWND = 0;

pub const GWLP_USERDATA: i32 = -21;
pub const GWL_STYLE: i32 = -16;
pub const GWL_EXSTYLE: i32 = -20;

pub const VK_RETURN: i32 = 0x0D;
pub const VK_SPACE: i32 = 0x20;
pub const VK_TAB: i32 = 0x09;
pub const VK_LEFT: i32 = 0x25;
pub const VK_UP: i32 = 0x26;
pub const VK_RIGHT: i32 = 0x27;
pub const VK_DOWN: i32 = 0x28;
pub const VK_ESCAPE: i32 = 0x1B;
pub const VK_DELETE: i32 = 0x2E;
pub const VK_F10: i32 = 0x79;
pub const VK_APPS: i32 = 0x5D;
pub const VK_SHIFT: i32 = 0x10;
pub const VK_CONTROL: i32 = 0x11;
pub const VK_MENU: i32 = 0x12;
pub const VK_LWIN: i32 = 0x5B;
pub const VK_RWIN: i32 = 0x5C;

pub const MK_LBUTTON: usize = 0x0001;
pub const MK_RBUTTON: usize = 0x0002;
pub const MK_SHIFT: usize = 0x0004;
pub const MK_CONTROL: usize = 0x0008;

pub const HTCLIENT: LRESULT = 1;
pub const HTCAPTION: LRESULT = 2;
pub const HTBOTTOMRIGHT: LRESULT = 17;

pub const IDOK: i32 = 1;
pub const IDCANCEL: i32 = 2;

pub const SM_CXSCREEN: i32 = 0;
pub const SM_CYSCREEN: i32 = 1;
pub const SM_XVIRTUALSCREEN: i32 = 76;
pub const SM_YVIRTUALSCREEN: i32 = 77;
pub const SM_CXVIRTUALSCREEN: i32 = 78;
pub const SM_CYVIRTUALSCREEN: i32 = 79;
pub const SM_CYCAPTION: i32 = 4;
pub const MONITORINFOF_PRIMARY: u32 = 0x00000001;
pub const MONITOR_DEFAULTTONEAREST: u32 = 0x00000002;
pub const QDC_ONLY_ACTIVE_PATHS: u32 = 0x00000002;

pub const MOD_ALT: u32 = 0x0001;
pub const MOD_CONTROL: u32 = 0x0002;
pub const MOD_SHIFT: u32 = 0x0004;
pub const MOD_WIN: u32 = 0x0008;
pub const HOTKEY_ID_UP: i32 = 1;
pub const HOTKEY_ID_DOWN: i32 = 2;

pub const IMAGE_ICON: u32 = 1;
pub const LR_LOADFROMFILE: u32 = 0x0010;
pub const LR_DEFAULTSIZE: u32 = 0x0040;
pub const IDI_APPLICATION: *const u16 = 32512 as *const u16;
pub const IDC_ARROW: *const u16 = 32512 as *const u16;
pub const IDC_HAND: *const u16 = 32649 as *const u16;
pub const IDC_SIZENWSE: *const u16 = 32642 as *const u16;

// ---- GDI ----
pub const PS_DOT: i32 = 2;
pub const PS_SOLID: i32 = 0;
pub const TRANSPARENT: i32 = 1;
pub const OPAQUE: i32 = 2;
pub const SRCCOPY: u32 = 0x00CC0020;
pub const DT_LEFT: u32 = 0x00000000;
pub const DT_CENTER: u32 = 0x00000001;
pub const DT_RIGHT: u32 = 0x00000002;
pub const DT_TOP: u32 = 0x00000000;
pub const DT_VCENTER: u32 = 0x00000004;
pub const DT_BOTTOM: u32 = 0x00000008;
pub const DT_SINGLELINE: u32 = 0x00000020;
pub const DT_WORDBREAK: u32 = 0x00000010;
pub const DT_CALCRECT: u32 = 0x00000400;
pub const DT_NOCLIP: u32 = 0x00000100;
pub const DT_NOPREFIX: u32 = 0x00000800;
pub const FW_NORMAL: i32 = 400;
pub const FW_MEDIUM: i32 = 500;
pub const DEFAULT_CHARSET: u32 = 1;
pub const OUT_DEFAULT_PRECIS: u32 = 0;
pub const CLIP_DEFAULT_PRECIS: u32 = 0;
pub const CLEARTYPE_QUALITY: u32 = 5;
pub const DEFAULT_PITCH: u32 = 0;
pub const FF_DONTCARE: u32 = 0;
pub const RGN_AND: i32 = 1;
pub const RGN_COPY: i32 = 5;

// ---- 托盘 / 菜单 ----
pub const NIF_MESSAGE: u32 = 0x00000001;
pub const NIF_ICON: u32 = 0x00000002;
pub const NIF_TIP: u32 = 0x00000004;
pub const NIF_STATE: u32 = 0x00000008;
pub const NIF_INFO: u32 = 0x00000010;
pub const NIF_GUID: u32 = 0x00000020;
pub const NIM_ADD: u32 = 0x00000000;
pub const NIM_MODIFY: u32 = 0x00000001;
pub const NIM_DELETE: u32 = 0x00000002;
pub const NIM_SETVERSION: u32 = 0x00000004;
pub const NOTIFYICON_VERSION_4: u32 = 4;

pub const MF_STRING: u32 = 0x00000000;
pub const MF_SEPARATOR: u32 = 0x00000800;
pub const MF_GRAYED: u32 = 0x00000001;
pub const MF_POPUP: u32 = 0x00000010;
pub const TPM_LEFTALIGN: u32 = 0x0000;
pub const TPM_RIGHTALIGN: u32 = 0x0008;
pub const TPM_LEFTBUTTON: u32 = 0x0000;
pub const TPM_RIGHTBUTTON: u32 = 0x0002;
pub const TPM_RETURNCMD: u32 = 0x0100;
pub const TPM_NONOTIFY: u32 = 0x0080;

pub const SW_SHOWNORMAL: i32 = 1;
pub const GWL_WNDPROC: i32 = -4;

// ---- 结构体 ----
#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct RECT {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl RECT {
    pub fn new(l: i32, t: i32, r: i32, b: i32) -> Self { RECT { left: l, top: t, right: r, bottom: b } }
    pub fn width(&self) -> i32 { self.right - self.left }
    pub fn height(&self) -> i32 { self.bottom - self.top }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }
    pub fn deflate(&self, d: i32) -> Self { RECT::new(self.left + d, self.top + d, self.right - d, self.bottom - d) }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct POINT { pub x: i32, pub y: i32 }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SIZE { pub cx: i32, pub cy: i32 }

#[repr(C)]
pub struct WNDCLASSW {
    pub style: u32,
    pub lpfnWndProc: WNDPROC,
    pub cbClsExtra: i32,
    pub cbWndExtra: i32,
    pub hInstance: HINSTANCE,
    pub hIcon: HICON,
    pub hCursor: isize,
    pub hbrBackground: HBRUSH,
    pub lpszMenuName: *const u16,
    pub lpszClassName: *const u16,
}

#[repr(C)]
pub struct MSG {
    pub hwnd: HWND,
    pub message: UINT,
    pub wParam: WPARAM,
    pub lParam: LPARAM,
    pub time: DWORD,
    pub pt: POINT,
    pub lPrivate: DWORD,
}

#[repr(C)]
pub struct PAINTSTRUCT {
    pub hdc: HDC,
    pub fErase: BOOL,
    pub rcPaint: RECT,
    pub fRestore: BOOL,
    pub fIncUpdate: BOOL,
    pub rgbReserved: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct GUID { pub data1: u32, pub data2: u16, pub data3: u16, pub data4: [u8; 8] }
impl Default for GUID { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
pub struct NOTIFYICONDATAW {
    pub cbSize: DWORD,
    pub hWnd: HWND,
    pub uID: UINT,
    pub uFlags: UINT,
    pub uCallbackMessage: UINT,
    pub hIcon: HICON,
    pub szTip: [u16; 128],
    pub dwState: DWORD,
    pub dwStateMask: DWORD,
    pub szInfo: [u16; 256],
    pub uVersion: UINT,
    pub szInfoTitle: [u16; 64],
    pub dwInfoFlags: DWORD,
    pub guidItem: GUID,
    pub hBalloonIcon: HICON,
}
impl Default for NOTIFYICONDATAW { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
pub struct MONITORINFOEXW {
    pub cbSize: DWORD,
    pub rcMonitor: RECT,
    pub rcWork: RECT,
    pub dwFlags: DWORD,
    pub szDevice: [u16; 32],
}
impl Default for MONITORINFOEXW { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
pub struct DISPLAY_DEVICEW {
    pub cb: DWORD,
    pub device_name: [u16; 32],
    pub device_string: [u16; 128],
    pub state_flags: DWORD,
    pub device_id: [u16; 128],
    pub device_key: [u16; 128],
}
impl Default for DISPLAY_DEVICEW { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
pub struct SYSTEMTIME {
    pub wYear: u16, pub wMonth: u16, pub wDayOfWeek: u16, pub wDay: u16,
    pub wHour: u16, pub wMinute: u16, pub wSecond: u16, pub wMilliseconds: u16,
}
impl Default for SYSTEMTIME { fn default() -> Self { unsafe { std::mem::zeroed() } } }

/// 对应 C# `RAMP`：3 × 256 个 WORD 的硬件 gamma 查找表。
#[repr(C)]
pub struct RAMP { pub red: [u16; 256], pub green: [u16; 256], pub blue: [u16; 256] }
impl Default for RAMP { fn default() -> Self { unsafe { std::mem::zeroed() } } }

/// 对应 C# `MAGCOLOREFFECT`：5×5 颜色变换矩阵。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct MAGCOLOREFFECT { pub transform: [f32; 25] }
impl Default for MAGCOLOREFFECT { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MINMAXINFO {
    pub ptReserved: POINT,
    pub ptMaxSize: POINT,
    pub ptMaxPosition: POINT,
    pub ptMinTrackSize: POINT,
    pub ptMaxTrackSize: POINT,
}

// ---- user32 ----
#[link(name = "user32")]
extern "system" {
    pub fn RegisterClassW(lpWndClass: *const WNDCLASSW) -> ATOM;
    pub fn CreateWindowExW(dwExStyle: u32, lpClassName: *const u16, lpWindowName: *const u16,
        dwStyle: u32, x: i32, y: i32, nWidth: i32, nHeight: i32,
        hWndParent: HWND, hMenu: HMENU, hInstance: HINSTANCE, lpParam: *const c_void) -> HWND;
    pub fn DestroyWindow(hWnd: HWND) -> BOOL;
    pub fn ShowWindow(hWnd: HWND, nCmdShow: i32) -> BOOL;
    pub fn IsWindowVisible(hWnd: HWND) -> BOOL;
    pub fn SetWindowPos(hWnd: HWND, hWndInsertAfter: HWND, x: i32, y: i32, cx: i32, cy: i32, uFlags: u32) -> BOOL;
    pub fn MoveWindow(hWnd: HWND, x: i32, y: i32, w: i32, h: i32, repaint: BOOL) -> BOOL;
    pub fn GetWindowRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn GetClientRect(hWnd: HWND, lpRect: *mut RECT) -> BOOL;
    pub fn SetWindowRgn(hWnd: HWND, hRgn: HRGN, bRedraw: BOOL) -> i32;
    pub fn SetWindowLongPtrW(hWnd: HWND, nIndex: i32, dwNewLong: isize) -> isize;
    pub fn GetWindowLongPtrW(hWnd: HWND, nIndex: i32) -> isize;
    pub fn CallWindowProcW(lpPrevWndFunc: isize, hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn DefWindowProcW(hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn GetMessageW(lpMsg: *mut MSG, hWnd: HWND, wMsgFilterMin: UINT, wMsgFilterMax: UINT) -> BOOL;
    pub fn PeekMessageW(lpMsg: *mut MSG, hWnd: HWND, min: UINT, max: UINT, remove: UINT) -> BOOL;
    pub fn MsgWaitForMultipleObjectsEx(nCount: u32, pHandles: *const isize, dwMilliseconds: u32,
                                       dwWakeMask: u32, dwFlags: u32) -> u32;
    pub fn TranslateMessage(lpMsg: *const MSG) -> BOOL;
    pub fn DispatchMessageW(lpMsg: *const MSG) -> LRESULT;
    pub fn PostQuitMessage(nExitCode: i32);
    pub fn PostMessageW(hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> BOOL;
    pub fn SendMessageW(hWnd: HWND, msg: UINT, wParam: WPARAM, lParam: LPARAM) -> LRESULT;
    pub fn SetTimer(hWnd: HWND, nIDEvent: usize, uElapse: u32, lpTimerFunc: *const c_void) -> usize;
    pub fn KillTimer(hWnd: HWND, nIDEvent: usize) -> BOOL;
    pub fn SetCapture(hWnd: HWND) -> HWND;
    pub fn ReleaseCapture() -> BOOL;
    pub fn GetCursorPos(lpPoint: *mut POINT) -> BOOL;
    pub fn GetAsyncKeyState(vKey: i32) -> i16;
    pub fn SetForegroundWindow(hWnd: HWND) -> BOOL;
    pub fn EnableWindow(hWnd: HWND, enable: BOOL) -> BOOL;
    pub fn SetFocus(hWnd: HWND) -> HWND;
    pub fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: BOOL) -> BOOL;
    pub fn UpdateWindow(hWnd: HWND) -> BOOL;
    pub fn BeginPaint(hWnd: HWND, lpPaint: *mut PAINTSTRUCT) -> HDC;
    pub fn EndPaint(hWnd: HWND, lpPaint: *const PAINTSTRUCT) -> BOOL;
    pub fn GetDC(hWnd: HWND) -> HDC;
    pub fn ReleaseDC(hWnd: HWND, hDC: HDC) -> i32;
    pub fn GetSystemMetrics(nIndex: i32) -> i32;
    pub fn GetDpiForWindow(hwnd: HWND) -> u32;
    pub fn SetProcessDpiAwarenessContext(value: isize) -> BOOL;
    pub fn LoadCursorW(hInstance: HINSTANCE, lpCursorName: *const u16) -> isize;
    pub fn LoadIconW(hInstance: HINSTANCE, lpIconName: *const u16) -> HICON;
    pub fn LoadImageW(hInst: HINSTANCE, name: *const u16, kind: u32, cx: i32, cy: i32, fuLoad: u32) -> isize;
    pub fn DrawIconEx(hdc: HDC, x: i32, y: i32, hIcon: HICON, cx: i32, cy: i32,
        istepIfAniCur: u32, hbrFlickerFreeDraw: HBRUSH, diFlags: u32) -> BOOL;
    pub fn DestroyIcon(hIcon: HICON) -> BOOL;
    pub fn CreateIconFromResourceEx(pbIconBits: *const u8, cbIconBits: u32, fIcon: BOOL,
        dwVersion: u32, cxDesired: i32, cyDesired: i32, uFlags: u32) -> HICON;
    pub fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> HWND;
    pub fn ClientToScreen(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn ScreenToClient(hWnd: HWND, lpPoint: *mut POINT) -> BOOL;
    pub fn EnumDisplayMonitors(hdc: HDC, lprcClip: *const RECT,
        lpfnEnum: Option<unsafe extern "system" fn(HWND, HDC, *mut RECT, LPARAM) -> BOOL>, dwData: LPARAM) -> BOOL;
    pub fn GetMonitorInfoW(hMonitor: HWND, lpmi: *mut MONITORINFOEXW) -> BOOL;
    pub fn EnumDisplayDevicesW(lpDevice: *const u16, iDevNum: u32,
        lpDisplayDevice: *mut DISPLAY_DEVICEW, dwFlags: u32) -> BOOL;
    pub fn RegisterHotKey(hWnd: HWND, id: i32, fsModifiers: u32, vk: u32) -> BOOL;
    pub fn UnregisterHotKey(hWnd: HWND, id: i32) -> BOOL;
    pub fn CreatePopupMenu() -> HMENU;
    pub fn DestroyMenu(hMenu: HMENU) -> BOOL;
    pub fn AppendMenuW(hMenu: HMENU, uFlags: u32, uIDNewItem: usize, lpNewItem: *const u16) -> BOOL;
    pub fn TrackPopupMenuEx(hMenu: HMENU, uFlags: u32, x: i32, y: i32, hWnd: HWND, lptpm: *const c_void) -> u32;
    pub fn MessageBoxW(hWnd: HWND, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    pub fn GetKeyNameTextW(lParam: LPARAM, lpString: *mut u16, cchSize: i32) -> i32;
    pub fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
    pub fn GetDoubleClickTime() -> u32;
    pub fn SetWindowTextW(hWnd: HWND, lpString: *const u16) -> BOOL;
    pub fn RedrawWindow(hWnd: HWND, lprcUpdate: *const RECT, hrgnUpdate: HRGN, flags: u32) -> BOOL;
}

// ---- gdi32 ----
#[link(name = "gdi32")]
extern "system" {
    pub fn CreateCompatibleDC(hdc: HDC) -> HDC;
    pub fn CreateCompatibleBitmap(hdc: HDC, cx: i32, cy: i32) -> HBITMAP;
    pub fn CreateSolidBrush(crColor: COLORREF) -> HBRUSH;
    pub fn CreatePen(iStyle: i32, cWidth: i32, crColor: COLORREF) -> HPEN;
    pub fn SelectObject(hdc: HDC, h: HGDIOBJ) -> HGDIOBJ;
    pub fn DeleteObject(ho: HGDIOBJ) -> BOOL;
    pub fn DeleteDC(hdc: HDC) -> BOOL;
    pub fn BitBlt(hdc: HDC, x: i32, y: i32, cx: i32, cy: i32, hdcSrc: HDC, x1: i32, y1: i32, rop: u32) -> BOOL;
    pub fn FillRect(hdc: HDC, lprc: *const RECT, hbr: HBRUSH) -> i32;
    pub fn RoundRect(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32, w: i32, h: i32) -> BOOL;
    pub fn Rectangle(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    pub fn Ellipse(hdc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> BOOL;
    pub fn DrawTextW(hdc: HDC, lpString: *const u16, nCount: i32, lpRect: *mut RECT, uFormat: u32) -> i32;
    pub fn DrawFocusRect(hdc: HDC, lprc: *const RECT) -> BOOL;
    pub fn TextOutW(hdc: HDC, x: i32, y: i32, lpString: *const u16, c: i32) -> BOOL;
    pub fn SetBkMode(hdc: HDC, mode: i32) -> i32;
    pub fn SetTextColor(hdc: HDC, color: COLORREF) -> COLORREF;
    pub fn CreateFontW(cHeight: i32, cWidth: i32, cEscapement: i32, cOrientation: i32, cWeight: i32,
        bItalic: u32, bUnderline: u32, bStrikeOut: u32, iCharSet: u32, iOutPrecision: u32,
        iClipPrecision: u32, iQuality: u32, iPitchAndFamily: u32, pszFaceName: *const u16) -> HFONT;
    pub fn GetTextExtentPoint32W(hdc: HDC, lpString: *const u16, c: i32, lpSize: *mut SIZE) -> BOOL;
    pub fn CreateRoundRectRgn(x1: i32, y1: i32, x2: i32, y2: i32, w: i32, h: i32) -> HRGN;
    pub fn MoveToEx(hdc: HDC, x: i32, y: i32, lpPoint: *mut POINT) -> BOOL;
    pub fn LineTo(hdc: HDC, x: i32, y: i32) -> BOOL;
    pub fn CreateDCW(lpszDriver: *const u16, lpszDevice: *const u16, lpszPort: *const u16, pData: *const c_void) -> HDC;
    pub fn GetDeviceGammaRamp(hdc: HDC, lpRamp: *mut RAMP) -> BOOL;
    pub fn SetDeviceGammaRamp(hdc: HDC, lpRamp: *const RAMP) -> BOOL;
}

// ---- kernel32 ----
#[link(name = "kernel32")]
extern "system" {
    pub fn GetModuleHandleW(lpModuleName: *const u16) -> HINSTANCE;
    pub fn CreateMutexW(lpMutexAttributes: *const c_void, bInitialOwner: BOOL, lpName: *const u16) -> HANDLE;
    pub fn CloseHandle(hObject: HANDLE) -> BOOL;
    pub fn GetLastError() -> DWORD;
    pub fn GetLocalTime(lpSystemTime: *mut SYSTEMTIME);
    pub fn GetTickCount64() -> u64;
    pub fn QueryPerformanceCounter(v: *mut i64) -> BOOL;
    pub fn QueryPerformanceFrequency(v: *mut i64) -> BOOL;
    pub fn LoadLibraryW(lpLibFileName: *const u16) -> HMODULE;
    pub fn GetProcAddress(hModule: HMODULE, lpProcName: *const u8) -> *const c_void;
    pub fn FreeLibrary(hModule: HMODULE) -> BOOL;
    pub fn GetTempPathW(nBufferLength: u32, lpBuffer: *mut u16) -> u32;
    pub fn ExitProcess(uExitCode: u32) -> !;
}

// ---- advapi32（对应 RegistryServices.cs 的自启动注册表读写） ----
pub const HKEY_CURRENT_USER: isize = 0x80000001u32 as isize;
pub const KEY_QUERY_VALUE: u32 = 0x0001;
pub const KEY_SET_VALUE: u32 = 0x0002;
pub const REG_SZ: u32 = 1;

#[link(name = "advapi32")]
extern "system" {
    pub fn RegOpenKeyExW(hKey: isize, lpSubKey: *const u16, ulOptions: u32, samDesired: u32, phkResult: *mut isize) -> i32;
    pub fn RegSetValueExW(hKey: isize, lpValueName: *const u16, reserved: u32, dwType: u32, lpData: *const u8, cbData: u32) -> i32;
    pub fn RegQueryValueExW(hKey: isize, lpValueName: *const u16, lpReserved: *mut u32, lpType: *mut u32, lpData: *mut u8, lpcbData: *mut u32) -> i32;
    pub fn RegDeleteValueW(hKey: isize, lpValueName: *const u16) -> i32;
    pub fn RegCloseKey(hKey: isize) -> i32;
}

// ---- shell32 ----
#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconW(dwMessage: DWORD, lpData: *mut NOTIFYICONDATAW) -> BOOL;
    pub fn ShellExecuteW(hwnd: HWND, lpOperation: *const u16, lpFile: *const u16,
        lpParameters: *const u16, lpDirectory: *const u16, nShowCmd: i32) -> isize;
}

// ---- 动态加载：Magnification.dll（对应 HardwareGammaService 的 Mag* 调用） ----
pub type FnMagInitialize = unsafe extern "system" fn() -> BOOL;
pub type FnMagUninitialize = unsafe extern "system" fn() -> BOOL;
pub type FnMagSetFullscreenColorEffect = unsafe extern "system" fn(*const MAGCOLOREFFECT) -> BOOL;

pub struct Magnification {
    pub init: Option<FnMagInitialize>,
    pub uninit: Option<FnMagUninitialize>,
    pub set_effect: Option<FnMagSetFullscreenColorEffect>,
}
impl Magnification {
    pub fn load() -> Self {
        let empty = Magnification { init: None, uninit: None, set_effect: None };
        unsafe {
            let lib = wide("Magnification.dll");
            let h = LoadLibraryW(lib.as_ptr());
            if h == NULL { return empty; }
            let get = |name: &[u8]| -> *const c_void {
                let mut n = name.to_vec(); n.push(0);
                GetProcAddress(h, n.as_ptr())
            };
            let a = get(b"MagInitialize");
            let b = get(b"MagUninitialize");
            let c = get(b"MagSetFullscreenColorEffect");
            if a.is_null() || b.is_null() || c.is_null() { return empty; }
            Magnification {
                init: Some(std::mem::transmute::<*const c_void, FnMagInitialize>(a)),
                uninit: Some(std::mem::transmute::<*const c_void, FnMagUninitialize>(b)),
                set_effect: Some(std::mem::transmute::<*const c_void, FnMagSetFullscreenColorEffect>(c)),
            }
        }
    }
}

// ---- 1:1 补齐：会话 / 电源 / 关机 / 钩子 / DWM / DisplayConfig 常量 ----
pub const WM_QUERYENDSESSION: UINT = 0x0011;
pub const WM_ENDSESSION: UINT = 0x0016;
pub const WM_MOUSEWHEEL_HOOK: UINT = 0x020A;
pub const WH_MOUSE_LL: i32 = 14;
pub const HC_ACTION: i32 = 0;
pub const WTS_SESSION_LOCK: usize = 0x7;
pub const WTS_SESSION_UNLOCK: usize = 0x8;
pub const NOTIFY_FOR_THIS_SESSION: u32 = 0;
pub const PBT_APMRESUMEAUTOMATIC: usize = 0x12;
pub const PBT_APMRESUMESUSPEND: usize = 0x7;
pub const PBT_APMSUSPEND: usize = 0x4;
pub const SPI_GETDROPSHADOW: u32 = 0x1024;
pub const DWMWA_USE_IMMERSIVE_DARK_MODE: i32 = 20;
pub const DWMWA_WINDOW_CORNER_PREFERENCE: i32 = 33;
pub const DWMWCP_ROUND: i32 = 2;
pub const MDT_EFFECTIVE_DPI: i32 = 0;
pub const DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME: u32 = 1;
pub const DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME: u32 = 2;
pub const DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO: u32 = 9;
pub const ERROR_SUCCESS: i32 = 0;

#[repr(C)]
#[derive(Clone, Copy, Default, Debug)]
pub struct LUID { pub low_part: u32, pub high_part: i32 }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DISPLAYCONFIG_DEVICE_INFO_HEADER {
    pub kind: u32, pub size: u32, pub adapter_id: LUID, pub id: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_SOURCE_DEVICE_NAME {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub view_gdi_device_name: [u16; 32],
}
impl Default for DISPLAYCONFIG_SOURCE_DEVICE_NAME { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_TARGET_DEVICE_NAME {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub flags: u32, pub output_technology: u32,
    pub edid_manufacture_id: u16, pub edid_product_code_id: u16, pub connector_instance: u32,
    pub monitor_friendly_device_name: [u16; 64],
    pub monitor_device_path: [u16; 128],
}
impl Default for DISPLAYCONFIG_TARGET_DEVICE_NAME { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub value: u32, pub color_encoding: u32, pub bits_per_color_channel: u32,
}
impl Default for DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct Rational { pub numerator: u32, pub denominator: u32 }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DISPLAYCONFIG_PATH_SOURCE_INFO {
    pub adapter_id: LUID, pub id: u32, pub mode_info_idx: u32, pub status_flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DISPLAYCONFIG_PATH_TARGET_INFO {
    pub adapter_id: LUID, pub id: u32, pub mode_info_idx: u32,
    pub output_technology: u32, pub rotation: u32, pub scaling: u32,
    pub refresh_rate: Rational, pub scanline_ordering: u32,
    pub target_available: u32, pub status_flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct DISPLAYCONFIG_PATH_INFO {
    pub source_info: DISPLAYCONFIG_PATH_SOURCE_INFO,
    pub target_info: DISPLAYCONFIG_PATH_TARGET_INFO,
    pub flags: u32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct DISPLAYCONFIG_MODE_INFO {
    pub info_type: u32, pub id: u32, pub adapter_id: LUID, pub mode_info: [u8; 64],
}
impl Default for DISPLAYCONFIG_MODE_INFO { fn default() -> Self { unsafe { std::mem::zeroed() } } }

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct NOTIFYICONIDENTIFIER {
    pub cb_size: u32, pub hwnd: HWND, pub uid: u32, pub guid_item: GUID,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct MSLLHOOKSTRUCT {
    pub pt: POINT, pub mouse_data: i32, pub flags: i32, pub time: i32, pub dw_extra_info: isize,
}

#[link(name = "user32")]
extern "system" {
    pub fn SetWindowsHookExW(id_hook: i32, lpfn: *const c_void, h_mod: HINSTANCE, thread_id: u32) -> isize;
    pub fn UnhookWindowsHookEx(hhk: isize) -> BOOL;
    pub fn CallNextHookEx(hhk: isize, code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
    pub fn SetCursorPos(x: i32, y: i32) -> BOOL;
    pub fn SystemParametersInfoW(action: u32, param: u32, value: *mut c_void, win_ini: u32) -> BOOL;
    pub fn GetDisplayConfigBufferSizes(flags: u32, paths: *mut u32, modes: *mut u32) -> i32;
    pub fn QueryDisplayConfig(flags: u32, path_count: *mut u32, paths: *mut DISPLAYCONFIG_PATH_INFO,
        mode_count: *mut u32, modes: *mut DISPLAYCONFIG_MODE_INFO, topology: *mut u32) -> i32;
    pub fn DisplayConfigGetDeviceInfo(request: *mut c_void) -> i32;
    pub fn MonitorFromPoint(pt: POINT, flags: u32) -> HWND;
    pub fn MonitorFromWindow(hwnd: HWND, flags: u32) -> HWND;
}

#[link(name = "shell32")]
extern "system" {
    pub fn Shell_NotifyIconGetRect(id: *const NOTIFYICONIDENTIFIER, rect: *mut RECT) -> i32;
    pub fn ExtractIconExW(szFileName: *const u16, nIconIndex: i32,
                         phiconLarge: *mut isize, phiconSmall: *mut isize, nIcons: u32) -> u32;
}

// ---- 动态加载：这些库没有稳定导入库可用 ----
pub type FnDwmSetWindowAttribute = unsafe extern "system" fn(HWND, i32, *const c_void, u32) -> i32;
pub type FnGetDpiForMonitor = unsafe extern "system" fn(HWND, i32, *mut u32, *mut u32) -> i32;
pub type FnWtsRegister = unsafe extern "system" fn(HWND, u32) -> BOOL;
pub type FnTimePeriod = unsafe extern "system" fn(u32) -> u32;
pub type FnWtsUnregister = unsafe extern "system" fn(HWND) -> BOOL;
pub type FnGetCurrentPackageFullName = unsafe extern "system" fn(*mut u32, *mut u16) -> i32;

pub struct ExtraApis {
    pub dwm_set: Option<FnDwmSetWindowAttribute>,
    pub dpi_for_monitor: Option<FnGetDpiForMonitor>,
    pub wts_register: Option<FnWtsRegister>,
    pub wts_unregister: Option<FnWtsUnregister>,
    pub package_full_name: Option<FnGetCurrentPackageFullName>,
    /// winmm 的 timeBeginPeriod / timeEndPeriod：提高系统时钟粒度，让 16ms 动画定时器稳定。
    pub time_begin: Option<FnTimePeriod>,
    pub time_end: Option<FnTimePeriod>,
}
impl ExtraApis {
    pub fn load() -> Self {
        unsafe fn proc(h: HMODULE, name: &[u8]) -> *const c_void {
            let mut n = name.to_vec(); n.push(0);
            unsafe { GetProcAddress(h, n.as_ptr()) }
        }
        unsafe {
            let get = |lib: &str| -> HMODULE {
                let w = wide(lib);
                LoadLibraryW(w.as_ptr())
            };
            let dwm = get("dwmapi.dll");
            let shcore = get("shcore.dll");
            let wts = get("wtsapi32.dll");
            let k32 = get("kernel32.dll");
            let winmm = get("winmm.dll");
            let mut out = ExtraApis { dwm_set: None, dpi_for_monitor: None,
                wts_register: None, wts_unregister: None, package_full_name: None,
                time_begin: None, time_end: None };
            let p = proc(dwm, b"DwmSetWindowAttribute");
            if !p.is_null() { out.dwm_set = Some(std::mem::transmute::<*const c_void, FnDwmSetWindowAttribute>(p)); }
            let p = proc(shcore, b"GetDpiForMonitor");
            if !p.is_null() { out.dpi_for_monitor = Some(std::mem::transmute::<*const c_void, FnGetDpiForMonitor>(p)); }
            let p = proc(wts, b"WTSRegisterSessionNotification");
            if !p.is_null() { out.wts_register = Some(std::mem::transmute::<*const c_void, FnWtsRegister>(p)); }
            let p = proc(wts, b"WTSUnRegisterSessionNotification");
            if !p.is_null() { out.wts_unregister = Some(std::mem::transmute::<*const c_void, FnWtsUnregister>(p)); }
            let p = proc(k32, b"GetCurrentPackageFullName");
            if !p.is_null() { out.package_full_name = Some(std::mem::transmute::<*const c_void, FnGetCurrentPackageFullName>(p)); }
            let p = proc(winmm, b"timeBeginPeriod");
            if !p.is_null() { out.time_begin = Some(std::mem::transmute::<*const c_void, FnTimePeriod>(p)); }
            let p = proc(winmm, b"timeEndPeriod");
            if !p.is_null() { out.time_end = Some(std::mem::transmute::<*const c_void, FnTimePeriod>(p)); }
            out
        }
    }
    pub fn apply_window_attributes(&self, hwnd: HWND) {
        unsafe {
            if let Some(f) = self.dwm_set {
                let round = DWMWCP_ROUND;
                f(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const c_void, 4);
                let dark = if apps_use_dark_theme() { 1i32 } else { 0i32 };
                f(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const c_void, 4);
            }
        }
    }
    pub fn is_msix(&self) -> bool {
        unsafe {
            match self.package_full_name {
                Some(f) => {
                    let mut len = 0u32;
                    f(&mut len, std::ptr::null_mut());
                    len > 0
                }
                None => false,
            }
        }
    }
}

/// 对应 `AppInfoWindow.OnSourceInitialized` 读取 `AppsUseLightTheme`。
pub fn apps_use_dark_theme() -> bool {
    unsafe {
        let sub = wide("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
        let name = wide("AppsUseLightTheme");
        let mut hk: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub.as_ptr(), 0, KEY_QUERY_VALUE, &mut hk) != 0 { return false; }
        let mut val: u32 = 1;
        let mut size = 4u32;
        let mut kind = 0u32;
        let r = RegQueryValueExW(hk, name.as_ptr(), std::ptr::null_mut(), &mut kind,
            &mut val as *mut _ as *mut u8, &mut size);
        RegCloseKey(hk);
        r == 0 && kind == 4 /* REG_DWORD */ && val == 0
    }
}

// ---- 工具 ----
/// 高精度计时（微秒级），用于性能实测。
pub const PM_REMOVE: u32 = 0x0001;
pub const QS_ALLINPUT: u32 = 0x04FF;
pub const MWMO_INPUTAVAILABLE: u32 = 0x0004;
pub const WAIT_TIMEOUT: u32 = 0x0102;
pub const INFINITE: u32 = 0xFFFF_FFFF;
pub const WM_QUIT: UINT = 0x0012;

pub fn qpc_now() -> i64 {
    let mut v: i64 = 0;
    unsafe { QueryPerformanceCounter(&mut v); }
    v
}
pub fn qpc_ms(delta: i64) -> f64 {
    let mut f: i64 = 0;
    unsafe { QueryPerformanceFrequency(&mut f); }
    if f <= 0 { 0.0 } else { delta as f64 * 1000.0 / f as f64 }
}

/// 高精度计时（微秒级），用于性能实测。
pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
pub const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}
pub fn loword(v: usize) -> i32 { (v & 0xFFFF) as u16 as i32 }
pub fn hiword(v: usize) -> i32 { ((v >> 16) & 0xFFFF) as u16 as i32 }
pub fn lparam_x(v: LPARAM) -> i32 { (v & 0xFFFF) as u16 as i16 as i32 }
pub fn lparam_y(v: LPARAM) -> i32 { ((v >> 16) & 0xFFFF) as u16 as i16 as i32 }
pub fn from_wide(p: *const u16) -> String {
    if p.is_null() { return String::new(); }
    let mut n = 0usize;
    while unsafe { *p.add(n) } != 0 { n += 1; if n > 4096 { break; } }
    String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
}
pub fn from_wide_buf(b: &[u16]) -> String {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf16_lossy(&b[..n])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn displayconfig_layout_matches_sdk() {
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_DEVICE_INFO_HEADER>(), 20);
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_PATH_INFO>(), 72);
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_MODE_INFO>(), 80);
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>(), 84);
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>(), 420);
        assert_eq!(std::mem::size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>(), 32);
        assert_eq!(std::mem::size_of::<NOTIFYICONIDENTIFIER>(), 40);
        assert_eq!(std::mem::size_of::<RAMP>(), 1536);
    }
    #[test]
    fn colorref_order() { assert_eq!(rgb(0x11, 0x22, 0x33), 0x332211); }
}
