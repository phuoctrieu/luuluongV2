# Thiết kế: Mưa quan trắc Vrain (3 trạm A Lưới)

Ngày 2026-11-05 · Chủ dự án: Đoàn Phước Triều · Trạng thái: **chờ duyệt**

## 1. Mục đích
Đưa lượng mưa thực đo của 3 trạm Vrain ở A Lưới (Hương Nguyên, A Đớt, Tà Lương) vào app, để:
1. **So sánh** với mưa trạm nhà máy (ECAPRO kênh 1) — phát hiện cảm biến lệch.
2. **Theo dõi mưa thượng nguồn** — mưa ở A Lưới báo trước nước về hồ.
Phụ: mưa của Vrain nằm cùng chỗ với số liệu lưu lượng về để người vận hành xem một màn hình.

## 2. Phạm vi
**Làm:** crate mới đọc Vrain; lưu SQLite; thẻ tóm tắt ở tab Tính toán; tab "Mưa" riêng; xuất CSV; cài đặt tài khoản.
**Không làm:** lấy cả 55 trạm; chạy nền khi app đóng; tự đổi công thức lưu lượng về (mục 3 đặc tả gốc giữ nguyên — mưa Vrain chỉ hiển thị, không đi vào Q); dự báo từ Vrain.
**Phản mục tiêu:** app không được đơ, không được ghi số sai khi Vrain đổi cấu trúc, không để lộ mật khẩu.

## 3. Quyết định của chủ dự án
- Mục đích: so sánh với trạm nhà máy + mưa thượng nguồn A Lưới.
- Chu kỳ lấy: **1 giờ** (`_1h`), lưu SQLite.
- Hiển thị: **cả hai** — thẻ tóm tắt ở tab Tính toán và tab "Mưa" riêng.
- Xác thực: đã dò bằng Chrome và script; dùng tài khoản dùng chung hiện có.

## 4. Sự thật đã xác minh về API
Xem `contexte/vrain-api.md` (nguồn chuẩn). Tóm tắt: đăng nhập `POST /api/vrain/public/v1/login` {username,password} → cookie `sid`; mưa theo giờ `GET /api/kttv/private/v1/organizations/details?from&to&i=_1h` → `stats[].{timePoint, stations[]}`; `depth` là chuỗi mm; `timePoint` dạng `HH:mm dd/MM` không có năm; mốc đầu là "Tổng". Spec cũ `vrain-rainfall-spec.md` sai ở phần "token".

## 5. Thiết kế

### 5.1 Crate `crates/vrain` (`luuluong-vrain`)
- `parse_details(json, năm_mốc) -> Result<Vec<Obs>>`: lọc 3 trạm theo `id`; bỏ "Tổng"; ép `depth` sang số; đổi `timePoint` thành giờ Việt Nam đầy đủ (suy năm: mốc không được ở tương lai quá 1 giờ, ngày/tháng lệch năm thì lùi một năm). JSON sai cấu trúc → lỗi, **không ghi gì**.
- `Client`: đăng nhập, giữ cookie, gọi `details`; 401 thì đăng nhập lại một lần; timeout 15 giây; thử lại tối đa 3 lần giãn dần.
- Luồng nền (mẫu `logger/poller.rs`): chạy khi mở app rồi mỗi 60 phút, nút làm mới tay (tối đa 1 lần/30 giây), gửi kết quả qua `mpsc`.
- Chỉ dùng `ureq` (đã có) và cookie giữ thủ công; **không thêm dependency**. Nếu `ureq` không đủ quản lý cookie thì hỏi lại.
- Mật khẩu: đọc từ Keychain (`storage/secret.rs`); khi chạy thử lấy từ `.env`. Không vào config, log, URL.

### 5.2 Lưu trữ (`crates/storage`)
Bảng `rain_obs`: `station_id TEXT, ts TEXT (ISO 8601, +07:00, MỐC CUỐI giờ), mm REAL, interval TEXT`, khóa chính `(station_id, ts, interval)`; ghi `INSERT OR REPLACE` (số mốc giờ hiện tại sẽ được cập nhật khi đủ giờ). Di trú giống `rain_mm` (tạo bảng nếu thiếu). Xuất CSV `;` thập phân `,` như các CSV khác, tên `vrain_mua_YYYY-MM.csv`. Cấu hình `[vrain]` trong `config.toml`: `enabled`, `username`, `refresh_minutes = 60`, `org_uuid`, danh sách 3 trạm (id + tên hiển thị).

### 5.3 Giao diện
**Thẻ "Mưa quan trắc thượng nguồn" (tab Tính toán, dưới thẻ trực tuyến):** 3 ô trạm. Mỗi ô: tên, mưa **1 giờ qua / 3 giờ / 24 giờ** (mm), mức mưa bằng chữ ("Không mưa / Mưa nhỏ / Vừa / To") + chấm màu (không chỉ dựa vào màu). Dòng dưới: "Trạm nhà máy (ECAPRO): x mm/h" để so sánh; giờ cập nhật; nhãn **"dữ liệu cũ"** khi quá 2 giờ không lấy được. Mưa ≥ ngưỡng cảnh báo ngày (cùng ngưỡng thời tiết, chỉnh trong Cài đặt) đổi màu cảnh báo.

