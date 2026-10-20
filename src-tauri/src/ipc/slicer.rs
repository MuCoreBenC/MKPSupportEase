//! 「用哪个切片软件打开模型」（2026-10-13）。
//!
//! 前端那一格（设置页「切片」+ 打开模型框里的「仅这一次」）只给一个 **id**：
//! `system` / `bambu` / `orca` —— 与 `src/app/shared/slicer.ts` 的 `SlicerId` 逐字对齐，
//! **id 是契约**。找应用、拼命令行都在这一侧，前端既不碰路径也不猜安装位置。
//!
//! # 三档各自是什么意思
//!
//! - `system`：交给**系统关联程序**（`.3mf` 双击是什么就是什么）—— 走 opener 插件，
//!   与这一条命令原来的行为一字不差；
//! - `bambu` / `orca`：**指定的那个应用**。用户明确点了哪一档才走到这里，
//!   所以"没装那个应用"要**照实报**（`NOT_FOUND` + 一句人话），**不许静默回落**成
//!   系统关联 —— 那等于把用户的选择当没听见，界面还会以为真按他说的开了。
//!
//! # 为什么是"候选路径"而不是一个写死的路径
//!
//! 两个都是用户自己装的第三方软件，装在哪不由我们定：macOS 有 `/Applications`
//! 与 `~/Applications` 两处；Windows 有系统级 `Program Files` 与用户级
//! `%LOCALAPPDATA%\Programs` 两种安装位（OrcaSlicer 的安装器两种都提供）。
//! 逐个 `exists()`，第一个命中的说了算；一个都没命中就是"没装"，不挑一个最近的猜。
//!
//! 认不出的 id **整次拒绝**（`INVALID_ARGUMENT`），理由同上：宁可报错也不悄悄换一档。

use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 前端那一格的四个 id。**与 `src/app/shared/slicer.ts` 的 `SlicerId` 一一对应**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlicerKind {
    /// 跟随系统关联程序（默认；不传参数时也是这一档）
    System,
    /// Bambu Studio
    Bambu,
    /// OrcaSlicer
    Orca,
    /// 用户**从磁盘指定**的那个（2026-10-13 第二刀）。本机没有固定落脚 ——
    /// 可执行物路径由前端随 `open_model` 一起递过来（壳读不到 webview 的 localStorage）。
    Custom,
}

impl SlicerKind {
    /// 认 id。`None` / 空串 = 没传 = 跟随系统（旧调用一个都不用改）
    pub fn parse(raw: Option<&str>) -> Result<Self, AppError> {
        match raw.map(str::trim) {
            None | Some("") | Some("system") => Ok(Self::System),
            Some("bambu") => Ok(Self::Bambu),
            Some("orca") => Ok(Self::Orca),
            Some("custom") => Ok(Self::Custom),
            Some(other) => Err(AppError::invalid_argument(format!(
                "不认识的切片软件：{other}（只有 system / bambu / orca / custom 四档）"
            ))),
        }
    }

    /// 给日志与错误话术用的 id（与前端那份同字）
    pub fn id(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Bambu => "bambu",
            Self::Orca => "orca",
            Self::Custom => "custom",
        }
    }

    /// 给人看的名字（错误话术里用）
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "系统默认程序",
            Self::Bambu => "Bambu Studio",
            Self::Orca => "OrcaSlicer",
            Self::Custom => "你指定的应用",
        }
    }
}

/// 这台机器上那个应用的可能落脚（顺序即优先级）。`system` 这一档没有落脚 —— 它走系统关联。
///
/// macOS 给的是 **.app 包**（启动时交给 `/usr/bin/open -a`，让 LaunchServices 处理
/// "已经在跑就复用那一个实例"）；Windows 给的是 **exe**（直接起进程）。
#[cfg(target_os = "macos")]
fn slicer_candidates(kind: SlicerKind) -> Vec<PathBuf> {
    let bundle = match kind {
        SlicerKind::Bambu => "BambuStudio.app",
        SlicerKind::Orca => "OrcaSlicer.app",
        /* 指定的那个没有落脚：路径由前端递过来（`launch_with`） */
        SlicerKind::System | SlicerKind::Custom => return Vec::new(),
    };
    let mut out = vec![PathBuf::from("/Applications").join(bundle)];
    if let Some(home) = std::env::var_os("HOME") {
        out.push(PathBuf::from(home).join("Applications").join(bundle));
    }
    out
}

