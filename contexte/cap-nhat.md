# Cập nhật ứng dụng từ GitHub (OTA) — điều đã học (2026-11-05)

Thiết kế: `docs/superpowers/specs/2026-11-05-cap-nhat-tu-dong-design.md`; kế hoạch: `docs/superpowers/plans/2026-11-05-cap-nhat-tu-dong.md`; quy trình phát hành: `livrables/QUY-TRINH-PHAT-HANH.md`. Mã: `crates/update` (logic), `crates/app/src/{update_state,update_screen}.rs` (giao diện).

## Luồng
`check` (GitHub API `releases/latest`) → `parse_release` (chỉ nhận bản chính thức, gói đúng tên theo nền tảng, `digest` bắt buộc dạng `sha256:`, URL đúng kho) → người dùng bấm + xác nhận → `install_release`: tạo thư mục giải nén CẠNH bản cài (kiểm quyền ghi trước khi tải) → tải qua `.part`, kiểm SHA-256 và kích thước → `unpack` (kiểm hết mục trước khi ghi) → `swap` bằng đổi tên (`X.app` → `X.app.old`, hoặc `luuluong.exe` → `luuluong.old.exe`), lỗi thì quay lui → app mở lại bằng `open -n` / chạy exe. Lần mở sau `cleanup` xóa `.old`.

## Bài học
- **`ditto -c -k` mặc định thêm file `._*` (AppleDouble) vào zip** khi file có thuộc tính mở rộng (vd. `com.apple.provenance`). Giải nén bằng trình đọc zip thường làm các file này lọt vào trong `.app` và **phá niêm phong chữ ký** (`codesign`: "a sealed resource is missing or invalid") — bản cập nhật sẽ bị macOS từ chối. Đã xử lý hai tầng: tạo zip bằng `ditto --norsrc --noextattr --noqtn`, và `unpack` bỏ qua `._*`/`__MACOSX`. Phải thay NGUYÊN gói `.app` (chữ ký ký tạm nằm trong gói), không thay riêng file chạy.
- **Chốt dải phiên bản dependency mới** (`zip = "7"`, `sha2 = "0.11"`, `rust_xlsxwriter = "0.96"`, `rfd = "0.17"`): khai báo `>=` làm build ngoài workspace (không có `Cargo.lock`) kéo `zip` 9.x đổi API (`name()` trả `Result`) và không biên dịch được.
- `zip::ZipFile::enclosed_name()` chỉ gọt dấu `/` đầu chứ không từ chối đường dẫn tuyệt đối; `unpack` tự từ chối tên bắt đầu bằng `/`, chứa `\`, hoặc là liên kết tượng trưng.
- Tên tài sản bắt buộc đúng mẫu: `luuluong-<ver>-macos-universal.app.zip`, `luuluong-<ver>-windows-x64.zip`. Thẻ bắt buộc dạng `v<x.y.z>`; phiên bản trong `crates/app/Cargo.toml` phải khớp thẻ.
- GitHub trả sẵn `digest: sha256:…` cho từng tài sản trong API bản phát hành, nên không cần file checksum riêng.

## Chưa thử thật
Toàn trình trên máy thật (mở lại app, quyền thư mục, Windows đổi tên exe đang chạy) cần một bản phát hành thử. Bản có tính năng này (0.3.0) phải cài tay.
