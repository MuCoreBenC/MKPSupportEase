//! **备份与恢复**（2026-10-17）：一个 zip = 这台机器上 SupportEase 的**全部家当**。
//!
//! # 备份里有什么（作者：「备份的文件需要齐全，足够，包括设置的那些，catalog 之类，
//!   toml，json」）
//!
//! 全部路径相对**内部根**（`appDataDir`）：
//!
//! ```text
//! backup-info.json          这一份备份的说明书（代次 / 时间 / 条目数）
//! catalog.json              目录（首屏唯一数据源）
//! run/app-state.json        应用状态：主题、数据源、正在用哪一份……
//! run/preset-history/*.json **修改历史**（2026-10-17：一份文件一本，git 式）
//! baseline/*.toml           云端各版的基准快照（「恢复默认」的摘库）
//! baseline-local/**         原文镜像（这份用户文件自己最初的样子）
//! user/presets-local/**     **用户自己的预设**（全部 toml，含导入进来的）
//! user/gcode_history/**     后处理执行归档（三件套）
//! ```
//!
//! **不进备份**：`cloud/`（云端下载区的原件 —— 重新下载即可）、`archive/`（换代归档）、
//! `logs/`（日志）、`user/exports` 与 `user/reports`（生成的产物，可重出）。
//! 备份要的是"丢了就再也回不来的东西"，不是"缓存"。
//!
//! # 恢复：**全量覆盖**（作者认可的第一版口径）
//!
//! 作者问「要不要选择性恢复」。这里不做，理由与成熟方案的选择一致：
//!
//! - **选择性恢复的难点是"同名文件怎么选"** —— 同名的可能是同一份的新旧版本，
//!   也可能是不同机器上的不同文件，要可靠区分只能靠血统 + SHA + 时间戳做冲突解决 UI，
//!   那是另外一整个界面；
//! - **成熟方案也是这两条路**：要么整机快照（Time Machine、手机厂商备份 —— 全量恢复，
//!   靠"多份快照按时间选"拿到细粒度），要么细粒度同步（git / VS Code Settings Sync ——
//!   但那要求每个文件都有稳定 id 与合并语义）。我们走前者：
//!   **多留几份备份 = 想回到哪个时间点就选哪一份**。
//!
//! 于是恢复的语义钉死为：**备份里有的一律覆盖；被管的那几棵树里、备份里没有的删掉**
//! （= 整棵换掉，回到"那一份备份那一刻"的样子）。
//!
//! # 恢复进去的文件与云端下的一样吗？还能否支持后续更新？
//!
//! 一样 —— 备份存的是**字节原样**（含血统三行 `based_on`），恢复就是原样放回去。
//! 后续更新照常：更新判据走的是血统与目录对齐那条正路（`local.revision != remote.revision`、
//! 随包资产兜底），恢复回来的文件带着自己的血统与 SHA，进预设页照常参与对齐。

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;

/// 备份格式的代次。加条目不升号，改语义才升
pub const BACKUP_SCHEMA: u32 = 1;

/// 备份里那份说明书的名字（恢复时第一个读它 —— 不是我们的备份就当场拒）
const INFO_FILE: &str = "backup-info.json";

/// 被管的**整棵树**（相对内部根）：恢复时这几棵**整棵换掉**
const MANAGED_DIRS: [&str; 5] = [
    "run/preset-history",
    "baseline",
    "baseline-local",
    "user/presets-local",
    "user/gcode_history",
];

/// 被管的**单份文件**（相对内部根）：有就覆盖
const MANAGED_FILES: [&str; 2] = ["catalog.json", "run/app-state.json"];

/// 备份里那份说明书
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub schema: u32,
    /// 打这份备份的客户端版本（给人看；不是兼容性判据 —— 代次才是）
    pub app_version: String,
    /// 什么时候打的（UTC epoch 秒）
    pub created_unix: i64,
    /// 里面一共几个文件
    pub files: usize,
}

