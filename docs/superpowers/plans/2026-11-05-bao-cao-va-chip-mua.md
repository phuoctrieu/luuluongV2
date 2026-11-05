# Chip cảnh báo mưa Vrain và Báo cáo Excel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (người dùng đã chọn Native) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Chip cảnh báo mưa lớn Vrain trên thanh đầu và tab "Báo cáo" xuất Excel (Ngày/Tháng/Năm) gồm lưu lượng về hồ và mưa Hương Nguyên, A Đớt.

**Architecture:** Tổng hợp theo kỳ là hàm thuần trong `crates/app` (`averages.rs` thêm `flow_between`, `rain.rs` thêm `rain_between`/`heavy_alert`, `report.rs` mới ghép thành các dòng báo cáo). Ghi .xlsx nằm ở `luuluong-storage` (`xlsx_report.rs`, kiểu `Sheet`/`Cell` độc lập với app). Giao diện chỉ vẽ.

**Tech Stack:** Rust 1.87, `rust_xlsxwriter` (MỚI, đã được chủ dự án đồng ý 2026-11-05), `calamine` (đã có, dùng để đọc lại file trong test), egui.

**Spec:** `docs/superpowers/specs/2026-11-05-bao-cao-va-chip-mua-design.md` (nối tiếp thiết kế Vrain). Lưu ý đã chốt trước: `rain_sum` neo mốc mới nhất; mốc giờ `ts` = CUỐI giờ.

## Global Constraints

- Giữ nguyên Global Constraints của `2026-11-05-vrain-rainfall.md` (tiếng Việt, số kiểu Việt, tương phản ≥ 4,5:1, không lộ mật khẩu, múi giờ UTC+7, T1–T6, clippy `--workspace -- -D warnings`).
- Dependency mới duy nhất: `rust_xlsxwriter` (storage). Ghi ngoại lệ vào `PLAN.md` và `CLAUDE.md`-liên quan (mục "quyết định đã chốt" của `PLAN.md`).
- Báo cáo chỉ 2 trạm: id `841697470576` (Hương Nguyên) và `84396786852` (A Đớt); lấy từ `config.vrain.stations` theo thứ tự đó, không cứng trong mã (tên + id nằm ở config; báo cáo chọn 2 trạm đầu theo id này).
- Kỳ không có số liệu → ô trống (không ghi 0). Số ghi dạng số, định dạng `#,##0.00`.
- Giới hạn khoảng báo cáo: `từ ≤ đến ≤ hôm nay` và ≤ 3660 ngày.
- Chip: số liệu trạm phải mới (mốc mới nhất cách `now` ≤ 3 giờ).

## Review Focus

1. Ranh giới tháng/năm giờ VN: 28→29/02 năm nhuận (2028), 31/12→01/01, kỳ đầu/cuối bị cắt theo khoảng chọn và theo hôm nay. → Task 3.
2. Kỳ không có số liệu hoặc chỉ có một trạm mưa: ô trống/TB trống, không 0, không chia cho 0. → Task 3.
3. Khoảng ngày sai (từ > đến, tương lai, > 10 năm, chuỗi lạ). → Task 3 (`parse_range`).
4. Chip không bật vì mưa cũ (mốc > 3 giờ) và không bật khi không có số liệu. → Task 2.
5. Tên trạm chứa ký tự đặc biệt (`=`, `+`, dấu nháy) không được hiểu là công thức Excel; file mở được bằng thư viện đọc. → Task 4.
6. Xuất khi không có `history` hoặc thư mục không ghi được: báo câu tiếng Việt, không panic. → Task 5.

---

### Task 1: `flow_between` — lưu lượng về trong một khoảng bất kỳ

**Files:** Modify `crates/app/src/averages.rs`

**Interfaces — Produces:**
```rust
pub struct FlowSpan { pub q_avg: Option<f64>, pub covered: f64, pub volume_m3: Option<f64> }
pub fn flow_between(records: &[Record], from: i64, to: i64) -> FlowSpan;   // [from, to) epoch giây
```
`volume_m3 = q_avg × (covered × (to − from))`; khoảng rỗng (`to ≤ from`) hoặc không chu kỳ nào → `q_avg None`, `volume None`, `covered 0`.

- [ ] **Step 1: Test thất bại** (trong `averages.rs`, dùng helper `cycle`/`rec_at` có sẵn): `flow_between_matches_hand_calculation` (hai chu kỳ 15 phút q=10 và q=20 liền nhau trong khoảng 1 giờ → `q_avg 15`, `covered 0.5`, `volume 15 × 1800 = 27000`); `flow_between_cuts_cycle_at_the_edges` (chu kỳ vắt qua `from` chỉ tính phần trong); `flow_between_empty_is_none` (không dòng, và `to ≤ from` → tất cả `None`/0); `flow_between_ignores_manual_rows` (dòng tính tay không tính).
- [ ] **Step 2:** `cargo test -p app --lib averages::tests::flow_between` → FAIL (thiếu hàm).
- [ ] **Step 3:** Cài đặt `flow_between` bằng `segments` + `span` có sẵn (không đổi hành vi hàm cũ).
- [ ] **Step 4:** PASS toàn bộ `averages::` (các test cũ giữ nguyên). **Step 5:** commit `Báo cáo K1: flow_between (4 test)`.

