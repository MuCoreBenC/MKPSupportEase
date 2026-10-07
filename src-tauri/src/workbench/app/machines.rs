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
use crate::workbench::load_presets;
use crate::workbench::presets::{
    BrandField, Dimensions, MachineField, Presets, VersionField, Zone,
};

use super::with_ctx_mut;

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
    pub recommended_bundle: Option<String>,
    pub tag: Option<String>,
    pub description: Option<String>,
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
    traced("wb_machines", |_| Ok(list_of(&load_presets()?)))
}

/// 加一个版本，**立刻落盘**，回一份新的清单。
///
/// # 为什么不经草稿
///
/// 参数值那套有草稿 + 撤销栈，因为改一个数是高频动作、而且常常要连改一片再一起看。
/// 「加一个版本」不是那种动作：它低频、而且**逆操作很直接**（删掉那个版本）。
/// 为它建一套页面级草稿会把参数页那一整套状态机复制一遍，换来的只是一个
/// 「还没保存」的中间态 —— 而那个中间态本身就是新的一类 bug 来源。
///
/// 代价写明白：**没有撤销**。所以校验要严（ID 字符集、重名），
/// 而写入用原子写（同目录临时文件 + rename），半个文件的 TOML 比没有更糟。
/// 写命令统一走 `Ctx`：**改的是 `Ctx` 里那份 `Presets`**（资产库 / 套餐页的读视图
/// 建在它上面），落盘之后两边看到的就是同一个事实。以前这里各自 `load_presets()`
/// fresh 一份来改 —— 落盘是对的，但 `Ctx` 里的机型清单还是启动时的旧指向，
/// 套餐页「被哪些版本指向」于是纹丝不动（作者截图点名「改了没反应」）。
/// 返回值仍从盘上重读：**界面看到的应该是落盘的结果**。

#[tauri::command]
pub fn wb_add_version(
    machine_id: String,
    id: String,
    name: String,
) -> Result<MachineList, AppError> {
    traced("wb_add_version", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets
                .catalog
                .machine_mut(&machine_id)?
                .add_version(&id, &name)?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            tracing::info!(machine = %machine_id, version = %id, "加了一个版本");
            // 从盘上重读再返回：**界面看到的应该是落盘的结果**，不是内存里的样子。
            // 这两者不一致的话（写失败但界面显示成功），是最难查的一类
            Ok(list_of(&load_presets()?))
        })
    })
}

/// **复制已有版本**（b05 Task 14.3 / doc §4.3 第 2–5 步）：选一个模板版本，
/// 填新 id / 名称 / tag / 描述，保存时**只写版本定义** —— 不碰参数正文
/// （那是 [`super::wb_copy_recipe`] 的独立动作，两步分离让每次写只落一个文件）。
///
/// `tag` / `description` 由前端拿模板值预填、人可改；`recommendedBundle` 抄模板。
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
        with_ctx_mut(|ctx| {
            ctx.presets.catalog.machine_mut(&machine_id)?.copy_version(
                &template_version_id,
                &id,
                &name,
                tag.as_deref(),
                description.as_deref(),
            )?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            tracing::info!(
                machine = %machine_id,
                template = %template_version_id,
                version = %id,
                "从模板复制了一个版本（只写版本定义）"
            );
            // 从盘上重读再返回：**界面看到的应该是落盘的结果**
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
            ctx.presets.catalog.add_machine(&id, &brand, &display)?;
            tracing::info!(machine = %id, "建了一台机型");
            // 重读盘再返回：**界面看到的是落盘的结果**。
            // 这一条尤其重要 —— 新建文件比改文件更容易出现"内存里成了、盘上没成"
            Ok(list_of(&load_presets()?))
        })
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
        Ok(load_presets()?.orphans_if_version_removed(&machine_id, &version_id))
    })
}

/// 删一个版本，立刻落盘。**不可逆**（没有回收站也没有撤销），
/// 所以界面那边是两步确认，而且确认框里要列出上面那条查出来的孤儿
#[tauri::command]
pub fn wb_remove_version(machine_id: String, version_id: String) -> Result<MachineList, AppError> {
    traced("wb_remove_version", |_| {
        with_ctx_mut(|ctx| {
            // 先记下来再删 —— 删完就查不出它被谁引用了
            let orphans = ctx
                .presets
                .orphans_if_version_removed(&machine_id, &version_id);
            ctx.presets
                .catalog
                .machine_mut(&machine_id)?
                .remove_version(&version_id)?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            if orphans.is_empty() {
                tracing::info!(machine = %machine_id, version = %version_id, "删了一个版本");
            } else {
                // 留下孤儿是**要留痕**的事，不能只在界面上闪一下
                tracing::warn!(
                    machine = %machine_id,
                    version = %version_id,
                    orphans = orphans.len(),
                    keys = %orphans.join(","),
                    "删了一个版本，留下了孤儿引用"
                );
            }
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
            ctx.presets
                .catalog
                .machine_mut(&machine_id)?
                .set_version_field(&version_id, field, value.as_deref())?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
            ctx.presets
                .catalog
                .machine_mut(&machine_id)?
                .set_field(field, value.as_deref())?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
            ctx.presets
                .catalog
                .set_brand_field(&brand_id, field, value.as_deref())?;
            ctx.presets.catalog.write_brands()?;
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
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
            Ok(list_of(&load_presets()?))
        })
    })
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
        with_ctx_mut(|ctx| {
            ctx.presets
                .catalog
                .machine_mut(&machine_id)?
                .set_dimensions(dimensions)?;
            ctx.presets.catalog.write_machine(&machine_id)?;
            Ok(list_of(&load_presets()?))
        })
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
        with_ctx_mut(|ctx| {
            ctx.presets.catalog.set_zones(&machine_id, out)?;
            Ok(list_of(&load_presets()?))
        })
    })
}

/// 新建一个品牌（id + 显示名；品牌图后配）。**即时落盘**，回一份新的清单。
///
/// 与「新增机型」不同：品牌全住一个 `brands.toml`，这里是往 `[[brands]]` 里加一段 ——
/// 不新建文件、不覆盖任何东西（那个「create_new 占路径」的风险在这里不存在）。
#[tauri::command]
pub fn wb_add_brand(id: String, name: String) -> Result<MachineList, AppError> {
    traced("wb_add_brand", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.catalog.add_brand(&id, &name)?;
            ctx.presets.catalog.write_brands()?;
            Ok(list_of(&load_presets()?))
        })
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
                            recommended_bundle: v.recommended_bundle.clone(),
                            tag: v.tag.clone(),
                            description: v.description.clone(),
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
}
