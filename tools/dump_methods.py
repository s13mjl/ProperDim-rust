"""审计工具：按方法名抽取上游 C# 方法体，供逐项核对。只读。"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UP = os.path.join(os.path.dirname(ROOT), "ProperDim", "ProperDim")

PAT = re.compile(r'^\s*(?:private|public|protected|internal)\s+(?:static\s+)?(?:async\s+)?[\w<>?\[\], .]+\s+(\w+)\s*\(')


def methods(path):
    lines = open(path, encoding="utf-8-sig", errors="ignore").read().splitlines()
    starts = [(i, PAT.match(l).group(1)) for i, l in enumerate(lines) if PAT.match(l)]
    out = {}
    for idx, (i, name) in enumerate(starts):
        end = starts[idx + 1][0] if idx + 1 < len(starts) else len(lines)
        out.setdefault(name, []).append("\n".join(lines[i:end]).rstrip())
    return out


def main():
    wanted = sys.argv[1:]
    for f in sorted(glob.glob(os.path.join(UP, "*.xaml.cs"))):
        ms = methods(f)
        for name in wanted:
            if name in ms:
                print("\n######## %s :: %s ########" % (os.path.basename(f), name))
                print(ms[name][0][:1800])


if __name__ == "__main__":
    main()
