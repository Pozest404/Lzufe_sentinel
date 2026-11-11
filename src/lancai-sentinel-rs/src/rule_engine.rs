//! 规则引擎：关键词打分 + 结构化字段抽取。
//!
//! 忠实移植 Java 版 `monitor/RuleEngine.java`，并保留其设计约束：
//! 1) 关键词分封顶，避免"堆词"把闲聊刷成重要提醒；
//! 2) 只有学分/名额/截止/报名方式/时间/地点这类硬信息才给大额分；
//! 3) 否定词（无需报名/没有学分/已取消）必须真的降分并清空报名字段；
//! 4) 所有正则都有界，避免回溯爆炸。

use std::sync::LazyLock;

use regex::Regex;

use crate::config::RulesConfig;
use crate::model::{Msg, Summary};
use crate::text;

pub const SCORE_IMPORTANT: i32 = 60;
pub const SCORE_URGENT: i32 = 80;

pub const KEYWORDS: &[&str] = &[
    "学分", "素拓", "素质拓展", "二课", "第二课堂", "综测", "加分", "学时", "志愿时长",
    "志愿", "志愿者", "公益", "招募", "义工",
    "讲座", "报告会", "宣讲", "学术报告", "论坛", "沙龙", "分享会", "培训",
    "报名", "接龙", "抢", "名额", "截止", "先到先得", "审核", "录取",
    "签到", "打卡", "活动", "比赛", "竞赛", "双创", "pu", "到梦空间",
];

const NEG_CREDIT: &[&str] = &[
    "没有学分", "无学分", "不加学分", "不计学分", "不给学分", "不算学分", "不认定学分", "非学分",
];
const NEG_SIGNUP: &[&str] = &[
    "无需报名", "不用报名", "不需报名", "不需要报名", "无需提前报名", "不接受报名", "不报名",
];
const NEG_CANCEL: &[&str] = &[
    "已取消", "活动取消", "取消举办", "已结束", "已过期", "名额已满", "报名结束", "已截止",
    "因故取消", "讲座取消", "会议取消", "培训取消", "比赛取消", "暂停举办",
    "取消本次", "本次取消", "取消通知", "活动暂缓",
];
const NEG_RESCHEDULE: &[&str] = &["改期", "延期", "时间调整", "推迟"];
const URGENT: &[&str] = &[
    "抢", "先到先得", "名额有限", "今晚", "明早", "马上", "手慢无", "报满即止", "额满即止", "速来",
];

const GENERIC_TITLES: &[&str] = &[
    "收到请回复", "收到", "好的", "谢谢", "通知", "公告", "提醒", "重要通知", "活动通知",
    "报名通知", "温馨提示", "转发", "名单", "安排", "重要提醒", "学分", "活动", "讲座", "报名",
];

const LEAD_MARKERS: &[&str] = &[
    "[有人@我]", "【有人@我】", "[@所有人]", "【@所有人】", "@所有人", "@全体成员",
    "有人@我", "【群公告】", "[群公告]",
];

const DATE_ANY: &str = r"(?:20\d{2}\s*[-/.年]\s*\d{1,2}\s*[-/.月]\s*\d{1,2}\s*[日号]?|\d{1,2}\s*月\s*\d{1,2}\s*[日号]?|\d{1,2}\s*[/\-]\s*\d{1,2}|(?:今|明|后|大后)天|(?:今|明)晚|(?:本|这|下|上)?(?:周|星期)[一二三四五六日天])";
const TIME_ANY: &str = r"(?:(?:上午|下午|晚上|中午|早上|清晨|傍晚|晚间|夜里|夜间|晚)\s*)?(?:\d{1,2}\s*[:：]\s*\d{2}|\d{1,2}\s*点\s*(?:半|一刻|三刻|\d{1,2}\s*分)?|\d{1,2}\s*时(?:\d{1,2}\s*分)?)";

fn date_time_any() -> &'static str {
    static PAT: LazyLock<String> =
        LazyLock::new(|| format!("{}(?:[\\s，,、（(的]{{0,4}}{})?", DATE_ANY, TIME_ANY));
    &PAT
}

