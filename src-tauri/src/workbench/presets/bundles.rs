//! 套餐域①层：`presets/bundles.toml` —— **一份集中定义**（b05 Task 10）。
//!
//! # 套餐是什么
//!
//! **套餐的内容就是 BBS 引用**（doc §12.4 的决定性事实，五个机型各一份、
//! `assetRefs` 里各一条 BBS 预设）。它不是"预设的集合"——预设的路径由命名规则算出
//! （doc §12.5：不为 MKP 预设建条目），套餐管的是**配发内容**。
//!
//! # 为什么必须成套配发
//!
//! MKP 侧的 `use_ironing_path = true`（拿熨烫路径当涂胶路径）只在 BBS 侧
//! `enable_support_ironing = "1"` 与那一组 `support_ironing_*` 成立时才有意义
//! （doc §12.4）。**发了 MKP 预设不发配套 BBS 预设，用户打出来的结果是错的** ——
//! 所以加载期就拦「套餐里一条 BBS 都没有」，见 [`super::Presets::check_bundle_refs`]。
//!
//! # 字段就是旧数据那五个，一个不多一个不少
//!
//! 旧仓 `source/bundles/*_default.toml` 实测：`id` / `display` / `machineId` /
//! `assetRefs` / `updatedAt`。这一版**照实搬**（裁决：先确认真实结构，不造默认值）。
//!
//! 刻意**不写**的：
//!
//! - **不写 MKP 预设的路径**：那条路径由命名规则算出（doc §12.5），登记一份就是冗余；
//! - **不写 `nozzle` / `layerHeight`**：旧仓那份 `source/preset_registry.toml` 有它们，
//!   但那是**交付索引**要用的字段（消费端按喷头/层高挑预设），属于 Task 12 ——
//!   在这里再存一份就是第二份真相，而且它一定先过期。
//!
//! # id 为什么还是旧写法（`A1_default`）
//!
//! 它被机型文件引用着（`defaultBundle` 五处、每个版本的 `recommendedBundle` 一处，
//! 实测 14 处非空引用；A2L 那处空串不算）。**改名要有命名规则**，而 G-0 那套管的是
//! **产物文件名**，不是套餐 id —— 没有规则就换个写法，只会让那 14 处引用跟着漂。
//! 等 Task 12 定交付命名时一起谈。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use toml_edit::DocumentMut;

use crate::error::AppError;

/// 定义文件（`presets/` 下）
pub const BUNDLES_FILE: &str = "bundles.toml";

/// 一条套餐定义。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Bundle {
    /// 主键。机型文件与版本定义引用它（`defaultBundle` / `recommendedBundle`）
    pub id: String,
    /// 给人看的名字（旧数据里五份都是 `官方推荐`）
    pub display: String,
    /// 归属机型。**必填**（套餐是按机型配的：实测五份一对一）
    pub machine_id: String,
    /// 引用的资产 id 列表。**至少一条，且至少一条是 BBS 预设**（见模块头）
    #[serde(default)]
    pub asset_refs: Vec<String>,
    /// 上一次改动日期（旧数据里是 `2026-07-12`）。
    /// **迁移不改内容，所以日期照旧** —— 写上今天就是把"搬了个文件"记成"改了套餐"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

/// 文件形状：`[[bundles]]` 数组表，与 `assets.toml` / `brands.toml` 同一写法
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BundlesFile {
    #[serde(default)]
    bundles: Vec<Bundle>,
}

/// 一次加载的套餐定义（保原文，写法与 [`super::Assets`] 同）
#[derive(Debug)]
pub struct Bundles {
    items: Vec<Bundle>,
    doc: DocumentMut,
    file: PathBuf,
}

impl Bundles {
    /// 读 `<root>/bundles.toml`。读不到 → 错误（不用空数据装成能跑）
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let file = root.join(BUNDLES_FILE);
        let text = super::read(&file)?;
        let doc = super::parse_text(&text, &file)?;
        let parsed: BundlesFile = toml_edit::de::from_str(&text).map_err(|e| {
            AppError::corrupted(format!("{} 读不成套餐定义", file.display()))
                .with_detail(e.to_string())
        })?;

