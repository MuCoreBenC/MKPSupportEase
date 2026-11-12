//! 入库产物 / 对照基线的**路径助手与同步入口**。
//!
//! # 这里不再渲染任何东西
//!
//! 2026-11-04 起 `gen-presets` 退役、2026-11-12 起 `preset_recipes.toml` 整链删干净：
//! `presets/*.toml` 是唯一权威源，产物由工作台 `wb_generate` 渲染并写到
//! [`assets_dir`]，本模块只负责「产物落在哪、文件名怎么算、基线怎么同步」。
//! 渲染实现在 `src-tauri/src/workbench/app/build.rs`，不在本 crate。
//!
//! # 幂等仍然是硬约束
//!
//! 这里不许出现 `uuid::new_v4()` / `now()` / `SystemTime`。
//! 门禁是 `tests/write_discipline_scan.rs` 的 `the_generator_stays_pure`。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 入库产物目录（`crates/preset/assets/presets/`）。**不是用户目录。**
pub fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/presets")
}

/// 产物的文件名：`<机型 id>-<版本 id 小写>.toml`。
///
/// **这是全仓唯一的命名实现**（规范见 `docs/ARCHITECTURE.md` §10）。生成、发布、
/// 消费端查找、目录 JSON 四方都走这一个函数；`src-tauri` 那边的
/// `workbench::app::build::preset_file_name` 是它的薄壳，`PresetKey::file_name`
/// 有变体那一档也转调这里（它无变体那一档是显式例外：老预设没有版本 id）。
///
/// 「唯一」由判据钉住：`naming_follows_the_one_rule`（规则本身）、
/// `naming_matches_the_preset_crate`（跨 crate 的薄壳）、
/// `pairing_goes_by_file_name`（产物与基线的名字集合相同）。
///
/// 之前是两份独立实现：这里直接拼 `{machine}-{variant}`，`src-tauri` 那边多做一次
/// `to_lowercase()`。两处形状相同但没有编译器连着它们 —— 漂移只是时间问题，
/// 而漂移的后果不是报错，是消费端找不到文件。
///
/// **小写在这里发生，只发生一次。** 配方里的变体本来就是小写（`standard` / `fast` /
/// `fastv3.3`），所以对生成这一侧是恒等变换，九份产物的名字一个都不变；
/// 机型 id 不动（`A1_MINI` 保持原样，它的下划线是身份的一部分）。
///
/// **刻意不沿用云端那批文件名**（`A1MF_260628.toml` 之类）：那些名字是另一套系统的，
/// 照抄只会在用户目录里制造同名混淆。旧名到新身份的映射是一次性的迁移输入。
pub fn file_name(machine: &str, variant: &str) -> String {
    format!("{machine}-{}.toml", variant.to_lowercase())
}

/// 对照基线目录（`crates/postprocess/tests/fixtures/presets/`）。
///
/// **这是内核判据的输入，也是「上一次审阅通过的样子」**。翻案之后（spec `recipe-workbench` §0）
/// 它不再是真源：真源是 `presets/*.toml`，产物由工作台生成，基线是产物的一份留影。
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../postprocess/tests/fixtures/presets")
}

/// 目录里的 `.toml`，按**文件名**索引（`A1-standard.toml` → 路径）。
///
/// **配对按文件名，是 b05 Task 5 之后才成立的事。** 以前基线沿用云端那套名字
/// （`A1F_260628.toml`），与产物名对不上，只能读文件头认身份（旧的 `pair_by_head`）。
/// 基线改名之后两边名字都由 [`file_name`] 算出 —— **名字就是身份**，
/// 于是配对不需要再读文件内容，能读到的第一处差异就是真差异。
fn toml_files(dir: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("读不到 {}：{e}", dir.display()))?;
    let mut out = BTreeMap::new();
    for path in entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
    {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        out.insert(name, path);
    }
    Ok(out)
}