---

### Task 2: Mưa theo khoảng và chip cảnh báo (hàm thuần)

**Files:** Modify `crates/app/src/rain.rs`

**Interfaces — Consumes:** `RainObs`, `latest_ts`, `rain_sum`, `level`, `RainLevel` (đã có). **Produces:**
```rust
pub struct RainSpan { pub mm: Option<f64>, pub hours: u32, pub expected: u32 }
pub fn rain_between(obs: &[RainObs], station_id: &str, from: i64, to: i64) -> RainSpan;
    // mốc ts ∈ (from, to]; expected = (to − from) / 3600 (làm tròn xuống); mm None nếu hours == 0; mốc trùng đếm một lần
pub struct HeavyAlert { pub text: String, pub station_id: String }
pub fn heavy_alert(obs: &[RainObs], stations: &[(String, String)], now: i64, warn_mm: f64) -> Option<HeavyAlert>;
```
Quy tắc `heavy_alert`: chỉ xét trạm có `latest_ts` với `now − ts ≤ 3·3600`. Nếu có trạm với `rain_sum(…, 24) ≥ warn_mm` → chọn mm 24 giờ cao nhất, văn bản `"Mưa lớn {tên}: {mm} mm / 24 giờ (Vrain)"` (mm 1 chữ số, dấu phẩy). Nếu không, trạm có `rain_sum(…, 1)` > 7,6 cao nhất → `"Mưa to {tên}: {mm} mm / giờ (Vrain)"`. Không có → `None`.

- [ ] **Step 1: Test thất bại:** `rain_between_sums_marks_in_half_open_range` (mốc đúng `from` bị loại, đúng `to` được tính); `rain_between_counts_hours_and_expected`; `rain_between_none_without_marks`; `heavy_alert_by_24h_picks_highest_station` (3 trạm, hai vượt ngưỡng → chọn cao hơn, text đúng `"Mưa lớn A Đớt: 63,4 mm / 24 giờ (Vrain)"`); `heavy_alert_by_hour_when_24h_below_threshold` (1 giờ = 9,2 → `"Mưa to Hương Nguyên: 9,2 mm / giờ (Vrain)"`); `heavy_alert_prefers_24h_over_hour`; `heavy_alert_ignores_stale_stations` (mốc mới nhất 3 giờ + 1 giây trước → `None`; đúng 3 giờ → vẫn xét); `heavy_alert_none_without_data`; `heavy_alert_exactly_at_threshold_fires` (24 giờ = warn_mm → có).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + `cargo clippy --workspace -- -D warnings`. **Step 5:** commit `Báo cáo K2: rain_between, heavy_alert (9 test)`.

---

### Task 3: Dòng báo cáo theo ngày/tháng/năm và kiểm tra khoảng ngày

**Files:** Create `crates/app/src/report.rs`; Modify `crates/app/src/lib.rs` (`pub mod report;`)

**Interfaces — Consumes:** `averages::{flow_between, FlowSpan}`, `rain::{rain_between, RainSpan}`, `Record`, `RainObs`, `VN_TZ`. **Produces:**
```rust
pub enum Granularity { Day, Month, Year }
pub struct Period { pub label: String, pub start: i64, pub end: i64 }        // [start, end) epoch, đã cắt theo khoảng và theo now
pub struct ReportRow { pub period: Period, pub q_avg: Option<f64>, pub volume_m3: Option<f64>, pub covered: f64,
                       pub rain_mm: [Option<f64>; 2], pub rain_mean: Option<f64>, pub rain_covered: Option<f64> }
pub fn periods(from: NaiveDate, to: NaiveDate, g: Granularity, now: i64) -> Vec<Period>;
pub fn build(records: &[Record], obs: &[RainObs], stations: [&str; 2], from: NaiveDate, to: NaiveDate, g: Granularity, now: i64) -> Vec<ReportRow>;
pub struct Totals { pub volume_m3: Option<f64>, pub q_avg: Option<f64>, pub rain_mm: [Option<f64>; 2], pub rain_mean: Option<f64> }
pub fn totals(rows: &[ReportRow]) -> Totals;
pub fn parse_range(from: &str, to: &str, today: NaiveDate) -> Result<(NaiveDate, NaiveDate), String>;  // Err = câu tiếng Việt
```
Nhãn: Day `"05/11/2026"`, Month `"11/2026"`, Year `"2026"`. `periods` cắt kỳ đầu theo `from`, kỳ cuối theo `to + 1 ngày` và theo `now` (không có kỳ hoàn toàn trong tương lai). `rain_mean` = trung bình hai trạm chỉ khi cả hai có `mm`; `rain_covered` = trung bình `hours/expected` của hai trạm (`None` nếu `expected == 0`). `totals.q_avg` = Σ(volume) / Σ(thời gian có số liệu) tính từ `volume` và `covered × độ dài kỳ` của từng hàng. Thông báo lỗi `parse_range`: `"Nhập ngày dạng 31/12/2026."`, `"Ngày bắt đầu phải trước hoặc bằng ngày kết thúc."`, `"Ngày kết thúc không được ở tương lai."`, `"Khoảng báo cáo tối đa 10 năm."`.

