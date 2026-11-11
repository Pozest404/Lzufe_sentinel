//! 文本工具：清洗、截断、关键词包含判断。
//!
//! 与原 Java 版保持一致的两个细节：
//! - 全角空格（U+3000）转半角空格，去掉零宽字符；
//! - 截断按“字符”而不是字节，避免切坏中文/emoji。

/// 清洗：全角空格→半角、去零宽字符、制表/回车→空格、压缩连续空格、trim。
pub fn clean(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = false;
    for ch in s.chars() {
        match ch {
            '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}' => continue,
            '\u{3000}' => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            '\t' | '\r' | '\u{000c}' => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            ' ' => {
                if !last_space {
                    out.push(' ');
                    last_space = true;
                }
            }
            _ => {
                out.push(ch);
                last_space = false;
            }
        }
    }
    out.trim().to_string()
}

/// 标题清洗：在 clean 基础上去掉首尾标点。
pub fn clean_title(s: &str) -> String {
    let v = clean(s);
    let v = v.trim_start_matches(|c: char| {
        matches!(c, '：' | ':' | '-' | '—' | '·' | '、' | '，' | ',' | ' ' | '\t')
    });
    let v = v.trim_end_matches(|c: char| {
        matches!(c, '。' | '；' | ';' | '！' | '!' | '？' | '?' | '，' | ',' | '、' | ' ')
    });
    v.trim().to_string()
}

/// 按字符截断，超长加省略号，保证总字符数 <= max。
pub fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

/// 单行摘要：换行转空格，按字符截断。
pub fn abbrev(s: &str, n: usize) -> String {
    let one = s.replace('\n', " ");
    let one = one.trim();
    let count = one.chars().count();
    if count <= n {
        one.to_string()
    } else {
        let mut out: String = one.chars().take(n).collect();
        out.push('…');
        out
    }
}

pub fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 忽略空白的包含判断："11月11日 18:00" 与 "11月11日18:00" 视为同一段。
pub fn loose_contains(haystack: &str, needle: &str) -> bool {
    let h: String = haystack.chars().filter(|c| !c.is_whitespace()).collect();
    let n: String = needle.chars().filter(|c| !c.is_whitespace()).collect();
    !h.is_empty() && !n.is_empty() && h.contains(&n)
}

pub fn contains_any(text: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| !k.is_empty() && text.contains(k))
}

pub fn split_words(raw: &str) -> Vec<String> {
    raw.split(|c: char| matches!(c, ',' | '，' | '、' | ';' | '；') || c.is_whitespace())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// ASCII 关键词要求词边界，避免 "pu" 命中 "public" / "update"。
pub fn contains_keyword(lower: &str, kw: &str) -> bool {
    if kw.is_empty() {
        return false;
    }
    if !is_ascii_word(kw) {
        return lower.contains(kw);
    }
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find(kw) {
        let i = from + rel;
        let end = i + kw.len();
        let left_ok = i == 0 || !is_ascii_letter(lower[..i].chars().next_back().unwrap_or(' '));
        let right_ok = end >= lower.len()
            || !is_ascii_letter(lower[end..].chars().next().unwrap_or(' '));
        if left_ok && right_ok {
            return true;
        }
        from = i + lower[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
    }
    false
}

fn is_ascii_word(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
}

fn is_ascii_letter(c: char) -> bool {
    c.is_ascii_alphabetic()
}

/// 字符数（与 Java String.length 在 BMP 范围内一致，emoji 按 1 个字符算）。
pub fn char_len(s: &str) -> usize {
    s.chars().count()
}

pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// 从 JSON 编码的 evaluateJavascript 返回值里剥引号（对应 Java unquote）。
pub fn unquote(raw: &str) -> String {
    let s = raw.trim();
    let s = if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        &s[1..s.len() - 1]
    } else {
        s
    };
    s.replace("\\\"", "\"").replace("\\\\", "\\")
}
