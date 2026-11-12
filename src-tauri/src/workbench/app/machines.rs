//! 「机型与版本」那一页的后端。**读写 `presets/machines/*.toml`。**
//!
//! # 为什么它不走 `Ctx` / `Committed` / `Draft`
//!
//! 那一套是**参数值**的（三层解析、草稿、撤销栈）。这一页管的是**清单** ——
//! 有哪些机型、每台有哪些版本、版本的六个元字段。两件事共用一套状态机没有好处：
//! 「新建一个机型」和「把 X 轴偏移改成 -1」在撤销语义上就不是一回事。
//!
//! 所以这一页自己读、自己写、自己一条一条落盘。代价是没有跨页撤销 ——
//! 换来的是这一页不会因为参数那边的状态而出错，也就是「页与页之间互不影响」那条。
//!
//! # 每次调用都重读一遍盘，这是刻意的
//!
//! 六个机型文件加起来 5 KB 出头，读一遍的代价可以忽略；
//! 而缓存的代价是「盘上改了但界面还显示旧的」—— 那种不一致查起来最费劲。
//! 这一页又不是每秒刷新的东西，所以**用简单换正确**。
//! （`param_registry.toml` 那 56 KB 不在这一页的读取面里。）

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::clock;
use crate::workbench::load_session_presets;
use crate::workbench::presets::{
    BrandField, Dimensions, MachineField, Presets, VersionField, Zone,
};

