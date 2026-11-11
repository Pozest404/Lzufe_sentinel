# AGENTS.md · 澜彩哨兵 Android 工程

本目录是与 Rust 核心一起打包的 Android 薄壳。仓库根目录还有一份精简版 `../AGENTS.md`，两份内容冲突时以本文件为准。

## 目录与职责

```
app/
├── AndroidManifest.xml          版本号、Activity/提醒 Receiver 注册、联网与通知权限
├── java/com/lancai/sentinel/
│   ├── core/Config.java         SharedPreferences 配置 → JSON
│   ├── core/RustCore.java       JNI 薄封装（加载 liblancai_sentinel.so）
│   ├── core/NotificationHelper.java  通知渠道、权限、提醒卡片
│   ├── core/ReminderScheduler.java   AlarmManager 排期（触发时间由 Rust 算）
│   ├── core/ReminderReceiver.java    到点显示通知
│   ├── core/BootReceiver.java        开机/更新后恢复排期
│   └── ui/                      首页、设置、二级菜单、第二课堂、日程、课表、UiKit
├── res/                         图标、颜色、字符串、主题
└── build.sh                     一键构建（Rust 双 ABI + Java + 打包签名）
```

Rust 工程在 `../lancai-sentinel-rs/`：规则引擎、字段抽取、消息库、AI（OpenAI 兼容多模型）、推送、流水线、日程存储、课表解析、能否参加判断、36 个测试和调试 CLI。

**v3.0 起没有 `monitor/`、`keepalive/`、`net/` 这些 Java 包。** 通知监听与保活已彻底删除，不要在 Java 侧重写。v3.2 只新增了本机提醒用的两个 Receiver，不读取任何其他应用的通知。v3.3 的自动报名必须由用户在日程里逐条确认：到点只发通知，用户点击后才进入报名页自动填写。

## 语言分工（硬性）

- 业务逻辑（分析、打分、抽取、存储、HTTP、AI）**全部在 Rust**。
- Java 只允许：Activity/View 构建、SharedPreferences 读写、Intent/分享接收、调用 `RustCore`。
- 需要新能力时：先加 Rust 模块与测试 → 加 `src/android.rs` 的 JNI 函数 → `RustCore.java` 同步方法 → UI 调用。

## 构建

```bash
cd ../lancai-sentinel-rs && cargo test      # 36 个必须全绿
cd ../lancai-sentinel-android && ./build.sh
```

依赖：Android SDK 34（build-tools 34.0.0、platform android-34、NDK 26.3.11579264）、JDK 17、Rust 两个 Android target（aarch64 / armv7）。

可用环境变量：`ANDROID_SDK_ROOT`、`ANDROID_NDK_HOME`、`LANCAI_JAVA_HOME`、`VER_NAME`、`VER_CODE`。

发版：

```bash
VER_NAME=3.1.1 VER_CODE=33 ./build.sh
```

`build.sh` 会用临时清单强制写入 `VER_*`，因此不需要手改 `AndroidManifest.xml`；但清单里的默认值也应同步更新，避免直接打开工程时版本对不上。

## 发布规则（build.sh 自动执行，勿绕过）

出包后按顺序自动完成，任何一步失败都不影响本地 APK：

1. **GoFile 同步** — `release-server/gofile-sync.py`，按 SHA256 增量上传，未变化则复用链接。`GOFILE_AUTO=0` 关闭。
2. **GitHub Release** — `release-server/gh-release.sh`，tag 为 `v<VER_NAME>`，上传 APK 与源码包；tag 已存在时 `--clobber` 覆盖。`GH_RELEASE=0` 关闭。
3. **保留策略** — 只留最近 `KEEP_VERSIONS` 个版本（默认 2）的 APK 与源码包：
   - `dist/` 由 `release-server/retention.py` 清理；
   - GitHub 上多余的 release 连同 tag 一起删除；
   - `gofile-links.md` 只展示这 2 版，完整历史仍归档在 `.gofile-sync.json`。

