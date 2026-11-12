#!/usr/bin/env bash
# 澜彩哨兵 · 单包构建（无 Gradle：Rust cdylib + aapt2 + javac + d8 + zipalign + apksigner）
#
# 产物：dist/lancai-sentinel-v<版本>.apk
# 依赖：
#   - Android SDK：build-tools 34.0.0、platforms/android-34、ndk 26.3.11579264
#   - Rust：已安装 aarch64-linux-android / armv7-linux-androideabi
#     （v3.3 起不再打包 x86_64，减小体积、加快构建）
#
# 可用环境变量覆盖：
#   ANDROID_SDK_ROOT  Rust 工程路径 RUST_CRATE  VER_NAME  VER_CODE

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK="${ANDROID_SDK_ROOT:-/workspace/.android-sdk}"
BT="$SDK/build-tools/34.0.0"
ANDROID_JAR="$SDK/platforms/android-34/android.jar"
NDK="${ANDROID_NDK_HOME:-$SDK/ndk/26.3.11579264}"
TOOLCHAIN="$NDK/toolchains/llvm/prebuilt/linux-x86_64/bin"
RUST_CRATE="${RUST_CRATE:-$ROOT/../lancai-sentinel-rs}"

APP="$ROOT/app"
BUILD="$ROOT/build"
DIST="$ROOT/dist"
KEYS="$ROOT/keys"
VER_NAME="${VER_NAME:-3.7.5}"
VER_CODE="${VER_CODE:-54}"
SO_NAME="liblancai_sentinel.so"

step() { printf '\n==> %s\n' "$*"; }
fail() { printf 'BUILD FAILED: %s\n' "$*" >&2; exit 1; }

# d8 8.2.2 在 JDK 25 下处理匿名类会内部报错；默认使用 JDK 17（与原项目构建环境一致）。
if [ -n "${LANCAI_JAVA_HOME:-}" ]; then
    export JAVA_HOME="$LANCAI_JAVA_HOME"
else
    for d in /root/.local/share/mise/installs/java/17* /usr/lib/jvm/java-17*; do
        if [ -x "$d/bin/javac" ]; then
            export JAVA_HOME="$d"
            break
        fi
    done
fi
[ -n "${JAVA_HOME:-}" ] && export PATH="$JAVA_HOME/bin:$PATH"
command -v javac >/dev/null || fail "找不到 javac"
printf 'javac: %s\n' "$(javac -version 2>&1)"

[ -x "$BT/aapt2" ] || fail "找不到 $BT/aapt2（先安装 build-tools;34.0.0）"
[ -f "$ANDROID_JAR" ] || fail "找不到 $ANDROID_JAR（先安装 platforms;android-34）"
[ -x "$TOOLCHAIN/llvm-strip" ] || fail "找不到 NDK 工具链：$TOOLCHAIN"

rm -rf "$BUILD"
mkdir -p "$BUILD/gen" "$BUILD/classes" "$BUILD/dex" "$DIST" "$KEYS"

# ---------------------------------------------------------------- 1. Rust 核心
step "构建 Rust 核心（arm64-v8a / armeabi-v7a）"
build_so() {
    local abi="$1" target="$2" cc="$3"
    local upper
    upper="$(echo "$target" | tr 'a-z-' 'A-Z_')"
    local linker="$TOOLCHAIN/$cc"
    [ -x "$linker" ] || fail "找不到链接器 $linker"
    echo "  - $abi ($target)"
    env \
        "CARGO_TARGET_${upper}_LINKER=$linker" \
        "TARGET_CC=$linker" \
        "TARGET_AR=$TOOLCHAIN/llvm-ar" \
        cargo build --release --manifest-path "$RUST_CRATE/Cargo.toml" --target "$target"
    local so="$RUST_CRATE/target/$target/release/$SO_NAME"
    [ -f "$so" ] || fail "Rust 产物不存在：$so"
    mkdir -p "$BUILD/lib/$abi"
    "$TOOLCHAIN/llvm-strip" --strip-unneeded "$so" -o "$BUILD/lib/$abi/$SO_NAME"
}
build_so arm64-v8a   aarch64-linux-android       aarch64-linux-android24-clang
build_so armeabi-v7a armv7-linux-androideabi     armv7a-linux-androideabi24-clang

