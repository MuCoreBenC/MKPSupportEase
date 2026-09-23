//! 状态派生（doc §5）：**一份状态，一个算处。**
//!
//! # 输入三样，输出一份
//!
//! ```text
//! 上游（只读）  +  已落盘（workbench/）  +  草稿（.draft/book.json）
//!                        ↓
//!                     Book（本模块）
//!                        ↓
//!            BookView / Matrix / BuildRow / BbsRow
//! ```
//!
//! [`Book`] 是**借用的组装件**，不是缓存：它不持有任何一份数据的副本。
//! 第二版的病就是前端、快照文件、Rust 各存一套"当前状态"，于是三份会不一致，
//! 而不一致的表现是界面上某个数字对不上 —— 没有任何一步会报错。
//!
//! # 五张覆盖表怎么合（doc §3.6）
//!
//! 每次派生现场合成两张「我们写的」表：
//!
//! ```text
//! 我们写的机型基底 = machines/{机型}.json  ⊕  草稿里 m:{机型}:* 那些
//! 我们写的版本覆盖 = versions/{版本}.json  ⊕  草稿里 v:{uid}:* 那些
//! ```
//!
//! 草稿里值为 `None` 的那条表示「把这个键删掉」，合成时要真的删掉 ——
//! 当成"没有这一条"处理的话，「挂回继承」在保存之前看不出效果。
//!
//! # 「暂无资源」的判据，和「未生成」分开（doc §10.1、§11）
//!
//! 两者都"没有产物"，但要人做的事不同：
//!
//! | | 判据 | 用户该做什么 |
//! |---|---|---|
//! | **暂无资源** | 没有 MKP 产物 **且** 没有 BBS **且** 这一版的参数全是出厂默认 | 先写配方 |
//! | **未生成** | 有配方（我们写过，或上游给了机型差异），只是还没烤 | 点生成 |
//!
//! 第三个条件是关键：A2L 命中它（上游给它 0 项机型差异，我们也没写过），
//! 而"刚在 A1 下新建的一个版本"不命中（A1 的上游机型差异非空）——
//! 后者该显示「未生成」，让用户知道点生成就有了。
//!
//! 「未配置」是第三件事（有效配方为空），单独一个布尔位，**不占状态档**。
//! 实测它在真上游永远为假（74 个参数都有出厂默认），但上游把某台机型的字段全排除掉时
//! 就会为真，那时候界面得说得出「这台机型一个参数都没有」。
//!
//! # 矩阵的行列（doc §8.1）
//!
//! - **列序照配方本顺序，不按勾选顺序** —— 按点击顺序会让同一份数据每次长得不一样
//! - **行取并集不取交集** —— 交集会让"只有 P1S 有的那几条"在勾了 P1S 的时候凭空消失
//! - 单元格三种 kind（不适用 / G-code / 值）+ 一个 `editable`。
//!   doc §8.3 的第三条分支（选中了就升级成真控件）是**前端的事** ——
//!   后端不知道现在选中的是哪一格

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::workbench::presets::registry::ParamDef;
use crate::workbench::presets::Presets;
use crate::workbench::upstream::catalog::MkpPreset;
use crate::workbench::upstream::{Asset, ResourceType, Upstream};

use super::layer::{no_overrides, Layers, Level, Origin, Overrides};
use super::patch::{BuiltRecord, BundleEdit, CatalogMachine, Committed, Draft, Visibility};
use super::variants::digest;
use super::visibility::{BlockScope, BlockedBy, Gate};
use super::wording as w;
use super::wording::{ArtifactState, BbsAssign, BbsSource, BuildState, SaveState, SnapshotState};

/* ---------- 版本身份 ---------- */

/// 一个版本**现在**是什么样（草稿的改名/移动/归档已经算进去了）
#[derive(Debug, Clone)]
pub struct VersionIdentity {
    pub uid: String,
    pub machine_id: String,
    /// 清单里的版本 id（`STANDARD` / `FASTV3.3`）
    pub version_id: String,
    pub name: String,
    pub tag: Option<String>,
    /// 上游给的产物。清单里有、上游还没有对应文件的版本为 `None` = 未生成
    pub mkp_preset: Option<MkpPreset>,
}

/* ---------- 整本 ---------- */

pub struct Book<'a> {
    pub up: &'a Upstream,
    /// 我们自己那份预设数据。**字段定义与三层取值全走它**（b04 Task 9）
    pub presets: &'a Presets,
    draft: &'a Draft,
    /// **清单**：有哪些机型、每台有哪些版本。来源是 `presets/machines/*.toml`
    /// （借自 [`Committed::catalog`]）。
    ///
    /// b04 Task 8 之前这里读的是 `up.catalog` —— 那是上游的构建产物，只读，
    /// 于是「加一个版本」保存之后它在树上看不见。现在清单是我们自己的数据
    pub(super) catalog: &'a [CatalogMachine],
    /// 机型 id → 机型层的那张表（`machineVariants` 的裸键 ⊕ 草稿）
    pub(super) bases: BTreeMap<String, Overrides>,
    /// uid → 版本层的那张表（`machineVariants` 的版本键 ⊕ 草稿）
    pub(super) overs: BTreeMap<String, Overrides>,
    /// 按机型顺序、机型内按清单顺序排好
    versions: Vec<VersionIdentity>,
    built: BTreeMap<String, BuiltRecord>,
    visibility: BTreeMap<String, Visibility>,
    bundles: BTreeMap<String, BundleEdit>,
}

impl<'a> Book<'a> {
    pub fn new(
        up: &'a Upstream,
        presets: &'a Presets,
        committed: &'a Committed,
        draft: &'a Draft,
    ) -> Self {
        // 两层的值都只有一个来源：`machineVariants` 归并出来的那两张表，
        // 草稿里未落盘的改动叠在上面。以前这里读的是我们自造的 json，
        // 于是每层各有"上游给的"与"我们写的"两张表（b04 Task 12 之前）
        let mut bases: BTreeMap<String, Overrides> = BTreeMap::new();
        let mut versions: Vec<VersionIdentity> = Vec::new();
        let mut overs: BTreeMap<String, Overrides> = BTreeMap::new();

        for m in &committed.catalog {
            // 归并是派生的（`domain::variants`），每加载一次算一次，不落盘
            let digested = digest(&presets.registry, &m.id, &m.version_ids);

            let mut base = digested.base.clone();
            overlay(&mut base, draft, Level::Machine, &m.id);
            bases.insert(m.id.clone(), base);

            for vid in &m.version_ids {
                let uid = format!("{}/{}", m.id, vid);
                let declared = committed.versions.get(&uid);
                let mut own = digested.versions.get(vid).cloned().unwrap_or_default();
                overlay(&mut own, draft, Level::Version, &uid);
                overs.insert(uid.clone(), own);

                versions.push(VersionIdentity {
                    uid: uid.clone(),
                    machine_id: m.id.clone(),
                    version_id: vid.clone(),
                    name: declared
                        .map(|x| x.name.clone())
                        .unwrap_or_else(|| vid.clone()),
                    tag: declared.and_then(|v| v.tag.clone()),
                    // 产物还在上游那一层（资源与套餐这一轮没搬，见 presets/mod.rs）。
                    // 清单里有、上游没有的版本 → `None` = 未生成，那是对的
                    mkp_preset: up
                        .catalog
                        .machine(&m.id)
                        .and_then(|x| x.version(vid))
                        .and_then(|x| x.mkp_preset.clone()),
                });
            }
        }

        let mut built = committed.built.clone();
        built.extend(draft.built.clone());
        let mut visibility = committed.visibility.clone();
        visibility.extend(draft.visibility.clone());
        let mut bundles = committed.bundles.clone();
        bundles.extend(draft.bundles.clone());

        Self {
            up,
            presets,
            draft,
            catalog: &committed.catalog,
            bases,
            overs,
            versions,
            built,
            visibility,
            bundles,
        }
    }

    /* ---------- 清单 ---------- */

    /// 清单里的机型，照文件名顺序
    pub fn machines(&self) -> &[CatalogMachine] {
        self.catalog
    }

