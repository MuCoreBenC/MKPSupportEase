//! 把标注**烧进图片**——产出交付给客户端的**成品图**（2026-10-16，方案 A）。
//!
//! > 作者：「有没有必要说它是这个渲染进图片里面呀？不是说靠拼接，而是直接渲染成一张图片。
//! >   就是说客户端收到的不是原文件，而是最终的渲染」、「我点了保存之后，它就立马就渲好
//! >   一张图片」。
//!
//! # 为什么要有这一步
//!
//! 在那之前，客户端是「读原图 + 前端按 `marks` 现画框」（`GuideImage.tsx`）—— 作者在客户端
//! 放大时看出**框是"贴上去的"**（框边与图分离、缩放时会露出图层感）。成品图把框烧进像素，
//! 客户端 `<img>` 直接显示，**跨平台 / 跨系统显示一致**（作者：「因为它已经被渲染成图片了，
//! 在哪显示都一样」）。
//!
//! # 谁还在乎原图
//!
//! 原图与源稿（`.anno.json`）像 PS 的 PSD 一样**自己留存、可再编辑** —— 它们仍住
//! `presets/assets/guides/`（源稿在同一层），只是**不进客户端交付**；成品图才是交付物。
//!
//! # 格式：**无损 WebP**（纯 Rust，无 C 依赖）
//!
//! 实测（`cwebp` 对真图 754×904）：原 PNG 309.5 KB → 无损 webp **77.9 KB**（小 75%）；
//! 有损 q90 只再省 ~7 KB ⇒ 不值得为这点体积引入 libwebp（三平台 CI 都要能编）。
//! `image` crate 的 webp 编码器走 `image-webp`，是**纯 Rust 无损**实现。
//!
//! # 画的规矩**必须与客户端一致**
//!
//! 成品图看起来要跟客户端原来"现画"的一模一样，否则改交付方式就成了改视觉。三件事对齐
//! （客户端在 `GuideImage.module.css` 的 `.mark` / `.badge`）：
//!
//! - 框线：强调色 `2px`（可被 `style.stroke` 覆盖）+ 外面再兜 `1px` 半透明黑
//!   （系统截图有浅有深，那圈黑把框从底上"抬"起来）；
//! - 圆角：`8px`（可被 `style.radius` 覆盖）；
//! - 编号：框左上角一颗 `18px` 实心强调色圆 + 白字（可被 `style.badge` 覆盖）。
//!
//! ★ **颜色不写进标注**（`AnnoStyle` 的注释说过：颜色跟主题走）—— 渲成品图时用**固定色**
//! （工作台强调色蓝 + 半透明黑 + 白字）；浅 / 深两张图烧同样的色，这也正是"在哪显示都一样"。

use std::path::Path;

use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont};
use image::{Rgba, RgbaImage};

use crate::error::AppError;

use super::annotation::{AnnoDoc, AnnoStyle};

/// 成品图的扩展名（摆原图旁边：`open-with-light.rendered.webp`）
pub const RENDER_EXT: &str = ".rendered.webp";

/// 框线 / 圆角 / 编号的缺省值 —— **客户端那三个数**（`GuideImage.module.css` 的
/// `--mark-stroke` / `--mark-radius` / `--badge-size`，以及标注台的镜像同值）。
///
/// ★★★ **这三个数是"屏幕上的尺寸"，不是"图上的尺寸"** —— 客户端的框是在**显示尺寸**
/// 下画的，而且**视觉大小恒定**（不随图缩放变）：不管那张图在页面上显示多大，框线
/// 永远是 2px、圆点永远 18px。
///
/// 2026-10-16 作者点名的病：「最终渲染出的角标，和实际上那个标注台里面显示的差别也太大了
/// 吧？这个怎么这么小，都不真实……包括这个框框的描边呀，粗细之类的」——
/// 第一版直接把 2 / 8 / 18 画在**原始像素**上（754 宽），而客户端是把它缩到 314 显示的，
/// 差 2.4 倍 ⇒ 烧出来的框看着比标注台细一大截、圆点小得看不清数字。
///
/// ★★★ **治法：屏幕尺寸 ÷ 参考显示比例**（`display_scale` = 这个"参考显示比例"）——
/// 它说的是**这份成品图按多大的显示比例定尺寸**：图上占 `屏幕尺寸 / display_scale` 个像素，
/// 于是"按这个比例显示它"时，框线正好是设计那 2px。
///
/// ```text
/// 参考比例 = 客户端把这张图显示多大 = 314 / 图宽
///            （`GuideImage.module.css` 的 `.shot { max-width: min(314px, 46vw) }`）
/// 754 宽的图 ⇒ 0.4164 ⇒ k ≈ 2.40（图上框线 4.8px；按 314px 显示时正好 2px）
/// ```
///
/// ★★★ **它现在是"参考"，不再等于"标注台当次缩放"**（2026-10-16 第二版）：
/// 上一版把标注台**当次的缩放**当参数传下来（作者原话：「就像那个摄像机一样……你那个
/// 适应视角下的缩放是多少你就显示出来，100 就是 100 的呀」）—— 代价是**同一份标注**
/// 在 42% 保存、拿到 51% 看就粗一大截（作者 2026-10-16 点名的「客户视角明显和在看效果
/// 差很远」）。现在**固定成客户端的参考比例**：换视角 / 换窗口 / 换设备，烧出来一模一样。
///
/// ★ 配套的另一半在标注台：画布上那份框按 `当前缩放 / 参考比例` 反着放大
/// （`.board` 的 `--zoom-k`）—— 于是"画布上怎么样显示、最终就怎么样"在任何缩放下都成立
/// （作者：「像 Photoshop 那人家的看起来怎么样，最后就怎么样，一点差别都没有」）。
const DEFAULT_STROKE: f64 = 2.0;
const DEFAULT_RADIUS: f64 = 8.0;
const DEFAULT_BADGE: f64 = 18.0;

