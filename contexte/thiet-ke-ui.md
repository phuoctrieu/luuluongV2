# Thiết kế UI

- Nguồn chuẩn: đặc tả mục 6 (bố cục, palette sáng/tối, font Be Vietnam Pro/Inter, bước 8 px, bo 8 px).
- `design-system/luu-luong-song-bo/MASTER.md` do skill ui-ux-pro-max sinh: gợi ý "Data-Dense Dashboard", palette cam-trên-nền-tối, Fira Sans. Chỉ tham khảo; palette đặc tả 6.3 thắng.
- Việc nên áp từ skill: tương phản ≥ 4,5:1, focus rõ khi dùng bàn phím, số chữ số đều (tabular), trạng thái rỗng/tải/lỗi/ngoại tuyến, không dùng emoji làm icon, lỗi ngay dưới ô nhập.
- Lưu ý: nhiều quy tắc skill dành cho web/mobile (touch 44 px, breakpoint 375); app này là desktop co giãn 900×600 → 4K.

## Kiểm tra tương phản palette đặc tả 6.3 (đã đo 2026-10-27)
- Đạt ≥ 4,5:1: chữ/thẻ sáng 18,2; lỗi/thẻ sáng 5,6; chính/thẻ sáng 8,4; chữ trắng trên chính 8,4; tối: chữ 13,4, lỗi 5,7, nhấn 7,7, chính 6,4.
- **Không đạt: nhấn `#FF8C32` trên thẻ trắng (sáng) chỉ 2,32:1.** Dùng cam làm nền nút TÍNH với chữ navy `#06113C` (đo lại), còn số kết quả ở giao diện sáng phải dùng cam đậm hơn (ví dụ ≈ `#B84A00`, đo lại trước khi chốt) hoặc màu Chính.
