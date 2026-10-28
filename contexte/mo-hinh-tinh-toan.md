# Mô hình tính toán

Q = (ΔV + V_tuabin + V_xả) / (T·3600). Chi tiết, quy tắc tra bảng và ca thử T1–T6: đặc tả mục 3.
Ghi chú vận hành: q mỗi tổ máy mặc định 17,3 m³/s (cần xác nhận có đổi theo công suất không); chưa tính bốc hơi/thấm.

## Lưu lượng về theo chu kỳ từ data logger (giai đoạn 6)
- Mực nước hồ lấy từ ECAPRO GPIs6.6DA, kênh 16 "MNTL hien tai" (`crates/logger`; cách gọi: `huong-dan-lay-mntl-hien-tai.md`). Máy chạy app phải cùng mạng LAN `192.168.200.x`.
- Q về chu kỳ = ΔV/Δt + 1,5 m³/s (xả môi trường, cấu hình `logger.eco_q`); không tính tuabin, không xả tràn. Là `balance::inflow_between_levels` = `compute` với H1 = H2 = 0. Ví dụ: mực nước không đổi → Q = 1,5.
- Chu kỳ 15 hoặc 30 phút, mốc cố định theo đồng hồ (:00/:15/:30/:45). Mực nước mỗi mốc là trung vị các lần đọc trong 1 phút trước mốc (khử nhiễu 1 mm). Thiếu số liệu ở một mốc (mất kết nối, máy ngủ, mới mở app) thì chu kỳ đó không tính, không nội suy; đổi chu kỳ thì bắt đầu lại.
- Mực nước ngoài 149–165 m: từ chối như tính tay. Trạng thái 0 (không kết nối) coi như không có số liệu.
- Mỗi chu kỳ tính xong tự lưu một dòng lịch sử, ghi chú "Tự động từ thiết bị ECAPRO".

## Dữ liệu thật từ ECAPRO (curl ngày 2026-10-28)
- Cột trạng thái KHÔNG dùng để loại số đo: kênh 16 (mực nước) báo trạng thái 0 nhưng giá trị đúng (157,189, khớp kênh 9 và 15); kênh 7, 11, 14 cũng 0 mà vẫn có số. Mã chỉ coi 2/3 là cảnh báo thấp/cao. (Ghi chú "0 = không kết nối" trong `huong-dan-lay-mntl-hien-tai.md` không đúng cho các kênh này.) Hệ quả chưa xử lý: nếu thiết bị mất cảm biến mà vẫn trả số cũ thì app không phân biệt được.
- Kênh dùng: 16 mực nước hồ; 5 lưu lượng máy H1; 6 lưu lượng máy H2 (m³/s).
- Kênh có thể dùng đối chiếu sau (chưa dùng): 2 `DCTT|LUULUONG` = 1,5 m³/s (xả môi trường đo thật); 7 `NHAMAY|LUULUONG` (tổng nhà máy); 4 `LUU LUONG VE old` và 14 `LLVT new` (lưu lượng về do chính thiết bị tính, để so với số của app); 11 `DENHO|LUULUONG` = 18,04.
