//! 分析流水线：手动文本 → 规则打分 → 入库 → （命中且已配置时）AI 总结 + 微信推送。
//!
//! 本版已移除微信通知监听、通知判重、低优先级简报与自动报名；
//! 所有动作都由用户主动粘贴/分享文本触发。

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::Serialize;

use crate::config::Config;
use crate::model::{Msg, Summary};
use crate::net::{AiClient, PushOutcome, PushPlusClient};
use crate::rule_engine;
use crate::store::MsgStore;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub analyzed: u64,
    pub important: u64,
    pub duplicates: u64,
    pub push_ok: u64,
    pub push_fail: u64,
    pub ai_ok: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MessageOutcome {
    pub duplicate: bool,
    pub score: i32,
    pub category: String,
    pub title: String,
    /// 达到阈值。
    pub important: bool,
    pub ai_summary: String,
    pub push_ok: Option<bool>,
    pub push_detail: String,
    pub summary: Summary,
}

pub struct Engine {
    pub cfg: Config,
    pub data_dir: PathBuf,
    pub store: MsgStore,
    pub stats: Stats,
    ai: AiClient,
    push: PushPlusClient,
}

impl Engine {
    pub fn new(cfg: Config, data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        let store = MsgStore::new(
            data_dir.join(&cfg.storage.msgs_file),
            data_dir.join(&cfg.storage.last_summary_file),
            cfg.storage.max_messages,
        );
        let ai = AiClient::new(&cfg.ai, cfg.debug.dry_run);
        let push = PushPlusClient::new(&cfg.pushplus, cfg.debug.dry_run);
        Ok(Self {
            cfg,
            data_dir,
            store,
            stats: Stats::default(),
            ai,
            push,
        })
    }

    pub fn from_config_path(cfg: Config, config_path: &Path) -> Result<Self> {
        let dir = cfg.data_dir(config_path);
        Self::new(cfg, dir)
    }

    /// 分析一段手动输入/分享的文本。`persist=true` 时写入本地消息库。
    pub fn analyze_text(&mut self, source: &str, raw: &str, persist: bool) -> MessageOutcome {
        let text = raw.trim().to_string();
        let source = if source.trim().is_empty() {
            "手动输入".to_string()
        } else {
            source.trim().to_string()
        };
        let now = chrono::Local::now().timestamp_millis();
        let m = Msg {
            id: now * 1000 + now.rem_euclid(1000),
            time: now,
            group: source,
            sender: String::new(),
            text: text.clone(),
            source: "manual".to_string(),
        };

        if persist {
            if !self.store.add(&m) {
                self.stats.duplicates += 1;
                let summary = rule_engine::analyze(&m, &self.cfg.rules, &self.cfg.analysis.keyword_extra);
                return MessageOutcome {
                    duplicate: true,
                    score: summary.score,
                    category: summary.category.clone(),
                    title: summary.title.clone(),
                    important: false,
                    ai_summary: String::new(),
                    push_ok: None,
                    push_detail: String::new(),
                    summary,
                };
            }
            self.stats.analyzed += 1;
        }

        let mut summary = rule_engine::analyze(&m, &self.cfg.rules, &self.cfg.analysis.keyword_extra);
        let important = summary.score >= self.cfg.analysis.threshold;
        summary.notified = important;

        let mut ai_summary = String::new();
        let mut push_ok = None;
        let mut push_detail = String::new();

        if important {
            self.stats.important += 1;
            if persist {
                let _ = self.store.set_last_summary(&summary);
            }
            if self.ai.enabled() {
                if let Ok(Some(t)) = self.ai.summarize(&m.text) {
                    if !t.is_empty() {
                        ai_summary = t.clone();
                        summary.ai_summary = t;
                        self.stats.ai_ok += 1;
                        if persist {
                            let _ = self.store.set_last_summary(&summary);
                        }
                    }
                }
            }
            if self.push.enabled() {
                match self.push.send(&summary.title_line(), &summary.card_text()) {
                    Ok(o) => {
                        push_ok = Some(o.ok);
                        push_detail = o.detail;
                        if o.ok {
                            self.stats.push_ok += 1;
                        } else {
                            self.stats.push_fail += 1;
                        }
                    }
                    Err(e) => {
                        push_ok = Some(false);
                        push_detail = e.to_string();
                        self.stats.push_fail += 1;
                    }
                }
            }
        }

        MessageOutcome {
            duplicate: false,
            score: summary.score,
            category: summary.category.clone(),
            title: summary.title.clone(),
            important,
            ai_summary,
            push_ok,
            push_detail,
            summary,
        }
    }

    /// 手动触发一次 AI 总结（AI 已启用即可；本地/免密接口可不填 Key）。
    pub fn ai_summary(&mut self, text: &str) -> Result<Option<String>> {
        self.ai.summarize(text)
    }

    /// 手动推送一条内容。
    pub fn push_text(&self, title: &str, content: &str) -> Result<PushOutcome> {
        self.push.send(title, content)
    }

    pub fn recent(&self, n: usize) -> Vec<Msg> {
        self.store.recent(n)
    }

    pub fn count(&self) -> usize {
        self.store.count()
    }

    pub fn last_summary(&self) -> Option<Summary> {
        self.store.get_last_summary()
    }

    pub fn clear(&mut self) {
        self.store.clear();
    }
}
