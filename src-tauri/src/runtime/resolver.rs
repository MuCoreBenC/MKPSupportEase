//! 资源寻址（SourceResolver）—— **全仓唯一被允许把「根 + 相对段」拼成地址的地方**。
//!
//! # 三张牌照（`docs/RESOURCE-ADDRESSING-ROADMAP.md` §0）
//!
//! ```text
//! 生成器：  "我把东西生产出来。"      → 唯一出口 presets/delivery/
//! Resolver："它在哪里？"             → 本模块
//! 业务层：  "我要它。"               → 只说 ResourceRef，永不拼路径
//! 下载器：  "好，我去拿。"（管道）    → Resolver 不管下载、缓存、重试、SHA
//! ```
//!
//! 业务层看不见任何"根"。`delivery`（交付根，source.json 所在目录）与
//! `filesRoot`（catalog.files 的锚，由 Manifest 声明）都是本模块的实现细节 ——
//! 对调用方只有 [`SourceResolver::resolve`] 一个入口。
//!
//! # 寻址规则声明（Source Manifest v2）
//!
//! `source.json` 不再"被猜"：它**声明**每类东西从哪个锚点找，全部**相对引用**
//! （以 Manifest 自身 URL 为基准）—— 同一份文件服务 GitHub / Gitee 两个镜像，
//! 写死任何绝对地址等于把另一个镜像的用户指回去。
//!
//! ```json
//! {
//!   "sourceSchema": 2,
//!   "catalog":   "catalog.json",
//!   "manifest":  "manifest.json",
//!   "release":   "release.json",
//!   "content":   "content/",
//!   "filesRoot": ".."
//! }
//! ```
//!
//! `filesRoot` 是**全局锚点规则**（"所有 catalog 文件的路径，从交付目录的上一层算"），
//! **不是 Entry 路径的组成部分** —— 它只在这里出现这一次；任何 `catalog.path` 里
//! 出现 `..` 仍然一律拒绝（路径穿越闸，与发布闸 ④ 同一条规矩）。
//!
//! # 云端与本地同一套逻辑模型
//!
//! `catalog.path` 一个值两用：`filesRoot + path` = 云端取哪；
//! `<appDataDir> + path` = 本地放哪（唯一路径语义，2026-10-04）。两者都由本模块说。

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::AppError;

use super::catalog::CatalogFile;

/// Source Manifest（`source.json`）的协议代次。认不得就拒绝 —— 不猜新版长什么样。
///
/// v2（2026-10-05，资源寻址改造）：从"只说 catalog 在哪"升级为**寻址规则声明**
/// （catalog / manifest / release / content / filesRoot，全相对引用）。
/// 1 代档没有 `filesRoot`，解析会当场报错 —— 那是"这份交付面太老"的如实说法。
pub const MANIFEST_SCHEMA: u32 = 2;

/// **Source Manifest**：`source.json` 的反序列化形状 + 解析期校验。
///
/// 多说一个字段都是两端要一起改的契约；少一个可选字段 = 客户端该类资源没有声明
/// （`resolve` 会如实报"未声明"，不是猜一个默认位置）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceManifest {
    pub source_schema: u32,
    /// catalog 相对引用（相对本文件所在目录）
    pub catalog: String,
    /// 发布账本相对引用；未声明 = 该交付面不提供
    pub manifest: Option<String>,
    /// 软件发布信息相对引用；未声明 = 该交付面不提供软件更新检查
    pub release: Option<String>,
    /// 目录类 JSON 的目录相对引用；未声明 = 客户端暂不读
    pub content: Option<String>,
    /// **全局锚点规则**：`catalog.files[].path` 的锚。只认 `"."`（交付根自己）
    /// 或 `".."`（交付目录的上一层，即发布根 `presets/`）。
    pub files_root: String,
}

