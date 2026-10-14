//! **开发台** —— 把"平时看不见的那几屏"做成工作台里一排按钮。
//!
//! # 它解决什么（作者 2026-10-14）
//!
//! 「我希望不只是输命令呀……我希望也是一个前端，而且是独立于我这个软件的窗口的前端，
//!  就好像那个工作台一样」→ 工作台加一页「开发台」；「我点那个演示的时候，就把那个
//!  客户端窗口置顶，然后演示」→ 点按钮 = 让**真正的客户端窗口**显示那一屏。
//!
//! # ★★ 为什么这里是"跨进程发消息"，而不是"直接喂自己那份 HookView"
//!
//! **工作台构建里没有客户端窗口** —— `tauri.workbench.conf.json` 的 `app.windows`
//! 是空数组（`lib.rs` 里那句注释写得很清楚：工作台起来"就只有工作台这一页"）。
//! 所以：
//!
//! ```text
//!   工作台进程（windows:[]，无客户端窗口）      客户端进程（npm run tauri dev，有 main 窗口）
//!     点「演示」──invoke──► wb_dev_run ─┐
//!                                       │ 走 hook 那条 loopback（同一条口、同一套消息）
//!                                       ▼
//!                            hook_ipc 监听器收到 DevScene
//!                                       │
//!                            hook_ui::HookView::run_preview
//!                                       │  ① 客户端窗口置顶（unminimize+show+focus）
//!                                       │  ② devsim::drive 演出 → apply（与真钩子同一落点）
//!                                       ▼
//!                            main 窗口的 PostProcessModal 显示（真实组件、纯模拟状态）
//! ```
//!
//! 我一开始把 `PostProcessModal` 直接嵌进工作台窗口里，那是**错的**：工作台窗口就是
//! 另一扇窗，嵌进去等于"在开发工具里另画一份"，不是作者要的"看客户端那一屏"。
//!
//! # 客户端没开怎么办（作者裁定：**不自动拉起**）
//!
//! `Client::connect_or_headless` —— 连不上就一条无头线。这一侧据此**如实报一句**
//! 「客户端窗口没启动，请先 `npm run tauri dev`」，一个进程也不拉起
//! （`connect_or_spawn` 拉起的是 `current_exe()`，工作台里那等于又把工作台起一个，
//! 作者明确不要）。
//!
//! # 只在 workbench 档里
//!
//! 这几条命令挂在 `#[cfg(feature = "workbench")]` 的清单里（`lib.rs` 两份命令清单中的
//! 工作台那一份）—— 给用户的客户端二进制里没有这一页、也没有这些命令。

use serde::Serialize;

use crate::devsim::Mode;
use crate::error::AppError;
use crate::fsx::paths;
use crate::hook_ipc::{Client, FromHook};
use postprocess::diag::CancelToken;

/// 开发台上一个可点的场景。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevScene {
    /// 稳定 id（`postprocess-success` 或 `import-collision` 之类）—— 命令按它分发
    pub id: String,
    /// 分组（界面按它分类摆）：`postprocess` / `import` / `file` / `trust` / `float` …
    pub group: String,
    /// 组名（人话）
    pub group_label: String,
    /// 行上的名字
    pub label: String,
    /// 一句话说清它会显示什么 / 什么时候真出现
    pub note: String,
    /// ★ **能点吗**：`false` 表示这一格还没接上（界面摆灰、给一句"还没做"），
    /// 不是"坏了"。开发台是会长的一页，先把不容易自然触发的那几格接上。
    pub ready: bool,
    /// **层类**（人话）：`后处理` / `标题栏` / `菜单` / `模态框` / `提示条`。
    ///
    /// 界面摆出来给人看"哪些能同时开" —— **不同类可以并存，同类只留最后点的那个**。
    /// 与前端 `src/app/dev/scene.ts` 的 `DevSlot` 是同一件事的两种说法
    /// （这边只给字，判定在那一侧）。
    pub slot_label: String,
}

