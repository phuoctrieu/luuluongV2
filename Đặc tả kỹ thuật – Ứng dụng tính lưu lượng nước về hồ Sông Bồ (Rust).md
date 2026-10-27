# Đặc tả kỹ thuật – Ứng dụng tính lưu lượng nước về hồ Sông Bồ (Rust)

Oct 27, 2026 · @Doan Phuoc Trieu

## 1. Tổng quan và mục tiêu

Phiên bản mới viết lại bằng Rust, chạy native trên macOS và Windows, tính đúng lưu lượng nước về hồ Sông Bồ và hiển thị dự báo thời tiết A Lưới trong cùng một màn hình.

**Hiện trạng (bản Python/Tkinter, `giaodien.py`):** nhập 5 số, tra bảng `tinhtoan.xlsx`, tính Q = (ΔV hồ + V tuabin) / t. File `.exe` nặng 16 MB, chỉ chạy Windows, giao diện cố định 500×450.

**Vấn đề cần giải quyết:**

- Tra bảng bằng so khớp ±0,01 m, không nội suy; bỏ sót dòng 1 (165 m); lỗi khi mực nước đầu bằng mực nước cuối.
- Công thức bỏ qua xả tràn, xả dòng chảy tối thiểu; lưu lượng 17,3 m³/s mỗi tổ máy gán cứng trong code.
- Không kiểm tra đầu vào, không nhận dấu phẩy thập phân, không lưu lịch sử.
- Đường dẫn file tương đối nên dễ lỗi khi mở từ shortcut.
- Chưa có thông tin thời tiết để vận hành chủ động.

**Mục tiêu:**

| Mục tiêu | Chỉ tiêu đo được |
| --- | --- |
| Đúng số liệu | Kết quả khớp bảng tính tay sai khác ≤ 0,01 m³/s trên bộ ca thử |
| Gọn nhẹ | File chạy ≤ 15 MB, khởi động ≤ 1 giây, RAM ≤ 100 MB |
| Đa nền tảng | Một mã nguồn, build macOS (Apple Silicon + Intel) và Windows 10/11 |
| Thời tiết | Dự báo 5 ngày cho A Lưới, vẫn tính lưu lượng được khi mất mạng |
| Dễ dùng | Người vận hành nhập xong 5 ô và có kết quả trong dưới 30 giây |

**Người dùng:** nhân viên vận hành ca và lãnh đạo nhà máy; dùng máy để bàn Windows tại phòng điều khiển và máy Mac của giám đốc.

**Ngoài phạm vi:** kết nối SCADA, ghi dữ liệu lên server, ứng dụng di động.

## 2. Yêu cầu chức năng

App có 3 màn hình: **Tính toán** (mặc định, kèm thời tiết), **Lịch sử** và **Cài đặt**. Ưu tiên: P0 bắt buộc cho bản đầu, P1 làm ngay sau, P2 để sau.

| Mã | Chức năng | Mô tả | Ưu tiên |
| --- | --- | --- | --- |
| F1 | Tính lưu lượng về hồ | Nhập mực nước đầu, mực nước cuối, thời gian tính, giờ chạy H1/H2. Trả về Q (m³/s) và tổng lượng nước về hồ (m³) | P0 |
| F2 | Thành phần xả | Ô tùy chọn: xả tràn (lưu lượng, số giờ) và xả dòng chảy tối thiểu (m³/s). Mặc định 0 | P0 |
| F3 | Tra đường đặc tính hồ | Nội suy tuyến tính theo bảng cao trình – dung tích 149–165 m, hiển thị dung tích tương ứng mỗi mực nước | P0 |
| F4 | Kiểm tra đầu vào | Báo lỗi ngay tại ô nhập bằng tiếng Việt dễ hiểu; nhận cả dấu phẩy và dấu chấm thập phân | P0 |
| F5 | Dự báo thời tiết | Thời tiết hiện tại + dự báo 5 ngày tại A Lưới qua OpenWeather (mục 5) | P0 |
| F6 | Lịch sử tính toán | Mỗi lần tính tự lưu: thời điểm, đầu vào, kết quả, người nhập (tùy chọn). Xem, lọc theo ngày, xóa | P1 |
| F7 | Xuất báo cáo | Xuất lịch sử ra CSV (mở được bằng Excel, mã hóa UTF-8 có BOM) | P1 |
| F8 | Cài đặt | Sửa lưu lượng mỗi tổ máy, API key, đơn vị, giao diện sáng/tối, đường dẫn bảng dung tích | P1 |
| F9 | Nạp bảng dung tích | Đọc CSV/XLSX do người dùng thay, kiểm tra độ hợp lệ (cao trình tăng dần, dung tích tăng dần) | P1 |
| F10 | Sao chép kết quả | Nút sao chép kết quả dạng văn bản để dán vào Zalo/báo cáo ca | P2 |

