use crate::{assets, git, repo, start_setup, Config, VERSION};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

const MAX_BODY: usize = 32 * 1024 * 1024;
const CT_HTML: &str = "text/html; charset=utf-8";
const CT_JSON: &str = "application/json; charset=utf-8";
const CT_TEXT: &str = "text/plain; charset=utf-8";

pub(crate) fn handle(mut stream: TcpStream, cfg: &Config) -> std::io::Result<()> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(15)));
    let mut buf: Vec<u8> = Vec::new();
    let mut tmp = [0u8; 8192];
    let header_end = loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            return Ok(());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(p) = find(&buf, b"\r\n\r\n") {
            break p + 4;
        }
        if buf.len() > 64 * 1024 {
            return respond(&mut stream, 431, CT_TEXT, "请求头过大".as_bytes());
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
        return respond(&mut stream, 413, CT_TEXT, "内容过大".as_bytes());
    }

    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }

    if method == "POST" {
        if let Some(o) = origin {
            if !origin_ok(&o, cfg.port) {
                return respond(&mut stream, 403, CT_TEXT, "来源不被允许".as_bytes());
            }
        }
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/") => respond(&mut stream, 200, CT_HTML, assets::editor_html().as_bytes()),
        ("GET", "/api/info") => {
            let body = info(cfg);
            respond(&mut stream, 200, CT_JSON, body.as_bytes())
        }
        ("GET", "/api/site") => serve_index(cfg, &mut stream),
        ("GET", "/api/publish/status") => {
            let body = cfg.publish.status_json();
            respond(&mut stream, 200, CT_JSON, &body)
        }
        ("POST", "/api/setup") | ("POST", "/api/setup-retry") => {
            let body = start_setup(cfg);
            respond(&mut stream, 200, CT_JSON, &body)
        }
        ("POST", "/api/save") => match repo::save(cfg, &body) {
            Ok(body) => respond(&mut stream, 200, CT_JSON, &body),
            Err(e) => err(&mut stream, 400, &e),
        },
        ("POST", "/api/avatar") => match repo::avatar(cfg, &body) {
            Ok(body) => respond(&mut stream, 200, CT_JSON, &body),
            Err(e) => err(&mut stream, 400, &e),
        },
        ("POST", "/api/publish") => match cfg.publish.enqueue() {
            Ok(()) => respond(&mut stream, 202, CT_JSON, br#"{"ok":true,"queued":true}"#),
            Err(e) => err(&mut stream, 500, &e),
        },
        ("GET", "/api/backups") => match repo::list_backups(cfg) {
            Ok(body) => respond(&mut stream, 200, CT_JSON, &body),
            Err(e) => err(&mut stream, 500, &e),
        },
        ("POST", "/api/backups/restore") => {
            let name = String::from_utf8_lossy(&body);
            match repo::restore(cfg, name.trim()) {
                Ok(body) => respond(&mut stream, 200, CT_JSON, &body),
                Err(e) => err(&mut stream, 400, &e),
            }
        }
        ("GET", p) if p.starts_with("/api/backup/") => {
            serve_backup(cfg, &p["/api/backup/".len()..], &mut stream)
        }
        ("POST", p) if p.starts_with("/api/restore/") || p.starts_with("/api/backups/restore/") => {
            let raw = p
                .strip_prefix("/api/restore/")
                .or_else(|| p.strip_prefix("/api/backups/restore/"))
                .unwrap_or("");
            match repo::restore(cfg, raw) {
                Ok(body) => respond(&mut stream, 200, CT_JSON, &body),
                Err(e) => err(&mut stream, 400, &e),
            }
        }
        ("GET", "/preview") => redirect(&mut stream, "/preview/"),
        ("GET", p) if p.starts_with("/preview/") => serve_preview(cfg, p, &mut stream),
        _ => respond(&mut stream, 404, CT_TEXT, "404 Not Found".as_bytes()),
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

fn respond(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
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
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn redirect(stream: &mut TcpStream, to: &str) -> std::io::Result<()> {
    let head = format!(
        "HTTP/1.1 301 Moved Permanently\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(head.as_bytes())?;
    stream.flush()
}

fn err(stream: &mut TcpStream, code: u16, message: &str) -> std::io::Result<()> {
    let body = format!(r#"{{"ok":false,"error":"{}"}}"#, js(message));
    respond(stream, code, CT_JSON, body.as_bytes())
}

fn serve_index(cfg: &Config, stream: &mut TcpStream) -> std::io::Result<()> {
    match fs::read(cfg.dir.join("index.html")) {
        Ok(body) => respond(stream, 200, CT_HTML, &body),
        Err(_) => respond(stream, 404, CT_TEXT, "index.html 不存在".as_bytes()),
    }
}

fn serve_preview(cfg: &Config, path: &str, stream: &mut TcpStream) -> std::io::Result<()> {
    let rel = percent_decode(&path["/preview/".len()..]);
    if rel.is_empty() {
        return serve_index(cfg, stream);
    }
    if rel.contains('/') || rel.contains('\\') || rel.contains("..") || rel.starts_with('.') {
        return respond(stream, 404, CT_TEXT, "404 Not Found".as_bytes());
    }
    match fs::read(cfg.dir.join(&rel)) {
        Ok(body) => respond(stream, 200, mime_for(&rel), &body),
        Err(_) => respond(stream, 404, CT_TEXT, "文件不存在".as_bytes()),
    }
}

fn serve_backup(cfg: &Config, raw: &str, stream: &mut TcpStream) -> std::io::Result<()> {
    let name = percent_decode(raw);
    if !name.starts_with("index-")
        || !name.ends_with(".html")
        || name.len() <= 11
        || !name[6..name.len() - 5].chars().all(|c| c.is_ascii_digit())
    {
        return respond(stream, 404, CT_TEXT, "备份不存在".as_bytes());
    }
    match fs::read(cfg.dir.join(".backups").join(&name)) {
        Ok(body) => respond(stream, 200, CT_HTML, &body),
        Err(_) => respond(stream, 404, CT_TEXT, "备份不存在".as_bytes()),
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
    } else if n.ends_with(".avif") {
        "image/avif"
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
    let remote = match git::run(&cfg.dir, &["remote", "get-url", "origin"]) {
        Ok((output, 0)) => output.trim().to_string(),
        _ => cfg.repo.clone(),
    };
    let (ready, running, error) = {
        let state = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
        (
            state.ready,
            state.running,
            state.error.clone().unwrap_or_default(),
        )
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

pub(crate) fn js(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
