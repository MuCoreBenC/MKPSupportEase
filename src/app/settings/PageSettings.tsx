/*
 * 设置页 —— 2026-10-13 重做（源 = 试验场 `frontend-lab` 的 a07 稿，2026-10-14 整份搬回本仓）。
 *
 * # 这一稿改了什么
 *
 * 作者看着旧稿问的原话：「这个设置页面，什么选中的呀？还有什么开关？都很丑，
 * 你有没有感觉？为什么是绿色的底…很奇葩呀」+「还有这个检查更新的按钮有没有可能居中呢？」
 * +「有没有必要放在最上面」。这一稿只动这一页：
 *
 * 1. **选中态 / 开关**：从"浏览器默认外观 + 全局品牌绿"换成**自绘墨色**。
 *    绿的根在 `src/styles/tokens.css` 的全站 `accent-color: var(--accent)` 上，
 *    改那一行会连带动到预设页 / 参数页（这一稿明说了只做设置页），所以这里走
 *    `appearance: none` 局部绕开 —— 见 `PageSettings.module.css` 的头注释。
 * 2. **「软件更新」提到第一块**：它原来是最后一块，作者问"有没有必要放在最上面"。
 * 3. **更新卡内：状态在左、动作在右上**。中间试过"按钮吊在卡片底下居中 / 靠右"，
 *    作者都不买账（「他应该放到上面呀…我就觉得他这个位置放在这不舒服」）——
 *    回到 a06 那个"同一行、右上、顶对齐"的老摆法。
 * 4. **按钮的语言**：「重新检查」不再用深墨那档（作者：「我不喜欢这个颜色太深了吧」）；
 *    有新版时绿色给「更新」那一颗（作者：「如果有新版的话，应该用新版的那个颜色」），
 *    文案也从「在应用内下载」收成两个字（作者：「不叫更新呀。文案改成更新」）。
 *    「查看更新」保留（作者：「算了，查看更新保留着吧」），跳转跟着下面选的数据源走。
 * 5. **动作条排序**（2026-10-13 当天第二轮，作者看截图后）：「更新」原来排在中间，
 *    作者：「这个更新按钮应该它有颜色不应该放到中间，应该放到右边啊」——
 *    定成 **重新检查 → 查看更新 → 更新**：能反复点的在左，有颜色的主操作压最右。
 * 6. **「切片」那格改下拉**（2026-10-13，作者看截图后）：「这个后续可能有更多的呢，
 *    改成那种下拉菜单…看项目之前用的是什么样的，可以去复用」+「也可以打开软件去
 *    指定呀…就像比如说 macOS 打开那个文件管理器的时候，就在 application」——
 *    三档单选换成下拉，并多一条「其他应用…」：从磁盘指定（挑中存 `clientSlicerApp`、
 *    值记 `'custom'`；真机上的成熟做法见 `MkpApi.pickSlicerApp` 的注释，口径都记在
 *    `shared/slicer.ts` 头注释里）。当天稍晚第三句：「这个下拉菜单也太简易了，整得
 *    太原生了，你得按照我们之前用过的下拉菜单去做复用」—— 落的**自绘共用件
 *    `SelectField`**（`components/field`；BBS 抽屉那两颗是同一天同一个改法）。
 *
 * # 执行记录（2026-10-13 作者口径）
 *
 * 后处理每跑一次都在盘上留三件套（原件备份 / 输出 / 执行记录），攒久了很占地方
 * （作者当时已经 400MB）。这一块只有一个开关：**自动清理旧记录**。
 *
 * 它**不是"存几天就删"**：纯按日历天数清会犯一个具体的错 —— 用户停用一个月再回来，
 * 最后一次的那条记录也被带走了。所以清理按「条数 + 空间」双限、**最新那条永远留着**，
 * 且全程**静默**（不提示、不打扰，记录页里直接少几条）。动手的是 Rust 侧
 * （`retention`，触发点是启动与每次后处理跑完），这一页只管把这一档写下来。
 *
 * # 软件更新这一块（保留下来的老口径）
 *
 * ★ **它是"有没有新版本 SupportEase"的唯一用户入口**（作者定死）。这一块与预设页的
 * 「读不懂」提示是**两条链**：这里答"软件有没有新版"，那里答"这批预设数据我读不读得懂"。
 * 两个事实**不许混成一句话**（禁区）。
 *
 * 信息源 = 仓库根的 `release.json`（`presets/` 之外，不是预设数据）：这是下半新开的一口，
 * 与 catalog 无关。检查**只在打开这一页时**发生（铁律 2：云端不参与首屏）。
 *
 * # 高级设置（最后一块）
 *
 * 为什么数据源住这里（作者裁决，「同步」页退役那一刀）：
 *  - 普通用户**完全不需要**知道"数据源"这个概念 —— 云端地址由工作台配置、
 *    构建时注入客户端（Bootstrap 那一刀）；catalog 随安装包走、更新是内部机制。
 *  - 这里留下的是**开发 / 排查的后门**：临时把下载指向本地 `python3 -m http.server`
 *    或别的源。默认状态就是"没被碰过"（内置默认 / 没配），普通用户什么都不用动。
 *
 * 交互：**两个云端源摆成固定单选**（Gitee / GitHub），选上**立即生效**并回写
 * （作者 2026-10-05 推翻旧的"选了不写、另摆按钮"那套 —— 「点了怎么没有立即帮我
 * 切换成 Gitee」）。自定义地址收在最后那一档，仍是两步：选上只展开输入框，
 * 按「应用」才写；保存前先探一次，取不到就整次拒绝。
 *
 * # 试验场专用
 *
 * 更新那四态 × 长/短文案的**开关只在试验场**：2026-10-14 作者要它从页面里消失、
 * 搬到预览外壳的工具条上 —— 这一页（产品里这一份）只认真账
 * `updateBusy / update / updateErr`，四态一律由真检查给。
 */

