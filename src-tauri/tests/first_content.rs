//! 新设备上的**第一批内容**（b05 Task 15.3 / 15.4 的命令层验收）：
//! **导入第一张图并关联到机型 → 导入第一条 BBS 预设 → 建第一个套餐 → 机型引用它。**
//!
//! 独立测试文件 = 独立测试进程：`MKPSE_REPO_DIR` 指向临时目录（`paths.rs`
//! 的第一优先级），整台"设备"落在临时目录里 —— 不碰真仓库；本文件只有一个
//! 测试，环境变量设置安全。
//!
//! # 为什么要走命令层
//!
//! 领域体那几条判据（`import_asset` / `add_bundle`）证明的是"函数对"，
//! 证明不了"人点一次按钮能成"：命令层的差异在**资产根由 `paths::assets_root()`
//! 决定**、写完之后**从盘上重读**再返回。这两件事只有在真命令上才验得到。
//!
//! # 顺序不是随便排的
//!
//! 建套餐要有 BBS（doc §12.4 成套配发），BBS 要先导入进来 —— 所以 15.3
//! "建第一个套餐"这条链路上必须先走 15.4 的导入。这也是判据里把两条任务
//! 串在一条链上的理由：**分开写两条的话，谁也证明不了这条依赖成立**。

use mkp_support_ease_lib::workbench::app::assets::wb_import_asset;
use mkp_support_ease_lib::workbench::app::bundles::wb_add_bundle;
use mkp_support_ease_lib::workbench::app::init::wb_init_workbench;
use mkp_support_ease_lib::workbench::app::machines::{
    wb_add_machine, wb_add_version, wb_machines, wb_set_machine_field,
};
use mkp_support_ease_lib::workbench::app::wb_boot;
use mkp_support_ease_lib::workbench::presets::{AssetKind, MachineField};

