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
- [x] B13 Bốn mục còn mở: thời điểm tính hiện `27/10/2026 14:09:13` (`view::display_time`, 2 test; chưa chụp được vì cần có kết quả); radar giới hạn khung theo vùng 768 px tile phủ và đặt giữa thẻ (hết dải trống); chú giải biểu đồ vẽ riêng phía trên khung vẽ; Cài đặt dùng lưới "nhãn | ô" như form Tính toán (đã chụp)
- [ ] Còn chưa xem bằng mắt: màn hình Lịch sử, hộp thoại, 4K, mất mạng, thẻ kết quả sau khi tính (nút Sao chép, thời điểm tính)

### Giai đoạn 5 — Đóng gói, CI
Theo đặc tả mục 9. Phiên bản lấy từ `Cargo.toml` (0.1.0). Kiểm tra mỗi bước ghi kèm.
- [x] D1 macOS: `livrables/luuluong-0.1.0-macos-universal.dmg` (8,2 MB), có biểu tượng giọt nước (`scripts/make_icon.py`). Đã kiểm: `lipo` có x86_64 + arm64, `codesign --verify` hợp lệ (ad-hoc), `minos 11.0` cả hai, mở `.app` từ bản đóng gói chạy và hiện dữ liệu thật. Script `package_macos.sh` tự cài Xcode CLT / Rust / hai target nếu thiếu rồi build + đóng gói (chạy được từ thư mục bất kỳ). Đo: RSS 142 MB sau 9 giây (mục tiêu đặc tả ≤ 100 MB — LỆCH, cần xem lại); thời gian khởi động chưa đo; chưa thử trên máy Intel thật
- [x] D2 Windows: CI build được `luuluong.exe` (PE32+ GUI x86-64, 7,2 MB; trong `.zip` 3,8 MB) có 7 ảnh biểu tượng + thông tin phiên bản (kiểm bằng cách đọc tài nguyên PE). `cargo test` pass trên Windows. CHƯA chạy `.exe` trên máy Windows thật và CHƯA chạy `build_windows.ps1` ở nhánh cài thiếu (CI đã có sẵn VS/Rust nên nhánh `winget` chưa được thử)
- [x] D4 GitHub `phuoctrieu/luuluongV2` (riêng tư, nhánh mặc định `dev`, không có `main`): thẻ `v0.1.0` → bản phát hành https://github.com/phuoctrieu/luuluongV2/releases/tag/v0.1.0 có `luuluong-0.1.0-macos-universal.dmg` (8 MB) và `luuluong-0.1.0-windows-x64.zip` (3,8 MB)
  - Lần CI 1 (2026-10-27): `cargo test` PASS trên macOS và Windows; `clippy` FAIL vì Rust 1.99.1 (CI dùng stable mới nhất) coi `Stroke::new(1.0, ..)` thiếu kiểu là lỗi — đã sửa 19 chỗ thành `1.0_f32` (sạch trên cả 1.87 và 1.99.1). Đã cài thêm toolchain 1.99.1 riêng (`cargo +1.99.1 clippy --target-dir <tạm>`) để tái hiện CI; bản mặc định vẫn 1.87
- [x] D3 Trang hướng dẫn mở lần đầu: `livrables/HUONG-DAN-MO-LAN-DAU.md`

### Giai đoạn 6 — Mực nước hồ hiện tại + lưu lượng về hiện tại (từ data logger ECAPRO)
Nguồn: `huong-dan-lay-mntl-hien-tai.md` (POST `http://192.168.200.211:8880/macros/UpdateMonitor`, kênh 16 "MNTL hien tai", cột 3 = giá trị, cột 5 = trạng thái 0/1/2/3). Q về chu kỳ = ΔV/Δt + 1,5 m³/s (xả môi trường; KHÔNG tính tuabin, H1 = H2 = 0) = `balance::compute` với `hours = chu kỳ/60`, `eco_q = 1,5`, không xả tràn. Công thức mục 3 giữ nguyên, chỉ cố định đầu vào. Mốc chu kỳ cố định theo đồng hồ.
Cấu trúc: tab mới **Hiện tại** (mặc định, hiện MNTL + Q về); tab **Tính toán** đổi tên **Tính bằng tay** (giữ nguyên form, thêm nút "Lấy mực nước từ thiết bị").
Không thêm dependency ngoài: tái dùng `ureq` (đã có ở `luuluong-weather`).

