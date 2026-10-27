# ĐẶC TẢ CHỈNH SỬA BỐ CỤC GIAO DIỆN
## Ứng dụng NM TĐ Sông Bồ – Lưu lượng về hồ

**Phiên bản:** 1.0  
**Phạm vi:** Tối ưu giao diện người dùng (UI/UX)  
**Định hướng:** Dashboard thủy văn hiện đại, trực quan, phù hợp với công tác theo dõi và vận hành nhà máy thủy điện.

---

# 1. MỤC TIÊU

Chỉnh sửa bố cục giao diện hiện tại nhằm đạt các mục tiêu:

1. Hiển thị thông tin quan trọng rõ ràng, giúp người dùng nắm bắt nhanh tình hình.
2. Giảm chiều cao các biểu mẫu và hạn chế thao tác cuộn trang không cần thiết.
3. Phân cấp rõ ràng giữa dữ liệu đầu vào, kết quả tính toán, thông tin thời tiết và cảnh báo.
4. Cải thiện biểu đồ so sánh lượng mưa giữa các nguồn dự báo.
5. Bổ sung cách hiển thị trạng thái cập nhật và chất lượng dữ liệu.
6. Tối ưu khả năng hiển thị trên máy tính, máy tính bảng và điện thoại.
7. Duy trì phong cách thiết kế hiện đại, chuyên nghiệp, phù hợp với môi trường kỹ thuật.

## 1.1. Nguyên tắc bắt buộc

Đây là công việc cải tiến giao diện, không phải xây dựng lại ứng dụng.

- Giữ nguyên toàn bộ thuật toán tính toán lưu lượng về hồ.
- Giữ nguyên cấu trúc dữ liệu và cơ sở dữ liệu hiện tại.
- Giữ nguyên các API và nguồn cung cấp dữ liệu thời tiết.
- Giữ nguyên chức năng lịch sử tính toán và cài đặt.
- Giữ nguyên các trường nhập liệu và điều kiện kiểm tra dữ liệu.
- Không tự ý thay đổi công thức tính toán.
- Không thay đổi đơn vị đo lường.
- Không tạo dữ liệu giả để thay thế dữ liệu thực.
- Không loại bỏ chức năng hiện có.
- Không thay đổi các quy tắc nghiệp vụ khi chưa được yêu cầu.

Trước khi chỉnh sửa, cần kiểm tra cấu trúc dự án và xác định các thành phần giao diện đang được sử dụng.

Ưu tiên tái sử dụng component, CSS và logic hiện có. Không viết lại toàn bộ ứng dụng nếu không cần thiết.

---

# 2. BỐ CỤC TỔNG THỂ

## 2.1. Thanh điều hướng

Duy trì thanh điều hướng phía trên với nền xanh dương đậm.

Bố cục:

```text
┌────────────────────────────────────────────────────────────────────┐
│ NM TĐ SÔNG BỒ   Tính toán   Lịch sử   Cài đặt       [Giao diện tối] │
└────────────────────────────────────────────────────────────────────┘
```

Yêu cầu:

- Tên ứng dụng nằm bên trái.
- Các mục điều hướng nằm cạnh tên ứng dụng.
- Mục đang được chọn phải có trạng thái trực quan rõ ràng.
- Nút chuyển giao diện sáng/tối nằm bên phải.
- Chiều cao thanh điều hướng khoảng 48–56 px.
- Giữ thanh điều hướng gọn, không chiếm nhiều diện tích theo chiều dọc.
- Trên điện thoại, các mục điều hướng cần được thu gọn hợp lý.

Không thay đổi đường dẫn hoặc hành vi của các mục điều hướng hiện tại.

## 2.2. Bố cục trang tính toán

Trên màn hình máy tính, sử dụng bố cục hai cột:

```text
┌────────────────────────────────────────────────────────────────────┐
│                         THANH ĐIỀU HƯỚNG                           │
├────────────────────────────────┬───────────────────────────────────┤
│                                │                                   │
│  1. THÔNG SỐ TÍNH TOÁN         │  3. THỜI TIẾT A LƯỚI             │
│                                │                                   │
│  - Mực nước đầu                │  - Thời tiết hiện tại             │
│  - Mực nước cuối               │  - Thời điểm cập nhật              │
│  - Thời gian tính              │  - Trạng thái dữ liệu              │
│  - Giờ chạy máy H1             │                                   │
│  - Giờ chạy máy H2             │  4. RADAR MƯA                     │
│                                │                                   │
│  2. THÔNG SỐ XẢ NƯỚC           │  - Bản đồ radar                   │
│                                │  - Thanh thời gian                 │
│  - Lưu lượng xả tràn           │  - Nút phát/tạm dừng               │
│  - Số giờ xả tràn              │                                   │
│  - Lưu lượng xả tối thiểu      │  5. DỰ BÁO THỜI TIẾT              │
│                                │                                   │
│  [TÍNH] [XÓA DỮ LIỆU]          │  - Dự báo từng ngày                │
│                                │                                   │
├────────────────────────────────┴───────────────────────────────────┤
│                                                                    │
│  6. KẾT QUẢ TÍNH TOÁN LƯU LƯỢNG VỀ HỒ                             │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  7. SO SÁNH DỰ BÁO MƯA GIỮA CÁC NGUỒN                             │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  8. KẾT LUẬN TỪNG NGÀY VÀ BẢNG SỐ LIỆU CHI TIẾT                   │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Quy tắc bố cục

- Cột trái chiếm khoảng 50% chiều rộng vùng nội dung.
- Cột phải chiếm khoảng 50% chiều rộng vùng nội dung.
- Có thể sử dụng tỷ lệ 52/48 hoặc 50/50 tùy theo kích thước màn hình.
- Khoảng cách giữa hai cột: 12–16 px.
- Khoảng cách giữa các thẻ nội dung: 12–16 px.
- Các thẻ có cùng hệ thống bo góc, đường viền và khoảng đệm.
- Kết quả tính toán phải dễ nhận biết nhưng không làm mất cân đối bố cục.
- Khu vực so sánh dự báo mưa và bảng dữ liệu chi tiết sử dụng toàn bộ chiều rộng trang.

Không cố định chiều cao các thẻ nếu việc này làm cắt nội dung hoặc tạo khoảng trống không cần thiết.

---

# 3. THIẾT KẾ LẠI KHU VỰC NHẬP THÔNG SỐ

## 3.1. Nhóm A – Thông số tính toán

Hiển thị các trường nhập liệu theo hai cột:

```text
┌────────────────────────────────────────────────┐
│ THÔNG SỐ TÍNH TOÁN                             │
├────────────────────────┬───────────────────────┤
│ Mực nước đầu           │ Mực nước cuối          │
│ [160.00          ] m   │ [161.00          ] m  │
│                        │                       │
│ Thời gian tính         │ Giờ chạy máy H1        │
│ [2               ] giờ│ [2               ] giờ│
│                        │                       │
│ Giờ chạy máy H2        │                       │
│ [2               ] giờ│                       │
└────────────────────────┴───────────────────────┘
```

Đây chỉ là minh họa bố cục. Không ghi cứng các giá trị ví dụ vào chương trình.

Yêu cầu:

- Nhãn nằm phía trên trường nhập.
- Đơn vị đo nằm cạnh trường nhập hoặc trong trường nhập nếu phù hợp.
- Các trường có chiều rộng đồng đều.
- Các trường cùng nhóm có khoảng cách nhất quán.
- Giữ nguyên các giới hạn giá trị và thông báo kiểm tra hiện có.
- Thông báo lỗi xuất hiện gần trường nhập tương ứng.
- Không sử dụng màu sắc quá nổi bật cho các trường bình thường.

## 3.2. Nhóm B – Xả nước

Đặt nhóm xả nước dưới nhóm thông số tính toán.

Có thể sử dụng thành phần thu gọn/mở rộng:

```text
▼ THÔNG SỐ XẢ NƯỚC (KHÔNG BẮT BUỘC)

Lưu lượng xả tràn       Số giờ xả tràn
[                 ] m³/s [                 ] giờ