# ---------------------------------------------------------------- 2. 内置源码
# 把当前 Android + Rust 源码打进 APK 的 assets，便于日后在手机上取回复现/修改。
step "内置源码包（assets/lancai-sentinel-src-v$VER_NAME.zip）"
mkdir -p "$BUILD/assets"
SRC_ZIP="$BUILD/assets/lancai-sentinel-src-v$VER_NAME.zip"
( cd "$(dirname "$ROOT")" && zip -q -r -9 "$SRC_ZIP" "$(basename "$ROOT")" \
    -x "*/build/*" "*/dist/*" "*/assets/*" "*/.git/*" )
RUST_NAME="$(basename "$RUST_CRATE")"
RUST_PARENT="$(cd "$RUST_CRATE/.." && pwd)"
( cd "$RUST_PARENT" && zip -q -r -9 "$SRC_ZIP" "$RUST_NAME" \
    -x "*/target/*" "*/data/*" "*/.git/*" )
# 仓库根目录的协作约定也一起带上（若存在）。
if [ -f "$ROOT/../AGENTS.md" ]; then
    ( cd "$ROOT/.." && zip -q -9 "$SRC_ZIP" AGENTS.md )
fi
[ -s "$SRC_ZIP" ] || fail "内置源码包生成失败"
cat > "$BUILD/assets/SOURCE_INFO.txt" <<EOF
澜彩哨兵 v$VER_NAME · 内置源码包（调试用）
================================================

文件：assets/lancai-sentinel-src-v$VER_NAME.zip
内容：Android 平台层（Java/res/清单/构建脚本/签名密钥）
      + Rust 核心（src/tests/samples/config.toml/CONFIG.md）
      + Cargo.lock，可离线复现本次构建。

提取：
    unzip -p lancai-sentinel-v$VER_NAME.apk assets/lancai-sentinel-src-v$VER_NAME.zip > src.zip
    unzip src.zip

重新构建：
    cd lancai-sentinel-rs && cargo test
    cd ../lancai-sentinel-android && ./build.sh

注意：
1. 包内含签名密钥 keys/lancai.jks，请勿公开分享本源码包。
2. 修改后发版需递增 VER_NAME / VER_CODE，并使用同一密钥签名，
   否则老用户必须先卸载旧版才能安装。
3. App 内「菜单 → 关于 → 版本与内核」可查看应用版本与 Rust 内核状态。
EOF

"$BT/aapt2" compile --dir "$APP/res" -o "$BUILD/res.zip"
# aapt2 link 的 --version-code/--version-name 在清单已写死时不生效，
# 这里生成一份临时清单强制写入 VER_*，保证「改环境变量就能发新版」。
MANIFEST="$BUILD/AndroidManifest.xml"
sed -e "s/android:versionCode=\"[0-9]*\"/android:versionCode=\"$VER_CODE\"/" \
    -e "s/android:versionName=\"[^\"]*\"/android:versionName=\"$VER_NAME\"/" \
    "$APP/AndroidManifest.xml" > "$MANIFEST"
"$BT/aapt2" link \
    -o "$BUILD/base.apk" \
    -I "$ANDROID_JAR" \
    -A "$BUILD/assets" \
    --manifest "$MANIFEST" \
    --java "$BUILD/gen" \
    --min-sdk-version 24 \
    --target-sdk-version 34 \
    --version-code "$VER_CODE" \
    --version-name "$VER_NAME" \
    --auto-add-overlay \
    "$BUILD/res.zip"

# ---------------------------------------------------------------- 3. Java
step "javac"
find "$APP/java" "$BUILD/gen" -name '*.java' | sort > "$BUILD/sources.txt"
javac -source 8 -target 8 -encoding UTF-8 -nowarn \
    -bootclasspath "$ANDROID_JAR" -classpath "$ANDROID_JAR" \
    -d "$BUILD/classes" "@$BUILD/sources.txt"

# ---------------------------------------------------------------- 4. dex
step "d8"
( cd "$BUILD/classes" && jar cf "$BUILD/classes.jar" . )
"$BT/d8" --release --min-api 24 --lib "$ANDROID_JAR" \
    --output "$BUILD/dex" "$BUILD/classes.jar"