/// 打完 / 恢复完的回执
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    /// **备份文件自己的绝对路径**（界面照原样显示给用户：他要知道东西落在哪了）
    pub path: String,
    pub files: usize,
    pub bytes: u64,
    pub created_unix: i64,
}

/// 备份文件名：`SupportEase-备份-20261017-153000.zip`。
///
/// 名字里带时刻，于是**同一天打两份不会互相盖掉** —— "多留几份"是这套方案的细粒度。
fn backup_name(created: i64) -> String {
    let t = OffsetDateTime::from_unix_timestamp(created).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    format!(
        "SupportEase-备份-{:04}{:02}{:02}-{:02}{:02}{:02}.zip",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// 一条相对路径进不进得了备份（也进不进得了恢复的目标）：**不许穿出根**。
///
/// 与用户文件那一族同一条纪律：名字是用户能改的东西，拼路径之前先过闸。
fn safe_rel(name: &str) -> Result<&str, AppError> {
    let path = Path::new(name);
    if path.is_absolute() {
        return Err(AppError::invalid_argument(format!(
            "备份里有一条绝对路径：{name}"
        )));
    }
    for part in path.components() {
        match part {
            Component::Normal(_) => {}
            _ => {
                return Err(AppError::invalid_argument(format!(
                    "备份里有一条不老实的路径：{name}"
                )))
            }
        }
    }
    Ok(name)
}

/// 一棵树里所有**文件**（相对内部根；目录不进 zip —— 解包时按文件路径自动建）
fn files_under(internal: &Path, rel_dir: &str, out: &mut Vec<String>) -> Result<(), AppError> {
    let dir = internal.join(rel_dir);
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in walk(&dir)? {
        let at = entry?;
        if at.is_file() {
            let rel = at
                .strip_prefix(internal)
                .map_err(|_| AppError::internal("备份时算不出相对路径"))?;
            out.push(rel.display().to_string().replace('\\', "/"));
        }
    }
    Ok(())
}

/// 递归列目录（不引新依赖：`walkdir` 那一档不值得为这一处进树）
fn walk(dir: &Path) -> Result<Vec<std::io::Result<PathBuf>>, AppError> {
    let mut out: Vec<std::io::Result<PathBuf>> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let entries = std::fs::read_dir(&at).map_err(|e| {
            AppError::io(format!("读不了目录：{}", at.display())).with_detail(e.to_string())
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| AppError::io("列目录失败").with_detail(e.to_string()))?;
            let at = entry.path();
            if at.is_dir() {
                stack.push(at);
            } else {
                out.push(Ok(at));
            }
        }
    }
    Ok(out)
}

/// **打一份备份**：把上面那些树与文件收进 zip，落在 `dest_dir`。
///
/// `dest_dir` 是用户挑的文件夹（界面那侧用系统文件选择框要的），文件名由这里定 ——
/// 用户不需要想它叫什么。
pub fn create(internal: &Path, dest_dir: &Path) -> Result<BackupSummary, AppError> {
    let created_unix = OffsetDateTime::now_utc().unix_timestamp();
    let mut names: Vec<String> = Vec::new();
    for f in MANAGED_FILES {
        if internal.join(f).is_file() {
            names.push(f.to_owned());
        }
    }
    for d in MANAGED_DIRS {
        files_under(internal, d, &mut names)?;
    }
    names.sort();

    let dest = dest_dir.join(backup_name(created_unix));
    let info = BackupInfo {
        schema: BACKUP_SCHEMA,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_unix,
        files: names.len(),
    };
    let info_bytes = serde_json::to_vec_pretty(&info)?;

    /*
     * 写盘仍走全仓唯一出口（`fsx::atomic`）—— 只是走**流式**那一档：
     * 备份可能几百 MB（执行归档三件套），不该先整个进内存再落。
     */
    crate::fsx::atomic::atomic_write_stream(&dest, |file| {
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file(INFO_FILE, opts).map_err(zip_err)?;
        zip.write_all(&info_bytes).map_err(io_err)?;
        for name in &names {
            safe_rel(name)?;
            zip.start_file(name.as_str(), opts).map_err(zip_err)?;
            let mut reader = std::fs::File::open(internal.join(name)).map_err(|e| {
                AppError::io(format!("读不了要备份的文件：{name}")).with_detail(e.to_string())
            })?;
            std::io::copy(&mut reader, &mut zip).map_err(|e| {
                AppError::io(format!("写不进备份：{name}")).with_detail(e.to_string())
            })?;
        }
        zip.finish().map_err(zip_err)?;
        Ok(())
    })?;

    let bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    Ok(BackupSummary {
        path: dest.display().to_string(),
        files: names.len(),
        bytes,
        created_unix,
    })
}

/// **从一份备份恢复**（全量覆盖）。
///
/// 三步：① 认说明书（不是我们的备份 / 代次读不懂 ⇒ 拒，一个字都不动）；
/// ② 把被管的几棵树整棵删掉；③ 逐条解开覆盖回去。
///
/// ★ 先认说明书再动手 —— 中途报错会留下"恢复了一半"的现场，越早拒越好。
pub fn restore(internal: &Path, zip_path: &Path) -> Result<BackupSummary, AppError> {
    let file = std::fs::File::open(zip_path).map_err(|e| {
        AppError::not_found(format!("读不到这份备份：{}", zip_path.display()))
            .with_detail(e.to_string())
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_err)?;

    /* ① 认说明书 */
    let info_text = {
        let mut f = archive
            .by_name(INFO_FILE)
            .map_err(|_| AppError::corrupted("这不是一份 SupportEase 备份（里面没有说明书）"))?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(io_err)?;
        s
    };
    let info: BackupInfo = serde_json::from_str(&info_text)
        .map_err(|e| AppError::corrupted("这份备份的说明书读不出来").with_detail(e.to_string()))?;
    if info.schema != BACKUP_SCHEMA {
        return Err(AppError::corrupted(format!(
            "这份备份是第 {} 代的格式，这个版本读不懂（先升级 SupportEase）",
            info.schema
        )));
    }

    /* ② 整棵换掉（"回到那一刻"的语义） */
    for d in MANAGED_DIRS {
        let at = internal.join(d);
        if at.is_dir() {
            std::fs::remove_dir_all(&at).map_err(|e| {
                AppError::io(format!("清不掉旧的那棵：{d}")).with_detail(e.to_string())
            })?;
        }
    }

    /* ③ 逐条解开 */
    let mut count = 0usize;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err)?;
        let name = entry.name().to_owned();
        if name == INFO_FILE || entry.is_dir() {
            continue;
        }
        let rel = safe_rel(&name)?;
        let target = internal.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(format!("建不了目录：{}", parent.display())).with_detail(e.to_string())
            })?;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(io_err)?;
        atomic_write(&target, &bytes)?;
        count += 1;
    }

    Ok(BackupSummary {
        path: zip_path.display().to_string(),
        files: count,
        bytes: std::fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0),
        created_unix: info.created_unix,
    })
}

