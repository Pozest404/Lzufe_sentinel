#!/usr/bin/env bash
# 把构建产物发布到 GitHub Releases，并按版本只保留最近 KEEP 个 release。
#
# 用法:
#   ./gh-release.sh <版本号> <apk> [源码包]
#
# 环境变量:
#   KEEP_VERSIONS  保留的 release 数量，默认 2；设为 0 表示不清理
#   GH_REPO        覆盖仓库，默认取 git remote origin
set -euo pipefail

VER="${1:?用法: gh-release.sh <版本号> <apk> [源码包]}"
APK="${2:?缺少 APK 路径}"
SRC="${3:-}"
KEEP="${KEEP_VERSIONS:-2}"
TAG="v$VER"

[ -f "$APK" ] || { echo "找不到 APK: $APK" >&2; exit 1; }

ASSETS=("$APK")
if [ -n "$SRC" ] && [ -f "$SRC" ]; then
    ASSETS+=("$SRC")
fi

NOTES="$(cat <<EOF
澜彩哨兵 $TAG

- 构建时间：$(date '+%Y-%m-%d %H:%M:%S')
- 安装包：$(basename "$APK")
- SHA256：$(sha256sum "$APK" | cut -d' ' -f1)

安装：下载 APK 后允许「安装未知来源应用」即可覆盖升级，需 Android 7.0 及以上。
EOF
)"

if gh release view "$TAG" >/dev/null 2>&1; then
    echo "  = $TAG 已存在，追加/覆盖产物"
    gh release upload "$TAG" "${ASSETS[@]}" --clobber
else
    echo "  + 创建 release $TAG"
    gh release create "$TAG" "${ASSETS[@]}" \
        --title "澜彩哨兵 $TAG" \
        --notes "$NOTES"
fi

# 只保留最近 KEEP 个 release（按版本号倒序）
if [ "$KEEP" != "0" ]; then
    mapfile -t versions < <(
        gh release list --limit 200 --json tagName \
            --jq '.[].tagName' | sed 's/^v//' | sort -V -r
    )
    n=0
    for v in "${versions[@]}"; do
        n=$((n + 1))
        if [ "$n" -gt "$KEEP" ]; then
            echo "  - 删除旧 release v$v"
            gh release delete "v$v" --yes --cleanup-tag || true
        fi
    done
fi

echo "RELEASE_OK $TAG"
