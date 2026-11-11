//! 课表：周视图数据、学期周次计算，以及从常见导出格式导入。
//!
//! 支持：
//! - `星课程表`/系统日历导出的 `.ics`（iCalendar，含 RRULE 周重复）
//! - `WakeUp` 等课程表 App 导出的 JSON 课程数组
//! - 带表头的 CSV（课程名称/教师/地点/星期/节次/周次）
//! - 直接粘贴的文本行，例如：`高等数学 周一 1-2节 1-16周 教一101 张老师`
//!
//! 解析全部在 Rust 完成；Java 只负责把用户选择/粘贴的文本传进来。

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const DEFAULT_WEEKS: u8 = 20;

/// 一天 10 节：上午 4 节、下午 4 节、晚上 2 节。
pub const SECTION_COUNT: u8 = 10;

const SECTION_STARTS: [u32; 10] = [
    8 * 60 + 30,
    9 * 60 + 25,
    10 * 60 + 40,
    11 * 60 + 35,
    14 * 60 + 30,
    15 * 60 + 25,
    16 * 60 + 30,
    17 * 60 + 25,
    19 * 60,
    19 * 60 + 55,
];
const SECTION_ENDS: [u32; 10] = [
    9 * 60 + 25,
    10 * 60 + 20,
    11 * 60 + 35,
    12 * 60 + 30,
    15 * 60 + 25,
    16 * 60 + 20,
    17 * 60 + 25,
    18 * 60 + 20,
    19 * 60 + 55,
    20 * 60 + 50,
];