        let out = Self {
            items: parsed.bundles,
            doc,
            file,
        };
        out.check()?;
        Ok(out)
    }

    pub fn items(&self) -> &[Bundle] {
        &self.items
    }

    /// 按 id 取，**大小写不敏感**（与 [`super::Assets::get`] 同一口径）
    pub fn get(&self, id: &str) -> Option<&Bundle> {
        let want = id.trim().to_lowercase();
        self.items.iter().find(|b| b.id.to_lowercase() == want)
    }

    /// 写回用的文本。**没改过就逐字节等于读进来那份**
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// 新增一条，由调用方决定何时落盘（[`Self::write`]）。
    ///
    /// 与 [`super::Assets::add`] 同一条纪律：先整套查一遍，不合格就一个字节都不写。
    /// **这里不查"资产存不存在"** —— 那要看得见 assets，在
    /// [`super::Presets::check_bundle_refs`] 里查（手写的与程序加的走同一套跨文件检查）
    pub fn add(&mut self, bundle: Bundle) -> Result<(), AppError> {
        let candidate = Bundle {
            id: bundle.id.trim().to_owned(),
            display: bundle.display.trim().to_owned(),
            machine_id: bundle.machine_id.trim().to_owned(),
            asset_refs: bundle
                .asset_refs
                .iter()
                .map(|s| s.trim().to_owned())
                .collect(),
            updated_at: bundle.updated_at,
        };
        if let Some(prev) = self.get(&candidate.id) {
            return Err(
                AppError::invalid_argument(format!("套餐 id 已经存在：{}", prev.id)).with_detail(
                    "id 是主键，且大小写不敏感 —— 换一个 id，或先删掉原来那条".to_owned(),
                ),
            );
        }
        self.check_one(&candidate)?;

        let mut t = toml_edit::Table::new();
        t["id"] = super::literal_str(&candidate.id);
        t["display"] = super::literal_str(&candidate.display);
        t["machineId"] = super::literal_str(&candidate.machine_id);
        let mut refs = toml_edit::Array::new();
        for r in &candidate.asset_refs {
            refs.push(r.as_str());
        }
        t["assetRefs"] = toml_edit::value(refs);
        if let Some(d) = &candidate.updated_at {
            t["updatedAt"] = super::literal_str(d);
        }

        let entry = self
            .doc
            .entry("bundles")
            .or_insert(toml_edit::Item::ArrayOfTables(
                toml_edit::ArrayOfTables::new(),
            ));
        let arr = entry.as_array_of_tables_mut().ok_or_else(|| {
            AppError::corrupted(format!("{} 的 bundles 不是表数组", self.file.display()))
        })?;
        arr.push(t);

        self.items.push(candidate);
        Ok(())
    }

    /// 原子写回（与 [`super::Assets::write`] 同一条出口）
    pub fn write(&self) -> Result<(), AppError> {
        crate::fsx::atomic::atomic_write(&self.file, self.to_toml().as_bytes())
    }

    /// 把某个资产从**每一条**套餐的 `assetRefs` 里去掉（反查之下的收尾动作）。
    ///
    /// 与 [`Self::add`] 同一条纪律：**先把所有套餐查一遍再动手**，任何一条不合格
    /// 就一个字节都不改。这里要拦的是「去掉之后套餐就空了」—— 那等于交付了一半
    /// （doc §12.4），与加载期拦的空 `assetRefs` 是同一条判据的两端。
    ///
    /// 返回去掉的**引用数**（不是套餐数）。没命中（返回 0）时文件一个字节都不动。
    pub fn drop_asset_refs(&mut self, asset_id: &str) -> Result<usize, AppError> {
        let want = asset_id.trim().to_lowercase();
        for b in &self.items {
            let has = b.asset_refs.iter().any(|r| r.trim().to_lowercase() == want);
            let rest = b
                .asset_refs
                .iter()
                .filter(|r| r.trim().to_lowercase() != want)
                .count();
            if has && rest == 0 {
                return Err(AppError::invalid_argument(format!(
                    "套餐 {} 的 assetRefs 只剩 {asset_id} 这一条 —— 去掉它套餐就空了",
                    b.id
                ))
                .with_detail(
                    "先把别的 BBS 预设配进来（MKP 与 BBS 必须成套配发，doc §12.4），\
                     或者整条删掉这份套餐"
                        .to_owned(),
                ));
            }
        }

        let mut hit = 0usize;

        for (idx, b) in self.items.iter_mut().enumerate() {
            let before = b.asset_refs.len();
            b.asset_refs.retain(|r| r.trim().to_lowercase() != want);
            if b.asset_refs.len() != before {
                hit += before - b.asset_refs.len();
                let id = b.id.clone();
                // 文档面一起改：只改内存的话，下一次 `write` 会把它写回来
                let arr = self
                    .doc
                    .get_mut("bundles")
                    .and_then(|i| i.as_array_of_tables_mut())
                    .ok_or_else(|| {
                        AppError::corrupted(format!(
                            "{} 的 bundles 不是表数组",
                            self.file.display()
                        ))
                    })?;
                let table = arr.get_mut(idx).ok_or_else(|| {
                    AppError::corrupted(format!("文档里没有第 {idx} 条套餐（内存里有 {id}）"))
                })?;
                let refs = table
                    .get_mut("assetRefs")
                    .and_then(|i| i.as_array_mut())
                    .ok_or_else(|| {
                        AppError::corrupted(format!("套餐 {id} 的 assetRefs 不是数组"))
                    })?;
                refs.retain(|v| v.as_str().is_some_and(|s| s.trim().to_lowercase() != want));
            }
        }
        Ok(hit)
    }

    /// 加载期：这套定义自己站不站得住
    fn check(&self) -> Result<(), AppError> {
        for b in &self.items {
            self.check_one(b)?;
        }
        let mut seen: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for b in &self.items {
            let lower = b.id.to_lowercase();
            if let Some(prev) = seen.insert(lower, b.id.clone()) {
                return Err(
                    AppError::corrupted(format!("套餐 id 撞了：{} 与 {}", prev, b.id))
                        .with_detail("id 是主键，且**大小写不敏感**".to_owned()),
                );
            }
        }
        Ok(())
    }

    /// 单条的形状检查（`add` 与 `check` 共用同一条，免得两条路分岔）
    fn check_one(&self, b: &Bundle) -> Result<(), AppError> {
        if b.id.trim().is_empty() {
            return Err(AppError::invalid_argument("套餐 id 不能为空"));
        }
        if b.display.trim().is_empty() {
            return Err(AppError::invalid_argument(format!(
                "套餐 {} 没有 display —— 界面上要显示的就是它",
                b.id
            )));
        }
        if b.machine_id.trim().is_empty() {
            return Err(
                AppError::invalid_argument(format!("套餐 {} 没有 machineId", b.id))
                    .with_detail("套餐是按机型配的，缺了它就没法判断「这份套餐给谁用」".to_owned()),
            );
        }
        if b.asset_refs.is_empty() {
            return Err(
                AppError::invalid_argument(format!("套餐 {} 的 assetRefs 是空的", b.id))
                    .with_detail(
                        "套餐的内容就是 BBS 引用（doc §12.4）：空的套餐等于交付了一半 —— \
                 MKP 预设与 BBS 预设必须成套配发"
                            .to_owned(),
                    ),
            );
        }
        let mut seen: Vec<String> = Vec::new();
        for r in &b.asset_refs {
            let key = r.trim().to_lowercase();
            if key.is_empty() {
                return Err(AppError::invalid_argument(format!(
                    "套餐 {} 的 assetRefs 里有一项是空的",
                    b.id
                ))
                .with_detail(
                    "要「没有」就整条去掉，别留空串 —— 空串读成「填过但填了个空」".to_owned(),
                ));
            }
            if seen.contains(&key) {
                return Err(AppError::invalid_argument(format!(
                    "套餐 {} 的 assetRefs 里有重复项：{r}",
                    b.id
                )));
            }
            seen.push(key);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(dir: &Path, text: impl AsRef<[u8]>) {
        crate::fsx::atomic::atomic_write(&dir.join(BUNDLES_FILE), text.as_ref()).expect("写探针");
    }

    fn one(id: &str, refs: &str) -> String {
        format!(
            "[[bundles]]\nid = '{id}'\ndisplay = '官方推荐'\nmachineId = 'A1'\n\
             assetRefs = [{refs}]\nupdatedAt = '2026-07-12'\n"
        )
    }

    /// 形状：一条读得出来，可选字段缺了不影响
    #[test]
    fn reads_a_minimal_definition() {
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), one("A1_default", "'a1-bbs-04-020'"));
        let b = Bundles::load_from(dir.path()).expect("读得通");
        assert_eq!(b.items().len(), 1);
        let it = b.get("a1_default").expect("按 id 取得到（大小写不敏感）");
        assert_eq!(it.machine_id, "A1");
        assert_eq!(it.asset_refs, vec!["a1-bbs-04-020".to_owned()]);
        assert_eq!(it.updated_at.as_deref(), Some("2026-07-12"));
    }

    /// **空 `assetRefs` 是错误**（10.8：套餐的内容就是 BBS 引用）
    #[test]
    fn an_empty_ref_list_is_refused() {
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), one("A1_default", ""));
        let err = Bundles::load_from(dir.path()).expect_err("空套餐必须被拦");
        assert!(
            err.message.contains("assetRefs 是空的"),
            "实测：{}",
            err.message
        );
    }

    /// 缺 `machineId` 是错误；`assetRefs` 里的空串与重复项都要拦
    #[test]
    fn shape_rules_are_enforced() {
        let dir = tempfile::tempdir().expect("临时目录");
        put(
            dir.path(),
            "[[bundles]]\nid = 'x'\ndisplay = '官方推荐'\nassetRefs = ['a']\n",
        );
        assert!(
            Bundles::load_from(dir.path()).is_err(),
            "缺 machineId 必须报错"
        );

        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), one("A1_default", "'a1-bbs-04-020', ''"));
        let err = Bundles::load_from(dir.path()).expect_err("空串项必须被拦");
        assert!(err.message.contains("空"), "实测：{}", err.message);

        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), one("A1_default", "'a', 'A'"));
        let err = Bundles::load_from(dir.path()).expect_err("重复项必须被拦");
        assert!(err.message.contains("重复"), "实测：{}", err.message);
    }

    /// id 唯一（大小写不敏感）
    #[test]
    fn ids_are_unique_case_insensitively() {
        let dir = tempfile::tempdir().expect("临时目录");
        let text = format!(
            "{}{}",
            one("A1_default", "'a1-bbs-04-020'"),
            one("A1_DEFAULT", "'a1-bbs-04-020'")
        );
        put(dir.path(), text);
        let err = Bundles::load_from(dir.path()).expect_err("撞 id 必须被拦");
        assert!(err.message.contains("撞了"), "实测：{}", err.message);
    }

    /// **零编辑往返逐字节相同**（保格式写回的底）
    #[test]
    fn a_zero_edit_roundtrip_is_byte_identical() {
        let dir = tempfile::tempdir().expect("临时目录");
        let text = one("A1_default", "'a1-bbs-04-020'");
        put(dir.path(), &text);
        let b = Bundles::load_from(dir.path()).expect("读得通");
        assert_eq!(b.to_toml(), text, "没改过就必须逐字节相同");
    }

    /// `add` 只追加一条、落盘后重读得到；重复 id 与空套餐都被拦
    #[test]
    fn add_appends_one_block_and_writes_once() {
        let dir = tempfile::tempdir().expect("临时目录");
        put(dir.path(), one("A1_default", "'a1-bbs-04-020'"));
        let mut b = Bundles::load_from(dir.path()).expect("读得通");
        let before = b.to_toml();

        b.add(Bundle {
            id: "P1S_default".to_owned(),
            display: "官方推荐".to_owned(),
            machine_id: "P1S".to_owned(),
            asset_refs: vec!["p1s-bbs-04-024".to_owned()],
            updated_at: Some("2026-07-12".to_owned()),
        })
        .expect("加一条");
        let after = b.to_toml();
        let (cut, put) = super::super::one_edit_only(&before, &after);
        assert!(cut.is_empty(), "不该删掉任何东西，实测删了：{cut:?}");
        assert!(put.contains("P1S_default"), "插入的那一段应当是它：{put:?}");

        b.add(Bundle {
            id: "A1_DEFAULT".to_owned(),
            display: "官方推荐".to_owned(),
            machine_id: "A1".to_owned(),
            asset_refs: vec!["a1-bbs-04-020".to_owned()],
            updated_at: None,
        })
        .expect_err("同 id 再插一条必须被拦");
        b.add(Bundle {
            id: "empty".to_owned(),
            display: "官方推荐".to_owned(),
            machine_id: "A1".to_owned(),
            asset_refs: Vec::new(),
            updated_at: None,
        })
        .expect_err("空套餐必须被拦");
        assert_eq!(b.items().len(), 2, "被拦下的两条都不该进内存");

        b.write().expect("写");
        let again = Bundles::load_from(dir.path()).expect("重读");
        assert_eq!(again.items().len(), 2);
        assert_eq!(
            again.get("p1s_default").map(|x| x.machine_id.as_str()),
            Some("P1S")
        );
    }

    /// **真仓库那份**：五份套餐读得通、零编辑往返逐字节相同
    #[test]
    fn the_real_bundles_file_loads_and_roundtrips() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            panic!("找不到 <repo>/presets —— 这条判据不能跳过");
        };
        let b = Bundles::load_from(&root).expect("presets/bundles.toml 必须读得通");
        assert_eq!(
            b.items().len(),
            5,
            "旧仓实测 5 份套餐（A1 / A1_MINI / P1S / P2S / X1C 各一）—— 条数变了就说清为什么"
        );
        let on_disk = std::fs::read_to_string(b.file()).expect("读原文");
        assert_eq!(b.to_toml(), on_disk, "零编辑往返必须逐字节相同");
    }

    /// `drop_asset_refs`：内存与**文档面**一起改，写回重读确实少了；
    /// 会造成**空套餐**的那次必须被整拦下。
    ///
    /// 为什么强调文档面：只改内存的话，下一次 `to_toml` 会把删掉的那条写回去 ——
    /// 而那要等到落盘之后（甚至下一次加载）才看得出来。
    #[test]
    fn dropping_an_asset_ref_updates_memory_and_document() {
        let dir = tempfile::tempdir().expect("临时目录");
        // A1_default 引了两条（去掉共享那条还有存量）；P1S_default 只有共享那一条
        let text = format!(
            "{}{}",
            one("A1_default", "'a1-bbs-04-020', 'a1-bbs-02-010'"),
            one("P1S_default", "'a1-bbs-04-020'")
        );
        put(dir.path(), &text);
        let mut b = Bundles::load_from(dir.path()).expect("读得通");

        // **先拦**：P1S_default 只剩这一条，去掉它套餐就空了 —— 与加载期拦
        // 空 `assetRefs` 是同一条判据的两端。整次操作一个字节都不动（A1_default
        // 明明可以去掉，也不许只改一半）
        let before = b.to_toml();
        let err = b
            .drop_asset_refs("a1-bbs-04-020")
            .expect_err("会造成空套餐的 drop 必须被拦");
        assert!(err.message.contains("就空了"), "实测：{}", err.message);
        assert_eq!(b.to_toml(), before, "被拦下就不该动文件");

        // **正路径**：drop 只在 A1_default 里的那条（去掉后它还剩一条存量）
        let hit = b.drop_asset_refs("A1-BBS-02-010").expect("去掉");
        assert_eq!(hit, 1, "返回的是去掉的引用数");
        assert!(
            !b.items()
                .iter()
                .any(|x| x.asset_refs.contains(&"a1-bbs-02-010".to_owned())),
            "内存里不该再有它"
        );
        assert!(
            !b.to_toml().contains("a1-bbs-02-010"),
            "文档面也要改，不然下次 write 会把它写回来：{}",
            b.to_toml()
        );
        assert_eq!(
            b.get("A1_default").expect("还在").asset_refs,
            vec!["a1-bbs-04-020".to_owned()],
            "别的引用一条不少"
        );

        b.write().expect("写");
        let again = Bundles::load_from(dir.path()).expect("重读");
        assert!(
            again
                .items()
                .iter()
                .all(|x| !x.asset_refs.contains(&"a1-bbs-02-010".to_owned())),
            "落盘之后重读也不该再有它"
        );

        // 零命中：没人引用的 id 返回 0，文件一个字节都不动
        let before = b.to_toml();
        let hit = b.drop_asset_refs("no-such-asset").expect("没人引用");
        assert_eq!(hit, 0);
        assert_eq!(b.to_toml(), before, "没命中就不该动文件");
    }
}