[ -f "$BUILD/dex/classes.dex" ] || fail "d8 未生成 classes.dex"

# ---------------------------------------------------------------- 5. 打包
step "打包 APK（dex + native libs）"
cp "$BUILD/base.apk" "$BUILD/unaligned.apk"
( cd "$BUILD" && zip -q -X -j unaligned.apk dex/classes.dex )
( cd "$BUILD" && zip -q -X -0 -r unaligned.apk lib )

# ---------------------------------------------------------------- 6. 对齐 + 签名
step "zipalign"
"$BT/zipalign" -p -f 4 "$BUILD/unaligned.apk" "$BUILD/aligned.apk"

KS="$KEYS/lancai.jks"
PASS="CHANGE-ME-BEFORE-USE"
if [ ! -f "$KS" ]; then
    step "生成签名密钥 $KS"
    keytool -genkeypair -keystore "$KS" -alias lancai \
        -keyalg RSA -keysize 2048 -validity 12000 \
        -storepass "$PASS" -keypass "$PASS" -noprompt \
        -dname "CN=Lancai Sentinel, OU=Student, O=Lancai, L=Beijing, C=CN" >/dev/null
fi

APK="$DIST/lancai-sentinel-v$VER_NAME.apk"
rm -f "$APK"
step "apksigner sign"
"$BT/apksigner" sign \
    --ks "$KS" --ks-key-alias lancai \
    --ks-pass "pass:$PASS" --key-pass "pass:$PASS" \
    --v1-signing-enabled true --v2-signing-enabled true --v3-signing-enabled true \
    --out "$APK" "$BUILD/aligned.apk"

step "apksigner verify"
"$BT/apksigner" verify --verbose --print-certs "$APK" | sed -n '1,12p'

step "内容检查"
unzip -l "$APK" | grep -E "classes.dex|lib/.*/$SO_NAME" || fail "APK 内缺少 dex 或 native 库"

printf '\nAPK: %s\n' "$APK"
printf '大小: %s\n' "$(du -h "$APK" | cut -f1)"
printf 'SHA256: %s\n' "$(sha256sum "$APK" | cut -d' ' -f1)"

# ---------------------------------------------------------------- 7. 自动同步 GoFile
# 只上传本次构建且内容有变化的文件（按 SHA256 去重），并更新下载索引。
# 网络失败不影响构建产物；GOFILE_AUTO=0 可关闭。
if [ "${GOFILE_AUTO:-1}" != "0" ]; then
    SYNC_PY="$ROOT/../../release-server/gofile-sync.py"
    if [ -f "$SYNC_PY" ]; then
        step "同步到 GoFile"
        python3 "$SYNC_PY" "$APK" "$SRC_ZIP" || printf 'GoFile 同步失败，不影响本次产物\n'
    else
        printf '\n未找到 %s，跳过 GoFile 同步\n' "$SYNC_PY"
    fi
fi

KEEP="${KEEP_VERSIONS:-2}"

# ---------------------------------------------------------------- 8. 发布到 GitHub Releases
# 上传 APK 与源码包，并按版本只保留最近 KEEP 个 release。
# gh 未登录时自动跳过；GH_RELEASE=0 可关闭。
if [ "${GH_RELEASE:-1}" != "0" ]; then
    REL_SH="$ROOT/../../release-server/gh-release.sh"
    if [ -f "$REL_SH" ] && command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
        step "发布到 GitHub Releases"
        KEEP_VERSIONS="$KEEP" bash "$REL_SH" "$VER_NAME" "$APK" "$SRC_ZIP" \
            || printf 'GitHub Release 发布失败，不影响本次产物\n'
    else
        printf '\ngh 未登录或脚本缺失，跳过 GitHub Release\n'
    fi
fi

# ---------------------------------------------------------------- 9. 清理旧版本
# 只保留最近 KEEP 个版本（默认 2）的 APK 与源码包，避免产物无限堆积。
if [ "$KEEP" != "0" ]; then
    RETENTION_PY="$ROOT/../../release-server/retention.py"
    if [ -f "$RETENTION_PY" ]; then
        step "清理旧版本（保留最近 $KEEP 版）"
        python3 "$RETENTION_PY" "$DIST" --keep "$KEEP" || true
    fi
fi

printf 'BUILD_OK\n'
