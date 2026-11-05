# Thiết kế: Chip cảnh báo mưa lớn Vrain và Báo cáo Excel

Ngày 2026-11-05 · Chủ dự án: Đoàn Phước Triều · Trạng thái: **chờ duyệt** · Nối tiếp `2026-11-05-vrain-rainfall-design.md`

## 1. Mục đích
1. Cảnh báo mưa lớn ở thượng nguồn nổi bật trên thanh đầu, như cảnh báo thời tiết hiện có.
2. Xuất báo cáo Excel: lưu lượng về hồ theo **ngày, tháng, năm** kèm lượng mưa thượng nguồn của **Hương Nguyên và A Đớt**, để nộp báo cáo ca/tháng/năm mà không phải tính tay.

## 2. Đã chốt (chủ dự án, 2026-11-05)
- Thêm thư viện **`rust_xlsxwriter`** (MIT/Apache, thuần Rust, chỉ ghi .xlsx, Rust ≥ 1.83). Đây là ngoại lệ so với đặc tả mục 7.1, ghi lại trong `PLAN.md`.
- Báo cáo chỉ gồm 2 trạm: **Hương Nguyên, A Đớt** (Tà Lương vẫn hiện trên màn hình, không vào báo cáo).
- Chip hiện khi **một trong hai**: mưa 24 giờ ≥ ngưỡng ngày (mặc định 50 mm, dùng cùng ngưỡng thời tiết) **hoặc** mưa 1 giờ > 7,6 mm (mức "Mưa to").
- Báo cáo ở **tab "Báo cáo" riêng** (phím 6, cạnh tab Mưa), xuất **một file nhiều sheet**.

## 3. Chip cảnh báo mưa Vrain
- Xét 2 trạm báo cáo và cả Tà Lương? **Quyết định của tôi:** xét cả 3 trạm hiển thị (cảnh báo là an toàn, báo cáo là số liệu nộp).
- Chỉ hiện khi số liệu của trạm đó **mới**: mốc mới nhất cách hiện tại ≤ 3 giờ (tránh báo mưa của vài ngày trước sau khi mở lại app).
- Ưu tiên: nếu có trạm vượt ngưỡng 24 giờ thì nêu trạm có mm 24 giờ cao nhất ("Mưa lớn A Đớt: 63,4 mm / 24 giờ (Vrain)"); nếu không, nêu trạm có mưa 1 giờ cao nhất > 7,6 ("Mưa to Hương Nguyên: 9,2 mm / giờ (Vrain)").
- Cùng kiểu chip đỏ với chip thời tiết (icon cảnh báo + chữ); hai chip có thể cùng hiện. Bấm chip Vrain chuyển tới tab **Mưa**.

## 4. Báo cáo Excel
**Màn hình (tab Báo cáo):** ô "Từ ngày", "Đến ngày" (dd/mm/yyyy; mặc định từ 01/01 năm nay đến hôm nay), nút nhanh "Tháng này" và "Năm nay"; bảng xem trước theo **tháng** trong khoảng; nút **Xuất Excel**; sau khi xuất hiện đường dẫn file và nút "Mở thư mục". Kiểm tra: ngày không hợp lệ, "từ" sau "đến", "đến" ở tương lai, khoảng > 10 năm → báo lỗi tiếng Việt ngay tại ô.

**File `bao-cao-luu-luong-ve-<từ>-<đến>.xlsx`** (thư mục `exports` của dữ liệu app): 3 sheet **Ngày**, **Tháng**, **Năm**. Mỗi sheet có tiêu đề, khoảng thời gian, dòng tiêu đề cột, các dòng số liệu và dòng **Tổng/Trung bình** cuối. Cột:
| Cột | Ý nghĩa |
|---|---|
| Thời gian | nhãn kỳ (`05/11/2026`, `11/2026`, `2026`) |
| Khoảng tính | từ – đến thực sự được tính (kỳ đầu/cuối bị cắt theo khoảng chọn và theo hôm nay) |
| Lưu lượng về TB (m³/s) | bình quân theo thời gian của các chu kỳ tự động đã lưu |
| Tổng lượng về (m³) | lưu lượng TB × thời gian có số liệu |
| Có số liệu (%) | phần thời gian của kỳ có chu kỳ lưu lượng |
| Mưa Hương Nguyên (mm) | cộng các mốc giờ trong kỳ |
| Mưa A Đớt (mm) | như trên |
| Mưa TB 2 trạm (mm) | trung bình hai trạm; trống nếu thiếu một trạm |
| Có số liệu mưa (%) | trung bình tỷ lệ giờ có số liệu của hai trạm |
Quy tắc: số lưu dạng **số** (Excel tiếng Việt tự hiện dấu phẩy), định dạng `#,##0.00`; kỳ không có số liệu thì **ô trống**, không ghi 0; dòng cuối: Tổng lượng về = cộng; Q TB = bình quân theo thời gian; mưa = cộng. Cuối sheet có ghi chú: "Chỉ gồm thời gian app đã chạy và có số liệu; xem cột Có số liệu."

## 5. Không làm
Lưu lượng về lớn nhất/nhỏ nhất, biểu đồ trong Excel, mưa trạm nhà máy trong báo cáo, in/PDF, gửi email. Mưa Vrain không đi vào công thức lưu lượng về.

## 6. Hạn chế cần biết
- Báo cáo chỉ có số liệu từ lúc app chạy và có kết nối (lưu lượng về từ ECAPRO, mưa từ Vrain). **Năm/tháng cũ trước khi dùng app sẽ trống** — cột "Có số liệu (%)" cho biết độ đầy đủ.
- Vrain chỉ trả mưa 2 ngày gần nhất mỗi lần lấy bù, nên mưa của những ngày app không chạy sẽ thiếu.

## 7. Còn mở
Không có câu hỏi chặn. Mặc định khoảng báo cáo, tên file và thứ tự cột do tôi chọn (sửa được khi xem).
