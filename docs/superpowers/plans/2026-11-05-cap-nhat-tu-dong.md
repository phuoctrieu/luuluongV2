# Cập nhật ứng dụng từ GitHub Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans (người dùng đã chọn Native cho các kế hoạch trước) to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** App kiểm tra bản mới trên GitHub Releases, báo bằng chip + thẻ Cài đặt, và khi người dùng bấm thì tải, kiểm SHA-256, giải nén, thay thế chính nó rồi mở lại.

**Architecture:** Crate mới `luuluong-update` (`crates/update`) chứa toàn bộ logic không GUI: so phiên bản, đọc phản hồi release, tải + kiểm hash, giải nén an toàn, thay thế bằng đổi tên có quay lui. `crates/app` chỉ giữ máy trạng thái, luồng nền và giao diện. Phát hành thêm tài sản `.app.zip` cho macOS.

**Tech Stack:** Rust 1.87, `ureq` 2 (đã có), `serde_json` (đã có), `sha2` và `zip` (MỚI, đã được chủ dự án đồng ý 2026-11-05), `mockito` (dev).

**Spec:** `docs/superpowers/specs/2026-11-05-cap-nhat-tu-dong-design.md`.

## Global Constraints

- Giữ Global Constraints của các kế hoạch Vrain/Báo cáo (tiếng Việt, không lộ lỗi kỹ thuật, tương phản ≥ 4,5:1, clippy `--workspace -- -D warnings`, T1–T6, không `push`/gắn thẻ/phát hành khi chưa hỏi).
- Dependency mới duy nhất: `sha2` và `zip` (đặt `zip` với `default-features = false, features = ["deflate"]`). Ghi ngoại lệ vào `PLAN.md`.
- Nguồn cố định: `https://api.github.com/repos/phuoctrieu/luuluongV2/releases/latest`; URL tải phải bắt đầu bằng `https://github.com/phuoctrieu/luuluongV2/releases/download/`. Chỉ HTTPS. Không dùng token.
- Không bao giờ cài khi chưa có xác nhận của người dùng; `digest` thiếu hoặc không phải `sha256:` → từ chối.
- Tên tài sản theo nền tảng: macOS `luuluong-<ver>-macos-universal.app.zip`; Windows `luuluong-<ver>-windows-x64.zip`.
- Giới hạn tải: ≤ 100 MB; timeout kết nối 15 giây, đọc 60 giây.
- Cập nhật không được đụng dữ liệu người dùng (thư mục dữ liệu/cấu hình nằm ngoài gói).

## Review Focus

1. Zip có đường dẫn thoát (`../x`, tuyệt đối): từ chối, không ghi gì ngoài thư mục đích. → Task 3.
2. `digest` sai/thiếu/không phải sha256: xóa file, không cài. → Task 1, 2.
3. So phiên bản: `0.10.0` > `0.9.0`; bằng nhau không cập nhật; bản "mới nhất" cũ hơn bản đang chạy không hạ cấp; tiền tố `v`; thẻ lạ (`v1.0`, `abc`) không làm app lỗi. → Task 1.
4. Mạng/HTTP: offline, 403 hết lượt, 404, 5xx, timeout, JSON sai → câu tiếng Việt, app chạy bình thường. → Task 1, 2.
5. Thay thế: thư mục cài không ghi được hoặc bước giữa chừng lỗi → quay lui, bản cũ còn nguyên chạy được. → Task 4.
6. Tải dở (mất mạng giữa chừng, tắt app) để lại file tạm: lần chạy sau dọn. → Task 4, 6.
7. Bấm "Tải và cài" hai lần / khi đang tải. → Task 6.

---

### Task 1: Phiên bản và đọc phản hồi release (`crates/update`)

**Files:** Create `crates/update/Cargo.toml` (deps: `thiserror = "1"`, `serde_json = "1"`, `ureq = { version = "2", features = ["json"] }`, `sha2`, `zip`; dev: `mockito = "1"`), `crates/update/src/{lib.rs,version.rs,release.rs,error.rs}`, `crates/update/tests/data/release_latest.json` (rút gọn từ phản hồi thật của `v0.2.0`: `tag_name`, `body`, `draft`, `prerelease`, `assets[]` có `name`, `browser_download_url`, `size`, `digest`); Modify `Cargo.toml` (members), `MAP.md`

