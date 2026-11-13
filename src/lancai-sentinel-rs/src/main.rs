//! `lancai` 调试 CLI：手动文本分析、配置体检、规则自检、推送/AI 测试。
//!
//! 本版不再包含微信通知监听与自动报名，所有输入都由 `--text` 或样例文件提供。

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use lancai_sentinel::config::Config;
use lancai_sentinel::diagnostics;
use lancai_sentinel::pipeline::Engine;

#[derive(Parser, Debug)]
#[command(name = "lancai", version, about = "澜彩哨兵 · 手动文本分析 CLI")]
struct Cli {
    /// 配置文件路径
    #[arg(long, global = true, default_value = "config.toml")]
    config: PathBuf,
    /// 只做本地分析，不发网络请求
    #[arg(long, global = true)]
    dry_run: bool,
    /// 输出 JSON
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// 分析一段文本
    Analyze {
        #[arg(long)]
        text: String,
        /// 来源标签，例如 班级群 / 同学转发
        #[arg(long, default_value = "手动输入")]
        source: String,
        /// 不写入本地消息库
        #[arg(long)]
        no_store: bool,
    },
    /// 配置相关
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// 用 3 条内置样例跑规则引擎
    Selftest,
    /// 发送一条测试推送
    PushTest {
        #[arg(long, default_value = "澜彩哨兵测试推送")]
        title: String,
        #[arg(long, default_value = "如果你收到这条消息，说明 PushPlus 配置正确。")]
        content: String,
    },
    /// 测试 AI 总结
    AiTest {
        #[arg(long, default_value = "本周五 19:00 图书馆报告厅讲座，0.5 学分，限 80 人，群内报名。")]
        text: String,
    },
    /// 性能基准：同进程内多次 analyze_text，输出单次平均耗时（ms）
    Bench {
        #[arg(long, default_value = "5000")]
        n: u32,
        #[arg(long, default_value = "本周五 19:00 图书馆报告厅讲座，0.5 学分，限 80 人，群内报名。主讲人王教授。")]
        text: String,
        #[arg(long, default_value = "手动输入")]
        source: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    Show,
    Check,
    Init,
    Paths,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (mut cfg, exists) = Config::load_or_default(&cli.config)?;
    if cli.dry_run {
        cfg.debug.dry_run = true;
    }
    let data_dir = cfg.data_dir(&cli.config);

    match cli.cmd {
        Cmd::Analyze {
            text,
            source,
            no_store,
        } => {
            if !exists {
                eprintln!("提示：未找到 {}，使用默认配置", cli.config.display());
            }
            let mut e = Engine::new(cfg, data_dir)?;
            let outcome = e.analyze_text(&source, &text, !no_store);
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&outcome)?);
            } else {
                println!("{}", outcome.summary.card_text());
                println!("\n评分：{} 分（{}）", outcome.score,
                    if outcome.important { "重要" } else { "普通" });
                if outcome.duplicate {
                    println!("提示：3 秒内相同内容已存在，本次未重复入库。");
                }
                if !outcome.ai_summary.is_empty() {
                    println!("AI：{}", outcome.ai_summary);
                }
                if let Some(ok) = outcome.push_ok {
                    println!("推送：{} {}", if ok { "成功" } else { "失败" }, outcome.push_detail);
                }
            }
        }
        Cmd::Config { cmd } => match cmd {
            ConfigCmd::Show => {
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&cfg.redacted())?);
                } else {
                    println!("{}", toml::to_string_pretty(&cfg.redacted())?);
                }
            }
            ConfigCmd::Check => {
                let checks = diagnostics::config_checks(&cfg);
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&checks_json(&checks))?);
                } else {
                    print!("{}", diagnostics::render_checks(&checks));
                }
            }
            ConfigCmd::Init => {
                if cli.config.exists() {
                    eprintln!("{} 已存在，未覆盖", cli.config.display());
                } else {
                    std::fs::write(&cli.config, DEFAULT_TOML)
                        .with_context(|| format!("写入失败: {}", cli.config.display()))?;
                    println!("已生成 {}", cli.config.display());
                }
            }
            ConfigCmd::Paths => {
                println!("配置文件：{}", cli.config.display());
                println!("数据目录：{}", data_dir.display());
                println!("消息库：{}", data_dir.join(&cfg.storage.msgs_file).display());
                println!("最近摘要：{}", data_dir.join(&cfg.storage.last_summary_file).display());
            }
        },
        Cmd::Selftest => {
            print!("{}", diagnostics::selftest(&cfg));
        }
        Cmd::PushTest { title, content } => {
            let e = Engine::new(cfg, data_dir)?;
            let o = e.push_text(&title, &content)?;
            println!("ok={} attempts={} {}", o.ok, o.attempts, o.detail);
        }
        Cmd::AiTest { text } => {
            let mut e = Engine::new(cfg, data_dir)?;
            match e.ai_summary(&text)? {
                Some(t) => println!("{}", t),
                None => println!("AI 未启用（也可能是 dry_run）"),
            }
        }
        Cmd::Bench { n, text, source } => {
            use std::time::Instant;
            let mut e = Engine::new(cfg, data_dir)?;
            // warmup
            for _ in 0..20 {
                let _ = e.analyze_text(&source, &text, false);
            }
            let t0 = Instant::now();
            for _ in 0..n {
                let _ = e.analyze_text(&source, &text, false);
            }
            let dt = t0.elapsed();
            let per_call_us = dt.as_micros() as f64 / n as f64;
            println!("bench: n={n}  total={:.3}ms  avg={per_call_us:.1} µs/call  ({:.3} ms/call)",
                     dt.as_secs_f64() * 1000.0, per_call_us / 1000.0);
        }
    }
    Ok(())
}

fn checks_json(checks: &[diagnostics::Check]) -> Vec<serde_json::Value> {
    checks
        .iter()
        .map(|c| {
            serde_json::json!({
                "level": match c.level {
                    diagnostics::Level::Error => "error",
                    diagnostics::Level::Warn => "warn",
                    diagnostics::Level::Info => "info",
                    diagnostics::Level::Ok => "ok",
                },
                "text": c.text,
            })
        })
        .collect()
}

const DEFAULT_TOML: &str = include_str!("../config.toml");