/// 读写出错（zip 里的条目也是普通 IO）—— 与下面的"格式读不动"分开：
/// 前者是"这份盘 / 这次写入有问题"，后者是"这根本不是一份能读的 zip"
fn io_err(e: std::io::Error) -> AppError {
    AppError::io("备份读写出错").with_detail(e.to_string())
}

fn zip_err(e: zip::result::ZipError) -> AppError {
    match e {
        zip::result::ZipError::Io(e) => AppError::io("备份文件读写出错").with_detail(e.to_string()),
        other => AppError::corrupted("这份 zip 读不动").with_detail(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(internal: &Path) {
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":1}").unwrap();
        atomic_write(
            &internal.join("run/app-state.json"),
            b"{\"theme\":\"dark\"}",
        )
        .unwrap();
        atomic_write(
            &internal.join("run/preset-history/abc.json"),
            b"{\"schema\":1,\"batches\":2,\"entries\":[]}",
        )
        .unwrap();
        atomic_write(
            &internal.join("user/presets-local/A1 涂胶.toml"),
            b"# machine: A1\n[toolhead]\noffset_x = -0.7\n",
        )
        .unwrap();
        /* 不该进备份的那些 */
        atomic_write(&internal.join("cloud/A1-standard.toml"), b"# cloud\n").unwrap();
        atomic_write(&internal.join("logs/app.log"), b"noise\n").unwrap();
    }

    /// 备份里**有**该有的（目录 / 状态 / 历史 / 用户 toml），**没有**可重建的那些
    #[test]
    fn a_backup_carries_everything_that_cannot_be_rebuilt() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);

        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let sum = create(&internal, &out).unwrap();

        let mut names = Vec::new();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&sum.path).unwrap()).unwrap();
        for i in 0..archive.len() {
            names.push(archive.by_index(i).unwrap().name().to_owned());
        }
        for want in [
            "backup-info.json",
            "catalog.json",
            "run/app-state.json",
            "run/preset-history/abc.json",
            "user/presets-local/A1 涂胶.toml",
        ] {
            assert!(
                names.contains(&want.to_owned()),
                "备份里该有 {want}：{names:?}"
            );
        }
        for bad in ["cloud/A1-standard.toml", "logs/app.log"] {
            assert!(!names.iter().any(|n| n == bad), "不该进备份：{bad}");
        }
        assert!(sum.path.ends_with(".zip"));
    }

    /// 同一天打两份不会互相盖掉（名字里带时刻）—— 这是"多留几份"能成立的前提
    #[test]
    fn two_backups_in_the_same_minute_do_not_overwrite_each_other() {
        assert_ne!(backup_name(1_700_000_000), backup_name(1_700_000_060));
    }

    /// **恢复 = 全量覆盖**：坏掉的现状被换回备份那一刻的样子，
    /// 且**备份里没有的那一份会被删掉**（整棵换掉才是"回到那一刻"）
    #[test]
    fn restoring_puts_back_what_the_backup_had_and_drops_the_rest() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out).unwrap().path
        };

        /* 现状被搞坏 / 多了东西 */
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":9}").unwrap();
        atomic_write(
            &internal.join("user/presets-local/多出来的.toml"),
            "# 后来加的\n".as_bytes(),
        )
        .unwrap();

        let sum = restore(&internal, std::path::Path::new(&zip_at)).unwrap();
        assert!(sum.files >= 4);

        let catalog = std::fs::read_to_string(internal.join("catalog.json")).unwrap();
        assert!(
            catalog.contains("\"catalogSchema\":1"),
            "覆盖回备份那一刻：{catalog}"
        );
        assert!(
            !internal.join("user/presets-local/多出来的.toml").exists(),
            "备份里没有的那一份要跟着删掉（整棵换掉）"
        );
        assert!(
            internal.join("user/presets-local/A1 涂胶.toml").is_file(),
            "备份里有的那份得回来"
        );
        assert!(
            internal.join("run/preset-history/abc.json").is_file(),
            "修改历史跟着备份一起回来"
        );
    }

    /// 不是我们的备份 ⇒ 一个字都不动（先认说明书再动手）
    #[test]
    fn a_zip_that_is_not_ours_is_refused_before_anything_is_touched() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let foreign = d.path().join("别的.zip");
        {
            let f = std::fs::File::create(&foreign).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            zip.start_file("whatever.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
        }
        let err = restore(&internal, &foreign).expect_err("该拒");
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
        assert!(
            std::fs::read_to_string(internal.join("catalog.json"))
                .unwrap()
                .contains("\"catalogSchema\":1"),
            "拒了就一个字都不许动"
        );
    }

    /// 穿出根的那种路径不许进来（名字是用户能改的东西）
    #[test]
    fn a_path_that_escapes_the_root_is_refused() {
        assert!(safe_rel("../../etc/passwd").is_err());
        assert!(safe_rel("/etc/passwd").is_err());
        assert!(safe_rel("user/presets-local/A1.toml").is_ok());
    }
}
