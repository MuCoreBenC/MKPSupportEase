//! 「标注台」的**源稿**（2026-10-16，作者要的那一份"像 PS 那样能再编辑、跟着仓库走"）。
//!
//! > 「那个源文件吧，就像 PS 里面的那种源文件呢，可以再次编辑的那一种，你也应该保存到
//! >   我的工作台那里面吧，而且我希望它是**随着这个项目仓库**的，而不是说到我本地的。
//! >   这样，我换了设备也可以继续编辑生成新的图片」
//!
//! # 它住哪儿，为什么
//!
//! `presets/assets/guides/open-with.anno.json` —— **与它描述的那两张图同层**（作者定的）。
//! 好处是换图 / 重裁时它就在旁边：一眼能看出"这份标注属于哪一对图"，也最容易发现
//! "换了图却没重量坐标"（那是 2026-10-16 真栽过的事故，见 `openWithGuide.ts` 头注）。
//!
//! | | |
//! |---|---|
//! | 谁读 | 工作台「标注台」；**客户端不读**（它读的是 `openWithGuide.ts` 里导出的那一段） |
//! | 进不进发布 | **不进**：它不是 `[[assets]]` 条目、不进 `files[]` ⇒ 天然不在交付集合里（判据另有） |
//! | 谁写 | 只有本模块这两条命令（`fsx::atomic::atomic_write`，坏值拒绝） |
//! | 时间 | `updatedAt` 由**后端**盖（`workbench::clock`）——前端不自己造时间 |
//!
//! # 为什么路径由后端算
//!
//! 前端只给**资产 id**，落点从台账那条资产的 `path` 派生（[`Assets::resolve`] 本来就会
//! 挡住越界路径）。前端要是能自己拼路径，"标注写哪儿"就有了第二个真相源 —— 与
//! 「文件名由谁定、路径就从谁那个源取」是同一条纪律。
//!
//! # 一组图共用一份
//!
//! 浅 / 深两张是**一组**（`-light` / `-dark` 两个 id），共用一份标注 —— 所以源稿的名字
//! 取"抹掉浅深后缀"的那个干名：`open-with-light.png` / `open-with-dark.png` ⇒
//! `open-with.anno.json`。★ 这条口径与工作台 `pairOf()` 必须一致（客户端另按源码常量
//! 的 `LIGHT_ID` / `DARK_ID` 消费，不读源稿）；改一处就要对一遍。
//!
//! ★★ 「一组」的**全部成员**记在 `AnnoSource.images` 里（2026-10-16 泛化）：落点仍按
//! 干名派生（所以浅 / 深两个 id 指向**同一份**），但源稿自己声明这一组有哪几张 ——
//! 将来一组三张以上（macOS + Windows 同一套引导）时，落点与语义都不用再改。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_json;
use crate::ipc::traced;
use crate::workbench::clock;

use super::with_ctx;

/// 源稿的格式代次。不认就拒 —— 与 catalog / 其它入库产物同一条规矩：
/// 往前（旧版读新版）与往后（新版读旧版）都不许静默错读。
pub const ANNO_SCHEMA: u32 = 1;

/// 源稿的扩展名（摆在图旁边：`open-with.anno.json`）
const ANNO_EXT: &str = ".anno.json";

/// 坐标的百分比范围（留一点点浮点余量：`left + width` 允许到 100.5）
const PCT_MAX: f64 = 100.0;
const PCT_SLOP: f64 = 0.5;
/// 样式值的上限（px）。给个"一眼就知道填错了"的天花板，不是精确判据
const STYLE_MAX: f64 = 400.0;

/// 一个框的位置与大小（**百分比**，相对整张图 —— 与客户端 `GuideMark.at` 同形）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoBox {
    pub left: f64,
    pub top: f64,
    pub width: f64,
    pub height: f64,
}

/// 一条标注画出来什么样（全可选；一个都不写 = 站点缺省：圆角 8 / 线宽 2 / 编号 18）。
/// ★ 颜色不在这儿：颜色必须跟主题走（浅深两套 token），写进标注会和主题系统打架。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub badge: Option<f64>,
}

/// 图上的一块标注
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoMark {
    /// 框角上那颗编号（从 1 起，一份里不许重号）
    pub step: u32,
    pub at: AnnoBox,
    /// 给自己看的名字（只进导出那行注释，不进客户端契约）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<AnnoStyle>,
}

/// 这份标注画在**哪几张图上**（路径 = **资产根**里的相对路径）。
///
/// ★ 2026-10-16 泛化：原来写死 `light` / `dark` 两个字段（浅 / 深一对），改成
/// **一个路径列表** —— 作者：「到时候可能真的超过 3 张……macOS 的呀，Windows 的呀，
/// 他们都是同一组那种感觉」。一份源稿＝一组，`images` 就是这一组的全部成员；
/// 浅 / 深那一对只是"这张列表现在恰好有两个成员"的特例。
/// ★ 没发布、没有兼容包袱 ⇒ 原地改形状，**代次仍是 1**（不写 v1→v2 迁移）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoSource {
    /// 这一组图（同一份标注画上去的那些）—— 顺序就是"组内次序"
    pub images: Vec<String>,
    /// 量这份标注时图的原生尺寸 —— 换图 / 重裁之后对一下就知道"框会飘"
    pub size: AnnoSize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoSize {
    pub w: u32,
    pub h: u32,
}