/// 开发台的场景清单。
///
/// ★ 这是**唯一一份**清单：界面照着它摆按钮，命令照着它分发。加一档只改这里
/// （以及 `run` 里那一处分发）。分组把"看得见/看不见"分开了 —— 作者先要的是
/// **不容易触发的**那一类（后处理、导入、冲突……），容易的那些以后补。
fn scenes() -> Vec<DevScene> {
    let mut out = Vec::new();

    // ---- 后处理那一组（★ 现在就能点）----
    let note = "切片器导出 G-code 时客户端弹出的那一屏 —— 平时要真去切一次片才看得到。\
                点一下把它演给**客户端窗口**看（客户端窗口会置顶；不读 G-code、不落记录）";
    /* ★ 「处理中」排最前（作者 2026-10-14：「不只是成功啊，还有处理中的也要显示」）——
    它演到第二遍扫描就**停住不动**，把跑着那一屏定在屏上（改样式 / 截图都用它）。 */
    out.push(DevScene {
        id: "postprocess-running".to_owned(),
        group: "postprocess".to_owned(),
        group_label: "后处理界面".to_owned(),
        label: "处理中（停住不动）".to_owned(),
        note: "跑着的那一屏（大百分比 + 步骤 + 预计还需 + 两组 facts）—— \
               演到第二遍扫描就**定住**，改样式 / 截图用它。再点别的格子它会自己让位"
            .to_owned(),
        ready: true,
        slot_label: slot_label_of("postprocess-running").to_owned(),
    });
    for m in Mode::ALL {
        out.push(DevScene {
            id: format!("postprocess-{}", m.id()),
            group: "postprocess".to_owned(),
            group_label: "后处理界面".to_owned(),
            label: m.label().to_owned(),
            note: note.to_owned(),
            ready: true,
            slot_label: "后处理".to_owned(),
        });
    }

    // ---- 其余界面格：id 是**稳定 slug**（前端 Provider 按它认领），label 才是人话 ----
    //
    // `ready` = 前端那一格接上了没有。这份清单是**唯一一份**：界面照它摆、命令照它发。
    // 加一格 = 在这里加一行 + 在客户端的持有者里 `useDevPreview('<slug>')`。
    let rows: &[(&str, &str, &str, &str, &str, bool)] = &[
        // ---- 导入与另存 ----
        (
            "import-collision",
            "import",
            "导入与另存",
            "导入遇到同名（改名那一格）",
            "拖进一份与盘上重名的文件时，每份一个输入框那一格（不覆盖、不自动改名）",
            true,
        ),
        (
            "import-result",
            "import",
            "导入与另存",
            "导入完成（结果那格）",
            "导完之后那格「导入完成 / 有几份没成」，逐份列出落到哪",
            true,
        ),
        // ---- 文件管理与冲突 ----
        (
            "rename",
            "file",
            "文件管理与冲突",
            "重命名本地预设",
            "改本地预设文件名时的输入框（含非法名 / 重名拦截）",
            true,
        ),
        (
            "delete-confirm",
            "file",
            "文件管理与冲突",
            "删除确认",
            "删一份本地预设前的确认框（确认层长在右键菜单里，这个是从菜单里直接摆出来的）",
            true,
        ),
        (
            "compare",
            "file",
            "文件管理与冲突",
            "参数对比台",
            "并排看 2~3 份本地预设的参数（右键菜单的那个入口）",
            true,
        ),
        (
            "export",
            "file",
            "文件管理与冲突",
            "导出执行报告",
            "报告页「导出」那一框：三件套（原始 / 处理后 / JSON）选目录导出",
            true,
        ),
        // ---- 校验与更新 ----
        (
            "update-prompt",
            "trust",
            "校验与更新",
            "软件更新提示",
            "标题栏那个下载指示器 + 详情面板（摆成「正在下载 62%」那一态）",
            true,
        ),
        // ---- 浮出的条 / 气泡 ----
        (
            "float-toast",
            "float",
            "浮出的条 / 气泡",
            "浮出的小条",
            "报告页那条提示条（in-flow、要手动关，不自己消失）—— 摆一条「已确认导出」",
            true,
        ),
    ];
    for (id, group, group_label, label, note, ready) in rows {
        out.push(DevScene {
            id: (*id).to_owned(),
            group: (*group).to_owned(),
            group_label: (*group_label).to_owned(),
            label: (*label).to_owned(),
            note: (*note).to_owned(),
            ready: *ready,
            slot_label: slot_label_of(id).to_owned(),
        });
    }
    out
}

/// 这一格属于哪一层类（**人话**）—— 与 `src/app/dev/scene.ts` 的 `DevSlot` 同一件事。
///
/// 界面摆出来给人看"哪些能同时开"；真正的判定在前端那一侧（这边只给字，不参与决策）。
fn slot_label_of(id: &str) -> &'static str {
    match id {
        "postprocess-running"
        | "postprocess-success"
        | "postprocess-fail"
        | "postprocess-cancel"
        | "postprocess-question" => "后处理",
        "update-prompt" => "标题栏",
        "delete-confirm" => "菜单",
        "float-toast" => "提示条",
        _ => "模态框",
    }
}

