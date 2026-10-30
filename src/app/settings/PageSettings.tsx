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

import { useCallback, useEffect, useRef, useState, type MouseEvent } from 'react'
/* 系统文件选择框：挑"备份放哪个文件夹"与"恢复哪一份 zip" —— 位置一律由用户点名 */
import { open } from '@tauri-apps/plugin-dialog'

import { api, errorText, isAppError } from '../../api'
import type { LocalePref, PresetSource, SoftwareUpdate, ThemePref } from '../../api/contract'
import { msg, useText, useT, type StringKey, type Text } from '../../i18n'
/* 换源广播（2026-10-08）：保存即生效、不许要求重启 —— 两个代次都只回答"要不要重读" */
import { deliveryMutated, sourceMutated } from '../state/deliveryState'
/* AppState 客户端：主题 / 语言这两格从它读；写完再兜底刷一次（浏览器 mock 没有事件通道） */
import { appStateMutated, useLocalePref, useThemePref } from '../state/appState'
import {
  SLICER_CHOOSE,
  slicerOptionsOf,
  systemSlicerLine,
  useSlicerAppPref,
  useSlicerPref,
  useSystemSlicerApp,
  type SlicerId,
} from '../shared/slicer'
/* 自绘下拉共用件（#33）：项目里的下拉一律用它，原生 `<select>` 一个不留 */
import { SelectField } from '../../components/field'
import type { FieldOption } from '../../components/field'
/* 圆钮开关共用件：设置页的布尔项**一律**用它。
   ★ 2026-10-30：引导 / 执行记录原来用勾选框（`Check`），与后处理那几块不一致 ——
   全部换成这一颗；多选一则统一用分段控件（`.seg`）。于是这一页不再 import `Check`。 */
import { Switch } from '../../components/switch'
/* 自绘提示气泡（2026-10-30）：替代原生 `title` —— 形状与避让都是自己的，
   Windows / WebView2 上看不会变成另一副样子。设置页的说明一律用它。 */
import { Tip } from '../../components/tooltip'
/* 「去系统里改…」那一处的**图片引导**（2026-10-16）：图 + 框 + 两步，见它们的头注。
   ★ 与 `guide/GuideCard` 那扇"屏幕引导"是两件事（那个贴到切片器窗口上指）。 */
import GuideImageModal from '../guide/GuideImage'
/* 后处理偏好那几块（2026-10-17）：各自一个文件，迁页时整块搬走即可 */
import PostprocModeSection from './PostprocModeSection'
import CalibSection from './CalibSection'
import BedGlueSection from './BedGlueSection'
import DataSection from './DataSection'
/* 预设那一块（2026-10-20）：与上面同一条规矩（自己一个文件、只管自己那一行） */
import PresetsSection from './PresetsSection'
import { openWithGuideFigure } from '../guide/openWithGuide'
/* 「保存后的重切引导」那个共用开关（与提示里的「不再显示」写同一个键）：
   这一页是它**唯一能重新打开**的地方，见 `guide/guidesPref` */
import { useGuidesPref } from '../guide/guidesPref'
/* 引导图那两张**住资产台账**（`presets/assets.toml` 的 `type = 'guide'`）——
   按 id 取 URL 要靠当前这份 catalog（与首页机型图同一条路） */
import { useCatalog } from '../home/useCatalog'
import { Btn } from '../ui/Controls'
/* 左目录那 12 枚图标走全站同一张 path 表（`theme` 是 2026-10-30 为「外观」加的） */
import Icon, { type IconName } from '../../components/Icon'
import s from './PageSettings.module.css'

/*
 * 左目录（2026-10-30）：12 项 = 这一页的 12 个 section，**顺序必须与 DOM 一致** ——
 * 滚到哪亮到哪（scrollspy）与点击跳转都按这个顺序对 id。
 *
 * ★ 这是设置页第一次有"目录"：原来一屏十几个灰卡片一路滚下来，找「备份」只能从头滚。
 *   窄窗（≤900）时它整块不显示（作者：「窄的时候直接不显示就算了」），
 *   定位改由章节标题接管。
 */
const NAV: { id: string; key: StringKey; icon: IconName }[] = [
  { id: 'upd', key: 'settings.update.title', icon: 'refresh' },
  { id: 'appearance', key: 'settings.appearance.title', icon: 'theme' },
  { id: 'slicer', key: 'settings.slicer.title', icon: 'box' },
  { id: 'guides', key: 'settings.guides.title', icon: 'doc' },
  { id: 'history', key: 'settings.history.title', icon: 'history' },
  { id: 'backup', key: 'settings.backup.title', icon: 'download' },
  { id: 'postproc', key: 'settings.postproc.title', icon: 'bolt' },
  { id: 'calib', key: 'settings.calib.title', icon: 'crosshair' },
  { id: 'glue', key: 'settings.glue.title', icon: 'drop' },
  { id: 'presets', key: 'settings.presets.title', icon: 'presets' },
  { id: 'data', key: 'settings.data.title', icon: 'folder' },
  { id: 'recovery', key: 'settings.recovery.title', icon: 'history' },
  { id: 'advanced', key: 'settings.advanced.title', icon: 'params' },
]

/**
 * 选的是**哪一个源**（`github` / `gitee` / `custom`）—— 与后端 `SourceMode` 一一对应，
 * **id 是契约**。作者 2026-05 拍：两个云端源 + 收起的自定义。
 */
type Mode = 'github' | 'gitee' | 'custom'

/**
 * 主题三档（`light` / `dark` / `system`）—— 与后端 `ThemePref` 一一对应，**id 是契约**。
 * 顺序就是摆出来的顺序：「跟随系统」在最前（没选过时生效的就是它）。
 */
