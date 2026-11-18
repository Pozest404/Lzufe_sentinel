use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const EDITOR_HTML: &str = include_str!("../assets/editor.html");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const MAX_BODY: usize = 32 * 1024 * 1024;
const CT_HTML: &str = "text/html; charset=utf-8";
const CT_JSON: &str = "application/json; charset=utf-8";
const CT_TEXT: &str = "text/plain; charset=utf-8";

#[derive(Clone)]
struct Config {
    dir: PathBuf,
    port: u16,
    site_url: String,
    repo: String,
    branch: String,
    no_browser: bool,
    setup: Arc<Mutex<SetupState>>,
}

#[derive(Default)]
struct SetupState {
    ready: bool,
    running: bool,
    error: Option<String>,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            eprintln!("\x1b[31m错误：{e}\x1b[0m");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("kiopen-homepage {VERSION}");
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    let mut cmd = "serve".to_string();
    let mut dir = default_dir();
    let mut port = env::var("KIOPEN_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(8787u16);
    let mut no_browser = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "serve" | "setup" | "publish" => cmd = args[i].clone(),
            "--dir" | "-d" => {
                i += 1;
                dir = PathBuf::from(args.get(i).ok_or("--dir 需要一个路径")?);
            }
            "--port" | "-p" => {
                i += 1;
                port = args
                    .get(i)
                    .ok_or("--port 需要一个端口")?
                    .parse()
                    .map_err(|_| "--port 不是有效端口")?;
            }
            "--no-browser" => no_browser = true,
            other => return Err(format!("未知参数：{other}（--help 查看用法）")),
        }
        i += 1;
    }

    let mut cfg = Config {
        dir,
        port,
        site_url: env::var("KIOPEN_SITE_URL").unwrap_or_else(|_| "https://kiopen.icu/".into()),
        repo: env::var("KIOPEN_REPO")
            .unwrap_or_else(|_| "https://github.com/Pozest404/Pozest404.github.io.git".into()),
        branch: env::var("KIOPEN_BRANCH").unwrap_or_else(|_| "main".into()),
        no_browser,
        setup: Arc::new(Mutex::new(SetupState::default())),
    };

    match cmd.as_str() {
        "setup" => {
            setup(&cfg)?;
            println!("仓库已就绪：{}", cfg.dir.display());
            Ok(())
        }
        "publish" => {
            setup(&cfg)?;
            let body = publish(&cfg)?;
            println!("{}", String::from_utf8_lossy(&body));
            Ok(())
        }
        _ => {
            if let Ok(p) = fs::canonicalize(&cfg.dir) {
                cfg.dir = p;
            }
            let ready = cfg.dir.join("index.html").exists();
            let (listener, real_port) = bind_listener(cfg.port)?;
            cfg.port = real_port;
            {
                let mut st = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
                st.ready = ready;
                st.running = !ready;
            }
            if !ready {
                let bg = cfg.clone();
                thread::spawn(move || run_setup(&bg));
            }
            serve(listener, &cfg)
        }
    }
}

fn run_setup(cfg: &Config) {
    {
        let mut st = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
        st.running = true;
        st.error = None;
    }
    let result = setup(cfg);
    let ready = cfg.dir.join("index.html").exists();
    let mut st = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
    st.ready = ready;
    st.running = false;
    match result {
        Ok(()) if ready => {
            st.error = None;
        }
        Ok(()) => {
            st.error = Some(format!("{} 里没有 index.html", cfg.dir.display()));
        }
        Err(e) => {
            st.error = Some(e);
        }
    }
}

fn bind_listener(start: u16) -> Result<(TcpListener, u16), String> {
    let mut last = String::new();
    for p in start..start.saturating_add(20) {
        match TcpListener::bind(format!("127.0.0.1:{p}")) {
            Ok(l) => return Ok((l, p)),
            Err(e) => last = format!("127.0.0.1:{p}：{e}"),
        }
    }
    Err(format!("无法监听本地端口（{last}）"))
}

