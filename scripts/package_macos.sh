#!/bin/bash
# Đóng gói bản macOS universal: .app (ký tạm ad-hoc) trong .dmg, đặt vào livrables/.
# Chạy ở thư mục gốc dự án sau khi build cả hai target (đặc tả mục 9.1):
#   MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release -p app --target aarch64-apple-darwin
#   MACOSX_DEPLOYMENT_TARGET=11.0 cargo build --release -p app --target x86_64-apple-darwin
# Biểu tượng (tùy chọn): đặt file crates/app/assets/icon.icns trước khi chạy.
set -euo pipefail

VERSION=$(grep -m1 '^version' crates/app/Cargo.toml | cut -d'"' -f2)
OUT=livrables
APP="$OUT/stage/Luu luong Song Bo.app"
DMG="$OUT/luuluong-$VERSION-macos-universal.dmg"

rm -rf "$OUT/stage" "$DMG"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
lipo -create -output "$APP/Contents/MacOS/luuluong" \
  target/aarch64-apple-darwin/release/app \
  target/x86_64-apple-darwin/release/app

ICON_KEY=""
if [ -f crates/app/assets/icon.icns ]; then
  cp crates/app/assets/icon.icns "$APP/Contents/Resources/icon.icns"
  ICON_KEY="<key>CFBundleIconFile</key><string>icon</string>"
fi

cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleExecutable</key><string>luuluong</string>
  <key>CFBundleIdentifier</key><string>vn.songbo.luuluong</string>
  <key>CFBundleName</key><string>Luu luong Song Bo</string>
  <key>CFBundleDisplayName</key><string>Lưu lượng Sông Bồ</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
  <key>LSApplicationCategoryType</key><string>public.app-category.utilities</string>
  $ICON_KEY
</dict></plist>
PLIST

# Ký tạm (không cần tài khoản Apple Developer); chưa công chứng nên lần đầu phải mở bằng chuột phải > Open.
codesign --force --deep --sign - "$APP"
codesign --verify --deep --strict "$APP"

ln -s /Applications "$OUT/stage/Applications"
hdiutil create -volname "Luu luong Song Bo" -srcfolder "$OUT/stage" -ov -format UDZO "$DMG" >/dev/null
rm -rf "$OUT/stage"

echo "Đã tạo $DMG ($(du -h "$DMG" | cut -f1))"
