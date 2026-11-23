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
//! <appDataDir>/content/<sha>.toml  云端原文内容仓库（2026-10-29）—— 预设没有 path 副本，
//!                                 同 SHA 多 path 共用一份；"手上有哪一版"由索引回答
//! <appDataDir>/run/delivery-index.json  path → 手上这一版的 sha + 目录身份四格
//! <appDataDir>/<catalog.path>   非预设交付件（A 类资产）—— 落点由目录自己说
//!                               （铁律 3：没下载就没有），**没有固定的"下载区根"**
//! <appDataDir>/recovery/<id>/   最近删除：文件 + 原文镜像 + 来源账
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
/// **原文镜像**（2026-10-08 作者改判，2026-10-29 改口径）：`baseline-local/` 存的是
/// **每份用户文件自己最初的样子**（按用户根相对路径镜像），服务"恢复默认"。云端原文
/// 在 [`content`] 里按 SHA 存，两件事别混。见 [`baseline`] 的模块头。
pub mod baseline;
/// **分支隔离**：每个分支一个文件夹（`presets-local/<folder>/`），同名文件各归各支。
/// 见 [`branch_dir`] 的模块头。
pub mod branch_dir;
pub mod catalog;
/// **客户端壳的偏好初值**（主题 / 语言 / 切片器 / 引导 / 预设那两步）：读
/// `presets/client-defaults.toml` 的 `[Client]` 段 —— 与 `[Prefs]` 同一份文件、同一份纪律
/// （出厂值只有一个地方可改）。盘上那些格仍是 `Option`，`None` 才落到这里。
pub mod client_defaults;
pub mod content;
pub mod delivery;
pub mod delivery_index;
/// **修改历史账本**（2026-10-17）：一份文件一本，git 式 —— 不清空、重开软件还在
/// （用户线按相对路径、云端线按文件名认文件，于是导入的 toml 也有自己的一本）。
/// 见 [`history`] 的模块头。
pub mod history;
/// **通用文件导入入口**（第十二层）：外部文件怎么安全地进入应用 —— 拖拽 / 文件选择器
/// 共用这一层，Preset 只是第一个消费者。与云端线 / 用户线都分开：
/// 它只管"进来"，不碰"使用"（不校验内容、不碰状态）。
pub mod import;
/// **用户线**：用户自己的预设（`presets-local/`）—— 与云端线（[`content`] /
/// [`delivery_index`] / `archive/`）分开
pub mod local;
/// **一次性存储迁移**（2026-10-29）：旧 delivery/baseline → content + 索引
pub mod migrate;
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
/// **来源记录**（用户那份是从哪份云端、哪一版拷出来改的）—— 写在文件头注释里
pub mod provenance;
pub mod recovery;
pub mod release;
/// **软件发布信息**（`release.json`）：「有没有新版本的 SupportEase」这条链的信息源。
/// 与预设数据（catalog）**两条链**——不进 catalog / manifest，不参与发布闸（作者定死）。
pub mod release_info;
/// **备注覆盖账**（客户端副标题）：用户改过的备注记在这一本 —— 「更新不覆盖、
/// 删了重下才回到工作台那句」靠它兑现
pub mod remarks;
pub mod resolver;
pub mod source;
/// **源身份**（2026-10-27）：把「云端在哪」规范化成 `平台#仓库@ref`，用来回答
/// 「本地这份与云端那份是不是同一个源」—— 换源之后 `BasedOn` 靠它区分
/// 「云端变了」与「两边本来就不是一回事」。见 [`source_identity`] 的模块头。
pub mod source_identity;
/// **源身份账**（2026-10-27）：`run/source-ledger.json`（见过哪些源、各自发布时刻）与
/// `run/provenance.json`（哪份副本是在哪个源下建的）。**不动用户的 preset 文件**，
/// 坏档按空账。见 [`source_ledger`] 的模块头。
pub mod source_ledger;
pub mod state;
/// **结构代次**：这批数据是什么结构、哪个客户端起读得懂（`docs/PUBLISH-ARCHITECTURE.md` §5.3）。
/// 签名是机器真值（从类型探），「签名 → 最低客户端版本」是人必须显式登记的那半。
pub mod structure;
pub mod update;
/// **自动检查软件更新**（2026-11-05）：只在「成功拉到云端目录」之后顺路查一次
/// `release.json`，有新版就推一个事件给右上角通知。本会话去重、失败静默、
/// 不进启动/首屏路径（铁律 2）。版本比较不重写，复用 [`release_info`]。
pub mod update_check;
/// **「联网功能」那道闸**：关着时这个程序一个字节都不出网（预设 / 模型下载、
/// 检查更新、匿名统计全停）。判据只有一条 —— [`app_state::online`]。
pub mod online;
pub mod update_state;
/// **应用内更新**（2026-10-05 第五刀）：下载 / 进度 / 暂停取消 / 解压 / 替换重启。
/// 与 `release_info` 分开：那份是"信息"，这份是"动作"
pub mod updater;
/// **「哪份预设用得最多」本机学习账**（2026-10-17）：`run/usage.json`，
/// **不进备份**（换设备就从零学）。见 [`usage`] 的模块头。
pub mod usage;
/// **匿名使用统计**（`run/usage-ping.json`）：一天最多报一次「今天这台机器用过」，
/// 出口仍是 [`net`]，挂在「成功拉到云端目录」之后，失败静默。见 [`usage_ping`] 的模块头。
pub mod usage_ping;
pub mod utxn;
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