#[cfg(target_os = "windows")]
fn slicer_candidates(kind: SlicerKind) -> Vec<PathBuf> {
    let (dir, exe) = match kind {
        SlicerKind::Bambu => ("Bambu Studio", "bambu-studio.exe"),
        SlicerKind::Orca => ("OrcaSlicer", "orca-slicer.exe"),
        /* 同上：指定的那个没有落脚 */
        SlicerKind::System | SlicerKind::Custom => return Vec::new(),
    };
    let mut out = Vec::new();
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(root) = std::env::var_os(var) {
            out.push(PathBuf::from(root).join(dir).join(exe));
        }
    }
    /* 用户级安装位（不需要管理员权限的那种安装器默认落这） */
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        out.push(PathBuf::from(local).join("Programs").join(dir).join(exe));
    }
    out
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn slicer_candidates(_kind: SlicerKind) -> Vec<PathBuf> {
    Vec::new()
}

/// 那个应用在这台机器上的确切位置（找不到 = `None`，**不猜**）
pub fn locate(kind: SlicerKind) -> Option<PathBuf> {
    slicer_candidates(kind).into_iter().find(|p| p.exists())
}

/// 把一份模型交给**指定**的切片软件打开。
///
/// `system` 那一档不走这里（它是"系统关联程序"的事，见 [`crate::ipc::mod::open_model`]）。
pub fn launch(kind: SlicerKind, file: &Path) -> Result<(), AppError> {
    let Some(target) = locate(kind) else {
        return Err(AppError::not_found(format!(
            "这台机器上没找到 {} —— 装好它，或者在设置里改回「跟随系统默认」",
            kind.label()
        )));
    };
    launch_with(&target, kind.label(), file)
}

/// Windows 上起那个应用时要给 `CreateProcessW` 的 `dwCreationFlags`（2026-10-20）。
///
/// # 为什么一个位都不能省
///
/// 不给时子进程**继承我们的控制台与进程组**（`CreateProcessW` 的默认行为）。
/// 开发构建是 console 子系统（`src/main.rs` 那条 `windows_subsystem` 只对 release 生效），
/// 从终端 `npm run tauri dev` 起 MKP 时那个控制台就是终端那一个 —— 关掉它、或者在里头
/// 按 Ctrl+C，控制事件会发给**挂在同一个控制台上的每一个进程**，于是"从这个 MKP 打开的
/// BBS 跟着 MKP 一起关闭"（作者 2026-10-20 报的那一处）。
///
/// # 这两位各自管什么
///
/// - `DETACHED_PROCESS`（0x8）：**不继承控制台** —— 没有那一层附着，控制台关不关都
///   落不到它身上；
/// - `CREATE_NEW_PROCESS_GROUP`（0x200）：自成一组，Ctrl+C 那一路广播也落不到它头上
///   （这两位可以同时给；互斥的是 `DETACHED_PROCESS` 与 `CREATE_NEW_CONSOLE`）。
///
/// 切片软件是**别人的窗口**、有它自己的生命周期：我们只是"把这份文件交给它"，
/// 交出去之后 MKP 关不关与它无关。
///
/// ★ 挂 `any(windows, test)`：值本身与运行平台无关，这样"这两位必须是哪两位"这条判据
/// 在三个 CI job 上都跑得到，不必等哪天有人在 Windows 上真开一次 BBS。
#[cfg(any(target_os = "windows", test))]
fn windows_creation_flags() -> u32 {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP
}

/// 把一份模型交给**一个明说了在哪**的应用打开（2026-10-13 第二刀的 `custom` 档走这里）。
///
/// 路径是前端从系统对话框挑的（macOS 是 `.app` 包、Windows 是 `.exe`），这一侧**不猜、
/// 不补后缀**：不在盘上就是不在，照实报 `NOT_FOUND`（让人回设置页重挑）。
pub fn launch_with(target: &Path, label: &str, file: &Path) -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    let spawned = std::process::Command::new("/usr/bin/open")
        .arg("-a")
        .arg(target)
        .arg(file)
        .spawn();

    #[cfg(target_os = "windows")]
    let spawned = {
        use std::os::windows::process::CommandExt as _;
        std::process::Command::new(target)
            .arg(file)
            .creation_flags(windows_creation_flags())
            .spawn()
    };

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let spawned = std::process::Command::new(target).arg(file).spawn();

    spawned.map(|_| ()).map_err(|e| {
        AppError::io(format!(
            "{label} 没打开这个文件（应用在 {}）",
            target.display()
        ))
        .with_detail(e.to_string())
    })
}

/* ---------- 「跟随系统默认」到底是哪个程序（2026-10-16） ---------- */

