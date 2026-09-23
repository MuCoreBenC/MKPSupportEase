//! 生成器的**两个动作**：检查（只读）与写盘。
//!
//! # 为什么这些代码不在 `bin/gen_presets.rs` 里
//!
//! 原来整套「渲染 9 份 → 与入库产物逐字节比 → 扫多余产物」在 CLI 的 `run()` 里。
//! 工作台（spec `recipe-workbench`）要用同一套：如果在命令那边再写一遍，
//! 迟早会出现「CLI 绿、界面红」而两边都自称对。所以本模块是**唯一真源**，
//! CLI 与 Tauri 命令都是它的薄壳。
//!
//! # 幂等仍然是硬约束
//!
//! 这里不许出现 `uuid::new_v4()` / `now()` / `SystemTime` —— `uuid` 与发布时间只能来自配方
//! （门禁 `scripts/check_generator_purity.py` 扫本文件）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::recipe::{Recipe, render};

/// 一次检查的结论。
///
/// `first_diff` 是 `None` 才算通过。**只报第一处** —— 九份文件的全量差异是给
/// `git diff` 看的，这里要的是「照着修哪一行」。
#[derive(Debug, Clone, PartialEq)]
pub struct CheckReport {
    /// 比过几份。
    pub checked: usize,
    /// 第一处不同在哪（文件名 + 行号 + 两边原文）。
    pub first_diff: Option<String>,
}

/// 仓库里那份配方的路径。
///
/// **`CARGO_MANIFEST_DIR` 是编译期常量**：它指向编译这个 crate 时的源码树。
/// 也就是说 —— 从仓库工作副本跑起来的 debug 程序能找到配方，
/// 而拷到别处的 `.app` 找不到（那正是我们要的：配方不随分发物走）。
/// **拿不到文件时不 panic**：调用方要能把这条路径显示给人看。
pub fn recipe_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/preset_recipes.toml")
}

/// 入库产物目录（`crates/preset/assets/presets/`）。**不是用户目录。**
pub fn assets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/presets")
}

/// 产物的文件名：`<机型>-<变体>.toml`。
///
/// **刻意不沿用云端那批文件名**（`A1MF_260628.toml` 之类）：那些名字是另一套系统的，
/// 照抄只会在用户目录里制造同名混淆。
pub fn file_name(machine: &str, variant: &str) -> String {
    format!("{machine}-{variant}.toml")
}

/// 第一处不同在哪 —— 报告要能直接照着修（行号 + 两边原文）。
pub fn first_diff(want: &str, got: &str) -> Option<String> {
    for (i, (a, b)) in want.lines().zip(got.lines()).enumerate() {
        if a != b {
            return Some(format!("第 {} 行\n    入库的：{a}\n    生成的：{b}", i + 1));
        }
    }
    if want.lines().count() != got.lines().count() {
        return Some(format!(
            "行数不同（入库 {} 行 / 生成 {} 行）",
            want.lines().count(),
            got.lines().count()
        ));
    }
    (want != got).then(|| "只有行尾或结尾换行不同".to_string())
}

/// 入库目录里有没有配方不认的 `.toml`。
///
/// 机型改名之后，老产物会留在那儿被 `include_str!` 带走 —— 那种错在配方里看不出来。
fn strays(dir: &Path, expected: &[String]) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".toml") && !expected.contains(n))
        .collect();
    out.sort();
    out
}

