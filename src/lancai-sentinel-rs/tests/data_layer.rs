use std::path::PathBuf;

use lancai_sentinel::model::{Msg, Summary};
use lancai_sentinel::store::MsgStore;

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "lancai-{}-{}-{}",
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

fn msg(group: &str, sender: &str, text: &str, time: i64) -> Msg {
    Msg {
        id: time,
        time,
        group: group.to_string(),
        sender: sender.to_string(),
        text: text.to_string(),
        source: "manual".to_string(),
    }
}

#[test]
fn msg_store_dedup_recent_and_reload() {
    let dir = tmpdir("store");
    let path = dir.join("msgs.jsonl");
    let last = dir.join("last_summary.json");
    let mut s = MsgStore::new(&path, &last, 300);
    s.clear();
    assert_eq!(s.count(), 0);

    let t0 = 1_760_000_000_000i64;
    assert!(s.add(&msg("班级群", "张三", "【学分】周五讲座 0.5 学分", t0)));
    assert!(!s.add(&msg("班级群", "张三", "【学分】周五讲座 0.5 学分", t0 + 1200)));
    assert!(s.add(&msg("2024级3班", "李四", "志愿活动招募 4 小时", t0 + 5000)));
    assert!(s.add(&msg("班级群", "张三", "【学分】周五讲座 0.5 学分", t0 + 20000)));
    assert_eq!(s.count(), 3);

    let recent = s.recent(2);
    assert_eq!(recent.len(), 2);
    assert_eq!(recent[0].text, "【学分】周五讲座 0.5 学分");
    assert_eq!(recent[1].text, "志愿活动招募 4 小时");

    let mut summary = Summary {
        category: "学分".to_string(),
        title: "《人工智能前沿》讲座".to_string(),
        when: "本周五 19:00".to_string(),
        where_: "图书馆报告厅".to_string(),
        credit: "0.5 学分".to_string(),
        quota: "限 80 人".to_string(),
        deadline: "11月11日 18:00 前".to_string(),
        how: "群接龙".to_string(),
        score: 96,
        ai_summary: "一句话：可拿 0.5 学分，今晚截止报名。".to_string(),
        ..Default::default()
    };
    summary.msg = None;
    s.set_last_summary(&summary).unwrap();
    let back = s.get_last_summary().unwrap();
    assert_eq!(back.category, "学分");
    assert_eq!(back.title, "《人工智能前沿》讲座");
    assert_eq!(back.where_, "图书馆报告厅");
    assert_eq!(back.score, 96);
    assert_eq!(back.ai_summary, summary.ai_summary);

    // 模拟重启
    let s2 = MsgStore::new(&path, &last, 300);
    assert_eq!(s2.count(), 3);
    assert_eq!(s2.recent(1)[0].text, "【学分】周五讲座 0.5 学分");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn summary_json_shape_matches_java() {
    let s = Summary {
        category: "学分".to_string(),
        title: "讲座".to_string(),
        when: "今晚".to_string(),
        where_: "图书馆".to_string(),
        ..Default::default()
    };
    let v = s.to_json();
    assert_eq!(v["when"], "今晚");
    assert_eq!(v["where"], "图书馆");
    assert!(v.get("aiSummary").is_some());
    assert!(v.get("msg").is_none());
}