- [x] E1 Crate `crates/logger` (`luuluong-logger`, thêm vào workspace): `parse_mntl(text: &str) -> Result<Reading, LoggerError>` (`Reading { z: f64, status: Status }`, `Status` = Disconnected|Normal|Low|High) và `fetch(url: &str, timeout: Duration) -> Result<Reading, LoggerError>` — kiểm tra: test `parse_mntl` với dòng mẫu `160.8/ 157.2/ 1.0,16,MNTL hien tai,157.192,m,0,0,0` → z = 157,192; thiếu kênh 16 → lỗi; giá trị không phải số → lỗi; trạng thái 0 → `Disconnected`; test `fetch` bằng `mockito` (200 / 500 / timeout)
- [x] E2 `config.toml` thêm mục `[logger]` (`url` mặc định `http://192.168.200.211:8880/macros/UpdateMonitor`, `poll_seconds` = 5, `period_minutes` = 15, chọn 15|30, `eco_q` = 1.5) trong `crates/storage/src/config.rs` — kiểm tra: thiếu khóa dùng mặc định; chu kỳ khác 15/30 bị đưa về 15 (test); ô sửa URL + chu kỳ trong màn Cài đặt (phần Cài đặt làm cùng E5)
- [x] E3 Lõi: `luuluong_core::balance::inflow_between_levels(t, z_prev, z_now, minutes, eco_q) -> Result<Outputs, CalcError>` (bọc `compute` với H1 = H2 = 0, không xả tràn) — kiểm tra: test z_prev = z_now → Q = `eco_q` = 1,5 (ΔV = 0 vẫn có nước về để bù xả môi trường); ca 160,0→160,1 trong 15 phút: Q = ΔV/900 + 1,5; ca 160,0→160,1 trong 15 phút khớp `compute` với `hours = 0,25`; z ngoài 149–165 → từ chối; T1–T6 vẫn pass
- [x] E4 App: luồng nền đọc thiết bị mỗi `poll_seconds` (mẫu `app/src/schedule.rs` + `mpsc`), không chặn UI; lưu `Sample { time, z }` vòng 2 giờ; mốc chu kỳ cố định theo đồng hồ (:00/:15/:30/:45 hoặc :00/:30) — tại mỗi mốc chốt `z_mốc` (trung vị 12 mẫu cuối để khử nhiễu 1 mm) — kiểm tra: test thuần (không GUI) cho `LevelTracker`: sau 2 mốc có `z_prev`, `z_now`; mất kết nối giữa chu kỳ thì chu kỳ đó không có Q (hiện "thiếu số liệu"), không nội suy (phần logic `LevelTracker` ở `crates/logger/src/tracker.rs`; luồng nền gắn vào app ở E5)
- [x] E5 Tab **Hiện tại**: thẻ lớn MNTL (định dạng Việt `157,192 m`, giờ đọc cuối, chấm trạng thái bằng icon vẽ code + chữ, không chỉ màu), thẻ "Lưu lượng về chu kỳ vừa qua" (m³/s, từ mốc nào đến mốc nào, ΔZ), công tắc chu kỳ 15/30 phút, dòng ghi chú "đã cộng 1,5 m³/s xả môi trường, không tính tuabin" (không có ô nhập); nhãn "dữ liệu cũ" khi quá 3×`poll_seconds` không đọc được; lỗi kết nối chỉ hiện câu tiếng Việt ("Không kết nối được thiết bị ECAPRO"), chi tiết ghi log — kiểm tra: chụp ảnh 900×600 sáng/tối với (a) bình thường, (b) mất kết nối, (c) chưa đủ 2 mốc; tương phản ≥ 4,5:1
- [x] E6 Tab **Tính bằng tay**: đổi tên `Tab::Calc` → hiển thị "Tính bằng tay" (tooltip phím tắt, chip cảnh báo mưa trỏ về tab đúng); nút "Lấy mực nước hiện tại" điền ô Z cuối từ lần đọc mới nhất, nút "Lấy mực nước chu kỳ trước" điền Z đầu từ mốc trước; vẫn sửa tay được — kiểm tra: test `form.rs` điền ô từ `Reading`; đọc không được thì báo ngắn gọn, giữ nguyên ô đang nhập
- [x] E7 Lưu lịch sử: mỗi mốc có Q về ghi một dòng vào SQLite (cột `source` = "logger" / "tay" để phân biệt), xuất CSV như cũ — kiểm tra: test `history.rs` ghi/đọc/xuất CSV có cột mới; CSV cũ vẫn mở được
- [ ] E8 Cập nhật `MAP.md`, `contexte/mo-hinh-tinh-toan.md` (cách tính Q chu kỳ + ví dụ số), nghiệm thu: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, thử trên mạng LAN thật `192.168.200.x` (chưa thử được nếu không ở nhà máy) — kiểm tra: ghi rõ phần đã/chưa thử thật