**Interfaces — Produces:**
```rust
pub struct Version(pub u32, pub u32, pub u32);
pub fn parse_version(tag: &str) -> Option<Version>;                  // chấp nhận "v0.2.0" và "0.2.0"; "1.0", "abc", "0.2.0-beta" → None
pub enum Platform { MacOs, Windows }
pub struct Release { pub version: Version, pub notes: String, pub asset_name: String, pub url: String, pub size: u64, pub sha256: String }
pub fn parse_release(json: &str, platform: Platform) -> Result<Release, UpdateError>;
pub enum UpdateError { Offline, RateLimited, NotFound, BadResponse, NoAsset, BadDigest, TooLarge, HashMismatch, NotWritable, NotInstalled, Io }  // Display tiếng Việt, PartialEq
pub fn is_newer(latest: Version, current: Version) -> bool;
pub fn notes_excerpt(notes: &str) -> String;                         // cắt ~600 ký tự tại ranh giới từ, thêm "…"
```
`parse_release` bỏ qua bản `draft`/`prerelease` (trả `NoAsset`), chọn tài sản theo `Platform`, yêu cầu `digest` dạng `sha256:<64 hex>` (chữ thường, ép về chữ thường), `url` đúng tiền tố cho phép, `size ≤ 100 MB`.

- [ ] **Step 1: Test thất bại:** `parse_version_accepts_v_prefix_and_rejects_oddities`; `ten_beats_nine` (`0.10.0` mới hơn `0.9.0`); `equal_or_older_is_not_newer`; `release_picks_asset_per_platform` (fixture có đủ `.dmg`, `.app.zip`, `.zip`: macOS lấy `.app.zip`, Windows lấy `windows-x64.zip`, không lấy `.dmg`); `release_without_digest_is_rejected`; `release_with_md5_digest_is_rejected`; `release_url_outside_github_is_rejected`; `release_over_100mb_is_rejected`; `prerelease_and_draft_are_ignored`; `release_missing_platform_asset_is_no_asset`; `bad_json_is_bad_response`; `notes_are_cut_at_a_word_boundary`; `error_messages_are_vietnamese_and_have_no_technical_text`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + `cargo clippy --workspace -- -D warnings`. **Step 5:** commit `Cập nhật M1: phiên bản, đọc release (13 test)`.

---

### Task 2: Gọi API và tải có kiểm SHA-256

**Files:** Create `crates/update/src/net.rs`; Modify `lib.rs`

**Interfaces — Consumes:** `Release`, `UpdateError`, `parse_release`. **Produces:**
```rust
pub const LATEST_URL: &str = "https://api.github.com/repos/phuoctrieu/luuluongV2/releases/latest";
pub fn check(base: &str, platform: Platform) -> Result<Release, UpdateError>;     // base = "https://api.github.com" (test: URL mockito); GET {base}/repos/…/releases/latest
pub fn download(url: &str, dest: &Path, expected_size: u64, expected_sha256: &str, progress: &mut dyn FnMut(u64, u64)) -> Result<(), UpdateError>;
```
`check` gửi `User-Agent: luuluong-updater`, `Accept: application/vnd.github+json`; ánh xạ lỗi: không kết nối/timeout → `Offline`; 403/429 → `RateLimited`; 404 → `NotFound`; 5xx → `Offline`; JSON sai → `BadResponse`. `download` nhận URL bất kỳ (việc kiểm tiền tố GitHub và HTTPS đã làm ở `parse_release`, nên test dùng được URL `mockito`); ghi vào `dest.part`, tính SHA-256 khi tải, gọi `progress(đã, tổng)`, dừng nếu vượt `expected_size`; khớp hash và size thì `rename` thành `dest`; sai → xóa `.part`, trả `HashMismatch`.

