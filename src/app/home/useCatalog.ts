/*
 * 首页与校准页共用的「文件体系」目录。
 *
 * 两个读，各管一级：
 *
 *   getMachines()        品牌 / 机型 / 版本 —— 三级选择的数据源（不再是手编表）
 *   getRuntimeCatalog()  MKP 预设清单 + 资产登记 + 品牌 —— 见下
 *
 * 作者定的方向：「首页也应该是消费那个文件，而不是用硬编码」——所以 A1 就是 A1
 * （`A1.toml`），P1S 的版本就是 `LITE`，不再靠一张手编表 + id 映射。
 *
 * # MKP 清单为什么吃 runtime catalog 的 `files[]`（2026-10-06 真机 bug 钉下的）
 *
 * `getPresetFiles()` 在真机上**只出切片器预设**（MKP 由 catalog 的 `files[]` 登记，
 * 那条路的 `usedByVersions` 恒空）—— 首页的「文件」这一级吃它就整级为空：
 * 抽屉里每项没有文件名小字、`fileOf` 恒 null，第二页的「应用」按钮点击在
 * `comboFileName === null` 处**静默返回**（作者：「点了没用」）。mock 里
 * getPresetFiles 恰好带 9 份 MKP 带倒查，浏览器里测不出来 —— 分歧以真机数据源
 * 为准：MKP 的权威登记在 catalog 的 `files[]`（与预设页同源），每条自带
 * `machineId` / `versionId`，倒查就是它自己。
 *
 * 整机图这条也一样（2026-10-03 第二刀）：机型写的是**资产 id**，文件在哪由 catalog 的
 * `assets[]` 说（`assetUrlOf`），客户端不再 import 具体文件。
 *
 * 都是静态读（每编辑一次就变的是参数值，不在这里），进页面拉一次就够。
 *
 * 跨页缓存（2026-10-05）：原状是「tab 切换会重挂，一次进入一个来回」，代价是每次
 * 切回都从空数组起跳、一切「先空后满」—— 校准页的 pill 与读数要等目录、底账、参数
 * 几趟 IPC 都回来才长出真值，进场后当着用户的面再变一次（页签切换的「闪一下」）。
 * 现在挂载先吃上一份结果渲染、照旧重拉（文件可能换过），回来后覆盖缓存与 state；
 * 先旧后新在静态目录上没有一致性风险。
 *
 * # 「还没读到」≠「没有」（2026-10-31）
 *
 * 首页要按目录决定"首屏该显示正式版面还是欢迎页"。这一层原来失败只打一行控制台、
 * 永远停在空数组上，于是调用方**分不出**「数据还没回来」和「目录里真的没有机型」——
 * 启动时先把未初始化当成"用户没选过机型"，闪一帧欢迎页，数据回来再换成正式版面
 * （作者逐帧截图点的那一下）。现在这一层给两个标记：
 *
 *   attempted  两趟目录都落定了（成功 / 失败 / 超时都算）
 *   failed     机型那一趟没回来 ⇒ 首屏进「读取失败 + 重试」，不许掉进欢迎页
 *
 * `reload()` 是那颗「重试」的唯一出口（清失败标记 + 重跑一趟）。
 * ★ 运行时目录（预设清单 / 资产 / 品牌 / 分组）失败只算降级，不拦首屏。
 */

import { useCallback, useEffect, useMemo, useState } from 'react'
import { api } from '../../api'
import { useLocale } from '../../i18n'
import { localizeMachines } from '../shared/dataLabel'
import type {
  CatalogAsset,
  CatalogGroup,
  Machine,
  PresetFileInfo,
  RuntimeCatalogBrand,
} from '../../api/contract'

export interface Catalog {
  /** 机型目录（含品牌与版本），按目录顺序 */
  machines: Machine[]
  /** MKP 预设清单（catalog `files[]` 里 `kind === 'mkp_preset'` 的那些；倒查 = 它自己的 machineId/versionId） */
  presets: PresetFileInfo[]
  /**
   * 资产登记（`assets[]`）。机型图按 id 在这里查 `path`。
   *
   * 拉不到就是空数组 —— 那条路只让大图回落品牌 logo，**不挡机型与版本**：
   * 资产登记读的是运行时那份 catalog.json，它比 TOML 树晚一步（也可能还没换新）。
   */
  assets: CatalogAsset[]
  /** 品牌（含品牌图的资产 id）。同样拉不到就是空 —— 大图再回落内置字标 */
  brands: RuntimeCatalogBrand[]
  /**
   * **分组登记**（2026-10-16）：哪几张图共用一份标注、每张演什么角色。
   * 引导图按它挑哪张（`role = 'light' / 'dark'`）—— 不再写死两个资产 id。
   * 拉不到就是空数组 ⇒ 那一处引导不出图（缺图一路往下落，不白屏）。
   */
  groups: CatalogGroup[]
  /**
   * **机型目录读完了吗**（2026-10-31）—— 成功 / 失败 / 超时都算"读完"。
   *
   * ★ 为什么必须有这一格：首页要区分「**还没读到**」与「**读了，确实没有**」。
   *   原来这一层失败只 `console.error`、永远停在空数组上，页面没法分辨
   *   ——「还没读到」就被当成「用户没有选过机型」，于是启动时先闪一帧欢迎页、
   *   数据回来再换成正式版面（作者 2026-10-31 逐帧截图点的那一下）。
   *
   * ★ 两趟一起等，但**只有机型那一趟失败才拦首屏**：
   *   `getMachines` 决定"有没有得选"，它挂了就没法判断该显示正式版面还是欢迎页；
   *   `getRuntimeCatalog`（预设清单 / 资产 / 品牌 / 分组）挂了只让大图与抽屉降级 ——
   *   那几份本来就有"缺了就往下落"的一路，不该把用户挡在启动画面上。
   *   所以这里等的是**两趟都落定**（首帧就是完整版面，大图不会后一帧才冒出来），
   *   但 `failed` 只看机型那一趟。
   */
  attempted: boolean
  /**
   * 上一趟**失败或超时**了（`attempted` 为真但目录可能还是空的）。
   *
   * 首页据此把启动画面停在"读取失败 + 重试"，不许把它当成"用户从来没选过机型"
   * 丢进欢迎页 —— 那正是这次要消灭的错误中间态。
   */
  failed: boolean
  /** 重读一遍（启动画面那颗「重试」）。清掉失败标记并重新拉一次 */
  reload: () => void
}