/// [`sync_baseline`] 的落点闸：只允许两类目标目录。
///
/// 它是全仓**唯一**能改内核判据期望值的写盘点，所以"写到哪"也要钉住，
/// 而不是只在文档里叮嘱一句：
///
/// - **真基线目录** [`fixtures_dir`]（产品路径）；
/// - **系统临时目录之下**（判据路径 —— 那几条测试用 `tempfile::tempdir()`）。
///
/// 其他任何路径一律拒绝。误传一个仓库内别的目录，后果是"判据的期望值被悄悄换掉"，
/// 而那种错在产物上看不出来 —— 与「禁止 `UPDATE_GOLDEN=1`」防的是同一件事。
fn check_baseline_target(fixtures: &Path) -> Result<(), String> {
    let target = fixtures
        .canonicalize()
        .map_err(|e| format!("基线目录不可用（{}）：{e}", fixtures.display()))?;
    if fixtures_dir().canonicalize().ok().as_deref() == Some(target.as_path()) {
        return Ok(());
    }
    if let Ok(tmp) = std::env::temp_dir().canonicalize()
        && target.starts_with(&tmp)
    {
        return Ok(());
    }
    Err(format!(
        "拒绝把对照基线写到 {} —— 只允许 {}（真基线）或系统临时目录（判据用）",
        target.display(),
        fixtures_dir().display()
    ))
}

/// 把入库产物同步成对照基线，返回同步了几份。
///
/// **它写的是内核判据的夹具**（`crates/postprocess/tests/fixtures/presets/`）——
/// 也就是说这个函数**有能力改判据的期望值**。全仓能做到这件事的只有它一处。
/// 迁移期那条"禁止 `UPDATE_GOLDEN=1`"的纪律讲的是同一件事：
/// 把现状抄成期望，判据就从"证明"退化成"自比自"。
///
/// **这是一个需要人先看过 diff 的动作**（doc §0 的 ③）：它把「现在的产物」定成
/// 「上一次审阅通过的样子」。自动化它等于把唯一的安全网拆了。
///
/// 落点由 [`check_baseline_target`] 咬住：只能写真基线目录或临时目录。
///
/// 基线那边的文件名**就是产物的文件名**（同一个 [`file_name`] 算出来的）——
/// 内容相同就不写，所以「同步」只在真的变了的时候落笔。
///
/// **写盘豁免**（`clippy::disallowed_methods`）：见上 —— 它的风险不在"截断半个文件"，
/// 而在"改了判据期望却没人看 diff"。兜着后者的是人工审阅、`git diff`
/// 与上面那条落点断言。
///
/// 2026-11-12：产物归属早已定案（工作台生成），但**基线同步仍由生成动作驱动** ——
/// 改了版本 id 之后，`wb_generate` 会把新名产物写进 `assets_dir`、清掉旧名，
/// 再调这里让基线跟上，改 id 不再需要人手补任何清单。
#[allow(clippy::disallowed_methods)]
pub fn sync_baseline(assets: &Path, fixtures: &Path) -> Result<usize, String> {
    check_baseline_target(fixtures)?;
    let made = toml_files(assets)?;
    let mut synced = 0usize;
    for (name, made_path) in &made {
        let text = std::fs::read_to_string(made_path).map_err(|e| format!("读不回来：{e}"))?;
        let target = fixtures.join(name);
        if std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
            continue; // 内容相同就不写：mtime 变动会让别的判据重跑
        }
        std::fs::write(&target, &text).map_err(|e| format!("写 {} 失败：{e}", target.display()))?;
        synced += 1;
    }
    Ok(synced)
}

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#[allow(clippy::disallowed_methods)]
#[cfg(test)]
mod tests {
    use super::*;

