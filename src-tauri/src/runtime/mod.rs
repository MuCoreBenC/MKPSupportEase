//! **新数据世界**（见 `docs/DATA-ARCHITECTURE.md`）。
//!
//! 总纲把所有文件划进四层：开发仓库 → 安装包 → 用户本地 → 云端。这一模块管的是
//! **第③层（用户本地）的运行时形态**与**第②层 → 第③层的释放口**：
//!
//! ```text
//! 安装包（二进制里嵌着的 catalog.generated.json）
//!     │  首启释放（release）
//!     ▼
//! <appDataDir>/catalog.json     说明书 —— 首屏唯一数据源（铁律 2：启动零网络）
//! <appDataDir>/<catalog.path>   交付文件 —— 落点由目录自己说（铁律 3：没下载就没有）
//!                               A 类是 `assets/…`、B 类是 `delivery/mkp/presets/…`；
//!                               **没有固定的"下载区根"**，目录在落盘那一刻按需建
//! ```
//!
//! 客户端首屏（`ipc::presets` 的九条读命令）从 catalog 的 definition 出数；
//! 旧世界（include_str! 13 份源 TOML 铺进 `presets/` 的 `client/` 模块）已随换源退役
//! —— 开发文件不再成为运行时数据库（铁律 1 收口，总纲欠账 #1）。
//!
//! 名字里没有版本号（作者裁决：新系统不背旧命名的包袱）——目录格式由
//! [`catalog::CATALOG_SCHEMA`] 表达，进化靠加字段，不靠改名。

/// **应用持久化状态（AppState）**——`run/app-state.json` 的唯一读写入口（2026-10-06
/// 架构决策，`docs/APP-STATE.md`）：一个应用一个状态文件，整份读-改-写 + 原子替换
pub mod app_state;

use postprocess::diag::messages;
/// **备份与恢复**（2026-10-17）：一个 zip = 这台机器的全部家当（设置 / catalog /
/// toml / 修改历史 / 基准 / 执行归档）；**恢复 = 全量覆盖**（理由见模块头）
pub mod backup;
/// **隐藏 baseline 存储**（2026-10-08 作者改判）：每一版下载过的交付预设在这里留一份
/// 不可见的基准快照，按 `sha256` 寻址。用户不可见、不进对比台，只服务"恢复默认"与
/// 血统里的 `based_on_sha256`。见 [`baseline`] 的模块头。
pub mod baseline;
pub mod catalog;
pub mod delivery;
/// **修改历史账本**（2026-10-17）：一份文件一本，git 式 —— 不清空、重开软件还在
/// （用户线按相对路径、云端线按文件名认文件，于是导入的 toml 也有自己的一本）。
/// 见 [`history`] 的模块头。
pub mod history;
/// **通用文件导入入口**（第十二层）：外部文件怎么安全地进入应用 —— 拖拽 / 文件选择器
/// 共用这一层，Preset 只是第一个消费者。与云端线 / 用户线都分开：
/// 它只管"进来"，不碰"使用"（不校验内容、不碰状态）。
pub mod import;
/// **血统三行**（用户那份是从哪份云端、哪一版拷出来改的）—— 写在文件头注释里
pub mod lineage;
/// **用户线**：用户自己的预设（`presets-local/`）—— 与云端线（`<catalog.path>` /
/// `archive/`）分开
pub mod local;
pub mod net;
/// **逐参数「云端更新」的比对**（2026-10-09）：我这份 / 云端旧值 / 云端新值三方账。
/// 纯算法（不读盘），落点是 `ipc::param_sync` 的两条命令。
pub mod param_sync;
pub mod paths;
/// 用户偏好 → IR（2026-10-17）：设置页那几档开关真正生效的唯一写入点。
pub mod postproc_prefs;
/// **预设事件时间模型**（下载 / 替换落账，发布反查目录与版本链）：凡是界面上要
/// "永久解释"的时间，先定义成事件、发生那一刻定格 —— mtime 从此只归文件系统
pub mod preset_events;
/// **出处账**（复制 / 导入的来源）：用户文件"从哪复制来的"这件事盘上不存在，记在这一本
pub mod provenance;
pub mod release;
/// **软件发布信息**（`release.json`）：「有没有新版本的 SupportEase」这条链的信息源。
/// 与预设数据（catalog）**两条链**——不进 catalog / manifest，不参与发布闸（作者定死）。
pub mod release_info;
/// **备注覆盖账**（客户端副标题）：用户改过的备注记在这一本 —— 「更新不覆盖、
/// 删了重下才回到工作台那句」靠它兑现
pub mod remarks;
pub mod resolver;
pub mod source;
pub mod state;
/// **结构代次**：这批数据是什么结构、哪个客户端起读得懂（`docs/PUBLISH-ARCHITECTURE.md` §5.3）。
/// 签名是机器真值（从类型探），「签名 → 最低客户端版本」是人必须显式登记的那半。
pub mod structure;
pub mod update;
/// **应用内更新**（2026-10-05 第五刀）：下载 / 进度 / 暂停取消 / 解压 / 替换重启。
/// 与 `release_info` 分开：那份是"信息"，这份是"动作"
pub mod updater;
/// **「哪份预设用得最多」本机学习账**（2026-10-17）：`run/usage.json`，
/// **不进备份**（换设备就从零学）。见 [`usage`] 的模块头。
pub mod usage;
/// **云端版本清单**（2026-10-08）：这台机器"见过"的交付预设版本 = 历次目录里登记过的
/// 那几版（当前目录 + 版本链），按内容摘要去重。见 [`versions`] 的模块头。
pub mod versions;

