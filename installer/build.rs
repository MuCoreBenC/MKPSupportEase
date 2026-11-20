use std::path::{Path, PathBuf};

fn main() {
    // payload 由 `scripts/build-windows-installer.mjs` 生成：
    //   客户端 exe + LICENSE → payload.zip（deflate）→ payload.xz（LZMA）。
    // 仓库里没有它也能编：这里往 OUT_DIR 放一个空文件，main.rs 运行时会报
    // 「安装包不完整」—— 于是「源码目录里缺一个二进制」不会让 `cargo check` 红掉。
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR 没设"));
    let target = out_dir.join("payload.xz");
    let source = Path::new("payload.xz");
    if source.exists() {
        std::fs::copy(source, &target).expect("复制 payload.xz 失败");
    } else {
        std::fs::write(&target, b"").expect("写占位 payload 失败");
    }
    println!("cargo:rerun-if-changed=payload.xz");
    println!("cargo:rerun-if-env-changed=MKP_APP_VERSION");

    // exe 图标（资源管理器 / 任务栏）。embed-resource 调 rc.exe，随 MSVC 工具链来
    let _ = embed_resource::compile("app.rc", embed_resource::NONE);
    println!("cargo:rerun-if-changed=app.rc");
    println!("cargo:rerun-if-changed=icons/icon.ico");
}
