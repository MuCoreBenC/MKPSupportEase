/*
 * 「改 .3mf 默认打开方式」这一处的**图片引导**：图 + 标注（2026-10-16）。
 *
 * # ★★★ 交付的是**成品图**（2026-10-16 方案 A，作者：「客户端收到的不是原文件，
 *   而是最终的渲染」）
 *
 * 台账那两条 `path` 指 **`.rendered.webp`** —— 框已经**烧进像素**了
 * （工作台「标注台」保存时由 `src-tauri/.../app/render.rs` 渲好）。
 * 所以这里 `rendered: true`：`GuideImage.tsx` **不再叠画 `marks`**，
 * 客户端 `<img>` 直接显示，**跨平台 / 跨系统一致**（放大到 400% 也不会露图层感）。
 *
 * 原图与源稿住在 `presets/assets/guides/source/` 与 `guides/<对>.anno.json`，
 * 像 PS 的 PSD 一样**自己留存、可再编辑**（**不进客户端交付** —— 只有成品图随包）。
 * 改了标注 ⇒ 标注台保存 ⇒ 重渲成品图 ⇒ 发一份新目录，客户端照旧按 id → path 取。
 *
 * # 图住哪儿（2026-10-16 作者拍的那条线）
 *
 * **住资产根 + 进台账**（`presets/assets/guides/` + `presets/assets.toml` 里那两个
 * `type = 'guide'` 条目），**不是** `src/app/assets/` —— 作者的原话：
 * 「为什么我要放 src 里面呢？那到时候那个 preset 里面又有什么作用呢？
 * 难道我不应该就直接放在 preset 里面吗？这样方便管理呀，到时候工作台也方便管理」。
 *
 * 两条线各管什么：
 *
 *   界面自带素材（`src/app/assets/`）  品牌 logo、首页那张 hero 合影 —— **跟着源码走**，
 *                                     改一张图要动代码，工作台看不见
 *   产品资产（`presets/assets/`）      机型图 / 图标 / 模型 / 切片器预设 / **引导图** ——
 *                                     台账登记、工作台看得见选得着、发布的唯一源
 *
 * 于是这个文件**只认一个组句柄**（2026-10-16 起）：组 → 成员 → `catalog.assets[]` →
 * `path` → `/assets/<path>`。换图 / 改裁切 / 换成员 / 改组名都在数据侧发生，这里一行不用动。
 * （从前这里写死两个资产 id，改名就得重编客户端 —— 组进来之后那条线断了。）
 *
 * # 以后要按需下载（作者定的方向）
 *
 * 「图片……后续按需下载……到那一页的时候就下载下来」。那时把台账那两行的
 * `delivery = 'bundled'` 删掉（不写 = 进交付集合 = 按需），再给客户端补一条
 * **"读下载区图片"**的口子（今天没有：下载区的东西只有文本走 IPC）。
 * ★ **这个文件的 `marks` 一个字都不用改**，只有取图那两行的来源变。
 *
 * # 坐标怎么量的（2026-10-16，作者的两张图 754×904）
 *
 * ★★★ **第一版的数是错的**（拿"50px 网格 + 眼睛读"量的 → 框落在下拉**下面**那一行、
 * 还伸出图外；作者当场指出「框框没框对」）。第二版改成**机器扫**：
 *
 * 1. 把 PNG 按 1:1 画进 `<canvas>`，`getImageData` 拿像素；
 * 2. **alpha 当底** —— 这张图四周是透明的（窗口自己的投影），"窗口那一块" = alpha>200；
 * 3. 面取右上角一块空白（浅色 255 / 深色按中位数），逐行数「跟面差 >6」的像素：
 *    整行铺满 = 下拉，有一段长连片 = 小按钮；再按列取范围 ⇒ 像素级的四边；
 * 4. 浅 / 深两张各跑一遍**对值**（差 1px 以内 = 同一裁切 ✓）。
 *
 * 结果（`{ left, top, width, height }` 一律百分比，四边**紧贴控件**、不留余量）：
 *
 * | 框 | 像素（754×904） | 百分比 |
 * | --- | --- | --- |
 * | ①「打开方式」下拉 | x 142..611 / y 526..565 | `18.8 / 58.2 / 62.3 / 4.4` |
 * | ②「全部更改…」 | x 142..287 / y 618..657 | `18.8 / 68.4 / 19.4 / 4.4` |
 *
 * ★★ 换图 / 重裁之后**要重量一次**（浅深两张同裁切 ⇒ 量一次两张共用）——
 * 量法有两条：上面那套 canvas 扫描（快），或者**工作台的「标注台」**（拖一遍导出
 * `marks`，作者自己也能改；见 `src/workbench/views/AnnotatePage.tsx`）。
 *
 * # 标注的"源稿"（2026-10-16 作者要的：像 PS 那样能再编辑、跟着仓库走）
 *
 * 「那个源文件吧，就像 PS 里面的那种源文件呢，可以再次编辑的那一种……我希望它是随着这个
 *   项目仓库的，而不是说到我本地的。这样，我换了设备也可以继续编辑生成新的图片」
 * ⇒ 可再编辑的那一份住在**这两张图的旁边**：`presets/assets/guides/<对>.anno.json`
 *   （工作台「标注台」读写它；**发布时排除** —— 客户端不需要，别塞进用户包里）。
 *
 * 这个文件里的 `marks` 是它**导出**出来的那一段真值（作者在标注台里拖好 / 调好样式 →
 * 「复制 TS」→ 贴回这里）。★ 两处一起看：**源稿是"能再编辑的"，`marks` 是"产品真在用的"**；
 * 改了源稿别忘了同步这一段（标注台上会标出"有改动没写进仓库"）。
 * ★ 颜色不在 `style` 的可调项里：颜色必须跟主题走（浅 / 深两套 token），见 `GuideMarkStyle`。
 */