/// 渲染全部组合并与入库产物逐字节比。**只读**，一个字节都不写。
pub fn check_all(recipe: &Recipe, dir: &Path) -> Result<CheckReport, String> {
    let mut checked = 0usize;
    let mut expected = Vec::new();

    for (machine, variant) in recipe.combos() {
        let text = render(recipe, &machine, &variant)
            .map_err(|e| format!("{machine}:{variant} 渲染失败：{e}"))?;
        let name = file_name(&machine, &variant);
        expected.push(name.clone());
        // **产物缺一份不是「跑不动」，它是一种差异**（spec `time-machine-and-delete` §0）：
        // 刚在工作台里加完一台机型时，它本来就还没有产物 —— 那是正常状态，
        // 报成错误会让状态条留着上一次的绿字，屏幕上的数字就开始骗人。
        let Ok(on_disk) = std::fs::read_to_string(dir.join(&name)) else {
            return Ok(CheckReport {
                checked,
                first_diff: Some(format!(
                    "配方里有 {machine}:{variant}，入库产物里还没有 {name} —— \
                     跑 `--write`，或者在工作台点「重新生成入库产物」"
                )),
            });
        };
        if let Some(where_) = first_diff(&on_disk, &text) {
            return Ok(CheckReport {
                checked,
                first_diff: Some(format!("{name} 与配方生成的结果不同 —— {where_}")),
            });
        }
        checked += 1;
    }

    let strays = strays(dir, &expected);
    if !strays.is_empty() {
        return Ok(CheckReport {
            checked,
            first_diff: Some(format!(
                "{} 里有配方不认的产物：{strays:?} —— 机型改名了？删掉它们",
                dir.display()
            )),
        });
    }
    Ok(CheckReport {
        checked,
        first_diff: None,
    })
}

/// 渲染全部组合并写进入库目录，返回写了几份。
///
/// 多余产物**不删**：删文件的动作不该藏在「生成」里。它们由 [`check_all`] 点名，人来删。
pub fn write_all(recipe: &Recipe, dir: &Path) -> Result<usize, String> {
    let mut written = 0usize;
    std::fs::create_dir_all(dir).map_err(|e| format!("建 {} 失败：{e}", dir.display()))?;
    for (machine, variant) in recipe.combos() {
        let text = render(recipe, &machine, &variant)
            .map_err(|e| format!("{machine}:{variant} 渲染失败：{e}"))?;
        let path = dir.join(file_name(&machine, &variant));
        std::fs::write(&path, &text).map_err(|e| format!("写 {} 失败：{e}", path.display()))?;
        written += 1;
    }
    Ok(written)
}

/// 对照基线目录（`crates/postprocess/tests/fixtures/presets/`）。
///
/// **这是内核判据的输入，也是「上一次审阅通过的样子」**。翻案之后（spec `recipe-workbench` §0）
/// 它不再是真源：真源是配方，产物由配方生成，基线是产物的一份留影。
pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../postprocess/tests/fixtures/presets")
}

/// 目录里的预设按 `(机型, 变体)` 配对。
///
/// **按头字段认，不认文件名**：入库产物叫 `A1-fastv3.3.toml`，
/// 而基线那边沿用云端的名字 `A1F_260628.toml` —— 同一份预设，两个名字。
/// 按名字配对会得到「9 份都缺失」这种毫无线索的结论。
pub fn pair_by_head(dir: &Path) -> Result<BTreeMap<(String, String), PathBuf>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("读不到 {}：{e}", dir.display()))?;
    let mut out = BTreeMap::new();
    for path in entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
    {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("读不到 {}：{e}", path.display()))?;
        let machine = head_value(&text, "machine");
        let variant = head_value(&text, "variant");
        match (machine, variant) {
            (Some(m), Some(v)) => {
                if let Some(old) = out.insert((m.clone(), v.clone()), path.clone()) {
                    return Err(format!(
                        "{} 与 {} 都自称是 {m}:{v} —— 同一个机型变体有两份文件",
                        old.display(),
                        path.display()
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "{} 的头上找不到机型或变体 —— 它不像一份预设",
                    path.display()
                ));
            }
        }
    }
    Ok(out)
}

/// `# machine: A1` → `Some("A1")`。只认第一处。
///
/// **非注释行要跳过、不能中断扫描**：预设的头几行是注释，但正文里全是键值 ——
/// 在第一行非注释处 `return None` 会让「头字段找不到」变成常态（这里踩过一次）。
fn head_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix('#') else {
            continue;
        };
        if let Some(v) = rest.trim_start().strip_prefix(key)
            && let Some(v) = v.trim_start().strip_prefix(':')
        {
            return Some(v.trim().to_string());
        }
    }
    None
}

