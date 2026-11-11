# 澜彩哨兵 · 单 APK 整合包

手动分析工具：把你粘贴或分享进来的一段通知文本，交给 Rust 核心打分、抽取时间/地点/学分/名额/截止/报名方式，并保存最近记录。可选接入任意 OpenAI 兼容模型总结、PushPlus 推送；v3.3 起课表改为星课程表风格，并能按课表判断活动能否参加、在用户确认后自动报名。

```
dist/lancai-sentinel-v3.3.0.apk
```

- 包名：`com.lancai.sentinel`
- 版本：3.3.0（versionCode 34）
- 最低系统：Android 7.0（API 24）
- 目标系统：Android 14（API 34）
- 支持 ABI：arm64-v8a / armeabi-v7a

> v3.0 已移除微信通知监听与保活服务。应用不再读取任何通知，所有输入都由用户主动提供。
> v3.1 增加多模型 AI（DeepSeek / OpenAI / 通义千问 / 智谱 GLM / Kimi / 自定义本地）与第二课堂入口。
> v3.2 把第二课堂改为应用内 WebView，新增日程/待办/活动、课表导入（星课程表 .ics / JSON / CSV / 文本）与系统通知提醒。
> v3.3 课表改为星课程表风格；保存带链接的活动时，先用本地课表判断能否参加，再弹窗询问是否自动报名（默认「仅提醒」）。自动报名到点只发通知，点击后才进入报名页自动填写；不做后台静默报名。

## 安装

```bash
adb install -r dist/lancai-sentinel-v3.3.0.apk
```

或把 APK 发到手机点击安装。**如果手机上已装过原版（使用其他签名），需先卸载旧版**，否则会提示签名冲突。

签名使用仓库内 `keys/lancai.jks`（自签名，密码 `CHANGE-ME-BEFORE-USE`）。正式分发前请替换为自己的密钥。

安装后无需开启通知使用权或保活权限；联网权限用于可选的 AI/推送，通知权限用于日程与活动提醒（Android 13+ 首次启动会询问）。

## 使用

1. 打开 App，在首页文本框粘贴一段通知（例如班级群公告）。
2. 点「分析」：Rust 核心给出评分、分类与抽取字段；达到阈值标记为「重要」。
3. 也可以在其他 App 里长按文本 →「分享」→「澜彩哨兵」，自动带入分析。
4. 结果卡支持复制、单独请求 AI 总结、发送 PushPlus 推送。
5. 首页底部显示最近 10 条记录，点击可回填原文。
6. 点右上角「第二课堂」，**在应用内**打开九学期（默认 `https://win.9xueqi.com/#/home`）；右上角可刷新或交给系统浏览器。网址可在设置里修改。
7. 首页「日程待办」：添加待办或第二课堂活动，设置时间与提前提醒；到点发系统通知，点通知直达应用内活动链接。支持勾选完成、清除已完成。
8. 首页「本周课表」：周视图查看课程；点右上角「导入」可从星课程表等应用导出的 `.ics` 日历、JSON、CSV 或课表文本导入；可设置学期开始日期与总周数、删除单门课程。

## 界面结构（v3.2）

```
首页
├── 右上角         第二课堂（应用内 WebView） · 菜单
├── 快捷入口       日程待办 · 本周课表
├── 输入卡         来源（可选） + 多行文本 + 分析/清空
├── 结果卡         重要/普通/重复 · 分数 · 分类 · 字段 · AI · 复制 / AI / 推送
├── 最近记录       最近 10 条，点击回填
└── 菜单（右上角）
    ├── 校园工具    第二课堂 / 日程与待办 / 课表
    ├── 分析与规则  → 规则设置 / 规则自检 / 清空最近记录
    ├── 推送与 AI   → 设置 / 测试推送 / 测试 AI 总结
    └── 关于        → 版本与内核
```

## 与 Rust 核心的分工

业务逻辑全部在 Rust，Java 只是薄壳：

```
粘贴 / 分享文本 → MainActivity（Java，取文本）
                → RustCore.analyze()  → liblancai_sentinel.so
                     ├─ rule_engine.rs   打分、分类、字段抽取
                     ├─ store.rs         本地 JSONL 消息库
                     ├─ net.rs           AI（OpenAI 兼容）/ PushPlus（可选）
                     └─ pipeline.rs      串联分析与推送

日程 / 待办 → RustCore.schedule*() → schedule.rs（存储、提醒时间计算）
课表导入    → RustCore.timetable*() → timetable.rs（ICS/JSON/CSV/文本解析、周次）
```

- Rust 入口：[../lancai-sentinel-rs/src/android.rs](../lancai-sentinel-rs/src/android.rs)
- Java 桥：[app/java/com/lancai/sentinel/core/RustCore.java](app/java/com/lancai/sentinel/core/RustCore.java)
- 首页：[app/java/com/lancai/sentinel/ui/MainActivity.java](app/java/com/lancai/sentinel/ui/MainActivity.java)

Java 侧不含任何解析、打分、网络或存储逻辑；这些改动都在 `../lancai-sentinel-rs/src/` 完成。

## 构建

```bash
./build.sh
```

发布新版本时递增版本号即可（脚本会强制写入清单，无需手改）：

```bash
VER_NAME=3.2.1 VER_CODE=34 ./build.sh
```

脚本会自动完成：

1. `cargo build --release` 交叉编译三个 ABI 的 `liblancai_sentinel.so`（NDK 26.3）；
2. 把当前 Android + Rust 源码打进 `assets/lancai-sentinel-src-v<版本>.zip`；
3. `aapt2 compile/link` 资源与清单；
4. `javac`（JDK 17）编译 Java；
5. `d8` 生成 `classes.dex`；
6. 把 dex 与两个 ABI（arm64-v8a / armeabi-v7a）的 `.so` 打进 APK；
7. `zipalign -p 4` 对齐；
8. `apksigner` 签名并校验。

依赖：

- Android SDK：`build-tools;34.0.0`、`platforms;android-34`、`ndk;26.3.11579264`
- Rust targets：`aarch64-linux-android`、`armv7-linux-androideabi`
- JDK 17（d8 8.2.2 在 JDK 25 下会有内部错误，脚本会自动切到 17）

SDK 路径默认 `/workspace/.android-sdk`，可用 `ANDROID_SDK_ROOT` 覆盖；`LANCAI_JAVA_HOME` 可强制指定 JDK。

## 两种配置的关系

| 场景 | 配置位置 |
| --- | --- |
| 手机上运行 APK | App 内「菜单 → 分析与规则 / 推送与 AI」（SharedPreferences） |
| 桌面调试 / 回放 | Rust 工程的 `config.toml`，逐字段说明见 [../lancai-sentinel-rs/CONFIG.md](../lancai-sentinel-rs/CONFIG.md) |

APK 内 Rust 核心的阈值、追加关键词、AI（Key / Base URL / 模型）与推送参数均实时读取 App 内配置，修改后无需重新打包。第二课堂网址、提醒开关保存在 SharedPreferences；日程与课表保存在应用私有目录的 `schedule.json` / `timetable.json`。

## 目录

```
lancai-sentinel-android/
├── app/                 # Android 薄壳（Java + res + manifest）
├── build.sh             # 单包构建脚本
├── keys/lancai.jks      # 自签名密钥（请替换）
├── build/               # 中间产物
└── dist/                # 最终 APK
```
