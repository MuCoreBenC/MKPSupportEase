//! 资产域①层：`presets/assets.toml` —— **一份集中定义**（b05 Task 8）。
//!
//! # 为什么是一份文件，不是一资产一文件
//!
//! 旧仓是 `source/assets/*.toml`、18 份一资产一文件（doc §12.5）。18 个条目时它已经难受：
//! 加一张图要新建一个文件，而"这批资产都属于 A1"这句话在目录里看不出来。doc §8 定为
//! **一份集中定义 + 目录约定**，门槛写在 doc §8：超过 20 个再拆。
//!
//! # 定义里**不写**什么
//!
//! - **不写 `fileName`**：路径只有 `path` 一处（旧仓同时存 `fileName` 与 `relativePath`，
//!   两处说的是同一件事，改名时必然有一处忘掉）。
//! - **不写 `sha256` / `size`**：那是**交付物**的属性，发布时按真实字节算（Task 13.6）。
//!   写进①层就是第二份真相，而且它一定先过期。
//! - **不给 MKP 预设建条目**（doc §12.5）：它的路径由命名规则算出（`preset_file_name`），
//!   登记一份就是冗余。
//!
//! # 文件本体在哪
//!
//! 不在本文件旁边，而在**资产载荷根**下（工作台那边是 `<repo>/public/assets/`）。
//! 条目里的 `path` **永远相对载荷根**，不相对本文件所在目录 —— 这条是刻意的：
//! 定义（①层，`presets/`）与载荷本来就是分家，路径的基准只有一处。
//!
//! **载荷根由调用方给**（[`Assets::set_asset_root`]）：这一层不决定数据在哪里。
//! 客户端这一轮只释放定义、不释放文件本体，所以它压根没有载荷根 ——
//! 那时 [`Assets::resolve`] 报"没有载荷根"，[`Assets::present`] 一律为 false。
//!
//! # 加载期就查掉的两条（Task 8.8）
//!
//! 1. **id 唯一且大小写不敏感** —— 只差大小写的两个 id 会让"查表结果"取决于顺序；
//! 2. **`path` 落在资产根内** —— 走 [`resolve_in`] 那三道闸（拒绝对路径 / 拒绝 `..` /
//!    比真实路径防符号链接）。
//!
//! 这两条坏掉的表现都**不是报错**，而是"某个地方少了一张图"。所以它们在加载期就拦，
//! 而不是等某个页面渲染时才发现。

use postprocess::diag::messages;

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::fsx::paths::{check_relative, resolve_in};

/// 定义文件（`presets/` 下）
pub const ASSETS_FILE: &str = "assets.toml";

/// 资产类型集合（闸 G-3：BBS 纳入、切片器维度**开放**；模型保留）。
///
/// 四个取值而不是三个：`image` 与 `icon` 分开是**消费方式**不同（一个是机型图，
/// 一个是矢量标记，前端一处 <img>、一处当符号用），而 doc §8 那句"图片 / 图标、模型"
/// 说的是**同一类文件形态**，不是同一个字段语义。
///
/// **`image` 回到台账了**（2026-10-03 改判，撤销 2026-10-01 第三刀的剥离）：整机图
/// 住在 `presets/assets/printers/`，台账里是 `type = 'image'` + `delivery = 'bundled'`
/// 的条目 —— **工作台看得见、选得着、换得掉**（作者：不会编程的用户改不了图才是问题），
/// 到客户端靠构建期复制随包，客户端不下载。判据在
/// [`crate::runtime::catalog::dest_of_asset`]（**按交付档位拦**，不按类型拦）。
///
/// **2026-10-02 复审结论：保留（去留已明确，别再问"要不要顺手删"）。** 三条依据：
/// ① 它不是"纯未使用枚举" —— 台账里四条 image 条目在用、[`crate::runtime::catalog`]
/// 的 kind 映射表里也有一格（返回 None = 下载区没有「图片」这一段）；
/// ② 也不是旧世界残留 —— 「进不进交付集合」是**档位**（`bundled`）的事，不是类型的事；
/// ③ 要删就是一次 schema 清理（动公开契约 `Machine.image`、mock、工作台两页、判据），
/// 属另案，不顺手做。
///
/// **没有 `mkpPreset`**：那一条按 doc §12.5 不建条目（见模块头）。这个 enum 是那句话的
/// 结构化版本 —— 想登记 MKP 预设得先改这里，而改这里要过一次 review。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Image,
    Icon,
    Model,
    /// **引导图**（2026-10-16 加的第五类）：图片引导那套里用的**系统窗口截图**
    /// （浅 / 深各一张，挂在"某一步"上，见 `src/app/guide/GuideImage.tsx`）。
    ///
    /// 与 [`Self::Image`] 分开：那一类**挂在机型上**（机型文件里的 `image` 指过来），
    /// 这一类**挂在某个步骤上**（引导 spec 按 id 取）—— 两类都够不上对方那套回落链。
    ///
    /// 今天不进下载面（`kind_of_asset` 给 `None`）：`delivery = 'bundled'` 先随包，
    /// 客户端走构建期装配出来的 `/assets/<path>` 取。作者要的**按需下载**是下一刀
    /// （那时在这儿与 `kind_of_asset` 各加一格，再给客户端补"读下载区图片"的口子）。
    Guide,
    /// 切片器预设。今天只有 BBS，`slicer` 字段留着别的切片器的位置
    SlicerProfile,
    /// **MKP 预设**（作者 2026-10-03：「为什么资产库里面不放 mkp 预设」——
    /// 一切配发内容都在资产库里有登记，套餐从这儿统一选）。
    ///
    /// 与前四类的**根本差别**：它的文件不是资产根下的静态文件，而是**生成产物**
    /// （`presets/delivery/mkp/presets/<产物名>`，命名规则算出）—— 所以这一类
    /// **不写 `path`**（那是 doc §12.5「路径由命名规则算出」的实现），改写
    /// `versionId`（`machineId` + `versionId` 定位那一版）。**文件在不在都不影响
    /// 登记与选用**（作者：「不只是没文件的时候可以选择，有文件也要可以选择」）——
    /// 「有没有生成」是另一条状态（生成页那四档），不由资产域管。
    MkPreset,
}

impl AssetKind {
    /// 写进 TOML 的那个字面量（与 serde 的 camelCase 一致）
    pub fn key(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Icon => "icon",
            Self::Model => "model",
            Self::SlicerProfile => "slicerProfile",
            Self::MkPreset => "mkPreset",
            Self::Guide => "guide",
        }
    }
}

