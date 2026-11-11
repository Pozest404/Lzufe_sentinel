//! Android JNI 桥。
//!
//! Java 侧 `com.lancai.sentinel.core.RustCore` 只是薄壳：所有分析、存储、
//! AI 总结与推送都在 Rust 完成。任何异常都返回 null / -1，由 Java 提示错误。

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jint, jlong, jstring};
use jni::JNIEnv;
use serde_json::json;

use crate::config::Config;
use crate::pipeline::Engine;
use crate::schedule::{self, ScheduleStore, Todo};
use crate::timetable::TimetableStore;

fn read(env: &mut JNIEnv, s: &JString) -> Option<String> {
    env.get_string(s).ok().map(|v| v.into())
}

fn new_string(env: &mut JNIEnv, s: String) -> jstring {
    match env.new_string(s) {
        Ok(v) => v.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn engine(cfg_json: &str, data_dir: &str) -> Option<Engine> {
    Engine::new(Config::from_json(cfg_json), PathBuf::from(data_dir)).ok()
}

fn result_or_null(env: &mut JNIEnv, out: Option<String>) -> jstring {
    match out {
        Some(s) => new_string(env, s),
        None => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    new_string(
        &mut env,
        format!(
            "Rust 核心 v{}（分析 / 存储 / AI / 推送 / 日程 / 课表）",
            env!("CARGO_PKG_VERSION")
        ),
    )
}

/// 分析文本，返回 MessageOutcome JSON。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeAnalyze(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
    data_dir: JString,
    source: JString,
    text: JString,
    persist: jboolean,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let cfg = read(&mut env, &cfg_json)?;
        let dir = read(&mut env, &data_dir)?;
        let src = read(&mut env, &source).unwrap_or_else(|| "手动输入".to_string());
        let body = read(&mut env, &text)?;
        let mut e = engine(&cfg, &dir)?;
        let outcome = e.analyze_text(&src, &body, persist != 0);
        serde_json::to_string(&outcome).ok()
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 返回最近消息 JSON 数组。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeRecent(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
    data_dir: JString,
    limit: jint,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let cfg = read(&mut env, &cfg_json)?;
        let dir = read(&mut env, &data_dir)?;
        let e = engine(&cfg, &dir)?;
        let n = if limit <= 0 { 20 } else { limit as usize };
        let msgs: Vec<_> = e.recent(n).iter().map(|m| m.to_json()).collect();
        serde_json::to_string(&msgs).ok()
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// AI 总结，返回 {"ok":bool,"text":...,"error":...}。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeAi(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
    data_dir: JString,
    text: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let cfg = read(&mut env, &cfg_json)?;
        let dir = read(&mut env, &data_dir)?;
        let body = read(&mut env, &text)?;
        let mut e = engine(&cfg, &dir)?;
        match e.ai_summary(&body) {
            Ok(Some(t)) => Some(json!({"ok": true, "text": t}).to_string()),
            Ok(None) => Some(
                json!({"ok": false, "text": "", "error": "AI 未启用或配置不完整"})
                    .to_string(),
            ),
            Err(err) => Some(
                json!({"ok": false, "text": "", "error": err.to_string()}).to_string(),
            ),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 推送，返回 {"ok":bool,"detail":...}。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativePush(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
    data_dir: JString,
    title: JString,
    content: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let cfg = read(&mut env, &cfg_json)?;
        let dir = read(&mut env, &data_dir)?;
        let t = read(&mut env, &title)?;
        let c = read(&mut env, &content)?;
        let e = engine(&cfg, &dir)?;
        match e.push_text(&t, &c) {
            Ok(o) => Some(json!({"ok": o.ok, "detail": o.detail}).to_string()),
            Err(err) => Some(
                json!({"ok": false, "detail": err.to_string()}).to_string(),
            ),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 清空本地消息，返回 1/0。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeClear(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
    data_dir: JString,
) -> jint {
    let ok = catch_unwind(AssertUnwindSafe(|| -> Option<bool> {
        let cfg = read(&mut env, &cfg_json)?;
        let dir = read(&mut env, &data_dir)?;
        let mut e = engine(&cfg, &dir)?;
        e.clear();
        Some(true)
    }))
    .ok()
    .flatten()
    .unwrap_or(false);
    if ok {
        1
    } else {
        0
    }
}

/// 规则自检：用 3 条样例返回可读文本。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeSelfTest(
    mut env: JNIEnv,
    _class: JClass,
    cfg_json: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let cfg = read(&mut env, &cfg_json)?;
        let cfg = Config::from_json(&cfg);
        let samples = crate::diagnostics::SAMPLE_TEXTS;
        let mut b = String::new();
        b.push_str(&format!("阈值：{} 分\n\n", cfg.analysis.threshold));
        for (i, (name, text)) in samples.iter().enumerate() {
            let m = crate::model::Msg::new("自检", "", *text);
            let s = crate::rule_engine::analyze(&m, &cfg.rules, &cfg.analysis.keyword_extra);
            b.push_str(&format!(
                "{}. {} → {} 分 · {}\n   {}\n",
                i + 1,
                name,
                s.score,
                if s.score >= cfg.analysis.threshold { "重要" } else { "普通" },
                s.title_line()
            ));
        }
        Some(b)
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

// ---------------------------------------------------------------------------
// 日程 / 待办 / 活动
// ---------------------------------------------------------------------------

fn schedule_store(dir: &str) -> ScheduleStore {
    ScheduleStore::new(PathBuf::from(dir).join("schedule.json"))
}

fn timetable_store(dir: &str) -> TimetableStore {
    TimetableStore::new(PathBuf::from(dir).join("timetable.json"))
}

/// 返回全部日程 JSON 数组（未完成在前，按时间排序）。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleList(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let items = schedule_store(&dir).load();
        serde_json::to_string(&items).ok()
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 新增或更新一条日程。`item_json` 可带 `due`（yyyy-MM-dd HH:mm）。
/// 返回 {"ok":bool,"item":{...}|"error":"..."}。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleSave(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    item_json: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let raw = read(&mut env, &item_json)?;
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
        let mut t: Todo = serde_json::from_value(v.clone()).unwrap_or_default();
        if let Some(due) = v.get("due").and_then(|x| x.as_str()) {
            match schedule::parse_due_ms(due) {
                Some(ms) => t.due_ms = ms,
                None => {
                    return Some(
                        json!({"ok": false, "error": "时间格式应为 yyyy-MM-dd HH:mm"})
                            .to_string(),
                    )
                }
            }
        }
        if t.title.trim().is_empty() {
            return Some(json!({"ok": false, "error": "标题不能为空"}).to_string());
        }
        match schedule_store(&dir).upsert(t) {
            Ok(saved) => Some(json!({"ok": true, "item": saved}).to_string()),
            Err(e) => Some(json!({"ok": false, "error": e.to_string()}).to_string()),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 删除一条日程，返回 1/0。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleDelete(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    id: jlong,
) -> jint {
    let ok = catch_unwind(AssertUnwindSafe(|| -> Option<bool> {
        let dir = read(&mut env, &data_dir)?;
        schedule_store(&dir).delete(id).ok()
    }))
    .ok()
    .flatten()
    .unwrap_or(false);
    if ok {
        1
    } else {
        0
    }
}

/// 勾选/取消完成，返回 1/0。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleToggle(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    id: jlong,
    done: jboolean,
) -> jint {
    let ok = catch_unwind(AssertUnwindSafe(|| -> Option<bool> {
        let dir = read(&mut env, &data_dir)?;
        schedule_store(&dir).set_done(id, done != 0).ok()
    }))
    .ok()
    .flatten()
    .unwrap_or(false);
    if ok {
        1
    } else {
        0
    }
}

/// 清除已完成，返回删除数量。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleClearDone(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
) -> jint {
    let n = catch_unwind(AssertUnwindSafe(|| -> Option<usize> {
        let dir = read(&mut env, &data_dir)?;
        schedule_store(&dir).clear_done().ok()
    }))
    .ok()
    .flatten()
    .unwrap_or(0);
    n as jint
}

/// 未来需要提醒的条目（Rust 计算触发时间，Java 只负责排 Alarm）。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeScheduleUpcoming(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    now_ms: jlong,
    horizon_ms: jlong,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let store = schedule_store(&dir);
        let mut list: Vec<serde_json::Value> = store
            .upcoming(now_ms, horizon_ms)
            .into_iter()
            .map(|(t, at)| {
                json!({
                    "id": t.id,
                    "title": t.title,
                    "note": t.note,
                    "kind": t.kind,
                    "url": t.url,
                    "due_ms": t.due_ms,
                    "trigger_ms": at,
                    "auto_signup": t.auto_signup,
                    "signup_at_ms": t.signup_at_ms,
                })
            })
            .collect();
        // 自动报名单独排期：Java 端按 signup=true 区分，打开带 AUTO_ENROLL 的报名页。
        for (t, at) in store.signups(now_ms, horizon_ms) {
            list.push(json!({
                "id": t.id,
                "title": t.title,
                "note": t.note,
                "kind": t.kind,
                "url": t.url,
                "due_ms": t.due_ms,
                "trigger_ms": at,
                "signup": true,
            }));
        }
        list.sort_by_key(|v| v.get("trigger_ms").and_then(|x| x.as_i64()).unwrap_or(0));
        serde_json::to_string(&list).ok()
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 判断一条通知/活动能否参加：返回 Assessment JSON。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeEnrollCheck(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    text: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let body = read(&mut env, &text)?;
        let tt = timetable_store(&dir).load();
        let now = chrono::Local::now().naive_local();
        serde_json::to_string(&crate::enroll::assess(&body, &tt, now)).ok()
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

// ---------------------------------------------------------------------------
// 课表
// ---------------------------------------------------------------------------

/// 课表视图：term_start / weeks / current_week / courses（含 idx）。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeTimetableGet(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        Some(timetable_store(&dir).load().to_json())
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 从 ICS/JSON/CSV/文本导入并覆盖课表。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeTimetableImport(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    text: JString,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let body = read(&mut env, &text)?;
        let today = chrono::Local::now().date_naive();
        match timetable_store(&dir).import(&body, today) {
            Ok(ok) => Some(
                json!({
                    "ok": true,
                    "count": ok.courses,
                    "format": ok.format,
                    "term_start": ok.term_start,
                })
                .to_string(),
            ),
            Err(e) => Some(json!({"ok": false, "error": e.to_string()}).to_string()),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 设置学期开始日期（自动对齐到周一）与总周数。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeTimetableSetTerm(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    term_start: JString,
    weeks: jint,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        let date = read(&mut env, &term_start)?;
        let w = if weeks <= 0 { 20 } else { weeks as u8 };
        match timetable_store(&dir).set_term(&date, w) {
            Ok(tt) => Some(
                json!({"ok": true, "term_start": tt.term_start, "weeks": tt.weeks})
                    .to_string(),
            ),
            Err(e) => Some(json!({"ok": false, "error": e.to_string()}).to_string()),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 删除一门课程（按导入后的索引）。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeTimetableDelete(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
    index: jint,
) -> jstring {
    let out = catch_unwind(AssertUnwindSafe(|| -> Option<String> {
        let dir = read(&mut env, &data_dir)?;
        match timetable_store(&dir).delete_course(index.max(0) as usize) {
            Ok(_) => Some(json!({"ok": true}).to_string()),
            Err(e) => Some(json!({"ok": false, "error": e.to_string()}).to_string()),
        }
    }))
    .ok()
    .flatten();
    result_or_null(&mut env, out)
}

/// 清空课表，返回 1/0。
#[no_mangle]
pub extern "system" fn Java_com_lancai_sentinel_core_RustCore_nativeTimetableClear(
    mut env: JNIEnv,
    _class: JClass,
    _cfg_json: JString,
    data_dir: JString,
) -> jint {
    let ok = catch_unwind(AssertUnwindSafe(|| -> Option<bool> {
        let dir = read(&mut env, &data_dir)?;
        timetable_store(&dir).clear().ok().map(|_| true)
    }))
    .ok()
    .flatten()
    .unwrap_or(false);
    if ok {
        1
    } else {
        0
    }
}