- [ ] **Step 1: Test thất bại (mockito):** `check_parses_a_good_response`; `check_maps_403_to_rate_limited`, `404_to_not_found`, `500_to_offline`; `check_unreachable_is_offline`; `download_writes_file_and_reports_progress` (nội dung 3 KB, progress đơn điệu, kết thúc `(3072, 3072)`); `download_with_wrong_hash_deletes_everything` (không còn `dest` lẫn `dest.part`); `download_longer_than_declared_is_rejected`; `download_http_error_is_offline_and_leaves_no_file`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy. **Step 5:** commit `Cập nhật M2: kiểm tra và tải có xác minh SHA-256 (8 test)`.

---

### Task 3: Giải nén an toàn

**Files:** Create `crates/update/src/unpack.rs`; Modify `lib.rs`

**Interfaces — Produces:**
```rust
pub fn unpack(zip_path: &Path, dest: &Path) -> Result<(), UpdateError>;   // ghi vào dest (tạo nếu chưa có)
```
Quy tắc: dùng `ZipFile::enclosed_name()`; có bất kỳ mục nào thoát khỏi `dest` (chứa `..`, đường dẫn tuyệt đối, ổ đĩa) → **không giải nén gì**, `Err(BadResponse)` (kiểm tra toàn bộ mục trước khi ghi); giữ bit thực thi trên Unix (`unix_mode`); giới hạn tổng dung lượng giải nén 300 MB (chống zip bomb) → `TooLarge`.

- [ ] **Step 1: Test thất bại** (dựng zip bằng chính thư viện `zip` trong test): `unpack_extracts_files_and_directories`; `unpack_keeps_executable_bit` (`#[cfg(unix)]`); `unpack_rejects_parent_traversal_and_writes_nothing` (mục `../evil.txt` kèm mục hợp lệ: `dest` vẫn trống, `../evil.txt` không tồn tại); `unpack_rejects_absolute_paths`; `unpack_rejects_oversized_archive`; `unpack_corrupt_zip_is_bad_response`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy. **Step 5:** commit `Cập nhật M3: giải nén zip an toàn (6 test)`.

---

### Task 4: Thay thế bằng đổi tên, có quay lui

**Files:** Create `crates/update/src/install.rs`; Modify `lib.rs`

**Interfaces — Produces:**
```rust
pub enum Target { MacApp(PathBuf), WinExe(PathBuf) }                   // MacApp = đường dẫn tới "….app"; WinExe = đường dẫn "luuluong.exe" đang chạy
pub fn detect_target(current_exe: &Path) -> Result<Target, UpdateError>; // …/X.app/Contents/MacOS/luuluong → MacApp(…/X.app); …/luuluong.exe → WinExe; còn lại (vd. target/debug/app) → Err(NotInstalled)
pub fn swap(target: &Target, new_payload_dir: &Path) -> Result<(), UpdateError>;
    // MacApp: new_payload_dir chứa "<tên>.app"; bước: kiểm "….app.old" không tồn tại (xóa nếu có), đổi tên app → app.old, đổi tên bản mới → vị trí cũ; lỗi bước 2 thì đổi lại app.old → app (quay lui) rồi trả NotWritable/Io.
    // WinExe: new_payload_dir chứa "luuluong.exe"; exe → "luuluong.old.exe", bản mới vào chỗ cũ; quay lui tương tự.
pub fn cleanup(target: &Target);                                       // xóa ".old" và thư mục tạm "luuluong-update-*" còn sót; không lỗi nếu không có
pub fn relaunch_command(target: &Target) -> (String, Vec<String>);      // macOS: ("open", ["-n", app]); Windows: (exe, [])
```
Thư mục tạm: `std::env::temp_dir().join("luuluong-update-<pid>")`; đổi tên trên cùng ổ đĩa — nếu `rename` báo khác ổ đĩa thì sao chép rồi đổi (nằm trong `swap`, test bằng cách đặt payload cùng thư mục cha).

