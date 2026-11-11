//! 配置模型：桌面 CLI 用 `config.toml`，Android 端由 Java 薄壳生成同形状 JSON 传入。
//!
//! 已移除微信通知监听与自动报名相关配置：本版只处理“用户主动粘贴/分享的文本”。

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub app: AppConfig,
    pub rules: RulesConfig,
    pub analysis: AnalysisConfig,
    pub ai: AiConfig,
    pub pushplus: PushPlusConfig,
    pub storage: StorageConfig,
    pub debug: DebugConfig,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("读取配置文件失败: {}", path.display()))?;
        let mut cfg: Config = toml::from_str(&text)
            .with_context(|| format!("解析 TOML 失败: {}", path.display()))?;
        cfg.apply_env();
        cfg.normalize();
        Ok(cfg)
    }

    /// Android 端传入的 JSON 配置；缺省字段全部取默认值。
    pub fn from_json(json: &str) -> Self {
        let mut cfg: Config = serde_json::from_str(json).unwrap_or_default();
        cfg.normalize();
        cfg
    }

    pub fn load_or_default(path: &Path) -> Result<(Self, bool)> {
        if path.exists() {
            Ok((Self::load(path)?, true))
        } else {
            let mut cfg = Config::default();
            cfg.apply_env();
            cfg.normalize();
            Ok((cfg, false))
        }
    }

    /// 环境变量优先级高于 config.toml，便于 CI/调试时临时覆盖密钥。
    pub fn apply_env(&mut self) {
        // LANCAI_AI_KEY 是通用变量；保留 LANCAI_DEEPSEEK_KEY 兼容旧脚本。
        for var in ["LANCAI_AI_KEY", "LANCAI_DEEPSEEK_KEY"] {
            if let Ok(v) = std::env::var(var) {
                if !v.trim().is_empty() {
                    self.ai.api_key = v.trim().to_string();
                    break;
                }
            }
        }
        if let Ok(v) = std::env::var("LANCAI_AI_BASE_URL") {
            if !v.trim().is_empty() {
                self.ai.base_url = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("LANCAI_AI_MODEL") {
            if !v.trim().is_empty() {
                self.ai.model = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("LANCAI_PUSHPLUS_TOKEN") {
            if !v.trim().is_empty() {
                self.pushplus.token = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("LANCAI_LOG_LEVEL") {
            if !v.trim().is_empty() {
                self.app.log_level = v.trim().to_string();
            }
        }
        if let Ok(v) = std::env::var("LANCAI_DRY_RUN") {
            if matches!(v.trim(), "1" | "true" | "TRUE" | "yes") {
                self.debug.dry_run = true;
            }
        }
    }

    pub fn normalize(&mut self) {
        self.analysis.threshold = self.analysis.threshold.clamp(10, 100);
        self.storage.max_messages = self.storage.max_messages.clamp(10, 5000);
        self.ai.api_key = self.ai.api_key.trim().to_string();
        self.ai.base_url = self.ai.base_url.trim().to_string();
        if self.ai.base_url.is_empty() {
            self.ai.base_url = AiConfig::default().base_url;
        }
        self.ai.model = self.ai.model.trim().to_string();
        if self.ai.model.is_empty() {
            self.ai.model = AiConfig::default().model;
        }
        self.pushplus.token = self.pushplus.token.trim().to_string();
    }

    pub fn data_dir(&self, config_path: &Path) -> PathBuf {
        let base = config_path.parent().unwrap_or_else(|| Path::new("."));
        let dir = Path::new(self.app.data_dir.trim());
        if dir.is_absolute() {
            dir.to_path_buf()
        } else {
            base.join(dir)
        }
    }

    /// 用于 `config show`：默认遮蔽密钥。
    pub fn redacted(&self) -> Config {
        let mut c = self.clone();
        if !c.ai.api_key.is_empty() {
            c.ai.api_key = mask(&c.ai.api_key);
        }
        if !c.pushplus.token.is_empty() {
            c.pushplus.token = mask(&c.pushplus.token);
        }
        c
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    /// 数据目录，相对路径以 config.toml 所在目录为基准。
    pub data_dir: String,
    pub log_level: String,
    pub timezone: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: "data".to_string(),
            log_level: "info".to_string(),
            timezone: "Asia/Shanghai".to_string(),
        }
    }
}

/// 分析参数：阈值与用户追加关键词。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnalysisConfig {
    /// 重要消息阈值，10..100。
    pub threshold: i32,
    /// 用户追加关键词，逗号/空白分隔；每个 +8 分，封顶 +16。
    pub keyword_extra: String,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            threshold: 55,
            keyword_extra: String::new(),
        }
    }
}

/// 规则打分权重。原 Java 版是常量，这里外置成配置，方便按学校/群活跃度调参。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RulesConfig {
    pub score_baseline: i32,
    pub keyword_first: i32,
    pub keyword_step: i32,
    pub keyword_cap: i32,
    pub custom_keyword_bonus: i32,
    pub custom_keyword_cap: i32,
    pub neg_credit_penalty: i32,
    pub neg_signup_penalty: i32,
    pub neg_cancel_penalty: i32,
    pub reschedule_penalty: i32,
    pub penalty_cap: i32,
    pub credit_bonus: i32,
    pub quota_bonus: i32,
    pub deadline_bonus: i32,
    pub how_bonus: i32,
    pub when_bonus: i32,
    pub where_bonus: i32,
    pub urgent_bonus: i32,
    pub attention_cap: i32,
    pub keywords_append: Vec<String>,
    pub neg_credit_append: Vec<String>,
    pub neg_signup_append: Vec<String>,
    pub neg_cancel_append: Vec<String>,
    pub reschedule_append: Vec<String>,
    pub urgent_append: Vec<String>,
}

