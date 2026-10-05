//! 「当前用哪个远端」——这份设置的读写与校验。
//!
//! # 为什么单独一个小模块
//!
//! 第二圈把下载从"本地目录源"换成真远端源之后，**远端的地址是谁、写在哪**成了一个
//! 必须有人管的持久状态。它不属于 [`super::delivery`]（那是管道，不管地址从哪来），
//! 也不属于 [`super::net`]（那是取字节的动作）——这里只回答一件事：
//! **这一台机器上，云端在哪**。
//!
//! # 为什么是这个形状
//!
//! 地址**不写进 catalog**：`presets/` 是内容源，换 Gitee 或换成自己的 CDN 是部署的事，
//! 不该为此重新发布一次说明书。于是分工变成:
//!
//! ```text
//! source.json（Source Manifest）说：寻址规则 —— catalog / manifest / release / content
//!                                    / filesRoot 各从哪个锚点找（全相对引用）
//! catalog 说：有一个文件叫 A1-standard.toml，它在交付集合里的位置是 delivery/mkp/presets/…
//! 这份设置说：当前的数据源是哪一份 source.json（官方 GitHub / 官方 Gitee / 自建）
//! 下载地址 = SourceResolver（唯一寻址出口）算，业务层不拼
//! ```
//!
//! 两半都是它们各自领域的唯一主人：Manifest 是**寻址规则声明**，Catalog 是**资源清单**
//! —— Catalog 不得自带第二套根/URL（`docs/RESOURCE-ADDRESSING-ROADMAP.md` 铁律 ⑥）。
//!
//! # 存储规矩（沿用 [`super::state`] 那一套，不另发明）
//!
//! 一种状态一个文件、住内部根 `run/` 下、带 `*Schema` 代次字段、写走 atomic_write、
//! 坏档报 `CORRUPTED` 不静默。它也**不进 localStorage**——C4 之后 localStorage 只住
//! 纯前端偏好，而这份地址最终要交给 Rust 侧去发起下载。
//!
//! # 两种入口（第十七刀起）
//!
//! "当前用哪个远端"有两个来源，语义**故意不同**：
//!
//! ```text
//! 手动覆盖（设置页 → 高级设置）  = 交付目录或 source.json 地址
//!                                 （开发 / 排查通道：本地 http.server、自建镜像…）
//! 内置（构建期注入）            = 一个 Bootstrap 地址：指向 source.json，
//!                                 由它声明寻址规则
//!                                 （正式通道：换部署位置只改 source.json，客户端不重发）
//! ```
//!
//! 覆盖优先。两条路解析出来的是**同一个形状**（[`ResolvedSource`]：一个
//! [`SourceResolver`]），下游（下载 / 检查更新 / 应用更新）只从它拿地址，不各自拼 URL。
//!
//! # 没有地址时怎么办
//!
//! **如实说还没配置，不编一个 URL 出来假装能下。** 默认地址由构建方注入：
//! 环境变量 `MKPSE_PRESET_SOURCE` > 工作台配置（`workbench/bootstrap.json`，
//! 见 `src-tauri/build.rs`）—— 注入了才有默认值。正常构建内置的是**官方 Bootstrap**；
//! "没配"只该发生在开发构建（既没注入也没手动指定）。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::paths::SOURCE_FILE;
use super::resolver::{ResourceRef, SourceResolver};

/// 设置格式的代次。加字段不升号，改语义才升（与 catalog / active-preset 同一条）。
///
/// ★ **2**（2026-10-05，作者拍"两个官方源 + 收起的自定义"）：从"一个地址"改成
/// **"选哪一个源"** —— 语义变了，所以升号。
/// **读 1 代老档不是兼容层**：1 代那个 `baseUrl` 就是"用户手填的地址"，
/// 它的含义在新模型里**正好是 `custom`**（`customUrl`）。不猜、不迁移、不回填。
pub const SOURCE_SCHEMA: u32 = 2;

/// 目录（catalog）在交付面上的固定文件名（`GetPlan` 的进度名也用它）。
/// 具体位置由 Source Manifest 的 `catalog` 声明，客户端不猜。
pub const CATALOG_FILE: &str = "catalog.json";

/// **软件发布信息**（`release.json`）的固定文件名（`GetPlan` 的进度名也用它）。
///
/// ★ 它**不是预设数据**：是「有没有新版本的 SupportEase」这条**软件发布链**的信息源，
/// 与"预设数据能不能读"是两件事。具体位置由 Source Manifest 的 `release` 声明
/// （2026-10-05 起住交付根 `presets/delivery/` 里，与其他交付元数据同边界）。
pub const RELEASE_FILE: &str = "release.json";

/// 构建方可注入的默认地址（`MKPSE_PRESET_SOURCE`）—— **第十七刀起语义是 Bootstrap
/// 地址**（指向 `source.json` 的文件地址）。**变量没设就再看工作台
/// 配置**（那是 build.rs 的事，到这里时已经被合并成一个值）。
const DEFAULT_BASE_URL: Option<&str> = option_env!("MKPSE_PRESET_SOURCE");

/// **第二个内置源**（Gitee）的 Bootstrap 地址（`MKPSE_PRESET_SOURCE_GITEE`）。
///
/// ★ 为什么是两个而不是"一个地址 + 用户自己改"（作者 2026-10-05）：国内直连 Gitee、
///   直连 GitHub 要靠代理 —— **两个都是官方源，用户只管选**。
///   没注入就不出现在界面里（不硬编一个地址进来：那会把"部署在哪"这件事写死在客户端）。
const GITEE_BASE_URL: Option<&str> = option_env!("MKPSE_PRESET_SOURCE_GITEE");

