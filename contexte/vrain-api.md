# Vrain API — điều đã xác minh (2026-11-05, bằng `scripts/probe_vrain_login.py`)

Nguồn mô tả ban đầu: `vrain-rainfall-spec.md`. Dưới đây là phần đã chạy thật; chỗ nào mâu thuẫn thì theo file này.

## Đăng nhập
- `POST https://www.vrain.vn/api/vrain/public/v1/login`, body JSON `{"username","password"}`.
- Header web gửi kèm: `x-org-uuid` (UUID tổ chức, không bí mật, lấy từ trình duyệt), `x-vrain-user-agent` (bằng User-Agent), `Origin`, `Referer`.
- Phản hồi 200 KHÔNG có token trong body (chỉ có `groupName`, `name`, các cờ quyền). Phiên giữ bằng **cookie `sid`** (Set-Cookie). Các lệnh sau gửi lại cookie.
- Các đường `/api/public/v2/login`, `/api/public/v1/login` không dùng được (401/404) — đừng thử lại bằng mật khẩu thật.
- Spec cũ ghi "token trong header": sai với luồng này. `X-Vrain-Token` có trong mã JS nhưng không cần cho hai API dưới đây.

## Lấy mưa
- `GET /api/kttv/private/v1/organizations/details?from=YYYY-MM-DD&to=YYYY-MM-DD&i=_1h` → 200 với cookie `sid`.
- Cấu trúc: `{ detailUUID, groupID, stats: [ { timePoint, stations: [ { id, sid, name, area, depth(chuỗi mm), cityName, address, project, waterStationType } ] } ] }`. Phản hồi >300 KB (cả 55 trạm) → luôn lọc 3 trạm theo `id`.
- `stats[0].timePoint == "Tổng"`: tổng cả khoảng. Các mốc sau có dạng `HH:mm dd/MM` (KHÔNG có năm). Gọi `from=to=hôm nay` ra các mốc từ 20:00 hôm trước đến giờ hiện tại (16 mốc lúc 10:00): ngày vrain tính từ 19:00.
- `stats/summary` trả **danh sách** (1 phần tử nhóm: `groupMapsUrl, groupName, groupOrgUuid, stats, statsMax`), mỗi trạm nằm trong `stats` lồng bên trong; `sumDepth`, `from`, `to`, `level` như spec.
- Chu kỳ hợp lệ: `_10m _1h _3h _6h _12h _1d _1mo`.

## Đã xác minh thêm (J0, 2026-11-05 10:34 giờ VN)
- **Nhiều ngày:** `from` = hôm kia, `to` = hôm nay cho 64 mốc, bắt đầu 20:00 ngày trước `from` (cửa sổ luôn bắt đầu 19:00 hôm trước `from`). Lấy bù 2 ngày dùng được.
- **`x-org-uuid` BẮT BUỘC:** thiếu header này → HTTP 401 dù cookie `sid` hợp lệ.
- **`timePoint` là mốc CUỐI của giờ:** lúc 10:34 mốc giờ cuối là "10:00" (= mưa 09:00–10:00), mốc `_10m` cuối là "10:30"; không có mốc một phần cho giờ đang chạy. Mốc "11:00" xuất hiện sau 11:00 (độ trễ vài phút chưa đo).

## Chưa xác minh
- Đối chiếu mưa thật với kênh 1 ECAPRO khi có mưa (Task 8 của kế hoạch).
- Độ trễ từ lúc hết giờ tới lúc mốc giờ xuất hiện.
- Điều khoản sử dụng của Vrain về lấy dữ liệu tự động (chủ dự án xác nhận được dùng chung tài khoản).
