use std::fmt;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const OUTPUT_CAP: usize = 64 * 1024;

#[derive(Debug)]
pub(crate) enum GitError {
    Timeout { command: String, seconds: u64 },
    Message(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout { command, seconds } => {
                write!(f, "git {command} 超时（{seconds} 秒）")
            }
            Self::Message(message) => f.write_str(message),
        }
    }
}

pub(crate) fn run(dir: &Path, args: &[&str]) -> Result<(String, i32), GitError> {
    let command = command_name(args);
    let seconds = match command {
        "clone" => 120,
        "fetch" | "pull" | "push" => 45,
        _ => 5,
    };
    let mut child = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| GitError::Message(format!("无法运行 git：{e}（请先安装 git）")))?;

    let stdout = child.stdout.take().expect("git stdout must be piped");
    let stderr = child.stderr.take().expect("git stderr must be piped");
    let stdout_reader = thread::spawn(move || drain(stdout));
    let stderr_reader = thread::spawn(move || drain(stderr));

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitError::Message(format!("等待 git 失败：{e}")));
            }
        }
        if start.elapsed() >= Duration::from_secs(seconds) {
            let _ = child.kill();
            let _ = child.wait();
            // 不等待读取线程：git 的子孙进程可能仍持有管道，join 会突破超时上限。
            return Err(GitError::Timeout {
                command: command.to_string(),
                seconds,
            });
        }
        thread::sleep(Duration::from_millis(50));
    };

    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    let mut output = String::with_capacity(stdout.len() + stderr.len());
    output.push_str(&stdout);
    output.push_str(&stderr);
    Ok((output, status.code().unwrap_or(-1)))
}

fn command_name<'a>(args: &[&'a str]) -> &'a str {
    args.iter()
        .copied()
        .find(|arg| {
            matches!(
                *arg,
                "add"
                    | "checkout"
                    | "clone"
                    | "commit"
                    | "fetch"
                    | "pull"
                    | "push"
                    | "remote"
                    | "reset"
                    | "rev-list"
                    | "rev-parse"
                    | "status"
            )
        })
        .unwrap_or("operation")
}

fn drain<R: Read>(mut reader: R) -> String {
    let mut output = Vec::with_capacity(4096);
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let remaining = OUTPUT_CAP.saturating_sub(output.len());
                output.extend_from_slice(&buffer[..n.min(remaining)]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => break,
        }
    }
    String::from_utf8_lossy(&output).into_owned()
}