static P_TITLE_BOOK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"《([^》\n]{2,40})》").unwrap());
static P_TITLE_QUOTE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[“"「『]([^”"」』\n]{2,40})[”"」』]"#).unwrap());
static P_TITLE_BRACKET: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"【([^】\n]{2,40})】").unwrap());

static P_WHEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!("({})", date_time_any())).expect("invalid P_WHEN")
});
static P_CLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(TIME_ANY).expect("invalid P_CLOCK"));
static P_DEADLINE_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:截止|截至|报名截止|截止时间|截止日期|最晚|最后期限)[\s:：]{0,3}([^\n。；;！!？?，,、]{2,26})")
        .expect("invalid P_DEADLINE_LABEL")
});
static P_DEADLINE_SUFFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        "({})\\s*(之前|以前|前|截止|截至)",
        date_time_any()
    ))
    .expect("invalid P_DEADLINE_SUFFIX")
});
static P_QUOTA_LIMIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:仅?限|限额|限报|招募|招|上限)\s*(?:前)?\s*(\d{1,4})\s*(?:人|名|位|个\s*名额)")
        .unwrap()
});
static P_QUOTA_WORD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{1,4})\s*(?:个)?\s*名额").unwrap());
static P_QUOTA_LABEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"名额[\s:：为是有约]{0,3}(\d{1,4})\s*(?:人|名|位|个)?").unwrap());
static P_QUOTA_FIRST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"前\s*(\d{1,4})\s*(?:名|人|位)").unwrap());

static P_CREDIT_NUM_UNIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(\d{1,3}(?:\.\d{1,2})?)\s*(?:个|分)?\s*(学分|素拓分|素质拓展分|二课分|第二课堂分|综测分|第二课堂学分|志愿时长|志愿时|服务时长|公益时长)",
    )
    .unwrap()
});
static P_CREDIT_UNIT_NUM: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(学分|素拓分|素质拓展分|二课分|第二课堂分|综测分|学时|志愿时长|志愿时|服务时长)\s*[:：为是有]{0,3}\s*(\d{1,3}(?:\.\d{1,2})?)",
    )
    .unwrap()
});
static P_CREDIT_CTX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(学分|素拓|素质拓展|二课|第二课堂|综测|志愿)[^\n。；;，,]{0,6}?(\d{1,3}(?:\.\d{1,2})?)\s*分")
        .unwrap()
});
static P_CREDIT_HOURS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(志愿|公益|服务|实践|素拓|二课)[^\n。；;，,]{0,10}?(\d{1,3}(?:\.\d{1,2})?)\s*(?:个)?\s*(?:小时|h|H|学时)")
        .unwrap()
});

static P_WHERE_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:地点|地址|位置|场地|活动地点|会议地点|举办地点|集合地点|签到地点|会议地址)\s*[:：]?\s*([^\n。；;！!？?，,、]{2,30})",
    )
    .unwrap()
});
static P_WHERE_ONLINE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(腾讯会议|钉钉|飞书|Zoom|zoom|ZOOM|雨课堂|学习通|腾讯课堂|线上直播|线上|直播间)")
        .unwrap()
});
static P_WHERE_AT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:在|于|到|地点[是为在]?)\s*([\u{4e00}-\u{9fa5}A-Za-z0-9（）()]{2,16}?(?:教学楼|实验楼|实训楼|教室|报告厅|大礼堂|礼堂|图书馆|体育馆|体育场|操场|田径场|会议室|活动室|中心|广场|校区|楼|馆|厅|室)\s*(?:[A-Za-z]?\d{1,4}\s*(?:室|教室)?)?)",
    )
    .unwrap()
});
static P_WHERE_BARE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"([\u{4e00}-\u{9fa5}A-Za-z0-9]{2,14}?(?:教学楼|实验楼|实训楼|报告厅|大礼堂|图书馆|体育馆|体育场|操场|田径场|会议室|活动室))",
    )
    .unwrap()
});
static P_WHERE_SPOT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(学校门口|校门口|正门口?|大门口?|南门|北门|东门|西门|食堂门口|食堂|宿舍楼下|宿舍区|敬老院|社区服务中心|社区|公园|集合点|集合地点)",
    )
    .unwrap()
});
static P_URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://[^\s，,。；;、）)】]{4,140}").unwrap());
static P_ORG_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:主办(?:方|单位)?|承办(?:方|单位)?|举办(?:方|单位)?|组织(?:方|单位)?|协办(?:方|单位)?)\s*[:：]?\s*([^\n。；;！!？?，,、]{2,24})",
    )
    .unwrap()
});
static P_ORG_WORD: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([\u{4e00}-\u{9fa5}]{2,10}(?:学院|学部|系|团委|团总支|学生会|研究生会|学生社团|协会|中心|办公室|工作部|委员会))")
        .unwrap()
});