    pub(super) fn machine(&self, id: &str) -> Option<&'a CatalogMachine> {
        self.catalog.iter().find(|m| m.id == id)
    }

    /* ---------- 取层 ---------- */

    /// 机型基底那一列的三层视图（版本层为空）
    pub fn machine_layers(&self, machine_id: &str) -> Option<Layers<'_>> {
        let base = self.bases.get(machine_id)?;
        let id = self.machine(machine_id)?.id.as_str();
        Some(Layers::new(&self.presets.registry, id, base, no_overrides()))
    }

    /// 一个版本的三层视图。
    ///
    /// 机型层取它那台机型的裸键，版本层只取 `A1: FAST` 这一条 ——
    /// 别的版本的键是别的版本的，不参与这一列的取值
    pub fn version_layers(&self, uid: &str) -> Option<Layers<'_>> {
        let v = self.version(uid)?;
        let base = self.bases.get(&v.machine_id)?;
        let over = self.overs.get(uid)?;
        let id = self.machine(&v.machine_id)?.id.as_str();
        Some(Layers::new(&self.presets.registry, id, base, over))
    }

    pub fn version(&self, uid: &str) -> Option<&VersionIdentity> {
        self.versions.iter().find(|v| v.uid == uid)
    }

    pub fn versions(&self) -> &[VersionIdentity] {
        &self.versions
    }

    /// 某台机型下的版本。以前这里还要滤掉归档的，**归档这个概念本身没有了**
    /// （REPORT §7.2：源数据里没这个字段，它是上一稿发明的 SOP）
    pub fn live_versions(&self, machine_id: &str) -> Vec<&VersionIdentity> {
        self.versions
            .iter()
            .filter(|v| v.machine_id == machine_id)
            .collect()
    }

    /* ---------- BBS ---------- */

    /// 一台机型的默认 BBS：`defaultBundle` 指的那个套餐里的曲线
    pub fn machine_default_bbs(&self, machine_id: &str) -> Vec<String> {
        let Some(m) = self.machine(machine_id) else {
            return Vec::new();
        };
        let Some(bid) = &m.default_bundle else {
            return Vec::new();
        };
        self.bundle_bbs(bid)
    }

    /// 套餐里的 BBS。我们的改动优先于上游那一份
    pub fn bundle_bbs(&self, bundle_id: &str) -> Vec<String> {
        if let Some(edit) = self.bundles.get(bundle_id) {
            return edit.bbs.clone();
        }
        self.up
            .manifest
            .bundle(bundle_id)
            .map(|b| {
                b.asset_refs
                    .iter()
                    .filter(|id| {
                        self.up
                            .manifest
                            .asset(id)
                            .is_some_and(|a| a.resource_type == ResourceType::BbsProfile)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 一个版本最终交付哪些 BBS = 它那台机型的默认那一份。
    ///
    /// 「这一版自己挑一串曲线」这条路径删掉了（REPORT §7.3）：版本已经有
    /// `recommendedBundle`，在版本上再存一份 asset id 就是两处真相。
    /// 要换就换它指向哪个套餐 —— 那是「机型与资源」页的事（Task 21）
    pub fn effective_bbs(&self, uid: &str) -> Vec<String> {
        match self.version(uid) {
            Some(v) => self.machine_default_bbs(&v.machine_id),
            None => Vec::new(),
        }
    }

    /// **现在只有一种来源。** 参数照收是为了不惊动调用方，但它已经不参与判断了 ——
    /// 等「机型与资源」页把套餐换成那个入口（Task 21），这里才会重新有第二种答案
    pub fn bbs_source(&self, _uid: &str) -> BbsSource {
        BbsSource::InheritedFromMachine
    }

    /* ---------- 生成状态 ---------- */

    /// 四档之一。判据是**指纹比对，不看文件时间**
    pub fn build_state(&self, uid: &str) -> BuildState {
        let Some(v) = self.version(uid) else {
            return BuildState::NeverBuilt;
        };
        let has_product = v.mkp_preset.is_some();
        let has_bbs = !self.effective_bbs(uid).is_empty();

        if !has_product && !has_bbs && !self.has_any_recipe(uid) {
            // 三个条件缺一不可，见模块文档那张表
            return BuildState::NoResources;
        }
        let Some(rec) = self.built.get(uid) else {
            return BuildState::NeverBuilt;
        };
        match self.version_layers(uid) {
            Some(l) if l.fingerprint() == rec.fingerprint => BuildState::Built,
            Some(_) => BuildState::Stale,
            // 层都取不出来（机型被上游删了）—— 说"待生成"而不是"已生成"：
            // 说错成"已生成"会让人以为客户端拿到的是对的
            None => BuildState::Stale,
        }
    }

    /// 这一版有没有「配方」= 机型层或版本层在这一台/这一版上钉过值。
    ///
    /// 全是出厂默认时为 false —— 那种情况下烤出来也只是一份全默认的 TOML。
    ///
    /// 以前这里要**分别**数"我们写的"与"上游给的"两张表，因为它们是两个文件；
    /// 现在两张表合成了同一条 `machineVariants`，一个空就够了
    pub fn has_any_recipe(&self, uid: &str) -> bool {
        let Some(v) = self.version(uid) else {
            return false;
        };
        let base = self.bases.get(&v.machine_id).is_some_and(|b| !b.is_empty());
        let over = self.overs.get(uid).is_some_and(|o| !o.is_empty());
        base || over
    }

    /// 有效配方为空 = 「未配置」。**不占状态档**，单独一个布尔位（tasks 7.3）
    pub fn recipe_empty(&self, uid: &str) -> bool {
        // 取不出层也算空。`map_or(true, ..)` 而不是 `is_none_or`：
        // 本仓 MSRV 是 1.77，后者要 1.82
        #[allow(clippy::unnecessary_map_or)]
        self.version_layers(uid)
            .map_or(true, |l| l.effective_recipe().is_empty())
    }

    pub fn last_build(&self, uid: &str) -> Option<&str> {
        self.built.get(uid).map(|r| r.stamp.as_str())
    }

    /* ---------- 视图 ---------- */

    pub fn book_view(&self) -> BookView {
        let mut machines = Vec::new();
        let mut base_items = 0usize;
        let mut over_items = 0usize;

        for m in self.catalog {
            let ml = self.machine_layers(&m.id);
            // 每层只有一张表，所以"这一层有几项"与"其中自己钉了几项"是同一个数。
            // 以前每层有两半（见 `domain::layer` 的模块文档），才需要两个数
            let base_count = ml.as_ref().map(|l| l.own_count(Level::Machine)).unwrap_or(0);
            base_items += base_count;

            let mut nodes = Vec::new();
            for v in self.versions.iter().filter(|v| v.machine_id == m.id) {
                let l = self.version_layers(&v.uid);
                let over_count = l.as_ref().map(|x| x.own_count(Level::Version)).unwrap_or(0);
                over_items += over_count;
                nodes.push(VersionNode {
                    uid: v.uid.clone(),
                    version_id: v.version_id.clone(),
                    name: v.name.clone(),
                    tag: v.tag.clone(),
                    items: over_count,
                    build: self.build_state(&v.uid),
                    bbs_source: self.bbs_source(&v.uid),
                    bbs_count: self.effective_bbs(&v.uid).len(),
                    recipe_empty: self.recipe_empty(&v.uid),
                    last_build: self.last_build(&v.uid).map(str::to_owned),
                    orphan_keys: l
                        .as_ref()
                        .map(|x| x.orphan_keys().into_iter().map(str::to_owned).collect())
                        .unwrap_or_default(),
                });
            }

            let live: Vec<BuildState> = self
                .live_versions(&m.id)
                .iter()
                .map(|v| self.build_state(&v.uid))
                .collect();

            machines.push(MachineNode {
                id: m.id.clone(),
                display: m.display.clone(),
                icon: m.icon.clone(),
                items: base_count,
                // 机型级：所有版本都「暂无资源」才算这台机型暂无资源
                build: roll_up(live.into_iter()),
                dimensions_missing: !m.has_dimensions,
                versions: nodes,
            });
        }

        let states: Vec<BuildState> = self
            .versions
            .iter()
            .map(|v| self.build_state(&v.uid))
            .collect();
        let artifact = artifact_state(states.into_iter());

        BookView {
            machines,
            badges: Badges {
                machines: self.catalog.len(),
                versions: self.versions.len(),
                base_items,
                override_items: over_items,
            },
            dirty_count: self.draft.dirty_count(),
            save: if self.draft.is_clean() {
                SaveState::Saved
            } else {
                SaveState::Dirty
            },
            artifact,
            last_build: self.built.values().map(|r| r.stamp.clone()).max(),
            build_rows: self.build_rows(),
            notices: Vec::new(),
            // 这一层看不到文件，所以快照状态由 `app` 覆盖。默认说「跟上了」
            snapshot: SnapshotState::Current,
        }
    }

    /// 生成视角那张表
    pub fn build_rows(&self) -> Vec<BuildRow> {
        self.versions
            .iter()
            .map(|v| {
                let state = self.build_state(&v.uid);
                BuildRow {
                    uid: v.uid.clone(),
                    machine_id: v.machine_id.clone(),
                    machine: self
                        .up
                        .catalog
                        .machine(&v.machine_id)
                        .map(|m| m.display.clone())
                        .unwrap_or_else(|| v.machine_id.clone()),
                    name: v.name.clone(),
                    state,
                    reason: state.explain().to_owned(),
                    buildable: state.buildable(),
                    disabled_reason: if state.buildable() {
                        None
                    } else {
                        Some(
                            match state {
                                BuildState::NoResources => w::disabled::BUILD_NO_RESOURCES,
                                _ => w::disabled::BUILD_NOTHING_TO_DO,
                            }
                            .to_owned(),
                        )
                    },
                    mkp_file: v.mkp_preset.as_ref().map(|p| p.file_name.clone()),
                    bbs_count: self.effective_bbs(&v.uid).len(),
                    last_build: self.last_build(&v.uid).map(str::to_owned),
                }
            })
            .collect()
    }

    /// 仓库盘点那张表：**只列交付物**（18 条），界面素材不算。
    ///
    /// 每条都有真 sha256 与真 size（来自 `manifest.assets`）——
    /// 所以这一页不该出现一个「未知」
    pub fn stock_rows(&self) -> Vec<StockRow> {
        let assigned: BTreeSet<String> = self
            .versions
            .iter()
            .flat_map(|v| self.effective_bbs(&v.uid))
            .collect();
        let in_bundle: BTreeSet<&str> = self
            .up
            .manifest
            .bundles()
            .iter()
            .flat_map(|b| {
                self.bundles
                    .get(&b.id)
                    .map(|e| {
                        e.presets
                            .iter()
                            .chain(e.bbs.iter())
                            .map(String::as_str)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| b.asset_refs.iter().map(String::as_str).collect())
            })
            .collect();

        let mut out: Vec<StockRow> = self
            .up
            .manifest
            .deliverables()
            .into_iter()
            .map(|a| StockRow {
                assign: self.assign_of(a, &assigned),
                in_any_bundle: in_bundle.contains(a.id.as_str()),
                visibility: self
                    .visibility
                    .get(&a.id)
                    .copied()
                    .unwrap_or(Visibility::Menu),
                id: a.id.clone(),
                resource_type: a.resource_type,
                machine_id: a.machine_id.clone(),
                file_name: a.file_name.clone(),
                relative_path: a.relative_path.clone(),
                sha256: a.sha256.clone(),
                size: a.size,
                updated_at: a.updated_at.clone(),
                nozzle: a.nozzle.clone(),
                layer_height: a.layer_height.clone(),
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    /// BBS 三态。**与「有没有进套餐」正交**（tasks 7.6）：
    /// 一条曲线可以已分配给某个版本、同时不属于任何套餐
    fn assign_of(&self, asset: &Asset, assigned: &BTreeSet<String>) -> BbsAssign {
        if self.visibility.get(&asset.id) == Some(&Visibility::ArchiveOnly) {
            return BbsAssign::ArchiveOnly;
        }
        if assigned.contains(&asset.id) {
            BbsAssign::Assigned
        } else {
            BbsAssign::Optional
        }
    }

    /* ---------- 矩阵 ---------- */

    /// 一屏矩阵。列由前端勾选给出，**但顺序由这里按配方本重排**
    pub fn matrix(&self, cols: &[ColRef], tab: Option<&str>, query: &str) -> Matrix {
        let cols = self.order_cols(cols);
        let q = query.trim().to_lowercase();
        let searching = !q.is_empty();

        // 行取并集：任意一列的机型有这个字段，这一行就在
        let mut keys: Vec<&str> = Vec::new();
        for c in &cols {
            for k in self.presets.registry.visible_keys(&c.machine_id) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        // 行序：**分类 → 组 → 父子 → 组内序**。见 `row_sort_key`
        keys.sort_by(|a, b| self.row_sort_key(a).cmp(&self.row_sort_key(b)));
        let total_rows = keys.len();

        let gates: Vec<Option<Gate<'_>>> = cols
            .iter()
            .map(|c| c.layers.as_ref().map(|l| Gate::new(&self.presets.registry, l)))
            .collect();

        let rows: Vec<Row> = keys
            .into_iter()
            .filter_map(|key| {
                let p = self.presets.registry.param(key)?;
                // **搜索一开，分类过滤让开**（doc §8.1）：
                // 否则用户搜一个词、没命中当前分类，会以为这个字段不存在
                if searching {
                    let hay = [
                        p.label.as_str(),
                        p.key.as_str(),
                        p.toml_key.as_str(),
                        p.layout.section_id.as_str(),
                        p.desc.as_str(),
                    ];
                    if !hay.iter().any(|h| h.to_lowercase().contains(&q)) {
                        return None;
                    }
                } else if let Some(t) = tab {
                    if !self.tab_of(&p.layout.section_id).is_some_and(|x| x == t) {
                        return None;
                    }
                }

                let cells = cols
                    .iter()
                    .zip(&gates)
                    .map(|(c, g)| self.cell(c, g.as_ref(), key))
                    .collect();
                let parent = p
                    .parent_key
                    .as_deref()
                    .and_then(|k| self.presets.registry.param(k));
                Some(Row {
                    key: key.to_owned(),
                    label: p.label.clone(),
                    desc: p.desc.clone(),
                    unit: p.unit.clone(),
                    section_id: p.layout.section_id.clone(),
                    section_label: self.section_label(&p.layout.section_id),
                    tab_id: self.tab_of(&p.layout.section_id).map(str::to_owned),
                    // 只有两级：上游 74 条里 7 条有 parentKey，没有一条的父自己还有父
                    depth: u8::from(p.parent_key.is_some()),
                    parent_key: p.parent_key.clone(),
                    parent_label: parent.map(|x| x.label.clone()),
                    parent_note: parent.map(|x| w::relate::belongs_to(&x.label)),
                    control_note: self.control_note(p),
                    gcode: w::is_gcode(p),
                    deprecated: p.deprecated,
                    cells,
                })
            })
            .collect();

        let note = if searching {
            Some(w::MATRIX_SEARCH_SPANS_ALL_TABS.to_owned())
        } else {
            None
        };
        let empty_reason = if cols.is_empty() {
            Some(w::MATRIX_NO_COLS.to_owned())
        } else if rows.is_empty() {
            Some(w::MATRIX_NO_MATCH.to_owned())
        } else {
            None
        };

        Matrix {
            cols: cols.into_iter().map(|c| c.head).collect(),
            rows,
            total_rows,
            note,
            empty_reason,
        }
    }

    /// 列序照配方本：机型按 catalog 顺序，机型内先基底列再版本列。
    /// **勾选顺序一律丢掉** —— 按点击顺序排会让同一份数据每次长得不一样
    fn order_cols(&self, want: &[ColRef]) -> Vec<ResolvedCol<'_>> {
        let wanted: BTreeSet<(&str, Option<&str>)> = want
            .iter()
            .map(|c| (c.machine_id.as_str(), c.version_uid.as_deref()))
            .collect();
        let mut out = Vec::new();
        for m in self.catalog {
            if wanted.contains(&(m.id.as_str(), None)) {
                let layers = self.machine_layers(&m.id);
                out.push(ResolvedCol {
                    head: Col {
                        key: m.id.clone(),
                        machine_id: m.id.clone(),
                        version_uid: None,
                        level: Level::Machine,
                        machine: m.display.clone(),
                        label: w::level_label(Level::Machine).to_owned(),
                        items: layers.as_ref().map(|l| l.own_count(Level::Machine)).unwrap_or(0),
                    },
                    machine_id: m.id.clone(),
                    level: Level::Machine,
                    layers,
                });
            }
            for v in self.versions.iter().filter(|v| v.machine_id == m.id) {
                if !wanted.contains(&(m.id.as_str(), Some(v.uid.as_str()))) {
                    continue;
                }
                let layers = self.version_layers(&v.uid);
                out.push(ResolvedCol {
                    head: Col {
                        key: v.uid.clone(),
                        machine_id: m.id.clone(),
                        version_uid: Some(v.uid.clone()),
                        level: Level::Version,
                        machine: m.display.clone(),
                        label: v.name.clone(),
                        items: layers.as_ref().map(|l| l.own_count(Level::Version)).unwrap_or(0),
                    },
                    machine_id: m.id.clone(),
                    level: Level::Version,
                    layers,
                });
            }
        }
        out
    }

    fn cell(&self, col: &ResolvedCol<'_>, gate: Option<&Gate<'_>>, key: &str) -> Cell {
        let Some(layers) = &col.layers else {
            return Cell::not_applicable();
        };
        let Some(p) = self.presets.registry.param(key) else {
            return Cell::not_applicable();
        };
        let Some(hit) = layers.effective(key) else {
            // 这台机型没有这一项。**与"被关着"分开说**：
            // 前者根本没有这一项，后者有值、会进产物，只是现在不该改
            return Cell::not_applicable();
        };
        let blocked = gate.map(|g| g.blocked(key)).unwrap_or_default();
        let own = layers.has_own(col.level, key);

        // 「谁把我关了」的整句 + 跳转目标。`blocked` 是**根在前**排的，
        // 所以取第一条就是"最上游那个要先动的东西"，中间环节不必全列出来
        let (blocked_note, jump_to) = match blocked.first() {
            None => (None, None),
            Some(b) => {
                let here = self
                    .presets
                    .registry
                    .param(&b.key)
                    .is_some_and(|c| c.applies_to(&col.machine_id));
                if here {
                    (Some(w::relate::blocked_note(&b.label, &b.need)), Some(b.key.clone()))
                } else {
                    // 控制它的那一项这台机型上根本没有 —— **不给跳转**。
                    // 这是上游数据问题，骗用户去点一个不存在的格子更糟
                    (Some(w::relate::controller_not_here(&b.label)), None)
                }
            }
        };

        let (kind, text, lines) = if w::is_gcode(p) {
            let (n, t) = w::gcode_text(hit.value);
            (CellKind::Gcode, t, Some(n))
        } else {
            (CellKind::Value, w::value_text(p, hit.value), None)
        };

        Cell {
            kind,
            text,
            lines,
            origin: Some(hit.origin),
            origin_label: Some(w::origin_label(hit.origin).to_owned()),
            origin_explain: Some(w::origin_explain(hit.origin).to_owned()),
            own,
            // 改动还在草稿里 —— 界面用蓝色描边标它，不换底色（doc §7 视觉规范）
            dirty: self.draft.pending(col.level, &col.key_of_level(), key).is_some(),
            editable: blocked.is_empty(),
            reason: if blocked.is_empty() {
                None
            } else {
                Some(w::disabled::BLOCKED_BY_CONDITION.to_owned())
            },
            blocked,
            blocked_note,
            jump_to,
            raw: hit.value.clone(),
        }
    }

    /// 行头常驻的那一句：这一项归谁管。
    ///
    /// **不看当前值** —— 所以它在格子还没变灰之前就已经在那儿了。
    /// 「不知道是哪个选项导致它灰色」的正解是这一句常驻，而不是等灰了再去悬停
    fn control_note(&self, p: &ParamDef) -> Option<String> {
        if let Some(sw) = &p.show_when {
            return Some(w::relate::controlled_by(self.label_of(&sw.key)));
        }
        // section 级条件：要用户做的事一样，但说法不同（「整组关着」vs「上一项没开」）
        let sw = self.presets.registry.section_show_when(&p.layout.section_id)?;
        Some(w::relate::group_controlled_by(self.label_of(&sw.key)))
    }

    /// 字段的中文名。查不到就退回 key —— 空字符串会让那句话变成「受「」控制」
    fn label_of<'s>(&'s self, key: &'s str) -> &'s str {
        self.presets
            .registry
            .param(key)
            .map_or(key, |p| p.label.as_str())
    }

    /// 行序的排序键：**五段**。
    ///
    /// 只按 `layout.order` 排是错的 —— 那是 section **内部**的序号（实测各组都从 1 开始，
    /// 还有 `0.5` 这种插队值）。把它当全局键，等于把 16 个分组洗牌，
    /// 出来就是「既不按逻辑、也不按字母」的那种乱序。
    ///
    /// 第三段是**父项的**序号，不是 `depth`：这样子项紧跟在自己的父项后面。
    /// 若改成 `depth` 单独占一段，一个有子项的父行会被自己的子项挤到组尾。
    ///
    /// 最后一段是 key，保证**同序时也稳定** —— 否则同一份数据两次渲染的顺序可能不同。
    fn row_sort_key<'k>(&self, key: &'k str) -> (i64, i64, i64, i64, &'k str) {
        let Some(p) = self.presets.registry.param(key) else {
            return (i64::MAX, i64::MAX, i64::MAX, i64::MAX, key);
        };
        let section = p.layout.section_id.as_str();
        let tab = self
            .tab_of(section)
            .map_or(f64::MAX, |t| self.presets.registry.tab_order(t));
        let group = self
            .presets
            .registry
            .section_meta(section)
            .map_or(f64::MAX, |s| s.order);
        let family = p
            .parent_key
            .as_deref()
            .and_then(|k| self.presets.registry.param(k))
            .map_or(p.layout.order, |parent| parent.layout.order);
        (
            milli(tab),
            milli(group),
            milli(family),
            milli(p.layout.order),
            key,
        )
    }

    /// 组的中文名。查不到就退化成 section id ——
    /// 空字符串会让分组行变成一条没有标题的空白，那比露出一个英文 id 更难查
    fn section_label(&self, section_id: &str) -> String {
        self.presets
            .registry
            .section_meta(section_id)
            .map_or_else(|| section_id.to_owned(), |s| s.label.clone())
    }

    /* ---------- 配方台（默认视角） ---------- */

    /**
    一个版本的分组列表。**建在 `matrix()` 上，不另起一套判定。**

    矩阵那一套已经把「行序五段键 / 搜索与分类过滤 / 单元格四分支 / 谁把我关了」都算好了。
    这里做的只有三件事：

    1. 按 `section_id` 切成组（行序已经保证同组连续，所以切一刀就够）
    2. 把子项挂到父项下面
    3. 父项把子项整组关掉时给一句话，让界面能把它们收起来

    左栏那份导航**不跟着搜索变**：它从字段定义直接数，
    否则搜一个词整棵导航树就塌了，而那正是用来换分组看的东西。
    */
    pub fn desk(&self, machine_id: &str, uid: Option<&str>, tab: Option<&str>, query: &str) -> Desk {
        let col = ColRef {
            machine_id: machine_id.to_owned(),
            version_uid: uid.map(str::to_owned),
        };
        let m = self.matrix(&[col], tab, query);
        let searching = !query.trim().is_empty();

        // 先按组切；同组连续是行序的保证，这里不再自己聚合
        let mut groups: Vec<DeskGroup> = Vec::new();
        for row in m.rows {
            let key = row.section_id.clone();
            if groups.last().map(|g| g.section_id.as_str()) != Some(key.as_str()) {
                groups.push(DeskGroup {
                    section_id: key,
                    label: row.section_label.clone(),
                    count: 0,
                    off_note: None,
                    items: Vec::new(),
                });
            }
            let g = groups.last_mut().expect("刚刚放进去的");
            g.count += 1;
            // 子项挂到父项下面。**搜索时不挂** —— 命中的可能只有子项，
            // 把它塞进一个没命中的父项里会让人以为父项也命中了
            let parent_here = !searching
                && row.parent_key.as_deref().is_some_and(|p| {
                    g.items.last().map(|i| i.row.key.as_str()) == Some(p)
                });
            if parent_here {
                g.items.last_mut().expect("上面刚判过").children.push(row);
            } else {
                g.items.push(DeskItem {
                    row,
                    children: Vec::new(),
                    off_note: None,
                });
            }
        }

        // 整组 / 整族被关掉的那句话
        for g in &mut groups {
            for it in &mut g.items {
                it.off_note = family_off_note(&it.row, &it.children);
            }
            g.off_note = group_off_note(g);
        }

        Desk {
            nav: self.desk_nav(machine_id),
            groups,
            total: m.total_rows,
            note: m.note,
            empty_reason: m.empty_reason,
        }
    }

    /// 左栏导航：tab → section 两级 + 计数。**直接从字段定义数**，不受搜索与分类影响
    fn desk_nav(&self, machine_id: &str) -> Vec<DeskNavTab> {
        let mut per_section: BTreeMap<&str, usize> = BTreeMap::new();
        for key in self.presets.registry.visible_keys(machine_id) {
            if let Some(p) = self.presets.registry.param(key) {
                *per_section.entry(p.layout.section_id.as_str()).or_default() += 1;
            }
        }
        self.presets
            .registry
            .param_tabs()
            .into_iter()
            .filter_map(|t| {
                let sections: Vec<DeskNavSection> = t
                    .sections
                    .iter()
                    .filter_map(|s| {
                        let count = per_section.get(s.id.as_str()).copied().unwrap_or(0);
                        // 这台机型一项都没有的组不列出来（A2L 那种机型会遇到）
                        (count > 0).then(|| DeskNavSection {
                            id: s.id.clone(),
                            label: s.label.clone(),
                            count,
                        })
                    })
                    .collect();
                let count = sections.iter().map(|s| s.count).sum();
                (count > 0).then_some(DeskNavTab {
                    id: t.id,
                    label: t.label,
                    count,
                    sections,
                })
            })
            .collect()
    }

    fn tab_of(&self, section_id: &str) -> Option<&str> {
        self.presets.registry.tab_of_section(section_id)
    }
}

/// 父项把下面整族关掉了吗。**判据是子项全部改不动，而且是同一个父项关的**
fn family_off_note(parent: &Row, children: &[Row]) -> Option<String> {
    if children.is_empty() {
        return None;
    }
    let all_off = children.iter().all(|c| {
        c.cells
            .first()
            .is_some_and(|cell| cell.blocked.iter().any(|b| b.key == parent.key))
    });
    if !all_off {
        return None;
    }
    let value = parent.cells.first().map_or("", |c| c.text.as_str());
    Some(w::relate::family_off(&parent.label, value, children.len()))
}

/// 整组被 section 级条件关掉了吗
fn group_off_note(g: &DeskGroup) -> Option<String> {
    let mut who: Option<(&str, &str)> = None;
    for it in &g.items {
        let cell = it.row.cells.first()?;
        let hit = cell
            .blocked
            .iter()
            .find(|b| b.scope == BlockScope::Section)?;
        match who {
            None => who = Some((&hit.key, &hit.label)),
            // 同一组里两个不同的 section 条件 —— 上游没有这种数据，有的话不合成一句
            Some((k, _)) if k != hit.key => return None,
            Some(_) => {}
        }
    }
    let (_, label) = who?;
    Some(w::relate::group_off(label, "", g.count))
}

/// `order` 压成能进元组比较的整数。上游最细到 0.1（实测有 `0.5` / `1.1`），乘 1000 留余量。
///
/// `f64::MAX` 这种「查不到」的哨兵**饱和**到 `i64::MAX`，
/// 不许让它回绕成负数排到最前面 —— 那会让一个上游未声明的分组顶到表头
fn milli(v: f64) -> i64 {
    const LIMIT: f64 = (i64::MAX / 1000) as f64;
    if v >= LIMIT {
        i64::MAX
    } else if v <= -LIMIT {
        i64::MIN
    } else {
        (v * 1000.0).round() as i64
    }
}

/// 把草稿里属于这一层的值叠进来。
///
/// **值为 `None` 的那条要真的删键** —— 当成"没有这一条"处理的话，
/// 「挂回继承」在保存之前看不出效果，用户会以为按钮坏了
fn overlay(own: &mut Overrides, draft: &Draft, level: Level, owner: &str) {
    for key in draft
        .values
        .keys()
        .filter_map(|k| super::patch::split_value_key(k, level, owner))
        .collect::<Vec<_>>()
    {
        match draft.pending(level, owner, &key) {
            Some(Some(v)) => {
                own.insert(key, v.clone());
            }
            Some(None) => {
                own.remove(&key);
            }
            None => {}
        }
    }
}

/// 机型级状态：**全部「暂无资源」才算这台机型暂无资源**。
/// 有一个版本能生成，这台机型就不是"什么都没有"
fn roll_up(states: impl Iterator<Item = BuildState>) -> BuildState {
    let all: Vec<BuildState> = states.collect();
    if all.is_empty() {
        return BuildState::NoResources;
    }
    if all.iter().all(|s| *s == BuildState::NoResources) {
        return BuildState::NoResources;
    }
    if all.contains(&BuildState::Stale) {
        return BuildState::Stale;
    }
    if all.contains(&BuildState::NeverBuilt) {
        return BuildState::NeverBuilt;
    }
    BuildState::Built
}

/// 整本产物状态。**「暂无资源」的版本不参与** —— 它们本来就不该有产物，
/// 把它们算进去会让整本永远是「未生成」
fn artifact_state(states: impl Iterator<Item = BuildState>) -> ArtifactState {
    let all: Vec<BuildState> = states
        .filter(|s| *s != BuildState::NoResources)
        .collect();
    if all.is_empty() || all.iter().all(|s| *s == BuildState::NeverBuilt) {
        return ArtifactState::Missing;
    }
    if all.iter().any(|s| s.buildable()) {
        return ArtifactState::Stale;
    }
    ArtifactState::Fresh
}

/* ---------- 返回给前端的形状 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BookView {
    pub machines: Vec<MachineNode>,
    pub badges: Badges,
    pub dirty_count: usize,
    pub save: SaveState,
    pub artifact: ArtifactState,
    pub last_build: Option<String>,
    pub build_rows: Vec<BuildRow>,
    /// 加载与草稿清理时发现的问题。**由 `app` 层填** ——
    /// 这一层看不到文件，而"仓库里有一份上游已经不存在的配方"只有读文件时才发现。
    /// 不交出来的话，那一份在树上会整个消失，用户只看到"我的改动去哪了"
    pub notices: Vec<String>,
    /// 崩溃快照跟上了没有。**与 `save` / `dirty_count` 是两件事** ——
    /// 「未保存」说仓库文件，「待落盘」说快照。由 `app` 层填
    pub snapshot: SnapshotState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Badges {
    pub machines: usize,
    pub versions: usize,
    /// 机型层一共有几项值
    pub base_items: usize,
    /// 版本层加起来一共有几项值
    pub override_items: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineNode {
    pub id: String,
    pub display: String,
    pub icon: Option<String>,
    /// 机型层钉着几项值
    pub items: usize,
    pub build: BuildState,
    /// 上游没登记这台机型的床身尺寸（A2L）。**不是 0，是没登记**
    pub dimensions_missing: bool,
    pub versions: Vec<VersionNode>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionNode {
    pub uid: String,
    pub version_id: String,
    pub name: String,
    pub tag: Option<String>,
    /// 版本层钉着几项值
    pub items: usize,
    pub build: BuildState,
    pub bbs_source: BbsSource,
    pub bbs_count: usize,
    /// 有效配方为空 = 「未配置」。**不占状态档**
    pub recipe_empty: bool,
    pub last_build: Option<String>,
    /// 写着但再也进不了产物的键
    pub orphan_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRow {
    pub uid: String,
    pub machine_id: String,
    pub machine: String,
    pub name: String,
    pub state: BuildState,
    /// 「现在该做什么」，不是「这个状态怎么算出来的」
    pub reason: String,
    pub buildable: bool,
    /// 不能生成时的那一句。**禁用必须有理由**
    pub disabled_reason: Option<String>,
    pub mkp_file: Option<String>,
    pub bbs_count: usize,
    pub last_build: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StockRow {
    pub id: String,
    pub resource_type: ResourceType,
    pub machine_id: Option<String>,
    pub file_name: String,
    pub relative_path: String,
    /// 真哈希。这一页不该出现「未知」
    pub sha256: String,
    pub size: u64,
    pub updated_at: String,
    pub nozzle: Option<String>,
    pub layer_height: Option<String>,
    pub assign: BbsAssign,
    /// **与 `assign` 正交**：可以已分配、同时不属于任何套餐
    pub in_any_bundle: bool,
    pub visibility: Visibility,
}

/* ---------- 矩阵 ---------- */

/// 配方台一屏：左栏导航 + 分组列表
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desk {
    /// 左栏。**不随搜索变** —— 它是换分组看的工具
    pub nav: Vec<DeskNavTab>,
    pub groups: Vec<DeskGroup>,
    /// 过滤前一共几项
    pub total: usize,
    pub note: Option<String>,
    pub empty_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskNavTab {
    pub id: String,
    pub label: String,
    pub count: usize,
    pub sections: Vec<DeskNavSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskNavSection {
    pub id: String,
    pub label: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskGroup {
    pub section_id: String,
    pub label: String,
    pub count: usize,
    /// 整组被 section 级条件关掉时的那一句。界面据此把整组收起来
    pub off_note: Option<String>,
    pub items: Vec<DeskItem>,
}

/// 一项，外加挂在它下面的子项。**只有两级**
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskItem {
    pub row: Row,
    pub children: Vec<Row>,
    /// 这一项把自己下面那几个关掉了时的那一句
    pub off_note: Option<String>,
}

/// 前端勾了什么。**顺序无所谓**，后端会按配方本重排
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColRef {
    pub machine_id: String,
    /// `None` = 机型基底那一列
    pub version_uid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Col {
    pub key: String,
    pub machine_id: String,
    pub version_uid: Option<String>,
    /// 写进哪一层。**不许弄反** —— 弄反了就是静默打破继承
    pub level: Level,
    pub machine: String,
    pub label: String,
    /// 这一列那一层钉着几项值
    pub items: usize,
}

struct ResolvedCol<'a> {
    head: Col,
    machine_id: String,
    level: Level,
    layers: Option<Layers<'a>>,
}

impl ResolvedCol<'_> {
    /// 草稿键里的 owner：机型列用机型 id，版本列用 uid
    fn key_of_level(&self) -> String {
        match self.level {
            Level::Machine => self.machine_id.clone(),
            Level::Version => self.head.version_uid.clone().unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Matrix {
    pub cols: Vec<Col>,
    pub rows: Vec<Row>,
    /// 过滤前一共有几行。说明条上写「显示 N / 共 M」
    pub total_rows: usize,
    pub note: Option<String>,
    /// 空的时候**写出为什么空**，不留白
    pub empty_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub key: String,
    pub label: String,
    pub desc: String,
    pub unit: Option<String>,
    pub section_id: String,
    /// 组的中文名。前端在它与上一行不同时插一条分组表头 ——
    /// **分组是后端排出来的**，前端只是照着变化分段，不自己聚合
    pub section_label: String,
    pub tab_id: Option<String>,
    /// 0 = 顶层，1 = 某个父字段的子项。**只有两级**，上游没有更深的
    pub depth: u8,
    pub parent_key: Option<String>,
    /// 父字段的中文名。父自己不可见（被 `machineFilter` 排掉 / 已废弃）时为 `None`，
    /// 但 `depth` 仍是 1 —— 它在数据上确实是子项
    pub parent_label: Option<String>,
    /// 行头那句「属于：X」。整句在后端拼好，**前端不做格式化**
    pub parent_note: Option<String>,
    /// 行头那句「受「X」控制」/「整组由「X」控制」。**与当前值无关**，
    /// 所以它在格子变灰之前就已经在那儿了
    pub control_note: Option<String>,
    /// G-code 行**拒绝批量**（doc §8.4）
    pub gcode: bool,
    pub deprecated: bool,
    pub cells: Vec<Cell>,
}

/// 三种 kind。**doc §8.3 的第四条分支（选中了升级成真控件）不在这里** ——
/// 后端不知道现在选中的是哪一格，那是前端的状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CellKind {
    NotApplicable,
    Gcode,
    Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub kind: CellKind,
    /// 已经格式化好的显示文本。**前端不做格式化**
    pub text: String,
    /// G-code 的行数
    pub lines: Option<usize>,
    pub origin: Option<Origin>,
    pub origin_label: Option<String>,
    pub origin_explain: Option<String>,
    /// 这一层我们自己写过 = 「挂回继承」可点
    pub own: bool,
    /// 改动还在草稿里。界面用蓝色描边，**不换底色** —— 这样"来源"和"改没改"两条信息都在
    pub dirty: bool,
    pub editable: bool,
    pub reason: Option<String>,
    /// 根在前
    pub blocked: Vec<BlockedBy>,
    /// 点开灰格子时显示的整句。两种：能跳的说「由「X」控制，需 Y」，
    /// 不能跳的说「控制它的「X」在这台机型上没有这一项」
    pub blocked_note: Option<String>,
    /// 「去改那一项」跳到哪个字段。`None` = **不给跳转按钮**
    pub jump_to: Option<String>,
    /// 原始值。受控控件要用它，不能拿格式化过的文本回填
    pub raw: Value,
}

impl Cell {
    fn not_applicable() -> Self {
        Self {
            kind: CellKind::NotApplicable,
            text: w::NOT_APPLICABLE.to_owned(),
            lines: None,
            origin: None,
            origin_label: None,
            origin_explain: None,
            own: false,
            dirty: false,
            editable: false,
            reason: Some(w::disabled::NOT_APPLICABLE.to_owned()),
            blocked: Vec::new(),
            blocked_note: None,
            jump_to: None,
            raw: Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, CommittedVersion, Patch};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture, FIXTURE_MACHINES};

    /* ---------- 这一轮删掉的那几条，以及它们为什么不复存在 ----------
     *
     * 1. `draft_structure_changes_show_in_the_tree`（草稿里的改名 / 移动 / 新建 / 归档
     *    要反映到树上）：`Patch` 现在只剩四种，**全都是值改动**。结构性操作写的是
     *    `presets/machines/{机型}.toml`，由「机型与版本」那一页即时落盘 —— 草稿里再也没有
     *    结构手势了（见 `patch.rs` 模块文档那张表）。
     * 2. `an_upstream_version_can_only_be_archived_not_purged`：**归档这个概念整个删掉了**。
     *    源数据里没有这个字段，它是上一稿自己发明的 SOP（REPORT §7.2）。
     * 3. `a_version_can_own_its_bbs_list`：「这一版自己挑一串曲线」删掉了（REPORT §7.3）——
     *    版本已经有 `recommendedBundle`，在版本上再存一份 asset id 是两处真相。
     * 4. 「五档查找 / 每层两半」那几条（徽章分开数「一共几项」与「我们自己几项」）：
     *    每层现在只有**一张**表（`Layers::new` 收四个参数），「归并出来的」与「我们写的」
     *    是同一份东西，`own_count` 一个数就够 —— 那一整套取舍钉在 `domain::layer`
     *    的单测里，这里不重复一遍。
     */

    /// 一份「已经落盘」的 [`Committed`]。
    ///
    /// **只有身份**：有哪些机型与版本、叫什么、什么角标，**一个值都没有** ——
    /// 值在这一层的唯一去处是 registry 的 `machineVariants`。
    /// 要造「已经写过的」用 [`saved_values`] 往那里写，不要往这里塞
    fn committed() -> Committed {
        let mut versions = BTreeMap::new();
        for (id, _bundle, vids) in FIXTURE_MACHINES {
            for (vid, name, tag) in *vids {
                versions.insert(
                    format!("{id}/{vid}"),
                    CommittedVersion {
                        machine_id: (*id).to_owned(),
                        version_id: (*vid).to_owned(),
                        name: (*name).to_owned(),
                        // 夹具里的空 tag 表示「没写过这一行」，不是「写了个空串」
                        tag: Some((*tag).to_owned()).filter(|t| !t.is_empty()),
                    },
                );
            }
        }
        Committed {
            versions,
            catalog: fixture_catalog(),
            ..Default::default()
        }
    }

    /// 造一批**已经落盘的值**，连同还活着的临时目录一起交出来。
    ///
    /// owner 机型层是 `"A1"`、版本层是 `"A1:STANDARD"` —— **冒号不是斜杠**：
    /// `machineVariants` 的键就长这样（doc §3.3）。
    ///
    /// 必须走 [`Fixture::into_parts`]：这批值要写盘，而盘在那个临时目录里。
    /// 目录被删之后再写的症状是一条八竿子打不着的「建不出文件」
    fn saved_values(
        edits: &[(&str, &str, serde_json::Value)],
    ) -> (tempfile::TempDir, Upstream, Presets) {
        let (dir, up, mut presets) = Fixture::load().into_parts();
        let batch: Vec<(String, String, Option<serde_json::Value>)> = edits
            .iter()
            .map(|(key, owner, v)| ((*key).to_owned(), (*owner).to_owned(), Some(v.clone())))
            .collect();
        presets
            .apply_values(&batch)
            .expect("夹具的字段与 owner 都是真的");
        (dir, up, presets)
    }

    fn cols(list: &[(&str, Option<&str>)]) -> Vec<ColRef> {
        list.iter()
            .map(|(m, v)| ColRef {
                machine_id: (*m).to_owned(),
                version_uid: v.map(str::to_owned),
            })
            .collect()
    }

    /* ---------- 整本 ---------- */

    /// 干净仓库的整本视图：机型树齐，徽章数的就是**合并后那一张表**里的项数
    #[test]
    fn a_clean_repo_derives_a_full_tree() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let view = Book::new(&f.up, &f.presets, &c, &d).book_view();

        assert_eq!(view.badges.machines, 3);
        assert_eq!(view.badges.versions, 4);
        assert_eq!(view.dirty_count, 0);
        assert_eq!(view.save, SaveState::Saved);

        // 每层只有一张表，所以这两个数就是「归并结果里那一层各有几项」：
        // P1S 只有 LITE 一个版本写了 offset.x → 上提到机型基底（doc §3.3 第 2 步）；
        // A1 的两个版本值不同 → 上提不成立，各留一条版本覆盖
        assert_eq!(view.badges.base_items, 1, "只有 P1S 那一项进了基底");
        assert_eq!(view.badges.override_items, 2, "A1 的两个版本各一条");

        // 徽章与树上逐个数的必须是同一个数 —— 这两个数在界面上是两处地方，对不上没人发现
        let per_machine: usize = view.machines.iter().map(|m| m.items).sum();
        let per_version: usize = view
            .machines
            .iter()
            .flat_map(|m| &m.versions)
            .map(|v| v.items)
            .sum();
        assert_eq!(per_machine, view.badges.base_items);
        assert_eq!(per_version, view.badges.override_items);

        let a1 = view.machines.iter().find(|m| m.id == "A1").unwrap();
        assert_eq!(a1.versions.len(), 2);
        assert!(!a1.dimensions_missing);
        assert_eq!(a1.items, 0, "A1 的两个版本值不同，上提不成立");
        let p1s = view.machines.iter().find(|m| m.id == "P1S").unwrap();
        assert_eq!(p1s.items, 1, "唯一版本的值被上提到了基底");
    }

    /// **写的地方就是看的地方**：值只进 `machineVariants`，树上立刻就是它，
    /// 中间没有第二份自造 json 要同步 —— 这一层不再有第二副本
    #[test]
    fn a_value_saved_to_the_registry_shows_up_on_the_tree() {
        let (_dir, up, presets) = saved_values(&[("wiping.child", "A1", serde_json::json!(33))]);
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&up, &presets, &c, &d);

        let l = b.machine_layers("A1").unwrap();
        assert_eq!(*l.effective("wiping.child").unwrap().value, serde_json::json!(33));
        assert_eq!(l.effective("wiping.child").unwrap().origin, Origin::Machine);

        // 两个版本自己都没写过这一项，继承的是基底
        for uid in ["A1/STANDARD", "A1/FAST"] {
            let l = b.version_layers(uid).unwrap();
            assert_eq!(*l.effective("wiping.child").unwrap().value, serde_json::json!(33));
            assert_eq!(
                l.effective("wiping.child").unwrap().origin,
                Origin::Machine,
                "{uid} 自己没写过这一项，取到的是机型层"
            );
        }

        let view = b.book_view();
        let a1 = view.machines.iter().find(|m| m.id == "A1").unwrap();
        assert_eq!(a1.items, 1, "机型层数出来的就是这一项");
        assert_eq!(view.badges.base_items, 2, "A1 这一项，加上 P1S 上提的那一项");
    }

    /// **A2L：参数侧与资源侧是两件事**（doc §11）。
    ///
    /// 资源那面「暂无资源」，而参数那面一项都不缺 —— 全落到出厂默认，能看能改
    #[test]
    fn a2l_says_no_resources_while_its_parameters_are_all_there() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        assert_eq!(b.build_state("A2L/STANDARD"), BuildState::NoResources);
        assert!(
            !b.has_any_recipe("A2L/STANDARD"),
            "machineVariants 没给它任何机型差异"
        );
        assert!(
            !b.recipe_empty("A2L/STANDARD"),
            "「未配置」是另一件事：它的有效配方不空，只是全是出厂默认"
        );

        // 参数那面：每一项都有值，来源都是出厂
        let l = b.version_layers("A2L/STANDARD").unwrap();
        let keys = l.keys();
        assert!(!keys.is_empty());
        for k in keys {
            let r = l.effective(k).expect("A2L 的参数不该缺");
            assert_eq!(r.origin, Origin::Factory);
        }

        let view = b.book_view();
        let a2l = view.machines.iter().find(|m| m.id == "A2L").unwrap();
        assert_eq!(a2l.build, BuildState::NoResources);
        assert!(a2l.dimensions_missing, "上游没登记它的尺寸");
    }

    /// 有一份归并出来的机型基底的版本要显示「未生成」而不是「暂无资源」——
    /// 后者会让人以为没救了，而这一版点一下生成就有了
    #[test]
    fn merged_base_makes_a_version_never_built_not_no_resources() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        // P1S 的唯一版本：`P1S:LITE` 那一条被上提成了机型基底 → 有配方
        assert!(b.has_any_recipe("P1S/LITE"));
        assert_eq!(b.build_state("P1S/LITE"), BuildState::NeverBuilt);
    }

    /// 树上的版本**照机型文件写下的顺序**，不是字典序。
    ///
    /// 顺序直接进界面，它是作者的话 —— 字典序会把 A1 的「标准版 / 高速版」换成对半翻转
    #[test]
    fn versions_keep_the_order_the_machine_file_wrote_them_in() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let got: Vec<&str> = b
            .live_versions("A1")
            .iter()
            .map(|v| v.version_id.as_str())
            .collect();
        assert_eq!(got, vec!["STANDARD", "FAST"]);
        // 反空转：夹具的顺序若哪天改成字典序，这条就失去对象了
        let mut sorted = got.clone();
        sorted.sort_unstable();
        assert_ne!(got, sorted, "夹具的顺序被改成字典序了，这条判据要重写");

        // 树 = 机型照机型文件顺序，机型内照 `[[versions]]` 顺序
        let uids: Vec<String> = b
            .book_view()
            .machines
            .iter()
            .flat_map(|m| &m.versions)
            .map(|v| v.uid.clone())
            .collect();
        assert_eq!(
            uids,
            vec!["A1/STANDARD", "A1/FAST", "A2L/STANDARD", "P1S/LITE"]
        );
    }

    /// 生成状态靠**指纹**，不靠文件时间
    #[test]
    fn build_state_compares_fingerprints() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();

        // 先记一条「按当前配方生成过」
        let fp = Book::new(&f.up, &f.presets, &c, &d)
            .version_layers("A1/STANDARD")
            .unwrap()
            .fingerprint();
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::MarkBuilt {
                uids: vec!["A1/STANDARD".to_owned()],
                stamp: "2026-01-01T00:00:00Z".to_owned(),
                fingerprints: [("A1/STANDARD".to_owned(), fp)].into_iter().collect(),
            }],
        )
        .unwrap();
        assert_eq!(
            Book::new(&f.up, &f.presets, &c, &d).build_state("A1/STANDARD"),
            BuildState::Built
        );

        // 改一个值 → 指纹变 → 待生成
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        assert_eq!(
            Book::new(&f.up, &f.presets, &c, &d).build_state("A1/STANDARD"),
            BuildState::Stale
        );
    }

    /// **改机型基底，跟着变的是没有自己覆盖的那些版本**（tasks 7.9）。
    ///
    /// A1 的两个版本在 `toolhead.offset.x` 上各有一条版本键（`A1:STANDARD` / `A1:FAST`）
    /// → 都不跟着变；`wiping.child` 两个版本都没写过 → 都跟着变。手算与派生要对上
    #[test]
    fn changing_the_machine_base_moves_exactly_the_versions_without_an_override() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();

        let before: Vec<String> = ["A1/STANDARD", "A1/FAST"]
            .iter()
            .map(|u| {
                Book::new(&f.up, &f.presets, &c, &d)
                    .version_layers(u)
                    .unwrap()
                    .fingerprint()
            })
            .collect();

        // ① 改一个两个版本都**有**版本键的字段 → 谁都不该变
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(123)),
            }],
        )
        .unwrap();
        let b = Book::new(&f.up, &f.presets, &c, &d);
        for (i, u) in ["A1/STANDARD", "A1/FAST"].iter().enumerate() {
            assert_eq!(
                b.version_layers(u).unwrap().fingerprint(),
                before[i],
                "{u} 在 offset.x 上有自己的那条键，改基底不该影响它"
            );
            assert_eq!(
                *b.version_layers(u)
                    .unwrap()
                    .effective("toolhead.offset.x")
                    .unwrap()
                    .value,
                if *u == "A1/STANDARD" {
                    serde_json::json!(-1)
                } else {
                    serde_json::json!(-0.7)
                },
                "版本层的键盖住机型基底"
            );
        }

        // ② 改一个两个版本都**没**写过的字段 → 两个都跟着变
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(44)),
            }],
        )
        .unwrap();
        let b = Book::new(&f.up, &f.presets, &c, &d);
        let mut moved = 0;
        for (i, u) in ["A1/STANDARD", "A1/FAST"].iter().enumerate() {
            if b.version_layers(u).unwrap().fingerprint() != before[i] {
                moved += 1;
            }
            let r = b.version_layers(u).unwrap();
            let hit = r.effective("wiping.child").unwrap();
            assert_eq!(*hit.value, serde_json::json!(44));
            assert_eq!(hit.origin, Origin::Machine, "值来自机型这一层");
        }
        assert_eq!(moved, 2, "两个版本都没写过它，所以两个都该跟着变");
    }

    /// 草稿里删键（挂回继承）**在保存之前就要看得出效果**
    #[test]
    fn a_pending_delete_shows_up_before_saving() {
        // 「已经写过的那一条」只能造在 registry 里 —— 值在这一层唯一的落盘处
        let (_dir, up, presets) =
            saved_values(&[("wiping.child", "A1:STANDARD", serde_json::json!(99))]);
        let c = committed();
        let mut d = Draft::default();

        let b = Book::new(&up, &presets, &c, &d);
        let l = b.version_layers("A1/STANDARD").unwrap();
        assert_eq!(*l.effective("wiping.child").unwrap().value, serde_json::json!(99));
        assert!(l.has_own(Level::Version, "wiping.child"), "这一层自己钉着它");

        apply(
            &mut d,
            &c,
            &presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: None,
            }],
        )
        .unwrap();

        // 挂回继承在这台机型上一路退到出厂默认：版本层删掉之后机型层那一项也不存在
        // （有落差时它只退一层，见 `domain::layer` 的 detaching_falls_back_one_level_at_a_time）
        let b = Book::new(&up, &presets, &c, &d);
        let l = b.version_layers("A1/STANDARD").unwrap();
        assert_eq!(*l.effective("wiping.child").unwrap().value, serde_json::json!(20));
        assert_eq!(l.effective("wiping.child").unwrap().origin, Origin::Factory);
        assert!(!l.has_own(Level::Version, "wiping.child"), "自有那一项该没了");
    }

    /* ---------- 矩阵 ---------- */

    /// **列序照配方本，不按勾选顺序**（tasks 7.5）。打乱输入，输出列序不变
    #[test]
    fn column_order_follows_the_recipe_book_not_the_click_order() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let want = ["A1", "A1/STANDARD", "A1/FAST", "P1S", "P1S/LITE"];
        for shuffled in [
            cols(&[
                ("P1S", Some("P1S/LITE")),
                ("A1", Some("A1/FAST")),
                ("A1", None),
                ("P1S", None),
                ("A1", Some("A1/STANDARD")),
            ]),
            cols(&[
                ("A1", Some("A1/STANDARD")),
                ("P1S", None),
                ("A1", None),
                ("P1S", Some("P1S/LITE")),
                ("A1", Some("A1/FAST")),
            ]),
        ] {
            let m = b.matrix(&shuffled, None, "");
            let got: Vec<&str> = m.cols.iter().map(|c| c.key.as_str()).collect();
            assert_eq!(got, want, "勾选顺序不该影响列序");
        }
    }

    /// 机型基底**本身就是一列**，而且它的 level 是 Machine —— 弄反就是静默打破继承
    #[test]
    fn the_machine_base_is_a_column_of_its_own() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let m = Book::new(&f.up, &f.presets, &c, &d).matrix(&cols(&[("A1", None)]), None, "");
        assert_eq!(m.cols.len(), 1);
        assert_eq!(m.cols[0].level, Level::Machine);
        assert_eq!(m.cols[0].version_uid, None);
        assert_eq!(m.cols[0].label, "机型基底");
    }

    /// **行取并集不取交集**：只有 P1S 有的那一项，在勾了 P1S 时不该消失
    #[test]
    fn rows_are_the_union_not_the_intersection() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None), ("P1S", None)]), None, "");
        let keys: Vec<&str> = m.rows.iter().map(|r| r.key.as_str()).collect();
        assert!(
            keys.contains(&"toolhead.only_p1s"),
            "交集会让这一项凭空消失：{keys:?}"
        );

        // 而 A1 那一列上它是「不适用」，不是空白
        let row = m.rows.iter().find(|r| r.key == "toolhead.only_p1s").unwrap();
        let a1_cell = &row.cells[0];
        assert_eq!(a1_cell.kind, CellKind::NotApplicable);
        assert_eq!(a1_cell.text, w::NOT_APPLICABLE);
        assert!(!a1_cell.editable);
        assert!(a1_cell.reason.is_some(), "禁用必须有理由");
        assert_eq!(row.cells[1].kind, CellKind::Value, "P1S 那一列是正常值");
    }

    /// 行序按**组**聚在一起：一个组名消失后又回来，就是两组被洗到一起了
    #[test]
    fn rows_are_grouped_by_section_not_interleaved() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None), ("P1S", None)]), None, "");
        let mut seen: Vec<&str> = Vec::new();
        let mut runs: Vec<&str> = Vec::new();
        for r in &m.rows {
            if runs.last() != Some(&r.section_label.as_str()) {
                runs.push(&r.section_label);
            }
        }
        for g in &runs {
            assert!(
                !seen.contains(g),
                "组「{g}」断开又回来了，说明分组被洗乱：{:?}",
                m.rows
                    .iter()
                    .map(|r| (&r.section_label, &r.key))
                    .collect::<Vec<_>>()
            );
            seen.push(g);
        }
        // 夹具的两组号段是**重叠**的，所以这个断言不是白跑的
        assert_eq!(runs, vec!["空间偏移", "擦料方式"], "组序照 tab.order");
        assert!(m.rows.iter().all(|r| !r.section_label.is_empty()));
    }

    /// 子项紧跟父项，并且带上父的中文名
    #[test]
    fn child_rows_follow_their_parent() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None)]), None, "");
        let at = |k: &str| m.rows.iter().position(|r| r.key == k).unwrap();
        assert_eq!(
            at("wiping.child"),
            at("wiping.mode") + 1,
            "子项必须紧跟父项：{:?}",
            m.rows.iter().map(|r| &r.key).collect::<Vec<_>>()
        );

        let child = &m.rows[at("wiping.child")];
        assert_eq!(child.depth, 1);
        assert_eq!(child.parent_key.as_deref(), Some("wiping.mode"));
        assert_eq!(child.parent_label.as_deref(), Some("擦拭部件"));

        let parent = &m.rows[at("wiping.mode")];
        assert_eq!(parent.depth, 0, "父项自己是顶层");
        assert!(parent.parent_label.is_none());
    }

    /// 同一份数据两次渲染的行序必须一字不差
    #[test]
    fn row_order_is_stable_for_equal_keys() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let want = &cols(&[("A1", None), ("P1S", None)]);
        let a: Vec<String> = b
            .matrix(want, None, "")
            .rows
            .into_iter()
            .map(|r| r.key)
            .collect();
        let z: Vec<String> = b
            .matrix(want, None, "")
            .rows
            .into_iter()
            .map(|r| r.key)
            .collect();
        assert_eq!(a, z);
    }

    /// 排序键的哨兵不许回绕：`f64::MAX` 压成整数后要是 `i64::MAX`，而不是一个负数
    #[test]
    fn unknown_order_saturates_instead_of_wrapping() {
        assert_eq!(milli(f64::MAX), i64::MAX);
        assert_eq!(milli(f64::MIN), i64::MIN);
        assert_eq!(milli(1.05), 1050);
        assert_eq!(milli(0.0), 0);
        assert!(milli(1.0) < milli(1.1) && milli(1.1) < milli(f64::MAX));
    }

    /// 关联关系要**常驻可读**：行头那句话在格子还没变灰的时候就在
    #[test]
    fn the_control_note_is_there_before_the_cell_goes_grey() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        // 干净状态下 wiping.mode 还是「擦料塔」，所以 wiping.child 是**可编辑**的
        let m = b.matrix(&cols(&[("A1", None)]), None, "");
        let child = m.rows.iter().find(|r| r.key == "wiping.child").unwrap();
        assert!(child.cells[0].editable, "这会儿还没被关着");
        assert_eq!(
            child.control_note.as_deref(),
            Some("受「擦拭部件」控制"),
            "没灰的时候也要说得出归谁管"
        );
        assert_eq!(child.parent_note.as_deref(), Some("属于：擦拭部件"));

        let mode = m.rows.iter().find(|r| r.key == "wiping.mode").unwrap();
        assert!(mode.control_note.is_none(), "它自己不受谁控制");
        assert!(mode.parent_note.is_none());
    }

    /// 灰格子要**说出是谁**并给出跳转目标 —— 光 `editable: false` 是在让人猜
    #[test]
    fn a_blocked_cell_names_its_controller_and_where_to_go() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("disk")),
            }],
        )
        .unwrap();

        let b = Book::new(&f.up, &f.presets, &c, &d);
        let m = b.matrix(&cols(&[("A1", None)]), None, "");
        let cell = &m
            .rows
            .iter()
            .find(|r| r.key == "wiping.child")
            .unwrap()
            .cells[0];

        assert!(!cell.editable);
        let note = cell.blocked_note.as_deref().expect("灰了必须有一句");
        assert!(note.contains("擦拭部件"), "没点名是谁关的：{note}");
        assert!(note.contains("等于 擦料塔"), "没说要改成什么：{note}");
        assert_eq!(cell.jump_to.as_deref(), Some("wiping.mode"), "得能跳过去");

        // 没被关着的格子不带这两样，否则界面上会多出一句空话
        let mode = &m
            .rows
            .iter()
            .find(|r| r.key == "wiping.mode")
            .unwrap()
            .cells[0];
        assert!(mode.blocked_note.is_none() && mode.jump_to.is_none());
    }

    /* ---------- 配方台 ---------- */

    /// 分组、计数、子项归位：配方台那一屏的骨架
    #[test]
    fn the_desk_groups_rows_and_hangs_children_under_their_parent() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let desk = b.desk("A1", Some("A1/STANDARD"), None, "");

        let labels: Vec<&str> = desk.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["空间偏移", "擦料方式"]);

        for g in &desk.groups {
            let rows: usize = g.items.iter().map(|i| 1 + i.children.len()).sum();
            assert_eq!(g.count, rows, "组「{}」的计数与行数对不上", g.label);
        }

        let wipe = desk.groups.iter().find(|g| g.label == "擦料方式").unwrap();
        let mode = wipe.items.iter().find(|i| i.row.key == "wiping.mode").unwrap();
        assert_eq!(mode.children.len(), 1);
        assert_eq!(mode.children[0].key, "wiping.child");
        assert!(
            wipe.items.iter().all(|i| i.row.key != "wiping.child"),
            "子项不该同时又是顶层项"
        );

        // 单列：每一行只有一格
        assert!(desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .all(|i| i.row.cells.len() == 1));
    }

    /// 左栏导航**不随搜索变**：它是换分组看的工具，搜一个词就塌掉的话就没用了
    #[test]
    fn the_nav_counts_do_not_follow_the_search() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let all = b.desk("A1", Some("A1/STANDARD"), None, "");
        let searched = b.desk("A1", Some("A1/STANDARD"), None, "擦料塔");
        assert_eq!(all.nav, searched.nav, "导航不该跟着搜索走");

        assert!(searched.groups.len() <= all.groups.len());
        assert!(searched.total > 0, "总数说的是过滤前");

        let a2l = b.desk("A2L", Some("A2L/STANDARD"), None, "");
        for t in &a2l.nav {
            assert!(t.count > 0);
            assert!(t.sections.iter().all(|s| s.count > 0));
        }
    }

    /// 父项把下面那几项关掉时，**给一句话让界面能收起来**
    #[test]
    fn a_closed_family_gets_one_sentence_instead_of_grey_cells() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("disk")),
            }],
        )
        .unwrap();

        let b = Book::new(&f.up, &f.presets, &c, &d);
        let desk = b.desk("A1", Some("A1/STANDARD"), None, "");
        let wipe = desk.groups.iter().find(|g| g.label == "擦料方式").unwrap();
        let mode = wipe.items.iter().find(|i| i.row.key == "wiping.mode").unwrap();

        let note = mode.off_note.as_deref().expect("关掉了就要说一句");
        assert!(note.contains("擦拭部件"), "要点名是谁关的：{note}");
        assert!(note.contains("圆盘擦拭"), "要说它现在是什么值：{note}");
        assert!(note.contains('1'), "要说关掉了几项：{note}");

        // 没关的时候不给这一句，否则界面上多一条空话
        let open =
            Book::new(&f.up, &f.presets, &c, &Draft::default()).desk("A1", Some("A1/STANDARD"), None, "");
        let mode2 = open
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.row.key == "wiping.mode")
            .unwrap();
        assert!(mode2.off_note.is_none());
    }

    /// 搜索时**不把子项塞进没命中的父项**，否则会让人以为父项也命中了
    #[test]
    fn searching_does_not_nest_children_under_a_parent_that_did_not_match() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        // 只命中子项（「塔位置 X」是 wiping.child 的中文名）
        let desk = b.desk("A1", Some("A1/STANDARD"), None, "塔位置");
        let items: Vec<&str> = desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .map(|i| i.row.key.as_str())
            .collect();
        assert_eq!(items, vec!["wiping.child"]);
        assert!(desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .all(|i| i.children.is_empty()));
        assert_eq!(desk.groups[0].label, "擦料方式");
    }

    /// 单元格三种 kind 各出现一次，且「被关着」与「不适用」分得开
    #[test]
    fn cells_separate_blocked_from_not_applicable() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        // 把 A1 基底的擦拭部件改成圆盘 → 塔位置被关着
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Machine,
                owner: "A1".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("disk")),
            }],
        )
        .unwrap();

        let b = Book::new(&f.up, &f.presets, &c, &d);
        let m = b.matrix(&cols(&[("A1", None)]), None, "");

        let child = m.rows.iter().find(|r| r.key == "wiping.child").unwrap();
        let cell = &child.cells[0];
        assert_eq!(cell.kind, CellKind::Value, "被关着的格子**有值**，会进产物");
        assert!(!cell.editable);
        assert_eq!(cell.blocked.len(), 1);
        assert_eq!(cell.blocked[0].key, "wiping.mode");
        assert_eq!(cell.blocked[0].need, "等于 擦料塔");
        assert_ne!(
            cell.reason.as_deref(),
            Some(w::disabled::NOT_APPLICABLE),
            "「被关着」和「不适用」要说不同的话"
        );

        // G-code 那一行报行数
        let script = m.rows.iter().find(|r| r.key == "toolhead.script").unwrap();
        assert!(script.gcode);
        assert_eq!(script.cells[0].kind, CellKind::Gcode);
        assert_eq!(script.cells[0].lines, Some(0), "出厂默认是空串");

        // 改过的那一格要带草稿标记
        let mode = m.rows.iter().find(|r| r.key == "wiping.mode").unwrap();
        assert!(mode.cells[0].dirty, "草稿里改过的格子要能标出来");
        assert_eq!(mode.cells[0].text, "圆盘擦拭", "枚举显示中文选项名");
        assert!(mode.cells[0].own, "我们在这一层写过它 → 挂回继承可点");
    }

    /// 分类过滤；**搜索一开分类让开**，并且说明条上写明这件事
    #[test]
    fn searching_overrides_the_tab_filter_and_says_so() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);
        let one = cols(&[("A1", None)]);

        let only_wiping = b.matrix(&one, Some("wiping"), "");
        assert!(only_wiping.rows.iter().all(|r| r.key.starts_with("wiping.")));
        assert!(only_wiping.note.is_none());

        // 搜一个只在 offset 分类里的词，但分类过滤停在 wiping
        let found = b.matrix(&one, Some("wiping"), "X 轴");
        assert!(
            found.rows.iter().any(|r| r.key == "toolhead.offset.x"),
            "搜索要跨全部分类，否则用户会以为这个字段不存在"
        );
        assert_eq!(found.note.as_deref(), Some(w::MATRIX_SEARCH_SPANS_ALL_TABS));

        // 搜 tomlKey 也要命中（三个偏移共享 tomlKey `offset`）
        assert!(b
            .matrix(&one, None, "offset")
            .rows
            .iter()
            .any(|r| r.key == "toolhead.offset.x"));
    }

    /// 空的时候**写出为什么空**，不留白
    #[test]
    fn an_empty_matrix_explains_itself() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        let no_cols = b.matrix(&[], None, "");
        assert_eq!(no_cols.empty_reason.as_deref(), Some(w::MATRIX_NO_COLS));

        let no_match = b.matrix(&cols(&[("A1", None)]), None, "根本没有这个词");
        assert_eq!(no_match.empty_reason.as_deref(), Some(w::MATRIX_NO_MATCH));
        assert!(no_match.total_rows > 0, "总行数要说过滤前的");
    }

    /* ---------- 仓库盘点 ---------- */

    /// 盘点只列交付物，且**每条都有真哈希** —— 这一页不该出现「未知」
    #[test]
    fn stock_lists_only_deliverables_with_real_hashes() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let rows = Book::new(&f.up, &f.presets, &c, &d).stock_rows();

        assert_eq!(rows.len(), 4, "3 个 MKP + 1 个 BBS，界面素材不算");
        assert!(rows.iter().all(|r| !r.sha256.is_empty() && r.size > 0));
        assert!(!rows.iter().any(|r| r.resource_type == ResourceType::Image));
    }

    /// **BBS 三态与「进没进套餐」正交**（tasks 7.6）
    #[test]
    fn bbs_assignment_and_bundle_membership_are_independent() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        // A1 的 defaultBundle 里有那条 BBS → A1 的两个版本继承它 → 已分配
        let row = b
            .stock_rows()
            .into_iter()
            .find(|r| r.id == "a1_bbs_04")
            .unwrap();
        assert_eq!(row.assign, BbsAssign::Assigned);
        assert!(row.in_any_bundle);
        assert_eq!(b.bbs_source("A1/STANDARD"), BbsSource::InheritedFromMachine);
        assert_eq!(b.effective_bbs("A1/STANDARD"), vec!["a1_bbs_04"]);

        // P1S 没有 defaultBundle → 它的版本一条 BBS 都没有
        assert!(b.effective_bbs("P1S/LITE").is_empty());

        // 标成仅归档 → 三态变了，但"进没进套餐"没变
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetVisibility {
                file_id: "a1_bbs_04".to_owned(),
                visibility: Visibility::ArchiveOnly,
            }],
        )
        .unwrap();
        let row = Book::new(&f.up, &f.presets, &c, &d)
            .stock_rows()
            .into_iter()
            .find(|r| r.id == "a1_bbs_04")
            .unwrap();
        assert_eq!(row.assign, BbsAssign::ArchiveOnly);
        assert!(row.in_any_bundle, "两个字段互不影响");
    }

    /// **一个版本交付哪几条曲线 = 它那台机型的默认套餐**（REPORT §7.3）。
    ///
    /// 「这一版自己挑一串」这条路径删掉了，所以改套餐会**同时**带动这台机型下的每一版 ——
    /// 没有哪一版能偷偷留一份
    #[test]
    fn every_version_of_a_machine_draws_from_the_same_bundle() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        let b = Book::new(&f.up, &f.presets, &c, &d);

        for uid in ["A1/STANDARD", "A1/FAST"] {
            assert_eq!(
                b.bbs_source(uid),
                BbsSource::InheritedFromMachine,
                "只有一种来源了"
            );
            assert_eq!(b.effective_bbs(uid), vec!["a1_bbs_04"]);
        }

        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetBundle {
                bundle_id: "A1_default".to_owned(),
                presets: Vec::new(),
                bbs: vec!["a1_bbs_04".to_owned(), "a1_bbs_06".to_owned()],
            }],
        )
        .unwrap();

        let b = Book::new(&f.up, &f.presets, &c, &d);
        for uid in ["A1/STANDARD", "A1/FAST"] {
            assert_eq!(
                b.effective_bbs(uid),
                vec!["a1_bbs_04", "a1_bbs_06"],
                "{uid} 跟着套餐变 —— 它自己存不了第二份"
            );
        }
        assert!(
            b.effective_bbs("P1S/LITE").is_empty(),
            "P1S 没有默认套餐，不受影响"
        );
    }

    /* ---------- 生成视角 ---------- */

    /// 生成表每行都要说得出「现在该做什么」；不能生成的行要有理由
    #[test]
    fn build_rows_always_carry_a_reason() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let rows = Book::new(&f.up, &f.presets, &c, &d).build_rows();

        assert_eq!(rows.len(), 4);
        for r in &rows {
            assert!(!r.reason.trim().is_empty(), "{} 没有理由", r.uid);
            if !r.buildable {
                assert!(r.disabled_reason.is_some(), "{} 禁用却没说为什么", r.uid);
            }
        }
        let a2l = rows.iter().find(|r| r.uid == "A2L/STANDARD").unwrap();
        assert_eq!(a2l.state, BuildState::NoResources);
        assert!(!a2l.buildable);
        assert_eq!(a2l.mkp_file, None);

        let a1 = rows.iter().find(|r| r.uid == "A1/STANDARD").unwrap();
        assert_eq!(a1.state, BuildState::NeverBuilt);
        assert!(a1.buildable);
        assert_eq!(a1.mkp_file.as_deref(), Some("A1.toml"));
    }

    /// 整本产物状态：**「暂无资源」的版本不参与** ——
    /// 算进去的话，只要仓库里有一台 A2L，整本永远是「未生成」
    #[test]
    fn the_overall_artifact_state_ignores_versions_with_no_resources() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();

        assert_eq!(
            Book::new(&f.up, &f.presets, &c, &d).book_view().artifact,
            ArtifactState::Missing
        );

        // 把三个有产物的版本都记成已生成
        let b = Book::new(&f.up, &f.presets, &c, &d);
        let uids = ["A1/STANDARD", "A1/FAST", "P1S/LITE"];
        let fps: BTreeMap<String, String> = uids
            .iter()
            .map(|u| ((*u).to_owned(), b.version_layers(u).unwrap().fingerprint()))
            .collect();
        drop(b);
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::MarkBuilt {
                uids: uids.iter().map(|u| (*u).to_owned()).collect(),
                stamp: "2026-01-01T00:00:00Z".to_owned(),
                fingerprints: fps,
            }],
        )
        .unwrap();

        let view = Book::new(&f.up, &f.presets, &c, &d).book_view();
        assert_eq!(
            view.artifact,
            ArtifactState::Fresh,
            "A2L 还是「暂无资源」，但它不该拖住整本"
        );
        assert_eq!(view.last_build.as_deref(), Some("2026-01-01T00:00:00Z"));
    }
}