/// 自定义源的**两种形状**（作者 2026-10-05 真机踩出来的）。
///
/// ```text
/// 交付目录        https://host/presets/delivery/          目录下就有 source.json
/// Bootstrap       https://host/presets/delivery/source.json  寻址规则声明
/// ```
///
/// ★ **同一个"官方地址"，用户十有八九会填后者**（他们看到的就是这个 `source.json` 地址）。
/// 自 Source Manifest v2 起**两种形状殊途同归**：最终都读 `{交付目录}/source.json`
/// （寻址规则只住这一份文件，"直接读 catalog.json"的第三种读法已废 —— 没有 Manifest
/// 就没有 `filesRoot`，客户端只能靠猜，那正是要消灭的）。形状的区别只剩
/// "用户填的是目录还是文件"，记在盘上只为省一次探测。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CustomShape {
    /// 还没探过（老档、或刚写下来还没联网）：**运行时自适应**
    #[default]
    Unset,
    /// 用户填的是**交付目录**（客户端补上 `/source.json` 再读）
    Root,
    /// 用户填的就是 `source.json` 地址（原样读）
    Bootstrap,
}

/// 用户在三个值里选哪一个（盘上就存这个）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceMode {
    /// 内置：Gitee（国内直连）
    Gitee,
    /// 内置：GitHub
    Github,
    /// 自定义（折叠在高级设置里的那一个）
    Custom,
}

impl SourceMode {
    /// 线上名（盘上写的就是它）。**认不出 → `None`**（不猜：宁可报"认不出"，
    /// 也不要悄悄落到某个源上——那会让用户以为自己在用 A、其实在用 B）。
    ///
    /// 顺带 **trim**（调用方不必各写一遍；前后空格是"复制地址"最常见的脏东西）。
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "gitee" => Some(Self::Gitee),
            "github" => Some(Self::Github),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    /// 界面上给用户看的那一句（**产品文案只有这一处**）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Gitee => "Gitee（国内直连）",
            Self::Github => "GitHub",
            Self::Custom => "自定义地址",
        }
    }

    /// 这个模式下"生效的地址"从哪来：内置看构建期注入，自定义看盘上那份。
    pub fn builtin_url(self) -> Option<&'static str> {
        match self {
            Self::Gitee => GITEE_BASE_URL,
            Self::Github => DEFAULT_BASE_URL,
            Self::Custom => None,
        }
    }
}

/// 出厂默认选哪个。
///
/// ★ 0.0.3 先是 **GitHub**（Gitee 镜像还没全部就绪，默认指过去等于把用户往取不到
///   数据的地方带）；Gitee 内容确认就绪后的下一版再翻成 [`SourceMode::Gitee`]。
pub const DEFAULT_MODE: SourceMode = SourceMode::Github;

/// 两个内置源（给界面摆两个固定选项用；没注入的那个**不出现**）。
pub fn builtin_sources() -> Vec<(SourceMode, String)> {
    [
        (SourceMode::Gitee, GITEE_BASE_URL),
        (SourceMode::Github, DEFAULT_BASE_URL),
    ]
    .into_iter()
    .filter_map(|(mode, url)| url.map(|u| (mode, u.to_owned())))
    .collect()
}

/// 当前选中的远端数据源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PresetSource {
    pub source_schema: u32,
    /// 用户选了哪一个（`gitee` / `github` / `custom`）
    pub mode: SourceMode,
    /// `mode = custom` 时的那份地址（其余模式为 `null`）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_url: Option<String>,
    /// 自定义那份地址是**哪种形状**（[`CustomShape::Unset`] = 还没探过 → 运行时自适应）
    #[serde(default)]
    pub custom_shape: CustomShape,
}

/// 当前这一台机器的"远端入口"（覆盖优先，见模块头）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceEntry {
    /// 手动指定：**用户填的那份地址** + 它是哪种形状。
    /// 开发 / 排查通道（本地 http.server、自建镜像、另一个官方源…）。
    ///
    /// ★ `shape` 是探出来的那一种（[`CustomShape::Unset`] = 还没探过 → 运行时自适应）：
    /// **同一个地址两种读法**（目录 / `source.json`）都能认，见 [`probe_custom_shape`]。
    Custom { url: String, shape: CustomShape },
    /// 构建期注入：**Bootstrap 地址** —— 指向 `source.json`，
    /// 由它声明寻址规则。正式通道
    Bootstrap { url: String },
}

/// 解析出来的"远端在哪"：一个装配好的 [`SourceResolver`]。**所有联网动作的唯一出发点**
/// （下载 / 检查更新 / 应用更新都从它拿地址，不各自拼 URL）
#[derive(Debug, Clone)]
pub struct ResolvedSource {
    pub resolver: SourceResolver,
}

impl ResolvedSource {
    /// catalog 的完整 URL（检查 / 应用更新取它）
    pub fn catalog_url(&self) -> Result<String, AppError> {
        self.resolver
            .resolve(ResourceRef::Catalog)?
            .remote_or("目录（catalog）")
    }

