//! 判据 K-LC0：`(机型, 变体)` 这把钥匙 —— 谁和谁是「同一份预设」。
//!
//! 这把钥匙决定三件事：列表里哪些条目该合成一条、副本叫什么名字、
//! 副本的来源找不到时拿谁作对照。三处用同一把，所以它算错一次会同时错三处。

use std::path::PathBuf;

// 基线目录只认 `generate::` 那一处（理由见 `tests/recipe.rs`）
use preset::generate::fixtures_dir;
use preset::{PresetKey, read_preset_from_bytes};

fn fixtures() -> Vec<PathBuf> {
    let dir = fixtures_dir();
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .expect("读 fixture 目录")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    out.sort();
    out
}

fn key_of(text: &str) -> PresetKey {
    PresetKey::of(&read_preset_from_bytes(text.to_string()).expect("预设能读"))
}

/// K-LC0：9 份 fixture 的钥匙与文件名逐份钉住。
///
/// 这里同时钉住**文件名**：副本、内置产物、以及将来的任何产物都按它命名，
/// 名字变了等于用户目录里多出一份看起来一样的东西。
///
/// b05 Task 5 之前这张表是**四元组**（旧云端名 → 机型 → 变体 → 产物名），
/// 因为两边命名不同；夹具改名之后那层映射没有了，剩下的就是
/// 「钥匙 → 名字」——而名字由 `preset::preset_file_name` 算出，不是查表来的。
#[test]
fn klc0_every_fixture_has_the_key_we_expect() {
    let want: Vec<(&str, &str, Option<&str>)> = vec![
        ("A1-standard.toml", "A1", Some("standard")),
        ("A1-fast.toml", "A1", Some("fast")),
        ("A1-fastv3.3.toml", "A1", Some("fastv3.3")),
        ("A1_MINI-standard.toml", "A1_MINI", Some("standard")),
        ("A1_MINI-fast.toml", "A1_MINI", Some("fast")),
        ("A1_MINI-fastv3.3.toml", "A1_MINI", Some("fastv3.3")),
        ("P1S-lite.toml", "P1S", Some("lite")),
        ("P2S-standard.toml", "P2S", Some("standard")),
        ("X1C-lite.toml", "X1C", Some("lite")),
    ];
    assert_eq!(fixtures().len(), 9);

    for (name, machine, variant) in want {
        let path = fixtures()
            .into_iter()
            .find(|p| p.file_name().unwrap() == name)
            .unwrap_or_else(|| panic!("K-LC0 红：没有 fixture {name}"));
        let key = key_of(&std::fs::read_to_string(&path).expect("读"));
        assert_eq!(
            (key.machine.as_str(), key.variant.as_deref()),
            (machine, variant),
            "K-LC0 红：{name} 的钥匙不对"
        );
        assert_eq!(
            key.file_name(),
            name,
            "K-LC0 红：{name} 这个名字与钥匙算出来的不一样"
        );
    }
    println!("K-LC0 绿：9 份 fixture 的钥匙与文件名逐份对上");
}

/// 云端旧名与我们那套写法说的是**同一把钥匙**。
///
/// 这条判据以前叫 `klc0_same_machine_and_variant_is_one_key_regardless_of_file_name`：
/// 拿夹具里那份 `A1MF.toml` 与内置产物 `A1_MINI-fast.toml` 逐字节比，说明「同一份内容、
/// 两个文件名 ⇒ 同一把钥匙」。b05 Task 5 把夹具也改名之后，「两个文件名」这个前提没了
/// —— 内容相等由 `generate.rs` 的 `kg0p_products_match_the_baseline` 守。
///
/// 留下的真问题是：**旧名字还算不算同一把钥匙**。旧仓与用户目录里仍然是 `A1MF.toml`，
/// 它必须归到 `A1_MINI:fast` —— 归错了，同一个预设会被当成两台机器的两份参数。
#[test]
fn the_cloud_shorthand_is_the_same_key_as_our_own_spelling() {
    let text = std::fs::read_to_string(fixtures_dir().join("A1_MINI-fast.toml"))
        .expect("读夹具那份（A1_MINI 的 fast）");
    let from_header = key_of(&text);
    // 旧名那套的简写：`M` = mini、`F` = 快拆（读法见 spec 的 `migration-map.md`）
    let from_shorthand = PresetKey::new("A1MF", Some("fast"));

    assert_eq!(
        from_header, from_shorthand,
        "K-LC0 红：云端旧名 `A1MF` 与我们那套 `A1_MINI:fast` 必须是同一把钥匙"
    );
    assert_eq!(
        from_header.file_name(),
        "A1_MINI-fast.toml",
        "钥匙对应的文件名必须与夹具那份同名 —— 名字是算出来的"
    );
}

/// 大小写不算差别（变体），别名归一（机型）；**没有变体是另一把钥匙**。
#[test]
fn klc0_case_and_missing_variant() {
    assert_eq!(
        PresetKey::new("A1_MINI", Some("FAST")),
        PresetKey::new("A1_MINI", Some("fast")),
        "变体大小写不该算两把钥匙"
    );
    // 机型走别名表（`machine_catalog_extra.json` 的 aliasMap，实测 23 条）。
    // **它只认表里那些写法**：`A1MINI` / `A1MF` / `A1M` 都归到 `A1_MINI`，
    // 而带空格的 `a1 mini` **不在表里** —— 那时保留原样（见下面那条断言）。
    assert_eq!(
        PresetKey::new("a1mini", Some("fast")).machine,
        "A1_MINI",
        "机型别名要归一（归一表在 postprocess 那侧，只认表里的写法）"
    );
    assert_eq!(
        PresetKey::new("A1MF", Some("fast")),
        PresetKey::new("A1_MINI", Some("fast")),
        "云端文件名那套简写（A1MF）也在别名表里，归到同一台机器"
    );
    assert_eq!(
        PresetKey::new("a1 mini", None).machine,
        "a1 mini",
        "带空格的写法别名表里没有 ⇒ 保留原样（不认识就照实说，别猜）"
    );

    let with = PresetKey::new("A1", Some("standard"));
    let without = PresetKey::new("A1", None);
    assert_ne!(
        with, without,
        "K-LC0 红：老预设没有 `# variant:` 时不许被并进 standard —— \
         猜错会把两台机器的参数混成一条"
    );
    assert_eq!(without.file_name(), "A1.toml");
    assert_eq!(without.label(), "A1");
    assert_eq!(with.label(), "A1 standard");

    // 空白与空串都当「没有变体」，但机型归一不出来时**保留原样**（不能折成空串，
    // 否则两台不认识的机器会变成同一把钥匙）
    assert_eq!(PresetKey::new("A1", Some("   ")), without);
    assert_eq!(
        PresetKey::new("NoSuchMachine", None).machine,
        "NoSuchMachine"
    );
    assert_ne!(
        PresetKey::new("NoSuchMachine", None),
        PresetKey::new("AlsoUnknown", None)
    );
    println!("K-LC0 绿：大小写/别名归一，缺变体与未知机型都不会被并成一把");
}
