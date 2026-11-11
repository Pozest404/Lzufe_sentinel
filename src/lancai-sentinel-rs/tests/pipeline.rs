use std::path::PathBuf;

use lancai_sentinel::config::Config;
use lancai_sentinel::pipeline::Engine;

fn tmpdir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "lancai-pipe-{}-{}-{}",
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

#[test]
fn important_text_is_scored_and_stored() {
    let dir = tmpdir("important");
    let mut e = Engine::new(Config::default(), dir.clone()).unwrap();
    let text = "【学分】本周五 19:00《人工智能前沿》讲座在图书馆报告厅举行，\
                可获 0.5 学分，限 80 人，11月11日 18:00 前群内报名";
    let o = e.analyze_text("同学转发", text, true);
    assert!(o.score >= 55, "score={}", o.score);
    assert!(o.important);
    assert!(!o.duplicate);
    assert_eq!(o.category, "讲座");
    assert!(!o.summary.title.is_empty());
    assert_eq!(e.count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn casual_text_is_not_important() {
    let dir = tmpdir("casual");
    let mut e = Engine::new(Config::default(), dir.clone()).unwrap();
    let o = e.analyze_text("手动输入", "哈哈哈哈今天食堂人好多", true);
    assert!(!o.important);
    assert!(o.score < 55);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn duplicate_text_is_not_stored_twice() {
    let dir = tmpdir("dup");
    let mut e = Engine::new(Config::default(), dir.clone()).unwrap();
    let text = "@所有人 明晚 19:00 教学楼 A301 开班会，请准时参加";
    let first = e.analyze_text("手动输入", text, true);
    assert!(!first.duplicate);
    let second = e.analyze_text("手动输入", text, true);
    assert!(second.duplicate);
    assert_eq!(e.count(), 1);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn recent_and_clear() {
    let dir = tmpdir("recent");
    let mut e = Engine::new(Config::default(), dir.clone()).unwrap();
    e.analyze_text("手动输入", "志愿活动：周六上午 8:30 校门口集合，名额 20 人，找团支书报名", true);
    e.analyze_text("同学转发", "明天下午三点在图书馆开会", true);
    assert_eq!(e.recent(10).len(), 2);
    e.clear();
    assert_eq!(e.count(), 0);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_network_config_still_works_locally() {
    let dir = tmpdir("offline");
    let mut e = Engine::new(Config::default(), dir.clone()).unwrap();
    let o = e.analyze_text("手动输入", "今晚八点抢名额，限 30 人，先到先得", true);
    assert!(o.push_ok.is_none(), "未配置推送时不应发起请求");
    assert!(o.ai_summary.is_empty());
    let _ = std::fs::remove_dir_all(dir);
}
