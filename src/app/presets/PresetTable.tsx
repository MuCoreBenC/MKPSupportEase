/*
 * 预设表（**列随类型变** + 常驻操作列）。本地表与云端表共用这一个件。
 *
 * ```
 * MKP 配置（5 列，没有喷嘴层高；「来源」列 2026-10-07 撤了 —— 来源在展开详情里说）
 * 名称                 机型      版本      时间    操作
 * A1.toml              A1       标准版    09-14   已应用
 * 副标题（备注）
 * A1MF_260628.toml     A1       快拆版…   09-14   [使用]
 *
 * 切片器配置（6 列，没有版本）
 * 名称                 机型      喷嘴  层高  时间    操作
 * MKPProcess A1 0.4…   A1       0.4   0.20  09-14   ● 已复制
 * ```
 *
 * # 副标题是备注，不是路径（2026-10-07 作者定；三轮收紧）
 *
 * 名称下面那行小字原来写「下载区 mkp · A1/FAST」这种盘上位置 —— 作者：「副标题不要
 * 这些，改用工作台的备注」。现在两张表两类来源、**互不干扰**：
 *
 *   本地表  备注覆盖账里用户改过的 → 那一版工作台写的 `remark` → 空着
 *   云端表  只读那一版工作台写的 `remark` → 空着（不读覆盖账、没有编辑格）
 *
 * 都没有备注就**空着**（空着**不占位** —— 没备注的行文件名自己垂直居中，有备注的行
 * 名字上移腾位置；位置文案不上副标题，路径永远在 `title` 里，不丢）。
 * 备注可以在展开详情里改（记进覆盖账：「更新不覆盖，删了重新下载才回到工作台那句」）。
 *
 * # MKP 和切片器是两种东西，列不一样
 *
 * 两种类型曾经硬塞进同一套列，于是 MKP 行的喷嘴 / 层高永远是「—」—— 占着两列说废话。
 * 作者原话：「MKP 没有喷嘴，没有层高就不要显示，不要占个位置在那里。那个版本的话呢，
 * 切片器没有版本的话就不要显示版本」。所以切分段控件时**表头列数真的跟着换**：
 *
 *   MKP 配置    名称 / 机型 / 版本 / 时间 / 操作（「来源」列 2026-10-07 撤了，详情里有）
 *   切片器配置  名称 / 机型 / 喷嘴 / 层高 / 时间 / 操作
 *
 * 不留空位、不画「—」占格。「类型」那一列也删了：分段控件已经把类型选定了，
 * 表里每一行都是同一种类型，再画一列写同一个词是重复。
 *
 * 「机型」是单独的一列 —— 机型下拉有「全部机型」一档，那一档下不写机型就分不出
 * 哪一行是哪台机器的。
 *
 * # 「版本」是一列，不是一个筛选器；而且**没有 `+N`**
 *
 * 筛选器的宽度随选项个数线性增长，列的宽度不随行数变 —— 版本再多也只是某一列多几个字。
 * 这一列**不可点、不排序、不分组**：做成可点的就等于把筛选器换个位置放回来。
 * 多个版本共用一个文件时用 `·` 把版本名**全写出来**，宽度不够由 CSS 截断（`title` 里是全的）——
 * `标准版 +2` 那种写法没了，作者看不懂它就是它的问题（见 `versionsText`）。
 *
 * # 「生效」按类型 / 按表各是各的
 *
 *   MKP 本地行   状态 已应用 / 未应用              操作 [使用] → 唯一底账（run/active-preset.json）
 *   MKP 云端行   状态 未获取 / 本地已有 / 有新版    操作 [下载] / [更新] / 灰字（本地已有不给）
 *                → `api.fetchCloudPreset()`（取回云端 + 落一份你的本地文件，**不改当前使用**）
 *   切片器本地行 状态 已复制 / 未复制              操作 [复制] → `api.copyToSlicer()`
 *   切片器云端行 状态 已下载 / 未下载              操作 [下载] → `api.downloadFiles()`（**照抛未实现**）
 *
 * 已生效 / 已下载 / 本地已有那一行的操作列是**灰字，不是按钮**：没有可点的动作。
 * 也**不做「取消应用」** —— 总得有一套在生效。
 *
 * # 操作列常驻，但右键菜单一个都没少
 *
 * `components/menu` 的文件头说「不做行尾的 ⋯ 按钮」，那一条针对的是**把整个菜单再开一个入口**。
 * 这里是作者选的另一件事：把**最常用的那一个动作**摆到明面上（`[应用]` / `[复制]` / `[下载]`），
 * 其余（置顶 / 复制 / 重命名 / 删除 / 在文件夹中显示 / 查看详情）仍然只在右键菜单里。
 *
 * # 表头 sticky 是硬要求
 *
 * 全局 CSS 把滚动条宽度置成 0（幽灵滚动条），滚起来没有任何位置参照 ——
 * 表头钉住是这个约定的配套，不是装饰。
 *
 * # 时间 / 大小：三种来源，title 分三句话
 *
 *   切片器云端行 `statFrom: 'file'` —— 真仓那份文件的字节数 + 上游 manifest 记的更新时间
 *   其余云端行   `PresetFileInfo.sizeText` / `.modifiedText` —— 按 path 的 FNV 哈希稳定推的演示值
 *   发布行       时间 = 发布 / 下载时刻（ISO）、大小 = TOML 字节数 —— 都不是编的
 *
 * **前端一行都不编**：拿不到就写「未知」并在 title 里说清为什么。
 * （用户自己那份的大小与时间**是真值**：后端扫盘拿的，`statFrom: 'file'` 那一档。）
 * 时间显示两种形态：行上只写 `MM-DD`、展开面板带年份 —— 见 `shortStatText` / `longStatText`。
 *
 * # 展开详情（2026-10-10 第八轮：抽卡页「两条独立」落地）
 *
 * ```
 * ┌ [icon] 本地 A1 mini 涂胶…toml  在用 · 2.0 KB · 修改于 …   [收起] ┐  ← 卡头
 * ├ 基本信息                    │ 快捷操作                       ┤
 * │ 版本 / 来源 / 基于           │ [看正文] [编辑]                  │
 * │ 备注 空着 = 副标题留空                              [编辑]     │
 * │ 归属 机型 A1_MINI · 预设类型 FAST                   [编辑]     │
 * ├──────────────────────────────────────────────────────────────┤  ← 编辑带（点「编辑」折出）
 * │ 备注 […………………………]        [恢复默认] [取消] [保存]             │
 * └──────────────────────────────────────────────────────────────┘
 * ```
 *
 * 第五轮压高度、第六轮「文件详情」卡头、第七轮「名字回卡上」——都过了。第八轮照作者
 * 在 `lab/preset-edit-gallery.html` 第 11 条挑定的方案（原话：「现在你这个换的结构
 * 是没问题的，这次终于可以了」）把编辑这一块重排：
 *
 *   1. **展示行住进信息区**：「备注 / 归属」是两行（label 56px —— 与版本 / 来源 / 基于
 *      同一条刻度）+ 右端小胶囊「编辑」；不再是一块独立的「并排两格」（`.dEdits` 退役）。
 *   2. **编辑带折在卡底**：点「编辑」→ 卡底折开一条（高度 `grid-template-rows` 0↔1
 *      + 内容从上落下），备注一条、归属一条，**两条独立、可同开**；再点同一颗「编辑」
 *      收回。卡片自己 `overflow: hidden` 裁圆角（抽卡页 #11 那个关键改动 —— 条不再
 *      自己管圆角、不用任何负 margin）。
 *   3. **备注输入换成自增高 textarea**（原来是单行 input）：内容或**宽度**变了都重算
 *      高度（ResizeObserver 盯宽度 —— 抽卡页上「窗口拉窄高度不跟、打一个字才跳、
 *      拉回来不缩」那个坑，作者点名修过）。
 *   4. **保存成功自动回展示态**：靠「打开时的行数据指纹」—— 页面重读后指纹不匹配，
 *      带在渲染时自然合上（还是收起的动画）；失败指纹不变，留在带里把输入还给用户。
 *   5. 悬浮感照旧靠层次（无边框、轻影子）；卡头加一颗安静的「收起」（与点行同一件事）。
 */


