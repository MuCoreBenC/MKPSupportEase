//! 集成判据共用的**显式**运行时初始化。
//!
//! 库没有备用数据源：`machine_dims::tables()` 没装就 panic，不会去仓库里找 `presets/`。
//! 所以每个要读预设 / 跑管线的集成判据都得自己把机型表装进来 —— 与真实入口
//! （发布物装随包 catalog）是同一个动作，只是数据树换成仓库这份。
//!
//! 进程内只装一次：`cargo test` 每个集成文件是一个进程，`OnceLock` 的语义正好。
#![allow(dead_code)]

use std::sync::Once;

/// 从仓库的 `presets/` 装一份机型表进本进程。
pub fn install_tables() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets");
        let tables = postprocess::postproc::machine_dims::load_presets_dir(&dir)
            .unwrap_or_else(|e| panic!("判据读不出来 {} 的机型表：{e}", dir.display()));
        postprocess::postproc::machine_dims::install(tables)
            .expect("集成判据进程里机型表只该装一次");
    });
}
