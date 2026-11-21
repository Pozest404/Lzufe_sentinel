use crate::git::{self, GitError};
use crate::{http, Config};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) struct PublishQueue {
    sender: Sender<()>,
    status: Mutex<PublishStatus>,
    control: Mutex<PublishControl>,
}

#[derive(Default)]
struct PublishControl {
    running: bool,
    pending: bool,
}

struct PublishStatus {
    state: &'static str,
    changed: bool,
    commit: String,
    log: String,
    ts: u64,
}

impl Default for PublishStatus {
    fn default() -> Self {
        Self {
            state: "idle",
            changed: false,
            commit: String::new(),
            log: String::new(),
            ts: 0,
        }
    }
}

impl PublishStatus {
    fn active(state: &'static str) -> Self {
        Self {
            state,
            ts: now_secs(),
            ..Self::default()
        }
    }

    fn finished(outcome: PublishOutcome) -> Self {
        Self {
            state: if outcome.success { "success" } else { "error" },
            changed: outcome.changed,
            commit: outcome.commit,
            log: outcome.log,
            ts: now_secs(),
        }
    }
}

impl PublishQueue {
    pub(crate) fn new() -> (Arc<Self>, Receiver<()>) {
        let (sender, receiver) = mpsc::channel();
        (
            Arc::new(Self {
                sender,
                status: Mutex::new(PublishStatus::default()),
                control: Mutex::new(PublishControl::default()),
            }),
            receiver,
        )
    }

    pub(crate) fn enqueue(&self) -> Result<(), String> {
        let mut control = lock(&self.control);
        let initial = !control.running;
        if !initial && control.pending {
            return Ok(());
        }
        if initial {
            control.running = true;
            *lock(&self.status) = PublishStatus::active("queued");
        } else {
            control.pending = true;
        }
        if let Err(e) = self.sender.send(()) {
            if initial {
                control.running = false;
                let mut status = lock(&self.status);
                *status = PublishStatus {
                    state: "error",
                    log: format!("发布线程不可用：{e}"),
                    ..PublishStatus::default()
                };
                status.ts = now_secs();
            } else {
                control.pending = false;
            }
            return Err(format!("无法加入发布队列：{e}"));
        }
        Ok(())
    }

    pub(crate) fn status_json(&self) -> Vec<u8> {
        let status = lock(&self.status);
        format!(
            r#"{{"state":"{}","changed":{},"commit":"{}","log":"{}","ts":{}}}"#,
            status.state,
            status.changed,
            http::js(&status.commit),
            http::js(&status.log),
            status.ts
        )
        .into_bytes()
    }
}

pub(crate) struct PublishOutcome {
    pub(crate) success: bool,
    pub(crate) changed: bool,
    pub(crate) commit: String,
    pub(crate) log: String,
}

pub(crate) fn publish_worker(cfg: &Config, receiver: Receiver<()>) {
    while receiver.recv().is_ok() {
        *lock(&cfg.publish.status) = PublishStatus::active("running");
        let outcome = publish(cfg);
        let mut control = lock(&cfg.publish.control);
        if control.pending {
            control.pending = false;
            continue;
        }
        control.running = false;
        *lock(&cfg.publish.status) = PublishStatus::finished(outcome);
    }
}

