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
use crate::workbench::presets::{AssetKind, Presets};

use super::layer::{no_overrides, Layers, Level, Origin, Overrides};
use super::patch::{variant_key, BuiltRecord, BundleEdit, CatalogMachine, Committed, Draft};
use super::variants::digest;
use super::visibility::{BlockScope, BlockedBy, Gate};
use super::wording as w;
use super::wording::{ArtifactState, BbsSource, BuildState, SaveState, SnapshotState};

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
    /// 这一版的产物文件名（`{机型}-{版本小写}.toml`）。**由命名规则算出** ——
    /// 不再读上游 manifest：名字只由「机型 id + 版本 id」决定，两者都在我们自己的清单里
    pub mkp_file: String,
}

/* ---------- 整本 ---------- */

pub struct Book<'a> {
    /// 我们自己那份预设数据（`<repo>/presets`，**唯一的预设真相源**）。
    /// 字段定义与三层取值全走它
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
    bundles: BTreeMap<String, BundleEdit>,
}

impl<'a> Book<'a> {
    pub fn new(presets: &'a Presets, committed: &'a Committed, draft: &'a Draft) -> Self {
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
                    // 产物名由命名规则算出（唯一实现在 `preset::preset_file_name`）——
                    // 不再查上游 manifest，清单里有这一版就有这个名字
                    mkp_file: preset::preset_file_name(&m.id, vid),
                });
            }
        }

        // 台账**只认已落盘的那一份**（2026-10-06 状态机修正）：生成事务直接把记录写进
        // `workbench/built.json`，草稿里没有生成记录这一说了 —— 叠加草稿会让「本机的
        // 已生成」与干净检出（CI）分叉，真机踩过（产物新、台账旧 → 本机 Built、CI Stale）。
        let built = committed.built.clone();
        let mut bundles = committed.bundles.clone();
        bundles.extend(draft.bundles.clone());

        Self {
            presets,
            draft,
            catalog: &committed.catalog,
            bases,
            overs,
            versions,
            built,
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
        Some(Layers::new(
            &self.presets.registry,
            id,
            base,
            no_overrides(),
        ))
    }

    /// 一个版本的三层视图。
    ///
    /// 机型层取它那台机型的裸键，版本层只取 `A1: FAST` 这一条 ——
    /// 别的版本的键是别的版本的，不参与这一列的取值。
    /// **走 `for_version`**：版本身份进指纹 —— 同机型两个版本值一模一样也是两份产物，
    /// 过期判定各算各的，不许共用一个指纹
    pub fn version_layers(&self, uid: &str) -> Option<Layers<'_>> {
        let v = self.version(uid)?;
        let base = self.bases.get(&v.machine_id)?;
        let over = self.overs.get(uid)?;
        let id = self.machine(&v.machine_id)?.id.as_str();
        Some(Layers::for_version(
            &self.presets.registry,
            id,
            v.version_id.as_str(),
            base,
            over,
        ))
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

    /// 套餐里的 BBS。草稿改动优先于 `bundles.toml` 登记的那一份
    pub fn bundle_bbs(&self, bundle_id: &str) -> Vec<String> {
        if let Some(edit) = self.bundles.get(bundle_id) {
            return edit.bbs.clone();
        }
        self.presets
            .bundles
            .get(bundle_id)
            .map(|b| {
                b.asset_refs
                    .iter()
                    .filter(|id| {
                        self.presets.assets.get(id).is_some_and(|a| {
                            a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs")
                        })
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// 一个版本最终交付哪些 BBS = **它自己指的那个套餐**（一版一套）。
    ///
    /// 指向模型是 C14 第十九轮定稿、作者 2026-09-30 拍板移植的「一版一套」：
    /// 版本的 `recommendedBundle` 优先；没指的（A2L 那处空串）回退机型 `defaultBundle`。
    /// 之前这里只看机型默认（REPORT §7.3 那刀收得太狠）—— 套餐页把「改指向」
    /// 接进来之后（P4），再只看机型的话界面就是在说谎：指过去了、交付却没跟上。
    ///
    /// 套餐的内容照 [`Self::bundle_bbs`] 的口径：改动 overlay 优先，没有就走上游。
    pub fn effective_bbs(&self, uid: &str) -> Vec<String> {
        let Some(v) = self.version(uid) else {
            return Vec::new();
        };
        let own = self
            .presets
            .catalog
            .machine(&v.machine_id)
            .and_then(|m| m.versions.iter().find(|x| x.id == v.version_id))
            .and_then(|x| x.recommended_bundle.as_deref())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let bid = match own {
            Some(b) => b,
            // 版本没指 → 机型默认（machine_default_bbs 自己处理「没配」那档）
            None => {
                return self.machine_default_bbs(&v.machine_id);
            }
        };
        self.bundle_bbs(bid)
    }

    /// 一版一套下的来源两档：版本自己指了套餐就是「本版本一份」，
    /// 没指（回退机型默认）才是「跟机型默认」。
    pub fn bbs_source(&self, uid: &str) -> BbsSource {
        let own = self.version(uid).is_some_and(|v| {
            self.presets
                .catalog
                .machine(&v.machine_id)
                .and_then(|m| m.versions.iter().find(|x| x.id == v.version_id))
                .and_then(|x| x.recommended_bundle.as_deref())
                .is_some_and(|s| !s.trim().is_empty())
        });
        if own {
            BbsSource::Own
        } else {
            BbsSource::InheritedFromMachine
        }
    }

    /* ---------- 生成状态 ---------- */

    /// 四档之一。判据是**指纹比对，不看文件时间**
    pub fn build_state(&self, uid: &str) -> BuildState {
        if self.version(uid).is_none() {
            return BuildState::NeverBuilt;
        }
        let has_bbs = !self.effective_bbs(uid).is_empty();

        // 产物名总是算得出来（命名规则只认机型 + 版本），所以「有没有可产出的东西」
        // 现在只看**配方**：机型层或版本层在这一版上钉过值，就有东西可产。
        // 全空的占位版本（既无配方也无 BBS）才是「暂无资源」
        if !self.has_any_recipe(uid) && !has_bbs {
            return BuildState::NoResources;
        }
        let Some(rec) = self.built.get(uid) else {
            // 生成记录里没有这一版 —— 但磁盘上可能躺着以前生成的产物
            //（记录丢了：快照回退 / 换机器 / 清过 store）。这时候说「未生成」
            // 是在撒谎：客户端现在就能下载到那份旧的。归成 Stale ——
            // 有旧的，且至少对不上「现在会算出来的这份」，该重新生成。
            // （真机踩过：预演说 9 份全是「修改」，版本行却全说「未生成」。）
            if self.product_on_disk(uid) {
                return BuildState::Stale;
            }
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

    /// 已生成记录里的配方指纹（没有记录 = `None`）。
    ///
    /// 生成侧拿它决定「这一行要不要补记」：产物字节没变、记录也对得上，
    /// 台账就不许动 —— no-op 的生成不该把 `built.json` 顶新（2026-10-05）。
    pub fn built_fingerprint(&self, uid: &str) -> Option<&str> {
        self.built.get(uid).map(|r| r.fingerprint.as_str())
    }

    /// 磁盘的交付根里有没有这一版的产物文件（只 stat，不读内容、不算哈希）。
    ///
    /// 路径与生成 / 发布写盘同源（`paths` 的 `DIST_SUBDIR` + `MKP_PRESETS_DIR`，
    /// 根用 `presets.root()` —— 与本 `Book` 读的数据同一棵树），不许第二处自拼。
    fn product_on_disk(&self, uid: &str) -> bool {
        let Some(v) = self.version(uid) else {
            return false;
        };
        self.presets
            .root()
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join(crate::workbench::paths::MKP_PRESETS_DIR)
            .join(&v.mkp_file)
            .exists()
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
            let base_count = ml
                .as_ref()
                .map(|l| l.own_count(Level::Machine))
                .unwrap_or(0);
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
                    mkp_file: Some(v.mkp_file.clone()),
                    bbs_count: self.effective_bbs(&v.uid).len(),
                    last_build: self.last_build(&v.uid).map(str::to_owned),
                }
            })
            .collect()
    }

    /* ---------- 矩阵 ---------- */

    /// 一屏矩阵。列由前端勾选给出，**但顺序由这里按配方本重排**。
    ///
    /// `base_machine`（对照模式的基准机型，C14 第四轮）带来三件事：
    ///
    /// 1. **行序跟基准机型走**：它自己的参数按它的分组顺序排完，
    ///    别的机型多出来的参数接在后面（正在看的人不用重新认位置）；
    /// 2. **差异判据**：每格 `differs` = 这个值与基准机型的机型基底不同
    ///    （比较格式化后的文本，与原型的 `cell.text` 同一条）；任一列不同的行
    ///    进 `diff_keys`（「仅显示差异」与状态列读它）。基准列自己不是差异；
    ///    基准机型没有的参数行没有基准可比，进 `not_own_keys`（状态列写
    ///    「本机无此项」）。
    /// 3. 差异格的悬停句 `diff_tip`（「机型基底是 X」）在后端拼好。
    ///
    /// 配方台（desk）传 `None`：那边没有「跟基准比」这个问题。
    pub fn matrix(
        &self,
        cols: &[ColRef],
        tab: Option<&str>,
        query: &str,
        base_machine: Option<&str>,
    ) -> Matrix {
        let cols = self.order_cols(cols);
        let q = query.trim().to_lowercase();
        let searching = !q.is_empty();

        // 基准机型的三层与它看得到的参数（`base_machine` 认不出时按没有基准算）
        let base_layers = base_machine.and_then(|id| self.machine_layers(id));
        let base_keys: Vec<&str> = match base_machine {
            Some(id) if self.catalog.iter().any(|m| m.id == id) => {
                self.presets.registry.desk_keys(id)
            }
            _ => Vec::new(),
        };
        let in_base = |key: &str| base_keys.contains(&key);

        // 行取并集：任意一列的机型有这个字段，这一行就在。
        // **走 `desk_keys` 不走 `visible_keys`**：这一屏是给人看的，弃用的参数
        // 要划线 + 禁用地摆着（C14 §五），不进产物是生成侧（`visible_keys`）的事
        let mut keys: Vec<&str> = Vec::new();
        for c in &cols {
            for k in self.presets.registry.desk_keys(&c.machine_id) {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        // 行序（C14 第四轮）：**基准机型的参数先排完，别家多出来的接后面**；
        // 没有基准时按全局五段键（配方台与旧对照的同一把尺）
        if !base_keys.is_empty() {
            let mut head: Vec<&str> = base_keys
                .iter()
                .copied()
                .filter(|k| keys.contains(k))
                .collect();
            let mut extras: Vec<&str> = keys
                .iter()
                .filter(|k| !base_keys.contains(k))
                .copied()
                .collect();
            extras.sort_by(|a, b| self.row_sort_key(a).cmp(&self.row_sort_key(b)));
            head.extend(extras);
            keys = head;
        } else {
            keys.sort_by(|a, b| self.row_sort_key(a).cmp(&self.row_sort_key(b)));
        }
        let total_rows = keys.len();

        let gates: Vec<Option<Gate<'_>>> = cols
            .iter()
            .map(|c| {
                c.layers
                    .as_ref()
                    .map(|l| Gate::new(&self.presets.registry, l))
            })
            .collect();

        let mut rows: Vec<Row> = Vec::new();
        let mut diff_keys: Vec<String> = Vec::new();
        let mut not_own_keys: Vec<String> = Vec::new();
        for key in keys {
            let p = self.presets.registry.param(key);
            let Some(p) = p else {
                continue;
            };
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
                    continue;
                }
            } else if let Some(t) = tab {
                if !self.tab_of(&p.layout.section_id).is_some_and(|x| x == t) {
                    continue;
                }
            }

            // 基准值（格式化后的文本）。基准机型没有这个参数 = 没有基准可比
            let base_text: Option<String> = match (&base_layers, in_base(key)) {
                (Some(l), true) => l.effective(key).map(|hit| w::value_text(p, hit.value)),
                _ => None,
            };

            let parent = p
                .parent_key
                .as_deref()
                .and_then(|k| self.presets.registry.param(k));
            let mut cells: Vec<Cell> = Vec::new();
            let mut row_differs = false;
            for (c, g) in cols.iter().zip(&gates) {
                let mut cell = self.cell(c, g.as_ref(), key);
                let is_base_col = c.machine_id.as_str() == base_machine.unwrap_or("")
                    && c.level == Level::Machine;
                let differs = match (&base_text, is_base_col) {
                    (Some(bt), false) => cell.text != *bt,
                    _ => false,
                };
                if differs {
                    row_differs = true;
                    cell.differs = true;
                    cell.diff_tip = base_text.as_deref().map(w::relate::base_value_is);
                }
                cells.push(cell);
            }
            if row_differs {
                diff_keys.push(key.to_owned());
            }
            if !in_base(key) {
                not_own_keys.push(key.to_owned());
            }

            rows.push(Row {
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
                // 矩阵的行跨多台机型，「改了影响谁」不知道该答哪一台 —— 那是
                // 配方台（desk）的事，那边填 [`DeskImpact`]
                impact: None,
                cells,
            });
        }

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
            diff_keys,
            not_own_keys,
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
                        items: layers
                            .as_ref()
                            .map(|l| l.own_count(Level::Machine))
                            .unwrap_or(0),
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
                        items: layers
                            .as_ref()
                            .map(|l| l.own_count(Level::Version))
                            .unwrap_or(0),
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
        // **这一屏走 `view_effective`**：弃用的参数也要显示它的值（C14 §五）。
        // 生成侧的 `effective` 会把弃用的键查成 None —— 那是「不进产物」的判据，
        // 不是「不适用」；两者在这一格上必须分开
        let Some(hit) = layers.view_effective(key) else {
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
                    (
                        Some(w::relate::blocked_note(&b.label, &b.need)),
                        Some(b.key.clone()),
                    )
                } else {
                    // 控制它的那一项这台机型上根本没有 —— **不给跳转**。
                    // 这是上游数据问题，骗用户去点一个不存在的格子更糟
                    (Some(w::relate::controller_not_here(&b.label)), None)
                }
            }
        };
        let blocked_hint = blocked
            .first()
            .map(|b| w::relate::blocked_hint(&b.label, &b.need));

        let (kind, text, lines) = if w::is_gcode(p) {
            let (n, t) = w::gcode_text(hit.value);
            (CellKind::Gcode, t, Some(n))
        } else {
            (CellKind::Value, w::value_text(p, hit.value), None)
        };

        // 已弃用（C14 §五）**不并进 blocked**：blocked 说「条件不成立，换个条件就能用」，
        // 弃用说「这一项正在退场」—— 对弃用的行说「要 X 才可改」是假话。
        // 它只做两件事：把 editable 关掉，给界面一档标记（Row.deprecated）去说自己的话。
        let deprecated = p.deprecated;

        Cell {
            kind,
            text,
            lines,
            origin: Some(hit.origin),
            origin_label: Some(w::origin_label(hit.origin).to_owned()),
            origin_explain: Some(w::origin_explain(hit.origin).to_owned()),
            own,
            // 改动还在草稿里 —— 界面用蓝色描边标它，不换底色（doc §7 视觉规范）
            dirty: self
                .draft
                .pending(col.level, &col.key_of_level(), key)
                .is_some(),
            editable: blocked.is_empty() && !deprecated,
            reason: if !blocked.is_empty() {
                Some(w::disabled::BLOCKED_BY_CONDITION.to_owned())
            } else if deprecated {
                Some(w::deprecated::PARAM_EXPLAIN.to_owned())
            } else {
                None
            },
            blocked,
            blocked_note,
            blocked_hint,
            jump_to,
            // 对照模式的差异在 matrix() 里按基准机型判（cell 不知道基准是谁）
            differs: false,
            diff_tip: None,
            raw: hit.value.clone(),
            rest: self.rest_at(col, key),
        }
    }

    /// 这一列**盘上**钉着的值（草稿不算）。`None` = 这一层没钉着它 ——
    /// 界面上的「恢复修改前的」要据此写 `null` **删键**、让它挂回继承，
    /// 而不是写回一个值（写回值会把继承钉死，那是另一回事）。
    fn rest_at(&self, col: &ResolvedCol<'_>, key: &str) -> Option<Value> {
        let p = self.presets.registry.param(key)?;
        p.machine_variants
            .get(&variant_key(col.level, &col.key_of_level()))
            .cloned()
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
        let sw = self
            .presets
            .registry
            .section_show_when(&p.layout.section_id)?;
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
    这里做的只有四件事：

    1. 按 `section_id` 切成组（行序已经保证同组连续，所以切一刀就够）
    2. 把子项挂到父项下面
    3. 父项把子项整组关掉时给一句话，让界面能把它们收起来
    4. **列给全**（C14）：基底 + 这一机型所有版本各一列 —— 右栏「各版本取值」
       每一层都是一行编辑控件（C14 的三栏工作台模型），不能让前端为选一个参数
       再去问一次矩阵。`cur` 指认请求的那一层在哪一列，前端不用猜

    左栏那份导航**不跟着搜索变**：它从字段定义直接数，
    否则搜一个词整棵导航树就塌了，而那正是用来换分组看的东西。
    */
    pub fn desk(
        &self,
        machine_id: &str,
        uid: Option<&str>,
        tab: Option<&str>,
        query: &str,
    ) -> Desk {
        // 基底列 + 这一机型的全部版本列。列序由 `order_cols` 定（基底最前、
        // 版本跟清单顺序），右栏「各版本取值」要的正是这个顺序
        let mut want = vec![ColRef {
            machine_id: machine_id.to_owned(),
            version_uid: None,
        }];
        for v in self.versions.iter().filter(|v| v.machine_id == machine_id) {
            want.push(ColRef {
                machine_id: machine_id.to_owned(),
                version_uid: Some(v.uid.clone()),
            });
        }
        let m = self.matrix(&want, tab, query, None);
        // 请求的那一层在哪一列。找不到（版本刚被删掉之类的竞态）退回基底列：
        // 界面显示的是「这一层现在长什么样」，基底永远存在
        let cur = m
            .cols
            .iter()
            .position(|c| c.version_uid.as_deref() == uid)
            .unwrap_or(0);
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
                && row
                    .parent_key
                    .as_deref()
                    .is_some_and(|p| g.items.last().map(|i| i.row.key.as_str()) == Some(p));
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

        // 整组 / 整族被关掉的那句话。**判读当前请求那一层** —— 关没关是跟着
        // 这一层的值走的（版本改了开关，基底还关着，在版本层看就是开着的）
        for g in &mut groups {
            for it in &mut g.items {
                it.off_note = family_off_note(&it.row, &it.children, cur);
            }
            g.off_note = group_off_note(g, cur);
        }

        // 「改了影响谁」（C14 抽屉的作用域栏）。**逐行各填各的** —— 子项也是
        // 可以选进右栏的，它们的传播面与父项无关
        let mut rows: Vec<&mut Row> = Vec::new();
        for g in &mut groups {
            for it in &mut g.items {
                rows.push(&mut it.row);
                rows.extend(it.children.iter_mut());
            }
        }
        for row in rows {
            row.impact = Some(self.impact_of(machine_id, uid, &row.key));
        }

        Desk {
            nav: self.desk_nav(machine_id),
            cols: m.cols,
            cur,
            groups,
            total: m.total_rows,
            note: m.note,
            empty_reason: m.empty_reason,
        }
    }

    /// 「改了影响谁」（C14）。
    ///
    /// 版本层的格子只影响这一版（别的版本各有各的一格，这正是覆盖制的形状）；
    /// 机型基底那一格影响**所有没自己钉**的版本。所以两档返回的不是同一个问题的答案，
    /// 界面上也就分开写：前者是「影响版本」，后者是「谁在跟着基底」。
    fn impact_of(&self, machine_id: &str, uid: Option<&str>, key: &str) -> DeskImpact {
        let label = |v: &VersionIdentity| -> String {
            let m = self
                .catalog
                .iter()
                .find(|m| m.id == v.machine_id)
                .map_or(&v.machine_id, |m| &m.display);
            format!("{m} / {}", v.name)
        };
        let pins = |v: &VersionIdentity| -> bool {
            self.version_layers(&v.uid)
                .is_some_and(|l| l.has_own(Level::Version, key))
        };
        let mine: Vec<&VersionIdentity> = self
            .versions
            .iter()
            .filter(|v| v.machine_id == machine_id)
            .collect();
        match uid {
            Some(u) => DeskImpact {
                targets: mine
                    .iter()
                    .filter(|v| v.uid == u)
                    .map(|v| label(v))
                    .collect(),
                followers: mine
                    .iter()
                    .filter(|v| v.uid != u && !pins(v))
                    .map(|v| label(v))
                    .collect(),
            },
            None => {
                let followers: Vec<&&VersionIdentity> = mine.iter().filter(|v| !pins(v)).collect();
                DeskImpact {
                    targets: followers.iter().map(|v| label(v)).collect(),
                    followers: Vec::new(),
                }
            }
        }
    }

    /// 左栏导航：tab → section 两级 + 计数。**直接从字段定义数**，不受搜索与分类影响。
    /// 与矩阵的行同一份来源（`desk_keys`）：弃用的参数占着位置，计数里也算它
    fn desk_nav(&self, machine_id: &str) -> Vec<DeskNavTab> {
        let mut per_section: BTreeMap<&str, usize> = BTreeMap::new();
        for key in self.presets.registry.desk_keys(machine_id) {
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

/// 父项把下面整族关掉了吗。**判据是子项全部改不动，而且是同一个父项关的**。
/// `cur` = 请求的那一层在 cells 里的下标 —— 关没关跟着这一层的值走
fn family_off_note(parent: &Row, children: &[Row], cur: usize) -> Option<String> {
    if children.is_empty() {
        return None;
    }
    let all_off = children.iter().all(|c| {
        c.cells
            .get(cur)
            .is_some_and(|cell| cell.blocked.iter().any(|b| b.key == parent.key))
    });
    if !all_off {
        return None;
    }
    let value = parent.cells.get(cur).map_or("", |c| c.text.as_str());
    Some(w::relate::family_off(&parent.label, value, children.len()))
}

/// 整组被 section 级条件关掉了吗。`cur` 同上
fn group_off_note(g: &DeskGroup, cur: usize) -> Option<String> {
    let mut who: Option<(&str, &str)> = None;
    for it in &g.items {
        let cell = it.row.cells.get(cur)?;
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
    let all: Vec<BuildState> = states.filter(|s| *s != BuildState::NoResources).collect();
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

/* ---------- 矩阵 ---------- */

/// 配方台一屏：左栏导航 + 分组列表 + **这一机型的全部层**（C14）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Desk {
    /// 左栏。**不随搜索变** —— 它是换分组看的工具
    pub nav: Vec<DeskNavTab>,
    /// 基底 + 这一机型所有版本，各一列。行的 `cells` 与它**一一对应** ——
    /// 右栏「各版本取值」每层一行编辑控件（C14 的三栏工作台），数据从这里出
    pub cols: Vec<Col>,
    /// 请求的那一层在 `cols` 里的下标。正文那格 = `row.cells[cur]`
    pub cur: usize,
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
    /// 有任一勾选列与基准机型基底**不同**的行（C14 对照）。前端「仅显示差异」
    /// 与状态列的「差异/一致」读它；没有基准（desk 路径）时为空
    pub diff_keys: Vec<String>,
    /// 基准机型**没有**的参数行（从别的机型并进来的）—— 状态列写「本机无此项」
    pub not_own_keys: Vec<String>,
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
    /// 「改了影响谁」（C14 抽屉的作用域栏）。**配方台逐行填**；矩阵的行跨多台
    /// 机型、答不出「哪一台」，是 `None`
    pub impact: Option<DeskImpact>,
    pub cells: Vec<Cell>,
}

/// 「改这里影响」+「谁在跟着基底」。
///
/// 版本层编辑：targets = 这一版本身，followers = 现在还跟着基底的其它版本
/// （「想一次改一片就得去基底」）；机型基底编辑：targets = 没自己钉的那些版本。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeskImpact {
    /// 「A1 / 标准版」这种，直接可显示
    pub targets: Vec<String>,
    pub followers: Vec<String>,
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
    /// 行上的短提示（C14 §一/二）：「要 X 才可改」。与 `blocked_note` 同源不同场合
    pub blocked_hint: Option<String>,
    /// 「去改那一项」跳到哪个字段。`None` = **不给跳转按钮**
    pub jump_to: Option<String>,
    /// 对照模式（C14 第四轮）：这一格的值与基准机型基底**不同**（绿底）。
    /// 基准列自己恒 false；desk 路径恒 false
    pub differs: bool,
    /// 差异格的悬停句（「机型基底是 X」）。**后端拼好的**，前端不组装
    pub diff_tip: Option<String>,
    /// 原始值。受控控件要用它，不能拿格式化过的文本回填
    pub raw: Value,
    /// 这一层**盘上**钉着的值（草稿不算）。`null` = 这一层没钉着它 ——
    /// 「恢复修改前的」要写 `null` 删键、挂回继承（批量抽屉用）
    pub rest: Option<Value>,
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
            blocked_hint: None,
            jump_to: None,
            differs: false,
            diff_tip: None,
            raw: Value::Null,
            rest: None,
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
    fn saved_values(edits: &[(&str, &str, serde_json::Value)]) -> (tempfile::TempDir, Presets) {
        let (dir, mut presets) = Fixture::load().into_parts();
        let batch: Vec<(String, String, Option<serde_json::Value>)> = edits
            .iter()
            .map(|(key, owner, v)| ((*key).to_owned(), (*owner).to_owned(), Some(v.clone())))
            .collect();
        presets
            .apply_values(&batch)
            .expect("夹具的字段与 owner 都是真的");
        (dir, presets)
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
        let view = Book::new(&f.presets, &c, &d).book_view();

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
        let (_dir, presets) = saved_values(&[("wiping.child", "A1", serde_json::json!(33))]);
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&presets, &c, &d);

        let l = b.machine_layers("A1").unwrap();
        assert_eq!(
            *l.effective("wiping.child").unwrap().value,
            serde_json::json!(33)
        );
        assert_eq!(l.effective("wiping.child").unwrap().origin, Origin::Machine);

        // 两个版本自己都没写过这一项，继承的是基底
        for uid in ["A1/STANDARD", "A1/FAST"] {
            let l = b.version_layers(uid).unwrap();
            assert_eq!(
                *l.effective("wiping.child").unwrap().value,
                serde_json::json!(33)
            );
            assert_eq!(
                l.effective("wiping.child").unwrap().origin,
                Origin::Machine,
                "{uid} 自己没写过这一项，取到的是机型层"
            );
        }

        let view = b.book_view();
        let a1 = view.machines.iter().find(|m| m.id == "A1").unwrap();
        assert_eq!(a1.items, 1, "机型层数出来的就是这一项");
        assert_eq!(
            view.badges.base_items, 2,
            "A1 这一项，加上 P1S 上提的那一项"
        );
    }

    /// **A2L：参数侧与资源侧是两件事**（doc §11）。
    ///
    /// 资源那面「暂无资源」，而参数那面一项都不缺 —— 全落到出厂默认，能看能改
    #[test]
    fn a2l_says_no_resources_while_its_parameters_are_all_there() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

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
        let b = Book::new(&f.presets, &c, &d);

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
        let b = Book::new(&f.presets, &c, &d);

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
        let mut c = committed();
        let d = Draft::default();

        // 先记一条「按当前配方生成过」—— 记在**台账**（2026-10-06 状态机修正：
        // 生成记录不住在草稿里，Book 的台账来源就是 committed.built）
        let fp = Book::new(&f.presets, &c, &d)
            .version_layers("A1/STANDARD")
            .unwrap()
            .fingerprint();
        c.built.insert(
            "A1/STANDARD".to_owned(),
            BuiltRecord {
                stamp: "2026-01-01T00:00:00Z".to_owned(),
                fingerprint: fp,
            },
        );
        assert_eq!(
            Book::new(&f.presets, &c, &d).build_state("A1/STANDARD"),
            BuildState::Built
        );

        // 改一个值 → 指纹变 → 待生成
        let mut d = Draft::default();
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
            Book::new(&f.presets, &c, &d).build_state("A1/STANDARD"),
            BuildState::Stale
        );
    }

    /// 生成记录丢了而磁盘上产物还在：说「待重新生成」，不说「未生成」。
    ///
    /// `built` 是"我们记的账"，磁盘是"真发生的事" —— 账丢了（快照回退 / 换机器 /
    /// 清过 store）而产物还在时，树上说「未生成」、预演却说「修改」，两个事实源
    /// 打架（作者真机踩过）。兜底：stat 一下交付根里的产物文件。
    #[test]
    fn build_state_falls_back_to_the_disk_when_the_record_is_gone() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();

        // 没记录、磁盘上也没有 → 未生成
        assert_eq!(
            Book::new(&f.presets, &c, &d).build_state("A1/STANDARD"),
            BuildState::NeverBuilt
        );

        // 磁盘上摆一份旧产物（与生成写盘同形：presets 根 + dist + mkp/presets）
        let mkp_file = Book::new(&f.presets, &c, &d)
            .version("A1/STANDARD")
            .expect("fixture 里有 A1/STANDARD")
            .mkp_file
            .clone();
        let target = f
            .presets
            .root()
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join(crate::workbench::paths::MKP_PRESETS_DIR)
            .join(&mkp_file);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        crate::fsx::atomic::atomic_write(&target, "# 以前生成的旧产物\n".as_bytes()).unwrap();

        assert_eq!(
            Book::new(&f.presets, &c, &d).build_state("A1/STANDARD"),
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
                Book::new(&f.presets, &c, &d)
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
        let b = Book::new(&f.presets, &c, &d);
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
                    serde_json::json!(-1.0)
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
        let b = Book::new(&f.presets, &c, &d);
        let mut moved = 0;
        for (i, u) in ["A1/STANDARD", "A1/FAST"].iter().enumerate() {
            if b.version_layers(u).unwrap().fingerprint() != before[i] {
                moved += 1;
            }
            let r = b.version_layers(u).unwrap();
            let hit = r.effective("wiping.child").unwrap();
            assert_eq!(*hit.value, serde_json::json!(44.0));
            assert_eq!(hit.origin, Origin::Machine, "值来自机型这一层");
        }
        assert_eq!(moved, 2, "两个版本都没写过它，所以两个都该跟着变");
    }

    /// 草稿里删键（挂回继承）**在保存之前就要看得出效果**
    #[test]
    fn a_pending_delete_shows_up_before_saving() {
        // 「已经写过的那一条」只能造在 registry 里 —— 值在这一层唯一的落盘处
        let (_dir, presets) =
            saved_values(&[("wiping.child", "A1:STANDARD", serde_json::json!(99))]);
        let c = committed();
        let mut d = Draft::default();

        let b = Book::new(&presets, &c, &d);
        let l = b.version_layers("A1/STANDARD").unwrap();
        assert_eq!(
            *l.effective("wiping.child").unwrap().value,
            serde_json::json!(99)
        );
        assert!(
            l.has_own(Level::Version, "wiping.child"),
            "这一层自己钉着它"
        );

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
        let b = Book::new(&presets, &c, &d);
        let l = b.version_layers("A1/STANDARD").unwrap();
        assert_eq!(
            *l.effective("wiping.child").unwrap().value,
            serde_json::json!(20.0)
        );
        assert_eq!(l.effective("wiping.child").unwrap().origin, Origin::Factory);
        assert!(
            !l.has_own(Level::Version, "wiping.child"),
            "自有那一项该没了"
        );
    }

    /* ---------- 矩阵 ---------- */

    /// **列序照配方本，不按勾选顺序**（tasks 7.5）。打乱输入，输出列序不变
    #[test]
    fn column_order_follows_the_recipe_book_not_the_click_order() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

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
            let m = b.matrix(&shuffled, None, "", None);
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
        let m = Book::new(&f.presets, &c, &d).matrix(&cols(&[("A1", None)]), None, "", None);
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
        let b = Book::new(&f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None), ("P1S", None)]), None, "", None);
        let keys: Vec<&str> = m.rows.iter().map(|r| r.key.as_str()).collect();
        assert!(
            keys.contains(&"toolhead.only_p1s"),
            "交集会让这一项凭空消失：{keys:?}"
        );

        // 而 A1 那一列上它是「不适用」，不是空白
        let row = m
            .rows
            .iter()
            .find(|r| r.key == "toolhead.only_p1s")
            .unwrap();
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
        let b = Book::new(&f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None), ("P1S", None)]), None, "", None);
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
        let b = Book::new(&f.presets, &c, &d);

        let m = b.matrix(&cols(&[("A1", None)]), None, "", None);
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
        let b = Book::new(&f.presets, &c, &d);

        let want = &cols(&[("A1", None), ("P1S", None)]);
        let a: Vec<String> = b
            .matrix(want, None, "", None)
            .rows
            .into_iter()
            .map(|r| r.key)
            .collect();
        let z: Vec<String> = b
            .matrix(want, None, "", None)
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
        let b = Book::new(&f.presets, &c, &d);

        // 干净状态下 wiping.mode 还是「擦料塔」，所以 wiping.child 是**可编辑**的
        let m = b.matrix(&cols(&[("A1", None)]), None, "", None);
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

        let b = Book::new(&f.presets, &c, &d);
        let m = b.matrix(&cols(&[("A1", None)]), None, "", None);
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
        let b = Book::new(&f.presets, &c, &d);

        let desk = b.desk("A1", Some("A1/STANDARD"), None, "");

        let labels: Vec<&str> = desk.groups.iter().map(|g| g.label.as_str()).collect();
        assert_eq!(labels, vec!["空间偏移", "擦料方式"]);

        for g in &desk.groups {
            let rows: usize = g.items.iter().map(|i| 1 + i.children.len()).sum();
            assert_eq!(g.count, rows, "组「{}」的计数与行数对不上", g.label);
        }

        let wipe = desk.groups.iter().find(|g| g.label == "擦料方式").unwrap();
        let mode = wipe
            .items
            .iter()
            .find(|i| i.row.key == "wiping.mode")
            .unwrap();
        assert_eq!(mode.children.len(), 1);
        assert_eq!(mode.children[0].key, "wiping.child");
        assert!(
            wipe.items.iter().all(|i| i.row.key != "wiping.child"),
            "子项不该同时又是顶层项"
        );

        // 列给全（C14）：基底 + 这一机型所有版本，每行一格一列；
        // `cur` 指认请求的那一层（A1/STANDARD），不是基底列
        assert_eq!(desk.cols.len(), 3, "A1 = 基底 + STANDARD + FAST");
        assert_eq!(desk.cols[0].version_uid, None);
        assert_eq!(desk.cur, 1, "请求的是 STANDARD 那一列");
        assert!(desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .all(|i| i.row.cells.len() == desk.cols.len() && i.row.impact.is_some()));
    }

    /// 弃用的参数在参数台上**看得见但改不动**（C14 §五）：行还在（不隐藏）、
    /// editable 关掉、reason 说的是「已弃用」那句话 —— 而不是「不适用」。
    /// 生成侧照旧不带它（见 `variants` 的测试）。
    #[test]
    fn deprecated_params_stay_visible_and_inert_in_the_desk() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

        let desk = b.desk("A1", None, None, "");
        let row = desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.row.key == "wiping.legacy")
            .expect("弃用的参数也要占一行 —— 藏起来会变成「明明有却找不到」");

        assert!(row.row.deprecated);
        let cell = &row.row.cells[desk.cur];
        assert!(!cell.editable, "弃用的格子改不动");
        assert!(
            cell.blocked.is_empty(),
            "弃用不是「条件不成立」，不许混进 blocked"
        );
        assert!(
            cell.reason
                .as_deref()
                .is_some_and(|r| r.contains("不再使用")),
            "要说清为什么改不动：{:?}",
            cell.reason
        );
        assert!(
            cell.blocked_hint.is_none(),
            "对弃用的行说「要 X 才可改」是假话"
        );

        // 对照：被条件关着的格子给的是**另一套**话
        let mut d2 = Draft::default();
        apply(
            &mut d2,
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
        let b2 = Book::new(&f.presets, &c, &d2);
        let desk2 = b2.desk("A1", None, None, "");
        let child = desk2
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .flat_map(|i| std::iter::once(&i.row).chain(i.children.iter()))
            .find(|r| r.key == "wiping.child")
            .unwrap();
        let cell = child.cells[desk2.cur].clone();
        assert!(
            cell.blocked_hint
                .as_deref()
                .is_some_and(|h| h.starts_with("要 ") && h.ends_with(" 才可改")),
            "被关着的行要给短提示：{:?}",
            cell.blocked_hint
        );
    }

    /// 「改了影响谁」（C14 抽屉的作用域栏）：改基底影响所有没自己钉的版本，
    /// 改版本只影响那一版 —— 其余没钉的列为「跟着基底走」。
    #[test]
    fn impact_tells_editing_a_base_apart_from_editing_a_version() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();

        // 基底视角：A1 的两个版本都没钉 toolhead.offset.x …… 不对，夹具的
        // machineVariants 里 A1:STANDARD / A1:FAST 都钉了这一项 —— 那就换一项：
        // toolhead.offset.z 只有 P1S:LITE 钉着，A1 两版都是继承
        let b = Book::new(&f.presets, &c, &d);
        let desk = b.desk("A1", None, None, "");
        let row = desk
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.row.key == "toolhead.offset.z")
            .unwrap();
        let impact = row.row.impact.as_ref().unwrap();
        assert_eq!(impact.targets.len(), 2, "改基底，两个没钉的版本都跟着变");
        assert!(impact.followers.is_empty());

        // 版本视角：targets 只有它自己；没钉的**别的**版本是「跟着基底走」
        let desk_v = b.desk("A1", Some("A1/STANDARD"), None, "");
        let row_v = desk_v
            .groups
            .iter()
            .flat_map(|g| g.items.iter())
            .find(|i| i.row.key == "toolhead.offset.z")
            .unwrap();
        let impact_v = row_v.row.impact.as_ref().unwrap();
        assert_eq!(impact_v.targets.len(), 1, "版本层只影响这一版");
        assert!(
            impact_v.targets[0].contains("标准版"),
            "点名要用人话：{}",
            impact_v.targets[0]
        );
        assert_eq!(impact_v.followers.len(), 1, "FAST 没钉它，还在跟着基底");
    }

    /// 对照矩阵的差异判据（C14 第四轮）：**基准写死基准机型的基底**。
    /// 基底列自己不是差异；与基底不同的格进 diff_keys（绿底 + 悬停句）；
    /// 基准机型没有的参数行进 not_own_keys（状态列「本机无此项」），
    /// 行序是基准机型先、别家多出来的接后面。
    #[test]
    fn the_matrix_diffs_against_the_base_machines_base() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

        let m = b.matrix(
            &cols(&[
                ("A1", None),
                ("A1", Some("A1/STANDARD")),
                ("P1S", Some("P1S/LITE")),
            ]),
            None,
            "",
            Some("A1"),
        );

        // ① 行序：A1 的参数排完，P1S 独有的 only_p1s 接在最后
        assert_eq!(
            m.rows.last().unwrap().key,
            "toolhead.only_p1s",
            "别家多出来的参数接后面（C14：行序跟着基准机型走）"
        );
        assert!(m.not_own_keys.contains(&"toolhead.only_p1s".to_owned()));

        // ② STANDARD 钉着 offset.x = -1，基底是出厂 0 → 差异行 + 差异格
        let row = m
            .rows
            .iter()
            .find(|r| r.key == "toolhead.offset.x")
            .unwrap();
        assert!(!row.cells[0].differs, "基准列自己不是差异");
        assert!(row.cells[1].differs, "STANDARD 的 -1 与基底不同");
        assert!(
            m.diff_keys.contains(&"toolhead.offset.x".to_owned()),
            "差异行进 diff_keys（「仅显示差异」与状态列读它）"
        );
        let tip = row.cells[1].diff_tip.as_deref().expect("差异格要有悬停句");
        assert!(
            tip.starts_with("机型基底是 ") && tip.ends_with("0 mm"),
            "悬停句说基准值：{tip}"
        );

        // ③ only_p1s 行：基准机型没有 → 没有差异可比，A1 的格子是「—」
        let row2 = m
            .rows
            .iter()
            .find(|r| r.key == "toolhead.only_p1s")
            .unwrap();
        assert!(!row2.cells.iter().any(|c| c.differs));
        assert_eq!(row2.cells[0].kind, CellKind::NotApplicable);

        // ④ 勾选列的值改回与基底一致后，差异消失（比的是值，不是出处）
        let mut d2 = Draft::default();
        apply(
            &mut d2,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "toolhead.offset.x".to_owned(),
                value: Some(serde_json::json!(0)),
            }],
        )
        .unwrap();
        let b2 = Book::new(&f.presets, &c, &d2);
        let m2 = b2.matrix(
            &cols(&[("A1", None), ("A1", Some("A1/STANDARD"))]),
            None,
            "",
            Some("A1"),
        );
        let row3 = m2
            .rows
            .iter()
            .find(|r| r.key == "toolhead.offset.x")
            .unwrap();
        assert!(
            !m2.diff_keys.contains(&"toolhead.offset.x".to_owned()),
            "值一样就不算差异（哪怕一个是出厂默认、一个是本版钉的 0）"
        );
        assert!(!row3.cells[1].differs);
    }

    /// 左栏导航**不随搜索变**：它是换分组看的工具，搜一个词就塌掉的话就没用了
    #[test]
    fn the_nav_counts_do_not_follow_the_search() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

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

        let b = Book::new(&f.presets, &c, &d);
        let desk = b.desk("A1", Some("A1/STANDARD"), None, "");
        let wipe = desk.groups.iter().find(|g| g.label == "擦料方式").unwrap();
        let mode = wipe
            .items
            .iter()
            .find(|i| i.row.key == "wiping.mode")
            .unwrap();

        let note = mode.off_note.as_deref().expect("关掉了就要说一句");
        assert!(note.contains("擦拭部件"), "要点名是谁关的：{note}");
        assert!(note.contains("圆盘擦拭"), "要说它现在是什么值：{note}");
        assert!(note.contains('1'), "要说关掉了几项：{note}");

        // 没关的时候不给这一句，否则界面上多一条空话
        let open =
            Book::new(&f.presets, &c, &Draft::default()).desk("A1", Some("A1/STANDARD"), None, "");
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
        let b = Book::new(&f.presets, &c, &d);

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

        let b = Book::new(&f.presets, &c, &d);
        let m = b.matrix(&cols(&[("A1", None)]), None, "", None);

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
        let b = Book::new(&f.presets, &c, &d);
        let one = cols(&[("A1", None)]);

        let only_wiping = b.matrix(&one, Some("wiping"), "", None);
        assert!(only_wiping
            .rows
            .iter()
            .all(|r| r.key.starts_with("wiping.")));
        assert!(only_wiping.note.is_none());

        // 搜一个只在 offset 分类里的词，但分类过滤停在 wiping
        let found = b.matrix(&one, Some("wiping"), "X 轴", None);
        assert!(
            found.rows.iter().any(|r| r.key == "toolhead.offset.x"),
            "搜索要跨全部分类，否则用户会以为这个字段不存在"
        );
        assert_eq!(found.note.as_deref(), Some(w::MATRIX_SEARCH_SPANS_ALL_TABS));

        // 搜 tomlKey 也要命中（三个偏移共享 tomlKey `offset`）
        assert!(b
            .matrix(&one, None, "offset", None)
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
        let b = Book::new(&f.presets, &c, &d);

        let no_cols = b.matrix(&[], None, "", None);
        assert_eq!(no_cols.empty_reason.as_deref(), Some(w::MATRIX_NO_COLS));

        let no_match = b.matrix(&cols(&[("A1", None)]), None, "根本没有这个词", None);
        assert_eq!(no_match.empty_reason.as_deref(), Some(w::MATRIX_NO_MATCH));
        assert!(no_match.total_rows > 0, "总行数要说过滤前的");
    }

    /// **BBS 与「进没进套餐」的正交面**（tasks 7.6）：A1 的 `defaultBundle`
    /// 里有那条 BBS，它的两个版本跟着；P1S 没有默认套餐 → 一条都没有
    #[test]
    fn a_machines_bbs_follows_its_default_bundle() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

        // 夹具里 A1 各版本自己就指着 A1_default（一版一套，P4 起）：来源是「本版本一份」
        assert_eq!(b.bbs_source("A1/STANDARD"), BbsSource::Own);
        assert_eq!(b.effective_bbs("A1/STANDARD"), vec!["a1-bbs-04-020"]);

        // P1S 没有 defaultBundle → 它的版本一条 BBS 都没有
        assert!(b.effective_bbs("P1S/LITE").is_empty());
    }

    /// **改一份套餐，指着它的版本全都跟着变**（一版一套，P4 起）。
    ///
    /// 夹具里 A1 两个版本的 `recommendedBundle` 都指 A1_default（与真数据同形状：
    /// 一个机型眼下就一套套餐），所以改套餐内容两个版本都跟 —— 各指各的套餐之后
    /// 就各跟各的了，那正是套餐页「改指向」要有的效果
    #[test]
    fn every_version_pointing_at_a_bundle_follows_it() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        let b = Book::new(&f.presets, &c, &d);

        for uid in ["A1/STANDARD", "A1/FAST"] {
            assert_eq!(
                b.effective_bbs(uid),
                vec!["a1-bbs-04-020"],
                "两个版本都指着 A1_default"
            );
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

        let b = Book::new(&f.presets, &c, &d);
        for uid in ["A1/STANDARD", "A1/FAST"] {
            assert_eq!(
                b.effective_bbs(uid),
                vec!["a1_bbs_04", "a1_bbs_06"],
                "{uid} 跟着套餐变"
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
        let rows = Book::new(&f.presets, &c, &d).build_rows();

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

        let a1 = rows.iter().find(|r| r.uid == "A1/STANDARD").unwrap();
        assert_eq!(a1.state, BuildState::NeverBuilt);
        assert!(a1.buildable);
        // 产物名由命名规则算出：`{机型}-{版本小写}.toml`
        assert_eq!(a1.mkp_file.as_deref(), Some("A1-standard.toml"));
    }

    /// 整本产物状态：**「暂无资源」的版本不参与** ——
    /// 算进去的话，只要仓库里有一台 A2L，整本永远是「未生成」
    #[test]
    fn the_overall_artifact_state_ignores_versions_with_no_resources() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();

        assert_eq!(
            Book::new(&f.presets, &c, &d).book_view().artifact,
            ArtifactState::Missing
        );

        // 把三个有产物的版本都记成已生成（记进**台账**，2026-10-06 状态机修正）
        let uids = ["A1/STANDARD", "A1/FAST", "P1S/LITE"];
        let fps: Vec<String> = {
            let b = Book::new(&f.presets, &c, &d);
            uids.iter()
                .map(|u| b.version_layers(u).unwrap().fingerprint())
                .collect()
        };
        let mut c = c;
        for (u, fp) in uids.iter().zip(fps) {
            c.built.insert(
                (*u).to_owned(),
                BuiltRecord {
                    stamp: "2026-01-01T00:00:00Z".to_owned(),
                    fingerprint: fp,
                },
            );
        }

        let view = Book::new(&f.presets, &c, &d).book_view();
        assert_eq!(
            view.artifact,
            ArtifactState::Fresh,
            "A2L 还是「暂无资源」，但它不该拖住整本"
        );
        assert_eq!(view.last_build.as_deref(), Some("2026-01-01T00:00:00Z"));
    }
}