/// 开发台能点哪些 —— 一组分好类的按钮，界面照它摆。
#[tauri::command]
pub fn wb_dev_devsim_scenes() -> Result<Vec<DevScene>, AppError> {
    crate::ipc::traced("wb_dev_devsim_scenes", |_| Ok(scenes()))
}

/// **开发台：触发一格界面** —— 把 `scene` 发给**客户端进程**，让**客户端窗口**显示它。
///
/// `scene` 是清单里的 id（`success` / `fail` / … 或 `import-collision` / `rename` / …）。
/// 本命令**不喂自己那份 `HookView`**（工作台窗口没人看这些），只把指令发过去；
/// 客户端收到后：若是后处理那一档就**演出**那一屏，否则发一条 [`crate::hook_ui::DEV_PREVIEW_EVENT`]
/// 让前端那格界面的持有者自己认领（见 `src/app/dev/preview.tsx`）。
///
/// 客户端没开时**如实报错**（不拉进程 —— 拉起来的是工作台自己，见文件头）。
///
/// ★★ **端点要算客户端那棵树，不是工作台这棵**：工作台的 identifier 是
/// `SupportEase-Workbench`，`app.path().app_data_dir()` 给的是**工作台自己**那棵
/// （`…/SupportEase-Workbench`），而客户端监听器（`hook_ipc::bind_for_display`）绑在
/// **`…/SupportEase`** 那棵（`fsx::paths::APP_IDENTIFIER`）。发错树 = 消息石沉大海
/// （两棵树的 `hook.port` 各写各的，谁也不看谁）。所以这里用
/// [`paths::internal_root_headless`] —— 它按 `APP_IDENTIFIER` 算，与客户端 / 钩子 /
/// 假钩子 CLI 是**同一棵**。
#[tauri::command]
pub fn wb_dev_trigger(scene: String) -> Result<(), AppError> {
    crate::ipc::traced("wb_dev_trigger", |_| {
        let Some(row) = scenes().into_iter().find(|r| r.id == scene) else {
            return Err(AppError::invalid_argument(format!("不认识的场景：{scene}")));
        };
        if !row.ready {
            return Err(AppError::invalid_argument(format!(
                "「{}」还没接上 —— 先把已接的那几格用起来",
                row.label
            )));
        }

        /* 客户端那棵内部根（不是工作台这棵）—— 见上面那段 ★★ */
        let Some(root) = paths::internal_root_headless() else {
            return Err(AppError::internal("算不出客户端内部数据根"));
        };

        /* ★ connect_or_headless：连不上就是"没有客户端窗口在听"，一个进程都不拉起 */
        let cancel = CancelToken::new();
        let client = Client::connect_or_headless(&root, &cancel);
        if !client.connected() {
            return Err(AppError::not_found(
                "客户端窗口没启动 —— 先跑 npm run tauri dev，再点「演示」",
            ));
        }

        /* 发那条指令；客户端收到后自己置顶窗口 + 显示那一格界面 */
        client.send(&FromHook::DevScene { scene });
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 后处理那一组**都 ready**，且四档都在（id 是 `postprocess-<mode>`）
    #[test]
    fn the_postprocess_group_is_ready_and_complete() {
        let rows = scenes();
        let pp: Vec<&DevScene> = rows.iter().filter(|r| r.group == "postprocess").collect();
        assert!(pp.iter().all(|r| r.ready), "后处理那一组该都能点");
        for m in Mode::ALL {
            let id = format!("postprocess-{}", m.id());
            assert!(pp.iter().any(|r| r.id == id), "少了 {id}");
        }
    }

    /// 场景 id **必须唯一**（命令按它分发；重了会发错格子）
    #[test]
    fn the_scene_ids_are_unique() {
        let rows = scenes();
        let mut ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
        let n = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), n, "场景 id 必须唯一");
    }

    /// 除了后处理那一组，其余格子的 id 一律是**稳定 slug**（kebab-case、无中文、无冒号）
    /// —— 前端 `useDevPreview('<slug>')` 按它认领，改个汉字就静默失配
    #[test]
    fn the_ui_scene_ids_are_stable_slugs() {
        for r in scenes().into_iter().filter(|r| r.group != "postprocess") {
            assert!(
                r.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "场景 id 只能是 kebab-case ASCII：{}",
                r.id
            );
        }
    }
}