impl SourceManifest {
    /// 解析并校验一份 `source.json` 字节。**解析期就把规则钉死**：
    /// 代次、相对引用合法性、`filesRoot` 的两个合法值 —— 坏声明进不了运行时。
    pub fn parse(bytes: &[u8]) -> Result<Self, AppError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Raw {
            source_schema: u32,
            catalog: String,
            #[serde(default)]
            manifest: Option<String>,
            #[serde(default)]
            release: Option<String>,
            #[serde(default)]
            content: Option<String>,
            files_root: String,
        }
        let raw: Raw = serde_json::from_slice(bytes).map_err(|e| {
            AppError::corrupted("Source Manifest（source.json）解析不了").with_detail(e.to_string())
        })?;
        if raw.source_schema != MANIFEST_SCHEMA {
            return Err(AppError::corrupted(format!(
                "Source Manifest 的协议代次认不了：文件是 {}，程序认 {}",
                raw.source_schema, MANIFEST_SCHEMA
            )));
        }
        let bad_rel = |what: &str, raw: &str| {
            AppError::corrupted(format!(
                "Source Manifest 里 {what} 的引用不合法：{raw:?}（要相对路径，不许 .. / 绝对 URL / 反斜杠）"
            ))
        };
        validate_rel_ref(&raw.catalog).map_err(|_| bad_rel("catalog", &raw.catalog))?;
        for (what, opt) in [
            ("manifest", &raw.manifest),
            ("release", &raw.release),
            ("content", &raw.content),
        ] {
            if let Some(rel) = opt {
                validate_rel_ref(rel).map_err(|_| bad_rel(what, rel))?;
            }
        }
        if raw.files_root != "." && raw.files_root != ".." {
            return Err(AppError::corrupted(format!(
                "Source Manifest 的 filesRoot 只认 \".\" 或 \"..\"，文件里是 {:?}",
                raw.files_root
            )));
        }
        Ok(Self {
            source_schema: raw.source_schema,
            catalog: raw.catalog,
            manifest: raw.manifest,
            release: raw.release,
            content: raw.content,
            files_root: raw.files_root,
        })
    }
}

/// 业务层的**类型化请求** —— 业务层唯一能表达的东西。
///
/// 穷尽 match 由编译器执法：新增一种资源种类，这里少一个分支编译不过，
/// 它会被**强制**走到 Resolver 面前（不可能绕出去自己拼）。
#[derive(Debug, Clone, Copy)]
pub enum ResourceRef<'a> {
    /// 目录本身（catalog.json）
    Catalog,
    /// 发布账本（manifest.json）
    Manifest,
    /// 软件发布信息（release.json）
    Release,
    /// 目录类 JSON（content/ 下，如 `machine_catalog.json`）
    Content(&'a str),
    /// 一份交付文件（预设 / BBS / 图标 / 模型……全走这一支）
    Entry(&'a CatalogFile),
}

/// Resolver 的答案：云端取哪 + 本地放哪，一次算清。
///
/// `remote` 是**不透明**的（调用方拿 `String` 去 GET，不许再拼）；
/// `local` 在 Resolver 没有本地根时为 `None`（工作台对账场景只要远端）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAddress {
    remote: Option<String>,
    local: Option<PathBuf>,
}

impl ResolvedAddress {
    /// 远端地址。`None` = 这个资源没有声明（如实，不猜）。
    pub fn remote(&self) -> Option<&str> {
        self.remote.as_deref()
    }

    /// 远端地址，没有就报"未声明"。给"必须有"的调用方（release 链）。
    pub fn remote_or(self, what: &str) -> Result<String, AppError> {
        self.remote.ok_or_else(|| {
            AppError::not_found(format!(
                "数据源没有声明{what}的地址（source.json 里没有这一项）"
            ))
        })
    }

    /// 本地落点。`None` = Resolver 没有本地根（工作台对账场景）。
    pub fn local(&self) -> Option<&Path> {
        self.local.as_deref()
    }
}

/// 资源寻址的唯一出口。
///
/// 持有一份已解析的 Source Manifest + 交付根 URL（Manifest 自身所在目录）。
/// 内部知道 `filesRoot` 怎么落成真实锚 —— **这是实现细节**，调用方只知道
/// "我要什么 → 它在哪"。
#[derive(Debug, Clone)]
pub struct SourceResolver {
    manifest: SourceManifest,
    /// 交付根（Manifest 所在目录），无尾斜杠
    delivery: String,
    /// 本地落点根（客户端 = appDataDir；`None` = 工作台对账，只要远端）
    internal_root: Option<PathBuf>,
}

