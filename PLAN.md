# PLAN — Ứng dụng lưu lượng Sông Bồ (Rust)

## Mục tiêu
Bản Rust khớp bản Python ≤ 0,01 m³/s, ≤ 15 MB, chạy macOS + Windows (đặc tả mục 1, 10).

## Các bước (kèm cách kiểm tra)
### Giai đoạn 1 — Lõi tính toán
- [x] Tạo Cargo workspace + crate `core` (package `luuluong-core`) — kiểm tra: `cargo build`
- [x] Chuyển `tinhtoan.xlsx` → CSV 125 dòng, sắp tăng dần — kiểm tra: test nạp bảng, cao trình tăng dần
- [x] `reservoir.rs` nội suy + `balance.rs` + `validate.rs` — kiểm tra: `cargo test` pass T1–T6
- [x] Ca đối chiếu với bản Python — 50 ca sinh tự động pass; CÒN THIẾU 30–50 ca thật của người vận hành (đặc tả 10.1)
### Giai đoạn 2 — Giao diện tính toán (egui)
### Giai đoạn 3 — Thời tiết
### Giai đoạn 4 — Lịch sử, CSV, Cài đặt
### Giai đoạn 5 — Đóng gói, CI

## Đang làm
Giai đoạn 1 xong về mặt mã (25 test pass, clippy sạch). Cổng giai đoạn 1 còn chờ ca thật từ người vận hành. Tiếp theo: giai đoạn 2 (egui).

## Quyết định đã chốt
- GUI: egui/eframe; HTTP: ureq; Lịch sử: rusqlite bundled; API key: keyring.
- Gói OpenWeather miễn phí là mặc định (chờ xác nhận mục 10.4).
- Palette giữ navy + cam theo đặc tả 6.3.

- Crate đặt tên `luuluong-core` vì tên `core` trùng thư viện chuẩn Rust (làm hỏng macro thiserror); thư mục vẫn là `crates/core`.
- Bảng dung tích mặc định để ở `crates/core/assets/` (không phải `app/assets/`) để `core` nhúng và test được.
- Bản Python cũ nằm ở `../luuluong/` (giaodien.py, tinhtoan.xlsx). Bản cũ có 2 lỗi đã biết: bỏ dòng 165 m, và trả 0 khi mực nước đầu = cuối; ca đối chiếu loại các trường hợp này.

## Việc cần chốt ở giai đoạn 2
- Chọn màu nhấn giao diện sáng đạt ≥ 4,5:1 cho số kết quả (xem contexte/thiet-ke-ui.md).
