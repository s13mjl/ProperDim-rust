"""审计工具：比对上游 C#/XAML 与 Rust 版的字符串字面量、数值常量与配置字段。
只读，不参与构建。用法: python tools/audit_literals.py
"""
import glob
import os
import re

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UP = os.path.join(os.path.dirname(ROOT), "ProperDim", "ProperDim")
RS = os.path.join(ROOT, "src")


def read(p):
    with open(p, encoding="utf-8-sig", errors="ignore") as f:
        return f.read()


def cs_strings():
    out = set()
    for f in glob.glob(os.path.join(UP, "*.cs")) + glob.glob(os.path.join(UP, "*.xaml")):
        for m in re.findall(r'"([^"\\]{3,120})"', read(f)):
            out.add(m.strip())
    return out


def rust_text():
    return "\n".join(read(f) for f in glob.glob(os.path.join(RS, "**", "*.rs"), recursive=True))


NOISE = (
    "http", "pack://", "Software", "{", "}", "System.", "Microsoft.",
    "ProperDim_", "Crash_", "yyyy", "HH", "StaticResource", "Binding",
    "TemplateBinding", "clr-namespace", "urn:", "#", "Generic", "DISPLAY",
)


def keep(x):
    return len(x) >= 4 and not any(n in x for n in NOISE) and not x.startswith("/")


def main():
    rs = rust_text()
    missing = sorted(x for x in cs_strings() if keep(x) and x not in rs)
    print("=== C# 有 / Rust 完全没有的文案或标识 (%d) ===" % len(missing))
    for x in missing[:160]:
        print("  ", x)

    print()
    print("=== 上游数值常量（时间/阈值/默认值） ===")
    nums = {}
    for f in glob.glob(os.path.join(UP, "*.cs")):
        s = read(f)
        for name, val in re.findall(r'(?:const|static readonly|TimeSpan\.From\w+|Interval =)\s*[^\n]{0,80}', s):
            pass
        for m in re.finditer(r'(TimeSpan\.From(?:Seconds|Milliseconds|Minutes)\((\d+(?:\.\d+)?)\)|=\s*(\d+(?:\.\d+)?)\s*;\s*//)', s):
            v = m.group(2) or m.group(3)
            if v:
                nums.setdefault(v, set()).add(os.path.basename(f))
    for v in sorted(nums, key=lambda x: float(x) if x.replace('.', '', 1).isdigit() else 0):
        print("  %-10s %s" % (v, ", ".join(sorted(nums[v]))))


if __name__ == "__main__":
    main()
