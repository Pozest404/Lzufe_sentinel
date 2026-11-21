mod assets;
mod git;
mod http;
mod repo;

use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub(crate) const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) dir: PathBuf,
    pub(crate) port: u16,
    pub(crate) site_url: String,
    pub(crate) repo: String,
    pub(crate) branch: String,
    pub(crate) no_browser: bool,
    pub(crate) setup: Arc<Mutex<SetupState>>,
    pub(crate) repo_lock: Arc<Mutex<()>>,
    pub(crate) io: Arc<Mutex<()>>,
    pub(crate) publish: Arc<repo::PublishQueue>,
}

#[derive(Default)]
pub(crate) struct SetupState {
    pub(crate) ready: bool,
    pub(crate) running: bool,
    pub(crate) error: Option<String>,
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

    let (publish, publish_receiver) = repo::PublishQueue::new();
    let mut cfg = Config {
        dir,
        port,
        site_url: env::var("KIOPEN_SITE_URL").unwrap_or_else(|_| "https://kiopen.icu/".into()),
        repo: env::var("KIOPEN_REPO")
            .unwrap_or_else(|_| "https://github.com/Pozest404/Pozest404.github.io.git".into()),
        branch: env::var("KIOPEN_BRANCH").unwrap_or_else(|_| "main".into()),
        no_browser,
        setup: Arc::new(Mutex::new(SetupState::default())),
        repo_lock: Arc::new(Mutex::new(())),
        io: Arc::new(Mutex::new(())),
        publish,
    };

    match cmd.as_str() {
        "setup" => {
            repo::setup(&cfg)?;
            println!("仓库已就绪：{}", cfg.dir.display());
            Ok(())
        }
        "publish" => {
            repo::setup(&cfg)?;
            let outcome = repo::publish(&cfg);
            if outcome.success {
                println!(
                    r#"{{"ok":true,"changed":{},"output":"{}"}}"#,
                    outcome.changed,
                    http::js(&outcome.log)
                );
                Ok(())
            } else {
                Err(outcome.log)
            }
        }
        _ => {
            if let Ok(p) = fs::canonicalize(&cfg.dir) {
                cfg.dir = p;
            }
            let ready = cfg.dir.join("index.html").exists();
            let (listener, real_port) = bind_listener(cfg.port)?;
            cfg.port = real_port;
            {
                let mut state = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
                state.ready = ready;
                state.running = !ready;
            }
            let worker = cfg.clone();
            thread::spawn(move || repo::publish_worker(&worker, publish_receiver));
            if !ready {
                let bg = cfg.clone();
                thread::spawn(move || run_setup(&bg));
            } else {
                let bg = cfg.clone();
                thread::spawn(move || match repo::sync_existing(&bg) {
                    Ok(()) => println!("已与远端同步"),
                    Err(e) => println!("自动同步跳过：{e}"),
                });
            }
            serve(listener, &cfg)
        }
    }
}

fn run_setup(cfg: &Config) {
    {
        let mut state = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
        state.running = true;
        state.error = None;
    }
    let result = repo::setup(cfg);
    let ready = cfg.dir.join("index.html").exists();
    let mut state = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
    state.ready = ready;
    state.running = false;
    match result {
        Ok(()) if ready => {
            state.error = None;
        }
        Ok(()) => {
            state.error = Some(format!("{} 里没有 index.html", cfg.dir.display()));
        }
        Err(e) => {
            state.error = Some(e);
        }
    }
}

pub(crate) fn start_setup(cfg: &Config) -> Vec<u8> {
    let mut state = cfg.setup.lock().unwrap_or_else(|e| e.into_inner());
    if state.running {
        return r#"{"ok":true,"started":false,"message":"正在初始化"}"#
            .as_bytes()
            .to_vec();
    }
    state.running = true;
    state.error = None;
    drop(state);
    let bg = cfg.clone();
    thread::spawn(move || run_setup(&bg));
    br#"{"ok":true,"started":true}"#.to_vec()
}

fn bind_listener(start: u16) -> Result<(TcpListener, u16), String> {
    let mut last = String::new();
    for p in start..start.saturating_add(20) {
        match TcpListener::bind(format!("127.0.0.1:{p}")) {
            Ok(listener) => return Ok((listener, p)),
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
        let url = url.clone();
        thread::spawn(move || open_browser(&url));
    }
    let shared = Arc::new(cfg.clone());
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            let cfg = shared.clone();
            thread::spawn(move || {
                let _ = http::handle(stream, cfg.as_ref());
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
    for (program, args) in attempts {
        if launch_browser(program, args, url) {
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

fn launch_browser(program: &str, args: &[&str], url: &str) -> bool {
    let mut command = Command::new(program);
    command
        .args(args)
        .arg(url)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(_) => return false,
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
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
