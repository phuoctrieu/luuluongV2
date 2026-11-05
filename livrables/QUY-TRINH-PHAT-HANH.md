# Quy trình phát hành (để tính năng "Cập nhật" trong app hoạt động)

App tự tìm bản mới ở trang **Releases** của kho `phuoctrieu/luuluongV2` và chỉ nhận **bản phát hành chính thức** (không phải bản nháp hay "pre-release"). Làm đúng các bước dưới đây thì mọi máy đã cài bản có tính năng cập nhật sẽ thấy chip "Có bản mới".

## Các bước
1. **Tăng phiên bản** trong `crates/app/Cargo.toml` (`version = "0.3.0"`). App so số này với `tag` của bản phát hành, nên hai số phải khớp.
2. Chạy `cargo test --workspace` và `cargo clippy --workspace -- -D warnings`; commit và đẩy lên nhánh `dev`.
3. **Gắn thẻ** đúng dạng `v<phiên bản>`, ví dụ `v0.3.0` (không `v0.3`, không `0.3.0-beta`; thẻ sai dạng thì app bỏ qua):
   ```
   git tag v0.3.0
   git push origin v0.3.0
   ```
   *(Đẩy thẻ là việc ra bên ngoài; chủ dự án tự làm.)*
4. Chờ GitHub Actions (`release`) chạy xong. Workflow tự build hai nền tảng và đính vào bản phát hành:
   - `luuluong-0.3.0-macos-universal.dmg` — cho lần cài đầu trên Mac.
   - `luuluong-0.3.0-macos-universal.app.zip` — gói mà tính năng cập nhật tải về (macOS).
   - `luuluong-0.3.0-windows-x64.zip` — lần cài đầu **và** gói cập nhật (Windows).
5. **Kiểm tra bản phát hành** trên GitHub: có đủ 3 file trên, tên **đúng mẫu** (app tìm theo đuôi `-macos-universal.app.zip` và `-windows-x64.zip`), và mỗi file có dòng `sha256:` (GitHub tự tính; app từ chối gói không có mã này).
6. Bản phát hành **không** đánh dấu "pre-release", không để ở trạng thái nháp.

## Thử cập nhật trước khi dùng thật
Cài bản cũ (có tính năng cập nhật) trên một máy, phát hành bản mới theo các bước trên, rồi mở bản cũ: chip "Có bản mới" hiện ở thanh đầu → Cài đặt → "Tải và cài đặt". Sau khi app mở lại ở bản mới: lịch sử và cài đặt còn nguyên; lần mở kế tiếp thư mục `.old` bị dọn. Thử cả macOS và Windows, thử cắt mạng giữa lúc tải, và thử bản cài ở thư mục không ghi được (app phải báo lỗi tiếng Việt, bản cũ vẫn chạy).

## Lưu ý
- **Bản đầu tiên có tính năng này (0.3.0) vẫn phải cài tay** trên từng máy; từ bản sau đó mới tự cập nhật.
- macOS: app chỉ ký tạm (ad-hoc). Gói cập nhật được tạo bằng `ditto --norsrc --noextattr` và app bỏ qua file metadata `._*` khi giải nén; nếu đổi cách đóng gói, kiểm lại bằng `codesign --verify --deep --strict` trên bản giải nén.
- Mức bảo vệ: app so SHA-256 của gói tải về với mã do GitHub cung cấp. Không chống được việc tài khoản GitHub bị chiếm quyền phát hành; hãy bật xác thực hai lớp cho tài khoản.
- App kiểm tra tối đa một lần mỗi ngày khi mở, và khi bấm "Kiểm tra cập nhật" trong Cài đặt.