/// 校准档表：源在 `presets/`，编译期钉进二进制。发布物磁盘上没有 presets/，
/// 而合体（Combined）校准件靠外形定档必须要这张表，缺了就会把校准件判成普通模型。
pub const EMBEDDED_CALIBRATION_MODELS: &str =
    include_str!("../../../presets/calibration_models.toml");

/// 把随包 catalog 的机型数据装进 postprocess 的进程级表。
///
/// 这是**唯一一条**装表路径：发布物与界面都由 [`crate::boot::Runtime::initialize`] 走到这里。
/// 随包数据读不出来就返回 `Err` —— 调用方停下，库那边也不会去别处找第二份数据。
///
/// 只装一次；重复调用（`OnceLock` 已装）返回 `Err`：那是入口接线的错误，不能吞。
pub fn install_machine_tables_from_catalog() -> Result<(), String> {
    let tables = machine_tables_from_catalog()?;
    postprocess::postproc::machine_dims::install(tables)
}

/// 从随包 catalog 构出机型表。纯函数，便于判据直接验数据。
fn machine_tables_from_catalog(
) -> Result<postprocess::postproc::machine_dims::MachineTables, String> {
    use postprocess::postproc::machine_dims::{
        MachineDimensions, MachineTables, ZonePoint, ZonePolygon,
    };

    let catalog =
        Catalog::parse(EMBEDDED_CATALOG).map_err(|e| format!("随包 catalog 读不出来：{e}"))?;

    let mut dimensions = std::collections::HashMap::new();
    let mut alias_map = std::collections::HashMap::new();
    let mut forbidden_zones = std::collections::HashMap::new();

    for machine in &catalog.machines {
        alias_map.insert(machine.id.to_uppercase(), machine.id.clone());
        for alias in &machine.external_aliases {
            alias_map.insert(alias.to_uppercase(), machine.id.clone());
        }
        if let Some(d) = &machine.dimensions {
            dimensions.insert(
                machine.id.clone(),
                MachineDimensions {
                    bed_size: postprocess::postproc::machine_dims::BedSize {
                        width: d.bed_size.width,
                        depth: d.bed_size.depth,
                    },
                    movement_range: postprocess::postproc::machine_dims::MovementRange {
                        min_x: d.movement_range.min_x,
                        max_x: d.movement_range.max_x,
                        min_y: d.movement_range.min_y,
                        max_y: d.movement_range.max_y,
                        max_z: d.movement_range.max_z,
                    },
                    edge_zone: d.edge_zone,
                    glue_area: postprocess::postproc::machine_dims::GlueArea {
                        glue_min_x: d.glue_area.glue_min_x,
                        glue_max_x: d.glue_area.glue_max_x,
                        glue_min_y: d.glue_area.glue_min_y,
                        glue_max_y: d.glue_area.glue_max_y,
                        wipe_x: d.glue_area.wipe_x,
                    },
                    calibration: postprocess::postproc::machine_dims::Calibration {
                        l_shape_base_x: d.calibration.l_shape_base_x,
                        l_shape_base_y: d.calibration.l_shape_base_y,
                        x_line_x: d.calibration.x_line_x,
                        x_line_y: d.calibration.x_line_y,
                        x_line_y_end: d.calibration.x_line_y_end,
                        y_line_x: d.calibration.y_line_x,
                        y_line_y: d.calibration.y_line_y,
                        y_line_x_end: d.calibration.y_line_x_end,
                        z_start_x: d.calibration.z_start_x,
                        z_start_y: d.calibration.z_start_y,
                    },
                    flags: postprocess::postproc::machine_dims::Flags {
                        gcode_marker: d.flags.gcode_marker.clone(),
                        has_second_fan: d.flags.has_second_fan,
                    },
                },
            );
        }
        if !machine.zones.is_empty() {
            forbidden_zones.insert(
                machine.id.clone(),
                machine
                    .zones
                    .iter()
                    .map(|z| ZonePolygon {
                        points: z
                            .points
                            .iter()
                            .map(|(x, y)| ZonePoint { x: *x, y: *y })
                            .collect(),
                    })
                    .collect(),
            );
        }
    }

    /* 档表读不出来就是硬错：装一张没有档的机型表会让合体校准件在用户机器上
     * 被判成"不是校准件"，然后静默按普通模型跑完。 */
    let calibration_models =
        postprocess::postproc::calib_models::parse(EMBEDDED_CALIBRATION_MODELS, "随包校准档表")?;

    Ok(MachineTables::from_catalog_parts(
        dimensions,
        alias_map,
        forbidden_zones,
        Some(calibration_models),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 发布物靠这张表后处理：尺寸 / 别名 / 禁区少一样都会在真机上静默算错几何。
    /// 这条把「装表」这件事钉住，防的是有人删掉 main 里的 install 接线后
    /// 又以 `Error code: 101` 的形式在用户机器上重现。
    #[test]
    fn catalog_machine_tables_carry_dimensions_aliases_and_zones() {
        let tables = machine_tables_from_catalog().expect("随包 catalog 必须能构出机型表");
        assert!(!tables.dimensions().is_empty(), "尺寸表空了");
        assert_eq!(
            tables.alias_map().get("A1F").map(String::as_str),
            Some("A1")
        );
        assert!(
            tables.forbidden_zones().values().any(|z| !z.is_empty()),
            "至少有台机型带禁区"
        );
        assert!(
            tables
                .calibration_models()
                .is_some_and(|models| !models.is_empty()),
            "合体校准件靠档表按外形定档，档表空了就会把它当普通模型"
        );
    }

    /// 二合一（Combined）身份必须靠档表按外形定档。档表空了 ⇒ 认不出 ⇒ 报「不是校准件」，
    /// 静默把校准件当普通模型跑。这条咬住「随包档表能被认档函数命中」，是那次回归的直接判据。
    #[test]
    fn bundled_calibration_profiles_are_recognisable_by_size() {
        use postprocess::postproc::calib_models::find_profile;

        let tables = machine_tables_from_catalog().expect("随包 catalog 必须能构出机型表");
        let models = tables
            .calibration_models()
            .expect("档表必须随包装入，否则合体校准件会报「不是校准件」");
        let with_shape: Vec<_> = models
            .iter()
            .filter(|p| p.has_footprint() && !p.mode.is_empty())
            .collect();
        assert!(!with_shape.is_empty(), "档表里没有参与认档的档");
        for p in with_shape {
            let mode = &p.mode[0];
            let hit = find_profile(models, mode, p.footprint_w, p.footprint_h);
            assert!(
                hit.is_some(),
                "档 {}（{mode} {}×{}）按自己的标称尺寸竟认不出 —— 认档链或档表坏了",
                p.id,
                p.footprint_w,
                p.footprint_h
            );
        }
    }

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