发版必须递增 `VER_CODE`。根目录不再存放发布包，产物统一以 `dist/` 为准。

产物：`dist/lancai-sentinel-v<VER_NAME>.apk`，同时把当前 Android + Rust 源码打进 `assets/lancai-sentinel-src-v<VER_NAME>.zip`，并附带 `assets/SOURCE_INFO.txt`。

## UI 结构（v3.3）

- 首页 `MainActivity`：右上角「第二课堂」（应用内 `DektActivity`，WebView 加载用户配置网址，右上角可刷新/交系统浏览器）与「菜单」；快捷入口「日程待办」「本周课表」；来源输入（可选）、多行文本、分析/清空；结果卡（重要/普通/重复、分数、分类、字段、能否参加、AI 总结、复制/AI/推送）；最近 10 条（点击回填）。支持 `ACTION_SEND` + `text/plain` 分享自动分析。
- 第二课堂 `DektActivity`：`WebView` + 进度条 + 刷新/浏览器；返回键优先网页后退；只允许 http/https 留在应用内，其他 scheme 交给系统。普通打开不注入页面；仅当 Intent 带 `EXTRA_AUTO_ENROLL=true`（用户在日程里确认过自动报名）时，加载完成后按字段名填写姓名/学号/手机号并点击含「报名」的按钮。
- 日程与待办 `ScheduleActivity`：列表、完成勾选、新建/编辑/删除、清除已完成；活动条目可带链接，点击直达应用内第二课堂，另有「一键报名」；保存带链接的活动时先用 Rust 判断能否参加，再弹窗询问「自动报名 / 仅提醒 / 取消」。
- 课表 `TimetableActivity`：星课程表风格（顶部渐变周次胶囊、今天按钮、日期表头、左侧节次时间、跨节圆角课程块、今天高亮）；学期设置、删除课程；导入支持 `.ics` / JSON / CSV / 文本，可从文件选择或剪贴板粘贴。
- 二级菜单 `MenuActivity`：可折叠分组
  - 校园工具：第二课堂、日程与待办、课表
  - 分析与规则：规则设置、规则自检、清空最近记录
  - 推送与 AI：推送与 AI 设置、测试推送、测试 AI 总结
  - 关于：版本与内核
- 设置 `SettingsActivity`：单页，重要线、追加关键词、AI 开关/服务商预设/Key/Base URL/模型、推送开关/Token、第二课堂网址、报名信息（姓名/学号/手机号）、提醒通知开关。
- 提醒：`NotificationHelper` 建渠道与权限；`ReminderScheduler` 用 `AlarmManager.setAndAllowWhileIdle` 排期（触发时间由 Rust `upcoming`/`signups` 计算，自动报名用 `lancai://signup/<id>` 单独排期）；`ReminderReceiver` 到点显示通知，报名通知点击后带 `AUTO_ENROLL` 打开报名页；`BootReceiver` 开机/更新后恢复。

所有界面元素由 `UiKit.java` 的工厂方法构建，**没有 layout XML**。新增页面必须：

1. 在 `AndroidManifest.xml` 注册（`exported="false"`，除非确需外部拉起）；
2. 用 `UiKit.topBar(this, "标题")` 提供返回；
3. 用 `ScrollView` 包裹可滚动内容，避免长内容截断。

## 已知坑（改之前必看）

