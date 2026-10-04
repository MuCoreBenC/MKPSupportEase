//! **版本号：唯一真值 + 单向派生**（第四刀 · 作者 2026-10-04 裁定）。
//!
//! # 一条链，不许有第二处真值
//!
//! ```text
//! src-tauri/Cargo.toml  [package].version      ← 唯一真值（人只改这一处）
//!         ├─ package.json              （派生）
//!         ├─ src-tauri/tauri.conf.json （派生 → 安装包版本）
//!         ├─ Cargo.lock                （派生：`cargo update --workspace --offline`）
//!         └─ 构建期 env!("CARGO_PKG_VERSION") → APP_VERSION → tag → release.json
//! ```
//!
//! ★ 作者的原话：**不要把"版本号四处写着"继续当设计目标**。其他地方都是**生成出来的
//! 结果**，不是让人手动维护的第二、第三、第四个真值。所以本模块只暴露"读真值"与
//! "把真值推到各处"，**没有"从别处读回来当真值"的入口**。
//!
//! # 为什么 `release.json` 的版本不在这里
//!
//! 它记的是"**已发布**出去的版本"，落后于当前开发版本是合法的（还没发的时候就是旧的）。
//! 所以它只由[`super::release_tx`]在上传成功后写，判据也不要求它与 manifest 相等 ——
//! 要求相等等于逼人"改版本号就必须立刻发版"。

use std::path::{Path, PathBuf};

use crate::error::AppError;
use crate::fsx;

/// 版本真值所在（**相对仓库根**）。★ 不是 workspace 根那份 `Cargo.toml` ——
/// 根上 `[workspace.package] version = "0.2.0"` 是给 `crates/preset` 用的，与 app 版本无关。
pub const MANIFEST_REL: &str = "src-tauri/Cargo.toml";
/// 派生物之一：npm 包名与版本（前端构建读它）
pub const PACKAGE_REL: &str = "package.json";
/// 派生物之二：Tauri 配置（**安装包版本 = 这一格**）
pub const CONF_REL: &str = "src-tauri/tauri.conf.json";
/// 派生物之三：锁文件（workspace 根）
pub const LOCK_REL: &str = "Cargo.lock";
/// `Cargo.lock` 里本 crate 的名字（找它的 version 用）
pub const LOCK_PACKAGE_NAME: &str = "mkp-support-ease";

/// 三段版本号 `x.y.z`。**只认这一种形状** —— 带 `v` 前缀、带 `-dev` 后缀的都不算
/// （它们只在"比较"里出现，不是真值那一格该有的样子）。
pub fn is_semver(v: &str) -> bool {
    let v = v.trim();
    if v.is_empty() || v.ends_with('.') || v.starts_with('.') {
        return false;
    }
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// **版本真值**：读 `src-tauri/Cargo.toml` 的 `[package].version`。
///
/// 用 `toml_edit` 而不是正则：manifest 里 `version` 出现不止一次（成员表、依赖表里也有），
/// 正则会改错地方 —— 那条教训在 `scripts/release.mjs` 里已经踩过一次。
pub fn app_version(repo_root: &Path) -> Result<String, AppError> {
    let text = read_text(repo_root, MANIFEST_REL)?;
    let doc = text.parse::<toml_edit::DocumentMut>().map_err(|e| {
        AppError::corrupted("读不了 src-tauri/Cargo.toml（版本号真值所在）")
            .with_detail(e.to_string())
    })?;
    let v = doc
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::corrupted("src-tauri/Cargo.toml 里没有 [package].version"))?;
    let v = v.trim().to_owned();
    if !is_semver(&v) {
        return Err(AppError::corrupted(format!("版本真值不是 x.y.z 形状：{v}")));
    }
    Ok(v)
}

/// 派生物之一：`package.json` 的 `version`。
pub fn package_version(repo_root: &Path) -> Result<String, AppError> {
    let v: serde_json::Value = serde_json::from_slice(&read(repo_root, PACKAGE_REL)?)
        .map_err(|e| AppError::corrupted("package.json 解析不了").with_detail(e.to_string()))?;
    v.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .ok_or_else(|| AppError::corrupted("package.json 里没有 version"))
}

