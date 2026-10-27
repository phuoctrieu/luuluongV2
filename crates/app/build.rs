//! Gắn biểu tượng và thông tin phiên bản vào file `.exe` khi build cho Windows; các nền tảng khác không làm gì.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Luu luong Song Bo");
        res.set("FileDescription", "Luu luong nuoc ve ho Song Bo");
        if let Err(e) = res.compile() {
            panic!("không gắn được biểu tượng vào .exe: {e}");
        }
    }
}
