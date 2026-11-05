# Mưa quan trắc Vrain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** App lấy mưa theo giờ của 3 trạm Vrain ở A Lưới mỗi 60 phút (và khi bấm "Cập nhật ngay"), lưu SQLite, hiển thị thẻ tóm tắt ở tab Tính toán và tab "Mưa" riêng, có xuất CSV.

**Architecture:** Crate mới `luuluong-vrain` (đăng nhập, lấy, phân tích, luồng nền) không phụ thuộc GUI. Lưu trữ thêm bảng `rain_obs` vào `history.db` của `luuluong-storage`. Phần tổng hợp thuần và máy trạng thái nút nằm trong `crates/app/src/rain.rs` (có test), giao diện chỉ vẽ.

**Tech Stack:** Rust 1.87, `ureq` 2 (đã có), `serde_json`, `chrono`, `rusqlite`, `mockito` (dev), egui/egui_plot. **Không thêm dependency mới.**

**Spec:** `docs/superpowers/specs/2026-11-05-vrain-rainfall-design.md`; API đã xác minh: `contexte/vrain-api.md`.

## Global Constraints

- Mọi chuỗi giao diện/lỗi bằng tiếng Việt, số kiểu Việt (`1.234,5`); không hiện lỗi kỹ thuật cho người vận hành, chi tiết ghi log.
- Mật khẩu Vrain chỉ ở Keychain (`SecretStore`, service `luuluong`, user `vrain-password`) hoặc `.env` khi chạy thử; KHÔNG vào config, log, URL, `Debug`, thông báo lỗi.
- Không thêm dependency ngoài đặc tả mục 7.1: cookie phiên `sid` được giữ thủ công (đọc `Set-Cookie`, gửi lại header `Cookie`), KHÔNG bật feature `cookies` của `ureq`.
- Không đổi công thức mục 3 đặc tả; mưa Vrain không đi vào Q.
- Mọi cặp chữ/nền đạt tương phản ≥ 4,5:1 sáng và tối; mức mưa luôn kèm chữ, không chỉ màu.
- Múi giờ Việt Nam (UTC+7) cho mọi mốc; lưu `ts` dạng ISO 8601 `+07:00` bằng `vn_timestamp`.
- Chu kỳ làm mới 60 phút; nút "Cập nhật ngay" cách nhau tối thiểu 30 giây.
- 6 ca T1–T6, `cargo test --workspace`, `cargo clippy --workspace -- -D warnings` luôn sạch.
- Mỗi task xong: tick `[x]` trong `PLAN.md` rồi commit ngắn, kết thúc bằng dòng `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.

## Review Focus

1. `timePoint` qua giao thừa ("00:00 01/01" khi hôm nay là 01/01, và "23:00 31/12" khi vừa sang năm mới) phải ra đúng năm. → Task 1.
2. `depth` là `""`, `null`, `"abc"` hoặc thiếu trạm trong một mốc: bỏ mốc/trạm đó, không panic, không ghi số sai. → Task 1.
3. Phiên hết hạn giữa chừng (401): đăng nhập lại đúng 1 lần, không lặp vô hạn; sai mật khẩu báo lỗi tài khoản. → Task 2.
4. Mật khẩu không xuất hiện trong `format!("{:?}", target)` hay thông báo lỗi. → Task 3.
5. Bấm "Cập nhật ngay" liên tục, hoặc khi chưa nhập tài khoản, hoặc khi đang tải. → Task 5.
6. Mốc giờ hiện tại được ghi lại khi đủ giờ (cùng khóa, giá trị mới thay giá trị cũ). → Task 4.

---

### Task 0: Xác minh ba điều còn mở (người dùng chạy script)

**Files:**
- Modify: `scripts/probe_vrain_login.py` (thêm kiểm tra, không ghi mật khẩu)

**Interfaces:** Produces: ba kết luận ghi vào `contexte/vrain-api.md` — (a) `from` nhiều ngày có trả đủ mốc không (quyết định `BACKFILL_DAYS` = 2 hay 0), (b) `x-org-uuid` có bắt buộc không, (c) mốc giờ hiện tại có số ngay không.

- [ ] **Step 1:** Thêm vào script: gọi `details?i=_1h` với `from` = hôm kia, `to` = hôm nay, in số mốc và `timePoint` đầu/cuối; gọi lại không có header `x-org-uuid`, in mã HTTP; in `timePoint` cuối cùng cùng giờ hiện tại để thấy mốc cuối có là giờ chưa trọn.
- [ ] **Step 2:** Người dùng chạy `! python3 scripts/probe_vrain_login.py`, dán kết quả.
- [ ] **Step 3:** Ghi kết luận vào `contexte/vrain-api.md` (mục "Chưa xác minh" → "Đã xác minh"), commit `docs: Vrain – kết quả thăm dò nhiều ngày`.

---

### Task 1: Crate `luuluong-vrain` — kiểu dữ liệu và phân tích phản hồi

**Files:**
- Create: `crates/vrain/Cargo.toml` (mẫu `crates/logger/Cargo.toml`; deps: `thiserror = "1"`, `ureq = { version = "2", features = ["json"] }`, `serde_json = "1"`, `chrono = { version = "0.4", default-features = false, features = ["std"] }`; dev: `mockito = "1"`), `crates/vrain/src/lib.rs`, `src/error.rs`, `src/parse.rs`, `crates/vrain/tests/data/vrain_details.json`
- Modify: `Cargo.toml` (thêm `"crates/vrain"` vào `members`), `MAP.md`

**Interfaces — Produces:**
```rust
pub struct Obs { pub station_id: String, pub ts: i64, pub mm: f64 }       // ts = epoch giây, CUỐI giờ
pub struct ParseOutcome { pub obs: Vec<Obs>, pub missing: Vec<String> }  // missing = id trạm không có trong phản hồi
pub enum VrainError { Credentials, Unreachable, Format }                  // Display tiếng Việt, PartialEq
pub fn parse_details(json: &str, ids: &[String], now: i64) -> Result<ParseOutcome, VrainError>;
pub fn vn_date(unix: i64) -> String;                                      // "YYYY-MM-DD" giờ VN
```
Thông báo: `Credentials` = "Tài khoản Vrain không đúng, kiểm tra trong Cài đặt."; `Unreachable` = "Không kết nối được Vrain."; `Format` = "Vrain trả dữ liệu không đọc được."

- [ ] **Step 1: Tạo fixture** `tests/data/vrain_details.json`: `stats` gồm mốc `"Tổng"`, `"09:00 05/11"`, `"10:00 05/11"`; mỗi mốc có `stations` gồm 3 trạm (`841697470576`, `84396786852`, `84846302387`, `depth` chuỗi như `"0.2"`) và 1 trạm lạ `84845529137`.
- [ ] **Step 2: Viết test thất bại** (`parse.rs`, `#[cfg(test)]`): `parses_three_stations_and_skips_total` (now = 2026-11-05T10:05 +07 → 6 `Obs`, không có mốc "Tổng", không có trạm lạ, `0.2` thành `0.2_f64`, ts của "10:00 05/11" = epoch của 2026-11-05T10:00+07:00); `missing_station_listed` (bỏ `84846302387` khỏi fixture → `missing == ["84846302387"]`, các trạm khác vẫn có); `bad_depth_skipped` (`""`, `null`, `"abc"` cho một mốc → mốc/trạm đó bị bỏ, kết quả còn lại giữ nguyên); `new_year_rollover` (now = 2027-01-01T00:30 +07, `timePoint` `"23:00 31/12"` → năm 2026, `"00:00 01/01"` → 2027); `future_stamp_goes_back_a_year` (now = 2026-01-01T00:30, `"23:00 31/12"` → 2025); `wrong_structure_is_format_error` (`{}` hoặc không phải JSON → `Err(Format)`).
- [ ] **Step 3: Chạy** `cargo test -p luuluong-vrain` → FAIL (chưa có hàm).
- [ ] **Step 4: Cài đặt** `parse_details` và `vn_date`. Cách làm: `serde_json::Value`, lấy `stats` (không phải mảng → `Format`); suy năm: ghép năm của `now` (giờ VN), nếu kết quả > `now + 3600` thì lùi 1 năm; đổi ra epoch bằng `chrono::NaiveDate`/`NaiveDateTime` trừ 7 giờ; `depth` parse bằng `str::parse::<f64>` và loại NaN/âm.
- [ ] **Step 5: Chạy** `cargo test -p luuluong-vrain && cargo clippy -p luuluong-vrain -- -D warnings` → PASS, sạch.
- [ ] **Step 6: Commit** `git add Cargo.toml crates/vrain MAP.md && git commit -m "Vrain 1: crate vrain, parse_details (6 test)"`.

