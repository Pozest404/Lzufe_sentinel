//! 网络 side-effect：PushPlus 推送 + DeepSeek 总结。
//!
//! 与 Android 版的差异：
//! - 同步阻塞（CLI/服务由调用方决定是否并发），返回 Result 而不是回调；
//! - 严格遵守 `debug.dry_run`，调试时绝不误发；
//! - 仍保留 AI 结果缓存与失败重试，避免同一条消息反复烧 token。

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::config::{AiConfig, PushPlusConfig};
use crate::text;

const SYSTEM_PROMPT: &str = concat!(
    "你是校园信息抽取器。只输出一个 JSON 对象，不要输出 markdown 代码块、不要解释、不要任何多余文字。",
    "字段固定为：",
    "{\"category\":\"学分|志愿|讲座|活动|通知|其他 之一\",",
    "\"title\":\"活动或事项名称\",",
    "\"when\":\"举办时间\",",
    "\"where\":\"地点\",",
    "\"credit\":\"学分或志愿时长\",",
    "\"quota\":\"名额，如 限80人\",",
    "\"deadline\":\"报名截止时间\",",
    "\"how\":\"报名方式，如 填写表单/公众号菜单/群接龙/链接\",",
    "\"urgent\":false,",
    "\"oneLine\":\"不超过40字的提醒，告诉大学生该做什么、什么时候截止\"}",
    "。所有字段都是字符串（urgent 是布尔值）；不知道的字段填空字符串 \"\"。"
);

fn agent(timeout_secs: u64) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(timeout_secs.max(1)))
        .try_proxy_from_env(true)
        .build()
}

// ---------------------------------------------------------------------------
// PushPlus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PushOutcome {
    pub ok: bool,
    pub attempts: u32,
    pub detail: String,
}

pub struct PushPlusClient {
    cfg: PushPlusConfig,
    dry_run: bool,
}

impl PushPlusClient {
    pub fn new(cfg: &PushPlusConfig, dry_run: bool) -> Self {
        Self {
            cfg: cfg.clone(),
            dry_run,
        }
    }

    pub fn enabled(&self) -> bool {
        self.cfg.enabled && !self.cfg.token.trim().is_empty() && !self.dry_run
    }

    pub fn send(&self, title: &str, content: &str) -> Result<PushOutcome> {
        self.send_with_template(title, content, guess_template(content))
    }

    pub fn send_with_template(
        &self,
        title: &str,
        content: &str,
        template: &str,
    ) -> Result<PushOutcome> {
        if self.dry_run {
            return Ok(PushOutcome {
                ok: false,
                attempts: 0,
                detail: "dry_run=true，未发送".to_string(),
            });
        }
        if !self.cfg.enabled {
            return Ok(PushOutcome {
                ok: false,
                attempts: 0,
                detail: "pushplus.enabled=false".to_string(),
            });
        }
        if self.cfg.token.trim().is_empty() {
            return Err(anyhow!("pushplus.token 未配置"));
        }
        let tpl = if template.eq_ignore_ascii_case("html") {
            "html"
        } else {
            "txt"
        };
        let body = json!({
            "token": self.cfg.token,
            "title": title,
            "content": content,
            "template": tpl,
        });
        let a = agent(20);
        let tries = self.cfg.max_tries.max(1);
        let mut last = String::new();
        for attempt in 1..=tries {
            let resp = match a
                .post(&self.cfg.url)
                .set("Content-Type", "application/json; charset=UTF-8")
                .send_json(body.clone())
            {
                Ok(r) => r.into_string().unwrap_or_default(),
                Err(ureq::Error::Status(_, r)) => r.into_string().unwrap_or_default(),
                Err(e) => {
                    last = format!("第 {} 次请求失败: {}", attempt, e);
                    if attempt < tries {
                        std::thread::sleep(Duration::from_millis(self.cfg.retry_gap_ms));
                    }
                    continue;
                }
            };
            match serde_json::from_str::<Value>(&resp) {
                Ok(v) => {
                    let code = v.get("code").and_then(Value::as_i64).unwrap_or(-1);
                    let msg = v.get("msg").and_then(Value::as_str).unwrap_or("");
                    if code == 200 {
                        return Ok(PushOutcome {
                            ok: true,
                            attempts: attempt,
                            detail: format!("code=200 msg={}", msg),
                        });
                    }
                    last = format!("第 {} 次失败 code={} msg={}", attempt, code, text::abbrev(msg, 120));
                }
                Err(_) => {
                    last = format!("第 {} 次返回非 JSON: {}", attempt, text::abbrev(&resp, 120));
                }
            }
            if attempt < tries {
                std::thread::sleep(Duration::from_millis(self.cfg.retry_gap_ms));
            }
        }
        Ok(PushOutcome {
            ok: false,
            attempts: tries,
            detail: last,
        })
    }
}