/// 规则分析入口。永不 panic（正则均为编译期固定模式）。
pub fn analyze(m: &Msg, rules: &RulesConfig, keyword_extra: &str) -> Summary {
    let mut s = Summary {
        msg: Some(m.clone()),
        ..Default::default()
    };
    let text = m.text.as_str();
    fill(&mut s, text, rules, keyword_extra);
    if s.title.trim().is_empty() {
        s.title = "群消息".to_string();
    }
    s
}

fn fill(s: &mut Summary, text: &str, rules: &RulesConfig, keyword_extra: &str) {
    let t: String = text.chars().take(3000).collect();
    let lower = t.to_lowercase();

    let mut score = rules.score_baseline;
    score += keyword_score(&lower, rules);

    // 用户追加关键词：每个 +8，封顶 +16。
    let mut extra = 0;
    for k in text::split_words(keyword_extra) {
        if text::char_len(&k) >= 2 && text::contains_keyword(&lower, &k.to_lowercase()) {
            extra += rules.custom_keyword_bonus;
        }
    }
    score += extra.min(rules.custom_keyword_cap);

    let no_credit = contains_any_extra(&t, NEG_CREDIT, &rules.neg_credit_append);
    let no_signup = contains_any_extra(&t, NEG_SIGNUP, &rules.neg_signup_append);
    let cancelled = contains_any_extra(&t, NEG_CANCEL, &rules.neg_cancel_append);
    let rescheduled = contains_any_extra(&t, NEG_RESCHEDULE, &rules.reschedule_append);

    let mut minus = 0;
    if no_credit {
        minus += rules.neg_credit_penalty;
    }
    if no_signup {
        minus += rules.neg_signup_penalty;
    }
    if cancelled {
        minus += rules.neg_cancel_penalty;
    } else if rescheduled {
        minus += rules.reschedule_penalty;
    }
    score -= minus.min(rules.penalty_cap);

    s.credit = if no_credit { String::new() } else { extract_credit(&t) };
    s.quota = if cancelled || no_signup {
        String::new()
    } else {
        extract_quota(&t)
    };
    s.deadline = if cancelled || no_signup {
        String::new()
    } else {
        extract_deadline(&t)
    };
    s.when = extract_when(&t, &s.deadline);
    s.where_ = extract_where(&t);
    s.how = if cancelled || no_signup {
        String::new()
    } else {
        extract_how(&t)
    };

    if !s.credit.is_empty() {
        score += rules.credit_bonus;
    }
    if !s.quota.is_empty() {
        score += rules.quota_bonus;
    }
    if !s.deadline.is_empty() {
        score += rules.deadline_bonus;
    }
    if !s.how.is_empty() {
        score += rules.how_bonus;
    } else if !cancelled && !no_signup && text::contains_any(&t, &["报名", "接龙", "报名链接"]) {
        score += rules.how_bonus;
    }
    if !s.when.is_empty() {
        score += rules.when_bonus;
    }
    if !s.where_.is_empty() {
        score += rules.where_bonus;
    }
    if !cancelled && !no_signup && contains_any_extra(&t, URGENT, &rules.urgent_append) {
        score += rules.urgent_bonus;
    }

    let mut attention = 0;
    if text::contains_any(&t, &["@所有人", "@全体成员"]) {
        attention += 10;
    }
    if text::contains_any(&t, &["[有人@我]", "@我"]) {
        attention += 12;
    }
    if text::contains_any(&t, &["收到请回复", "收到回复", "请回复"]) {
        attention += 6;
    }
    if text::contains_any(&t, &["请尽快", "尽快"]) {
        attention += 4;
    }
    if text::contains_any(&t, &["重要通知", "重要提醒", "紧急通知"]) {
        attention += 8;
    }
    score += attention.min(rules.attention_cap);

    s.score = score.clamp(0, 100);
    s.title = extract_title(&t);
    s.category = category_of(&t, s);

    if no_credit && s.credit.is_empty() && s.category == "学分" {
        s.category = "通知".to_string();
    }
    if cancelled && s.score < SCORE_IMPORTANT {
        s.category = "通知".to_string();
    } else if no_signup
        && s.score < SCORE_IMPORTANT
        && s.category != "志愿"
        && s.category != "讲座"
    {
        s.category = "通知".to_string();
    }
}

