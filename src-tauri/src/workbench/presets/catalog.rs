//! 机型目录：机型 + 品牌 + 禁区。对应 mkppanel 那个「机型目录」页。
//!
//! # 一个机型 = 一个文件
//!
//! `presets/machines/A1.toml` 里有那台机器的全部：元信息、`[dimensions]` 全族、
//! 以及 `[[versions]]` 列表。**「加一个版本」就是往那个文件里加一个 `[[versions]]` 块** ——
//! 这就是之前问「有位置给我加吗」时答不出来的那个位置。
//!
//! # 为什么每个机型都拖着一份 `DocumentMut`
//!
//! 因为往返保真（见模块头）。下面那些 `pub` 字段是**只读视图**，方便界面取数；
//! 真正的真相是 `doc`。写回时输出 `doc.to_string()`，没改过就逐字节等于原文。
//!
//! 反过来做（从只读字段重新序列化）会丢掉两样东西，而且都是静默的：
//! 我没建模的字段（`[dimensions.calibration]` 那十来个标定点在这一轮就没建模），
//! 以及原文的排版。前者是数据损坏，后者让 diff 变成一片红。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use toml_edit::DocumentMut;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

use super::literal_str;

/// 品牌。`presets/brands.toml` 里的 `[[brands]]`
#[derive(Debug, Clone)]
pub struct Brand {
    pub id: String,
    pub name: String,
    pub logo: Option<String>,
}

/// 一个版本。六个字段，与 mkppanel 版本卡上那六格一一对应
#[derive(Debug, Clone)]
pub struct MachineVersion {
    pub id: String,
    pub name: String,
    pub preset_file: Option<String>,
    pub recommended_bundle: Option<String>,
    pub tag: Option<String>,
    pub description: Option<String>,
}

/// 一块禁区 —— 一串点围成的多边形
#[derive(Debug, Clone)]
pub struct Zone {
    pub points: Vec<(f64, f64)>,
}

/// 版本身上可以改的那几格。
///
/// **用枚举而不是字符串字段名**：字符串会把「改一个不存在的字段」推到运行时，
/// 而且 IPC 边界上一个拼错的名字会静默变成"什么都没改"。枚举让它在反序列化就失败。
///
/// `id` **不在这里** —— 改 ID 等于删一个再加一个（引用会跟着断），风险不同，单独做
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VersionField {
    Name,
    PresetFile,
    RecommendedBundle,
    Tag,
    Description,
}

impl VersionField {
    fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::PresetFile => "presetFile",
            Self::RecommendedBundle => "recommendedBundle",
            Self::Tag => "tag",
            Self::Description => "description",
        }
    }

    /// 哪几格**不许清空**。`name` 是给人看的唯一标识，空了之后版本卡上只剩一个 ID
    fn required(self) -> bool {
        matches!(self, Self::Name)
    }
}

/// 机型身上可以改的那几格。`id` 不在这里（它是文件名）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MachineField {
    Display,
    Brand,
    Name,
    Image,
    Icon,
}

impl MachineField {
    fn key(self) -> &'static str {
        match self {
            Self::Display => "display",
            Self::Brand => "brand",
            Self::Name => "name",
            Self::Image => "image",
            Self::Icon => "icon",
        }
    }

    fn required(self) -> bool {
        matches!(self, Self::Display | Self::Brand)
    }
}

/// 一台机型。`pub` 字段是只读视图，写回看 [`Machine::to_toml`]
pub struct Machine {
    pub id: String,
    /// 界面上显示的名字。实测有机型的 `name` 是空串而 `display` 才是人看的
    pub display: String,
    pub name: String,
    pub brand: String,
    pub default_bundle: Option<String>,
    /// 外部别名（`A1C` / `A1F` 这种）。**不许与任何机型 ID 相撞**
    pub external_aliases: Vec<String>,
    pub image: Option<String>,
    pub icon: Option<String>,
    /// 这一轮**不建细模型**：尺寸页是后面的任务，现在只需要知道有没有。
    /// 字段本身在 `doc` 里一个不少 —— 建模不全不等于会丢
    pub has_dimensions: bool,
    pub versions: Vec<MachineVersion>,
    doc: DocumentMut,
    file: PathBuf,
}

impl std::fmt::Debug for Machine {
    /// 手写：`DocumentMut` 打出来是整个文件，一条日志能糊满屏
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Machine")
            .field("id", &self.id)
            .field("display", &self.display)
            .field("versions", &self.versions.len())
            .field("has_dimensions", &self.has_dimensions)
            .finish()
    }
}

impl Machine {
    /// 写回用的文本。**没改过就逐字节等于读进来那份**
    pub fn to_toml(&self) -> String {
        self.doc.to_string()
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    /// 加一个版本。**只往文件里插一个 `[[versions]]` 块，别处一个字节不动。**
    ///
    /// 这就是「加一个新版本怎么加」那个问题的答案所在：
    /// 一个版本 = 机型文件里的一个 `[[versions]]` 块。
    ///
    /// 六个字段里只要 `id` 与 `name` —— 其余四个（presetFile / recommendedBundle /
    /// tag / description）**留空不写**，而不是写成空串：
    /// 空串会在界面上显示成"已经填过但填了个空"，和"还没填"是两件事。
    pub fn add_version(&mut self, id: &str, name: &str) -> Result<(), AppError> {
        let id = id.trim();
        let name = name.trim();
        if id.is_empty() {
            return Err(AppError::invalid_argument("版本 ID 不能为空"));
        }
        if name.is_empty() {
            return Err(AppError::invalid_argument("版本名称不能为空"));
        }
        // ID 会进文件名与产物的键，所以限死字符集。
        // 放宽的话，一个带空格或中文的 ID 会在生成 TOML 那一步才炸，那时离现场很远了
        if !id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(AppError::invalid_argument(
                "版本 ID 只能用大写字母、数字和下划线",
            ));
        }
        if self.versions.iter().any(|v| v.id == id) {
            return Err(AppError::invalid_argument(format!(
                "这台机型已经有一个叫 {id} 的版本了"
            )));
        }

        let mut t = toml_edit::Table::new();
        t["id"] = literal_str(id);
        t["name"] = literal_str(name);

        // `versions` 可能整个不存在（一个版本都没有的机型）
        let entry = self
            .doc
            .entry("versions")
            .or_insert(toml_edit::Item::ArrayOfTables(
                toml_edit::ArrayOfTables::new(),
            ));
        let arr = entry.as_array_of_tables_mut().ok_or_else(|| {
            AppError::corrupted(format!("{} 的 versions 不是表数组", self.file.display()))
        })?;
        arr.push(t);

        self.versions.push(MachineVersion {
            id: id.to_owned(),
            name: name.to_owned(),
            preset_file: None,
            recommended_bundle: None,
            tag: None,
            description: None,
        });
        Ok(())
    }