pub use catalog::Catalog;

/// 读**释放进内部根的那份** catalog 的原始字节。盘上没有（setup 释放失败、或文件被删）
/// 就就地补一次再读：那是兜底，不是正常路径。
///
/// 兜底走 [`release::ensure_released`] —— **只补空位，绝不覆盖**。它以前调
/// `release_catalog`（= `release_bytes(EMBEDDED)`，升级语义），于是"随手读一下"
/// 也可能把 OTA 成果换成随包那份（2026-10-04 与启动回退一并修掉）。
///
/// 给"按字节缓存"的消费者用（`ipc::presets` 的九条读命令）：同一份字节只 parse 一次，
/// 目录换新（升级 / 应用远端更新）后字节变，缓存自动失效——**不需要失效钩子**。
pub fn load_released_catalog_bytes(
    root: &std::path::Path,
) -> Result<Vec<u8>, crate::error::AppError> {
    let path = paths::catalog_file(root);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(bytes),
        Err(_) => {
            release::ensure_released(root)?;
            std::fs::read(&path).map_err(|e| {
                crate::error::AppError::io(messages::text("ui.runtime.mod.416", &[]))
                    .with_detail(e.to_string())
            })
        }
    }
}

/// 读**释放进内部根的那份** catalog（铁律 4：运行时只认自己的运行时数据）。
/// 命令层（`ipc::catalog`）与将来的写路径共用这一条入口，读法只有这一份。
pub fn load_released_catalog(root: &std::path::Path) -> Result<Catalog, crate::error::AppError> {
    Catalog::parse(&load_released_catalog_bytes(root)?)
}

/// **界面要用的那份目录**：② 为准，**随包那份只兜底**（2026-10-16，作者定）。
///
/// 兜的只有一样：② 里**没有、而随包那份有**的 `bundled` 资产条目（机型整机图、
/// 引导图那些 —— 字节随程序本体走、客户端不下载）。理由与边界写在
/// [`Catalog::with_missing_bundled_from`]。
///
/// # 为什么不在读 ② 那一层直接改
///
/// [`load_released_catalog`] 是"**盘上那一份**"的字面读法，别处（可信度判定、
/// 换代归档、检查更新）要的就是它原样 —— 只在这里多走一步，是因为**只有界面**
/// 需要"随包的东西不依赖云端那份目录"。
///
/// ★ 这与「启动绝不覆盖 ②」是同一条纪律的两面：**盘上的字节一个都不动**，
///   这一步只在内存里补几条给界面看。
pub fn catalog_for_ui(root: &std::path::Path) -> Result<Catalog, crate::error::AppError> {
    let released = load_released_catalog(root)?;
    let embedded = Catalog::parse(EMBEDDED_CATALOG)?;
    Ok(released.with_missing_bundled_from(&embedded))
}

