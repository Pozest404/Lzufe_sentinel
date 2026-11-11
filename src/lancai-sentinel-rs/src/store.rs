//! 消息本地存储（JSONL，最近 N 条）+ 最近一条重要摘要。

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::model::{Msg, Summary};

pub struct MsgStore {
    path: PathBuf,
    last_summary_path: PathBuf,
    cache: Vec<Msg>,
    max: usize,
    loaded: bool,
}

impl MsgStore {
    pub fn new(
        path: impl Into<PathBuf>,
        last_summary_path: impl Into<PathBuf>,
        max: usize,
    ) -> Self {
        let mut s = Self {
            path: path.into(),
            last_summary_path: last_summary_path.into(),
            cache: Vec::new(),
            max: max.max(10),
            loaded: false,
        };
        s.ensure_loaded();
        s
    }

    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        let Ok(raw) = fs::read_to_string(&self.path) else {
            return;
        };
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(m) = serde_json::from_str::<Msg>(line) {
                self.cache.push(m);
            }
        }
        while self.cache.len() > self.max {
            self.cache.remove(0);
        }
    }

    /// 返回 false 表示 3 秒内同群同文重复，未入库。
    pub fn add(&mut self, m: &Msg) -> bool {
        self.ensure_loaded();
        let from = self.cache.len().saturating_sub(8);
        for x in &self.cache[from..] {
            if same(x, m) {
                return false;
            }
        }
        self.cache.push(m.clone());
        while self.cache.len() > self.max {
            self.cache.remove(0);
        }
        self.append(m);
        true
    }

    pub fn recent(&self, n: usize) -> Vec<Msg> {
        self.cache.iter().rev().take(n).cloned().collect()
    }

    pub fn count(&self) -> usize {
        self.cache.len()
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.loaded = true;
        let _ = fs::remove_file(&self.path);
    }

    pub fn set_last_summary(&self, s: &Summary) -> std::io::Result<()> {
        if let Some(p) = self.last_summary_path.parent() {
            fs::create_dir_all(p)?;
        }
        let json = serde_json::to_string_pretty(&s.to_json())?;
        fs::write(&self.last_summary_path, json)
    }

    pub fn get_last_summary(&self) -> Option<Summary> {
        let raw = fs::read_to_string(&self.last_summary_path).ok()?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        serde_json::from_value(v).ok()
    }

    fn append(&self, m: &Msg) {
        if let Some(p) = self.path.parent() {
            let _ = fs::create_dir_all(p);
        }
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            let _ = writeln!(f, "{}", m.to_json());
        }
    }
}

fn same(a: &Msg, b: &Msg) -> bool {
    a.text == b.text && a.group == b.group && (a.time - b.time).abs() < 3000
}
