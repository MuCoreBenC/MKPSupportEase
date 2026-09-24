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
//! 不在本文件旁边，而在**资产根** `<repo>/public/assets/` 下（[`paths::assets_root`]）。
//! 条目里的 `path` **永远相对资产根**，不相对本文件所在目录 —— 这条是刻意的：
//! 定义（①层，`presets/`）与载荷（②层，`public/assets/`）本来就分家，路径的基准只有一处。
//!
//! # 加载期就查掉的两条（Task 8.8）
//!
//! 1. **id 唯一且大小写不敏感** —— 只差大小写的两个 id 会让"查表结果"取决于顺序；
//! 2. **`path` 落在资产根内** —— 走 [`resolve_in`] 那三道闸（拒绝对路径 / 拒绝 `..` /
//!    比真实路径防符号链接）。
//!
//! 这两条坏掉的表现都**不是报错**，而是"某个地方少了一张图"。所以它们在加载期就拦，
//! 而不是等某个页面渲染时才发现。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::fsx::paths::resolve_in;
use crate::workbench::paths;

/// 定义文件（`presets/` 下）
pub const ASSETS_FILE: &str = "assets.toml";

/// 资产类型集合（闸 G-3：BBS 纳入、切片器维度**开放**；模型保留）。
///
/// 四个取值而不是三个：`image` 与 `icon` 分开是**消费方式**不同（一个是机型图，
/// 一个是矢量标记，前端一处 <img>、一处当符号用），而 doc §8 那句"图片 / 图标、模型"
/// 说的是**同一类文件形态**，不是同一个字段语义。
///
/// **没有 `mkpPreset`**：那一条按 doc §12.5 不建条目（见模块头）。这个 enum 是那句话的
/// 结构化版本 —— 想登记 MKP 预设得先改这里，而改这里要过一次 review。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Image,
    Icon,
    Model,
    /// 切片器预设。今天只有 BBS，`slicer` 字段留着别的切片器的位置
    SlicerProfile,
}

impl AssetKind {
    /// 写进 TOML 的那个字面量（与 serde 的 camelCase 一致）
    pub fn key(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Icon => "icon",
            Self::Model => "model",
            Self::SlicerProfile => "slicerProfile",
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
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slicer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// 文件形状。`[[assets]]` 数组表，与 `brands.toml` 那份"一份集中定义"同一写法。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssetsFile {
    #[serde(default)]
    assets: Vec<Asset>,
}

/// 一次加载的资产定义。
///
/// 与 [`super::Catalog`] 同一套路：**留着原文文档**（`DocumentMut`），所以
/// "读进来不改再写出去"在构造上就是逐字节相同的，不靠谁记得别动格式。
#[derive(Debug)]
pub struct Assets {
    items: Vec<Asset>,
    doc: DocumentMut,
    file: PathBuf,
}

impl Assets {
    /// 读 `<root>/assets.toml`。读不到 → 错误（**不用空数据装成能跑**：那会让人以为
    /// "我们一条资产都没有"，而那和"读不出来"是两件事 —— 与 [`super::Presets::load`] 同一口径）
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join(ASSETS_FILE);
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let parsed: AssetsFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(format!("{} 读不成资产定义", file.display()))
                .with_detail(e.to_string())
        })?;

        let out = Self {
            items: parsed.assets,
            doc,
            file,
        };
        out.check()?;
        Ok(out)
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

    /// 一个条目的**真实路径**：资产根 + `path`，走 [`resolve_in`] 的三道闸。
    pub fn resolve(&self, asset: &Asset) -> Result<PathBuf, AppError> {
        let root = paths::assets_root()?;
        resolve_in(&root, asset.path.trim()).map_err(|e| {
            AppError::invalid_argument(format!("资产 {} 的 path 越界：{}", asset.id, asset.path))
                .with_detail(format!(
                    "path 必须落在资产根（{}）之内，且只能是相对路径；原错误：{e}",
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
            return Err(
                AppError::invalid_argument(format!("资产 id 已经存在：{}", prev.id)).with_detail(
                    "id 是主键，且大小写不敏感 —— 换一个 id，或先删掉原来那条".to_owned(),
                ),
            );
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
            AppError::corrupted(format!("{} 的 assets 不是表数组", self.file.display()))
        })?;
        arr.push(t);

        self.items.push(candidate);
        Ok(())
    }

    /// 原子写回。**唯一的写盘点**（与 [`super::Catalog::write_machine`] 同一条出口）
    pub fn write(&self) -> Result<(), AppError> {
        crate::fsx::atomic::atomic_write(&self.file, self.to_toml().as_bytes())
    }

    /// 加载期那两条（Task 8.8）+ 类型与专有字段的搭配
    fn check(&self) -> Result<(), AppError> {
        for a in &self.items {
            if a.id.trim().is_empty() {
                return Err(AppError::corrupted(format!(
                    "{} 里有一条资产没有 id",
                    self.file.display()
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
                return Err(
                    AppError::corrupted(format!("资产 id 撞了：{} 与 {}", prev, a.id)).with_detail(
                        "id 是主键，且**大小写不敏感** —— 只差大小写的两条会让查表结果取决于顺序"
                            .to_owned(),
                    ),
                );
            }
        }
        Ok(())
    }

    /// 单条的形状检查。`add` 与 `check` 共用一条 —— 两条路走同一套判据，
    /// 免得"手写的能过、程序加的过不了"或者反过来
    fn check_one(&self, a: &Asset) -> Result<(), AppError> {
        if a.id.trim().is_empty() {
            return Err(AppError::invalid_argument("资产 id 不能为空"));
        }
        if a.name.trim().is_empty() {
            return Err(AppError::invalid_argument(format!(
                "资产 {} 没有 name —— 界面上要显示的就是它",
                a.id
            )));
        }
        // 路径：走那道防穿越闸（顺带证明它不是空的）
        let _ = self.resolve(a)?;

        match a.kind {
            AssetKind::SlicerProfile => {
                if a.slicer.as_deref().unwrap_or("").trim().is_empty() {
                    return Err(AppError::invalid_argument(format!(
                        "资产 {} 是切片器预设，但没有写 slicer",
                        a.id
                    ))
                    .with_detail(
                        "slicer 是 G-3 留的开放维度（今天只有 `bbs`）—— 缺了它这份预设\
                         就不知道该给谁读"
                            .to_owned(),
                    ));
                }
            }
            _ => {
                if a.slicer.is_some() || a.profile.is_some() {
                    return Err(AppError::invalid_argument(format!(
                        "资产 {} 不是切片器预设，却写了 slicer / profile",
                        a.id
                    ))
                    .with_detail("这两项只对 `type = 'slicerProfile'` 有意义".to_owned()));
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
                return Err(AppError::corrupted(format!(
                    "资产 {} 归属机型 {m}，但没有这台机型",
                    a.id
                ))
                .with_detail("机型清单在 presets/machines/*.toml".to_owned()));
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
            slicer: None,
            profile: None,
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
            slicer: None,
            profile: None,
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
            slicer: None,
            profile: None,
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
        let Some(root) = crate::workbench::paths::presets_root() else {
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
}