/// 一条资产定义。字段就六个，每个都有理由：
///
/// - `id`：主键。**查表、反查、交付索引全用它**，不用路径（路径会改，id 不改 ——
///   与上游 `mkpPresetAssetId` 同一条理由）；
/// - `kind`：类型，决定它进哪一段交付索引、是不是要跟着切片器走；
/// - `machine_id`：归属机型。**可以没有**（将来的公共素材），有就必须是真机型
///   （跨文件那条在 [`super::Presets::check_cross_consistency`] 里查）；
/// - `name`：给人看的名字。界面上的"这张图叫什么"不该靠文件名猜；
/// - `path`：相对资产根的文件位置。**唯一一份路径**；
/// - `slicer` / `profile`：只属于 `slicerProfile`。`slicer` 是 G-3 留的开放维度
///   （今天只有 `bbs`），`profile` 是该切片器下的档位（旧仓的 `category`，实测都是 `process`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Asset {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: AssetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub machine_id: Option<String>,
    pub name: String,
    /// 相对资产根的文件位置。**唯一一份路径**。
    ///
    /// `mkpPreset` 类**留空** —— 它的文件是生成产物（落点在 `presets/delivery/mkp/presets/`），
    /// 路径由命名规则算出，不在这里再登记一份（那是 doc §12.5 的原话；
    /// 作者 2026-10-03 要求「进资产库」指的是**登记与选用**，不是把产物路径抄进台账）。
    #[serde(default)]
    pub path: String,
    /// **归属版本**（`mkpPreset` 类专有）：`machineId` + `versionId` 定位那一版。
    /// 其余三类留空 —— 它们的归属是机型（`machineId`）或全局
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slicer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// **交付身份**（作者 2026-10-03 新增，默认 `download`）。
    ///
    /// - `download`（默认）：进交付集合，客户端**按需下载**（BBS / 图标 / 模型 / MKP 产物）；
    /// - `bundled`：**随包不下载** —— 台账里登记、工作台里看得见选得着，构建期从
    ///   数据目录复制进客户端资源，客户端不下载也不更新它（整机图：用户不换机器就不换图，
    ///   但**换图的人有权在工作台里换** —— 2026-10-01 第三刀把它挪进客户端源码是错的，
    ///   已作废）。
    ///
    /// 这是**静态声明**（写在 `assets.toml` 里），与运行时那个「在菜单 / 仅归档」
    /// （`patch::Visibility`，工作台可改、存 store）正交：那份管**客户端菜单里看不看得到**，
    /// 这一栏管**它到不到客户端手里**。
    #[serde(default)]
    pub delivery: Delivery,
    /// **这份模型是哪一种校准件**（2026-10-19 加）—— 只有 `type = 'model'` 用得上，
    /// 其余类留空。
    ///
    /// 值的真源住在模型文件自己身上（`3D/3dmodel.model` 的 `<metadata name="MKPSE:Calibration">`），
    /// 台账这一格是**预期**：检查脚本拿它与文件里读到的那份比，对不上就报"改了没同步"。
    ///
    /// 为什么不靠 id 后缀猜：id 是被人起的名，改个名字不该让校验跟着变；写在这里，
    /// "这份文件该是什么身份"就有一个可读的真值。
    ///
    /// 取值只认四个：`ZOffset` / `Precise` / `Rough` / `Repetition`（解析出口是
    /// `postprocess::calibration_asset::validate_identity`）。
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub calibration: String,
}

/// [`Asset::delivery`] —— 这份资产**怎么到客户端手里**
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Delivery {
    /// 进交付集合，客户端按需下载（默认）
    #[default]
    Download,
    /// 随包不下载：构建期从数据目录带进客户端，客户端不下载、不更新
    Bundled,
}

/// **一组图**里的一名成员（作者 2026-10-16）。
///
/// 为什么成员要带 `role`：客户端要按主题挑图（浅色版 / 深色版），而"哪张是浅色那张"
/// 是**内容属性**，不是文件名属性 —— 所以它写在台账里，由人指定（也可以不填）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GroupMember {
    /// 指 [`Asset::id`]（**指向 id，不指路径** —— 路径会改，id 不改）
    pub asset: String,
    /// 这张在这个组里演什么角色（`light` / `dark` / …）。**可以空** ——
    /// 空 = "只是一伙的、不担任什么"，客户端挑不出具体一张时回落到组里第一张
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub role: String,
}

/// **一组图**：共用一份标注源稿的那几张（作者 2026-10-16）。
///
/// > 「不能只用文件名去实现，必须有一个去管理它，像那个 catalog 那种的」
///
/// 从前"哪几张是一组"是**从文件名算**的（抹掉 `-light` / `-dark` 后缀）。现在它是一条
/// 登记：组有名、有 id、成员按顺序摆着，跟资产一样入库、跟仓库走、编进 catalog。
///
/// # `id` 为什么同时就是那份源稿的文件名
///
/// 一份源稿的落点是 `<组成员所在目录>/<组 id>.anno.json`。id 与文件名同值 ⇒
/// **改组名不动落点**，而且"这一组共用哪份标注"在目录里一眼看得出来。
/// 现有那对引导图的组 id 正好是 `open-with`（旧口径 `pair_stem` 算出来也是它）
/// ⇒ 老文件零迁移。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Group {
    /// 主键，**大小写不敏感**唯一；也用作那份源稿的文件名（所以必须是安全的文件名）
    pub id: String,
    /// 给人看的组名（作者要的"套装的感觉"就靠它）
    pub name: String,
    /// 成员，**有序**（第一个是"没指定角色时"的兜底）
    pub members: Vec<GroupMember>,
}

/// 旧口径的"一对图"共同干名：`open-with-light` / `open-with-dark` ⇒ `open-with`。
///
/// ★ 现在它是**兜底**（[`Assets::anno_stem_of`] 的第 ② 条），不再是分组的主判据 ——
/// 分组请用 [`Group`]。留着它只为"没建组的那张图也能单独标一下"。
///
/// ★ 客户端 `openWithGuide.ts` 那一侧的浅深配对是**另一回事**（它按组的 `role` 挑），
/// 那边已经不吃这条了。
pub fn pair_stem(stem: &str) -> &str {
    for suf in ["-light", "-dark", "_light", "_dark"] {
        if let Some(rest) = stem.strip_suffix(suf) {
            return rest;
        }
    }
    stem
}

/// 文件形状。`[[assets]]` 数组表，与 `brands.toml` 那份"一份集中定义"同一写法。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssetsFile {
    #[serde(default)]
    assets: Vec<Asset>,
    /// **分组登记**（2026-10-16 加）。缺省空 = 一份组都没有（旧文件照读）
    #[serde(default)]
    groups: Vec<Group>,
}