/// K-G0'：入库产物与对照基线九对九逐字节相同。
///
/// **不看配方** —— 「产物与配方一致」是 [`check_all`] 的事。这一条只回答
/// 「产物变过没有、变的那一处审阅过没有」。两条分开，红的时候才知道该修哪一头。
pub fn check_baseline(assets: &Path, fixtures: &Path) -> Result<CheckReport, String> {
    let made = pair_by_head(assets)?;
    let base = pair_by_head(fixtures)?;
    let mut checked = 0usize;

    for ((machine, variant), made_path) in &made {
        let Some(base_path) = base.get(&(machine.clone(), variant.clone())) else {
            return Ok(CheckReport {
                checked,
                first_diff: Some(format!(
                    "{machine}:{variant} 在对照基线里没有对应的那一份 —— 新机型？确认过就同步基线"
                )),
            });
        };
        let a = std::fs::read_to_string(base_path).map_err(|e| format!("读不回来：{e}"))?;
        let b = std::fs::read_to_string(made_path).map_err(|e| format!("读不回来：{e}"))?;
        if let Some(where_) = first_diff(&a, &b) {
            return Ok(CheckReport {
                checked,
                first_diff: Some(format!(
                    "{machine}:{variant}（基线 {} / 产物 {}）不同 —— {where_}",
                    base_path.file_name().unwrap_or_default().to_string_lossy(),
                    made_path.file_name().unwrap_or_default().to_string_lossy(),
                    where_ = where_
                )),
            });
        }
        checked += 1;
    }

    for (machine, variant) in base.keys() {
        if !made.contains_key(&(machine.clone(), variant.clone())) {
            return Ok(CheckReport {
                checked,
                first_diff: Some(format!(
                    "对照基线里有 {machine}:{variant}，而配方生成不出这一份 —— 机型删了？基线也该跟"
                )),
            });
        }
    }
    Ok(CheckReport {
        checked,
        first_diff: None,
    })
}