Lưu lượng dòng chảy tối thiểu
[                 ] m³/s
```

Yêu cầu:

- Mặc định có thể giữ trạng thái đang sử dụng của ứng dụng hiện tại.
- Nếu chọn thu gọn mặc định, phải đảm bảo người dùng vẫn nhận biết được nhóm thông số này tồn tại.
- Không tự động xóa dữ liệu khi thu gọn.
- Giữ nguyên cách xử lý trường để trống.
- Giữ nguyên công thức tính toán liên quan đến xả nước.

## 3.3. Nhóm C – Nút thao tác

Bố trí hai nút trên cùng một hàng:

```text
[ TÍNH TOÁN ]    [ XÓA DỮ LIỆU ]
```

Yêu cầu:

- Nút tính toán là nút chính, có màu cam.
- Nút xóa dữ liệu là nút phụ, màu trung tính.
- Hai nút có chiều cao đồng đều.
- Có trạng thái đang xử lý khi thực hiện tính toán.
- Ngăn việc gửi lặp nhiều lần do người dùng bấm liên tục.
- Chỉ hiển thị thông báo thành công khi phép tính thực sự hoàn tất.
- Việc xóa dữ liệu phải tuân thủ hành vi hiện tại; không tự ý xóa lịch sử tính toán.

---

# 4. THIẾT KẾ LẠI KẾT QUẢ TÍNH TOÁN

## 4.1. Cấu trúc hiển thị

Khu vực kết quả sử dụng toàn bộ chiều rộng nội dung.

```text
┌────────────────────────────────────────────────────────────────────┐
│ KẾT QUẢ TÍNH TOÁN                                                  │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│ LƯU LƯỢNG VỀ HỒ                                                    │
│                                                                    │
│ 161,76 m³/s                                                        │
│                                                                    │
├───────────────────┬────────────────────┬───────────────────────────┤
│ TỔNG LƯỢNG NƯỚC   │ MỰC NƯỚC ĐẦU      │ MỰC NƯỚC CUỐI             │
│                   │                    │                           │
│ 1.164.655,14 m³   │ 160,00 m           │ 161,00 m                  │
├───────────────────┴────────────────────┴───────────────────────────┤
│ THÔNG TIN CHI TIẾT                                                  │
│                                                                    │
│ Thời gian tính: ...                                                │
│ Giờ chạy máy H1: ...                                               │
│ Giờ chạy máy H2: ...                                               │
│ Thông số xả nước: ...                                              │
│ Thời điểm tính toán: ...                                           │
└────────────────────────────────────────────────────────────────────┘
```

Các số liệu trên chỉ minh họa cách trình bày, không phải giá trị mặc định mới.

## 4.2. Phân cấp thông tin

Ưu tiên theo thứ tự:

1. Lưu lượng về hồ.
2. Tổng lượng nước về hồ.
3. Các thông số đầu vào quan trọng.
4. Thông tin chi tiết và trạng thái tính toán.

Yêu cầu:

- Lưu lượng về hồ sử dụng cỡ chữ lớn nhất.
- Đơn vị đo hiển thị ngay cạnh giá trị.
- Không để giá trị và đơn vị xuống hai dòng nếu không cần thiết.
- Giữ độ chính xác số theo quy tắc hiện tại.
- Không tự làm tròn kết quả khác với cách tính hiện hành.
- Có thể thu gọn phần thông tin chi tiết.
- Nếu chưa có kết quả, hiển thị trạng thái hướng dẫn thay vì hiển thị số 0 giả định.

Nếu có dữ liệu thời điểm tính toán, cần hiển thị thời điểm đó. Không tự lấy thời gian hiện tại để thay thế thời điểm tính toán thực tế nếu hai thời điểm khác nhau.

---

# 5. THIẾT KẾ LẠI THẺ THỜI TIẾT

## 5.1. Thời tiết hiện tại

Giữ thẻ thời tiết ở đầu cột bên phải.

```text
┌────────────────────────────────────────────────┐
│ THỜI TIẾT A LƯỚI              [LÀM MỚI]        │
├────────────────────────────────────────────────┤
│                                                │
│ 28,2°C                        [Biểu tượng]     │
│ Mây rải rác                                    │
│                                                │
│ Độ ẩm: 62%        Gió: 2,4 m/s                 │
│                                                │
│ Nguồn: OpenWeather                             │
│ Cập nhật lúc: 11:39                            │
│ Trạng thái: Đã cập nhật                        │
└────────────────────────────────────────────────┘
```

Yêu cầu:

- Nhiệt độ là thông tin nổi bật nhất.
- Mô tả thời tiết và các chỉ số phụ có cỡ chữ nhỏ hơn.
- Nguồn dữ liệu và thời điểm cập nhật phải dễ tìm.
- Nút làm mới có trạng thái đang tải.
- Nếu API lỗi, hiển thị thông báo phù hợp và giữ lại dữ liệu cũ nếu ứng dụng hiện hỗ trợ cơ chế này.
- Phân biệt dữ liệu cũ với dữ liệu vừa được cập nhật thành công.

Không hiển thị trạng thái “Đã cập nhật” nếu yêu cầu lấy dữ liệu mới thất bại.

## 5.2. Dự báo theo ngày

Các thẻ dự báo tiếp tục nằm bên dưới radar mưa.

Trên máy tính, sử dụng hàng ngang có thể cuộn khi không đủ chiều rộng.

Mỗi thẻ gồm:

- Ngày.
- Biểu tượng thời tiết.
- Nhiệt độ thấp nhất và cao nhất.
- Lượng mưa dự báo.
- Xác suất mưa nếu có.
- Trạng thái thiếu dữ liệu nếu cần.

Không hiển thị xác suất mưa mặc định là 0% khi nguồn không cung cấp thông tin.

---

# 6. THIẾT KẾ LẠI RADAR MƯA

## 6.1. Bố cục

```text
┌────────────────────────────────────────────────┐
│ RADAR MƯA                                      │
├────────────────────────────────────────────────┤
│                                                │
│                                                │
│                  BẢN ĐỒ RADAR                  │
│                                                │
│                                                │
├────────────────────────────────────────────────┤
│ [▶/Ⅱ] ━━━━━━━━━━━━━━━●━━━━━━━━━━━━━━━━━━━━━━  │
│                                                │
│ Khung ảnh: 11:30                               │
│ Cập nhật dữ liệu: ...                          │
│                                                │
│ Chú giải cường độ mưa                           │
└────────────────────────────────────────────────┘
```

Yêu cầu:

- Bản đồ sử dụng toàn bộ chiều rộng thẻ.
- Giữ tỷ lệ bản đồ phù hợp, không kéo giãn hình ảnh.
- Các nút điều khiển đặt bên dưới bản đồ.
- Thanh thời gian đủ lớn để thao tác.
- Thời điểm của khung radar đang xem phải rõ ràng.
- Chú giải màu đặt bên dưới thanh điều khiển.
- Có trạng thái tải dữ liệu và thông báo khi không thể lấy ảnh radar.

Nếu nguồn radar hỗ trợ phát lại, giữ nguyên khả năng phát/tạm dừng và chọn khung thời gian.

Không giả lập dữ liệu radar khi nguồn cung cấp bị lỗi.

---

# 7. THIẾT KẾ LẠI KHU VỰC SO SÁNH DỰ BÁO MƯA

Đây là khu vực cần được ưu tiên cải thiện về khả năng đọc dữ liệu.

## 7.1. Cấu trúc tổng thể

```text
┌────────────────────────────────────────────────────────────────────┐
│ SO SÁNH DỰ BÁO MƯA GIỮA CÁC NGUỒN                                │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│ ⚠ CẢNH BÁO BẤT ĐỒNG DỰ BÁO                                        │
│                                                                    │
│ Ngày 28/10: ECMWF 44,5 mm; NOAA GFS 6,0 mm.                       │
│ Cần theo dõi thêm và kiểm tra các bản tin cập nhật.                │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│                    BIỂU ĐỒ SO SÁNH                                 │
│                                                                    │
│ 60 ─────────────────────────────────────────────────────────────   │
│                                                                    │
│ 40 ─────────────────────────────────────────────────────────────   │
│                                                                    │
│ 20 ─────────────────────────────────────────────────────────────   │
│                                                                    │
│  0 ─────────────────────────────────────────────────────────────   │
│       27/10       28/10       29/10       30/10       31/10         │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ KẾT LUẬN TỪNG NGÀY                                  [Thu gọn ▲]   │
│                                                                    │
│ 27/10  Các nguồn dự báo tương đối đồng thuận.                      │
│ 28/10  Bất đồng lớn giữa các mô hình dự báo.                       │
│ 29/10  Các nguồn dự báo tương đối đồng thuận.                      │
│                                                                    │
├────────────────────────────────────────────────────────────────────┤
│ BẢNG SỐ LIỆU CHI TIẾT                                [Thu gọn ▲]   │
│                                                                    │
│ Nguồn             27/10    28/10    29/10    30/10    31/10        │
│ OpenWeather        ...      ...      ...      ...      ...         │
│ NOAA GFS           ...      ...      ...      ...      ...         │
│ ECMWF              ...      ...      ...      ...      ...         │
│ DWD ICON            ...      ...      ...      ...      ...         │
│ Ước tính tổng hợp   ...      ...      ...      ...      ...         │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

