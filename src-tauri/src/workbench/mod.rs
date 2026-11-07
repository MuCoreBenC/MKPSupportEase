//! 工作台的 Rust 侧。
//!
//! **整个模块只在 `workbench` feature 下存在。** `lib.rs` 里的 `mod workbench;` 挂着
//! `#[cfg(feature = "workbench")]`，所以默认构建连编译都不会碰这些文件 —— 给用户的
//! 二进制里搜不到下面任何一个命令名。
//!
//! # 当前状态
//!
//! 第二版那 30 多个 `wb_*` 命令与整个 domain 层已全部摘除（doc §6：旧契约作废），
//! 新架构按三层重排：
//!
//! - [`presets`] 预设真相源（`<repo>/presets/*.toml`，读它也写它）
//! - [`domain`] 规则与派生（三层取值、归并、可见性、patch、状态、预览、文案）
//! - [`app`] IPC 入口与落盘（**写只有 `wb_apply_draft` 一条**）
//!
//! 留用的底座：[`paths`]（数据根）/ [`clock`]（时间戳唯一来源）/
//! [`store`]（原子写、id 白名单、通用 JSON IO、草稿/快照/回收站的落盘位置）。
//!
//! **只剩一个 workspace 数据根**：`<repo>/presets`。旧的 `upstream/`（只读别人的
//! `mkpse-presets/content/*.json`）连同它的三级定位、环境变量、fallback 与设置项
//! 一起退休了 —— 不 fallback、不猜路径，运行时只认这一个根。

pub mod app;
pub mod clock;
pub mod domain;
pub mod paths;

/// 产物头那一行的前缀。**全仓只有这一处**定义它（读、写、剥离都走这里）。
///
/// 它记的是"这份产物最近一次成功生成"的时刻（ISO 8601 + 偏移，本地可读），
/// 客户端会读它显示。但它**不参与"内容变没变"的判断** —— 判断前先剥掉这一行
/// （[`strip_generated_at`]），否则每点一次生成都会"有变化"。
pub const GENERATED_AT_PREFIX: &str = "# generated_at:";

/// 从文本里剥掉 `# generated_at:` 行，得到"业务内容"。
///
/// 它只回答"内容变没变"。**要不要写**是另一个问题，见 [`up_to_date`]。
pub fn strip_generated_at(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|l| !l.starts_with(GENERATED_AT_PREFIX))
        .collect()
}

/// 把**不表达内容身份**的历史字段抹平，只留下业务字节。
///
/// | 文件 | 抹掉的 |
/// |---|---|
/// | `*.toml` | `# generated_at:` 那整行 |
/// | 其它（`*.json`） | `"generatedAt"` / `"generated_at"` / `"updated"` 三个历史写盘键的值 |
///
/// 候选字节本身不含这些字段，所以对候选是恒等；对盘上的历史产物才会动字节。
/// 相等的含义只有一条：**内容一样**。不要拿它回答"要不要写"（见 [`up_to_date`]）。
pub fn normalize_for_compare(rel: &str, bytes: &[u8]) -> Vec<u8> {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return bytes.to_vec();
    };
    if rel.ends_with(".toml") {
        return strip_generated_at(text).into_bytes();
    }
    /* 历史 JSON 里可能带过写盘时刻；值归一成固定串，避免同一份内容因时间被判成两份。 */
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let legacy = ["\"generatedAt\":", "\"generated_at\":", "\"updated\":"]
            .iter()
            .any(|k| trimmed.starts_with(k));
        if legacy {
            let indent = &line[..line.len() - trimmed.len()];
            let key = trimmed.split_once(':').map_or(trimmed, |(k, _)| k);
            let nl = if line.ends_with('\n') { "\n" } else { "" };
            out.push_str(indent);
            out.push_str(key);
            out.push_str(": \"\",");
            out.push_str(nl);
        } else {
            out.push_str(line);
        }
    }
    out.into_bytes()
}