/// 派生物之二：`tauri.conf.json` 的 `version`（**安装包版本就是它**）。
pub fn conf_version(repo_root: &Path) -> Result<String, AppError> {
    let v: serde_json::Value =
        serde_json::from_slice(&read(repo_root, CONF_REL)?).map_err(|e| {
            AppError::corrupted("src-tauri/tauri.conf.json 解析不了").with_detail(e.to_string())
        })?;
    v.get("version")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_owned())
        .ok_or_else(|| AppError::corrupted("src-tauri/tauri.conf.json 里没有 version"))
}

/// 派生物之三：`Cargo.lock` 里本 crate 的版本。`None` = lock 里没有这一条（不该发生）。
pub fn lock_version(repo_root: &Path) -> Result<Option<String>, AppError> {
    let text = read_text(repo_root, LOCK_REL)?;
    let doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| AppError::corrupted("Cargo.lock 解析不了").with_detail(e.to_string()))?;
    // `[[package]]` 是**表数组**（`ArrayOfTables`）：按它取，每一项就是一张 `Table`
    let Some(packages) = doc.get("package").and_then(|p| p.as_array_of_tables()) else {
        return Ok(None);
    };
    for table in packages.iter() {
        if table.get("name").and_then(|n| n.as_str()) == Some(LOCK_PACKAGE_NAME) {
            return Ok(table
                .get("version")
                .and_then(|v| v.as_str())
                .map(str::to_owned));
        }
    }
    Ok(None)
}

/// **一致性检查**：真值与三个派生物里"对不上的那几个"（相对仓库根的路径）。
///
/// 空 = 全部一致。给界面（闸的一格）与判据用**同一个函数** —— 与发布闸同一条纪律。
/// ★ 只**读**，不写：它是判据，跑它不许动工作区一个字节。
pub fn derived_mismatch(repo_root: &Path) -> Result<Vec<String>, AppError> {
    let truth = app_version(repo_root)?;
    let mut bad = Vec::new();
    if package_version(repo_root)? != truth {
        bad.push(PACKAGE_REL.to_owned());
    }
    if conf_version(repo_root)? != truth {
        bad.push(CONF_REL.to_owned());
    }
    match lock_version(repo_root)? {
        Some(v) if v == truth => {}
        // lock 里没有这一条 = 还没刷过 —— 也算对不上（下一次任何人跑 cargo 都会改写它）
        _ => bad.push(LOCK_REL.to_owned()),
    }
    Ok(bad)
}

/// **把真值推到三个派生物**（纯文件操作，**不跑 cargo**）。返回**确实改写了的**文件清单。
///
/// 第三个是 `Cargo.lock`：它也记着本 crate 的版本号。漏掉的后果是下一次任何人跑 cargo
/// 都会被它自动改写 —— 工作区凭空变脏，而入库的 lock 与 manifest 不一致
/// （v0.0.1 那次就是这么漏的，教训记在 `scripts/release.mjs` 里）。
///
/// ★ **为什么是改文件而不是 `cargo update --workspace`**（与 `scripts/release.mjs` 的老做法不同）：
/// 那条命令是子进程，要 cargo 在 PATH 上、要真 workspace 在场 —— 而"派生"这件事
/// 本身只是"改一行"。做成纯文件操作之后，判据可以在临时目录里验它，
/// 事务也不会因为某个环境里没有 cargo 就整条链走不下去。
/// （真仓库上想再让 cargo 自己刷一遍，用 [`refresh_lock`]。）
///
/// ★ 只改 `version` 那一格：两个 JSON 用 `serde_json::Value` 读回再写，
/// `preserve_order` 已开（见根 `Cargo.toml`）⇒ 其余键与键序原样保留。
pub fn write_derived(repo_root: &Path, version: &str) -> Result<Vec<String>, AppError> {
    if !is_semver(version) {
        return Err(AppError::invalid_argument(format!(
            "版本号不是 x.y.z 形状：{version}"
        )));
    }
    let mut changed = Vec::new();
    for rel in [PACKAGE_REL, CONF_REL] {
        let path = repo_root.join(rel);
        let bytes = read(repo_root, rel)?;
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| {
            AppError::corrupted(format!("{rel} 解析不了")).with_detail(e.to_string())
        })?;
        let slot = value
            .get_mut("version")
            .ok_or_else(|| AppError::corrupted(format!("{rel} 里没有 version 那一格")))?;
        let old = slot.as_str().unwrap_or_default().to_owned();
        if old == version {
            continue;
        }
        *slot = serde_json::Value::String(version.to_owned());
        let out = serde_json::to_string_pretty(&value).map_err(|e| {
            AppError::internal(format!("{rel} 序列化失败")).with_detail(e.to_string())
        })? + "\n";
        fsx::atomic::atomic_write(&path, out.as_bytes())?;
        changed.push(rel.to_owned());
    }
    changed.extend(write_lock(repo_root, version)?);
    Ok(changed)
}

