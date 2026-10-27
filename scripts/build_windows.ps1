# Build bản Windows (chạy trong PowerShell trên máy Windows 10/11 có Rust). Đặc tả mục 9.1.
# Kết quả: livrables/luuluong-<phiên bản>-windows-x64.zip chứa luuluong.exe
$ErrorActionPreference = "Stop"

rustup target add x86_64-pc-windows-msvc
# Liên kết tĩnh MSVC runtime để máy đích không cần cài Visual C++ Redistributable.
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --locked -p app --target x86_64-pc-windows-msvc

$version = (Select-String -Path crates/app/Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
$stage = "livrables/stage-windows"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target/x86_64-pc-windows-msvc/release/app.exe "$stage/luuluong.exe"
$zip = "livrables/luuluong-$version-windows-x64.zip"
Compress-Archive -Force -Path "$stage/luuluong.exe" -DestinationPath $zip
Remove-Item -Recurse -Force $stage
"Đã tạo $zip ($([math]::Round((Get-Item $zip).Length / 1MB, 1)) MB)"