    /// 软件发布信息（release.json）的完整 URL —— **由 Manifest 声明**，
    /// 不再从 base"上跳一级"去猜（那是 2026-10-05 寻址改造废除的第二套推导）。
    pub fn release_url(&self) -> Result<String, AppError> {
        self.resolver
            .resolve(ResourceRef::Release)?
            .remote_or("软件发布信息（release）")
    }
}

/// 设置文件的落点：`<appDataDir>/run/preset-source.json`
pub fn source_file(root: &Path) -> PathBuf {
    root.join(SOURCE_FILE)
}

/// 读当前设置。文件不存在 → `Ok(None)`（**没配过是合法状态**）；
/// 读得出来但解析不了 / 代次认不出 → `Err(CORRUPTED)`，不把它当"没配"。
///
/// 静默吞配置等于骗人：用户明明选过 Gitee，界面却说"没配"，那是界面在编数据。
pub fn load_source(root: &Path) -> Result<Option<PresetSource>, AppError> {
    let bytes = match std::fs::read(source_file(root)) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::io("读不到数据源设置").with_detail(e.to_string())),
    };
    // ★ **1 代老档**（2026-10-05 之前只有 `baseUrl`）：它的含义**正好**是"用户手填的
    //   地址" = 新模型里的 `custom`。这不是兼容层、也不猜：那份地址本来就是用户填的。
    if let Some(old) = read_v1(&bytes)? {
        return Ok(Some(old));
    }
    let source: PresetSource = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("数据源设置读不出来").with_detail(e.to_string()))?;
    if source.source_schema != SOURCE_SCHEMA {
        return Err(AppError::corrupted(format!(
            "数据源设置的格式代次认不了：文件是 {}，程序认 {}",
            source.source_schema, SOURCE_SCHEMA
        )));
    }
    Ok(Some(source))
}

/// 1 代档（`{sourceSchema: 1, baseUrl}`）读成 2 代的自定义源。**不是 1 代 → `None`**。
fn read_v1(bytes: &[u8]) -> Result<Option<PresetSource>, AppError> {
    let Ok(v) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return Ok(None); // 连 JSON 都不是 → 交给下面的 2 代解析去报错
    };
    if v.get("sourceSchema").and_then(|x| x.as_u64()) != Some(1) {
        return Ok(None);
    }
    let Some(base) = v.get("baseUrl").and_then(|x| x.as_str()) else {
        return Err(AppError::corrupted(
            "数据源设置是老格式，但里面没有地址 —— 认不出这一份要连到哪",
        ));
    };
    Ok(Some(PresetSource {
        source_schema: SOURCE_SCHEMA,
        mode: SourceMode::Custom,
        custom_url: Some(normalize_base_url(base)?),
        // ★ 老档**没探过形状** → `Unset` → 运行时自适应（先目录后 Bootstrap）。
        //   0.0.2 之前用户手填 `source.json` 地址的那些档，就是这样被救回来的。
        custom_shape: CustomShape::Unset,
    }))
}

/// 记下"当前用这个源"。整份替换（写新盖旧）。
///
/// `mode = custom` 时地址**必须给且合法**（空地址在这里就拒：写进去等于制造一个
/// "配了但配成空"的第三种状态，下游要为它单独想一套分支）。
/// 内置两个模式**不许带地址**（带了说明调用方糊涂了 —— 内置地址是构建期注入的）。
pub fn save_source(
    root: &Path,
    mode: SourceMode,
    raw_custom_url: Option<&str>,
) -> Result<PresetSource, AppError> {
    save_source_with(root, mode, raw_custom_url, CustomShape::Unset)
}

/// [`save_source`] 的完整形状版：**形状已经探出来时**连它一起写（省掉一次网络）。
pub fn save_source_with(
    root: &Path,
    mode: SourceMode,
    raw_custom_url: Option<&str>,
    custom_shape: CustomShape,
) -> Result<PresetSource, AppError> {
    let custom_url = match mode {
        SourceMode::Custom => Some(normalize_base_url(raw_custom_url.unwrap_or_default())?),
        _ => {
            if raw_custom_url
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            {
                return Err(AppError::invalid_argument(
                    "选内置源的时候不用给地址 —— 地址是程序自带的",
                ));
            }
            None
        }
    };
    let source = PresetSource {
        source_schema: SOURCE_SCHEMA,
        mode,
        custom_url,
        custom_shape: if mode == SourceMode::Custom {
            custom_shape
        } else {
            CustomShape::Unset
        },
    };
    let json = serde_json::to_vec_pretty(&source)
        .map_err(|e| AppError::internal("数据源设置序列化失败").with_detail(e.to_string()))?;
    atomic_write(&source_file(root), &json)?;
    Ok(source)
}

/// `source.json` 的**文件**名（`RELEASE_FILE` 是 `release.json`，两者别混 —— 这个坑
/// 名字上就分不开，所以给它一个自己的常量）。
const SOURCE_FILE_NAME: &str = "source.json";

/// **取字节的注入点**（判据用假的，产品的门只用真的）。
///
/// ★ 为什么有这一层：形状判定（目录还是 `source.json`）本质是"依次试几个 URL，看哪个
///   能取到东西"。把它写成"直接调 `net::get_bytes`"的话，判据就得**开一个本地端口**
///   才能跑 —— 而客户端产品不许开端口（`check:zero-network` 那道闸会报红，2026-10-05
///   真被它抓过一次）。**注入取字节的动作，判据就能在纯内存里跑完整条判定。**
pub type FetchBytes<'a> = &'a dyn Fn(&str, &str) -> Result<Vec<u8>, AppError>;