---

### Task 2: Client — đăng nhập, cookie `sid`, lấy theo giờ

**Files:**
- Create: `crates/vrain/src/client.rs`; Modify: `crates/vrain/src/lib.rs` (`pub use`)

**Interfaces — Consumes:** `parse_details`, `ParseOutcome`, `VrainError`, `vn_date` (Task 1). **Produces:**
```rust
pub struct Credentials { pub username: String, pub password: String }   // Debug tự viết, che password
pub struct Client { /* base, org_uuid, sid: Option<String> */ }
impl Client {
    pub fn new(base: &str, org_uuid: &str) -> Client;
    pub fn fetch_hourly(&mut self, c: &Credentials, from: &str, to: &str, ids: &[String], now: i64)
        -> Result<ParseOutcome, VrainError>;
}
```
Giá trị cố định: đăng nhập `POST {base}/api/vrain/public/v1/login` body `{"username","password"}`; lấy `GET {base}/api/kttv/private/v1/organizations/details?from=&to=&i=_1h`; gửi header `x-org-uuid`, `Origin: https://www.vrain.vn`, `Referer: https://www.vrain.vn/landing`, `Cookie: sid=…`; timeout 15 giây; thử lại tối đa 3 lần cách 1 s, 2 s khi lỗi mạng/5xx.

