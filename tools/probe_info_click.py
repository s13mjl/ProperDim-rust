"""探针：真实点击 Info 窗口的 Cool 按钮，记录各窗口可见性变化。只读观察，不改代码。"""
import ctypes
import ctypes.wintypes as wt
import subprocess
import time
import sys
import os
from PIL import ImageGrab

u32 = ctypes.WinDLL("user32", use_last_error=True)
u32.FindWindowW.restype = wt.HWND
u32.FindWindowW.argtypes = [wt.LPCWSTR, wt.LPCWSTR]
u32.IsWindowVisible.argtypes = [wt.HWND]
u32.PostMessageW.argtypes = [wt.HWND, wt.UINT, wt.WPARAM, wt.LPARAM]
u32.ShowWindow.argtypes = [wt.HWND, ctypes.c_int]

CLASSES = ["ProperDimRustControlPanel", "ProperDimRustInfo", "ProperDimRustTrayMenu",
           "ProperDimRustSchedule", "ProperDimRustMin", "ProperDimRustWarning"]

WM_MOUSEMOVE = 0x0200
WM_LBUTTONDOWN = 0x0201
WM_LBUTTONUP = 0x0202
WM_KEYDOWN = 0x0100


def wnd(cls):
    return u32.FindWindowW(cls, None)


def click(hwnd, x, y):
    lp = (y << 16) | (x & 0xFFFF)
    u32.PostMessageW(hwnd, WM_MOUSEMOVE, 0, lp)
    time.sleep(0.03)
    u32.PostMessageW(hwnd, WM_LBUTTONDOWN, 1, lp)
    time.sleep(0.05)
    u32.PostMessageW(hwnd, WM_LBUTTONUP, 0, lp)
    time.sleep(0.25)


def snap(tag):
    row = []
    for c in CLASSES:
        h = wnd(c)
        row.append("%s=%s" % (c.replace("ProperDimRust", ""), "vis" if h and u32.IsWindowVisible(h) else ("hidden" if h else "-")))
    print("%-22s %s" % (tag, "  ".join(row)))
    return row


exe = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "target", "release", "ProperDim.exe")
p = subprocess.Popen([exe, "--demo"])
time.sleep(2.5)
try:
    snap("启动后")
    panel = wnd("ProperDimRustControlPanel")
    if not panel:
        print("!! 控制面板没出现")
        sys.exit(1)
    # 切到 Options 页（第 4 个 Tab：x≈5+3*tw, y=31..55）
    click(panel, 300, 43)
    snap("点 Options 页")
    # 点 "Info..." 链接（窗口右下 w-58, h-54）
    click(panel, 317, 154)
    snap("点 Info...")
    info = wnd("ProperDimRustInfo")
    if info:
        # Info 窗口 "Cool" 按钮：(w-70)/2 .. +70, y=322..347
        click(info, 160, 304)
        burst = []
        for _ in range(6):
            try:
                g = ImageGrab.grab(bbox=(785, 440, 1135, 640)).convert("RGB")
                px = list(g.resize((12, 8)).getdata())
                burst.append(round(sum(sum(q) for q in px) / len(px) / 3))
            except Exception:
                burst.append(-1)
        print("返回瞬间(面板本就可见) 6 帧平均亮度:", burst)
        snap("点 Cool")
        time.sleep(0.6)
        snap("Cool 之后 0.6s")

        # 复现用户场景：面板本身是隐的（从 QuickDim/托盘菜单进 Info），Cool 应「返回」到面板
        click(panel, 317, 154)          # 再打开一次 Info
        time.sleep(0.3)
        u32.ShowWindow(panel, 0)        # SW_HIDE 模拟「面板本来是隐藏的」
        time.sleep(0.2)
        snap("面板置为隐藏")
        info2 = wnd("ProperDimRustInfo")
        click(info2, 160, 312)
        # 连拍：看「返回」瞬间主窗口有没有露出未绘制的空白帧
        rect = (785, 440, 1135, 640)
        frames = []
        for _ in range(8):
            try:
                g = ImageGrab.grab(bbox=rect).convert("RGB")
                px = list(g.resize((12, 8)).getdata())
                frames.append(round(sum(sum(p) for p in px) / len(px) / 3))
            except Exception:
                frames.append(-1)
            time.sleep(0.012)
        print("返回瞬间 8 帧平均亮度:", frames)
        snap("隐藏态下点 Cool")
    else:
        print("!! Info 窗口未创建")
finally:
    p.terminate()
    try:
        p.wait(timeout=3)
    except Exception:
        p.kill()