/// 真的那份取字节（产品路径唯一入口：网络只住 `net`）。
fn real_fetch(url: &str, what: &str) -> Result<Vec<u8>, AppError> {
    crate::runtime::net::get_bytes(
        url,
        &crate::runtime::net::GetPlan::new(what),
        &crate::runtime::net::noop_tick,
    )
}

/// **这个地址可能是哪一个文件**（纯函数，判据钉它）。
///
/// ★ **用户复制来的地址通常已经带着 `source.json`**（他们从浏览器地址栏复制的就是它）
///   —— 那种情况再拼一层 `/source.json` 会拼出 `…/source.json/source.json`，永远不通
///   （2026-10-05 真机：手动指定"官方地址"反而不通，就是这一处）。
/// 所以：**原样先试**，不成再在目录上补文件名。返回值带上"命中它该记哪个形状"。
fn bootstrap_candidates(url: &str) -> Vec<(String, CustomShape)> {
    let trimmed = url.trim().trim_end_matches('/');
    let mut out: Vec<(String, CustomShape)> = Vec::new();
    if trimmed.ends_with(SOURCE_FILE_NAME) {
        out.push((trimmed.to_owned(), CustomShape::Bootstrap));
    }
    let base = normalize_base_url(trimmed).unwrap_or_else(|_| trimmed.to_owned());
    let appended = format!("{base}/{SOURCE_FILE_NAME}");
    if !out.iter().any(|(u, _)| *u == appended) {
        out.push((appended, CustomShape::Root));
    }
    out
}

/// **探一个自定义地址是哪种形状**（真联网：原样、再补文件名，各一次机会）。
/// **两条都不通** → 如实报错，并把两条各自的失败原因都摆出来（只说"不行"等于让用户猜）。
pub fn probe_custom_shape(url: &str) -> Result<(CustomShape, ResolvedSource), AppError> {
    probe_custom_shape_with(url, &real_fetch)
}

/// [`probe_custom_shape`] 的注入版（判据走它，**不开端口**）。
pub fn probe_custom_shape_with(
    url: &str,
    fetch: FetchBytes<'_>,
) -> Result<(CustomShape, ResolvedSource), AppError> {
    let mut reasons = Vec::new();
    for (candidate, shape) in bootstrap_candidates(url) {
        match parse_source_json_with(fetch, &candidate).and_then(|r| fetch_and_accept(fetch, &r)) {
            Ok(resolved) => return Ok((shape, resolved)),
            Err(e) => reasons.push(format!("{candidate}（{e}）")),
        }
    }
    Err(AppError::invalid_argument(format!(
        "这个地址下面读不到寻址规则声明（{SOURCE_FILE_NAME}）—— 试了 {} 次也不通：{}",
        reasons.len(),
        reasons.join("；")
    )))
}

/// 取一次 catalog 并确认它是**一份可用的目录**（解析过了才算）。
fn fetch_and_accept(
    fetch: FetchBytes<'_>,
    resolved: &ResolvedSource,
) -> Result<ResolvedSource, AppError> {
    let catalog_url = resolved.catalog_url()?;
    let bytes = fetch(&catalog_url, CATALOG_FILE)?;
    crate::runtime::Catalog::parse(&bytes)
        .map_err(|e| AppError::corrupted("取到的不是一份可用的预设目录").with_detail(e.message))?;
    Ok(resolved.clone())
}

/// 撤掉用户覆盖：删掉设置文件（幂等）。
///
/// 「回到内置默认」只有这一条路 —— [`save_source`] 拒绝空地址（写空 = 制造第三种状态），
/// 所以原来的出口是"用户手删文件"；设置页把那件事变成一次显式动作。
pub fn clear_source(root: &Path) -> Result<(), AppError> {
    match std::fs::remove_file(source_file(root)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::io("撤掉数据源覆盖失败").with_detail(e.to_string())),
    }
}

/// 构建期注入的默认地址（没注就是 `None`）。界面用它区分"出厂默认值"与"用户改过的"，
/// 免得读一次之后分不清当前地址是自己选的还是出厂的
pub fn builtin_default() -> Option<String> {
    DEFAULT_BASE_URL.map(str::to_owned)
}

/// **当前选的是哪一个**（没写过设置 = 出厂默认 [`DEFAULT_MODE`]）。
pub fn current_mode(root: &Path) -> Result<SourceMode, AppError> {
    Ok(load_source(root)?.map_or(DEFAULT_MODE, |s| s.mode))
}

/// 当前生效的**入口**：按选中的模式解析（内置两个走构建期注入的 Bootstrap 地址，
/// 自定义走盘上那份地址 + **记下来的形状**），内置地址一个都没注入 → 没配。
///
/// 返回 `None` 的那一路要被界面原样说出来——那是唯一诚实的答案。
pub fn current_entry(root: &Path) -> Result<Option<SourceEntry>, AppError> {
    let stored = load_source(root)?;
    let mode = stored.as_ref().map_or(DEFAULT_MODE, |s| s.mode);
    if mode == SourceMode::Custom {
        return Ok(match stored {
            Some(s) => s.custom_url.map(|url| SourceEntry::Custom {
                url,
                shape: s.custom_shape,
            }),
            // 选了自定义却没地址 = 用户删了输入框里的字。**不静默回落**到默认源：
            // 那会让界面显示"用 Gitee"、实际连的是别的地方。
            None => None,
        });
    }
    Ok(mode.builtin_url().map(|url| SourceEntry::Bootstrap {
        url: url.to_owned(),
    }))
}