/// 给界面看的**一个应用**：`name` 摆在下拉那一行的右边，`path` 是应用包 / 可执行物。
///
/// 与 `src/api/contract.ts` 的 `SlicerApp` 逐字对齐 —— 那个类型本来是「从磁盘挑一个」
/// （`pickSlicerApp`）的返回值，这里复用同一形状：界面读的是同一件事，不该有两套。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SlicerApp {
    pub name: String,
    pub path: String,
}

/// 这台机器上 `.3mf` **现在**交给谁 —— 界面上「跟随系统默认」那一档的含义。
///
/// # 为什么值得专门问一句
///
/// 四档里只有它名字里没有应用名：用户看着「跟随系统默认」没法知道双击会开哪个软件
/// （作者 2026-10-16：「我不知道它默认的是哪一个」）。系统那边有真值，这里去取回来。
///
/// # 两个平台各自问谁
///
/// - **macOS**：LaunchServices（`LSCopyDefaultApplicationURLForURL`）。它按**文件**认类型 ——
///   路径不存在时答 `null`（2026-10-16 实测：不存在的 `.3mf` → `null`，空文件 → 真答案），
///   所以先落一份**空的临时 `.3mf`** 拿它去问：内容不参与判断，扩展名才是判据。
/// - **Windows**：注册表两跳（`HKCR\.3mf` 的默认值 = ProgID → `…\shell\open\command`）。
/// - 其余平台没有这一档的说法，恒 `None`。
///
/// # 问不出来就给 `None`，**不编一个名字**
///
/// "这台机器没关联任何程序"与"这一次查不了"都回落 `None`：界面照原来那句话显示
/// （「用 .3mf 关联的程序打开」）。**绝不拿猜出来的应用名顶上** —— 那句话是给用户看的。
pub fn system_default_app() -> Option<SlicerApp> {
    #[cfg(target_os = "macos")]
    let found = macos_system_default_app();
    #[cfg(target_os = "windows")]
    let found = windows_system_default_app();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let found = None;
    found
}

#[cfg(target_os = "macos")]
fn macos_system_default_app() -> Option<SlicerApp> {
    use core_foundation::base::TCFType;
    use core_foundation::url::CFURL;

    /* LaunchServices 是 CoreServices 伞框架下的 C API（不是 objc 那一套），哪条线程都能问 */
    #[link(name = "CoreServices", kind = "framework")]
    extern "C" {
        /// 系统认为该用哪个应用打开这个 URL。**+1 规则**：回来的引用归调用方所有 ——
        /// 所以只许用 `wrap_under_create_rule` 接它（用 get_rule 接等于少还一次）。
        fn LSCopyDefaultApplicationURLForURL(
            url: core_foundation::url::CFURLRef,
            roles: u32,
            error: *mut core_foundation::base::CFTypeRef,
        ) -> core_foundation::url::CFURLRef;
    }

    /// `LSRolesMask::kLSRolesViewer` —— 「打开一份文件」要问的就是这一档
    const ROLES_VIEWER: u32 = 0x0000_0002;

    /* 临时空文件：`tempfile` 落它、析构即删 —— 只活在这一次询问里 */
    let probe = tempfile::Builder::new()
        .prefix("mkp-slicer-probe-")
        .suffix(".3mf")
        .tempfile()
        .ok()?;
    let url = CFURL::from_path(probe.path(), false)?;
    let app_ref = unsafe {
        LSCopyDefaultApplicationURLForURL(
            url.as_concrete_TypeRef(),
            ROLES_VIEWER,
            std::ptr::null_mut(),
        )
    };
    if app_ref.is_null() {
        /* 系统那边没有关联的程序 —— 照实说"问不出来" */
        return None;
    }
    let app_url = unsafe { CFURL::wrap_under_create_rule(app_ref) };
    let path = app_url.to_path()?;
    let name = bundle_display_name(&app_url).unwrap_or_else(|| app_stem(&path));
    Some(SlicerApp {
        name,
        path: path.to_string_lossy().into_owned(),
    })
}

/// 应用包自己说它叫什么（`CFBundleDisplayName` → `CFBundleName`）。
///
/// 为什么不用路径末段：`/Applications/BambuStudio.app` 的**显示名**是 "Bambu Studio"
/// （两个词），锯掉 `.app` 只会得到 "BambuStudio" —— 差一个空格也是差。
#[cfg(target_os = "macos")]
fn bundle_display_name(app_url: &core_foundation::url::CFURL) -> Option<String> {
    use core_foundation::bundle::CFBundle;
    use core_foundation::string::CFString;

    let info = CFBundle::new(app_url.clone())?.info_dictionary();
    for key in ["CFBundleDisplayName", "CFBundleName"] {
        let key = CFString::from_static_string(key);
        let Some(value) = info.find(&key) else {
            continue;
        };
        let Some(text) = value.downcast::<CFString>() else {
            continue;
        };
        let text = text.to_string();
        if !text.is_empty() {
            return Some(text);
        }
    }
    None
}