fn default_weeks() -> u8 {
    DEFAULT_WEEKS
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Course {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub teacher: String,
    #[serde(default)]
    pub location: String,
    /// 1=周一 … 7=周日
    #[serde(default)]
    pub day: u8,
    #[serde(default)]
    pub start_section: u8,
    #[serde(default)]
    pub end_section: u8,
    #[serde(default)]
    pub start_time: String,
    #[serde(default)]
    pub end_time: String,
    /// 空表示每周都有。
    #[serde(default)]
    pub weeks: Vec<u8>,
}

impl Course {
    pub fn occurs_in(&self, week: u8) -> bool {
        self.weeks.is_empty() || self.weeks.contains(&week)
    }

    fn normalize(&mut self) {
        self.name = self.name.trim().to_string();
        self.teacher = self.teacher.trim().to_string();
        self.location = self.location.trim().to_string();
        self.day = self.day.clamp(1, 7);
        self.start_section = self.start_section.clamp(1, SECTION_COUNT);
        self.end_section = self.end_section.clamp(self.start_section, SECTION_COUNT);
        self.weeks.retain(|w| (1..=30).contains(w));
        self.weeks.sort_unstable();
        self.weeks.dedup();
        if self.start_time.is_empty() {
            self.start_time = section_start_text(self.start_section);
        }
        if self.end_time.is_empty() {
            self.end_time = section_end_text(self.end_section);
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Timetable {
    /// 学期第一周的周一，格式 yyyy-MM-dd；空表示尚未设置。
    #[serde(default)]
    pub term_start: String,
    #[serde(default = "default_weeks")]
    pub weeks: u8,
    #[serde(default)]
    pub courses: Vec<Course>,
}

pub struct ImportOk {
    pub courses: usize,
    pub format: String,
    pub term_start: String,
}

/// 课表文件存储：`timetable.json`。
pub struct TimetableStore {
    path: PathBuf,
}

impl TimetableStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Timetable {
        let mut tt = match fs::read_to_string(&self.path) {
            Ok(raw) => serde_json::from_str::<Timetable>(&raw).unwrap_or_default(),
            Err(_) => Timetable::default(),
        };
        if tt.weeks == 0 {
            tt.weeks = DEFAULT_WEEKS;
        }
        for c in &mut tt.courses {
            c.normalize();
        }
        tt
    }

    fn save(&self, tt: &Timetable) -> Result<()> {
        if let Some(p) = self.path.parent() {
            fs::create_dir_all(p)
                .with_context(|| format!("创建目录失败: {}", p.display()))?;
        }
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_string_pretty(tt)?)
            .with_context(|| format!("写入失败: {}", tmp.display()))?;
        fs::rename(&tmp, &self.path)
            .with_context(|| format!("替换文件失败: {}", self.path.display()))?;
        Ok(())
    }

    /// 导入并覆盖当前课表。返回课程数与识别到的格式。
    pub fn import(&self, input: &str, today: NaiveDate) -> Result<ImportOk> {
        let (mut courses, detected_term, format) = parse_import(input, today)?;
        if courses.is_empty() {
            bail!("没有识别到课程，请检查导出格式或粘贴内容");
        }
        for c in &mut courses {
            c.normalize();
        }
        courses.sort_by(|a, b| {
            a.day
                .cmp(&b.day)
                .then(a.start_section.cmp(&b.start_section))
                .then(a.name.cmp(&b.name))
        });
        let mut seen = Vec::new();
        courses.retain(|c| {
            let key = (
                c.name.clone(),
                c.day,
                c.start_section,
                c.end_section,
                c.weeks.clone(),
                c.location.clone(),
            );
            if seen.contains(&key) {
                false
            } else {
                seen.push(key);
                true
            }
        });

        let max_week = courses
            .iter()
            .flat_map(|c| c.weeks.iter().copied())
            .max()
            .unwrap_or(DEFAULT_WEEKS)
            .max(1)
            .min(30);
        // 文本 / CSV 本身没有学期信息。此时若已有 term_start 就沿用，
        // 否则每次导入都会把「第 1 周」重置为当前周，周次整体错位。
        let existing_term = self.load().term_start;
        let term_start = match detected_term {
            Some(d) => monday_of(d),
            None => NaiveDate::parse_from_str(existing_term.trim(), "%Y-%m-%d")
                .map(monday_of)
                .unwrap_or_else(|_| monday_of(today)),
        };
        let tt = Timetable {
            term_start: term_start.format("%Y-%m-%d").to_string(),
            weeks: max_week,
            courses,
        };
        self.save(&tt)?;
        Ok(ImportOk {
            courses: tt.courses.len(),
            format: format.to_string(),
            term_start: tt.term_start.clone(),
        })
    }

    pub fn set_term(&self, term_start: &str, weeks: u8) -> Result<Timetable> {
        let mut tt = self.load();
        if !term_start.trim().is_empty() {
            let d = NaiveDate::parse_from_str(term_start.trim(), "%Y-%m-%d")
                .map_err(|_| anyhow::anyhow!("日期格式应为 yyyy-MM-dd"))?;
            tt.term_start = monday_of(d).format("%Y-%m-%d").to_string();
        }
        tt.weeks = weeks.clamp(1, 30);
        self.save(&tt)?;
        Ok(tt)
    }

    pub fn delete_course(&self, index: usize) -> Result<Timetable> {
        let mut tt = self.load();
        if index >= tt.courses.len() {
            bail!("课程不存在");
        }
        tt.courses.remove(index);
        self.save(&tt)?;
        Ok(tt)
    }

    pub fn clear(&self) -> Result<()> {
        let _ = fs::remove_file(&self.path);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// 周次与节次工具
// ---------------------------------------------------------------------------

pub fn monday_of(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

pub fn current_week(term_start: &str, today: NaiveDate, weeks: u8) -> u8 {
    let Ok(start) = NaiveDate::parse_from_str(term_start.trim(), "%Y-%m-%d") else {
        return 1;
    };
    let diff = (monday_of(today) - monday_of(start)).num_days() / 7;
    (diff + 1).clamp(1, weeks.max(1) as i64) as u8
}

fn minutes(t: NaiveTime) -> u32 {
    t.hour() * 60 + t.minute()
}

/// 按「某节的结束时间」定位开始节次。
///
/// 不能再用「开始时间 <= 给定时间」的写法：那会让 14:00 的课落进第 4 节
/// （11:35 开始），跨过整个午休空档。改成找「第一节结束时间晚于起点」的课，
/// 08:00→1、10:20→3、14:00→5、16:10→6、19:00→9。
fn section_for_start(t: NaiveTime) -> u8 {
    let m = minutes(t);
    for (i, end) in SECTION_ENDS.iter().enumerate() {
        if m < *end {
            return (i + 1) as u8;
        }
    }
    SECTION_COUNT
}

/// 按「某节的结束时间」定位结束节次：第一节结束时间不早于 `m` 的课即为最后一节。
/// 08:00-09:40 → 第 2 节；14:00-15:40 → 第 6 节。
fn section_for_end(t: NaiveTime) -> u8 {
    let m = minutes(t);
    for (i, end) in SECTION_ENDS.iter().enumerate() {
        if m <= *end {
            return (i + 1) as u8;
        }
    }
    SECTION_COUNT
}

pub fn section_start_text(sec: u8) -> String {
    let s = SECTION_STARTS[(sec.clamp(1, SECTION_COUNT) - 1) as usize];
    format!("{:02}:{:02}", s / 60, s % 60)
}

pub fn section_end_text(sec: u8) -> String {
    let s = SECTION_ENDS[(sec.clamp(1, SECTION_COUNT) - 1) as usize];
    format!("{:02}:{:02}", s / 60, s % 60)
}

/// 第 `sec` 节的开始时间（从 0 点起的分钟数）。
pub fn section_start_min(sec: u8) -> u32 {
    SECTION_STARTS[(sec.clamp(1, SECTION_COUNT) - 1) as usize]
}

/// 第 `sec` 节的结束时间（从 0 点起的分钟数）。
pub fn section_end_min(sec: u8) -> u32 {
    SECTION_ENDS[(sec.clamp(1, SECTION_COUNT) - 1) as usize]
}

fn parse_hhmm(s: &str) -> Option<NaiveTime> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    for fmt in ["%H:%M", "%H:%M:%S", "%H：%M"] {
        if let Ok(t) = NaiveTime::parse_from_str(s, fmt) {
            return Some(t);
        }
    }
    if s.len() == 4 && s.chars().all(|c| c.is_ascii_digit()) {
        let h: u32 = s[..2].parse().ok()?;
        let m: u32 = s[2..].parse().ok()?;
        return NaiveTime::from_hms_opt(h, m, 0);
    }
    None
}

// ---------------------------------------------------------------------------
// 导入入口
// ---------------------------------------------------------------------------

fn parse_import(
    input: &str,
    today: NaiveDate,
) -> Result<(Vec<Course>, Option<NaiveDate>, &'static str)> {
    let text = input.trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        bail!("内容为空");
    }
    if text.contains("BEGIN:VCALENDAR") || text.contains("BEGIN:VEVENT") {
        let (courses, start) = parse_ics(text);
        if courses.is_empty() {
            bail!("没有在日历文件中找到课程事件");
        }
        return Ok((courses, start, "ICS 日历"));
    }

    let trimmed = text.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        if let Some((courses, term)) = parse_json_schedule(trimmed) {
            return Ok((courses, term, "JSON 课表"));
        }
    }

    if let Some(courses) = parse_csv(text) {
        return Ok((courses, None, "CSV 课表"));
    }

    let courses = parse_text(text, today);
    if courses.is_empty() {
        bail!("没有识别到课程，请检查导出格式或粘贴内容");
    }
    Ok((courses, None, "文本课表"))
}

// ---------------------------------------------------------------------------
// ICS
// ---------------------------------------------------------------------------

fn ics_unfold(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.lines() {
        let line = raw.trim_end_matches('\r');
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(last) = out.last_mut() {
                last.push_str(&line[1..]);
            }
        } else {
            out.push(line.to_string());
        }
    }
    out
}

fn ics_unescape(s: &str) -> String {
    s.replace("\\n", "\n")
        .replace("\\N", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

fn parse_ics_dt(v: &str) -> Option<NaiveDateTime> {
    let v = v.trim();
    let utc = v.ends_with('Z');
    let s = v.trim_end_matches('Z');
    let ndt = if s.len() >= 15 {
        NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S")
            .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M"))
            .ok()?
    } else if s.len() >= 8 {
        NaiveDate::parse_from_str(&s[..8], "%Y%m%d")
            .ok()?
            .and_hms_opt(0, 0, 0)?
    } else {
        return None;
    };
    if utc {
        let dt = chrono::Utc.from_utc_datetime(&ndt);
        Some(dt.with_timezone(&Local).naive_local())
    } else {
        Some(ndt)
    }
}

fn rrule_weeks(rrule: Option<&String>, start: NaiveDateTime) -> u8 {
    let Some(r) = rrule else {
        return 1;
    };
    for part in r.split(';') {
        let up = part.trim().to_uppercase();
        if let Some(v) = up.strip_prefix("COUNT=") {
            if let Ok(n) = v.parse::<u32>() {
                return n.clamp(1, 30) as u8;
            }
        }
        if let Some(v) = up.strip_prefix("UNTIL=") {
            if let Some(until) = parse_ics_dt(v) {
                let days = (until.date() - start.date()).num_days().max(0);
                return ((days / 7) + 1).clamp(1, 30) as u8;
            }
        }
    }
    16
}

fn extract_teacher(desc: &str) -> String {
    let re = Regex::new(r"(?i)(?:教师|老师|授课教师)\s*[:：]\s*([^\n;；,，]+)").unwrap();
    re.captures(desc)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().trim().to_string())
        .unwrap_or_default()
}

/// 返回课程（weeks 仍为相对事件自身的 1..count）与最早事件日期。
fn parse_ics(text: &str) -> (Vec<Course>, Option<NaiveDate>) {
    let lines = ics_unfold(text);
    let mut events: Vec<(Course, NaiveDate, u8)> = Vec::new();
    let mut in_event = false;
    let mut map: Map<String, Value> = Map::new();

    for line in lines {
        let up = line.trim().to_uppercase();
        if up == "BEGIN:VEVENT" {
            in_event = true;
            map.clear();
            continue;
        }
        if up == "END:VEVENT" {
            in_event = false;
            if let Some((course, date, count)) = event_from_map(&map) {
                events.push((course, date, count));
            }
            continue;
        }
        if !in_event {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let base = key
            .split(';')
            .next()
            .unwrap_or(key)
            .trim()
            .to_uppercase();
        map.insert(base, Value::String(ics_unescape(value)));
    }

    let Some(first) = events.iter().map(|(_, d, _)| *d).min() else {
        return (Vec::new(), None);
    };
    let term = monday_of(first);
    let mut out: Vec<Course> = Vec::new();
    for (mut c, date, count) in events {
        let base = ((monday_of(date) - term).num_days() / 7 + 1).clamp(1, 30) as u8;
        let weeks: Vec<u8> = (base..base.saturating_add(count).min(31)).collect();
        // 有些导出器（星课程表逐周导出、系统日历重复事件）不带 RRULE，
        // 而是每周一个 VEVENT。按「同名 / 同天 / 同节 / 同地点」合并周次，
        // 否则一门 16 周的课会被拆成 16 门独立的课。
        if let Some(same) = out.iter_mut().find(|x| {
            x.name == c.name
                && x.day == c.day
                && x.start_section == c.start_section
                && x.end_section == c.end_section
                && x.location == c.location
        }) {
            same.weeks.extend(weeks);
            continue;
        }
        c.weeks = weeks;
        out.push(c);
    }
    for c in &mut out {
        c.normalize();
    }
    (out, Some(term))
}

fn event_from_map(map: &Map<String, Value>) -> Option<(Course, NaiveDate, u8)> {
    let start_raw = map.get("DTSTART")?.as_str()?;
    let start = parse_ics_dt(start_raw)?;
    let end = map
        .get("DTEND")
        .and_then(|v| v.as_str())
        .and_then(parse_ics_dt);
    let name = map
        .get("SUMMARY")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let location = map
        .get("LOCATION")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let desc = map
        .get("DESCRIPTION")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let teacher = extract_teacher(&desc);
    let start_sec = section_for_start(start.time());
    let end_sec = end
        .map(|e| section_for_end(e.time()))
        .unwrap_or(start_sec)
        .max(start_sec);

    let course = Course {
        name,
        teacher,
        location,
        day: start.date().weekday().number_from_monday() as u8,
        start_section: start_sec,
        end_section: end_sec,
        start_time: start.format("%H:%M").to_string(),
        end_time: end
            .map(|e| e.format("%H:%M").to_string())
            .unwrap_or_else(|| section_end_text(end_sec)),
        weeks: Vec::new(),
    };
    let count = map
        .get("RRULE")
        .and_then(|v| v.as_str())
        .map(|r| rrule_weeks(Some(&r.to_string()), start))
        .unwrap_or(1);
    Some((course, start.date(), count))
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

fn parse_json_schedule(text: &str) -> Option<(Vec<Course>, Option<NaiveDate>)> {
    let value: Value = serde_json::from_str(text).ok()?;
    let mut objects = Vec::new();
    collect_objects(&value, &mut objects, 0);
    let mut courses = Vec::new();
    for m in objects {
        if let Some(c) = course_from_map(m) {
            courses.push(c);
        }
    }
    if courses.is_empty() {
        return None;
    }
    let term = find_term_start(&value);
    Some((courses, term))
}

fn collect_objects<'a>(v: &'a Value, out: &mut Vec<&'a Map<String, Value>>, depth: usize) {
    if depth > 8 {
        return;
    }
    match v {
        Value::Object(m) => {
            out.push(m);
            for x in m.values() {
                collect_objects(x, out, depth + 1);
            }
        }
        Value::Array(a) => {
            for x in a {
                collect_objects(x, out, depth + 1);
            }
        }
        _ => {}
    }
}

fn find_term_start(v: &Value) -> Option<NaiveDate> {
    let keys = [
        "termstart",
        "term_start",
        "startdate",
        "start_date",
        "semesterstart",
        "semester_start",
        "开学日期",
    ];
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                let lk = k.to_lowercase();
                if keys.contains(&lk.as_str()) {
                    if let Some(s) = val.as_str() {
                        if let Some(d) = parse_flexible_date(s) {
                            return Some(d);
                        }
                    }
                }
            }
            m.values().find_map(find_term_start)
        }
        Value::Array(a) => a.iter().find_map(find_term_start),
        _ => None,
    }
}

