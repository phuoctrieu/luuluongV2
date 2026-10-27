# Build bản Windows 10/11 (chạy trong PowerShell, từ bất kỳ thư mục nào):  .\scripts\build_windows.ps1
# Tự kiểm tra và cài các thành phần còn thiếu: Visual Studio Build Tools (C++ + Windows SDK), Rust, target MSVC.
# Cài Build Tools cần quyền quản trị (hộp thoại UAC hiện ra). Kết quả: livrables\luuluong-<phiên bản>-windows-x64.zip
$ErrorActionPreference = "Stop"
Set-Location (Join-Path $PSScriptRoot "..")

# Lệnh ngoài (winget, rustup, cargo) không tự dừng khi lỗi: kiểm tra mã thoát sau mỗi lệnh.
function Check([string]$what) {
    if ($LASTEXITCODE -ne 0) { throw "$what thất bại (mã $LASTEXITCODE)" }
}

# Sau khi cài, PATH của phiên này chưa có chương trình mới: nạp lại từ hệ thống.
function Update-SessionPath {
    $machine = [Environment]::GetEnvironmentVariable("Path", "Machine")
    $user = [Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = "$machine;$user;$env:USERPROFILE\.cargo\bin"
}

Write-Host "== Kiểm tra và cài các thành phần còn thiếu =="

# 1. Visual Studio Build Tools: trình liên kết MSVC và Windows SDK (cần cho Rust trên Windows).
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasMsvc = $false
if (Test-Path $vswhere) {
    $found = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    $hasMsvc = [bool]$found
}
if (-not $hasMsvc) {
    Write-Host "Chưa có Visual Studio Build Tools: cài (vài phút, cần quyền quản trị)..."
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements `
        --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    Check "Cài Visual Studio Build Tools"
}

# 2. Rust (qua rustup) và target MSVC.
Update-SessionPath
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    Write-Host "Chưa có Rust: cài bằng rustup..."
    winget install --id Rustlang.Rustup -e --accept-package-agreements --accept-source-agreements
    Check "Cài Rust"
    Update-SessionPath
}
if (-not (Get-Command rustup -ErrorAction SilentlyContinue)) {
    throw "Không tìm thấy rustup sau khi cài. Mở lại PowerShell rồi chạy lại script."
}
rustup default stable
Check "rustup default stable"
rustup target add x86_64-pc-windows-msvc
Check "rustup target add"

Write-Host "== Build =="
# Liên kết tĩnh MSVC runtime để máy đích không cần cài Visual C++ Redistributable.
$env:RUSTFLAGS = "-C target-feature=+crt-static"
cargo build --release --locked -p app --target x86_64-pc-windows-msvc
Check "cargo build"

Write-Host "== Đóng gói =="
$version = (Select-String -Path crates/app/Cargo.toml -Pattern '^version = "(.*)"' | Select-Object -First 1).Matches.Groups[1].Value
$stage = "livrables/stage-windows"
Remove-Item -Recurse -Force $stage -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage | Out-Null
Copy-Item target/x86_64-pc-windows-msvc/release/app.exe "$stage/luuluong.exe"
$zip = "livrables/luuluong-$version-windows-x64.zip"
Compress-Archive -Force -Path "$stage/luuluong.exe" -DestinationPath $zip
Remove-Item -Recurse -Force $stage
"Đã tạo $zip ($([math]::Round((Get-Item $zip).Length / 1MB, 1)) MB)"
