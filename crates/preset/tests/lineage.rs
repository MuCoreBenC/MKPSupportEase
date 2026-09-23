//! 判据 K-O2 / K-O2b：建副本只加三行，正文一个字节不动，而且副本还读得回来。
//!
//! 为什么要在 9 份 fixture 上全跑一遍：真实预设里有 inline table（`offset = { x = … }`）、
//! `"""` 多行 G-code、约 70 条注释 / 100 行、混大小写键（`MKP_retract`）。
//! 「拷字节 + 加三行」听起来不可能出错，但只要有人哪天把它改成
//! 「读成结构体再写出来」，这些东西会一起消失，而界面上完全看不出来。

use std::path::{Path, PathBuf};

use mkp_preset::lineage::{make_copy, sha256_hex, strip_lineage_for_compare};

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/presets");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("读 fixture 目录")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    out.sort();
    assert!(out.len() >= 9, "fixture 应有 9 份，实测 {}", out.len());
    out
}

/// K-O2：副本 = 来源 + 三行。剪掉三行必须逐字节相同，且来源文件没被碰过。
#[test]
fn ko2_a_copy_differs_from_its_source_only_by_the_lineage_lines() {
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let before = std::fs::metadata(&path).expect("读属性");
        let src = std::fs::read_to_string(&path).expect("读来源");

        let copy = make_copy(&src, &format!("mkp/{name}"));

        assert_eq!(
            strip_lineage_for_compare(&copy),
            src,
            "K-O2 红：{name} 的副本剪掉三行之后与来源不同 —— 正文被动过了"
        );
        assert_eq!(
            copy.len(),
            src.len() + copy.len() - strip_lineage_for_compare(&copy).len(),
            "K-O2 红：{name} 的长度账算不平"
        );

        // 三行都在，且摘要就是来源全文的摘要（不是副本自己的）
        assert!(copy.contains(&format!("# based_on: mkp/{name}")));
        assert!(copy.contains(&format!("# based_on_sha256: {}", sha256_hex(&src))));

        // **值必须一模一样**。剪三行再比只能证明「加了三行」，证明不了那三行加在了安全的位置 ——
        // 插进 `"""` 多行 G-code 里面，剪的时候一样能剪掉，但那段 G-code 已经多了一行。
        // 所以这里再比一次 serde 面。
        let a = mkp_preset::read_preset_from_bytes(src.clone()).expect("来源读得回来");
        let b = mkp_preset::read_preset_from_bytes(copy.clone()).expect("副本读得回来");
        assert_eq!(
            a.config, b.config,
            "K-O2 红：{name} 的副本值变了 —— 三行插到了会改变内容的位置"
        );

        // 来源文件本身：一个字节、一个 mtime 都没动（我们只读了它）
        let after = std::fs::metadata(&path).expect("读属性");
        assert_eq!(
            before.len(),
            after.len(),
            "K-O2 红：{name} 的来源字节数变了"
        );
        assert_eq!(
            before.modified().unwrap(),
            after.modified().unwrap(),
            "K-O2 红：{name} 的来源 mtime 变了 —— 建副本不许写来源"
        );
    }
    println!("K-O2 绿：9 份 fixture 的副本都只多了三行血统，来源字节与 mtime 未动");
}

/// K-O2b：副本仍是一份**能用的预设** —— `read_preset` 读得回来、`load_ir` 跑得通、血统三项都在。
///
/// 血统是注释，理论上不影响解析；但 `deny_unknown_fields` 与头注释匹配器都在这条路上，
/// 「理论上不影响」这句话得有判据兜着。
#[test]
fn ko2b_a_copy_is_still_a_usable_preset() {
    let dir = tempfile::tempdir().expect("临时目录");
    let mut checked = 0usize;
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&path).expect("读来源");
        let copy = make_copy(&src, &format!("mkp/{name}"));

        let out = dir.path().join(&name);
        std::fs::write(&out, &copy).expect("写副本");

        let file = mkp_preset::read_preset(&out)
            .unwrap_or_else(|e| panic!("K-O2b 红：{name} 的副本读不回来了：{e}"));
        let lineage = file
            .lineage
            .unwrap_or_else(|| panic!("K-O2b 红：{name} 的副本没有血统"));
        assert_eq!(
            lineage.based_on.as_deref(),
            Some(format!("mkp/{name}").as_str())
        );
        assert_eq!(
            lineage.based_on_sha256.as_deref(),
            Some(sha256_hex(&src).as_str())
        );
        // 原有的头字段没被血统挤掉
        assert!(!file.machine.is_empty(), "K-O2b 红：{name} 的 machine 丢了");

        mkp_preset::load_ir(&out, None)
            .unwrap_or_else(|e| panic!("K-O2b 红：{name} 的副本 load_ir 失败：{e}"));
        checked += 1;
    }
    assert!(checked >= 9, "K-O2b 红：只验了 {checked} 份");
    println!("K-O2b 绿：{checked} 份副本都能 read_preset + load_ir，血统三项读得回来");
}

