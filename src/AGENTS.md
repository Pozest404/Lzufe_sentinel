# AGENTS.md · 澜彩哨兵

给 AI 编码代理（Kilo / Cursor / Claude Code 等）的项目约定。先读本文件再改代码。

## 项目是什么

手动分析工具：用户主动粘贴或从微信/浏览器「分享」一段文本 → Rust 规则引擎打分抽取 → 本地存储 / AI 总结 / PushPlus 推送。

v3.0 起已彻底移除：微信通知监听、NotificationListenerService、无障碍/保活服务。不要再加回来。
v3.1 增加 OpenAI 兼容多模型配置，以及「第二课堂」入口。
v3.2 把第二课堂改为应用内 WebView，新增日程/待办/活动、课表导入（星课程表 .ics / JSON / CSV / 文本）与系统通知提醒；仅新增本机提醒用的 `ReminderReceiver` / `BootReceiver`，仍然不监听任何外部通知。
v3.3 新增「能否参加」判断与自动报名：只在用户在日程里逐条确认后启用，到点先发通知，再由用户点击打开报名页并自动填写；不做后台静默报名、不监听通知。

- `lancai-sentinel-rs/`：Rust 核心库 + 调试 CLI（36 个测试）
- `lancai-sentinel-android/`：Android 薄壳（Java + res + manifest，无 Gradle）+ 单包构建脚本
- 详细构建、模块映射、UI 结构见 `lancai-sentinel-android/AGENTS.md`

## 语言与性能硬性要求（最重要）

1. **之后的业务代码必须建立在 Rust 等速度优先的语言上。** 新增的分析、抽取、存储、网络、AI 调用一律写在 Rust。
2. **Java 只做薄壳**：Activity/View、SharedPreferences、Intent、JNI 调用。禁止在 Java 里写文本解析、打分、字段抽取、网络请求、消息存储、AI 请求。
3. 性能目标：单条文本分析在 release 构建的中端设备上 < 5 ms；批量/回放不出现 O(n²)。
4. Rust release 配置必须保持：`opt-level = "z"`、`lto = true`、`codegen-units = 1`、`strip = "symbols"`。新增依赖前先评估编译后体积与启动开销。
5. 新功能顺序：Rust 模块 + 单元测试 → `src/android.rs` 暴露 JNI → `RustCore.java` 包装 → UI 调用。

## 常用命令（在仓库根执行）

```bash
cd lancai-sentinel-rs
cargo test                                   # 必须全绿，当前 36 个
cargo run -- config check                    # 配置体检
cargo run -- selftest                        # 3 条样例自检
cargo run -- analyze --text "周五 19:00 讲座，0.5 学分，限 80 人"

cd ../lancai-sentinel-android
./build.sh                                   # 出包：dist/lancai-sentinel-v<版本>.apk
VER_NAME=3.1.1 VER_CODE=33 ./build.sh        # 递增版本后发版
```

## 硬性约束

1. 不要修改 `/workspace/.uploads` 下任何文件。
2. 不要动 `lancai-sentinel-android/keys/lancai.jks`；不要把密钥、Token、API Key 写进代码或提交到公开仓库。
3. 不要引入 Gradle、AndroidX 或第三方 Java 依赖；构建链路固定为 aapt2 + javac + d8 + zipalign + apksigner。
4. 发版必须递增 `VER_CODE`，并沿用同一签名密钥，否则无法覆盖安装。
5. 不要重新引入通知监听、保活服务；自动报名只能由用户在日程里逐条确认，触发时必须先发通知、由用户点击进入报名页，禁止后台静默提交。提醒只允许本机 `ReminderReceiver` / `BootReceiver`。
6. 新增业务逻辑只能落在 Rust 侧（见上一节）。
7. 不要改包名 `com.lancai.sentinel`，否则无法覆盖升级。

## 改动前先确认边界

| 层 | 位置 | 说明 |
| --- | --- | --- |
| Rust 业务核心 | `lancai-sentinel-rs/src/` | 规则、文本工具、存储、AI/推送、流水线；改完必须 `cargo test` |
| JNI 桥 | `src/android.rs` + `RustCore.java` | 跨语言边界，改动风险最高，两端签名必须一致 |
| Java 薄壳 | `app/java/.../core`（Config/RustCore）、`.../ui` | 只做配置存取、界面与 JNI 调用 |
| 构建 | `build.sh`、`app/AndroidManifest.xml` | 版本号强制写入、双 ABI 交叉编译（arm64-v8a / armeabi-v7a）、签名 |

## 验收清单（提交前逐项确认）

- [ ] `cargo test` 全部通过（36 个）
- [ ] `./build.sh` 输出 `BUILD_OK`
- [ ] `aapt2 dump badging` 的 versionCode/versionName 正确
- [ ] 清单中没有 service / NotificationListener；只允许 ReminderReceiver / BootReceiver
- [ ] 新增逻辑都在 Rust；Java 未新增业务处理
- [ ] 涉及 UI 的改动已在设备或模拟器上实际点过（无设备时必须在回复中说明未验证）
- [ ] diff 只包含任务相关文件，没有格式化无关代码

## 给免费模型路由的协作规则

- 一次只做一个明确小任务；超过 2-3 个文件的改动先拆分。
- 回复中给出改动的文件路径和验证命令输出，不要只说“已完成”。
- 不确定 Android API 行为时先查代码/文档，不要猜。
- 遇到构建环境缺失（SDK/NDK/JDK17/Rust target）直接说明，不要伪造构建成功。
- 免费模型可能被用于训练：不要粘贴密钥、签名文件、真实用户文本。