/** 「文件」这一级的 id：`机型/版本`，与工作台发布 uid 同一形状 */
export function uidOfFile(file: PresetFileInfo): string | null {
  const ref = file.usedByVersions[0]
  return ref === undefined ? null : `${ref.machine}/${ref.version}`
}

/**
 * 目录最多等这么久（毫秒）。
 *
 * 超了按"落定"处理：机型没回来 ⇒ `failed`（首页进「读取失败 + 重试」），
 * 只有运行时目录没回来 ⇒ 照常放行（大图回落字标）。宁可降级，也不无限等待。
 */
const CATALOG_TIMEOUT_MS = 8000

/** 缓存里连"读到哪一步"一起存：下一处挂载先吃上一份结果时，就绪标记也跟着继承 */
type CatalogState = Omit<Catalog, 'reload'>

let catalogCache: CatalogState = {
  machines: [],
  presets: [],
  assets: [],
  brands: [],
  groups: [],
  attempted: false,
  failed: false,
}

export function useCatalog(): Catalog {
  const locale = useLocale()
  const [cat, setCat] = useState<CatalogState>(catalogCache)
  /* 重读的拨片：`reload()` 拨一下，下面那个 effect 重跑 */
  const [nonce, setNonce] = useState(0)

  const reload = useCallback(() => {
    /*
     * 两个标记**一起退回原样**：`attempted: false` 让首屏闸门重新关上（否则那一帧会
     * 拿着上一趟的空目录就放行，又冒出欢迎页），`failed: false` 让启动画面从"失败"
     * 回到"正在读"。数据一回来 `settle` 再把它们拨回去。
     */
    catalogCache = { ...catalogCache, attempted: false, failed: false }
    setCat(catalogCache)
    setNonce((n) => n + 1)
  }, [])

  useEffect(() => {
    let alive = true
    /* 写缓存 + 落 state：卸载之后仍写缓存（下次挂载能先吃到），只是不再 setState */
    const settle = (patch: Partial<CatalogState>) => {
      catalogCache = { ...catalogCache, ...patch }
      if (alive) setCat(catalogCache)
    }

    /*
     * ★★ 一次等齐两趟，但**不是**原来那个 `Promise.all`（2026-10-31）：
     *   原来任何一趟 reject 都会把另一趟的结果一起丢掉（`getMachines` 明明回来了，
     *   界面却一个机型都没有），而且"两趟都落地"这件事外面根本看不见。
     *
     *   现在 `allSettled` + 各自认领：机型失败 ⇒ `failed`（首屏停在「读取失败 + 重试」）；
     *   运行时目录失败 ⇒ 只是大图 / 抽屉 / 预设清单空着（它们本来就有降级路径）。
     *   两趟都落定才置 `attempted` —— 这样首帧就是**完整**的版面（大图不会后一帧才冒出来）。
     */
    const timer = window.setTimeout(() => {
      settle({ attempted: true, failed: true })
    }, CATALOG_TIMEOUT_MS)

    void Promise.allSettled([api.getMachines(), api.getRuntimeCatalog()]).then(([m, w]) => {
      window.clearTimeout(timer)
      const patch: Partial<CatalogState> = { attempted: true, failed: m.status === 'rejected' }
      if (m.status === 'fulfilled') {
        patch.machines = m.value
      } else {
        /* 失败照旧留一行控制台；界面上这一次不再"空着假装没有机型"，见 `failed` 那条注释 */
        console.error('[catalog] 机型目录拉取失败', m.reason)
      }
      if (w.status === 'fulfilled') {
        patch.presets = w.value.files
          .filter((f) => f.kind === 'mkp_preset')
          .map(
            (f): PresetFileInfo => ({
              id: f.fileName,
              fileName: f.fileName,
              path: f.path,
              kind: 'mkp_preset',
              category: '',
              machineIds: [f.machineId],
              usedByVersions: [{ machine: f.machineId, version: f.versionId }],
              delivery: 'default',
              inBundles: [],
            }),
          )
        patch.assets = w.value.assets ?? []
        patch.brands = w.value.brands ?? []
        patch.groups = w.value.groups ?? []
      } else {
        /* 这一趟拉不到照旧空着：三级选择不显形（不摆假选项），大图回落内置字标。 */
        console.error('[catalog] 运行时目录拉取失败', w.reason)
      }
      settle(patch)
    })

    return () => {
      alive = false
      window.clearTimeout(timer)
    }
  }, [nonce])

  /*
   * 版本显示名按语言摊开（缓存里存原样：缓存在语言切换前就建好了，不能把某一册写死进去）。
   * `tag` 不摊 —— 它还要给 `TAG_TONE` 当键，显示那一步单独挑。
   */
  const machines = useMemo(() => localizeMachines(cat.machines, locale), [cat.machines, locale])
  return { ...cat, machines, reload }
}
