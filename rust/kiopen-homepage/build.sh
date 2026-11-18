#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"

TARGET=x86_64-unknown-linux-musl
cargo build --release --target "$TARGET"

BIN="target/$TARGET/release/kiopen-homepage"
STAGE="build/kiopen-homepage"

rm -rf build
mkdir -p "$STAGE/DEBIAN" \
         "$STAGE/usr/bin" \
         "$STAGE/usr/share/applications" \
         "$STAGE/usr/share/icons/hicolor/scalable/apps" \
         "$STAGE/usr/share/doc/kiopen-homepage"

install -m 755 "$BIN" "$STAGE/usr/bin/kiopen-homepage"
install -m 644 packaging/kiopen-homepage.desktop "$STAGE/usr/share/applications/"
install -m 644 packaging/kiopen-homepage.svg "$STAGE/usr/share/icons/hicolor/scalable/apps/"
install -m 644 README.md "$STAGE/usr/share/doc/kiopen-homepage/"

SIZE=$(du -sk "$STAGE" | cut -f1)
sed "s/__SIZE__/$SIZE/" packaging/control > "$STAGE/DEBIAN/control"

dpkg-deb --build --root-owner-group "$STAGE" "kiopen-homepage_2.0.0_amd64.deb"
echo "已生成 kiopen-homepage_2.0.0_amd64.deb"