import { Fragment, useCallback, useEffect, useRef, useState } from 'react'
import type { ContextMenuApi } from '../../components/menu'
/* a03：归属控件用 FieldPopover 做可输入 + 可选的自定义组合框（替代原生 select/datalist） */
import { FieldPopover } from '../../components/field'
/* 展开详情卡头那一枚文件图标（2026-10-10，参考稿的绿色文档图标 —— 本项目走描边风格） */
import Icon from '../../components/Icon'
import type { Machine } from '../../api'
/* a03：机型列不再上彩色识别色（色彩收敛）；模块保留，data-machine 仍按机型落值 */
import { machineToneKeyOf } from '../ui/machineTone'
import { longStatText, shortStatText } from '../store/package'
/* a03：所有 WHY（悬停用）导入已随「去悬停提示」移除 */
import {
  ACTION_TEXT,
  archiveOpenText,
  ASSET_STATE_TEXT,
  ASSET_USE_TEXT,
  BASED_ON_KEY,
  basedOnCellText,
  CLOUD_LOCAL_ACTION,
  CLOUD_LOCAL_TEXT,
  CLOUD_STATE_TEXT,
  DASH_,
  EDIT_TEXT,
  LOCAL_DRAWER,
  LOCAL_UNREADABLE_TEXT,
  RELEASE_UNTRUSTED_NOTE,
  UPDATE_ACTION_TEXT,
  UPDATE_STATE_TEXT,
  KIND_AXIS_TEXT,
  LIVE_TEXT,
  originCellOf,
  updateStateOf,
  UNKNOWN,
  versionsText,
} from './presetTree'
import type {
  LocateTarget,
  PresetCloudRow,
  PresetKindAxis,
  PresetLocalRow,
  PresetScopeAxis,
  PresetTableRow,
} from './presetTree'
import s from './PresetTable.module.css'

/**
 * MKP 云端行操作列那一格（2026-10-09）：三态两种动作，**状态与动作用词都从一处出**
 * （[`CLOUD_LOCAL_TEXT`] / [`CLOUD_LOCAL_ACTION`] / [`CLOUD_LOCAL_WHY`]）。
 *
 * 「本地已有」那一档**不给按钮**：那颗按钮按下去没有意义（要改、要用都在本地表那一行上），
 * 而摆一个看起来能点的东西比不摆糟得多 —— 画一枚灰字，`title` 里说清该去哪儿。
 */
function CloudLocalAction({
  row,
  busy,
  onDownload,
}: {
  row: PresetCloudRow
  busy: boolean
  onDownload: (row: PresetTableRow) => void
}) {
  const state = row.localState
  if (state === undefined) return null
  const action = CLOUD_LOCAL_ACTION[state]
  if (action === null) {
    /* 「本地已有」：不给按钮（那件事在本地表那一行上做），画一个破折号 + 为什么 */
    return (
      <span className={s.actNone}>
        {DASH_}
      </span>
    )
  }
  return (
    <button
      type="button"
      className={s.actBtn}
      disabled={busy}
     
      onClick={() => onDownload(row)}
    >
      {action}
    </button>
  )
}

interface Props {
  scope: PresetScopeAxis
  kind: PresetKindAxis
  rows: PresetTableRow[]
  /** 筛前条数 —— 空态要分清「被搜索词筛掉了」与「本来就没有」 */
  total: number
  query: string
  /**
   * 这张表自己的加载失败。
   *
   * 现在到不了这一支：整页三态（loading / error / ready）在 `PagePresets` 就拦住了。
   * 留着是因为**组件不该假设调用方一定先拦** —— 真后端上这两张表是两个不同的读，
   * 一个挂了另一个还能画，那时候这句话就有地方说了。
   */
  failure: string | null
  /** 摊在行上的右键菜单属性（自带 tabIndex） */
  triggerProps: ContextMenuApi<PresetTableRow>['triggerProps']
  /** 菜单正开在哪一行上。行要跟着高亮，否则弹出来之后看不出问的是谁 */
  activeKey: string | null
  /**
   * 展开详情的那一行（rowKey）。**状态放在页面层**：右键菜单里的「查看详情」
   * 也要能展开它，与行上那一下点击共享同一个状态（与参数页的 expandedKey 同构）。
   * null = 全部收起。同一时刻只开一行。
   */
  expandedKey: string | null
  onToggleExpand: (rowKey: string) => void
  /** 正在做动作的那一行（rowKey）。只禁那一颗按钮，不锁整张表 */
  busyKey: string | null
  /**
   * 定位到来源那一行（「复制自 X」可点击那一格的落点）：切轴 + 展开 + 滚动 + 闪光
   * 全在页面层（与状态条「定位」同一个 overlay 机制），表这边只把点击交出去。
   */
  onLocate: (target: LocateTarget) => void
  /**
   * 当前数据源（`gitee` / `github` / `custom`）—— release 行「来源」列显示
   * GitHub / Gitee / 自定义源 的依据。null = 没配源，来源退「云端」。
   */
  sourceLabel: string | null
  /**
   * 这一份在归档区里有几个**旧版本**（按文件名对）。
   * `0` = 没有 → 展开详情里那一格**不显示**（没有 ≠ 未知，不必占一个"—"）
   */
  archiveCountOf: (fileName: string) => number
  /** 打开「旧版本」抽屉（归档的可视化在页面层的抽屉里，表这边只给入口） */
  onOpenArchive: (row: PresetTableRow) => void
  /** 打开「本地这一份」抽屉（本地文件的正文：**只读**） */
  onOpenLocal: (row: PresetTableRow) => void
  /**
   * 开始改这一份（临时编辑那条链）。两边共一个入口：
   * 交付行只给"与目录一致"的那一份；我自己那份都能改（认不出是哪一类的不给）。
   */
  onEdit: (row: PresetTableRow) => void
  /** 本地表那一颗按钮：MKP 是「应用」，切片器是「复制」。两件事一个入口，由 `kind` 分 */
  onLive: (row: PresetLocalRow) => void
  /**
   * **MKP 本地行唯一的按钮：使用**（2026-10-09）。这一档只有 `presets-local/` 里那几行
   * （你自己那份本地文件）——「使用」= 把它设为当前生效。云端那一份的取用
   * （下载 / 更新 = 取回 + 落一份）在云端表，不走这里。
   */
  onUse: (row: PresetLocalRow) => void
  /** 云端表那一颗按钮。**真调 `downloadFiles`，照抛未实现** —— 不编假进度条 */
  onDownload: (row: PresetTableRow) => void
  /**
   * **机型目录**（`data.machines`）：详情面板「归属」编辑格的两个下拉的数据源
   * （机型 → 它的版本）。
   */
  machines: Machine[]
  /**
   * **备注覆盖账**（副标题的键 → 用户改过的备注）：详情面板「备注」格的当前值与
   * 「有没有覆盖」都从它读。**只有本地表的行有备注格** —— 云端行只读工作台那句
   * （`remarkKey === null`，见 `PresetRowBase.remarkKey`）。空对象 = 一个都没改过。
   */
  remarks: Record<string, string>
  /** 改备注（写覆盖账；写什么存什么，空也存；`null` = 恢复默认）。失败由页面说出来 */
  onSetRemark: (key: string, remark: string | null) => Promise<void>
  /** 改用户预设的归属（机型 / 版本，文件头那两行）。失败由页面说出来 */
  onSetAttribution: (
    row: PresetTableRow,
    machineId: string,
    versionId: string,
  ) => Promise<void>
}