#[cfg(target_os = "windows")]
fn windows_system_default_app() -> Option<SlicerApp> {
    let command = windows_open_command("3mf")?;
    let path = PathBuf::from(exe_from_command_line(&command)?);
    let name = known_label(&path)
        .map(str::to_string)
        .unwrap_or_else(|| app_stem(&path));
    Some(SlicerApp {
        name,
        path: path.to_string_lossy().into_owned(),
    })
}

/// 注册表两跳：`HKCR\.3mf` → ProgID → 那一支的 `shell\open\command` 命令行。
///
/// `HKCR` 是 `HKLM\Software\Classes` 与 `HKCU\Software\Classes` 的**合并视图**
/// （用户那半优先），所以问它就等于问"双击会发生什么"，不必自己拼优先级。
#[cfg(target_os = "windows")]
fn windows_open_command(ext: &str) -> Option<String> {
    use winreg::enums::HKEY_CLASSES_ROOT;
    use winreg::RegKey;

    let hkcr = RegKey::predef(HKEY_CLASSES_ROOT);
    let ext_key = hkcr.open_subkey(format!(".{ext}")).ok()?;
    let mut prog_id: String = ext_key.get_value("").unwrap_or_default();
    if prog_id.is_empty() {
        /* 「打开方式 → 总是用这个」记的是这一支（扩展名那格自己的默认值仍是空的） */
        prog_id = ext_key
            .open_subkey("OpenWithProgids")
            .ok()?
            .enum_values()
            .filter_map(Result::ok)
            .map(|(name, _)| name)
            .next()?;
    }
    let command: String = hkcr
        .open_subkey(format!("{prog_id}\\shell\\open\\command"))
        .ok()?
        .get_value("")
        .ok()?;
    let command = command.trim().to_string();
    (!command.is_empty()).then_some(command)
}

/// 那两个已知应用**在这台机器上的落点**与解析结果完全一致时，用它们自己的名字
/// （`bambu-studio.exe` 这种文件名不是给人看的）。macOS 走应用包的显示名，用不到这条。
#[cfg(target_os = "windows")]
fn known_label(path: &Path) -> Option<&'static str> {
    let want = path.to_string_lossy();
    for kind in [SlicerKind::Bambu, SlicerKind::Orca] {
        if let Some(found) = locate(kind) {
            if found.to_string_lossy().eq_ignore_ascii_case(&want) {
                return Some(kind.label());
            }
        }
    }
    None
}

/// 从注册表那句命令行里取出**可执行物**。
///
/// 两种写法都要认：
/// - 引号包住的（`"C:\Program Files\…\app.exe" "%1"`）—— 系统自己的写法，闭引号即终点；
/// - 没引号的（`C:\tools\app.exe "%1"`）—— 路径里有空格时无法与参数区分开，
///   只能认**第一个 `.exe` / `.bat` / `.cmd` 结尾**（安装器写的都是这几个后缀）。
///
/// ★ 挂 `any(target_os = "windows", test)`：Windows 上它是产品代码，别的平台上它只在
/// 测试里编出来 —— 这段解析逻辑不该因为"我们此刻在 macOS 上"就没单测（它最终跑在 Windows 上）。
#[cfg(any(target_os = "windows", test))]
fn exe_from_command_line(command: &str) -> Option<String> {
    let text = command.trim();
    if let Some(rest) = text.strip_prefix('"') {
        let end = rest.find('"')?;
        let exe = rest[..end].trim();
        return (!exe.is_empty()).then(|| exe.to_string());
    }
    let lower = text.to_ascii_lowercase();
    for suffix in [".exe", ".bat", ".cmd"] {
        if let Some(at) = lower.find(suffix) {
            let exe = text[..at + suffix.len()].trim();
            return (!exe.is_empty()).then(|| exe.to_string());
        }
    }
    None
}

/// 路径末段锯掉 `.app` / `.exe` 那层壳（`/Applications/PrusaSlicer.app` → `PrusaSlicer`）——
/// 只在"这个应用自己没报名"时兜底（见 [`bundle_display_name`]、[`known_label`]）。
fn app_stem(path: &Path) -> String {
    let base = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());
    /* `to_ascii_lowercase` 逐字节等长 ⇒ 拿它的位置去切原串不会落到半个字上 */
    let lower = base.to_ascii_lowercase();
    for suffix in [".app", ".exe"] {
        if lower.ends_with(suffix) {
            return base[..base.len() - suffix.len()].to_string();
        }
    }
    base
}

