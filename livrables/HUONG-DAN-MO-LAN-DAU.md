# Cài đặt và mở lần đầu — Lưu lượng Sông Bồ

Bản này **chưa ký số** (dùng nội bộ nhà máy), nên máy sẽ cảnh báo ở lần mở đầu tiên. Chỉ cần làm theo các bước dưới một lần; các lần sau mở bình thường.

## macOS (11 trở lên, Apple Silicon và Intel)
1. Mở file `luuluong-<phiên bản>-macos-universal.dmg`, kéo **Luu luong Song Bo** vào thư mục **Applications**.
2. Vào Applications, **bấm chuột phải** vào ứng dụng → **Open** → bấm **Open** ở hộp thoại cảnh báo.
3. Nếu macOS vẫn chặn: **Cài đặt hệ thống → Quyền riêng tư & Bảo mật**, kéo xuống bấm **Vẫn mở** (Open Anyway) cạnh tên ứng dụng.

## Windows (10 trở lên)
1. Giải nén `luuluong-<phiên bản>-windows-x64.zip`, đặt `luuluong.exe` vào thư mục tùy ý.
2. Bấm đúp `luuluong.exe`. Nếu hiện **Windows protected your PC**: bấm **More info → Run anyway**.

## Dữ liệu và API key
- Lịch sử tính và cấu hình lưu trong thư mục dữ liệu của người dùng. Mở nhanh bằng **Cài đặt → Mở thư mục dữ liệu**.
- API key OpenWeather (nếu dùng) lưu trong Keychain (macOS) hoặc Credential Manager (Windows), không nằm trong file cấu hình. Nhập ở **Cài đặt**.
- Không có key vẫn dùng được: thời tiết lấy từ Open-Meteo, phần tính lưu lượng không cần mạng.
- Gỡ cài đặt: xóa ứng dụng (macOS kéo vào Trash; Windows xóa `luuluong.exe`). Dữ liệu người dùng giữ nguyên cho tới khi xóa thư mục dữ liệu.

## Khi gặp sự cố
Ghi lại ca nhập (mực nước đầu/cuối, giờ chạy máy) và giờ xảy ra rồi gửi cho người phụ trách. App không hiện lỗi kỹ thuật; chi tiết nằm trong log.
