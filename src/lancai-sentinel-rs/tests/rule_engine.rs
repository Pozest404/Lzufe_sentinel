use lancai_sentinel::config::RulesConfig;
use lancai_sentinel::model::Msg;
use lancai_sentinel::rule_engine::{analyze, organizer_of};

fn analyze_text(text: &str) -> lancai_sentinel::model::Summary {
    analyze(&Msg::new("测试班级群", "同学", text), &RulesConfig::default(), "")
}

fn contains(name: &str, actual: &str, expect: &str) {
    assert!(
        actual.contains(expect),
        "{}: actual=[{}] expect contains [{}]",
        name,
        actual,
        expect
    );
}

fn empty(name: &str, actual: &str) {
    assert!(actual.trim().is_empty(), "{}: actual=[{}] expect empty", name, actual);
}

#[test]
fn case1_full_activity() {
    let s = analyze_text(
        "【班级通知】本周五 18:30 在第三教学楼302举办《人工智能前沿讲座》，0.5 学分，限 80 人，11月11日 18:00 前扫码报名，先到先得！",
    );
    contains("when", &s.when, "本周五");
    contains("when.clock", &s.when, "18:30");
    contains("where", &s.where_, "第三教学楼");
    contains("where.room", &s.where_, "302");
    contains("credit", &s.credit, "0.5");
    contains("credit.unit", &s.credit, "学分");
    contains("quota", &s.quota, "80");
    contains("deadline", &s.deadline, "11月11日");
    contains("deadline.clock", &s.deadline, "18:00");
    contains("how", &s.how, "扫码");
    contains("title", &s.title, "人工智能前沿讲座");
    assert_eq!(s.category, "讲座");
    assert!((80..=100).contains(&s.score), "score={}", s.score);
    assert!(s.title.chars().count() <= 40);
}

#[test]
fn case2_multi_field() {
    let s = analyze_text("明天下午 2 点，图书馆报告厅，二课 0.3 分，群里接龙报名，名额 30，先到先得");
    contains("when", &s.when, "明天");
    contains("when.clock", &s.when, "2 点");
    contains("where", &s.where_, "图书馆报告厅");
    contains("credit", &s.credit, "二课");
    contains("credit.num", &s.credit, "0.3");
    contains("quota", &s.quota, "30");
    contains("how", &s.how, "接龙");
    assert_eq!(s.category, "学分");
    assert!((70..=100).contains(&s.score), "score={}", s.score);
}

#[test]
fn case3_online_volunteer() {
    let s = analyze_text("2026-11-12 19:00 腾讯会议，志愿时长 4 小时，报名截止 11月10日 20:00，请尽快报名");
    contains("when", &s.when, "2026-11-12");
    contains("when.clock", &s.when, "19:00");
    contains("where", &s.where_, "腾讯会议");
    contains("credit", &s.credit, "志愿时长");
    contains("credit.num", &s.credit, "4");
    contains("deadline", &s.deadline, "11月10日");
    assert_eq!(s.category, "志愿");
    assert!((80..=100).contains(&s.score), "score={}", s.score);
}

#[test]
fn case4_chatter() {
    let s = analyze_text("明天开班会，收到回复一下");
    contains("when", &s.when, "明天");
    assert!(s.score <= 54, "score={}", s.score);
    assert!(!s.title.is_empty());
}

#[test]
fn case5_negation() {
    let s = analyze_text("讲座已取消，无需报名，没有学分");
    assert!(s.score <= 20, "score={}", s.score);
    empty("credit", &s.credit);
    empty("deadline", &s.deadline);
    empty("how", &s.how);
    empty("quota", &s.quota);
}

#[test]
fn case6_notice_title() {
    let s = analyze_text("【重要通知】图书馆闭馆通知\n本周日全天闭馆，请同学们注意");
    contains("title", &s.title, "闭馆");
    assert_eq!(s.category, "通知");
    assert!(s.score <= 54, "score={}", s.score);
}