- [ ] **Step 1: Test thất bại bằng `mockito`:** `logs_in_then_fetches` (mock login trả 200 + `Set-Cookie: sid=abc; Path=/`; mock details yêu cầu header `cookie: sid=abc` và `x-org-uuid`, trả fixture → có `Obs`); `relogins_once_on_401` (details trả 401 lần đầu, login lần 2, details 200 → thành công, login được gọi đúng 2 lần); `no_endless_relogin` (details luôn 401 → `Err(Credentials)`, login gọi tối đa 2 lần); `wrong_password` (login 401 → `Err(Credentials)`); `server_500_retries_then_unreachable`; `bad_json_is_format`; `debug_hides_password` (`format!("{:?}", Credentials{password:"hunter2".into(),..})` không chứa `hunter2`).
- [ ] **Step 2: Chạy** `cargo test -p luuluong-vrain client` → FAIL.
- [ ] **Step 3: Cài đặt** `Client`: đọc `Set-Cookie` lấy `sid=<giá trị>` thủ công; không log header/cookie/body; lỗi mạng ghi `eprintln`-không, dùng `Display` của `VrainError` (log chi tiết do tầng app lo).
- [ ] **Step 4: Chạy** test + clippy → PASS.
- [ ] **Step 5: Commit** `Vrain 2: client đăng nhập cookie sid, relogin 1 lần (7 test)`.

---

### Task 3: Luồng nền có nút kích hoạt

**Files:**
- Create: `crates/vrain/src/poller.rs`; Modify: `crates/vrain/src/lib.rs`

**Interfaces — Consumes:** `Client`, `Credentials`, `vn_date`. **Produces:**
```rust
pub struct Target { pub base: String, pub org_uuid: String, pub creds: Credentials,
                    pub ids: Vec<String>, pub every: Duration, pub backfill_days: u32 }  // Debug che mật khẩu (qua Credentials)
pub type PollResult = (i64, Result<ParseOutcome, VrainError>);
pub fn spawn(target: Arc<Mutex<Target>>, tx: Sender<PollResult>, trigger: Receiver<()>,
             wake: impl Fn() + Send + 'static) -> JoinHandle<()>;
```
Hành vi: vòng lặp: lấy kết quả → `tx.send((now, kết quả))` → `wake()` → chờ `trigger.recv_timeout(every)`; nhận tín hiệu hoặc hết hạn thì lấy lại ngay. Lần đầu dùng `from = vn_date(now − backfill_days·86400)`, các lần sau `from = to = hôm nay`. Dừng khi `tx.send` lỗi hoặc `trigger` bị bỏ (Disconnected).

- [ ] **Step 1: Test thất bại:** `fetches_on_start_and_on_trigger` (mock details `expect_at_least(2)`, `every` = 1 giờ, gửi `()` vào trigger → nhận kết quả thứ hai trong < 3 s); `stops_when_receiver_dropped`; `target_debug_hides_password`; `first_fetch_uses_backfill_range` (mock kiểm tham số `from` bằng `Matcher::Regex`).
- [ ] **Step 2–4:** chạy FAIL → cài đặt theo mẫu `crates/logger/src/poller.rs` → PASS + clippy.
- [ ] **Step 5: Commit** `Vrain 3: luồng nền có trigger (4 test)`.