/// 把 `Cargo.lock` 里**本 crate** 那一行的 version 改掉。
///
/// 只改那一行（`toml_edit` 按结构定位），其余包与整个文件的格式原样保留 ——
/// 手写正则改 lock 会碰到几十个同名 `version` 键，那是这条路最典型的翻车方式。
/// lock 不在（不该发生）→ 如实报；**没有本 crate 那一条 → 也如实报**（不静默跳过：
/// 那意味着 lock 与 manifest 已经对不上，正是这一刀要消灭的状态）。
fn write_lock(repo_root: &Path, version: &str) -> Result<Vec<String>, AppError> {
    let text = read_text(repo_root, LOCK_REL)?;
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| AppError::corrupted("Cargo.lock 解析不了").with_detail(e.to_string()))?;
    // 先**只读**地定位（下标），再动它 —— 一边迭代一边拿整份 doc 去序列化会撞借用
    let index = doc
        .get("package")
        .and_then(|p| p.as_array_of_tables())
        .and_then(|a| {
            a.iter()
                .position(|t| t.get("name").and_then(|n| n.as_str()) == Some(LOCK_PACKAGE_NAME))
        });
    let Some(index) = index else {
        return Err(AppError::corrupted(format!(
            "Cargo.lock 里没有 {LOCK_PACKAGE_NAME} 这一条 —— 它和 manifest 对不上了"
        )));
    };
    let packages = doc
        .get_mut("package")
        .and_then(|p| p.as_array_of_tables_mut())
        .ok_or_else(|| AppError::corrupted("Cargo.lock 的 [[package]] 取不出来"))?;
    let table = packages
        .get_mut(index)
        .ok_or_else(|| AppError::corrupted(format!("Cargo.lock 的第 {index} 条包取不出来")))?;
    let old = table
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    if old == version {
        return Ok(Vec::new());
    }
    table["version"] = toml_edit::value(version);
    let path = repo_root.join(LOCK_REL);
    fsx::atomic::atomic_write(&path, doc.to_string().as_bytes())?;
    Ok(vec![LOCK_REL.to_owned()])
}

/// **刷新 `Cargo.lock`**（`cargo update --workspace --offline`）。
///
/// 它也记着本 crate 的版本号。漏掉的后果是下一次任何人跑 cargo 都会被它自动改写 ——
/// 工作区凭空变脏，而入库的 lock 与 manifest 不一致（v0.0.1 那次就是这么漏的，
/// 教训记在 `scripts/release.mjs` 里）。
/// `--offline`：只刷新本 workspace 的条目，不去网络升级依赖。
pub fn refresh_lock(repo_root: &Path) -> Result<(), AppError> {
    let out = std::process::Command::new("cargo")
        .args(["update", "--workspace", "--offline"])
        .current_dir(repo_root)
        .output()
        .map_err(|e| {
            AppError::io("起不了 cargo —— 它可能不在 PATH 上").with_detail(e.to_string())
        })?;
    if !out.status.success() {
        return Err(AppError::io("刷新 Cargo.lock 失败")
            .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned()));
    }
    Ok(())
}