fn contains_any_extra(t: &str, base: &[&str], extra: &[String]) -> bool {
    text::contains_any(t, base)
        || extra
            .iter()
            .any(|k| !k.is_empty() && t.contains(k.as_str()))
}

/// 关键词计分：首个 +12，之后每个 +6，封顶 30；被更长命中词包含的短词只算一次。
fn keyword_score(lower: &str, rules: &RulesConfig) -> i32 {
    let mut hits: Vec<String> = Vec::new();
    for k in KEYWORDS {
        if text::char_len(k) < 2 {
            continue;
        }
        if text::contains_keyword(lower, &k.to_lowercase()) {
            hits.push((*k).to_string());
        }
    }
    for k in &rules.keywords_append {
        if text::char_len(k) < 2 {
            continue;
        }
        if text::contains_keyword(lower, &k.to_lowercase()) {
            hits.push(k.clone());
        }
    }
    let mut distinct = 0;
    for (i, a) in hits.iter().enumerate() {
        let subsumed = hits.iter().enumerate().any(|(j, b)| {
            i != j && text::char_len(b) > text::char_len(a) && b.contains(a.as_str())
        });
        if !subsumed {
            distinct += 1;
        }
    }
    if distinct <= 0 {
        return 0;
    }
    (rules.keyword_first + (distinct - 1) * rules.keyword_step).min(rules.keyword_cap)
}

fn category_of(t: &str, s: &Summary) -> String {
    if text::contains_any(t, &["志愿", "志愿者", "公益", "志愿时长", "服务时长", "义工"]) {
        return "志愿".to_string();
    }
    if text::contains_any(t, &["讲座", "报告会", "宣讲", "学术报告", "论坛", "沙龙", "分享会"]) {
        return "讲座".to_string();
    }
    if !s.credit.trim().is_empty()
        || text::contains_any(t, &["学分", "素拓", "二课", "第二课堂", "综测", "加分", "学时"])
    {
        return "学分".to_string();
    }
    if text::contains_any(
        t,
        &[
            "活动", "比赛", "竞赛", "培训", "招募", "报名", "接龙", "招聘会", "宣讲会", "双选会",
            "交流会", "见面会", "开放日",
        ],
    ) {
        return "活动".to_string();
    }
    if text::contains_any(t, &["通知", "公告", "提醒", "会议", "班会", "安排"]) {
        return "通知".to_string();
    }
    "其他".to_string()
}

fn extract_title(t: &str) -> String {
    if let Some(c) = P_TITLE_BOOK.captures(t) {
        let v = text::clean_title(&c[1]);
        if text::char_len(&v) >= 2 {
            return text::clip(&v, 40);
        }
    }
    for c in P_TITLE_QUOTE.captures_iter(t) {
        let v = text::clean_title(&c[1]);
        if text::char_len(&v) >= 4 && !is_generic_title(&v) {
            return text::clip(&v, 40);
        }
    }
    for c in P_TITLE_BRACKET.captures_iter(t) {
        let v = text::clean_title(&c[1]);
        if text::char_len(&v) >= 4 && !is_generic_title(&v) {
            return text::clip(&v, 40);
        }
    }
    for line in t.split('\n') {
        let v = text::clean_title(line);
        if v.is_empty() {
            continue;
        }
        let v = strip_leading_markers(&v);
        let v = strip_leading_label(&v);
        let v = text::clean_title(&v);
        if v.is_empty() {
            continue;
        }
        return text::clip(&v, 40);
    }
    "群消息".to_string()
}

