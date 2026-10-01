//! 运行时说明书（catalog）：有哪些机型版本、每份交付文件叫什么、SHA 是多少、下载地址在哪。
//!
//! # 它是谁的产物
//!
//! 发布构建（`cargo run --bin gen-catalog`）从**层①**（`<repo>/presets` 的源 TOML +
//! `presets/dist/presets/mkp` 的交付产物真字节）构建出来，落成
//! `src/runtime/catalog.generated.json` 编进二进制（**层②**），首启释放进
//! `<appDataDir>/catalog.json`（**层③**）。层③的运行时只读释放下来的那一份。
//!
//! # 第一圈的边界（刻意的少）
//!
//! 只有机型 / 版本 / MKP 预设文件三样。预设外的资产（图标、3mf、BBS 配置）、云端地址、
//! 状态位都**还没进**——Catalog 是逐渐长出来的（见 DATA-INVENTORY §4 的收口顺序），
//! 不是一步设计一个巨大 JSON。[`CATALOG_SCHEMA`] 表达格式代次：将来加字段不升号，
//! 改语义才升。
//!
//! # revision 是什么
//!
//! 对 machines + files 的稳定序列化取的 SHA256 前 16 位 ——「这份目录描述的输入
//! 和上次是不是同一份」。它**不是**完整性校验（那是文件条目里每个 `sha256` 的事）。
//! 刻意没有 `generatedAt`：没有可信时间源之前不编一个上去（与 ClientDataPackage
//! 同一条裁决）。

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

/// 目录格式的代次。读到的文件比这新 → `CORRUPTED`（程序老）；比这旧同理（文件是旧程序写的，
/// 这一版还没有那种文件，判据先立着）
pub const CATALOG_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub catalog_schema: u32,
    /// 目录指纹。源或交付产物变了它就变
    pub revision: String,
    pub machines: Vec<CatalogMachine>,
    pub files: Vec<CatalogFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogMachine {
    pub id: String,
    pub display: String,
    pub brand: String,
    pub versions: Vec<CatalogVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogVersion {
    pub id: String,
    pub name: String,
}

/// 一份交付文件。`path` 是相对**内部根**的落点 —— 下载它就该落到那（铁律 3：
/// 没下载就没有；下载了才出现在 `mkp/`）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogFile {
    /// 第一圈只有 `mkp_preset` 一种
    pub kind: String,
    pub file_name: String,
    pub path: String,
    pub machine_id: String,
    pub version_id: String,
    /// 对**交付产物真字节**算的 —— 不是对源 TOML。下载后的校验（产品规则 §10）拿它当期望值
    pub sha256: String,
    pub size: u64,
}

impl Catalog {
    pub fn parse(bytes: &[u8]) -> Result<Catalog, AppError> {
        let catalog: Catalog = serde_json::from_slice(bytes)
            .map_err(|e| AppError::corrupted("catalog 读不出来").with_detail(e.to_string()))?;
        if catalog.catalog_schema != CATALOG_SCHEMA {
            return Err(AppError::corrupted(format!(
                "catalog 格式代次认不了：文件是 {}，程序认 {}",
                catalog.catalog_schema, CATALOG_SCHEMA
            )));
        }
        Ok(catalog)
    }

    /// 产物形态：pretty JSON + 结尾换行（进仓库当判据资产，diff 要能看）
    pub fn to_pretty_json(&self) -> Result<String, AppError> {
        let mut s = serde_json::to_string_pretty(self)
            .map_err(|e| AppError::internal("catalog 序列化失败").with_detail(e.to_string()))?;
        s.push('\n');
        Ok(s)
    }
}

impl Catalog {
    /// 从**层①**构建：`<repo>/presets`（源定义）+ `crates/preset/assets/presets`（交付产物真字节）。
    ///
    /// 产物字节取**入库**的那一份（`BUILTIN_PRESETS` 编进二进制的同一批文件）——
    /// `presets/dist/` 是本机 gitignore 掉的暂存，进不了 CI，不能当判据输入。
    /// 每个机型版本都必须配齐产物，缺一份就失败 —— 第一圈宁可红着，不让目录里出现
    /// 「版本在、文件没有」这种静默的坑（那正是要收掉的旧账）。
    pub fn build_from_repo(repo_root: &Path) -> Result<Catalog, AppError> {
        let presets = crate::presetdata::Presets::load_from(&repo_root.join("presets"))?;
        let assets = repo_root
            .join("crates")
            .join("preset")
            .join("assets")
            .join("presets");
        Self::build_from_presets(&presets, &assets)
    }