pub(crate) fn setup(cfg: &Config) -> Result<(), String> {
    if cfg.dir.exists() && !cfg.dir.is_dir() {
        return Err(format!("{} 不是目录", cfg.dir.display()));
    }
    if cfg.dir.join("index.html").exists() {
        if cfg.dir.join(".git").exists() {
            let _ = sync_existing(cfg);
        }
        ensure_exclude(cfg)?;
        return Ok(());
    }
    if cfg.dir.join(".git").exists() {
        let _repo = lock(&cfg.repo_lock);
        let _io = lock(&cfg.io);
        let _ = run_git(cfg, &["fetch", "--depth", "1", "origin"]);
        let _ = run_git(cfg, &["checkout", &cfg.branch]);
        let _ = run_git(cfg, &["pull", "--ff-only"]);
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
    match git::run(
        Path::new("."),
        &["clone", "--depth", "1", &cfg.repo, &dir_s],
    ) {
        Ok((_, 0)) => {}
        Ok((out, _)) => return Err(format!("克隆失败：{}", out.trim())),
        Err(GitError::Timeout { .. }) => {
            return Err(
                "克隆超时：无法访问 GitHub。请检查网络，或设置 KIOPEN_REPO 为可达的镜像仓库".into(),
            );
        }
        Err(e) => return Err(e.to_string()),
    }
    ensure_exclude(cfg)?;
    println!("已克隆 {} → {}", cfg.repo, cfg.dir.display());
    Ok(())
}

/// 已有仓库时对齐到远端最新；失败只返回原因，不影响启动。
pub(crate) fn sync_existing(cfg: &Config) -> Result<(), String> {
    let _repo = lock(&cfg.repo_lock);
    ensure_exclude(cfg)?;
    let (status, code) =
        run_git(cfg, &["status", "--porcelain"]).map_err(|e| format!("无法读取 git 状态：{e}"))?;
    if code != 0 {
        return Err("不是有效的 git 仓库".into());
    }
    if !status.trim().is_empty() {
        return Err("本地有未提交的修改，跳过同步".into());
    }
    let branch = match run_git(cfg, &["rev-parse", "--abbrev-ref", "HEAD"]) {
        Ok((s, 0)) if !s.trim().is_empty() && s.trim() != "HEAD" => s.trim().to_string(),
        _ => cfg.branch.clone(),
    };
    let shallow = cfg.dir.join(".git").join("shallow").exists();
    let fetched = if shallow {
        // 浅克隆无法可靠判断领先/落后，先补全历史（首次后即不再重复）
        run_git(cfg, &["fetch", "--unshallow", "origin"])
    } else {
        run_git(cfg, &["fetch", "origin"])
    };
    match fetched {
        Ok((_, 0)) => {}
        Ok((out, _)) => return Err(format!("fetch 失败：{}", out.trim())),
        Err(e) => return Err(format!("fetch 失败：{e}")),
    }
    let target = format!("origin/{branch}");
    let ahead = match run_git(cfg, &["rev-list", "--count", &format!("{target}..HEAD")]) {
        Ok((s, 0)) => s.trim().parse::<u64>().unwrap_or(1),
        _ => return Err("无法比较本地与远端".into()),
    };
    let behind = match run_git(cfg, &["rev-list", "--count", &format!("HEAD..{target}")]) {
        Ok((s, 0)) => s.trim().parse::<u64>().unwrap_or(0),
        _ => return Err("无法比较本地与远端".into()),
    };
    if behind == 0 {
        return Ok(());
    }
    if ahead > 0 {
        return Err("本地有未推送的提交，跳过同步".into());
    }

    // repo → io：网络 fetch 不持有 io，reset 前在 io 下重新确认工作区干净。
    let _io = lock(&cfg.io);
    let (status, code) =
        run_git(cfg, &["status", "--porcelain"]).map_err(|e| format!("无法读取 git 状态：{e}"))?;
    if code != 0 {
        return Err("不是有效的 git 仓库".into());
    }
    if !status.trim().is_empty() {
        return Err("本地有未提交的修改，跳过同步".into());
    }
    match run_git(cfg, &["reset", "--hard", &target]) {
        Ok((_, 0)) => Ok(()),
        Ok((out, _)) => Err(format!("reset 失败：{}", out.trim())),
        Err(e) => Err(format!("reset 失败：{e}")),
    }
}

pub(crate) fn save(cfg: &Config, body: &[u8]) -> Result<Vec<u8>, String> {
    if body.is_empty() {
        return Err("内容为空".into());
    }
    let html = std::str::from_utf8(body).map_err(|_| "内容不是 UTF-8")?;
    if !html.to_ascii_lowercase().contains("<html") {
        return Err("内容不像完整的 HTML".into());
    }
    let _io = lock(&cfg.io);
    let index = cfg.dir.join("index.html");
    let backup = backup(&index, &cfg.dir, "index")?;
    let temp = cfg.dir.join(".index.html.kiopen.tmp");
    if let Err(e) = fs::write(&temp, html.as_bytes()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("写入 index.html 临时文件失败：{e}"));
    }
    if let Err(e) = fs::rename(&temp, &index) {
        let _ = fs::remove_file(&temp);
        return Err(format!("替换 index.html 失败：{e}"));
    }
    let backup_s = if backup.as_os_str().is_empty() {
        String::new()
    } else {
        backup.to_string_lossy().to_string()
    };
    Ok(format!(
        r#"{{"ok":true,"backup":"{}","bytes":{}}}"#,
        http::js(&backup_s),
        body.len()
    )
    .into_bytes())
}