#[test]
fn case7_quoted_title() {
    let s = analyze_text("请报名《数学建模竞赛》，组队参加");
    assert_eq!(s.title, "数学建模竞赛");
}

#[test]
fn case8_noise_date_and_credit() {
    let s = analyze_text("11-12号楼集合，记得签到");
    empty("when", &s.when);

    let s2 = analyze_text("本次活动加 0.5 学分");
    empty("when2", &s2.when);
    contains("credit", &s2.credit, "0.5");
}

#[test]
fn case9_deadline_only() {
    let s = analyze_text("今晚 24 点截止报名，要去的私聊我");
    contains("deadline", &s.deadline, "今晚");
    empty("when", &s.when);
}

#[test]
fn case10_credit_only() {
    let s = analyze_text("0.5学分，签到即可获得");
    contains("credit", &s.credit, "0.5");
    contains("credit.unit", &s.credit, "学分");
}

#[test]
fn organizer_cases() {
    assert_eq!(
        organizer_of("主办单位：校团委，承办单位：计算机学院学生会"),
        "校团委"
    );
    contains(
        "org2",
        &organizer_of("本次活动由计算机学院学生会主办"),
        "计算机学院",
    );
    empty("org3", &organizer_of("明天下午开会"));
}

#[test]
fn pipeline_samples() {
    struct Case(&'static str, bool, &'static str, &'static [&'static str]);
    let cases = [
        Case("【学分】本周五 19:00《人工智能前沿》讲座在图书馆报告厅举行，可获 0.5 学分，限 80 人，11月11日 18:00 前在群里接龙报名", true, "讲座", &["credit","quota","when","where","deadline","how"]),
        Case("志愿活动：本周六上午 8:30 校门口集合，去敬老院服务，志愿时长 4 小时，名额 20 人，先到先得", true, "志愿", &["credit","quota","when","where"]),
        Case("第二课堂活动：到梦空间报名，完成签到可获素拓分 0.3 分，活动地点：大学生活动中心 302", true, "学分", &["credit","where"]),
        Case("腾讯会议线上讲座：今晚 20:00《考研经验分享》，会议号 123-456-789，无需报名直接参加", false, "讲座", &["when","where"]),
        Case("@所有人 明天下午 2 点在教学楼 A301 开班会，请全体同学准时参加", false, "通知", &["when","where"]),
        Case("提醒：本次实践活动没有学分，也不需要报名，收到请回复", false, "通知", &[]),
        Case("【抢名额】今晚 21:00 开放报名，限 30 人，名额有限先到先得，链接：https://example.edu/signup", true, "活动", &["quota"]),
        Case("群接龙报名：周末敬老院志愿服务，接龙格式 姓名+学号，志愿服务时长 3 小时", true, "志愿", &["credit"]),
        Case("哈哈哈笑死我了，中午吃啥", false, "", &[]),
        Case("[图片]", false, "", &[]),
        Case("【通知】原定本周五的讲座因故取消，具体时间另行通知", false, "通知", &[]),
        Case("重要通知：11月20日 14:00 在体育馆举办秋季招聘会，现场投递简历，凭学生证入场", false, "活动", &["when","where"]),
    ];
    for (i, c) in cases.iter().enumerate() {
        let s = analyze_text(c.0);
        let hit = s.score >= 55;
        assert_eq!(hit, c.1, "#{} 提醒判断: score={} cat={} {:?}", i + 1, s.score, s.category, s);
        if !c.2.is_empty() {
            assert_eq!(s.category, c.2, "#{} 分类: {:?}", i + 1, s);
        }
        for f in c.3 {
            let v = match *f {
                "credit" => &s.credit,
                "quota" => &s.quota,
                "when" => &s.when,
                "where" => &s.where_,
                "deadline" => &s.deadline,
                "how" => &s.how,
                "title" => &s.title,
                _ => unreachable!(),
            };
            assert!(!v.trim().is_empty(), "#{} 字段空: {} -> {:?}", i + 1, f, s);
        }
    }
}
