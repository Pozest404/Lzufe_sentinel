//! 日程 / 待办 / 第二课堂活动：本地 JSON 存储与提醒时间计算。
//!
//! 数据只保存在应用私有目录；通知由 Android 端按 `upcoming()` 的计算结果排期，
//! 这里不依赖任何系统能力，便于单元测试。

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone};
use serde::{Deserialize, Serialize};

fn default_remind() -> i64 {
    10
}

fn default_kind() -> String {
    "todo".to_string()
}

/// 一条待办或第二课堂活动。`due_ms=0` 表示没有具体时间。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Todo {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub due_ms: i64,
    /// 提前多少分钟提醒；-1 表示不提醒。
    #[serde(default = "default_remind")]
    pub remind_min: i64,
    #[serde(default)]
    pub done: bool,
    /// "todo" 普通待办；"activity" 第二课堂活动（可带链接）。
    #[serde(default = "default_kind")]
    pub kind: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub created_ms: i64,
    /// 用户确认过的自动报名；为 true 时到 signup_at_ms 会提醒并打开报名页。
    #[serde(default)]
    pub auto_signup: bool,
    /// 自动报名触发时间（毫秒）；0 表示不单独排期。
    #[serde(default)]
    pub signup_at_ms: i64,
}

impl Todo {
    /// 提醒触发时间（毫秒）；不需要提醒时返回 None。
    pub fn remind_at_ms(&self) -> Option<i64> {
        if self.done || self.due_ms <= 0 || self.remind_min < 0 {
            return None;
        }
        Some(self.due_ms - self.remind_min * 60_000)
    }

    pub fn is_activity(&self) -> bool {
        self.kind == "activity"
    }
}

fn now_ms() -> i64 {
    Local::now().timestamp_millis()
}

fn sort_key(a: &Todo, b: &Todo) -> std::cmp::Ordering {
    a.done
        .cmp(&b.done)
        .then_with(|| match (a.due_ms > 0, b.due_ms > 0) {
            (true, true) => a.due_ms.cmp(&b.due_ms),
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.created_ms.cmp(&b.created_ms),
        })
}

/// 日程存储：`schedule.json`，整体读写，条目数量很小。
pub struct ScheduleStore {
    path: PathBuf,
}