### Luồng sử dụng chính

1. Mở app: hiện màn hình Tính toán và dự báo thời tiết (lấy từ cache nếu mất mạng).
2. Nhập 5 ô bắt buộc; ô xả để trống nếu không xả.
3. Nhấn Enter hoặc nút TÍNH: kết quả hiện ngay, đồng thời tự lưu vào Lịch sử.
4. Xem lại hoặc xuất CSV khi cần báo cáo.

## 3. Mô hình tính toán

Lưu lượng nước về hồ được tính từ cân bằng nước: lượng thay đổi trong hồ cộng mọi lượng nước đã ra khỏi hồ, chia cho thời gian.

### 3.1. Công thức

```latex
V(Z) = V_i + \frac{(Z - Z_i)\,(V_{i+1} - V_i)}{Z_{i+1} - Z_i} \qquad Z_i \le Z \le Z_{i+1}
\Delta V = \left[ V(Z_{cuoi}) - V(Z_{dau}) \right] \times 10^{6}
V_{tb} = (t_{H1} \, q_{H1} + t_{H2} \, q_{H2}) \times 3600
V_{xa} = Q_{tran}\, t_{tran}\, 3600 + Q_{mt}\, T\, 3600
Q_{ve} = \frac{\Delta V + V_{tb} + V_{xa}}{T \times 3600} \qquad W_{ve} = \Delta V + V_{tb} + V_{xa}
```

Ký hiệu: Z là mực nước hồ (m); V là dung tích (triệu m³ trong bảng, đổi ra m³ khi tính); T là thời gian tính (giờ); t là giờ chạy máy; q là lưu lượng mỗi tổ máy (mặc định 17,3 m³/s như bản cũ); Q\_tran, t\_tran là lưu lượng và số giờ xả tràn; Q\_mt là lưu lượng xả dòng chảy tối thiểu.

Khi không nhập xả tràn và xả dòng chảy tối thiểu, kết quả trùng với công thức của bản Python.

### 3.2. Quy tắc tra bảng

- Bảng được sắp xếp tăng dần theo cao trình khi nạp (file gốc đang sắp xếp giảm dần).
- Đọc **từ dòng 1**, không bỏ dòng nào (bản cũ bỏ dòng 165 m).
- Giá trị cao trình làm tròn để loại sai số số thực (ví dụ 160,1000000000001 đọc thành 160,1).
- Tìm khoảng \[Z\_i, Z\_i+1\] bằng tìm kiếm nhị phân; nội suy tuyến tính. Cao trình trùng điểm bảng lấy nguyên giá trị.
- Ngoài khoảng 149–165 m: **từ chối tính** và báo rõ khoảng hợp lệ, không ngoại suy.
- Mực nước đầu bằng mực nước cuối là hợp lệ: ΔV = 0.

### 3.3. Hạn chế của mô hình (hiển thị trên màn hình kết quả)

- Chưa tính bốc hơi và thấm, nên kết quả là lưu lượng *đến hồ ước tính*, không phải số đo trực tiếp.
- Lưu lượng tổ máy là hằng số cài đặt, thực tế phụ thuộc cột nước và công suất phát. Đề xuất cho phiên bản sau: bảng q theo công suất.
- Độ chính xác phụ thuộc độ phân giải mực nước đo và bảng dung tích (bước 0,1 m).

### 3.4. Bộ ca thử bắt buộc

| Ca | Đầu vào | Kết quả mong đợi |
| --- | --- | --- |
| T1 | Z đầu 160,0; Z cuối 160,1; T = 1 giờ; H1 = 1 giờ; H2 = 0 | ΔV ≈ 89 163 m³; Q ≈ 42,07 m³/s |
| T2 | Z đầu = Z cuối = 155,0; T = 2; H1 = 2; H2 = 2 | ΔV = 0; Q = 17,3 × 4 / 2 = 34,6 m³/s |
| T3 | Z cuối = 160,05 (giữa hai điểm bảng) | V là trung bình của V(160,0) và V(160,1) |
| T4 | Z đầu = 165,0 | Tra được V = 14,1506 triệu m³, không báo lỗi |
| T5 | Z = 148,9 hoặc 165,1 | Từ chối, báo khoảng hợp lệ 149–165 m |
| T6 | T = 0 | Từ chối, báo thời gian phải lớn hơn 0 |

## 4. Dữ liệu và cấu hình

Tất cả dữ liệu nằm trong một thư mục riêng của app, không cần server. Bảng dung tích mặc định được nhúng sẵn trong file chạy, người dùng có thể thay bằng file khác.