/// 把入库产物同步成对照基线，返回同步了几份。
///
/// **这是一个需要人先看过 diff 的动作**（doc §0 的 ③）：它把「现在的产物」定成
/// 「上一次审阅通过的样子」。自动化它等于把唯一的安全网拆了。
///
/// 基线那边**保留原有文件名**；新机型在基线里还没有对应文件时，按产物的名字新建一份。
pub fn sync_baseline(assets: &Path, fixtures: &Path) -> Result<usize, String> {
    let made = pair_by_head(assets)?;
    let base = pair_by_head(fixtures)?;
    let mut synced = 0usize;
    for ((machine, variant), made_path) in &made {
        let text = std::fs::read_to_string(made_path).map_err(|e| format!("读不回来：{e}"))?;
        let target = match base.get(&(machine.clone(), variant.clone())) {
            Some(p) => p.clone(),
            None => fixtures.join(file_name(machine, variant)),
        };
        if std::fs::read_to_string(&target).ok().as_deref() == Some(text.as_str()) {
            continue; // 内容相同就不写：mtime 变动会让别的判据重跑
        }
        std::fs::write(&target, &text).map_err(|e| format!("写 {} 失败：{e}", target.display()))?;
        synced += 1;
    }
    Ok(synced)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe() -> Recipe {
        let text = std::fs::read_to_string(recipe_path()).expect("仓库里的配方");
        Recipe::parse(&text).expect("配方合法")
    }

    /// 入库产物与配方一致（与 `gen-presets --check` 同一条结论，这里从库里咬一次）。
    #[test]
    fn stored_presets_match_the_recipe() {
        let report = check_all(&recipe(), &assets_dir()).expect("检查能跑完");
        assert_eq!(
            report.first_diff, None,
            "入库产物与配方生成的结果不同：{:?}",
            report.first_diff
        );
        assert_eq!(report.checked, 9, "9 个组合都要比过");
    }

    /// 写盘是幂等的：写两次、内容不变（防 uuid/时间溜进渲染路径）。
    #[test]
    fn writing_twice_changes_nothing() {
        let dir = tempfile::tempdir().expect("临时目录");
        let r = recipe();
        assert_eq!(write_all(&r, dir.path()).expect("第一次写"), 9);
        let first: Vec<String> = r
            .combos()
            .iter()
            .map(|(m, v)| std::fs::read_to_string(dir.path().join(file_name(m, v))).expect("读回"))
            .collect();
        assert_eq!(write_all(&r, dir.path()).expect("第二次写"), 9);
        for ((m, v), before) in r.combos().iter().zip(first) {
            let after = std::fs::read_to_string(dir.path().join(file_name(m, v))).expect("读回");
            assert_eq!(after, before, "{m}:{v} 两次生成的内容不同");
        }
    }

    /// 多余产物要被点名（改机型名之后最容易留下的那种）。
    #[test]
    fn a_stray_product_is_named() {
        let dir = tempfile::tempdir().expect("临时目录");
        let r = recipe();
        write_all(&r, dir.path()).expect("写产物");
        std::fs::write(dir.path().join("A9-old.toml"), "# 老产物\n").expect("造一份多余的");
        let report = check_all(&r, dir.path()).expect("检查能跑完");
        let why = report.first_diff.expect("必须点名");
        assert!(why.contains("A9-old.toml"), "实测：{why}");
    }

    /// 按头字段配对：两边**文件名不同**也要配上（产物 `A1-fastv3.3.toml` ⇄ 基线 `A1F_260628.toml`）。
    #[test]
    fn pairing_goes_by_head_not_by_file_name() {
        let made = pair_by_head(&assets_dir()).expect("产物能配对");
        let base = pair_by_head(&fixtures_dir()).expect("基线能配对");
        assert_eq!(made.len(), 9, "产物应有 9 份");
        assert_eq!(base.len(), 9, "基线应有 9 份");
        let key = ("A1".to_string(), "fastv3.3".to_string());
        let a = made.get(&key).expect("产物里有 A1:fastv3.3");
        let b = base.get(&key).expect("基线里有 A1:fastv3.3");
        assert_ne!(
            a.file_name(),
            b.file_name(),
            "这条判据的前提就是两边命名不同；名字一样了说明有人统一过命名，这个断言要重写"
        );
    }

    /// K-G0'：产物与对照基线九对九逐字节相同。
    #[test]
    fn kg0p_products_match_the_baseline() {
        let report = check_baseline(&assets_dir(), &fixtures_dir()).expect("能跑完");
        assert_eq!(
            report.first_diff, None,
            "产物与对照基线不同：{:?}",
            report.first_diff
        );
        assert_eq!(report.checked, 9, "9 对都要比过");
    }

    /// 基线里少一份 ⇒ 点名说「新机型？确认过就同步基线」，不是静默跳过。
    #[test]
    fn a_missing_baseline_is_named() {
        let assets = tempfile::tempdir().expect("临时目录");
        let base = tempfile::tempdir().expect("临时目录");
        let r = recipe();
        write_all(&r, assets.path()).expect("写产物");
        // 基线只放 8 份：抄过去之后删掉一份
        for (m, v) in r.combos() {
            let name = file_name(&m, &v);
            if name == "X1C-lite.toml" {
                continue;
            }
            std::fs::copy(assets.path().join(&name), base.path().join(&name)).expect("抄一份");
        }
        let report = check_baseline(assets.path(), base.path()).expect("能跑完");
        let why = report.first_diff.expect("必须点名");
        assert!(why.contains("X1C:lite"), "实测：{why}");
    }

    /// 同步基线：内容相同的不写（mtime 不动），改过的那一份写回去。
    #[test]
    fn syncing_the_baseline_only_writes_what_changed() {
        let assets = tempfile::tempdir().expect("临时目录");
        let base = tempfile::tempdir().expect("临时目录");
        let r = recipe();
        write_all(&r, assets.path()).expect("写产物");
        write_all(&r, base.path()).expect("基线先与产物一致");
        assert_eq!(
            sync_baseline(assets.path(), base.path()).expect("同步"),
            0,
            "两边一致时一个字节都不该写"
        );

        // 扰动基线的**一行**，不是整份文件：整份换掉会让它连头字段都没有，
        // 那时候红的是「这文件不像预设」而不是「基线与产物不同」—— 两回事。
        let victim = base.path().join("A1-standard.toml");
        let text = std::fs::read_to_string(&victim).expect("读基线");
        std::fs::write(
            &victim,
            text.replace("speed_limit = 70", "speed_limit = 71"),
        )
        .expect("扰动基线");
        assert_eq!(
            sync_baseline(assets.path(), base.path()).expect("同步"),
            1,
            "只该写回那一份"
        );
        let report = check_baseline(assets.path(), base.path()).expect("能跑完");
        assert_eq!(report.first_diff, None, "同步之后应当全等");
    }
}