/// 把入口解析成"远端在哪"。
///
/// - 自定义（[`SourceEntry::Custom`]）：**按记下来的形状**读；形状还没探过（老档）
///   就**自适应**：先当 `source.json` 文件地址，读不到再当交付目录补文件名
///   —— 这正是 0.0.2 那个"填官方地址反而不通"的修法；
/// - 内置（[`SourceEntry::Bootstrap`]）：**读 `source.json`**（一次很小的网络请求），
///   按它声明的寻址规则算 —— 部署位置变了只改它，客户端不用重发。
pub fn resolve_entry(entry: SourceEntry) -> Result<ResolvedSource, AppError> {
    match entry {
        SourceEntry::Custom { url, shape } => match shape {
            // 原样先试（用户复制来的地址通常已经带着 `source.json`）
            CustomShape::Bootstrap => parse_source_json(&url),
            // 填的是交付目录：补上文件名读
            CustomShape::Root => {
                let base = normalize_base_url(&url)?;
                parse_source_json(&format!("{base}/{SOURCE_FILE_NAME}"))
            }
            // 自适应：两条都试，报错时把两次尝试的原因都摆出来（只说"不行"等于让用户猜）
            CustomShape::Unset => {
                let mut last = None;
                for (candidate, _) in bootstrap_candidates(&url) {
                    match parse_source_json(&candidate) {
                        Ok(r) => return Ok(r),
                        Err(e) => last = Some(e),
                    }
                }
                Err(last.expect("候选表不为空（至少会补一个文件名）"))
            }
        },
        SourceEntry::Bootstrap { url } => parse_source_json(&url),
    }
}

/// 这台机器上的远端入口 → 装配好的 Resolver；**没配就报错、并且说清去哪儿配**。
pub fn resolve_source(root: &Path) -> Result<ResolvedSource, AppError> {
    let entry = current_entry(root)?.ok_or_else(|| {
        AppError::not_implemented(
            "这个构建没有内置官方源、你也没手动指定 —— 开发 / 排查时可以到「设置 → 高级设置 → 预设数据源」指定一个",
        )
    })?;
    let resolved = resolve_entry(entry)?;
    Ok(resolved.with_internal_root(root))
}

/// **探一次当前这个源**：解析入口 + 真去取一次 catalog。
///
/// ★ 它是"换源之前先看一眼"的实现（作者 2026-10-05：用户容易输错地址）：
/// **取不到就整次拒绝**，而不是"先存上再说" —— 错地址留在设置里比"没配"更难查。
/// 自定义源探的是 `{地址}/source.json` 声明的那份 catalog；内置源探的是它自己
/// 在 `source.json` 里说的那个 catalog 地址。
pub fn probe_current(root: &Path) -> Result<ResolvedSource, AppError> {
    let entry = current_entry(root)?.ok_or_else(|| {
        AppError::invalid_argument("还没有选数据源 —— 先在上面选一个官方源，或填一个地址")
    })?;
    let resolved = resolve_entry(entry)?;
    let catalog_url = resolved.catalog_url()?;
    let bytes = crate::runtime::net::get_bytes(
        &catalog_url,
        &crate::runtime::net::GetPlan::new(CATALOG_FILE),
        &crate::runtime::net::noop_tick,
    )?;
    // 解析一次：能取到但不是合法目录，那也是"这个地址不对"（用户看得懂的话）
    crate::runtime::Catalog::parse(&bytes).map_err(|e| {
        AppError::corrupted("这个地址下面不是一份可用的预设目录").with_detail(e.message)
    })?;
    Ok(resolved)
}

/// 取一个 Bootstrap（`source.json`）并解析成寻址规则。
/// **内置与自定义的 Bootstrap 形状共用这一处**（少一处就少一处不一致）。
fn parse_source_json(url: &str) -> Result<ResolvedSource, AppError> {
    parse_source_json_with(&real_fetch, url)
}

fn parse_source_json_with(fetch: FetchBytes<'_>, url: &str) -> Result<ResolvedSource, AppError> {
    let bytes = fetch(url, SOURCE_FILE_NAME)?;
    parse_manifest_at(url, &bytes)
}

/// 解析一份已取回的 Manifest 字节。**交付根 = Manifest 自身所在目录** ——
/// 这是唯一无歧义的事实，Manifest 里的相对引用都以它为基准（客户端不猜别的锚）。
pub fn parse_manifest_at(url: &str, bytes: &[u8]) -> Result<ResolvedSource, AppError> {
    let resolver = SourceResolver::from_bootstrap(directory_of(url)?, bytes)?;
    Ok(ResolvedSource { resolver })
}

/// Bootstrap 地址 → 它声明的交付根（= source.json 所在目录）。
/// 发布闸⑯用它从 `workbench/bootstrap.json` 反推"客户端会认哪个交付根"。
pub fn bootstrap_delivery_url(bootstrap_url: &str) -> Result<String, AppError> {
    directory_of(bootstrap_url)
}

