#!/usr/bin/env python3
"""
澜彩哨兵发布站 — 单文件零依赖。
用法: python3 server.py [目录] [端口]
默认目录: 当前工作目录 (含 .apk / .zip 发布文件)
默认端口: 8080   (可用 PORT 环境变量覆盖)
"""
import os
import re
import sys
import json
import time
import shutil
import hashlib
import mimetypes
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote, urlparse
from pathlib import Path

# ---------- 配置 ----------
SERVE_DIR = Path(sys.argv[1] if len(sys.argv) > 1 else os.getcwd()).resolve()
PORT = int(sys.argv[2] if len(sys.argv) > 2 else os.environ.get("PORT", 8080))
HOST = os.environ.get("HOST", "0.0.0.0")
# 简单 token 保护删除端点；空字符串则禁用保护
DELETE_TOKEN = os.environ.get("DELETE_TOKEN", "")

# ---------- 工具 ----------
def human_size(n):
    for u in ("B", "KB", "MB", "GB"):
        if n < 1024:
            return f"{n:.1f} {u}" if u != "B" else f"{n} B"
        n /= 1024
    return f"{n:.1f} TB"

def human_time(ts):
    return time.strftime("%Y-%m-%d %H:%M", time.localtime(ts))

# 从文件名解析版本号 (v3.7.2 等)
VER_RE = re.compile(r"-v?(\d+\.\d+\.\d+)")
TYPE_MAP = {
    ".apk": "APK 安装包",
    ".zip": "源码 / 归档",
}

def scan_files():
    out = []
    for p in sorted(SERVE_DIR.iterdir()):
        if not p.is_file():
            continue
        ext = p.suffix.lower()
        if ext not in (".apk", ".zip"):
            continue
        m = VER_RE.search(p.stem)
        version = m.group(1) if m else ""
        out.append({
            "name": p.name,
            "size": p.stat().st_size,
            "size_h": human_size(p.stat().st_size),
            "mtime": int(p.stat().st_mtime),
            "mtime_h": human_time(p.stat().st_mtime),
            "version": version,
            "type": TYPE_MAP.get(ext, ext),
            "ext": ext[1:],
        })
    # 按版本倒序 (版本字符串可直接比较: 3.7.2 > 3.7.1)
    out.sort(key=lambda x: (x["version"], x["mtime"]), reverse=True)
    return out

def group_by_version(files):
    groups = {}
    for f in files:
        key = f["version"] or "未标记"
        groups.setdefault(key, []).append(f)
    # 版本字符串排序
    return dict(sorted(groups.items(), key=lambda kv: kv[0], reverse=True))

# multipart/form-data 解析 (手写; Python 3.15 要删 cgi)
def parse_multipart(body, boundary):
    boundary = b"--" + boundary
    parts = body.split(boundary)
    for part in parts:
        if not part or part in (b"--\r\n", b"--"):
            continue
        if part.startswith(b"\r\n"):
            part = part[2:]
        if part.endswith(b"\r\n"):
            part = part[:-2]
        header_end = part.find(b"\r\n\r\n")
        if header_end < 0:
            continue
        headers = part[:header_end].decode("latin-1")
        content = part[header_end + 4:]
        disp_match = re.search(r'Content-Disposition:\s*form-data;\s*([^\r\n]*)',
                               headers, re.IGNORECASE)
        if not disp_match:
            continue
        kv = {}
        for piece in disp_match.group(1).split(";"):
            piece = piece.strip()
            if "=" in piece:
                k, v = piece.split("=", 1)
                kv[k.strip()] = v.strip().strip('"')
        yield kv, content

