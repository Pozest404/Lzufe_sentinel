#!/usr/bin/env python3
"""GoFile 批量上传工具：扫描目录里的 .apk / .zip，逐个上传，生成 Markdown 清单。

用法: python3 gofile-upload.py [目录] [输出.md]
目录默认: /workspace
输出默认: /workspace/gofile-links.md
"""
import os, sys, json, time, hashlib, subprocess
from pathlib import Path
from datetime import datetime

GOFILE_UPLOAD = "https://upload.gofile.io/uploadfile"

def upload(path: Path) -> dict:
    """上传单文件，返回 GoFile 响应 data 或 None。"""
    print(f"▶ {path.name}  ({path.stat().st_size/1024/1024:.1f} MB)", end="  ", flush=True)
    try:
        res = subprocess.run(
            ["curl", "-sS", "--max-time", "180",
             "-F", f"file=@{path}", GOFILE_UPLOAD],
            capture_output=True, text=True, timeout=200,
        )
        d = json.loads(res.stdout)
        if d.get("status") != "ok":
            print(f"失败: {d.get('status')} {d.get('errorMessage','')}")
            return None
        data = d["data"]
        print(f"✓  https://gofile.io/d/{data['parentFolderCode']}  (file id: {data['id'][:8]}…)")
        return data
    except Exception as e:
        print(f"失败: {e}")
        return None

def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "/workspace").resolve()
    out_path = Path(sys.argv[2] if len(sys.argv) > 2 else "/workspace/gofile-links.md")

    files = sorted([p for p in root.iterdir()
                     if p.is_file() and p.suffix.lower() in (".apk", ".zip")])
    if not files:
        print("没有找到 .apk / .zip 文件")
        return

    print(f"\033[32m澜彩哨兵 → GoFile 批量上传\033[0m")
    print(f"目录: {root}")
    print(f"文件: {len(files)}")
    print()

    results = []
    for p in files:
        data = upload(p)
        results.append({"local": p, "gofile": data})
        time.sleep(0.5)  # 礼貌节流

    # 生成清单
    groups = {}
    for r in results:
        f = r["local"]
        import re
        m = re.search(r"-v?(\d+\.\d+\.\d+)", f.stem)
        ver = m.group(1) if m else "未标记"
        groups.setdefault(ver, []).append(r)

    lines = [
        "# 澜彩哨兵 · GoFile 下载索引",
        "",
        f"> 生成时间: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}",
        f"> 目录扫描: `{root}`",
        f"> 文件总数: {len(files)}",
        "",
        "## 版本索引",
        "",
        "| 版本 | 文件 | 类型 | 大小 | 下载页 |",
        "|------|------|------|------|--------|",
    ]
    for ver in sorted(groups.keys(), reverse=True):
        for r in groups[ver]:
            f = r["local"]
            data = r["gofile"]
            size_h = f"{f.stat().st_size/1024/1024:.1f} MB"
            if data:
                link = f"[{data['parentFolderCode']}]({data['downloadPage']})"
            else:
                link = "_上传失败_"
            lines.append(f"| v{ver} | `{f.name}` | {f.suffix[1:].upper()} | {size_h} | {link} |")

    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print()
    print(f"\033[32m✓ 清单已生成: {out_path}\033[0m")
    print(f"  http://127.0.0.1:8765/{out_path.name}")

if __name__ == "__main__":
    main()