/// `https://host/a/b/source.json` → `https://host/a/b`（交付根锚点的唯一来源）。
/// 只认 http(s)、scheme 后面要有 host；"https://host" 这种没有路径段的形状也拒
/// （它不是一个"文件地址"，回退不出目录）
fn directory_of(url: &str) -> Result<String, AppError> {
    let bad =
        || AppError::invalid_argument(format!("Bootstrap 地址不像一个 http(s) 文件地址：{url}"));
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or_else(bad)?;
    if rest.is_empty() || rest.starts_with('/') {
        return Err(bad());
    }
    /* scheme 双斜杠之后的第一个 '/' 才是目录的分界（8 = "https://" 的长度） */
    let idx = url.rfind('/').filter(|&i| i >= 8).ok_or_else(bad)?;
    let dir = url[..idx].trim_end_matches('/');
    if dir.ends_with(':') {
        return Err(bad());
    }
    Ok(dir.to_owned())
}

/// 把人填进来的地址收拾干净：去首尾空白、砍掉末尾多余的斜杠。
///
/// 只认 `http(s)`：这是**出站下载**用的地址。`file://`、UNC 路径这类Scheme
/// 在下载栈里会被理解成别的东西——那既是"下载能读到本机任意文件"的门，
/// 也绕过了「云端是远端」这个前提，趁它还没有第二个用途时关上。
pub fn normalize_base_url(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err(AppError::invalid_argument("数据源地址是空的"));
    }
    let ok = trimmed.starts_with("http://") || trimmed.starts_with("https://");
    if !ok {
        return Err(AppError::invalid_argument(format!(
            "数据源地址只认 http:// 或 https://，填进来的是 {trimmed}"
        )));
    }
    Ok(trimmed.to_owned())
}

impl ResolvedSource {
    /// 给 Resolver 本地落点根（客户端 = appDataDir）。
    fn with_internal_root(mut self, root: &Path) -> Self {
        self.resolver = self.resolver.with_internal_root(root.to_path_buf());
        self
    }
}

#[cfg(test)]
mod two_source_tests {
    //! **双源 + 自适应形状**的判据（2026-10-05）。
    //!
    //! 重点是最后那条：0.0.2 真机里用户把**官方 `source.json` 地址**填进「手动指定」
    //! 反而不能用（手动那条只认"数据源根"），而内置官方源同一个地址是通的 ——
    //! 那不是用户错，是我们把"一个地址有两种读法"当成了两种东西。

    use super::*;

    fn dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// 1 代老档（只有 `baseUrl`）读成**自定义源** —— 那份地址本来就是用户手填的
    #[test]
    fn the_v1_file_reads_as_a_custom_source() {
        let d = dir();
        atomic_write(
            &source_file(d.path()),
            br#"{"sourceSchema":1,"baseUrl":"https://example.com/presets/delivery/"}"#,
        )
        .unwrap();

        let s = load_source(d.path()).unwrap().expect("老档该读得出来");
        assert_eq!(s.mode, SourceMode::Custom);
        assert_eq!(
            s.custom_url.as_deref(),
            Some("https://example.com/presets/delivery")
        );
        assert_eq!(
            s.custom_shape,
            CustomShape::Unset,
            "老档没探过形状 → 运行时自适应（这才救得回填了 source.json 地址的那份）"
        );
    }

