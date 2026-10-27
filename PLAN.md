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
- [x] Nguồn thời tiết: Open-Meteo (best_match, NOAA GFS, ECMWF, DWD ICON) cạnh OpenWeather; mưa theo từng mô hình hiện trên thẻ ngày; cảnh báo khi một mô hình báo ≥ ngưỡng; nguồn dự phòng tự động; đã gọi API Open-Meteo thật (test `--ignored live`)
- [x] Crate `luuluong-storage`: config.toml, lịch sử SQLite, xuất CSV (UTF-8 BOM, `;`, thập phân `,`), API key trong Keychain, nạp bảng CSV/XLSX (đã thử với `tinhtoan.xlsx` cũ)
- [x] App: màn hình Lịch sử (lọc ngày, xóa có xác nhận, xuất CSV) và Cài đặt (tổ máy, nguồn, tọa độ, key, giao diện, bảng dung tích, mở thư mục dữ liệu); tự lưu lịch sử mỗi lần tính; 107 test pass
- [x] Nhiều nguồn cùng lúc: `fetch_all` (OpenWeather một luồng + một lần gọi Open-Meteo cho nhiều mô hình), mỗi nguồn có kết quả/lỗi riêng; bảng "So sánh nguồn" trên màn hình chính (mưa, xác suất, nhiệt độ theo ngày + hàng chênh lệch); Cài đặt chọn nguồn chính + tick nguồn so sánh; cache theo nguồn. Thay cho cơ chế nguồn dự phòng cũ (đã xóa `chain.rs`)
- [x] Thiết kế lại màn hình chính (skill ui-ux-pro-max): hai cột (trái: nhập + kết quả; phải: thẻ thời tiết có hiệu ứng, radar, 5 ngày), hàng dưới là biểu đồ cột so sánh nguồn + kết luận bằng lời; hiệu ứng và biểu tượng vẽ bằng code (nắng, trăng sao, mây, mưa, dông, sương mù); tùy chọn tắt hiệu ứng động trong Cài đặt
- [x] Radar mưa: RainViewer (6 khung, 10 phút/khung) trên nền OpenStreetMap, ghép 3×3 tile zoom 7 quanh nhà máy, phát lại tự động, cache tile 14 ngày; đã gọi dịch vụ thật (test `--ignored live_radar`). Bản release 6,5 MB
- [ ] CHƯA nhìn thấy bằng mắt: thẻ thời tiết/radar/biểu đồ mới, màn hình Lịch sử và Cài đặt (chỉ có test logic + build); cần chạy `cargo run -p app` và thử từng thẻ (cờ dev `--tab=history|settings`)
- [ ] Chưa thử lưu/đọc API key thật trong Keychain; chưa thử nạp bảng ngoài trên giao diện
### Giai đoạn 4b — Chỉnh bố cục theo `UI_LAYOUT_REDESIGN.md`
- [x] Đối chiếu đặc tả với màn hình đã làm: đã có (hai cột, radar, 5 ngày, biểu đồ, kết luận, bảng) → giữ; sửa (nút, nhóm xả nước, thẻ kết quả, trạng thái thời tiết, thẻ cảnh báo) → xong; bỏ qua (responsive điện thoại, spinner tính vì phép tính tức thời, ngưỡng cấu hình riêng vì đã dùng `rain_alert_mm`)
- [x] Sửa nhóm 3, 4, 5.1, 7.2 (test + clippy pass; CHƯA nhìn bằng mắt, cần `cargo run -p app`) — còn lại để xem sau khi duyệt hình: nav (2.1), nhập liệu + nút (3), kết quả (4), thẻ thời tiết/radar/dự báo (5–6), so sánh mưa + cảnh báo (7), trạng thái dữ liệu (10) — kiểm tra: `cargo test --workspace`, clippy sạch, T1–T6 pass, chụp ảnh 900×600 và 4K sáng/tối
- [x] Phong cách: Industrial 50% + Weather 25% + Swiss 15% + Editorial 10% (thẻ không viền, nhãn mục viết hoa, ô KPI, cảnh báo dạng nhận định có vạch dọc) — chưa xem bằng mắt
- [x] Gộp bố cục: hàng 1 = nhập | thời tiết+dự báo (một thẻ, cao bằng nhau), hàng 2 = kết quả, hàng 3 = radar | so sánh mưa (cao bằng nhau), hàng 4 = kết luận + bảng; biểu đồ có khung thông tin khi rê chuột (từng nguồn, chênh lệch, nhận định) — chưa xem bằng mắt
- [x] Tối ưu diện tích (A+B+D): hàng 1 = nhập | kết quả (cao bằng nhau, chi tiết đóng mặc định, 2 cột), thẻ thời tiết toàn chiều rộng (hero trái, 5 ngày phải), nút TÍNH to / XÓA dạng nút phụ, bỏ thanh trạng thái đáy (trạng thái thời tiết lên thanh trên, phiên bản vào Cài đặt), thẻ có viền + bóng nhẹ — test + clippy pass, CHƯA nhìn bằng mắt. Chưa làm: C (thẻ tab radar/so sánh/bảng), E (hero thấp, giảm fps, nút biểu tượng), F (Cài đặt 2 cột, chế độ compact)
- [x] Bố cục lần 2 (theo yêu cầu): điều hướng Tính toán/Lịch sử/Cài đặt thành thanh bên phải (tách khỏi tiêu đề), bỏ nút giao diện sáng/tối ở thanh trên (đổi trong Cài đặt), nhập + kết quả gộp một thẻ, 'Kết luận theo ngày' và 'Số liệu chi tiết' thành 2 thẻ riêng, biểu tượng thời tiết dùng ảnh Meteocons; sửa chiều cao thẻ và khung biểu đồ hiện nhầm — test + clippy pass, chờ xem bằng mắt
- [x] Bố cục lần 3: 3 tab ngay dưới tiêu đề (thay thanh bên phải), nền tương phản sau biểu tượng thời tiết (ô màu trời ở thẻ ngày, vòng tối ở hero), thẻ Kết luận + Số liệu chi tiết cùng một hàng (38/62, bảng cuộn ngang khi hẹp) — test + clippy pass, chờ xem bằng mắt
- [ ] Nghiệm thu theo mục 13 (bỏ mục JavaScript/điện thoại vì app native desktop)