const THEME_CHOICES: { id: ThemePref; label: StringKey; hint: StringKey }[] = [
  { id: 'system', label: 'settings.theme.system', hint: 'settings.theme.systemHint' },
  { id: 'light', label: 'settings.theme.light', hint: 'settings.theme.lightHint' },
  { id: 'dark', label: 'settings.theme.dark', hint: 'settings.theme.darkHint' },
]

/**
 * 界面语言三档（2026-10-25）—— 与后端 `LocalePref` 一一对应，**id 是契约**。
 *
 * ★ 这里的 label / hint 存的是**字典键**而不是句子：三种语言名本身各用各的写法
 *   （「简体中文」/ English），换界面语言时这一行也要跟着换（见 `zh-CN.ts`）。
 */
const LOCALE_CHOICES: { id: LocalePref; label: StringKey; hint: StringKey }[] = [
  { id: 'system', label: 'settings.locale.system', hint: 'settings.locale.systemHint' },
  { id: 'zh-CN', label: 'settings.locale.zh', hint: 'settings.locale.zhHint' },
  { id: 'zh-TW', label: 'settings.locale.zhTW', hint: 'settings.locale.zhTWHint' },
  { id: 'ja', label: 'settings.locale.ja', hint: 'settings.locale.jaHint' },
  { id: 'en', label: 'settings.locale.en', hint: 'settings.locale.enHint' },
]