fn strip_leading_markers(s: &str) -> String {
    let mut out = s.trim().to_string();
    loop {
        let mut changed = false;
        for mk in LEAD_MARKERS {
            if out.starts_with(mk) {
                out = out[mk.len()..].trim().to_string();
                changed = true;
            }
        }
        if !changed {
            return out;
        }
    }
}

fn strip_leading_label(s: &str) -> String {
    if s.starts_with('【') {
        if let Some(end) = s.find('】') {
            let inner = &s['【'.len_utf8()..end];
            let label_len = text::char_len(inner);
            if end > '【'.len_utf8() && label_len <= 12 {
                let label = inner.trim();
                if is_generic_title(label)
                    || label.ends_with("通知")
                    || label.ends_with("公告")
                    || label.ends_with("提醒")
                    || label.ends_with("须知")
                {
                    return s[end + '】'.len_utf8()..].trim().to_string();
                }
            }
        }
    }
    if s.starts_with('[') {
        if let Some(end) = s.find(']') {
            let inner = &s[1..end];
            let label_len = text::char_len(inner);
            if end > 1 && label_len <= 12 {
                let label = inner.trim();
                if is_generic_title(label) || label.ends_with("通知") || label.ends_with("公告") {
                    return s[end + 1..].trim().to_string();
                }
            }
        }
    }
    s.to_string()
}

fn is_generic_title(v: &str) -> bool {
    if GENERIC_TITLES.iter().any(|g| *g == v) {
        return true;
    }
    text::char_len(v) <= 6
        && (v.ends_with("通知")
            || v.ends_with("公告")
            || v.ends_with("提醒")
            || v.ends_with("须知"))
}

fn clean_deadline_or_when(raw: &str) -> String {
    let v = raw.trim();
    let v = v.trim_start_matches(|c: char| {
        matches!(c, '为' | '是' | '到' | '于' | '在' | '：' | ':' | ' ' | '\t')
    });
    v.trim_end_matches(|c: char| matches!(c, '，' | ',' | '、' | '。' | '；' | ';' | ' '))
        .to_string()
}

fn looks_like_deadline_value(v: &str) -> bool {
    if v.is_empty() {
        return false;
    }
    if text::contains_any(v, &["今天", "明天", "后天", "今晚", "明晚", "周", "星期", "号", "月", "点", "时", "前"])
    {
        return true;
    }
    v.chars().any(|c| c.is_ascii_digit())
}

fn extract_deadline(t: &str) -> String {
    for c in P_DEADLINE_LABEL.captures_iter(t) {
        let v = clean_deadline_or_when(&c[1]);
        if text::char_len(&v) >= 2 && looks_like_deadline_value(&v) {
            return text::clip(&v, 24);
        }
    }
    for c in P_DEADLINE_SUFFIX.captures_iter(t) {
        let end = c.get(0).map(|m| m.end()).unwrap_or(0);
        if t[end..].chars().next() == Some('往') {
            continue;
        }
        let raw = format!("{} {}", &c[1], &c[2]);
        let v = clean_deadline_or_when(&raw);
        if text::char_len(&v) >= 2 {
            return text::clip(&v, 24);
        }
    }
    String::new()
}

fn extract_when(t: &str, deadline: &str) -> String {
    for c in P_WHEN.captures_iter(t) {
        let m = c.get(0).unwrap();
        let cand = clean_deadline_or_when(&c[1]);
        if cand.is_empty() {
            continue;
        }
        if is_bad_date_context(t, m.start(), m.end()) {
            continue;
        }
        if text::loose_contains(deadline, &cand) {
            continue;
        }
        return cand;
    }
    for m in P_CLOCK.find_iter(t) {
        let cand = m.as_str().trim();
        if cand.is_empty() {
            continue;
        }
        if is_inside_url(t, m.start()) {
            continue;
        }
        if text::loose_contains(deadline, cand) {
            continue;
        }
        return cand.to_string();
    }
    String::new()
}