| Dữ liệu | Định dạng | Vị trí | Ghi chú |
| --- | --- | --- | --- |
| Bảng cao trình – dung tích | CSV 2 cột: `cao_trinh_m`, `dung_tich_trieu_m3` | Nhúng sẵn; bản thay thế trong thư mục dữ liệu | Chuyển từ `tinhtoan.xlsx` (125 dòng), sắp tăng dần, bỏ hai ô ghi chú của C2 và C11 |
| Cấu hình | TOML `config.toml` | Thư mục cấu hình của hệ điều hành | Xem mẫu bên dưới |
| Lịch sử tính toán | SQLite `history.db` | Thư mục dữ liệu của hệ điều hành | Một bảng `calc`, thêm khóa theo thời điểm |
| Cache thời tiết | JSON `weather_cache.json` | Thư mục cache | Kèm thời điểm lấy để biết dữ liệu cũ hay mới |
| API key | Kho khóa của hệ điều hành (Keychain / Credential Manager) | Hệ điều hành | Không lưu dạng chữ thường trong `config.toml` |

### Mẫu `config.toml`

```toml
[reservoir]
z_min = 149.0
z_max = 165.0
volume_table = "builtin"        # hoặc đường dẫn tới file CSV

[turbine]
q_h1 = 17.3                     # m3/s
q_h2 = 17.3                     # m3/s

[weather]
lat = 16.22                     # xấp xỉ A Lưới, cần chỉnh theo vị trí nhà máy
lon = 107.25
units = "metric"
lang = "vi"
refresh_minutes = 60

[ui]
theme = "system"                # system | light | dark
```

### Bảng `calc` trong SQLite

| Cột | Kiểu | Ý nghĩa |
| --- | --- | --- |
| `id` | INTEGER, khóa chính | Số thứ tự tự tăng |
| `created_at` | TEXT (ISO 8601, giờ Việt Nam) | Thời điểm bấm TÍNH |
| `z_start`, `z_end` | REAL | Mực nước đầu, cuối (m) |
| `hours`, `h1`, `h2` | REAL | Thời gian tính, giờ chạy H1, H2 |
| `spill_q`, `spill_h`, `eco_q` | REAL | Xả tràn, xả dòng chảy tối thiểu |
| `q_in`, `w_in` | REAL | Kết quả: m³/s và m³ |
| `note` | TEXT | Ghi chú tùy chọn |

## 5. Tích hợp OpenWeather