export default function PresetTable({
  scope,
  kind,
  rows,
  total,
  query,
  failure,
  triggerProps,
  activeKey,
  busyKey,
  /** 展开详情的那一行(rowKey)。状态在页面层(右键菜单也要能展开它) */
  expandedKey,
  onToggleExpand,
  onLocate,
  sourceLabel,
  archiveCountOf,
  onOpenArchive,
  onOpenLocal,
  onEdit,
  onLive,
  onUse,
  onDownload,
  machines,
  remarks,
  onSetRemark,
  onSetAttribution,
}: Props) {
  /* 这一个布尔决定表头有几列、每行画哪几格。两处都读它，不许各判一次 */
  const mkp = kind === 'mkp'

  /*
   * 展开卡底部那两条编辑带（备注 / 归属）的开关 —— 2026-10-10 第八轮。
   * 存的是**打开那一刻的行数据指纹**（见下面每行的 `remarkBand / attrBand`）：
   * 页面重读后（保存成功 / 恢复默认）指纹对不上，渲染时自然回到展示态 ——
   * 与原来「key 重挂载」同一件事，但现在带是常驻 DOM（收起动画要靠它），
   * 所以由指纹来判。失败时指纹不变，带留在原地把输入还给用户。
   * 同一时刻只有一行展开，所以一份状态就够；换行 / 收起行由下面的 effect 清掉。
   */
  const [bands, setBands] = useState<{ remark: string | null; attr: string | null }>({
    remark: null,
    attr: null,
  })
  useEffect(() => {
    setBands({ remark: null, attr: null })
  }, [expandedKey])

  /*
   * **收起动画 + 点击排队**（2026-10-10 第九/十轮，第十一轮修关键时序）——
   *
   * 收起的动画靠「这一行先留在 DOM 里、把高度收成 0，动画走完再卸载」（CSS `.dFold`），
   * 「哪些行正在收」就是 `closing`。**关键：记账必须和 `onToggleExpand` 在同一次
   * 事件里发生**（同一个批处理、同一帧提交）—— 放在 `useEffect` 里晚一帧，那一帧
   * 旧行就已经被卸载了，收起动画根本没机会播（作者：「收起一点动画都没有，它直接
   * 就消失了」）。做成数组：连着点几行时，没收完的互不打断。
   *
   * 排队规则（作者：「我收起来的时候，我打断了它，我又点了一下展开它好像就卡住了……
   * 动画不要被打断，要执行完。你可以加速把它收起，然后再展开」）：
   *   · 点一个**正在收**的行 → 这一下不掐断收起：记进 `queuedRef`，把当前收起**加速**
   *     （`rush` → CSS 里 240ms 变 140ms），收完（`finishClose`）立刻接着展开；
   *   · 「展开中再点收」不排队：收起直接接过去（过渡从当前高度反向，不跳）。
   */
  const [closing, setClosing] = useState<string[]>([])
  const [rush, setRush] = useState(false)
  const queuedRef = useRef<string | null>(null)
  /** 一次收起的收尾：卸载那一行、撤掉加速，并兑现排着的那次「展开」（如果就是它） */
  const finishClose = (key: string) => {
    setClosing((c) => c.filter((k) => k !== key))
    setRush(false)
    if (queuedRef.current === key) {
      queuedRef.current = null
      onToggleExpand(key)
    }
  }
  /** 让一行开始收：记账（与下面的 `onToggleExpand` 同帧）+ 排好收尾（300ms ≈ 240ms 过渡 + 余量） */
  const startClose = (key: string) => {
    setClosing((c) => (c.includes(key) ? c : [...c, key]))
    window.setTimeout(() => finishClose(key), 300)
  }
  /** 行上那一下点击、卡头那颗「收起」—— 都从这里进（见上面那段说明） */
  const requestToggle = (rowKey: string) => {
    /* 正在收的这一行又被点开：加速收尾 + 排队（收完再展开，不互相掐） */
    if (closing.includes(rowKey)) {
      setRush(true)
      queuedRef.current = rowKey
      window.setTimeout(() => finishClose(rowKey), 160)
      return
    }
    /* 换行：旧行从这一刻开始收（紧跟的 onToggleExpand 会展开新行，同帧提交） */
    if (expandedKey !== null && expandedKey !== rowKey) startClose(expandedKey)
    /* 收起这一行：它自己开始收 */
    if (expandedKey === rowKey) startClose(rowKey)
    onToggleExpand(rowKey)
  }

  /** 空表三种话，说的是三件不同的事 —— 混成一句「没有数据」就查不出是哪一件 */
  const emptyText = (): string => {
    if (failure !== null) return `加载失败：${failure}`
    if (total === 0) {
      /* MKP 那一档是一对表（本地 / 云端），空表按表说 —— 本地是你自己那份，云端是云端 */
      if (mkp) {
        return scope === 'local'
          ? '本地还没有这一档的预设 —— 你自己那份本地文件（presets-local）会出现在这里；从云端「下载」一份就会有'
          : '云端没有这一档的云端预设 —— 云端目录里没有登记的，就下不到'
      }
      return scope === 'local'
        ? `本机还没有这个组合的「${KIND_AXIS_TEXT[kind]}」—— 云端文件下载之后会出现在这里，你自己放进预设目录的也会`
        : `云端没有这个组合的「${KIND_AXIS_TEXT[kind]}」—— 菜单上没有，就下不到`
    }
    return `这一档的 ${total} 条都被搜索词「${query.trim()}」筛掉了`
  }

  /* a03：statWhyOf（时间格悬停 title）已随「去悬停提示」移除。 */

  /** 行上那一格：只写月-日 —— `shortStatText` 统一三种来源的写法；没有不渲染「未知」 */
  const statCell = (row: PresetTableRow) => {
    const text = shortStatText(row.modifiedText)
    return (
      <td className={s.stat}>
        {text !== undefined && <span>{text}</span>}
      </td>
    )
  }

  return (
    <div className={s.hold}>
      <div className={s.wrap}>
        <table className={s.table}>
          <thead className={s.head}>
            <tr>
              <th className={s.thName} scope="col">
                名称
              </th>
              <th className={s.thMachine} scope="col">
                机型
              </th>

              {/*
               * 这里就是「列数真的跟着换」那一下：MKP 只有版本，切片器只有喷嘴 + 层高。
               * 另一种类型的那几列**整个不渲染**，不是渲染成空的 —— 空格子照样占宽度。
               */}
              {mkp ? (
                <th className={s.thVersion} scope="col">
                  版本
                </th>
              ) : (
                <>
                  <th className={s.thNum} scope="col">
                    喷嘴
                  </th>
                  <th className={s.thNum} scope="col">
                    层高
                  </th>
                </>
              )}

              {/*
               * 「时间」这一格的表头随表分语义：云端表是**云端更新**（发布侧盖的戳），
               * 本地表是**时间**（云端行是仓库记的更新时间、交付行是下载时间、本地是修改时间
               * —— 一列三种真来源，格上的 tooltip 说清各自是哪一个，见 `statWhyOf`）。
               *
               * ★ **MKP 档 2026-10-09 又把这一列接回来了**：位置那一轴回来之后，
               * 云端表说的就是"云端这次发布是什么时候发的"（目录只盖了一个戳），
               * 本地表说的是"我这份什么时候改的" —— 两句话又各有主语了。
               */}
              <th className={s.thTime} scope="col">
                {scope === 'cloud' ? '云端更新' : '时间'}
              </th>

              {/*
               * 「来源」列撤了（2026-10-07 作者：「来源不用显示在右侧」）——
               * 它还在展开详情的「来源」格里（那里还能点「复制自 X」定位）。
               */}
              <th className={s.thAct} scope="col">
                操作
              </th>
            </tr>
          </thead>

          <tbody>
            {rows.map((row) => {
              const version = versionsText(row.versions)
              /*
               * 机型识别色（2026-10-08）：`A1` 与 `A1 mini` 只差四个字母，作者在真机上
               * 把 A1 那行看成了 A1 mini、点了下载才发现。一色一机型，见 `ui/machineTone.ts`。
               * 「本地文件」按**文件自己标的归属**上色（`machineToneKeyOf` 收的规矩）。
               */
              const machineToneKey = machineToneKeyOf(row)
              /*
               * 行左边那道绿竖线只标**「正在生效的那一份」**：本地表是 live（MKP 已应用 /
               * 切片器已复制），云端表是 applied（仓库里那一份就是你在用的那一份）。
               * 云端的「已下载」不画线 —— 下载不等于生效，九行里有八行带绿线那道线就没意思了。
               */
              const inUse = row.scope === 'local' ? row.live : row.applied
              const busy = row.rowKey === busyKey
              /*
               * release 行的**云端 vs 盘上**三态（未下载 / 已下载 / 有更新）——
               * 状态主词与按钮都从它出（见 `updateStateOf`）。以前"盘上字节认不出"那一档
               * 把「内容异常」顶在最前面、按钮叫「重新下载」：用户被吓着，而云端明明只是
               * 有更新。现在主词说「有更新」，认不出的实情在展开详情里有一行注记说全。
               */
              const updateState =
                row.releaseState !== undefined ? updateStateOf(row.releaseState) : undefined
              const needsUpdate = updateState === 'update'
              /* 认不出是哪一版的注记只在展开详情里出现（tampered 那一档）——
                 **MKP 档不出现**：那一档按 `localState` 说，不碰下载区四态（2026-10-09） */
              const untrusted =
                row.releaseState === 'tampered' &&
                !(row.scope === 'cloud' && row.localState !== undefined)
              /*
               * 这一份在归档里有几个旧版本（换版本时被换下来的）。0 = 没有。
               * **本地文件那一份问都不问**：归档是云端版本生命周期的事，
               * 用户自己的文件不进归档，也不该因为同名就借到云端的旧版本。
               * **MKP 档整个不显示它**（2026-10-09：云端原件与归档都是内部数据，不进用户世界）
               * —— 归档抽屉只留切片器档。
               */
              const archiveCount = row.origin === 'local' || mkp ? 0 : archiveCountOf(row.fileName)
              /** 云端那一格：与目录一致的那一份在本机（切片器云端行看 `downloaded`，交付行看三态） */
              const gotIt =
                row.scope === 'cloud' &&
                (row.releaseState === undefined ? row.downloaded : row.releaseState === 'ok')
              /* 来源那一格（列 + 展开详情共用）：三种来源三种说法，可定位的带 `locate` */
              const origin = originCellOf(row, sourceLabel)
              /*
               * 缩窄到手：`origin.locate` 的判空进不了 JSX 回调（TS 对属性访问的收窄
               * 不跨函数边界），先落一个本地的量。
               */
              const locateTarget = origin.locate ?? null
              /* 与参数页同一套交互：点行展开下方的内容，同一时刻只开一行 */
              const expanded = row.rowKey === expandedKey
              /*
               * 「这一行还热着」= 展开着，或者正播**收起动画**（在 `closing` 里）——
               * 行白底在这期间保持、卡片留在 DOM 里收高度；两样都读它
               * （见上面 `closing` 那段）。
               */
              const rowHot = expanded || closing.includes(row.rowKey)
              /*
               * 备注 / 归属这两块的身份（第八轮）：
               * `localAttr` = 这一行有没有归属可改（我自己那份、认得出的 MKP 预设，
               * 原来在 JSX 里判两次，现在提一个量）；`ownMachineId / ownVersionId`
               * 是那两个值，显式收窄落成局部量（TS 不认布尔常量的收窄，而展示行与
               * 编辑带都要用它们）；两个 `*Band` 是编辑带的**打开指纹** —— 打开时
               * 记下、渲染时对表，数据重读后对不上就自动合上（见上面 `bands` 那段）。
               */
              const localAttr =
                row.origin === 'local' && row.kind === 'mkp_preset' && row.localState !== 'unreadable'
              const ownMachineId =
                row.origin === 'local' && row.kind === 'mkp_preset' ? (row.ownMachineId ?? null) : null
              const ownVersionId =
                row.origin === 'local' && row.kind === 'mkp_preset' ? (row.ownVersionId ?? null) : null
              const remarkBand = row.remarkKey === null ? null : `${row.rowKey}:${row.subtitle}`
              const attrBand = localAttr
                ? `${row.rowKey}:${ownMachineId ?? ''}:${ownVersionId ?? ''}`
                : null
              return (
                <Fragment key={row.rowKey}>
                <tr
                  className={rowHot ? `${s.row} ${s.rowOpen}` : s.row}
                  data-live={inUse}
                  /* 置顶只有一层**很浅的底**（2026-10-10：不要字、不要图钉，见 CSS 那一段） */
                  data-pinned={row.pinned}
                  data-active={row.rowKey === activeKey}
                  /* 定位滚动/闪光用（「复制自 X」点过来要滚到这一行）——选择器认它 */
                  data-rowkey={row.rowKey}
                  {...triggerProps(row)}
                  onClick={(e) => {
                    /* 点在按钮上不算「展开这一行」—— 按钮有自己的动作 */
                    if ((e.target as HTMLElement).closest('button') !== null) return
                    requestToggle(row.rowKey)
                  }}
                >
                  <td className={s.name}>
                    {/*
                     * 行里的文件名**常显**（第十轮撤回了第七轮那条「展开时隐去」——
                     * 作者：「我希望那个列表的那个文字不要消失」）。卡片头上那一份
                     * 照旧（他要卡片显示名字）：两边都读得到，不再隐来隐去。
                     */}
                    <span className={s.nameLine}>
                      <span className={s.nameText}>
                        {row.fileName}
                      </span>
                      {/*
                        **参数改了还没保存**（2026-10-10）：一枚小圆点，不是徽章 ——
                        作者：「不想用徽章……徽章太多、太占位置」。全文在 `title` 里。
                        置顶是另一回事：它没有标记，只有一层很浅的行底（见 CSS）。
                      */}
                      {row.unsaved && (
                        <span
                          className={s.unsavedDot}
                          role="img"
                          aria-label="有未保存的参数改动"
                          title="有未保存的参数改动 —— 打开「修改参数」接着改"
                        />
                      )}
                      {/*
                        云端表：**下过几代旧版**（2026-10-08）。
                        这一行现在只代表"云上这一个条目"（一个目录条目一行 —— 见
                        `presetTree.cloudRows` 那段：逐版各开一行时会长出三行同名）。
                        旧版本身以用户自己的名字在本地表里，这里只把那个数说出来。
                      */}
                      {row.scope === 'cloud' && (row.olderVersions ?? 0) > 0 && (
                        <span
                          className={s.olderVersions}
                         
                        >
                          旧版 ×{row.olderVersions}
                        </span>
                      )}
                      {/*
                        **我这边怎么样**（MKP 云端行，2026-10-09）：未获取 / 本地已有 / 有新版。
                        这一格回答的是用户点开"云端"时问的那件事 ——
                        "云端有这些，那我手上有没有、要不要拿"。判据在 `cloudRows` 里。
                      */}
                      {row.scope === 'cloud' && row.localState !== undefined && (
                        <span
                          className={s.localState}
                          data-state={row.localState}
                         
                        >
                          {CLOUD_LOCAL_TEXT[row.localState]}
                        </span>
                      )}
                      {/*
                        「云端模板已更新」那枚小字**退役了**（2026-10-09）：云端改过的项
                       现在按**参数逐项**处理（打开参数 → 行上的「用新值 / 保持本地」），
                       不再在列表上留一句
                        "云端那边有新的了"让人自己去猜是哪几项。
                      */}
                      {/*
                       * ★ **「未标机型」那枚角标退役了**（2026-10-10 作者：「这个未标机型
                       * 也没必要显示出来呀，就是说没必要显示这几个字占位置」）。
                       * 数据口径照旧（`untagged` 仍在：机型那格写 `—`、任何机型档下列出来、
                       * 展开详情里的「归属」还能改），只是名称列不再挂那四个字。
                       */}
                      {/*
                       * ★ **"基于旧版官方"那枚徽章退役了**（2026-10-08 作者裁决）：
                       * 用户那份不是"过时文件"，它是一份完整、可继续使用的预设；
                       * 系统既不自动更新它，也不在列表上暗示"你落后了"。
                       * 血统照旧写在文件头（展开详情「基于」那一格还答"从哪一版派生"），
                       * 只是不再据此给状态色。
                       */}
                      {/*
                       * 第九层：读不出来的那一份（编码 / TOML 语法 / 指向用户根之外）——
                       * 一眼看得见；为什么读不出来在 title 里。**照常列出来**，不藏。
                       */}
                      {row.scope === 'local' && row.localState === 'unreadable' && (
                        <span
                          className={s.unreadable}
                         
                        >
                          {LOCAL_UNREADABLE_TEXT}
                        </span>
                      )}
                    </span>
                    {/* 第二行等宽小字：**备注** —— 没有就空着、**不占位**：
                        没备注时文件名自己垂直居中；有备注时两行整块居中
                        （名字上移、备注在下面）。磁盘位置在 title 里，不丢。
                        第十轮起展开时也常显（同上一格，撤回「隐去」）。 */}
                    <span className={s.path}>
                      {row.subtitle}
                    </span>
                  </td>

                  {/*
                    a03：机型格是**安静的中性小标签**，不再上彩色识别色；
                    没有机型（「—」）时**整枚不渲染**。`data-machine` 留着给探针取值。
                  */}
                  <td className={s.machine}>
                    {row.machineText !== DASH_ && (
                      <span
                        className={s.machineChip}
                        data-machine={machineToneKey}
                       
                      >
                        {row.machineText}
                      </span>
                    )}
                  </td>

                  {mkp ? (
                    /* a03：版本列空着不渲染「—」 */
                    <td className={s.version}>
                      {row.versions.length > 0 && (
                        <span>{version.text}</span>
                      )}
                    </td>
                  ) : (
                    <>
                      <td className={s.num}>{row.nozzle ?? ''}</td>
                      <td className={s.num}>{row.layerHeight ?? ''}</td>
                    </>
                  )}

                  {statCell(row)}

                  {/*
                   * 操作列常驻。三种画法，**已经生效 / 已经下载的那一种是灰字不是按钮**：
                   * 已经在用的东西没有可点的动作，给个按钮只会让人点一下看看会发生什么。
                   *
                   * 2026-10-10：这一格**自己吃掉点击**（`stopPropagation`）—— 操作列是
                   * 动作的地盘，点到格子里任何地方（按钮边上、按钮下面）都不该展开行。
                   * 作者的抱怨就是"很容易点击了就导致它展开"：按钮热区小、点偏一格就展开。
                   */}
                  <td className={s.act} onClick={(e) => e.stopPropagation()}>
                    {row.scope === 'cloud' && row.localState !== undefined ? (
                      /*
                       * **MKP 云端行的三个状态、两个动作**（2026-10-09）。
                       *
                       * 判据在 `cloudRows` 里算好了（`localState`）：问我这边有没有一份
                       * **基于它**的本地文件，以及云端换没换版 —— **不看下载区字节**。
                       *
                       *   未获取  → [下载]  取回云端 + 落一份你的（本地表立刻多一行）
                       *   有新版  → [更新]  把云端新版取回来；**你那份一个字节都不动**
                       *   本地已有 → 不给按钮，画一枚灰字（要改要用都在本地表那一行上）
                       *
                       * 三档的"为什么"都在 `title` 里（灰一个动作不说为什么，用户只会以为坏了）。
                       */
                      <CloudLocalAction
                        row={row}
                        busy={busy}
                        onDownload={onDownload}
                      />
                    ) : row.scope === 'local' && mkp ? (
                      /*
                       * **MKP 本地行唯一的动作：使用**（2026-10-09）。
                       *
                       * 这一档只有 `presets-local/` 里那几行（你自己那份本地文件）——
                       * 云端原件不进这张表。使用 = 把它设为当前生效的那一份。
                       * 两处不给按钮（不给点了必被拒的按钮）：认不出是 MKP 预设的（`.json`），
                       * 以及第九层读不出来的。
                       */
                      row.live ? (
                        /*
                         * 正在使用（2026-10-10 第二轮）：**disabled 的同一颗按钮**。
                         *
                         * 以前这里是一个 `<span>「使用中」`——它落在行点击里，点它会把行展开
                         * （作者：「我点击它不要有任何反应，不要说点击了就展开」）。现在：
                         *   · 它是 button → 行点击那一关（`closest('button')`）直接放过；
                         *   · `disabled` → 点它什么也不发生；
                         *   · 与「使用」是**同一颗元素的两个状态** → 切换由 CSS transition
                         *     过渡过去（背景 / 字色 / 阴影），不再是两个节点硬切。
                         * 样式：外阴影=凸起（使用）/ 内阴影=凹下（使用中）—— 见 `.useBtn`。
                         */
                        <button type="button" className={s.useBtn} data-live="true" disabled>
                          {ASSET_USE_TEXT.on}
                        </button>
                      ) : row.origin === 'local' && row.kind !== 'mkp_preset' ? (
                        <span className={s.actNone}>
                          {DASH_}
                        </span>
                      ) : row.origin === 'local' && row.localState === 'unreadable' ? (
                        <span className={s.actNone}>
                          {DASH_}
                        </span>
                      ) : (
                        /* 使用：定宽深色胶囊，用回中文（2026-10-10 作者：不要光秃秃的勾） */
                        <button
                          type="button"
                          className={s.useBtn}
                          disabled={busy}
                         
                          onClick={() => onUse(row)}
                        >
                          {ASSET_USE_TEXT.off}
                        </button>
                      )
                    ) : row.scope === 'local' ? (
                      needsUpdate ? (
                        /*
                         * 盘上那一份与目录不符（旧版本 / 认不出）→ **不给「应用」**。应用会
                         * 拿它去对 SHA，必被拒（`applyActivePreset` 的第一道闸）。按钮统一
                         * 叫「更新」—— 换新版与修坏档是同一条管道、同一个动作；两档的差别
                         * 在 tooltip 里说清（旧版本：换当前版；认不出：我们也不认盘上那份）。
                         * **正在用的那一份也照给**：正在用的东西一样会有更新 —— 它已经在
                         * 名称列转绿、展开详情里写「· 正在用」，这里再画一个灰字反而把
                         * 修它的入口藏掉了（作者实测那份"正在用的旧版"就是这样没地方点）。
                         */
                        <button
                          type="button"
                          className={s.actBtn}
                          disabled={busy}
                         
                          onClick={() => onDownload(row)}
                        >
                          {UPDATE_ACTION_TEXT.update}
                        </button>
                      ) : row.live ? (
                        <span className={s.actDone}>
                          {LIVE_TEXT[kind].on}
                        </span>
                      ) : row.origin === 'local' ? (
                        /*
                         * **用户自己那份也能被应用**（第七层）：与云端那份同一个入口、
                         * 同一条底账 —— "只读"是文件归属的属性，不是"能不能被使用"的属性。
                         * 两档不给按钮（不给必报错的按钮）：认不出是 MKP 预设的（`.json`），
                         * 以及**第九层读不出来的**（后端 `read_preset_text` 的第一关就会拒）。
                         */
                        row.kind !== 'mkp_preset' ? (
                          <span className={s.actNone}>
                            {DASH_}
                          </span>
                        ) : row.localState === 'unreadable' ? (
                          <span className={s.actNone}>
                            {DASH_}
                          </span>
                        ) : (
                          /* 同一颗定宽胶囊：这一档的动作按类型叫（复制 / 应用） */
                          <button
                            type="button"
                            className={s.useBtn}
                            disabled={busy}
                           
                            onClick={() => onLive(row)}
                          >
                            {ACTION_TEXT[kind]}
                          </button>
                        )
                      ) : row.releaseUid !== undefined && row.kind !== 'mkp_preset' ? (
                        /*
                         * 切片器那一类的交付行（catalog 登记、字节在盘上）：动作是**「复制」** ——
                         * 复制进切片器自己的用户配置目录它才生效（MKP 的「启用」是另一回事）。
                         * 2026-10-09 起这条路认**文件名**（与下载 / 应用 / 读正文同一个取用口径），
                         * 交付行接得上了 —— 原来它只认资产库 asset id，交付行只能画个灰杠。
                         */
                        <button
                          type="button"
                          className={s.actBtn}
                          disabled={busy}
                         
                          onClick={() => onLive(row)}
                        >
                          {ACTION_TEXT[kind]}
                        </button>
                      ) : row.releaseUid === undefined && row.kind === 'mkp_preset' ? (
                        /*
                         * **云端 MKP 行没有交付身份（releaseUid）就没有「应用」**（A2 修缝）——
                         * 应用只认目录登记的那一份。两种真实状态共用：用户自己放进
                         * `mkp/` 的来路不明文件；以及旧资产库形状的行（演示数据里有）。
                         * 原来这里放行到「应用」按钮、点了再被守卫拦下，报一串
                         * asset id / 契约的术语 —— 现在按钮根本不给，原因用人话说。
                         */
                        <span className={s.actNone}>
                          {DASH_}
                        </span>
                      ) : row.assetId === undefined && row.releaseUid === undefined ? (
                        /* 什么身份都没有的行：复制 / 应用都无从谈起（不给点了没反应的按钮） */
                        <span className={s.actNone}>
                          {DASH_}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={s.actBtn}
                          disabled={busy}
                         
                          onClick={() => onLive(row)}
                        >
                          {ACTION_TEXT[kind]}
                        </button>
                      )
                    ) : gotIt ? (
                      <span className={s.actDone}>
                        {CLOUD_STATE_TEXT.downloaded}
                      </span>
                    ) : row.releaseUid !== undefined ? (
                      <button
                        type="button"
                        className={s.actBtn}
                        disabled={busy}
                       
                        onClick={() => onDownload(row)}
                      >
                        {needsUpdate ? UPDATE_ACTION_TEXT.update : UPDATE_ACTION_TEXT.missing}
                      </button>
                    ) : (
                      /*
                       * **官方仓库文件不给「下载」**（2026-10-09 用户裁断：残支连入口一起撤）——
                       * 下载只走两条真管道：MKP 的「取回」与登记过的交付下载。
                       */
                      <span className={s.actNone}>
                        {DASH_}
                      </span>
                    )}
                  </td>
                </tr>

                {rowHot && (
                  <tr
                    className={s.expandRow}
                    data-open={expanded}
                    /* 加速（第十轮）：收起动画中这一行又被点了「展开」—— 这条收起快放完 */
                    data-rush={rush && closing.includes(row.rowKey)}
                  >
                    {/*
                     * colSpan 必须**跟着上面的表头列数走**：MKP 5 列（名称 / 机型 / 版本 / 时间 /
                     * 操作）、切片器 6 列（名称 / 机型 / 喷嘴 / 层高 / 时间 / 操作）。
                     * 比表头多出的那一跨会撑出一个匿名列，吃掉表格右侧的全部余量 ——
                     * 改列数时这里要一起改。
                     */}
                    <td colSpan={mkp ? 5 : 6}>
                      {/*
                       * —— 展开 / 收起的小动画（2026-10-10 第九轮，抽卡页那套）——
                       * 表格里没法给 `<tr>` 直接做高度过渡（高度由内容撑），所以把
                       * 折动画放在 td 里这一层 `.dFold` 上（`grid-template-rows`
                       * 0fr ↔ 1fr），tr 高度跟着它逐帧走。
                       * 入场：挂载即播（条件渲染没有「上一帧」，用 CSS animation）；
                       * 退场：收起时这一行先留在 DOM 里（`closing`）把高度收回 0，
                       * 动画走完再卸载（见上面 `closing` 那段）。
                       * 三段内衬：`.dFold`（折）→ `.dFoldCut`（裁）→ `.dFoldIn`
                       * （内容落下 + 内边距 —— 卡片影子要的余地也在这里，
                       * 贴着裁剪边影子会被切）。
                       */}
                      <div className={s.dFold}>
                        <div className={s.dFoldCut}>
                          <div className={s.dFoldIn}>
                      {/*
                       * a03：详情是一张**浮起的白卡**（白底 + 圆角 + 投影），
                       * 不再是全宽染色块；colSpan 仍跟表头：MKP 5、切片器 6。
                       */}
                      <div className={s.detailCard}>
                        {/*
                         * 内衬（第八轮）：卡自己只管外形（圆角 / 裁切 / 影子），
                         * 四周内边距落这一层 —— 卡底两条编辑带要**横向通铺**到卡的边，
                         * 不能被卡的 padding 挤进来（抽卡页 #11 的 `.p1` / `.p1-main` 做法）。
                         */}
                        <div className={s.dMain}>
                        {/*
                         * —— 卡头（2026-10-10 第七轮：名字回到卡上；第八轮加「收起」）——
                         * 作者看过第六轮后改判：「这个卡片，它显示名字，然后上面的名字
                         * 不显示」—— 卡头写回文件名（上面那一行的名称列在展开时隐去，
                         * 见 `.nameLine` / `.path` 的 `data-hidden`），右侧小事实照旧；
                         * 最右一颗安静的「收起」（与点这一行同一件事 —— 抽卡页 #11 有）。
                         */}
                        <div className={s.dHead}>
                          <Icon name="doc" size={15} className={s.dHeadIcon} />
                          <span className={s.dHeadName}>{row.fileName}</span>
                          <span className={s.dHeadMeta}>
                            <span>
                              {row.scope === 'cloud' && row.localState !== undefined
                                ? CLOUD_LOCAL_TEXT[row.localState]
                                : row.releaseState !== undefined
                                  ? UPDATE_STATE_TEXT[updateState ?? 'latest']
                                  : mkp && row.scope === 'local'
                                    ? ASSET_STATE_TEXT[row.live ? 'on' : 'off']
                                    : row.scope === 'local'
                                      ? LIVE_TEXT[kind][row.live ? 'on' : 'off']
                                      : row.downloaded
                                        ? CLOUD_STATE_TEXT.downloaded
                                        : CLOUD_STATE_TEXT.pending}
                              {/*
                               * 「正在用」只在状态主词没说这件事时补 —— 主词已经写着
                               * 「在用 / 已复制」的（本地行 live 那一档）不再追加，
                               * 否则就是「在用 · 正在用 · 2.0 KB」这种同一句话说两遍
                               * （作者截图点名）；云端行的主词说的是"我这边取到没有"，
                               * 与「正在用」是两件事，照旧都要说。
                               */}
                              {row.applied && row.scope !== 'local' && ' · 正在用'}
                            </span>
                            <i className={s.dDot} aria-hidden>·</i>
                            <span>{row.sizeText ?? UNKNOWN}</span>
                            <i className={s.dDot} aria-hidden>·</i>
                            <span>
                              {row.origin === 'release'
                                ? row.scope === 'cloud'
                                  ? '云端更新 '
                                  : row.arrivalBy === 'replaced' ? '替换于 ' : '下载于 '
                                : '修改于 '}
                              {longStatText(row.modifiedText) ?? UNKNOWN}
                            </span>
                          </span>
                          {/*
                            收起：与点这一行（再点一次）同一件事 —— 安静的小按钮，
                            不抢摘要；也走 `requestToggle`（收起动画中点它 = 排队再开）
                          */}
                          <button
                            type="button"
                            className={s.dHeadFold}
                            title="收起详情（与点这一行同一件事）"
                            onClick={() => requestToggle(row.rowKey)}
                          >
                            收起
                          </button>
                        </div>

                        {/*
                         * —— 主体两栏（参考稿那三栏里，「文件信息」并进了卡头）——
                         * 左栏「基本信息」；右栏「快捷操作」在下面（由 `</div>` 收口）。
                         * 窄卡两栏自己折成上下（flex-wrap 在 CSS）—— 第六轮那个"窄卡把
                         * 快捷操作顶到上方"的 container 规则，作者看过效果说乱，第七轮撤了。
                         */}
                        <div className={s.dGrid}>
                          <div className={s.dCol}>
                            <span className={s.dColLabel}>基本信息</span>
                            <div className={s.dGroup}>
                              {mkp ? (
                                row.versions.length > 0 && (
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>版本</span>
                                    <span className={s.dVal}>{version.text}</span>
                                  </div>
                                )
                              ) : (
                                <>
                                  {row.nozzle != null && (
                                    <div className={s.dRow}>
                                      <span className={s.dKey}>喷嘴</span>
                                      <span className={s.dVal}>{row.nozzle}</span>
                                    </div>
                                  )}
                                  {row.layerHeight != null && (
                                    <div className={s.dRow}>
                                      <span className={s.dKey}>层高</span>
                                      <span className={s.dVal}>{row.layerHeight}</span>
                                    </div>
                                  )}
                                </>
                              )}

                              <div className={s.dRow}>
                                <span className={s.dKey}>来源</span>
                                <span className={s.dVal}>
                                  {locateTarget !== null ? (
                                    <button
                                      type="button"
                                      className={s.dLink}
                                      onClick={() => onLocate(locateTarget)}
                                    >
                                      {origin.text}
                                    </button>
                                  ) : (
                                    <span>{origin.text}</span>
                                  )}
                                </span>
                              </div>

                              {/* 血统：我那份从哪一版改出来的 */}
                              {row.origin === 'local' && (
                                <div className={s.dRow}>
                                  <span className={s.dKey}>{BASED_ON_KEY}</span>
                                  <span className={s.dVal}>{basedOnCellText(row)}</span>
                                </div>
                              )}
                            </div>
                          </div>

                          {/* 右栏：快捷操作 —— 原「卡底」那三颗按钮搬上来（2026-10-10） */}
                          {(row.origin === 'local' || archiveCount > 0) && (
                            <div className={s.dCol}>
                              <span className={s.dColLabel}>快捷操作</span>
                              <div className={s.dActs}>
                                {archiveCount > 0 && (
                                  <button
                                    type="button"
                                    className={s.ghostBtn}
                                    onClick={() => onOpenArchive(row)}
                                  >
                                    {archiveOpenText(archiveCount)}
                                  </button>
                                )}
                                {row.origin === 'local' && (
                                  <button
                                    type="button"
                                    className={s.ghostBtn}
                                    onClick={() => onOpenLocal(row)}
                                  >
                                    {LOCAL_DRAWER.open}
                                  </button>
                                )}
                                {row.origin === 'local' &&
                                  row.kind === 'mkp_preset' &&
                                  row.localState !== 'unreadable' && (
                                    <button
                                      type="button"
                                      className={s.ghostBtn}
                                      onClick={() => onEdit(row)}
                                    >
                                      {EDIT_TEXT.open}
                                    </button>
                                  )}
                              </div>
                            </div>
                          )}
                        </div>

                        {/* release 行的版本指纹（云端最新版本/发布时间），第二行 muted meta */}
                        {row.origin === 'release' && (
                          <div className={s.dMeta}>
                            {row.scope === 'local' &&
                              row.ownPublishedAt != null &&
                              row.ownPublishedAt !== row.publishedAt && (
                                <>
                                  <span>
                                    本机这份发布于 {longStatText(row.ownPublishedAt) ?? UNKNOWN}
                                  </span>
                                  <i className={s.dDot} aria-hidden>·</i>
                                </>
                              )}
                            {row.scope === 'local' && (
                              <>
                                <span>
                                  云端最新版发布于{' '}
                                  {longStatText(row.publishedAt ?? undefined) ?? UNKNOWN}
                                </span>
                                <i className={s.dDot} aria-hidden>·</i>
                              </>
                            )}
                            <span>
                              云端最新版本 {row.releaseVersion ?? UNKNOWN}
                            </span>
                          </div>
                        )}

                        {/* 认不出的注记：不顶在主词上，放成讲道理的一行 */}
                        {untrusted && (
                          <p className={s.dNote}>
                            {RELEASE_UNTRUSTED_NOTE}
                          </p>
                        )}

                        {/*
                         * —— 备注 / 归属（2026-10-10 第八轮：展示行 + 卡底编辑带）——
                         * 展示行就在这里：label 与「版本 / 来源 / 基于」同一条 56px 刻度、
                         * 右端一颗小胶囊「编辑」；点它折出的是**卡底那条带**（本块 DOM
                         * 之后的 `.dBand`，备注一条、归属一条、两条独立可同开）。
                         * 两块都没有（云端行 / release 行）时整块不渲染。
                         */}
                        {(row.remarkKey !== null || localAttr) && (
                          <div className={s.editrows}>
                            {row.remarkKey !== null && (
                              <div className={s.dRow}>
                                <span className={s.dKey}>备注</span>
                                <span
                                  className={s.editValue}
                                  data-empty={row.subtitle.trim() === ''}
                                >
                                  {row.subtitle.trim() === ''
                                    ? '空着 = 副标题留空'
                                    : row.subtitle}
                                </span>
                                <button
                                  type="button"
                                  className={s.editTrigger}
                                  title="编辑备注 —— 列表里文件名下面那行小字"
                                  onClick={() =>
                                    setBands((b) => ({
                                      ...b,
                                      remark: b.remark === remarkBand ? null : remarkBand,
                                    }))
                                  }
                                >
                                  编辑
                                </button>
                              </div>
                            )}

                            {localAttr && (
                              <div className={s.dRow}>
                                <span className={s.dKey}>归属</span>
                                <span className={s.editValue}>
                                  <span className={s.editKey}>机型</span>
                                  <span>{ownMachineId ?? DASH_}</span>
                                  <span className={s.editKey}>预设类型</span>
                                  <span>{ownVersionId ?? DASH_}</span>
                                </span>
                                <button
                                  type="button"
                                  className={s.editTrigger}
                                  title="编辑归属 —— 写进文件头的 # machine: / # variant: 两行"
                                  onClick={() =>
                                    setBands((b) => ({
                                      ...b,
                                      attr: b.attr === attrBand ? null : attrBand,
                                    }))
                                  }
                                >
                                  编辑
                                </button>
                              </div>
                            )}
                          </div>
                        )}

                        </div>

                        {/*
                         * —— 卡底两条编辑带（第八轮）：备注 / 归属各一条、独立开合 ——
                         * 常驻 DOM（收起时高度归零，折动画见 CSS `.dBand`）；
                         * `open` = 「指纹对得上」—— 保存成功、页面重读后指纹变了，
                         * 这里自然合上（原来「key 重挂载回展示态」那件事的等价物）。
                         */}
                        {remarkBand !== null && (
                          <RemarkBand
                            open={bands.remark === remarkBand}
                            current={row.subtitle}
                            hasOverride={
                              row.remarkKey !== null && remarks[row.remarkKey] !== undefined
                            }
                            busy={row.rowKey === busyKey}
                            onSave={(remark) => onSetRemark(row.remarkKey ?? '', remark)}
                            onClose={() => setBands((b) => ({ ...b, remark: null }))}
                          />
                        )}
                        {attrBand !== null && (
                          <AttrBand
                            open={bands.attr === attrBand}
                            busy={row.rowKey === busyKey}
                            machines={machines}
                            machineId={ownMachineId}
                            versionId={ownVersionId}
                            onSave={(m, v) => onSetAttribution(row, m, v)}
                            onClose={() => setBands((b) => ({ ...b, attr: null }))}
                          />
                        )}

                      </div>
                          </div>
                        </div>
                      </div>
                    </td>
                  </tr>
                )}
                </Fragment>
              )
            })}
          </tbody>

        </table>

        {rows.length === 0 && <p className={s.empty}>{emptyText()}</p>}
      </div>
    </div>
  )
}