Số liệu trong ví dụ chỉ nhằm minh họa cấu trúc giao diện.

## 7.2. Thẻ cảnh báo

Đưa cảnh báo bất đồng lên ngay dưới tiêu đề và trước biểu đồ.

Các mức trạng thái:

- Bình thường: các nguồn tương đối đồng thuận.
- Chú ý: xuất hiện chênh lệch đáng kể.
- Bất đồng lớn: các nguồn dự báo khác biệt rõ rệt.

Quy tắc:

- Ngưỡng phân loại phải được cấu hình hoặc xác định rõ.
- Không tự suy diễn nguy cơ lũ chỉ từ chênh lệch giữa các mô hình.
- Nội dung cảnh báo phải lấy từ dữ liệu thực tế.
- Nếu dữ liệu không đủ để đánh giá, hiển thị “Chưa đủ dữ liệu đánh giá”.
- Nếu các nguồn sử dụng khoảng thời gian dự báo khác nhau, phải chuẩn hóa trước khi so sánh hoặc ghi rõ sự khác biệt.

## 7.3. Biểu đồ

Giữ dạng biểu đồ cột nhiều nguồn nếu thư viện hiện tại hỗ trợ tốt.

Yêu cầu:

- Chiếm toàn bộ chiều rộng thẻ.
- Trục tung ghi rõ đơn vị mm/ngày hoặc đơn vị thực tế tương ứng.
- Trục hoành hiển thị ngày.
- Chú giải đặt ở vị trí không che khuất dữ liệu.
- Các cột có khoảng cách đủ để phân biệt.
- Giá trị được hiển thị trên đầu cột khi không gây chồng lấn.
- Tooltip hiển thị đầy đủ nguồn, ngày và giá trị.
- Đường ngưỡng cảnh báo chỉ xuất hiện khi đã cấu hình ngưỡng phù hợp.
- Nếu biểu đồ có quá nhiều nguồn, cho phép bật/tắt từng nguồn.