fn parse_flexible_date(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    for fmt in ["%Y-%m-%d", "%Y/%m/%d", "%Y%m%d", "%Y年%m月%d日"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, fmt) {
            return Some(d);
        }
    }
    None
}

fn map_get<'a>(m: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    for (k, v) in m {
        let lk = k.to_lowercase();
        if keys.iter().any(|x| *x == lk) {
            return Some(v);
        }
    }
    None
}

/// 按 `keys` 的**优先级顺序**取值，而不是按 map 的键顺序。
/// 周次同时存在 `week` 和 `weeks` 时，必须优先取 `weeks`。
fn map_get_ordered<'a>(m: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    for k in keys {
        for (mk, mv) in m {
            if mk.to_lowercase() == *k {
                return Some(mv);
            }
        }
    }
    None
}

fn map_str(m: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    map_get(m, keys).and_then(|v| match v {
        Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

const NAME_KEYS: &[&str] = &[
    "name",
    "coursename",
    "course_name",
    "course",
    "title",
    "subject",
    "课程",
    "课程名称",
    "科目",
];
const TEACHER_KEYS: &[&str] = &[
    "teacher",
    "instructor",
    "teachers",
    "教师",
    "老师",
    "授课教师",
];
const LOCATION_KEYS: &[&str] = &[
    "room",
    "location",
    "place",
    "classroom",
    "address",
    "地点",
    "教室",
    "上课地点",
];
const WEEK_KEYS: &[&str] = &[
    "weeklist",
    "weeks",
    "weekindexes",
    "week",
    "weekstr",
    "周次",
    "周数",
];
const DAY_KEYS: &[&str] = &[
    "dayofweek",
    "day_of_week",
    "weekday",
    "day",
    "星期",
    "周几",
];

fn course_from_map(m: &Map<String, Value>) -> Option<Course> {
    let name = map_str(m, NAME_KEYS)?;
    // "week" 在不同导出里既可能是星期（3）也可能是周次（"1-16周"）。
    // 先按星期解析；解析不出来时再回退到 "week"，但不要因此丢掉整门课。
    let day = map_get_ordered(m, DAY_KEYS)
        .and_then(day_from_value)
        .or_else(|| map_get_ordered(m, &["week"]).and_then(day_from_value))?;

    let mut start_sec = 0u8;
    let mut end_sec = 0u8;
    for key in ["sections", "section", "jieci", "节次", "节"] {
        if let Some(v) = map_get(m, &[key]) {
            let (a, b) = sections_from_value(v);
            if a > 0 {
                start_sec = a;
                end_sec = b;
                break;
            }
        }
    }
    if start_sec == 0 {
        for key in ["startsection", "start_section", "startnode", "startunit", "开始节"] {
            if let Some(v) = map_get(m, &[key]) {
                if let Some(n) = value_u8(v) {
                    start_sec = n;
                    break;
                }
            }
        }
    }
    if end_sec == 0 {
        for key in ["endsection", "end_section", "endnode", "endunit", "结束节"] {
            if let Some(v) = map_get(m, &[key]) {
                if let Some(n) = value_u8(v) {
                    end_sec = n;
                    break;
                }
            }
        }
    }

    let start_time = map_str(m, &["starttime", "start_time", "begintime", "begin_time", "start"])
        .unwrap_or_default();
    let end_time =
        map_str(m, &["endtime", "end_time", "finishtime", "finish_time", "end"]).unwrap_or_default();

    if start_sec == 0 {
        if let Some(t) = parse_hhmm(&start_time) {
            start_sec = section_for_start(t);
        }
    }
    if start_sec == 0 {
        start_sec = 1;
    }
    if end_sec == 0 {
        if let Some(t) = parse_hhmm(&end_time) {
            end_sec = section_for_end(t);
        }
    }
    if end_sec == 0 {
        end_sec = start_sec;
    }

    let weeks = map_get_ordered(m, WEEK_KEYS)
        .map(weeks_from_value)
        .unwrap_or_default();

    Some(Course {
        name,
        teacher: map_str(m, TEACHER_KEYS).unwrap_or_default(),
        location: map_str(m, LOCATION_KEYS).unwrap_or_default(),
        day,
        start_section: start_sec,
        end_section: end_sec,
        start_time,
        end_time,
        weeks,
    })
}

fn value_u8(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => n.as_u64().map(|x| x as u8),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn day_from_value(v: &Value) -> Option<u8> {
    match v {
        Value::Number(n) => {
            let d = n.as_i64()?;
            if d == 0 {
                Some(7)
            } else if (1..=7).contains(&d) {
                Some(d as u8)
            } else {
                None
            }
        }
        Value::String(s) => day_from_text(s),
        _ => None,
    }
}

fn day_from_text(s: &str) -> Option<u8> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        return None;
    }
    if s.contains('一') || s.contains("mon") {
        return Some(1);
    }
    if s.contains('二') || s.contains("tue") {
        return Some(2);
    }
    if s.contains('三') || s.contains("wed") {
        return Some(3);
    }
    if s.contains('四') || s.contains("thu") {
        return Some(4);
    }
    if s.contains('五') || s.contains("fri") {
        return Some(5);
    }
    if s.contains('六') || s.contains("sat") {
        return Some(6);
    }
    if s.contains('日') || s.contains('天') || s.contains("sun") {
        return Some(7);
    }
    match s.as_str() {
        "1" => Some(1),
        "2" => Some(2),
        "3" => Some(3),
        "4" => Some(4),
        "5" => Some(5),
        "6" => Some(6),
        "7" => Some(7),
        _ => None,
    }
}

fn sections_from_value(v: &Value) -> (u8, u8) {
    match v {
        Value::Number(n) => n
            .as_u64()
            .map(|x| (x as u8, x as u8))
            .unwrap_or((0, 0)),
        Value::String(s) => sections_from_text(s),
        _ => (0, 0),
    }
}

fn sections_from_text(s: &str) -> (u8, u8) {
    let cleaned = s
        .replace('第', "")
        .replace('节', "")
        .replace('，', "-")
        .replace(',', "-")
        .replace('~', "-")
        .replace('至', "-")
        .replace('—', "-");
    let parts: Vec<&str> = cleaned.split('-').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
    if parts.len() >= 2 {
        if let (Ok(a), Ok(b)) = (parts[0].parse::<u8>(), parts[1].parse::<u8>()) {
            return (a, b.max(a));
        }
    }
    if let Some(p) = parts.first() {
        if let Ok(a) = p.parse::<u8>() {
            return (a, a);
        }
    }
    (0, 0)
}

fn weeks_from_value(v: &Value) -> Vec<u8> {
    match v {
        Value::Array(a) => a
            .iter()
            .filter_map(|x| match x {
                Value::Number(n) => n.as_u64().map(|x| x as u8),
                Value::String(s) => s.trim().parse::<u8>().ok(),
                _ => None,
            })
            .filter(|w| (1..=30).contains(w))
            .collect(),
        Value::String(s) => weeks_from_text(s),
        Value::Number(n) => n
            .as_u64()
            .map(|x| x as u8)
            .filter(|w| (1..=30).contains(w))
            .map(|w| vec![w])
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

fn weeks_from_text(s: &str) -> Vec<u8> {
    let parity = if s.contains('单') {
        Some(1u8)
    } else if s.contains('双') {
        Some(0u8)
    } else {
        None
    };
    let cleaned = s.replace('第', "").replace('周', " ").replace(['（', '）', '(', ')'], " ");

    // 逗号 / 顿号 / 分号 / 空格 = 离散周次；连字符 / ~ / 至 = 区间。
    // 旧实现把所有分隔符统一当区间，`1,3,5,7周` 会被错误展开成 1-3 周。
    let mut weeks: Vec<u8> = Vec::new();
    for part in cleaned
        .split(|c: char| {
            c == ',' || c == '，' || c == '、' || c == ';' || c == '；' || c.is_whitespace()
        })
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
    {
        let range: Vec<&str> = part
            .split(['-', '~', '—', '－', '至'])
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .collect();
        if range.len() >= 2 {
            if let (Some(a), Some(b)) = (leading_digits(range[0]), leading_digits(range[1])) {
                let (a, b) = if a <= b { (a, b) } else { (b, a) };
                for w in a..=b {
                    weeks.push(w);
                }
                continue;
            }
        }
        if let Some(n) = leading_digits(part) {
            weeks.push(n);
        }
    }
    if let Some(p) = parity {
        weeks.retain(|w| w % 2 == p);
    }
    weeks.sort_unstable();
    weeks.dedup();
    weeks
}

/// 取字符串开头连续的 ASCII 数字，忽略后面的单位或文字（`16周` → 16）。
fn leading_digits(s: &str) -> Option<u8> {
    let digits: String = s.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<u8>().ok().filter(|w| (1..=30).contains(w))
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in line.chars() {
        match ch {
            '"' => quoted = !quoted,
            ',' | '\t' | ';' if !quoted => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    out.push(cur.trim().to_string());
    out
}

fn is_known_key(s: &str) -> bool {
    let lk = s.trim().to_lowercase();
    NAME_KEYS.contains(&lk.as_str())
        || TEACHER_KEYS.contains(&lk.as_str())
        || LOCATION_KEYS.contains(&lk.as_str())
        || WEEK_KEYS.contains(&lk.as_str())
        || DAY_KEYS.contains(&lk.as_str())
        || ["sections", "section", "jieci", "节次", "节", "开始节", "结束节"]
            .contains(&lk.as_str())
}

fn parse_csv(text: &str) -> Option<Vec<Course>> {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() < 2 {
        return None;
    }
    let header = split_csv_line(lines[0]);
    if !header.iter().any(|h| is_known_key(h)) {
        return None;
    }
    let mut out = Vec::new();
    for line in &lines[1..] {
        let cols = split_csv_line(line);
        if cols.iter().all(|c| c.trim().is_empty()) {
            continue;
        }
        let mut m = Map::new();
        for (i, h) in header.iter().enumerate() {
            if let Some(v) = cols.get(i) {
                m.insert(h.trim().to_string(), Value::String(v.trim().to_string()));
            }
        }
        if let Some(c) = course_from_map(&m) {
            out.push(c);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

// ---------------------------------------------------------------------------
// 纯文本
// ---------------------------------------------------------------------------

struct TextRules {
    day: Regex,
    sec_range: Regex,
    sec_single: Regex,
    week_range: Regex,
    week_single: Regex,
    time: Regex,
}

impl TextRules {
    fn new() -> Self {
        Self {
            day: Regex::new(r"(?:周|星期|礼拜)\s*([一二三四五六日天1-7])").unwrap(),
            sec_range: Regex::new(r"(\d{1,2})\s*[-~—－至]\s*(\d{1,2})\s*节").unwrap(),
            sec_single: Regex::new(r"第\s*(\d{1,2})\s*节").unwrap(),
            week_range: Regex::new(r"(\d{1,2})\s*[-~—－至]\s*(\d{1,2})\s*周").unwrap(),
            week_single: Regex::new(r"第?\s*(\d{1,2})\s*周").unwrap(),
            time: Regex::new(
                r"(\d{1,2})\s*[:：]\s*(\d{2})\s*[-~—－至]\s*(\d{1,2})\s*[:：]\s*(\d{2})",
            )
            .unwrap(),
        }
    }
}

fn is_noise(token: &str) -> bool {
    let t = token.trim();
    if t.is_empty() {
        return true;
    }
    let lower = t.to_lowercase();
    matches!(
        lower.as_str(),
        "课程" | "课程表" | "课表" | "节次" | "第" | "节" | "周" | "星期" | "周次" | "时间"
            | "地点" | "教师" | "老师"
    ) || t.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '~' || c == '、')
}

fn looks_like_location(t: &str) -> bool {
    ["楼", "室", "馆", "场", "中心", "校区", "教", "实验", "机房", "报告厅"]
        .iter()
        .any(|k| t.contains(k))
}

fn parse_text(text: &str, today: NaiveDate) -> Vec<Course> {
    let rules = TextRules::new();
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.chars().count() < 3 {
            continue;
        }
        let Some(day_cap) = rules.day.captures(line) else {
            continue;
        };
        let Some(day) = day_cap.get(1).and_then(|m| day_from_text(m.as_str())) else {
            continue;
        };

        let mut start_sec = 0u8;
        let mut end_sec = 0u8;
        if let Some(c) = rules.sec_range.captures(line) {
            start_sec = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            end_sec = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
        } else if let Some(c) = rules.sec_single.captures(line) {
            start_sec = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            end_sec = start_sec;
        }

        let mut start_time = String::new();
        let mut end_time = String::new();
        if let Some(c) = rules.time.captures(line) {
            let h1: u32 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let m1: u32 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let h2: u32 = c.get(3).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let m2: u32 = c.get(4).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            start_time = format!("{:02}:{:02}", h1, m1);
            end_time = format!("{:02}:{:02}", h2, m2);
            if start_sec == 0 {
                if let Some(t) = NaiveTime::from_hms_opt(h1, m1, 0) {
                    start_sec = section_for_start(t);
                }
            }
            if end_sec == 0 {
                if let Some(t) = NaiveTime::from_hms_opt(h2, m2, 0) {
                    end_sec = section_for_end(t);
                }
            }
        }

        let mut weeks = Vec::new();
        if let Some(c) = rules.week_range.captures(line) {
            let a: u8 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            let b: u8 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
            weeks = (a..=b.min(30)).collect();
        } else if let Some(c) = rules.week_single.captures(line) {
            if let Some(w) = c.get(1).and_then(|m| m.as_str().parse::<u8>().ok()) {
                weeks = vec![w];
            }
        }
        if line.contains("单周") || line.contains("（单）") || line.contains("(单)") {
            weeks.retain(|w| w % 2 == 1);
        } else if line.contains("双周") || line.contains("（双）") || line.contains("(双)") {
            weeks.retain(|w| w % 2 == 0);
        }

        let mut cleaned = line.to_string();
        for re in [
            &rules.time,
            &rules.week_range,
            &rules.week_single,
            &rules.sec_range,
            &rules.sec_single,
            &rules.day,
        ] {
            cleaned = re.replace_all(&cleaned, " ").to_string();
        }
        cleaned = cleaned
            .replace("单周", " ")
            .replace("双周", " ")
            .replace("（单）", " ")
            .replace("（双）", " ")
            .replace("(单)", " ")
            .replace("(双)", " ");

        let tokens: Vec<String> = cleaned
            .split(|c: char| {
                c.is_whitespace() || "|/,;；，、·".contains(c)
            })
            .map(|t| t.trim().trim_matches('：').trim().to_string())
            // 「1.5学分」「考查」这类字段不是教师名，混进 teacher 会让课表显示错。
            .filter(|t| !is_noise(t) && !t.contains("学分") && !t.contains("学时"))
            .collect();
        if tokens.is_empty() {
            continue;
        }

        let name = tokens[0].clone();
        let mut teacher = String::new();
        let mut location = String::new();
        for t in tokens.iter().skip(1) {
            let cleaned_t = t
                .trim_start_matches("教师:")
                .trim_start_matches("教师：")
                .trim_start_matches("老师:")
                .trim_start_matches("老师：");
            if looks_like_location(cleaned_t) && location.is_empty() {
                location = cleaned_t.to_string();
            } else if teacher.is_empty() {
                teacher = cleaned_t
                    .trim_end_matches("老师")
                    .trim_end_matches("教师")
                    .to_string();
            }
        }

        let course = Course {
            name,
            teacher,
            location,
            day,
            start_section: if start_sec == 0 { 1 } else { start_sec },
            end_section: if end_sec == 0 {
                if start_sec == 0 {
                    2
                } else {
                    start_sec
                }
            } else {
                end_sec
            },
            start_time,
            end_time,
            weeks,
        };
        out.push(course);
    }
    let _ = today;
    out
}

// ---------------------------------------------------------------------------
// 视图 JSON
// ---------------------------------------------------------------------------

impl Timetable {
    /// 给 Java 端展示：附带当前周和每条课程的索引。
    pub fn to_json(&self) -> String {
        let today = Local::now().date_naive();
        let week = current_week(&self.term_start, today, self.weeks);
        let courses: Vec<Value> = self
            .courses
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut v = serde_json::to_value(c).unwrap_or(Value::Null);
                if let Value::Object(ref mut m) = v {
                    m.insert("idx".to_string(), Value::from(i));
                }
                v
            })
            .collect();
        serde_json::json!({
            "term_start": self.term_start,
            "weeks": self.weeks,
            "current_week": week,
            "today": today.format("%Y-%m-%d").to_string(),
            "courses": courses,
        })
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parse_ics_weekly_events() {
        let ics = "\u{feff}BEGIN:VCALENDAR\r\n\
VERSION:2.0\r\n\
BEGIN:VEVENT\r\n\
SUMMARY:高等数学\r\n\
DTSTART;TZID=Asia/Shanghai:20260907T080000\r\n\
DTEND;TZID=Asia/Shanghai:20260907T094000\r\n\
LOCATION:教一101\r\n\
DESCRIPTION:教师：张三\r\n\
RRULE:FREQ=WEEKLY;COUNT=16\r\n\
END:VEVENT\r\n\
BEGIN:VEVENT\r\n\
SUMMARY:大学英语\r\n\
DTSTART;TZID=Asia/Shanghai:20260909T143000\r\n\
DTEND;TZID=Asia/Shanghai:20260909T161000\r\n\
LOCATION:外语楼302\r\n\
RRULE:FREQ=WEEKLY;UNTIL=20261220T000000\r\n\
END:VEVENT\r\n\
END:VCALENDAR\r\n";
        let (courses, term) = parse_ics(ics);
        assert_eq!(courses.len(), 2);
        assert_eq!(term, Some(date("2026-09-07")));
        let math = courses.iter().find(|c| c.name == "高等数学").unwrap();
        assert_eq!(math.day, 1);
        assert_eq!(math.start_section, 1);
        assert_eq!(math.end_section, 2);
        assert_eq!(math.teacher, "张三");
        assert_eq!(math.location, "教一101");
        assert_eq!(math.weeks.first(), Some(&1));
        assert!(math.weeks.contains(&16));
        let eng = courses.iter().find(|c| c.name == "大学英语").unwrap();
        assert_eq!(eng.day, 3);
        assert_eq!(eng.start_section, 5);
        assert!(eng.weeks.contains(&1));
    }

    #[test]
    fn parse_wakeup_json_schedule() {
        let json = r#"{
          "termStart": "2026-09-07",
          "schedule": [
            {"name":"线性代数","teacher":"王五","room":"教二201","dayOfWeek":3,
             "startSection":5,"endSection":6,"weekList":[1,2,3,4],"startTime":"14:00","endTime":"15:40"},
            {"courseName":"程序设计","teacher":"赵六","location":"机房A","day":5,
             "sections":"1-2","weeks":"1-16","startTime":"08:00","endTime":"09:40"}
          ]
        }"#;
        let (courses, term) = parse_json_schedule(json).unwrap();
        assert_eq!(courses.len(), 2);
        assert_eq!(term, Some(date("2026-09-07")));
        let alg = courses.iter().find(|c| c.name == "线性代数").unwrap();
        assert_eq!(alg.day, 3);
        assert_eq!(alg.start_section, 5);
        assert_eq!(alg.end_section, 6);
        assert_eq!(alg.weeks, vec![1, 2, 3, 4]);
        let prog = courses.iter().find(|c| c.name == "程序设计").unwrap();
        assert_eq!(prog.day, 5);
        assert_eq!(prog.weeks.len(), 16);
    }

    #[test]
    fn parse_csv_with_header() {
        let csv = "课程名称,教师,地点,星期,节次,周次\n\
大学英语,李四,外语楼302,星期一,3-4,1-16\n\
体育,王教练,体育馆,星期五,5-6,2-16\n";
        let courses = parse_csv(csv).unwrap();
        assert_eq!(courses.len(), 2);
        let eng = courses.iter().find(|c| c.name == "大学英语").unwrap();
        assert_eq!(eng.day, 1);
        assert_eq!(eng.start_section, 3);
        assert_eq!(eng.end_section, 4);
        assert_eq!(eng.location, "外语楼302");
        assert_eq!(eng.weeks.len(), 16);
        let pe = courses.iter().find(|c| c.name == "体育").unwrap();
        assert_eq!(pe.day, 5);
        assert_eq!(pe.weeks.first(), Some(&2));
    }

    #[test]
    fn parse_text_lines() {
        let lines = "高等数学 周一 1-2节 1-16周 教一101 张老师\n\
大学英语 星期三 第3-4节 第1-16周 外语楼302 李四\n\
体育 星期五 5-6节 2-16周(双) 体育馆\n";
        let courses = parse_text(lines, date("2026-09-07"));
        assert_eq!(courses.len(), 3);
        let math = &courses[0];
        assert_eq!(math.name, "高等数学");
        assert_eq!(math.day, 1);
        assert_eq!(math.start_section, 1);
        assert_eq!(math.end_section, 2);
        assert_eq!(math.location, "教一101");
        assert_eq!(math.teacher, "张");
        assert_eq!(math.weeks.len(), 16);

        let eng = &courses[1];
        assert_eq!(eng.day, 3);
        assert_eq!(eng.start_section, 3);
        assert_eq!(eng.end_section, 4);
        assert_eq!(eng.location, "外语楼302");
        assert_eq!(eng.teacher, "李四");

        let pe = &courses[2];
        assert_eq!(pe.day, 5);
        assert!(pe.weeks.iter().all(|w| w % 2 == 0));
        assert_eq!(pe.location, "体育馆");
    }

    #[test]
    fn ics_without_rrule_merges_weekly_events() {
        // 星课程表逐周导出：同一门课每周一个 VEVENT，没有 RRULE。
        let mut ics = String::from("BEGIN:VCALENDAR\r\n");
        for day in ["20260907", "20260914", "20260921"] {
            ics.push_str("BEGIN:VEVENT\r\n");
            ics.push_str("SUMMARY:高等数学\r\n");
            ics.push_str(&format!("DTSTART;TZID=Asia/Shanghai:{}T080000\r\n", day));
            ics.push_str(&format!("DTEND;TZID=Asia/Shanghai:{}T094000\r\n", day));
            ics.push_str("LOCATION:教一101\r\n");
            ics.push_str("END:VEVENT\r\n");
        }
        ics.push_str("END:VCALENDAR\r\n");

        let (courses, term) = parse_ics(&ics);
        assert_eq!(courses.len(), 1, "同一门课不应被拆成多门");
        assert_eq!(term, Some(date("2026-09-07")));
        assert_eq!(courses[0].weeks, vec![1, 2, 3]);
        assert_eq!(courses[0].day, 1);
    }

    #[test]
    fn weeks_text_accepts_discrete_list() {
        assert_eq!(weeks_from_text("1,3,5,7周"), vec![1, 3, 5, 7]);
        assert_eq!(weeks_from_text("1、3、5周"), vec![1, 3, 5]);
        assert_eq!(weeks_from_text("第1-4,6-8周"), vec![1, 2, 3, 4, 6, 7, 8]);
        assert_eq!(weeks_from_text("1-16周(单)").len(), 8);
        assert_eq!(weeks_from_text("2-16周").first(), Some(&2));
    }

    #[test]
    fn section_mapping_skips_lunch_gap() {
        let at = |h, m| NaiveTime::from_hms_opt(h, m, 0).unwrap();
        // 08:00 的课属于第 1 节；旧实现会因为 08:00 < 08:30 而保持 1，这条是防回归。
        assert_eq!(section_for_start(at(8, 0)), 1);
        assert_eq!(section_for_end(at(9, 40)), 2);
        // 14:00 的课必须落到下午第 1 节（第 5 节），不能算成线段 4。
        assert_eq!(section_for_start(at(14, 0)), 5);
        assert_eq!(section_for_end(at(15, 40)), 6);
        // 16:10 / 19:00 保持原映射。
        assert_eq!(section_for_start(at(16, 10)), 6);
        assert_eq!(section_for_start(at(19, 0)), 9);
        assert_eq!(section_for_end(at(20, 50)), 10);
    }

    #[test]
    fn import_keeps_existing_term_start() {
        let dir = std::env::temp_dir().join(format!(
            "lancai-tt-term-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = TimetableStore::new(dir.join("timetable.json"));

        store.set_term("2026-09-07", 18).unwrap();
        // 文本里没有学期信息：应沿用已有 term_start，而不是重置为当周周一。
        store
            .import("高等数学 周一 1-2节 1-16周 教一101 张老师", date("2026-11-11"))
            .unwrap();
        assert_eq!(store.load().term_start, "2026-09-07");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn json_week_number_falls_back_to_day() {
        // 有些导出把星期写成 `week`（3=周三），同时用 `weeks` 存周次。
        // 旧实现里 `week` 既在 DAY_KEYS 又在 WEEK_KEYS，容易解析失败后整门课被丢掉。
        let json = r#"[{"name":"大学物理","teacher":"钱七","room":"教三101",
                        "week":3,"sections":"5-6","weeks":"1-16"}]"#;
        let (courses, _) = parse_json_schedule(json).unwrap();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].day, 3);
        assert_eq!(courses[0].start_section, 5);
        assert_eq!(courses[0].weeks.len(), 16);
    }

    #[test]
    fn week_calculation() {
        let today = date("2026-09-23");
        assert_eq!(current_week("2026-09-07", today, 20), 3);
        assert_eq!(current_week("", today, 20), 1);
        assert_eq!(current_week("2026-09-09", date("2026-09-07"), 20), 1);
    }

    #[test]
    fn timetable_roundtrip_and_delete() {
        let dir = std::env::temp_dir().join(format!(
            "lancai-tt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let store = TimetableStore::new(dir.join("timetable.json"));
        let ok = store
            .import("高等数学 周一 1-2节 1-16周 教一101 张老师", date("2026-09-07"))
            .unwrap();
        assert_eq!(ok.courses, 1);
        let tt = store.load();
        assert_eq!(tt.courses.len(), 1);
        assert_eq!(tt.term_start, "2026-09-07");
        assert_eq!(tt.weeks, 16);

        let tt = store.set_term("2026-09-10", 18).unwrap();
        assert_eq!(tt.term_start, "2026-09-07");
        assert_eq!(tt.weeks, 18);

        let tt = store.delete_course(0).unwrap();
        assert!(tt.courses.is_empty());
        store.clear().unwrap();
        assert!(store.load().courses.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