import type { CatalogAsset, CatalogGroup } from '../../api/contract'
import { assetUrlOf } from '../home/heroArt'
import type { GuideFigureSpec } from './GuideImage'

/**
 * 这一处引导用**哪个组** —— 台账 `[[groups]]` 里的组 id。
 *
 * ★ 2026-10-16 起的口径：客户端**认组、不认哪张图**。从前这里写死两个资产 id
 * （`LIGHT_ID` / `DARK_ID`），一改名或一换图就得重编客户端；现在只留**一个组句柄**，
 * 组里装谁、叫什么、哪张是浅色那张，全由台账说了算 ⇒ 换图 / 换成员 / 改组名都只是
 * 发一份新目录，**不用重编**（`catalog.groups`，见 `RuntimeCatalog.groups`）。
 */
const OPEN_WITH_GROUP = 'open-with'

/** 组里演某个角色的那张，换成能直接喂给 `<img src>` 的地址；组里没这个角色 = `null` */
function urlOfRole(assets: CatalogAsset[], group: CatalogGroup, role: string): string | null {
  const m = group.members.find((x) => (x.role ?? '').trim().toLowerCase() === role)
  return m === undefined ? null : assetUrlOf(assets, m.asset)
}

/**
 * 按当前的资产登记 + 分组登记把这处引导的图取出来。**取不到就 `null`**（引导照常开，
 * 只是没有图 —— 缺图一路往下落、不白屏，是全站同一条规矩）。
 *
 * 挑法的兜底：组里**没有** `light` / `dark` 角色时，两张都用**第一张成员**
 * （一组只有一张也一样成立 —— `role` 可以空）。
 */
export function openWithGuideFigure(
  assets: CatalogAsset[],
  groups: CatalogGroup[] | undefined,
): GuideFigureSpec | null {
  const want = OPEN_WITH_GROUP.toLowerCase()
  const group = (groups ?? []).find((g) => g.id.trim().toLowerCase() === want)
  if (group === undefined) return null
  const light =
    urlOfRole(assets, group, 'light') ??
    assetUrlOf(assets, group.members[0]?.asset ?? '')
  if (light === null) return null
  const dark = urlOfRole(assets, group, 'dark') ?? light
  return {
    light,
    dark,
    alt: '系统「显示简介」窗口：展开的「打开方式」，里面有应用下拉与「全部更改…」',
    /*
     * ★★★ **交付的是"框已经烧进像素"的成品图**（2026-10-16 方案 A）：
     * 台账那条 `path` 指 `.rendered.webp`，框是工作台保存标注时用 `render.rs` 渲进去的。
     * ⇒ 这个 `rendered: true` 让 `GuideImage` **别再叠画**下面的 `marks`
     *   （否则就是双份框；放大到 400% 时那点百分比取整差会露成重影）。
     *
     * `marks` **仍然留着**，理由有两条：
     * 1. 成品图没渲出来时（老数据 / 成员资产还是原图）能**回落**到现画 ——
     *    把 `rendered` 那行删掉就是这个老行为；
     * 2. 它是"框该在哪儿"的**文字真值**，与工作台源稿对得上，方便人工核对。
     */
    rendered: true,
    marks: [
      /* ①「打开方式」下拉（这里选好你要的那个） */
      { step: 1, at: { left: 18.8, top: 58.2, width: 62.3, height: 4.4 } },
      /* ②「全部更改…」（选好之后它才会亮） */
      { step: 2, at: { left: 18.8, top: 68.4, width: 19.4, height: 4.4 } },
    ],
  }
}