/// 一次加载的资产定义。
///
/// 与 [`super::Catalog`] 同一套路：**留着原文文档**（`DocumentMut`），所以
/// "读进来不改再写出去"在构造上就是逐字节相同的，不靠谁记得别动格式。
#[derive(Debug)]
pub struct Assets {
    items: Vec<Asset>,
    /// 分组登记（[`Group`]）。与 `items` 一样是**从文档读出来的**，改动同样要落到 `doc` 上
    groups: Vec<Group>,
    doc: DocumentMut,
    file: PathBuf,
    /// 载荷根。**由调用方给**，没有就是"这一侧没有资产文件本体"（见模块头）
    asset_root: Option<PathBuf>,
}

impl Assets {
    /// 读 `<root>/assets.toml`。读不到 → 错误（**不用空数据装成能跑**：那会让人以为
    /// "我们一条资产都没有"，而那和"读不出来"是两件事 —— 与 [`super::Presets::load`] 同一口径）
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join(ASSETS_FILE);
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let parsed: AssetsFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(messages::text(
                "ui.presetdata.assets.168",
                &[("v1", &file.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;

        let out = Self {
            items: parsed.assets,
            groups: parsed.groups,
            doc,
            file,
            asset_root: None,
        };
        out.check()?;
        Ok(out)
    }

    /// 载荷根。**由调用方给**（工作台给 `<repo>/public/assets`）—— 这一层不决定数据在哪
    pub fn set_asset_root(&mut self, root: &Path) {
        self.asset_root = Some(root.to_path_buf());
    }

    pub fn asset_root(&self) -> Option<&Path> {
        self.asset_root.as_deref()
    }

    pub fn items(&self) -> &[Asset] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 按 id 取。**大小写不敏感** —— 加载期已保证不会有两个只差大小写的 id，
    /// 所以这里放宽匹配不会撞上歧义，而调用方（机型里的 `image` / `icon` 字段）
    /// 不必关心当初是谁按什么大小写写下的
    pub fn get(&self, id: &str) -> Option<&Asset> {
        let want = id.trim().to_lowercase();
        self.items.iter().find(|a| a.id.to_lowercase() == want)
    }

    /// 全部分组登记（按文档里的顺序）
    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    /// 按组 id 取（大小写不敏感，同 [`Self::get`] 的理由）
    pub fn group_by_id(&self, id: &str) -> Option<&Group> {
        let want = id.trim().to_lowercase();
        self.groups.iter().find(|g| g.id.to_lowercase() == want)
    }

    /// 这条资产属于哪一组（不属任何组 ⇒ `None`）。
    ///
    /// 判据链只有这一条，别处别再算一遍（"一张图至多属一组"由 [`Self::check_groups`] 保着）
    pub fn group_of(&self, asset_id: &str) -> Option<&Group> {
        let want = asset_id.trim().to_lowercase();
        self.groups.iter().find(|g| {
            g.members
                .iter()
                .any(|m| m.asset.trim().to_lowercase() == want)
        })
    }

    /// 这张资产生的那份**标注源稿干名**（⇒ `<同一目录>/<干名>.anno.json`）。
    ///
    /// ★ **唯一判据**（`workbench::app::annotation` 也走这一条，别在那边再写一份）：
    /// ① 有登记的组 → **组 id**；② 没有 → 旧的 [`pair_stem`]（抹掉浅 / 深后缀）兜底。
    ///
    /// ② 留着是为了"给一张图单独标一下"这种小事不用先建组 —— 代价是那份源稿的落点
    /// 仍受文件名影响（改名会让它挪窝）。真在意稳定的话，给它建个组。
    pub fn anno_stem_of(&self, asset_id: &str) -> Option<String> {
        if let Some(g) = self.group_of(asset_id) {
            return Some(g.id.clone());
        }
        let a = self.get(asset_id)?;
        let stem = Path::new(a.path.trim())
            .file_stem()
            .and_then(|s| s.to_str())?;
        Some(pair_stem(stem).to_owned())
    }

    /// 一个条目的**真实路径**：载荷根 + `path`，走 [`resolve_in`] 的三道闸。
    ///
    /// 没有载荷根时报错而不是猜一个 —— 猜出来的路径要么查不到文件、要么查到别人的文件。
    pub fn resolve(&self, asset: &Asset) -> Result<PathBuf, AppError> {
        let root = self.asset_root.as_ref().ok_or_else(|| {
            AppError::not_found(messages::text("ui.presetdata.assets.169", &[])).with_detail(
                "资产文件本体与定义分家：定义在 presets/assets.toml，载荷在 \
                 presets/assets/（2026-10-03 前是 public/assets/，资产链第一刀搬的）。\
                 没有载荷根时按「文件还没到」处理，不要猜路径"
                    .to_owned(),
            )
        })?;
        resolve_in(root, asset.path.trim()).map_err(|e| {
            AppError::invalid_argument(messages::text(
                "ui.presetdata.assets.170",
                &[
                    ("v1", &asset.id.to_string()),
                    ("v2", &asset.path.to_string()),
                ],
            ))
            .with_detail(format!(
                "path 必须落在资产载荷根（{}）之内，且只能是相对路径；原错误：{e}",
                root.display()
            ))
        })
    }

    /// 这条资产的文件在不在。
    ///
    /// **Task 9 之前它会普遍是 `false`** —— 条目与文件一起在那边落地。这里不做成
    /// 加载期错误：定义与搬运是两步，而"还没搬"要能被说出来（界面显示、Task 11 的
    /// 校验层按 warning 报），不该让工作台起不来。
    pub fn present(&self, asset: &Asset) -> bool {
        self.resolve(asset).is_ok_and(|p| p.is_file())
    }

    /// 写回用的文本。**没改过就逐字节等于读进来那份**
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// 新增一条，**由调用方决定何时落盘**（[`Self::write`]）。
    ///
    /// 为什么是"改内存 + 显式写"而不是像机型那样"加法即落盘"：资产条目将来是**批量**来的
    /// （搬一次资产 = 十几条），一次写一个文件比写十几遍好；而且中途失败会留下
    /// "登记了一半"的定义 —— 那种状态没有任何判据能描述它。
    pub fn add(&mut self, asset: Asset) -> Result<(), AppError> {
        let candidate = Asset {
            id: asset.id.trim().to_owned(),
            name: asset.name.trim().to_owned(),
            path: asset.path.trim().to_owned(),
            ..asset
        };
        if let Some(prev) = self.get(&candidate.id) {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.assets.171",
                &[("v1", &prev.id.to_string())],
            ))
            .with_detail(::postprocess::diag::messages::text(
                "ui.presetdata.why.id_is_key",
                &[],
            )));
        }
        self.check_one(&candidate)?;

        let mut t = toml_edit::Table::new();
        t["id"] = super::literal_str(&candidate.id);
        t["type"] = super::literal_str(candidate.kind.key());
        if let Some(m) = &candidate.machine_id {
            t["machineId"] = super::literal_str(m);
        }
        t["name"] = super::literal_str(&candidate.name);
        t["path"] = super::literal_str(&candidate.path);
        if let Some(s) = &candidate.slicer {
            t["slicer"] = super::literal_str(s);
        }
        if let Some(p) = &candidate.profile {
            t["profile"] = super::literal_str(p);
        }

        // `assets` 可能整个不存在（骨架文件只有注释）
        let entry = self
            .doc
            .entry("assets")
            .or_insert(toml_edit::Item::ArrayOfTables(
                toml_edit::ArrayOfTables::new(),
            ));
        let arr = entry.as_array_of_tables_mut().ok_or_else(|| {
            AppError::corrupted(messages::text(
                "ui.presetdata.assets.172",
                &[("v1", &self.file.display().to_string())],
            ))
        })?;
        arr.push(t);

        self.items.push(candidate);
        Ok(())
    }