import { useCallback, useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import type { PresetSource, SoftwareUpdate, ThemePref } from '../../api/contract'
/* 换源广播（2026-10-08）：保存即生效、不许要求重启 —— 两个代次都只回答"要不要重读" */
import { deliveryMutated, sourceMutated } from '../state/deliveryState'
/* AppState 客户端：主题这一格从它读；写完再兜底刷一次（浏览器 mock 没有事件通道） */
import { appStateMutated, useThemePref } from '../state/appState'
import {
  SLICER_CHOOSE,
  slicerOptionsOf,
  useSlicerAppPref,
  useSlicerPref,
  type SlicerId,
} from '../shared/slicer'
/* 自绘下拉共用件（#33）：项目里的下拉一律用它，原生 `<select>` 一个不留 */
import { SelectField } from '../../components/field'
import type { FieldOption } from '../../components/field'
import { Btn } from '../ui/Controls'
import s from './PageSettings.module.css'

/**
 * 选的是**哪一个源**（`github` / `gitee` / `custom`）—— 与后端 `SourceMode` 一一对应，
 * **id 是契约**。作者 2026-05 拍：两个云端源 + 收起的自定义。
 */
type Mode = 'github' | 'gitee' | 'custom'

/**
 * 主题三档（`light` / `dark` / `system`）—— 与后端 `ThemePref` 一一对应，**id 是契约**。
 * 顺序就是摆出来的顺序：「跟随系统」在最前（没选过时生效的就是它）。
 */
const THEME_CHOICES: { id: ThemePref; label: string; hint: string }[] = [
  { id: 'system', label: '跟随系统', hint: '系统切深色时就跟着深色（没选过时就是这个）' },
  { id: 'light', label: '浅色', hint: '一直用浅色' },
  { id: 'dark', label: '深色', hint: '一直用深色' },
]

export default function PageSettings() {
  const [source, setSource] = useState<PresetSource | null>(null)
  const [loadErr, setLoadErr] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('github')
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: string; bad?: boolean; detail?: string } | null>(null)
  /*
   * 软件更新这一块的状态。三态各有各的话：
   *   `loading`  还在问
   *   `null`     问不到（没配源 / 离线 / 远端没部署）—— 如实说"这次没查到"，不编"已是最新"
   *   `SoftwareUpdate`  查到了，按 `hasUpdate` 分两句
   */
  const [update, setUpdate] = useState<SoftwareUpdate | null>(null)
  /* 查更新**失败的原因**（断网 / 没配源 / 远端没发 release.json）——
     与「没有新版」严格分开（A-53：失败不许长得像"已是最新"，也不许吞掉原因） */
  const [updateErr, setUpdateErr] = useState<string | null>(null)
  const [updateBusy, setUpdateBusy] = useState(true)
  /* 问更新读到的那份「当前版本」：即使远端问不到也要能显示它（它来自构建期，一定拿得到） */
  const [appVersion, setAppVersion] = useState('')



  /*
   * 主题（「外观」那一块）：当前档从 AppState 客户端读（`<html data-theme>` 也由它落地，
   * 这一页只管发一次写命令）；`themeBusy` 防连点，失败**不乐观改单选**——
   * 界面上显示的水远是盘上那一档（与换数据源同一条守则）。
   */
  const theme = useThemePref()
  const [themeBusy, setThemeBusy] = useState(false)
  const [themeErr, setThemeErr] = useState<string | null>(null)

  /*
   * 切片（「切片」那一块）：默认用哪个切片软件打开模型。纯前端偏好 ——
   * 落 localStorage（`shared/slicer`），不走 AppState：壳将来打开模型时由前端
   * 把这个值随 `openModel` 调用带给它。选上**立即生效**（与主题同一条口径）。
   *
   * ★ 2026-10-13 第二刀：还多一条「其他应用…」—— 走系统对话框从磁盘挑一个
   * （`slicerApp` 是它的记录，挑中才写、取消不写）。`slicerBusy` 防连点。
   */
  const [slicer, setSlicer] = useSlicerPref()
  const [slicerApp, setSlicerApp] = useSlicerAppPref()
  const [slicerBusy, setSlicerBusy] = useState(false)
  const [slicerErr, setSlicerErr] = useState<string | null>(null)

  /*
   * 执行记录（「执行记录」那一块）：`null` = 还没读到（控件先禁用，
   * **不拿一个猜出来的默认值顶上去**）。点一下立即落盘，失败**不改开关**、只把原因摆出来。
   */
  const [autoClean, setAutoClean] = useState<boolean | null>(null)
  const [autoCleanBusy, setAutoCleanBusy] = useState(false)
  const [autoCleanErr, setAutoCleanErr] = useState<string | null>(null)

  const read = useCallback(async () => {
    setLoadErr(null)
    try {
      const got = await api.getPresetSource()
      setSource(got)
      /* 初值跟着生效值走：选了自定义 = 自定义那一档；其余 = 出厂默认那个内置源 */
      setMode((got?.mode ?? 'github') as Mode)
      setDraft(got?.mode === 'custom' ? (got?.address ?? '') : '')
    } catch (e) {
      setSource(null)
      setLoadErr(errorText(e))
    }
  }, [])

  useEffect(() => {
    void read()
  }, [read])

  /*
   * 软件更新：**只在打开这一页时问一次**（铁律 2：更新检查不在启动 / 首屏路径）。
   * 两件事各问各的：版本号（本地、一定拿得到）与远端发布信息（可能问不到）。
   * 远端问不到 = `update` 留在 `null`，界面说"这次没查到" —— **不冒充"已是最新"**。
   */
  const checkUpdate = useCallback(async () => {
    setUpdateBusy(true)
    setUpdateErr(null)
    try {
      const [version, got] = await Promise.all([
        api.getAppVersion().catch(() => ''),
        api.checkSoftwareUpdate(),
      ])
      setAppVersion(version || got.currentVersion)
      setUpdate(got)
    } catch (e) {
      /* 没配源 / 离线 / 远端还没发 release.json：都不该让设置页出问题 ——
         如实留"没查到"，**并且把原因摆出来**（吞掉原因 = 失败和"没有新版"长一样） */
      setUpdate(null)
      setUpdateErr(errorText(e))
      const v = await api.getAppVersion().catch(() => '')
      setAppVersion(v)
    } finally {
      setUpdateBusy(false)
    }
  }, [])

  useEffect(() => {
    void checkUpdate()
  }, [checkUpdate])

  /*
   * 自动清理那一格：**只在打开这一页时读一次**（它不参与首屏，也不联网）。
   * 读不出来就留在 `null`（控件禁用 + 把原因摆出来）—— 这一格**不许猜**：
   * 猜出来的默认值迟早与真正在跑清理的那一侧漂开。
   */
  const readAutoClean = useCallback(async () => {
    try {
      setAutoClean(await api.getAutoCleanHistory())
      setAutoCleanErr(null)
    } catch (e) {
      setAutoClean(null)
      setAutoCleanErr(`读不出来：${errorText(e)}`)
    }
  }, [])

  useEffect(() => {
    void readAutoClean()
  }, [readAutoClean])

  /** 在系统浏览器里打开 Release 页（**走命令**，不走 `<a>`） */
  const openReleasePage = async (url: string) => {
    try {
      await api.openUrl(url)
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    }
  }

  /** 开始在应用内下载（标题栏那枚环会接手；这一页只负责发起） */
  const startDownload = async () => {
    try {
      await api.startUpdate()
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    }
  }

  /*
   * 选云端源 = **立即生效**（作者 2026-10-05：选了没反应让人怀疑没点上）。
   * 乐观把单选切过去，后端落盘成功就用回传值校准；失败**回到生效的那一档**，
   * 结论讲成人话、技术细节进折叠。自定义仍只展开输入框，不在这里写。
   */
  const pick = async (next: Mode) => {
    setNote(null)
    if (next === 'custom') {
      setMode('custom')
      return
    }
    if (busy) return
    setBusy(true)
    setMode(next)
    try {
      const got = await api.setPresetSource(next)
      if (got === null) {
        /* 演示后端没有内置源（真机必有）：选内置 = 回到"没配"，如实照做 */
        setSource(null)
        setMode(next)
        sourceChanged()
        return
      }
      setSource(got)
      setMode(got.mode as Mode)
      setNote({ text: `已切到：${got.label}` })
      sourceChanged()
    } catch (e) {
      setMode((source?.mode as Mode | undefined) ?? 'github')
      setNote({
        text: `没换成，现在用的还是 ${source?.label ?? '默认源'}。`,
        bad: true,
        detail: errorText(e),
      })
    } finally {
      setBusy(false)
    }
  }

  /*
   * 换主题：**立即生效**（与换数据源同一条口径 —— 单选本身就是明确意图，不再要一颗确认键）。
   * 写完 `appStateMutated()` 兜底刷一次快照：桌面端后端也会广播一次，两边都只触发一次重读。
   * 失败**不改单选**：界面显示的必须是盘上真账，把原因说出来就够。
   */
  const pickTheme = async (next: ThemePref) => {
    if (themeBusy) return
    setThemeBusy(true)
    setThemeErr(null)
    try {
      await api.setTheme(next)
      appStateMutated()
    } catch (e) {
      setThemeErr(errorText(e))
    } finally {
      setThemeBusy(false)
    }
  }

  /*
   * 开关自动清理：**立即生效**（与主题同一条口径 —— 开关本身就是明确的意图）。
   * 写完拿回传值校准这一格；失败**不动开关**，只把原因讲成人话。
   */
  const pickAutoClean = async (next: boolean) => {
    if (autoCleanBusy) return
    setAutoCleanBusy(true)
    setAutoCleanErr(null)
    try {
      setAutoClean(await api.setAutoCleanHistory(next))
    } catch (e) {
      setAutoCleanErr(`没改成：${errorText(e)}`)
    } finally {
      setAutoCleanBusy(false)
    }
  }

  /*
   * 「切片」那格的下拉（2026-10-13 第二刀）：内置档选上**立即生效**（与主题同一条口径）；
   * 「其他应用…」是动作项 —— 走系统对话框挑一个（`api.pickSlicerApp`），挑中 = 存下
   * 并切到「自定义」。**取消（`null`）什么都不写** —— 值没动过，共用件选完自己关弹层，
   * 不需要"恢复"代码。
   */
  const pickSlicer = (raw: string) => {
    if (raw !== SLICER_CHOOSE) {
      setSlicer(raw as SlicerId)
      return
    }
    if (slicerBusy) return
    setSlicerBusy(true)
    setSlicerErr(null)
    void (async () => {
      try {
        const app = await api.pickSlicerApp()
        if (app === null) return
        setSlicerApp(app)
        setSlicer('custom')
      } catch (e) {
        setSlicerErr(`没挑成：${errorText(e)}`)
      } finally {
        setSlicerBusy(false)
      }
    })()
  }

  /* 当前生效的那一句账：读不出来 / 没配 / 你指定的 / 内置源 —— 四种都要说得出 */
  const currentText =
    loadErr !== null
      ? `读不出来：${loadErr}`
      : source === null
        ? '还没配置 —— 下载与检查更新会先如实拒绝，并说明去哪儿配'
        : source.mode === 'custom'
          ? `你指定：${source.address === '' ? '（还没填地址）' : source.address}`
          : `${source.label}：${source.address}`

  const canSet = draft.trim() !== ''

  /* 单选显示的永远是**盘上真账**：还没选过（null）时按"跟随系统"那一档显示 */
  const currentTheme: ThemePref = theme ?? 'system'

  /* 切片下拉的选项：内置 + 已指定 + 最后那条动作项（共用件 `SelectField` 要的形状） */
  const slicerList: FieldOption[] = [
    ...slicerOptionsOf(slicerApp),
    { value: SLICER_CHOOSE, label: '其他应用…', note: '从磁盘选择' },
  ]
  /* 选着"你从磁盘指定的"时的那个应用（其余档 `null`） */
  const slicerCustom = slicer === 'custom' ? slicerApp : null
  /* `custom` 但记录不在（清过缓存）：按跟随系统显示 —— 值不在选项里时触发器显示原值，更难懂 */
  const slicerShow: SlicerId = slicer === 'custom' && slicerCustom === null ? 'system' : slicer

  /**
   * **换源之后广播一声**（2026-10-08 作者要求：地址保存即生效，**不许要求重启**）。
   *
   * 做两件事：`sourceMutated()` 让预设页**按新地址重来一遍**（包括允许后台再检查一次
   * 目录 —— 那道"本次运行只检查一次"的刹由收方复位）；`deliveryMutated()` 让所有读
   * 投递面的页面按新目录重读。两个都只是代次，不携带数据。
   */
  const sourceChanged = () => {
    sourceMutated()
    deliveryMutated()
  }

  /* 自定义地址的落盘（先探后写，后端做）。失败不回滚单选：输入框留着让人改地址重试 */
  const apply = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const got = await api.setPresetSource('custom', draft.trim())
      setSource(got)
      setNote({ text: '已应用：现在用的就是你指定的这个地址。' })
      sourceChanged()
    } catch (e) {
      setNote({
        text: `这个地址取不到预设数据，没有改，现在用的还是 ${source?.label ?? '默认源'}。请检查地址（或直接选上面的 Gitee / GitHub）。`,
        bad: true,
        detail: errorText(e),
      })
    } finally {
      setBusy(false)
    }
  }

  /* 这一屏真正要画的那几格：**只有真账**（四态由真检查给，不点播） */
  const upd = update
  const hasUpdate = upd?.hasUpdate === true
  const notes = hasUpdate ? upd?.notes : undefined
  const releaseUrl = hasUpdate ? upd?.url : undefined

  /*
   * 「查看更新」跳哪儿：**跟着这页下面选的数据源走**（作者 2026-10-13：
   * 「我那个不是有个自定义的数据源吗？我选哪一个它就可以点击跳转到我哪一个」）。
   * 自定义源就用自定义地址，云端源用 release.json 给的那条。
   *
   * ★ 产品里这条规则应由 `check_software_update` 一起给（release.json 本来就是按源取的），
   *   这里把它在演示里摆出来；搬回产品时随那两条命令一起定稿。
   */
  const jumpUrl =
    source?.mode === 'custom' && source.address !== '' ? source.address : releaseUrl

  return (
    <div className={s.page}>
      <header className={s.head}>
        <h1 className={s.title}>设置</h1>
        <p className={s.sub}>软件更新与应用设置。</p>
      </header>

      {/*
       * 软件更新 —— **a07 提到第一块**（作者 2026-10-13：「有没有必要放在最上面」）。
       * 这一块与预设页的「读不懂」提示是两条链，两个事实不许混成一句话（见文件头）。
       *
       * 卡内一层：**状态在左、动作在右上**（作者看了两版后的裁决：
       * 「他应该放到上面呀…我就觉得他这个位置放在这不舒服」—— 按钮吊在卡片底下
       * 显得空、也够不着）。说明块仍封顶可滚，长文案不会把按钮挤下去。
       */}
      <section className={s.section} aria-label="软件更新">
        <div className={s.updateTop}>
          <div className={s.updateRow}>
            <div className={s.updateHeadline}>
              {updateBusy ? (
                <span className={s.updateIdle}>正在检查……</span>
              ) : hasUpdate ? (
                <span className={s.updateFresh}>有新版本 SupportEase</span>
              ) : upd !== null ? (
                /* ★ 「已是最新」是**好消息**（作者 2026-10-05：字体要绿的）——
                   与「有新版本」同一个绿；「没查到」才是灰的（不是坏消息，是不知道）。 */
                <span className={s.updateLatest}>已是最新版本</span>
              ) : (
                <span className={s.updateIdle}>这次没能查到更新</span>
              )}
            </div>

            {/* 版本对照：当前 → 最新。当前版本一定拿得到（来自构建期） */}
            <p className={s.current}>
              当前版本 {appVersion || '—'}
              {hasUpdate ? ` → 最新版本 ${upd?.latestVersion}` : ''}
            </p>

            {/* 更新说明：长文案时这块自己滚（`.notes` 封顶），卡片高度不再跟着长 */}
            {notes !== undefined && <p className={s.notes}>{notes}</p>}

            {upd === null && !updateBusy && (
              <p className={s.fieldNote}>
                {updateErr !== null
                  ? `这次检查失败了：${updateErr}`
                  : '更新检查需要能连上发布地址（见下方「高级设置」）。连不上时如实说没查到，不冒充“已是最新”。'}
              </p>
            )}
          </div>

          <div className={s.secActions}>
            {/*
             * ★ 动作条从左到右 = 干扰小 → 干扰大（2026-10-13 第二轮，作者看截图后：
             *   「这个更新按钮应该它有颜色不应该放到中间，应该放到右边啊」）：
             *   最左「重新检查」（反复点也安全；不用深墨那档 —— 作者当天先说过
             *   「我不喜欢这个颜色太深了吧」）→ 中间「查看更新」（外链）→
             *   最右「更新」（绿色主操作，作者：「如果有新版的话，应该用新版的那个颜色」）。
             *   **有颜色的那颗永远压最右** —— 夹在中间会把整张卡的视觉焦点拽到中央，
             *   两边各剩一颗白按钮，越看越怪。
             */}
            <Btn disabled={updateBusy} onClick={() => void checkUpdate()}>
              {updateBusy ? '正在检查……' : '重新检查'}
            </Btn>
            {/*
              ★ 「查看更新」用 `api.openUrl`，**不用 `<a target="_blank">`**：
              Tauri 的 webview 没开 opener 权限，`<a>` 点了**什么都不发生**
              （0.0.2 用户实测："点了没反应，没弹出浏览器"）。修法是走命令（第五刀）。
            */}
            {jumpUrl !== undefined && (
              <Btn onClick={() => void openReleasePage(jumpUrl)}>查看更新</Btn>
            )}
            {/* 有安装包（release.json 给了 asset）才摆「更新」；没有就只留左边那两颗 */}
            {hasUpdate && upd?.asset !== undefined && (
              <Btn variant="accent" disabled={updateBusy} onClick={() => void startDownload()}>
                更新
              </Btn>
            )}
          </div>
        </div>
      </section>

      {/*
       * 外观（2026-10-12 深色模式那一刀）：三档单选 —— 跟随系统 / 浅色 / 深色。
       * 选上**立即生效**（与下面的数据源同一条口径）；真正落地的是 `<html data-theme>`
       * （`state/appState.ts` 的 applyTheme），这一页只把"用户选了哪一档"写进 AppState。
       *
       * ★ a07：整行都是 `<label>`（说明文字也能点），标记换成自绘墨点（见 CSS 头注释）。
       */}
      <section className={s.section} aria-label="外观">
        <h2 className={s.secTitle}>外观</h2>

        <div className={s.field}>
          <div className={s.fieldTitle}>主题</div>
          <p className={s.fieldNote}>换完立刻生效，记在应用状态里 —— 下次打开还是这一档。</p>

          {THEME_CHOICES.map((t) => (
            <label key={t.id} className={s.choice}>
              <input
                type="radio"
                name="theme"
                className={s.choiceInput}
                checked={currentTheme === t.id}
                onChange={() => void pickTheme(t.id)}
                disabled={themeBusy}
              />
              <span className={`${s.mark} ${s.markRadio}`} aria-hidden="true" />
              <span className={s.choiceText}>
                <span className={s.choiceLabel}>{t.label}</span>
                <span className={s.choiceHint}>{t.hint}</span>
              </span>
            </label>
          ))}
        </div>

        {themeErr !== null && <p className={`${s.note} ${s.noteBad}`}>没换成：{themeErr}</p>}
      </section>

      {/*
       * 切片（2026-10-13 作者口径）：「打开模型」默认交给哪个切片软件。
       *
       * ★ 2026-10-13 第二刀：三档单选改成**下拉**（作者：「这个后续可能有更多的呢，
       *   改成那种下拉菜单…看项目之前用的是什么样的，可以去复用」），并多一条
       *   「其他应用…」：走系统对话框从磁盘挑一个（作者：「也可以打开软件去指定呀…
       *   就像比如说 macOS 打开那个文件管理器的时候，就在 application」）。
       *   「打开模型」的框里还能「仅这一次」临时换（不写回这里）。
       *
       * ★ 2026-10-13 第三刀：第一版用的原生 `<select>`（作者看截图：「这个下拉菜单
       *   也太简易了，整得太原生了，你得按照我们之前用过的下拉菜单去做复用」）——
       *   换成**自绘下拉共用件 `SelectField`**（`shared-ui/field`，"打开模型"框里那只
       *   一并换；BBS 抽屉那两颗是同一天同一个改法）。
       */}
      <section className={s.section} aria-label="切片">
        <h2 className={s.secTitle}>切片</h2>

        <div className={s.field}>
          <div className={s.fieldTitle}>默认用哪个切片软件打开模型</div>
          <p className={s.fieldNote}>立刻生效。「打开模型」的框里还可以只对那一次换一个，不动这里的默认。</p>

          {/* 外面这层只为定宽/别被卡片拉满：共用件的 `.wrap` 是 inline-flex，自己不会伸 */}
          <span className={s.slicerPick}>
            <SelectField
              label="默认用哪个切片软件打开模型"
              options={slicerList}
              value={slicerShow}
              disabled={slicerBusy}
              onChange={pickSlicer}
            />
          </span>

          {/* 完整路径太长，不进弹层那列 note —— 选着它时摆在下拉下面一行 */}
          {slicerCustom !== null && (
            <p className={s.slicerHint}>
              打开模型时交给「{slicerCustom.name}」：{slicerCustom.path}
            </p>
          )}
          {slicerErr !== null && <p className={`${s.note} ${s.noteBad}`}>{slicerErr}</p>}
        </div>
      </section>

      {/*
       * 执行记录（产品 2026-10-13 那一刀，a07 照搬进来）：后处理每跑一次都在盘上留三件套，
       * 攒久了很占地方 —— 这一格管"要不要自动清"。
       *
       * ★ 措辞刻意**只说"最近 20 条"**，不写"存几天"：清理不按日历天数走
       *（那会让"停用一个月再回来"的用户连最后一次的记录一起丢），这个口径说不出来。
       * 也不写"清了多少"——清理是静默的，界面不报账。
       */}
      <section className={s.section} aria-label="执行记录">
        <h2 className={s.secTitle}>执行记录</h2>

        <div className={s.field}>
          <div className={s.fieldTitle}>自动清理执行记录</div>
          <p className={s.fieldNote}>
            每跑一次后处理都会存下原件、输出与执行记录，攒久了很占地方。开着时只保留最近
            20 条，更旧的会被自动清掉 —— 清理不打扰你（执行记录页里直接少几条）；
            最新那条永远留着，停用多久回来都还在。
          </p>

          <label className={s.choice}>
            <input
              type="checkbox"
              className={s.choiceInput}
              checked={autoClean ?? false}
              onChange={(e) => void pickAutoClean(e.target.checked)}
              disabled={autoClean === null || autoCleanBusy}
            />
            <span className={`${s.mark} ${s.markCheck}`} aria-hidden="true" />
            <span className={s.choiceText}>
              <span className={s.choiceLabel}>
                {autoClean === null
                  ? '自动清理（还没读到这一档）'
                  : autoClean
                    ? '自动清理：开 —— 只保留最近 20 条，更旧的静默清掉'
                    : '自动清理：关 —— 记录一直留着'}
              </span>
            </span>
          </label>
        </div>

        {autoCleanErr !== null && (
          <p className={`${s.note} ${s.noteBad}`}>{autoCleanErr}</p>
        )}
      </section>

      <section className={s.section} aria-label="高级设置">
        <h2 className={s.secTitle}>高级设置</h2>
        <p className={s.fieldNote}>以下仅供开发排查，普通使用不需要改动。</p>

        <div className={s.field}>
          <div className={s.fieldTitle}>预设数据源</div>
          <p className={s.fieldNote}>
            两个都是云端源，选一个就行（国内选 Gitee 更稳）。自定义地址只在开发 / 排查时用。
          </p>

          {/* ★ 两个固定单选 = 防错第一道：用户没有输错地址的机会。选上**立即生效**。
              地址提示跟在**各自**那一档里面（原来两条 URL 挤在一处，看不出归属） */}
          {(source?.builtin ?? []).map((b) => (
            <label key={b.id} className={s.choice}>
              <input
                type="radio"
                name="preset-source"
                className={s.choiceInput}
                checked={mode === b.id}
                onChange={() => void pick(b.id as Mode)}
                disabled={busy}
              />
              <span className={`${s.mark} ${s.markRadio}`} aria-hidden="true" />
              <span className={s.choiceText}>
                <span className={s.choiceLabel}>
                  {b.label}
                  {b.id === source?.defaultMode ? '（默认）' : ''}
                </span>
                <span className={s.choiceHint}>{b.address}</span>
              </span>
            </label>
          ))}

          {/* 收起的自定义：默认不展开（普通用户不需要看见输入框）；选上只展开，按「应用」才写 */}
          <label className={s.choice}>
            <input
              type="radio"
              name="preset-source"
              className={s.choiceInput}
              checked={mode === 'custom'}
              onChange={() => void pick('custom')}
              disabled={busy}
            />
            <span className={`${s.mark} ${s.markRadio}`} aria-hidden="true" />
            <span className={s.choiceText}>
              <span className={s.choiceLabel}>自定义地址（开发 / 排查）</span>
            </span>
          </label>
          {mode === 'custom' && (
            <div className={s.row}>
              <input
                className={s.input}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder="https://…/presets/delivery/ 或 …/source.json"
                aria-label="数据源地址"
              />
              <Btn onClick={() => void apply()} disabled={busy || !canSet}>
                {busy ? '正在检查……' : '应用'}
              </Btn>
            </div>
          )}
          {mode === 'custom' && (
            <p className={s.hint}>
              数据源根与 source.json 地址都认；应用前会先探一次，取不到就不改。
            </p>
          )}
        </div>

        <p className={s.current}>当前：{currentText}</p>
        {note !== null && (
          <div className={s.noteBlock}>
            <p className={`${s.note} ${note.bad ? s.noteBad : ''}`}>{note.text}</p>
            {note.detail !== undefined && (
              /* 技术细节收进折叠：普通用户只看结论（作者 2026-10-05：
                 「出了一堆东西我也看不懂，你要给用户看得懂的东西」） */
              <details className={s.noteDetail}>
                <summary>技术详情（开发排查用）</summary>
                <p>{note.detail}</p>
              </details>
            )}
          </div>
        )}
      </section>
    </div>
  )
}