Duy trì màu sắc nhất quán giữa chú giải, biểu đồ và bảng dữ liệu.

Không thay đổi dữ liệu đầu vào hoặc cách tổng hợp chỉ để biểu đồ trông đẹp hơn.

## 7.4. Kết luận từng ngày

Mỗi ngày hiển thị một kết luận ngắn.

Ví dụ:

- Các nguồn dự báo tương đối đồng thuận.
- Có chênh lệch đáng chú ý giữa các mô hình.
- Bất đồng lớn, cần theo dõi các lần cập nhật tiếp theo.
- Chưa đủ dữ liệu để đánh giá.

Nếu có thể, cho phép bấm vào một ngày để xem các nguồn dự báo tương ứng.

Không dùng kết luận “an toàn” hoặc “không có nguy cơ lũ” chỉ vì các mô hình có giá trị dự báo thấp.

## 7.5. Bảng số liệu chi tiết

Bảng phải dễ đọc, đặc biệt khi có nhiều ngày dự báo.

Yêu cầu:

- Cột đầu tiên chứa tên nguồn.
- Các cột tiếp theo là ngày dự báo.
- Giá trị lượng mưa là thông tin chính.
- Xác suất mưa và khoảng nhiệt độ là thông tin phụ nếu có.
- Có thể cuộn ngang trên màn hình nhỏ.
- Có đường phân cách hàng nhẹ.
- Tiêu đề bảng cần dễ nhận biết.
- Có thể thu gọn bảng để giảm chiều dài trang.

Hàng chênh lệch chỉ nên hiển thị khi thuật toán tính chênh lệch đã được xác định rõ và các giá trị được so sánh trên cùng khoảng thời gian.

---

# 8. QUY CHUẨN MÀU SẮC VÀ KIỂU CHỮ

## 8.1. Bảng màu

Giữ phong cách xanh dương hiện tại.

Bảng màu đề xuất:

| Thành phần | Màu |
|---|---|
| Thanh điều hướng | `#2F527D` |
| Màu nhấn chính | `#315F91` |
| Nền tổng thể | `#F4F6F9` |
| Nền thẻ | `#FFFFFF` |
| Viền thẻ | `#D9E0E8` |
| Chữ chính | `#243247` |
| Chữ phụ | `#66758A` |
| Nút tính toán | `#F28C00` |
| Trạng thái bình thường | `#2E7D5B` |
| Trạng thái chú ý | `#E99B19` |
| Trạng thái nguy hiểm | `#C93636` |

