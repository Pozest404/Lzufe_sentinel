#!/usr/bin/env python3
"""按版本清理发布产物，只保留最近 N 个版本。

用法:
    python3 retention.py <目录> [<目录> ...] [--keep 2] [--dry-run]

识别规则:
    文件名含 -v<主>.<次>.<修订>，扩展名为 .apk / .zip（含 .apk.idsig）。
    没有版本号的产物一律不动。
"""
import re
import sys
from pathlib import Path

VER_RE = re.compile(r"-v?(\d+\.\d+\.\d+)")
EXTS = (".apk", ".zip", ".idsig")


def version_key(ver):
    return tuple(int(x) for x in ver.split("."))


def collect(directory):
    """返回 {版本: [文件...]}。"""
    groups = {}
    for p in Path(directory).iterdir():
        if not p.is_file() or not p.name.endswith(EXTS):
            continue
        m = VER_RE.search(p.name)
        if not m:
            continue
        groups.setdefault(m.group(1), []).append(p)
    return groups


def main():
    args = [a for a in sys.argv[1:]]
    keep = 2
    dry = "--dry-run" in args
    args = [a for a in args if a != "--dry-run"]
    if "--keep" in args:
        i = args.index("--keep")
        keep = int(args[i + 1])
        del args[i:i + 2]

    if not args or keep <= 0:
        return 0

    total = 0
    for directory in args:
        d = Path(directory)
        if not d.is_dir():
            continue
        groups = collect(d)
        versions = sorted(groups, key=version_key, reverse=True)
        doomed = versions[keep:]
        if not doomed:
            continue
        for ver in doomed:
            for p in groups[ver]:
                total += 1
                if dry:
                    print(f"  would delete {p.name}")
                else:
                    try:
                        p.unlink()
                        print(f"  - {p.name}")
                    except OSError as exc:
                        print(f"  ! 删除失败 {p.name}: {exc}")
        print(f"  {d}: 保留 {len(versions) - len(doomed)} 版，清理 {len(doomed)} 版")

    if total == 0:
        print("  无需清理")
    return 0


if __name__ == "__main__":
    sys.exit(main())