impl ScheduleStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Vec<Todo> {
        let mut items = match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str::<Vec<Todo>>(&raw).unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        items.sort_by(sort_key);
        items
    }

    fn save(&self, items: &[Todo]) -> Result<()> {
        if let Some(p) = self.path.parent() {
            fs::create_dir_all(p)
                .with_context(|| format!("创建目录失败: {}", p.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(items)?)
            .with_context(|| format!("写入失败: {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("替换文件失败: {}", self.path.display()))?;
        Ok(())
    }

    /// 新增或更新一条；返回落盘后的条目（包含生成的 id）。
    pub fn upsert(&self, mut t: Todo) -> Result<Todo> {
        t.title = t.title.trim().to_string();
        t.note = t.note.trim().to_string();
        t.url = t.url.trim().to_string();
        if t.kind != "activity" {
            t.kind = "todo".to_string();
        }
        if t.remind_min < -1 {
            t.remind_min = -1;
        }
        if t.remind_min > 7 * 24 * 60 {
            t.remind_min = 7 * 24 * 60;
        }
        if t.created_ms <= 0 {
            t.created_ms = now_ms();
        }
        if !t.auto_signup || t.url.trim().is_empty() {
            t.auto_signup = false;
            t.signup_at_ms = 0;
        }
        if t.signup_at_ms < 0 {
            t.signup_at_ms = 0;
        }

        let mut items = self.load();
        if t.id <= 0 {
            let max_id = items.iter().map(|x| x.id).max().unwrap_or(0);
            t.id = (now_ms() * 1000).max(max_id + 1);
        }
        match items.iter_mut().find(|x| x.id == t.id) {
            Some(slot) => *slot = t.clone(),
            None => items.push(t.clone()),
        }
        self.save(&items)?;
        Ok(t)
    }

    pub fn delete(&self, id: i64) -> Result<bool> {
        let mut items = self.load();
        let before = items.len();
        items.retain(|x| x.id != id);
        if items.len() == before {
            return Ok(false);
        }
        self.save(&items)?;
        Ok(true)
    }

    pub fn set_done(&self, id: i64, done: bool) -> Result<bool> {
        let mut items = self.load();
        let Some(t) = items.iter_mut().find(|x| x.id == id) else {
            return Ok(false);
        };
        t.done = done;
        self.save(&items)?;
        Ok(true)
    }

    pub fn clear_done(&self) -> Result<usize> {
        let mut items = self.load();
        let before = items.len();
        items.retain(|x| !x.done);
        let removed = before - items.len();
        if removed > 0 {
            self.save(&items)?;
        }
        Ok(removed)
    }

    /// 未来 [now, now+horizon] 内需要提醒的条目，附带触发时间。
    pub fn upcoming(&self, now: i64, horizon: i64) -> Vec<(Todo, i64)> {
        let end = now.saturating_add(horizon.max(0));
        let mut out: Vec<(Todo, i64)> = self
            .load()
            .into_iter()
            .filter_map(|t| {
                let at = t.remind_at_ms()?;
                if at > now && at <= end {
                    Some((t, at))
                } else {
                    None
                }
            })
            .collect();
        out.sort_by_key(|(_, at)| *at);
        out.truncate(64);
        out
    }

    /// 未来 [now, now+horizon] 内需要触发自动报名的条目，附带触发时间。
    /// 没有链接、未勾选自动报名或已完成的条目会被跳过。
    pub fn signups(&self, now: i64, horizon: i64) -> Vec<(Todo, i64)> {
        let end = now.saturating_add(horizon.max(0));
        let mut out: Vec<(Todo, i64)> = self
            .load()
            .into_iter()
            .filter(|t| {
                t.auto_signup
                    && !t.done
                    && !t.url.trim().is_empty()
                    && t.signup_at_ms > now
                    && t.signup_at_ms <= end
            })
            .map(|t| {
                let at = t.signup_at_ms;
                (t, at)
            })
            .collect();
        out.sort_by_key(|(_, at)| *at);
        out.truncate(64);
        out
    }
}

/// 解析用户输入的截止时间，支持：
/// 空（无时间）、毫秒/秒时间戳、`yyyy-MM-dd HH:mm`、`MM-dd HH:mm`、
/// `今天/明天/后天 HH:mm`、`HH:mm`（今天已过则顺延到明天）。
pub fn parse_due_ms(input: &str) -> Option<i64> {
    let raw = input.trim();
    if raw.is_empty() {
        return Some(0);
    }
    if raw.chars().all(|c| c.is_ascii_digit()) {
        let n: i64 = raw.parse().ok()?;
        return Some(if n < 100_000_000_000 { n * 1000 } else { n });
    }

    let today = Local::now().date_naive();
    let mut rest = raw
        .replace('年', "-")
        .replace('月', "-")
        .replace("日", "")
        .replace('/', "-")
        .replace('：', ":")
        .replace('点', ":")
        .replace('时', ":")
        .replace("分", "")
        .trim()
        .to_string();

    let mut date: Option<NaiveDate> = None;
    for (prefix, offset) in [("今天", 0i64), ("今晚", 0), ("明日", 1), ("明天", 1), ("后天", 2)] {
        if let Some(r) = rest.strip_prefix(prefix) {
            date = Some(today + Duration::days(offset));
            rest = r.trim().to_string();
            break;
        }
    }

    let mut time: Option<(u32, u32)> = None;
    let mut date_part = String::new();
    if date.is_none() {
        let mut buf = String::new();
        for ch in rest.chars() {
            if ch.is_ascii_digit() || ch == '-' || ch == ':' {
                buf.push(ch);
            } else if ch == 'T' || ch == 't' {
                buf.push(' ');
            } else {
                buf.push(' ');
                continue;
            }
        }
        let parts: Vec<&str> = buf.split_whitespace().collect();
        for p in parts {
            if p.contains(':') {
                let (h, m) = p.split_once(':')?;
                time = Some((h.parse().ok()?, m.parse().ok()?));
            } else if p.contains('-') && p.len() >= 5 {
                date_part = p.to_string();
            }
        }
        if !date_part.is_empty() {
            let mut segs: Vec<&str> = date_part.split('-').filter(|s| !s.is_empty()).collect();
            let (y, m, d) = match segs.len() {
                3 => (
                    segs[0].parse::<i32>().ok()?,
                    segs[1].parse::<u32>().ok()?,
                    segs[2].parse::<u32>().ok()?,
                ),
                2 => (
                    today.year(),
                    segs[0].parse::<u32>().ok()?,
                    segs[1].parse::<u32>().ok()?,
                ),
                _ => return None,
            };
            segs.clear();
            date = NaiveDate::from_ymd_opt(y, m, d);
        }
    } else {
        // 已识别“今天/明天”，剩余部分只取时间。
        let cleaned = rest.replace(':', ":");
        if let Some((h, m)) = cleaned.split_once(':') {
            let h = h.trim().parse::<u32>().ok()?;
            let m = m.trim().parse::<u32>().ok()?;
            time = Some((h, m));
        }
    }

    let date = date.unwrap_or(today);
    let (h, m) = time.unwrap_or((9, 0));
    let ndt = date.and_hms_opt(h.min(23), m.min(59), 0)?;
    let ms = Local.from_local_datetime(&ndt).single()?.timestamp_millis();

    // 只写 HH:mm 且今天已过 → 视为明天。
    if time.is_some() && date == today && ms <= Local::now().timestamp_millis() && !raw.contains('-') {
        let ndt = (today + Duration::days(1)).and_hms_opt(h.min(23), m.min(59), 0)?;
        return Local
            .from_local_datetime(&ndt)
            .single()
            .map(|x| x.timestamp_millis());
    }
    Some(ms)
}

/// 给 Java 展示用的简短时间文本。
pub fn format_ms(ms: i64, with_year: bool) -> String {
    if ms <= 0 {
        return String::new();
    }
    let Some(dt) = Local.timestamp_millis_opt(ms).single() else {
        return String::new();
    };
    if with_year {
        dt.format("%Y-%m-%d %H:%M").to_string()
    } else {
        dt.format("%m-%d %H:%M").to_string()
    }
}

/// 解析 `yyyy-MM-dd`，失败返回 None。
pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub fn now_text() -> String {
    Local::now().format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmpdir(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "lancai-sched-{}-{}-{}",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn todo(title: &str, due_ms: i64, remind: i64) -> Todo {
        Todo {
            id: 0,
            title: title.to_string(),
            note: String::new(),
            due_ms,
            remind_min: remind,
            done: false,
            kind: "todo".to_string(),
            url: String::new(),
            created_ms: 0,
            auto_signup: false,
            signup_at_ms: 0,
        }
    }

    #[test]
    fn upsert_list_toggle_delete() {
        let dir = tmpdir("store");
        let store = ScheduleStore::new(dir.join("schedule.json"));
        let now = Local::now().timestamp_millis();

        let a = store.upsert(todo("交作业", now + 3_600_000, 10)).unwrap();
        let b = store.upsert(todo("领取材料", now + 7_200_000, 0)).unwrap();
        assert!(a.id > 0 && b.id > a.id);

        let items = store.load();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].title, "交作业");

        assert!(store.set_done(b.id, true).unwrap());
        let items = store.load();
        assert!(items[0].title == "交作业" && items[1].done);

        assert!(store.delete(a.id).unwrap());
        assert_eq!(store.load().len(), 1);
        assert!(store.delete(9999).unwrap() == false);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn upcoming_filters_and_sorts() {
        let dir = tmpdir("upcoming");
        let store = ScheduleStore::new(dir.join("schedule.json"));
        let now = Local::now().timestamp_millis();
        store.upsert(todo("一小时后", now + 3_600_000, 10)).unwrap();
        store.upsert(todo("十分钟后", now + 600_000, 0)).unwrap();
        let done = store.upsert(todo("已完成的", now + 900_000, 5)).unwrap();
        store.set_done(done.id, true).unwrap();
        store.upsert(todo("不提醒", now + 1_200_000, -1)).unwrap();

        let up = store.upcoming(now, 24 * 3_600_000);
        assert_eq!(up.len(), 2);
        assert_eq!(up[0].0.title, "十分钟后");
        assert_eq!(up[1].0.title, "一小时后");
        assert_eq!(up[1].1, now + 3_600_000 - 600_000);

        let none = store.upcoming(now, 60_000);
        assert!(none.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn signups_respect_confirmation_and_url() {
        let dir = tmpdir("signup");
        let store = ScheduleStore::new(dir.join("schedule.json"));
        let now = Local::now().timestamp_millis();
        let mut a = todo("讲座报名", now + 7_200_000, 10);
        a.kind = "activity".to_string();
        a.url = "https://example.com/a".to_string();
        a.auto_signup = true;
        a.signup_at_ms = now + 3_600_000;
        let saved = store.upsert(a).unwrap();
        assert!(saved.auto_signup);

        let list = store.signups(now, 24 * 3_600_000);
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].0.id, saved.id);

        // 没有链接时自动报名会被关闭。
        let mut b = todo("无链接", now + 7_200_000, 10);
        b.auto_signup = true;
        b.signup_at_ms = now + 3_600_000;
        let saved_b = store.upsert(b).unwrap();
        assert!(!saved_b.auto_signup);
        assert_eq!(saved_b.signup_at_ms, 0);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_due_formats() {
        let today = Local::now().date_naive();
        let ms = parse_due_ms(&format!("{} 18:30", today.format("%Y-%m-%d"))).unwrap();
        let dt = Local.timestamp_millis_opt(ms).single().unwrap();
        assert_eq!(dt.format("%H:%M").to_string(), "18:30");
        assert_eq!(dt.date_naive(), today);

        assert_eq!(parse_due_ms("").unwrap(), 0);
        assert_eq!(parse_due_ms("   ").unwrap(), 0);

        let tomorrow = parse_due_ms("明天 08:00").unwrap();
        let dt = Local.timestamp_millis_opt(tomorrow).single().unwrap();
        assert_eq!(dt.date_naive(), today + Duration::days(1));

        let n = parse_due_ms("2026-11-11 09:05").unwrap();
        let dt = Local.timestamp_millis_opt(n).single().unwrap();
        assert_eq!(dt.format("%Y-%m-%d %H:%M").to_string(), "2026-11-11 09:05");
    }
}