# ---------- 服务 ----------
class Handler(BaseHTTPRequestHandler):
    server_version = "LancaiRelease/1.0"

    def log_message(self, fmt, *args):
        sys.stderr.write(f"[{time.strftime('%H:%M:%S')}] {fmt % args}\n")

    # ---- 响应工具 ----
    def _send(self, code, body, ctype="text/plain; charset=utf-8", extra=None):
        if isinstance(body, str):
            body = body.encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        for k, v in (extra or {}).items():
            self.send_header(k, v)
        self.end_headers()
        self.wfile.write(body)

    def _json(self, code, obj):
        self._send(code, json.dumps(obj, ensure_ascii=False), "application/json; charset=utf-8")

    # ---- 路由 ----
    def do_GET(self):
        p = urlparse(self.path)
        path = unquote(p.path)
        if path == "/" or path == "/index.html":
            self._send(200, HTML_PAGE, "text/html; charset=utf-8")
            return
        if path == "/api/files":
            files = scan_files()
            total_size = sum(f["size"] for f in files)
            self._json(200, {
                "files": files,
                "groups": group_by_version(files),
                "count": len(files),
                "total_size": total_size,
                "total_size_h": human_size(total_size),
                "serve_dir": str(SERVE_DIR),
            })
            return
        if path.startswith("/f/"):
            self._serve_file(path[3:])
            return
        self._send(404, "Not Found")

    def do_POST(self):
        p = urlparse(self.path)
        if p.path != "/upload":
            self._send(404, "Not Found")
            return
        self._handle_upload()

    def do_DELETE(self):
        p = urlparse(self.path)
        if not p.path.startswith("/d/"):
            self._send(404, "Not Found")
            return
        name = unquote(p.path[3:])
        token = self.headers.get("X-Delete-Token", "") or urlparse(self.path).query
        if DELETE_TOKEN and token != DELETE_TOKEN:
            self._json(403, {"ok": False, "error": "token required"})
            return
        self._handle_delete(name)

    # ---- 下载 ----
    def _serve_file(self, name):
        name = Path(name).name  # 防目录穿越
        target = SERVE_DIR / name
        if not target.is_file():
            self._send(404, "Not Found")
            return
        stat = target.stat()
        ctype = mimetypes.guess_type(name)[0] or "application/octet-stream"
        # Range 支持 (断点续传)
        range_hdr = self.headers.get("Range")
        try:
            if range_hdr and range_hdr.startswith("bytes="):
                start_s, end_s = range_hdr[6:].split("-", 1)
                start = int(start_s) if start_s else 0
                end = int(end_s) if end_s else stat.st_size - 1
                end = min(end, stat.st_size - 1)
                length = end - start + 1
                with open(target, "rb") as f:
                    f.seek(start)
                    chunk = f.read(length)
                self.send_response(206)
                self.send_header("Content-Type", ctype)
                self.send_header("Content-Length", str(length))
                self.send_header("Content-Range",
                                 f"bytes {start}-{end}/{stat.st_size}")
                self.send_header("Accept-Ranges", "bytes")
                self.send_header("Content-Disposition",
                                 f'attachment; filename="{name}"')
                self.end_headers()
                self.wfile.write(chunk)
                return
        except Exception:
            pass
        # 全量
        self.send_response(200)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(stat.st_size))
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Disposition",
                         f'attachment; filename="{name}"')
        self.end_headers()
        with open(target, "rb") as f:
            shutil.copyfileobj(f, self.wfile, 1 << 16)

    # ---- 上传 ----
    def _handle_upload(self):
        ctype = self.headers.get("Content-Type", "")
        if not ctype.startswith("multipart/form-data"):
            # 允许单文件原始 body (方便 curl/wget)
            name = self.headers.get("X-Filename", "upload-" + str(int(time.time())))
            length = int(self.headers.get("Content-Length", 0) or 0)
            if length <= 0:
                self._json(400, {"ok": False, "error": "empty body"})
                return
            safe = Path(name).name
            dest = SERVE_DIR / safe
            try:
                with open(dest, "wb") as f:
                    shutil.copyfileobj(self.rfile, f, 1 << 16)
                self._json(200, {"ok": True, "name": safe,
                                  "size": dest.stat().st_size})
            except Exception as e:
                self._json(500, {"ok": False, "error": str(e)})
            return

        length = int(self.headers.get("Content-Length", 0) or 0)
        boundary = ctype.split("boundary=")[-1].split(";")[0].strip().strip('"').encode()
        body = self.rfile.read(length)

        saved = 0
        for kv, content in parse_multipart(body, boundary):
            if "filename" not in kv or not content:
                continue
            safe = Path(kv["filename"]).name
            dest = SERVE_DIR / safe
            try:
                with open(dest, "wb") as f:
                    f.write(content)
                saved += 1
            except Exception as e:
                self._json(500, {"ok": False, "error": str(e)})
                return
        if saved == 0:
            self._json(400, {"ok": False, "error": "no file found"})
            return
        self._json(200, {"ok": True, "saved": saved})

    # ---- 删除 ----
    def _handle_delete(self, name):
        name = Path(name).name
        target = SERVE_DIR / name
        if not target.is_file():
            self._json(404, {"ok": False, "error": "not found"})
            return
        try:
            target.unlink()
            self._json(200, {"ok": True, "name": name})
        except Exception as e:
            self._json(500, {"ok": False, "error": str(e)})