/// 盘上这一份是不是**就是候选字节**：同内容**且**同形状 —— 「要不要写」的唯一判据。
///
/// [`normalize_for_compare`] 只回答"内容变没变"，这一处回答"要不要写"。两件事必须分开：
///
/// | 盘上那份 | 内容 | 要不要写 |
/// |---|---|---|
/// | 形状当前、正文相同 | 一致 | 不写 |
/// | 带已废弃的写盘字段、正文相同 | 一致 | **要写**（清掉那几个字节） |
///
/// 为什么"内容一致"也得写：`catalog.json` 的 size / sha256 按**候选字节**登记，
/// 盘上多一行就永远对不上 —— 生成说"没有哪一份会变"、核对说"清单和文件对不上"，
/// 两边都自认为对，僵住不动。
///
/// 形状判据是**单向**的（只惩罚盘上旧）：候选自己还带这些字段时归一化对它是恒等，
/// 不构成"盘上过期"，否则每次生成都会重写一遍。
///
/// # 谁会调它
///
/// 生成事务（[`crate::workbench::app::delivery::generate_transaction`]）、
/// 生成前预演、待更新标记（[`crate::workbench::domain::derive`] 的 `build_state`）
/// 全部走这一处。内核产物那条（`app::build` 写 `crates/preset/assets/…`）**不走**：
/// 它写出去的字节自己带 `# generated_at:`，问题不是"候选字节"而是"补一行时间"。
pub fn up_to_date(rel: &str, on_disk: &[u8], candidate: &[u8]) -> bool {
    let norm_old = normalize_for_compare(rel, on_disk);
    let norm_want = normalize_for_compare(rel, candidate);
    if norm_old != norm_want {
        return false;
    }
    /* 内容一致：形状也必须是当前的。候选自己带旧字段时（归一化对它不是恒等）
       不能因为盘上也带就判"过期"—— 那种情况下重写也清不掉。 */
    let candidate_needs_no_cleanup = norm_want == candidate;
    let on_disk_is_already_clean = norm_old == on_disk;
    !(candidate_needs_no_cleanup && !on_disk_is_already_clean)
}
/// 预设真相源。**它不住在这里** —— 代码搬去了 [`crate::presetdata`]（客户端也要读预设，
/// 而这一整棵子树是 feature gate 的），这里只是把它**别名**回 `presets`：
/// 工作台侧几十处 `crate::workbench::presets::X` 因此一个都不用改。
///
/// 数据根仍然是工作台自己的那一个（`<repo>/presets`），经 [`load_presets`] 给进去 ——
/// **代码共用一份，数据根两套**。
pub use crate::presetdata as presets;
pub mod store;

/// 两端一致性判据：客户端那层来源记录封装与库里那份必须给出同样的结果（**只在测试里编**）
#[cfg(test)]
mod provenance_parity;

use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::error::AppError;

/// 读工作台的预设数据根：`<repo>/presets`。
///
/// 这是工作台与客户端**唯一**的分岔点：两边跑同一份 [`presetdata`] 代码，
/// 只是根不同 —— 客户端给 `appDataDir/presets`，工作台给仓库。
///
/// 资产**载荷**根（`<repo>/public/assets`）只在这一档挂上：客户端这一轮只释放定义、
/// 不释放文件本体，所以它那边 [`presets::Assets::present`] 一律为 false。
pub fn load_presets() -> Result<crate::presetdata::Presets, AppError> {
    let root = paths::presets_root().ok_or_else(|| {
        AppError::not_found("找不到工作台的预设数据根 presets/").with_detail(
            "期望 <repo>/presets/registry/param_registry.toml 存在 —— \
             工作台的数据根指向**仓库里的开发源数据**，不是 appDataDir"
                .to_owned(),
        )
    })?;
    let mut p = crate::presetdata::Presets::load_from(&root)?;
    if let Ok(asset_root) = paths::assets_root() {
        p.set_asset_root(&asset_root);
    }
    Ok(p)
}

/// **编辑态**的那一份预设（影子树）。
///
/// 2026-10-31 起工作台的所有编辑只写影子树（`workbench/.draft/presets/`），
/// 于是「读当前状态」必须是读它 —— 读真仓库会拿到保存前的那一份，
/// 表现为「改了没反应」（正是这次要根治的病）。
pub fn load_session_presets() -> Result<crate::presetdata::Presets, AppError> {
    let store = crate::workbench::store::Store::open()?;
    let shadow = crate::workbench::domain::txn::shadow_root(&store.root);
    let root = if shadow
        .join("registry")
        .join("param_registry.toml")
        .is_file()
    {
        shadow
    } else {
        crate::workbench::paths::presets_root().ok_or_else(|| {
            AppError::not_found("找不到工作台的预设数据根 presets/")
                .with_detail("影子树与仓库里的 presets/ 都没定位到".to_owned())
        })?
    };
    let mut p = crate::presetdata::Presets::load_from(&root)?;
    if let Ok(asset_root) = crate::workbench::paths::assets_root() {
        p.set_asset_root(&asset_root);
    }
    Ok(p)
}