/// 强调色的两档 —— **与客户端/工作台 CSS 的 `--accent` 同值**（`tokens.css` 与
/// `c14tokens.css` 都写着这两个数）：
///
/// ```text
/// 浅色  --accent: #20b25a      深色  --accent: #2f9e5b
/// ```
///
/// ★★★ 2026-10-16 修的那条（作者：「我这上面明明是绿色的框，到那边显示蓝色了」）：
/// 第一版这里写死了一个蓝 —— 框的颜色**与客户端不一致**（客户端是打印机绿）。
/// 颜色从 CSS 走、成品图是**像素**，所以只能把这两个数搬过来 —— 但**只在源里出现这一处**。
///
/// ★ 一张成品图**一份颜色**：浅色那张截图烧浅色绿、深色那张烧深色绿。
/// 客户端本来就是"浅色模式显示浅色图 / 深色模式显示深色图" ⇒ 两边永远对得上，
/// 这也正是"颜色跟主题走"在"图已经烧死"之后唯一自洽的落法。
const ACCENT_LIGHT: Rgba<u8> = Rgba([32, 178, 90, 255]);
const ACCENT_DARK: Rgba<u8> = Rgba([47, 158, 91, 255]);
/// 框线外那一圈半透明黑（客户端 `.mark` 的第二层影：`rgb(0 0 0 / 28%)`）
const HALO: Rgba<u8> = Rgba([0, 0, 0, 71]);
/// 编号里的字（客户端 `.badge` 的 `color: #fff`）
const BADGE_INK: Rgba<u8> = Rgba([255, 255, 255, 255]);

/// 这张图是深色那张吗 —— 按**文件名里有没有 `dark` / 深**判。
///
/// 与工作台标注台的 `pairOf`（抹 `light|dark|浅色|深色`）和客户端的浅深配对
/// **同一套词**。判不出来就当浅色（浅色是缺省那一档）。
fn is_dark_image(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.contains("dark") || rel.contains('深')
}

/// 原图所在的**子目录名**（约定：原图放 `source/`，成品图放它的上一层）。
///
/// 2026-10-16 作者定的落法：`guides/source/open-with-light.png`（原图，留存可再编辑）
/// ⇒ `guides/open-with-light.rendered.webp`（成品图，进台账、随包发货）。
/// 这一层目录差是**刻意的**：标注台显示原图、客户端读成品图，两份不在一处才不会互相覆盖。
pub const SOURCE_SUBDIR: &str = "source";

/// 成品图的落点：**原图同层 + 抹掉原名后缀 + [`RENDER_EXT`]**，但要先**掀掉
/// [`SOURCE_SUBDIR`] 那一层** —— 原图在 `source/` 里，成品图在它上一层。
///
/// - `guides/source/open-with-light.png` ⇒ `guides/open-with-light.rendered.webp`（今天这条）
/// - `guides/open-with-light.png` ⇒ `guides/open-with-light.rendered.webp`（没放 `source/` 时）
///
/// 与源稿落点（`doc_rel_of`）同一条思路：前端只说"哪条资产"，路径由这里从 `path` 派生。
pub fn rendered_rel_of(asset_rel: &str) -> String {
    let p = Path::new(asset_rel);
    let dir = p.parent().unwrap_or_else(|| Path::new(""));
    /* 落点目录：原图在 `source/` 子目录里时，成品放它的上一层 */
    let out_dir = if dir.file_name().and_then(|s| s.to_str()) == Some(SOURCE_SUBDIR) {
        dir.parent().unwrap_or_else(|| Path::new("")).to_path_buf()
    } else {
        dir.to_path_buf()
    };
    /*
     * ★ **幂等**（2026-10-16）：输入已经是 `x.rendered.webp` 时，`file_stem()` 给的是
     * `x.rendered` ⇒ 会拼出 `x.rendered.rendered.webp`（实测真在盘上留下了这种脏文件）。
     * 先把已有的 `RENDER_EXT` 抹掉再拼。
     */
    let base = p
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    /* ① 已经是成品名 ⇒ 只抹掉 `RENDER_EXT`（结果是"干名"） */
    let stem = match base.strip_suffix(RENDER_EXT) {
        Some(s) => s.to_owned(),
        /* ② 否则抹掉任意一个扩展名（`x.png` ⇒ `x`） */
        None => base
            .rsplit_once('.')
            .map(|(s, _)| s.to_owned())
            .unwrap_or_else(|| base.clone()),
    };
    out_dir
        .join(format!("{stem}{RENDER_EXT}"))
        .to_string_lossy()
        .replace('\\', "/")
}

