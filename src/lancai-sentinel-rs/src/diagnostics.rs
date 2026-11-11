//! 配置体检与规则自检，供 CLI 与 Android 诊断页共用。

use crate::config::Config;
use crate::model::Msg;
use crate::rule_engine;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Ok,
}

#[derive(Debug, Clone)]
pub struct Check {
    pub level: Level,
    pub text: String,
}

impl Check {
    fn new(level: Level, text: impl Into<String>) -> Self {
        Self {
            level,
            text: text.into(),
        }
    }
}

/// 3 条典型文本，用于 `lancai selftest` 与 Android 诊断页。
pub const SAMPLE_TEXTS: &[(&str, &str)] = &[
    (
        "学分讲座",
        "【学术讲座】本周五 19:00 图书馆报告厅《人工智能前沿》，可获 0.5 学分，限 80 人，11 月 11 日 18:00 前群内回复报名。",
    ),
    (
        "志愿活动",
        "招募志愿者：周六上午 9 点校门口集合，服务时长 4 小时，名额 20 人，找团支书报名，截止周四。",
    ),
    (
        "普通闲聊",
        "哈哈哈哈今天食堂人好多",
    ),
];

pub fn config_checks(cfg: &Config) -> Vec<Check> {
    let mut out = Vec::new();
    let t = cfg.analysis.threshold;
    if (10..=100).contains(&t) {
        out.push(Check::new(Level::Ok, format!("阈值 {t} 分，范围正常")));
    } else {
        out.push(Check::new(
            Level::Error,
            format!("阈值 {t} 分超出 10-100，将按边界处理"),
        ));
    }
    if t < 40 {
        out.push(Check::new(Level::Warn, "阈值偏低，闲聊可能被判为重要"));
    } else if t > 75 {
        out.push(Check::new(Level::Warn, "阈值偏高，可能漏掉有用通知"));
    }
    if cfg.analysis.keyword_extra.trim().is_empty() {
        out.push(Check::new(Level::Info, "未追加关键词，使用内置词表"));
    } else {
        out.push(Check::new(
            Level::Ok,
            format!("追加关键词：{}", cfg.analysis.keyword_extra.trim()),
        ));
    }
    if cfg.ai.enabled {
        if cfg.ai.api_key.trim().is_empty() {
            out.push(Check::new(
                Level::Info,
                "AI 已开启但未填 API Key（本地 Ollama / 免密代理可忽略）",
            ));
        }
        out.push(Check::new(
            Level::Ok,
            format!(
                "AI 已开启：{} @ {}",
                cfg.ai.model,
                crate::net::ai_endpoint(&cfg.ai.base_url)
            ),
        ));
    } else {
        out.push(Check::new(Level::Info, "AI 未开启（不影响本地规则分析）"));
    }
    if cfg.pushplus.enabled {
        if cfg.pushplus.token.trim().is_empty() {
            out.push(Check::new(Level::Error, "已开启推送但未填 PushPlus Token"));
        } else {
            out.push(Check::new(Level::Ok, "微信推送已开启"));
        }
    } else {
        out.push(Check::new(Level::Info, "微信推送未开启"));
    }
    if cfg.debug.dry_run {
        out.push(Check::new(Level::Warn, "dry_run=true，所有网络请求被跳过"));
    }
    out.push(Check::new(
        Level::Info,
        format!("消息库上限 {} 条", cfg.storage.max_messages),
    ));
    out
}

pub fn render_checks(checks: &[Check]) -> String {
    let mut b = String::new();
    for c in checks {
        let tag = match c.level {
            Level::Error => "[x]",
            Level::Warn => "[!]",
            Level::Info => "[i]",
            Level::Ok => "[ok]",
        };
        b.push_str(tag);
        b.push(' ');
        b.push_str(&c.text);
        b.push('\n');
    }
    b
}

/// 用内置样例跑一遍规则引擎，输出分数与抽取结果。
pub fn selftest(cfg: &Config) -> String {
    let mut b = String::new();
    b.push_str(&format!("规则自检（阈值 {} 分）\n\n", cfg.analysis.threshold));
    for (i, (name, text)) in SAMPLE_TEXTS.iter().enumerate() {
        let m = Msg::new("自检", "", *text);
        let s = rule_engine::analyze(&m, &cfg.rules, &cfg.analysis.keyword_extra);
        b.push_str(&format!(
            "{}. {} → {} 分（{}）\n   {}\n",
            i + 1,
            name,
            s.score,
            if s.score >= cfg.analysis.threshold {
                "重要"
            } else {
                "普通"
            },
            s.title_line()
        ));
        if !s.when.is_empty() {
            b.push_str(&format!("   时间：{}\n", s.when));
        }
        if !s.where_.is_empty() {
            b.push_str(&format!("   地点：{}\n", s.where_));
        }
        if !s.credit.is_empty() {
            b.push_str(&format!("   学分：{}\n", s.credit));
        }
        if !s.deadline.is_empty() {
            b.push_str(&format!("   截止：{}\n", s.deadline));
        }
        b.push('\n');
    }
    b
}
