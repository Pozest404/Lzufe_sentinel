//! 能否参加判断：从通知/日程文本中提取活动时间，与本地课表比较。
//!
//! 只做本地规则推理，不联网。输出给 Android 端用于「保存活动时询问是否自动报名」，
//! 以及首页分析结果里的「能否参加」提示。

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use regex::Regex;
use serde::Serialize;
use std::sync::OnceLock;

use crate::timetable;

const DAY_CN: [&str; 7] = ["一", "二", "三", "四", "五", "六", "日"];
const DEFAULT_DURATION_MIN: i64 = 90;

#[derive(Debug, Clone, Serialize)]
pub struct Assess {
    /// 课表上没有冲突时为 true；没识别到时间也返回 true，但 time_known=false。
    pub can_attend: bool,
    pub reason: String,
    /// 与活动时间重叠的课程，已格式化为可直接展示的中文。
    pub conflicts: Vec<String>,
    pub event_start_ms: i64,
    pub event_end_ms: i64,
    pub deadline_ms: i64,
    /// 是否从文本里识别到了具体时间。
    pub time_known: bool,
}

struct Patterns {
    date_full: Regex,
    date_cn: Regex,
    date_md: Regex,
    weekday: Regex,
    range: Regex,
    single: Regex,
    cn_time: Regex,
    deadline: Regex,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| Patterns {
        date_full: Regex::new(r"(\d{4})\s*[-/.年]\s*(\d{1,2})\s*[-/.月]\s*(\d{1,2})\s*日?").unwrap(),
        date_cn: Regex::new(r"(\d{1,2})\s*月\s*(\d{1,2})\s*日?").unwrap(),
        date_md: Regex::new(r"(\d{1,2})\s*[-/]\s*(\d{1,2})").unwrap(),
        weekday: Regex::new(r"(?:周|星期|礼拜)\s*([一二三四五六日天])").unwrap(),
        range: Regex::new(r"(\d{1,2})\s*:\s*(\d{2})\s*[-~—－至]\s*(\d{1,2})\s*:\s*(\d{2})")
            .unwrap(),
        single: Regex::new(r"(\d{1,2})\s*:\s*(\d{2})").unwrap(),
        cn_time: Regex::new(r"(\d{1,2})\s*[点时](?:\s*(\d{1,2})\s*分?)?").unwrap(),
        deadline: Regex::new(r"(?:报名)?截止").unwrap(),
    })
}

fn normalize(s: &str) -> String {
    s.replace('：', ":")
        .replace('－', "-")
        .replace('—', "-")
        .replace('～', "~")
        .replace('　', " ")
}

fn weekday_num(ch: &str) -> Option<u8> {
    match ch {
        "一" => Some(1),
        "二" => Some(2),
        "三" => Some(3),
        "四" => Some(4),
        "五" => Some(5),
        "六" => Some(6),
        "日" | "天" => Some(7),
        _ => None,
    }
}

/// 修改小时数：晚上 7 点 → 19 点；中午 12 点保持不变。
fn apply_period(h: u32, before: &str) -> u32 {
    let tail: String = before
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    if tail.contains("下午") || tail.contains("晚上") || tail.ends_with('晚') || tail.contains("傍晚") {
        if h < 12 {
            return h + 12;
        }
    }
    if tail.contains("上午") || tail.contains("早上") || tail.contains("凌晨") {
        if h == 12 {
            return 0;
        }
    }
    h
}

struct Event {
    start: NaiveDateTime,
    end: NaiveDateTime,
    start_ms: i64,
    end_ms: i64,
}