#[test]
fn first_content_lands_through_import_bundle_and_reference() {
    let device = tempfile::tempdir().unwrap();
    std::env::set_var("MKPSE_REPO_DIR", device.path());
    // 源文件在"设备之外"（人从磁盘上挑的），导入 = 复制进来
    let picks = tempfile::tempdir().unwrap();
    let image_src = picks.path().join("a1.webp");
    let bbs_src = picks.path().join("MKPProcess A1 0.4 0.20.json");
    mkp_support_ease_lib::fsx::atomic::atomic_write(&image_src, b"image-bytes").unwrap();
    mkp_support_ease_lib::fsx::atomic::atomic_write(&bbs_src, b"{\"bbs\":true}").unwrap();

    // ① 空白设备：初始化 + 建第一台机型与第一个版本
    wb_init_workbench().unwrap();
    wb_add_machine("A1".to_owned(), "Bambu Lab".to_owned(), "A1".to_owned()).unwrap();
    wb_add_version("A1".to_owned(), "STANDARD".to_owned(), "标准版".to_owned()).unwrap();

    // ② 导入第一张图：文件进资产根 + 条目进 assets.toml
    let list = wb_import_asset(
        image_src.display().to_string(),
        "a1-image".to_owned(),
        AssetKind::Image,
        "A1 外观图".to_owned(),
        Some("A1".to_owned()),
        None,
        None,
    )
    .unwrap();
    let got = list
        .assets
        .iter()
        .find(|a| a.id == "a1-image")
        .expect("导入的条目在清单里");
    assert_eq!(got.path, "printers/a1.webp", "落点 = 类型子目录 + 源文件名");
    assert!(got.present, "文件真的在（导入不是只登记）");
    assert!(
        device
            .path()
            .join("public/assets/printers/a1.webp")
            .is_file(),
        "资产根下有那份文件"
    );
    assert_eq!(
        std::fs::read(device.path().join("public/assets/printers/a1.webp")).unwrap(),
        b"image-bytes".to_vec(),
        "复制过去的内容与源文件逐字节相同"
    );

    // ③ 关联到机型（`image` 那一格写的是资产 id）
    let list = wb_set_machine_field(
        "A1".to_owned(),
        MachineField::Image,
        Some("a1-image".to_owned()),
    )
    .unwrap();
    assert_eq!(
        list.machines[0].image.as_deref(),
        Some("a1-image"),
        "机型引用了刚导入的那张图"
    );

    // ④ 导入第一条 BBS 预设（建套餐的前提：doc §12.4 成套配发）
    let list = wb_import_asset(
        bbs_src.display().to_string(),
        "a1-bbs-04-020".to_owned(),
        AssetKind::SlicerProfile,
        "A1：0.4 喷头 0.20 层高".to_owned(),
        Some("A1".to_owned()),
        Some("bbs".to_owned()),
        Some("process".to_owned()),
    )
    .unwrap();
    let bbs = list
        .assets
        .iter()
        .find(|a| a.id == "a1-bbs-04-020")
        .expect("BBS 条目在");
    assert!(bbs.present);
    assert_eq!(bbs.path, "bbs/MKPProcess A1 0.4 0.20.json");

    // ⑤ 建第一个套餐
    let list = wb_add_bundle(
        "A1_default".to_owned(),
        "官方推荐".to_owned(),
        "A1".to_owned(),
        vec!["a1-bbs-04-020".to_owned()],
    )
    .unwrap();
    assert_eq!(list.bundles.len(), 1, "第一份套餐落盘");
    let b = &list.bundles[0];
    assert_eq!(b.machine_id, "A1");
    assert!(b.asset_refs.iter().all(|r| r.resolvable), "引用都解析得到");
    assert!(b.asset_refs.iter().any(|r| r.is_bbs), "至少一条 BBS");

    // ⑥ 被机型引用（`defaultBundle` 那一格）
    let list = wb_set_machine_field(
        "A1".to_owned(),
        MachineField::DefaultBundle,
        Some("A1_default".to_owned()),
    )
    .unwrap();
    assert_eq!(
        list.machines[0].default_bundle.as_deref(),
        Some("A1_default"),
        "机型引用了刚建的那份套餐"
    );

    // ⑦ 重开工作台：整台设备的数据仍然读得通（没有被写成加载不起来的样子）
    let boot = wb_boot().unwrap();
    assert!(boot.initialized);
    assert!(
        boot.problem
            .as_deref()
            .unwrap_or_default()
            .contains("上游未配置"),
        "这台设备没有上游 —— 降级说明，不是数据坏了：{:?}",
        boot.problem
    );
    let list = wb_machines().unwrap();
    assert_eq!(list.machines.len(), 1);
    assert_eq!(
        list.machines[0].default_bundle.as_deref(),
        Some("A1_default")
    );

    // ⑧ 反向：**写不成的形状一律被拦**，而且已有数据一个不动
    wb_add_bundle(
        "bad".to_owned(),
        "官方推荐".to_owned(),
        "A1".to_owned(),
        vec!["no-such-asset".to_owned()],
    )
    .expect_err("引用不存在的资产必须被拦");
    wb_add_bundle(
        "bad".to_owned(),
        "官方推荐".to_owned(),
        "A1".to_owned(),
        vec!["a1-image".to_owned()],
    )
    .expect_err("没有 BBS 的套餐必须被拦");
    wb_add_bundle(
        "bad".to_owned(),
        "官方推荐".to_owned(),
        "NO_SUCH".to_owned(),
        vec!["a1-bbs-04-020".to_owned()],
    )
    .expect_err("归属一台不存在的机型必须被拦");
    wb_set_machine_field(
        "A1".to_owned(),
        MachineField::DefaultBundle,
        Some("NO_SUCH".to_owned()),
    )
    .expect_err("引用不存在的套餐必须被拦");
    wb_set_machine_field(
        "A1".to_owned(),
        MachineField::Icon,
        Some("no-such-asset".to_owned()),
    )
    .expect_err("引用不存在的资产必须被拦");

    let list = wb_machines().unwrap();
    assert_eq!(
        list.machines[0].default_bundle.as_deref(),
        Some("A1_default"),
        "被拦下的写入没有动已有数据"
    );
    assert_eq!(list.machines[0].image.as_deref(), Some("a1-image"));
    assert!(list.machines[0].icon.is_none(), "被拦下的那一格没有写进去");
}