- [ ] **Step 1: Test thất bại:** `periods_day_count_and_labels`; `periods_month_clips_first_and_last` (10/11/2026 → 20/12/2026, nhãn `11/2026`, `12/2026`, kỳ đầu bắt đầu 10/11 00:00 VN); `periods_handle_leap_february_2028` (28/02→01/03/2028 ra 29/02); `periods_year_boundary` (31/12/2026 → 01/01/2027 ở mức Day có 2 kỳ, ở mức Year có 2 kỳ `2026`, `2027`); `periods_never_start_after_now`; `build_blank_when_no_data` (Q và mưa `None`, `covered 0`, không NaN); `build_one_station_missing_leaves_mean_blank`; `build_combines_flow_and_rain_for_a_day` (số tính tay); `totals_volume_is_sum_and_q_is_time_weighted`; `totals_blank_when_all_blank`; `parse_range_ok_and_errors` (cả 4 lỗi, và `"5/11/2026"` hợp lệ).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy. **Step 5:** commit `Báo cáo K3: periods, build, totals, parse_range (11 test)`.

---

### Task 4: Ghi file Excel (`luuluong-storage`)

**Files:** Modify `crates/storage/Cargo.toml` (`rust_xlsxwriter = "0.96"` — dùng bản mới nhất tương thích Rust 1.87 mà Cargo chọn), `crates/storage/src/lib.rs`; Create `crates/storage/src/xlsx_report.rs`; Modify `PLAN.md` (ghi ngoại lệ dependency)

**Interfaces — Produces:**
```rust
pub enum Cell { Empty, Text(String), Num(f64) }
pub struct Sheet { pub name: String, pub title: String, pub subtitle: String,
                   pub headers: Vec<String>, pub rows: Vec<Vec<Cell>>, pub footer: Vec<Cell>, pub note: String }
pub fn write_workbook(path: &Path, sheets: &[Sheet]) -> Result<(), StorageError>;
```
Bố cục sheet: A1 `title` (đậm), A2 `subtitle`, dòng 4 `headers` (đậm, nền xám nhạt, căn giữa), dữ liệu từ dòng 5, một dòng `footer` (đậm) ngay sau dữ liệu, rồi một dòng trống và `note`. `Cell::Num` định dạng `#,##0.00`; `Cell::Empty` bỏ trống; `Cell::Text` luôn ghi bằng `write_string` (không bao giờ là công thức). Độ rộng cột: 24 cho cột 0–1, 18 cho các cột còn lại. Đóng băng dòng tiêu đề (dòng 4).

- [ ] **Step 1: Test thất bại** (dùng `calamine` để đọc lại file tạm): `workbook_roundtrips_sheets_headers_and_numbers` (3 sheet đúng tên; ô tiêu đề; một số `1234.5` đọc lại là số `1234.5`); `empty_cell_stays_blank_not_zero`; `formula_like_text_is_stored_as_text` (`"=1+1"` đọc lại đúng chuỗi `=1+1`, không phải `2`); `footer_row_follows_data`; `unwritable_path_is_a_storage_error` (đường dẫn trong thư mục không tồn tại → `Err`, không panic).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy (`cargo clippy --workspace -- -D warnings`). **Step 5:** `PLAN.md` ghi: "Ngoại lệ 7.1: thêm `rust_xlsxwriter` (xuất Excel) — chủ dự án đồng ý 2026-11-05". Commit `Báo cáo K4: xuất .xlsx bằng rust_xlsxwriter (5 test)`.

---

### Task 5: Tab "Báo cáo"

**Files:** Create `crates/app/src/report_screen.rs`; Modify `crates/app/src/main.rs` (`Tab::Report`, thanh tab thứ 6 "Báo cáo" phím 6, `--tab=report`, state: `report_from`, `report_to`, `report_notice`), `MAP.md`