/// 一份标注源稿（入库的那一份）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnnoDoc {
    pub anno_schema: u32,
    pub source: AnnoSource,
    pub marks: Vec<AnnoMark>,
    /// 后端盖的时间（UTC、秒级、带 `Z`）
    pub updated_at: String,
}

/// 读回来的样子：源稿 + 它在仓库里的落点（界面要显示"这份在哪"）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnnoView {
    /// **没有源稿是合法状态**（第一次进标注台、或还没拖过的那张图）⇒ `null`，不报错
    pub doc: Option<AnnoDoc>,
    /// 相对资产根的落点，例如 `guides/open-with.anno.json`
    pub path: String,
    /// **标注台该显示的那张图**（相对资产根）—— **原图**，不是台账那条成品图
    /// （2026-10-16 方案 A：台账 `path` 指成品 `.rendered.webp`，框已烧进像素；
    /// 在它上面再画框就是"框上叠框"）。
    ///
    /// 判据（唯一一处，见 [`source_image_rel_of`]）：原图在**同层 `source/` 子目录**里、
    /// 同名同后缀；找不到就回落台账那条（老数据没挪过 source/ 时仍能标）。
    pub source_image: String,
}

/// 标注台该显示的那张**原图**（相对资产根）。
///
/// 两条判据，按序：
/// 1. **源稿里记着**（`doc.source.images`）：找到与这条资产**同干名**的那条
///    （`open-with-light.png` 的干名 `open-with-light`）—— 真值来源
///    （原图可以有任意后缀，只有源稿知道它叫什么）；
/// 2. 源稿里没有 / 记的是**成品图** ⇒ 回落**同层 `source/` 子目录 + 同干名 + `.png`**：
///    `guides/open-with-light.rendered.webp` ⇒ `guides/source/open-with-light.png`。
///
/// ★★★ **绝不返回成品图**（2026-10-16 作者：「标注台显示的**不是原本的图片**，是已经被改过的
/// 图片……右边把眼睛关了它还显示」）。病根：源稿在方案 A 之后记的就是 `.rendered.webp`，
/// 而第一版判据 ①"源稿里记着就信它" ⇒ 直接拿到了**烧好框的成品**——
/// 于是"关掉眼睛"也不管用（框是像素里就有的），标注台也变成了"在成品上再画框"。
/// 现在：判据 ① 里**跳过任何成品路径**（[`crate::workbench::app::render::RENDER_EXT`]），
/// 于是**永远显示原图**；要看成品去「客户视角」（那是另一条路，见 `AnnotatePage`）。
///
/// ★ 干名用 `RENDER_EXT` 也抹一次 —— 台账那条是 `x.rendered.webp`，干名要得到 `x`。
fn source_image_rel_of(asset_rel: &str, doc: Option<&AnnoDoc>) -> String {
    let stem_of = |rel: &str| -> String {
        let base = std::path::Path::new(rel)
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stripped = base
            .strip_suffix(super::render::RENDER_EXT)
            .unwrap_or(&base)
            .to_owned();
        std::path::Path::new(&stripped)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let is_rendered = |rel: &str| rel.ends_with(super::render::RENDER_EXT);
    let want = stem_of(asset_rel);

    // ① 源稿里记着的那张（同干名）—— 真值，但**成品图不算**（那正是要避开的东西）
    if let Some(d) = doc {
        if let Some(hit) = d
            .source
            .images
            .iter()
            .find(|p| stem_of(p) == want && !is_rendered(p))
        {
            return hit.clone();
        }
    }

    // ② 同层 `source/` 子目录 + 同干名 + `.png` 兜底
    let p = std::path::Path::new(asset_rel);
    let dir = p.parent().unwrap_or_else(|| std::path::Path::new(""));
    let cand = if dir.file_name().and_then(|s| s.to_str()) == Some(super::render::SOURCE_SUBDIR) {
        /* 台账那条本来就在 source/ 里（老数据）⇒ 同干名换 .png 原位找 */
        dir.join(format!("{want}.png"))
    } else {
        dir.join(super::render::SOURCE_SUBDIR)
            .join(format!("{want}.png"))
    };
    cand.to_string_lossy().replace('\\', "/")
}

/// 源稿的落点：**同级目录 + [`stem`] + `.anno.json`**。
///
/// ★ `stem` **不在这里算** —— 它是"这一组叫什么"，由台账说了算
/// （[`crate::presetdata::Assets::anno_stem_of`]：有登记的组 → 组 id；没有 → 旧口径
/// 抹掉浅深后缀兜底）。判据**只有那一处**，这里只管拼路径。
fn doc_rel_of(asset_rel: &str, stem: &str) -> Result<(String, PathBuf), AppError> {
    if stem.trim().is_empty() {
        return Err(
            AppError::invalid_argument(format!("这条件名算不出标注干名：{asset_rel}")).with_detail(
                "干名要么是它所属分组的 id，要么是文件名抹掉浅 / 深后缀 —— 两个都算不出来"
                    .to_owned(),
            ),
        );
    }
    let rel = Path::new(asset_rel);
    let dir = rel.parent().unwrap_or_else(|| Path::new(""));
    let rel_out = dir.join(format!("{stem}{ANNO_EXT}"));
    Ok((rel_out.to_string_lossy().replace('\\', "/"), rel_out))
}

/// 写之前把这份源稿从头到脚看一遍 —— **坏值拒绝，不写下去**
/// （入库的东西宁可不写，也不能留一份"看着有、其实读不了"的）
fn check(doc: &AnnoDoc) -> Result<(), AppError> {
    if doc.anno_schema != ANNO_SCHEMA {
        return Err(
            AppError::invalid_argument(format!("源稿格式代次不认：{}", doc.anno_schema))
                .with_detail(format!("这一版只认 {ANNO_SCHEMA}")),
        );
    }
    if doc.source.size.w == 0 || doc.source.size.h == 0 {
        return Err(AppError::invalid_argument("源稿没记图的尺寸"));
    }
    if doc.source.images.is_empty() {
        return Err(AppError::invalid_argument("源稿没记它画在哪几张图上"));
    }
    if doc.source.images.iter().any(|p| p.trim().is_empty()) {
        return Err(AppError::invalid_argument("源稿里有一张图没写路径"));
    }

    let mut seen: Vec<u32> = Vec::new();
    for (i, m) in doc.marks.iter().enumerate() {
        let n = i + 1;
        let bad = |why: &str| AppError::invalid_argument(format!("第 {n} 个框{why}"));
        if m.step == 0 || m.step > 99 {
            return Err(bad("的编号不在 1..99"));
        }
        if seen.contains(&m.step) {
            return Err(bad("的编号和别的框重了"));
        }
        seen.push(m.step);

        let at = &m.at;
        if !at.left.is_finite()
            || !at.top.is_finite()
            || !at.width.is_finite()
            || !at.height.is_finite()
        {
            return Err(bad("的坐标不是数"));
        }
        if at.width <= 0.0 || at.height <= 0.0 {
            return Err(bad("宽或高不是正数"));
        }
        if at.left < 0.0 || at.top < 0.0 {
            return Err(bad("跑到图外面去了（左 / 上是负的）"));
        }
        if at.left + at.width > PCT_MAX + PCT_SLOP || at.top + at.height > PCT_MAX + PCT_SLOP {
            return Err(bad("跑到图外面去了（超出 100%）"));
        }

        if let Some(style) = &m.style {
            for (what, v) in [
                ("圆角", style.radius),
                ("线宽", style.stroke),
                ("编号直径", style.badge),
            ] {
                if let Some(v) = v {
                    if !v.is_finite() || v <= 0.0 || v > STYLE_MAX {
                        return Err(bad(&format!("的{what}不在 0~{STYLE_MAX} 之间")));
                    }
                }
            }
        }
    }
    Ok(())
}

/// 读一处标注的源稿。**没有源稿不是错误**（返回 `null`）—— 第一次进标注台就是这样。
#[tauri::command(async)]
pub fn wb_anno_read(asset_id: String) -> Result<AnnoView, AppError> {
    traced("wb_anno_read", |_| {
        with_ctx(|ctx| {
            let asset =
                ctx.presets.assets.get(&asset_id).ok_or_else(|| {
                    AppError::not_found(format!("台账里没有这条资产：{asset_id}"))
                })?;
            let stem = ctx
                .presets
                .assets
                .anno_stem_of(&asset.id)
                .unwrap_or_default();
            let (rel, rel_path) = doc_rel_of(&asset.path, &stem)?;
            let file = ctx.presets.assets.resolve(asset)?.with_file_name(
                rel_path
                    .file_name()
                    .map(|s| s.to_owned())
                    .unwrap_or_default(),
            );

            if !file.exists() {
                return Ok(AnnoView {
                    source_image: source_image_rel_of(&asset.path, None),
                    doc: None,
                    path: rel,
                });
            }
            let bytes = std::fs::read(&file).map_err(|e| {
                AppError::io(format!("读不了源稿 {rel}")).with_detail(e.to_string())
            })?;
            let doc: AnnoDoc = serde_json::from_slice(&bytes).map_err(|e| {
                AppError::corrupted(format!("源稿 {rel} 读不出来")).with_detail(e.to_string())
            })?;
            Ok(AnnoView {
                source_image: source_image_rel_of(&asset.path, Some(&doc)),
                doc: Some(doc),
                path: rel,
            })
        })
    })
}

/// 一组标注要烧的每一张：`(原图相对路径, 成品相对路径)`。
///
/// - **输入走 [`source_image_rel_of`]**（永远原图，绝不拿上一版成品当输入）；
/// - **输出**：一张成员时就用资产那条 `path` 推（`guides/x.rendered.webp`）；
///   多张成员时按各自原图推（浅 / 深各归各）。
fn render_targets(doc: &AnnoDoc, asset_path: &str) -> Vec<(String, String)> {
    let mut targets: Vec<(String, String)> = Vec::new();
    for img_rel in &doc.source.images {
        let original = source_image_rel_of(img_rel, Some(doc));
        let out_rel = if doc.source.images.len() > 1 {
            super::render::rendered_rel_of(&original)
        } else {
            super::render::rendered_rel_of(asset_path)
        };
        if !targets.iter().any(|(o, _)| *o == original) {
            targets.push((original, out_rel));
        }
    }
    if targets.is_empty() {
        targets.push((
            source_image_rel_of(asset_path, Some(doc)),
            super::render::rendered_rel_of(asset_path),
        ));
    }
    targets
}

/// 把这一组标注**按参考显示比例**烘成成品图字节：`(原图相对路径, 成品相对路径, webp 字节)`。
///
/// `display_scale` = **这份成品图的参考显示比例**（客户端把这张图显示多大，
/// 即 `314 / 图宽`）—— 见 `render::render_webp`。**与标注台当前缩放无关**。读的是**原图**。
///
/// ★★★ `root` = **资产载荷根，由调用方从已经借到的 `ctx` 里取好传进来** ——
/// 这个函数**自己绝不 `with_ctx`**（那是 `Mutex`，不可重入：调用方已经在 `with_ctx` 里，
/// 这里再借一次就是**自己等自己** ⇒ 永久卡住。2026-10-16 作者实测「一点生成预览它就一直转圈」）。
///
/// 渲不出来的那几张**只喊一声、不整次失败**（原图被挪走之类 —— 不该把调用方拖红）。
fn render_group(
    root: &Path,
    doc: &AnnoDoc,
    asset_path: &str,
    display_scale: f64,
) -> Result<Vec<(String, String, Vec<u8>)>, AppError> {
    let mut out = Vec::new();
    for (original, out_rel) in render_targets(doc, asset_path) {
        let src = root.join(&original);
        if !src.is_file() {
            eprintln!("[anno] 原图不在，跳过渲成品图：{}", src.display());
            continue;
        }
        let bytes = match std::fs::read(&src) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("[anno] 原图读不出来（{}）：{e}", src.display());
                continue;
            }
        };
        match super::render::render_webp(&bytes, doc, &original, display_scale) {
            Ok(webp) => out.push((original, out_rel, webp)),
            Err(e) => eprintln!("[anno] 渲成品图失败（{original}）：{e}"),
        }
    }
    Ok(out)
}

