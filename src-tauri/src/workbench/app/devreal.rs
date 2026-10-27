//! **开发台 · 运行真实后处理**（2026-10-19）：让**真的客户端窗口**跑一趟真的管线。
//!
//! # 它与「后处理预览档」是两件事
//!
//! | | 预览档（`devsim`，①） | 这一条（②） |
//! |---|---|---|
//! | 跑没跑管线 | 没有，只是按节奏演 | **真的跑**（真读文件、真算、真落盘） |
//! | 界面上的标记 | 带「预览模式」、页脚换成「关闭预览」 | **没有任何预览标记**（`preview` 恒 false） |
//! | 那几颗真业务按钮 | 点了必然报错（钩子根本没连） | **点了真的有用**（真有一条钩子进程连着） |
//!
//! 作者 2026-10-19 的裁定：「不是实现不了真的，而是要分清楚：哪些已经实现成真功能，
//! 哪些只是开发台里的演示场景」。这一条是前者。
//!
//! # ★★ 它**不是**新协议，而是复用真机那条入口
//!
//! 真机（切片器）调我们的方式就是 **起一个带参数的子进程**：
//!
//! ```text
//! "<本程序>" --Toml "<预设.toml>" --Gcode <输入.gcode>
//! ```
//!
//! `src-tauri/src/main.rs` 按 [`crate::args`] 分流：认出这两个开关 ⇒ `Mode::Hook`
//! ⇒ `hook::run_with_channel(&job)`（真跑管线、真归档、跑完就退）。
//!
//! 所以工作台要做的只是**自己 spawn 一个这样的子进程**。那个子进程：
//!
//! - 用 [`crate::fsx::paths::internal_root_headless`] 算内部根（= **客户端那棵** `SupportEase`，
//!   与客户端、真钩子、假钩子 CLI 同一棵）；
//! - 连上客户端那条 loopback，把 `Hello` / `Progress` / `Question` / `Finished` 真发过去；
//! - 于是客户端窗口里那一屏是**产品行为本身**，连"预览"这个标记都不存在。
//!
//! ⇒ **不加 `FromHook` 变体、客户端侧一行不改、产品逻辑一行不改。**
//! （交接文档 §6 那个未验证假设，本轮验证成立。）
//!
//! # 为什么先复制一份副本再跑
//!
//! 真后处理是**原地覆盖输入**（`FinishedPayload.output` 就是输入那一份）。直接拿用户选的
//! 那份跑，第二趟跑的就是已处理过的产物 ——「能反复跑」这条判据当场失效。
//! 所以这里先复制一份到 `<客户端内部根>/run/dev-run/`，**源文件一个字节都不动**。
//!
//! # 客户端没开时：**不替用户拉进程**
//!
//! 与 `devfault` / `devsim` 同一条口径：先探（`connect_or_headless`），连不上就如实报错。
//! 钩子自己也有 `connect_or_spawn`，但那个拉起的是 `current_exe()` —— 在工作台语境下
//! 就是工作台自己（作者明确不要）。
//!
//! # ★ 工作台的测试模式（沙箱）管不到这一趟
//!
//! 沙箱开关是**模块级 `AtomicBool`**（`workbench::paths::set_sandbox`），不写环境变量，
//! 子进程不会继承 ⇒ 钩子进程走的是**客户端正式那棵树**。界面上要把这句说清楚，
//! 别让人以为切了沙箱就能安全试。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::paths;
use crate::hook_ipc::Client;
use postprocess::diag::CancelToken;

/// 副本落在客户端内部根的这一处。
///
/// 与 `run/hook.port` 同族：**进程内部产物**，不进备份、也不给用户当数据看。
/// 只增不减（与 `run/` 下其它东西一样，归 `retention` 那一套口径之外）。
pub const DEV_RUN_DIR: &str = "run/dev-run";

