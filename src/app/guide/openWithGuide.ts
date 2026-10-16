/*
 * 「改 .3mf 默认打开方式」这一处的**图片引导**：图 + 标注（2026-10-16）。
 *
 * 图 = 系统「显示简介」窗口那张截图（**浅 / 深两张，同尺寸同裁切**），标注只写一份、
 * 两张共用 —— 框的位置 / 粗细 / 圆角由 `GuideImage.tsx` 画，见它的头注。
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
 * 于是这个文件**只认资产 id**（与首页机型图同一条路：`catalog.assets[]` → `path` →
 * `/assets/<path>`）—— 换图 / 改裁切都在数据侧发生，这里一行不用动。
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
 * 把图按 1:1 摆进浏览器、盖一层**每 50px 一条线的网格**截图，读两个矩形的边缘
 * （下拉 x≈205..707 / y≈632..680；「全部更改…」x≈205..355 / y≈730..772），
 * 换成百分比、四个边各留 4px 余量。
 * ★ 换图 / 重裁之后**要重量一次**（浅深两张同裁切 ⇒ 量一次两张共用）。
 */

import type { CatalogAsset } from '../../api/contract'
import { assetUrlOf } from '../home/heroArt'
import type { GuideFigureSpec } from './GuideImage'

/** 那两张的**资产 id** —— 台账 `presets/assets.toml` 里的主键（客户端只认 id，不认文件名） */
const LIGHT_ID = 'guide-open-with-light'
const DARK_ID = 'guide-open-with-dark'

/**
 * 按当前的资产登记把这处引导的图取出来。**查不到就 `null`**（引导照常开，只是没有图 ——
 * 缺图一路往下落、不白屏，是全站同一条规矩）。
 */
export function openWithGuideFigure(assets: CatalogAsset[]): GuideFigureSpec | null {
  const light = assetUrlOf(assets, LIGHT_ID)
  const dark = assetUrlOf(assets, DARK_ID)
  if (light === null || dark === null) return null
  return {
    light,
    dark,
    alt: '系统「显示简介」窗口：展开的「打开方式」，里面有应用下拉与「全部更改…」',
    marks: [
      /* ① 应用下拉（这里选好你要的那个） */
      { step: 1, at: { left: 26.7, top: 69.5, width: 67.6, height: 6.2 } },
      /* ② 「全部更改…」（选好之后它才会亮） */
      { step: 2, at: { left: 26.7, top: 80.3, width: 20.9, height: 5.5 } },
    ],
  }
}