// ——————————————————————————————————————————————————————————————
// K-O3：差异的五种状态 + 三个值
// ——————————————————————————————————————————————————————————————

use mkp_preset::lineage::{DiffState, FieldDiff, diff};
use mkp_preset::write::EditValue;

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
    // 我的副本：改了 MKP_retract / wiper_y / wipetower_speed
    let mine = preset(
        &["speed_limit = 70", "MKP_retract = 5"],
        &[
            "wiper_y = 30",
            "wipetower_speed = 25",
            "nozzle_cooling_flag = false",
        ],
    );
    // 官方现在：改了 speed_limit / wiper_y / wipetower_speed，
    // 加了 user_dry_time，删了 nozzle_cooling_flag
    let official = preset(
        &["speed_limit = 65", "MKP_retract = 0"],
        &["wiper_y = 40", "wipetower_speed = 25", "user_dry_time = 5"],
    );

    let diffs = diff(&mine, Some(&base), &official).expect("比差异");

    // ① 只有官方变了 —— 可以放心采纳
    let d = find(&diffs, "toolhead.speed_limit");
    assert_eq!(d.state, DiffState::OfficialOnly);
    assert_eq!(
        (d.mine.clone(), d.base.clone(), d.official.clone()),
        (
            Some(EditValue::Int(70)),
            Some(EditValue::Int(70)),
            Some(EditValue::Int(65))
        ),
        "K-O3 红：三个值要各就各位（我的 / 基线 / 官方）"
    );

    // ② 只有我改了 —— 保持
    let d = find(&diffs, "toolhead.MKP_retract");
    assert_eq!(d.state, DiffState::MineOnly);
    assert_eq!(d.mine, Some(EditValue::Int(5)));
    assert_eq!(d.official, Some(EditValue::Int(0)));

    // ③ 两边都变了 —— 真冲突，只能人来选
    let d = find(&diffs, "wiping.wiper_y");
    assert_eq!(d.state, DiffState::BothChanged);
    assert_eq!(
        (d.mine.clone(), d.base.clone(), d.official.clone()),
        (
            Some(EditValue::Int(30)),
            Some(EditValue::Int(20)),
            Some(EditValue::Int(40))
        )
    );

    // ④ 官方新增
    let d = find(&diffs, "wiping.user_dry_time");
    assert_eq!(d.state, DiffState::OfficialAdded);
    assert_eq!(d.mine, None, "我的副本里没有这一项");
    assert_eq!(d.official, Some(EditValue::Int(5)));

    // ⑤ 官方移除
    let d = find(&diffs, "wiping.nozzle_cooling_flag");
    assert_eq!(d.state, DiffState::OfficialRemoved);
    assert_eq!(d.mine, Some(EditValue::Bool(false)));
    assert_eq!(d.official, None);

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
    let mine = preset(&["speed_limit = 70"], &["wiper_y = 30"]);
    let official = preset(&["speed_limit = 65"], &["wiper_y = 30"]);

    let diffs = diff(&mine, None, &official).expect("比差异");
    assert_eq!(diffs.len(), 1, "只有 speed_limit 不同：{:?}", paths(&diffs));
    assert_eq!(diffs[0].path, "toolhead.speed_limit");
    assert_eq!(
        diffs[0].state,
        DiffState::BothChanged,
        "K-O3b 红：基线不知道的时候必须保守 —— 多问一句，而不是替用户决定"
    );
    assert_eq!(diffs[0].base, None, "不知道就是 None，不许拿默认值假装知道");

    assert_eq!(diffs, diff(&mine, None, &official).expect("再比一遍"));
    println!("K-O3b 绿：无基线 ⇒ BothChanged 且 base 为 None；同输入两遍结果相同");
}