/// 跑起来之后的回执（界面拿它显示"这一趟在处理哪一份"）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevRunAck {
    /// **真被处理的那一份**（副本，不是用户选的原文件 —— 真后处理原地覆盖）
    pub gcode: String,
    /// 用的预设（原样，不复制 —— 预设是只读输入）
    pub preset: String,
}

/// 拼给钩子的 argv。**形态与切片器注册的那一行逐字一致。**
///
/// ★ [`crate::args`] 是**精确匹配且大小写敏感**（`--toml` 不认），所以这两个串一个
/// 字母都不能改。抽成纯函数是为了让判据咬得住"顺序与大小写" —— 拼错了的表现
/// 是"界面一动不动，而钩子其实根本没当钩子跑"。
fn hook_args(toml: &str, gcode: &str) -> Vec<String> {
    vec![
        "--Toml".to_owned(),
        toml.to_owned(),
        "--Gcode".to_owned(),
        gcode.to_owned(),
    ]
}

/// 两个输入像不像"能跑的那一对"。**只报不修** —— 替用户猜一个文件出来最糟。
fn check_input(toml: &Path, gcode: &Path) -> Result<(), AppError> {
    if !toml.is_file() {
        return Err(AppError::not_found(format!("预设不在：{}", toml.display())));
    }
    if !toml
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("toml"))
    {
        return Err(AppError::invalid_argument(format!(
            "预设要是一份 .toml：{}",
            toml.display()
        )));
    }
    if !gcode.is_file() {
        return Err(AppError::not_found(format!(
            "G-code 不在：{}",
            gcode.display()
        )));
    }
    if !gcode
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gcode"))
    {
        return Err(AppError::invalid_argument(format!(
            "测试件要是一份 .gcode：{}",
            gcode.display()
        )));
    }
    Ok(())
}

/// 把 G-code 复制一份到 [`DEV_RUN_DIR`]，返回副本落点。**源文件一字不动**。
///
/// 名字带毫秒时间戳：同一份连着跑两趟时，第二趟不该读到第一趟的产物。
fn copy_for_run(root: &Path, gcode: &Path) -> Result<PathBuf, AppError> {
    let dir = root.join(DEV_RUN_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| {
        AppError::io(format!("建不出副本目录：{}", dir.display())).with_detail(e.to_string())
    })?;
    let name = gcode
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "input.gcode".to_owned());
    let dest = dir.join(format!("{}-{}", crate::hook_ipc::unix_ms_now(), name));
    std::fs::copy(gcode, &dest).map_err(|e| {
        AppError::io(format!("复制不出副本：{}", gcode.display())).with_detail(e.to_string())
    })?;
    Ok(dest)
}

/// 连客户端那条 loopback（**客户端那棵树**，不是工作台这棵）。
///
/// 与 `devfault::to_client` 同一个坑：`app_data_dir()` 给的是工作台自己那棵
/// （`…/SupportEase-Workbench`），而客户端监听器绑在 `…/SupportEase`。发错树 = 石沉大海。
fn to_client() -> Result<Client, AppError> {
    let Some(root) = paths::internal_root_headless() else {
        return Err(AppError::internal("算不出客户端内部数据根"));
    };
    let cancel = CancelToken::new();
    let client = Client::connect_or_headless(&root, &cancel);
    if !client.connected() {
        return Err(AppError::not_found(
            "客户端窗口没启动 —— 先跑 npm run tauri dev",
        ));
    }
    Ok(client)
}