- [ ] **Step 1: Test thất bại (thư mục tạm thật, không đụng hệ thống):** `detect_target_for_mac_bundle_path`; `detect_target_for_windows_exe_path`; `detect_target_rejects_dev_binary` (`…/target/debug/app` → `NotInstalled`); `swap_mac_replaces_bundle_and_keeps_old_as_dot_old`; `swap_mac_rolls_back_when_second_rename_fails` (payload không tồn tại → bản cũ còn nguyên, không có `.old` treo); `swap_replaces_a_stale_dot_old`; `swap_fails_cleanly_when_parent_is_read_only` (`#[cfg(unix)]`, chmod 0555 thư mục cha → `NotWritable`, bản cũ nguyên vẹn); `swap_windows_style_exe`; `cleanup_removes_old_and_temp_dirs_and_tolerates_absence`; `relaunch_command_for_both_targets`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy. **Step 5:** commit `Cập nhật M4: thay thế bằng đổi tên, quay lui, dọn (10 test)`.

---

### Task 5: Cấu hình `[update]` và lịch kiểm tra

**Files:** Modify `crates/storage/src/config.rs`, `crates/app/src/settings.rs`

**Interfaces — Produces:**
```rust
// storage::config
pub struct Update { pub auto_check: bool, pub last_check: i64 }     // mặc định: true, 0
// app::update_state (file mới crates/app/src/update_state.rs, thuần logic)
pub fn check_due(auto_check: bool, last_check: i64, now: i64) -> bool;   // true nếu auto_check và now − last_check ≥ 24·3600 (kể cả last_check = 0; now < last_check (đồng hồ lùi) → true)
```
`SettingsForm` thêm `update_auto_check: bool`; `to_config` giữ nguyên `last_check`.

- [ ] **Step 1: Test thất bại:** `update_defaults_and_old_files_without_section` (config); `check_due_after_24h_and_when_never_checked`; `check_not_due_within_24h`; `check_not_due_when_auto_check_off`; `check_due_when_clock_went_backwards`; `settings_roundtrip_keeps_last_check` (settings.rs).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS + clippy. **Step 5:** commit `Cập nhật M5: cấu hình [update], lịch kiểm tra (6 test)`.

---

### Task 6: Tích hợp vào app (trạng thái, luồng nền, giao diện)

**Files:** Create `crates/app/src/update_state.rs` (máy trạng thái thuần) và `crates/app/src/update_screen.rs` (thẻ Cài đặt, chip, hộp xác nhận); Modify `crates/app/Cargo.toml` (`luuluong-update`), `crates/app/src/lib.rs`, `main.rs` (state, nhận kết quả từ luồng nền, chip trên thanh đầu, dọn rác khi khởi động), `settings_screen.rs` (gắn thẻ), `MAP.md`

**Interfaces — Produces:**
```rust
// update_state.rs
pub enum UpdateState { Idle, Checking, UpToDate, Available(Release), Downloading { done: u64, total: u64 }, Installing, Failed(String) }
pub enum UpdateEvent { CheckStarted, Checked(Result<Release, UpdateError>), InstallClicked, Progress(u64, u64), Installed, Error(UpdateError) }
pub fn next(state: UpdateState, event: UpdateEvent, current: Version) -> UpdateState;
pub fn can_install(state: &UpdateState) -> bool;     // chỉ khi Available
```
`Checked(Ok(r))` mà `!is_newer(r.version, current)` → `UpToDate`; `Checked(Err(e))` → `Failed(e.to_string())`; `InstallClicked` chỉ có tác dụng ở `Available` (còn lại giữ nguyên → chống bấm đúp); `Progress` chỉ ở `Downloading`; `Installed` → `Installing`.

Giao diện: thẻ **"Cập nhật"** (trong Cài đặt, đặt trên thẻ "Giao diện"): dòng "Phiên bản đang chạy 0.2.0", công tắc "Tự kiểm tra khi mở app", nút "Kiểm tra cập nhật"; khi `Available`: "Có bản 0.3.0", ghi chú (`notes_excerpt`), nút **"Tải và cài đặt"** (mở hộp xác nhận theo thiết kế mục 4.3) và "Mở trang tải"; khi `Downloading`: thanh tiến trình + phần trăm; `Failed`: câu lỗi tiếng Việt kèm "Mở trang tải"; `NotInstalled`/`NotWritable` kèm gợi ý riêng ("App đang chạy từ môi trường phát triển." / "Không ghi được vào thư mục cài đặt, hãy tải bản mới từ trang phát hành."). Chip trên thanh đầu khi `Available`: "Có bản mới 0.3.0" (nền `p.primary`, chữ `p.on_primary` hoặc cặp đạt ≥ 4,5:1; kiểm bằng công cụ như các chip trước), bấm → tab Cài đặt. Khi khởi động: `luuluong_update::cleanup`; nếu `check_due` → kiểm tra nền và ghi `last_check` khi thành công. Cài đặt thành công → chạy `relaunch_command` rồi thoát (`ViewportCommand::Close` sau khi đã `spawn`).