    /// 删掉一条，返回被删的那个。
    ///
    /// **本函数不判断"有没有人用它"** —— 它看不见机型与套餐。那道判断在
    /// [`super::Presets::remove_asset`]，**删资产要走那一条**（b05 Task 9.5）。
    pub fn remove(&mut self, id: &str) -> Result<Asset, AppError> {
        let want = id.trim().to_lowercase();
        let Some(pos) = self.items.iter().position(|a| a.id.to_lowercase() == want) else {
            return Err(AppError::not_found(messages::text(
                "ui.presetdata.assets.173",
                &[("id", id)],
            )));
        };
        let removed = self.items.remove(pos);

        // 文档面同样要删掉那一段 —— 只改内存的话，下一次 `write` 会把它写回来
        let arr = self
            .doc
            .get_mut("assets")
            .and_then(|i| i.as_array_of_tables_mut())
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.assets.174",
                    &[("v1", &self.file.display().to_string())],
                ))
            })?;
        let idx = arr
            .iter()
            .position(|t| {
                t.get("id")
                    .and_then(|i| i.as_str())
                    .is_some_and(|s| s.to_lowercase() == want)
            })
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.assets.175",
                    &[("v1", &removed.id.to_string())],
                ))
            })?;
        arr.remove(idx);
        Ok(removed)
    }

    /// 改一条资产的**交付档位**（`download` ↔ `bundled`）。
    ///
    /// 内存与文档面一起改（同 [`Self::remove`] 的纪律 —— 只改内存的话下一次
    /// `write` 会把旧值写回去）。值没变就不动文档。
    ///
    /// **`mkPreset` 不许设成 bundled**：它的文件是生成产物、落点在交付根
    /// `mkp/presets/`，没有「随包复制」这条路径（构建期复制脚本只搬有 `path` 的）
    pub fn set_delivery(
        &mut self,
        id: &str,
        delivery: crate::presetdata::assets::Delivery,
    ) -> Result<(), AppError> {
        let want = id.trim().to_lowercase();
        let asset = self
            .items
            .iter_mut()
            .find(|a| a.id.to_lowercase() == want)
            .ok_or_else(|| {
                AppError::not_found(messages::text("ui.presetdata.assets.176", &[("id", id)]))
            })?;
        if asset.kind == AssetKind::MkPreset
            && delivery == crate::presetdata::assets::Delivery::Bundled
        {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.assets.177",
                &[("id", id)],
            ))
            .with_detail(
                "MKP 预设的文件是生成产物（mkp/presets/…），没有随包复制这条路径；\
                 它的「在不在」由生成页的生成状态说"
                    .to_owned(),
            ));
        }
        if asset.delivery == delivery {
            return Ok(());
        }
        asset.delivery = delivery;

        // 文档面：那一段表格里的 delivery 键（缺省 download 可以不写 —— 删掉键就是回到缺省）
        let arr = self
            .doc
            .get_mut("assets")
            .and_then(|i| i.as_array_of_tables_mut())
            .ok_or_else(|| {
                AppError::corrupted(messages::text(
                    "ui.presetdata.assets.178",
                    &[("v1", &self.file.display().to_string())],
                ))
            })?;
        let table = arr
            .iter_mut()
            .find(|t| {
                t.get("id")
                    .and_then(|i| i.as_str())
                    .is_some_and(|s| s.to_lowercase() == want)
            })
            .ok_or_else(|| {
                AppError::corrupted(messages::text("ui.presetdata.assets.179", &[("id", id)]))
            })?;
        if delivery == crate::presetdata::assets::Delivery::Download {
            table.remove("delivery");
        } else {
            table["delivery"] = super::literal_str("bundled");
        }
        Ok(())
    }

    /// **新建或改写一个分组**（作者 2026-10-16）。
    ///
    /// 先过 [`Self::check_groups`] 那几条不变量（**坏值整次拒绝，不写下去** ——
    /// 与 [`Self::set_delivery`] 同一条纪律），再内存与文档面一起改。
    ///
    /// 文档面**整段重写** `[[groups]]`：分组是工具管的登记段（不是人手写的大注释区），
    /// 整段重写比"精确改某一行"少一类漂移。`[[assets]]` 那一段一个字节都不碰。
    pub fn upsert_group(&mut self, group: Group) -> Result<(), AppError> {
        let candidate = Group {
            id: group.id.trim().to_owned(),
            name: group.name.trim().to_owned(),
            members: group
                .members
                .into_iter()
                .map(|m| GroupMember {
                    asset: m.asset.trim().to_owned(),
                    role: m.role.trim().to_owned(),
                })
                .collect(),
        };
        let want = candidate.id.to_lowercase();
        let mut next: Vec<Group> = self
            .groups
            .iter()
            .filter(|g| g.id.to_lowercase() != want)
            .cloned()
            .collect();
        next.push(candidate);
        next.sort_by_key(|g| g.id.to_lowercase());

        /* 先把候选摆进去试一遍，过了才认 —— 拒绝时内存与盘上都不留痕 */
        let prev = std::mem::replace(&mut self.groups, next);
        if let Err(e) = self.check_groups() {
            self.groups = prev;
            return Err(e);
        }
        self.write_groups_doc();
        Ok(())
    }

    /// 删掉一个分组。成员**原地变回"未分组"**（资产一条不动 —— 分组只是登记，不是归属）。
    pub fn remove_group(&mut self, id: &str) -> Result<Group, AppError> {
        let want = id.trim().to_lowercase();
        let Some(pos) = self.groups.iter().position(|g| g.id.to_lowercase() == want) else {
            return Err(AppError::not_found(messages::text(
                "ui.presetdata.assets.180",
                &[("id", id)],
            )));
        };
        let removed = self.groups.remove(pos);
        self.write_groups_doc();
        Ok(removed)
    }

    /// 把内存里那份 `groups` 整段写回文档面（`[[groups]]` 表数组）。
    ///
    /// ★ 只在 [`Self::upsert_group`] / [`Self::remove_group`] 里调 —— 别在加载路径上调，
    /// 那会把"读进来不改再写出去逐字节相同"这条底破掉。
    fn write_groups_doc(&mut self) {
        let mut arr = toml_edit::ArrayOfTables::new();
        for g in &self.groups {
            let mut t = toml_edit::Table::new();
            t["id"] = super::literal_str(&g.id);
            t["name"] = super::literal_str(&g.name);
            /* ★ 先把**全部**成员攒进一个表数组再挂上去 —— 逐个 `t["members"] = …` 是覆盖，
            只剩最后一个（2026-10-16 实测踩到：写出去只剩一名成员，读回来少一半） */
            let mut inner = toml_edit::ArrayOfTables::new();
            for m in &g.members {
                let mut mt = toml_edit::Table::new();
                mt["asset"] = super::literal_str(&m.asset);
                if !m.role.is_empty() {
                    mt["role"] = super::literal_str(&m.role);
                }
                inner.push(mt);
            }
            t["members"] = toml_edit::Item::ArrayOfTables(inner);
            arr.push(t);
        }
        if arr.is_empty() {
            self.doc.remove("groups");
        } else {
            self.doc["groups"] = toml_edit::Item::ArrayOfTables(arr);
        }
    }

    /// 原子写回。**唯一的写盘点**（与 [`super::Catalog::write_machine`] 同一条出口）
    pub fn write(&self) -> Result<(), AppError> {
        crate::fsx::atomic::atomic_write(&self.file, self.to_toml().as_bytes())
    }

    /// 加载期那两条（Task 8.8）+ 类型与专有字段的搭配
    fn check(&self) -> Result<(), AppError> {
        for a in &self.items {
            if a.id.trim().is_empty() {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.181",
                    &[("v1", &self.file.display().to_string())],
                )));
            }
            self.check_one(a)?;
        }

        // id 唯一（**大小写不敏感**）：`A1-Image` 与 `a1-image` 是同一个 id 的两种写法，
        // 留两条的话"`image = 'A1-IMAGE'` 指向哪一条"就取决于遍历顺序
        let mut seen: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for a in &self.items {
            let lower = a.id.to_lowercase();
            if let Some(prev) = seen.insert(lower, a.id.clone()) {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.182",
                    &[("v1", &prev.to_string()), ("v2", &a.id.to_string())],
                ))
                .with_detail(
                    "id 是主键，且**大小写不敏感** —— 只差大小写的两条会让查表结果取决于顺序"
                        .to_owned(),
                ));
            }
        }

        self.check_groups()?;
        Ok(())
    }

    /// **分组那几条不变量**（作者 2026-10-16 新增）。
    ///
    /// 为什么放在加载期：这些坏掉的表现都**不是报错**，而是"某几张图共用错了标注"或
    /// "两份源稿抢同一个文件"—— 与 id 撞车同一类，早拦早好。
    ///
    /// 1. `id` 非空、**大小写不敏感唯一**、且是**安全的文件名**（它同时是源稿的文件名）；
    /// 2. `name` 非空（界面上要显示的就是它）；
    /// 3. 成员至少一个、每个都指着真资产、同一组里不重复；
    /// 4. **同一组的成员必须在同一个目录**（源稿落点 = 成员所在目录 + 组 id ⇒ 否则不唯一）；
    /// 5. **一张资产至多属于一个组**（否则"它跟谁共用一份标注"有两种答案）；
    /// 6. 组 id 不许与**同目录下某张未分组资产**的旧口径干名撞（那样两份源稿同名）。
    fn check_groups(&self) -> Result<(), AppError> {
        let mut seen_ids: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        /* 组 id → 那个组所在目录（第 6 条要用） */
        let mut group_dirs: std::collections::BTreeMap<String, PathBuf> =
            std::collections::BTreeMap::new();
        /* 资产 id（小写）→ 它已经属于哪个组（第 5 条） */
        let mut owned: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();

        for g in &self.groups {
            let id = g.id.trim();
            let lower = id.to_lowercase();
            if id.is_empty() {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.183",
                    &[("v1", &self.file.display().to_string())],
                )));
            }
            /* 安全文件名（它同时是那份源稿的文件名）：借 path 那两道与根无关的闸 */
            check_relative(&format!("{id}.anno.json")).map_err(|e| {
                AppError::corrupted(messages::text("ui.presetdata.assets.184", &[("id", id)]))
                    .with_detail(format!(
                        "组 id 同时就是那份标注源稿的文件名（`<目录>/<组 id>.anno.json`）—— \
                     得是个安全的名字。{e}"
                    ))
            })?;
            if let Some(prev) = seen_ids.insert(lower.clone(), id.to_owned()) {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.185",
                    &[("prev", &prev.to_string()), ("id", id)],
                ))
                .with_detail(::postprocess::diag::messages::text(
                    "ui.presetdata.why.id_is_key",
                    &[],
                )));
            }
            if g.name.trim().is_empty() {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.186",
                    &[("id", id)],
                ))
                .with_detail(::postprocess::diag::messages::text(
                    "ui.presetdata.why.display_is_face",
                    &[],
                )));
            }
            if g.members.is_empty() {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.187",
                    &[("id", id)],
                ))
                .with_detail(
                    "分组就是「哪几张图共用一份标注」—— 没有成员的组没有意义，删掉它".to_owned(),
                ));
            }

            let mut dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
            for m in &g.members {
                let mid = m.asset.trim();
                let Some(a) = self.get(mid) else {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.assets.188",
                        &[("id", id), ("mid", mid)],
                    ))
                    .with_detail(::postprocess::diag::messages::text(
                        "ui.presetdata.why.member_is_asset_id",
                        &[],
                    )));
                };
                let dir = Path::new(a.path.trim())
                    .parent()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
                    .unwrap_or_default();
                dirs.insert(dir);
                if let Some(prev) = owned.insert(a.id.to_lowercase(), id.to_owned()) {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.assets.189",
                        &[
                            ("v1", &a.id.to_string()),
                            ("prev", &prev.to_string()),
                            ("id", id),
                        ],
                    ))
                    .with_detail(
                        "一张图只能跟一伙人共用一份标注 —— 它属于谁，那份 `.anno.json` 就只有一个"
                            .to_owned(),
                    ));
                }
            }
            if dirs.len() > 1 {
                let which: Vec<String> = dirs.iter().cloned().collect();
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.190",
                    &[("id", id)],
                ))
                .with_detail(format!(
                    "分别在：{}。一份标注源稿只能落在一个目录里（`<目录>/<组 id>.anno.json`）",
                    which.join("、")
                )));
            }
            group_dirs.insert(
                lower,
                PathBuf::from(dirs.first().cloned().unwrap_or_default()),
            );
        }

        /* 第 6 条：未分组资产的旧口径干名，不许与同目录的组 id 撞 */
        for a in &self.items {
            if owned.contains_key(&a.id.to_lowercase()) {
                continue;
            }
            let Some(stem) = Path::new(a.path.trim())
                .file_stem()
                .and_then(|s| s.to_str())
            else {
                continue;
            };
            let derived = pair_stem(stem);
            if derived.trim().is_empty() {
                continue;
            }
            let dir = Path::new(a.path.trim())
                .parent()
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            if let Some(g) = group_dirs.get(&derived.to_lowercase()) {
                if g.to_string_lossy().replace('\\', "/") == dir {
                    return Err(AppError::corrupted(messages::text(
                        "ui.presetdata.assets.191",
                        &[("v1", &a.id.to_string()), ("derived", derived)],
                    ))
                    .with_detail(
                        "那样两份源稿会抢同一份文件。把这张图收进那个组，或者给它单独建一个组"
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// 单条的形状检查。`add` 与 `check` 共用一条 —— 两条路走同一套判据，
    /// 免得"手写的能过、程序加的过不了"或者反过来
    fn check_one(&self, a: &Asset) -> Result<(), AppError> {
        if a.id.trim().is_empty() {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.assets.192",
                &[],
            )));
        }
        if a.name.trim().is_empty() {
            return Err(AppError::invalid_argument(messages::text(
                "ui.presetdata.assets.193",
                &[("v1", &a.id.to_string())],
            )));
        }
        // 路径：走那两道不需要根的闸（非空 / 非绝对 / 无 `..`，顺带证明它不是空的）。
        // **第三道（比真实路径防符号链接）要有载荷根才查得了**，那一步在 [`Self::resolve`]。
        // 加载期不拿载荷根：客户端这一轮压根没有资产文件本体，而"路径不许 `../x`"
        // 这条判据与根在不在无关
        //
        // **`mkpPreset` 例外**（作者 2026-10-03）：它的文件是生成产物、路径由命名规则
        // 算出，台账里**不写 path** —— 写的就是第二份会过期的真相
        if a.kind == AssetKind::MkPreset {
            if !a.path.trim().is_empty() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.assets.194",
                    &[("v1", &a.id.to_string())],
                ))
                .with_detail(
                    "MKP 预设的文件是生成产物（落点由命名规则算出，doc §12.5）—— \
                     台账里存一份路径就是第二份会过期的真相。归属写 machineId + versionId。"
                        .to_owned(),
                ));
            }
            if a.machine_id.as_deref().unwrap_or("").trim().is_empty()
                || a.version_id.as_deref().unwrap_or("").trim().is_empty()
            {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.presetdata.assets.195",
                    &[("v1", &a.id.to_string())],
                ))
                .with_detail(
                    "这一类靠「机型 + 版本」定位那一版的产物（两个字段都必填）—— \
                     少了它就不知道这份登记说的是哪一版"
                        .to_owned(),
                ));
            }
        } else {
            check_relative(a.path.trim()).map_err(|e| {
                AppError::invalid_argument(messages::text(
                    "ui.presetdata.assets.196",
                    &[("v1", &a.id.to_string()), ("v2", &a.path.to_string())],
                ))
                .with_detail(e.to_string())
            })?;
        }

        match a.kind {
            AssetKind::SlicerProfile => {
                if a.slicer.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.assets.197",
                        &[("v1", &a.id.to_string())],
                    ))
                    .with_detail(
                        "slicer 是 G-3 留的开放维度（今天只有 `bbs`）—— 缺了它这份预设\
                         就不知道该给谁读"
                            .to_owned(),
                    ));
                }
            }
            AssetKind::MkPreset => {
                // 专有字段是 machineId + versionId（上面查过了），slicer/profile 一律不写
                if a.slicer.is_some() || a.profile.is_some() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.assets.198",
                        &[("v1", &a.id.to_string())],
                    ))
                    .with_detail(::postprocess::diag::messages::text(
                        "ui.presetdata.why.slicer_only",
                        &[],
                    )));
                }
            }
            _ => {
                if a.slicer.is_some() || a.profile.is_some() {
                    return Err(AppError::invalid_argument(messages::text(
                        "ui.presetdata.assets.199",
                        &[("v1", &a.id.to_string())],
                    ))
                    .with_detail(::postprocess::diag::messages::text(
                        "ui.presetdata.why.slicer_only",
                        &[],
                    )));
                }
            }
        }
        Ok(())
    }

    /// **归属机型必须是真机型**（跨文件那条，由 [`super::Presets`] 在加载后调用）。
    ///
    /// 与 `machineVariants` 的键写错同一类错：写一个不存在的机型 id，界面上只会表现为
    /// "这台机型的图没了"，而没有任何东西报错
    pub fn check_against_machines(&self, machines: &[&str]) -> Result<(), AppError> {
        for a in &self.items {
            let Some(m) = a.machine_id.as_deref() else {
                continue;
            };
            if !machines.contains(&m) {
                return Err(AppError::corrupted(messages::text(
                    "ui.presetdata.assets.200",
                    &[("v1", &a.id.to_string()), ("m", m)],
                ))
                .with_detail(::postprocess::diag::messages::text(
                    "ui.presetdata.why.machine_list",
                    &[],
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 写一份探针定义。**走仓库那条唯一写盘出口** —— 连测试也不破例，
    /// `clippy.toml` 的 `disallowed-methods` 对本 crate 全量生效
    fn put(dir: &Path, text: impl AsRef<[u8]>) {
        crate::fsx::atomic::atomic_write(&dir.join(ASSETS_FILE), text.as_ref())
            .expect("写探针定义");
    }

    fn fixture(text: &str) -> (tempfile::TempDir, Assets) {
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let assets = Assets::load_from(dir.path()).expect("探针应当读得通");
        (dir, assets)
    }

    fn one(id: &str, kind: &str, extra: &str) -> String {
        format!("[[assets]]\nid = '{id}'\ntype = '{kind}'\nname = 'n'\npath = 'p.webp'\n{extra}")
    }

    /// 形状：`[[assets]]` 一条一条读得出来，可选字段缺了不影响
    #[test]
    fn reads_a_minimal_definition() {
        let (_d, a) = fixture(&one("a1-image", "image", "machineId = 'A1'\n"));
        assert_eq!(a.items().len(), 1);
        let it = a.get("a1-image").expect("按 id 取得到");
        assert_eq!(it.kind, AssetKind::Image);
        assert_eq!(it.machine_id.as_deref(), Some("A1"));
        assert!(it.slicer.is_none());
    }

    /// **id 唯一，大小写不敏感**（Task 8.8 第一条）
    #[test]
    fn ids_are_unique_case_insensitively() {
        let text = format!(
            "{}{}",
            one("a1-image", "image", ""),
            one("A1-Image", "image", "")
        );
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("只差大小写的两个 id 必须被拦");
        assert!(
            err.message.contains("撞了"),
            "错误信息要指名是什么撞了，实测：{}",
            err.message
        );
    }

    /// **path 必须落在资产根内**（Task 8.8 第二条）
    #[test]
    fn path_must_stay_inside_the_asset_root() {
        for bad in ["../secret.webp", "/etc/passwd", "a/../../b.webp"] {
            let text =
                format!("[[assets]]\nid = 'x'\ntype = 'image'\nname = 'n'\npath = '{bad}'\n");
            let dir = tempfile::tempdir().expect("临时目录");
            put(dir.path(), text);
            let err = Assets::load_from(dir.path()).expect_err(&format!("{bad} 应当被拦下来"));
            assert!(
                err.message.contains("越界"),
                "{bad} 的报错要说清是越界，实测：{}",
                err.message
            );
        }
    }

    /// 空 path 也是错误（`resolve_in` 的第一道）
    #[test]
    fn an_empty_path_is_refused() {
        let text = "[[assets]]\nid = 'x'\ntype = 'image'\nname = 'n'\npath = ''\n";
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        assert!(Assets::load_from(dir.path()).is_err());
    }

    /// 切片器预设必须写 `slicer`；别的类型不许写它
    #[test]
    fn slicer_fields_belong_to_slicer_profiles_only() {
        let text = one("a1-bbs", "slicerProfile", "");
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("缺 slicer 必须被拦");
        assert!(err.message.contains("slicer"), "实测：{}", err.message);

        let text = one("a1-image", "image", "slicer = 'bbs'\n");
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("非切片器类型不许带 slicer");
        assert!(err.message.contains("slicer"), "实测：{}", err.message);

        let (_d, a) = fixture(&one(
            "a1-bbs",
            "slicerProfile",
            "slicer = 'bbs'\nprofile = 'process'\n",
        ));
        assert_eq!(a.items()[0].slicer.as_deref(), Some("bbs"));
    }

    /// **键名写错要响亮**：`pth` 会让它变成"没有 path"，而不是悄悄少一个字段
    #[test]
    fn a_typo_in_a_key_is_loud() {
        let text = "[[assets]]\nid = 'x'\ntype = 'image'\nname = 'n'\npth = 'p.webp'\n";
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        assert!(
            Assets::load_from(dir.path()).is_err(),
            "少写一个字段名必须报错，不能当默认值放过去"
        );
    }

    /// 归属机型必须是真机型（跨文件那条）
    #[test]
    fn the_machine_attribution_must_be_a_real_machine() {
        let (_d, a) = fixture(&one("a1-image", "image", "machineId = 'A1'\n"));
        assert!(a.check_against_machines(&["A1", "P1S"]).is_ok());
        let err = a
            .check_against_machines(&["P1S"])
            .expect_err("归属一台不存在的机型必须被拦");
        assert!(err.message.contains("A1"), "实测：{}", err.message);
    }

    /// **零编辑往返逐字节相同**（保格式写回的底）
    #[test]
    fn a_zero_edit_roundtrip_is_byte_identical() {
        let dir = tempfile::tempdir().expect("临时目录");
        let text = format!(
            "{}{}",
            one("a1-image", "image", "machineId = 'A1'\n"),
            one(
                "p1s-bbs",
                "slicerProfile",
                "slicer = 'bbs'\nprofile = 'process'\n"
            )
        );
        put(dir.path(), &text);
        let a = Assets::load_from(dir.path()).expect("读得通");
        assert_eq!(a.to_toml(), text, "没改过就必须逐字节相同");
    }

    /// `add` 只追加一条，别处一个字节不动；落盘后重读得到同样的结果
    #[test]
    fn add_appends_one_block_and_writes_once() {
        let (_d, mut a) = fixture(&one("a1-image", "image", "machineId = 'A1'\n"));
        let before = a.to_toml();

        a.add(Asset {
            id: "p1s-icon".to_owned(),
            kind: AssetKind::Icon,
            machine_id: Some("P1S".to_owned()),
            name: "P1S 图标".to_owned(),
            path: "icons/p1s.svg".to_owned(),
            version_id: None,
            slicer: None,
            profile: None,
            delivery: Delivery::Download,
            calibration: String::new(),
        })
        .expect("加一条");
        let after = a.to_toml();

        let (cut, put) = super::super::one_edit_only(&before, &after);
        assert!(cut.is_empty(), "不该删掉任何东西，实测删了：{cut:?}");
        assert!(
            put.contains("p1s-icon") && put.contains("icons/p1s.svg"),
            "插入的那一段应当就是新条目，实测：{put:?}"
        );

        a.add(Asset {
            id: "a1-image".to_owned(),
            kind: AssetKind::Image,
            machine_id: None,
            name: "重名".to_owned(),
            path: "x.webp".to_owned(),
            version_id: None,
            slicer: None,
            profile: None,
            delivery: Delivery::Bundled,
            calibration: String::new(),
        })
        .expect_err("同 id 再插一条必须被拦");

        // **`add` 与加载期走同一套检查**：越界的 path 在加法上也要被拦下来，
        // 否则"手写的过不了加载、程序加的写得进去"，两条路会分岔
        a.add(Asset {
            id: "escape".to_owned(),
            kind: AssetKind::Image,
            machine_id: None,
            name: "越界".to_owned(),
            path: "../outside.webp".to_owned(),
            version_id: None,
            slicer: None,
            profile: None,
            delivery: Delivery::Download,
            calibration: String::new(),
        })
        .expect_err("越界 path 必须被拦");
        assert_eq!(a.items().len(), 2, "被拦下的两条都不该进内存");

        // 落到盘上再读回来：条目数与内容都对得上
        a.write().expect("写");
        let again = Assets::load_from(a.file().parent().expect("有父目录")).expect("重读");
        assert_eq!(again.items().len(), 2);
        assert_eq!(
            again.get("P1S-ICON").map(|x| x.kind),
            Some(AssetKind::Icon),
            "重读之后按**大小写不敏感**的 id 仍然取得到"
        );
    }

    /// 真仓库那份骨架：读得通、零编辑往返逐字节相同
    #[test]
    fn the_real_assets_file_loads_and_roundtrips() {
        let Some(root) = crate::presetdata::repo_presets_root() else {
            panic!("找不到 <repo>/presets —— 这条判据不能跳过");
        };
        let a = Assets::load_from(&root).expect("presets/assets.toml 必须读得通");
        let on_disk = std::fs::read_to_string(a.file()).expect("读原文");
        assert_eq!(a.to_toml(), on_disk, "零编辑往返必须逐字节相同");
        // 反空转：骨架里现在是空的（Task 9 才灌条目），但**文件本身必须在**
        assert!(
            a.file().is_file(),
            "presets/assets.toml 应当存在 —— 它是资产域①层的定义文件"
        );
    }

    /* ---------------- 分组（`[[groups]]`，2026-10-16） ---------------- */

    /// 旧口径那条——抹掉浅 / 深后缀。**它现在只是兜底**，但算法本身不能变
    #[test]
    fn pair_stem_strips_the_theme_suffix() {
        assert_eq!(pair_stem("open-with-light"), "open-with");
        assert_eq!(pair_stem("open-with-dark"), "open-with");
        assert_eq!(pair_stem("open-with_light"), "open-with");
        assert_eq!(pair_stem("open-with"), "open-with", "没有浅深后缀就原样");
    }

    /// `anno_stem_of`：**有组 → 组 id；没组 → 旧口径兜底**
    #[test]
    fn the_source_stem_comes_from_the_group_then_falls_back_to_the_name() {
        let text = "[[assets]]\nid = 'g-light'\ntype = 'guide'\nname = 'n'\npath = 'guides/x-light.png'\n\
                    [[assets]]\nid = 'g-dark'\ntype = 'guide'\nname = 'n'\npath = 'guides/x-dark.png'\n\
                    [[assets]]\nid = 'lonely'\ntype = 'guide'\nname = 'n'\npath = 'guides/y-dark.png'\n\
                    [[groups]]\nid = 'x'\nname = 'X 那套'\n\
                    [[groups.members]]\nasset = 'g-light'\nrole = 'light'\n\
                    [[groups.members]]\nasset = 'g-dark'\nrole = 'dark'\n";
        let (_d, a) = fixture(text);
        assert_eq!(
            a.anno_stem_of("g-light").as_deref(),
            Some("x"),
            "进组的用组 id"
        );
        assert_eq!(a.anno_stem_of("g-dark").as_deref(), Some("x"));
        assert_eq!(
            a.anno_stem_of("lonely").as_deref(),
            Some("y"),
            "没进组的走旧口径（抹掉 `-dark`）"
        );
        assert_eq!(
            a.group_of("g-light").map(|g| g.name.as_str()),
            Some("X 那套")
        );
        assert!(a.group_of("lonely").is_none());
    }

    /// **同一组的成员必须在同一个目录**（否则那份源稿不知道落哪）
    #[test]
    fn members_of_one_group_must_share_a_directory() {
        let text = "[[assets]]\nid = 'a'\ntype = 'guide'\nname = 'n'\npath = 'guides/a.png'\n\
                    [[assets]]\nid = 'b'\ntype = 'guide'\nname = 'n'\npath = 'printers/b.png'\n\
                    [[groups]]\nid = 'mix'\nname = '混的'\n\
                    [[groups.members]]\nasset = 'a'\n\
                    [[groups.members]]\nasset = 'b'\n";
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("跨目录的组必须被拦");
        assert!(
            err.message.contains("不在同一个目录"),
            "实测：{}",
            err.message
        );
    }

    /// **一张资产至多属一个组**（否则"它跟谁共用标注"有两种答案）
    #[test]
    fn an_asset_belongs_to_at_most_one_group() {
        let text = "[[assets]]\nid = 'a'\ntype = 'guide'\nname = 'n'\npath = 'guides/a.png'\n\
                    [[groups]]\nid = 'one'\nname = '一'\n\
                    [[groups.members]]\nasset = 'a'\n\
                    [[groups]]\nid = 'two'\nname = '二'\n\
                    [[groups.members]]\nasset = 'a'\n";
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("一图两组必须被拦");
        assert!(
            err.message.contains("同时属于两个组"),
            "实测：{}",
            err.message
        );
    }

    /// **组 id 不许与同目录下某张未分组资产的旧口径干名撞**（两份源稿会同名）
    #[test]
    fn a_group_id_may_not_collide_with_an_ungrouped_stem() {
        let text =
            "[[assets]]\nid = 'a'\ntype = 'guide'\nname = 'n'\npath = 'guides/x-light.png'\n\
                    [[assets]]\nid = 'b'\ntype = 'guide'\nname = 'n'\npath = 'guides/b.png'\n\
                    [[groups]]\nid = 'x'\nname = '撞的'\n\
                    [[groups.members]]\nasset = 'b'\n";
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), text);
        let err = Assets::load_from(dir.path()).expect_err("撞名必须被拦");
        assert!(err.message.contains("撞"), "实测：{}", err.message);
    }

    /// 坏值**整次拒绝、内存与盘上都不留痕**；写下去的那份能被重读、且带 `[[groups]]`
    #[test]
    fn upserting_a_group_is_all_or_nothing_and_lands_in_the_file() {
        let text = "[[assets]]\n# 这行注释必须活下来（改分组不许动 assets 那一段）\nid = 'a'\n\
                    type = 'guide'\nname = 'n'\npath = 'guides/a.png'\n\
                    [[assets]]\nid = 'b'\ntype = 'guide'\nname = 'n'\npath = 'guides/b-dark.png'\n";
        let (_d, mut a) = fixture(text);

        // 坏值：成员指向不存在的资产 ⇒ 拒绝，而且一条都不留下
        let bad = Group {
            id: "g".into(),
            name: "G".into(),
            members: vec![GroupMember {
                asset: "nope".into(),
                role: String::new(),
            }],
        };
        assert!(a.upsert_group(bad).is_err());
        assert!(a.groups().is_empty(), "拒绝之后内存里不许留半条");

        // 好值：写下去、重读得到、`role` 保住、`members` 顺序保住
        a.upsert_group(Group {
            id: "g".into(),
            name: "G 那套".into(),
            members: vec![
                GroupMember {
                    asset: "a".into(),
                    role: "light".into(),
                },
                GroupMember {
                    asset: "b".into(),
                    role: "dark".into(),
                },
            ],
        })
        .expect("好值该过");
        a.write().expect("写");
        let again = Assets::load_from(a.file().parent().expect("有父目录")).expect("重读");
        assert_eq!(again.groups().len(), 1);
        assert_eq!(again.groups()[0].id, "g");
        assert_eq!(again.groups()[0].members[0].role, "light");
        assert_eq!(again.groups()[0].members[1].asset, "b");
        assert_eq!(again.anno_stem_of("a").as_deref(), Some("g"));
        assert!(
            again.to_toml().contains("# 这行注释必须活下来"),
            "改分组不许动 `[[assets]]` 那一段（注释也得原样）"
        );

        // 删组：成员原地变回"未分组"，资产一条不动
        let mut again = again;
        again.remove_group("g").expect("删组");
        assert!(again.groups().is_empty());
        assert_eq!(again.items().len(), 2, "删组不许动资产");
    }
}
