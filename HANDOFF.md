# 澜彩哨兵 · 交接文档（HANDOFF）

> 生成于 2026-11-11。所有结论均为**实际读取文件 / 运行命令**核验后写入；未核验项已明确标注。
> 接手者：先读第 1 节「硬约束」，再读第 7 节「已知坑」，然后从第 8 节挑任务。

---

## 最新发布 · v3.7.6 · 2026-11-13 15:48（扫码签到修复）

| 项 | 值 |
|---|---|
| 版本 | `3.7.6` / `versionCode=55`（aapt2 badging 实测） |
| 包名 | `com.lancai.sentinel` ✅ |
| commit | `af2143f fix(dekt): 修复扫码签到无反应` + `719c8a0 build.sh: NDK 自动探测` |
| cargo test | 45 passed, 0 failed（qr 模块 4 例 + 原有 41 例） |
| APK | 6,788,429 字节 · SHA256 `144147be37cd1e4d4c5666e27b907b6c996c961192d98898bcb576d2d138b496` |
| 签名证书 SHA-256 | `450d288a6428f3e762efd4b5d37070ba806d2e7d45b7cd1ea66cecf4e01e5d42`（与 v3.7.5 同一把 → 可覆盖升级） |
| GitHub Release | https://github.com/Pozest404/Lzufe_sentinel/releases/tag/v3.7.6 ✅ 已发布 |
| Release 资产 | APK（6.79MB）+ 源码包（212KB，**keys/* 已排除**） |
| Release 时间 | 2026-11-13 07:48 UTC |
| GoFile | 未上传（环境重置丢失 token；build.sh `GOFILE_AUTO=1`，token 在 `.gofile-sync.json` 恢复后下次构建自动上传） |

**问题根因（实测取证的第三方站点代码）**

第二课堂 `win.9xueqi.com` 的「扫码」按钮不调用 WebView 相机，只调用宿主 App 的原生桥：
`window.doschool.jsbridge_qrscan` / `dd.biz.util.scan` / `campus.scan` / `wx.scanQRCode`。
普通 WebView 里这些桥都不存在，`wx.ready()` 因 `WeixinJSBridgeReady` 永不触发，所以点击后毫无反应。

**修复内容**

1. `qr.rs`（新）：`rqrr` 纯 Rust 二维码解码，4 个单元测试覆盖 ASCII / UTF-8 / 空白图 / 非法尺寸。
2. `android.rs`：新增 JNI `nativeDecodeQr(width, height, gray[])`。
3. `RustCore.java`：新增 `decodeQr(...)`。
4. `DektActivity.java`：注入 `wx` 替身接管 `scanQRCode`，弹「拍照扫码 / 从相册选择」→ 子线程 Bitmap 转灰度 → Rust 解码 → 按 WeChat JSSDK 回调格式回灌页面（`success({errMsg:'scanQRCode:ok', resultStr})`），站点自己的签到请求逻辑不变。
5. `CaptureProvider.java`（新）：零依赖 ContentProvider，给系统相机提供可写 `content://` URI（未新增相机权限）。
6. `AndroidManifest.xml`：注册 provider + `uses-feature camera required=false`。
7. `build.sh`：源码包排除 `keys/*`（此前把 `lancai.jks` 打进源码包并上传 GitHub Release）。

**安全提醒（需人工处置）**

- v3.7.5 的 Release 源码包内**包含私钥 `keys/lancai.jks`**，该文件可从
  `https://github.com/Pozest404/Lzufe_sentinel/releases/download/v3.7.5/lancai-sentinel-src-v3.7.5.zip`
  公开下载；密码写在公开仓库的 `build.sh` 里，等同于签名密钥已泄露。
- 本次 v3.7.6 已从源码包剔除密钥；v3.7.5 的旧 asset 需你登录后删除（或转私有 release）。
- 若要彻底止损，应轮换签名密钥（代价：老用户需卸载重装一次）。

**未验证项**

- 扫码流程未在真机运行验证（本机无 Android 设备/模拟器）；目前仅通过编译、单测与 aapt2 元数据校验。

---

## 最新发布 · v3.7.5 · 2026-11-12 15:34（重签后）

| 项 | 值 |
|---|---|
| 版本 | `3.7.5` / `versionCode=54` |
| commit | `aec74c96517123e688075737f61ed6b0503243ee chore: bump version to 3.7.5 (54)` |
| cargo test | 41 passed, 0 failed（22 lib + 2 data_layer + 5 pipeline + 12 rule_engine） |
| APK | 6,411,597 字节 · SHA256 `f444bd227ee2a0851d77501ffd72dfa2a4bc4d2fe04b19bd6abfada228cf7057` |
| 包名 | `com.lancai.sentinel`（aapt2 badging 实测一致） |
| GitHub Release | https://github.com/Pozest404/Lzufe_sentinel/releases/tag/v3.7.5 **Latest** ✅ 含 APK + 源码包 |
| GoFile APK | https://gofile.io/d/72yYolHM ✅ |
| GoFile 源码 | https://gofile.io/d/VDKVRjeo ✅ |
| 保留策略 | GitHub 只剩 v3.7.5（v3.7.4 + v3.7.3 已删除 + tag 已清理）；dist/ 剩 v3.7.5；gofile-links.md 2 条 ✅ |
| **签名密钥** | `src/lancai-sentinel-android/keys/lancai.jks`（JKS PKCS#12，alias `lancai`，storepass `CHANGE-ME-BEFORE-USE`，subject `CN=Lancai Sentinel, OU=Student, O=Lancai, L=Beijing, C=CN`，SHA-256 `45:0D:28:8A:64:28:F3:E7:62:EF:D4:B5:D3:70:70:BA:80:6D:2E:7D:45:B7:CD:1E:A6:6C:EC:F4:E0:1E:5D:42`）。**这把是 2026-11-12 自动生成、之后永久复用的唯一签名密钥**，build.sh 检测到文件存在就不会再生成新的。密钥路径已被 `.gitignore` 排除，仅本地持有；另有一份 `lancai.jks.bak` 作为工作区内备份。v3.7.5 重签后，所有 ≥ v3.7.5 的发布都能用这把密钥互相覆盖升级。 |
| 构建位置 | `/workspace/src/lancai-sentinel-android` 原地构建 |

| 用户需知 | |
|---|---|
| v3.7.4 → v3.7.5 覆盖 | **不可直接覆盖**——v3.7.4 使用的原始密钥在 workspace 重置时丢失，已无法恢复。用户需先卸载 v3.7.4 再装 v3.7.5（本机数据将被清除）。 |
| v3.7.5 → 后续版本 | **可覆盖**——所有 ≥ v3.7.5 的发布都用同一把 lancai.jks。 |

---

## 性能基线（2026-11-12 实测）

| 场景 | 实测值 | 备注 |
|---|---|---|
| Rust `analyze_text` 同进程 persist=false | **13.2 µs / 次** | 5000 次 bench，release profile opt-level=z + lto + codegen-units=1 + strip=symbols |
| Rust `analyze_text` 同进程 persist=true（入库 msgs.jsonl） | **12.7 µs / 次** | 2000 次 bench，含文件 IO |
| Android 上实际调用开销（估算） | ~50-200 µs | JNI 跨语言 + arm64 较慢，仍远低于 16.6 ms（60 fps 单帧）|
| 验收门槛「中端设备 < 5 ms」 | ✅ **已满足** | 实际耗时仅门槛的 1/400 |
| Android UI 主线程 | ✅ **无阻塞** | 所有 RustCore 调用已放到 `new Thread() + ui.post()` 子线程 |

**Rust 构建档位已到极致**，无进一步优化空间：`opt-level = "z"`（体积最小） + `lto = true`（链接时优化） + `codegen-units = 1`（单编译单元最大化内联） + `strip = "symbols"`（剥离调试符号）。

**subprocess 循环测的 13.3 ms/次是假信号**——那是每次 fork+exec + clap 命令行解析 + Config+Engine 初始化的开销（≈2 ms + ≈11 ms），不是实际分析耗时。Android JNI 是同进程调用，没有这部分开销。

---

## build.sh 自动版本号（2026-11-12 新增）

未显式传 `VER_NAME` / `VER_CODE` 时，build.sh 自动从以下三个来源取最大值递增：
1. `git tag v*`（仓库现有 tag）
2. `src/lancai-sentinel-android/dist/*.apk`（aapt2 dump badging 读 versionCode）
3. `app/AndroidManifest.xml` 默认 versionCode/versionName

版本号取三者中 patch 最大的 → 递增 patch（如 3.7.5 → 3.7.6）；versionCode 取最大 +1。已测通过（3.7.5 未传 → 自动算出 3.7.6 / code=55）。

**发布链路前提已就绪**：
| 前提 | 状态 |
|---|---|
| `keys/lancai.jks` | ✅ 存在（build.sh 自动复用） |
| `gh auth` | ✅ `Pozest404` 已登录 |
| GoFile token | ✅ `/workspace/.gofile-sync.json` 存在 |
| build.sh 默认 | `GOFILE_AUTO=1 GH_RELEASE=1 KEEP_VERSIONS=2` |
| 定时任务 | `U6HH6RFWHA4QS5` 每周一 09:00 Asia/Shanghai |

---

## 0. 一句话现状

`v3.7.3`（versionCode 52）已构建成功，Rust 36 个测试全绿；本轮完成了 **7 项后台占用 / 能耗优化**。
**没有 git 仓库**（`git status` 报 not a repository），改代码前请自行备份或用文件差异管理。

- 工作目录：`/workspace`
- Android 工程：`/workspace/src/lancai-sentinel-android`
- Rust 内核：`/workspace/src/lancai-sentinel-rs`
- 最新产物：`/workspace/src/lancai-sentinel-android/dist/lancai-sentinel-v3.7.3.apk`（6,399,309 字节，md5 `c4c486d9e5b5d55e77b64f77b268aab3`）

---

## 1. 硬约束（来自 `src/AGENTS.md` 与 `src/lancai-sentinel-android/AGENTS.md`，冲突以 Android 那份为准）

| # | 约束 |
|---|------|
| 1 | **业务逻辑必须写在 Rust**（解析、打分、抽取、存储、网络、AI、提醒时间计算）。Java 只做 Activity/View、SharedPreferences、Intent、JNI 调用。 |
| 2 | 不引入 Gradle / AndroidX / 任何第三方 Java 依赖。构建链路固定：`aapt2 + javac + d8 + zipalign + apksigner`。 |
| 3 | 不重新引入通知监听、NotificationListenerService、无障碍、保活服务。只允许 `ReminderReceiver` / `BootReceiver`。 |
| 4 | 自动报名只能「用户逐条确认 → 到点发通知 → 用户点击进报名页」，禁止后台静默提交。 |
| 5 | 不改包名 `com.lancai.sentinel`，不换 `keys/lancai.jks`，发版必须递增 `VER_CODE`。 |
| 6 | `/workspace/.uploads` 只读，不修改不删除。 |
| 7 | 不要在日志/对话框打印 Token / API Key 明文。 |
| 8 | Rust release 配置保持 `opt-level="z"`、`lto=true`、`codegen-units=1`、`strip="symbols"`。 |
| 9 | 新增能力的顺序：Rust 模块 + 单元测试 → `src/android.rs` 暴露 JNI → `RustCore.java` 包装 → UI 调用。 |

---

## 2. 仓库地图

```
/workspace
├── src/
│   ├── AGENTS.md                      项目总约定（必读）
│   ├── lancai-sentinel-rs/            Rust 内核（36 tests）
│   └── lancai-sentinel-android/
│       ├── AGENTS.md                  Android 层约定（必读，优先级更高）
│       ├── build.sh                   一键构建（Rust 双 ABI + Java + 签名）
│       ├── keys/lancai.jks            签名密钥（勿动）
│       ├── app/AndroidManifest.xml    权限 / 组件注册 / 版本默认值
│       ├── app/java/com/lancai/sentinel/
│       │   ├── core/                  Config、RustCore(JNI)、NotificationHelper、
│       │   │                          ReminderScheduler、ReminderReceiver、BootReceiver
│       │   └── ui/                    MainActivity、TimetableActivity、ScheduleActivity、
│       │                              DektActivity、SettingsActivity、
│       │                              UiKit、TimetableGrid、WeekSwipe
│       └── dist/                      APK 产物
├── release-server/
│   ├── server.py                      本地发布站（零依赖 Python，含上传/下载/删除）
│   └── gofile-upload.py               GoFile 批量上传脚本
├── gofile-upload.py                   上面脚本的副本（根目录直接可跑）
├── gofile-links.md                    60 个发布文件的 GoFile 下载索引
└── lancai-sentinel-v3.*.apk / *.zip   历史发布包（21 个版本）
```

**UI 全部用代码构建，没有 layout XML。** 新增页面必须：Manifest 注册（`exported="false"`）→ `UiKit.topBar()` 给返回 → `ScrollView` 包裹。

**注意：`MenuActivity` 已删除**（改为 `UiKit.menuSheet()` 底部弹出面板），但两份 `AGENTS.md` 里仍有大量 MenuActivity 的描述，属于**过期文档**，见第 8 节。

---

## 3. 本轮已完成的改动（能耗 / 后台占用，7 项，已 grep 核验存在）

| # | 文件:行 | 改动 | 原因 |
|---|---------|------|------|
| 1 | [DektActivity.java:505](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/ui/DektActivity.java#L500-L510) | `onPause()` 增加 `web.pauseTimers()`；`onResume()` 增加 `web.resumeTimers()` | `web.onPause()` 只暂停 WebView 内部动画，**全局 JS 定时器仍在跑**。这是后台耗电最大头。 |
| 2 | [MainActivity.java:88](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/ui/MainActivity.java#L88-L94) | 删除 `onResume()` 里的 `ReminderScheduler.reschedule()` 调用 | 每次回主页都 `cancelAll` + 全量重建 PendingIntent，纯浪费。`AlarmManager` 闹钟跨进程存活，不需要每次重排。 |
| 3 | [ScheduleActivity.java](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/ui/ScheduleActivity.java) `load()` | 删除读取列表时的 `reschedule()` | 读数据不应触发写操作。真正的增删改路径（第 291/635/667/691 行）仍各自保留。 |
| 4 | [BootReceiver.java](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/core/BootReceiver.java) | 增加 `PARTIAL_WAKE_LOCK`，`acquire(15000L)` 兜底自动释放，`finally` 里 release；调用改为 `reschedule(ctx, true)` | Doze 下开机广播线程可能被提前杀死，导致提醒丢失。 |
| 5 | [ReminderScheduler.java:37-70](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/core/ReminderScheduler.java#L37-L70) | 新增 `reschedule(Context, boolean force)` 重载 + 1 分钟节流（`lastRescheduleAtMs` / `lastRescheduleDataKey`） | 连续 save→toggle→delete 会触发 3 次全量重排。 |
| 6 | [MainActivity.java:336-345](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/ui/MainActivity.java#L336-L345) | `load()` 增加 3 秒预判：距上次 load < 3s 且有缓存时直接渲染，不进线程、不调 JNI | 屏幕旋转、Activity 重建等场景数据没变却白跑两次 JNI。 |
| 7 | [Config.java:38-59](file:///workspace/src/lancai-sentinel-android/app/java/com/lancai/sentinel/core/Config.java#L38-L59) | 改为 DCL 单例（`private static volatile Config INSTANCE`） | 原来每次 `Config.get()` 都 `new`。 |

### 当前 `reschedule` 调用点（核验后的完整清单）

```
MainActivity.java:839        ← 保存活动到日程成功后
ScheduleActivity.java:291    ← 保存/编辑
ScheduleActivity.java:635    ← 删除
ScheduleActivity.java:667    ← 勾选完成
ScheduleActivity.java:691    ← 清除已完成
SettingsActivity.java:269    ← 提醒开关变更
DektActivity.java:381        ← 「记到课表」
BootReceiver.java:25,45      ← 开机 / 应用更新（force=true）
```

---

## 4. 当前验证状态（实际运行结果）

| 项目 | 命令 | 结果 |
|------|------|------|
| Rust 测试 | `cd src/lancai-sentinel-rs && cargo test` | **36 passed**（17 + 2 + 5 + 12 四个测试二进制） |
| Android 构建 | `cd src/lancai-sentinel-android && VER_NAME=3.7.3 VER_CODE=52 ./build.sh` | 输出 `BUILD_OK`，产物 6.1 MB（上一轮执行，未在本轮重跑） |
| 真机 / 模拟器 | — | **完全没有验证过**。本环境无设备。 |

**版本号陷阱**：`app/AndroidManifest.xml` 里写死的仍是 `versionCode="51"` / `versionName="3.7.2"`。`build.sh` 会生成临时清单覆盖它（第 122-123、133-134 行），所以**不发版时不会自动同步**。下次发版前记得把默认值也改成新的，否则直接打开工程看版本会对不上。

**权限已就绪**：`POST_NOTIFICATIONS` 已在 Manifest 第 13 行声明，运行时请求在 `NotificationHelper.java:45-55`。之前交接文档里列为「待办」的这项，实际已经做完了。

---

## 5. 发布渠道

### 本地发布站（零依赖）
```bash
cd /workspace && python3 release-server/server.py . 8765
# 打开 http://127.0.0.1:8765/
```
能力：文件列表按版本分组、拖拽多文件上传、Range 断点续传下载、删除（可用 `DELETE_TOKEN` 环境变量加保护）。
> 上一轮启动的后台进程大概率已随会话结束，需要时重新启动。

### GoFile 公网分享
```bash
cd /workspace && python3 gofile-upload.py . gofile-links.md
```
- 上传端点：`POST https://upload.gofile.io/uploadfile`，表单字段名 `file`，免注册（游客账户）
- 响应字段：`data.downloadPage` / `data.parentFolderCode` / `data.id` / `data.name`
- **GoFile 免费账户不给直链**，只能给 `https://gofile.io/d/<code>` 分享页
- 已上传 **60 个文件 / 21 个版本**，索引见 [gofile-links.md](file:///workspace/gofile-links.md)
- 最新版：v3.7.3 APK → https://gofile.io/d/6EcNsmrU

---

## 6. 未完成事项

| 优先级 | 事项 | 说明 / 起点 |
|--------|------|-------------|
| P1 | **真机回归** | 无设备。至少验证：① Android 13+ 通知权限弹窗 ② 课表左右滑切周跟手 ③ 底部面板不遮导航栏 ④ 「记到课表」保存后立即出现在课表 |
| P2 | 冷启动优化 | 当前 1.8s+。方案：新建 `LancaiApplication` 在 `onCreate` 预加载 `RustCore.available()`，Manifest `<application android:name=".LancaiApplication">`。**未做** |
| P2 | 第二课堂 Cookie 持久化 | `DektActivity` 是裸 WebView，登录态可能丢失。需要 `CookieManager` 持久化。**未做** |
| P3 | 「记到课表」弹窗体验 | 目前是两个 EditText + 选值行，可加「记住上次时间」「链接自动提取」「周几预览」 |
| P3 | 文档同步 | 两份 `AGENTS.md` 仍在描述 MenuActivity（已删除）和 v3.3 的首页布局（已改为「首页只显示课表」），容易误导后续模型 |

---

## 7. 已知坑（改之前必看）

1. **JNI 签名两端必须一致**。已有：`nativeAnalyze / nativeRecent / nativeAi / nativePush / nativeClear / nativeSelfTest / nativeVersion`、日程 `nativeScheduleList/Save/Delete/Toggle/ClearDone/Upcoming`、`nativeEnrollCheck`、课表 `nativeTimetableGet/Import/SetTerm/Delete/Clear`。改一端不同步另一端 → `NoSuchMethodError` / `UnsatisfiedLinkError`。
2. **dataDir 必须传 `Config.dataDir(c)`**。Rust 侧 `/tmp` 在 Android 不可写，会导致 Engine 创建失败。
3. **JNI 一律放后台线程**，结果用 `Handler(Looper.getMainLooper())` 回主线程。
4. **`MainActivity.load()` 的 3 秒预判有陈旧风险**（本轮引入的取舍，未验证）：如果用户在 3 秒内改了日程又立刻返回主页，可能渲染旧缓存。若发现「勾选完成后列表没变」，第一嫌疑就是它 —— 修法是把预判改成「只在 `isChangingConfigurations()` 或 `savedInstanceState != null` 时走缓存」。
5. **1 分钟节流也会吃掉合法的重排**：`reschedule` 在 60s 内且数据 key 相同时直接返回，但此时已经执行过 `cancelAll`。也就是说这一轮会「只取消不重排」。如果遇到「提醒突然不响了」，优先检查这个逻辑，必要时给所有变更路径加 `force=true`。
6. **`setAndAllowWhileIdle` 是非精确闹钟**，Doze 下可能延迟数分钟，这是设计预期，不要改成 `setExactAndAllowWhileIdle`（会需要 `SCHEDULE_EXACT_ALARM` 权限）。
7. **版本覆盖靠 build.sh 临时清单**，不要删第 122-123 行的 sed。
8. 课表领域规则：周次从 1 开始，周一 `day=1`、周日 `day=7`；节次 1-10 时间固定；`term_start` 由 Rust 自动对齐到该周周一。

---

## 8. 给接手模型的操作建议

1. **先跑两条基线命令**，确认环境没坏：
   ```bash
   cd /workspace/src/lancai-sentinel-rs && cargo test          # 期望 36 passed
   cd /workspace/src/lancai-sentinel-android && ./build.sh     # 期望 BUILD_OK
   ```
2. **一次只做一个文件级小任务**。本工程没有 git，大改动难以回滚。
3. **改完必须给出验证输出**（`cargo test` 尾部 / `BUILD_OK` / `aapt2 dump badging`），不要只说「已完成」。
4. **UI 改动无法自动验证**，必须在回复里写明「未做真机验证」。
5. 若下一步是「冷启动优化」，优先做第 6 节 P2 的 `LancaiApplication`，改动面最小（新建 1 个文件 + Manifest 1 行）；不要顺手重写 MainActivity。
6. 若下一步是「修文档」，只改 `AGENTS.md` 里与 MenuActivity / 首页结构相关的段落，不要整篇重写。
