# Hướng dẫn lấy dữ liệu "MNTL hien tai" từ data logger ECAPRO

Mục tiêu: ứng dụng (Rust) đọc mực nước thời gian thực **MNTL hien tai** (kênh 16) từ thiết bị ECAPRO GPIs6.6DA.

## 1. Thông tin thiết bị

| Mục | Giá trị |
|---|---|
| Thiết bị | ECAPRO GPIs6.6DA (V0821) |
| Địa chỉ | `192.168.200.211` |
| Cổng web | `8880` |
| Giao thức | WebIOPi, HTTP POST |
| Đường dẫn dữ liệu | `/macros/UpdateMonitor` |

> Lưu ý: tab Modbus TCP của thiết bị là **master (client)**, không phải server, nên không đọc Modbus TCP trực tiếp từ thiết bị được.

## 2. Kiểm tra nhanh bằng terminal

```bash
curl -s -X POST http://192.168.200.211:8880/macros/UpdateMonitor | grep ",16,MNTL"
```

Kết quả mẫu:

```
160.8/ 157.2/ 1.0,16,MNTL hien tai,157.192,m,0,0,0
```

## 3. Định dạng dữ liệu

Mỗi dòng ứng với một kênh, tách bằng dấu `,`:

| Cột (index) | Ý nghĩa | Ví dụ |
|---|---|---|
| 0 | max / min / avg | `160.8/ 157.2/ 1.0` |
| 1 | Số kênh | `16` |
| 2 | Tên kênh | `MNTL hien tai` |
| 3 | **Giá trị** | `157.192` |
| 4 | Đơn vị | `m` |
| 5 | Trạng thái | `0` |

Trạng thái: `0` = không kết nối, `1` = bình thường, `2` = báo thấp, `3` = báo cao.

Macro trả về đủ 16 kênh trong một lần gọi. Thiết bị cập nhật mỗi 2 giây.

## 4. Code Rust

**Cargo.toml**

```toml
[dependencies]
reqwest = { version = "0.12", features = ["blocking"] }
```

**main.rs**

```rust
use std::{thread, time::Duration};

const URL: &str = "http://192.168.200.211:8880/macros/UpdateMonitor";

fn doc_mntl_hien_tai() -> Result<Option<f64>, Box<dyn std::error::Error>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let text = client.post(URL).send()?.error_for_status()?.text()?;

    for line in text.lines() {
        let f: Vec<&str> = line.split(',').collect();
        if f.len() >= 4 && f[1].trim() == "16" {
            return Ok(f[3].trim().parse::<f64>().ok());
        }
    }
    Ok(None)
}

fn main() {
    loop {
        match doc_mntl_hien_tai() {
            Ok(Some(v)) => println!("MNTL hien tai: {:.3} m", v),
            Ok(None) => println!("Không tìm thấy kênh 16"),
            Err(e) => println!("Lỗi kết nối: {}", e),
        }
        thread::sleep(Duration::from_secs(5));
    }
}
```

## 5. Tích hợp vào ứng dụng

- Chạy hàm đọc trong thread riêng, chu kỳ 2-5 giây; không gọi trên thread giao diện.
- Nếu dùng `tokio`: bỏ feature `blocking`, dùng `reqwest::Client` và `.await`.
- Máy chạy ứng dụng phải cùng mạng LAN `192.168.200.x`.
- Đổi IP thiết bị thì sửa hằng số `URL`.