impl SourceResolver {
    /// 从"Manifest 字节 + 它所在的 URL"构建。**这是客户端解析 source.json 后的唯一入口**。
    pub fn from_bootstrap(delivery_url: impl Into<String>, bytes: &[u8]) -> Result<Self, AppError> {
        let manifest = SourceManifest::parse(bytes)?;
        Ok(Self {
            manifest,
            delivery: trim_slashes(&delivery_url.into()),
            internal_root: None,
        })
    }

    /// 给 Resolver 本地落点根（客户端 = appDataDir）。链式。
    pub fn with_internal_root(mut self, root: PathBuf) -> Self {
        self.internal_root = Some(root);
        self
    }

    /// 交付根 URL（诊断用；闸⑯的对账报告要印它）
    pub fn delivery_url(&self) -> &str {
        &self.delivery
    }

    pub fn manifest(&self) -> &SourceManifest {
        &self.manifest
    }

    /// 两个锚点的真实 URL（交付根，文件根）。**`pub(crate)`**：只给发布闸⑯的对账用 ——
    /// 它要把"客户端拼出的 URL"折回仓库相对路径核对磁盘；业务层拿不到这个口
    /// （锚点是实现细节，见模块头）。
    /// 闸⑯住在 `workbench` feature 门内 —— 默认构建里这一口没人调（如实标注，不删）。
    #[cfg_attr(not(feature = "workbench"), allow(dead_code))]
    pub(crate) fn anchor_urls(&self) -> Result<(String, String), AppError> {
        Ok((self.delivery.clone(), self.files_root()?))
    }

    /// **唯一寻址入口**：业务层说"我要什么"，这里答"在哪"。
    pub fn resolve(&self, r: ResourceRef<'_>) -> Result<ResolvedAddress, AppError> {
        match r {
            ResourceRef::Catalog => {
                let remote = join(&self.delivery, &self.manifest.catalog);
                Ok(self.answer(remote, self.catalog_local()))
            }
            ResourceRef::Manifest => {
                let rel = self
                    .manifest
                    .manifest
                    .as_deref()
                    .ok_or_else(undeclared("manifest"))?;
                Ok(self.answer(join(&self.delivery, rel), None))
            }
            ResourceRef::Release => {
                let rel = self
                    .manifest
                    .release
                    .as_deref()
                    .ok_or_else(undeclared("release"))?;
                Ok(self.answer(join(&self.delivery, rel), None))
            }
            ResourceRef::Content(rel) => {
                validate_rel_ref(rel).map_err(|_| invalid_rel("content 子路径", rel))?;
                let dir = self
                    .manifest
                    .content
                    .as_deref()
                    .ok_or_else(undeclared("content"))?;
                let base = join(&self.delivery, dir);
                Ok(self.answer(join(&base, rel), None))
            }
            ResourceRef::Entry(file) => {
                validate_entry_path(&file.path)
                    .map_err(|why| AppError::corrupted(format!("catalog.path 不合法：{}", why)))?;
                let remote = join(&self.files_root()?, &file.path);
                let local = self
                    .internal_root
                    .as_ref()
                    .map(|root| root.join(&file.path));
                Ok(self.answer(remote, local))
            }
        }
    }

    /* ---------- 私有：锚点表（实现细节，出不了这个 impl） ---------- */

    /// `catalog.files[].path` 的锚 —— 由 Manifest 的 `filesRoot` 声明落成真实位置。
    fn files_root(&self) -> Result<String, AppError> {
        match self.manifest.files_root.as_str() {
            "." => Ok(self.delivery.clone()),
            // "上一层"：交付根在发布根（presets/）里面，去一段就是它
            ".." => parent_dir(&self.delivery),
            other => Err(AppError::corrupted(format!(
                "filesRoot 只认 \".\" / \"..\"，这份 Manifest 里是 {other:?}"
            ))),
        }
    }

    fn catalog_local(&self) -> Option<PathBuf> {
        // 生效目录住内部根自己的位置（OTA 落点），与远端文件名无关
        self.internal_root
            .as_ref()
            .map(|root| root.join(super::paths::CATALOG_FILE))
    }