pub(crate) fn avatar(cfg: &Config, body: &[u8]) -> Result<Vec<u8>, String> {
    if body.len() > 10 * 1024 * 1024 {
        return Err("图片不能超过 10MB".into());
    }
    let _io = lock(&cfg.io);
    let ext = if body.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        "png"
    } else if body.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if body.len() > 12 && &body[0..4] == b"RIFF" && &body[8..12] == b"WEBP" {
        "webp"
    } else if body.starts_with(b"GIF8") {
        "gif"
    } else if body.len() > 12
        && &body[4..8] == b"ftyp"
        && (&body[8..12] == b"avif" || &body[8..12] == b"avis")
    {
        "avif"
    } else if body.len() > 12 && &body[4..8] == b"ftyp" && &body[8..12] == b"heic" {
        return Err("HEIC 格式暂不支持，请在相册中导出为 JPG / PNG 后再上传".into());
    } else {
        return Err("仅支持 PNG / JPG / WEBP / GIF / AVIF".into());
    };

    let file = format!("avatar.{ext}");
    fs::write(cfg.dir.join(&file), body).map_err(|e| format!("写入头像失败：{e}"))?;
    let index = cfg.dir.join("index.html");
    if let Ok(current) = fs::read_to_string(&index) {
        let mut next = current.clone();
        for old in [
            "avatar.png",
            "avatar.jpg",
            "avatar.jpeg",
            "avatar.webp",
            "avatar.gif",
            "avatar.avif",
        ] {
            next = next.replace(old, &file);
        }
        if next != current {
            let _ = backup(&index, &cfg.dir, "index");
            fs::write(&index, next).map_err(|e| format!("更新 index.html 失败：{e}"))?;
        }
    }
    Ok(format!(r#"{{"ok":true,"file":"{}","bytes":{}}}"#, file, body.len()).into_bytes())
}

fn valid_backup_name(name: &str) -> bool {
    name.starts_with("index-")
        && name.ends_with(".html")
        && name.len() > 11
        && name[6..name.len() - 5].chars().all(|c| c.is_ascii_digit())
}

pub(crate) fn list_backups(cfg: &Config) -> Result<Vec<u8>, String> {
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
        .map(|(n, b, t)| format!(r#"{{"name":"{}","bytes":{},"time":{}}}"#, http::js(n), b, t))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(r#"{{"ok":true,"items":[{list}]}}"#).into_bytes())
}

pub(crate) fn restore(cfg: &Config, raw: &str) -> Result<Vec<u8>, String> {
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
    let _io = lock(&cfg.io);
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
        http::js(&name),
        http::js(&saved_s)
    )
    .into_bytes())
}

pub(crate) fn publish(cfg: &Config) -> PublishOutcome {
    let _repo = lock(&cfg.repo_lock);
    let (commit, mut log) = {
        let io = lock(&cfg.io);
        let (add_out, add_code) = match run_git(cfg, &["add", "-A"]) {
            Ok(result) => result,
            Err(e) => return publish_failure(format!("git add 失败：{e}"), false, String::new()),
        };
        if add_code != 0 {
            return publish_failure(format!("git add 失败：{add_out}"), false, String::new());
        }
        let (status, status_code) = match run_git(cfg, &["status", "--porcelain"]) {
            Ok(result) => result,
            Err(e) => {
                return publish_failure(format!("git status 失败：{e}"), false, String::new());
            }
        };
        if status_code != 0 {
            return publish_failure(format!("git status 失败：{status}"), false, String::new());
        }
        let changed = !status.trim().is_empty();
        let mut log = String::from("$ git add -A\n");
        if !add_out.trim().is_empty() {
            log.push_str(&add_out);
        }
        if !changed {
            log.push_str("没有需要提交的改动\n");
            return PublishOutcome {
                success: true,
                changed: false,
                commit: String::new(),
                log,
            };
        }

        let msg = format!("update homepage: {}", now_str());
        let (commit_out, commit_code) = match run_git(
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
        ) {
            Ok(result) => result,
            Err(e) => return publish_failure(format!("提交失败：\n{e}"), true, String::new()),
        };
        log.push_str(&format!("$ git commit -m \"{msg}\"\n{commit_out}"));
        if commit_code != 0 {
            return publish_failure(format!("提交失败：\n{commit_out}"), true, String::new());
        }
        let commit = match run_git(cfg, &["rev-parse", "HEAD"]) {
            Ok((hash, 0)) => hash.trim().to_string(),
            _ => String::new(),
        };
        drop(io);
        (commit, log)
    };

    // 推送只保留 repo 锁，保存、头像和恢复仍可取得 io 锁。
    let (push_out, push_code) = match run_git(cfg, &["push"]) {
        Ok(result) => result,
        Err(e) => {
            return PublishOutcome {
                success: false,
                changed: true,
                commit,
                log: format!("推送失败：\n{e}"),
            };
        }
    };
    log.push_str(&format!("$ git push\n{push_out}"));
    if push_code != 0 {
        return PublishOutcome {
            success: false,
            changed: true,
            commit,
            log: format!("推送失败：\n{push_out}"),
        };
    }
    PublishOutcome {
        success: true,
        changed: true,
        commit,
        log,
    }
}

fn publish_failure(log: String, changed: bool, commit: String) -> PublishOutcome {
    PublishOutcome {
        success: false,
        changed,
        commit,
        log,
    }
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

fn run_git(cfg: &Config, args: &[&str]) -> Result<(String, i32), String> {
    git::run(&cfg.dir, args).map_err(|e| e.to_string())
}

fn now_str() -> String {
    if let Ok(o) = Command::new("date").arg("+%Y-%m-%d %H:%M").output() {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !s.is_empty() {
            return s;
        }
    }
    now_secs().to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}