**Interfaces — Consumes:** `report::{parse_range, build, totals, periods, Granularity}`, `storage::xlsx_report::{Sheet, Cell, write_workbook}`, `History::{list_since, rain_since}`.

Nội dung cố định: tiêu đề thẻ "Báo cáo lưu lượng về hồ và mưa thượng nguồn"; ô "Từ ngày"/"Đến ngày" (mặc định 01/01 năm nay → hôm nay), nút "Tháng này", "Năm nay"; lỗi `parse_range` hiện dưới ô theo kiểu `paint::note` Warn; bảng xem trước **theo tháng** (cột Thời gian, Lưu lượng về TB, Tổng lượng về, Mưa Hương Nguyên, Mưa A Đớt, Có số liệu %) với dòng Tổng cuối; nút "Xuất Excel" (tắt khi khoảng sai); xuất → `<data>/exports/bao-cao-luu-luong-ve-<ddmmyyyy>-<ddmmyyyy>.xlsx`, 3 sheet `Ngày`, `Tháng`, `Năm` theo bố cục Task 4 và cột trong thiết kế mục 4; thông báo `"Đã xuất báo cáo: <đường dẫn>"` kèm nút "Mở thư mục"; lỗi: `"Không xuất được file Excel. Kiểm tra ổ đĩa còn chỗ trống."`; không có `history`: `"Chưa có dữ liệu lịch sử để lập báo cáo."`. Truy vấn: `list_since(từ − 1 ngày)` và `rain_since(từ − 1 ngày)`.

- [ ] **Step 1: Test thất bại** (hàm thuần dựng sheet nằm trong `report.rs` để test được không cần GUI): `sheets_from_rows_have_three_named_sheets_in_order`, `sheet_columns_match_the_design` (9 tiêu đề đúng chữ), `blank_rows_become_empty_cells`, `footer_contains_totals`, `file_name_uses_range`. Chữ ký: `pub fn to_sheets(rows_by_g: [(Granularity, Vec<ReportRow>); 3], names: [&str; 2], from: NaiveDate, to: NaiveDate) -> Vec<Sheet>` (đặt trong `report.rs` nhưng `report.rs` khi đó dùng kiểu `Sheet` của storage — `app` đã phụ thuộc `luuluong-storage`) và `pub fn file_name(from: NaiveDate, to: NaiveDate) -> String`.
- [ ] **Step 2–4:** FAIL → cài đặt `to_sheets`, `file_name` → PASS. **Step 5:** viết `report_screen.rs` + nối `main.rs`; `cargo build -p app`, `cargo test --workspace`, clippy sạch.
- [ ] **Step 6: Kiểm tay (người dùng):** mở tab Báo cáo, xuất, mở file bằng Excel/Numbers; so một ngày với bảng "Lịch sử".
- [ ] **Step 7: Commit** `Báo cáo K5: tab Báo cáo, xuất Excel`.

---

### Task 6: Chip cảnh báo mưa Vrain trên thanh đầu

**Files:** Modify `crates/app/src/main.rs` (thanh `nav`: thêm chip thứ hai; `goto_rain`), `crates/app/src/rain_sync.rs` (hàm `vrain_alert(&self) -> Option<String>` gọi `rain::heavy_alert` với 3 trạm của `config.vrain.stations`, `now = unix_now()`, `warn = config.weather.rain_alert_mm`)

- [ ] **Step 1:** `vrain_alert` chỉ là lớp mỏng quanh `heavy_alert` (đã có test ở Task 2); nối vào `nav` bên cạnh chip thời tiết, cùng kiểu (nền `p.error`, chữ `p.on_error`, icon cảnh báo), tooltip "Xem chi tiết ở tab Mưa"; bấm → `self.tab = Tab::Rain`.
- [ ] **Step 2:** `cargo build`, `cargo test --workspace`, clippy sạch. **Step 3: Kiểm tay (người dùng):** với dữ liệu thật/giả có mưa lớn, chip hiện; tắt Vrain hoặc mốc cũ > 3 giờ thì chip biến mất.
- [ ] **Step 4: Commit** `Báo cáo K6: chip cảnh báo mưa lớn Vrain`.

---

### Task 7: Nghiệm thu, tài liệu

**Files:** `MAP.md`, `PLAN.md` (Giai đoạn 10 K1–K7), `contexte/mo-hinh-tinh-toan.md` (một đoạn: cách tính báo cáo, hạn chế dữ liệu)

- [ ] `cargo test --workspace` toàn pass (kể cả T1–T6), `cargo clippy --workspace -- -D warnings` sạch. Ghi số test mới. Đo kích thước file chạy release trước/sau khi thêm `rust_xlsxwriter` (`cargo build --release -p app`; so với mục tiêu ≤ 15 MB). Commit `Báo cáo K7: nghiệm thu, tài liệu`.
