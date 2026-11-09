"""前端预览截图工具：启动 --demo 模式，模拟点击切换 4 个 Tab，拼出一张总览图。
仅用于开发验收，不参与 cargo 构建。"""
import ctypes
import os
import struct
import subprocess
import sys
import time
from ctypes import wintypes

from PIL import Image, ImageDraw, ImageFont

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)


class RECT(ctypes.Structure):
    _fields_ = [("left", wintypes.LONG), ("top", wintypes.LONG),
                ("right", wintypes.LONG), ("bottom", wintypes.LONG)]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [("biSize", wintypes.DWORD), ("biWidth", wintypes.LONG),
                ("biHeight", wintypes.LONG), ("biPlanes", wintypes.WORD),
                ("biBitCount", wintypes.WORD), ("biCompression", wintypes.DWORD),
                ("biSizeImage", wintypes.DWORD), ("biXPelsPerMeter", wintypes.LONG),
                ("biYPelsPerMeter", wintypes.LONG), ("biClrUsed", wintypes.DWORD),
                ("biClrImportant", wintypes.DWORD)]


class BITMAPINFO(ctypes.Structure):
    _fields_ = [("bmiHeader", BITMAPINFOHEADER), ("bmiColors", wintypes.DWORD * 3)]


user32.FindWindowW.argtypes = [wintypes.LPCWSTR, wintypes.LPCWSTR]
user32.FindWindowW.restype = wintypes.HWND
user32.GetWindowRect.argtypes = [wintypes.HWND, ctypes.POINTER(RECT)]
user32.GetWindowDC.argtypes = [wintypes.HWND]
user32.GetWindowDC.restype = wintypes.HDC
user32.PrintWindow.argtypes = [wintypes.HWND, wintypes.HDC, wintypes.UINT]
user32.SendMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]
user32.ReleaseDC.argtypes = [wintypes.HWND, wintypes.HDC]
gdi32.CreateCompatibleDC.argtypes = [wintypes.HDC]
gdi32.CreateCompatibleDC.restype = wintypes.HDC
gdi32.CreateCompatibleBitmap.argtypes = [wintypes.HDC, ctypes.c_int, ctypes.c_int]
gdi32.CreateCompatibleBitmap.restype = wintypes.HBITMAP
gdi32.SelectObject.argtypes = [wintypes.HDC, wintypes.HGDIOBJ]
gdi32.SelectObject.restype = wintypes.HGDIOBJ
gdi32.GetDIBits.argtypes = [wintypes.HDC, wintypes.HBITMAP, wintypes.UINT, wintypes.UINT,
                            ctypes.c_void_p, ctypes.POINTER(BITMAPINFO), wintypes.UINT]
gdi32.DeleteObject.argtypes = [wintypes.HGDIOBJ]
gdi32.DeleteDC.argtypes = [wintypes.HDC]

WM_LBUTTONDOWN, WM_LBUTTONUP = 0x0201, 0x0202


def cjk_font(size):
    """PIL 默认位图字体不含中文字形，截图标注要用系统雅黑。"""
    for path in ("C:/Windows/Fonts/msyh.ttc", "C:/Windows/Fonts/msyhbd.ttc",
                 "C:/Windows/Fonts/simhei.ttf"):
        try:
            return ImageFont.truetype(path, size)
        except Exception:
            continue
    return ImageFont.load_default()


def grab(hwnd):
    r = RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(r))
    w, h = r.right - r.left, r.bottom - r.top
    if w <= 0 or h <= 0:
        return None
    hdc = user32.GetWindowDC(hwnd)
    mem = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    old = gdi32.SelectObject(mem, bmp)
    user32.PrintWindow(hwnd, mem, 2)
    bi = BITMAPINFO()
    bi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.bmiHeader.biWidth = w
    bi.bmiHeader.biHeight = -h
    bi.bmiHeader.biPlanes = 1
    bi.bmiHeader.biBitCount = 32
    bi.bmiHeader.biCompression = 0
    buf = ctypes.create_string_buffer(w * h * 4)
    gdi32.GetDIBits(mem, bmp, 0, h, buf, ctypes.byref(bi), 0)
    img = Image.frombuffer("RGB", (w, h), buf, "raw", "BGRX", 0, 1).copy()
    gdi32.SelectObject(mem, old)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(hwnd, hdc)
    return img