/// **推进版本号**：改真值那一格 → 推到三个派生。返回改动的文件清单（含 manifest）。
///
/// ★ `next` 必须**严格新于**当前版本（相等不许写，回退更不许）——
/// 版本号只往前走，"原地重写一遍"没有意义还平白弄脏工作区。
pub fn bump(repo_root: &Path, next: &str) -> Result<Vec<String>, AppError> {
    let next = next.trim();
    if !is_semver(next) {
        return Err(AppError::invalid_argument(format!(
            "版本号不是 x.y.z 形状：{next}"
        )));
    }
    let current = app_version(repo_root)?;
    if next == current {
        return Err(AppError::invalid_argument(format!(
            "版本号已经是 {current} —— 换一个再发"
        )));
    }
    if !crate::runtime::structure::version_at_least(next, &current) {
        return Err(AppError::invalid_argument(format!(
            "版本号只能往前走：{current} → {next} 是回退"
        )));
    }

    let path = repo_root.join(MANIFEST_REL);
    let text = read_text(repo_root, MANIFEST_REL)?;
    let mut doc = text.parse::<toml_edit::DocumentMut>().map_err(|e| {
        AppError::corrupted("改不了 src-tauri/Cargo.toml").with_detail(e.to_string())
    })?;
    // ★ 只改 `[package]` 表里的那一格（`toml_edit` 按结构定位，不会碰到依赖表里的 version）
    doc["package"]["version"] = toml_edit::value(next);
    fsx::atomic::atomic_write(&path, doc.to_string().as_bytes())?;

    let mut changed = vec![MANIFEST_REL.to_owned()];
    changed.extend(write_derived(repo_root, next)?);
    Ok(changed)
}

/// 读仓库根下的一个文件。**只读** —— 本模块唯一的写是 [`write_derived`] / [`bump`]。
fn read(repo_root: &Path, rel: &str) -> Result<Vec<u8>, AppError> {
    let path: PathBuf = repo_root.join(rel);
    std::fs::read(&path).map_err(|e| {
        AppError::io(format!("读不了 {rel}")).with_detail(format!("{}：{e}", path.display()))
    })
}