---

### Task 4: Lưu trữ SQLite, CSV, cấu hình

**Files:**
- Modify: `crates/storage/src/history.rs` (thêm bảng + hàm), `crates/storage/src/csv_export.rs`, `crates/storage/src/config.rs`, `crates/storage/src/lib.rs`
- Test: trong từng file `#[cfg(test)]`

**Interfaces — Produces:**
```rust
// history.rs
pub struct RainObs { pub station_id: String, pub ts: i64, pub mm: f64 }
impl History {
    pub fn upsert_rain(&self, obs: &[RainObs]) -> Result<usize, StorageError>;   // INSERT OR REPLACE, khóa (station_id, ts, interval)
    pub fn rain_since(&self, from_unix: i64) -> Result<Vec<RainObs>, StorageError>; // tăng dần theo ts
}
// csv_export.rs
pub fn rain_to_csv(obs: &[RainObs], names: &[(String, String)]) -> String;      // cột: Thời điểm;Trạm;Lượng mưa (mm); dấu ';' và ',', có BOM như CSV lịch sử
// config.rs
pub struct Vrain { pub enabled: bool, pub username: String, pub refresh_minutes: u32,
                   pub org_uuid: String, pub stations: Vec<VrainStation> }
pub struct VrainStation { pub id: String, pub name: String }
impl Vrain { pub fn refresh(&self) -> u32 }   // 0 hoặc < 5 → 60
```
Mặc định `Vrain`: `enabled = false`, `username = ""`, `refresh_minutes = 60`, `org_uuid = "d50c9e31-74b5-4f44-b2c9-ba76c073340a"`, 3 trạm: `841697470576` "Hương Nguyên", `84396786852` "A Đớt", `84846302387` "Tà Lương". Bảng `rain_obs(station_id TEXT NOT NULL, ts TEXT NOT NULL, mm REAL NOT NULL, interval TEXT NOT NULL DEFAULT '_1h', PRIMARY KEY(station_id, ts, interval))` tạo bằng `CREATE TABLE IF NOT EXISTS` cùng chỗ mở DB.

- [ ] **Step 1: Test thất bại:** `upsert_is_idempotent` (ghi 2 lần cùng mốc → còn 1 dòng); `later_value_replaces_partial_hour` (ghi `mm=0.2`, ghi lại cùng khóa `mm=0.6` → đọc ra `0.6`); `rain_since_filters_and_orders`; `old_db_gets_table` (DB tạo bằng schema cũ không có `rain_obs` → mở lại không lỗi, ghi được); `csv_has_bom_semicolon_comma` (`0.2` → `0,2`); `config_missing_vrain_uses_defaults` (file cũ không có `[vrain]` vẫn nạp); `refresh_below_5_becomes_60`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy.
- [ ] **Step 5: Commit** `Vrain 4: bảng rain_obs, CSV, [vrain] (7 test)`.

---

### Task 5: Logic thuần của app — tổng hợp, mức mưa, cổng nút làm mới

**Files:**
- Create: `crates/app/src/rain.rs`; Modify: `crates/app/src/lib.rs` (khai báo mod như `averages`), `crates/app/Cargo.toml` (thêm `luuluong-vrain = { path = "../vrain" }`)

**Interfaces — Consumes:** `luuluong_storage::RainObs`, `averages::window_averages` (để lấy mưa ECAPRO 3 giờ/12 giờ/1 ngày/1 tuần). **Produces:**
```rust
pub fn rain_sum(obs: &[RainObs], station_id: &str, now: i64, hours: u32) -> Option<f64>;
    // tổng các mốc ts ∈ (now − hours·3600, now]; None nếu số mốc < hours (thiếu số liệu, không nội suy)
pub enum RainLevel { None, Light, Moderate, Heavy }
pub fn level(mm_per_hour: f64) -> RainLevel;   // 0 → None; ≤ 2,5 → Light; ≤ 7,6 → Moderate; > 7,6 → Heavy
pub fn level_label(l: RainLevel) -> &'static str; // "Không mưa" / "Mưa nhỏ" / "Mưa vừa" / "Mưa to"
pub fn is_stale(last_ok: Option<i64>, now: i64) -> bool;  // > 2 giờ hoặc chưa từng lấy được
pub fn hourly_series(obs: &[RainObs], station_id: &str, now: i64, hours: u32) -> Vec<(i64, f64)>;
pub fn compare_note(vrain_mean_mm: Option<f64>, ecapro_mm: Option<f64>, warn_mm: f64) -> Option<String>;
    // chênh ≥ warn_mm/2 → Some("Trạm nhà máy lệch so với trung bình 3 trạm Vrain…"); thiếu 1 trong 2 → None
pub struct RefreshGate { /* last_request, busy */ }
pub enum RefreshState { Idle, Loading, Wait(u32) , NeedsSetup }
impl RefreshGate {
    pub fn new() -> Self;
    pub fn state(&self, now: i64, configured: bool) -> RefreshState;
    pub fn try_start(&mut self, now: i64, configured: bool) -> bool;  // true = được gửi tín hiệu trigger
    pub fn finish(&mut self);
}
```
Quy tắc cổng: chưa `configured` → `NeedsSetup`, `try_start` = false; đang `busy` → `Loading`, false; cách lần bấm trước < 30 s → `Wait(30 − Δ)`, false; còn lại `Idle` và `try_start` đặt `busy`, ghi `last_request`.

