//! 「资产库」的后端：读 `presets/assets.toml`（资产域①层，b05 Task 8）。
//!
//! # 读之外有了「导入」（b05 Task 15.4）
//!
//! `Assets::add` / `write` 早就在数据层就位，接上它要有界面（Task 14 的纪律）。
//! 现在界面归 Task 17，而**「导入一张图」这个动作本身先把命令层建出来** ——
//! 空白初始化的那台设备上，第一张图与第一条 BBS 预设只能靠它进来
//! （建套餐又要先有 BBS，见 [`super::bundles::add_bundle`]）。
//!
//! 导入 = **复制文件进资产根 + 登记一条定义**，两件事都做完才算数：
//! 只登记不复制，条目会永远 `present: false`；只复制不登记，文件是谁都看不见的孤儿。
//!
//! 删除（连同反查守卫）已经在数据层（[`Presets::remove_asset`]），
//! 但**没有命令** —— 写命令一律要有界面，删除入口归 Task 17。
//!
//! # `present` 为什么现在就报
//!
//! Task 9 之前它**普遍是 false**（条目与文件一起在那边落地）。这不是错误值：
//! 界面要能说出"这一条登记了、文件还没搬"，而不是显示一张空图让人猜。
//!
//! # URL 由后端给，前端负责编码
//!
//! `url` 是 `/assets/<path>`（vite 的 `public/` 直通）。**路径里可能有空格**
//! （实测 BBS 文件名就是 `MKPProcess A1 0.2 0.10.json`），所以前端用它之前要
//! `encodeURI` —— 前缀只有这一处，别在 TSX 里再拼一遍。

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::resolve_in;
use crate::ipc::traced;
use crate::workbench::paths;
use crate::workbench::presets::{Asset, AssetKind, Presets};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetView {
    pub id: String,
    pub kind: AssetKind,
    /// 归属机型；不属于任何机型时是 `null`
    pub machine_id: Option<String>,
    pub name: String,
    /// 相对资产根的一段
    pub path: String,
    /// 前端可直接用的 URL（**用之前 encodeURI**）
    pub url: String,
    pub slicer: Option<String>,
    pub profile: Option<String>,
    /// 文件在不在。**Task 9 之前普遍 `false`** —— 那是还没搬，不是错
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetList {
    pub assets: Vec<AssetView>,
    /// 资产根的绝对路径，显示在状态条上
    pub root: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUsageView {
    pub id: String,
    /// 直接引用它的机型（`image` / `icon` 字段写着这个 id）
    pub machines: Vec<String>,
    /// 引用它的套餐（`assetRefs` 写着这个 id 的套餐 id，b05 Task 10 起）。
    /// **归属不是引用**：`p1s-icon` 归 P1S，但借它当图标的另有其人
    pub bundles: Vec<String>,
}

/// **谁在用它**（b05 Task 9.4）。删资产之前先问这一条 ——
/// 删掉一张还被机型引用着的图，界面上只表现为"那台机型的图没了"
#[tauri::command]
pub fn wb_asset_usage(asset_id: String) -> Result<AssetUsageView, AppError> {
    traced("wb_asset_usage", |_| {
        let presets = Presets::load()?;
        let usage = presets.asset_usage(&asset_id)?;
        Ok(AssetUsageView {
            id: asset_id.clone(),
            machines: usage.machines,
            bundles: usage.bundles,
        })
    })
}

/// 资产库清单。**只读**
#[tauri::command]
pub fn wb_assets() -> Result<AssetList, AppError> {
    traced("wb_assets", |_| list_of(&Presets::load()?))
}

fn list_of(presets: &Presets) -> Result<AssetList, AppError> {
    let root = paths::assets_root()?;
    let assets = presets
        .assets
        .items()
        .iter()
        .map(|a| AssetView {
            id: a.id.clone(),
            kind: a.kind,
            machine_id: a.machine_id.clone(),
            name: a.name.clone(),
            path: a.path.clone(),
            url: format!("/assets/{}", a.path),
            slicer: a.slicer.clone(),
            profile: a.profile.clone(),
            present: presets.assets.present(a),
        })
        .collect();
    Ok(AssetList {
        assets,
        root: root.display().to_string(),
    })
}

/* ---------- 导入（b05 Task 15.4） ---------- */

/// 一次导入的请求。**源文件只读** —— 它是人在磁盘上挑的，
/// 导入做的是「复制进资产根 + 登记一条定义」，不移动也不改原件。
#[derive(Debug, Clone)]
pub struct ImportRequest {
    /// 源文件的绝对路径（界面从文件对话框拿）
    pub source: PathBuf,
    pub id: String,
    pub kind: AssetKind,
    pub name: String,
    /// 归属机型。**给了就必须是真机型**（见 [`import_asset`] 第 ② 条）
    pub machine_id: Option<String>,
    /// 只给 `slicerProfile`：`slicer`（G-3 的开放维度，今天只有 `bbs`）与档位
    pub slicer: Option<String>,
    pub profile: Option<String>,
}

/// 领域体：导入一份资产。**资产根由调用方给**（判据用临时目录）。
///
/// # 四条先查再写
///
/// 这里的每一条失败都会让**下一次加载起不来**，而不是报一次错就完 ——
/// 所以它们必须在写之前拦下，与 [`super::Presets::apply_values`] 同一条理由：
///
/// 1. **源文件要是一个真文件**：路径不存在 / 是个目录 / 读不出来，都当场说清；
/// 2. **归属机型要是真机型**：`machineId` 写错一个字母，界面上只表现为
///    「这台机型的图没了」，而加载期的 `check_against_machines` 会把整个
///    `presets/` 判成 Corrupted；
/// 3. **id 不许撞**：id 是主键（大小写不敏感），撞了等于改掉别人那条；
/// 4. **落点不许覆盖**（见 [`target_path`]）：同名文件加序号避让，绝不覆盖已有资产。
///
/// # 顺序：先复制，再登记
///
/// 定义写不进去时把刚复制的那个文件删掉回滚 —— 只复制不登记会留下一个谁也
/// 看不见的孤儿文件，那种脏没有任何判据能描述它。反过来（登记了但文件没复制）
/// 是 `present: false`，那是**已知且会报出来**的状态（`wb_assets` 说、Task 11.1
/// 会升成一条 warning），所以回滚那一步只针对孤儿。
pub fn import_asset(p: &mut Presets, root: &Path, req: &ImportRequest) -> Result<Asset, AppError> {
    // ① 源文件
    if !req.source.is_file() {
        return Err(
            AppError::not_found(format!("源文件不是文件：{}", req.source.display()))
                .with_detail("给一个真实文件的路径（目录、不存在的路径、通配符都不算）"),
        );
    }

    // ② 归属机型必须是真机型
    let machine = req
        .machine_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(m) = machine {
        if p.catalog.machine(m).is_none() {
            return Err(AppError::not_found(format!("没有机型 {m}")).with_detail(
                "资产条目的 machineId 必须是真机型（清单在 presets/machines/*.toml）；\
                 不属于任何机型时这一项留空"
                    .to_owned(),
            ));
        }
    }

    // ③ id 不许撞（大小写不敏感）
    if p.assets.get(&req.id).is_some() {
        return Err(
            AppError::invalid_argument(format!("资产 id 已经存在：{}", req.id)).with_detail(
                "id 是主键，且大小写不敏感 —— 换一个 id，或先在资产库里删掉原来那条".to_owned(),
            ),
        );
    }

    // ④ 落点：`<kind.dir()>/<源文件名>`，同名按序号避让，绝不覆盖
    let rel = target_path(root, &req.source, req.kind)?;
    let dest = resolve_in(root, &rel)?;
    let bytes = std::fs::read(&req.source).map_err(|e| {
        AppError::io(format!("读不出源文件：{}", req.source.display())).with_detail(e.to_string())
    })?;

    let asset = Asset {
        id: req.id.trim().to_owned(),
        kind: req.kind,
        machine_id: machine.map(str::to_owned),
        name: req.name.trim().to_owned(),
        path: rel,
        slicer: req
            .slicer
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
        profile: req
            .profile
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned),
    };

    atomic_write(&dest, &bytes)?;
    // 登记失败 → 回滚那个文件（它是我们刚建出来的，删掉不会伤到已有数据）
    if let Err(e) = p.assets.add(asset.clone()).and_then(|()| p.assets.write()) {
        let _ = std::fs::remove_file(&dest);
        return Err(e);
    }
    tracing::info!(asset = %asset.id, path = %asset.path, "导入了一份资产");
    Ok(asset)
}

/// 资产根下的落点：**相对路径**，形如 `printers/a1.webp`。
///
/// 子目录按类型定（[`AssetKind::dir`]，与真数据那 21 条的布局一致），文件名沿用
/// 源文件名 —— 文件名里可以带空格（真数据里 BBS 就是 `MKPProcess A1 0.4 0.20.json`）。
///
/// **同名不覆盖**：已存在就加 `-2` / `-3`…（插在扩展名之前）。覆盖一份已有资产是
/// 不可逆的，而「导入了两张同名的图」完全可以用两个名字表达清楚。
fn target_path(root: &Path, source: &Path, kind: AssetKind) -> Result<String, AppError> {
    let file = source
        .file_name()
        .and_then(|s| s.to_str())
        .map(str::to_owned)
        .ok_or_else(|| {
            AppError::invalid_argument(format!("取不到文件名：{}", source.display()))
                .with_detail("给一个以文件名结尾的路径（目录本身没有文件名）")
        })?;
    if file.trim().is_empty() {
        return Err(AppError::invalid_argument(format!(
            "源文件没有文件名：{}",
            source.display()
        )));
    }

    let first = format!("{}/{file}", kind.dir());
    if !resolve_in(root, &first)?.exists() {
        return Ok(first);
    }
    for n in 2..1000 {
        let candidate = format!("{}/{}", kind.dir(), numbered(&file, n));
        if !resolve_in(root, &candidate)?.exists() {
            return Ok(candidate);
        }
    }
    Err(AppError::invalid_argument(format!(
        "{file} 在资产根里已经有 999 个同名文件了 —— 换个文件名再来"
    )))
}

/// `a1.webp` → `a1-2.webp`；没有扩展名时 `a1` → `a1-2`
fn numbered(file: &str, n: usize) -> String {
    let p = Path::new(file);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or(file);
    match p.extension().and_then(|s| s.to_str()) {
        Some(ext) => format!("{stem}-{n}.{ext}"),
        None => format!("{stem}-{n}"),
    }
}

/// **导入一份资产**（b05 Task 15.4）：复制文件进资产根 + 登记一条定义，
/// 然后**从盘上重读**再返回 —— 界面看到的必须是落盘的结果。
#[tauri::command]
pub fn wb_import_asset(
    source: String,
    id: String,
    kind: AssetKind,
    name: String,
    machine_id: Option<String>,
    slicer: Option<String>,
    profile: Option<String>,
) -> Result<AssetList, AppError> {
    traced("wb_import_asset", |_| {
        let root = paths::assets_root()?;
        let req = ImportRequest {
            source: PathBuf::from(source),
            id,
            kind,
            name,
            machine_id,
            slicer,
            profile,
        };
        let mut p = Presets::load()?;
        import_asset(&mut p, &root, &req)?;
        // 重读盘：定义与文件都真的在，才算导入完成
        list_of(&Presets::load()?)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::testkit::Fixture;

    /// 一台"设备"：资产根（临时）+ 源文件（临时，资产根**之外**）+ 夹具那份 presets。
    /// 三个临时目录都得活到测试结束，所以一起交回来（第三个是夹具目录 ——
    /// presets 会往 `presets/assets.toml` 里写）
    fn import_env() -> (
        tempfile::TempDir,
        tempfile::TempDir,
        tempfile::TempDir,
        Presets,
    ) {
        let root = tempfile::tempdir().expect("资产根");
        let outside = tempfile::tempdir().expect("源文件目录");
        let (fx_dir, _up, presets) = Fixture::load().into_parts();
        (root, outside, fx_dir, presets)
    }

    fn req(source: PathBuf, id: &str, kind: AssetKind) -> ImportRequest {
        ImportRequest {
            source,
            id: id.to_owned(),
            kind,
            name: format!("{id} 的名字"),
            machine_id: Some("A1".to_owned()),
            slicer: (kind == AssetKind::SlicerProfile).then_some("bbs".to_owned()),
            profile: (kind == AssetKind::SlicerProfile).then_some("process".to_owned()),
        }
    }

    fn files_under(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    out.push(p.strip_prefix(root).unwrap_or(&p).display().to_string());
                }
            }
        }
        out.sort();
        out
    }

    /// **导入 = 复制文件 + 登记定义**，两件事都真的落到盘上（b05 Task 15.4）。
    ///
    /// 判据看的是盘：落点文件内容与源文件逐字节相同、定义重读之后条目在。
    /// 只看内存的话等于什么都没验
    #[test]
    fn importing_a_file_copies_it_and_registers_it() {
        let (root, outside, _fx, mut p) = import_env();
        let src = outside.path().join("a1 shot.webp");
        atomic_write(&src, b"image-bytes").expect("源文件");

        let a = import_asset(&mut p, root.path(), &req(src, "a1-shot", AssetKind::Image))
            .expect("导入");
        assert_eq!(
            a.path, "printers/a1 shot.webp",
            "落点：类型子目录 + 源文件名"
        );
        assert_eq!(a.kind, AssetKind::Image);
        assert_eq!(
            std::fs::read(root.path().join(&a.path)).expect("落点文件在"),
            b"image-bytes".to_vec(),
            "复制过去的内容要与源文件逐字节相同"
        );

        // 定义也落盘了（重读盘，不看内存）
        let again = Presets::load_from(p.root()).expect("导入后的 presets 仍读得通");
        let got = again.assets.get("a1-shot").expect("条目在定义里");
        assert_eq!(got.path, a.path);
        assert_eq!(got.machine_id.as_deref(), Some("A1"));
    }

    /// **同名不覆盖**：第二次导入同一个文件名落在 `-2`，第一份一个字节不动。
    ///
    /// 覆盖一份已有资产是不可逆的 —— 「导入了两张同名的图」完全可以用两个名字表达
    #[test]
    fn importing_the_same_file_name_twice_never_overwrites() {
        let (root, outside, _fx, mut p) = import_env();
        let src = outside.path().join("a1.webp");
        atomic_write(&src, b"first").expect("源文件");

        let one = import_asset(
            &mut p,
            root.path(),
            &req(src.clone(), "a1-shot", AssetKind::Image),
        )
        .expect("第一次");
        let two = import_asset(
            &mut p,
            root.path(),
            &req(src, "a1-shot-2", AssetKind::Image),
        )
        .expect("第二次");

        assert_eq!(one.path, "printers/a1.webp");
        assert_eq!(two.path, "printers/a1-2.webp", "同名按序号避让");
        assert_eq!(
            std::fs::read(root.path().join(&one.path)).unwrap(),
            b"first".to_vec(),
            "第一份一个字节不动"
        );
        assert_eq!(
            files_under(root.path()).len(),
            2,
            "盘上是两份，不是一份被覆盖"
        );
    }

    /// 四条先查再写，**被拦下时一个字节都不写**（源文件不是文件 / 假机型 / id 撞）
    #[test]
    fn a_refused_import_writes_nothing() {
        let (root, outside, _fx, mut p) = import_env();
        let src = outside.path().join("a1.webp");
        atomic_write(&src, b"x").expect("源文件");

        // ① 源不是文件（目录 / 不存在）
        let err = import_asset(
            &mut p,
            root.path(),
            &req(outside.path().to_path_buf(), "a1-shot", AssetKind::Image),
        )
        .expect_err("目录不是文件");
        assert!(err.message.contains("不是文件"), "实测：{}", err.message);
        let missing = outside.path().join("no-such.webp");
        import_asset(
            &mut p,
            root.path(),
            &req(missing, "a1-shot", AssetKind::Image),
        )
        .expect_err("不存在的路径必须被拦");

        // ② 假机型（写了它下一次加载整个 presets/ 起不来）
        let mut bad = req(src.clone(), "a1-shot", AssetKind::Image);
        bad.machine_id = Some("NO_SUCH".to_owned());
        let err = import_asset(&mut p, root.path(), &bad).expect_err("假机型必须被拦");
        assert!(err.message.contains("没有机型"), "实测：{}", err.message);

        // ③ id 撞（夹具里已有 a1-image）
        let dup = req(src, "a1-image", AssetKind::Image);
        let err = import_asset(&mut p, root.path(), &dup).expect_err("撞 id 必须被拦");
        assert!(err.message.contains("已经存在"), "实测：{}", err.message);

        assert!(
            files_under(root.path()).is_empty(),
            "被拦下就不该往资产根里写任何东西，实测写了：{:?}",
            files_under(root.path())
        );
    }

    /// **回填会不会误删原有文件**（只判回滚的那个动作）。
    ///
    /// 这是全文件唯一一处 `remove_file`，而它删的是刚刚 `atomic_write` 出来的那个
    /// 落点**。要证明它伤不到别人的东西，就得让落点旁边站着一个同名的老文件：
    /// `printers/a1.webp` 先有了（别人的图），再导入一份同名的、且**登记必然失败**
    /// 的资产（`name` 留空 → `Assets::add` 的 `check_one` 拒收）——
    /// 落点会避让到 `a1-2.webp`，复制这一步确实发生过，于是回滚那条分支被真正走到。
    ///
    /// 判据要同时说出三件事：①刚复制的那个没了；②`a1.webp` 一个字节没动；
    /// ③定义里没有多出条目。
    #[test]
    fn a_failed_registration_rolls_back_only_the_file_it_wrote() {
        let (root, outside, _fx, mut p) = import_env();
        let victim = root.path().join("printers/a1.webp");
        atomic_write(&victim, b"someone-elses-bytes").expect("先放一份别人的图");
        let before = p.assets.items().len();

        let src = outside.path().join("a1.webp");
        atomic_write(&src, b"mine").expect("源文件");
        let mut req = req(src, "a1-shot", AssetKind::Image);
        req.name = String::new(); // 登记必然失败：`name` 是界面上要显示的东西

        let err = import_asset(&mut p, root.path(), &req).expect_err("登记失败必须上报");
        assert!(err.message.contains("name"), "实测：{}", err.message);

        assert_eq!(
            std::fs::read(&victim).expect("别人的图还在"),
            b"someone-elses-bytes".to_vec(),
            "同名老文件一个字节都不许动"
        );
        assert!(
            !root.path().join("printers/a1-2.webp").exists(),
            "落点（a1-2.webp）是本次刚写的，回滚要把它删掉 —— 留下就是孤儿文件"
        );
        assert_eq!(
            Presets::load_from(p.root())
                .expect("失败之后 presets 仍读得通")
                .assets
                .items()
                .len(),
            before,
            "没登记成功就不该多出条目"
        );
    }

    /// **真数据上的一条**：条目数与文件都在（b05 Task 9 的验收）。
    ///
    /// 它比 `presets::tests` 那条多看一层：命令返回的 DTO 是不是也对
    /// （`present`、`url` 前缀、类型都在）。
    #[test]
    fn the_real_asset_list_is_complete_and_present() {
        let list = wb_assets().expect("真 presets 读得通");
        assert_eq!(
            list.assets.len(),
            21,
            "机型图 6 + 图标 3 + 模型 3 + BBS 9 —— 条数变了就该在提交里说清为什么"
        );

        let missing: Vec<&str> = list
            .assets
            .iter()
            .filter(|a| !a.present)
            .map(|a| a.id.as_str())
            .collect();
        assert!(missing.is_empty(), "这些资产的文件不在：{missing:?}");

        assert!(
            list.assets.iter().any(|a| a.kind == AssetKind::Model),
            "模型这一类要在（裁决：保留 model 类型）"
        );
        assert!(
            list.assets.iter().all(|a| a.url.starts_with("/assets/")),
            "URL 前缀只有后端一处，别在 TSX 里再拼"
        );
        assert!(
            list.assets.iter().all(|a| !a.name.trim().is_empty()),
            "每条都要有给人看的名字"
        );
    }

    /// 反查在真数据上也说得清是谁在用（b05 Task 9.4）
    #[test]
    fn the_real_usage_lookup_names_the_machines() {
        let u = wb_asset_usage("p1s-icon".to_owned()).expect("反查");
        assert_eq!(
            u.machines,
            vec!["P1S".to_owned(), "P2S".to_owned(), "X1C".to_owned()],
            "三个机型共用这一份图标（归属写 P1S，借用的是另外两台）"
        );
        assert!(u.bundles.is_empty(), "图标不是套餐的配发内容");

        // 机型图各归各的
        let u = wb_asset_usage("a1-image".to_owned()).expect("反查");
        assert_eq!(u.machines, vec!["A1".to_owned()]);
    }
}