fn read_text(repo_root: &Path, rel: &str) -> Result<String, AppError> {
    String::from_utf8(read(repo_root, rel)?)
        .map_err(|_| AppError::corrupted(format!("{rel} 不是 UTF-8")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ★ **唯一真值**：真仓库的 manifest 与三处派生、以及构建期 `APP_VERSION` 全部同源。
    /// 这条判据存在的理由就是作者那句"不要再让四处都写着版本号" —— 不一致即红。
    ///
    /// ★★ 只**读**。这条跑在真仓库里，写一下就把工作区弄脏了。
    #[test]
    fn the_app_version_has_one_source_of_truth() {
        let root = crate::workbench::paths::repo_root();
        let truth = app_version(&root).expect("读得到版本真值");
        assert!(is_semver(&truth), "真值该是 x.y.z 形状：{truth}");

        assert_eq!(
            package_version(&root).expect("package.json"),
            truth,
            "package.json 是派生，该跟着真值走"
        );
        assert_eq!(
            conf_version(&root).expect("tauri.conf.json"),
            truth,
            "tauri.conf.json 是派生（安装包版本），该跟着真值走"
        );
        assert_eq!(
            lock_version(&root).expect("Cargo.lock"),
            Some(truth.clone()),
            "Cargo.lock 是派生，该跟着真值走"
        );
        // 构建期常量同源：它是 APP_VERSION 的唯一来源
        assert_eq!(crate::runtime::structure::APP_VERSION, truth);
        assert!(
            derived_mismatch(&root).expect("一致性检查").is_empty(),
            "真仓库该是四處一致的"
        );
    }

    /// 形状判定：只认 `x.y.z`，带 v / 带后缀 / 少一段都不算。
    #[test]
    fn semver_shape_is_three_numeric_segments() {
        for ok in ["0.0.1", "1.10.200", "0.0.2"] {
            assert!(is_semver(ok), "{ok} 该认");
        }
        for no in ["", "v0.0.1", "0.0", "0.0.1-dev", "0.0.a", ".0.1", "0.0."] {
            assert!(!is_semver(no), "{no} 不该认");
        }
    }

    /// **派生是纯文件操作**（临时仓库里验）：改完 manifest 再 `write_derived`，
    /// 两个 JSON 与 lock 那一行跟着走，且**只动 version 那一格**（`productName` 等原样保留）。
    #[test]
    fn writing_the_derived_files_only_touches_the_version_slot() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(PACKAGE_REL),
            br#"{
  "name": "mkp-support-ease",
  "private": true,
  "version": "0.0.1"
}"#,
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(CONF_REL),
            br#"{
  "productName": "SupportEase",
  "version": "0.0.1"
}"#,
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(LOCK_REL),
            br#"[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "mkp-support-ease"
version = "0.0.1"
"#,
        )
        .unwrap();

        let changed = write_derived(root, "0.0.2").expect("推派生");
        assert_eq!(changed.len(), 3, "三处都该改写：{changed:?}");
        assert_eq!(package_version(root).unwrap(), "0.0.2");
        assert_eq!(conf_version(root).unwrap(), "0.0.2");
        assert_eq!(lock_version(root).unwrap(), Some("0.0.2".to_owned()));
        // lock 里**别的包**一行都没动
        let lock = std::fs::read_to_string(root.join(LOCK_REL)).unwrap();
        assert!(
            lock.contains("name = \"serde\"\nversion = \"1.0.0\""),
            "别的包原样：{lock}"
        );

        // 别的键一个都没动
        let pkg = std::fs::read_to_string(root.join(PACKAGE_REL)).unwrap();
        assert!(
            pkg.contains("\"name\": \"mkp-support-ease\""),
            "键序与值原样：{pkg}"
        );
        let conf = std::fs::read_to_string(root.join(CONF_REL)).unwrap();
        assert!(
            conf.contains("\"productName\": \"SupportEase\""),
            "值原样：{conf}"
        );

        // 再写一次同样的版本 = 没有改动（不弄脏工作区）
        let again = write_derived(root, "0.0.2").expect("再推一次");
        assert!(again.is_empty(), "值没变就不该重写：{again:?}");
    }

    /// 不写不是版本的字符串：`write_derived` 是"派生的唯一入口"，它自己要先挡住。
    #[test]
    fn a_non_semver_version_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_derived(dir.path(), "v1.0").is_err());
        assert!(write_derived(dir.path(), "next").is_err());
    }

    /// ★ **只改一处，其余跟着来**：`bump` 一次调用之后，真值与三个派生同时到位
    /// （作者要的"改一次 `0.0.1 → 0.0.2`，其他自动跟着变"）。
    #[test]
    fn one_bump_moves_the_truth_and_every_derived_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(MANIFEST_REL),
            br#"[package]
name = "mkp-support-ease"
version = "0.0.1"

[dependencies]
tauri = { version = "2.9.1" }
"#,
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(PACKAGE_REL), b"{\"version\":\"0.0.1\"}")
            .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(CONF_REL), b"{\"version\":\"0.0.1\"}").unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(LOCK_REL),
            b"[[package]]\nname = \"mkp-support-ease\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();

        let changed = bump(root, "0.0.2").expect("推进");
        assert_eq!(
            changed,
            vec![
                MANIFEST_REL.to_owned(),
                PACKAGE_REL.to_owned(),
                CONF_REL.to_owned(),
                LOCK_REL.to_owned()
            ],
            "四处一起走：{changed:?}"
        );
        assert!(derived_mismatch(root).unwrap().is_empty(), "推完就该一致");
        // 依赖表里的 version 原样（真值只认 [package] 那一格）
        let manifest = std::fs::read_to_string(root.join(MANIFEST_REL)).unwrap();
        assert!(
            manifest.contains("tauri = { version = \"2.9.1\" }"),
            "依赖表不许被碰：{manifest}"
        );
    }

    /// `bump` 只往前走：相等与回退都要被拒（回退发布会把客户端的版本比较带进死胡同）。
    #[test]
    fn bump_only_moves_forward() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(MANIFEST_REL),
            br#"[package]