/* ——————————————————————————————————————————————————————————————
 * 卡底的两条编辑带（备注 / 归属）—— 2026-10-10 第八轮，抽卡页 #11 落地。
 * 两条独立、可同开；常驻 DOM（收起时高度归零），折动画见 CSS `.dBand`。
 * 输入状态都在自己的小盒子里，保存交回页面层（错误由页面说出来）——
 * 「保存成功自动回展示态」由外层的**打开指纹**负责（数据重读 → 指纹不匹配 → 合上）。
 * —————————————————————————————————————————————————————————————— */

/**
 * **备注带**。`current` = 账上有覆盖时的那句（没有覆盖时是回落链的显示值）。
 * **用户写什么就是什么 —— 包括空串**（2026-10-07 作者改口：「可以空着，不要回退」：
 * 存空副标题就空着，不偷偷退回工作台那句）；「恢复默认」才是把覆盖拿掉、
 * 退回落入「工作台写的 → 空着」。恢复默认只在**真有覆盖**时出现。
 *
 * 第八轮三件事：输入从单行 `<input>` 换成**自增高 `<textarea>`**（备注可以很长）；
 * 高度在**内容或宽度**变化时都重算（抽卡页上「窗口拉窄折行变多、高度不跟，打一个字
 * 才跳高、拉回来不缩」那个坑，作者点名修 —— ResizeObserver 盯宽度，宽度没变就跳过，
 * 因为自己改高度也会触发回调、不跳过就打转）；打开时光标进输入框（原来靠 `autoFocus`，
 * 现在是常驻 DOM，`autoFocus` 只会在页面初次挂载时响、还可能把焦点抢到收着的带上）。
 *
 * Enter 保存、Shift+Enter 换行（备注可以是多行的）、Esc 取消 —— 与按钮同一件事。
 */