def click(hwnd, x, y):
    lp = (y << 16) | x
    user32.SendMessageW(hwnd, WM_LBUTTONDOWN, 0, lp)
    user32.SendMessageW(hwnd, WM_LBUTTONUP, 0, lp)
    time.sleep(0.25)


def main():
    exe = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                       "target", "release", "ProperDim.exe")
    out = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
                       "preview", "frontend.png")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    p = subprocess.Popen([exe, "--demo"])
    time.sleep(2.0)
    hwnd = user32.FindWindowW("ProperDimRustControlPanel", None)
    if not hwnd:
        print("WINDOW_NOT_FOUND")
        p.terminate()
        return 1
    shots = []
    tabs = [("控制页", 45), ("日程页", 130), ("热键页", 215), ("选项页", 300)]
    for name, _ in tabs:
        pass
    # 依次点击 4 个 Tab（控件中心坐标按 350×200 逻辑布局）
    centers = [47, 132, 217, 302]
    for (name, _), cx in zip(tabs, centers):
        click(hwnd, cx, 43)
        img = grab(hwnd)
        if img:
            shots.append((name, img))
            print("captured", name, img.size)
    # ---- 第二阶段：4 个对话框 ----
    dialog_shots = []
    # Controls tab -> Adjust Minimum Brightness
    click(hwnd, 47, 43)
    click(hwnd, 175, 145)
    time.sleep(0.4)
    # Schedule tab -> +
    click(hwnd, 132, 43)
    click(hwnd, 328, 72)
    time.sleep(0.4)
    # Options tab -> Info...
    click(hwnd, 302, 43)
    click(hwnd, 317, 154)
    time.sleep(0.4)
    # Options tab -> Clear Schedule (warning)
    click(hwnd, 110, 180)
    time.sleep(0.4)
    for cls, name in [("ProperDimRustMin", "调整最低亮度"),
                      ("ProperDimRustSchedule", "日程设置"),
                      ("ProperDimRustInfo", "关于"),
                      ("ProperDimRustWarning", "确认框")]:
        dh = user32.FindWindowW(cls, None)
        if dh:
            img = grab(dh)
            if img:
                dialog_shots.append((name, img))
                print("captured", name, img.size)
        else:
            print("missing", cls)

    p.terminate()
    try:
        p.wait(timeout=3)
    except Exception:
        p.kill()
    if not shots:
        print("NO_SHOTS")
        return 1
    if dialog_shots:
        dw = max(i.width for _, i in dialog_shots)
        dh = max(i.height for _, i in dialog_shots)
        pad, label_h = 24, 30
        DW = 2 * dw + 3 * pad
        DH = 2 * (dh + label_h) + 3 * pad + 46
        dcanvas = Image.new("RGB", (DW, DH), (247, 247, 245))
        dd = ImageDraw.Draw(dcanvas)
        dd.text((pad, 14), "ProperDim · Rust / 纯 Win32 对话框", fill=(40, 40, 40), font=cjk_font(14))
        for i, (name, img) in enumerate(dialog_shots[:4]):
            cx = pad + (i % 2) * (dw + pad)
            cy = 46 + pad + (i // 2) * (dh + label_h + pad)
            dd.text((cx, cy - 20), name, fill=(90, 90, 90), font=cjk_font(12))
            dcanvas.paste(img, (cx, cy))
        dout = os.path.join(os.path.dirname(out), "dialogs.png")
        dcanvas.save(dout)
        print("SAVED", dout, dcanvas.size)
    cw, ch = shots[0][1].size
    pad, label_h = 24, 34
    cols = 2
    rows = (len(shots) + cols - 1) // cols
    W = cols * cw + (cols + 1) * pad
    H = rows * (ch + label_h) + (rows + 1) * pad + 46
    canvas = Image.new("RGB", (W, H), (247, 247, 245))
    d = ImageDraw.Draw(canvas)
    d.text((pad, 14), "ProperDim · Rust / 纯 Win32 + GDI 界面", fill=(40, 40, 40), font=cjk_font(14))
    for i, (name, img) in enumerate(shots):
        cx = pad + (i % cols) * (cw + pad)
        cy = 46 + pad + (i // cols) * (ch + label_h + pad)
        d.text((cx, cy - 22), name, fill=(90, 90, 90), font=cjk_font(12))
        canvas.paste(img, (cx, cy))
    canvas.save(out)
    print("SAVED", out, canvas.size)
    return 0


if __name__ == "__main__":
    sys.exit(main())