impl Default for RulesConfig {
    fn default() -> Self {
        Self {
            score_baseline: 8,
            keyword_first: 12,
            keyword_step: 6,
            keyword_cap: 30,
            custom_keyword_bonus: 8,
            custom_keyword_cap: 16,
            neg_credit_penalty: 30,
            neg_signup_penalty: 25,
            neg_cancel_penalty: 30,
            reschedule_penalty: 10,
            penalty_cap: 60,
            credit_bonus: 22,
            quota_bonus: 10,
            deadline_bonus: 10,
            how_bonus: 15,
            when_bonus: 8,
            where_bonus: 5,
            urgent_bonus: 18,
            attention_cap: 20,
            keywords_append: Vec::new(),
            neg_credit_append: Vec::new(),
            neg_signup_append: Vec::new(),
            neg_cancel_append: Vec::new(),
            reschedule_append: Vec::new(),
            urgent_append: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AiConfig {
    pub enabled: bool,
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub timeout_seconds: u64,
    pub max_tokens: u32,
    pub temperature: f32,
    pub dedupe_minutes: i64,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: String::new(),
            model: "deepseek-chat".to_string(),
            base_url: "https://api.deepseek.com/chat/completions".to_string(),
            timeout_seconds: 20,
            max_tokens: 400,
            temperature: 0.1,
            dedupe_minutes: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PushPlusConfig {
    pub enabled: bool,
    pub token: String,
    pub url: String,
    pub max_tries: u32,
    pub retry_gap_ms: u64,
}

impl Default for PushPlusConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token: String::new(),
            url: "https://www.pushplus.plus/send".to_string(),
            max_tries: 3,
            retry_gap_ms: 2000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageConfig {
    pub msgs_file: String,
    pub max_messages: usize,
    pub last_summary_file: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            msgs_file: "msgs.jsonl".to_string(),
            max_messages: 300,
            last_summary_file: "last_summary.json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DebugConfig {
    /// 打印每个字段的抽取结果。
    pub dump_summary: bool,
    /// 只做本地分析，不发起任何网络请求。
    pub dry_run: bool,
}

pub fn mask(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let head: String = chars.iter().take(4).collect();
    let tail: String = chars.iter().skip(chars.len() - 4).collect();
    format!("{}****{}", head, tail)
}
