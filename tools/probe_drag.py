"""探针：模拟拖动亮度滑块，检查「Brightness Level」数字是否跟随、圆点是否移动。只读观察。"""
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
u32.SetForegroundWindow.argtypes = [wt.HWND]
u32.SetWindowPos.argtypes = [wt.HWND, wt.HWND, ctypes.c_int, ctypes.c_int,
                             ctypes.c_int, ctypes.c_int, wt.UINT]
SWP_NOSIZE, SWP_NOMOVE, SWP_SHOWWINDOW = 0x0001, 0x0002, 0x0040

WM_MOUSEMOVE, WM_LBUTTONDOWN, WM_LBUTTONUP = 0x0200, 0x0201, 0x0202


def lp(x, y):
    return (y << 16) | (x & 0xFFFF)


def rect(h):
    r = wt.RECT()
    u32.GetWindowRect(h, ctypes.byref(r))
    return (r.left, r.top, r.right, r.bottom)


def changed(a, b):
    """返回变化像素数"""
    diff = ImageChops.difference(a.convert("RGB"), b.convert("RGB"))
    return sum(1 for p in diff.getdata() if sum(p) > 24)


exe = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "target", "release", "ProperDim.exe")
p = subprocess.Popen([exe, "--demo"])
time.sleep(2.5)
try:
    panel = u32.FindWindowW("ProperDimRustControlPanel", None)
    if not panel:
        print("!! 没有控制面板")
        sys.exit(1)
    # 后台进程的 SetForegroundWindow 常被拒绝，直接顶到最上层
    u32.SetWindowPos(panel, -1, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_SHOWWINDOW)
    u32.SetForegroundWindow(panel)
    time.sleep(0.5)
    l, t, r, b = rect(panel)
    print("panel rect", (l, t, r, b))

    before = ImageGrab.grab(bbox=(l, t, r, b))
    # 滑块：逻辑 y=82..102 → 客户端中心 y≈92；轨道 x 从 25 到 w-25
    w = r - l
    mid_y = 92
    u32.PostMessageW(panel, WM_LBUTTONDOWN, 1, lp(40, mid_y))
    time.sleep(0.05)
    for x in range(40, int(w * 0.75), 6):
        u32.PostMessageW(panel, WM_MOUSEMOVE, 1, lp(x, mid_y))
        time.sleep(0.004)          # ≈250Hz，模拟高刷鼠标
    time.sleep(0.15)
    during = ImageGrab.grab(bbox=(l, t, r, b))
    u32.PostMessageW(panel, WM_LBUTTONUP, 0, lp(int(w * 0.75), mid_y))
    time.sleep(0.3)
    after = ImageGrab.grab(bbox=(l, t, r, b))

    # 数字区域（Brightness Level 右侧的百分比）：y≈58..80，x≈150..230
    box = (150, 56, 235, 82)
    n_before = changed(before.crop(box), during.crop(box))
    n_after = changed(during.crop(box), after.crop(box))
    print("数字区域 变化像素: 拖动中 %d, 松手后 %d" % (n_before, n_after))
    print("整窗 变化像素: 拖动中 %d" % changed(before, during))
    before.crop((0, 0, w, b - t)).save("preview/_drag_before.png")
    during.crop((0, 0, w, b - t)).save("preview/_drag_during.png")
    print("OK" if n_before > 40 else "!! 数字没跟着变")
finally:
    p.terminate()
    try:
        p.wait(timeout=3)
    except Exception:
        p.kill()
