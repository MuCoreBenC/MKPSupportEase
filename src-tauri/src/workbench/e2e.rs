//! 端到端：把 doc §16.4 那条链路跑一遍。
//!
//! 改配方 → 保存 → 草稿 → 生成待更新项 → 收录 BBS → 上架 → 编套餐 → 出货检查 → 发布
//!
//! **诚实边界**：这条链路走的是 Rust 侧的函数，不是点界面。GUI 上的点击、
//! 页签切换、输入框行为不在这里的覆盖范围内 —— 那需要真实窗口，本稿没做。
//! 所以它证明的是"这套数据流是通的"，不是"界面好用"。

use crate::workbench::bbs;
use crate::workbench::capability::{CapField, ClientCapability, Support};
use crate::workbench::catalog::{self, BbsEntry, BbsOffering, Bundle, Bundles, PresetEntry};
use crate::workbench::clock;
use crate::workbench::generate;
use crate::workbench::model::{builtin_registry, BbsBinding, Machine, Version};
use crate::workbench::preflight;
use crate::workbench::publish;
use crate::workbench::state::{self, GenState};
use crate::workbench::store::{empty_params, Draft, Store};

fn caps() -> Vec<ClientCapability> {
    vec![ClientCapability {
        client_version: "0.0.1".into(),
        catalog_schema_version: 1,
        machines: vec!["A1".into()],
        fields: builtin_registry()
            .fields
            .iter()
            .map(|f| CapField {
                key: f.key.clone(),
                value_type: f.value_type,
                min: f.min,
                max: f.max,
                step: f.step,
            })
            .collect(),
    }]
}

#[test]
fn full_flow_from_recipe_to_publish() {
    let dist = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let s = Store::at(home.path());

    /* 0. 首次启动：目录建齐、全局配置补上、**一个配方都没写** */
    let boot = s.bootstrap().unwrap();
    assert!(boot.wrote_registry && !boot.has_machines);

    // 能力定义是只读输入，这里当作"客户端侧给过来的"放进去
    s.write_doc(
        "capability/support.json",
        &Support {
            supported: vec!["0.0.1".into()],
        },
    )
    .unwrap();
    s.write_doc("capability/client-0.0.1.json", &caps()[0]).unwrap();

    /* 1. 建机型与版本 */
    let mut base = empty_params();
    base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
    base.insert("toolhead.offset_x".into(), serde_json::json!(256.0));
    base.insert("motion.travel_speed".into(), serde_json::json!(300.0));
    base.insert("support.enable_brim".into(), serde_json::json!(true));
    s.save_machine(&Machine {
        id: "A1".into(),
        display_name: "A1".into(),
        base,
        default_bbs: vec![],
    })
    .unwrap();
    s.save_version(&Version {
        id: "std".into(),
        display_name: "标准版".into(),
        machine_id: "A1".into(),
        overrides: empty_params(),
        bbs: BbsBinding::Inherit,
    })
    .unwrap();

    /* 2. 改配方但**先不保存** —— 草稿要能把它留住 */
    let mut pending = empty_params();
    pending.insert("toolhead.z_offset".into(), serde_json::json!(0.15));
    s.save_draft(
        "A1",
        "std",
        &Draft {
            base_hash: "开着的时候那个".into(),
            overrides: pending.clone(),
            saved_at: clock::now_iso8601(),
        },
    )
    .unwrap();

    // 换一个 Store 实例 = 关掉再开
    let s = Store::at(home.path());
    assert_eq!(
        s.draft("A1", "std").unwrap().unwrap().overrides["toolhead.z_offset"],
        serde_json::json!(0.15),
        "重开后草稿没了"
    );

    /* 3. 保存 —— 保存**不会**自动生成 TOML */
    let mut v = s.version("A1", "std").unwrap();
    v.overrides = pending;
    s.save_version(&v).unwrap();
    s.clear_draft("A1", "std").unwrap();

    let before = state::status_all(&s, dist.path()).unwrap();
    assert_eq!(before[0].state, GenState::Stale, "保存后应当是待生成");
    assert!(
        !dist.path().join("presets").exists(),
        "保存却顺手生成了 TOML"
    );

    /* 4. 生成待更新项 */
    let plan = state::plan_stale(&s, dist.path()).unwrap();
    assert_eq!(plan, vec![("A1".to_string(), "std".to_string())]);
    let out = generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
    assert_eq!(out.min_client_version, "0.0.1");
    assert_eq!(
        state::status_all(&s, dist.path()).unwrap()[0].state,
        GenState::Generated
    );
    // 再点一次：待更新清单应当是空的
    assert!(state::plan_stale(&s, dist.path()).unwrap().is_empty());

    /* 5. 收录 BBS。它默认是「仅归档」—— 客户端看不到 */
    bbs::import(&s, "BBS-01", b"{\"kind\":\"process\"}").unwrap();
    assert_eq!(bbs::list(&s).unwrap()[0].state, bbs::BbsState::ArchivedOnly);

    /* 6. 上架：预设 + BBS。仓库里有 ≠ 客户端能看到，这一步才是"能看到" */
    catalog::save_catalog(
        &s,
        &catalog::Catalog {
            presets: vec![PresetEntry {
                preset_id: out.preset_id.clone(),
                machine: "A1".into(),
                version: "std".into(),
                display_name: "A1 标准版".into(),
                resource: out.output_rel.clone(),
                sha256: None,
                size: None,
                min_client_version: None,
                standalone: true,
            }],
            bbs: vec![BbsEntry {
                bbs_id: "BBS-01".into(),
                display_name: "工艺件".into(),
                resource: "bbs/BBS-01.json".into(),
                sha256: None,
                size: None,
                min_client_version: None,
                offering: BbsOffering::Optional,
            }],
            ..Default::default()
        },
    )
    .unwrap();

    /* 7. 套餐：一个预设 + 一瓶饮料 */
    catalog::save_bundles(
        &s,
        &Bundles {
            bundles: vec![Bundle {
                bundle_id: "a1-full".into(),
                display_name: "A1 全套".into(),
                presets: vec![out.preset_id.clone()],
                bbs: vec!["BBS-01".into()],
                min_client_version: None,
            }],
        },
    )
    .unwrap();

    /* 8. 出货检查 */
    let pre = preflight::run(&s, dist.path()).unwrap();
    assert!(pre.can_publish, "{:?}", pre.findings);

    /* 9. 发布。菜单最后写，开发配方不出现在发布目录里 */
    let pub_report = publish::run(&s, dist.path()).unwrap();
    assert_eq!(pub_report.files.last().unwrap(), "catalog.json");
    assert_eq!(pub_report.preset_count, 1);
    assert_eq!(pub_report.bbs_count, 1);
    assert_eq!(pub_report.bundle_count, 1);

    let published: catalog::Catalog =
        serde_json::from_str(&std::fs::read_to_string(dist.path().join("catalog.json")).unwrap())
            .unwrap();
    assert_eq!(
        published.presets[0].min_client_version.as_deref(),
        Some("0.0.1")
    );
    assert!(published.presets[0].sha256.is_some());

    // 开发配方的三样东西都不该出现在发布目录里
    for forbidden in ["registry.json", "fallback.json", "machines"] {
        assert!(
            !dist.path().join(forbidden).exists(),
            "{forbidden} 被发布出去了"
        );
    }

    /* 10. 发布之后再改一笔配方：状态立刻回到"待生成"。
    发布**不会**把配方冻住，也不代表客户端已经更新 */
    let mut v2 = s.version("A1", "std").unwrap();
    v2.overrides
        .insert("toolhead.z_offset".into(), serde_json::json!(0.2));
    s.save_version(&v2).unwrap();
    assert_eq!(
        state::status_all(&s, dist.path()).unwrap()[0].state,
        GenState::Stale
    );
}