/// 起一个**真的钩子子进程**，让它把这一趟的数据落到 `data_root`。
///
/// # ★ 为什么这里要显式给数据根
///
/// 钩子那侧算根有两个入口（[`crate::fsx::paths::internal_root_headless`] 与
/// [`crate::fsx::paths::user_root_headless`]），两者都先看环境变量
/// [`crate::fsx::paths::DATA_ROOT_ENV`]（= `MKP_SUPPORT_EASE_DATA`）—— 给了就整个盖掉。
///
/// 于是**"数据落哪棵树"与"消息发给谁"被拆成了两件事**：
///
/// | | 由什么决定 |
/// | --- | --- |
/// | 产物 / 执行记录 / 日志 / 用量账 | `data_root`（本函数的第二个参数） |
/// | 消息发到哪个窗口 | `data_root/run/hook.port` 里那个端口号（普通文本文件） |
///
/// 单趟入口把 `data_root` 传成客户端内部根（= 与不设环境变量等价）；
/// 矩阵传成一个一次性的临时根，并在里面**预先写好客户端的端口号** ——
/// 这就是"数据全落临时目录、界面照样是真的"那个做法（见 `devmatrix`）。
///
/// 返回 `Child` 是为了让调用方（矩阵）能 `try_wait()` 判"这一趟还在不在跑"。
pub fn spawn_hook(
    toml: &str,
    gcode: &str,
    data_root: &Path,
) -> Result<std::process::Child, AppError> {
    let exe = std::env::current_exe()
        .map_err(|e| AppError::internal("取不到本程序的路径").with_detail(e.to_string()))?;
    /* stdio 全接 null：与 `hook_ipc::spawn_display` 同一条理由 —— 钩子那几行结论
    本来就落 `logs/`（`obs::hook_log`），不需要靠谁的终端来看 */
    std::process::Command::new(&exe)
        .args(hook_args(toml, gcode))
        .env(paths::DATA_ROOT_ENV, data_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| AppError::io("起不了那个钩子进程").with_detail(e.to_string()))
}

/// **开发台：跑一趟真的后处理**（不是演出）。
///
/// 选中一份预设与一份测试 G-code，点一下就是这一条：
///
/// ```text
///   ① 校验两个文件在不在、后缀对不对
///   ② 把 G-code 复制一份到 <客户端内部根>/run/dev-run/（源文件不动）
///   ③ 探客户端窗口在不在听（不在就如实报错，不替用户拉进程）
///   ④ spawn 本程序 --Toml <预设> --Gcode <副本>（**不等它**）
/// ```
///
/// 之后那一趟就与切片器触发的一模一样：真预检、真进度、真问句、真落盘、真归档，
/// 客户端窗口里那几颗「停止 / 后台运行 / 强制继续 / 终止」也都是真的。
///
/// ★ **它只在 workbench 构建里**（命令挂在工作台那份清单里），正式客户端编不到。
#[tauri::command(async)]
pub fn wb_dev_run_real(toml: String, gcode: String) -> Result<DevRunAck, AppError> {
    crate::ipc::traced("wb_dev_run_real", |_| {
        let toml_path = PathBuf::from(&toml);
        let gcode_path = PathBuf::from(&gcode);
        check_input(&toml_path, &gcode_path)?;

        let Some(root) = paths::internal_root_headless() else {
            return Err(AppError::internal("算不出客户端内部数据根"));
        };
        let copy = copy_for_run(&root, &gcode_path)?;
        let copy_text = copy.display().to_string();

        /* 先探客户端在不在听，再起子进程 —— 起出来的那个钩子自己也会连线，
        但连不上时它会去拉 `current_exe()`（= 工作台自己），那是我们明确不要的 */
        to_client()?;

        /* 数据根 = 客户端内部根（与不设环境变量等价）；矩阵那一侧会传临时根 */
        let _child = spawn_hook(&toml, &copy_text, &root)?;

        tracing::info!(
            preset = %toml,
            gcode = %copy_text,
            "开发台：已起一趟真实后处理（客户端窗口会显示真进度）"
        );
        Ok(DevRunAck {
            gcode: copy_text,
            preset: toml,
        })
    })
}