    /// 删掉一个版本。
    ///
    /// # 这一条的风险和前两条都不同
    ///
    /// 加版本最坏是排版乱、加机型最坏是覆盖文件 —— 都在**这一个文件**里。
    /// 删版本的坏处会**跨文件**：`param_registry.toml` 的 `[params.machineVariants]`
    /// 里可能有 `{机型}:{版本}` 这样的键，删掉版本之后它们就成了指向不存在对象的孤儿。
    /// 那种损坏不会立刻报错，而是很久以后以「某台机器的某个参数莫名其妙不生效」出现。
    ///
    /// 所以**查引用不在这里做** —— 这一层只看得见机型文件。跨文件的那一半在
    /// [`super::Presets::orphans_if_version_removed`]，由调用方在删之前先问、并给人看。
    ///
    /// 「删掉最后一个版本」**允许**：零版本机型是合法的中间状态（刚建出来的机型就是）。
    /// 它在生成那一步会被判为阻断，那一层已有判据 —— 在这里拦住等于把一个可恢复的状态说成非法
    pub fn remove_version(&mut self, id: &str) -> Result<(), AppError> {
        let id = id.trim();
        let at = self
            .versions
            .iter()
            .position(|v| v.id == id)
            .ok_or_else(|| AppError::not_found(format!("{} 里没有叫 {id} 的版本", self.id)))?;

        let file = self.file.display().to_string();
        let mem = self.versions.len();
        let arr = self
            .doc
            .get_mut("versions")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .ok_or_else(|| AppError::corrupted(format!("{file} 的 versions 不是表数组")))?;
        // 文件里的顺序与 `self.versions` 一致（两者来自同一次解析），所以下标通用。
        // 不在 arr 里按 id 再找一遍是刻意的：两处各查一次就有两个答案的可能
        if at >= arr.len() {
            return Err(AppError::corrupted(format!(
                "{file} 的版本数与文件对不上（内存 {mem} 条，文件 {} 条）",
                arr.len()
            )));
        }
        arr.remove(at);
        self.versions.remove(at);
        Ok(())
    }

    /// 改版本的一格。
    ///
    /// # 「清空」是删键，不是写空串
    ///
    /// 这是本条独有的决定，前两条都没碰到：`tag = ''` 和**没有 `tag` 这一行**
    /// 在界面上是两件事 —— 前者读成「填过，填了个空」，后者是「还没填」。
    /// 所以传 `None`（或空串）时把那个键**整行删掉**。
    ///
    /// 与之对称：`name` / `display` / `brand` 这几格**不许清空**，
    /// 空了之后卡片上只剩一个 ID，人就认不出它是什么了。
    pub fn set_version_field(
        &mut self,
        version: &str,
        field: VersionField,
        value: Option<&str>,
    ) -> Result<(), AppError> {
        let version = version.trim();
        let at = self
            .versions
            .iter()
            .position(|v| v.id == version)
            .ok_or_else(|| AppError::not_found(format!("{} 里没有叫 {version} 的版本", self.id)))?;
        let val = value.map(str::trim).filter(|s| !s.is_empty());
        if val.is_none() && field.required() {
            return Err(AppError::invalid_argument(format!(
                "{} 不能清空",
                match field {
                    VersionField::Name => "版本名称",
                    _ => field.key(),
                }
            )));
        }

        let file = self.file.display().to_string();
        let arr = self
            .doc
            .get_mut("versions")
            .and_then(toml_edit::Item::as_array_of_tables_mut)
            .ok_or_else(|| AppError::corrupted(format!("{file} 的 versions 不是表数组")))?;
        let t = arr
            .get_mut(at)
            .ok_or_else(|| AppError::corrupted(format!("{file} 的版本数与文件对不上")))?;
        match val {
            Some(s) => t[field.key()] = literal_str(s),
            None => {
                t.remove(field.key());
            }
        }

        let v = &mut self.versions[at];
        let owned = val.map(str::to_owned);
        match field {
            VersionField::Name => v.name = owned.unwrap_or_default(),
            VersionField::PresetFile => v.preset_file = owned,
            VersionField::RecommendedBundle => v.recommended_bundle = owned,
            VersionField::Tag => v.tag = owned,
            VersionField::Description => v.description = owned,
        }
        Ok(())
    }

    /// 改机型自己的一格。`None`/空 = 删键（同 [`Self::set_version_field`]）
    pub fn set_field(&mut self, field: MachineField, value: Option<&str>) -> Result<(), AppError> {
        let val = value.map(str::trim).filter(|s| !s.is_empty());
        if val.is_none() && field.required() {
            return Err(AppError::invalid_argument(format!(
                "{} 不能清空",
                match field {
                    MachineField::Display => "显示名",
                    MachineField::Brand => "品牌",
                    _ => field.key(),
                }
            )));
        }
        match val {
            Some(s) => self.doc[field.key()] = literal_str(s),
            None => {
                self.doc.remove(field.key());
            }
        }
        let owned = val.map(str::to_owned);
        match field {
            MachineField::Display => self.display = owned.unwrap_or_default(),
            MachineField::Brand => self.brand = owned.unwrap_or_default(),
            MachineField::Name => self.name = owned.unwrap_or_default(),
            MachineField::Image => self.image = owned,
            MachineField::Icon => self.icon = owned,
        }
        Ok(())
    }
}

/// 机型目录
#[derive(Debug)]
pub struct Catalog {
    brands: Vec<Brand>,
    machines: Vec<Machine>,
    /// 机型 ID → 禁区。只有三台机器有
    zones: BTreeMap<String, Vec<Zone>>,
    /// 数据根。**新建机型要在这里落文件** ——
    /// 从已有机型的路径反推是不行的：一台机型都没有的时候就推不出来了
    root: PathBuf,
}

