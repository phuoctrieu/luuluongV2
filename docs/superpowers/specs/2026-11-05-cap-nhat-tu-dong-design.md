# Thiết kế: Cập nhật ứng dụng từ GitHub (OTA)

Ngày 2026-11-05 · Chủ dự án: Đoàn Phước Triều · Trạng thái: **chờ duyệt**

## 1. Mục đích
App tự biết khi có bản mới trên GitHub Releases và cập nhật được bằng một nút, không phải tải `.dmg`/`.zip` rồi cài tay trên từng máy (phòng điều khiển Windows và máy Mac của giám đốc). Đặc tả gốc mục 9.3 để việc này ở P2 ("trước mắt kiểm tra thủ công").

## 2. Đã chốt (chủ dự án, 2026-11-05)
- **Mức tự động:** app báo "Có bản mới", chỉ tải và cài khi người dùng bấm. Không bao giờ tự cài.
- **Toàn vẹn:** so SHA-256 của file tải về với mã `digest` mà GitHub trả trong API bản phát hành (qua HTTPS). Không có chữ ký riêng.
- **Giải nén:** thêm thư viện `zip` (cho cả macOS và Windows); kèm `sha2`. Hai ngoại lệ mới so với đặc tả mục 7.1 (ngoài `rust_xlsxwriter`, `rfd`).
- **Khi kiểm tra:** mỗi lần mở app nếu lần kiểm tra trước cách ≥ 24 giờ, cộng nút "Kiểm tra cập nhật" trong Cài đặt.

## 3. Hiện trạng đã kiểm
Kho `phuoctrieu/luuluongV2` **công khai** (không cần token), đã có bản `v0.1.0`, `v0.2.0` kèm `digest: sha256:…` do GitHub tính. Tài sản hiện tại: `luuluong-<ver>-macos-universal.dmg` và `luuluong-<ver>-windows-x64.zip` (chứa `luuluong.exe`). Bản macOS chỉ ký tạm (ad-hoc), chưa công chứng.

## 4. Luồng
1. Mở app (hoặc bấm nút): luồng nền gọi `GET https://api.github.com/repos/phuoctrieu/luuluongV2/releases/latest` (bỏ qua bản nháp/tiền phát hành). So `tag_name` với phiên bản đang chạy (`CARGO_PKG_VERSION`) theo số `x.y.z`.
2. Có bản mới hơn: hiện chip **"Có bản mới 0.3.0"** (màu thông tin, không đỏ) trên thanh đầu, bấm chuyển tới Cài đặt; thẻ "Cập nhật" trong Cài đặt hiện phiên bản hiện tại, phiên bản mới, ghi chú phát hành (văn bản thuần, cắt ~600 ký tự), nút **"Tải và cài đặt"** và nút "Mở trang tải" (phương án dự phòng).
3. Bấm "Tải và cài đặt" → hộp xác nhận: "Cài đặt sẽ khởi động lại app trong vài giây. Lịch sử và cài đặt giữ nguyên. Số liệu trực tuyến bị gián đoạn, chu kỳ đang chạy sẽ ghi 'thiếu số liệu'. Nên cài ngay sau một mốc chu kỳ."
4. Tải về thư mục tạm (thanh tiến trình), tính SHA-256 trong lúc tải, so với `digest`; sai hoặc thiếu `digest` → **hủy, xóa file, không cài**.
5. Giải nén vào thư mục tạm (chặn đường dẫn thoát khỏi thư mục đích), tìm đúng gói.
6. Thay thế bằng **đổi tên** (nhanh, quay lui được): bản cũ → `.old`, bản mới → vị trí cũ, mở bản mới rồi thoát. Lần chạy sau xóa `.old` và thư mục tạm còn sót.
   - **macOS:** thay cả `Luu luong Song Bo.app` (phải thay nguyên gói để chữ ký ad-hoc còn hợp lệ); tài sản mới `luuluong-<ver>-macos-universal.app.zip`.
   - **Windows:** đổi tên `luuluong.exe` đang chạy thành `luuluong.old.exe` (Windows cho đổi tên file đang chạy), đặt `luuluong.exe` mới vào, mở lại.
7. Không ghi được vào thư mục cài (vd. `/Applications` của tài khoản không phải quản trị, `Program Files`), hoặc đang chạy từ `cargo run` (không phải `.app`/`.exe` đã cài): **không làm gì dở dang**; báo câu tiếng Việt và gợi ý "Mở trang tải".

## 5. Thay đổi phát hành (CI)
`scripts/package_macos.sh` thêm `ditto -c -k --keepParent` ra `luuluong-<ver>-macos-universal.app.zip`; `release.yml` đính kèm cả `.app.zip` (vẫn giữ `.dmg` cho lần cài đầu). Windows giữ nguyên. Nhắc: `Cargo.toml` phải tăng phiên bản trước khi gắn thẻ, và thẻ phải là `v<phiên bản>`.

## 6. Cấu hình
`config.toml` thêm `[update]`: `auto_check = true`, `last_check = 0` (giây epoch lần kiểm tra thành công gần nhất). Cài đặt có công tắc "Tự kiểm tra khi mở app".

## 7. Không làm
Cài ngầm không hỏi; tải nền sẵn; chữ ký số của app; kênh beta; quay về phiên bản cũ từ giao diện; cập nhật dữ liệu/bảng dung tích. Không dùng token GitHub.

## 8. Rủi ro và hạn chế cần biết
- **Bản đầu có tính năng này (vd. 0.3.0) vẫn phải cài tay** trên mọi máy; từ bản sau mới tự cập nhật được.
- Chỉ SHA-256 do GitHub cung cấp: chống tải hỏng/bị can thiệp trên đường truyền, **không chống** trường hợp tài khoản GitHub bị chiếm quyền phát hành. Nếu cần mức đó sau này, thêm chữ ký ed25519.
- GitHub API không đăng nhập giới hạn 60 lượt/giờ/IP; app chỉ gọi tối đa 1 lần/ngày + khi bấm nút, nên đủ.
- Chưa thể thử toàn trình (relaunch, quyền thư mục) bằng test tự động; cần phát hành một bản thử (vd. 0.2.1) và cập nhật từ bản cũ trên cả macOS và Windows (mục 5 kế hoạch).
- Thao tác phát hành (đẩy thẻ, tạo release) là việc ra bên ngoài: tôi sẽ không làm khi chưa được bạn bảo.

## 9. Còn mở
Không có câu hỏi chặn. Chữ trên giao diện, ngưỡng 24 giờ, giới hạn ghi chú 600 ký tự do tôi chọn.