/// 反面：**没生成就发布**要被拦住。
/// 这条是对上面那条的反空转 —— 如果 preflight 什么都不拦，上面那条也就没有意义
#[test]
fn publishing_without_generating_is_blocked() {
    let dist = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let s = Store::at(home.path());
    s.bootstrap().unwrap();
    s.write_doc(
        "capability/support.json",
        &Support {
            supported: vec!["0.0.1".into()],
        },
    )
    .unwrap();
    s.write_doc("capability/client-0.0.1.json", &caps()[0]).unwrap();

    let mut base = empty_params();
    base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
    s.save_machine(&Machine {
        id: "A1".into(),
        display_name: "A1".into(),
        base,
        default_bbs: vec![],
    })
    .unwrap();
    s.save_version(&Version {
        id: "std".into(),
        display_name: "标准版".into(),
        machine_id: "A1".into(),
        overrides: empty_params(),
        bbs: BbsBinding::Inherit,
    })
    .unwrap();

    // 上架了，但一次都没生成
    catalog::save_catalog(
        &s,
        &catalog::Catalog {
            presets: vec![PresetEntry {
                preset_id: "a1-std".into(),
                machine: "A1".into(),
                version: "std".into(),
                display_name: "A1 标准版".into(),
                resource: "presets/a1-std.toml".into(),
                sha256: None,
                size: None,
                min_client_version: None,
                standalone: true,
            }],
            ..Default::default()
        },
    )
    .unwrap();

    assert!(!preflight::run(&s, dist.path()).unwrap().can_publish);
    assert!(publish::run(&s, dist.path()).is_err());
    assert!(
        !dist.path().join("catalog.json").exists(),
        "被拦住了却还是写了菜单"
    );
}