impl Catalog {
    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let brands = load_brands(&root.join("brands.toml"))?;
        let machines = load_machines(&root.join("machines"))?;
        check_unique_ids(
            &machines
                .iter()
                .map(|m| {
                    (
                        m.id.clone(),
                        m.versions.iter().map(|v| v.id.clone()).collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
        )?;
        let zones = load_zones(&root.join("forbidden_zones"))?;
        Ok(Self {
            brands,
            machines,
            zones,
            root: root.to_path_buf(),
        })
    }

    pub fn brands(&self) -> &[Brand] {
        &self.brands
    }

    pub fn machines(&self) -> &[Machine] {
        &self.machines
    }

    /// 按 ID 找机型。**别名不走这里** —— 别名是外部叫法，不是主键
    pub fn machine(&self, id: &str) -> Option<&Machine> {
        self.machines.iter().find(|m| m.id == id)
    }

    /// 要改它的时候用这个。返回 `Result` 而不是 `Option`：
    /// 「机型不存在」在编辑路径上是个要报给人看的错，不是一个可以静默跳过的分支
    pub fn machine_mut(&mut self, id: &str) -> Result<&mut Machine, AppError> {
        self.machines
            .iter_mut()
            .find(|m| m.id == id)
            .ok_or_else(|| AppError::not_found(format!("没有机型 {id}")))
    }

    pub fn zones(&self, machine_id: &str) -> Option<&[Zone]> {
        self.zones.get(machine_id).map(Vec::as_slice)
    }

    /// 全部 `机型:版本` 键。`param_registry.toml` 的 `machineVariants`
    /// 用的就是这个形状，跨文件校验要拿它比对（见 [`super::Presets::check_cross_consistency`]）
    pub fn machine_keys(&self) -> BTreeSet<String> {
        self.machines
            .iter()
            .flat_map(|m| m.versions.iter().map(move |v| format!("{}:{}", m.id, v.id)))
            .collect()
    }

    /// 把一台机型写回它自己的文件。原子写（同目录临时文件 + rename）——
    /// 半个文件的 TOML 比没有更糟
    pub fn write_machine(&self, id: &str) -> Result<(), AppError> {
        let m = self
            .machine(id)
            .ok_or_else(|| AppError::not_found(format!("没有机型 {id}")))?;
        atomic_write(m.file(), m.to_toml().as_bytes())
    }

    /// 新建一台机型 = 新建一个 `presets/machines/{ID}.toml`。
    ///
    /// # 这一条的风险和「加版本」完全不同
    ///
    /// 加版本是改一个已有文件，最坏情况是排版被搅乱 —— 难看，但数据还在。
    /// 加机型要**新建文件**，最坏情况是**覆盖掉一个已经存在的机型**，那是不可逆的数据丢失。
    ///
    /// 所以这里不用 `atomic_write`（它的 rename 会覆盖），
    /// 而是先用 `create_new` 原子地占住那个路径：它要么创建成功，要么因为已存在而失败。
    /// 「先 stat 再写」有一个窗口 —— 桌面单用户下几乎撞不上，
    /// 但"几乎不会"和"不可能"在不可逆操作上是两件事。
    ///
    /// # ID 要查两处
    ///
    /// 不只是已有机型的 `id`，还有**所有 `externalAliases`**。
    /// 撞了别名之后「按 ID 找机型」会有两个答案，而那种歧义要到很久以后才会以
    /// 「参数写到了另一台机器上」的形式暴露出来。
    pub fn add_machine(&mut self, id: &str, brand: &str, display: &str) -> Result<(), AppError> {
        let id = id.trim();
        let brand = brand.trim();
        let display = display.trim();
        if id.is_empty() {
            return Err(AppError::invalid_argument("机型 ID 不能为空"));
        }
        if display.is_empty() {
            return Err(AppError::invalid_argument("显示名不能为空"));
        }
        if brand.is_empty() {
            return Err(AppError::invalid_argument("品牌不能为空"));
        }
        // ID 直接变成文件名，所以字符集限死 —— 放宽的话一个带 `/` 或空格的 ID
        // 会在建文件那一步才炸，而那时已经离现场很远
        if !id
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(AppError::invalid_argument(
                "机型 ID 只能用大写字母、数字和下划线（它会直接变成文件名）",
            ));
        }
        if self.machine(id).is_some() {
            return Err(AppError::invalid_argument(format!(
                "已经有一台叫 {id} 的机型"
            )));
        }
        if let Some(owner) = self
            .machines
            .iter()
            .find(|m| m.external_aliases.iter().any(|a| a == id))
        {
            return Err(AppError::invalid_argument(format!(
                "{id} 已经是 {} 的外部别名了，换一个 —— 否则按 ID 找机型会有两个答案",
                owner.id
            )));
        }

        let dir = self.root.join("machines");
        std::fs::create_dir_all(&dir).map_err(|e| {
            AppError::internal(format!("建不出 {}", dir.display())).with_detail(e.to_string())
        })?;
        let file = dir.join(format!("{id}.toml"));

        // **原子地占住这个路径。** 已存在就失败，绝不覆盖。
        //
        // `File::create_new` 就是 `OpenOptions::new().write(true).create_new(true).open(..)`
        // （std 的实现即是如此），而后者被 `clippy.toml` 列进了禁列 —— 它是绕开
        // "写盘只经一处"的两条闸门之一。这里要的语义恰好是 `create_new`：
        // **不可能截断已有文件**，那正是禁列允许的那一类
        std::fs::File::create_new(&file).map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => AppError::invalid_argument(format!(
                "{} 已经存在了。**没有覆盖它** —— 如果那是一台你想改的机型，去左边选它",
                file.display()
            )),
            _ => {
                AppError::internal(format!("建不出 {}", file.display())).with_detail(e.to_string())
            }
        })?;

        let mut doc = DocumentMut::new();
        doc["id"] = literal_str(id);
        doc["display"] = literal_str(display);
        doc["brand"] = literal_str(brand);
        // 其余字段**一个都不写**：`name` / `defaultBundle` / `externalAliases` /
        // `image` / `icon` / `[dimensions]` / `[[versions]]` 都留空。
        // 写成空串或空数组会让界面显示成"填过但填了个空"，和"还没填"是两件事
        atomic_write(&file, doc.to_string().as_bytes())?;

        self.machines.push(Machine {
            id: id.to_owned(),
            display: display.to_owned(),
            name: String::new(),
            brand: brand.to_owned(),
            default_bundle: None,
            external_aliases: Vec::new(),
            image: None,
            icon: None,
            has_dimensions: false,
            versions: Vec::new(),
            doc,
            file,
        });
        // 顺序照文件名 —— 与 `load_machines` 的排序保持一致，
        // 否则新建之后的列表顺序和重启之后不一样
        self.machines.sort_by(|a, b| a.file.cmp(&b.file));
        Ok(())
    }
}

fn parse(path: &Path) -> Result<DocumentMut, AppError> {
    let text = super::read(path)?;
    super::parse_text(&text, path)
}