/* ---------- 「去系统里改默认应用」那一步（2026-10-16） ---------- */

/// **把用户送到"改 `.3mf` 默认应用"的那一处** —— 我们**不替他改**（作者 2026-10-16 口径：
/// 「还是不要帮我改了……打开系统那个页面给用户自己去改」）。
///
/// 系统默认应用是**用户自己的账**：改不改、改成什么，该由他在系统那一侧按。我们只把他送过去。
///
/// # 两个平台各自怎么办
///
/// - **macOS**：**系统设置里根本没有"文件关联"这一页**（2026-10-16 查过：`/System/Library/
///   ExtensionKit/Extensions` 那 280 个设置扩展里没有任何一个管默认应用，`Desktop & Dock`
///   那页只有"默认网页浏览器"）—— 唯一能改的地方是访达的**「显示简介 → 打开方式 →
///   更改全部…」**，所以这里就**把那个「显示简介」窗口打开**。
///   ★ 2026-10-16 走过一圈弯路：先做成"只在访达里定位那份文件"，作者当场纠回来
///   （「我不是要打开那个文件夹呀，我是要像你那个打开显示简介一样呀」）—— **定位不算**，
///   要的就是那个窗口。
///   ★★ 走 `osascript` 的 Finder 口令 `open information window`（系统没给这条路任何 URL
///   scheme 或公开 API）。**它要过一次「自动化 → 控制访达」的授权**：第一次弹一张系统框，
///   点「好」之后不再弹。拒绝了 `osascript` 报 `-1743`（`errAEEventNotPermitted`）——
///   那时走 [`information_window_failed`] 那条**退路**：在访达里定位那份文件 + 把手工
///   三步写进错误里（作者 2026-10-16：「如果点了不允许的话，就打开那一个文件夹的位置吧」）。
/// - **Windows**：「你要如何打开这个文件？」那个框（`shell32.dll,OpenAs_RunDLL`）——
///   系统自己的 UI，勾上"始终"就是改默认，**我们一个字都不往注册表里写**。
/// - 其余平台没有这条入口。
///
/// 回来的那句是**下一步提示**（界面照原样弹一条）："到了那儿还要点哪儿"是平台知识，
/// 由这一侧说；前端不复述、不翻译。
///
/// 为什么要一份 `.3mf`：两个平台的 UI 都是**对一份文件**发问的（"拿它怎么办"）。
/// 那份文件由调用方给 —— 一份**空的示例**，不依赖本机下载过什么，见
/// `ipc::app_settings_sample`。
pub fn open_default_app_settings(file: &Path) -> Result<String, AppError> {
    #[cfg(target_os = "macos")]
    let out = macos_open_default_app_settings(file);
    #[cfg(target_os = "windows")]
    let out = windows_open_default_app_settings(file);
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let out = Err(AppError::not_implemented(
        "这台系统上还没有「去系统里改默认应用」这条入口",
    ));
    out
}

/// 「自动化」授权被拒后的**反悔入口**（2026-10-17 作者：「如果用户后悔了，有再次触发的
/// 开关吗」）。
///
/// ★ macOS 的规矩：**拒绝过一次，系统就永远不会再弹那个授权框** —— TCC 没有"再问一次"
///   的 API，`osascript` 每一次都会直接吃 `-1743`。用户反悔的唯一正路是
///   **系统设置 → 隐私与安全性 → 自动化** 里把「访达」勾回来。
///
///   这里能做的就是把他**送到那一页**：macOS 给"直达某个设置面板"的 URL
///   （`x-apple.systempreferences:com.apple.preference.security?Privacy_Automation`）——
///   用它开，用户落点就在「自动化」那一页，不用自己在系统设置里翻。
///   ★ 依旧**不替他勾**：勾不勾还是他自己的账，我们只负责"别让他找不到在哪儿"。
/// - Windows：那条「打开方式」框没有"授权"这一说（勾"始终"就是改默认），
///   这条入口在 Windows 上没有意义 —— 返回"没有这一页"。
pub fn open_automation_settings() -> Result<(), AppError> {
    #[cfg(target_os = "macos")]
    let out = std::process::Command::new("/usr/bin/open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Automation")
        .spawn()
        .map(|_| ())
        .map_err(|e| AppError::io("打不开系统设置的「自动化」页").with_detail(e.to_string()));
    #[cfg(not(target_os = "macos"))]
    let out: Result<(), AppError> = Err(AppError::not_implemented(
        "这台系统上没有「自动化授权」这一页（那是 macOS 的东西）",
    ));
    out
}

