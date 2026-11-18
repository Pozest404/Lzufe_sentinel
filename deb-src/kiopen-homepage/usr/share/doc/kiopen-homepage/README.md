# Kiopen 主页编辑器

本地可视化编辑并发布 `https://kiopen.icu/` 个人主页。

## 安装

```bash
sudo dpkg -i kiopen-homepage_1.0.0_all.deb
```

如果提示缺少依赖：

```bash
sudo apt-get install -f
```

## 启动

命令行：

```bash
kiopen-homepage
```

或在应用菜单里搜索「Kiopen 主页编辑器」。

首次启动会自动把主页仓库克隆到 `~/kiopen-homepage`，然后打开
`http://127.0.0.1:8765/`。

## 使用

- **常用**：姓名、简介、打字机文案、技术标签、社交链接、SEO、头像
- **项目**：三张项目卡片的标题、描述、状态、标签、链接
- **源码**：直接编辑完整 HTML
- **保存**：写入 `index.html`，旧版本自动备份到 `.backups/`
- **发布**：`git commit` + `git push`，GitHub Pages 约 1 分钟后更新

发布前请确保本机已配置 GitHub 认证，任选其一：

```bash
gh auth login
# 或使用 SSH key
git remote set-url origin git@github.com:Pozest404/Pozest404.github.io.git
```

## 命令行参数

```bash
kiopen-homepage [edit|serve|setup|publish] [--dir DIR] [--port 8765] [--no-browser]

kiopen-homepage setup --dir ~/site   # 只克隆仓库
kiopen-homepage serve --no-browser   # 不自动开浏览器
kiopen-homepage publish              # 命令行直接发布
```

## 备份与恢复

每次保存都会在 `.backups/` 生成 `index-日期-时间.html`。恢复时把它
复制回 `index.html` 即可。

## 卸载

```bash
sudo apt remove kiopen-homepage
```

`~/kiopen-homepage` 和备份不会被删除，需要时手动删除。