- [ ] **Step 1: Test thất bại (`update_state.rs`):** `checked_newer_becomes_available`; `checked_same_or_older_is_up_to_date`; `checked_error_becomes_failed_with_vietnamese_text`; `install_click_only_works_from_available` (từ `Checking`, `Downloading`, `Installing`, `UpToDate`: không đổi); `progress_only_in_downloading`; `installed_moves_to_installing`; `can_install_only_when_available`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. **Step 5:** viết `update_screen.rs` + nối `main.rs`/`settings_screen.rs`; `cargo build -p app`, `cargo test --workspace`, clippy sạch. **Step 6: Kiểm tay (người dùng):** thẻ Cập nhật hiện, nút "Kiểm tra cập nhật" báo "Đang dùng bản mới nhất" (hoặc có bản mới), chip đúng màu và bấm đi tới Cài đặt. **Step 7:** commit `Cập nhật M6: trạng thái, luồng nền, thẻ Cập nhật, chip (7 test)`.

---

### Task 7: Đóng gói và phát hành

**Files:** Modify `scripts/package_macos.sh`, `.github/workflows/release.yml`; Create `livrables/QUY-TRINH-PHAT-HANH.md`

- [ ] **Step 1:** `package_macos.sh` sau khi ký tạm và trước khi `rm -rf stage`: `ditto -c -k --keepParent "$APP" "$OUT/luuluong-$VERSION-macos-universal.app.zip"`; in dung lượng. Kiểm: chạy script cục bộ (người dùng) tạo cả `.dmg` và `.app.zip`; `unzip -l` thấy `Contents/MacOS/luuluong` và `Contents/_CodeSignature`; giải nén ra rồi `codesign --verify --deep --strict` vẫn hợp lệ.
- [ ] **Step 2:** `release.yml` (job macOS): `upload-artifact` và `action-gh-release` lấy thêm `livrables/*.app.zip`. Kiểm: đọc lại YAML hợp lệ (`python3 -c "import yaml…"` hoặc `actionlint` nếu có); KHÔNG chạy workflow, KHÔNG gắn thẻ.
- [ ] **Step 3:** `QUY-TRINH-PHAT-HANH.md`: các bước tăng `version` trong `crates/app/Cargo.toml`, `cargo test`, commit, gắn thẻ `v<phiên bản>` (người dùng làm), chờ CI, kiểm hai tài sản; tên tài sản bắt buộc đúng mẫu; lưu ý bản đầu có tính năng này phải cài tay.
- [ ] **Step 4:** commit `Cập nhật M7: tài sản .app.zip, quy trình phát hành`.

---

### Task 8: Nghiệm thu

- [ ] `cargo test --workspace` toàn pass (kể cả T1–T6), `cargo clippy --workspace -- -D warnings` sạch; đo kích thước file chạy release (mục tiêu ≤ 15 MB); `PLAN.md` Giai đoạn 12 + ngoại lệ 7.1 (`sha2`, `zip`); `MAP.md`; một đoạn trong `contexte/` về luồng cập nhật.
- [ ] **Kiểm toàn trình (người dùng, cần phát hành thử):** (1) cài bản N (có tính năng này) từ `.dmg`/`.zip`; (2) tăng phiên bản lên N+1, gắn thẻ, chờ CI, bản phát hành xuất hiện; (3) mở app bản N: chip "Có bản mới", bấm "Tải và cài đặt", app mở lại ở bản N+1, lịch sử và cài đặt còn nguyên, `.old` bị dọn ở lần mở kế tiếp; (4) thử trên cả macOS và Windows; (5) thử cắt mạng giữa lúc tải; (6) thử bản cài ở thư mục không ghi được. Ghi kết quả vào `PLAN.md`.