Mặc định dùng gói **miễn phí** (không cần thẻ): thời tiết hiện tại và dự báo 5 ngày mỗi 3 giờ, gộp lại thành 5 dòng theo ngày. Gói miễn phí có giới hạn 60 lần gọi mỗi phút và 1.000.000 lần mỗi tháng, dư rất nhiều cho app này ([bảng giá OpenWeather](https://openweathermap.org/full-price)). Dự báo theo ngày 8 ngày của One Call 3.0 là tùy chọn, cần đăng ký riêng bằng thẻ thanh toán (1.000 lần gọi đầu mỗi ngày miễn phí, vượt thì tính phí) ([giới thiệu One Call 3.0](https://openweather.co.uk/blog/post//upgrade-your-weather-data-experience-one-call-api-30)).

### 5.1. Nhà cung cấp và endpoint

| Nhà cung cấp | Endpoint | Dùng cho | Chi phí |
| --- | --- | --- | --- |
| `Free` (mặc định) | `GET https://api.openweathermap.org/data/2.5/weather` | Thời tiết hiện tại | Miễn phí |
| `Free` (mặc định) | `GET https://api.openweathermap.org/data/2.5/forecast` | Dự báo 5 ngày, bước 3 giờ (40 mốc) | Miễn phí |
| `OneCall` (tùy chọn) | `GET https://api.openweathermap.org/data/3.0/onecall` | Hiện tại, 48 giờ, 8 ngày | 1.000 lần/ngày miễn phí, sau đó trả phí |

Tham số chung: `lat`, `lon`, `appid` (API key), `units=metric`, `lang=vi`. Kiến trúc dùng một trait `WeatherProvider` để đổi nhà cung cấp trong Cài đặt mà không sửa giao diện.

### 5.2. Dữ liệu hiển thị

- **Hiện tại:** nhiệt độ, độ ẩm, gió, mô tả và biểu tượng thời tiết, giờ cập nhật.
- **Mỗi ngày trong 5 ngày tới:** nhiệt độ thấp nhất/cao nhất, **tổng lượng mưa dự kiến (mm)**, **xác suất mưa cao nhất (%)**, mô tả phổ biến nhất.
- Lượng mưa là thông tin quan trọng nhất cho vận hành hồ, nên được làm nổi bật: thẻ ngày có mưa ≥ 50 mm đổi màu cảnh báo (ngưỡng chỉnh được trong Cài đặt).

### 5.3. Quy tắc gộp theo ngày

1. Dùng `city.timezone` trong phản hồi (độ lệch giây so với UTC) để đổi `dt` sang giờ địa phương, nhóm theo ngày lịch.
2. Nhiệt độ: lấy min/max của `main.temp` trong ngày. Lượng mưa: cộng `rain.3h` (thiếu trường này = 0). Xác suất mưa: lấy `pop` lớn nhất.
3. Ngày đầu và ngày cuối có thể thiếu mốc; hiển thị ghi chú "dữ liệu một phần" thay vì tự đoán.

### 5.4. Cache, làm mới và mất mạng

- Làm mới tự động mỗi 60 phút và khi mở app; có nút làm mới thủ công (tối đa 1 lần/30 giây).
- Chạy nền bằng tác vụ bất đồng bộ, không đơ giao diện; timeout 8 giây.
- Mất mạng hoặc lỗi API: hiển thị dữ liệu cache kèm nhãn "Cập nhật lúc HH:MM – dữ liệu cũ". Phần tính lưu lượng không phụ thuộc mạng.

### 5.5. Xử lý lỗi API

| Mã | Nguyên nhân | Thông báo cho người dùng |
| --- | --- | --- |
| 401 | API key sai hoặc chưa kích hoạt | "API key không hợp lệ. Key mới tạo có thể cần đợi vài giờ để kích hoạt." |
| 429 | Vượt giới hạn gọi | "Quá nhiều yêu cầu, thử lại sau." Giãn cách làm mới theo cấp số nhân |
| 5xx, timeout | Phía OpenWeather hoặc mạng | Dùng cache, thử lại sau 5 phút |
| Phản hồi sai cấu trúc | API đổi định dạng | Ghi log, dùng cache, không làm app dừng |

### 5.6. Bảo mật

- API key lưu trong kho khóa của hệ điều hành (crate `keyring`), không đưa vào mã nguồn hay file cấu hình.
- Không ghi API key vào log; che key trong URL khi in lỗi.
- Chỉ gọi HTTPS.

**Nguồn:** [Detailed pricing – OpenWeather](https://openweathermap.org/full-price), [One Call API 3.0 – OpenWeather](https://openweather.co.uk/blog/post//upgrade-your-weather-data-experience-one-call-api-30). Tên trường JSON của phản hồi (`main.temp`, `pop`, `rain.3h`, `city.timezone`) cần đối chiếu lại với tài liệu API khi lập trình, vì trang tài liệu chi tiết chưa mở được trong lần soạn này.

## 6. Thiết kế giao diện

Giao diện một cửa sổ co giãn được, chứa ba vùng: thanh điều hướng trên cùng, khu nhập liệu và kết quả ở giữa, dải dự báo thời tiết 5 ngày ở dưới. Kết quả là thứ lớn nhất trên màn hình.

&#91;embedded content: màn hình Tính toán · bố cục phác thảo\]

Bố cục phác thảo: số liệu 222,17 và 799.802,19 lấy từ ví dụ ghi trong `tinhtoan.xlsx`; màu và khoảng cách sẽ chốt khi thiết kế chi tiết.

### 6.1. Khung cửa sổ

| Thuộc tính | Giá trị |
| --- | --- |
| Kích thước mặc định / tối thiểu | 1100×720 / 900×600, co giãn tự do (bản cũ cố định 500×450) |
| Điều hướng | 3 thẻ trên thanh đầu: Tính toán, Lịch sử, Cài đặt |
| Thanh trạng thái | Trạng thái mạng, giờ cập nhật thời tiết, phiên bản app |
| Ngôn ngữ | Tiếng Việt, định dạng số theo kiểu Việt (dấu phẩy thập phân, dấu chấm ngăn nhóm) |
| Màn hình độ phân giải cao | Tự co theo tỷ lệ hiển thị (Retina, 4K) |

### 6.2. Màn hình Tính toán

- **Thẻ "Thông số tính toán" (bên trái):** 5 ô bắt buộc (mực nước đầu, mực nước cuối, thời gian tính, giờ chạy H1, giờ chạy H2). Mỗi ô có nhãn, đơn vị cố định bên phải, dòng gợi ý khoảng hợp lệ ("149,0 – 165,0 m") và dòng lỗi màu đỏ ngay dưới ô. Phần "Xả nước" thu gọn mặc định, mở ra để nhập xả tràn và xả dòng chảy tối thiểu.
- **Thẻ "Kết quả" (bên phải):** số Q cỡ chữ lớn kèm đơn vị m³/s; dưới là tổng lượng nước về hồ (m³) và bảng diễn giải: V đầu, V cuối, ΔV, V qua tuabin, V xả. Có nút Sao chép kết quả.
- **Đồ thị đường đặc tính hồ (P1):** đường cao trình – dung tích, đánh dấu hai điểm Z đầu và Z cuối để người dùng kiểm tra bằng mắt số nhập có hợp lý không.
- **Dải thời tiết:** thẻ "Hiện tại" bên trái, 5 thẻ ngày bên phải (thứ, ngày, biểu tượng, nhiệt độ thấp/cao, lượng mưa mm, xác suất mưa %). Ngày mưa lớn đổi màu cảnh báo.
- Trạng thái rỗng, đang tải, lỗi, ngoại tuyến đều có thiết kế riêng, không để ô trống.

### 6.3. Ngôn ngữ thiết kế

| Thành phần | Sáng | Tối | Dùng cho |
| --- | --- | --- | --- |
| Nền | `#F5F7FA` | `#0F1626` | Nền cửa sổ |
| Thẻ | `#FFFFFF` | `#18233A` | Nền các thẻ |
| Chính | `#324E7B` | `#86A6DF` | Thanh đầu, tiêu đề, điểm nhấn |
| Nhấn | `#FF8C32` | `#FF9F52` | Nút TÍNH, số kết quả |
| Chữ | `#06113C` | `#E8EDF7` | Nội dung |
| Lỗi | `#C62828` | `#FF6B6B` | Thông báo lỗi, cảnh báo mưa lớn |

Bảng màu giữ lại bộ màu xanh navy và cam của bản cũ, nhưng sửa lỗi chữ trắng trên nền xanh nhạt khó đọc. Mọi cặp chữ/nền phải đạt tỉ lệ tương phản từ 4,5:1 trở lên (kiểm tra bằng công cụ trước khi chốt). Màu sắc chỉ là giá trị đề xuất, chốt khi làm bản thiết kế.

- **Font:** nhúng sẵn một font hỗ trợ đầy đủ dấu tiếng Việt (ví dụ Be Vietnam Pro hoặc Inter, cả hai theo giấy phép mở SIL OFL), không phụ thuộc font hệ thống.
- **Cỡ chữ:** số kết quả 44 pt; tiêu đề thẻ 16 pt đậm; nhãn ô nhập 13 pt; ghi chú 11 pt. Số dùng kiểu chữ có bề rộng đều để các chữ số thẳng hàng.
- **Khoảng cách:** bước 8 px; góc bo 8 px; thẻ có viền mỏng, không đổ bóng nặng.
- **Giao diện sáng/tối:** theo hệ điều hành, đổi được trong Cài đặt.

### 6.4. Phím tắt và thao tác

| Phím | Hành động |
| --- | --- |
| Enter (trong ô nhập) | Tính ngay nếu đủ dữ liệu; nếu thiếu, nhảy tới ô đầu tiên bị lỗi |
| Tab / Shift+Tab | Chuyển ô theo thứ tự trên xuống |
| Cmd/Ctrl + Enter | Tính |
| Cmd/Ctrl + Backspace | Xóa các ô nhập |
| Cmd/Ctrl + 1, 2, 3 | Chuyển sang Tính toán, Lịch sử, Cài đặt |
| Cmd/Ctrl + R | Làm mới thời tiết |

Nút Thoát hiện hộp xác nhận chung ("Bạn có muốn thoát không?"), không dùng tên riêng. Nếu đã có dữ liệu chưa tính thì hỏi, còn không thì đóng luôn.

## 7. Kiến trúc và công nghệ Rust

Rust cho ra một file chạy duy nhất, không cần cài Python hay thư viện đi kèm. Logic tính toán tách thành thư viện thuần (không phụ thuộc giao diện) để kiểm thử được bằng `cargo test`.

&#91;embedded content: kiến trúc · 4 thư viện, 3 nguồn ngoài\]

Lõi tính toán đứng riêng; giao diện gọi trực tiếp, riêng mạng đi qua luồng phụ.

### 7.1. Lựa chọn công nghệ

| Hạng mục | Đề xuất | Lý do | Phương án thay thế |
| --- | --- | --- | --- |
| Ngôn ngữ | Rust (bản ổn định mới nhất, edition 2021 trở lên) | File đơn, nhẹ, an toàn bộ nhớ | — |
| GUI | `eframe` / `egui` | Giấy phép MIT/Apache, một mã cho cả macOS và Windows, rất nhẹ, dễ vẽ thẻ và đồ thị | `Slint` (giao diện khai báo, đẹp hơn nhưng cần xem lại điều khoản giấy phép); `Tauri` (giao diện web, phải viết thêm JS) |
| Đồ thị | `egui_plot` | Cùng hệ với GUI | — |
| HTTP | `ureq` (chạy trong luồng phụ) | Nhẹ hơn `reqwest` + `tokio`, đủ cho 2 request mỗi giờ | `reqwest` |
| JSON, TOML, CSV | `serde`, `serde_json`, `toml`, `csv` | Chuẩn hàng đầu | — |
| Đọc Excel | `calamine` | Chỉ dùng khi nạp bảng dung tích từ `.xlsx` | Bỏ, chỉ nhận CSV |
| Lịch sử | `rusqlite` (bật `bundled`) | Một file, không cần cài SQLite | File CSV nối đuôi |
| Giờ, thư mục | `chrono`, `directories` | Giờ Việt Nam, đường dẫn dữ liệu đúng chuẩn từng hệ điều hành | — |
| API key | `keyring` | Keychain trên macOS, Credential Manager trên Windows | File mã hóa |
| Lỗi, log | `thiserror`, `tracing` | Lỗi có kiểu, log ra file xoay vòng | — |

Các con số kích thước và tốc độ trong mục 1 là mục tiêu, cần đo thực tế sau bản build đầu tiên (xem mục 10).

### 7.2. Cấu trúc dự án

```text
luuluong/
  Cargo.toml                 # workspace
  crates/
    core/                    # thư viện thuần, không GUI
      src/reservoir.rs       # bảng + nội suy
      src/balance.rs         # công thức cân bằng nước
      src/validate.rs        # kiểm tra đầu vào, phân tích số kiểu Việt
      src/error.rs
    weather/                 # nhà cung cấp thời tiết
      src/provider.rs        # trait WeatherProvider
      src/free.rs            # /weather + /forecast
      src/onecall.rs         # tùy chọn
      src/aggregate.rs       # gộp 3 giờ thành ngày
      src/cache.rs
    storage/                 # cấu hình, SQLite, xuất CSV
    app/                     # giao diện egui, main.rs
      assets/                # font, icon, bảng dung tích mặc định
```

### 7.3. Mô hình luồng

- Luồng giao diện chỉ vẽ và nhận sự kiện. Phép tính rất nhanh nên chạy ngay trên luồng này.
- Việc mạng chạy ở luồng phụ, gửi kết quả về qua kênh `mpsc`; giao diện được đánh thức lại khi có dữ liệu mới.
- Ghi SQLite làm ngay sau mỗi lần tính; lỗi ghi không chặn kết quả, chỉ hiện cảnh báo nhỏ.

### 7.4. Cấu hình biên dịch (giảm kích thước)

```toml
[profile.release]
opt-level = "s"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = true
```

### 7.5. Giao diện của hàm tính (thư viện `core`)

```rust
pub struct Inputs {
    pub z_start: f64, pub z_end: f64,   // m
    pub hours: f64,                     // giờ tính toán
    pub h1: f64, pub h2: f64,           // giờ chạy máy
    pub spill_q: f64, pub spill_h: f64, // xả tràn: m3/s, giờ
    pub eco_q: f64,                     // xả dòng chảy tối thiểu: m3/s
}

pub struct Outputs {
    pub v_start: f64, pub v_end: f64,   // m3
    pub delta_v: f64, pub v_turbine: f64, pub v_release: f64,
    pub q_in: f64,                      // m3/s
    pub w_in: f64,                      // m3
}

pub fn compute(t: &VolumeTable, cfg: &Turbine, i: &Inputs)
    -> Result<Outputs, CalcError>;
```

## 8. Kiểm tra đầu vào và xử lý lỗi

Nguyên tắc: lỗi hiện ngay tại ô nhập bằng tiếng Việt dễ hiểu, nút TÍNH chỉ bị khóa khi còn lỗi; app không được dừng đột ngột vì bất kỳ lỗi nào.

### 8.1. Quy tắc từng ô

| Ô | Quy tắc | Thông báo |
| --- | --- | --- |
| Mực nước đầu, cuối | Số thực, 149,0 ≤ Z ≤ 165,0 | "Mực nước phải trong khoảng 149,0 – 165,0 m" |
| Thời gian tính | Số thực > 0 | "Thời gian phải lớn hơn 0" |
| Giờ chạy H1, H2 | 0 ≤ giờ ≤ thời gian tính; ô trống coi là 0 | "Giờ chạy máy không được lớn hơn thời gian tính" |
| Xả tràn: lưu lượng, số giờ | ≥ 0; số giờ ≤ thời gian tính | "Số giờ xả tràn không được lớn hơn thời gian tính" |
| Xả dòng chảy tối thiểu | ≥ 0 | "Giá trị không được âm" |
| Mọi ô số | Nhận dấu phẩy và dấu chấm thập phân; loại NaN, vô cực, chữ | "Vui lòng nhập số, ví dụ 160,5" |

### 8.2. Phân tích số kiểu Việt

- Chỉ có một dấu (`,` hoặc `.`) là dấu thập phân; cả hai dấu cùng xuất hiện thì báo lỗi mơ hồ, không đoán.
- Bỏ khoảng trắng hai đầu. Không chấp nhận ký hiệu khoa học (1e3) để tránh nhập nhầm.
- Hiển thị kết quả theo kiểu Việt: 799.802,19 m³ và 222,17 m³/s.

### 8.3. Phân loại lỗi hệ thống

| Loại | Ví dụ | Cách xử lý |
| --- | --- | --- |
| Lỗi nhập liệu | Chữ trong ô số, ngoài khoảng | Báo ngay tại ô, không tính |
| Lỗi bảng dung tích | File thiếu cột, cao trình không tăng dần, trùng cao trình | Hiện hộp thoại nêu dòng lỗi, quay về bảng nhúng sẵn |
| Lỗi mạng/API | Xem mục 5.5 | Giữ dữ liệu cache, báo bằng nhãn trạng thái |
| Lỗi lưu trữ | Không ghi được lịch sử, hết đĩa | Cảnh báo nhỏ, vẫn hiển thị kết quả |
| Lỗi nội bộ (panic) | Lỗi lập trình | Ghi log kèm stack, hiện hộp thoại "Có lỗi, đã lưu báo cáo" rồi mở lại app |

Mọi thông báo bằng tiếng Việt, nêu việc cần làm tiếp theo; không hiện chuỗi lỗi kỹ thuật cho người vận hành. Chi tiết kỹ thuật ghi vào file log trong thư mục dữ liệu.

## 9. Build và phân phối trên macOS và Windows

Một mã nguồn, hai gói cài đặt. Máy Mac của anh build được bản macOS trực tiếp; bản Windows build trên máy Windows hoặc bằng CI để khỏi phải cài thêm công cụ chéo.

| Hạng mục | macOS | Windows |
| --- | --- | --- |
| Target | `aarch64-apple-darwin` + `x86_64-apple-darwin`, gộp thành universal bằng `lipo` | `x86_64-pc-windows-msvc` |
| Hệ điều hành tối thiểu | Đặt khi build, đề xuất macOS 11 trở lên | Windows 10 trở lên |
| Đóng gói | `.app` trong `.dmg` (dùng `cargo-packager` hoặc `cargo-bundle`) | `.exe` chạy trực tiếp trong `.zip`, tùy chọn trình cài `.msi` |
| Biểu tượng | `.icns` từ `icon.ico` hiện có | `.ico` gắn vào file `.exe` bằng crate `winresource` |
| Ẩn cửa sổ console | Không cần | `#![windows_subsystem = "windows"]` |
| Thư viện đi kèm | Không | Liên kết tĩnh MSVC runtime (`+crt-static`) để không phải cài Visual C++ Redistributable |
| Ký số | Developer ID + công chứng (`notarytool`), cần tài khoản Apple Developer trả phí | Chứng chỉ ký mã (tùy chọn, có phí) |
| Nếu chưa ký | Người dùng mở bằng chuột phải > Open lần đầu | Windows SmartScreen cảnh báo, chọn "More info" > "Run anyway" |

Cách chạy phần mềm đã ký hay chưa là quyết định của anh: với dùng nội bộ trong nhà máy, có thể bỏ qua ký số và hướng dẫn mở lần đầu bằng một trang A4.

### 9.1. Lệnh build thủ công

```bash
# macOS (trên máy Mac)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release -p app --target aarch64-apple-darwin
cargo build --release -p app --target x86_64-apple-darwin
lipo -create -output luuluong \
  target/aarch64-apple-darwin/release/app \
  target/x86_64-apple-darwin/release/app

# Windows (PowerShell, trên máy Windows)
rustup target add x86_64-pc-windows-msvc
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release -p app --target x86_64-pc-windows-msvc
```

### 9.2. Tự động hóa bằng CI

1. Đẩy mã lên kho Git và gắn thẻ phiên bản dạng `v1.0.0`.
2. GitHub Actions chạy hai job song song trên `macos-latest` và `windows-latest`: `cargo test`, `cargo clippy`, `cargo build --release`.
3. Mỗi job đóng gói và đính kèm file vào bản phát hành: `luuluong-1.0.0-macos-universal.dmg`, `luuluong-1.0.0-windows-x64.zip`.
4. Khóa ký số (nếu có) để trong secrets của CI, không đưa vào kho mã.

### 9.3. Cài đặt và dữ liệu người dùng

- App không cần quyền quản trị, không ghi vào thư mục cài đặt.
- Dữ liệu nằm trong thư mục chuẩn của hệ điều hành (`~/Library/Application Support/luuluong` trên macOS, `%APPDATA%\luuluong` trên Windows), nên gỡ và cài lại không mất lịch sử.
- Bản cài thêm nút "Mở thư mục dữ liệu" trong Cài đặt để sao lưu dễ dàng.
- Cập nhật tự động để giai đoạn sau (P2); trước mắt thay bằng kiểm tra phiên bản mới thủ công.

## 10. Kiểm thử, nghiệm thu và lộ trình

App được nghiệm thu khi kết quả khớp bản Python trên các ca hợp lệ và đúng bảng tính tay trên các ca mới, đồng thời đạt các chỉ tiêu nhẹ ở mục 1.

### 10.1. Kế hoạch kiểm thử

| Lớp | Nội dung | Công cụ |
| --- | --- | --- |
| Đơn vị | Nội suy, công thức cân bằng, phân tích số kiểu Việt, gộp dữ liệu thời tiết theo ngày | `cargo test`, 6 ca ở mục 3.4 là bắt buộc |
| Đối chiếu | Chạy 30–50 ca thật do người vận hành cung cấp qua cả bản Python (ca hợp lệ chỉ tra trùng điểm bảng) và bản Rust, so sánh | Kịch bản so sánh, sai khác ≤ 0,01 m³/s |
| Tích hợp | Thời tiết với phản hồi giả lập: đúng, 401, 429, timeout, JSON sai cấu trúc, mất mạng | Server giả trong test (`mockito`) |
| Giao diện | Co giãn 900×600 đến 4K, giao diện sáng/tối, phím tắt, hiển thị dấu tiếng Việt | Kiểm tra tay trên 2 máy thật |
| Hiệu năng | Đo thời gian khởi động, RAM, kích thước file | Đo trên máy cấu hình thấp nhất của nhà máy |
| Cài đặt | Cài sạch, gỡ, cài đè; mở lần đầu khi chưa ký số | Kiểm tra tay trên macOS và Windows 10/11 |

### 10.2. Tiêu chí nghiệm thu

- 100% ca kiểm thử đơn vị đạt; không có cảnh báo `clippy`.
- Ca đối chiếu: sai khác so với bản Python ≤ 0,01 m³/s ở mọi ca hợp lệ của bản cũ; các ca nội suy so với tính tay.
- Khởi động ≤ 1 giây, RAM ≤ 100 MB, file chạy ≤ 15 MB (đo thực tế, điều chỉnh nếu lệch).
- Mất mạng: app vẫn tính đúng, thời tiết hiện dữ liệu cache kèm nhãn cũ.
- Người vận hành mới tự tính được một ca trong dưới 30 giây mà không cần hướng dẫn.
- Chạy được trên macOS (Apple Silicon và Intel) và Windows 10/11.

### 10.3. Lộ trình đề xuất (thời gian dự kiến, chưa cam kết)

1. **Giai đoạn 1 – Lõi tính toán (khoảng 1 tuần):** thư viện `core`, chuyển bảng dung tích, toàn bộ ca thử đơn vị và ca đối chiếu. Cổng: khớp bản Python.
2. **Giai đoạn 2 – Giao diện tính toán (khoảng 1–2 tuần):** màn hình Tính toán, kiểm tra đầu vào, giao diện sáng/tối, phím tắt. Cổng: người vận hành dùng thử và đồng ý giao diện.
3. **Giai đoạn 3 – Thời tiết (khoảng 1 tuần):** `weather`, cache, xử lý lỗi API, dải dự báo. Cổng: chạy ổn định 3 ngày liên tục, kể cả ngắt mạng.
4. **Giai đoạn 4 – Lịch sử, xuất CSV, Cài đặt (khoảng 1 tuần):** SQLite, nạp bảng dung tích ngoài, API key trong kho khóa.
5. **Giai đoạn 5 – Đóng gói và triển khai (khoảng 1 tuần):** CI cho hai nền tảng, biểu tượng, trang hướng dẫn, chạy song song với bản Python ít nhất một tuần rồi mới thay hẳn.

### 10.4. Các câu hỏi cần anh xác nhận

- Toạ độ chính xác của nhà máy để đặt mặc định cho thời tiết (hiện đang đặt xấp xỉ A Lưới).
- Mực nước chết và mực nước dâng bình thường có nằm trong khoảng 149–165 m của bảng không, để chặn nhập sai chuẩn hơn.
- Lưu lượng 17,3 m³/s của H1 và H2 là số chủ định hay thay đổi theo công suất.
- Có ghi nhận xả tràn và xả dòng chảy tối thiểu ở đâu để đưa vào các ô xả không.
- Có cần ký số ứng dụng hay chỉ dùng nội bộ.
- Chấp nhận dùng gói OpenWeather miễn phí (dự báo 5 ngày) hay muốn One Call 3.0 (8 ngày, cần thẻ thanh toán).