/// **生成预览**（2026-10-16 作者：「就是我觉得它生成一个预览的，然后我在客户视角下去看没问题了。
/// 我点保存它就存进去，不然它就只是一个临时的……就等于说渲染一样啊，拍一张照只是不存回
/// 我们的相册这种感觉」）。
///
/// 按**参考显示比例**（`display_scale`，与标注台当前缩放**无关**）把标注烘成成品图，
/// **只回字节、一个盘都不写** —— 前端拿去做 `blob:` URL，在「客户视角」里预览。
/// 这就是"拍照不存相册"。
#[tauri::command]
pub fn wb_anno_render(
    asset_id: String,
    doc: AnnoDoc,
    display_scale: f64,
) -> Result<Vec<RenderedShot>, AppError> {
    traced("wb_anno_render", |_| {
        with_ctx(|ctx| {
            let asset =
                ctx.presets.assets.get(&asset_id).ok_or_else(|| {
                    AppError::not_found(format!("台账里没有这条资产：{asset_id}"))
                })?;
            /* 预览也要过一遍校验 —— 免得给一张框飘出画布的"预览照" */
            check(&doc)?;
            /* ★ `root` 从**已经借到的** `ctx` 里取（`render_group` 自己不再 `with_ctx`，见它的头注） */
            let root = ctx
                .presets
                .assets
                .asset_root()
                .ok_or_else(|| AppError::io("这一侧没有资产载荷根，渲不了成品图"))?
                .to_path_buf();
            let shots = render_group(&root, &doc, &asset.path, display_scale)?;
            Ok(shots
                .into_iter()
                .map(|(original, out_rel, webp)| RenderedShot {
                    original,
                    out: out_rel,
                    /* 前端拿去做 `new Blob([...])` —— 这里给 base64（IPC 只能传 JSON） */
                    bytes_b64: b64(&webp),
                })
                .collect())
        })
    })
}

