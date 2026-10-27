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
- [x] Crate `app`: `form.rs` (kiểm tra theo ô, 9 test), `theme.rs` (sáng/tối, font Be Vietnam Pro), `main.rs` (màn hình Tính toán, phím tắt, xác nhận thoát)
- [ ] Người vận hành dùng thử và đồng ý giao diện (cổng giai đoạn 2); chưa kiểm tay luồng nhập → TÍNH → kết quả, 900×600 và 4K, sáng/tối
- [ ] Đồ thị đường đặc tính (P1, `egui_plot`) — chưa làm
### Giai đoạn 3 — Thời tiết
- [x] Crate `luuluong-weather`: gộp 3 giờ → ngày, cache JSON, `FreeProvider` (/weather + /forecast), ánh xạ lỗi 401/429/5xx/timeout/JSON sai; test bằng `mockito`. Tên trường JSON đã đối chiếu tài liệu OpenWeather ngày 2026-10-27
- [x] App: luồng nền + `mpsc`, lịch làm mới (60 phút, thử lại 5 phút, lùi dần khi 429, thủ công ≥ 30 giây, Cmd/Ctrl+R), dải dự báo, cảnh báo mưa ≥ 50 mm, nhãn "dữ liệu cũ", cache khi mở app
- [ ] Thử với API key thật (đặt `OPENWEATHER_API_KEY`) — chưa làm, chưa gọi API thật lần nào
- [ ] Cổng giai đoạn 3: chạy ổn định 3 ngày liên tục, kể cả ngắt mạng — chưa làm
- [ ] Biểu tượng thời tiết (hiện chỉ có chữ mô tả); ngưỡng mưa chỉnh trong Cài đặt (giai đoạn 4)
### Giai đoạn 4 — Lịch sử, CSV, Cài đặt
### Giai đoạn 5 — Đóng gói, CI

## Đang làm
Giai đoạn 1 xong về mặt mã, còn chờ ca thật. Giai đoạn 2: màn hình Tính toán chạy được và đã kiểm tay ca 160,0→161. Giai đoạn 3: mã xong, 58 test pass, chưa thử API thật.

## Quyết định đã chốt
- GUI: egui/eframe; HTTP: ureq; Lịch sử: rusqlite bundled; API key: keyring.
- Gói OpenWeather miễn phí là mặc định (chờ xác nhận mục 10.4).
- Palette giữ navy + cam theo đặc tả 6.3.

- Crate đặt tên `luuluong-core` vì tên `core` trùng thư viện chuẩn Rust (làm hỏng macro thiserror); thư mục vẫn là `crates/core`.
- Bảng dung tích mặc định để ở `crates/core/assets/` (không phải `app/assets/`) để `core` nhúng và test được.
- Bản Python cũ nằm ở `../luuluong/` (giaodien.py, tinhtoan.xlsx). Bản cũ có 2 lỗi đã biết: bỏ dòng 165 m, và trả 0 khi mực nước đầu = cuối; ca đối chiếu loại các trường hợp này.

- eframe/egui 0.32.3 (không phải 0.37): máy đang dùng Rust 1.87, bản 0.37 cần 1.95. `rust-version = "1.87"` + resolver 3 để Cargo tự chọn bản tương thích. Nâng Rust rồi nâng eframe sau nếu muốn.
- Font Be Vietnam Pro không bật được số đều (tabular) trong egui; chữ số kết quả có thể lệch nhẹ.

## Việc cần chốt ở giai đoạn 2 (đã chốt: cam đậm `#B84A00` cho số kết quả giao diện sáng, 5,23:1)
- Chọn màu nhấn giao diện sáng đạt ≥ 4,5:1 cho số kết quả (xem contexte/thiet-ke-ui.md).