1. **JNI 签名必须两端一致**：`nativeAnalyze(cfgJson, dataDir, source, text, persist)`、`nativeRecent(cfgJson, dataDir, limit)`、`nativeAi(cfgJson, dataDir, text)`、`nativePush(cfgJson, dataDir, title, content)`、`nativeClear(cfgJson, dataDir)`、`nativeSelfTest(cfgJson)`、`nativeVersion()`；日程 `nativeScheduleList/Save/Delete/Toggle/ClearDone/Upcoming`；能否参加 `nativeEnrollCheck(cfgJson, dataDir, text)`；课表 `nativeTimetableGet/Import/SetTerm/Delete/Clear`。改一端必须同步另一端，否则运行时报 `NoSuchMethodError` / `UnsatisfiedLinkError`。
2. **dataDir 必须传 `Config.dataDir(c)`**：Rust 里用 `/tmp` 在 Android 上不可写，会导致 Engine 创建失败。ai/push 也必须带 dataDir。
3. **版本覆盖**：`aapt2 link --version-code/--version-name` 对已写死的清单不生效，`build.sh` 已通过临时清单解决。不要删掉这段 sed。
4. **后台线程**：JNI 调用统一放在 `new Thread` 里，结果用 `Handler(Looper.getMainLooper())` 回主线程，禁止主线程做网络/分析。
5. **AI/推送为可选**：未配置时必须静默跳过，不能阻塞本地分析。
6. **UI 无自动化测试**：本环境没有模拟器和真机。改 UI 后如果无法实际运行，必须在回复里明确写“未做运行验证”，并说明做了哪些静态检查。
7. **通知提醒**：Android 13+ 需要 `POST_NOTIFICATIONS` 运行时权限；用 `AlarmManager.setAndAllowWhileIdle`（非精确闹钟，不需要 `SCHEDULE_EXACT_ALARM`）。提醒时间由 Rust `ScheduleStore::upcoming` 计算，Java 不重复计算。重启/应用更新后由 `BootReceiver` 重新排期；关闭设置里的提醒开关时必须 `ReminderScheduler.cancelAll`。
8. **不要打印密钥**：任何日志/对话框都不要输出 Token / Key 明文。

## 常见任务入口

| 任务 | 先看 | 改完验证 |
| --- | --- | --- |
| 调整打分、抽取字段 | `../lancai-sentinel-rs/src/rule_engine.rs` | `cargo test` |
| 调整文本清洗/截断 | `../lancai-sentinel-rs/src/text.rs` | `cargo test` |
| 改消息库 | `../lancai-sentinel-rs/src/store.rs` | `cargo test` |
| 改 AI / 推送 | `../lancai-sentinel-rs/src/net.rs`、`pipeline.rs` | `cargo run -- ai-test` / `push-test` |
| 改 JNI | `../lancai-sentinel-rs/src/android.rs` + `core/RustCore.java` | `./build.sh` + `nm -D` 查符号 |
| 改设置项 / 二级菜单 | `ui/SettingsActivity.java`、`ui/MenuActivity.java` | `./build.sh` + 真机 |
| 改版本 / 打包 | `build.sh`、`app/AndroidManifest.xml` | `aapt2 dump badging` |

## 验收命令

```bash
cd ../lancai-sentinel-rs && cargo test
cd ../lancai-sentinel-android && VER_NAME=<新版本> VER_CODE=<递增> ./build.sh
/workspace/.android-sdk/build-tools/34.0.0/aapt2 dump badging dist/lancai-sentinel-v<新版本>.apk | head -1
```

期望：`cargo test` 全绿、`BUILD_OK`、badging 的 versionCode/versionName 与新版本一致。

## 禁止事项

- 不要提交 `keys/`、`build/`、`dist/`、`*.jks`、任何 Token/Key。
- 不要改包名 `com.lancai.sentinel`，否则无法覆盖升级。
- 不要重新引入通知监听、保活。自动报名只允许用户逐条确认的「通知 + 点击打开报名页」流程，禁止后台静默报名。只允许本机提醒用的 `ReminderReceiver` / `BootReceiver`。
- 不要在 Java 里新增业务逻辑（解析/打分/网络/存储）。日程与课表的解析、存储、提醒时间计算都在 Rust。
- 不要引入新的上传域名或上传文本；应用承诺本地处理，只有开启 AI/推送才发给用户配置的服务商。第二课堂例外：仅在用户点击时于应用内 WebView 加载用户配置的网址，不上传任何文本、不注入页面。
