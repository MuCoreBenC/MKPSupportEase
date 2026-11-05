//! 判据 K-O3：参数差异的五种状态 + 三个值。
//!
//! 2026-11-04 从 `tests/lineage.rs` 拆出来：来源记录（provenance）与参数差异（diff）
//! 是两件事，判据也不再挤在同一个文件里。

#![allow(clippy::disallowed_methods)]

// ——————————————————————————————————————————————————————————————
// K-O3：差异的五种状态 + 三个值
// ——————————————————————————————————————————————————————————————

use preset::diff::{DiffState, FieldDiff, diff};
use preset::write::EditValue;

/// 拼一份最小可解析的预设（`snapshot` 只要求它是合法 TOML）。
fn preset(toolhead: &[&str], wiping: &[&str]) -> String {
    format!(
        "# machine: A1\n[toolhead]\n{}\n[wiping]\n{}\n",
        toolhead.join("\n"),
        wiping.join("\n")
    )
}

fn paths(diffs: &[FieldDiff]) -> Vec<&str> {
    diffs.iter().map(|d| d.path.as_str()).collect()
}

fn find<'a>(diffs: &'a [FieldDiff], path: &str) -> &'a FieldDiff {
    diffs.iter().find(|d| d.path == path).unwrap_or_else(|| {
        panic!(
            "K-O3 红：`{path}` 应当出现在差异里，实测只有 {:?}",
            paths(diffs)
        )
    })
}

/// K-O3：五种状态各一例 + 「两边都改成同一个值」不算差异。
#[test]
fn ko3_five_states_and_three_numbers() {
    // 基线：当初拷来的样子
    let base = preset(
        &["speed_limit = 70", "MKP_retract = 0"],
        &[
            "wiper_y = 20",
            "wipetower_speed = 20",
            "nozzle_cooling_flag = false",
        ],
    );
    // 本地副本：改了 MKP_retract / wiper_y / wipetower_speed
    let local = preset(
        &["speed_limit = 70", "MKP_retract = 5"],
        &[
            "wiper_y = 30",
            "wipetower_speed = 25",
            "nozzle_cooling_flag = false",
        ],
    );
    // 云端现在：改了 speed_limit / wiper_y / wipetower_speed，
    // 加了 user_dry_time，删了 nozzle_cooling_flag
    let cloud = preset(
        &["speed_limit = 65", "MKP_retract = 0"],
        &["wiper_y = 40", "wipetower_speed = 25", "user_dry_time = 5"],
    );

    let diffs = diff(&local, Some(&base), &cloud).expect("比差异");

    // ① 只有云端变了 —— 可以放心采纳
    let d = find(&diffs, "toolhead.speed_limit");
    assert_eq!(d.state, DiffState::CloudOnly);
    assert_eq!(
        (d.local.clone(), d.base.clone(), d.cloud.clone()),
        (
            Some(EditValue::Int(70)),
            Some(EditValue::Int(70)),
            Some(EditValue::Int(65))
        ),
        "K-O3 红：三个值要各就各位（本地 / 基线 / 云端）"
    );

    // ② 只有我改了 —— 保持
    let d = find(&diffs, "toolhead.MKP_retract");
    assert_eq!(d.state, DiffState::LocalOnly);
    assert_eq!(d.local, Some(EditValue::Int(5)));
    assert_eq!(d.cloud, Some(EditValue::Int(0)));

    // ③ 两边都变了 —— 真冲突，只能人来选
    let d = find(&diffs, "wiping.wiper_y");
    assert_eq!(d.state, DiffState::BothChanged);
    assert_eq!(
        (d.local.clone(), d.base.clone(), d.cloud.clone()),
        (
            Some(EditValue::Int(30)),
            Some(EditValue::Int(20)),
            Some(EditValue::Int(40))
        )
    );

    // ④ 云端新增
    let d = find(&diffs, "wiping.user_dry_time");
    assert_eq!(d.state, DiffState::CloudAdded);
    assert_eq!(d.local, None, "本地副本里没有这一项");
    assert_eq!(d.cloud, Some(EditValue::Int(5)));

    // ⑤ 云端移除
    let d = find(&diffs, "wiping.nozzle_cooling_flag");
    assert_eq!(d.state, DiffState::CloudRemoved);
    assert_eq!(d.local, Some(EditValue::Bool(false)));
    assert_eq!(d.cloud, None);

    // 「两边都改成了同一个值」不是差异 —— 没什么要决定的
    assert!(
        !paths(&diffs).contains(&"wiping.wipetower_speed"),
        "K-O3 红：两边都改成 25 了，不该还问一遍：{:?}",
        paths(&diffs)
    );
    // 没动过的键也不出现
    assert!(!paths(&diffs).contains(&"toolhead.offset.x"));

    assert_eq!(
        diffs.len(),
        5,
        "K-O3 红：应当恰好 5 项：{:?}",
        paths(&diffs)
    );
    println!("K-O3 绿：5 种状态各一例、三个值各就各位；两边同值与未变的键都不出现");
}

/// K-O3b：基线不可用时**保守**当真冲突；`diff` 是纯函数（同输入两遍同结果）。
#[test]
fn ko3b_without_a_baseline_everything_is_a_conflict() {
    let local = preset(&["speed_limit = 70"], &["wiper_y = 30"]);
    let cloud = preset(&["speed_limit = 65"], &["wiper_y = 30"]);

    let diffs = diff(&local, None, &cloud).expect("比差异");
    assert_eq!(diffs.len(), 1, "只有 speed_limit 不同：{:?}", paths(&diffs));
    assert_eq!(diffs[0].path, "toolhead.speed_limit");
    assert_eq!(
        diffs[0].state,
        DiffState::BothChanged,
        "K-O3b 红：基线不知道的时候必须保守 —— 多问一句，而不是替用户决定"
    );
    assert_eq!(diffs[0].base, None, "不知道就是 None，不许拿默认值假装知道");

    assert_eq!(diffs, diff(&local, None, &cloud).expect("再比一遍"));
    println!("K-O3b 绿：无基线 ⇒ BothChanged 且 base 为 None；同输入两遍结果相同");
}
