//! 领域模型：Msg / Summary，与 Java 版字段一一对应。

use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::text;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Msg {
    pub id: i64,
    /// 收到时间，Unix 毫秒。
    pub time: i64,
    pub group: String,
    pub sender: String,
    pub text: String,
    pub source: String,
}

impl Msg {
    pub fn new(group: impl Into<String>, sender: impl Into<String>, text: impl Into<String>) -> Self {
        let now = chrono::Local::now().timestamp_millis();
        Self {
            id: now * 1000 + (now % 1000),
            time: now,
            group: group.into(),
            sender: sender.into(),
            text: text.into(),
            source: "manual".to_string(),
        }
    }

    pub fn hhmm(&self) -> String {
        chrono::Local
            .timestamp_millis_opt(self.time)
            .single()
            .map(|d| d.format("%H:%M").to_string())
            .unwrap_or_default()
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "time": self.time,
            "group": self.group,
            "sender": self.sender,
            "text": self.text,
            "source": self.source,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Summary {
    pub msg: Option<Msg>,
    pub score: i32,
    pub category: String,
    pub title: String,
    #[serde(rename = "when")]
    pub when: String,
    #[serde(rename = "where")]
    pub where_: String,
    pub credit: String,
    pub quota: String,
    pub deadline: String,
    pub how: String,
    #[serde(rename = "aiSummary")]
    pub ai_summary: String,
    pub notified: bool,
}

impl Default for Summary {
    fn default() -> Self {
        Self {
            msg: None,
            score: 0,
            category: "通知".to_string(),
            title: String::new(),
            when: String::new(),
            where_: String::new(),
            credit: String::new(),
            quota: String::new(),
            deadline: String::new(),
            how: String::new(),
            ai_summary: String::new(),
            notified: false,
        }
    }
}

impl Summary {
    pub fn title_line(&self) -> String {
        let t = if self.title.trim().is_empty() {
            "群里有新消息"
        } else {
            self.title.as_str()
        };
        format!("【{}】{}", self.category, t)
    }

    pub fn card_text(&self) -> String {
        let mut b = String::new();
        b.push_str(&self.title_line());
        append_field(&mut b, "时间", &self.when);
        append_field(&mut b, "地点", &self.where_);
        append_field(&mut b, "学分", &self.credit);
        append_field(&mut b, "名额", &self.quota);
        append_field(&mut b, "截止", &self.deadline);
        append_field(&mut b, "报名", &self.how);
        if !self.ai_summary.trim().is_empty() {
            b.push('\n');
            b.push_str(&self.ai_summary);
        }
        if let Some(m) = &self.msg {
            if !m.text.trim().is_empty() {
                b.push_str("\n————\n");
                b.push_str(&text::abbrev(&m.text, 180));
            }
        }
        b
    }

    pub fn one_line(&self) -> String {
        let mut b = String::new();
        if !self.credit.trim().is_empty() {
            b.push_str(&self.credit);
            b.push_str(" · ");
        }
        if !self.deadline.trim().is_empty() {
            b.push_str("截止 ");
            b.push_str(&self.deadline);
            b.push_str(" · ");
        }
        if !self.quota.trim().is_empty() {
            b.push_str(&self.quota);
            b.push_str(" · ");
        }
        if !self.when.trim().is_empty() {
            b.push_str(&self.when);
        }
        let mut s = b.trim().to_string();
        if s.ends_with('·') {
            s = s[..s.len() - '·'.len_utf8()].trim().to_string();
        }
        if s.is_empty() {
            if let Some(m) = &self.msg {
                s = text::abbrev(&m.text, 60);
            }
        }
        s
    }

    pub fn to_json(&self) -> Value {
        let mut o = serde_json::Map::new();
        if let Some(m) = &self.msg {
            o.insert("msg".to_string(), m.to_json());
        }
        o.insert("score".to_string(), json!(self.score));
        o.insert("category".to_string(), json!(self.category));
        o.insert("title".to_string(), json!(self.title));
        o.insert("when".to_string(), json!(self.when));
        o.insert("where".to_string(), json!(self.where_));
        o.insert("credit".to_string(), json!(self.credit));
        o.insert("quota".to_string(), json!(self.quota));
        o.insert("deadline".to_string(), json!(self.deadline));
        o.insert("how".to_string(), json!(self.how));
        o.insert("aiSummary".to_string(), json!(self.ai_summary));
        o.insert("notified".to_string(), json!(self.notified));
        Value::Object(o)
    }
}

fn append_field(b: &mut String, key: &str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    b.push('\n');
    b.push_str(key);
    b.push_str("：");
    b.push_str(value);
}
