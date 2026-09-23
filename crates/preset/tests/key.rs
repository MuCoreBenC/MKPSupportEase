//! 判据 K-LC0：`(机型, 变体)` 这把钥匙 —— 谁和谁是「同一份预设」。
//!
//! 这把钥匙决定三件事：列表里哪些条目该合成一条、副本叫什么名字、
//! 副本的来源找不到时拿谁作对照。三处用同一把，所以它算错一次会同时错三处。

use std::path::{Path, PathBuf};

use mkp_preset::{PresetKey, read_preset_from_bytes};

fn fixtures() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../core/tests/fixtures/presets");
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
#[test]
fn klc0_every_fixture_has_the_key_we_expect() {
    let want: Vec<(&str, &str, Option<&str>, &str)> = vec![
        ("A1.toml", "A1", Some("standard"), "A1-standard.toml"),
        ("A1F.toml", "A1", Some("fast"), "A1-fast.toml"),
        (
            "A1F_260628.toml",
            "A1",
            Some("fastv3.3"),
            "A1-fastv3.3.toml",
        ),
        (
            "A1M.toml",
            "A1_MINI",
            Some("standard"),
            "A1_MINI-standard.toml",
        ),
        ("A1MF.toml", "A1_MINI", Some("fast"), "A1_MINI-fast.toml"),
        (
            "A1MF_260628.toml",
            "A1_MINI",
            Some("fastv3.3"),
            "A1_MINI-fastv3.3.toml",
        ),
        ("P1.toml", "P1S", Some("lite"), "P1S-lite.toml"),
        ("P2.toml", "P2S", Some("standard"), "P2S-standard.toml"),
        ("X1.toml", "X1C", Some("lite"), "X1C-lite.toml"),
    ];
    assert_eq!(fixtures().len(), 9);

    for (name, machine, variant, file_name) in want {
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
        assert_eq!(key.file_name(), file_name, "K-LC0 红：{name} 的文件名不对");
    }
    println!("K-LC0 绿：9 份 fixture 的钥匙与文件名逐份对上");
}

/// 同一台机器的同一个变体、两个文件名 ⇒ **同一把钥匙**。
///
/// 这正是现实里的样子：云端叫 `A1MF.toml`、我们的内置产物叫 `A1_MINI-fast.toml`，
/// 内容完全相同（实测 sha 都是 `115e061f…`）。
#[test]
fn klc0_same_machine_and_variant_is_one_key_regardless_of_file_name() {
    let cloud = std::fs::read_to_string(
        fixtures()
            .into_iter()
            .find(|p| p.file_name().unwrap() == "A1MF.toml")
            .expect("A1MF"),
    )
    .expect("读云端那份");
    let builtin = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/presets/A1_MINI-fast.toml"),
    )
    .expect("读内置那份");

    assert_eq!(cloud, builtin, "前提：这两份内容本来就相同");
    assert_eq!(
        key_of(&cloud),
        key_of(&builtin),
        "K-LC0 红：同机同变体的两份文件必须是同一把钥匙"
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
        "机型别名要归一（归一表在 mkp_pp 那侧，只认表里的写法）"
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