pub fn guess_template(content: &str) -> &'static str {
    let low = content.to_lowercase();
    if ["<br", "<p>", "<p ", "<div", "<b>", "<h3", "<ul"]
        .iter()
        .any(|k| low.contains(k))
    {
        "html"
    } else {
        "txt"
    }
}

// ---------------------------------------------------------------------------
// AI 总结（OpenAI Chat Completions 兼容接口）
// 兼容 DeepSeek / OpenAI / 通义千问 / 智谱 GLM / Kimi / Ollama 等；
// Base URL 既可只填到 /v1，也可直接贴完整的 /chat/completions。
// ---------------------------------------------------------------------------

struct CacheEntry {
    at: i64,
    text: String,
}

pub struct AiClient {
    cfg: AiConfig,
    dry_run: bool,
    cache: HashMap<String, CacheEntry>,
}

impl AiClient {
    pub fn new(cfg: &AiConfig, dry_run: bool) -> Self {
        Self {
            cfg: cfg.clone(),
            dry_run,
            cache: HashMap::new(),
        }
    }

    /// Key 允许为空：本地 Ollama / 免密代理等接口不需要鉴权。
    pub fn enabled(&self) -> bool {
        self.cfg.enabled && !self.dry_run
    }

    pub fn summarize(&mut self, msg_text: &str) -> Result<Option<String>> {
        let text = msg_text.trim();
        if text.is_empty() || self.dry_run || !self.cfg.enabled {
            return Ok(None);
        }

        let key = format!("{}:{}", hash64(text), text.chars().count());
        let now = chrono::Local::now().timestamp_millis();
        self.cleanup(now);
        if let Some(c) = self.cache.get(&key) {
            return Ok(Some(c.text.clone()));
        }

        let result = self.request(text)?;
        self.cache.insert(
            key,
            CacheEntry {
                at: now,
                text: result.clone(),
            },
        );
        Ok(Some(result))
    }

    fn request(&self, text: &str) -> Result<String> {
        let url = ai_endpoint(&self.cfg.base_url);
        match self.post(&url, text, true) {
            Ok(s) => Ok(s),
            Err(first) if first.is_json_mode_rejection() => {
                // 少数兼容接口不支持 response_format，去掉后重试一次。
                self.post(&url, text, false).map_err(AiError::into_anyhow)
            }
            Err(e) => Err(e.into_anyhow()),
        }
    }

    fn post(&self, url: &str, text: &str, json_mode: bool) -> std::result::Result<String, AiError> {
        let mut body = json!({
            "model": self.cfg.model,
            "temperature": self.cfg.temperature,
            "max_tokens": self.cfg.max_tokens,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": format!("请抽取下面这条班级群消息，只输出 JSON：\n\n{}", text)}
            ]
        });
        if json_mode {
            body["response_format"] = json!({"type": "json_object"});
        }

        let a = agent(self.cfg.timeout_seconds);
        let mut req = a
            .post(url)
            .set("Accept", "application/json")
            .set("Content-Type", "application/json; charset=UTF-8");
        let key = self.cfg.api_key.trim();
        if !key.is_empty() {
            req = req.set("Authorization", &format!("Bearer {}", key));
        }
        let resp = match req.send_json(body) {
            Ok(r) => r.into_string().unwrap_or_default(),
            Err(ureq::Error::Status(status, r)) => {
                return Err(AiError::Http {
                    status,
                    body: r.into_string().unwrap_or_default(),
                });
            }
            Err(e) => return Err(AiError::Network(e.to_string())),
        };
        parse_ai_response(&resp)
    }

    fn cleanup(&mut self, now: i64) {
        let ttl = self.cfg.dedupe_minutes.max(1) * 60 * 1000;
        self.cache.retain(|_, c| now - c.at <= ttl);
        if self.cache.len() > 200 {
            self.cache.clear();
        }
    }
}

/// 统一接口地址：只填域名或 /v1 时补 `/chat/completions`。
pub fn ai_endpoint(base_url: &str) -> String {
    let base = base_url.trim();
    if base.is_empty() {
        return "https://api.deepseek.com/chat/completions".to_string();
    }
    let base = base.trim_end_matches('/');
    if base.contains("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    }
}

#[derive(Debug)]
enum AiError {
    Http { status: u16, body: String },
    Network(String),
    Bad(String),
}

impl AiError {
    /// 400/422 通常说明接口不接受 response_format；其他状态不重试。
    fn is_json_mode_rejection(&self) -> bool {
        matches!(self, AiError::Http { status, .. } if *status == 400 || *status == 422)
    }

    fn into_anyhow(self) -> anyhow::Error {
        match self {
            AiError::Http { status, body } => {
                let hint = match status {
                    401 | 403 => "，请检查 API Key",
                    404 => "，请检查 Base URL",
                    429 => "，请求过于频繁",
                    _ => "",
                };
                let detail = error_message(&body);
                if detail.is_empty() {
                    anyhow!(
                        "HTTP {}{}：{}",
                        status,
                        hint,
                        text::abbrev(body.trim(), 160)
                    )
                } else {
                    anyhow!("HTTP {}{}：{}", status, hint, detail)
                }
            }
            AiError::Network(m) => anyhow!("网络请求失败：{}", m),
            AiError::Bad(m) => anyhow!("{}", m),
        }
    }
}

fn error_message(body: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return String::new();
    };
    v.get("error")
        .and_then(|e| {
            e.get("message")
                .and_then(Value::as_str)
                .or_else(|| e.as_str())
        })
        .or_else(|| v.get("message").and_then(Value::as_str))
        .map(|s| text::abbrev(s.trim(), 160))
        .unwrap_or_default()
}

