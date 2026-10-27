# luuluongV2 — Ứng dụng tính lưu lượng nước về hồ Sông Bồ (Rust)

Đầu mỗi hội thoại: đọc CLAUDE.md, MAP.md, PLAN.md. Điều học được lâu dài ghi vào contexte/.

## Vai trò
Người dùng: Đoàn Phước Triều, chủ dự án tại nhà máy thủy điện Sông Bồ. Claude là lập trình viên Rust + hỗ trợ thiết kế UI.

## Mục tiêu
Viết lại bản Python/Tkinter thành app Rust native (macOS + Windows): tính Q về hồ đúng số liệu, kèm dự báo thời tiết A Lưới (OpenWeather). Đặc tả đầy đủ: `Đặc tả kỹ thuật – Ứng dụng tính lưu lượng nước về hồ Sông Bồ (Rust).md` (nguồn chuẩn; mâu thuẫn thì theo đặc tả).

## Cách làm việc
- Trả lời tiếng Việt, ngắn gọn, nêu việc cần làm tiếp theo.
- Mọi chuỗi giao diện/lỗi bằng tiếng Việt, số theo kiểu Việt (799.802,19).
- UI: theo `contexte/thiet-ke-ui.md` và `design-system/` (skill ui-ux-pro-max). Palette trong đặc tả mục 6.3 được ưu tiên hơn palette sinh tự động.

## Quy tắc bắt buộc
- Logic tính toán nằm trong crate `luuluong-core` (thư mục `crates/core`), thuần, không phụ thuộc GUI, có `cargo test`.
- 6 ca thử T1–T6 (đặc tả 3.4) phải luôn pass.
- Tra bảng từ dòng 1, nội suy tuyến tính, ngoài 149–165 m thì từ chối, không ngoại suy.
- Không hiện lỗi kỹ thuật cho người vận hành; chi tiết ghi log.
- Mọi cặp chữ/nền đạt tương phản ≥ 4,5:1, cả sáng và tối.

## Tuyệt đối không
- Không ghi API key vào mã, config, log hay commit; `.env` không vào Git.
- Không `git push --force`, không push thẳng `main`.
- Không đổi công thức ở đặc tả mục 3 khi chưa hỏi.
- Không thêm dependency ngoài mục 7.1 khi chưa hỏi.

## Lệnh
- Build: `cargo build --release -p app`
- Test: `cargo test --workspace`
- Lint: `cargo clippy --workspace -- -D warnings`
(Chưa có Cargo workspace — tạo ở giai đoạn 1 của PLAN.md.)

## Quy tắc viết code
1. Nghĩ trước khi code: nêu giả định; yêu cầu mơ hồ thì hỏi lại hoặc đưa các cách hiểu, không tự chọn im lặng.
2. Đơn giản trước: code tối thiểu để giải quyết đúng việc được giao. Không tính năng, cấu hình hay xử lý lỗi cho tình huống chưa xảy ra.
3. Sửa đúng chỗ: chỉ chạm phần cần sửa; không refactor hay định dạng lại code không liên quan; theo phong cách sẵn có; chỉ dọn rác do chính mình tạo ra.
4. Có tiêu chí thành công: biến việc thành điều kiện kiểm chứng được (vd. viết test tái hiện lỗi rồi làm cho pass); việc nhiều bước thì mỗi bước kèm cách kiểm tra, ghi vào PLAN.md.
Ngoại lệ: người dùng yêu cầu rõ (refactor, dọn dẹp, tách module) thì làm theo yêu cầu.

## Checkpoint
- Mỗi bước xong: tick `[x]` trong PLAN.md rồi commit với thông điệp ngắn.
- Đầu phiên mới hoặc sau gián đoạn: đọc PLAN.md để tiếp tục từ bước đang dở.