### Giai đoạn 7 — Sau khi chạy thật (2026-10-28): trung bình dài hạn, trạm mưa, tách tab Tính bằng tay
Chủ dự án đã chạy app với ECAPRO thật, số mực nước và lưu lượng về chu kỳ 11:00 đúng. Ba yêu cầu:
- [x] H1 Trạm đo mưa: đọc kênh 1 `LUU LUONG MUA` (mm/h) vào `Reading.rain`, hiện trong thẻ trực tuyến — kiểm tra: test parse (số thật 8,0 / 2,0; thiếu kênh → None; không nhầm kênh 10–16)
- [x] H2 Lưu lượng về trung bình 3 giờ, 12 giờ, 1 ngày, 1 tuần, 1 tháng (30 ngày), 1 năm (365 ngày): bình quân theo thời gian các chu kỳ tự động trong lịch sử (Σ W / Σ thời gian), kèm tỷ lệ thời gian có số liệu — kiểm tra: test thuần `window_averages` (chu kỳ lẫn 15/30 phút, khoảng trống, ngoài cửa sổ, dòng tính tay không tính), thử trên `history.db` thật
- [x] H3 Hiển thị rõ chu kỳ đang dùng ("Chu kỳ 15 phút: 10:45 – 11:00") và mô tả cách app khởi động lại — kiểm tra: chụp ảnh (đã chụp bản thử: chu kỳ ghi "Chu kỳ 15 phút: …")
- [x] H4 Tách `Tính bằng tay` thành tab riêng (Tính toán | Tính bằng tay | Lịch sử | Cài đặt, phím 1–4); thẻ trực tuyến + thời tiết ở tab Tính toán — kiểm tra: chụp ảnh, Cmd+Enter chỉ tính khi đang ở tab Tính bằng tay
- [x] H5 Cập nhật `contexte`, `MAP.md`; chạy test + clippy

### Giai đoạn 8 — Mưa cộng dồn và biểu đồ theo ngày (yêu cầu 2026-10-28)
- [x] I1 Lượng mưa trung bình mỗi chu kỳ từ kênh 1 (`CycleMean.rain_mm_h`, `push_rain`); thiếu mưa không làm chu kỳ thiếu lưu lượng — test tracker (3), live (2)
- [x] I2 Cột `rain_mm` (nullable) trong `calc`: di trú DB cũ bằng `ALTER TABLE`, CSV thêm cột "Lượng mưa (mm)" — test di trú, roundtrip, CSV
- [x] I3 `window_averages` cộng dồn mưa theo khoảng (cắt tỷ lệ chu kỳ vắt qua đầu khoảng), `daily_stats` theo ngày giờ Việt Nam (chu kỳ vắt qua nửa đêm chia hai ngày) — 14 test
- [x] I4 Giao diện: hàng "Mưa cộng dồn" trong bảng trung bình, thẻ "Lưu lượng về và mưa theo ngày" (7/14/30 ngày, hai biểu đồ cột, rê chuột xem chi tiết) — chụp ảnh bản thử
- [x] I5 Tài liệu; `cargo test --workspace`, clippy sạch