# ---------- HTML ----------
HTML_PAGE = """<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>澜彩哨兵 · 发布站</title>
<style>
  :root{--ink:#1B1B20;--violet:#5B5BD6;--violet-soft:#EAEAFB;--sub:#9A9AA6;--hairline:#EFEFF4;--bg:#FFFFFF;--warn:#B4573F}
  *{box-sizing:border-box}
  body{margin:0;background:var(--bg);color:var(--ink);font:14px/1.6 -apple-system,"Segoe UI","PingFang SC","Microsoft YaHei",sans-serif}
  .wrap{max-width:1100px;margin:0 auto;padding:0 20px}
  header{border-bottom:1px solid var(--hairline);padding:18px 0}
  header .wrap{display:flex;align-items:center;gap:14px;justify-content:space-between}
  header h1{font:600 20px/1.4 Georgia,"Noto Serif SC","Source Han Serif",serif;margin:0;color:var(--ink);letter-spacing:.5px}
  header h1 .dot{width:10px;height:10px;border-radius:50%;background:var(--violet);display:inline-block;margin-right:8px;vertical-align:middle}
  header .stat{color:var(--sub);font-size:12.5px}
  main{padding:24px 0 48px}
  .topbar{display:flex;gap:12px;flex-wrap:wrap;margin-bottom:22px}
  .btn{display:inline-flex;align-items:center;gap:6px;padding:9px 18px;border-radius:20px;border:1px solid var(--hairline);background:var(--bg);color:var(--ink);font-size:13px;cursor:pointer;transition:.15s}
  .btn:hover{border-color:var(--violet);color:var(--violet)}
  .btn.primary{background:var(--violet);color:#fff;border-color:var(--violet)}
  .btn.primary:hover{opacity:.88;color:#fff}
  .btn.ghost{background:transparent}
  .btn.danger{border-color:#F2D6D2;color:var(--warn)}
  .btn.danger:hover{background:#FDF2EF;border-color:var(--warn)}
  .drop{border:2px dashed var(--hairline);border-radius:16px;padding:34px 20px;text-align:center;color:var(--sub);transition:.2s}
  .drop.hover{border-color:var(--violet);background:var(--violet-soft);color:var(--violet)}
  .drop input[type=file]{display:none}
  .drop .hint{font-size:12.5px;margin-top:6px}
  .progress{margin-top:12px;height:6px;background:var(--hairline);border-radius:3px;overflow:hidden;display:none}
  .progress .bar{height:100%;background:var(--violet);width:0%;transition:.2s}
  .progress.show{display:block}
  .section{margin-top:26px}
  .group-title{font:600 16px Georgia,"Noto Serif SC",serif;margin:26px 0 10px;color:var(--ink);padding-bottom:6px;border-bottom:1px solid var(--hairline)}
  .group-title .ver-tag{display:inline-block;background:var(--violet-soft);color:var(--violet);padding:2px 10px;border-radius:10px;font-size:12px;margin-left:8px;font-weight:500}
  table{width:100%;border-collapse:collapse;font-size:13px}
  th,td{padding:10px 12px;text-align:left;border-bottom:1px solid var(--hairline)}
  th{color:var(--sub);font-weight:500;font-size:12.5px}
  tr:hover td{background:#FAFAFC}
  td a{color:var(--violet);text-decoration:none}
  td a:hover{text-decoration:underline}
  .badge{display:inline-block;padding:2px 8px;border-radius:8px;font-size:11.5px;background:var(--violet-soft);color:var(--violet)}
  .badge.apk{background:#FDEAE6;color:var(--warn)}
  .empty{padding:40px;text-align:center;color:var(--sub)}
  .toast{position:fixed;top:20px;left:50%;transform:translateX(-50%);padding:10px 22px;background:var(--ink);color:#fff;border-radius:22px;font-size:13px;z-index:99;opacity:0;transition:.25s}
  .toast.show{opacity:1}
  footer{color:var(--sub);font-size:12px;text-align:center;padding:20px;border-top:1px solid var(--hairline)}
  .muted{color:var(--sub)}
  .row-actions{display:flex;gap:8px}
  .row-actions .mini{padding:3px 10px;font-size:12px;border-radius:12px;border:1px solid var(--hairline);background:#fff;cursor:pointer;color:var(--ink)}
  .row-actions .mini:hover{border-color:var(--violet);color:var(--violet)}
  .row-actions .mini.del:hover{border-color:var(--warn);color:var(--warn)}
</style>
</head>
<body>
<header><div class="wrap">
  <h1><span class="dot"></span>澜彩哨兵 · 发布站</h1>
  <div class="stat" id="stat">加载中…</div>
</div></header>
<main class="wrap">
  <div class="topbar">
    <button class="btn primary" onclick="document.getElementById('file').click()">＋ 上传新版本</button>
    <button class="btn ghost" onclick="load()">↻ 刷新</button>
    <button class="btn" onclick="document.getElementById('bulk').click()">📦 批量打包下载</button>
  </div>

  <!-- 拖拽上传区 -->
  <div class="drop" id="drop" onclick="document.getElementById('file').click()">
    <div style="font-size:32px">⬆</div>
    <div><b>拖拽 .apk / .zip 到这里</b> 或点击选择文件（支持多文件）</div>
    <div class="hint">文件将保存到服务器 serve 目录；同名覆盖</div>
    <input id="file" type="file" multiple accept=".apk,.zip">
    <div class="progress" id="progress"><div class="bar"></div></div>
    <div class="muted" id="progressText" style="font-size:12px;margin-top:4px"></div>
  </div>
  <input id="bulk" type="file" multiple accept=".apk,.zip" style="display:none">

  <div id="content"></div>
</main>
<footer>Python 3 标准库 · 零依赖 · 单文件后端</footer>
<div class="toast" id="toast"></div>

<script>
let DATA = null;

function toast(msg){
  const t=document.getElementById('toast');
  t.textContent=msg; t.classList.add('show');
  clearTimeout(toast._t);
  toast._t=setTimeout(()=>t.classList.remove('show'),1800);
}

async function load(){
  try{
    const r=await fetch('/api/files');
    DATA=await r.json();
    document.getElementById('stat').textContent=
      `${DATA.count} 个文件 · 共 ${DATA.total_size_h} · serve: ${DATA.serve_dir}`;
    render();
  }catch(e){
    toast('加载失败: '+e);
  }
}

function render(){
  if(!DATA){document.getElementById('content').innerHTML='<div class="empty">加载中…</div>';return;}
  const root=document.getElementById('content');
  if(DATA.count===0){
    root.innerHTML='<div class="empty">还没有发布文件。点上方按钮上传 .apk 或 .zip。</div>';
    return;
  }
  let html='';
  const groups=DATA.groups;
  const keys=Object.keys(groups).sort((a,b)=>b.localeCompare(a));
  for(const v of keys){
    const files=groups[v];
    html+=`<div class="section">
      <div class="group-title">${v==='未标记'?'未标记版本':('v'+v)} <span class="ver-tag">${files.length} 个文件</span></div>
      <table><thead><tr>
        <th style="width:44%">文件名</th><th>类型</th><th style="width:100px">大小</th>
        <th style="width:160px">修改时间</th><th style="width:160px">操作</th>
      </tr></thead><tbody>`;
    for(const f of files){
      const badgeClass=f.ext==='apk'?'apk':'';
      html+=`<tr>
        <td><a href="/f/${encodeURIComponent(f.name)}">${f.name}</a></td>
        <td><span class="badge ${badgeClass}">${f.ext.toUpperCase()}</span></td>
        <td>${f.size_h}</td>
        <td class="muted">${f.mtime_h}</td>
        <td class="row-actions">
          <a class="mini" href="/f/${encodeURIComponent(f.name)}">下载</a>
          <button class="mini del" onclick="del('${f.name.replace(/'/g,"\\\\'")}')">删除</button>
        </td>
      </tr>`;
    }
    html+='</tbody></table></div>';
  }
  root.innerHTML=html;
}

async function del(name){
  if(!confirm('确认删除 '+name+'?'))return;
  try{
    const r=await fetch('/d/'+encodeURIComponent(name),{method:'DELETE'});
    const j=await r.json();
    if(j.ok){toast('已删除');load();}else toast('删除失败: '+j.error);
  }catch(e){toast('删除请求失败');}
}

// 上传
function uploadFile(file){
  return new Promise((resolve,reject)=>{
    const x=new XMLHttpRequest();
    const fd=new FormData(); fd.append('file',file);
    x.open('POST','/upload');
    x.upload.onprogress=e=>{
      if(e.lengthComputable){
        const pct=Math.round(e.loaded/e.total*100);
        const bar=document.getElementById('progress');
        bar.classList.add('show');
        bar.querySelector('.bar').style.width=pct+'%';
        document.getElementById('progressText').textContent=
          `${file.name}  ${(e.loaded/1024/1024).toFixed(1)} / ${(e.total/1024/1024).toFixed(1)} MB  ${pct}%`;
      }
    };
    x.onload=()=>{
      const bar=document.getElementById('progress');
      bar.classList.remove('show');
      document.getElementById('progressText').textContent='';
      if(x.status>=200&&x.status<300) resolve(JSON.parse(x.responseText));
      else reject(new Error('HTTP '+x.status));
    };
    x.onerror=()=>reject(new Error('network'));
    x.send(fd);
  });
}

async function handleFiles(list){
  if(!list||!list.length)return;
  const files=[...list].filter(f=>f.name.match(/\\.(apk|zip)$/i));
  if(!files.length){toast('只接受 .apk / .zip');return;}
  toast(`开始上传 ${files.length} 个文件…`);
  for(const f of files){
    try{await uploadFile(f);}catch(e){toast(`${f.name} 上传失败: ${e.message}`);return;}
  }
  toast('上传完成 ✓');
  load();
}

document.getElementById('file').addEventListener('change',e=>handleFiles(e.target.files));
document.getElementById('bulk').addEventListener('change',e=>handleFiles(e.target.files));

const drop=document.getElementById('drop');
['dragenter','dragover'].forEach(ev=>{drop.addEventListener(ev,e=>{e.preventDefault();drop.classList.add('hover')});});
['dragleave','drop'].forEach(ev=>{drop.addEventListener(ev,e=>{e.preventDefault();drop.classList.remove('hover')});});
drop.addEventListener('drop',e=>handleFiles(e.dataTransfer.files));

// 键盘快捷键: u 键打开文件选择
document.addEventListener('keydown',e=>{
  if(e.key==='u'&&!e.ctrlKey&&!e.metaKey&&document.activeElement.tagName!=='INPUT'&&document.activeElement.tagName!=='TEXTAREA')
    document.getElementById('file').click();
});

load();
</script>
</body>
</html>
"""

# ---------- 启动 ----------
def main():
    print(f"\033[32m▶ 澜彩哨兵发布站\033[0m")
    print(f"  目录: {SERVE_DIR}")
    print(f"  端口: http://{HOST}:{PORT}")
    print(f"  文件数: {len(list(SERVE_DIR.iterdir()))}")
    if DELETE_TOKEN:
        print(f"  删除接口: 已启用 token 保护")
    print()
    httpd = ThreadingHTTPServer((HOST, PORT), Handler)
    try:
        httpd.serve_forever()
    except KeyboardInterrupt:
        print("\n收到中断，退出。")
        httpd.server_close()

if __name__ == "__main__":
    main()