/// 那把 `osascript` 的口令，三段各一条 `-e`（`on run argv` / 正文 / `end run`）。
///
/// ★ 路径**不走这几段**，走末尾那个 argv 实参 —— 拼进字符串就得处理引号与转义，
///   而路径里的字符不由我们定。这条"路径只出现在最后一个实参里"由单测钉着
///   （真跑起来会开窗，单测只能钉参数形状）。
#[cfg(any(target_os = "macos", test))]
const FINDER_INFORMATION_WINDOW_SCRIPT: [&str; 3] = [
    "on run argv",
    "tell application \"Finder\" to open information window of (POSIX file (item 1 of argv) as alias)",
    "end run",
];

/// `osascript` 那一整套实参：`-e 段 -e 段 -e 段 <路径>`。
#[cfg(any(target_os = "macos", test))]
fn osascript_information_window_args(file: &Path) -> Vec<std::ffi::OsString> {
    let mut args: Vec<std::ffi::OsString> =
        Vec::with_capacity(FINDER_INFORMATION_WINDOW_SCRIPT.len() * 2 + 1);
    for chunk in FINDER_INFORMATION_WINDOW_SCRIPT {
        args.push("-e".into());
        args.push(chunk.into());
    }
    args.push(file.as_os_str().to_owned());
    args
}

#[cfg(target_os = "macos")]
fn macos_open_default_app_settings(file: &Path) -> Result<String, AppError> {
    /* 这条 `osascript` 一下子就走完，**等它**（开不出来要照实报）；Windows 那个「打开方式」
    框开在那儿等用户点，那边只能 spawn 不能等 */
    match std::process::Command::new("/usr/bin/osascript")
        .args(osascript_information_window_args(file))
        .output()
    {
        /* 成功时它没有别的话要说（stdout 会把那个窗口对象打回来，不解析） */
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
            return Err(information_window_failed(file, &why));
        }
        Err(e) => return Err(information_window_failed(file, &e.to_string())),
    }

    let name = file
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| file.to_string_lossy().into_owned());
    /* ★ 「更改全部…」那一步必须点名：不点它只对**这一个文件**生效 —— 作者在访达里
    "选了却没有"，十有八九就是这一步没点（macOS 没有 Windows 那个"始终"勾选框）。
    ★★ 顺带交代"窗口会开在哪儿"（2026-10-18 作者实况：人在桌面2用 MKP，访达的窗口
    都在桌面1，点完人就被带回桌面1，以为窗口没弹）：macOS 的规矩是**应用被激活时跳到
    有它窗口的那个桌面**，而改默认应用只能走访达这一个宿主 —— 窗口落在访达那个桌面、
    人跟着过去，都是系统那一层定的，不是没开。这一句写在这儿（平台知识在这一侧），
    前端照原样摆，不复述。 */
    Ok(format!(
        "已打开「{name}」的「显示简介」—— 在「打开方式」里选好应用，再点右边的「更改全部…」\
         （不点它只对这一个文件生效）。这窗口由 macOS 开在访达所在的那个桌面 —— \
         眼前没看到的话，去访达那个桌面找"
    ))
}

/// 「显示简介」窗口开不出来时的**退路**（作者 2026-10-16 的口径：「如果点了不允许的话，
/// 就打开那一个文件夹的位置吧。让用户自己手动的去点，个详细的步骤」）。
///
/// 做两件事：① 把那份文件**在访达里定位出来**（`open -R`）——用户对着它自己右键，走的是
/// 同一条路；② 把**接着要做的三步**写进错误里。
///
/// ★ 这**不是**"假装成功"：回来的仍然是错（界面按 `PERMISSION_DENIED` / `IO` 区分语气），
///   只是这条错里带着他能立刻照做的话 —— 他本来就是因为"找不到在哪儿改"才点的这一下。
/// ★ 定位那一步做到没做到**照实说**（没做到就把路径报出来），不写"已定位"。
#[cfg(target_os = "macos")]
fn information_window_failed(file: &Path, why: &str) -> AppError {
    let located = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(file)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    let where_it_is = if located {
        "已经帮你在访达里定位到那份文件".to_string()
    } else {
        format!("没能在访达里定位它（它在 {}）", file.display())
    };
    information_window_failure(why, &where_it_is)
}