### Giai đoạn 6c — Xả môi trường đo từ kênh 2 (yêu cầu 2026-10-28)
Lưu lượng về vẫn tự tính từ mực nước (không dùng kênh 4/14 của thiết bị). Xả môi trường lấy từ kênh 2 `DCTT|LUULUONG`, trung bình trong chu kỳ, thay hằng số 1,5; thiếu kênh 2 thì chu kỳ "thiếu số liệu". Đã bỏ `eco_q` khỏi `[logger]` và Cài đặt (file cũ còn khóa này vẫn nạp được). Đã sửa lỗi coi trạng thái 0 là mất kết nối (dữ liệu thật). 
- [x] G1 logger/tracker/live/lịch sử/cài đặt dùng kênh 2 — test pass, clippy sạch
- [x] G2 Đối chiếu 10:30 (thiết bị giả, trạng thái 0 ở kênh 16): app 68,89 m³/s = số tự tính từ nhật ký 68,89 (ΔZ +0,050; H1 16,54; H2 8,81; xả MT 1,666). Sửa ký tự → thiếu glyph thành "–"

### Giai đoạn 6b — Đọc lưu lượng tổ máy (kênh 5, 6) và gộp vào màn Tính toán (yêu cầu 2026-10-28)
Thay đổi so với 6: logger có kênh 5 "LUU LUONG MAY 1" (H1) và 6 "LUU LUONG MAY 2" (H2), đơn vị m³/s (ví dụ H1 chạy = 16,54; hai máy ≈ 33). Tự nhận biết máy chạy/dừng; Q về PHẢI tính tuabin (thay cho "không tính tuabin" ở trên). Mực nước + lưu lượng về nằm chung màn **Tính toán**, không tách tab riêng.
Công thức mục 3 giữ nguyên: `V_tuabin = (H1·Q1 + H2·Q2)·3600` với H = giờ trong chu kỳ, Q = lưu lượng đo trung bình trong chu kỳ (số đo thay cho hằng số 17,3); cộng thêm xả môi trường 1,5.
- [x] F1 logger: `Reading` thêm `q_h1`, `q_h2` (`Option<f64>`, None nếu thiếu kênh hoặc không phải số; trạng thái 0 vẫn hợp lệ, sửa sau khi có dữ liệu thật); `RUNNING_MIN_Q` = 5,0 m³/s (chủ dự án: 1–3 m³/s là nhiễu cảm biến; đã sửa từ 1,0): lớn hơn thì máy "đang chạy", không thì lưu lượng máy = 0 — kiểm tra: test parse đúng mẫu của người dùng (16,54 và 0), thiếu kênh 5/6 → None, kênh 16 vẫn là mực nước
- [x] F2 core: `inflow_between_levels(..., eco_q, q_h1, q_h2)` — kiểm tra: mực nước không đổi, H1 = 16,54 → Q = 18,04; hai máy 33,08 → 34,58; T1–T6 vẫn pass
- [x] F3 tracker: trung bình lưu lượng mỗi máy và tỷ lệ thời gian chạy trong chu kỳ; thiếu phủ (< 80 % chu kỳ) hoặc không có dữ liệu tổ máy → chu kỳ "thiếu số liệu" — kiểm tra: test trung bình, máy dừng giữa chu kỳ, thiếu dữ liệu
- [x] F4 `live.rs` dùng số liệu tổ máy; lịch sử ghi giờ chạy H1/H2 thực tế — kiểm tra: test Flow có Q tuabin, `history_record` ghi h1/h2
- [x] F5 Giao diện: bỏ tab Hiện tại; màn **Tính toán** có thẻ trực tuyến ở trên (mực nước, H1/H2 chạy/dừng + lưu lượng, lưu lượng về chu kỳ, chọn 15/30 phút), form tính tay bên dưới — kiểm tra: chụp ảnh 900×600 sáng/tối với 0, 1, 2 máy chạy bằng thiết bị giả

Đã chốt (2026-10-28, phần xả môi trường và mốc đồng hồ vẫn đúng; phần tuabin bị 6b thay), chỉ tính xả môi trường 1,5 m³/s; mốc chu kỳ cố định theo đồng hồ. Cần xác nhận: 1,5 m³/s được CỘNG vào ΔV/Δt (nước xả đi nên lượng về lớn hơn mực nước tăng), không phải trừ.

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