### Giai đoạn 4c — Sửa 12 hạn chế sau review giao diện (2026-10-27)
Mỗi bước kiểm tra: `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, chạy `target/debug/app` rồi chụp màn hình.
- [x] B1 Icon vẽ bằng code (ok/cảnh báo/thông tin) thay glyph ✓ ⚠ ℹ; cũng bỏ → (chỉ có trong font Hack, hiện ô vuông) — kiểm tra: không còn glyph trong `crates/app/src`, ảnh chụp không có ô vuông rỗng
- [x] B2 Lưới nhập 3 cột cho nhóm giờ (Thời gian | H1 | H2) và nhóm xả (Q | giờ | tối thiểu); thẻ kết quả có khung rỗng giữ chỗ (—) — kiểm tra: ảnh chụp trước khi tính không còn ô lẻ, không còn nửa thẻ trống
- [x] B3 Hero: lớp nền tối cố định sau cụm chữ — kiểm tra: ảnh chụp, chữ rõ khi có mây/mưa
- [x] B4 Chip cảnh báo mưa lớn trên thanh trên (mọi tab), bấm để về Tính toán — kiểm tra: ngưỡng thử hạ thấp thì chip hiện
- [x] B5 Chữ tối thiểu 12 px — kiểm tra: `grep -n "11.0" crates/app/src` không còn cỡ chữ 11
- [x] B6 Bảng Số liệu chi tiết + Kết luận đặt ngay dưới thẻ thời tiết, biểu đồ/radar xuống dưới; chú thích biểu đồ không chỉ dựa vào rê chuột
- [x] B7 Hộp thoại theo palette (nút chính cam, nút Thoát màu lỗi, Esc đóng) và tab hiện phím tắt ngay trên nhãn
- [x] B8 Thẻ ngày cao bằng nhau khi hẹp; nhãn xác suất đổi thành "Khả năng mưa tối đa" (đặc tả 3: lấy `pop` lớn nhất, KHÔNG đổi công thức)
- [x] B9 Chụp lại 900×600 và 1100×720, sáng và tối (cờ dev `--size=WxH`, `--scroll=N`; HOME tạm để thử chế độ tối và ngưỡng cảnh báo thấp mà không đụng cấu hình thật). Đã thấy bằng mắt: thanh trên + chip, tab, nhập, kết quả rỗng, thời tiết, kết luận, bảng, radar, biểu đồ. CHƯA xem: Lịch sử, Cài đặt, hộp thoại, 4K, trạng thái lỗi mạng

- [x] B10 Sau khi người dùng chạy thử (ảnh toàn màn hình): bảng Số liệu chi tiết không bẻ dòng ô (đã chụp xác nhận); nút "Sao chép kết quả" canh trái thay vì kéo giãn (chưa chụp được vì cần nhập số — người dùng xác nhận)
- [x] B11 Theo phản hồi người dùng: lưới nhập một cột đồng nhất (nhãn | ô 140 px + đơn vị, gợi ý/lỗi dưới ô; cùng cho nhóm xả); Kết luận + Số liệu chi tiết về cuối trang (hoàn lại thứ tự cũ của B6); tab bỏ "Cmd+n" khỏi nhãn, phím tắt trong tooltip (đã chụp xác nhận lưới nhập và tab)
- [x] B12 Phương án A (người dùng chọn): `best_match` đổi tên "Tổng hợp", loại khỏi bảng/biểu đồ/kết luận/cảnh báo nguồn khác khi các mô hình riêng đang bật (`luuluong_weather::source::in_comparison`, 2 test mới); vẫn là nguồn chính được. Người dùng đã xóa `UI_LAYOUT_REDESIGN.md` (bỏ khỏi MAP.md; PLAN 4b vẫn nhắc tên file như lịch sử)
- [ ] Còn mở: thời điểm tính hiện dạng ISO (`2026-10-27T14:09:13+07:00`) → nên hiện `27/10/2026 14:09:13`; ảnh radar để dải trống bên trái khi thẻ rộng; chú giải biểu đồ che vạch ngưỡng 50 mm

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
- NOAA/NWS (`api.weather.gov`) trả 404 cho A Lưới (đã thử 2026-10-27) nên không dùng; NOAA GFS lấy qua Open-Meteo `gfs_seamless`. Các mô hình bất đồng lớn: dự báo mưa 28/10 là 6,0 (GFS) / 12,1 (ICON) / 40,9 (best_match) / 44,5 mm (ECMWF).
- Nguồn mặc định: chính `open-meteo:best_match`, so sánh cả 4 lựa chọn Open-Meteo (không cần key). OpenWeather phải tick trong Cài đặt. "Hiện tại" của Open-Meteo là một bộ chung cho cả lần gọi (API không tách theo mô hình). Open-Meteo miễn phí cho phi thương mại: cần xác nhận nhà máy có thuộc diện này không.
- CSV dùng `;` và thập phân `,` để Excel tiếng Việt mở đúng; lịch sử có thêm cột `operator` (đặc tả F6 yêu cầu "người nhập").
- Radar: RainViewer chỉ miễn phí cho cá nhân/giáo dục (cần xác nhận khi dùng ở nhà máy); OpenStreetMap yêu cầu ghi nguồn (đã ghi trên bản đồ) và nhận diện app qua User-Agent; không có khung dự báo (chỉ quan trắc 2 giờ gần nhất, app dùng 6 khung). Phủ sóng A Lưới đã kiểm (coverage mask + ảnh tile).
- "Tổng hợp" (Open-Meteo best_match) không phải mô hình độc lập nên không tính vào so sánh khi GFS/ECMWF/ICON bật (2026-10-27).
- Bảng màu biểu đồ Okabe–Ito (5 màu) + số trên đầu cột để không chỉ dựa vào màu; kết luận bằng lời: bất đồng lớn khi chênh ≥ nửa ngưỡng cảnh báo.
- Font Be Vietnam Pro không bật được số đều (tabular) trong egui; chữ số kết quả có thể lệch nhẹ.

## Việc cần chốt ở giai đoạn 2 (đã chốt: cam đậm `#B84A00` cho số kết quả giao diện sáng, 5,23:1)
- Chọn màu nhấn giao diện sáng đạt ≥ 4,5:1 cho số kết quả (xem contexte/thiet-ke-ui.md).
