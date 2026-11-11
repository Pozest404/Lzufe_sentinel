# 配置说明（config.toml）

本项目的可调行为集中在 `config.toml`，**改完保存即可生效，不需要重新编译**。
CLI 每次启动都会重新读取配置；Android 宿主每次分析都会用 App 内设置重建配置。

v3.0 起只处理「用户主动粘贴或分享的文本」，已删除通知监听、群名匹配、自动报名相关配置。

推荐排查顺序：

```bash
cargo run -- config check      # 1. 先看配置有没有硬伤
cargo run -- selftest          # 2. 用 3 条典型文本验证规则引擎
cargo run -- analyze --text "【学分】周五 19:00 讲座，0.5 学分，限 80 人"
```

---

## 1. 加载顺序与优先级

1. 内置默认值（`src/config.rs` 中的 `Default` 实现）；
2. `config.toml`（路径可用 `--config <文件>` 或 `-c` 指定，默认当前目录的 `config.toml`）；
3. 环境变量（优先级最高，见第 7 节）。

配置中**未知字段会直接报错**（`deny_unknown_fields`），拼错字段名时启动即失败，不会静默忽略——这正是为了方便调试。

```bash
cargo run -- config init          # 生成模板，已存在则拒绝覆盖
cargo run -- config show          # 查看生效配置（密钥自动遮蔽）
cargo run -- config paths         # 打印配置、数据目录、消息库路径
```

---

## 2. `[app]` 运行环境

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `data_dir` | `"data"` | 数据目录。相对路径以 `config.toml` 所在目录为基准；也可写绝对路径。 |
| `log_level` | `"info"` | `trace/debug/info/warn/error`，预留；CLI 目前直接写 stdout/stderr。 |
| `timezone` | `"Asia/Shanghai"` | 仅用于展示与时间解析，不影响存储的 Unix 毫秒时间戳。 |

调试时建议给回放单独建目录，避免污染正式数据：

```bash
mkdir -p /tmp/lancai-debug
cp config.toml /tmp/lancai-debug/
cargo run -- --config /tmp/lancai-debug/config.toml analyze --text "周五讲座"
```

---

## 3. `[analysis]` 分析参数（最常用）

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `threshold` | `55` | 重要线，10~100。评分 ≥ 阈值即标记「重要」。越低越灵敏。 |
| `keyword_extra` | `""` | 你所在学院/社团的追加关键词，逗号或空白分隔；每个命中 +8 分、封顶 +16。 |

Android 端的「菜单 → 分析与规则 → 规则设置」对应这两个字段。

---

## 4. `[rules]` 打分权重

一般不需要改。想整体调灵敏/保守时改 `[analysis].threshold` 即可。

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `score_baseline` | `8` | 任何文本的底分 |
| `keyword_first` | `12` | 命中第一个关键词的加分 |
| `keyword_step` | `6` | 之后每个关键词的递增分 |
| `keyword_cap` | `30` | 关键词加分上限 |
| `custom_keyword_bonus` | `8` | 每个追加关键词加分 |
| `custom_keyword_cap` | `16` | 追加关键词加分上限 |
| `neg_credit_penalty` | `30` | 「没有学分」等否定扣分 |
| `neg_signup_penalty` | `25` | 「不需要报名」等否定扣分 |
| `neg_cancel_penalty` | `30` | 「活动取消」扣分 |
| `reschedule_penalty` | `10` | 「改期」扣分 |
| `penalty_cap` | `60` | 总扣分上限 |
| `credit_bonus` | `22` | 抽到学分/志愿时长加分 |
| `quota_bonus` | `10` | 抽到名额加分 |
| `deadline_bonus` | `10` | 抽到截止时间加分 |
| `how_bonus` | `15` | 抽到报名方式加分 |
| `when_bonus` | `8` | 抽到时间加分 |
| `where_bonus` | `5` | 抽到地点加分 |
| `urgent_bonus` | `18` | 「今晚/马上/先到先得」等加分 |
| `attention_cap` | `20` | 关注度加分上限 |

追加词表（空数组表示只用内置词表）：

| 字段 | 说明 |
| --- | --- |
| `keywords_append` | 额外关键词，例如 `["素拓", "到梦空间"]` |
| `neg_credit_append` | 额外否定词 |
| `neg_signup_append` | 额外「无需报名」类词 |
| `neg_cancel_append` | 额外「取消」类词 |
| `reschedule_append` | 额外「改期」类词 |
| `urgent_append` | 额外「紧急」类词 |