/// 工作台窗口的标签。与客户端的 `main` 分开，`get_webview_window` 拿的是各自那一个
///
/// ★ `pub`：换主题那一档要按它找到这扇窗（见 [`crate::ipc::prefs::set_theme`]）。
pub const WINDOW_LABEL: &str = "workbench";

/// 工作台窗口标题。一度带「【测试模式】」前缀，沙箱删掉之后只剩这一个名字。
fn window_title() -> String {
    "SupportEase 工作台".to_owned()
}

/// 开工作台窗口。已经开着就聚焦，不重复开第二个。
///
/// 刻意**不带** transparent / titleBarStyle=Overlay 这些客户端的窗口花活：
/// 工作台是密集表格界面，原生标题栏反而省事，也不需要为它再写一套拖拽与缩放边。
pub fn open_window(app: &AppHandle) -> Result<(), AppError> {
    if let Some(win) = app.get_webview_window(WINDOW_LABEL) {
        let _ = win.set_focus();
        return Ok(());
    }

    let win =
        WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("workbench.html".into()))
            .title(window_title())
            /* 默认尺寸（作者 2026-10-14「工作台默认窗口不要那么大」）：1360×900 太大 ——
            改 1080×720（常见的紧凑工作区尺寸）。最小尺寸不变，用户照旧能拉到更大 */
            .inner_size(1080.0, 720.0)
            .min_inner_size(900.0, 560.0)
            .resizable(true)
            /*
             * ★ **先藏着，等前端首帧就绪再露面**（作者 2026-10-07）：
             * dev 下前端首次加载要把整棵模块树编译一遍（以分钟计），窗口若当场就露，
             * 人看到的是"一块白板先出现、然后干等"—— 作者原话"不要先出现白屏然后再等"。
             * 露面那一半在前端入口（`src/workbench/main.tsx` 首帧后调 `window.show()`）；
             * 这里同时挂**兜底**（见下）：前端因任何原因没来敲（编译错误 / JS 崩），
             * 藏着的窗口不会有任何报错出口 —— 到点无条件 show，宁可白板也不能永不出现。
             */
            .visible(false)
            .build()
            .map_err(|e| AppError::internal("建不出工作台窗口").with_detail(e.to_string()))?;

    {
        let win = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(60));
            let _ = win.show();
        });
    }

    /*
     * ★★ **原生那一层**（标题栏 / 滚动条 / 原生控件）也要跟着盘上那一档
     * （2026-10-16「工作台也做深色」）。
     *
     * 这扇窗用的是**系统原生装饰** —— CSS 只画得到内容区。开着的时候不设它，
     * 就是"页面深了、标题栏还浅着"。客户端那扇窗对应的是"圆角那一圈填色"
     * （`chrome::repaint_corner_fill`），两条路干的是同一件事：**原生那一层不由 CSS 管**。
     *
     * 「跟随系统」那一档交给系统自己（`None`）—— 别把系统现在的这一档钉死在窗口上。
     */
    crate::ipc::prefs::apply_native_theme(&win, crate::ipc::prefs::theme_pref(app));

    /* 草稿在内存里，磁盘上只有崩溃快照。落盘三个时机里的两个挂在窗口事件上：
    失焦（去别的窗口了，这会儿写不碍事）与关闭前（最后一次机会）。
    第三个是「空闲 2 秒」，由下面那个计时器管。 */
    win.on_window_event(|e| match e {
        tauri::WindowEvent::Focused(false) | tauri::WindowEvent::CloseRequested { .. } => {
            app::flush_now();
        }
        _ => {}
    });

    // 第三个时机是「空闲 2 秒」，由一个每秒醒一次的线程管。
    // 放在窗口创建之后：没有工作台窗口时这件事根本不该发生
    app::spawn_flusher();

    tracing::info!("工作台窗口已打开");
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Roots {
    /// 开发源数据根（仓库里的 `workbench/`）
    pub workbench: String,
    /// 预设真相源（仓库里的 `presets/`）—— 机型 / 参数 / 套餐 / 资产 / 交付产物都在它下面
    pub presets: String,
    /// 交付目录（仓库里的 `presets/delivery/`）
    pub delivery: String,
}