/// 一张烘好的图：原图是谁、成品该落哪、字节（base64）。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedShot {
    /// 输入用哪张原图（相对资产根）
    pub original: String,
    /// 成品该落在哪（相对资产根）—— 预览时前端只当标识，保存时后端照这个写
    pub out: String,
    /// webp 字节（base64）
    pub bytes_b64: String,
}

/// 极简 base64（只给预览回传用；不引依赖）
fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// 写一处标注的源稿（**入库的那一份**）＋ **同趟烘成品图**。
///
/// 路径由后端从台账派生；坏值拒绝、不写下去。`updatedAt` 由这里盖章（前端不造时间）。
///
/// `display_scale` = **这份成品图的参考显示比例**（客户端把这张图显示多大，即 `314 / 图宽`）——
/// 同一份标注不论在哪个视角保存，烧出来**一模一样**（见 `render::render_webp` 的 `DEFAULT_STROKE` 那段）。
#[tauri::command]
pub fn wb_anno_write(
    asset_id: String,
    doc: AnnoDoc,
    display_scale: f64,
) -> Result<AnnoView, AppError> {
    traced("wb_anno_write", |_| {
        with_ctx(|ctx| {
            let asset =
                ctx.presets.assets.get(&asset_id).ok_or_else(|| {
                    AppError::not_found(format!("台账里没有这条资产：{asset_id}"))
                })?;
            let stem = ctx
                .presets
                .assets
                .anno_stem_of(&asset.id)
                .unwrap_or_default();
            let (rel, rel_path) = doc_rel_of(&asset.path, &stem)?;
            let file = ctx.presets.assets.resolve(asset)?.with_file_name(
                rel_path
                    .file_name()
                    .map(|s| s.to_owned())
                    .unwrap_or_default(),
            );

            let mut doc = doc;
            doc.updated_at = clock::now_iso8601();
            check(&doc)?;

            if let Some(dir) = file.parent() {
                std::fs::create_dir_all(dir).map_err(|e| {
                    AppError::io(format!("建不了目录 {}", dir.display())).with_detail(e.to_string())
                })?;
            }
            atomic_write_json(&file, &doc)?;

            /*
             * ★★★ 同趟烘成品图（2026-10-16，方案 A ＋ 作者定口径）。
             *
             * 保存 = **存进相册**：源稿落盘（可再编辑的那一份）＋ 成品图按**当前视角**烧好落盘。
             * 不想落盘只要看效果时先点「生成预览」（`wb_anno_render`，一个盘都不写）。
             *
             * 取舍：
             * - **源稿先落、成品后渲**：源稿是"能再编辑的那一份"，任何一步失败它都已经在盘上；
             * - **渲不出来只喊一声、不整次失败**（原图被挪走之类不该把"保存源稿"回滚）；
             * - **渲输入永远走原图**（`render_group` 里那条注释写了为什么）。
             */
            let root = ctx.presets.assets.asset_root().map(|p| p.to_path_buf());
            if let Some(root) = root {
                match render_group(&root, &doc, &asset.path, display_scale) {
                    Ok(shots) => {
                        for (original, out_rel, webp) in shots {
                            let out = root.join(&out_rel);
                            if let Some(dir) = out.parent() {
                                if let Err(e) = std::fs::create_dir_all(dir) {
                                    eprintln!("[anno] 建不了目录 {}：{e}", dir.display());
                                    continue;
                                }
                            }
                            if let Err(e) = crate::fsx::atomic::atomic_write(&out, &webp) {
                                eprintln!("[anno] 成品图写不进去（{out_rel}）：{e}");
                            }
                            let _ = original;
                        }
                    }
                    Err(e) => eprintln!("[anno] 这一趟没烘成品图：{e}"),
                }
            } else {
                eprintln!("[anno] 这一侧没有资产载荷根，跳过渲成品图");
            }

            Ok(AnnoView {
                source_image: source_image_rel_of(&asset.path, Some(&doc)),
                doc: Some(doc),
                path: rel,
            })
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_of(marks: Vec<AnnoMark>) -> AnnoDoc {
        AnnoDoc {
            anno_schema: ANNO_SCHEMA,
            source: AnnoSource {
                images: vec![
                    "guides/open-with-light.png".into(),
                    "guides/open-with-dark.png".into(),
                ],
                size: AnnoSize { w: 754, h: 904 },
            },
            marks,
            updated_at: "2026-10-16T00:00:00Z".into(),
        }
    }

    fn mark(step: u32, left: f64, top: f64, w: f64, h: f64) -> AnnoMark {
        AnnoMark {
            step,
            at: AnnoBox {
                left,
                top,
                width: w,
                height: h,
            },
            note: String::new(),
            style: None,
        }
    }

    /// 标注台该显示**原图**：台账 path 是成品 `.rendered.webp`，原图在同层 `source/` 下
    #[test]
    fn the_annotate_page_points_at_the_source_image_not_the_rendered_one() {
        let mut d = doc_of(vec![]);
        d.source.images = vec![
            "guides/source/open-with-light.png".into(),
            "guides/source/open-with-dark.png".into(),
        ];
        // ① 源稿里记着 ⇒ 用源稿那条（真值）
        assert_eq!(
            source_image_rel_of("guides/open-with-light.rendered.webp", Some(&d)),
            "guides/source/open-with-light.png"
        );
        // ② 没源稿 ⇒ 回落同层 `source/` + 同干名 + `.png`
        assert_eq!(
            source_image_rel_of("guides/open-with-light.rendered.webp", None),
            "guides/source/open-with-light.png"
        );
        // 台账那条本来就在 source/ 里（老数据）⇒ 原位换 .png
        assert_eq!(
            source_image_rel_of("guides/source/open-with-light.png", None),
            "guides/source/open-with-light.png"
        );
    }

    /// ★★★ **源稿记着成品图时，标注台照样显示原图**（2026-10-16 作者截图的 bug）。
    ///
    /// 方案 A 之后源稿的 `source.images` 记的是 `.rendered.webp` —— 判据 ① 若照单全收，
    /// 标注台就显示**烧好框的成品**：关掉眼睛也没用、还会"在成品上再画框"。
    /// 这条钉死"永远给原图"。
    #[test]
    fn the_annotate_page_never_shows_the_rendered_product() {
        let mut d = doc_of(vec![]);
        /* 源稿记的是**成品**（作者保存之后就是这个样子） */
        d.source.images = vec![
            "guides/open-with-light.rendered.webp".into(),
            "guides/open-with-dark.rendered.webp".into(),
        ];
        assert_eq!(
            source_image_rel_of("guides/open-with-light.rendered.webp", Some(&d)),
            "guides/source/open-with-light.png",
            "源稿记着成品 ⇒ 必须回落到 source/ 里的原图，绝不能返回成品"
        );
        /* 源稿记的是原图路径时，照它走（真值） */
        let mut e = doc_of(vec![]);
        e.source.images = vec!["guides/source/open-with-light.jpg".into()];
        assert_eq!(
            source_image_rel_of("guides/open-with-light.rendered.webp", Some(&e)),
            "guides/source/open-with-light.jpg",
            "源稿里记着原图（jpg 这种别的后缀）⇒ 按它走"
        );
    }

    /// ★★★ **`render_group` 不许自己再借一次 ctx**（2026-10-16 作者实测「一点生成预览就卡住转圈」）。
    ///
    /// `with_ctx` 是 `Mutex`、**不可重入** —— 调用方已经在 `with_ctx` 里，里面再借一次
    /// 就是自己等自己、**永久阻塞**。这条用一个带超时的线程把那个死锁钉死：
    /// 在 `with_ctx` 里跑 `render_group`，5 秒内没回来就判失败（原来那版就是永远不回来）。
    #[test]
    fn render_group_does_not_reenter_the_session_lock() {
        use std::sync::mpsc;
        use std::time::Duration;

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            /* ① 借一次 ctx（真机上 `wb_anno_render` 就是这么进来的） */
            let out = super::with_ctx(|ctx| {
                let root = ctx
                    .presets
                    .assets
                    .asset_root()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::path::PathBuf::from("/nonexistent"));
                /* ② 传入 root（**不再自己 `with_ctx`**）—— 老版本这里会永久卡住 */
                let d = AnnoDoc {
                    anno_schema: ANNO_SCHEMA,
                    source: AnnoSource {
                        images: vec!["guides/x.png".into()],
                        size: AnnoSize { w: 10, h: 10 },
                    },
                    marks: vec![],
                    updated_at: "2026-10-16T00:00:00Z".into(),
                };
                let _ = super::render_group(&root, &d, "guides/x.rendered.webp", 1.0);
                Ok(())
            });
            let _ = tx.send(out.is_ok());
        });

        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(ok) => assert!(ok, "render_group 在 ctx 里跑挂了"),
            Err(_) => panic!("render_group 在 `with_ctx` 里重入取锁 ⇒ 永久阻塞（死锁回归）"),
        }
    }

    /// 一组图**共用一份**：干名 = 台账里那个组的 id（2026-10-16 改判 —— 从前是抹文件名）
    #[test]
    fn a_group_shares_one_source_document() {
        let presets = crate::workbench::load_presets().expect("真 presets 读得通");
        let assets = &presets.assets;
        let light = assets
            .anno_stem_of("guide-open-with-light")
            .expect("浅色那张算得出干名");
        let dark = assets
            .anno_stem_of("guide-open-with-dark")
            .expect("深色那张算得出干名");
        assert_eq!(light, "open-with", "干名 = 台账 `[[groups]]` 那个 id");
        assert_eq!(light, dark, "同一组的两张必须算到同一份源稿");

        let (a, _) = doc_rel_of("guides/open-with-light.png", &light).expect("落点算得出来");
        let (b, _) = doc_rel_of("guides/open-with-dark.png", &dark).expect("落点算得出来");
        assert_eq!(a, "guides/open-with.anno.json");
        assert_eq!(a, b, "同一组落点必须一样 —— 一份标注那几张共用");
    }

    /// ★★★ **渲成品图要从"原图"读输入，不能读上一版成品**（2026-10-16 作者：
    /// 「我加了第 3 个框框，按了保存之后，看客户视角，结果发现还是两个框框」）。
    ///
    /// 病根：`source.images` 记的是 `.rendered.webp` ⇒ 直接当输入就会"读旧成品再往上画"。
    /// 这条钉住"渲输入走 `source_image_rel_of`"。
    #[test]
    fn the_render_reads_the_original_not_the_previous_product() {
        let mut d = doc_of(vec![]);
        d.source.images = vec![
            "guides/open-with-light.rendered.webp".into(),
            "guides/open-with-dark.rendered.webp".into(),
        ];
        for img in &d.source.images {
            let input = source_image_rel_of(img, Some(&d));
            assert!(
                !input.ends_with(crate::workbench::app::render::RENDER_EXT),
                "渲输入不许是成品：{input}"
            );
            assert!(
                input.contains("/source/"),
                "该落到 source/ 下的原图：{input}"
            );
        }
    }

    /// **没进组的图仍能单独标一下**（旧口径兜底）：干名 = 文件名抹掉浅深后缀
    #[test]
    fn an_ungrouped_image_falls_back_to_the_old_stem() {
        let presets = crate::workbench::load_presets().expect("真 presets 读得通");
        let assets = &presets.assets;
        /* 挑一条**不在任何组里**的资产：图标（`icons/a1.svg` ⇒ `a1`） */
        let some = assets
            .items()
            .iter()
            .find(|a| assets.group_of(&a.id).is_none() && !a.path.trim().is_empty())
            .expect("台账里总有没进组的资产");
        let stem = assets.anno_stem_of(&some.id).expect("兜底也算得出干名");
        assert_eq!(
            stem,
            crate::presetdata::assets::pair_stem(
                std::path::Path::new(some.path.trim())
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
            ),
            "没进组的走旧口径（抹浅深后缀）"
        );
    }

    /// **源稿不是资产、更不许进交付**（作者 2026-10-16 定的："发布时排除，不发给用户"）。
    ///
    /// 它压根不是 `[[assets]]` 条目 ⇒ 天然不在交付集合里 —— 但这条判据要把它钉死：
    /// 哪天有人"顺手"把 `.anno.json` 也登记成一条资产，或让交付按目录整拷，
    /// 这条会立刻红，而不是等用户在包里发现一份没人读的 JSON。
    #[test]
    fn the_source_document_is_neither_an_asset_nor_a_delivery_item() {
        let presets = crate::workbench::load_presets().expect("真 presets 读得通");
        let assets = &presets.assets;

        for a in assets.items() {
            assert!(
                !a.path.ends_with(ANNO_EXT),
                "源稿不许登记成资产：{}（{}）",
                a.id,
                a.path
            );
            if let Some(dest) = crate::runtime::catalog::dest_of_asset(a) {
                assert!(!dest.ends_with(ANNO_EXT), "源稿不许进交付目录：{dest}");
            }
        }

        /* 引导图那一对：源稿该落在它旁边，而那条路径**不在**任何资产条目里 */
        let light = assets
            .get("guide-open-with-light")
            .expect("引导图浅色那条要在台账里");
        let stem = assets
            .anno_stem_of(&light.id)
            .expect("干名算得出来（它在 `[[groups]]` 那一组里）");
        let (rel, _) = doc_rel_of(&light.path, &stem).expect("落点算得出来");
        assert_eq!(rel, "guides/open-with.anno.json");
        assert!(
            assets.items().iter().all(|a| a.path != rel),
            "源稿的落点撞上了某条资产 —— 那就是它进了交付集合"
        );
    }

    /// **仓库里那份真的源稿**要能读、能过校验（`presets/assets/guides/open-with.anno.json`）。
    ///
    /// 与"真台账读得通"同一条纪律：入库的东西由判据盯着 —— 手改坏了（或字段名改了）
    /// 当场红，而不是等下一次在标注台里打开才发现读不出来。
    #[test]
    fn the_real_source_document_parses_and_passes_checks() {
        let presets = crate::workbench::load_presets().expect("真 presets 读得通");
        let light = presets
            .assets
            .get("guide-open-with-light")
            .expect("引导图浅色那条要在台账里");
        let file = presets
            .assets
            .resolve(light)
            .expect("路径解析得出来")
            .with_file_name("open-with.anno.json");
        if !file.exists() {
            /* 还没拖过就是没有源稿 —— 那是合法状态，不是失败 */
            return;
        }
        let bytes = std::fs::read(&file).expect("源稿读得出来");
        let doc: AnnoDoc = serde_json::from_slice(&bytes).expect("源稿解析得出来");
        check(&doc).expect("源稿过得了校验");
        assert_eq!(doc.source.size.w, 754, "这份是照 754×904 量的");
        /*
         * 框的数量**跟着作者在标注台里拖出来的走**（这一条只是"读得出来且有条数"的
         * 形态判据，不是"必须有 2 个"）。第一版写的 `== 2`，作者当天加了第 3 个框就红了 ——
         * 那不是他错，是这条判据把"作者拖了几个"当成了契约。改成"至少 1 个、编号从 1 连续"。
         */
        assert!(!doc.marks.is_empty(), "源稿至少该有一个框");
        let steps: Vec<u32> = doc.marks.iter().map(|m| m.step).collect();
        let mut sorted = steps.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(steps.len(), sorted.len(), "编号不许重：{steps:?}");
        /*
         * 记的是这一组的**两张**（浅 / 深）。★ 后缀不钉死：源稿在 2026-10-16 方案 A 之后
         * 记的是成品路径（`.rendered.webp`），之前是原图（`.png`）—— 判据认**干名**就够，
         * 认后缀只会把"换交付格式"这种正当改动判成红。
         */
        assert!(
            doc.source
                .images
                .iter()
                .any(|p| p.contains("open-with-light"))
                && doc
                    .source
                    .images
                    .iter()
                    .any(|p| p.contains("open-with-dark")),
            "源稿要写清它画在哪几张上：{:?}",
            doc.source
        );
    }

    /// 坏值一律拒绝：写进仓库的东西"看着有、其实读不了"比没写更坏
    #[test]
    fn check_rejects_broken_documents() {
        assert!(
            check(&doc_of(vec![mark(1, 10.0, 10.0, 20.0, 5.0)])).is_ok(),
            "正常的一份要过"
        );
        assert!(
            check(&doc_of(vec![])).is_ok(),
            "一个框都没有也合法（清空过）"
        );

        /* 编号 */
        assert!(
            check(&doc_of(vec![mark(0, 1.0, 1.0, 2.0, 2.0)])).is_err(),
            "编号从 1 起"
        );
        assert!(
            check(&doc_of(vec![
                mark(1, 1.0, 1.0, 2.0, 2.0),
                mark(1, 5.0, 5.0, 2.0, 2.0)
            ]))
            .is_err(),
            "重号要拒"
        );

        /* 几何 */
        assert!(
            check(&doc_of(vec![mark(1, -1.0, 1.0, 2.0, 2.0)])).is_err(),
            "负坐标要拒"
        );
        assert!(
            check(&doc_of(vec![mark(1, 1.0, 1.0, 0.0, 2.0)])).is_err(),
            "0 宽要拒"
        );
        assert!(
            check(&doc_of(vec![mark(1, 90.0, 1.0, 20.0, 2.0)])).is_err(),
            "伸出图外要拒"
        );
        assert!(
            check(&doc_of(vec![mark(1, 90.0, 1.0, 10.5, 2.0)])).is_ok(),
            "贴到边（含浮点余量）要放行"
        );

        /* 样式 */
        let mut styled = doc_of(vec![mark(1, 1.0, 1.0, 2.0, 2.0)]);
        styled.marks[0].style = Some(AnnoStyle {
            radius: Some(0.0),
            ..AnnoStyle::default()
        });
        assert!(check(&styled).is_err(), "圆角 0 要拒");
        styled.marks[0].style = Some(AnnoStyle {
            badge: Some(500.0),
            ..AnnoStyle::default()
        });
        assert!(check(&styled).is_err(), "编号直径大得离谱要拒");

        /* 一组有几张图（`images`）—— 2026-10-16 泛化后至少一张、且每张都要有路径 */
        let mut no_img = doc_of(vec![]);
        no_img.source.images.clear();
        assert!(check(&no_img).is_err(), "一张图都没记要拒");
        let mut blank_img = doc_of(vec![]);
        blank_img.source.images = vec!["".into()];
        assert!(check(&blank_img).is_err(), "空路径要拒");
        let mut three = doc_of(vec![mark(1, 1.0, 1.0, 2.0, 2.0)]);
        three.source.images = vec![
            "guides/a.png".into(),
            "guides/b.png".into(),
            "guides/c.png".into(),
        ];
        assert!(
            check(&three).is_ok(),
            "一组三张（将来 macOS + Windows）也要过"
        );

        /* 代次与尺寸 */
        let mut future = doc_of(vec![]);
        future.anno_schema = ANNO_SCHEMA + 1;
        assert!(check(&future).is_err(), "未来的代次不许静默读");
        let mut no_size = doc_of(vec![]);
        no_size.source.size = AnnoSize { w: 0, h: 904 };
        assert!(check(&no_size).is_err(), "没记尺寸要拒（换图时全靠它提醒）");
    }
}