/// 那条退了半步的错的**话与码**（纯函数：定位已经在上面做完了，这里只负责说）。
///
/// 单测钉的就是它 —— 这段是用户**唯一能照着做**的话（"你接着要做什么"），
/// 出错的那句话里必须带着那三步。
#[cfg(target_os = "macos")]
fn information_window_failure(why: &str, where_it_is: &str) -> AppError {
    let steps = "手动来：右键它 →「显示简介」→「打开方式」里选好应用 → 点「更改全部…」\
                 （不点只对这一个文件生效）";
    /* -1743 = errAEEventNotPermitted：用户点了「不允许」，MKP 控制不了访达 */
    if why.contains("-1743") {
        AppError::permission_denied(format!(
            "没能替你打开「显示简介」—— 你点了「不允许」，MKP 操作不了访达。{where_it_is}，{steps}"
        ))
        .with_detail(why.to_string())
    } else {
        AppError::io(format!("没能替你打开「显示简介」。{where_it_is}，{steps}"))
            .with_detail(why.to_string())
    }
}

#[cfg(target_os = "windows")]
fn windows_open_default_app_settings(file: &Path) -> Result<String, AppError> {
    /* 「你要如何打开这个文件？」：系统自己的框，勾"始终"就改了默认 —— 我们不碰注册表 */
    std::process::Command::new("rundll32.exe")
        .arg("shell32.dll,OpenAs_RunDLL")
        .arg(file)
        .spawn()
        .map(|_| {
            "在「你要如何打开这个文件？」里选好应用，再勾上「始终使用此应用打开 .3mf 文件」"
                .to_string()
        })
        .map_err(|e| AppError::io("没能打开「打开方式」那个框").with_detail(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_ids_are_rejected_instead_of_falling_back() {
        /* 不传 / 空串 = 跟随系统（旧调用不用改） */
        assert_eq!(SlicerKind::parse(None).unwrap(), SlicerKind::System);
        assert_eq!(SlicerKind::parse(Some("")).unwrap(), SlicerKind::System);
        assert_eq!(
            SlicerKind::parse(Some(" system ")).unwrap(),
            SlicerKind::System
        );
        assert_eq!(SlicerKind::parse(Some("bambu")).unwrap(), SlicerKind::Bambu);
        assert_eq!(SlicerKind::parse(Some("orca")).unwrap(), SlicerKind::Orca);
        /* 「从磁盘指定的那个」：认得，但它没有落脚 —— 路径由前端递过来 */
        assert_eq!(
            SlicerKind::parse(Some("custom")).unwrap(),
            SlicerKind::Custom
        );
        assert!(locate(SlicerKind::Custom).is_none());
        /* 认不出来就整次拒绝：不静默换成 system —— 那等于把用户点的当没听见 */
        for bad in ["prusa", "Bambu", "bamboo", "orcaslicer"] {
            assert!(SlicerKind::parse(Some(bad)).is_err(), "{bad} 该被拒");
        }
    }

    /// 起切片软件那两位的**位值**（2026-10-20）：少一位就是"MKP 一关 BBS 跟着没"。
    #[test]
    fn the_slicer_is_started_outside_our_console_and_process_group() {
        let flags = windows_creation_flags();
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        assert_eq!(
            flags & DETACHED_PROCESS,
            DETACHED_PROCESS,
            "不继承控制台（关终端 / 关控制台不该带走它）"
        );
        assert_eq!(
            flags & CREATE_NEW_PROCESS_GROUP,
            CREATE_NEW_PROCESS_GROUP,
            "自成一组（Ctrl+C 的广播不该落到它头上）"
        );
    }

    #[test]
    fn the_system_choice_has_no_place_of_its_own() {
        /* 它交给系统关联程序，没有"我们去找"这一步 */
        assert!(slicer_candidates(SlicerKind::System).is_empty());
        assert!(locate(SlicerKind::System).is_none());
    }

    #[test]
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    fn the_two_slicers_each_look_at_a_path_named_after_itself() {
        for kind in [SlicerKind::Bambu, SlicerKind::Orca] {
            let candidates = slicer_candidates(kind);
            assert!(!candidates.is_empty(), "{:?} 至少要有一个落脚", kind);
            /* 末段必须是那个应用自己的名字 —— 不是随便一个目录 */
            let name = candidates[0]
                .file_name()
                .expect("候选一定带末段")
                .to_string_lossy()
                .to_lowercase();
            let want = match kind {
                SlicerKind::Bambu => "bambu",
                _ => "orca",
            };
            assert!(name.contains(want), "{name} 里没有 {want}");
        }
    }

    #[test]
    fn a_quoted_command_line_gives_the_exe_untouched() {
        /* 系统自己写的形状：引号里的才是可执行物，后面那些是参数 */
        assert_eq!(
            exe_from_command_line(r#""C:\Program Files\Bambu Studio\bambu-studio.exe" "%1""#)
                .as_deref(),
            Some(r"C:\Program Files\Bambu Studio\bambu-studio.exe")
        );
        assert_eq!(
            exe_from_command_line(r#"  "C:\tools\app.exe"  "#).as_deref(),
            Some(r"C:\tools\app.exe")
        );
    }

    #[test]
    fn an_unquoted_command_line_gives_up_at_the_first_exe_suffix() {
        /* 没引号：只能认后缀（路径里的空格与参数的空格在这儿本来就分不开） */
        assert_eq!(
            exe_from_command_line(r"C:\tools\orca-slicer.exe %1").as_deref(),
            Some(r"C:\tools\orca-slicer.exe")
        );
        assert_eq!(
            exe_from_command_line(r"C:\tools\launch.cmd --file %1").as_deref(),
            Some(r"C:\tools\launch.cmd")
        );
    }

    #[test]
    fn a_command_line_we_cannot_read_says_so_instead_of_guessing() {
        for bad in ["", "   ", r#""""#, r#""" "%1""#, "nonsense"] {
            assert!(
                exe_from_command_line(bad).is_none(),
                "{bad:?} 该被认成读不出来"
            );
        }
    }

    #[test]
    fn the_fallback_name_strips_the_bundle_shell() {
        assert_eq!(
            app_stem(Path::new("/Applications/PrusaSlicer.app")),
            "PrusaSlicer"
        );
        /* 大小写不敏感也要锯得对 */
        assert_eq!(app_stem(Path::new("/Applications/PRUSA.APP")), "PRUSA");
        /* Windows 那半给的是裸文件名（`Path::file_name` 认 `\`，不必在这儿再分一次段） */
        assert_eq!(app_stem(Path::new("orca-slicer.exe")), "orca-slicer");
        /* 没有那层壳就原样（不是"猜一个"） */
        assert_eq!(app_stem(Path::new("/usr/bin/open")), "open");
    }

    /// 「去系统里改」那把口令：路径**只出现在最后一个实参里**，三段口令里一个字都不带它。
    /// 这条钉的是"口令不会被路径里的引号/反斜杠改样"（路径不由我们定）。
    #[test]
    fn the_finder_script_takes_the_path_as_an_argument() {
        let args = osascript_information_window_args(Path::new("/tmp/a b\".3mf"));
        assert_eq!(args.len(), 7, "三条 -e 各两段 + 末尾那个路径");
        let script: String = args[..6]
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(
            script.contains("open information window"),
            "口令说的得是「显示简介」"
        );
        assert!(
            !script.contains("a b") && !script.contains(".3mf"),
            "路径不许进口令（它走 argv）：{script}"
        );
        assert_eq!(args[6], std::ffi::OsString::from("/tmp/a b\".3mf"));
    }

    /// 点了「不允许」那条退路（2026-10-16 作者口径）：码要是 `PERMISSION_DENIED`，
    /// 话里必须带着**他接着要做的三步**（"找不到在哪儿改"的人就靠这一句）。
    #[test]
    #[cfg(target_os = "macos")]
    fn a_denied_permission_still_tells_the_user_the_three_steps() {
        let err = information_window_failure(
            "-1743 Not authorized to send Apple events",
            "已经帮你在访达里定位到那份文件",
        );
        assert_eq!(err.code, crate::error::ErrorCode::PermissionDenied);
        for want in ["不允许", "显示简介", "打开方式", "更改全部…"] {
            assert!(
                err.message.contains(want),
                "这句话里没有「{want}」：{}",
                err.message
            );
        }
        /* 别的失败（比如 osascript 起不来）也得给那三步，只是语气没那么具体 */
        let other = information_window_failure("No such file or directory", "没能在访达里定位它");
        assert_eq!(other.code, crate::error::ErrorCode::Io);
        assert!(other.message.contains("更改全部…"), "{}", other.message);
    }

    /// 真机冒烟：这台机器怎么答都算过（CI 上没有关联程序 ⇒ `None`），
    /// 但**答出来的那一个必须形状对** —— 这条判据两台机器上都成立。
    #[test]
    #[cfg(target_os = "macos")]
    fn the_answer_is_either_nothing_or_a_well_formed_app() {
        if let Some(app) = system_default_app() {
            assert!(!app.name.trim().is_empty(), "名字不能是空的");
            assert!(
                Path::new(&app.path).is_absolute(),
                "路径必须是绝对的：{}",
                app.path
            );
        }
    }
}