function RemarkBand(props: {
  open: boolean
  busy: boolean
  /** 当前显示值（有覆盖 = 用户的；没有 = 回落链的那句） */
  current: string
  hasOverride: boolean
  onSave: (remark: string | null) => Promise<void>
  /** 合上这条带（取消 / 再点同一颗「编辑」） */
  onClose: () => void
}) {
  const [text, setText] = useState(props.current)
  const [saving, setSaving] = useState(false)
  const taRef = useRef<HTMLTextAreaElement>(null)
  /*
   * 下面两件事（复位 / 自增高）的依赖都用局部量：`props.open` / `props.current`
   * 直接进依赖数组会被 react-hooks 的 lint 当成「可变成员」告警。
   */
  const open = props.open
  const current = props.current
  /* 动过输入框才算有要存的 —— 初始值就是现在显示的那一句；存空也是存（空也是覆盖） */
  const dirty = text !== current
  /* 打开时从当前显示值重来（首开、重开、数据重读之后都算）——不带上次没存完的草稿 */
  useEffect(() => {
    if (!open) return
    setText(current)
    /*
     * 光标进输入框，但**不许它带滚动**（`preventScroll`）——默认的 focus 会把
     * 元素滚进视口，折动画刚起步就滚一下，看着像「停了一下」（第九轮）。
     */
    taRef.current?.focus({ preventScroll: true })
  }, [open, current])
  /** 高度贴内容：先归 auto 再量 scrollHeight（textarea 的常规玩法） */
  const fit = useCallback(() => {
    const el = taRef.current
    if (el === null) return
    el.style.height = 'auto'
    el.style.height = `${el.scrollHeight}px`
  }, [])
  /*
   * 内容变了就重算 —— **「打开」这件事不算**（第九轮）：打开瞬间重算高度会让
   * 折动画的目标高度中途变一下（transition 得重算一次），看着就是「停一下」。
   * 收着的时候宽度和内容一样有效，高度平时就是对的（宽度由下面那个 RO 盯）。
   */
  useEffect(() => {
    fit()
  }, [text, fit])
  /*
   * 宽度变了（窗口拉宽拉窄 → 折行数变）也重算：ResizeObserver 盯**宽度**，
   * 宽度没变就跳过 —— 自己改高度也会触发回调，不跳过会打转。
   */
  useEffect(() => {
    const el = taRef.current
    if (el === null) return
    let w = el.clientWidth
    const ro = new ResizeObserver(() => {
      if (el.clientWidth === w) return
      w = el.clientWidth
      fit()
    })
    ro.observe(el)
    return () => ro.disconnect()
  }, [fit])
  const save = async (remark: string | null) => {
    setSaving(true)
    try {
      await props.onSave(remark)
    } finally {
      setSaving(false)
    }
  }
  return (
    <div className={s.dBand} data-open={open}>
      <div className={s.dBandCut}>
        <div className={s.dBandIn}>
          <div className={s.dBandPane}>
            {/*
             * 一行式（与抽卡页 #11 同形）：label 在左、自增高 textarea 居中、
             * 按钮贴右端 —— 按钮一律 `flex-shrink: 0`，窄了也先挤输入框（CSS）。
             */}
            <div className={s.bandHead}>
              <span className={s.dKey}>备注</span>
              <textarea
                ref={taRef}
                className={s.remarkArea}
                rows={1}
                value={text}
                placeholder="空着 = 副标题留空"
                disabled={props.busy}
                onChange={(e) => setText(e.target.value)}
                onKeyDown={(e) => {
                  /* Enter 保存、Shift+Enter 换行；Esc = 取消（与按钮同一件事） */
                  if (e.key === 'Enter' && !e.shiftKey) {
                    e.preventDefault()
                    if (!saving && !props.busy && dirty) void save(text.trim())
                  }
                  if (e.key === 'Escape') props.onClose()
                }}
              />
              {props.hasOverride && (
                <button
                  type="button"
                  className={s.bandQuiet}
                  disabled={saving || props.busy}
                  title="拿掉覆盖，退回工作台写的那句（没有就空着）"
                  onClick={() => void save(null)}
                >
                  恢复默认
                </button>
              )}
              <button
                type="button"
                className={s.ghostBtn}
                disabled={saving || props.busy}
                onClick={props.onClose}
              >
                取消
              </button>
              <button
                type="button"
                className={s.saveBtn}
                disabled={saving || props.busy || !dirty}
                onClick={() => void save(text.trim())}
              >
                保存
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

/* ——————————————————————————————————————————————————————————————
 * 组合框（可输入 + 可选）。a03：机型与版本统一用这一件，
 * 替代原生 select 与黑色原生 datalist 弹层；候选浮层走 FieldPopover。
 * —————————————————————————————————————————————————————————————— */

/** 一条候选：`value` = 选中后填入（写进文件头的值） */
interface ComboOption {
  value: string
  /** 主行 */
  title: string
  /** 主行右侧小字 */
  right?: string
  /** 第二行描述 */
  sub?: string
}

function ComboField(props: {
  value: string
  onValue: (next: string) => void
  options: ComboOption[]
  placeholder?: string
}) {
  const wrapRef = useRef<HTMLSpanElement>(null)
  const [open, setOpen] = useState(false)
  const cur = props.value.trim()
  const pick = (v: string) => {
    props.onValue(v)
    setOpen(false)
  }
  return (
    <span className={s.comboHold} ref={wrapRef}>
      <input
        className={s.comboInput}
        value={props.value}
        placeholder={props.placeholder}
       
        onFocus={() => setOpen(true)}
        onChange={(e) => {
          props.onValue(e.target.value)
          setOpen(true)
        }}
        onKeyDown={(e) => {
          if (e.key === 'Enter') setOpen(false)
        }}
      />
      {/* 展开候选的小箭头：在锚点（wrap）内部，点击不会被浮层当成外部点击关掉 */}
      <button
        type="button"
        className={s.comboCaret}
        tabIndex={-1}
        aria-label="展开候选"
       
        onClick={() => {
          setOpen((v) => !v)
          wrapRef.current?.querySelector('input')?.focus()
        }}
      >
        ▾
      </button>
      {open && (
        <FieldPopover
          anchor={wrapRef.current}
          matchWidth
          onClose={() => setOpen(false)}
        >
          <div className={s.comboMenu}>
            {props.options.length === 0 && (
              <p className={s.comboEmpty}>没有候选，可以直接输入</p>
            )}
            {props.options.map((o) => (
              <button
                key={o.value}
                type="button"
                className={s.comboItem}
                data-on={o.value === cur}
               
                onClick={() => pick(o.value)}
              >
                <span className={s.comboCheck} aria-hidden>
                  ✓
                </span>
                <span className={s.comboText}>
                  <span className={s.comboTitle}>{o.title}</span>
                  {o.sub !== undefined && <span className={s.comboSub}>{o.sub}</span>}
                </span>
                {o.right !== undefined && (
                  <span className={s.comboRight}>{o.right}</span>
                )}
              </button>
            ))}
          </div>
        </FieldPopover>
      )}
    </span>
  )
}

/**
 * **归属带**（机型 / 预设类型）：两个**组合框**（都能自己输入、也能选候选）。
 * 写进文件头 `# machine:` / `# variant:` 两行（正文一字节不动）；
 * 两边都填上、且有改动时「保存」才可点。内部字段照旧叫 machineId / versionId。
 *
 * 第八轮与备注带同一条规矩：折在卡底、独立开合、常驻 DOM；label 与展示行
 * 同一条 56px 刻度，名字明标「机型 / 预设类型」（作者：「归属区域明确标出
 * 机型和预设类型」）；换机型清掉预设类型（候选跟着机型换）。
 * 「成功自动回展示态」同样靠外层的打开指纹（数据重读 → 指纹不匹配 → 合上）。
 */
function AttrBand(props: {
  open: boolean
  busy: boolean
  machines: Machine[]
  machineId: string | null
  versionId: string | null
  onSave: (machineId: string, versionId: string) => Promise<void>
  /** 合上这条带（取消 / 再点同一颗「编辑」） */
  onClose: () => void
}) {
  const [machineId, setMachineId] = useState(props.machineId ?? '')
  const [versionId, setVersionId] = useState(props.versionId ?? '')
  const [saving, setSaving] = useState(false)
  /* 打开时从当前值重来（首开、重开、数据重读之后都算）——不带上次没存完的草稿 */
  useEffect(() => {
    if (!props.open) return
    setMachineId(props.machineId ?? '')
    setVersionId(props.versionId ?? '')
  }, [props.open, props.machineId, props.versionId])
  const versions = props.machines.find((m) => m.id === machineId.trim())?.versions ?? []
  const dirty = machineId !== (props.machineId ?? '') || versionId !== (props.versionId ?? '')
  const ready = machineId.trim() !== '' && versionId.trim() !== ''

  const machineOptions: ComboOption[] = props.machines.map((m) => ({
    value: m.id,
    title: m.display,
    right: m.id !== m.display ? m.id : undefined,
  }))
  const versionOptions: ComboOption[] = versions.map((v) => ({
    value: v.id,
    title: v.id,
    sub: v.name,
  }))

  return (
    <div className={s.dBand} data-open={props.open}>
      <div className={s.dBandCut}>
        <div className={s.dBandIn}>
          <div className={s.dBandPane}>
            {/* 一列两行：label 56px + 框占满剩余（与展示行、上面信息行同刻度） */}
            <div className={s.bandRow}>
              <span className={s.dKey}>机型</span>
              <ComboField
                value={machineId}
                onValue={(v) => {
                  setMachineId(v)
                  /* 换机型：版本候选跟着换，已填版本清空 */
                  setVersionId('')
                }}
                options={machineOptions}
                placeholder="选择或自己填"
              />
            </div>
            <div className={s.bandRow}>
              <span className={s.dKey}>预设类型</span>
              <ComboField
                value={versionId}
                onValue={setVersionId}
                options={versionOptions}
                placeholder="选择或自己填"
              />
            </div>
            <div className={s.bandFoot}>
              <button
                type="button"
                className={s.ghostBtn}
                disabled={saving || props.busy}
                onClick={props.onClose}
              >
                取消
              </button>
              <button
                type="button"
                className={s.saveBtn}
                disabled={saving || props.busy || !ready || !dirty}
                onClick={() =>
                  void (async () => {
                    setSaving(true)
                    try {
                      await props.onSave(machineId.trim(), versionId.trim())
                    } finally {
                      setSaving(false)
                    }
                  })()
                }
              >
                保存
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