/// 随安装包走的那份 catalog —— 发布构建（`cargo run --bin gen-catalog`）的产物。
///
/// 编译期钉进二进制，运行时与仓库无关（用户机器上没有仓库）。它是不是最新的，
/// 由 [`catalog::tests::embedded_matches_rebuild`] 那条判据守着：
/// 重新构建的结果与这份**逐字节一致**，否则测试红 ——
/// 「没有未经审阅的变化」在构造上成立，与对照基线同一思路。
pub const EMBEDDED_CATALOG: &[u8] = include_bytes!("catalog.generated.json");

#[cfg(test)]
mod tests {
    use super::*;

    /// 嵌进二进制的那份必须是当前源 + 交付产物能重新构建出来的那一份。
    /// 改了源忘了跑 `gen-catalog`，这条就红 —— 它是层① → 层② 的失联报警。
    #[test]
    fn embedded_matches_rebuild() {
        let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest.parent().expect("src-tauri 上面就是仓库根");
        let catalog = catalog::Catalog::build_from_repo(repo).expect("目录构建不出来");
        let json = catalog.to_pretty_json().expect("序列化失败");
        assert_eq!(
            json.as_bytes(),
            EMBEDDED_CATALOG,
            "仓库里的 catalog.generated.json 与重新构建的不一致 —— 跑一遍 `cargo run --bin gen-catalog`"
        );
    }

    /// 嵌入的那份本身得能读：schema 认得、机型与文件非空。
    /// （它坏了对用户的表现是第一屏空 —— 那种错在真机上最费劲，在这里先拦住。）
    #[test]
    fn embedded_parses_and_is_not_empty() {
        let catalog = Catalog::parse(EMBEDDED_CATALOG).expect("嵌进二进制的 catalog 读不出来");
        assert_eq!(catalog.catalog_schema, catalog::CATALOG_SCHEMA);
        assert!(!catalog.machines.is_empty(), "机型清单不该是空的");
        assert!(!catalog.files.is_empty(), "文件清单不该是空的");
        assert_eq!(
            catalog.files[0].kind, "mkp_preset",
            "第一圈的清单只有 MKP 预设一种"
        );
    }

    /// **随包那份只兜底**（2026-10-16，作者「编译进去的，就只当兜底」）——
    /// 界面读的那份 = ② + ② 里缺的 `bundled` 条目。
    ///
    /// 摆一次事故现场：盘上那份目录是"加引导图之前"的（`assets` 空），
    /// 界面仍要能从**随包那份**里找到那两张引导图（作者：「诶图片不见了」就是这儿丢的）；
    /// 而要按需下载的条目一件都不许兜上来（补了就是个下不到的条目）。
    #[test]
    fn ui_catalog_backfills_bundled_assets_that_the_released_one_lacks() {
        use crate::fsx::atomic::atomic_write;
        use crate::presetdata::assets::Delivery;
        use crate::presetdata::AssetKind;

        let d = tempfile::tempdir().unwrap();
        let released = r#"{ "catalogSchema": 1, "revision": "加引导图之前那一版", "machines": [], "files": [], "assets": [] }"#;
        atomic_write(&paths::catalog_file(d.path()), released.as_bytes()).unwrap();

        let ui = catalog_for_ui(d.path()).expect("盘上那份读得出来");

        let guides: Vec<&str> = ui
            .assets
            .iter()
            .filter(|a| a.kind == AssetKind::Guide)
            .map(|a| a.id.as_str())
            .collect();
        assert!(
            guides.contains(&"guide-open-with-light") && guides.contains(&"guide-open-with-dark"),
            "两张引导图都得从随包那份兜上来，读到的是 {guides:?}"
        );
        assert!(
            ui.assets.iter().all(|a| a.delivery == Delivery::Bundled),
            "只有随包条目能被兜上来 —— 下载面的一条都不许进"
        );
        assert!(
            ui.files.is_empty(),
            "② 说没有交付文件，这一步不许给它编出文件来"
        );
    }
}