    fn answer(&self, remote: String, local: Option<PathBuf>) -> ResolvedAddress {
        ResolvedAddress {
            remote: Some(remote),
            local,
        }
    }
}

fn undeclared(what: &'static str) -> impl Fn() -> AppError {
    move || AppError::not_found(format!("数据源没有声明 {what}（source.json 里没有这一项）"))
}

fn invalid_rel(what: &str, raw: &str) -> AppError {
    AppError::corrupted(format!(
        "Source Manifest 里 {what} 的引用不合法：{raw:?}（要相对路径，不许 .. / 绝对 URL / 反斜杠）"
    ))
}

/// 相对引用合法性（Manifest 里的 catalog/manifest/release/content）：
/// 非空、非绝对、无 `..` 段、无反斜杠。**这是声明层的闸** —— 坏声明进不了运行时。
fn validate_rel_ref(rel: &str) -> Result<(), String> {
    validate_entry_path(rel)
}

/// `catalog.path` 的合法性：非空、不以 `/` 开头、不含 `..` 段、无反斜杠、无 scheme。
///
/// `..` 在这里被**整段拒绝**（不是"解析掉"）：路径穿越是发布闸 ④ 与
/// `rejects_traversal_in_catalog_path` 的同一块禁地，寻址层再拦一道。
fn validate_entry_path(path: &str) -> Result<(), String> {
    if path.is_empty() {
        return Err("路径是空的".to_owned());
    }
    if path.starts_with('/') {
        return Err("路径是绝对地址（以 / 开头）".to_owned());
    }
    if path.contains('\\') {
        return Err("路径里有反斜杠（Windows 写法不该出现在交付面）".to_owned());
    }
    if path.split('/').any(|seg| seg == "..") {
        return Err("路径里有 .. 段 —— 交付面禁止路径穿越".to_owned());
    }
    if ["http://", "https://", "file://"]
        .iter()
        .any(|s| path.starts_with(s))
    {
        return Err("路径里写的是绝对 URL —— 交付面只收相对路径".to_owned());
    }
    Ok(())
}

/// 拼接：`base`（无尾斜杠）+ 相对段（百分号编码，保留 `/`）。
/// **模块私有** —— 全仓只许 resolver 拼地址（M4 立法后由可见性 + CI 双重把守）。
fn join(base: &str, rel: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        encode_path(rel.trim_start_matches('/'))
    )
}

/// `https://host/a/b` → `https://host/a`。只认 http(s)。
/// （manifest 所在目录的"上一层"= `filesRoot: ".."` 的落点。）
fn parent_dir(url: &str) -> Result<String, AppError> {
    let bad =
        || AppError::invalid_argument(format!("地址不像一个可退回上一级的 http(s) 地址：{url}"));
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(bad)?;
    if rest.is_empty() || rest.starts_with('/') {
        return Err(bad());
    }
    let idx = url.rfind('/').filter(|&i| i >= 8).ok_or_else(bad)?;
    let dir = url[..idx].trim_end_matches('/');
    if dir.ends_with(':') {
        return Err(bad());
    }
    Ok(dir.to_owned())
}

fn trim_slashes(url: &str) -> String {
    url.trim().trim_end_matches('/').to_owned()
}