fn load_brands(path: &Path) -> Result<Vec<Brand>, AppError> {
    let doc = parse(path)?;
    let Some(arr) = doc.get("brands").and_then(|i| i.as_array_of_tables()) else {
        return Ok(Vec::new());
    };
    Ok(arr
        .iter()
        .filter_map(|t| {
            Some(Brand {
                id: t.get("id")?.as_str()?.to_owned(),
                name: t
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                logo: t
                    .get("logo")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
                    .filter(|s| !s.is_empty()),
            })
        })
        .collect())
}

/// 机型与版本的 id 唯一性（b05 Task 11.3）—— **大小写不敏感**。
///
/// 四类 id（机型 / 版本 / 资产 / 套餐）都要过这一条：资产与套餐在各自 load 时查了
/// （Task 8.8 / Task 10），这里补机型与版本这两类。doc §9 的理由：仅大小写不同的
/// 两个 id 在「文件名统一小写」之后会撞成同一个文件，而「按 id 查」的结果取决于
/// 谁排在前面 —— 撞了必须当场报错，不能等到某个平台或某次改名才炸。
///
/// 输入是 `(机型 id, 版本 id 列表)` 的切片而不是 `&[Machine]`：检查的只是 id 集合，
/// 给更小的输入面，测试也能直接喂（Windows 的文件系统大小写不敏感，
/// 仅大小写不同的两份机型文件在夹具里根本写不出来）。
fn check_unique_ids(machines: &[(String, Vec<String>)]) -> Result<(), AppError> {
    let mut seen: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for (id, versions) in machines {
        let lower = id.to_lowercase();
        if let Some(prev) = seen.insert(lower, id.clone()) {
            return Err(AppError::corrupted(format!(
                "机型 id 撞了：{prev} 与 {id}（仅大小写不同）"
            ))
            .with_detail(
                "id 是主键，且大小写不敏感 —— 撞了的话「按 id 查」的结果取决于谁排在前面，\
                 而文件名统一小写后两者还会撞成同一个文件"
                    .to_owned(),
            ));
        }
        // 版本 id 是**机型内**唯一（11.3）：不同机型有同名版本是合法的（P1S 与 X1C 都有 lite）
        let mut versions_seen: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for v in versions {
            let vl = v.to_lowercase();
            if let Some(prev) = versions_seen.insert(vl, v.clone()) {
                return Err(AppError::corrupted(format!(
                    "机型 {id} 的版本 id 撞了：{prev} 与 {v}（仅大小写不同）"
                ))
                .with_detail(
                    "版本 id 在机型内唯一，且大小写不敏感（doc §9）：\
                     产物文件名统一小写，`Fast` 与 `FAST` 会写成同一个文件"
                        .to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn load_machines(dir: &Path) -> Result<Vec<Machine>, AppError> {
    let mut out = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        AppError::not_found(format!("读不到机型目录 {}", dir.display())).with_detail(e.to_string())
    })?;
    // 收集再排序：`read_dir` 的顺序是文件系统说的，不稳定。
    // 机型顺序会直接进界面，必须是确定的
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();

    for file in files {
        let doc = parse(&file)?;
        let id = doc
            .get("id")
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                AppError::corrupted(format!("{} 里没有 id", file.display()))
                    .with_detail("机型文件的第一行就该是 id = '...'".to_owned())
            })?;
        let s = |k: &str| {
            doc.get(k)
                .and_then(|v| v.as_str())
                .map(str::to_owned)
                .filter(|v| !v.is_empty())
        };
        let versions = doc
            .get("versions")
            .and_then(|i| i.as_array_of_tables())
            .map(|arr| {
                arr.iter()
                    .filter_map(|t| {
                        let g = |k: &str| {
                            t.get(k)
                                .and_then(|v| v.as_str())
                                .map(str::to_owned)
                                .filter(|v| !v.is_empty())
                        };
                        Some(MachineVersion {
                            id: t.get("id")?.as_str()?.to_owned(),
                            name: g("name").unwrap_or_default(),
                            preset_file: g("presetFile"),
                            recommended_bundle: g("recommendedBundle"),
                            tag: g("tag"),
                            description: g("description"),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        out.push(Machine {
            display: s("display").unwrap_or_else(|| id.clone()),
            name: s("name").unwrap_or_default(),
            brand: s("brand").unwrap_or_default(),
            default_bundle: s("defaultBundle"),
            external_aliases: doc
                .get("externalAliases")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            image: s("image"),
            icon: s("icon"),
            has_dimensions: doc.get("dimensions").is_some(),
            versions,
            id,
            doc,
            file,
        });
    }
    Ok(out)
}

fn load_zones(dir: &Path) -> Result<BTreeMap<String, Vec<Zone>>, AppError> {
    let mut out = BTreeMap::new();
    // 禁区目录可以整个不存在 —— 一台有禁区的机器都没有是合法状态
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Ok(out);
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "toml"))
        .collect();
    files.sort();

    for file in files {
        // 文件名就是机型 ID（实测 `P1S.toml` / `P2S.toml` / `X1C.toml`），
        // 文件里面没有 id 字段
        let Some(machine) = file.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let doc = parse(&file)?;
        let zones = doc
            .get("zones")
            .and_then(|i| i.as_array_of_tables())
            .map(|arr| {
                arr.iter()
                    .map(|z| Zone {
                        points: z
                            .get("points")
                            .and_then(|i| i.as_array_of_tables())
                            .map(|pts| {
                                pts.iter()
                                    .filter_map(|p| {
                                        Some((p.get("x")?.as_float()?, p.get("y")?.as_float()?))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.insert(machine.to_owned(), zones);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::paths;
    use crate::workbench::presets::{can_be_literal, one_edit_only};

    fn catalog() -> Option<(PathBuf, Catalog)> {
        let root = paths::presets_root()?;
        let c = Catalog::load_from(&root).expect("机型目录读不齐");
        Some((root, c))
    }

    /// **本轮最重要的一条**：读进来不改再写出去，逐字节不变。
    ///
    /// 它是往 `presets/` 写第一个字节之前必须先绿的判据 —— 否则第一次点保存
    /// 就可能把六个机型文件的排版搅乱，而那种损坏在 diff 里是一片红，
    /// 真正改了什么反而看不出来
    #[test]
    fn reading_and_writing_back_changes_nothing() {
        let Some((_root, c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        assert!(
            !c.machines().is_empty(),
            "一个机型都没读到，下面的断言会空转"
        );

        for m in c.machines() {
            let original = std::fs::read_to_string(m.file()).expect("刚读过的文件");
            let round = m.to_toml();
            assert_eq!(
                original,
                round,
                "{} 读进来再写出去变了。\n差异从第 {} 个字节开始",
                m.file().display(),
                original
                    .bytes()
                    .zip(round.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(original.len().min(round.len())),
            );
        }
    }

    /// 反空转：判据必须能抓到真的改动。
    ///
    /// 不验这一条的话，上面那条在「`to_toml` 直接返回原文字符串」这种假实现下也会绿
    #[test]
    fn the_fidelity_judge_would_catch_a_real_edit() {
        let Some((_root, c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let m = c.machine("A1").expect("A1 必须在");
        let mut doc: DocumentMut = m.to_toml().parse().expect("自己写出来的应该能再读回");
        doc["display"] = toml_edit::value("改过了");
        assert_ne!(m.to_toml(), doc.to_string(), "改了一个字段却比不出差异");
        // 而且**只有那一处**变了：保真不是"整体重排后碰巧相等"
        assert!(doc.to_string().contains("改过了"), "改动没落到文本里");
        assert!(
            doc.to_string().contains("externalAliases"),
            "改一个字段把别的字段弄丢了"
        );
    }

    /// 原子写走一遍真磁盘：写到临时目录，读回来和源文件一致
    #[test]
    fn writing_a_machine_back_is_atomic_and_faithful() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (_tmp, c2) = copy_of(&root);
        let before = std::fs::read_to_string(c2.machine("A1").unwrap().file()).unwrap();
        c2.write_machine("A1").expect("写回");
        let after = std::fs::read_to_string(c2.machine("A1").unwrap().file()).unwrap();
        assert_eq!(before, after, "没改任何东西的一次写回改变了文件");
    }

    /// 把机型目录复制到临时目录，**在副本上改**，不碰真数据
    fn copy_of(root: &Path) -> (tempfile::TempDir, Catalog) {
        let tmp = tempfile::tempdir().expect("临时目录");
        let dst = tmp.path().join("machines");
        std::fs::create_dir_all(&dst).unwrap();
        for f in std::fs::read_dir(root.join("machines")).unwrap() {
            let p = f.unwrap().path();
            std::fs::copy(&p, dst.join(p.file_name().unwrap())).unwrap();
        }
        std::fs::copy(root.join("brands.toml"), tmp.path().join("brands.toml")).unwrap();
        let c = Catalog::load_from(tmp.path()).expect("副本也该读得通");
        (tmp, c)
    }

    /* ---------- 唯一性（b05 Task 11.3） ---------- */

    /// **仅大小写不同的 id 必须当场炸**（11.3 / doc §9）。
    ///
    /// 四类 id 里资产与套餐已各有判据，这一条补机型与版本。检查函数收
    /// `(机型 id, 版本 id 列表)` 切片而不是 `&[Machine]`：Windows 的文件系统
    /// 大小写不敏感，仅大小写不同的两份机型文件在夹具里根本写不出来 ——
    /// 输入面收小之后，这个形状才能直接喂进来
    #[test]
    fn case_only_id_collisions_are_refused() {
        // 机型 id 撞（A1 vs a1）
        let err = check_unique_ids(&[("A1".to_owned(), vec![]), ("a1".to_owned(), vec![])])
            .expect_err("撞了必须报错");
        assert!(
            err.message.contains("机型 id 撞了"),
            "实测：{}",
            err.message
        );
        assert!(
            err.message.contains("A1") && err.message.contains("a1"),
            "要说清撞的是哪两个：{}",
            err.message
        );

        // 版本 id 撞（机型内 FAST vs fast）；**不同机型的同名版本是合法的**
        let err = check_unique_ids(&[(
            "A1".to_owned(),
            vec!["STANDARD".to_owned(), "fast".to_owned(), "FAST".to_owned()],
        )])
        .expect_err("机型内撞版本 id 必须报错");
        assert!(
            err.message.contains("版本 id 撞了"),
            "实测：{}",
            err.message
        );

        // P1S 与 X1C 都有 lite —— 机型内唯一就够，跨机型同名不是错
        check_unique_ids(&[
            ("P1S".to_owned(), vec!["LITE".to_owned()]),
            ("X1C".to_owned(), vec!["lite".to_owned()]),
        ])
        .expect("不同机型的同名版本不该报错");
    }

    /// 加载期同一道闸：**从盘上读出来的数据撞 id 也一样拦**（副本上改，不碰真数据）
    #[test]
    fn loading_a_directory_with_case_only_collisions_fails() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };

        // 机型 id 撞：文件名可以不同（Windows 允许），文件**里**的 id 只差大小写
        let (tmp, _c) = copy_of(&root);
        crate::fsx::atomic::atomic_write(
            &tmp.path().join("machines").join("TEST_LOWER.toml"),
            "id = 'a1'\ndisplay = '小写撞名机'\nbrand = 'Bambu Lab'\n".as_bytes(),
        )
        .unwrap();
        let err = Catalog::load_from(tmp.path()).expect_err("撞机型 id 必须整目录拒载");
        assert!(
            err.message.contains("机型 id 撞了"),
            "实测：{}",
            err.message
        );

        // 版本 id 撞：往 A1.toml 尾部 append 一个 fast（A1 已有 FAST）
        let (tmp, _c) = copy_of(&root);
        let p = tmp.path().join("machines").join("A1.toml");
        let mut text = std::fs::read_to_string(&p).unwrap();
        text.push_str("\n[[versions]]\nid = 'fast'\nname = '撞名版'\n");
        crate::fsx::atomic::atomic_write(&p, text.as_bytes()).unwrap();
        let err = Catalog::load_from(tmp.path()).expect_err("机型内撞版本 id 必须整目录拒载");
        assert!(
            err.message.contains("版本 id 撞了"),
            "实测：{}",
            err.message
        );
    }

    /// 判据的判据。`one_edit_only` 是个会被三种改动共用的工具，
    /// **它自己错了的话，上面那些测试会一起变成空转** ——
    /// 比如实现成"永远返回两个空串"，那么「什么都没删」这类断言就白通过了。
    ///
    /// 四种形状各验一次：纯追加 / 纯删除 / 中间替换 / 完全没变
    #[test]
    fn the_one_edit_judge_reports_the_right_span() {
        // 纯追加（加版本就是这一种）
        let (rm, ins) = one_edit_only("abc\ndef\n", "abc\ndef\nghi\n");
        assert!(rm.is_empty());
        assert_eq!(ins, "ghi\n");

        // 纯删除（删版本会是这一种）—— 前缀断言在这里直接失效，这一条不会
        let (rm, ins) = one_edit_only("abc\ndef\nghi\n", "abc\nghi\n");
        assert_eq!(rm, "def\n");
        assert!(ins.is_empty());

        // 中间替换（改一个字段）
        let (rm, ins) = one_edit_only("a = 1\nb = 2\n", "a = 9\nb = 2\n");
        assert_eq!(rm, "1");
        assert_eq!(ins, "9");

        // 完全没变 —— 两边都空
        let (rm, ins) = one_edit_only("same\n", "same\n");
        assert!(rm.is_empty() && ins.is_empty());

        // 中文不许被劈成半个字（机型名与版本名都是中文）
        let (rm, ins) = one_edit_only("name = '标准版'\n", "name = '快拆版'\n");
        assert!(rm.chars().count() > 0 && ins.chars().count() > 0);
        assert!(rm.contains('标') || rm.contains('准'), "切坏了：{rm:?}");
    }

    /* ---------- 「加一台机型」 ---------- */

    /// **写出来的东西必须能被自己的 loader 读回去。**
    ///
    /// 这是本条独有的风险：加版本是往一个已经合法的文件里插东西，
    /// 而加机型是**从零造一个文件** —— 少写一个必需字段就会造出一个读不回来的垃圾，
    /// 而那种失败在"写入成功"这一层看不出来
    #[test]
    fn a_new_machine_file_can_be_read_back_by_our_own_loader() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        c.add_machine("TEST_X9", "Bambu Lab", "测试机 X9")
            .expect("加得上");

        // **从盘上重读**，走完整的 loader
        let c2 = Catalog::load_from(tmp.path()).expect("重读整个目录");
        let m = c2.machine("TEST_X9").expect("新机型必须读得回来");
        assert_eq!(m.display, "测试机 X9");
        assert_eq!(m.brand, "Bambu Lab");
        // 零版本、无尺寸、无禁区 —— 一台刚建出来的机型就该是这样
        assert!(m.versions.is_empty());
        assert!(!m.has_dimensions);
        assert_eq!(c2.zones("TEST_X9").map_or(0, <[_]>::len), 0);
        // 其余字段留空不写，不是空串
        assert!(m.default_bundle.is_none() && m.image.is_none() && m.icon.is_none());
        assert!(m.external_aliases.is_empty());

        // 原有六台一台没少，而且顺序仍然照文件名（新建之后的顺序要和重启之后一样）
        assert_eq!(c2.machines().len(), 7);
        let ids: Vec<&str> = c2.machines().iter().map(|m| m.id.as_str()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        assert_eq!(ids, sorted, "顺序不是稳定的：{ids:?}");

        // 新建之后**不重读**也能立刻看到（界面依赖这个）
        assert!(c.machine("TEST_X9").is_some());
        assert_eq!(c.machines().len(), 7);
    }

    /// **绝不覆盖已存在的文件。** 这是本条唯一的不可逆风险
    #[test]
    fn adding_a_machine_refuses_to_overwrite_an_existing_file() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        let a1 = tmp.path().join("machines").join("A1.toml");
        let before = std::fs::read_to_string(&a1).unwrap();

        // 走「ID 已存在」这条：先被 `machine()` 查重拦下
        let e = c
            .add_machine("A1", "Bambu Lab", "冒牌 A1")
            .expect_err("该被拒");
        assert!(e.message.contains("A1"));
        assert_eq!(
            std::fs::read_to_string(&a1).unwrap(),
            before,
            "A1.toml 被动了"
        );

        // 再走「文件已存在但内存里没这台机型」这条 —— 那是 `create_new` 兜的底。
        // 手动造出这个状态：盘上有文件，但 Catalog 是在它出现之前加载的
        let ghost = tmp.path().join("machines").join("GHOST.toml");
        atomic_write(&ghost, b"id = 'GHOST'\n").unwrap();
        let e2 = c
            .add_machine("GHOST", "Bambu Lab", "幽灵")
            .expect_err("文件已存在就该拒");
        assert!(
            e2.message.contains("GHOST.toml"),
            "要说出是哪个文件：{}",
            e2.message
        );
        assert_eq!(
            std::fs::read_to_string(&ghost).unwrap(),
            "id = 'GHOST'\n",
            "已存在的文件被覆盖了 —— 这是不可逆的数据丢失"
        );
    }

    /// ID 撞上别名也要拒。用真数据里的 `A1C`（A1 的外部别名）
    #[test]
    fn a_machine_id_that_collides_with_an_alias_is_refused() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        assert!(
            c.machine("A1")
                .unwrap()
                .external_aliases
                .contains(&"A1C".to_owned()),
            "前提变了：A1 不再有 A1C 这个别名，这条测试要重写"
        );

        let e = c
            .add_machine("A1C", "Bambu Lab", "撞别名的")
            .expect_err("该被拒");
        assert!(
            e.message.contains("A1C") && e.message.contains("A1"),
            "{}",
            e.message
        );
        // 文件**一个都没建**
        assert!(!tmp.path().join("machines").join("A1C.toml").exists());
        assert_eq!(c.machines().len(), 6, "被拒的调用改动了状态");
    }

    /// 三条格式拒绝，每条有自己的话
    #[test]
    fn adding_a_machine_refuses_bad_input_with_a_reason() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        let empty = c.add_machine("", "B", "D").expect_err("空 ID");
        let no_display = c.add_machine("OK1", "B", "").expect_err("空显示名");
        let no_brand = c.add_machine("OK2", "", "D").expect_err("空品牌");
        // 带斜杠的 ID 尤其要拦：它会变成文件名
        let slash = c.add_machine("A/B", "B", "D").expect_err("非法字符");

        let msgs = [&empty, &no_display, &no_brand, &slash].map(|e| e.message.clone());
        for (i, a) in msgs.iter().enumerate() {
            assert!(!a.is_empty());
            for b in &msgs[i + 1..] {
                assert_ne!(a, b, "两种错说了同一句话");
            }
        }
        // 一个文件都没建出来
        let n = std::fs::read_dir(tmp.path().join("machines"))
            .unwrap()
            .count();
        assert_eq!(n, 6, "被拒的调用建出了文件");
    }

    /// 引号风格的判据自己也要被验一次。
    ///
    /// 它用的是 `set_repr_unchecked`（**不校验** repr 与值是否一致），
    /// 所以 `can_be_literal` 错一点就会写出非法 TOML 或值被悄悄改掉。
    /// 判据：写出去再读回来，**值必须一模一样**
    #[test]
    fn literal_quoting_never_changes_the_value_it_writes() {
        let cases = [
            "标准版",        // 中文，能用单引号
            "A1 fast",       // 带空格
            "it's mine",     // **含单引号 → 必须退回双引号**
            "line1\nline2",  // 含换行 → 退回双引号
            "tab\there",     // 控制字符
            "quote\"inside", // 含双引号，但能用单引号
            "both'and\"",    // 两种都有
        ];
        for c in cases {
            let mut doc = DocumentMut::new();
            doc["k"] = literal_str(c);
            let text = doc.to_string();
            // 写出去的必须是合法 TOML，而且读回来的值一字不差
            let back: DocumentMut = text
                .parse()
                .unwrap_or_else(|e| panic!("写出了非法 TOML：{text:?} / {e}"));
            assert_eq!(
                back["k"].as_str(),
                Some(c),
                "值被表示形式改掉了：写的 {c:?}，读回 {:?}（文本 {text:?}）",
                back["k"].as_str()
            );
            // 能用字面量的就该用字面量 —— 否则这个函数等于没起作用
            if can_be_literal(c) {
                assert!(
                    text.contains(&format!("'{c}'")),
                    "该用单引号却没有：{text:?}"
                );
            }
        }
    }

    /* ---------- 「改一格」 ---------- */

    /// 改一个值 = `one_edit_only` 的**第三种形状**：两边都非空。
    ///
    /// 顺带盯一件前两条没碰到的事：**改完之后引号风格会不会漂**。
    /// 真数据全用单引号（`name = '标准版'`），而 `toml_edit::value()` 默认输出双引号 ——
    /// 如果漂了，这条测试会把实际输出打出来，那是个要正面决定的事，不是"反正能跑"
    #[test]
    fn editing_one_field_replaces_exactly_that_span() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        let before = c.machine("A1").unwrap().to_toml();
        let v0 = c.machine("A1").unwrap().versions[0].id.clone();

        c.machine_mut("A1")
            .unwrap()
            .set_version_field(&v0, VersionField::Name, Some("改过的名字"))
            .expect("改得动");
        c.write_machine("A1").expect("写回");

        let c2 = Catalog::load_from(tmp.path()).expect("重读");
        let m = c2.machine("A1").unwrap();
        assert_eq!(m.versions[0].name, "改过的名字");
        assert_eq!(m.versions[0].id, v0, "改名字不该动 ID");
        assert_eq!(m.versions.len(), 3, "版本数变了");

        // 两边都非空 —— 这就是"替换"的形状
        let (removed, inserted) = one_edit_only(&before, &m.to_toml());
        assert!(!removed.is_empty() && !inserted.is_empty(), "不是替换形状");
        assert!(
            inserted.contains("改过的名字"),
            "新值不在插入段：{inserted:?}"
        );
        // **别的字段一个都不许进这次改动的范围**
        assert!(
            !removed.contains("presetFile"),
            "波及了 presetFile：{removed:?}"
        );
        assert!(
            !removed.contains("[[versions]]"),
            "波及了整个块：{removed:?}"
        );

        // **引号风格**：真数据全用单引号。这一条是拿来**测量**的 ——
        // 通过说明 `toml_edit` 跟着原文；失败就打出实际输出，那时要正面决定
        // 是接受风格漂移，还是显式保留原来的表示
        let line = m
            .to_toml()
            .lines()
            .find(|l| l.contains("改过的名字"))
            .expect("改完的那一行")
            .to_owned();
        assert!(
            line.contains('\'') && !line.contains('"'),
            "引号风格漂了（真数据是单引号，写出来是）：{line}"
        );
    }

    /// 「清空」是**删键**，不是写空串。
    ///
    /// `tag = ''` 和没有 `tag` 这一行在界面上是两件事：
    /// 前者读成「填过，填了个空」，后者是「还没填」
    #[test]
    fn clearing_an_optional_field_removes_the_key_instead_of_writing_an_empty_string() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        // 找一个本来有 tag 的版本
        let (vid, _) = c
            .machine("A1")
            .unwrap()
            .versions
            .iter()
            .find_map(|v| v.tag.as_ref().map(|t| (v.id.clone(), t.clone())))
            .expect("A1 至少有一版带 tag");

        c.machine_mut("A1")
            .unwrap()
            .set_version_field(&vid, VersionField::Tag, None)
            .expect("清得掉");
        c.write_machine("A1").expect("写回");

        let c2 = Catalog::load_from(tmp.path()).expect("重读");
        let v = c2
            .machine("A1")
            .unwrap()
            .versions
            .iter()
            .find(|v| v.id == vid)
            .unwrap();
        assert!(v.tag.is_none());
        // **文件里那一行整行没了**，而不是变成 tag = ''
        let text = c2.machine("A1").unwrap().to_toml();
        assert!(!text.contains("tag = ''"), "写成了空串：\n{text}");
        assert!(!text.contains("tag = \"\""), "写成了空串：\n{text}");
    }

    /// 必填的那几格不许清空，而且**拒绝之后什么都没动**
    #[test]
    fn required_fields_refuse_to_be_cleared() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (_tmp, mut c) = copy_of(&root);
        let before = c.machine("A1").unwrap().to_toml();
        let v0 = c.machine("A1").unwrap().versions[0].id.clone();
        let m = c.machine_mut("A1").unwrap();

        let e1 = m
            .set_version_field(&v0, VersionField::Name, None)
            .expect_err("版本名称不许清空");
        let e2 = m
            .set_field(MachineField::Display, Some("  "))
            .expect_err("显示名不许清空");
        let e3 = m
            .set_field(MachineField::Brand, None)
            .expect_err("品牌不许清空");
        assert!(e1.message.contains("版本名称"), "{}", e1.message);
        assert!(e2.message.contains("显示名"), "{}", e2.message);
        assert!(e3.message.contains("品牌"), "{}", e3.message);
        // 只有空白的输入也算清空 —— 不能靠"非空串"就放过去
        assert_eq!(m.to_toml(), before, "被拒的调用改动了文本");

        // 可选的那几格**可以**清空 —— 对照组，证明上面拦的是"必填"不是"全部"
        m.set_field(MachineField::Icon, None).expect("图标可以清空");
    }

    /// 删除的**形状**：`one_edit_only` 的插入段必须为空。
    ///
    /// 这正是「新文本以旧文本为前缀」那条判据换掉的理由 —— 它在这里根本不成立
    #[test]
    fn removing_a_version_deletes_one_block_and_touches_nothing_else() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        let before = c.machine("A1").unwrap().to_toml();
        // 删中间那一个 —— 删末尾那个会让"前缀"判据碰巧也通过，测不出东西
        let victim = c.machine("A1").unwrap().versions[1].id.clone();
        let survivors: Vec<String> = c
            .machine("A1")
            .unwrap()
            .versions
            .iter()
            .filter(|v| v.id != victim)
            .map(|v| v.id.clone())
            .collect();

        c.machine_mut("A1")
            .unwrap()
            .remove_version(&victim)
            .expect("删得掉");
        c.write_machine("A1").expect("写回");

        let c2 = Catalog::load_from(tmp.path()).expect("重读");
        let m = c2.machine("A1").unwrap();
        let now: Vec<String> = m.versions.iter().map(|v| v.id.clone()).collect();
        assert_eq!(now, survivors, "剩下的版本或顺序不对");

        let (removed, inserted) = one_edit_only(&before, &m.to_toml());
        assert!(inserted.trim().is_empty(), "删除却插入了东西：{inserted:?}");
        assert!(
            removed.contains(&victim),
            "删掉的那段里没有它的 id：{removed:?}"
        );
        // 别的版本的 id 不许出现在被删掉的那一段里
        for s in &survivors {
            assert!(!removed.contains(s), "把 {s} 一起删掉了");
        }
    }

    /// 删不存在的版本要报错，**而且什么都不动**
    #[test]
    fn removing_a_missing_version_changes_nothing() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (_tmp, mut c) = copy_of(&root);
        let before = c.machine("A1").unwrap().to_toml();
        let n = c.machine("A1").unwrap().versions.len();

        let e = c
            .machine_mut("A1")
            .unwrap()
            .remove_version("NOPE")
            .expect_err("该报错");
        assert!(
            e.message.contains("NOPE") && e.message.contains("A1"),
            "{}",
            e.message
        );
        assert_eq!(c.machine("A1").unwrap().versions.len(), n);
        assert_eq!(c.machine("A1").unwrap().to_toml(), before, "文本被动了");
    }

    /// **允许删到零版本。** 零版本机型是合法的中间状态（刚建出来的就是），
    /// 在这一层拦住等于把一个可恢复的状态说成非法 —— 生成那一步才该拦
    #[test]
    fn removing_the_last_version_is_allowed_and_leaves_a_readable_file() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (tmp, mut c) = copy_of(&root);
        // A2L 实测只有一个版本
        let only = c.machine("A2L").unwrap().versions[0].id.clone();
        assert_eq!(c.machine("A2L").unwrap().versions.len(), 1, "前提变了");

        c.machine_mut("A2L")
            .unwrap()
            .remove_version(&only)
            .expect("该允许");
        c.write_machine("A2L").expect("写回");

        // 关键：**删空之后文件还得读得回来**（不能留下一个坏掉的 TOML）
        let c2 = Catalog::load_from(tmp.path()).expect("删空之后整个目录还要读得通");
        let m = c2.machine("A2L").expect("机型本身还在");
        assert!(m.versions.is_empty());
        assert_eq!(m.display, "A2L", "别的字段被带走了");
    }

    /// 加一个版本 = 往机型文件里插一个 `[[versions]]` 块。
    ///
    /// **三条一起断言**：新版本在、别的版本一个没动、**除了插进去的那一段别处逐字节不变**。
    /// 只断言"新版本在"的话，一个把整个文件重排了的实现也会通过 ——
    /// 而那种实现会让每次保存的 diff 变成一片红
    #[test]
    fn adding_a_version_inserts_one_block_and_touches_nothing_else() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (_tmp, mut c) = copy_of(&root);
        let before = c.machine("A1").unwrap().to_toml();
        let before_ids: Vec<String> = c
            .machine("A1")
            .unwrap()
            .versions
            .iter()
            .map(|v| v.id.clone())
            .collect();

        c.machine_mut("A1")
            .unwrap()
            .add_version("TEST_NEW", "我的新版本")
            .expect("加得上");
        c.write_machine("A1").expect("写回");

        // 从盘上重读 —— 断言的是**落盘的结果**，不是内存里的样子
        let c2 = Catalog::load_from(_tmp.path()).expect("重读");
        let m = c2.machine("A1").unwrap();
        assert_eq!(m.versions.len(), before_ids.len() + 1);
        let v = m.versions.last().unwrap();
        assert_eq!(v.id, "TEST_NEW");
        assert_eq!(v.name, "我的新版本");
        // 四个可选字段**留空不写**，不是写成空串
        assert!(v.preset_file.is_none() && v.tag.is_none() && v.description.is_none());

        // 原有版本原封不动
        let now_ids: Vec<String> = m.versions.iter().map(|v| v.id.clone()).collect();
        assert_eq!(
            &now_ids[..before_ids.len()],
            &before_ids[..],
            "原有版本被动了"
        );

        // **只动一处**：什么都没删，插进去的那一段里有新版本的 id 与名字
        let (removed, inserted) = one_edit_only(&before, &m.to_toml());
        assert!(
            removed.trim().is_empty(),
            "加一个版本却删掉了东西：{removed:?}"
        );
        assert!(
            inserted.contains("TEST_NEW"),
            "插入段不含新 id：{inserted:?}"
        );
        assert!(inserted.contains("我的新版本"));
    }

    /// 四条拒绝路径。**每一条都要有自己的话**，不能都回一句「参数不对」
    #[test]
    fn adding_a_version_refuses_bad_input_with_a_reason() {
        let Some((root, _c)) = catalog() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let (_tmp, mut c) = copy_of(&root);
        let m = c.machine_mut("A1").unwrap();

        let empty_id = m.add_version("", "名字").expect_err("空 ID 该被拒");
        let empty_name = m.add_version("OK_ID", "").expect_err("空名字该被拒");
        let bad_chars = m.add_version("有中文", "名字").expect_err("非法字符该被拒");
        let dup = m
            .add_version("STANDARD", "名字")
            .expect_err("重复 ID 该被拒");

        for (what, e) in [
            ("空 ID", &empty_id),
            ("空名字", &empty_name),
            ("非法字符", &bad_chars),
            ("重复", &dup),
        ] {
            assert!(!e.message.is_empty(), "{what} 没给出原因");
        }
        assert!(dup.message.contains("STANDARD"), "重复要点名是哪个 ID");
        assert_ne!(empty_id.message, empty_name.message, "两种错说了同一句话");

        // 一条都没加进去：**拒绝要在改动之前**，不能先加后回滚
        assert_eq!(m.versions.len(), 3, "被拒的调用改动了状态");
        assert!(!m.to_toml().contains("OK_ID"));
    }
}