/// 把一份标注烧进 `src`（PNG 原图），产出 webp 字节。
///
/// `doc.source.size` 是量标注时图的原生尺寸 —— 与真实图不符时**报错**（框会飘，
/// 宁可红也不要一张框错位的成品图）。
///
/// `image_rel` = 这一张原图在资产根里的相对路径 —— 只用来看它是不是**深色那张**
/// （决定框烧哪一档绿，见 [`ACCENT_LIGHT`] / [`ACCENT_DARK`]）。
///
/// `display_scale` = **这份成品图按多大的显示比例定尺寸**（客户端把这张图显示多大，
/// 即 `参考宽度 / 图宽` —— 见 [`DEFAULT_STROKE`] 那一段）。三个屏幕尺寸按
/// `1 / display_scale` 放大到原始像素上。
///
/// ★★★ 它**与标注台当前缩放无关**（这是刻意的）：要的就是"同一份标注烧出来一模一样"。非法 /
/// 太小的值一律当 1（宁可原样烧，也别除出个天文数字）。
pub fn render_webp(
    src_bytes: &[u8],
    doc: &AnnoDoc,
    image_rel: &str,
    display_scale: f64,
) -> Result<Vec<u8>, AppError> {
    let img = image::load_from_memory(src_bytes)
        .map_err(|e| {
            AppError::invalid_argument("原图解不出来（只认 PNG 这类）").with_detail(e.to_string())
        })?
        .to_rgba8();

    let (w, h) = (img.width(), img.height());
    if w != doc.source.size.w || h != doc.source.size.h {
        return Err(AppError::invalid_argument(format!(
            "图是 {w}×{h}，标注量在 {}×{} 上 —— 框会飘，先重量一次",
            doc.source.size.w, doc.source.size.h
        )));
    }

    /* 框的颜色跟**这张图自己的主题**走（浅色图烧浅色绿、深色图烧深色绿）—— */
    let accent = if is_dark_image(image_rel) {
        ACCENT_DARK
    } else {
        ACCENT_LIGHT
    };

    /* 显示倍率：非法 / 太小的值一律当 1（宁可原样烧，也别除出个天文数字） */
    let scale = if display_scale.is_finite() && display_scale > 0.02 {
        display_scale
    } else {
        1.0
    };

    let mut canvas = img;
    for m in &doc.marks {
        draw_mark(&mut canvas, m, accent, scale);
    }

    let mut out = Vec::new();
    {
        use image::ImageEncoder;
        // 无损：`image-webp` 的编码器只有无损一档（`features = ["webp"]`）
        let enc = image::codecs::webp::WebPEncoder::new_lossless(&mut out);
        enc.write_image(
            canvas.as_raw(),
            canvas.width(),
            canvas.height(),
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| AppError::io("成品图编不出来").with_detail(e.to_string()))?;
    }
    Ok(out)
}

/// 画一个框 + 它的编号。**与客户端 `.mark` / `.badge` 同一条几何**（百分比 → 像素）。
///
/// `display_scale` = 这份成品图的**参考显示比例** —— 三个屏幕尺寸按 `k = 1 / display_scale`
/// 放大到图上像素（按这个比例显示它时，框线正好是设计那 2px）。
fn draw_mark(
    img: &mut RgbaImage,
    m: &super::annotation::AnnoMark,
    accent: Rgba<u8>,
    display_scale: f64,
) {
    let style = m.style.clone().unwrap_or_default();
    let (w, h) = (img.width() as f64, img.height() as f64);

    /*
     * ★★★ 三个数从**屏幕尺寸**换算到**图上像素**（见 [`DEFAULT_STROKE`] 那段）：
     * CSS 里的 2 / 8 / 18 是"按参考比例显示"时的屏幕尺寸；
     * 烧进原始像素要乘 `1 / display_scale`（754 宽的图按 314 显示 ⇒ k ≈ 2.40）。
     * 外圈那 1px 的黑同理（`stroke + k`）。
     */
    let k = 1.0 / display_scale;
    let stroke = style.stroke.unwrap_or(DEFAULT_STROKE) * k;
    let radius = style.radius.unwrap_or(DEFAULT_RADIUS) * k;
    let badge = style.badge.unwrap_or(DEFAULT_BADGE) * k;

    let x = m.at.left / 100.0 * w;
    let y = m.at.top / 100.0 * h;
    let bw = m.at.width / 100.0 * w;
    let bh = m.at.height / 100.0 * h;

    let rect = RoundRect {
        x,
        y,
        w: bw,
        h: bh,
        r: radius,
    };
    /*
     * 两层影照客户端那样叠（**先大后小**，都只往外）：黑先铺满 `stroke + k`，
     * 绿再压在上面 ⇒ 黑只在绿之外那一圈（1 个**屏幕**像素）露出来。
     */
    stroke_round_rect(img, &rect, HALO, stroke + k);
    stroke_round_rect(img, &rect, accent, stroke);

    // 编号圆点：框左上角，圆心落在角上（客户端 `.badge` 是 `left/top: -badge/2`）
    let cx = x;
    let cy = y;
    disk(img, cx, cy, badge / 2.0, accent);
    // 里面的数字：用**内嵌的开源字体**画（与前端同一份字体）—— `1..=99`
    draw_number(img, cx, cy, badge, m.step);
}