/// 路径段的百分号编码。**保留 `/`**（路径分隔符），其余逐字节编码。
///
/// 资产文件名实测有空格（`MKPProcess A1 0.2 0.10.json`）与中文 —— 不编码的话
/// 拼出的 URL 在网络层才炸。编码只发生在拼接这一处（与旧 `join_url` 同一算法）。
fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char)
            }
            // 中文文件名也是逐字节编码（UTF-8 的百分号形式，服务端按同一套解）
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 目标布局（M3 之后）的标准 Manifest
    const DELIVERY_MANIFEST: &str = r#"{
        "sourceSchema": 2,
        "catalog": "catalog.json",
        "manifest": "manifest.json",
        "release": "release.json",
        "content": "content/",
        "filesRoot": ".."
    }"#;

    const GITHUB_DELIVERY: &str =
        "https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery";
    const GITEE_DELIVERY: &str =
        "https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery";

    fn resolver_for(delivery: &str) -> SourceResolver {
        SourceResolver::from_bootstrap(delivery, DELIVERY_MANIFEST.as_bytes())
            .expect("标准 Manifest 该解析得动")
            .with_internal_root(PathBuf::from("/appdata"))
    }

    fn mk_entry(kind: &str, path: &str, name: &str) -> CatalogFile {
        CatalogFile {
            kind: kind.to_owned(),
            file_name: name.to_owned(),
            path: path.to_owned(),
            machine_id: String::new(),
            version_id: String::new(),
            sha256: None,
            size: None,
        }
    }

    /// **Golden Fixture —— 地址一页纸**（`RESOURCE-ADDRESSING-ROADMAP.md` §6.2）。
    ///
    /// 全家族 × GitHub/Gitee × 本地落点的期望地址写成字面量。任何锚点/声明/布局
    /// 改动，第一个看到的就是这张表的可读 diff —— 错误停在合并之前，不在用户手机上。
    #[test]
    fn golden_addresses_match_the_address_table() {
        let r = resolver_for(GITHUB_DELIVERY);

        // —— MKP 预设（B 类：delivery/ 里面）——
        let addr = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "mkp_preset",
                "delivery/mkp/presets/A1-fast.toml",
                "A1-fast.toml",
            )))
            .expect("该解析得出");
        assert_eq!(
            addr.remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/mkp/presets/A1-fast.toml")
        );
        assert_eq!(
            addr.local(),
            Some(Path::new("/appdata/delivery/mkp/presets/A1-fast.toml"))
        );

        // —— BBS / 图标 / 模型（A 类：assets/ 在 delivery 外面，filesRoot ".." 跳出去）——
        let bbs = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "bbs_config",
                "assets/bbs/Process/0.2mm/MKPProcess A1 0.2 0.10.json",
                "MKPProcess A1 0.2 0.10.json",
            )))
            .expect("该解析得出");
        assert_eq!(
            bbs.remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/assets/bbs/Process/0.2mm/MKPProcess%20A1%200.2%200.10.json")
        );
        let icon = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "icon",
                "assets/icons/a1.svg",
                "a1.svg",
            )))
            .expect("该解析得出");
        assert_eq!(
            icon.remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/assets/icons/a1.svg")
        );
        let model = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "model",
                "assets/models/Precise_Calibration.3mf",
                "Precise_Calibration.3mf",
            )))
            .expect("该解析得出");
        assert_eq!(
            model.remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/assets/models/Precise_Calibration.3mf")
        );

        // —— 交付元数据（锚 = 交付根自己）——
        assert_eq!(
            r.resolve(ResourceRef::Catalog).expect("该解析得出").remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/catalog.json")
        );
        assert_eq!(
            r.resolve(ResourceRef::Manifest).expect("该解析得出").remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/manifest.json")
        );
        assert_eq!(
            r.resolve(ResourceRef::Release).expect("该解析得出").remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/release.json")
        );
        assert_eq!(
            r.resolve(ResourceRef::Content("machine_catalog.json"))
                .expect("该解析得出")
                .remote(),
            Some("https://raw.githubusercontent.com/MuCoreBenC/MKPSupportEase/main/presets/delivery/content/machine_catalog.json")
        );

        // —— Gitee 镜像：同一份 Manifest，换 host 只换 host（相对引用的意义）——
        let g = resolver_for(GITEE_DELIVERY);
        assert_eq!(
            g.resolve(ResourceRef::Entry(&mk_entry(
                "mkp_preset",
                "delivery/mkp/presets/A1-fast.toml",
                "A1-fast.toml"
            )))
            .expect("该解析得出")
            .remote(),
            Some("https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/mkp/presets/A1-fast.toml")
        );
        assert_eq!(
            g.resolve(ResourceRef::Release).expect("该解析得出").remote(),
            Some("https://gitee.com/MuCoreBenC/MKPSupportEase/raw/main/presets/delivery/release.json")
        );
    }

    /// 中文文件名逐字节编码，服务端按 UTF-8 解回
    #[test]
    fn encodes_non_ascii_but_keeps_slashes() {
        assert_eq!(
            encode_path("mkp/bbs/中文.json"),
            "mkp/bbs/%E4%B8%AD%E6%96%87.json"
        );
    }

    /// `filesRoot` 的两个合法值各落对位置
    #[test]
    fn files_root_dot_anchors_at_the_delivery_root_itself() {
        let bytes = br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":"."}"#;
        let r = SourceResolver::from_bootstrap("https://cdn.example.com/mirror/delivery", bytes)
            .expect("该解析得出")
            .with_internal_root(PathBuf::from("/appdata"));
        let addr = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "mkp_preset",
                "mkp/presets/A1-fast.toml",
                "A1-fast.toml",
            )))
            .expect("该解析得出");
        assert_eq!(
            addr.remote(),
            Some("https://cdn.example.com/mirror/delivery/mkp/presets/A1-fast.toml")
        );
    }

    /// 坏声明进不了运行时：代次 / filesRoot / 绝对 URL / 缺字段
    #[test]
    fn bad_manifests_are_refused_at_parse_time() {
        // 代次
        let e = SourceManifest::parse(
            br#"{"sourceSchema":1,"catalog":"catalog.json","filesRoot":".."}"#,
        )
        .expect_err("1 代该拒");
        assert!(e.message.contains("协议代次认不了"), "{}", e.message);
        // filesRoot 越界
        let e = SourceManifest::parse(
            br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":"../.."}"#,
        )
        .expect_err("filesRoot 只许 . / ..");
        assert!(e.message.contains("filesRoot"), "{}", e.message);
        // 交付面混进绝对 URL
        let e = SourceManifest::parse(
            br#"{"sourceSchema":2,"catalog":"https://cdn.example.com/catalog.json","filesRoot":".."}"#,
        )
        .expect_err("绝对 URL 该拒");
        assert!(e.message.contains("不合法"), "{}", e.message);
        // 缺 filesRoot（防"猜"回潮：必须显式声明）
        let e = SourceManifest::parse(br#"{"sourceSchema":2,"catalog":"catalog.json"}"#)
            .expect_err("缺 filesRoot 该拒");
        assert!(e.message.contains("解析不了"), "{}", e.message);
    }

    /// `..` 在 entry path 里被整段拒绝 —— 交付面禁止路径穿越
    #[test]
    fn traversal_in_entry_path_is_refused() {
        let r = resolver_for(GITHUB_DELIVERY);
        for bad in [
            "../assets/icons/a1.svg",
            "delivery/../../etc/passwd",
            "/absolute/path.toml",
            "delivery\\windows\\path.toml",
            "https://evil.example/x.toml",
            "",
        ] {
            let e = r
                .resolve(ResourceRef::Entry(&mk_entry("mkp_preset", bad, "x.toml")))
                .expect_err(&format!("{bad:?} 该拒"));
            assert!(
                e.message.contains("catalog.path 不合法"),
                "{bad:?}: {}",
                e.message
            );
        }
    }

    /// 未声明的资源如实报"未声明"，不猜默认位置
    #[test]
    fn undeclared_resources_report_honestly() {
        let bytes = br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":".."}"#;
        let r = SourceResolver::from_bootstrap(GITHUB_DELIVERY, bytes).expect("该解析得出");
        let e = r.resolve(ResourceRef::Release).expect_err("未声明该报");
        assert!(e.message.contains("没有声明 release"), "{}", e.message);
    }

    /// 交付根没有上一层可退时（host 根）如实拒
    #[test]
    fn files_root_dotdot_at_host_root_is_refused() {
        let r =
            SourceResolver::from_bootstrap("https://host.example", DELIVERY_MANIFEST.as_bytes())
                .expect("Manifest 本身该过");
        let e = r
            .resolve(ResourceRef::Entry(&mk_entry(
                "mkp_preset",
                "delivery/mkp/presets/A1-fast.toml",
                "A1-fast.toml",
            )))
            .expect_err("host 根退不出上一级");
        assert!(e.message.contains("可退回上一级"), "{}", e.message);
    }
}