export default function PageSettings() {
  const [source, setSource] = useState<PresetSource | null>(null)
  const [loadErr, setLoadErr] = useState<string | null>(null)
  const [mode, setMode] = useState<Mode>('github')
  const [draft, setDraft] = useState('')
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: Text; bad?: boolean; detail?: string } | null>(null)
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
   * 界面语言（2026-10-25）：与主题同一条口径 —— 当前档从 AppState 读、
   * 选上立即写、失败不乐观改单选。`tr` 是这一页取字典文案的入口
   * （不叫 `t`：下面 `THEME_CHOICES.map((t) => …)` 已经在用那个名字）。
   */
  const localePref = useLocalePref()
  const [localeBusy, setLocaleBusy] = useState(false)
  const [localeErr, setLocaleErr] = useState<string | null>(null)
  const tr = useT()
  /* 存在 state 里的那句回执（备份 / 换源）：渲染时再翻 */
  const say = useText()

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
  /*
   * 「跟随系统默认」到底指谁（2026-10-16）：问壳要一次系统真值（macOS LaunchServices /
   * Windows 注册表），挂在下拉下面那行小字里 —— 那一档名字里没有应用名，光看认不出来。
   * 问不出来给 `null` ⇒ 那行不摆（回落原来那句话术，不编一个名字）。
   */
  const systemApp = useSystemSlicerApp()
  const [slicerBusy, setSlicerBusy] = useState(false)
  const [slicerErr, setSlicerErr] = useState<Text | null>(null)
  /* 「去系统里改…」那一颗：点了会弹一个系统窗口/框，防连点 */
  const [slicerToSystem, setSlicerToSystem] = useState(false)
  /*
   * 「去系统里改…」之后那张图片引导开着没有 —— 非 null 就是开着，内容是**壳给的那句话**
   * （成功 = 已打开那份文件的「显示简介」；失败 = 那条错的原话）。见 `openSystemDefaults`。
   */
  const [guide, setGuide] = useState<string | null>(null)
  /* 引导图按**资产 id** 取（见 `guide/openWithGuide.ts`）：URL 只能从这份登记里算 */
  const catalog = useCatalog()

  /*
   * 「保存后的重切引导」共用开关（2026-10-17）：与参数已保存那一屏里的「不再显示」
   * **写同一个键**（`guide/guidesPref`）。这一格是把它**重新打开**的唯一入口 ——
   * 用户在提示里勾过「不再显示」之后，到这里就能解开。写盘**立即生效**（与主题 /
   * 自动清理那几格同一条口径），`useGuidesPref` 会广播一声，已开着的那几处一起变。
   */
  const [showGuides, setShowGuides] = useGuidesPref()

  /*
   * 执行记录（「执行记录」那一块）：`null` = 还没读到（控件先禁用，
   * **不拿一个猜出来的默认值顶上去**）。点一下立即落盘，失败**不改开关**、只把原因摆出来。
   */
  const [autoClean, setAutoClean] = useState<boolean | null>(null)
  const [autoCleanBusy, setAutoCleanBusy] = useState(false)
  const [autoCleanErr, setAutoCleanErr] = useState<Text | null>(null)

  /*
   * **备份与恢复**（2026-10-17）：两个动作都要先问系统要一个位置（文件夹 / 那份 zip）。
   *
   * ★ 「恢复」是**不可撤销**的（它把现在的数据换掉）⇒ 与「清空历史」同一条规矩：
   *   点一下只是武装，再点才真做（二次确认在界面这一侧，后端只管照做）。
   * ★ 恢复完要**整窗重载**：盘上的数据换过了，内存里的都不算数。
   */
  const [backupBusy, setBackupBusy] = useState(false)
  const [restoreBusy, setRestoreBusy] = useState(false)
  /* 恢复的二次确认：点了「从备份恢复」之后摆出来（再点一次才真恢复） */
  const [restoreArm, setRestoreArm] = useState<string | null>(null)
  const [backupNote, setBackupNote] = useState<{ text: Text; bad: boolean } | null>(null)

  /** 备份：问系统"放到哪个文件夹"，剩下的（文件名、打包）全交给后端 */
  const doBackup = useCallback(async () => {
    setBackupBusy(true)
    setBackupNote(null)
    try {
      const picked = await open({ directory: true, title: tr('settings.backup.pickDir') })
      const dir = Array.isArray(picked) ? picked[0] : picked
      if (typeof dir !== 'string' || dir === '') {
        setBackupBusy(false)
        return
      }
      const sum = await api.createBackup(dir)
      setBackupNote({
        text: msg('settings.backup.done', { path: sum.path, files: sum.files }),
        bad: false,
      })
    } catch (e) {
      setBackupNote({ text: msg('settings.backup.failed', { detail: errorText(e) }), bad: true })
    } finally {
      setBackupBusy(false)
    }
  }, [tr])

  /** 恢复：问系统"哪一份备份"，**二次确认**之后才真的覆盖 */
  const doRestore = useCallback(async () => {
    /* 第一下：挑一份备份 + 武装；第二下：拿已经挑好的那份真恢复 */
    if (restoreArm === null) {
      setBackupNote(null)
      try {
        const picked = await open({
          directory: false,
          multiple: false,
          title: tr('settings.restore.pickTitle'),
          filters: [{ name: tr('settings.restore.pickFilter'), extensions: ['zip'] }],
        })
        const at = Array.isArray(picked) ? picked[0] : picked
        if (typeof at !== 'string' || at === '') return
        setRestoreArm(at)
        setBackupNote({
          text: msg('settings.restore.ask', { path: at }),
          bad: false,
        })
      } catch (e) {
        setBackupNote({
          text: msg('settings.restore.pickFailed', { detail: errorText(e) }),
          bad: true,
        })
      }
      return
    }
    setRestoreBusy(true)
    try {
      const sum = await api.restoreBackup(restoreArm)
      /* 盘上换过了 ⇒ 整窗重载（内存里的都不算数） */
      setBackupNote({ text: msg('settings.restore.done', { files: sum.files }), bad: false })
      window.setTimeout(() => window.location.reload(), 600)
    } catch (e) {
      setBackupNote({ text: msg('settings.restore.failed', { detail: errorText(e) }), bad: true })
      setRestoreArm(null)
      setRestoreBusy(false)
    }
  }, [restoreArm, tr])

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
      setAutoCleanErr(msg('settings.history.readFailed', { detail: errorText(e) }))
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
      setNote({ text: msg('settings.advanced.switched', { label: got.label }) })
      await afterSourceChanged()
    } catch (e) {
      setMode((source?.mode as Mode | undefined) ?? 'github')
      setNote({
        text: msg('settings.advanced.switchFailed', {
          label: source?.label ?? tr('settings.advanced.defaultSource'),
        }),
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
   * 换界面语言：与换主题同一条口径 —— **单选本身就是明确意图**，选了就写、立即生效。
   * 写成功后 `appStateMutated()` 读回新档，`appState.ts` 落地时通知 `i18n` 换语言，
   * 所有用 `useT()` 的地方同帧换字（不用重启）。
   *
   * ★ 失败**不改单选**：界面显示的必须是与盘上一致的那一档，原因讲成人话。
   */
  const pickLocale = async (next: LocalePref) => {
    if (localeBusy) return
    setLocaleBusy(true)
    setLocaleErr(null)
    try {
      await api.setLocale(next)
      appStateMutated()
    } catch (e) {
      setLocaleErr(errorText(e))
    } finally {
      setLocaleBusy(false)
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
      setAutoCleanErr(msg('settings.history.writeFailed', { detail: errorText(e) }))
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
        setSlicerErr(msg('settings.slicer.pickFailed', { detail: errorText(e) }))
      } finally {
        setSlicerBusy(false)
      }
    })()
  }

  /*
   * 「去系统里改…」（2026-10-16，作者：「还是不要帮我改了……打开系统那个页面给用户
   * 自己去改」）：**我们不替他改**系统默认应用 —— 系统默认是他自己的账，改不改、改成
   * 什么由他在系统那一侧按。这一颗只把他送过去：
   *   macOS  → 打开那份空的示例 `.3mf` 的**「显示简介」窗口**（要过一次「自动化 → 控制访达」
   *            的授权；只把文件夹定位出来不算 —— 作者 2026-10-16 纠过这一条）；
   *   Windows→ 打开「打开方式」那个框。
   * 两边都在壳那一侧，见 `ipc::slicer`。
   *
   * 回来后**开那张图片引导**（`GuideModal`，不是通知）：壳给的那句是"此刻在哪"
   * （成功 = 已打开那份文件的「显示简介」；失败 = 那条错的原话，里面写着手工怎么做），
   * 引导里那两步是"接着怎么点"。前端不复述、不翻译壳给的那句。
   *
   * ★ 失败**照样开这张引导**：他手里的活没变（还是去访达里改那两下）——
   *   权限被拒（他点了「不允许」）是预期内的一条走法，壳那边已经退回去在访达里定位好了。
   *
   * ★★ **拒绝态要给"反悔"的入口**（2026-10-17 作者：「如果用户后悔了，有再次触发的开关吗」）：
   *   macOS 拒过一次就**永远不会再弹**那个授权框（TCC 没有"再问一次"的 API）——
   *   用户反悔的唯一正路是系统设置 → 自动化里把「访达」勾回来。⇒ 错的码是
   *   `PERMISSION_DENIED` 时，在引导里摆一颗「去重新允许」，点了把**那一页**直达打开
   *   （`openAutomationSettings`），他自己勾完回来再点一次「去系统里改…」就走通了。
   */
  const openSystemDefaults = () => {
    if (slicerToSystem) return
    setSlicerToSystem(true)
    api.openDefaultAppSettings().then(
      (hint) => {
        setSlicerToSystem(false)
        setGuideAskGrant(false)
        setGuide(hint)
      },
      (e: unknown) => {
        setSlicerToSystem(false)
        setGuideAskGrant(isAppError(e) && e.code === 'PERMISSION_DENIED')
        setGuide(errorText(e))
      },
    )
  }

  /** 引导里那颗「去重新允许」摆不摆 —— 只有"自动化授权被拒"这一种错才摆 */
  const [guideAskGrant, setGuideAskGrant] = useState(false)

  /** 把用户送到系统设置 → 自动化那一页（勾「访达」那一下还是他自己来） */
  const openAutomationSettings = async () => {
    try {
      await api.openAutomationSettings()
    } catch (e) {
      setNote({ text: errorText(e), bad: true })
    }
  }

  /* 当前生效的那一句账：读不出来 / 没配 / 你指定的 / 内置源 —— 四种都要说得出 */
  const currentText =
    loadErr !== null
      ? tr('settings.advanced.readFailed', { detail: loadErr })
      : source === null
        ? tr('settings.advanced.unconfigured')
        : source.mode === 'custom'
          ? tr('settings.advanced.customText', {
              address:
                source.address === ''
                  ? tr('settings.advanced.emptyAddress')
                  : source.address,
            })
          : tr('settings.advanced.builtinText', {
              label: source.label,
              address: source.address,
            })

  const canSet = draft.trim() !== ''

  /* 单选显示的永远是**盘上真账**：还没选过（null）时按"跟随系统"那一档显示 */
  const currentTheme: ThemePref = theme ?? 'system'

  /* 语言同理：没选过 = 跟随系统（`null` 与 `'system'` 只在盘上分得开） */
  const currentLocale: LocalePref = localePref ?? 'system'

  /* 切片下拉的选项：内置 + 已指定 + 最后那条动作项（共用件 `SelectField` 要的形状） */
  const slicerList: FieldOption[] = [
    ...slicerOptionsOf(tr, slicerApp, systemApp),
    {
      value: SLICER_CHOOSE,
      label: tr('settings.slicer.other'),
      note: tr('settings.slicer.otherNote'),
    },
  ]
  /* 选着"你从磁盘指定的"时的那个应用（其余档 `null`） */
  const slicerCustom = slicer === 'custom' ? slicerApp : null
  /* `custom` 但记录不在（清过缓存）：按跟随系统显示 —— 值不在选项里时触发器显示原值，更难懂 */
  const slicerShow: SlicerId = slicer === 'custom' && slicerCustom === null ? 'system' : slicer
  /* 「跟随系统默认」在这台机器上指谁那一句（问不到 = `null` ⇒ 那行不摆） */
  const slicerSystem = systemSlicerLine(tr, systemApp)

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

  /**
   * 换源之后**立刻把目录同步一次**（2026-10-30 作者实测：应用完新地址，首页还停在旧目录
   * —— X1C 没图、参数全是「—」，要重启客户端才正常）。
   *
   * 为什么不能只靠 `usePresetData` 里那条换源 effect：它挂在**首页 / 预设页**上，还要过
   * 「本次运行只检查一次」那道闸 —— 换源发生在设置页时，轮不轮得到它得看那两页挂没挂、
   * 谁先跑。这里显式走一遍；**失败不挡保存**（地址已经落盘了），只把原因说出来。
   *
   * 返回 `'same'`（本来同一代）/ `'synced'`（换掉了）/ 其余 = 失败原因那句。
   */
  const syncCatalog = async (): Promise<'same' | 'synced' | string> => {
    try {
      const check = await api.checkRemoteUpdate()
      if (check.upToDate) return 'same'
      /* 读不懂就不换（与预设页同一条规矩）：不落盘，只如实说 */
      if (!check.readable) return 'unreadable'
      await api.applyRemoteUpdate()
      deliveryMutated()
      return 'synced'
    } catch (e: unknown) {
      return isAppError(e) ? e.message : String(e)
    }
  }

  /** 换源成功之后那一步：先报"在了"，同步完再报结果 */
  const afterSourceChanged = async () => {
    sourceChanged()
    setNote({ text: msg('settings.advanced.syncing') })
    const r = await syncCatalog()
    if (r === 'synced' || r === 'same') {
      setNote({ text: msg(r === 'synced' ? 'settings.advanced.synced' : 'settings.advanced.applied') })
      return
    }
    setNote({
      text: msg('settings.advanced.syncFailed', { detail: r }),
      bad: true,
    })
  }

  /* 自定义地址的落盘（先探后写，后端做）。失败不回滚单选：输入框留着让人改地址重试 */
  const apply = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const got = await api.setPresetSource('custom', draft.trim())
      setSource(got)
      /* 地址生效了还要**当场把目录同步过来** —— 不然界面停在旧目录直到重启 */
      await afterSourceChanged()
    } catch (e) {
      setNote({
        text: msg('settings.advanced.applyFailed', {
          label: source?.label ?? tr('settings.advanced.defaultSource'),
        }),
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
   * 左列**只有两行**（状态 + 版本对照）⇒ 动作条竖向居中（2026-10-30 作者：
   * 「这两行的时候，看起来是居中的」）。多出说明 / 失败原因 / 提示任何一段就退回
   * 顶对齐 —— 那几段会把左列撑高，按钮跟着居中就掉到卡片中部去了（a07 的老裁决）。
   */
  const compactUpdate = upd !== null && (notes ?? '').trim() === ''

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

  /*
   * 这一页的根（`.page`）= 「顶栏页签以下那一整块」。交给模态框当 `host`，
   * 遮罩 `absolute inset:0` 才会铺满这一块（与 `PagePresets` 的 `pageEl` 同一个做法）。
   * 只在「最近删除」那个框上用得到 —— 列表搬进框里之后（2026-10-29）。
   */
  const [pageEl, setPageEl] = useState<HTMLDivElement | null>(null)

  /* ---------- 左目录：滚到哪亮到哪（2026-10-30） ---------- */

  const mainRef = useRef<HTMLElement | null>(null)
  const [active, setActive] = useState<string>('upd')

  useEffect(() => {
    const el = mainRef.current
    if (el === null) return

    const spy = () => {
      const line = el.getBoundingClientRect().top + 60
      let cur = 'upd'
      el.querySelectorAll<HTMLElement>('[data-sec]').forEach((sec) => {
        if (sec.getBoundingClientRect().top <= line) cur = sec.id
      })
      setActive(cur)
    }

    spy()
    el.addEventListener('scroll', spy, { passive: true })
    return () => el.removeEventListener('scroll', spy)
  }, [])

  /* 点目录：滚到那一章（与产品别处的 `behavior: smooth` 同一条口径） */
  const jump = (id: string) => (e: MouseEvent<HTMLAnchorElement>) => {
    e.preventDefault()
    const box = mainRef.current
    const el = box?.querySelector<HTMLElement>(`#${id}`)
    if (box !== null && box !== undefined && el !== null && el !== undefined) {
      box.scrollTo({ top: box.scrollTop + el.getBoundingClientRect().top - 8, behavior: 'smooth' })
    }
  }

  /* 分段控件下面那句小字：只说**当前选中的那一档**是什么意思（三档的说明不再同时铺开） */
  const themeRow = THEME_CHOICES.find((c) => c.id === currentTheme)
  const localeRow = LOCALE_CHOICES.find((c) => c.id === currentLocale)

  /* ---------- 页头搜索（2026-10-30，a11 的搜索框补进产品） ----------
   * 只做**视图过滤**：按行文字命中决定显不显示，一个命令都不发、不写 AppState。
   * 行用 inline style 藏（React 没声明过 style.display，重渲染不会把它抹掉）；
   * 章节整块没命中就藏，左目录跟着藏（否则目录里会留一堆点进去是空的项）。 */
  const [q, setQ] = useState('')
  const [hiddenSecs, setHiddenSecs] = useState<string[]>([])
  const [noHit, setNoHit] = useState(false)

  useEffect(() => {
    const el = mainRef.current
    if (el === null) return

    const term = q.trim().toLowerCase()
    const hidden: string[] = []

    el.querySelectorAll<HTMLElement>('[data-sec]').forEach((sec) => {
      let hit = 0
      Array.from(sec.children).forEach((child) => {
        const node = child as HTMLElement
        /* 章节标题（h2）不参与命中判断，也不藏 —— 章还在，只是行被筛掉 */
        if (node.tagName === 'H2') return
        const on = term === '' || (node.textContent ?? '').toLowerCase().includes(term)
        node.style.display = on ? '' : 'none'
        if (on) hit += 1
      })
      const show = term === '' || hit > 0
      sec.style.display = show ? '' : 'none'
      if (!show) hidden.push(sec.id)
    })

    setHiddenSecs(hidden)
    setNoHit(term !== '' && hidden.length === el.querySelectorAll('[data-sec]').length)
  }, [q])

  return (
    <div className={s.page} ref={setPageEl}>
      <header className={s.head}>
        <h1 className={s.title}>{tr('settings.title')}</h1>
        <label className={s.search}>
          <Icon name="search" size={14} />
          <input
            type="search"
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder={tr('settings.search')}
            aria-label={tr('settings.search')}
          />
        </label>
      </header>

      <div className={s.body}>
        <nav className={s.side} aria-label={tr('settings.title')}>
          {NAV.filter((n) => !hiddenSecs.includes(n.id)).map((n) => (
            <a
              key={n.id}
              className={`${s.navLink} ${active === n.id ? s.navOn : ''}`}
              href={`#${n.id}`}
              aria-current={active === n.id ? 'true' : undefined}
              onClick={jump(n.id)}
            >
              <Icon name={n.icon} size={15} />
              {tr(n.key)}
            </a>
          ))}
          <div className={s.sideFoot}>{appVersion}</div>
        </nav>

        <main className={s.main} ref={mainRef}>
          {noHit && <p className={s.note}>{tr('settings.searchEmpty')}</p>}
      {/*
       * 软件更新 —— **a07 提到第一块**（作者 2026-10-13：「有没有必要放在最上面」）。
       * 这一块与预设页的「读不懂」提示是两条链，两个事实不许混成一句话（见文件头）。
       *
       * 卡内一层：**状态在左、动作在右上**（作者看了两版后的裁决：
       * 「他应该放到上面呀…我就觉得他这个位置放在这不舒服」—— 按钮吊在卡片底下
       * 显得空、也够不着）。说明块仍封顶可滚，长文案不会把按钮挤下去。
       */}
      <section className={s.section} id="upd" data-sec aria-label={tr('settings.update.title')}>
        <div className={`${s.updateTop} ${compactUpdate ? s.updateTopCentered : ''}`}>
          <div className={s.updateRow}>
            <div className={s.updateHeadline}>
              {updateBusy ? (
                <span className={s.updateIdle}>{tr('settings.update.checking')}</span>
              ) : hasUpdate ? (
                <span className={s.updateFresh}>{tr('settings.update.hasNew')}</span>
              ) : upd !== null ? (
                /* ★ 「已是最新」是**好消息**（作者 2026-10-05：字体要绿的）——
                   与「有新版本」同一个绿；「没查到」才是灰的（不是坏消息，是不知道）。 */
                <span className={s.updateLatest}>{tr('settings.update.latest')}</span>
              ) : (
                <span className={s.updateIdle}>{tr('settings.update.unknown')}</span>
              )}
            </div>

            {/* 版本对照：当前 → 最新。当前版本一定拿得到（来自构建期） */}
            <p className={s.current}>
              {hasUpdate
                ? tr('settings.update.versionTo', {
                    version: appVersion || '—',
                    latest: upd.latestVersion ?? '—',
                  })
                : tr('settings.update.version', { version: appVersion || '—' })}
            </p>

            {/* 更新说明：长文案时这块自己滚（`.notes` 封顶），卡片高度不再跟着长 */}
            {notes !== undefined && <p className={s.notes}>{notes}</p>}

            {upd === null && !updateBusy && (
              <p className={s.fieldNote}>
                {updateErr !== null
                  ? tr('settings.update.failed', { detail: updateErr })
                  : tr('settings.update.hint')}
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
              {updateBusy ? tr('settings.update.checking') : tr('settings.update.recheck')}
            </Btn>
            {/*
              ★ 「查看更新」用 `api.openUrl`，**不用 `<a target="_blank">`**：
              Tauri 的 webview 没开 opener 权限，`<a>` 点了**什么都不发生**
              （0.0.2 用户实测："点了没反应，没弹出浏览器"）。修法是走命令（第五刀）。
            */}
            {jumpUrl !== undefined && (
              <Btn onClick={() => void openReleasePage(jumpUrl)}>
                {tr('settings.update.view')}
              </Btn>
            )}
            {/* 有安装包（release.json 给了 asset）才摆「更新」；没有就只留左边那两颗 */}
            {hasUpdate && upd?.asset !== undefined && (
              <Btn variant="accent" disabled={updateBusy} onClick={() => void startDownload()}>
                {tr('settings.update.install')}
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
      <section className={s.section} id="appearance" data-sec aria-label={tr('settings.appearance.title')}>
        <h2 className={s.secTitle}>{tr('settings.appearance.title')}</h2>

        {/* 2026-10-30：三行竖排单选 → 一行分段控件（作者：「一条一条的参数、选项太多了」）。
            分段下面那一句只讲**当前这一档**什么意思，不再把三档的说明同时铺开。 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.theme.label')}
              {themeRow !== undefined && <Tip text={tr(themeRow.hint)} />}
            </div>
          </div>
          <div className={s.seg} role="radiogroup" aria-label={tr('settings.theme.label')}>
            {THEME_CHOICES.map((row) => (
              <button
                key={row.id}
                type="button"
                role="radio"
                aria-checked={currentTheme === row.id}
                disabled={themeBusy}
                onClick={() => void pickTheme(row.id)}
              >
                {tr(row.label)}
              </button>
            ))}
          </div>
        </div>

        {themeErr !== null && (
          <p className={`${s.note} ${s.noteBad}`}>
            {tr('settings.theme.failed', { detail: themeErr })}
          </p>
        )}

        {/*
         * 界面语言（2026-10-25）：三档单选，与主题并排在同一块「外观」下。
         * ★ 这一行的文案走字典（`tr`）—— 切到 English 之后它自己先变英文，
         *   这一页其余句子还在阶段 1 的迁移清单里（还没搬）。
         */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.locale.label')}
              {localeRow !== undefined && <Tip text={tr(localeRow.hint)} />}
            </div>
          </div>
          <div className={s.seg} role="radiogroup" aria-label={tr('settings.locale.label')}>
            {LOCALE_CHOICES.map((row) => (
              <button
                key={row.id}
                type="button"
                role="radio"
                aria-checked={currentLocale === row.id}
                disabled={localeBusy}
                onClick={() => void pickLocale(row.id)}
              >
                {tr(row.label)}
              </button>
            ))}
          </div>
        </div>

        {localeErr !== null && (
          <p className={`${s.note} ${s.noteBad}`}>
            {tr('settings.locale.failed', { detail: localeErr })}
          </p>
        )}
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
      <section className={s.section} id="slicer" data-sec aria-label={tr('settings.slicer.title')}>
        <h2 className={s.secTitle}>{tr('settings.slicer.title')}</h2>

        {/* 2026-10-30：一行 —— 左边标题与"现在指向谁"，右边下拉 + 「去系统里改…」。
            完整路径太长不进弹层，仍摆在标题下面一行。 */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.slicer.label')}
              <Tip text={tr('settings.slicer.note')} />
            </div>
            {/*
             * 「跟随系统默认」那一档（2026-10-16）：名字里没有应用名，把它在这台机器上
             * **指谁**摆在标题下面（与弹层里那条 note 是同一句话，`systemSlicerLine`）。
             */}
            {/*
             * 「去系统里改…」2026-10-30 从右侧搬到这里（作者：「这个去系统里改这个按钮，
             * 我不喜欢放在这里吧，反正就是不舒服」）—— 原来它和下拉并排挤在右缘，
             * 两个"框"抢同一块地方、没有主次。现在它退成**说明尾巴上的一条文字链接**：
             * 句子还在这行里，动作缩到最小。
             */}
            {slicerShow === 'system' && (
              <p className={s.slicerHint}>
                {slicerSystem}
                <button
                  type="button"
                  className={s.inlineLink}
                  disabled={slicerToSystem}
                  onClick={openSystemDefaults}
                >
                  {tr('settings.slicer.toSystem')}
                </button>
              </p>
            )}
            {slicerCustom !== null && (
              <p className={s.slicerHint}>
                {tr('settings.slicer.customHandoff', {
                  name: slicerCustom.name,
                  path: slicerCustom.path,
                })}
              </p>
            )}
          </div>

          <div className={s.ctl}>
            {/* 外面这层只为定宽：共用件的 `.wrap` 是 inline-flex，自己不会伸 */}
            <span className={s.slicerPick}>
              <SelectField
                label={tr('settings.slicer.label')}
                options={slicerList}
                value={slicerShow}
                disabled={slicerBusy}
                onChange={pickSlicer}
              />
            </span>
          </div>
        </div>

        {slicerErr !== null && <p className={`${s.note} ${s.noteBad}`}>{say(slicerErr)}</p>}
      </section>

      {/*
       * 引导（2026-10-17 作者：「能不能做一个不再显示的选项……然后设置里面能解开，显示引导的」）。
       *
       * 管的就是"参数保存成功 → 该回切片器重切一次"那一屏（`guide/ResliceNotice`）。
       * ★ 它与那一屏里的「不再显示」**是同一个开关**（`guide/guidesPref` 里那个键），
       *   不是第二份偏好 —— 一处关、处处关；这一格是把它**重新打开**的地方。
       *   **参数页与校准页共用**（同一个组件，同一个键）。
       *
       * 写盘**立即生效**（与主题 / 自动清理那几格同一条口径 —— 开关本身就是明确的意图）；
       * 默认（没设过）是**显示**：只有用户明确关掉才不弹。
       */}
      <section className={s.section} id="guides" data-sec aria-label={tr('settings.guides.title')}>
        <h2 className={s.secTitle}>{tr('settings.guides.title')}</h2>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.guides.label')}
              <Tip text={tr('settings.guides.note')} />
            </div>
          </div>
          <Switch
            checked={showGuides}
            ariaLabel={tr('settings.guides.label')}
            onChange={setShowGuides}
          />
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
      <section className={s.section} id="history" data-sec aria-label={tr('settings.history.title')}>
        <h2 className={s.secTitle}>{tr('settings.history.title')}</h2>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.history.label')}
              <Tip text={tr('settings.history.note')} />
            </div>
          </div>
          <Switch
            checked={autoClean ?? false}
            ariaLabel={tr('settings.history.label')}
            onChange={(v) => void pickAutoClean(v)}
            disabled={autoClean === null || autoCleanBusy}
          />
        </div>

        {autoCleanErr !== null && (
          <p className={`${s.note} ${s.noteBad}`}>{say(autoCleanErr)}</p>
        )}
      </section>

      {/*
       * **备份与恢复**（2026-10-17）。
       *
       * 一个 zip = 这台机器的全部家当：设置（app-state）、目录（catalog）、
       * **全部用户预设 toml（含导入的）**、**修改历史**、云端基准与原文镜像、执行归档。
       * 云端下载区原件 / 日志 / 导出产物不进（那些能重新拿到）。
       *
       * 恢复 = **全量覆盖**（备份里有的一律覆盖、被管的几棵树里备份没有的删掉）——
       * 想回到"某个时间点"就**多留几份备份、挑哪一份恢复**，而不是在一份里挑文件
       * （同名文件"选哪一份"没有天然答案，那是另一整套冲突解决界面）。
       */}
      <section className={s.section} id="backup" data-sec aria-label={tr('settings.backup.title')}>
        <h2 className={s.secTitle}>{tr('settings.backup.title')}</h2>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.backup.label')}
              {/* 原来那句里 `noteKeep` 是加粗强调；气泡是一段纯文本，
                  拼接顺序与原文一致，读起来不缺词 */}
              <Tip
                text={`${tr('settings.backup.notePre')}${tr('settings.backup.noteKeep')}${tr('settings.backup.notePost')}`}
              />
            </div>
          </div>
          <div className={s.ctl}>
            <Btn disabled={backupBusy} onClick={() => void doBackup()}>
              {backupBusy ? tr('settings.backup.busy') : tr('settings.backup.do')}
            </Btn>
          </div>
        </div>

        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>
              {tr('settings.restore.label')}
              <Tip
                text={`${tr('settings.restore.notePre')}${tr('settings.restore.noteKeep')}${tr('settings.restore.notePost')}`}
              />
            </div>
          </div>
          <div className={s.ctl}>
            <Btn disabled={restoreBusy} onClick={() => void doRestore()}>
              {restoreBusy
                ? tr('settings.restore.busy')
                : restoreArm === null
                  ? tr('settings.restore.do')
                  : tr('settings.restore.confirm')}
            </Btn>
            {restoreArm !== null && (
              <Btn
                onClick={() => {
                  setRestoreArm(null)
                  setBackupNote(null)
                }}
              >
                {tr('settings.restore.cancel')}
              </Btn>
            )}
          </div>
        </div>

        {backupNote !== null && (
          <p className={`${s.note} ${backupNote.bad ? s.noteBad : ''}`}>{say(backupNote.text)}</p>
        )}
      </section>

      {/*
       * 后处理那几块 + 预设 + 数据/卸载（2026-10-17）。
       *
       * ★ 作者：「有的事设置页面的，有的到时候我要迁移到其他页面，但是你先都做出来吧，
       *   都放在设置页面」—— 所以**暂时**全摆在这儿。将来迁走时，把对应的那个
       *   组件（`PostprocModeSection` / `CalibSection` / `BedGlueSection` /
       *   `PresetsSection` / `DataSection`）整块搬到目标页就行：它们各自只依赖
       *   `usePostprocPrefs`（或自己那两条命令）与自己的命令，不依赖这一页。
       *
       * ★ 顺序：后处理模式 → 校准与补偿 → 热床预涂胶 → 预设 → 数据与维护 → 清理与卸载。
       *   危险块（热床 / 卸载）放在常规块之后 —— 一路读下来先看常规项。
       */}
      {/* 这几块各自住在自己的文件里（迁页时整块搬走）——
          这里只给它们套一层带 id 的壳，左目录才跳得到、spy 才认得出 */}
      <div id="postproc" data-sec>
        <PostprocModeSection />
      </div>
      <div id="calib" data-sec>
        <CalibSection />
      </div>
      <div id="glue" data-sec>
        <BedGlueSection />
      </div>
      <div id="presets" data-sec>
        <PresetsSection />
      </div>
      <div id="data" data-sec>
        <DataSection host={pageEl} />
      </div>

      <section className={s.section} id="advanced" data-sec aria-label={tr('settings.advanced.title')}>
        <h2 className={s.secTitle}>{tr('settings.advanced.title')}</h2>
        <p className={s.fieldNote}>{tr('settings.advanced.note')}</p>

        {/* 2026-10-30：三行竖排单选 → 一行分段。下面那句就是"现在用的是哪一个"（原在章末） */}
        <div className={`${s.field} ${s.fieldRow}`}>
          <div className={s.fieldText}>
            <div className={s.fieldTitle}>{tr('settings.advanced.sourceLabel')}</div>
            {/* 「现在用的是哪一个」要**常显**（不是说明），所以不走气泡 */}
            <p className={s.current}>
              {tr('settings.advanced.current', { text: currentText })}
            </p>
          </div>
          <div className={s.seg} role="radiogroup" aria-label={tr('settings.advanced.sourceLabel')}>
            {/* ★ 两个固定档 = 防错第一道：用户没有输错地址的机会。选上**立即生效** */}
            {(source?.builtin ?? []).map((b) => (
              <button
                key={b.id}
                type="button"
                role="radio"
                aria-checked={mode === b.id}
                disabled={busy}
                data-tip={
                  b.id === source?.defaultMode
                    ? `${b.address} ${tr('settings.advanced.defaultTag')}`
                    : b.address
                }
                onClick={() => void pick(b.id as Mode)}
              >
                {b.label}
              </button>
            ))}
            <button
              type="button"
              role="radio"
              aria-checked={mode === 'custom'}
              disabled={busy}
              onClick={() => void pick('custom')}
            >
              {tr('settings.advanced.customLabel')}
            </button>
          </div>
        </div>

        {/* 自定义：选上只展开（不写）；按「应用」才先探后写 */}
        {mode === 'custom' && (
          <div className={`${s.field} ${s.fieldRow}`}>
            <div className={s.fieldText}>
              <div className={s.fieldTitle}>
                {tr('settings.advanced.customLabel')}
                <Tip text={tr('settings.advanced.hint')} />
              </div>
            </div>
            <div className={s.ctl}>
              <input
                className={s.input}
                value={draft}
                onChange={(e) => setDraft(e.target.value)}
                placeholder={tr('settings.advanced.addressPlaceholder')}
                aria-label={tr('settings.advanced.addressAria')}
              />
              <Btn onClick={() => void apply()} disabled={busy || !canSet}>
                {busy ? tr('settings.advanced.checking') : tr('settings.advanced.apply')}
              </Btn>
            </div>
          </div>
        )}
        {note !== null && (
          <div className={s.noteBlock}>
            <p className={`${s.note} ${note.bad ? s.noteBad : ''}`}>{say(note.text)}</p>
            {note.detail !== undefined && (
              /* 技术细节收进折叠：普通用户只看结论（作者 2026-10-05：
                 「出了一堆东西我也看不懂，你要给用户看得懂的东西」） */
              <details className={s.noteDetail}>
                <summary>{tr('settings.advanced.details')}</summary>
                <p>{note.detail}</p>
              </details>
            )}
          </div>
        )}
      </section>
        </main>
      </div>

      {/*
       * 图片引导（2026-10-16）：**模态框**，不是通知。
       * 作者看过那条通知之后说的：「这个通知时间有点短呀，你看是做成模态框的还是说延长一下
       * 这个通知时间」—— 加了图之后答案就定了：通知装不下图（通知只是角落里一小条），
       * 而这一步要的正是"照着这张图点"。所以这一处的两条通知都撤掉，换成这张引导。
       *
       * ★ 图与标注住在 `guide/openWithGuide.ts`（浅 / 深两张 + 百分比框），
       *   通用件是 `guide/GuideImage.tsx`（模态框 + 画框那个图），见它们各自的头注。
       *
       * ★ 那两张图**住资产台账**（`presets/assets/guides/` + `presets/assets.toml` 里
       *   `type = 'guide'` 那两条，`delivery = 'bundled'` 先随包）—— 作者 2026-10-16 拍的：
       *   「难道我不应该就直接放在 preset 里面吗？这样方便管理呀，到时候工作台也方便管理」。
       *   所以这一页只做一件事：把**当前 catalog** 交给取图那一步（id → path → URL）。
       *
       * ★ 作者的系统截图（浅 / 深各一张，754×904）**2026-10-16 已经放进来了**，
       *   那两个方框的坐标就是按它量的（量法写在 `openWithGuide.ts` 头注里）。
       *   以后**换图或重裁要重量一次** —— 百分比坐标是相对整张图算的。
       */}
      <GuideImageModal
        open={guide !== null}
        title={tr('settings.guide.title')}
        status={guide ?? ''}
        figure={openWithGuideFigure(catalog.assets, catalog.groups)}
        statusExtra={
          guideAskGrant ? (
            <p className={s.guideGrant}>
              <Btn
                onClick={() => {
                  void openAutomationSettings()
                }}
              >
                {tr('settings.guide.grant')}
              </Btn>
              <span className={s.guideGrantNote}>{tr('settings.guide.grantNote')}</span>
            </p>
          ) : undefined
        }
        steps={[
          {
            text: tr('settings.guide.step1'),
            note: tr('settings.guide.step1Note'),
          },
          {
            text: tr('settings.guide.step2'),
            note: tr('settings.guide.step2Note'),
          },
        ]}
        onClose={() => setGuide(null)}
      />
    </div>
  )
}