- [ ] **Step 1: Test thất bại:** `sum_1h_3h_24h_exact`, `sum_none_when_hour_missing`, `sum_other_station_ignored`, `level_thresholds` (0 → None; 2,5 → Light; 2,6 → Moderate; 7,6 → Moderate; 7,7 → Heavy), `stale_after_two_hours_or_never`, `compare_note_threshold`, `gate_blocks_second_click_within_30s_then_allows`, `gate_busy_blocks_until_finish`, `gate_needs_setup_when_not_configured`, `gate_wait_counts_down`, `series_orders_and_limits`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy.
- [ ] **Step 5: Commit** `Vrain 5: tổng hợp mưa, mức mưa, cổng nút cập nhật (11 test)`.

---

### Task 6: Nối luồng nền vào app và Cài đặt

**Files:**
- Modify: `crates/app/src/main.rs` (state: `vrain_target`, `vrain_rx`, `vrain_trigger: Sender<()>`, `rain_obs: Vec<RainObs>`, `rain_last_ok: Option<i64>`, `rain_error: Option<String>`, `gate: RefreshGate`; khởi động luồng khi `enabled` và có mật khẩu; mỗi khung nhận `try_recv`, ghi `upsert_rain`, nạp lại `rain_since(now − 8 ngày)`; lỗi ghi lưu → cảnh báo nhỏ, vẫn hiển thị), `crates/app/src/settings.rs` + `settings_screen.rs` (mục "Vrain": công tắc bật, tài khoản, mật khẩu → `SecretStore` (`KeyringStore::new("luuluong","vrain-password")`), chu kỳ làm mới, nút "Thử đăng nhập")
- Test: `settings.rs` (kiểm tra biểu mẫu)

**Interfaces — Consumes:** Task 3 `spawn/Target`, Task 4 `Config.vrain/History`, Task 5 `RefreshGate`.

- [ ] **Step 1: Test thất bại** (`settings.rs`): `vrain_requires_username_when_enabled` ("Vui lòng nhập tài khoản Vrain."); `vrain_refresh_minutes_range_5_to_1440`; `password_never_written_to_config` (lưu config rồi đọc file `config.toml` dạng chuỗi → không chứa mật khẩu).
- [ ] **Step 2–3:** FAIL → cài đặt kiểm tra biểu mẫu → PASS.
- [ ] **Step 4:** Nối luồng nền và Cài đặt (không có test GUI): bật/tắt công tắc khởi động/dừng luồng; đổi tài khoản/mật khẩu cập nhật `Arc<Mutex<Target>>`; "Thử đăng nhập" gọi `trigger` và hiện kết quả bằng câu tiếng Việt từ `VrainError`.
- [ ] **Step 5: Kiểm tay:** chạy `cargo run -p app`, bật Vrain với tài khoản thật (nhập mật khẩu trong Cài đặt), xem `history.db` có dòng `rain_obs` (`sqlite3 history.db "select count(*) from rain_obs"`).
- [ ] **Step 6: Commit** `Vrain 6: nối luồng nền, Cài đặt Vrain, Keychain`.

---

### Task 7: Giao diện — thẻ tóm tắt, tab "Mưa", nút "Cập nhật ngay"