use super::with_struct_ctx;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandView {
    pub id: String,
    /// 显示名（对着人读的那个：`拓竹 (Bambu Lab)`）。空 = 没填过，界面回落显示 id
    pub name: String,
    /// 品牌图 = **资产 id**（不是文件名）。`None` = 没配 —— 消费侧回落内置字标
    pub logo: Option<String>,
    /// 这个品牌下的机型 id（反查）。**归属不是引用**：机型 `brand` 字段写着它
    pub machines: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    pub id: String,
    pub name: String,
    /// 各语言显示名（2026-10-31 多语言数据）。可空，客户端按语言挑、空则回落 `name`
    pub name_en: Option<String>,
    pub name_tw: Option<String>,
    pub name_ja: Option<String>,
    pub recommended_bundle: Option<String>,
    pub tag: Option<String>,
    /// 各语言角标。可空
    pub tag_en: Option<String>,
    pub tag_tw: Option<String>,
    pub tag_ja: Option<String>,
    pub description: Option<String>,
    /// 各语言说明。可空
    pub description_en: Option<String>,
    pub description_tw: Option<String>,
    pub description_ja: Option<String>,
    /// **备注**（客户端副标题）。可空
    pub remark: Option<String>,
    /// **这一版专属的外观图**（资产 id）。`None` = 回落机型图（`Machine::image`）——
    /// 界面上要说明白那是回落，不是"没配"
    pub image: Option<String>,
    /// **参数正文已补**（b05 Task 14.4 / doc §4.3 第 6 步）：这个版本在
    /// `param_registry.toml` 的 `machineVariants` 里有没有 `{机型}:{版本}` 形状的
    /// 显式键。`false` = 纯继承基底，界面上标「参数源待补」——
    /// **版本不因缺参数源而隐藏**（它的有效配方靠 defaults 兜底照样能渲染）
    pub has_recipe: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineView {
    pub id: String,
    /// 人看的名字。实测有机型的 `name` 是空串而 `display` 才是给人看的
    pub display: String,
    pub name: String,
    pub brand: String,
    pub default_bundle: Option<String>,
    pub external_aliases: Vec<String>,
    pub image: Option<String>,
    /// 第二个图位（快拆版外观图）。今天只有 A1 mini 有 —— 见 `presetdata::Machine::image_variant`
    pub image_variant: Option<String>,
    pub icon: Option<String>,
    /// **启用 / 停用**（2026-10-30）。停用的机型不进目录 ⇒ 客户端不显示、
    /// 后处理拒绝。这一页照常列出来（还能把它打开），只是打上「已停用」
    pub enabled: bool,
    /// 有没有 `[dimensions]` —— 界面上要能看出"这台还没配尺寸"
    pub has_dimensions: bool,
    /// **`[dimensions]` 的逐格视图**（2026-10-03 尺寸卡六组）。`None` = 这台没配尺寸
    /// （与 `has_dimensions` 同一件事，两处都在 `list_of` 里一次读出）
    pub dimensions: Option<Dimensions>,
    /// 禁区块数。0 = 这台没有禁区文件
    pub zone_count: usize,
    /// **禁区的原始点**（画布要用）。空数组 = 没有禁区文件 ——
    /// 与 `zone_count == 0` 同一件事（同一处算出，不会各说各话）
    pub zones: Vec<ZoneView>,
    pub versions: Vec<VersionView>,
    /// 它自己那个 toml 文件的名字（`A1.toml`）。**给人看的**，
    /// 让"我在改哪个文件"这件事不用猜
    pub file: String,
}

/// 一块禁区（画布上的一个多边形）。形状与 `presetdata::Zone` 一一对应。
///
/// **`Serialize` 与 `Deserialize` 都要**：出参（清单里的 `zones`）走前者，
/// 入参（`wb_set_machine_zones` 的 `zones`）走后者 —— 同一个形状两处都用。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZoneView {
    /// `[x, y]` 点对，机器坐标 mm（原点在床身前左角、y 向上）
    pub points: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineList {
    pub brands: Vec<BrandView>,
    pub machines: Vec<MachineView>,
    /// 数据根的绝对路径，显示在状态条上
    pub root: String,
}

/// 机型与版本清单。**这一页唯一的读入口**
#[tauri::command(async)]
pub fn wb_machines() -> Result<MachineList, AppError> {
    traced("wb_machines", |_| Ok(list_of(&load_session_presets()?)))
}

/// 加一个版本：**先落进草稿**（影子树），回一份新的清单。
///
/// # 为什么是一条结构编辑，而不是另建页面级草稿
///
/// 一条命令 = 一个对象的文件前后字节。新建只往 `machines/{机型}.toml` 插一个
/// `[[versions]]` 块，撤销就是把之前那份字节写回去 —— 与改机型字段走同一条路
/// （[`with_struct_ctx`]），不需要再复制一套状态机。保存（`wb_save`）才把影子树
/// 提交到真仓库，放弃就把整份拉回去。
///
/// 页头「未保存 N」由 `ctx.history` 数出来：新建没有字段账，按**一条**算。
/// 校验仍然要严（ID 字符集、重名），写入用原子写 —— 半个文件的 TOML 比没有更糟。
/// 写命令统一走 `Ctx`：**改的是 `Ctx` 里那份 `Presets`**（资产库 / 套餐页的读视图
/// 建在它上面），写完两边看到的就是同一个事实。以前这里各自 `load_presets()`
/// fresh 一份来改 —— 写是写对了，但 `Ctx` 里的机型清单还是启动时的旧指向，
/// 套餐页「被哪些版本指向」于是纹丝不动（作者截图点名「改了没反应」）。
/// 返回值仍从影子树重读：**界面看到的应该是写完的结果**。

#[tauri::command]
pub fn wb_add_version(
    machine_id: String,
    id: String,
    name: String,
) -> Result<MachineList, AppError> {
    traced("wb_add_version", |_| {
        with_struct_ctx(
            "",
            |_| vec![format!("machines/{machine_id}.toml")],
            |ctx| {
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .add_version(&id, &name)?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                tracing::info!(machine = %machine_id, version = %id, "加了一个版本");
                // 从盘上重读再返回：**界面看到的应该是落盘的结果**，不是内存里的样子。
                // 这两者不一致的话（写失败但界面显示成功），是最难查的一类
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 改一个版本的 id（**主键**）：写的是影子树（草稿），保存才进真仓库。
///
/// 与 [`wb_set_version_field`] 分开：那一条改的是这一版的某一格值，只动一个文件；
/// 这一条换主键，`A1:OLD` 那些键与归属资产要跟着搬，风险不同（与 `VersionField`
/// 的枚举注释同一条口径）。
///
/// # 「多个地方」都要跟着搬
///
/// 主键换了，指着它的东西一共五处，一条都不能漏：
///
/// | 在哪 | 怎么搬 |
/// |---|---|
/// | 机型文件 `[[versions]].id` | `Presets::rename_version_uid` |
/// | 参数四张表（值 / 范围 / 跟随） | 同上（表与内联表两种形状） |
/// | 资产归属 `versionId` | 同上 |
/// | **还没保存的值草稿与字段账** | `Ctx::retarget_version_uid` |
/// | 它独占的套餐 id（`{机型}_{版本 id}`） | `Presets::rename_version_bundle` |
///
/// 第四处是"先改了几格参数、再改 id"的那条路：只改前三处的话，那几格还挂在
/// 旧 uid 上，保存时写进一个不存在的版本。所以这里**不拒绝带草稿的改名**，
/// 而是把草稿一起搬过去（作者 2026-11-12）。
///
/// 第五处是 2026-11-12 补的：`FASTV3.3` 改成 `FAST260628` 之后版本行还显示
/// `A1_FASTV3.3` —— 套餐 id 没跟着版本走。只改**这一版独占**的套餐；
/// 多版共用的不碰（那会把别人的指向一起改掉）。
#[tauri::command]
pub fn wb_set_version_id(
    machine_id: String,
    old_version_id: String,
    new_version_id: String,
) -> Result<MachineList, AppError> {
    traced("wb_set_version_id", |_| {
        with_struct_ctx(
            "",
            |ctx| {
                /* 套餐改名会重指**所有**机型文件里指向它的那几处（`rename_bundle`），
                所以快照要覆盖全部机型文件 —— 只列当前这台会漏掉撤销点 */
                let mut files: Vec<String> = ctx
                    .presets
                    .catalog
                    .machines()
                    .iter()
                    .map(|m| format!("machines/{}.toml", m.id))
                    .collect();
                files.push("registry/param_registry.toml".to_owned());
                files.push("assets.toml".to_owned());
                files.push("bundles.toml".to_owned());
                files
            },
            |ctx| {
                let old_vid = old_version_id.trim();
                let new_vid = new_version_id.trim();

                let moved = ctx
                    .presets
                    .rename_version_uid(&machine_id, old_vid, new_vid)?;
                ctx.presets.registry.write_back()?;
                ctx.presets.assets.write()?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                let bundle_moved = ctx.presets.rename_version_bundle(
                    &machine_id,
                    old_vid,
                    new_vid,
                    &clock::now_iso8601(),
                )?;
                /* 草稿里那些格子还挂在旧 uid 上 —— 一起搬到新 uid，
                保存时才写得回去。比这一笔更早的文件字节不动：撤销是后进先出，
                这一条会先被撤掉，id 退回旧值之后那些字节又是对的 */
                ctx.retarget_version_uid(&machine_id, old_vid, new_vid);

                tracing::info!(
                    machine = %machine_id,
                    from = %old_vid,
                    to = %new_vid,
                    params = moved.params.len(),
                    assets = moved.assets.len(),
                    bundle = bundle_moved.as_ref().map(|(a, b)| format!("{a}→{b}")).unwrap_or_default(),
                    "改了一个版本的 id（参数记录、资产归属、独占套餐与草稿一起搬）"
                );
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// **复制已有版本**（b05 Task 14.3 / doc §4.3 第 2–5 步）：选一个模板版本，
/// 填新 id / 名称，落盘版本定义 + 它指向的套餐 —— 不碰参数正文
/// （那是 [`super::wb_copy_recipe`] 的独立动作）。
///
/// 除 id 与名称外各格**逐格照抄模板**（三册译名、角标、说明、备注、版图）；
/// `tag` / `description` 给了就是改写那两格。
///
/// # 套餐为什么跟着走（2026-11-12）
///
/// 作者把 `FASTV3.3` 改名成 `FAST260628`、又复制出 `FAST261111` 之后，两行版本
/// 都显示 `A1_FASTV3.3`：复制把指针照抄了，而「一版一套」要求每版有自己的套餐。
/// 现在复制版本时一并建 `{机型}_{新版本 id}` 的套餐（内容拷自模板，MKP 登记换成
/// 新版本自己的那份），新版本指过去；模板那份不动。
#[tauri::command]
pub fn wb_copy_version(
    machine_id: String,
    template_version_id: String,
    id: String,
    name: String,
    tag: Option<String>,
    description: Option<String>,
) -> Result<MachineList, AppError> {
    traced("wb_copy_version", |_| {
        with_struct_ctx(
            "",
            |_| {
                vec![
                    format!("machines/{machine_id}.toml"),
                    "bundles.toml".to_owned(),
                    "assets.toml".to_owned(),
                ]
            },
            |ctx| {
                let copied = ctx.presets.copy_version_with_bundle(
                    &machine_id,
                    &template_version_id,
                    &id,
                    &name,
                    tag.as_deref(),
                    description.as_deref(),
                    &clock::now_iso8601(),
                )?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                if copied.is_some() {
                    ctx.presets.bundles.write()?;
                    ctx.presets.assets.write()?;
                }
                tracing::info!(
                    machine = %machine_id,
                    template = %template_version_id,
                    version = %id,
                    bundle = copied.as_ref().map(|c| c.bundle_id.as_str()).unwrap_or_default(),
                    asset = copied.as_ref().and_then(|c| c.asset_id.as_deref()).unwrap_or_default(),
                    "从模板复制了一个版本（定义 + 套餐；参数正文是下一条命令）"
                );
                // 从盘上重读再返回：**界面看到的应该是落盘的结果**
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 加一台机型 = **新建一个 `presets/machines/{ID}.toml`**，立刻落盘。
///
/// 风险与加版本不同：那个最坏是排版被搅乱（数据还在），
/// 这个最坏是**覆盖掉一台已存在的机型**。所以底下用 `create_new` 原子地占路径，
/// 已存在一律拒绝 —— 详见 `Catalog::add_machine` 的注释
#[tauri::command]
pub fn wb_add_machine(id: String, brand: String, display: String) -> Result<MachineList, AppError> {
    traced("wb_add_machine", |_| {
        with_struct_ctx(
            "",
            |_| vec![format!("machines/{id}.toml")],
            |ctx| {
                ctx.presets.catalog.add_machine(&id, &brand, &display)?;
                tracing::info!(machine = %id, "建了一台机型");
                // 重读盘再返回：**界面看到的是落盘的结果**。
                // 这一条尤其重要 —— 新建文件比改文件更容易出现"内存里成了、盘上没成"
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 删这个版本会让哪些字段留下孤儿引用。**删之前先问这一条。**
///
/// 它是删除独有的风险：`[params.machineVariants]` 里 `A1:FAST` 这样的键
/// 在版本删掉之后会指向一个不存在的对象，而那种损坏**不报错** ——
/// 解析照样通过，只是那一项在那台机器上悄悄不生效了。
/// 所以要在删之前摆给人看，而不是删完让他自己发现
#[tauri::command(async)]
pub fn wb_version_orphans(machine_id: String, version_id: String) -> Result<Vec<String>, AppError> {
    traced("wb_version_orphans", |_| {
        Ok(load_session_presets()?.orphans_if_version_removed(&machine_id, &version_id))
    })
}

/// 一条归属某个版本的资产，连同**谁在引用它**。
///
/// 有引用的那几条删版本时会被拒（`remove_assets_of_version` 与删资产同一道闸），
/// 所以这件事必须在确认框里先摆出来 —— 让人点完才看到报错是最差的顺序。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionAssetView {
    pub id: String,
    pub machines: Vec<String>,
    pub bundles: Vec<String>,
}

/// 这一版还挂着哪些资产 —— **删版本会连带删掉它们**。确认框点之前就要摆出来
#[tauri::command(async)]
pub fn wb_version_assets(
    machine_id: String,
    version_id: String,
) -> Result<Vec<VersionAssetView>, AppError> {
    traced("wb_version_assets", |_| {
        let presets = load_session_presets()?;
        let mut out = Vec::new();
        for id in presets.assets_of_version(&machine_id, &version_id) {
            let usage = presets.asset_usage(&id)?;
            out.push(VersionAssetView {
                id,
                machines: usage.machines,
                bundles: usage.bundles,
            });
        }
        Ok(out)
    })
}

/// 删一个版本：**先进草稿**（撤销能整条退回来），保存才提交到真仓库。
///
/// 连带清掉的东西（归属资产、套餐里指向它的引用、参数四张表的键）必须在确认框里
/// 先摆出来 —— 界面上是两步确认，因为这一步确实会让东西消失，只是能撤销。
#[tauri::command]
pub fn wb_remove_version(machine_id: String, version_id: String) -> Result<MachineList, AppError> {
    traced("wb_remove_version", |_| {
        with_struct_ctx(
            "",
            |_| {
                vec![
                    format!("machines/{machine_id}.toml"),
                    "registry/param_registry.toml".to_owned(),
                    "bundles.toml".to_owned(),
                    "assets.toml".to_owned(),
                ]
            },
            |ctx| {
                // 先记下来再删 —— 删完就查不出它被谁引用了
                let orphans = ctx
                    .presets
                    .orphans_if_version_removed(&machine_id, &version_id);
                /* ★ 归属这一版的资产记录必须跟着走：版本没了它们就指向一个不存在的
                对象，下一次启动 `check_cross_consistency` 会把整份数据判成
                Corrupted（整个工作台起不来）。它已被套餐引用的那几条会先**从套餐里
                摘掉**，否则套餐里那条 `assetRef` 就成了指向空气的引用。 */
                let assets = ctx.presets.remove_assets_of_version(
                    &machine_id,
                    &version_id,
                    &crate::workbench::clock::now_iso8601(),
                )?;
                /* 同一条理由：`A1:FAST` 在 machineVariants / follows 里留着的键
                也是指向空气。**这一步不删的话删版本永远成功不了** —— 上面那条资产
                检查过了，下一次 load 会栽在这些孤儿键上。 */
                let overrides = ctx
                    .presets
                    .drop_version_overrides(&machine_id, &version_id)?;
                if !overrides.is_empty() {
                    ctx.presets.write_back()?;
                }
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .remove_version(&version_id)?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                // 连带清掉的东西**要留痕**，不能只在界面上闪一下
                tracing::info!(
                    machine = %machine_id,
                    version = %version_id,
                    assets = assets.removed.len(),
                    detached = assets.detached.len(),
                    bundles = %assets
                        .detached
                        .iter()
                        .map(|(b, _)| b.as_str())
                        .collect::<Vec<_>>()
                        .join(","),
                    overrides = overrides.len(),
                    keys = %orphans.join(","),
                    "删了一个版本（归属资产、套餐引用与参数覆盖一起清了）"
                );
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 改版本的一格。`value` 为 `None`/空 = **清空**，而那在文件里是**删掉那一行**，
/// 不是写 `tag = ''`（后者读成"填过，填了个空"）。
///
/// `name` 不许清空 —— 空了之后版本卡上只剩一个 ID，人就认不出它是什么了
#[tauri::command]
pub fn wb_set_version_field(
    machine_id: String,
    version_id: String,
    field: VersionField,
    value: Option<String>,
) -> Result<MachineList, AppError> {
    traced("wb_set_version_field", |_| {
        with_struct_ctx(
            field.key(),
            |_| vec![format!("machines/{machine_id}.toml")],
            |ctx| {
                /* 改之前那一格要抄下来 —— 「改了什么」显示 `tag: → 推荐` 这种真实前后值 */
                let before = ctx
                    .presets
                    .catalog
                    .machine(&machine_id)
                    .and_then(|m| m.versions.iter().find(|v| v.id == version_id))
                    .and_then(|v| version_field_text(v, field));
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .set_version_field(&version_id, field, value.as_deref())?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                ctx.reload_presets()?;
                let after = ctx
                    .presets
                    .catalog
                    .machine(&machine_id)
                    .and_then(|m| m.versions.iter().find(|v| v.id == version_id))
                    .and_then(|v| version_field_text(v, field));
                ctx.pending_fields = vec![super::one_field(
                    format!("{machine_id}:{version_id}"),
                    field.key(),
                    version_name_of(ctx, &machine_id, &version_id),
                    before,
                    after,
                )];
                Ok(list_of(&ctx.presets))
            },
        )
    })
}

/// 改机型自己的一格。`display` / `brand` 不许清空
#[tauri::command]
pub fn wb_set_machine_field(
    machine_id: String,
    field: MachineField,
    value: Option<String>,
) -> Result<MachineList, AppError> {
    traced("wb_set_machine_field", |_| {
        with_struct_ctx(
            field.key(),
            |_| vec![format!("machines/{machine_id}.toml")],
            |ctx| {
                let before = ctx
                    .presets
                    .catalog
                    .machine(&machine_id)
                    .and_then(|m| machine_field_text(m, field));
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .set_field(field, value.as_deref())?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                ctx.reload_presets()?;
                let after = ctx
                    .presets
                    .catalog
                    .machine(&machine_id)
                    .and_then(|m| machine_field_text(m, field));
                ctx.pending_fields = vec![super::one_field(
                    machine_id.clone(),
                    field.key(),
                    machine_id.clone(),
                    before,
                    after,
                )];
                Ok(list_of(&ctx.presets))
            },
        )
    })
}

/// 改品牌的一格（`name` / `logo`）。**即时落盘**（同机型那一套：没有草稿、没有撤销）。
///
/// 品牌图是**资产 id** —— 与机型的图 / 图标同一条口径（挑选走资产选择器，不手填路径）；
/// 清空 = 删键（消费侧回落内置字标，那是兜底不是常态）。
///
/// 2026-10-03（作者：「品牌也要像机型一样能编辑，不管客户端消不消费都提供」）。
#[tauri::command]
pub fn wb_set_brand_field(
    brand_id: String,
    field: BrandField,
    value: Option<String>,
) -> Result<MachineList, AppError> {
    traced("wb_set_brand_field", |_| {
        with_struct_ctx(
            field.key(),
            |_| vec!["brands.toml".to_owned()],
            |ctx| {
                ctx.presets
                    .catalog
                    .set_brand_field(&brand_id, field, value.as_deref())?;
                ctx.presets.catalog.write_brands()?;
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// **把一台机型挪到另一个品牌下**（2026-10-03，作者：「把某一个机型移到其他品牌下，
/// 就是那种正常的移动」）。
///
/// 落点极窄：**只改机型文件的 `brand` 一格**（复用 [`wb_set_machine_field`] 那条路，
/// 值面 + 文档面一起改）。品牌侧的 `machines` 列表是**反查**，不落盘 ——
/// 所以不存在"两份归属"要同步的问题。
///
/// 多出来的一件事是**校验目标品牌真的存在**：手动改一格时前端给的是下拉里的选项，
/// 而移动是一次显式动作，打错一个字就会在盘上留下一个悬空的 `brand` 值
/// （界面上那台机器会从所有分组里消失，因为没有一个品牌认领它）。
#[tauri::command]
pub fn wb_move_machine_to_brand(
    machine_id: String,
    brand_id: String,
) -> Result<MachineList, AppError> {
    traced("wb_move_machine_to_brand", |_| {
        with_struct_ctx(
            "brand",
            |_| vec![format!("machines/{machine_id}.toml")],
            |ctx| {
                let target = brand_id.trim();
                let brand = ctx
                    .presets
                    .catalog
                    .brand(target)
                    .ok_or_else(|| AppError::not_found(format!("没有品牌 {target}")))?;
                // 写进文件的是品牌**自己的 id 写法**（大小写可能与传进来的不同）
                let canonical = brand.id.clone();
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .set_field(MachineField::Brand, Some(&canonical))?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 机型某一格现在是什么（给人看的字符串）。`None` = 这一格空着
fn machine_field_text(m: &crate::presetdata::catalog::Machine, f: MachineField) -> Option<String> {
    match f {
        MachineField::Display => Some(m.display.clone()).filter(|s| !s.is_empty()),
        MachineField::Name => Some(m.name.clone()).filter(|s| !s.is_empty()),
        MachineField::Brand => Some(m.brand.clone()).filter(|s| !s.is_empty()),
        MachineField::Image => m.image.clone(),
        MachineField::ImageVariant => m.image_variant.clone(),
        MachineField::Icon => m.icon.clone(),
        MachineField::Enabled => Some(if m.enabled { "启用" } else { "停用" }.to_owned()),
    }
}

/// 版本某一格现在是什么。`None` = 这一格空着
///
/// **每个 `VersionField` 都要有一条胳膊**（不许 `_ => ""`）：漏了的那几格
/// （标签 / 说明 / 备注 / 版图）会算出 `None → None`，于是「改了它」在字段账上
/// 是"没变"—— 页头写着「未保存 0」而文件确实改了（作者 2026-11-11 报的）。
fn version_field_text(
    v: &crate::presetdata::catalog::MachineVersion,
    f: VersionField,
) -> Option<String> {
    let text = match f {
        VersionField::Name => v.name.clone(),
        VersionField::NameEn => v.name_en.clone().unwrap_or_default(),
        VersionField::NameTw => v.name_tw.clone().unwrap_or_default(),
        VersionField::NameJa => v.name_ja.clone().unwrap_or_default(),
        VersionField::RecommendedBundle => v.recommended_bundle.clone().unwrap_or_default(),
        VersionField::Tag => v.tag.clone().unwrap_or_default(),
        VersionField::TagEn => v.tag_en.clone().unwrap_or_default(),
        VersionField::TagTw => v.tag_tw.clone().unwrap_or_default(),
        VersionField::TagJa => v.tag_ja.clone().unwrap_or_default(),
        VersionField::Description => v.description.clone().unwrap_or_default(),
        VersionField::DescriptionEn => v.description_en.clone().unwrap_or_default(),
        VersionField::DescriptionTw => v.description_tw.clone().unwrap_or_default(),
        VersionField::DescriptionJa => v.description_ja.clone().unwrap_or_default(),
        VersionField::Remark => v.remark.clone().unwrap_or_default(),
        VersionField::Image => v.image.clone().unwrap_or_default(),
    };
    Some(text).filter(|s| !s.is_empty())
}

/// 版本的使用名（给字段账的 `label` 用）
fn version_name_of(ctx: &super::Ctx, machine_id: &str, version_id: &str) -> String {
    ctx.presets
        .catalog
        .machine(machine_id)
        .and_then(|m| m.versions.iter().find(|v| v.id == version_id))
        .map(|v| v.name.clone())
        .unwrap_or_else(|| version_id.to_owned())
}

/// **写一台机型的整张 `[dimensions]`**（六组：床身 / 移动范围 / 边缘 / 涂胶 / 标定点 / 标志位）。
///
/// 写命令（不带 `async`，与这一页其余几条一致）：落盘 + 回一份新清单。
/// 校验在数据层（[`Dimensions::check_finite`]）：床身宽深必须为正、其余数字必须有限
/// —— `NaN` / 无穷大写进 TOML 之后**每一次读都失败**，现场却已经没了。
///
/// 全零的可选组由 `set_dimensions` 剔除（口径写死在后端一处，前端那颗只是为了少发几个字节）。
#[tauri::command]
pub fn wb_set_machine_dimensions(
    machine_id: String,
    dimensions: Dimensions,
) -> Result<MachineList, AppError> {
    traced("wb_set_machine_dimensions", |_| {
        with_struct_ctx(
            "dimensions",
            |_| vec![format!("machines/{machine_id}.toml")],
            |ctx| {
                /* ★ **改之前的尺寸要抄下来**（2026-11-03）：`set_dimensions` 是整张替换，
                写完就再也拿不到旧的那一组。而「改了什么」要显示
                `bedSize.depth: 256 → 250` 这种真实前后值。 */
                let before = ctx
                    .presets
                    .catalog
                    .machine(&machine_id)
                    .and_then(|m| m.dimensions.clone());
                ctx.presets
                    .catalog
                    .machine_mut(&machine_id)?
                    .set_dimensions(dimensions)?;
                ctx.presets.catalog.write_machine(&machine_id)?;
                ctx.reload_presets()?;
                if let Some(old) = before {
                    if let Some(now) = ctx
                        .presets
                        .catalog
                        .machine(&machine_id)
                        .and_then(|m| m.dimensions.clone())
                    {
                        /* `summary(&old)` 拿**现在这份**去比 `old`，返回的是
                        `bedSize.depth: 256 → 250` 这样的字段差 */
                        let fields = now.summary(&old);
                        ctx.pending_fields =
                            super::structural_fields(&machine_id, machine_id.clone(), fields);
                    }
                }
                Ok(list_of(&ctx.presets))
            },
        )
    })
}

/// **写一台机型的禁区**。空数组 = 删掉 `forbidden_zones/<id>.toml`（清空是删文件，
/// 不是留一个空文件 —— 见 `presetdata::Catalog::set_zones` 的理由）。
///
/// 三道校验都在这里（数据层只管落盘）：每块至少 3 点（少于 3 点围不出面）、
/// 坐标必须有限、块数上限 32（防手滑把整屏点都塞进来 —— 这个数字是"人画得出来的
/// 禁区数量"的上界，不是技术限制）。
#[tauri::command]
pub fn wb_set_machine_zones(
    machine_id: String,
    zones: Vec<ZoneView>,
) -> Result<MachineList, AppError> {
    traced("wb_set_machine_zones", |_| {
        const MIN_POINTS: usize = 3;
        const MAX_ZONES: usize = 32;
        if zones.len() > MAX_ZONES {
            return Err(AppError::invalid_argument(format!(
                "一块机型的禁区最多 {MAX_ZONES} 块（收到 {}）",
                zones.len()
            )));
        }
        let mut out: Vec<Zone> = Vec::with_capacity(zones.len());
        for (i, z) in zones.iter().enumerate() {
            if z.points.len() < MIN_POINTS {
                return Err(AppError::invalid_argument(format!(
                    "第 {} 块禁区只有 {} 个点 —— 少于 {MIN_POINTS} 个围不出面",
                    i + 1,
                    z.points.len()
                )));
            }
            for (x, y) in &z.points {
                if !x.is_finite() || !y.is_finite() {
                    return Err(AppError::invalid_argument(format!(
                        "第 {} 块禁区里有非有限坐标",
                        i + 1
                    )));
                }
            }
            out.push(Zone {
                points: z.points.clone(),
            });
        }
        with_struct_ctx(
            "zones",
            |_| vec![format!("forbidden_zones/{machine_id}.toml")],
            |ctx| {
                ctx.presets.catalog.set_zones(&machine_id, out)?;
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

/// 新建一个品牌（id + 显示名；品牌图后配）。**即时落盘**，回一份新的清单。
///
/// 与「新增机型」不同：品牌全住一个 `brands.toml`，这里是往 `[[brands]]` 里加一段 ——
/// 不新建文件、不覆盖任何东西（那个「create_new 占路径」的风险在这里不存在）。
#[tauri::command]
pub fn wb_add_brand(id: String, name: String) -> Result<MachineList, AppError> {
    traced("wb_add_brand", |_| {
        with_struct_ctx(
            "",
            |_| vec!["brands.toml".to_owned()],
            |ctx| {
                ctx.presets.catalog.add_brand(&id, &name)?;
                ctx.presets.catalog.write_brands()?;
                Ok(list_of(&ctx.presets.reload()?))
            },
        )
    })
}

fn list_of(p: &Presets) -> MachineList {
    MachineList {
        brands: p
            .catalog
            .brands()
            .iter()
            .map(|b| BrandView {
                id: b.id.clone(),
                name: b.name.clone(),
                logo: b.logo.clone(),
                // 「哪些机型是这个品牌的」—— 反查在后端一处算（前端不复判任何一条关系）
                machines: p
                    .catalog
                    .machines()
                    .iter()
                    .filter(|m| m.brand.eq_ignore_ascii_case(&b.id))
                    .map(|m| m.id.clone())
                    .collect(),
            })
            .collect(),
        machines: p
            .catalog
            .machines()
            .iter()
            .map(|m| MachineView {
                id: m.id.clone(),
                display: m.display.clone(),
                name: m.name.clone(),
                brand: m.brand.clone(),
                default_bundle: m.default_bundle.clone(),
                external_aliases: m.external_aliases.clone(),
                image: m.image.clone(),
                image_variant: m.image_variant.clone(),
                icon: m.icon.clone(),
                enabled: m.enabled,
                has_dimensions: m.has_dimensions,
                dimensions: m.dimensions.clone(),
                // 禁区两格**从同一个来源算出**（一处取，一处 len）——
                // 分开算迟早会漂成「说有 2 块但画不出来」
                zone_count: p.catalog.zones(&m.id).map_or(0, <[_]>::len),
                zones: p
                    .catalog
                    .zones(&m.id)
                    .unwrap_or_default()
                    .iter()
                    .map(|z| ZoneView {
                        points: z.points.clone(),
                    })
                    .collect(),
                versions: m
                    .versions
                    .iter()
                    .map(|v| {
                        let uid = format!("{}:{}", m.id, v.id);
                        VersionView {
                            id: v.id.clone(),
                            name: v.name.clone(),
                            name_en: v.name_en.clone(),
                            name_tw: v.name_tw.clone(),
                            name_ja: v.name_ja.clone(),
                            recommended_bundle: v.recommended_bundle.clone(),
                            tag: v.tag.clone(),
                            tag_en: v.tag_en.clone(),
                            tag_tw: v.tag_tw.clone(),
                            tag_ja: v.tag_ja.clone(),
                            description: v.description.clone(),
                            description_en: v.description_en.clone(),
                            description_tw: v.description_tw.clone(),
                            description_ja: v.description_ja.clone(),
                            remark: v.remark.clone(),
                            image: v.image.clone(),
                            has_recipe: p.registry.version_has_variants(&uid),
                        }
                    })
                    .collect(),
                file: m
                    .file()
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            })
            .collect(),
        root: p.root().display().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DTO 能从真数据拼出来，而且**关键字段不是空的**。
    ///
    /// 只断言"没报错"的话，一个全是空串的列表也会过
    #[test]
    fn the_machine_list_carries_real_values() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = Presets::load_from(&root).expect("读得通");
        let a1 = p.catalog.machine("A1").expect("A1 在");

        assert_eq!(a1.versions.len(), 3);
        assert_eq!(
            a1.file().file_name().unwrap().to_string_lossy(),
            "A1.toml",
            "界面上要显示「我在改哪个文件」"
        );
        // A1 有尺寸、无禁区；P1S 有禁区 —— 几个状态各自独立，界面要分别显示
        assert!(a1.has_dimensions);
        assert_eq!(p.catalog.zones("A1").map_or(0, <[_]>::len), 0);
        // P1S 有禁区
        assert!(p.catalog.zones("P1S").map_or(0, <[_]>::len) > 0);

        // 品牌：真数据里只有一家，五台机型全归它（「哪些机型是这个品牌的」在后端算）
        let list = list_of(&p);
        let b = list
            .brands
            .iter()
            .find(|b| b.id == "Bambu Lab")
            .expect("Bambu Lab 在清单里");
        assert_eq!(b.name, "拓竹 (Bambu Lab)", "显示名是给人读的那个");
        assert_eq!(
            b.machines.len(),
            5,
            "五台机型都归这个品牌 —— 条数变了就说清为什么：{:?}",
            b.machines
        );
        assert!(b.machines.contains(&"A1".to_owned()));
        assert_eq!(
            b.logo.as_deref(),
            Some("bambu-lab-logo"),
            "品牌图 = 资产 id"
        );
    }

    /// **版本每一格都要说得出"现在是什么"**。
    ///
    /// `None` 只该出现在**真的空着**的那几格。漏一条胳膊（早先是 `_ => ""`）时，
    /// 标签 / 说明 / 备注 / 版图的改前与改后都是 `None` ⇒ 字段账上"没变" ⇒
    /// 页头写着「未保存 0」而文件确实改了（作者 2026-11-11 报的那一幕）。
    #[test]
    fn every_version_field_knows_its_own_text() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = Presets::load_from(&root).expect("读得通");
        let v = p
            .catalog
            .machine("A1")
            .and_then(|m| m.versions.iter().find(|v| v.id == "FASTV3.3"))
            .expect("夹具前提：A1 有 FASTV3.3");

        /* 夹具前提：这一版这十三格都填着东西 */
        for f in [
            VersionField::Name,
            VersionField::NameEn,
            VersionField::NameTw,
            VersionField::NameJa,
            VersionField::RecommendedBundle,
            VersionField::Tag,
            VersionField::TagEn,
            VersionField::TagTw,
            VersionField::TagJa,
            VersionField::Description,
            VersionField::DescriptionEn,
            VersionField::DescriptionTw,
            VersionField::DescriptionJa,
        ] {
            assert!(
                version_field_text(v, f).is_some(),
                "{f:?} 这一格填着东西却读成 None —— 改它会算成没改"
            );
        }

        /* 这两格这一版确实没填 —— None 是对的 */
        assert_eq!(version_field_text(v, VersionField::Remark), None);
        assert_eq!(version_field_text(v, VersionField::Image), None);
    }
}