fn parse_ai_response(resp: &str) -> std::result::Result<String, AiError> {
    if resp.trim().is_empty() {
        return Err(AiError::Bad("接口返回空响应".to_string()));
    }
    let root: Value = serde_json::from_str(resp)
        .map_err(|_| AiError::Bad(format!("接口返回非 JSON：{}", text::abbrev(resp, 160))))?;
    if root.get("error").is_some() {
        let msg = error_message(resp);
        let msg = if msg.is_empty() {
            "未知错误".to_string()
        } else {
            msg
        };
        return Err(AiError::Bad(format!("模型接口报错：{}", msg)));
    }
    let content = root
        .get("choices")
        .and_then(|v| v.get(0))
        .and_then(|v| v.get("message"))
        .and_then(|v| v.get("content"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim();
    if content.is_empty() {
        return Err(AiError::Bad(
            "模型返回内容为空（检查模型名或 Base URL）".to_string(),
        ));
    }
    format_ai_content(content).ok_or_else(|| AiError::Bad("模型返回内容为空".to_string()))
}

/// FNV-1a 64 位，仅用于 AI 缓存键，不参与安全用途。
fn hash64(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

/// 把模型返回的严格 JSON 压成一行中文提醒。失败返回 None。
pub fn format_ai_content(content: &str) -> Option<String> {
    let json_str = match (content.find('{'), content.rfind('}')) {
        (Some(st), Some(ed)) if ed > st => &content[st..=ed],
        _ => {
            let s = content
                .replace('\r', " ")
                .replace('\n', " ")
                .replace("```json", "")
                .replace("```", "");
            let s = s.trim();
            return if s.is_empty() {
                None
            } else {
                Some(text::abbrev(s, 120))
            };
        }
    };
    let Ok(d) = serde_json::from_str::<Value>(json_str) else {
        let s = content.replace('\r', " ").replace('\n', " ");
        return Some(text::abbrev(s.trim(), 120));
    };
    let get = |k: &str| {
        d.get(k)
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string()
    };
    let urgent = d.get("urgent").and_then(Value::as_bool).unwrap_or(false);
    let mut b = String::new();
    if urgent {
        b.push_str("【急】");
    }
    let one = get("oneLine");
    if !one.is_empty() {
        b.push_str(&one);
    } else {
        let title = get("title");
        let when = get("when");
        let where_ = get("where");
        let deadline = get("deadline");
        let credit = get("credit");
        b.push_str(if title.is_empty() {
            "这条群消息可能和活动报名有关"
        } else {
            &title
        });
        if !when.is_empty() {
            b.push_str("，时间 ");
            b.push_str(&when);
        }
        if !where_.is_empty() {
            b.push_str("，地点 ");
            b.push_str(&where_);
        }
        if !credit.is_empty() {
            b.push('，');
            b.push_str(&credit);
        }
        if !deadline.is_empty() {
            b.push_str("，报名截止 ");
            b.push_str(&deadline);
        }
    }
    let out = b.trim().to_string();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_endpoint_appends_chat_path_once() {
        assert_eq!(ai_endpoint(""), "https://api.deepseek.com/chat/completions");
        assert_eq!(
            ai_endpoint(" https://api.deepseek.com "),
            "https://api.deepseek.com/chat/completions"
        );
        assert_eq!(
            ai_endpoint("https://api.openai.com/v1/"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            ai_endpoint("https://api.moonshot.cn/v1/chat/completions"),
            "https://api.moonshot.cn/v1/chat/completions"
        );
    }

    #[test]
    fn ai_response_parses_content_and_reports_errors() {
        let ok = r#"{"choices":[{"message":{"content":"{\"title\":\"讲座\",\"oneLine\":\"周五 19:00 讲座\"}"}}]}"#;
        assert!(parse_ai_response(ok).unwrap().contains("周五"));
        assert!(parse_ai_response(r#"{"error":{"message":"invalid api key"}}"#).is_err());
        assert!(parse_ai_response(r#"{"choices":[]}"#).is_err());
    }
}