#[cfg(test)]
mod tests {
    // 判据要造真实输入文件 —— 生产代码的写盘走 `fsx::atomic` 那一个洞。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    fn tree(root: &Path, files: &[&str], body: &str) {
        for rel in files {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().expect("有父目录")).expect("建目录");
            std::fs::write(&p, body).expect("写文件");
        }
    }

    /// **argv 与切片器注册的那一行逐字一致** —— 大小写与顺序错了不会编译报错，
    /// 只会"钩子没当钩子跑、界面一动不动"
    #[test]
    fn the_hook_argv_matches_the_slicer_line() {
        assert_eq!(
            hook_args("/p/A1.toml", "/g/in.gcode"),
            vec!["--Toml", "/p/A1.toml", "--Gcode", "/g/in.gcode"]
        );
    }

    /// 副本落在 `run/dev-run/` 下、名字带原名，**源文件一个字节都不动**
    /// （真后处理原地覆盖 —— 不复制的话"能反复跑"这条判据当场失效）
    #[test]
    fn the_copy_lands_in_the_dev_run_dir_and_the_source_is_untouched() {
        let d = tempfile::tempdir().expect("临时根");
        let src = d.path().join("in");
        tree(&src, &["plate_1.gcode"], "ORIGINAL");
        let gcode = src.join("plate_1.gcode");

        let copy = copy_for_run(d.path(), &gcode).expect("复制");
        assert_eq!(
            copy.parent().expect("有父目录"),
            d.path().join(DEV_RUN_DIR),
            "副本要落在内部根的 run/dev-run 下"
        );
        assert!(
            copy.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with("plate_1.gcode")),
            "副本名字要带原名：{copy:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&copy).expect("读副本"),
            "ORIGINAL",
            "副本内容与源一致"
        );
        assert_eq!(
            std::fs::read_to_string(&gcode).expect("读源"),
            "ORIGINAL",
            "源文件一个字节都不许动"
        );
    }

    /// 同一份连着跑两趟 ⇒ 两个副本**不同名**（第二趟不该读到第一趟的产物）
    #[test]
    fn two_runs_of_the_same_file_get_two_copies() {
        let d = tempfile::tempdir().expect("临时根");
        let src = d.path().join("in");
        tree(&src, &["a.gcode"], "X");
        let gcode = src.join("a.gcode");
        let first = copy_for_run(d.path(), &gcode).expect("第一趟");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = copy_for_run(d.path(), &gcode).expect("第二趟");
        assert_ne!(first, second, "两趟该各有一份副本");
    }

    /// 缺文件 / 错后缀：照实说哪一份不对，**不许替用户挑一个**
    #[test]
    fn a_missing_or_mistyped_input_is_refused_by_name() {
        let d = tempfile::tempdir().expect("临时根");
        tree(
            d.path(),
            &["A1.toml", "plate.gcode", "notes.txt"],
            "whatever",
        );

        assert!(check_input(&d.path().join("A1.toml"), &d.path().join("plate.gcode")).is_ok());

        let e = check_input(&d.path().join("nope.toml"), &d.path().join("plate.gcode"))
            .expect_err("预设不在该报错");
        assert!(e.message.contains("预设不在"), "{}", e.message);

        let e = check_input(&d.path().join("A1.toml"), &d.path().join("gone.gcode"))
            .expect_err("G-code 不在该报错");
        assert!(e.message.contains("G-code 不在"), "{}", e.message);

        let e = check_input(&d.path().join("notes.txt"), &d.path().join("plate.gcode"))
            .expect_err("预设不是 .toml 该报错");
        assert!(e.message.contains(".toml"), "{}", e.message);

        let e = check_input(&d.path().join("A1.toml"), &d.path().join("notes.txt"))
            .expect_err("测试件不是 .gcode 该报错");
        assert!(e.message.contains(".gcode"), "{}", e.message);
    }

    /// 副本目录与 `run/hook.port` 同族 —— 它住在内部根里，不在用户那一棵树
    #[test]
    fn the_dev_run_dir_lives_under_the_internal_root() {
        assert!(
            DEV_RUN_DIR.starts_with("run/"),
            "副本属内部产物：{DEV_RUN_DIR}"
        );
    }
}
