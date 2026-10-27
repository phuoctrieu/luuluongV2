# Thiết kế UI

- Nguồn chuẩn: đặc tả mục 6 (bố cục, palette sáng/tối, font Be Vietnam Pro/Inter, bước 8 px, bo 8 px).
- `design-system/luu-luong-song-bo/MASTER.md` do skill ui-ux-pro-max sinh: gợi ý "Data-Dense Dashboard", palette cam-trên-nền-tối, Fira Sans. Chỉ tham khảo; palette đặc tả 6.3 thắng.
- Việc nên áp từ skill: tương phản ≥ 4,5:1, focus rõ khi dùng bàn phím, số chữ số đều (tabular), trạng thái rỗng/tải/lỗi/ngoại tuyến, không dùng emoji làm icon, lỗi ngay dưới ô nhập.
- Lưu ý: nhiều quy tắc skill dành cho web/mobile (touch 44 px, breakpoint 375); app này là desktop co giãn 900×600 → 4K.

## Kiểm tra tương phản palette đặc tả 6.3 (đã đo 2026-10-27)
- Đạt ≥ 4,5:1: chữ/thẻ sáng 18,2; lỗi/thẻ sáng 5,6; chính/thẻ sáng 8,4; chữ trắng trên chính 8,4; tối: chữ 13,4, lỗi 5,7, nhấn 7,7, chính 6,4.
- **Không đạt: nhấn `#FF8C32` trên thẻ trắng (sáng) chỉ 2,32:1.** Dùng cam làm nền nút TÍNH với chữ navy `#06113C` (đo lại), còn số kết quả ở giao diện sáng phải dùng cam đậm hơn (ví dụ ≈ `#B84A00`, đo lại trước khi chốt) hoặc màu Chính.

## Bài học egui (rút ra 2026-10-27, khi xem ảnh chụp thật)
- Ký tự ngoài tiếng Việt chỉ an toàn nếu có trong Be Vietnam Pro, Ubuntu-Light, NotoEmoji hoặc emoji-icon-font. `→` chỉ có trong Hack (monospace) nên hiện ô vuông; dùng `/` hoặc `–`. Biểu tượng trạng thái dùng `paint::note` / `paint::Icon` (vẽ bằng nét), không dùng glyph ✓ ⚠ ℹ.
- `ui.allocate_ui` kế thừa layout của cha: trong `horizontal_top` nội dung thẻ chảy NGANG. Dùng `allocate_ui_with_layout(.., Layout::top_down(Align::Min), ..)`.
- `ui.columns` dùng layout canh đều hai bên: nhãn dài xuống dòng bị giãn chữ. Bọc nhãn trong `with_layout(Layout::top_down(Align::Min), ..)`.
- Thẻ cùng hàng cao bằng nhau: lưu chiều cao khung trước (`store_height`); mọi thẻ phải có cùng viền (viền trong suốt khi không cảnh báo) nếu không chiều cao tăng dần mỗi khung.
- Chụp ảnh kiểm tra: `target/debug/app --size=1100x900 --scroll=1100 --tab=history`; chạy với `HOME` tạm + `config.toml` riêng để thử giao diện tối/ngưỡng mưa mà không đụng cấu hình thật. Chụp theo id cửa sổ (JXA `CGWindowListCopyWindowInfo`), không chụp cả màn hình.
- Chữ nhỏ nhất 12 px; chữ trên nền màu lỗi dùng `on_error` (trắng sáng, navy tối).