fn date_from_explicit(p: &Patterns, text: &str) -> Option<NaiveDate> {
    if let Some(c) = p.date_full.captures(text) {
        let y = c[1].parse::<i32>().ok()?;
        let m = c[2].parse::<u32>().ok()?;
        let d = c[3].parse::<u32>().ok()?;
        return NaiveDate::from_ymd_opt(y, m, d);
    }
    let (m, d) = if let Some(c) = p.date_cn.captures(text) {
        (c[1].parse::<u32>().ok()?, c[2].parse::<u32>().ok()?)
    } else if let Some(c) = p.date_md.captures(text) {
        (c[1].parse::<u32>().ok()?, c[2].parse::<u32>().ok()?)
    } else {
        return None;
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let today = Local::now().date_naive();
    let mut date = NaiveDate::from_ymd_opt(today.year(), m, d)?;
    // 跨年：如果已过去较久，视为明年。
    if (today - date).num_days() > 180 {
        date = NaiveDate::from_ymd_opt(today.year() + 1, m, d)?;
    }
    Some(date)
}

fn parse_event(p: &Patterns, text: &str, today: NaiveDate) -> Option<Event> {
    let (h1, m1, h2, m2, has_range) = if let Some(c) = p.range.captures(text) {
        (
            c[1].parse::<u32>().ok()?,
            c[2].parse::<u32>().ok()?,
            c[3].parse::<u32>().ok()?,
            c[4].parse::<u32>().ok()?,
            true,
        )
    } else if let Some(c) = p.single.captures(text) {
        (
            c[1].parse::<u32>().ok()?,
            c[2].parse::<u32>().ok()?,
            0,
            0,
            false,
        )
    } else if let Some(c) = p.cn_time.captures(text) {
        let h = c[1].parse::<u32>().ok()?;
        let m = c
            .get(2)
            .and_then(|x| x.as_str().parse::<u32>().ok())
            .unwrap_or(0);
        (h, m, 0, 0, false)
    } else {
        return None;
    };

    let before = &text[..text.len().min(
        p.range
            .find(text)
            .or_else(|| p.single.find(text))
            .or_else(|| p.cn_time.find(text))
            .map(|m| m.start())
            .unwrap_or(0),
    )];
    let h1 = apply_period(h1, before);
    let mut h2 = if has_range { apply_period(h2, before) } else { 0 };
    if h1 > 23 || m1 > 59 {
        return None;
    }
    if has_range && (h2 > 23 || m2 > 59) {
        return None;
    }
    // 13:00-2:00 这类跨中午的写法，补 12 小时更合理。
    if has_range && h2 < h1 && h2 + 12 > h1 {
        h2 += 12;
    }

    let date = date_from_explicit(p, text).unwrap_or_else(|| {
        if let Some(c) = p.weekday.captures(text) {
            if let Some(target) = c.get(1).and_then(|m| weekday_num(m.as_str())) {
                let diff = (target as i64
                    - today.weekday().number_from_monday() as i64)
                    .rem_euclid(7);
                return today + Duration::days(diff);
            }
        }
        today
    });

    let start = NaiveTime::from_hms_opt(h1, m1, 0)?;
    let start_dt = date.and_time(start);
    let end_dt = if has_range {
        date.and_time(NaiveTime::from_hms_opt(h2, m2, 0)?)
    } else {
        start_dt + Duration::minutes(DEFAULT_DURATION_MIN)
    };
    let start_ms = Local
        .from_local_datetime(&start_dt)
        .single()
        .map(|x| x.timestamp_millis())
        .unwrap_or(0);
    let end_ms = Local
        .from_local_datetime(&end_dt)
        .single()
        .map(|x| x.timestamp_millis())
        .unwrap_or(0);
    Some(Event {
        start: start_dt,
        end: end_dt,
        start_ms,
        end_ms,
    })
}

fn parse_deadline(p: &Patterns, text: &str, today: NaiveDate) -> i64 {
    let Some(m) = p.deadline.find(text) else {
        return 0;
    };
    let rest: String = text[m.end()..].chars().take(30).collect();
    let rest = rest.trim_start_matches(|c: char| {
        !c.is_ascii_digit() && !"一二三四五六七八九十".contains(c)
    });
    // 截止内容里可能只有日期，此时默认当天 23:59。
    let date = date_from_explicit(p, rest).or_else(|| {
        if rest.starts_with("今天") || rest.starts_with("今晚") {
            Some(today)
        } else if rest.starts_with("明天") || rest.starts_with("明日") {
            Some(today + Duration::days(1))
        } else if rest.starts_with("后天") {
            Some(today + Duration::days(2))
        } else {
            None
        }
    });
    let Some(date) = date else {
        return 0;
    };
    let (h, m) = if let Some(c) = p.range.captures(&rest) {
        (
            c[1].parse::<u32>().unwrap_or(23),
            c[2].parse::<u32>().unwrap_or(59),
        )
    } else if let Some(c) = p.single.captures(&rest) {
        (
            c[1].parse::<u32>().unwrap_or(23),
            c[2].parse::<u32>().unwrap_or(59),
        )
    } else {
        (23, 59)
    };
    let Some(dt) = date.and_hms_opt(h.min(23), m.min(59), 0) else {
        return 0;
    };
    Local
        .from_local_datetime(&dt)
        .single()
        .map(|x| x.timestamp_millis())
        .unwrap_or(0)
}

fn clock_of(t: &str) -> Option<(u32, u32)> {
    let t = t.trim();
    let (h, m) = t.split_once(':')?;
    let h = h.trim().parse::<u32>().ok()?;
    let m = m.trim().parse::<u32>().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some((h, m))
}

fn conflicts_for(tt: &timetable::Timetable, event: &Event) -> Vec<String> {
    let date = event.start.date();
    let wd = date.weekday().number_from_monday() as u8;
    let week = if tt.term_start.trim().is_empty() {
        0
    } else {
        timetable::current_week(&tt.term_start, date, tt.weeks)
    };
    let es = event.start.hour() * 60 + event.start.minute();
    let ee = event.end.hour() * 60 + event.end.minute();

    let mut out = Vec::new();
    for c in &tt.courses {
        if c.day != wd {
            continue;
        }
        if week > 0 && !c.occurs_in(week) {
            continue;
        }
        let (cs, ce) = match clock_of(&c.start_time) {
            Some((h, m)) => {
                let cs = h * 60 + m;
                let ce = clock_of(&c.end_time)
                    .map(|(eh, em)| eh * 60 + em)
                    .unwrap_or_else(|| timetable::section_end_min(c.end_section));
                (cs, ce.max(cs))
            }
            None => (
                timetable::section_start_min(c.start_section),
                timetable::section_end_min(c.end_section),
            ),
        };
        if cs < ee && es < ce {
            out.push(format!(
                "周{} 第{}-{}节 {}（{:02}:{:02}-{:02}:{:02}）",
                DAY_CN[(wd - 1) as usize],
                c.start_section,
                c.end_section,
                c.name,
                cs / 60,
                cs % 60,
                ce / 60,
                ce % 60
            ));
        }
    }
    out
}

/// 判断某条通知/日程能否参加，并给出截止时间。
pub fn assess(text: &str, tt: &timetable::Timetable, now: NaiveDateTime) -> Assess {
    let p = patterns();
    let normalized = normalize(text);
    let today = now.date();
    let deadline_ms = parse_deadline(p, &normalized, today);

    let Some(event) = parse_event(p, &normalized, today) else {
        return Assess {
            can_attend: true,
            reason: "没有识别到具体时间，建议手动确认".to_string(),
            conflicts: Vec::new(),
            event_start_ms: 0,
            event_end_ms: 0,
            deadline_ms,
            time_known: false,
        };
    };

    let conflicts = conflicts_for(tt, &event);
    let (can_attend, reason) = if tt.courses.is_empty() {
        (true, "课表为空，可以参加".to_string())
    } else if conflicts.is_empty() {
        (true, "课表上没有冲突，可以参加".to_string())
    } else {
        (
            false,
            format!("与课程时间冲突：{}", conflicts.join("；")),
        )
    };

    Assess {
        can_attend,
        reason,
        conflicts,
        event_start_ms: event.start_ms,
        event_end_ms: event.end_ms,
        deadline_ms,
        time_known: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timetable::Course;

    fn now_at(y: i32, m: u32, d: u32) -> NaiveDateTime {
        Local
            .with_ymd_and_hms(y, m, d, 9, 0, 0)
            .single()
            .unwrap()
            .naive_local()
    }

    fn tt_with(course: Course) -> timetable::Timetable {
        timetable::Timetable {
            term_start: "2026-09-07".to_string(),
            weeks: 16,
            courses: vec![course],
        }
    }

    fn course(day: u8, start_sec: u8, end_sec: u8, st: &str, et: &str) -> Course {
        Course {
            name: "高等数学".to_string(),
            day,
            start_section: start_sec,
            end_section: end_sec,
            start_time: st.to_string(),
            end_time: et.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn conflict_with_evening_course() {
        let tt = tt_with(course(5, 9, 10, "19:00", "20:40"));
        let a = assess("周五 19:00-21:00 学术讲座", &tt, now_at(2026, 11, 11));
        assert!(a.time_known);
        assert!(!a.can_attend);
        assert_eq!(a.conflicts.len(), 1);
        assert!(a.conflicts[0].contains("高等数学"));
        assert!(a.event_start_ms > 0);
    }

    #[test]
    fn free_time_can_attend() {
        let tt = tt_with(course(5, 1, 2, "08:00", "09:40"));
        let a = assess("11月13日 19:00 讲座", &tt, now_at(2026, 11, 11));
        assert!(a.can_attend);
        assert!(a.conflicts.is_empty());
        assert!(a.reason.contains("没有冲突"));
    }

    #[test]
    fn no_time_is_unknown() {
        let tt = tt_with(course(5, 9, 10, "19:00", "20:40"));
        let a = assess("周五 学术讲座，欢迎参加", &tt, now_at(2026, 11, 11));
        assert!(!a.time_known);
        assert!(a.can_attend);
    }

    #[test]
    fn deadline_is_parsed() {
        let tt = tt_with(course(5, 9, 10, "19:00", "20:40"));
        let a = assess("周五 19:00 讲座，报名截止 11月14日 18:00", &tt, now_at(2026, 11, 11));
        let dt = Local.timestamp_millis_opt(a.deadline_ms).single().unwrap();
        assert_eq!(dt.format("%m-%d %H:%M").to_string(), "11-14 18:00");
    }

    #[test]
    fn overnight_weekday_is_resolved() {
        let tt = timetable::Timetable::default();
        // 2026-11-11 是周三，下一周一是 11-16。
        let a = assess("周一 08:00 升旗仪式", &tt, now_at(2026, 11, 11));
        let dt = Local.timestamp_millis_opt(a.event_start_ms).single().unwrap();
        assert_eq!(dt.format("%m-%d").to_string(), "11-16");
    }
}