/// 描一个圆角矩形上的一条带：`d ∈ [0, t]` —— **离边界往外 `t`**（`t` = 这一层影的 `spread`）。
///
/// ★★★ **只往外画** —— 与客户端 `box-shadow: 0 0 0 <spread> <color>` 逐字同构：外阴影
/// **画在 border-box 外面**（盒内的部分是被裁掉的），所以盒子里面一个像素都不动。
/// 两层影的叠法也照 CSS（**先大后小**）：
///
/// ```text
/// 黑（halo）  t = stroke + k     先画 —— 铺满整条带（含绿那一段，被绿盖住）
/// 绿（强调色）t = stroke         后画 —— 压在上面 ⇒ 黑只在绿之外那一圈露出来
/// ```
///
/// ★ 2026-10-16 第二版从"以边界为中心"（`d ∈ [-t/2, +t/2]`）改成这个。那一版是当年手写距离场
/// 怕把 SDF 符号搞反而选的保守画法（下面那段注释记着两次翻车），代价是框线**往控件里也多伸了
/// 一半**（`stroke/2` 个像素，按 `754/314` 放大后 ≈ 2.4px）—— 在作者要的"画布上怎么样、最终
/// 就怎么样、零差别"里，那条内伸正是看得出的差别。改成"只向外"后与 CSS 逐像素同构。
fn stroke_round_rect(img: &mut RgbaImage, rect: &RoundRect, color: Rgba<u8>, t: f64) {
    let RoundRect { x, y, w, h, r } = *rect;
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let x0 = x.floor().max(0.0) as u32;
    let y0 = y.floor().max(0.0) as u32;
    let x1 = (x + w).ceil().min(img.width() as f64).max(0.0) as u32;
    let y1 = (y + h).ceil().min(img.height() as f64).max(0.0) as u32;
    // 判定范围要把这条带的外沿算进去（往外溢 t）
    let pad = t.ceil() as i64 + 1;
    for py in
        (y0 as i64 - pad).max(0) as u32..=(y1 as i64 + pad).min(img.height() as i64 - 1) as u32
    {
        for px in
            (x0 as i64 - pad).max(0) as u32..=(x1 as i64 + pad).min(img.width() as i64 - 1) as u32
        {
            let d = round_rect_sdf(px as f64 + 0.5, py as f64 + 0.5, x, y, w, h, r);
            /*
             * 这条带是 `d ∈ [0, t]`（`d > 0` = 框外 ⇒ 一个像素都不落进盒子里）。
             *
             * ★★ **两个边沿各按半像素羽化**（与 `disk` 那条抗锯齿同一思路）：
             *   外沿不羽化 ⇒ 整条框边比客户端硬一格（作者要的"零差别"就卡在这一格上）；
             *   内沿不羽化 ⇒ 边界上那一格会偏实 —— 客户端那里是被裁的，本来就只有一半覆盖。
             */
            let cov = (t + 0.5 - d).clamp(0.0, 1.0).min((d + 0.5).clamp(0.0, 1.0));
            if cov > 0.0 {
                blend_alpha(img, px, py, color, cov);
            }
        }
    }
}

/// 一个圆角矩形（图中像素，浮点）
struct RoundRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    r: f64,
}

/// 圆角矩形的**有符号距离**（标准式）：点在框外为正、框内为负、边界上恰为 0。
///
/// ```text
/// q = |p − center| − (half − r)
/// d = length(max(q, 0)) + min(max(q.x, q.y), 0) − r
/// ```
///
/// ★★★ **这就是唯一对的写法，别再改**（2026-10-16 在这上面反复搞反过两次符号：
/// 先写成"只向外"、又试着把末尾 `- r` 去掉，结果都是框线整体偏移十几像素）。
/// 数值自检（框 (40,40)/80×40/r=10）：
/// 左边线中点 `(40,60) → 0`、中心 `(80,60) → −20`、框外 `(35,60) → +5`、
/// 左上角外 `(30,30) → 18.28`（= 到圆心 (50,50) 的距离 28.28 再 −10 ✓）。
///
/// ★ 配套的**画法**也定死：`stroke_round_rect` 取 `d ∈ [0, t]`（**只向外**，与客户端
/// `box-shadow` 的 `spread` 同一条 —— 见它的头注）。754 宽真图上框顶边那一竖列 =
/// 绿 5 + 黑 1 = 6px，**全在框外**，与客户端 `2px 线 + 1px 外影` 按 `754/314` 放大后一致 ✓。
fn round_rect_sdf(px: f64, py: f64, x: f64, y: f64, w: f64, h: f64, r: f64) -> f64 {
    let dx = (px - (x + w / 2.0)).abs() - (w / 2.0 - r);
    let dy = (py - (y + h / 2.0)).abs() - (h / 2.0 - r);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    outside + dx.max(dy).min(0.0) - r
}

/// 画一个实心圆（编号底）
fn disk(img: &mut RgbaImage, cx: f64, cy: f64, r: f64, color: Rgba<u8>) {
    let x0 = (cx - r - 1.0).max(0.0) as u32;
    let y0 = (cy - r - 1.0).max(0.0) as u32;
    let x1 = (cx + r + 1.0).min(img.width() as f64).max(0.0) as u32;
    let y1 = (cy + r + 1.0).min(img.height() as f64).max(0.0) as u32;
    for py in y0..y1 {
        for px in x0..x1 {
            let dx = px as f64 + 0.5 - cx;
            let dy = py as f64 + 0.5 - cy;
            // 边缘半像素抗锯齿
            let d = (dx * dx + dy * dy).sqrt();
            if d <= r - 0.5 {
                blend(img, px, py, color);
            } else if d < r + 0.5 {
                let a = r + 0.5 - d;
                blend_alpha(img, px, py, color, a);
            }
        }
    }
}

/// 编号字体 —— **前端用的是同一个文件**（`public/fonts/badge-digits.ttf`）：
/// Poppins（OFL 1.1，许可全文在 `public/fonts/OFL.txt`）裁到只剩 `0-9`，2 KB。
/// 两处一套字，才谈得上"烧出来的编号和标注台里那颗是同一个样子"。
/// ★ 2026-10-17 由 Inter 换成 Poppins（作者：「这个 1 看起来不在视觉中心」——
/// Inter 的 1 带底座、墨迹偏一侧；Poppins 的 1 无底座、旗标对称）。
const BADGE_FONT: &[u8] = include_bytes!("../../../../public/fonts/badge-digits.ttf");

/// 字号占圆径的比例 —— **与前端 `.badge` 同值**：`font-size: 11px` ÷ `--badge-size: 18px`。
const BADGE_FONT_RATIO: f64 = 11.0 / 18.0;