fn is_bad_date_context(t: &str, start: usize, end: usize) -> bool {
    if let Some(next) = t[end..].chars().next() {
        if matches!(
            next,
            '号' | '栋' | '室' | '门' | '人' | '名' | '元' | '块' | '%' | '％' | '届' | '级'
                | '个' | '点' | '时' | '分' | '秒' | '天'
        ) {
            return true;
        }
    }
    t[..start].chars().next_back().map(|c| c.is_ascii_digit()).unwrap_or(false)
}

fn is_inside_url(t: &str, start: usize) -> bool {
    let mut idx = start.saturating_sub(24);
    while idx > 0 && !t.is_char_boundary(idx) {
        idx -= 1;
    }
    let before = &t[idx..start];
    before.contains("http") || before.contains("://")
}

fn extract_quota(t: &str) -> String {
    for c in P_QUOTA_LIMIT.captures_iter(t) {
        if let Ok(n) = c[1].parse::<i32>() {
            if n > 0 && n <= 2000 {
                return format!("限 {} 人", n);
            }
        }
    }
    for c in P_QUOTA_WORD.captures_iter(t) {
        if let Ok(n) = c[1].parse::<i32>() {
            if n > 0 && n <= 2000 {
                return format!("{} 个名额", n);
            }
        }
    }
    for c in P_QUOTA_LABEL.captures_iter(t) {
        if let Ok(n) = c[1].parse::<i32>() {
            if n > 0 && n <= 2000 {
                return format!("限 {} 人", n);
            }
        }
    }
    for c in P_QUOTA_FIRST.captures_iter(t) {
        if let Ok(n) = c[1].parse::<i32>() {
            if n > 0 && n <= 2000 {
                return format!("前 {} 名", n);
            }
        }
    }
    String::new()
}

fn extract_credit(t: &str) -> String {
    if let Some(c) = P_CREDIT_NUM_UNIT.captures(t) {
        let num = &c[1];
        let unit = &c[2];
        if is_reasonable_num(num, 0.1, 50.0) {
            return fmt_credit(num, unit);
        }
    }
    if let Some(c) = P_CREDIT_UNIT_NUM.captures(t) {
        let unit = &c[1];
        let num = &c[2];
        if is_reasonable_num(num, 0.1, 50.0) {
            return fmt_credit(num, unit);
        }
    }
    if let Some(c) = P_CREDIT_CTX.captures(t) {
        let ctx = &c[1];
        let num = &c[2];
        if is_reasonable_num(num, 0.1, 50.0) {
            return format!("{} {} 分", ctx, num);
        }
    }
    if let Some(c) = P_CREDIT_HOURS.captures(t) {
        let ctx = &c[1];
        let num = &c[2];
        if is_reasonable_num(num, 0.5, 100.0) {
            if text::contains_any(ctx, &["志愿", "公益", "服务", "实践"]) {
                return format!("志愿时长 {} 小时", num);
            }
            return format!("{} 小时", num);
        }
    }
    String::new()
}

fn fmt_credit(num: &str, unit: &str) -> String {
    if text::contains_any(unit, &["志愿", "服务", "公益", "实践"]) {
        return format!("{} {} 小时", unit, num);
    }
    if unit == "学时" {
        return format!("{} 学时", num);
    }
    format!("{} {}", num, unit)
}

fn is_reasonable_num(num: &str, min: f64, max: f64) -> bool {
    num.parse::<f64>()
        .map(|d| d >= min && d <= max)
        .unwrap_or(false)
}

