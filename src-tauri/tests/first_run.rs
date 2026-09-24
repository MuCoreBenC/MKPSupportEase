//! 新设备第一次打开（b05 Task 15 的验收场景，2026-09-24 指定）：
//! **在一台没有工作台数据的新设备上打开软件，能不能正常进入初始化、
//! 创建好数据后再正常使用？**
//!
//! 独立测试文件 = 独立测试进程：`MKPSE_REPO_DIR` 指向临时目录（`paths.rs`
//! 的第一优先级），整台"设备"落在临时目录里 —— 不碰真仓库；本文件只有
//! 一个测试，环境变量设置安全。这台设备上没有上游仓库 → 顺带覆盖裁定①
//! （降级模式）与裁定②（重复初始化拒绝）。

use mkp_support_ease_lib::workbench::app::init::wb_init_workbench;
use mkp_support_ease_lib::workbench::app::machines::{wb_add_machine, wb_add_version, wb_machines};
use mkp_support_ease_lib::workbench::app::wb_boot;

#[test]
fn first_run_on_a_fresh_device_boots_into_the_initializer_then_recovers() {
    let device = tempfile::tempdir().unwrap();
    std::env::set_var("MKPSE_REPO_DIR", device.path());

    // ① 第一次打开：未初始化 —— boot 给引导语，只开放初始化入口
    let boot = wb_boot().unwrap();
    assert!(!boot.initialized, "新设备：工作台数据未初始化");
    assert!(
        boot.problem
            .as_deref()
            .unwrap_or_default()
            .contains("尚未初始化"),
        "引导语要说出下一步：{:?}",
        boot.problem
    );
    assert!(boot.info.is_none(), "未初始化没有上游统计");

    // ② 执行初始化：骨架建出来，根目录从此被识别
    let out = wb_init_workbench().unwrap();
    assert_eq!(out.created.len(), 5, "五个骨架文件（machines/ 是目录）");
    assert!(
        device
            .path()
            .join("presets/registry/param_registry.toml")
            .is_file(),
        "标志文件落地 = 已初始化"
    );

    // ③ 再打开：已初始化。这台设备没有上游 → 引导/降级（裁定①），但业务可用
    let boot = wb_boot().unwrap();
    assert!(boot.initialized, "初始化之后 presets_root() 才识别");
    assert!(boot.info.is_none(), "没有上游就没有统计 —— 降级不是失败");
    let problem = boot.problem.as_deref().unwrap_or_default();
    assert!(problem.contains("上游未配置"), "降级要说原因：{problem}");

    // ④ 建第一台机型与第一个版本（不要求参数源已存在）
    let list = wb_machines().unwrap();
    assert!(list.machines.is_empty(), "空白起点：零机型");
    let list = wb_add_machine("A1".to_owned(), "Bambu Lab".to_owned(), "A1".to_owned()).unwrap();
    assert_eq!(list.machines.len(), 1, "第一台机型落盘");
    let list = wb_add_version("A1".to_owned(), "STANDARD".to_owned(), "标准版".to_owned()).unwrap();
    assert_eq!(list.machines[0].versions.len(), 1, "第一个版本落盘");
    assert!(
        !list.machines[0].versions[0].has_recipe,
        "新版本标「参数源待补」，不要求参数源已存在"
    );

    // ⑤ 重复初始化：拒绝且不动已有数据（裁定②）
    assert!(wb_init_workbench().is_err(), "已有数据，初始化必须拒绝");
    let list = wb_machines().unwrap();
    assert_eq!(list.machines.len(), 1, "拒绝初始化没有动数据");
    assert_eq!(list.machines[0].versions.len(), 1);
}