/// 用**真字体**画数字（编号只 `1..=99` 两种位数）。
///
/// ★★★ 2026-10-16 作者点名的病：「为什么编号的数字要用矢量图啊？他不应该用那种开源的
/// 免费字体吗」—— 第一版是**手写矢量描点**（一段段折线硬编码在 `glyph_of` 里），图的是
/// "不引字体库"；代价是**字形与标注台那份对不上**，而且那套描点被反复挑剔地改过好几次
/// （`'1'` 从"一根竖线"补成"斜杠 + 主干 + 底座"等）。现在换成内嵌开源字体 [`BADGE_FONT`]。
///
/// 对中方式**与前端 `.badge` 逐条对齐**（`display: grid; place-items: center`）：
/// ① 水平按**字宽和**居中（不是按墨迹 —— 那才与 CSS 一致）；
/// ② 竖直把**行盒**的中点落在圆心：盒心在基线上方 `(ascent + descent) / 2`
///   （`descent` 在 ab_glyph 里是负的 ⇒ 基线落在圆心**下方**）。
///
/// ★★★ **`PxScale` 是"字的总高"、不是 em**（2026-10-16 第二版修的那条）：ab_glyph 的
/// `PxScale` 按定义是 `ascent − descent`（Inter 上是 1.21 em），而 CSS 那格 `font-size`
/// 说的是 **em**。直接把 CSS 的数字当 `PxScale` 传 ⇒ 烧出来的数字小 17%（实测：画布那颗
/// 19px、成品图上只有 15.5px —— 作者：「我感觉这个 1 和 2 还是不太好……还差一点」）。
/// 换算：`PxScale = em × height_unscaled / units_per_em`（也就是 `pt_to_px_scale` 那一步）。
///
/// ★ `OutlinedGlyph::draw` 回调给的坐标是**相对该字包围盒左上角**的（imageproc 同款用法），
/// 所以还要把盒子的原点加回去。
fn draw_number(img: &mut RgbaImage, cx: f64, cy: f64, badge: f64, n: u32) {
    /* 字体要是坏了，**只是没有编号** —— 别 panic 掉整张成品图 */
    let Ok(font) = FontRef::try_from_slice(BADGE_FONT) else {
        return;
    };
    let Some(upem) = font.units_per_em() else {
        return;
    };
    /* CSS 那格的 `font-size`（**em**）—— 再按上面的换算折成 `PxScale` */
    let em = badge * BADGE_FONT_RATIO;
    let scale = PxScale::from((em * font.height_unscaled() as f64 / upem as f64) as f32);
    let scaled = font.as_scaled(scale);

    /* ① 先量这几位字的**总字宽**（水平按它居中，与 CSS 同一条） */
    let text = n.to_string();
    let mut advance = 0.0_f32;
    for ch in text.chars() {
        advance += scaled.h_advance(font.glyph_id(ch));
    }
    let ox = cx - f64::from(advance) / 2.0;
    let oy = cy + f64::from(scaled.ascent() + scaled.descent()) / 2.0;

    /* ② 按上面那个笔位原点排字（基线落在 `oy`） */
    let mut glyphs: Vec<Glyph> = Vec::new();
    let mut pen = 0.0_f32;
    for ch in text.chars() {
        let id = font.glyph_id(ch);
        let at = ab_glyph::point((ox + f64::from(pen)) as f32, oy as f32);
        glyphs.push(id.with_scale_and_position(scale, at));
        pen += scaled.h_advance(id);
    }

    /* ③ 逐个栅格化（坐标加上该字包围盒的原点） */
    for g in &glyphs {
        let Some(og) = font.outline_glyph(g.clone()) else {
            continue;
        };
        let b = og.px_bounds();
        let gx = b.min.x.round() as i32;
        let gy = b.min.y.round() as i32;
        og.draw(|x, y, coverage| {
            if coverage <= 0.0 {
                return;
            }
            let px = x as i32 + gx;
            let py = y as i32 + gy;
            if px < 0 || py < 0 {
                return;
            }
            /* 用覆盖率当 alpha ⇒ 小字边缘平滑（与 `disk` 那条半像素抗锯齿同一条） */
            blend_alpha(img, px as u32, py as u32, BADGE_INK, coverage as f64);
        });
    }
}

/// 直接覆盖（不透明色用）
fn blend(img: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>) {
    if x < img.width() && y < img.height() {
        img.put_pixel(x, y, color);
    }
}

/// 按 `a`（0..1）把 `color` 混到像素上（抗锯齿 / 半透明用）
fn blend_alpha(img: &mut RgbaImage, x: u32, y: u32, color: Rgba<u8>, a: f64) {
    if x >= img.width() || y >= img.height() {
        return;
    }
    let a = a.clamp(0.0, 1.0);
    let src_a = (color.0[3] as f64 / 255.0) * a;
    if src_a <= 0.0 {
        return;
    }
    let dst = img.get_pixel(x, y).0;
    let mix = |s: u8, d: u8| ((s as f64 * src_a) + (d as f64 * (1.0 - src_a))).round() as u8;
    let out = Rgba([
        mix(color.0[0], dst[0]),
        mix(color.0[1], dst[1]),
        mix(color.0[2], dst[2]),
        (dst[3] as f64).max(color.0[3] as f64 * a) as u8,
    ]);
    img.put_pixel(x, y, out);
}