Các màu trên là màu tham khảo. Có thể giữ lại màu hiện tại nếu dự án đã có hệ thống màu nhất quán.

## 8.2. Kiểu chữ

Ưu tiên font sans-serif dễ đọc, hỗ trợ tiếng Việt đầy đủ.

Thứ tự phân cấp:

- Tiêu đề trang: 20–24 px.
- Tiêu đề thẻ: 16–18 px.
- Nhãn trường: 13–14 px.
- Nội dung thông thường: 13–15 px.
- Giá trị kết quả chính: 38–46 px trên máy tính.
- Chú thích: 12–13 px.

Không sử dụng cỡ chữ quá nhỏ cho thông tin quan trọng.

## 8.3. Thẻ nội dung

Quy chuẩn thống nhất:

- Bo góc: 8–12 px.
- Đường viền mảnh.
- Khoảng đệm: 16–20 px.
- Khoảng cách giữa các thành phần: 8–12 px.
- Hạn chế bóng đổ mạnh.
- Không sử dụng hiệu ứng chuyển động không cần thiết.

---

# 9. THIẾT KẾ RESPONSIVE

## 9.1. Màn hình lớn, tối ưu cho ứng dụng desktop

Với màn hình rộng từ khoảng 1200 px:

- Sử dụng hai cột cho phần thông số và thời tiết.
- Kết quả tính toán và so sánh dự báo sử dụng toàn bộ chiều rộng.
- Biểu đồ có đủ không gian cho chú giải.
- Bảng dữ liệu hiển thị nhiều ngày nhất có thể.



# 10. TRẠNG THÁI DỮ LIỆU

Bổ sung cách thể hiện trạng thái dữ liệu nhất quán.

Mỗi nguồn dữ liệu nên hỗ trợ các trạng thái phù hợp:

| Trạng thái | Cách hiển thị |
|---|---|
| Đã cập nhật | Dấu hiệu bình thường, kèm thời điểm cập nhật |
| Đang tải | Hiển thị trạng thái tải |
| Cập nhật thất bại | Hiển thị thông báo lỗi |
| Dữ liệu cũ | Hiển thị thời điểm dữ liệu gần nhất |
| Thiếu dữ liệu | Hiển thị rõ trường dữ liệu chưa có |
| Không hỗ trợ | Không hiển thị trường như thể có dữ liệu |

Yêu cầu:

- Không thay dữ liệu lỗi bằng giá trị 0.
- Không coi dữ liệu thiếu là không có mưa.
- Không hiển thị thời điểm cập nhật giả định.
- Không che giấu lỗi API.
- Không để một nguồn dữ liệu lỗi làm mất toàn bộ giao diện nếu các nguồn khác vẫn hoạt động.
- Giữ lại dữ liệu cũ để tham khảo nếu ứng dụng hiện có cơ chế lưu trữ phù hợp, đồng thời ghi rõ dữ liệu đã cũ.

---

# 11. TỐI ƯU TRẢI NGHIỆM NGƯỜI DÙNG

Cần bổ sung hoặc hoàn thiện các trạng thái tương tác:

1. Hiển thị trạng thái đang tính toán.
2. Ngăn bấm nút tính liên tục khi tác vụ chưa hoàn tất.
3. Hiển thị lỗi gần trường nhập liệu.
4. Giữ nguyên giá trị đầu vào khi xảy ra lỗi không nghiêm trọng.
5. Có trạng thái tải cho dữ liệu thời tiết và radar.
6. Thông báo rõ khi không có dữ liệu dự báo.
7. Giữ trạng thái mở/thu gọn của các khu vực khi chuyển đổi nếu phù hợp.
8. Hỗ trợ thao tác bằng bàn phím.
9. Bảo đảm độ tương phản giữa chữ và nền.
10. Không sử dụng màu sắc làm phương tiện duy nhất để truyền đạt cảnh báo.

---

# 12. YÊU CẦU KỸ THUẬT KHI TRIỂN KHAI

Trước khi sửa mã nguồn, hãy thực hiện:

### Bước 1 – Khảo sát dự án

- Xác định framework và thư viện giao diện đang sử dụng.
- Xác định các trang, component và stylesheet liên quan.
- Xác định component biểu đồ.
- Xác định logic nhập liệu và hiển thị kết quả.
- Xác định nguồn dữ liệu thời tiết, radar và dự báo.
- Xác định cơ chế responsive hiện có.

