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

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::presets::{MachineField, Presets, VersionField};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrandView {
    pub id: String,
    pub name: String,
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    pub id: String,
    pub name: String,
    pub preset_file: Option<String>,
    pub recommended_bundle: Option<String>,
    pub tag: Option<String>,
    pub description: Option<String>,
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
    pub icon: Option<String>,
    /// 有没有 `[dimensions]`。A2L 实测没有 —— 界面上要能看出"这台还没配尺寸"
    pub has_dimensions: bool,
    /// 禁区块数。0 = 这台没有禁区文件
    pub zone_count: usize,
    pub versions: Vec<VersionView>,
    /// 它自己那个 toml 文件的名字（`A1.toml`）。**给人看的**，
    /// 让"我在改哪个文件"这件事不用猜
    pub file: String,
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
#[tauri::command]
pub fn wb_machines() -> Result<MachineList, AppError> {
    traced("wb_machines", |_| Ok(list_of(&Presets::load()?)))
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
#[tauri::command]
pub fn wb_add_version(
    machine_id: String,
    id: String,
    name: String,
) -> Result<MachineList, AppError> {
    traced("wb_add_version", |_| {
        let mut p = Presets::load()?;
        p.catalog
            .machine_mut(&machine_id)?
            .add_version(&id, &name)?;
        p.catalog.write_machine(&machine_id)?;
        tracing::info!(machine = %machine_id, version = %id, "加了一个版本");
        // 从盘上重读再返回：**界面看到的应该是落盘的结果**，不是内存里的样子。
        // 这两者不一致的话（写失败但界面显示成功），是最难查的一类
        Ok(list_of(&Presets::load()?))
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
        let mut p = Presets::load()?;
        p.catalog.add_machine(&id, &brand, &display)?;
        tracing::info!(machine = %id, "建了一台机型");
        // 重读盘再返回：**界面看到的是落盘的结果**。
        // 这一条尤其重要 —— 新建文件比改文件更容易出现"内存里成了、盘上没成"
        Ok(list_of(&Presets::load()?))
    })
}

/// 删这个版本会让哪些字段留下孤儿引用。**删之前先问这一条。**
///
/// 它是删除独有的风险：`[params.machineVariants]` 里 `A1:FAST` 这样的键
/// 在版本删掉之后会指向一个不存在的对象，而那种损坏**不报错** ——
/// 解析照样通过，只是那一项在那台机器上悄悄不生效了。
/// 所以要在删之前摆给人看，而不是删完让他自己发现
#[tauri::command]
pub fn wb_version_orphans(machine_id: String, version_id: String) -> Result<Vec<String>, AppError> {
    traced("wb_version_orphans", |_| {
        Ok(Presets::load()?.orphans_if_version_removed(&machine_id, &version_id))
    })
}

/// 删一个版本，立刻落盘。**不可逆**（没有回收站也没有撤销），
/// 所以界面那边是两步确认，而且确认框里要列出上面那条查出来的孤儿
#[tauri::command]
pub fn wb_remove_version(machine_id: String, version_id: String) -> Result<MachineList, AppError> {
    traced("wb_remove_version", |_| {
        let mut p = Presets::load()?;
        // 先记下来再删 —— 删完就查不出它被谁引用了
        let orphans = p.orphans_if_version_removed(&machine_id, &version_id);
        p.catalog
            .machine_mut(&machine_id)?
            .remove_version(&version_id)?;
        p.catalog.write_machine(&machine_id)?;
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
        Ok(list_of(&Presets::load()?))
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
        let mut p = Presets::load()?;
        p.catalog.machine_mut(&machine_id)?.set_version_field(
            &version_id,
            field,
            value.as_deref(),
        )?;
        p.catalog.write_machine(&machine_id)?;
        Ok(list_of(&Presets::load()?))
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
        let mut p = Presets::load()?;
        p.catalog
            .machine_mut(&machine_id)?
            .set_field(field, value.as_deref())?;
        p.catalog.write_machine(&machine_id)?;
        Ok(list_of(&Presets::load()?))
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
                icon: m.icon.clone(),
                has_dimensions: m.has_dimensions,
                zone_count: p.catalog.zones(&m.id).map_or(0, <[_]>::len),
                versions: m
                    .versions
                    .iter()
                    .map(|v| VersionView {
                        id: v.id.clone(),
                        name: v.name.clone(),
                        preset_file: v.preset_file.clone(),
                        recommended_bundle: v.recommended_bundle.clone(),
                        tag: v.tag.clone(),
                        description: v.description.clone(),
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
        // A2L：有版本、没尺寸、没禁区 —— 三个状态各自独立，界面要分别显示
        let a2l = p.catalog.machine("A2L").expect("A2L 在");
        assert!(!a2l.has_dimensions);
        assert_eq!(p.catalog.zones("A2L").map_or(0, <[_]>::len), 0);
        assert_eq!(a2l.versions.len(), 1);
        // P1S 有禁区
        assert!(p.catalog.zones("P1S").map_or(0, <[_]>::len) > 0);
    }
}
