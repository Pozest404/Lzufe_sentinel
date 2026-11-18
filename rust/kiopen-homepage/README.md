# Kiopen 主页编辑器

用 Rust 编写的本地可视化编辑器，用来修改 [kiopen.icu](https://kiopen.icu/) 的内容并一键发布到 GitHub Pages。

单文件静态二进制，零运行时依赖（不需要 Python / Node），只依赖系统自带的 `git`。

![编辑器界面](docs/editor.jpg)

## 下载

- 发行版页面：[gitee.com/kiopen/kiopen-homepage/releases](https://gitee.com/kiopen/kiopen-homepage/releases/tag/v2.1.1)
- Debian / Ubuntu / Pop!_OS：[kiopen-homepage_2.1.1_amd64.deb](https://gitee.com/kiopen/kiopen-homepage/releases/download/v2.1.1/kiopen-homepage_2.1.1_amd64.deb)
- 任意 x86_64 Linux：[kiopen-homepage-x86_64-linux](https://gitee.com/kiopen/kiopen-homepage/releases/download/v2.1.1/kiopen-homepage-x86_64-linux)（静态二进制，可直接运行）

## 安装

```bash
sudo apt install ./kiopen-homepage_2.1.1_amd64.deb
```

或直接使用二进制：

```bash
chmod +x kiopen-homepage-x86_64-linux
./kiopen-homepage-x86_64-linux
```

## 使用

```bash
kiopen-homepage
```

首次启动会把 [Pozest404.github.io](https://github.com/Pozest404/Pozest404.github.io) 克隆到 `~/kiopen-homepage`，然后打开浏览器里的编辑器。

也可以在应用菜单里搜索「Kiopen 主页编辑器」。

### 打不开 / 启动没反应？

- 请从终端运行 `kiopen-homepage`，终端会打印地址与错误信息
- 页面会立即打开并显示初始化进度；失败时显示具体原因和「重试」按钮
- 首次运行需要 `git` 以及能访问 GitHub；无法访问时用镜像：
  `KIOPEN_REPO=https://gitee.com/you/homepage.git kiopen-homepage`
- 端口 8787 被占用会自动顺延到 8788、8789…，终端会打印实际地址
- 如果提示 `package architecture (amd64) does not match system`，说明系统不是 x86_64，请改用对应架构的二进制

### 功能

| 分区 | 内容 |
| --- | --- |
| 常用 | 姓名、简介、打字机文案、技术标签、GitHub / Gitee / 邮箱 |
| 项目 | 增删项目、拖动或 ↑↓ 排序、描述、状态徽章、标签、任意数量链接 |
| 头像 | 点击或拖拽上传，自动识别 PNG / JPG / WEBP / GIF 并替换引用 |
| SEO | 页面标题、描述，自动写入 OG / Twitter Card |
| 历史 | 浏览、预览、恢复每一次自动备份 |
| 源码 | 直接编辑完整 HTML |

- 右侧实时预览，支持桌面 / 手机两种视图，可一键隐藏
- 每次保存自动备份到 `.backups/`（已加入 git 忽略），可随时回滚
- 草稿自动保存在浏览器，刷新或误关页面后可恢复
- 命令面板（`Ctrl + K`）快速执行保存、发布、切页、换主题等操作
- 多级撤销 / 重做（`Ctrl + Z` / `Ctrl + Shift + Z`）
- 深色 / 浅色主题，跟随系统或手动切换
- **自动发布**：开启后每次保存成功会自动提交并推送
- `Ctrl + S` 保存，`Ctrl + Enter` 发布
- 服务只监听 `127.0.0.1`，并校验 Origin，局域网和外网都无法访问

### 发布

发布就是在本目录执行 `git add -A && git commit && git push`，所以需要先配置 GitHub 凭据：

```bash
gh auth login
# 或者把 remote 换成 SSH
git remote set-url origin git@github.com:Pozest404/Pozest404.github.io.git
```

推送后 GitHub Pages 通常 1 分钟内完成构建。

## 命令行

```bash
kiopen-homepage [serve|setup|publish] [选项]

  serve      启动本地编辑器（默认）
  setup      克隆 / 更新仓库
  publish    不开界面，直接提交并推送

  -d, --dir <路径>   仓库目录（默认 ~/kiopen-homepage）
  -p, --port <端口>  监听端口（默认 8787）
      --no-browser   不自动打开浏览器
  -V, --version      显示版本
  -h, --help         显示帮助
```

环境变量：

| 变量 | 默认值 |
| --- | --- |
| `KIOPEN_HOMEPAGE_DIR` | `~/kiopen-homepage` |
| `KIOPEN_REPO` | `https://github.com/Pozest404/Pozest404.github.io.git` |
| `KIOPEN_BRANCH` | `main` |
| `KIOPEN_SITE_URL` | `https://kiopen.icu/` |
| `KIOPEN_PORT` | `8787` |

## 从源码构建

```bash
rustup target add x86_64-unknown-linux-musl
sudo apt install musl-tools
cargo build --release --target x86_64-unknown-linux-musl
```

产物：`target/x86_64-unknown-linux-musl/release/kiopen-homepage`（静态链接，约 630KB）。

打包 deb：

```bash
./build.sh
```

## 卸载

```bash
sudo apt remove kiopen-homepage
rm -rf ~/kiopen-homepage      # 如需连同本地仓库一起删除
```

## 许可证

MIT