### Bước 2 – Lập kế hoạch chỉnh sửa

- Liệt kê các file cần thay đổi.
- Ưu tiên chỉnh sửa component hiện có.
- Hạn chế thêm thư viện mới nếu không cần thiết.
- Không thay đổi API hoặc cấu trúc cơ sở dữ liệu chỉ vì lý do giao diện.

### Bước 3 – Triển khai

- Thực hiện từng nhóm giao diện.
- Giữ nguyên logic nghiệp vụ.
- Tái sử dụng các component dùng chung.
- Đảm bảo các trạng thái tải, lỗi và dữ liệu trống hoạt động đúng.
- Không ghi cứng dữ liệu mẫu vào giao diện thực tế.

### Bước 4 – Kiểm tra

Kiểm tra tối thiểu:

- Màn hình desktop rộng.
- Màn hình laptop.
- Trường hợp chưa có kết quả tính toán.
- Trường hợp nhập dữ liệu không hợp lệ.
- Trường hợp API thời tiết lỗi.
- Trường hợp một số nguồn dự báo thiếu dữ liệu.
- Trường hợp biểu đồ có giá trị lớn bất thường.
- Trường hợp bảng dữ liệu có nhiều ngày.

### Bước 5 – Báo cáo

Sau khi hoàn thành, báo cáo:

- Các file đã chỉnh sửa.
- Các component đã thay đổi.
- Các điểm giao diện đã cải thiện.
- Những chức năng được giữ nguyên.
- Kết quả kiểm thử.
- Các vấn đề còn tồn tại.

---

# 13. TIÊU CHÍ NGHIỆM THU

Giao diện được coi là hoàn thành khi đáp ứng các điều kiện:

- [ ] Thanh điều hướng gọn và nhất quán.
- [ ] Biểu mẫu nhập liệu được chia nhóm rõ ràng.
- [ ] Các trường nhập liệu không bị chồng lấn.
- [ ] Kết quả tính toán là thành phần nổi bật.
- [ ] Thời tiết và radar được bố trí hợp lý.
- [ ] Biểu đồ so sánh sử dụng toàn bộ chiều rộng phù hợp.
- [ ] Cảnh báo bất đồng dự báo dễ nhận biết.
- [ ] Bảng dữ liệu dễ đọc.
- [ ] Các trạng thái tải, lỗi và thiếu dữ liệu được thể hiện đúng.
- [ ] Giao diện hoạt động trên desktop và điện thoại.
- [ ] Không có lỗi JavaScript hoặc lỗi giao diện nghiêm trọng.
- [ ] Thuật toán tính toán không bị thay đổi.
- [ ] Các API và nguồn dữ liệu hiện tại vẫn hoạt động.
- [ ] Lịch sử tính toán và cài đặt vẫn sử dụng được.
- [ ] Không có dữ liệu mẫu bị hiển thị nhầm thành dữ liệu thực.

---

# 14. CHỈ DẪN CUỐI CÙNG CHO AI LẬP TRÌNH

Hãy đóng vai trò một kỹ sư frontend cấp cao có kinh nghiệm thiết kế dashboard kỹ thuật và ứng dụng giám sát vận hành.

Đọc mã nguồn hiện tại trước khi thay đổi.

Không triển khai lại toàn bộ ứng dụng từ đầu.

Không tự ý đổi công nghệ, cấu trúc dữ liệu, API hoặc thuật toán tính toán.

Ưu tiên chỉnh sửa bố cục, hệ thống component, CSS, khả năng responsive và cách trình bày dữ liệu.

Giữ lại các chức năng hiện có, đồng thời bảo đảm các cảnh báo và trạng thái dữ liệu phản ánh đúng thông tin thực tế.

Sau khi chỉnh sửa, hãy chạy các kiểm tra phù hợp với dự án và báo cáo những gì đã hoàn thành, những gì chưa hoàn thành và lý do.

**Mục tiêu cuối cùng:** tạo ra một dashboard thủy văn hiện đại, gọn gàng, dễ đọc và phù hợp với công tác theo dõi lưu lượng về hồ tại Nhà máy thủy điện Sông Bồ, trong đó tính chính xác và khả năng truy vết dữ liệu được ưu tiên hơn hiệu ứng trang trí.