---

## 5. `[ai]` AI 总结（可选，支持多模型）

兼容任何 OpenAI Chat Completions 协议的接口；换模型只需改 `base_url` 与 `model`。

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `enabled` | `false` | 总开关。关闭时不会发任何 AI 请求。 |
| `api_key` | `""` | API Key；本地 Ollama / 免密代理可留空。建议用环境变量。 |
| `model` | `"deepseek-chat"` | 模型名 |
| `base_url` | `https://api.deepseek.com/chat/completions` | 接口地址；只填到 `/v1` 时会自动补 `/chat/completions` |
| `timeout_seconds` | `20` | 请求超时 |
| `max_tokens` | `400` | 最大输出长度 |
| `temperature` | `0.1` | 采样温度 |
| `dedupe_minutes` | `10` | 相同文本在该时间窗内复用上次总结 |

常见服务商：

| 服务商 | `base_url` | `model` 示例 |
| --- | --- | --- |
| DeepSeek | `https://api.deepseek.com` | `deepseek-chat` |
| OpenAI | `https://api.openai.com/v1` | `gpt-4o-mini` |
| 通义千问 | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `qwen-plus` |
| 智谱 GLM | `https://open.bigmodel.cn/api/paas/v4` | `glm-4-flash` |
| Kimi | `https://api.moonshot.cn/v1` | `moonshot-v1-8k` |
| 本地 Ollama | `http://192.168.x.x:11434/v1` | `qwen2.5:7b` |

只有消息被判定为「重要」且 `enabled = true` 时才会自动总结；错误不会阻塞本地分析。
请求失败时可在 App 的「测试 AI 总结」里看到 HTTP 状态码、服务商错误信息与排查提示。

---

## 6. `[pushplus]` 微信推送（可选）

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `enabled` | `false` | 总开关 |
| `token` | `""` | PushPlus Token。建议用环境变量 `LANCAI_PUSHPLUS_TOKEN`。 |
| `url` | `https://www.pushplus.plus/send` | 接口地址 |
| `max_tries` | `3` | 失败重试次数 |
| `retry_gap_ms` | `2000` | 重试间隔（毫秒） |

测试：`cargo run -- push-test`。

---

## 7. `[storage]` 与 `[debug]`

`[storage]`：

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `msgs_file` | `"msgs.jsonl"` | 最近消息文件（JSONL） |
| `max_messages` | `300` | 超过后滚动删除最旧的（10~5000） |
| `last_summary_file` | `"last_summary.json"` | 最近一次摘要 |

`[debug]`：

| 字段 | 默认值 | 说明 |
| --- | --- | --- |
| `dump_summary` | `false` | 打印字段抽取细节 |
| `dry_run` | `false` | `true` = 跳过所有网络请求 |

环境变量：

| 变量 | 作用 |
| --- | --- |
| `LANCAI_AI_KEY` | 覆盖 `ai.api_key`（兼容旧名 `LANCAI_DEEPSEEK_KEY`） |
| `LANCAI_AI_BASE_URL` | 覆盖 `ai.base_url` |
| `LANCAI_AI_MODEL` | 覆盖 `ai.model` |
| `LANCAI_PUSHPLUS_TOKEN` | 覆盖 `pushplus.token` |
| `LANCAI_LOG_LEVEL` | 覆盖 `app.log_level` |
| `LANCAI_DRY_RUN=1` | 等价于 `debug.dry_run = true` |

---

## 8. Android 端配置

APK 内没有 `config.toml`。Java 薄壳把你填的设置拼成 JSON 传给 Rust，字段形状与 TOML 相同：

```json
{
  "analysis": { "threshold": 55, "keyword_extra": "素拓,到梦空间" },
  "ai":       { "enabled": true, "api_key": "sk-...", "model": "deepseek-chat",
                "base_url": "https://api.deepseek.com/chat/completions" },
  "pushplus": { "enabled": true, "token": "..." }
}
```

对应界面：首页右上角「菜单 → 分析与规则 / 推送与 AI」。改完立即生效，无需重新打包。
AI 设置页提供 DeepSeek / OpenAI / 通义千问 / 智谱 GLM / Kimi 预设，也可选「自定义 / 本地」手填 Base URL 与模型名。

第二课堂入口：首页右上角「第二课堂」按钮用系统浏览器打开，默认 `https://win.9xueqi.com/#/home`；
可在「菜单 → 推送与 AI → 设置」的「校园服务」里改成学校自己的网址。只做跳转，不自动报名、不注入页面。
