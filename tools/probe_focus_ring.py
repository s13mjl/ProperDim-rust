"""探针：在同一个位置点两次滑块（亮度不变），比对两次截图。
若还画焦点框，第二次会比第一次多出一圈边线。只读观察。"""
import ctypes
import ctypes.wintypes as wt
import subprocess
import time
import os
import sys
from PIL import ImageGrab, ImageChops

u32 = ctypes.WinDLL("user32", use_last_error=True)
u32.FindWindowW.restype = wt.HWND
u32.FindWindowW.argtypes = [wt.LPCWSTR, wt.LPCWSTR]
u32.PostMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
u32.GetWindowRect.argtypes = [wt.HWND, ctypes.POINTER(wt.RECT)]
u32.SetWindowPos.argtypes = [wt.HWND, wt.HWND, ctypes.c_int, ctypes.c_int,
                             ctypes.c_int, ctypes.c_int, wt.UINT]

WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP = 0x0200, 0x0201, 0x0202
SWP_NOSIZE, SWP_NOMOVE, SWP_SHOWWINDOW = 0x0001, 0x0002, 0x0040


def lp(x, y):
    return (y << 16) | (x & 0xFFFF)


def click(h, x, y):
    u32.PostMessageW(h, WM_MOUSEMOVE, 0, lp(x, y))
    time.sleep(0.02)
    u32.PostMessageW(h, WM_LBUTTONDOWN, 1, lp(x, y))
    time.sleep(0.05)
    u32.PostMessageW(h, WM_LBUTTONUP, 0, lp(x, y))
    time.sleep(0.25)


exe = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "target", "release", "ProperDim.exe")
p = subprocess.Popen([exe, "--demo"])
time.sleep(2.5)
try:
    panel = u32.FindWindowW("ProperDimRustControlPanel", None)
    if not panel:
        print("!! 没有控制面板")
        sys.exit(1)
    u32.SetWindowPos(panel, -1, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_SHOWWINDOW)
    u32.SetForegroundWindow(panel)
    time.sleep(0.5)
    r = wt.RECT()
    u32.GetWindowRect(panel, ctypes.byref(r))
    box = (r.left, r.top, r.right, r.bottom)

    # 先点到 100%（最右端），亮度已是 100% —— 再点同一位置值不变
    click(panel, 324, 92)
    a = ImageGrab.grab(bbox=box).convert("RGB")
    click(panel, 324, 92)
    b = ImageGrab.grab(bbox=box).convert("RGB")

    diff = ImageChops.difference(a, b)
    n = sum(1 for px in diff.getdata() if sum(px) > 24)
    print("两次点击同一位置后的差异像素:", n)

    # 单独检查滑块矩形四周的边框行（若有焦点框，这些位置最亮）
    w, h = b.size
    edge_rows = list(range(80, 84)) + list(range(100, 105))
    bright = 0
    for y in edge_rows:
        for x in range(18, w - 18):
            px = b.getpixel((x, y))
            if min(px) > 150:      # 近白
                bright += 1
    print("滑块上下边框上的近白像素:", bright)
    b.save("preview/_focus_check.png")

    # Tab 键：焦点框应当出现在某个控件上（键盘聚焦才显示）
    u32.PostMessageW(panel, 0x0100, 9, 0)   # WM_KEYDOWN VK_TAB
    time.sleep(0.25)
    c = ImageGrab.grab(bbox=box).convert("RGB")
    n_tab = sum(1 for px in ImageChops.difference(b, c).getdata() if sum(px) > 24)
    print("按 Tab 后的差异像素:", n_tab, "（>0 说明键盘焦点框有画出来）")
    c.save("preview/_focus_tab.png")
    print("OK" if n < 40 and bright < 30 else "!! 仍有多余边框")
finally:
    p.terminate()
    try:
        p.wait(timeout=3)
    except Exception:
        p.kill()