**Files:**
- Create: `crates/app/src/rain_screen.rs`; Modify: `crates/app/src/live_screen.rs` (thẻ "Mưa quan trắc thượng nguồn"), `crates/app/src/chart.rs` (biểu đồ cột theo giờ nhiều chuỗi: 3 trạm Vrain + ECAPRO, Okabe–Ito, số trên đầu cột), `crates/app/src/main.rs` (`Tab::Rain` là tab thứ 5, nút trên thanh đầu, phím `Cmd/Ctrl+5`, phím `Cmd/Ctrl+Shift+R` = "Cập nhật ngay"; phím 1–4 và `Cmd+R` giữ nguyên)

**Interfaces — Consumes:** `rain::{rain_sum, level, level_label, is_stale, hourly_series, compare_note, RefreshGate/RefreshState}`.

Nội dung cố định:
- **Thẻ tóm tắt:** tiêu đề "Mưa quan trắc thượng nguồn"; 3 ô trạm (tên, 1 giờ / 3 giờ / 24 giờ bằng định dạng Việt `0,2 mm`, `—` khi `None`, chữ mức mưa + chấm màu, đổi màu cảnh báo khi 24 giờ ≥ ngưỡng mưa ngày của thời tiết); dòng "Trạm nhà máy (ECAPRO): …" cho 3 giờ; "Cập nhật lúc HH:MM"; nhãn "dữ liệu cũ" khi `is_stale`; nút **Cập nhật ngay**.
- **Nút:** `Idle` → "Cập nhật ngay"; `Loading` → "Đang cập nhật…" (khóa); `Wait(n)` → "Thử lại sau n giây" (khóa); `NeedsSetup` → "Cài đặt Vrain" (chuyển tới Cài đặt); lỗi hiện câu từ `VrainError` kèm giữ số cũ.
- **Tab Mưa:** bảng 3 trạm × (1 giờ, 3 giờ, 6 giờ, 12 giờ, 24 giờ, 7 ngày) + hàng ECAPRO (chỉ cột có tương ứng 3 giờ/12 giờ/1 ngày/1 tuần, còn lại `—`); chọn 24/48/72 giờ cho biểu đồ cột; dòng `compare_note`; nút Cập nhật ngay; nút "Xuất CSV" (`rain_to_csv`, hộp lưu file như tab Lịch sử); trạng thái rỗng "Chưa có số liệu Vrain — bật trong Cài đặt".

- [ ] **Step 1:** Cài đặt giao diện theo nội dung trên, theo phong cách `live_screen.rs`/`history_screen.rs`; chuỗi tiếng Việt trong `view.rs` nếu là hàm định dạng dùng lại.
- [ ] **Step 2: Chụp ảnh** 900×600 sáng và tối với: (a) bình thường, (b) mất mạng/dữ liệu cũ, (c) mưa lớn, (d) chưa bật/chưa có dữ liệu, (e) nút ở 4 trạng thái; kiểm tương phản ≥ 4,5:1 bằng công cụ như các màn trước. Ghi đường dẫn ảnh vào `PLAN.md`.
- [ ] **Step 3:** `cargo build -p app` sạch cảnh báo; commit `Vrain 7: thẻ tóm tắt, tab Mưa, nút Cập nhật ngay`.

---

### Task 8: Nghiệm thu và tài liệu

**Files:** Modify `MAP.md`, `PLAN.md`, `contexte/vrain-api.md`, `contexte/mo-hinh-tinh-toan.md` (một đoạn: mưa Vrain chỉ để hiển thị); xóa `scripts/probe_vrain_login.py` và `scripts/__pycache__` (rác do ta tạo), hoặc giữ nếu chủ dự án muốn.

- [ ] **Step 1:** `cargo test --workspace` → toàn bộ pass (kể cả T1–T6); `cargo clippy --workspace -- -D warnings` → sạch.
- [ ] **Step 2: Thử thật:** chạy app với tài khoản thật 1 giờ+; so một mốc giờ với trang vrain.vn; khi có mưa, đối chiếu `timePoint` là cuối hay đầu giờ với kênh 1 ECAPRO, ghi kết luận vào `contexte/vrain-api.md`. Nếu là đầu giờ: đổi `ts` thành `+1 giờ` trong `parse_details` và sửa test (một chỗ).
- [ ] **Step 3:** Đo RSS/khởi động như `PLAN.md` D1 (không được xấu hơn rõ rệt); ghi số đo.
- [ ] **Step 4:** Cập nhật tài liệu; ghi rõ phần đã/chưa thử thật; commit `Vrain 8: nghiệm thu, tài liệu`. Hỏi chủ dự án trước khi tăng phiên bản (0.2.0 → 0.3.0) hoặc gắn thẻ.
