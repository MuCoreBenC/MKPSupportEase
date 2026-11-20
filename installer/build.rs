use std::path::{Path, PathBuf};

fn main() {
    // payload 由 `scripts/build-windows-installer.mjs` 生成（客户端 exe + LICENSE）。
    // 仓库里没有它也能编：这里往 OUT_DIR 放一个 22 字节的空 zip（EOCD），
    // main.rs 用 include_bytes!(OUT_DIR/payload.zip) —— 于是「源码目录里缺一个
    // 二进制文件」不会让 `cargo check` 红掉。
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR 没设"));
    let target = out_dir.join("payload.zip");
    let source = Path::new("payload.zip");
    if source.exists() {
        std::fs::copy(source, &target).expect("复制 payload.zip 失败");
    } else {
        std::fs::write(
            &target,
            b"PK\x05\x06\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
        )
        .expect("写占位 payload 失败");
    }
    println!("cargo:rerun-if-changed=payload.zip");
    println!("cargo:rerun-if-env-changed=MKP_APP_VERSION");

    tauri_build::build()
}