/// 便捷：`AnnoStyle` 的缺省（与客户端一致）—— 供判据对照
pub fn defaults_of(_style: Option<&AnnoStyle>) -> (f64, f64, f64) {
    (DEFAULT_STROKE, DEFAULT_RADIUS, DEFAULT_BADGE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::annotation::{AnnoBox, AnnoDoc, AnnoMark, AnnoSize, AnnoSource};

    fn doc(w: u32, h: u32, marks: Vec<AnnoMark>) -> AnnoDoc {
        AnnoDoc {
            anno_schema: 1,
            source: AnnoSource {
                images: vec!["guides/x.png".into()],
                size: AnnoSize { w, h },
            },
            marks,
            updated_at: "2026-10-16T00:00:00Z".into(),
        }
    }

    fn mark(step: u32, l: f64, t: f64, w: f64, h: f64) -> AnnoMark {
        AnnoMark {
            step,
            at: AnnoBox {
                left: l,
                top: t,
                width: w,
                height: h,
            },
            note: String::new(),
            style: None,
        }
    }

    /// 落点：原名后缀换掉、加 `.rendered.webp`；原图在 `source/` 里时**掀掉那一层**
    #[test]
    fn rendered_path_swaps_the_extension() {
        // 今天这条：原图在 `source/`，成品在它上一层
        assert_eq!(
            rendered_rel_of("guides/source/open-with-light.png"),
            "guides/open-with-light.rendered.webp"
        );
        // 没放 `source/` 时：同层改名
        assert_eq!(
            rendered_rel_of("guides/open-with-light.png"),
            "guides/open-with-light.rendered.webp"
        );
        assert_eq!(rendered_rel_of("a/b/c.PNG"), "a/b/c.rendered.webp");
        // `source` 只在**目录段**上算，文件名里带 source 的不受影响
        assert_eq!(
            rendered_rel_of("guides/my-source.png"),
            "guides/my-source.rendered.webp"
        );
        /* ★ 幂等：拿成品路径再算一次，还是它自己（不然会拼出 `.rendered.rendered.webp`） */
        assert_eq!(
            rendered_rel_of("guides/my-source.rendered.webp"),
            "guides/my-source.rendered.webp"
        );
        assert_eq!(rendered_rel_of("a/b/c.PNG"), "a/b/c.rendered.webp");
    }

    /// 框的颜色 = **客户端那个绿**（浅 `#20b25a` / 深 `#2f9e5b`），**不是蓝**。
    ///
    /// 2026-10-16 作者截图点名的病：「我这上面明明是绿色的框，到那边显示蓝色了」——
    /// 第一版这里写死了一个蓝。这条判据钉死"烧出来的框是绿的"，谁再改回蓝当场红。
    #[test]
    fn the_frame_is_the_clients_green_not_blue() {
        let mut img = image::RgbaImage::new(200, 200);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");
        let d = doc(200, 200, vec![mark(1, 20.0, 20.0, 40.0, 20.0)]);
        let back = image::load_from_memory(
            &render_webp(&png, &d, "guides/x-light.png", 0.416).expect("渲得出来"),
        )
        .expect("读回来")
        .to_rgba8();
        /*
         * 框左边线在 x=40 附近（线**只往外** ⇒ 落在 40 左边那一薄条）。★ 采样点取 **y=75**：框的竖直中段、
         * 但**避开左上角那颗编号圆点**（badge 半径 ≈21 ⇒ y 40±21，y=50 会落在圆里、
         * 采到白色数字 ⇒ 假红。第一版就是栽在这）。框 20..80% 高 ⇒ y=75 还在框内。
         */
        let px = (37..45u32)
            .map(|x| back.get_pixel(x, 75).0)
            .max_by_key(|p| p[1] as i32 - p[0] as i32)
            .expect("这一段非空");
        assert!(
            px[1] > px[0] && px[1] > px[2],
            "框该是绿的（G 最大），量到 {px:?}"
        );
        /* ★ 浅色那张 = 客户端 `--accent` #20b25a。放宽一档是因为线上有半透明的外圈黑 */
        assert!(
            (px[0] as i32 - ACCENT_LIGHT.0[0] as i32).abs() <= 60
                && (px[1] as i32 - ACCENT_LIGHT.0[1] as i32).abs() <= 60,
            "浅色那张该用 --accent #20b25a，量到 {px:?}"
        );
    }

    /// ★ 框在**图上**要按"显示比例"放大 —— 754 宽那条上框线该明显比 2px 粗。
    ///
    /// 2026-10-16 作者点名的病：「最终渲染出的角标，和实际上那个标注台里面显示的差别也太大了
    /// 吧？这个怎么这么小……包括这个框框的描边呀、粗细之类的」。
    /// 判据：一张 754 宽的图上，沿框顶边竖着数「被画过」的像素 —— 该接近
    /// `2 × 754/314 ≈ 4.8`，绝不该还是 2。
    #[test]
    fn the_frame_is_scaled_to_the_image_so_it_matches_the_annotate_page() {
        let (w, h) = (754u32, 904u32);
        let mut img = image::RgbaImage::new(w, h);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");
        /* 框放在图中间，四边都不贴边，方便竖着数线宽 */
        let d = doc(w, h, vec![mark(1, 30.0, 30.0, 40.0, 40.0)]);
        let back = image::load_from_memory(
            &render_webp(&png, &d, "guides/x-light.png", 0.416).expect("渲"),
        )
        .expect("读回")
        .to_rgba8();

        let x = ((30.0 + 20.0) / 100.0 * w as f64) as u32; // 框水平中点
        let top = (30.0 / 100.0 * h as f64) as u32; // 框顶边

        /* 线**只往外**（与 `box-shadow` 同一条）⇒ 在顶边上下 24px 的窗口里数"非底色"= 线厚 */
        let thick = (1..24u32)
            .flat_map(|d| [top.saturating_sub(d), top + d])
            .filter(|&yy| {
                let p = back.get_pixel(x, yy).0;
                p[0] != 250 || p[1] != 250 || p[2] != 250
            })
            .count();
        /* 缩到 41.6% 看 ⇒ k = 1/0.416 ≈ 2.40 ⇒ 线宽 2 × 2.40 ≈ 4.8px（外圈再各 1px） */
        let want = 2.0 / 0.416;
        assert!(
            thick as f64 >= want * 0.8,
            "框线在图上该有 ~{want:.1}px 厚（显示比例换算后），量到 {thick}px"
        );
    }

    /// ★★★ **参考比例越小 ⇒ 烧出来的框越粗**（`k = 1 / 参考比例` 这一步没写反的判据）。
    ///
    /// 参考比例就是"这份成品图按多大显示"：754 宽的图按客户端那 314 显示 ⇒ 0.4164 ⇒
    /// 图上框线 ≈ 4.8px（按 314px 看时正好是设计那 2px）。比例越小，同一个 2px 要在图上占得越多。
    /// ★ 这个参数**不再等于标注台当次缩放**（见 [`DEFAULT_STROKE`] 那段）—— 标注台永远传
    /// "客户端的参考比例"，所以同一份标注烧出来是**固定**的。
    #[test]
    fn a_smaller_display_scale_burns_a_thicker_frame() {
        let (w, h) = (754u32, 904u32);
        let mut img = image::RgbaImage::new(w, h);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");
        let d = doc(w, h, vec![mark(1, 30.0, 30.0, 40.0, 40.0)]);

        let thickness = |scale: f64| -> usize {
            let back = image::load_from_memory(
                &render_webp(&png, &d, "guides/x-light.png", scale).unwrap(),
            )
            .unwrap()
            .to_rgba8();
            let x = ((30.0 + 20.0) / 100.0 * w as f64) as u32;
            let top = (30.0 / 100.0 * h as f64) as u32;
            (1..24u32)
                .flat_map(|dy| [top.saturating_sub(dy), top + dy])
                .filter(|&yy| {
                    let p = back.get_pixel(x, yy).0;
                    p[0] != 250 || p[1] != 250 || p[2] != 250
                })
                .count()
        };

        let at_100 = thickness(1.0);
        let at_fit = thickness(0.42);
        assert!(
            at_fit > at_100,
            "缩到 42% 看时烧出来该更粗（100% = {at_100}px，42% = {at_fit}px）"
        );
        /* 100% 时 k = 1 ⇒ 线宽就是 CSS 那个 2px（外圈那 1px 的影与线同心，多数像素被线盖住） */
        assert!(
            (2..=6).contains(&at_100),
            "100% 时线宽该就是 CSS 那个 2px，量到 {at_100}"
        );
    }

    /// 浅 / 深两张烧**不同档**的绿（浅 `#20b25a` / 深 `#2f9e5b`）
    #[test]
    fn light_and_dark_images_get_their_own_accent() {
        assert!(!is_dark_image("guides/source/open-with-light.png"));
        assert!(is_dark_image("guides/source/open-with-dark.png"));
        assert!(is_dark_image("guides/深色.png"));
        assert!(ACCENT_LIGHT.0[1] > ACCENT_DARK.0[1], "深色那档压暗一点");
    }

    /// 渲得出 webp、且**比原 PNG 小**（作者定的门槛：「只要体积比原始 PNG 小就可以接受」）
    ///
    /// ★★ 夹具是**带纹理的底**、不是纯色（2026-10-16）：纯色底上 PNG 靠游程编码能压得比
    /// 无损 webp 还小 —— 那是个不真实的极端（真图是系统截图，实测 webp 只有 PNG 的 63%）。
    /// 框线加了半像素抗锯齿之后，那个极端会翻过来（沿框边多出一圈渐变），于是夹具得
    /// **像真图那样有纹理**，这条判据才在测玩家伙事。
    #[test]
    fn renders_a_smaller_webp_than_the_png() {
        let mut img = image::RgbaImage::new(400, 300);
        for (x, y, p) in img.enumerate_pixels_mut() {
            /* 一段确定性的噪声（截图那种"处处不一样"的底）—— 固定式，别用随机 */
            let n = (x.wrapping_mul(37) ^ y.wrapping_mul(101)).wrapping_mul(31) as u8;
            *p = image::Rgba([
                190u8.wrapping_add(n),
                196u8.wrapping_add(n),
                200u8.wrapping_add(n),
                255,
            ]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");
        let d = doc(
            400,
            300,
            vec![
                mark(1, 10.0, 10.0, 30.0, 6.0),
                mark(2, 10.0, 30.0, 12.0, 6.0),
            ],
        );
        let webp = render_webp(&png, &d, "guides/x-light.png", 0.416).expect("渲得出来");
        assert!(!webp.is_empty());
        assert!(
            webp.len() < png.len(),
            "webp({}) 该小于 png({})",
            webp.len(),
            png.len()
        );
        // 真的是 webp（RIFF....WEBP）
        assert_eq!(&webp[0..4], b"RIFF");
        assert_eq!(&webp[8..12], b"WEBP");
    }

    /// 图尺寸与标注记的尺寸不一致 ⇒ **报错**（框会飘，宁可红）
    #[test]
    fn a_size_mismatch_is_refused() {
        let mut img = image::RgbaImage::new(100, 100);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([255, 255, 255, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");
        let bad = doc(100, 200, vec![]); // 图是 100×100，标注说 100×200
        assert!(
            render_webp(&png, &bad, "guides/x-light.png", 0.416).is_err(),
            "尺寸对不上要报错"
        );
    }

    /// 框真的画上去了：画框前后那一点的像素变了
    #[test]
    fn the_frame_actually_lands_on_pixels() {
        let mut img = image::RgbaImage::new(200, 200);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");

        let d = doc(200, 200, vec![mark(1, 20.0, 20.0, 40.0, 20.0)]);
        let webp = render_webp(&png, &d, "guides/x-light.png", 0.416).expect("渲得出来");
        let back = image::load_from_memory(&webp).expect("读回来").to_rgba8();
        /* 框左边线（20% of 200 = x40）附近应当有被画过的像素。
        ★ y=75：框内竖直中段、避开左上角那颗编号圆点（见上一条测试的注释） */
        let painted = (37..46u32).any(|x| {
            let p = back.get_pixel(x, 75).0;
            (p[0], p[1], p[2]) != (250, 250, 250)
        });
        assert!(painted, "框左边线那一段该被画过");
    }

    /// ★★★ 编号是**用内嵌字体画出来的**（2026-10-16 那条「为什么编号的数字要用矢量图」）。
    ///
    /// 判据：圆点里该有**白字像素**。底色是强调色绿（`#20b25a` = R32/G178/B90 ⇒ R 与 B 都低），
    /// 白字那一路 R 与 B 都高 —— 数一数圆心附近这种像素就有没有字。
    /// 换实现（手写描点 → `ab_glyph`）时它会红：两条路的字形不一样，白像素数差很远。
    #[test]
    fn the_badge_digit_is_drawn_with_the_embedded_font() {
        let (w, h) = (200u32, 200u32);
        let mut img = image::RgbaImage::new(w, h);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");

        let d = doc(w, h, vec![mark(7, 20.0, 20.0, 40.0, 20.0)]);
        /* scale = 1 ⇒ 编号就是 CSS 那个 18px，白字在圆心附近一眼能数 */
        let back = image::load_from_memory(
            &render_webp(&png, &d, "guides/x-light.png", 1.0).expect("渲得出来"),
        )
        .expect("读回来")
        .to_rgba8();

        let cx = (20.0 / 100.0 * w as f64) as i32;
        let cy = (20.0 / 100.0 * h as f64) as i32;
        let mut ink = 0;
        /*
         * ★★ 采样窗**必须整个落在圆点里**（半径 7 < 圆点半径 9）：底色是浅灰 250，
         * 而"白字"那条判据（R/B 都高）它**也满足** —— 窗开大了量到的是底色，
         * 这两条编号判据就都在自欺（2026-10-16 加字高判据时当场踩到）。
         */
        for y in (cy - 12)..=(cy + 12) {
            for x in (cx - 12)..=(cx + 12) {
                if (x - cx).pow(2) + (y - cy).pow(2) > 7 * 7 {
                    continue;
                }
                let p = back.get_pixel(x as u32, y as u32).0;
                /* 白字（含抗锯齿那圈）：R 与 B 都被抬起来了 */
                if p[0] > 150 && p[2] > 150 {
                    ink += 1;
                }
            }
        }
        assert!(ink >= 6, "编号圆点里该有白字像素，量到 {ink}");
    }

    /// ★★★ 编号要**正好落在圆点当中** —— 换字体实现后最容易错的就是这一步。
    ///
    /// 前两版栽的就是这儿：手写描点时是"字高 40% + 手工估的偏移"，看着偏上/顶边。
    /// 现在按**字宽和 + 行盒**对中（与前端 `.badge` 的 `place-items: center` 同一条）——
    /// Inter 上这两种度量本来就重合：`capHeight / 2` = `(ascent − |descent|) / 2` ≈ 0.364 em。
    ///
    /// 判据：圆点范围内**白字像素的包围盒中心**与圆心差 ≤ 1.5px。单数与双数各量一遍
    /// （双数走的是 `h_advance` 排字位那条路，单数走不到）。
    #[test]
    fn the_badge_digit_sits_in_the_middle_of_the_dot() {
        let (w, h) = (200u32, 200u32);
        let mut img = image::RgbaImage::new(w, h);
        for (_, _, p) in img.enumerate_pixels_mut() {
            *p = image::Rgba([250, 250, 250, 255]);
        }
        let mut png = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .expect("编 PNG");

        let (cx, cy) = (40i32, 40i32); // 框左上角（20% of 200）＝ 编号圆心
        for step in [8u32, 12, 30, 99] {
            let d = doc(w, h, vec![mark(step, 20.0, 20.0, 40.0, 20.0)]);
            let back = image::load_from_memory(
                &render_webp(&png, &d, "guides/x-light.png", 1.0).expect("渲得出来"),
            )
            .expect("读回来")
            .to_rgba8();

            let (mut min_x, mut max_x) = (i32::MAX, i32::MIN);
            let (mut min_y, mut max_y) = (i32::MAX, i32::MIN);
            /* ★ 窗开成**圆**、半径 7（圆点半径 9）：底色浅灰 250 也满足"白"那条判据，
            开成方窗就会把圆点外的底色算成字（见上一条测试的注释） */
            for y in (cy - 9)..=(cy + 9) {
                for x in (cx - 9)..=(cx + 9) {
                    if (x - cx).pow(2) + (y - cy).pow(2) > 7 * 7 {
                        continue;
                    }
                    let p = back.get_pixel(x as u32, y as u32).0;
                    if p[0] > 150 && p[2] > 150 {
                        min_x = min_x.min(x);
                        max_x = max_x.max(x);
                        min_y = min_y.min(y);
                        max_y = max_y.max(y);
                    }
                }
            }
            assert!(min_x <= max_x, "step {step}：圆点里没画出字");
            /*
             * ★★★ 顺带钉住**字有多大**（2026-10-16 那个 `PxScale` 不是 em 的坑）：
             * 圆径 18 ⇒ CSS 那格 `font-size` = 18 × 11/18 = 11px ⇒ Inter 的数字高 =
             * capHeight 0.7275 em ≈ 8px。把 CSS 的数字当 `PxScale` 直传时只有 6.6px
             * （小 17%）—— 作者一眼看出「这个 1 和 2 还差一点」，这条判据就会红。
             */
            let ink_h = max_y - min_y + 1;
            assert!(
                (7..=9).contains(&ink_h),
                "step {step}：数字该有 ~8px 高（capHeight 0.7275 × 11px），量到 {ink_h}px"
            );
            let mid_x = f64::from(min_x + max_x) / 2.0;
            let mid_y = f64::from(min_y + max_y) / 2.0;
            assert!(
                (mid_x - f64::from(cx)).abs() <= 1.5,
                "step {step}：字没横向居中（字心 {mid_x}，圆心 {cx}）"
            );
            assert!(
                (mid_y - f64::from(cy)).abs() <= 1.5,
                "step {step}：字没纵向居中（字心 {mid_y}，圆心 {cy}）"
            );
        }
    }
}