fn default_dir() -> PathBuf {
    if let Ok(d) = env::var("KIOPEN_HOMEPAGE_DIR") {
        return PathBuf::from(d);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("kiopen-homepage")
}

fn print_help() {
    println!(
        "Kiopen 主页编辑器 v{VERSION}

用法:
  kiopen-homepage [serve|setup|publish] [选项]

命令:
  serve      启动本地编辑器（默认）
  setup      克隆/更新仓库
  publish    不开界面，直接提交并推送

选项:
  -d, --dir <路径>   仓库目录（默认 ~/kiopen-homepage）
  -p, --port <端口>  监听端口（默认 8787，仅本机）
      --no-browser   不自动打开浏览器
  -V, --version      显示版本
  -h, --help         显示帮助

环境变量:
  KIOPEN_HOMEPAGE_DIR  仓库目录
  KIOPEN_REPO          仓库地址
  KIOPEN_BRANCH        分支（默认 main）
  KIOPEN_SITE_URL      线上站点地址
  KIOPEN_PORT          监听端口"
    );
}

fn setup(cfg: &Config) -> Result<(), String> {
    if cfg.dir.exists() && !cfg.dir.is_dir() {
        return Err(format!("{} 不是目录", cfg.dir.display()));
    }
    if cfg.dir.join("index.html").exists() {
        ensure_exclude(cfg)?;
        return Ok(());
    }
    if cfg.dir.join(".git").exists() {
        let _ = git(cfg, &["fetch", "--depth", "1", "origin"]);
        let _ = git(cfg, &["checkout", &cfg.branch]);
        let _ = git(cfg, &["pull", "--ff-only"]);
        if cfg.dir.join("index.html").exists() {
            ensure_exclude(cfg)?;
            return Ok(());
        }
        return Err(format!("目录 {} 里没有 index.html", cfg.dir.display()));
    }

    let empty = fs::read_dir(&cfg.dir)
        .map(|mut d| d.next().is_none())
        .unwrap_or(true);
    if cfg.dir.exists() && !empty {
        return Err(format!(
            "目录 {} 已存在且不是空目录，请换一个 --dir",
            cfg.dir.display()
        ));
    }
    if let Some(parent) = cfg.dir.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("创建目录失败：{e}"))?;
    }
    let dir_s = cfg.dir.to_string_lossy().to_string();
    let mut child = Command::new("git")
        .args(["clone", "--depth", "1", &cfg.repo, &dir_s])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法运行 git：{e}（请先安装 git）"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(e) => return Err(format!("等待 git 失败：{e}")),
        }
        if start.elapsed() > Duration::from_secs(120) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(
                "克隆超时：无法访问 GitHub。请检查网络，或设置 KIOPEN_REPO 为可达的镜像仓库".into(),
            );
        }
        thread::sleep(Duration::from_millis(200));
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("读取 git 输出失败：{e}"))?;
    if !out.status.success() {
        return Err(format!(
            "克隆失败：{}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    ensure_exclude(cfg)?;
    println!("已克隆 {} → {}", cfg.repo, cfg.dir.display());
    Ok(())
}

fn ensure_exclude(cfg: &Config) -> Result<(), String> {
    let info = cfg.dir.join(".git").join("info");
    if !info.exists() {
        return Ok(());
    }
    fs::create_dir_all(&info).map_err(|e| format!("创建 .git/info 失败：{e}"))?;
    let ex = info.join("exclude");
    let mut s = fs::read_to_string(&ex).unwrap_or_default();
    if !s.lines().any(|l| l.trim() == ".backups/") {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(".backups/\n");
        fs::write(&ex, s).map_err(|e| format!("写入 .git/info/exclude 失败：{e}"))?;
    }
    Ok(())
}

fn serve(listener: TcpListener, cfg: &Config) -> Result<(), String> {
    let addr = format!("127.0.0.1:{}", cfg.port);
    let url = format!("http://{addr}/");
    println!("\x1b[36mKiopen 主页编辑器 v{VERSION}\x1b[0m");
    println!("目录  {}", cfg.dir.display());
    println!("地址  {url}");
    if !cfg.setup.lock().unwrap_or_else(|e| e.into_inner()).ready {
        println!("首次运行正在准备仓库，请稍候…");
    }
    println!("按 Ctrl+C 退出\n");
    if !cfg.no_browser {
        open_browser(&url);
    }
    let shared = Arc::new(cfg.clone());
    for stream in listener.incoming() {
        if let Ok(s) = stream {
            let c = shared.clone();
            thread::spawn(move || {
                let _ = handle(s, c.as_ref());
            });
        }
    }
    Ok(())
}

fn open_browser(url: &str) {
    let attempts: [(&str, &[&str]); 8] = [
        ("xdg-open", &[]),
        ("gio", &["open"]),
        ("sensible-browser", &[]),
        ("x-www-browser", &[]),
        ("firefox", &[]),
        ("chromium", &[]),
        ("google-chrome", &[]),
        ("microsoft-edge", &[]),
    ];
    for (prog, args) in attempts {
        if launch_browser(prog, args, url) {
            return;
        }
    }
    println!("（未能自动打开浏览器，请手动访问上面的地址）");
    let _ = Command::new("notify-send")
        .args(["Kiopen 主页编辑器", &format!("请在浏览器打开 {url}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

fn launch_browser(prog: &str, args: &[&str], url: &str) -> bool {
    let mut c = Command::new(prog);
    c.args(args).arg(url).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = match c.spawn() {
        Ok(ch) => ch,
        Err(_) => return false,
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return st.success(),
            Ok(None) => {}
            Err(_) => return false,
        }
        if start.elapsed() > Duration::from_millis(1500) {
            // 进程仍在运行，通常说明浏览器已成功接管
            return true;
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn handle(mut s: TcpStream, cfg: &Config) -> std::io::Result<()> {
    let _ = s.set_read_timeout(Some(Duration::from_secs(15)));
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = s.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            break p + 4;
        }
        if buf.len() > 64 * 1024 {
            return respond(&mut s, 431, CT_TEXT, "请求头过大".as_bytes());
        }
    };

    let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
    let mut lines = head.split("\r\n");
    let req = lines.next().unwrap_or("");
    let mut it = req.split_whitespace();
    let method = it.next().unwrap_or("").to_string();
    let raw_path = it.next().unwrap_or("/").to_string();
    let path = raw_path.split('?').next().unwrap_or("/").to_string();

    let mut content_length = 0usize;
    let mut origin: Option<String> = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            if k == "content-length" {
                content_length = v.parse().unwrap_or(0);
            } else if k == "origin" {
                origin = Some(v.to_string());
            }
        }
    }
    if content_length > MAX_BODY {
        return respond(&mut s, 413, CT_TEXT, "内容过大".as_bytes());
    }

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = s.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    if method == "POST" {
        if let Some(o) = origin {
            if !origin_ok(&o, cfg.port) {
                return respond(&mut s, 403, CT_TEXT, "来源不被允许".as_bytes());
            }
        }
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/") => respond(&mut s, 200, CT_HTML, EDITOR_HTML.as_bytes()),
        ("GET", "/api/info") => {
            let b = info(cfg);
            respond(&mut s, 200, CT_JSON, b.as_bytes())
        }
        ("GET", "/api/site") => serve_index(cfg, &mut s),
        ("POST", "/api/setup") => {
            let b = start_setup(cfg);
            respond(&mut s, 200, CT_JSON, &b)
        }
        ("POST", "/api/save") => match save(cfg, &body) {
            Ok(b) => respond(&mut s, 200, CT_JSON, &b),
            Err(e) => err(&mut s, 400, &e),
        },
        ("POST", "/api/avatar") => match avatar(cfg, &body) {
            Ok(b) => respond(&mut s, 200, CT_JSON, &b),
            Err(e) => err(&mut s, 400, &e),
        },
        ("POST", "/api/publish") => match publish(cfg) {
            Ok(b) => respond(&mut s, 200, CT_JSON, &b),
            Err(e) => err(&mut s, 500, &e),
        },
        ("GET", "/api/backups") => match list_backups(cfg) {
            Ok(b) => respond(&mut s, 200, CT_JSON, &b),
            Err(e) => err(&mut s, 500, &e),
        },
        ("GET", p) if p.starts_with("/api/backup/") => {
            serve_backup(cfg, &p["/api/backup/".len()..], &mut s)
        }
        ("POST", p) if p.starts_with("/api/restore/") => match restore(cfg, &p["/api/restore/".len()..])
        {
            Ok(b) => respond(&mut s, 200, CT_JSON, &b),
            Err(e) => err(&mut s, 400, &e),
        },
        ("GET", "/preview") => redirect(&mut s, "/preview/"),
        ("GET", p) if p.starts_with("/preview/") => serve_preview(cfg, p, &mut s),
        _ => respond(&mut s, 404, CT_TEXT, "404 Not Found".as_bytes()),
    }
}

fn origin_ok(origin: &str, port: u16) -> bool {
    let a = format!("http://127.0.0.1:{port}");
    let b = format!("http://localhost:{port}");
    origin == a || origin == b || origin == format!("{a}/") || origin == format!("{b}/")
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn respond(s: &mut TcpStream, status: u16, ctype: &str, body: &[u8]) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        301 => "Moved Permanently",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    };
    let head = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    s.write_all(head.as_bytes())?;
    s.write_all(body)?;
    s.flush()
}

fn redirect(s: &mut TcpStream, to: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    s.write_all(head.as_bytes())?;
    s.flush()
}

fn err(s: &mut TcpStream, code: u16, msg: &str) -> std::io::Result<()> {
    let b = format!(r#"{{"ok":false,"error":"{}"}}"#, js(msg));
    respond(s, code, CT_JSON, b.as_bytes())
}

fn serve_index(cfg: &Config, s: &mut TcpStream) -> std::io::Result<()> {
    match fs::read(cfg.dir.join("index.html")) {
        Ok(b) => respond(s, 200, CT_HTML, &b),
        Err(_) => respond(s, 404, CT_TEXT, "index.html 不存在".as_bytes()),
    }
}

fn serve_preview(cfg: &Config, path: &str, s: &mut TcpStream) -> std::io::Result<()> {
    let rel = percent_decode(&path["/preview/".len()..]);
    if rel.is_empty() {
        return serve_index(cfg, s);
    }
    if rel.contains('/') || rel.contains('\\') || rel.contains("..") || rel.starts_with('.') {
        return respond(s, 404, CT_TEXT, "404 Not Found".as_bytes());
    }
    match fs::read(cfg.dir.join(&rel)) {
        Ok(b) => respond(s, 200, mime_for(&rel), &b),
        Err(_) => respond(s, 404, CT_TEXT, "文件不存在".as_bytes()),
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn mime_for(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.ends_with(".html") || n.ends_with(".htm") {
        CT_HTML
    } else if n.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if n.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if n.ends_with(".json") {
        CT_JSON
    } else if n.ends_with(".png") {
        "image/png"
    } else if n.ends_with(".jpg") || n.ends_with(".jpeg") {
        "image/jpeg"
    } else if n.ends_with(".webp") {
        "image/webp"
    } else if n.ends_with(".gif") {
        "image/gif"
    } else if n.ends_with(".svg") {
        "image/svg+xml"
    } else if n.ends_with(".ico") {
        "image/x-icon"
    } else if n.ends_with(".woff2") {
        "font/woff2"
    } else {
        "application/octet-stream"
    }
}

fn info(cfg: &Config) -> String {
    let remote = git(cfg, &["remote", "get-url", "origin"])
        .map(|(o, code)| {
            if code == 0 {
                o.trim().to_string()
            } else {
                cfg.repo.clone()
            }
        })
        .unwrap_or_else(|_| cfg.repo.clone());
    let (ready, running, error) = {
        let st = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
        (st.ready, st.running, st.error.clone().unwrap_or_default())
    };
    format!(
        r#"{{"dir":"{}","remote":"{}","branch":"{}","site_url":"{}","version":"{}","ready":{},"setup_running":{},"setup_error":"{}"}}"#,
        js(&cfg.dir.to_string_lossy()),
        js(&remote),
        js(&cfg.branch),
        js(&cfg.site_url),
        VERSION,
        ready,
        running,
        js(&error)
    )
}

fn start_setup(cfg: &Config) -> Vec<u8> {
    let mut st = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
    if st.running {
        return r#"{"ok":true,"started":false,"message":"正在初始化"}"#.as_bytes().to_vec();
    }
    st.running = true;
    st.error = None;
    drop(st);
    let bg = cfg.clone();
    thread::spawn(move || run_setup(&bg));
    br#"{"ok":true,"started":true}"#.to_vec()
}

fn save(cfg: &Config, body: &[u8]) -> Result<Vec<u8>, String> {
    if body.is_empty() {
        return Err("内容为空".into());
    }
    let html = std::str::from_utf8(body).map_err(|_| "内容不是 UTF-8")?;
    if !html.to_ascii_lowercase().contains("<html") {
        return Err("内容不像完整的 HTML".into());
    }
    let index = cfg.dir.join("index.html");
    let backup = backup(&index, &cfg.dir, "index")?;
    fs::write(&index, html.as_bytes()).map_err(|e| format!("写入 index.html 失败：{e}"))?;
    let backup_s = if backup.as_os_str().is_empty() {
        String::new()
    } else {
        backup.to_string_lossy().to_string()
    };
    Ok(format!(
        r#"{{"ok":true,"backup":"{}","bytes":{}}}"#,
        js(&backup_s),
        body.len()
    )
    .into_bytes())
}

fn avatar(cfg: &Config, body: &[u8]) -> Result<Vec<u8>, String> {
    if body.len() > 10 * 1024 * 1024 {
        return Err("图片不能超过 10MB".into());
    }
    let ext = if body.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "png"
    } else if body.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if body.len() > 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        "webp"
    } else if body.starts_with(b"GIF8") {
        "gif"
    } else {
        return Err("仅支持 PNG / JPG / WEBP / GIF".into());
    };

    let file = format!("avatar.{ext}");
    let index = cfg.dir.join("index.html");
    if let Ok(current) = fs::read_to_string(&index) {
        let mut next = current.clone();
        for old in ["avatar.png", "avatar.jpg", "avatar.jpeg", "avatar.webp", "avatar.gif"] {
            next = next.replace(old, &file);
        }
        if next != current {
            let _ = backup(&index, &cfg.dir, "index");
            fs::write(&index, next).map_err(|e| format!("更新 index.html 失败：{e}"))?;
        }
    }
    fs::write(cfg.dir.join(&file), body).map_err(|e| format!("写入头像失败：{e}"))?;
    Ok(format!(
        r#"{{"ok":true,"file":"{}","bytes":{}}}"#,
        file,
        body.len()
    )
    .into_bytes())
}

fn valid_backup_name(name: &str) -> bool {
    name.starts_with("index-")
        && name.ends_with(".html")
        && name.len() > 11
        && name[6..name.len() - 5]
            .chars()
            .all(|c| c.is_ascii_digit())
}

fn list_backups(cfg: &Config) -> Result<Vec<u8>, String> {
    let bdir = cfg.dir.join(".backups");
    let mut items: Vec<(String, u64, u128)> = Vec::new();
    if let Ok(rd) = fs::read_dir(&bdir) {
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !valid_backup_name(&name) {
                continue;
            }
            let meta = match e.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };
            let time = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis())
                .unwrap_or(0);
            items.push((name, meta.len(), time));
        }
    }
    items.sort_by(|a, b| b.2.cmp(&a.2));
    let list = items
        .iter()
        .map(|(n, b, t)| format!(r#"{{"name":"{}","bytes":{},"time":{}}}"#, js(n), b, t))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(r#"{{"ok":true,"items":[{list}]}}"#).into_bytes())
}

fn serve_backup(cfg: &Config, raw: &str, s: &mut TcpStream) -> std::io::Result<()> {
    let name = percent_decode(raw);
    if !valid_backup_name(&name) {
        return respond(s, 404, CT_TEXT, "备份不存在".as_bytes());
    }
    match fs::read(cfg.dir.join(".backups").join(&name)) {
        Ok(b) => respond(s, 200, CT_HTML, &b),
        Err(_) => respond(s, 404, CT_TEXT, "备份不存在".as_bytes()),
    }
}

fn restore(cfg: &Config, raw: &str) -> Result<Vec<u8>, String> {
    let name = percent_decode(raw);
    if !valid_backup_name(&name) {
        return Err("备份名称不合法".into());
    }
    let src = cfg.dir.join(".backups").join(&name);
    let data = fs::read(&src).map_err(|_| "备份不存在".to_string())?;
    let html = std::str::from_utf8(&data).map_err(|_| "备份不是 UTF-8")?;
    if !html.to_ascii_lowercase().contains("<html") {
        return Err("备份内容不完整".into());
    }
    let index = cfg.dir.join("index.html");
    let saved = backup(&index, &cfg.dir, "index")?;
    fs::write(&index, data).map_err(|e| format!("写入 index.html 失败：{e}"))?;
    let saved_s = if saved.as_os_str().is_empty() {
        String::new()
    } else {
        saved.to_string_lossy().to_string()
    };
    Ok(format!(
        r#"{{"ok":true,"restored":"{}","backup":"{}"}}"#,
        js(&name),
        js(&saved_s)
    )
    .into_bytes())
}

fn publish(cfg: &Config) -> Result<Vec<u8>, String> {
    let (add_out, add_code) = git(cfg, &["add", "-A"])?;
    if add_code != 0 {
        return Err(format!("git add 失败：{add_out}"));
    }
    let (status, _) = git(cfg, &["status", "--porcelain"])?;
    let changed = !status.trim().is_empty();
    let mut log = String::from("$ git add -A\n");
    if !add_out.trim().is_empty() {
        log.push_str(&add_out);
    }

    if changed {
        let msg = format!("update homepage: {}", now_str());
        let (cout, code) = git(
            cfg,
            &[
                "-c",
                "user.name=Kiopen",
                "-c",
                "user.email=zpozest@163.com",
                "commit",
                "-m",
                &msg,
            ],
        )?;
        log.push_str(&format!("$ git commit -m \"{msg}\"\n{cout}"));
        if code != 0 {
            return Err(format!("提交失败：\n{cout}"));
        }
        let (pout, code) = git(cfg, &["push"])?;
        log.push_str(&format!("$ git push\n{pout}"));
        if code != 0 {
            return Err(format!("推送失败：\n{pout}"));
        }
    } else {
        log.push_str("没有需要提交的改动\n");
    }

    Ok(format!(
        r#"{{"ok":true,"changed":{changed},"output":"{}"}}"#,
        js(&log)
    )
    .into_bytes())
}

fn backup(index: &Path, dir: &Path, prefix: &str) -> Result<PathBuf, String> {
    if !index.exists() {
        return Ok(PathBuf::new());
    }
    let bdir = dir.join(".backups");
    fs::create_dir_all(&bdir).map_err(|e| format!("创建备份目录失败：{e}"))?;
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let name = format!("{prefix}-{ms}.html");
    fs::copy(index, bdir.join(&name)).map_err(|e| format!("备份失败：{e}"))?;
    Ok(PathBuf::from(".backups").join(name))
}

fn git(cfg: &Config, args: &[&str]) -> Result<(String, i32), String> {
    let out = Command::new("git")
        .current_dir(&cfg.dir)
        .args(args)
        .output()
        .map_err(|e| format!("无法运行 git：{e}"))?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Ok((s, out.status.code().unwrap_or(-1)))
}

fn now_str() -> String {
    if let Ok(o) = Command::new("date").arg("+%Y-%m-%d %H:%M").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    format!(
        "{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    )
}

fn js(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
