#!/bin/bash
# Build + đóng gói bản macOS universal: .app (ký tạm ad-hoc) trong .dmg, đặt vào livrables/.
# Tự kiểm tra và cài các thành phần còn thiếu (Xcode Command Line Tools, Rust, hai target Apple).
# Chạy từ bất kỳ thư mục nào:  ./scripts/package_macos.sh
# Biểu tượng: crates/app/assets/icon.icns (tạo bằng scripts/make_icon.py).
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== Kiểm tra và cài các thành phần còn thiếu =="
# 1. Xcode Command Line Tools: trình liên kết, lipo, codesign, hdiutil.
if ! xcode-select -p >/dev/null 2>&1; then
  echo "Chưa có Xcode Command Line Tools: mở hộp thoại cài đặt. Cài xong hãy chạy lại script này."
  xcode-select --install || true
  exit 1
fi
# 2. Rust (qua rustup).
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
if ! command -v cargo >/dev/null 2>&1; then
  echo "Chưa có Rust: cài bằng rustup (https://rustup.rs)..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  . "$HOME/.cargo/env"
fi
# 3. Hai target Apple Silicon + Intel (không làm gì nếu đã có).
rustup target add aarch64-apple-darwin x86_64-apple-darwin

echo "== Build hai kiến trúc (macOS 11 trở lên) =="
export MACOSX_DEPLOYMENT_TARGET=11.0
cargo build --release --locked -p app --target aarch64-apple-darwin
cargo build --release --locked -p app --target x86_64-apple-darwin

echo "== Đóng gói =="
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