fn extract_where(t: &str) -> String {
    if let Some(c) = P_WHERE_LABEL.captures(t) {
        let v = clean_where(&c[1]);
        if text::char_len(&v) >= 2 {
            return v;
        }
    }
    if let Some(c) = P_WHERE_ONLINE.captures(t) {
        return c[1].to_string();
    }
    if let Some(c) = P_WHERE_AT.captures(t) {
        let v = clean_where(&c[1]);
        if text::char_len(&v) >= 2 {
            return v;
        }
    }
    if let Some(c) = P_WHERE_BARE.captures(t) {
        let v = clean_where(&c[1]);
        if text::char_len(&v) >= 2 {
            return v;
        }
    }
    if let Some(c) = P_WHERE_SPOT.captures(t) {
        let v = clean_where(&c[1]);
        if text::char_len(&v) >= 2 {
            return v;
        }
    }
    String::new()
}

fn clean_where(raw: &str) -> String {
    let v = raw.trim();
    let v = v.trim_start_matches(|c: char| matches!(c, '在' | '于' | '是' | '到' | '：' | ':' | ' '));
    let mut v = v
        .trim_end_matches(|c: char| matches!(c, '，' | ',' | '、' | '。' | '；' | ';' | '！' | '!' | '？' | '?' | ' '))
        .to_string();
    for suffix in ["举行", "开展", "进行", "集合", "签到"] {
        if v.ends_with(suffix) {
            v.truncate(v.len() - suffix.len());
            break;
        }
    }
    text::clip(&v, 30)
}

fn extract_how(t: &str) -> String {
    if !text::contains_any(
        t,
        &["报名", "接龙", "扫码", "预约", "填表", "填问卷", "收集表", "报名表"],
    ) {
        return String::new();
    }
    if t.contains("接龙") {
        return "群接龙".to_string();
    }
    if text::contains_any(t, &["到梦空间", "到梦"]) {
        return "到梦空间".to_string();
    }
    if text::contains_keyword(&t.to_lowercase(), "pu") {
        return "PU 平台".to_string();
    }
    if text::contains_any(t, &["扫码报名", "扫描二维码", "扫二维码", "扫码进群", "扫码"]) {
        return "扫码报名".to_string();
    }
    if text::contains_any(t, &["公众号", "微信推文"]) {
        return "公众号报名".to_string();
    }
    if text::contains_any(t, &["问卷", "填表", "收集表", "腾讯文档", "金山文档", "报名表"]) {
        return "填写报名表单".to_string();
    }
    if text::contains_any(t, &["qq群", "QQ群", "Qq群", "进群", "加群"]) {
        return "进群报名".to_string();
    }
    if P_URL.is_match(t) {
        return "报名链接".to_string();
    }
    "报名".to_string()
}

/// 主办方抽取（public 工具方法）。
pub fn organizer_of(text: &str) -> String {
    let t: String = text.chars().take(3000).collect();
    extract_organizer(&t)
}

fn extract_organizer(t: &str) -> String {
    for c in P_ORG_LABEL.captures_iter(t) {
        let v = clean_org(&c[1]);
        if v.is_empty() {
            continue;
        }
        let label = c.get(0).map(|m| m.as_str()).unwrap_or("");
        if label.contains('方') || label.contains("单位") || looks_like_org(&v) {
            return v;
        }
    }
    if text::contains_any(t, &["主办", "承办", "举办", "组织"]) {
        if let Some(c) = P_ORG_WORD.captures(t) {
            return clean_org(&c[1]);
        }
    }
    String::new()
}

fn clean_org(raw: &str) -> String {
    let v = raw.trim();
    let v = v.trim_start_matches(|c: char| matches!(c, '：' | ':' | ' '));
    let v = v
        .trim_end_matches(|c: char| matches!(c, '，' | ',' | '、' | '。' | '；' | ';' | '！' | '!' | '？' | '?' | ' '));
    if v.starts_with('《') || v.starts_with('“') || v.starts_with('"') {
        return String::new();
    }
    text::clip(v, 24)
}

fn looks_like_org(v: &str) -> bool {
    [
        "学院", "学部", "系", "团委", "团总支", "学生会", "研究生会", "协会", "中心", "办公室",
        "工作部", "委员会", "处", "部",
    ]
    .iter()
    .any(|s| v.ends_with(s))
}