    /// 从**已加载的预设源 + 一个产物目录**构建。这是两端共用的构建本体：
    /// - 安装包侧（[`Catalog::build_from_repo`]）：产物目录 = 入库产物，**严格**——缺一份就失败；
    /// - 发布侧（工作台 `wb_publish`）：产物目录 = `dist/presets/mkp`，**宽松**——
    ///   没有产物的版本是合法状态（交付集合本来就不含它），跳过。
    pub fn build_from_presets(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Result<Catalog, AppError> {
        let (catalog, missing) = Self::collect(presets, artifacts_dir);
        if let Some(first) = missing.first() {
            return Err(AppError::not_found(format!(
                "{first} —— 入库产物目录里没有这一份，先补齐再构建目录"
            )));
        }
        Ok(catalog.finalize())
    }

    /// 宽松版：没有产物的版本合法，只登记真实存在的产物。工作台发布 `dist/catalog.json` 用它
    pub fn build_from_presets_lenient(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> Catalog {
        let (catalog, _) = Self::collect(presets, artifacts_dir);
        catalog.finalize()
    }

    /// 走一遍源 + 产物目录。返回目录与**缺失清单**（严格/宽松由调用方裁决）
    fn collect(
        presets: &crate::presetdata::Presets,
        artifacts_dir: &Path,
    ) -> (Catalog, Vec<String>) {
        // 品牌显示名：机型文件里写的是 id，给人看的是 brands.toml 里的名字（与 get_machines 同一条）
        let brands: HashMap<&str, &str> = presets
            .catalog
            .brands()
            .iter()
            .map(|b| (b.id.as_str(), b.name.as_str()))
            .collect();

        let mut machines = Vec::new();
        let mut files = Vec::new();
        let mut missing = Vec::new();

        for m in presets.catalog.machines() {
            machines.push(CatalogMachine {
                id: m.id.clone(),
                display: if m.display.trim().is_empty() {
                    m.id.clone()
                } else {
                    m.display.clone()
                },
                brand: brands
                    .get(m.brand.as_str())
                    .map(|s| (*s).to_owned())
                    .unwrap_or_else(|| m.brand.clone()),
                versions: m
                    .versions
                    .iter()
                    .map(|v| CatalogVersion {
                        id: v.id.clone(),
                        name: v.name.clone(),
                    })
                    .collect(),
            });

            for v in &m.versions {
                let file_name = crate::presetdata::mkp_file_name(&m.id, &v.id);
                let Ok(bytes) = std::fs::read(artifacts_dir.join(&file_name)) else {
                    missing.push(format!("{} / {}（{}）", m.id, v.id, file_name));
                    continue;
                };
                files.push(CatalogFile {
                    kind: "mkp_preset".to_owned(),
                    path: format!("mkp/{file_name}"),
                    file_name,
                    machine_id: m.id.clone(),
                    version_id: v.id.clone(),
                    sha256: hex(&Sha256::digest(&bytes)),
                    size: bytes.len() as u64,
                });
            }
        }

        (
            Catalog {
                catalog_schema: CATALOG_SCHEMA,
                revision: String::new(),
                machines,
                files,
            },
            missing,
        )
    }

    /// 指纹收口。构建路径（严格/宽松）与将来的手工组装都从这里过——
    /// revision 的算法只有这一处
    pub(crate) fn finalize(mut self) -> Catalog {
        self.revision = revision_of(&self);
        self
    }
}

/// 对 machines + files 的稳定序列化取摘要。`revision` 本身不在输入里，没有自指问题
fn revision_of(catalog: &Catalog) -> String {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Payload<'a> {
        machines: &'a [CatalogMachine],
        files: &'a [CatalogFile],
    }
    let bytes = serde_json::to_vec(&Payload {
        machines: &catalog.machines,
        files: &catalog.files,
    })
    .unwrap_or_default();
    hex(&Sha256::digest(&bytes))[..16].to_owned()
}

/// SHA256 摘要的 hex 字符串。构建器算指纹、delivery 校验落盘字节，共用这一条实现
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 真仓库的源 + 入库产物 → 真目录。与 `embedded_matches_rebuild` 共用这条输入
    fn repo_root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    #[test]
    fn builds_five_machines_and_nine_files() {
        let catalog = Catalog::build_from_repo(&repo_root()).expect("构建不该失败");
        assert_eq!(catalog.machines.len(), 5, "5 台机型");
        assert_eq!(
            catalog
                .machines
                .iter()
                .map(|m| m.versions.len())
                .sum::<usize>(),
            9,
            "9 个版本，每个版本一份交付产物"
        );
        assert_eq!(catalog.files.len(), 9);
        assert_eq!(catalog.revision.len(), 16, "指纹取 16 位");

        // 文件条目与命名规则对得上：A1 + FASTV3.3 → A1-fastv3.3.toml，落在 mkp/ 下
        let a1_fast = catalog
            .files
            .iter()
            .find(|f| f.machine_id == "A1" && f.version_id == "FASTV3.3")
            .expect("A1/FASTV3.3 该有交付产物");
        assert_eq!(a1_fast.file_name, "A1-fastv3.3.toml");
        assert_eq!(a1_fast.path, "mkp/A1-fastv3.3.toml");
        assert_eq!(a1_fast.sha256.len(), 64, "SHA256 的 hex 长度");
        assert!(a1_fast.size > 0);
    }

    #[test]
    fn revision_tracks_the_inputs() {
        let mut a = Catalog::build_from_repo(&repo_root()).unwrap();
        let b = Catalog::build_from_repo(&repo_root()).unwrap();
        assert_eq!(a.revision, b.revision, "同样的输入，指纹必须一样");

        a.files[0].sha256 = "0".repeat(64);
        a.revision = revision_of(&a);
        assert_ne!(a.revision, b.revision, "文件字节变了，指纹得跟着变");
    }

    /// 格式代次不认就拒 —— 往前与往后都不许静默错读
    #[test]
    fn parse_rejects_a_future_schema() {
        let json = r#"{ "catalogSchema": 99, "revision": "x", "machines": [], "files": [] }"#;
        let e = Catalog::parse(json.as_bytes()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }
}
