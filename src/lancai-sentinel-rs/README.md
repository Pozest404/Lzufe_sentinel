# 澜彩哨兵 · Rust 核心

手动分析工具的核心：把你粘贴或分享进来的一段文本，做规则打分、分类与字段抽取（时间/地点/学分/名额/截止/报名方式），并负责本地存储、DeepSeek 总结与 PushPlus 推送。

v3.0 起已移除微信通知监听、群名匹配、保活与 H5 注入。所有输入都由用户主动提供；v3.3 的自动报名只在用户于日程里逐条确认后启用，Rust 只负责判断能否参加与计算触发时间。

- 配置逐字段说明：[CONFIG.md](CONFIG.md)
- 核心库 `src/`
- 调试样例 `samples/messages.txt`

---

## 1. 模块映射

| Rust 模块 | 职责 |
| --- | --- |
| [config.rs](src/config.rs) | 全部配置、默认值、环境变量覆盖、密钥遮蔽；TOML 与 Android JSON 双入口 |
| [model.rs](src/model.rs) | `Msg`、`Summary` 与 JSON 字段形状 |
| [text.rs](src/text.rs) | 清洗、截断、关键词边界判断 |
| [rule_engine.rs](src/rule_engine.rs) | 计分、否定词、分类、时间/地点/学分/名额/截止/报名抽取、主办方 |
| [store.rs](src/store.rs) | JSONL 最近 300 条 + 同文 3 秒去重 |
| [schedule.rs](src/schedule.rs) | 日程/待办/活动 JSON 存储、截止时间解析、提醒触发时间 |
| [timetable.rs](src/timetable.rs) | 课表存储、ICS/JSON/CSV/文本解析、学期周次计算 |
| [enroll.rs](src/enroll.rs) | 从通知/日程提取活动时间，与课表比较判断能否参加、解析报名截止 |
| [net.rs](src/net.rs) | PushPlus 推送、DeepSeek 总结、重试、代理、dry-run |
| [pipeline.rs](src/pipeline.rs) | 分析 → 存储 → 重要消息 AI 总结 → 推送 |
| [diagnostics.rs](src/diagnostics.rs) | 配置体检 + 3 条样例自检 |
| [android.rs](src/android.rs) | JNI 导出，供 Android 薄壳调用 |
| [main.rs](src/main.rs) | `lancai` 调试 CLI |

---

## 2. 构建与测试

需要 Rust 1.75+（2021 edition）。

```bash
cargo build            # 生成 target/debug/lancai 与 liblancai_sentinel
cargo test             # 36 个回归测试
cargo run -- --help    # 查看全部命令
```

测试覆盖：

| 文件 | 内容 |
| --- | --- |
| `tests/rule_engine.rs` | 12 条典型文本的字段抽取、分类、否定词、主办方、阈值判断 |
| `tests/data_layer.rs` | 消息库持久化/重载、摘要 JSON 形状 |
| `tests/pipeline.rs` | 重要/普通/重复、最近记录与清空、无网络配置 |
| `src/schedule.rs` | 日程增删改查、完成状态、提醒窗口与排序、截止时间格式、自动报名排期 |
| `src/timetable.rs` | ICS 周重复、WakeUp JSON、CSV、文本课表、周次计算、导入去重 |
| `src/enroll.rs` | 时间段/周几/日期解析、课程冲突、截止时间、跨周解析 |

---

## 3. CLI 用法

全局参数：`--config <文件>`（默认 `config.toml`）、`--json`、`--dry-run`。

```bash
# 配置
cargo run -- config check          # 配置体检
cargo run -- config show           # 查看生效配置（密钥遮蔽）
cargo run -- config init           # 生成带注释模板
cargo run -- config paths          # 打印配置与数据文件路径

# 规则调试
cargo run -- selftest                                  # 3 条样例自检
cargo run -- analyze --text "【学分】周五 19:00 讲座，0.5 学分，限 80 人"

# 外部服务
cargo run -- push-test --title 测试 --content 内容
cargo run -- ai-test --text "本周五 19:00 图书馆讲座，0.5 学分"
```

`analyze` 会把结果写入本地消息库（`--no-store` 可关闭），达到阈值时按配置自动总结并推送。

---

## 4. Android 宿主如何接入

Rust 库导出为 `rlib` + `cdylib`，Android 侧通过 JNI 调用。边界很薄：

| Java 侧 | Rust 侧 |
| --- | --- |
| `MainActivity` 取粘贴/分享文本 | `nativeAnalyze(cfgJson, dataDir, source, text, persist)` |
| 最近记录列表 | `nativeRecent(cfgJson, dataDir, limit)` |
| 设置页保存阈值/密钥 | 拼成 JSON 传入，Rust 每次重建配置 |
| 测试推送 / 测试 AI | `nativePush(...)` / `nativeAi(...)` |
| 版本与内核状态 | `nativeVersion()` |

最小调用示例：

```rust
use lancai_sentinel::config::Config;
use lancai_sentinel::pipeline::Engine;

let (cfg, _exists) = Config::load_or_default(std::path::Path::new("config.toml"))?;
let data_dir = cfg.data_dir(std::path::Path::new("config.toml"));
let mut engine = Engine::new(cfg, data_dir)?;
let outcome = engine.analyze_text("手动输入", "周五 19:00 讲座，0.5 学分", true);
```

`MessageOutcome` 返回：是否重复、分数、分类、标题、是否重要、AI 摘要、推送结果与可读摘要。Java 层只消费这个结构，不重复实现业务判断。

---

## 5. 关键行为

- **打分**：基础分 + 关键词分 + 硬信息加分 + 关注度加分 − 否定词惩罚，限制 0~100；阈值默认 55。
- **字段抽取**：时间、地点、学分/志愿时长、名额、截止、报名方式；带否定词时清空对应字段。
- **去重**：消息库同文 3 秒内不重复入库。
- **AI**：仅重要消息触发；相同文本 `dedupe_minutes` 内复用；失败不影响主流程。
- **推送**：仅重要消息触发；可配置重试；失败只记录结果，不抛异常。
- **失败隔离**：网络、AI、推送任一失败都不会中断本地分析与存储。

---

## 6. 目录结构

```
lancai-sentinel-rs/
├── Cargo.toml
├── config.toml          # 带注释的完整配置（可直接改）
├── CONFIG.md            # 配置逐字段说明与调参手册
├── README.md
├── samples/messages.txt # 典型文本样例
├── src/                 # 核心库 + CLI
└── tests/               # 回归用例
```