**Tab "Mưa" (phím 5; Lịch sử/Cài đặt dời thành 6, 7 — *xem mục 7*):**
- Bảng 3 trạm × (1h, 3h, 6h, 12h, 24h, 7 ngày), thêm hàng ECAPRO.
- Biểu đồ cột theo giờ (24/48/72 giờ) hoặc theo ngày (7/14/30 ngày) với 3 chuỗi trạm + chuỗi ECAPRO, bảng màu Okabe–Ito, số trên đầu cột, rê chuột xem chi tiết.
- Dòng kết luận tự động: "3 trạm đồng thuận" / "Trạm nhà máy lệch so với trung bình 3 trạm Vrain" khi chênh ≥ nửa ngưỡng cảnh báo.
- Nút xuất CSV, nút làm mới, trạng thái (đang tải / lỗi / ngoại tuyến) đều có thiết kế riêng.

**Cài đặt:** mục "Vrain": bật/tắt, tài khoản, mật khẩu (Keychain), nút "Thử đăng nhập", chu kỳ làm mới.

### 5.4 Lỗi (đúng nguyên tắc mục 8 đặc tả gốc)
Mạng/401/5xx → dùng dữ liệu đã lưu kèm nhãn; sai tài khoản → "Tài khoản Vrain không đúng, kiểm tra trong Cài đặt"; JSON đổi cấu trúc → ghi log, dừng cập nhật Vrain, **phần tính lưu lượng không ảnh hưởng**. Chi tiết kỹ thuật chỉ ghi log.

### 5.3b Nút "Cập nhật ngay" (chủ dự án yêu cầu 2026-11-05)
- Có ở **thẻ tóm tắt** (tab Tính toán) và **tab Mưa**, cùng một hành vi: lấy `details` ngay, không chờ chu kỳ 60 phút.
- Khi đang tải: nút khóa, hiện "Đang cập nhật…"; xong hiện "Cập nhật lúc HH:MM"; lỗi thì giữ số cũ kèm câu tiếng Việt ngắn.
- Bấm liên tiếp: chặn 30 giây từ lần bấm trước, nút hiện "Thử lại sau N giây" (tránh gây tải cho Vrain; ngưỡng đặt trong mã, không thành cấu hình).
- Chưa cấu hình tài khoản: nút dẫn tới Cài đặt → Vrain.
- Phím tắt: Cmd/Ctrl + Shift + R (Cmd/Ctrl + R đã dùng để làm mới thời tiết).
- Kiểm: test thuần cho bộ hạn chế 30 giây và máy trạng thái nút (rảnh → đang tải → rảnh/lỗi); chụp ảnh các trạng thái.

## 6. Kiểm chứng (từng bước kèm cách kiểm)
1. Parser: test trên mẫu thật rút gọn (`tests/data/vrain_details.json` chỉ giữ 3 trạm, không chứa dữ liệu nhạy cảm): mốc "Tổng" bị bỏ; `depth` chuỗi → số; năm suy đúng qua giao thừa; thiếu trạm → cảnh báo, không lỗi; cấu trúc sai → lỗi.
2. Client với `mockito`: 200, 401 → đăng nhập lại 1 lần, 500, timeout, JSON sai.
3. Lưu trữ: ghi trùng không nhân đôi, cập nhật mốc giờ hiện tại, di trú DB cũ, CSV.
4. Tổng hợp (hàm thuần trong `app`): cộng dồn 1h/3h/24h từ các mốc giờ, mốc thiếu → "thiếu số liệu", không nội suy.
5. Giao diện: chụp 900×600 sáng/tối với (a) bình thường, (b) mất mạng, (c) mưa lớn, (d) chưa có dữ liệu; tương phản ≥ 4,5:1.
6. Thử thật: chạy app với tài khoản thật, đối chiếu một mốc giờ với trang vrain.vn.
`cargo test --workspace` và `cargo clippy --workspace -- -D warnings` sạch; T1–T6 vẫn pass.

## 7. Quyết định của tôi (chủ dự án xem lại)
- **Mốc `timePoint` = cuối giờ** (đã xác minh bằng J0; còn đối chiếu với ECAPRO khi có mưa).
- Lần đầu mở app lấy bù tối đa **3 ngày** (`from` = hôm kia); API trả nhiều ngày (đã xác minh J0).
- Phím tắt tab: thêm tab "Mưa" làm **tab thứ 5** để không đổi phím 1–4 đang dùng (Lịch sử và Cài đặt giữ nguyên).
- Ngưỡng cảnh báo dùng lại ngưỡng mưa ngày của thời tiết (50 mm).
- Mức mưa bằng chữ lấy từ chính Vrain khi có (`level` ở `stats/summary`); nếu chỉ có `details` thì tự phân loại theo mm/giờ (ngưỡng nêu trong kế hoạch).
- Mưa Vrain **không** đi vào công thức lưu lượng về.

## 8. Còn mở (chỉ chủ dự án trả lời được)
1. ~~Tài khoản dùng chung có được dùng cho app tự động không?~~ **Đã chốt (2026-11-05): chủ dự án xác nhận được dùng chung tài khoản.** Điều khoản Vrain về lấy dữ liệu tự động vẫn chưa đọc; giữ chu kỳ 60 phút và nút Cập nhật ngay cách nhau ≥ 30 giây để nhẹ tải.
2. Có dùng thêm các trạm khác ở A Lưới (Hồng Vân, Nhâm, Hồng Thái, Hồ A Lá…) sau này không — nếu có, danh sách trạm nên là cấu hình ngay từ đầu (thiết kế đã để sẵn).
