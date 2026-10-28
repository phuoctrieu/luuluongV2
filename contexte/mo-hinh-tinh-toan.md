# Mô hình tính toán

Q = (ΔV + V_tuabin + V_xả) / (T·3600). Chi tiết, quy tắc tra bảng và ca thử T1–T6: đặc tả mục 3.
Ghi chú vận hành: q mỗi tổ máy mặc định 17,3 m³/s (cần xác nhận có đổi theo công suất không); chưa tính bốc hơi/thấm.

## Lưu lượng về theo chu kỳ từ data logger (giai đoạn 6)
- Lưu lượng về LUÔN tự tính từ mực nước đầu/cuối chu kỳ, không lấy số "lưu lượng về" thiết bị tính sẵn (kênh 4, 14) — chủ dự án chốt 2026-10-28.
- Số đo lấy từ ECAPRO GPIs6.6DA (`crates/logger`; cách gọi: `huong-dan-lay-mntl-hien-tai.md`): kênh 16 mực nước hồ, kênh 5/6 lưu lượng máy H1/H2, kênh 2 xả môi trường (m³/s). Máy chạy app phải cùng mạng LAN `192.168.200.x`.
- Q về chu kỳ = ΔV/Δt + Q_H1 + Q_H2 + Q_xả môi trường, với Q là trung bình các lần đọc trong chu kỳ. Là `balance::inflow_between_levels` = `compute` với giờ chạy = cả chu kỳ, lưu lượng tổ máy = số đo. Ví dụ: mực nước không đổi, H1 = 16,54, xả 1,5 → Q = 18,04. Máy chạy khi lưu lượng > 5,0 m³/s (chạy thật ≈ 16,5); số đọc nhỏ hơn như 1–3 m³/s là nhiễu cảm biến: máy coi là dừng và lưu lượng tính bằng 0 (không cộng nhiễu vào lưu lượng về). Xả môi trường không lọc vì giá trị thật vốn nhỏ (≈ 1,5).
- Chu kỳ 15 hoặc 30 phút, mốc cố định theo đồng hồ (:00/:15/:30/:45). Mực nước mỗi mốc là trung vị các lần đọc trong 1 phút trước mốc (khử nhiễu 1 mm).
- Thiếu số liệu thì chu kỳ đó không tính, không nội suy, không giả định bằng 0: mất mực nước ở một mốc, thiếu kênh 5/6/2, dữ liệu phủ dưới 80 % chu kỳ, máy ngủ, mới mở app. Mới qua mốc đầu thì "đang chờ" mốc thứ hai (không phải lỗi). Đổi chu kỳ thì bắt đầu lại.
- Mực nước ngoài 149–165 m: từ chối như tính tay.
- Mỗi chu kỳ tính xong tự lưu một dòng lịch sử (giờ chạy H1/H2 thực tế, xả môi trường đo được), ghi chú "Tự động từ thiết bị ECAPRO".

## Khởi động lại, trung bình dài hạn, trạm mưa (2026-10-28)
- Thiết bị chỉ trả giá trị hiện tại, không có lịch sử, nên lúc app tắt thì không có số liệu và app KHÔNG tự bù. Mở lại: bộ nhớ chu kỳ bắt đầu từ đầu, cần hai mốc đồng hồ liên tiếp (chờ tối đa 15–30 phút) mới có lưu lượng về mới. Các chu kỳ đã tính trước đó nằm trong lịch sử SQLite và vẫn dùng cho trung bình.
- Trung bình 3 giờ, 12 giờ, 1 ngày, 1 tuần, 1 tháng (30 ngày), 1 năm (365 ngày) = bình quân theo thời gian các chu kỳ tự động đã lưu (`crates/app/src/averages.rs`), chỉ tính phần thời gian có số liệu; kèm tỷ lệ "có số liệu" của khoảng. Dòng tính tay không tính. Hai dòng chồng một chu kỳ thì giữ dòng lưu trước.
- Trạm đo mưa: kênh 1 `LUU LUONG MUA` (mm/h), chỉ hiển thị cường độ hiện tại (chưa cộng dồn).
- Màn hình: tab Tính toán (số liệu trực tuyến + thời tiết), tab Tính bằng tay (nhập tay), Lịch sử, Cài đặt.

## Dữ liệu thật từ ECAPRO (curl ngày 2026-10-28)
- Cột trạng thái KHÔNG dùng để loại số đo: kênh 16 (mực nước) báo trạng thái 0 nhưng giá trị đúng (157,189, khớp kênh 9 và 15); kênh 7, 11, 14 cũng 0 mà vẫn có số. Mã chỉ coi 2/3 là cảnh báo thấp/cao. (Ghi chú "0 = không kết nối" trong `huong-dan-lay-mntl-hien-tai.md` không đúng cho các kênh này.) Hệ quả chưa xử lý: nếu thiết bị mất cảm biến mà vẫn trả số cũ thì app không phân biệt được.
- Kênh dùng: 16 mực nước hồ; 5 lưu lượng máy H1; 6 lưu lượng máy H2; 2 `DCTT|LUULUONG` = 1,5 m³/s (xả môi trường đo thật).
- Kênh có thể dùng đối chiếu sau (chưa dùng): 7 `NHAMAY|LUULUONG` (tổng nhà máy); 4 `LUU LUONG VE old` và 14 `LLVT new` (lưu lượng về do chính thiết bị tính, để so với số của app); 11 `DENHO|LUULUONG` = 18,04.
