#!/usr/bin/env python3
"""GoFile 增量同步：只上传内容有变化的文件，并把结果并入 gofile-links.md。

用法:
    python3 gofile-sync.py <file> [<file> ...]

环境变量:
    GOFILE_MANIFEST  去重清单，默认 /workspace/.gofile-sync.json
    GOFILE_INDEX     下载索引，默认 /workspace/gofile-links.md
    GOFILE_QUIET=1   只输出结果行

退出码:
    0  全部成功（或无需上传）
    1  有文件上传失败（构建流程会忽略，不影响 APK）
"""
import os
import re
import sys
import json
import time
import hashlib
import subprocess
from pathlib import Path

UPLOAD_URL = "https://upload.gofile.io/uploadfile"
MANIFEST = Path(os.environ.get("GOFILE_MANIFEST", "/workspace/.gofile-sync.json"))
INDEX = Path(os.environ.get("GOFILE_INDEX", "/workspace/gofile-links.md"))
QUIET = os.environ.get("GOFILE_QUIET") == "1"

VER_RE = re.compile(r"-v?(\d+\.\d+\.\d+)")


def log(msg):
    if not QUIET:
        print(msg, flush=True)


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def human_size(n):
    for unit in ("B", "KB", "MB", "GB"):
        if n < 1024 or unit == "GB":
            return f"{n:.1f} {unit}" if unit != "B" else f"{n} B"
        n /= 1024


def load_manifest():
    if MANIFEST.is_file():
        try:
            data = json.loads(MANIFEST.read_text(encoding="utf-8"))
            if isinstance(data, dict) and "by_sha" in data:
                return data
        except Exception:
            pass
    return {"by_sha": {}, "entries": []}


def seed_from_index(manifest):
    """首次运行时，把已有索引里的历史记录接进来，避免重建索引时丢链接。"""
    if manifest["entries"] or not INDEX.is_file():
        return
    for line in INDEX.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if not line.startswith("|") or "`" not in line:
            continue
        cols = [c.strip() for c in line.strip("|").split("|")]
        if len(cols) < 5:
            continue
        name_m = re.search(r"`([^`]+)`", cols[1])
        link_m = re.search(r"\]\((https?://[^)]+)\)", cols[4])
        if not name_m or not link_m:
            continue
        ver = cols[0].lstrip("v")
        manifest["entries"].append({
            "name": name_m.group(1),
            "version": ver,
            "size_h": cols[3],
            "link": link_m.group(1),
            "sha256": "",
            "uploaded_at": 0,
        })


def upload(path):
    """返回 (ok, link, 错误信息)。"""
    try:
        res = subprocess.run(
            ["curl", "-sS", "--max-time", "180",
             "-F", f"file=@{path}", UPLOAD_URL],
            capture_output=True, text=True, timeout=200,
        )
        if res.returncode != 0:
            return False, "", (res.stderr or "curl 失败").strip()
        data = json.loads(res.stdout)
        if data.get("status") != "ok":
            return False, "", str(data.get("errorMessage") or data.get("status"))
        return True, data["data"]["downloadPage"], ""
    except Exception as exc:  # 网络异常、JSON 解析失败都归到这里
        return False, "", str(exc)


def save_manifest(manifest):
    MANIFEST.write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )


def ver_key(v):
    try:
        return tuple(int(x) for x in v.split("."))
    except (ValueError, AttributeError):
        return (0,)


def write_index(manifest):
    entries = [e for e in manifest["entries"] if e.get("link")]
    entries.sort(key=lambda e: (ver_key(e.get("version", "")), e.get("name", "")), reverse=True)

    # 索引只展示最近 KEEP_VERSIONS 个版本；完整历史仍保留在 .gofile-sync.json。
    keep = int(os.environ.get("KEEP_VERSIONS", "2") or 0)
    if keep > 0:
        versions = sorted({e.get("version") or "" for e in entries},
                          key=ver_key, reverse=True)
        allowed = set(versions[:keep])
        entries = [e for e in entries if (e.get("version") or "") in allowed]

    lines = [
        "# 澜彩哨兵 · GoFile 下载索引",
        "",
        f"> 最后更新: {time.strftime('%Y-%m-%d %H:%M:%S')}",
        f"> 条目数: {len(entries)}（仅最近 {keep if keep > 0 else '全部'} 版）",
        "> 由 `release-server/gofile-sync.py` 自动维护，构建后自动更新。",
        "",
        "| 版本 | 文件 | 大小 | 下载页 |",
        "|------|------|------|--------|",
    ]
    for e in entries:
        lines.append(
            f"| v{e.get('version', '?')} | `{e['name']}` | {e.get('size_h', '')} | "
            f"[{e['link'].rsplit('/', 1)[-1]}]({e['link']}) |"
        )
    INDEX.write_text("\n".join(lines) + "\n", encoding="utf-8")


def main():
    files = [Path(a).resolve() for a in sys.argv[1:] if Path(a).is_file()]
    if not files:
        log("gofile-sync: 没有可上传的文件")
        return 0

    manifest = load_manifest()
    seed_from_index(manifest)
    failed = 0

    for path in files:
        digest = sha256(path)
        known = manifest["by_sha"].get(digest)
        if known:
            log(f"  = {path.name} 已上传过，复用链接")
            print(f"GOFILE {path.name} {known['link']}")
            continue

        log(f"  ↑ {path.name} ({human_size(path.stat().st_size)}) …")
        ok, link, err = upload(path)
        if not ok:
            print(f"GOFILE-FAILED {path.name} {err}")
            log(f"  ! 上传失败：{err}")
            failed += 1
            continue

        m = VER_RE.search(path.stem)
        entry = {
            "name": path.name,
            "version": m.group(1) if m else "",
            "size_h": human_size(path.stat().st_size),
            "link": link,
            "sha256": digest,
            "uploaded_at": int(time.time()),
        }
        # 同名文件保留最新一条，避免索引里堆积历史记录
        manifest["entries"] = [e for e in manifest["entries"] if e["name"] != path.name]
        manifest["entries"].append(entry)
        manifest["by_sha"][digest] = entry
        save_manifest(manifest)
        print(f"GOFILE {path.name} {link}")
        log(f"  ✓ {link}")

    write_index(manifest)
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