    /// 选内置源的时候不许带地址（带了说明调用方糊涂了）
    #[test]
    fn a_builtin_mode_refuses_an_address() {
        let d = dir();
        let e = save_source(d.path(), SourceMode::Github, Some("https://x.example/")).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// 选了自定义却**没地址** → 没有入口。**不许静默回落到内置源** ——
    /// 那会让界面显示"自定义"、实际连着别的地方
    #[test]
    fn a_custom_mode_without_an_address_has_no_entry() {
        let d = dir();
        save_source(d.path(), SourceMode::Custom, None).unwrap_err(); // 空地址直接拒
                                                                      // 手工造一份"有 mode、没地址"的档（模拟手改坏的文件）
        atomic_write(
            &source_file(d.path()),
            br#"{"sourceSchema":2,"mode":"custom","customShape":"root"}"#,
        )
        .unwrap();
        assert!(
            current_entry(d.path()).unwrap().is_none(),
            "没有地址就没有入口 —— 界面会据此报错，不静默用默认源"
        );
    }

    /// 模式 id 是契约：认这三个，别的**不猜**
    #[test]
    fn only_three_source_modes_are_recognized() {
        assert_eq!(SourceMode::parse("github"), Some(SourceMode::Github));
        assert_eq!(SourceMode::parse(" gitee "), Some(SourceMode::Gitee));
        assert_eq!(SourceMode::parse("custom"), Some(SourceMode::Custom));
        assert_eq!(SourceMode::parse("gitee2"), None);
        assert_eq!(SourceMode::parse(""), None);
    }

    /// ★ **同一个地址，两种读法都认**（这次真机 bug 的钉子）。
    ///
    /// 服务端把 `source.json`（v2 寻址规则声明）摆在交付目录里。于是：
    /// - 填**目录** → 探成 `Root`（补文件名读）；
    /// - 填 **`source.json` 地址** → 原样读，探成 `Bootstrap`。
    ///
    /// 两条都通 —— 用户填哪一种都能用，这正是"填官方地址反而不通"的修法。
    #[test]
    fn a_custom_url_is_read_as_a_root_or_as_a_source_json() {
        // 假取字节：目录里有 source.json（v2），别的 404。
        // ★ 判据**不开端口**（客户端产品不许开端口，`check:zero-network` 守着那道闸）——
        //   形状判定做成"可注入的取字节"，就能在纯内存里跑完整条判定。
        let fetch = fake_fetch(&["delivery/source.json", "delivery/catalog.json"]);
        let root_url = "https://mirror.example/presets/delivery";

        let (root_shape, resolved) = probe_custom_shape_with(root_url, &fetch).expect("当目录该通");
        assert_eq!(root_shape, CustomShape::Root);
        assert!(
            resolved
                .catalog_url()
                .expect("该推得出")
                .ends_with("/catalog.json"),
            "交付目录形状：catalog 地址由 Manifest 声明"
        );

        // ★ 用户复制来的地址**本身带着 source.json**（0.0.2 真机就是这条路不通的）
        let bootstrap_url = "https://mirror.example/presets/delivery/source.json";
        let (shape, resolved) =
            probe_custom_shape_with(bootstrap_url, &fetch).expect("当 source.json 也该通");
        assert_eq!(shape, CustomShape::Bootstrap);
        assert_eq!(
            resolved.catalog_url().expect("该推得出"),
            format!("{root_url}/catalog.json"),
            "两个地址都由 Manifest 相对声明给出"
        );
    }

    /// 两条都不通 → 拒绝，且**两条的原因都摆出来**（只说"不行"等于让用户猜）
    #[test]
    fn an_unusable_address_is_refused_with_both_reasons() {
        // 目录里什么都没有：原样与补文件名两条都要试、都要说清为什么
        let fetch = fake_fetch(&[]);
        // 地址本身带 source.json → 两条候选（原样 + 补名去重后同一条）都会试
        let e =
            probe_custom_shape_with("https://empty.example/presets/delivery/source.json", &fetch)
                .unwrap_err();
        let text = format!("{}{}", e.message, e.detail.unwrap_or_default());
        assert!(
            text.contains(SOURCE_FILE_NAME),
            "说了 source.json 那条为什么：{text}"
        );
    }

    /// 判据用的**假取字节**：目录里有哪几个文件就答哪几个，其余一律 404。
    fn fake_fetch(
        files: &'static [&'static str],
    ) -> impl Fn(&str, &str) -> Result<Vec<u8>, AppError> + 'static {
        move |url: &str, what: &str| {
            // ★ 只看**最后两段**（`delivery/source.json` 这样的形状）：
            //   前缀千变万化不该影响判定，但 `…/source.json/source.json` 的最后两段是
            //   `source.json/source.json` —— 它必须 404，否则"补名试"会误判成功。
            let path = url
                .split("://")
                .nth(1)
                .and_then(|rest| rest.find('/').map(|i| &rest[i..]))
                .unwrap_or("");
            let mut tail: Vec<&str> = path.rsplit('/').filter(|s| !s.is_empty()).take(2).collect();
            tail.reverse();
            let tail = tail.join("/");
            let hit = files.iter().any(|f| tail == *f);
            match hit {
                true => Ok(match what {
                    CATALOG_FILE => br#"{"catalogSchema":1,"revision":"r1","files":[]}"#.to_vec(),
                    SOURCE_FILE_NAME => {
                        br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":".."}"#.to_vec()
                    }
                    other => panic!("判据不该问 {other}"),
                }),
                false => Err(AppError::io("404").with_detail(url.to_owned())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().expect("临时目录建不出来")
    }

    /// 存进去再读出来必须还是那份——这条看着弱，但它钉的是"写的那套格式就是读的那套"
    #[test]
    fn roundtrips_the_chosen_source() {
        let dir = root();
        let saved = save_source(
            dir.path(),
            SourceMode::Custom,
            Some("https://cdn.example.com/mkp/"),
        )
        .expect("写设置不该失败");
        let loaded = load_source(dir.path()).expect("读设置不该失败");
        assert_eq!(loaded, Some(saved));
        assert_eq!(
            loaded.expect("刚写完的").custom_url.as_deref(),
            Some("https://cdn.example.com/mkp"),
            "末尾多余的斜杠在写盘前就砍掉了"
        );
    }

    /// 没配过 ≠ 坏了：文件不在就是 `None`，不报错（默认值的分支由 `current_entry` 管）
    #[test]
    fn missing_file_is_not_corrupted() {
        let dir = root();
        assert_eq!(load_source(dir.path()).expect("没配过不该报错"), None);
    }

    /// 撤覆盖 = 删文件：撤完 load 就是 `None`（回不回内置默认由 `current_entry` 管）
    #[test]
    fn clearing_removes_the_override() {
        let dir = root();
        save_source(
            dir.path(),
            SourceMode::Custom,
            Some("https://cdn.example.com/mkp"),
        )
        .expect("写设置");
        assert!(load_source(dir.path()).expect("读设置").is_some());

        clear_source(dir.path()).expect("撤覆盖不该失败");
        assert_eq!(load_source(dir.path()).expect("读设置"), None);
        assert!(!source_file(dir.path()).exists());
    }

    /// 撤一个本来就没有的覆盖 = 幂等（与"撤销使用"同一条规矩：没有不是错）
    #[test]
    fn clearing_when_nothing_was_set_is_fine() {
        let dir = root();
        clear_source(dir.path()).expect("没有覆盖时撤覆盖也不该失败");
        assert_eq!(load_source(dir.path()).expect("读设置"), None);
    }

    /// 坏档不静默：字节坏了要说出来，不能当成"没配"让用户再选一次却依然读不出
    #[test]
    fn unreadable_file_is_reported_not_swallowed() {
        let dir = root();
        atomic_write(&source_file(dir.path()), "这不是 JSON".as_bytes()).expect("写脏文件");

        let e = load_source(dir.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /// 代次认不出同样是坏档：老版本写的东西不说"看不懂"，就会在下一次读的时候变成谜
    #[test]
    fn future_schema_is_rejected() {
        let dir = root();
        atomic_write(
            &source_file(dir.path()),
            r#"{"sourceSchema": 99, "baseUrl": "https://x.example.com"}"#.as_bytes(),
        )
        .expect("写脏文件");

        let e = load_source(dir.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    /* ---------- Source Manifest（source.json v2）的解析 ---------- */

    /// 最基本的一份：交付根 = source.json 所在目录，catalog 地址由声明给出
    #[test]
    fn manifest_anchors_everything_at_its_own_directory() {
        let r = parse_manifest_at(
            "https://raw.githubusercontent.com/o/r/main/presets/delivery/source.json",
            br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":".."}"#,
        )
        .expect("好档该解析得动");
        assert_eq!(
            r.catalog_url().expect("该推得出"),
            "https://raw.githubusercontent.com/o/r/main/presets/delivery/catalog.json"
        );
        // filesRoot ".." = 交付目录的上一层（发布根 presets/）
        assert_eq!(
            r.resolver.delivery_url(),
            "https://raw.githubusercontent.com/o/r/main/presets/delivery"
        );
    }

    /// release 声明了就从声明来（不再"上跳一级"去猜）
    #[test]
    fn release_url_comes_from_the_manifest_declaration() {
        let r = parse_manifest_at(
            "https://host/a/presets/delivery/source.json",
            br#"{"sourceSchema":2,"catalog":"catalog.json","release":"release.json","filesRoot":".."}"#,
        )
        .expect("好档该解析得动");
        assert_eq!(
            r.release_url().expect("声明了就该给"),
            "https://host/a/presets/delivery/release.json"
        );
    }

    /// 代次认不得 / catalog 越过目录 / catalog 写成绝对 URL / 缺 filesRoot / 不是 JSON ——
    /// 一律如实拒，不猜（校验本体住 resolver，这里钉"入口同样拒"）
    #[test]
    fn manifest_bad_shapes_are_refused() {
        let url = "https://host/a/source.json";
        for bad in [
            r#"{"sourceSchema": 99, "catalog": "catalog.json", "filesRoot": ".."}"#,
            r#"{"sourceSchema": 2, "catalog": "", "filesRoot": ".."}"#,
            r#"{"sourceSchema": 2, "catalog": "../outside.json", "filesRoot": ".."}"#,
            r#"{"sourceSchema": 2, "catalog": "https://elsewhere.example.com/catalog.json", "filesRoot": ".."}"#,
            r#"{"sourceSchema": 2, "catalog": "catalog.json"}"#,
            r#"不是 JSON"#,
        ] {
            assert!(
                parse_manifest_at(url, bad.as_bytes()).is_err(),
                "{bad} 该被拒"
            );
        }
    }

    /// 本地文件（工作台校验交付面里那一份）**不需要 URL 也能验内容**：
    /// SourceManifest::parse 是纯解析 —— 这是发布闸 ⑬ 的用法。
    #[test]
    fn manifest_parses_without_any_url() {
        let m = crate::runtime::resolver::SourceManifest::parse(
            br#"{"sourceSchema":2,"catalog":"catalog.json","filesRoot":".."}"#,
        )
        .expect("本地合法档该验得过，不需要 URL");
        assert_eq!(m.catalog, "catalog.json");
        assert_eq!(m.files_root, "..");
    }

    /// 目录回退的边界：host 后面的第一段才是目录；没有路径段 / 没有 host 都拒
    #[test]
    fn directory_fallback_edges() {
        assert_eq!(
            directory_of("https://host/source.json").expect("host 根"),
            "https://host"
        );
        assert_eq!(
            directory_of("https://host/a/b/source.json").expect("两层"),
            "https://host/a/b"
        );
        for bad in [
            "https://host",
            "https://",
            "source.json",
            "file:///a/source.json",
        ] {
            assert!(directory_of(bad).is_err(), "{bad} 该被拒");
        }
    }

    /// 空地址不许写盘：它会制造"配了但等于没配"的第三种状态
    #[test]
    fn empty_base_url_is_refused_before_writing() {
        let dir = root();
        let e = save_source(dir.path(), SourceMode::Custom, Some("   ")).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(!source_file(dir.path()).exists(), "拒绝时不留半个文件");
    }

    /// 只放出站 HTTP：**本地文件 Scheme 走同一个下载栈会造成"下载读到本机任意文件"**
    #[test]
    fn local_paths_are_not_accepted_as_a_source() {
        for raw in [
            "file:///Users/me/presets",
            "/Users/me/presets",
            "\\\\server\\share",
            "ftp://example.com/presets",
        ] {
            let e = normalize_base_url(raw).unwrap_err();
            assert_eq!(
                e.code,
                crate::error::ErrorCode::InvalidArgument,
                "拒绝 {raw}"
            );
        }
    }
}