name = "mkp-support-ease"
version = "0.0.1"
"#,
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(PACKAGE_REL), b"{\"version\":\"0.0.1\"}")
            .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(CONF_REL), b"{\"version\":\"0.0.1\"}").unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(LOCK_REL),
            b"[[package]]\nname = \"mkp-support-ease\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();

        assert!(bump(root, "0.0.1").is_err(), "相等不许写");
        assert!(bump(root, "0.0.0").is_err(), "回退不许写");
        assert!(bump(root, "0.1").is_err(), "形状不对不许写");
        // 上面几次失败之后，真值一格都没被改（**在写盘之前**就被挡住了）
        assert_eq!(app_version(root).unwrap(), "0.0.1", "失败不留痕");
    }

    /// ★ 真值只认 `[package].version`：依赖表里的 `version` 不许被"改版本号"碰掉。
    /// 这条是 `scripts/release.mjs` 用正则踩过的坑（`^version = ` 只碰第一行，
    /// 换个 manifest 写法就改错地方）—— `toml_edit` 按结构定位，构造上不会再犯。
    #[test]
    fn the_truth_slot_is_the_package_table_not_any_version_line() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        let manifest = br#"[package]
name = "mkp-support-ease"
version = "0.0.1"

[dependencies]
tauri = { version = "2.9.1" }

[build-dependencies]
serde_json.workspace = true
"#;
        crate::fsx::atomic::atomic_write(&root.join(MANIFEST_REL), manifest).unwrap();
        assert_eq!(app_version(root).unwrap(), "0.0.1");

        // 手动复刻 bump 里改真值那一格的做法（bump 整体要跑 cargo，这里只验定位）
        let mut doc = std::fs::read_to_string(root.join(MANIFEST_REL))
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        doc["package"]["version"] = toml_edit::value("9.9.9");
        let out = doc.to_string();
        assert!(out.contains("version = \"9.9.9\""), "真值那一格改了");
        assert!(
            out.contains("tauri = { version = \"2.9.1\" }"),
            "依赖表里的 version 一个字都不许动：{out}"
        );
    }

    /// `Cargo.lock` 里找的是**本 crate** 那一条，不是别的包。
    #[test]
    fn lock_version_finds_our_own_package() {
        let dir = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write(
            &dir.path().join(LOCK_REL),
            br#"# This file is automatically @generated by Cargo.
version = 4

[[package]]
name = "serde"
version = "1.0.0"

[[package]]
name = "mkp-support-ease"
version = "0.0.1"
"#,
        )
        .unwrap();
        assert_eq!(lock_version(dir.path()).unwrap(), Some("0.0.1".to_owned()));
    }

    /// 一致性检查本身：临时仓库里造一处不一致，它要指名是哪一个文件。
    #[test]
    fn a_mismatch_names_the_offending_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src-tauri")).unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(MANIFEST_REL),
            b"[package]\nname = \"mkp-support-ease\"\nversion = \"0.0.2\"\n",
        )
        .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(PACKAGE_REL), b"{\"version\":\"0.0.1\"}")
            .unwrap();
        crate::fsx::atomic::atomic_write(&root.join(CONF_REL), b"{\"version\":\"0.0.2\"}").unwrap();
        crate::fsx::atomic::atomic_write(
            &root.join(LOCK_REL),
            b"[[package]]\nname = \"mkp-support-ease\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();

        let bad = derived_mismatch(root).expect("检查");
        assert_eq!(
            bad,
            vec![PACKAGE_REL.to_owned(), LOCK_REL.to_owned()],
            "该点名这两个"
        );
    }
}