    /// 命名规范（`docs/ARCHITECTURE.md` §10）的三条不变式。
    ///
    /// 这个函数现在是**全仓唯一**的命名实现，`src-tauri` 那边是薄壳。以前两处各拼一遍，
    /// 而两份独立实现之间没有编译器 —— 这条判据就是那个编译器。
    #[test]
    fn naming_follows_the_one_rule() {
        // ① 版本 id 小写化，且只在这里发生一次
        assert_eq!(file_name("A1", "FASTV3.3"), "A1-fastv3.3.toml");
        assert_eq!(file_name("A1", "fastv3.3"), "A1-fastv3.3.toml");

        // ② 机型 id 原样保留 —— `A1_MINI` 的下划线是身份的一部分，不许跟着小写或被换成 `-`
        assert_eq!(file_name("A1_MINI", "STANDARD"), "A1_MINI-standard.toml");

        // ③ 分隔符是 `-`：机型 id 自己含 `_`，用 `_` 分隔会歧义
        assert_eq!(file_name("P1S", "lite"), "P1S-lite.toml");
    }

    /// 仅大小写不同的版本 id 会塌成同一个文件名。
    ///
    /// 这不是"可能冲突"，是**必然冲突**：`docs/ARCHITECTURE.md` §10.4 因此要求唯一性检查
    /// 大小写不敏感。冲突的表现不是报错，是后写的产物静默覆盖先写的 —— 所以这里先把
    /// 事实钉住，校验层（b05 Task 11）再据此拦。
    #[test]
    fn case_only_differences_collide() {
        assert_eq!(file_name("A1", "Fast"), file_name("A1", "FAST"));
        assert_eq!(file_name("A1", "fast"), file_name("A1", "FaSt"));
    }

    /// **两边文件名集合一份不差** —— 配对直接按名字（b05 Task 5）。
    ///
    /// 改了版本 id 之后，生成器必须同时把入库产物与对照基线都跟上：只写新名不清旧名、
    /// 或只清了一侧，这里都会红。IR 判据的冻结输入已搬到 `fixtures/ir/presets/`，
    /// 这个目录只装生成基线，所以能咬严格相等。份数不写死 —— 版本增删由清单决定。
    #[test]
    fn pairing_goes_by_file_name() {
        let made = toml_files(&assets_dir()).expect("产物能列出来");
        let base = toml_files(&fixtures_dir()).expect("基线能列出来");
        assert!(!made.is_empty(), "入库产物一份都没有");
        let made_names: std::collections::BTreeSet<&String> = made.keys().collect();
        let base_names: std::collections::BTreeSet<&String> = base.keys().collect();
        assert_eq!(
            made_names, base_names,
            "两边文件名集合不同 —— 只在一侧出现的那些就是配不上的那些"
        );
    }

    /// 同步基线：内容相同的不写（mtime 不动），改过的那一份写回去。
    #[test]
    fn syncing_the_baseline_only_writes_what_changed() {
        let assets = tempfile::tempdir().expect("临时目录");
        let base = tempfile::tempdir().expect("临时目录");
        let a = assets.path().join("A1-standard.toml");
        let b = assets.path().join("A1-fast.toml");
        std::fs::write(&a, "# machine: A1\nkey = 1\n").expect("造产物");
        std::fs::write(&b, "# machine: A1\nkey = 2\n").expect("造产物");
        std::fs::copy(&a, base.path().join("A1-standard.toml")).expect("抄一份");
        std::fs::copy(&b, base.path().join("A1-fast.toml")).expect("抄一份");

        assert_eq!(
            sync_baseline(assets.path(), base.path()).expect("同步"),
            0,
            "两边一致时一个字节都不该写"
        );

        let victim = base.path().join("A1-standard.toml");
        std::fs::write(&victim, "# machine: A1\nkey = 9\n").expect("扰动基线");
        assert_eq!(
            sync_baseline(assets.path(), base.path()).expect("同步"),
            1,
            "只该写回那一份"
        );
        assert_eq!(
            std::fs::read_to_string(&victim).expect("读回"),
            "# machine: A1\nkey = 1\n"
        );
    }
}
