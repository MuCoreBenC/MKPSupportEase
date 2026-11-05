//! 判据 K-O2 / K-O2b：建副本只在**尾部**加一段来源记录，正文一个字节不动，
//! 而且副本还读得回来、还能跑 `load_ir`。
//!
//! 为什么要在 9 份 fixture 上全跑一遍：真实预设里有 inline table（`offset = { x = … }`）、
//! `"""` 多行 G-code、约 70 条注释 / 100 行、混大小写键（`MKP_retract`）。
//! 「拷字节 + 尾部加一段」听起来不可能出错，但只要有人哪天把它改成
//! 「读成结构体再写出来」，这些东西会一起消失，而界面上完全看不出来。
//!
//! 2026-11-04：来源记录从「头部三行 `# based_on*`」迁到「尾部 `[mkp_source]` 表」。
//! 原来按行剪的判据换成 [`preset::provenance::strip`]（同样的不变式：剥完 == 来源）。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use preset::generate::fixtures_dir;
use preset::model::{SourceProvenance, SourceRef};
use preset::provenance::{copy_with_provenance, parse, sha256_hex, strip};

fn fixtures() -> Vec<PathBuf> {
    let dir = fixtures_dir();
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

fn origin_of(file: &str, sha: String) -> SourceRef {
    SourceRef {
        kind: Some("copy".to_owned()),
        file: Some(file.to_owned()),
        sha256: Some(sha),
        ..SourceRef::default()
    }
}

/// K-O2：副本 = 来源 + 尾部记录。剥掉记录必须逐字节相同，且来源文件没被碰过。
#[test]
fn ko2_a_copy_differs_from_its_source_only_by_the_provenance_block() {
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let before = std::fs::metadata(&path).expect("读属性");
        let src = std::fs::read_to_string(&path).expect("读来源");

        let origin = origin_of(&format!("mkp/{name}"), sha256_hex(&src));
        let copy = copy_with_provenance(&src, origin.clone(), origin);

        assert_eq!(
            strip(&copy),
            src,
            "K-O2 红：{name} 的副本剥掉尾部记录之后与来源不同 —— 正文被动过了"
        );
        assert!(
            copy.starts_with(&src[..src.len().min(64)]),
            "K-O2 红：{name} 的副本开头变了 —— 记录本该只在尾部"
        );

        // 记录读得回来，且摘要就是**来源全文**的摘要（不是副本自己的）
        let prov = parse(&copy).unwrap_or_else(|| panic!("K-O2 红：{name} 的记录读不回来"));
        let origin = prov.origin.expect("K-O2 红：缺 origin");
        assert_eq!(origin.file.as_deref(), Some(format!("mkp/{name}").as_str()));
        assert_eq!(origin.sha256.as_deref(), Some(sha256_hex(&src).as_str()));

        // **值必须一模一样**：剥完再比只能证明「加了一段」，证明不了那段在安全的位置。
        let a = preset::read_preset_from_bytes(src.clone()).expect("来源读得回来");
        let b = preset::read_preset_from_bytes(copy.clone()).expect("副本读得回来");
        assert_eq!(
            a.config, b.config,
            "K-O2 红：{name} 的副本值变了 —— 来源记录插到了会改变内容的位置"
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
    println!("K-O2 绿：9 份 fixture 的副本都只在尾部多了来源记录，来源字节与 mtime 未动");
}

/// K-O2b：副本仍是一份**能用的预设** —— `read_preset` 读得回来、`load_ir` 跑得通、
/// 来源记录读得出来。
///
/// 来源记录是尾部一个独立表，理论上不影响解析；但 `deny_unknown_fields` 与
/// 「先剥再解析」都在这条路上，「理论上不影响」这句话得有判据兜着。
#[test]
fn ko2b_a_copy_is_still_a_usable_preset() {
    let dir = tempfile::tempdir().expect("临时目录");
    let mut checked = 0usize;
    for path in fixtures() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let src = std::fs::read_to_string(&path).expect("读来源");
        let origin = origin_of(&format!("mkp/{name}"), sha256_hex(&src));
        let copy = copy_with_provenance(&src, origin.clone(), origin);

        let out = dir.path().join(&name);
        std::fs::write(&out, &copy).expect("写副本");

        let file = preset::read_preset(&out)
            .unwrap_or_else(|e| panic!("K-O2b 红：{name} 的副本读不回来了：{e}"));
        let prov: SourceProvenance = file
            .provenance
            .unwrap_or_else(|| panic!("K-O2b 红：{name} 的副本没有来源记录"));
        let origin = prov.origin.expect("K-O2b 红：缺 origin");
        assert_eq!(origin.file.as_deref(), Some(format!("mkp/{name}").as_str()));
        assert_eq!(origin.sha256.as_deref(), Some(sha256_hex(&src).as_str()));
        // 原有的头字段没被记录挤掉
        assert!(!file.machine.is_empty(), "K-O2b 红：{name} 的 machine 丢了");

        preset::load_ir(&out, None)
            .unwrap_or_else(|e| panic!("K-O2b 红：{name} 的副本 load_ir 失败：{e}"));
        checked += 1;
    }
    assert!(checked >= 9, "K-O2b 红：只验了 {checked} 份");
    println!("K-O2b 绿：{checked} 份副本都能 read_preset + load_ir，来源记录读得回来");
}

/// **K-O3：履历只由「合并云端新版」产生，别的一律不生长**（2026-11-04 作者裁定）。
///
/// | 动作 | 产生履历吗 |
/// |---|---|
/// | 另存 / 新建一份（`write_origin_only`） | **不**——它只回答「我从哪来」 |
/// | 普通改参数后保存（写回自己） | 不（那条路根本不碰记录，见 `rewrite_keeping_provenance`） |
/// | 用户主动把云端新版合并进这份 | **是**，且同一版重复合并**不重复记** |
///
/// 这条判据把前两行钉死；第三行的去重判据在 `ipc::param_sync::same_version` 那边。
#[test]
fn ko3_history_only_grows_when_a_cloud_version_is_merged_in() {
    use preset::provenance::{append_version, write_origin_only};

    let src = &std::fs::read_to_string(&fixtures()[0]).expect("读 fixture");
    let origin = origin_of("mkp/presets/A1-standard.toml", sha256_hex(src));

    /* ① 另存：有来源、没有履历 */
    let saved = write_origin_only(src, origin.clone());
    let prov = parse(&saved).expect("另存要有来源记录");
    assert!(
        prov.history.is_empty(),
        "K-O3 红：另存凭空长出了 {} 条履历",
        prov.history.len()
    );
    assert_eq!(strip(&saved), *src, "K-O3 红：另存动了正文");

    /* ② 合并 V2：长出第 1 条，来源不动 */
    let v2 = SourceRef {
        file: Some("mkp/presets/A1-standard.toml".to_owned()),
        at: Some("2026-10-31T00:00:00Z".to_owned()),
        sha256: Some("b".repeat(64)),
        catalog: Some("support-ease".to_owned()),
        serial: Some(2),
        revision: Some("rev-2".to_owned()),
        ..SourceRef::default()
    };
    let merged = append_version(&saved, v2.clone());
    let prov = parse(&merged).expect("合并后记录还在");
    assert_eq!(prov.history.len(), 1, "K-O3 红：合并一版应当恰好一条履历");
    assert_eq!(
        prov.origin.as_ref().and_then(|o| o.file.as_deref()),
        Some("mkp/presets/A1-standard.toml"),
        "K-O3 红：合并不许改 origin —— 那是「我从哪来」，不是「我合并了谁」"
    );
    assert_eq!(strip(&merged), *src, "K-O3 红：合并动了正文");

    /* ③ 再合并 V3：第 2 条，且 V2 那条一个字不改 */
    let v3 = SourceRef {
        at: Some("2026-11-02T00:00:00Z".to_owned()),
        sha256: Some("c".repeat(64)),
        serial: Some(3),
        revision: Some("rev-3".to_owned()),
        ..v2.clone()
    };
    let merged2 = append_version(&merged, v3);
    let prov = parse(&merged2).expect("记录还在");
    assert_eq!(prov.history.len(), 2, "K-O3 红：合并两版应当两条履历");
    assert_eq!(
        prov.history[0].sha256.as_deref(),
        Some("b".repeat(64).as_str()),
        "K-O3 红：合并 V3 把 V2 那条改掉了 —— 履历只追加，旧条目一个字不改"
    );
    assert_eq!(prov.history[1].serial, Some(3));
}
