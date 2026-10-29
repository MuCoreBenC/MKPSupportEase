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
 * │ 版本 / 来源 / 基于           │ [旧版本 N 份] [编辑]             │
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
/* 组合框那颗下拉箭头走**共享那一颗**（`ui.selectCaret`）：本页别处的图标在
   `components/Icon`，但箭头要和报告页 / 参数卡 / 历史抽屉是同一颗形状 —— 两个图标集
   并存是既有的债（见 `app/shell/icons` 头注释），箭头这一处先统一 */
import Chevron from '../shell/icons'
import ui from '../ui/ui.module.css'
import { useT } from '../../i18n'
import type { Tr } from '../../i18n'
import type { Machine } from '../../api'
/*
 * ★ 2026-10-14：原来这里 `import { machineToneKeyOf } from '../ui/machineTone'`，
 * 给机型胶囊落一个 `data-machine` 属性 —— a03「色彩收敛」之后**那个属性已经没有任何消费者**
 * （CSS 里没有 `[data-machine]` 选择器、探针也没读它），整条链（导入 + 变量 + 属性 +
 * `ui/machineTone.ts` 整个模块）一起删了。
 */
import { longStatText, shortStatText } from '../store/package'
/* a03：所有 WHY（悬停用）导入已随「去悬停提示」移除 */
import {
  ACTION_TEXT,
  archiveOpenText,
  ASSET_STATE_TEXT,
  ASSET_USE_TEXT,
  BASED_ON_KEY,
  basedOnCellText,
  RELEASE_LINE_KEY,
  releaseLineText,
  cloudClaimBadge,
  CLOUD_LOCAL_ACTION,
  CLOUD_LOCAL_TEXT,
  CLOUD_STATE_TEXT,
  DASH_,
  LOCAL_DRAWER,
  LOCAL_UNREADABLE_TEXT,
  PARAMS_TEXT,
  PRESET_UNTRUSTED_NOTE,
  UPDATE_ACTION_TEXT,
  UPDATE_STATE_TEXT,
  KIND_AXIS_TEXT,
  LIVE_TEXT,
  USAGE_HINT,
  USAGE_KEY,
  usageCellText,
  isDetachedCopy,
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
  PresetSort,
  PresetSortKey,
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
/**
 * 展开详情里「发布线」那一行（2026-10-28）。
 *
 * **`text == null` 就整行不渲染** —— 老目录 / 老副本没有这条信息，摆一行"未知"
 * 只会让人以为界面没做完（这条判断在 `releaseLineText` 里，一处出）。
 */
function ReleaseLineRow({ tr, text }: { tr: Tr; text: string | null }) {
  if (text == null) return null
  return (
    <div className={s.dRow}>
      <span className={s.dKey}>{tr(RELEASE_LINE_KEY)}</span>
      <span className={`${s.dVal} ${s.dOne}`} title={text}>
        {text}
      </span>
    </div>
  )
}

function CloudLocalAction({
  row,
  busy,
  onDownload,
}: {
  row: PresetCloudRow
  busy: boolean
  onDownload: (row: PresetTableRow) => void
}) {
  const tr = useT()
  /*
   * ★ **这一格是「云端行动作」的唯一出口**，所以「云端较旧」的闸必须在这一行的最前面 ——
   *   2026-10-27 作者截图：名称列已经什么都不显示了，右边却还挂着一颗绿色「更新」。
   *
   *   原来这个组件只认 `row.localState`（`missing` / `stale` / `has`），完全不看
   *   `row.basedOn`；主链上那句 `needsUpdate && !cloudIsOlder` 守的是**另一条**渲染
   *   路径，够不到这里。于是"云端比我这份旧"的行照样长出一颗更新按钮。
   *
   *   判据只有一条：结论是 `newer`（云端较旧）⇒ 这一格**什么都不给**，画破折号。
   *   想知道为什么，展开详情里有「云端较旧」和那句"要云端版就先删掉本地再下载"。
   *   ★ 以后再加动作，请统一从 `basedOn` 起手，别再只看 `localState`。
   */
  if (row.basedOn === 'newer') {
    return (
      <span className={s.actNone} title={tr('presets.table.staleOlder.title')}>
        {DASH_}
      </span>
    )
  }
  /*
   * ★ **只有 `outdated` 才给「更新」主按钮**（2026-10-28 三问题模型）。
   *
   *   以前这里只看 `localState === 'stale'`，而 `stale` 是"本地那份和目录字节不同"——
   *   它同时覆盖了「真有新版」「说不清」「目录自相矛盾」三种完全不同的情形，
   *   于是三种都长出了同一颗绿色按钮。
   *
   *   现在按结论分派：
   *     outdated     ⇒ 给（唯一一档是"确定有新版"）
   *     unchanged    ⇒ 破折号（发布前进了，但这份没变）
   *     inconsistent ⇒ 破折号 + 警告字样（数据坏了，不能覆盖）
   *     unverified   ⇒ **也给**，但换了说法（见下）
   *
   *   ★ `unverified` 必须留一条出路（2026-10-29）：老副本（血统里没有发布线/序号）
   *     会被判成"说不清"，如果连按钮都不给，那一份就**永远**没有建立新基线的入口 ——
   *     只能删掉重下。而"接受当前云端版"这件事本来就得由用户明确点一下才算数
   *     （下载 ≠ 接受版本），所以按钮留着，`title` 里把话说清：点它 = 以当前云端版
   *     建立起新基线。至于"有新版"那三个字，那是徽章的事，已经不在这格说了。
   */
  if (row.basedOn === 'unchanged' || row.basedOn === 'inconsistent') {
    const why =
      row.basedOn === 'inconsistent'
        ? tr('presets.table.inconsistent.title')
        : tr('presets.table.unchanged.title')
    return (
      <span
        className={row.basedOn === 'inconsistent' ? s.actWarn : s.actNone}
        title={why}
      >
        {row.basedOn === 'inconsistent' ? tr('presets.table.inconsistent.badge') : DASH_}
      </span>
    )
  }
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
  /*
   * **「下载」与「更新」要分得开**（2026-10-20 作者：「我希望区分一下下载和更新啊，
   * 更新的话，就跟那本地的那个快捷操作里面的一样，用绿色背景吧」）。
   *
   * 判据是**这一份是不是"第一次取回来"**：`localState === 'missing'` 才是「下载」
   * （白底、与原来一样）；「有新版」那一档是「更新」—— 绿底，与本地行
   * 「快捷操作」里那颗绿色「更新」同一副样子（同一个 `.updateBtn`）。
   */
  const isUpdate = state === 'stale'
  return (
    <button
      type="button"
      className={isUpdate ? s.updateBtn : s.actBtn}
      disabled={busy}
      onClick={() => onDownload(row)}
      /* 说不清版本关系时，这一下点击的含义要多说一句（见上面那段注释） */
      title={
        row.basedOn === 'unverified'
          ? tr('presets.table.unverified.updateTitle')
          : undefined
      }
    >
      {tr(action)}
    </button>
  )
}

/**
 * **一枚能点着排序的表头**（2026-10-16 作者：「表头加上排序吧……只用很小的图标，不占位置的」）。
 *
 * # "不占位置"是怎么做到的
 *
 * `table-layout: fixed`，列宽由那几条 `clamp()` 定死、内容不参与算宽 ⇒
 * 箭头内联跟在文字后面也**不撑宽任何一列**，点一下排序不会让列变宽或变窄。
 *
 * # 什么时候显示
 *
 * - **正在排的那一列**：箭头常驻、颜色是 `--text-2`（看得见"现在是按它排的"）；
 * - **其余列**：默认不画，鼠标移到这一格才淡淡地露出来（提示"这列也能排"）。
 *   ⇒ 六列同时挂箭头会很吵，也把"当前按哪列排"这件事讲没了。
 *
 * ★ 箭头**内联跟在文字后面**（2026-10-16 作者：「紧跟着文字」）。`table-layout: fixed`
 *   下列宽由 th 的 clamp() 定死，内容不参与算宽 ⇒ 进流也不撑列，"不占位置"仍成立。
 *   布局走普通流 + text-align（不走 flex —— th 的 justify-content 是 normal，
 *   flex 会把居中列的表头顶到左边）。降序时 CSS 把箭头转 180°，
 *   不为"下"再造一枚图标：它是同一颗箭头换个向，不是两个相反的语义
 *   （`pin` / `pinOff` 那种才需要两枚）。
 */
function SortTh({
  label,
  colKey,
  cls,
  sort,
  onSort,
}: {
  label: string
  colKey: PresetSortKey
  cls: string
  sort: PresetSort
  onSort: (key: PresetSortKey) => void
}) {
  const on = sort.key === colKey
  const tr = useT()
  /*
   * 名称列**三循环**（2026-10-20），别的列两态 —— 提示要说清"点一下会变成什么"：
   * 默认档（族优先）没有方向，那一档不画箭头（CSS 按 `data-dir` 收起来）。
   */
  const next =
    !on
      ? tr('presets.table.sort.by', { label })
      : colKey === 'name'
        ? sort.dir === 'family'
          ? tr('presets.table.sort.az', { label })
          : sort.dir === 'asc'
            ? tr('presets.table.sort.desc', { label })
            : tr('presets.table.sort.default')
        : sort.dir === 'asc'
          ? tr('presets.table.sort.descCol', { label })
          : tr('presets.table.sort.asc', { label })
  return (
    <th
      className={cls}
      scope="col"
      aria-sort={
        !on
          ? 'none'
          : sort.dir === 'asc'
            ? 'ascending'
            : sort.dir === 'desc'
              ? 'descending'
              : 'other'
      }
    >
      <button
        type="button"
        className={s.sortBtn}
        data-on={on ? '1' : '0'}
        /* 降序时 CSS 把那颗箭头转 180°；默认（族优先）那一档没有方向，不画箭头 */
        data-dir={on ? sort.dir : undefined}
        onClick={() => onSort(colKey)}
        title={next}
      >
        <span className={s.sortText}>{label}</span>
        {/* 常驻渲染、由 CSS 管显隐 —— 非当前列 hover 才淡淡露出（见上面那段） */}
        <Icon name="caret" className={s.caret} />
      </button>
    </th>
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
   * 当前数据源（`gitee` / `github` / `custom`）—— preset 行「来源」列显示
   * GitHub / Gitee / 自定义源 的依据。null = 没配源，来源退「云端」。
   */
  sourceLabel: string | null
  /** 表头那一列怎么排（页面层持有；落会话，切页回来不复位） */
  sort: PresetSort
  /** 点表头。同一列升 / 降互换，换列从升序起（时间列除外，见 `usePresetData`） */
  onSort: (key: PresetSortKey) => void
  /**
   * 这一份在归档区里有几个**旧版本**（按文件名对）。
   * `0` = 没有 → 展开详情里那一格**不显示**（没有 ≠ 未知，不必占一个"—"）
   */
  archiveCountOf: (fileName: string) => number
  /** 打开「旧版本」抽屉（归档的可视化在页面层的抽屉里，表这边只给入口） */
  onOpenArchive: (row: PresetTableRow) => void
  /**
   * 打开「本地这一份」抽屉（本地文件的正文：**只读**）。
   * ★ 2026-10-16 起只有**切片器档**的本地行用它 —— MKP 行的查看并进了「编辑」
   * 模态框的只读态（作者：「不需要什么看正文了，你就留一个编辑」）。
   */
  onOpenLocal: (row: PresetTableRow) => void
  /**
   * 开始改这一份（临时编辑那条链）。两边共一个入口：
   * 交付行只给"与目录一致"的那一份；我自己那份都能改（认不出是哪一类的不给）。
   */
  /**
   * **展开详情「快捷操作」里的「修改参数」**（2026-10-20 作者：「我希望是那个修改参数作为
   * 快捷操作……打开的是修改参数的模态框」）。
   *
   * 打开的是**参数表那一屏**（页面层 `openParams(row, false)`），与右键菜单里
   * 「修改参数」同一个动作 —— 「编辑」（改 TOML 正文）仍留在右键菜单里。
   */
  onParams: (row: PresetTableRow) => void
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
   * **本地行那颗「更新」**（2026-10-20 作者：「这个下面应该加一个更新按钮，就是说有新的时候
   * 就出现一个更新按钮。我点了之后，它就跟云端一样更新了，就不用跳转到云端那边去更新」）。
   *
   * 走的是**与云端行同一个入口**（`PagePresets.runUpdate`）—— 只是行从本地那一份来：
   * 「有新版」时给，点了就地消化（0 项直接落盘刷血统、有改动项顺势开「选择要更新的参数」）。
   */
  onUpdateLocal: (row: PresetLocalRow) => void
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
  /**
   * 本机学习账（`run/usage.json`）：钥匙 → 后处理**成功**跑过的次数。
   *
   * 详情面板那一格「后处理 N 次」读它。空 Map = 一次都没跑过（换台设备从零学），
   * 不是错误 —— 所以那一格写「还没跑过」，不写「—」也不报错。
   * 钥匙在行上（`row.usageKey`），表这边不拿 `path` 现拼（口径两张表不同）。
   */
  usage: Map<string, number>
  /**
   * 设置页那一格「复制出来的也跟着云端更新」（2026-10-20，出厂 = 关）。
   *
   * **关**时复制 / 导入出来的那几份是**独立的一份**：它们不认云端 —— 云端换版时
   * 本地行不许挂「有新版」（作者：「我设置里面没开，它就不应该把那些复制出来的也显示，
   * 它就只显示那一个」）。**开**时与原件同权，照挂。
   *
   * 判据只有这一格（后端 `presets.copiesFollowCloud`）：这里不按名字、不按血统自己猜。
   */
  copiesFollow: boolean
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
  sort,
  onSort,
  archiveCountOf,
  onOpenArchive,
  onOpenLocal,
  onParams,
  onLive,
  onUse,
  onDownload,
  onUpdateLocal,
  machines,
  remarks,
  onSetRemark,
  onSetAttribution,
  usage,
  copiesFollow,
}: Props) {
  /* 这一个布尔决定表头有几列、每行画哪几格。两处都读它，不许各判一次 */
  const mkp = kind === 'mkp'
  const tr = useT()

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
   * **收起没有动画**（2026-10-17 第七轮，作者：「旧的没有收起动画」「我希望是保持高度
   * 不变……而且只是收起的话，这个动画也不好，先快后慢，最后的慢有点不舒服」）。
   *
   * 两条事实叠在一起，才得出这个结论：
   *   · 行高**本来就是瞬间到位**的（2026-10-16 第四轮：`<table>` 里动行高 = 每帧整表重排，
   *     见 CSS 那段），所以"收起"能播的动画只剩内容层的淡出 —— 但内容还占着那一整行高度，
   *     看到的顺序是「先淡一点 → 停一下 → 整行突然消失」，作者说的"最后的慢"就是这一段。
   *   · 换行时旧那张卡还留在 DOM 里播淡出 ⇒ 那一帧**两张卡同时开着**（作者连拍截到的就是
   *     它），表也跟着高出一整张。
   *
   * 所以收起一律**瞬时**：`onToggleExpand` 直接换键，旧行同帧卸载 —— 任何时刻最多一张卡，
   * 表高只换一次。要动的只剩入场那一层（CSS `.dFoldIn` 的 opacity / transform）。
   * 原来那套 `closing / rush / queuedRef`（收起动画 + 点击排队 + 加速收尾）整段退役 ——
   * 没有收起动画，也就没有"收一半又被点开"要排队。
   */

  /** 空表三种话，说的是三件不同的事 —— 混成一句「没有数据」就查不出是哪一件 */
  const emptyText = (): string => {
    if (failure !== null) return tr('presets.table.empty.failed', { detail: failure })
    if (total === 0) {
      /* MKP 那一档是一对表（本地 / 云端），空表按表说 —— 本地是你自己那份，云端是云端 */
      if (mkp) {
        return scope === 'local'
          ? tr('presets.table.empty.mkpLocal')
          : tr('presets.table.empty.mkpCloud')
      }
      return scope === 'local'
        ? tr('presets.table.empty.slicerLocal', { kind: tr(KIND_AXIS_TEXT[kind]) })
        : tr('presets.table.empty.slicerCloud', { kind: tr(KIND_AXIS_TEXT[kind]) })
    }
    return tr('presets.table.empty.filtered', { total, query: query.trim() })
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
              {/*
               * **每一列都能点着排序**（2026-10-16 作者：「表头加上排序吧……名称排序，
               * 机型排序，所有的都排序，时间也排序」）。
               *
               * ★「操作」那一列**不排** —— 它不是数据列，排它没有意义。
               * ★ 置顶**不参与**：置顶的那几份永远压在最上面（那是另一件事的判据），
               *   列排序只决定"同一堆里谁在前"。
               */}
              <SortTh label={tr('presets.table.col.name')} colKey="name" cls={s.thName} sort={sort} onSort={onSort} />
              <SortTh label={tr('presets.table.col.machine')} colKey="machine" cls={s.thMachine} sort={sort} onSort={onSort} />

              {/*
               * 这里就是「列数真的跟着换」那一下：MKP 只有版本，切片器只有喷嘴 + 层高。
               * 另一种类型的那几列**整个不渲染**，不是渲染成空的 —— 空格子照样占宽度。
               */}
              {mkp ? (
                <SortTh label={tr('presets.table.col.version')} colKey="version" cls={s.thVersion} sort={sort} onSort={onSort} />
              ) : (
                <>
                  <SortTh label={tr('presets.table.col.nozzle')} colKey="nozzle" cls={s.thNum} sort={sort} onSort={onSort} />
                  <SortTh label={tr('presets.table.col.layer')} colKey="layer" cls={s.thNum} sort={sort} onSort={onSort} />
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
              <SortTh
                label={scope === 'cloud' ? tr('presets.table.col.cloudTime') : tr('presets.table.col.time')}
                colKey="time"
                cls={s.thTime}
                sort={sort}
                onSort={onSort}
              />

              {/*
               * 「来源」列撤了（2026-10-07 作者：「来源不用显示在右侧」）——
               * 它还在展开详情的「来源」格里（那里还能点「复制自 X」定位）。
               */}
              <th className={s.thAct} scope="col">
                {tr('presets.table.col.act')}
              </th>
            </tr>
          </thead>

          <tbody>
            {rows.map((row) => {
              const version = versionsText(tr, row.versions)
              /*
               * 机型识别色（2026-10-08 起、a03「色彩收敛」收掉）：`A1` 与 `A1 mini` 只差
               * 四个字母，作者曾在真机上把 A1 那行看成 A1 mini、点了下载才发现 —— 那一版
               * 给每个机型配了色、按 `data-machine` 上色。**色彩收敛后那套色表整链已删**
               * （CSS 无 `[data-machine]` 选择器、探针也不读），机型列现在只当文字看。
               */
              /*
               * 行左边那道绿竖线只标**「正在生效的那一份」**：本地表是 live（MKP 已应用 /
               * 切片器已复制），云端表是 applied（仓库里那一份就是你在用的那一份）。
               * 云端的「已下载」不画线 —— 下载不等于生效，九行里有八行带绿线那道线就没意思了。
               */
              const inUse = row.scope === 'local' ? row.live : row.applied
              const busy = row.rowKey === busyKey
              /*
               * preset 行的**云端 vs 盘上**三态（未下载 / 已下载 / 有更新）——
               * 状态主词与按钮都从它出（见 `updateStateOf`）。以前"盘上字节认不出"那一档
               * 把「内容异常」顶在最前面、按钮叫「重新下载」：用户被吓着，而云端明明只是
               * 有更新。现在主词说「有更新」，认不出的实情在展开详情里有一行注记说全。
               */
              const updateState =
                row.presetState !== undefined ? updateStateOf(row.presetState) : undefined
              /* 云端较旧 ⇒ 一律不给更新（列表上也不显示任何徽章，详情里才说） */
              const cloudIsOlder = row.scope === 'cloud' && row.basedOn === 'newer'
              const needsUpdate = updateState === 'update' && !cloudIsOlder
              /* 认不出是哪一版的注记只在展开详情里出现（tampered 那一档）——
                 **MKP 档不出现**：那一档按 `localState` 说，不碰下载区四态（2026-10-09） */
              const untrusted =
                row.presetState === 'tampered' &&
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
                (row.presetState === undefined ? row.downloaded : row.presetState === 'ok')
              /* 来源那一格（列 + 展开详情共用）：三种来源三种说法，可定位的带 `locate` */
              const origin = originCellOf(tr, row, sourceLabel)
              /*
               * 缩窄到手：`origin.locate` 的判空进不了 JSX 回调（TS 对属性访问的收窄
               * 不跨函数边界），先落一个本地的量。
               */
              const locateTarget = origin.locate ?? null
              /*
               * **这一行是"跟云端没关系的一份"**（2026-10-20）：复制 / 导入出来的，
               * 而设置页「复制出来的也跟着云端更新」关着 ⇒ 云端换版不该在它身上报「有新版」。
               * 判据只看后端那一格 + 出处账给出的 `provenance`（不按名字猜）。
               */
              const detached = isDetachedCopy(row, copiesFollow)
              /* 与参数页同一套交互：点行展开下方的内容，同一时刻只开一行 */
              const expanded = row.rowKey === expandedKey
              /*
               * 「这一行还热着」= 就它展开着。收起没有动画（见上面那段），
               * 所以这里没有"正在收"这第三态 —— 行白底与卡片同进同出。
               */
              const rowHot = expanded
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
                    onToggleExpand(row.rowKey)
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
                          aria-label={tr('presets.table.unsaved.aria')}
                          title={tr('presets.table.unsaved.title')}
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
                          {tr('presets.table.olderVersions', { count: row.olderVersions ?? 0 })}
                        </span>
                      )}
                      {/*
                        **本地行也有「有新版」**（2026-10-20 作者：「这种有新版的在本地也需要出现呀」）。
                        判据就是这一份的血统（`basedOn === 'outdated'` = 它基于的那一版云端已经换代）——
                        与云端行那枚徽章同一句话、同一个样式，紧跟文件名。

                        ★ **复制 / 导入出来的那几份不挂这一枚**（作者当天第二张图：
                          「我设置里面没开，它就不应该把那些复制出来的也显示……它就只显示那一个」）。
                          设置页「复制出来的也跟着云端更新」关着时，复制件是**独立的一份**：
                          它跟云端没关系，云端换版也不该在它身上报更新 —— 判据走后端那一格
                          （`copiesFollow === false` 且 `provenance` 是 copy / import），别处不许再写第二份。
                      */}
                      {row.scope === 'local' && row.basedOn === 'outdated' && !detached && (
                        <span
                          className={s.localState}
                          data-state="stale"
                          title={tr('presets.table.stale.title')}
                        >
                          {tr('presets.table.stale')}
                        </span>
                      )}
                      {/*
                        **「云端较旧」在列表上不挂徽章**（2026-10-27 作者：「本地也不用显示了吧」）。
                        这条结论只在两处出现：云端行展开详情头部、本地行详情「基于」那一格。
                        列表上什么都不说 —— 没有新版就是没有新版，不拿"云端比我旧"占位置。
                      */}
                      {/*
                        **「另一个源」这枚标签撤了**（2026-10-27 作者：「不是同一个源也不要
                        显示另一个源吧，主要是不要显示更新呀」）：源不同本身不是用户要看的
                        信息 —— 内容不同、且**云端更旧**才挂「云端较旧」，其余情况什么都不挂、
                        也不给「更新」。判定只看内容与时间（见 `runtime::local::based_on`）。
                      */}
                      {/*
                        **我这边怎么样**（MKP 云端行，2026-10-09）：未获取 / 本地已有 / 有新版。
                        这一格回答的是用户点开"云端"时问的那件事 ——
                        "云端有这些，那我手上有没有、要不要拿"。判据在 `cloudRows` 里。
                      */}
                      {row.scope === 'cloud' &&
                        (() => {
                          const badge = cloudClaimBadge(row)
                          return badge === null ? null : (
                            <span className={s.localState} data-state={badge.state}>
                              {tr(badge.text)}
                            </span>
                          )
                        })()}
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
                       * ★ **"基于旧版官方"（现称交付预设）那枚徽章退役了**（2026-10-08 作者裁决）：
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
                      <span className={s.machineChip}>{row.machineText}</span>
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
                       *
                       * ★★ **这一格只有「使用 / 在用」两态，永远不换词**（2026-10-20 作者：
                       *   「他云端有更新的时候，只在那个快捷操作里面显示更新，右侧的那个按钮
                       *   始终都是显示使用……本地的按钮就只有使用和在用两种状态，
                       *   没有下载、更新那些。那些是云端的按钮」）。
                       *   「有新版」是**展开详情「快捷操作」**里那颗绿色「更新」的事，
                       *   这里不许跟着变 —— 本地行与云端行复用的是同一套组件，
                       *   但两边的按钮语义必须分得干净：下载 / 更新是云端那一侧的词。
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
                          {tr(ASSET_USE_TEXT.on)}
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
                          {tr(ASSET_USE_TEXT.off)}
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
                         *
                         * ★ 这一档是**云端按钮**（切片器交付行的本地那份与目录不符），
                         *   所以绿底「更新」—— 与「下载」分得开（2026-10-20）。
                         */
                        <button
                          type="button"
                          className={s.updateBtn}
                          disabled={busy}
                          onClick={() => onDownload(row)}
                        >
                          {tr(UPDATE_ACTION_TEXT.update)}
                        </button>
                      ) : row.live ? (
                        <span className={s.actDone}>
                          {tr(LIVE_TEXT[kind].on)}
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
                            {tr(ACTION_TEXT[kind])}
                          </button>
                        )
                      ) : row.presetUid !== undefined && row.kind !== 'mkp_preset' ? (
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
                          {tr(ACTION_TEXT[kind])}
                        </button>
                      ) : row.presetUid === undefined && row.kind === 'mkp_preset' ? (
                        /*
                         * **云端 MKP 行没有交付身份（presetUid）就没有「应用」**（A2 修缝）——
                         * 应用只认目录登记的那一份。两种真实状态共用：用户自己放进
                         * `mkp/` 的来路不明文件；以及旧资产库形状的行（演示数据里有）。
                         * 原来这里放行到「应用」按钮、点了再被守卫拦下，报一串
                         * asset id / 契约的术语 —— 现在按钮根本不给，原因用人话说。
                         */
                        <span className={s.actNone}>
                          {DASH_}
                        </span>
                      ) : row.assetId === undefined && row.presetUid === undefined ? (
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
                          {tr(ACTION_TEXT[kind])}
                        </button>
                      )
                    ) : gotIt ? (
                      <span className={s.actDone}>
                        {tr(CLOUD_STATE_TEXT.downloaded)}
                      </span>
                    ) : row.basedOn === 'newer' ? (
                      /* 云端比我这份还旧 ⇒ 这一格**不给动作**（「云端旧的就不用显示更新」） */
                      <span className={s.actNone}>{DASH_}</span>
                    ) : row.presetUid !== undefined ? (
                      /*
                       * 云端交付行：**未获取 →「下载」白底**，**有更新 →「更新」绿底**
                       * （2026-10-20 作者：「区分一下下载和更新啊……更新的话，
                       * 跟本地那个快捷操作里面的一样，用绿色背景」）。
                       */
                      <button
                        type="button"
                        className={needsUpdate ? s.updateBtn : s.actBtn}
                        disabled={busy}
                        onClick={() => onDownload(row)}
                      >
                        {needsUpdate
                          ? tr(UPDATE_ACTION_TEXT.update)
                          : tr(UPDATE_ACTION_TEXT.missing)}
                      </button>
                    ) : (
                      /*
                       * **云端仓库文件不给「下载」**（2026-10-09 用户裁断：残支连入口一起撤）——
                       * 下载只走两条真管道：MKP 的「取回」与登记过的交付下载。
                       */
                      <span className={s.actNone}>
                        {DASH_}
                      </span>
                    )}
                  </td>
                </tr>

                {rowHot && (
                  <tr className={s.expandRow}>
                    {/*
                     * colSpan 必须**跟着上面的表头列数走**：MKP 5 列（名称 / 机型 / 版本 / 时间 /
                     * 操作）、切片器 6 列（名称 / 机型 / 喷嘴 / 层高 / 时间 / 操作）。
                     * 比表头多出的那一跨会撑出一个匿名列，吃掉表格右侧的全部余量 ——
                     * 改列数时这里要一起改。
                     */}
                    <td colSpan={mkp ? 5 : 6}>
                      {/*
                       * —— 展开的小动画（2026-10-16 第四轮：行高不再动）——
                       * 三段内衬：`.dFold`（撑开格子）→ `.dFoldCut`（裁）→ `.dFoldIn`
                       * （内容落下 + 渐显 + 内边距 —— 卡片影子要的余地也在这里，
                       * 贴着裁剪边影子会被切）。
                       * 动画只跑在 `.dFoldIn` 上（opacity / transform，走合成器）：
                       * 在 `<table>` 里动行高 = 每帧整表重排，那是不顺的根源。
                       * **退场没有动画**（2026-10-17 第七轮）：这一行卸载就是收起，
                       * 同帧换到新那一行，任何时刻最多一张卡（见上面那段说明）。
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
                              {row.scope === 'cloud' && row.basedOn === 'newer'
                                ? tr('presets.table.staleOlder')
                                : row.scope === 'cloud' && row.localState !== undefined
                                  ? tr(CLOUD_LOCAL_TEXT[row.localState])
                                : row.presetState !== undefined
                                  ? tr(UPDATE_STATE_TEXT[updateState ?? 'latest'])
                                  : mkp && row.scope === 'local'
                                    ? tr(ASSET_STATE_TEXT[row.live ? 'on' : 'off'])
                                    : row.scope === 'local'
                                      ? tr(LIVE_TEXT[kind][row.live ? 'on' : 'off'])
                                      : row.downloaded
                                        ? tr(CLOUD_STATE_TEXT.downloaded)
                                        : tr(CLOUD_STATE_TEXT.pending)}
                              {/*
                               * 「正在用」只在状态主词没说这件事时补 —— 主词已经写着
                               * 「在用 / 已复制」的（本地行 live 那一档）不再追加，
                               * 否则就是「在用 · 正在用 · 2.0 KB」这种同一句话说两遍
                               * （作者截图点名）；云端行的主词说的是"我这边取到没有"，
                               * 与「正在用」是两件事，照旧都要说。
                               */}
                              {row.applied && row.scope !== 'local' && tr('presets.table.inUse')}
                            </span>
                            <i className={s.dDot} aria-hidden>·</i>
                            <span>{row.sizeText ?? tr(UNKNOWN)}</span>
                            <i className={s.dDot} aria-hidden>·</i>
                            <span>
                              {row.origin === 'preset'
                                ? row.scope === 'cloud'
                                  ? tr('presets.table.when.cloud')
                                  : row.arrivalBy === 'replaced' ? tr('presets.table.when.replaced') : tr('presets.table.when.downloaded')
                                : tr('presets.table.when.modified')}
                              {longStatText(row.modifiedText) ?? tr(UNKNOWN)}
                            </span>
                          </span>
                          {/*
                            收起：与点这一行（再点一次）同一件事 —— 安静的小按钮，不抢摘要。
                            收起没有动画（见上面那段），点它就是这一行消失。
                          */}
                          <button
                            type="button"
                            className={s.dHeadFold}
                            title={tr('presets.table.collapse.title')}
                            onClick={() => onToggleExpand(row.rowKey)}
                          >
                            {tr('presets.table.collapse')}
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
                            <span className={s.dColLabel}>{tr('presets.table.detail.basic')}</span>
                            <div className={s.dGroup}>
                              {mkp ? (
                                row.versions.length > 0 && (
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>{tr('presets.table.detail.key.version')}</span>
                                    <span className={s.dVal}>{version.text}</span>
                                  </div>
                                )
                              ) : (
                                <>
                                  {row.nozzle != null && (
                                    <div className={s.dRow}>
                                      <span className={s.dKey}>{tr('presets.table.detail.key.nozzle')}</span>
                                      <span className={s.dVal}>{row.nozzle}</span>
                                    </div>
                                  )}
                                  {row.layerHeight != null && (
                                    <div className={s.dRow}>
                                      <span className={s.dKey}>{tr('presets.table.detail.key.layer')}</span>
                                      <span className={s.dVal}>{row.layerHeight}</span>
                                    </div>
                                  )}
                                </>
                              )}

                              <div className={s.dRow}>
                                {/*
                                  **云端行那一格叫「源配置」，不叫「来源」**（2026-10-27 作者：
                                  「来源和云端源感觉有点歧义和重复」）：它的值是"自定义源 / GitHub /
                                  Gitee"这种**模式名**，而下面「云端源」是规范化身份
                                  （`gitee#owner/repo@ref`）—— 两个都叫"来源"就分不清谁是谁。
                                  本地行那一格的值是"复制自 X"，仍叫「来源」。
                                */}
                                <span className={s.dKey}>
                                  {tr(
                                    row.origin === 'preset'
                                      ? 'presets.table.detail.key.sourceMode'
                                      : 'presets.table.detail.key.source',
                                  )}
                                </span>
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
                                  <span className={s.dKey}>{tr(BASED_ON_KEY)}</span>
                                  <span className={s.dVal}>{basedOnCellText(tr, row)}</span>
                                </div>
                              )}

                              {/*
                                **版本来源**（2026-10-27 作者：「展开的详细里面显示哪些源的，
                                本地源什么，云端源什么，不同的时间之类的」）。

                                四行都**照实说**：认不出就写「未知」，不拿当前源去补血统源 ——
                                「不知道它来自哪」本身就是用户需要知道的那句话（换源误判的根源）。
                                数据来自后端 `basedOnFacts`，界面不重算。
                              */}
                              {row.origin === 'local' && row.basedOnFacts !== undefined && (
                                <>
                                  {/*
                                    **发布线**（2026-10-28）：我这份基于哪条线的第几号。
                                    两格同生同死，缺一个就整行不渲染（见 `releaseLineText`）。
                                  */}
                                  <ReleaseLineRow
                                    tr={tr}
                                    text={releaseLineText(
                                      tr,
                                      row.basedOnFacts.lineageCatalog,
                                      row.basedOnFacts.lineageSerial,
                                    )}
                                  />
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.local')}
                                    </span>
                                    <span
                                      className={`${s.dVal} ${s.dOne}`}
                                      title={row.basedOnFacts.lineageSource ?? undefined}
                                    >
                                      {row.basedOnFacts.lineageSource ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.localTime')}
                                    </span>
                                    <span className={s.dVal}>
                                      {longStatText(
                                        row.basedOnFacts.lineagePublishedAt ??
                                          row.basedOnFacts.lineageRelease ??
                                          undefined,
                                      ) ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.cloud')}
                                    </span>
                                    <span
                                      className={`${s.dVal} ${s.dOne}`}
                                      title={row.basedOnFacts.cloudSource ?? undefined}
                                    >
                                      {row.basedOnFacts.cloudSource ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.cloudTime')}
                                    </span>
                                    <span className={s.dVal}>
                                      {longStatText(
                                        row.basedOnFacts.cloudPublishedAt ?? undefined,
                                      ) ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                </>
                              )}

                              {/*
                                云端行也摆这份账（2026-10-27 作者：「云端的更需要看那些信息呀」）：
                                这个云端是哪个源、这个目录什么时候发布、**我本地那一份是什么时候的**
                                —— 三行并排就是一次对比。判定不读它们，纯展示。
                              */}
                              {row.origin === 'preset' && row.basedOnFacts !== undefined && (
                                <>
                                  {/*
                                    **发布线**（2026-10-28）：云端这份目录是第几号。
                                    与本地那一行同键同档 —— 两行并排就能看出差了几代。
                                  */}
                                  <ReleaseLineRow
                                    tr={tr}
                                    text={releaseLineText(
                                      tr,
                                      row.basedOnFacts.cloudCatalog,
                                      row.basedOnFacts.cloudSerial,
                                    )}
                                  />
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.cloud')}
                                    </span>
                                    <span
                                      className={`${s.dVal} ${s.dOne}`}
                                      title={row.basedOnFacts.cloudSource ?? undefined}
                                    >
                                      {row.basedOnFacts.cloudSource ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.cloudTime')}
                                    </span>
                                    <span className={s.dVal}>
                                      {longStatText(
                                        row.basedOnFacts.cloudPublishedAt ?? undefined,
                                      ) ?? tr(UNKNOWN)}
                                    </span>
                                  </div>
                                  {/*
                                    **本地版本时间**（作者：「云端应该也能把本地的那个显示一下
                                    吧，对比的感觉」）：两行并排就能一眼看出我这份比云端新还是旧；
                                    没下过就照实说「还没下载」，不编。
                                  */}
                                  <div className={s.dRow}>
                                    <span className={s.dKey}>
                                      {tr('presets.table.detail.ver.localTime')}
                                    </span>
                                    <span className={s.dVal}>
                                      {row.localReleaseText ?? tr('presets.table.detail.ver.notDownloaded')}
                                    </span>
                                  </div>
                                </>
                              )}

                              {/*
                                云端较旧的说明（2026-10-27 作者：「详细里面能看到，但是不显示更新，
                                提示可以删除本地再下载」）—— 列表上什么都不显示，这里把话说全。
                              */}
                              {row.origin === 'preset' && row.basedOn === 'newer' && (
                                <p className={s.dNote}>{tr('presets.table.detail.olderHint')}</p>
                              )}

                              {/*
                                后处理跑成过几次（2026-10-17 作者：「加一个显示的字段，
                                后处理使用次数……在展开的详细里面吧」）。

                                数据是本机学习账（`run/usage.json`），不在任何契约里；
                                钥匙在行上（`usageKey`，见 `PresetRowBase`）。只有 MKP 预设
                                才有这一格 —— 切片器配置不跑后处理，那几行连钥匙都没有。
                                0 次照常显示「还没跑过」：这是事实，不是缺数据。

                                ★ **云端行不显示**（2026-10-27 作者：「云端的不用显示后处理次数」）：
                                  那本账记的是**本机跑过几次**，而云端那一行是"云上这一版"，
                                  两者不是同一个东西 —— 在云端行写「还没跑过」会让人以为
                                  云上这一版从没被跑过，其实只是我这台没跑过它。
                              */}
                              {row.scope === 'local' && row.usageKey !== undefined && (
                                <div className={s.dRow} title={tr(USAGE_HINT)}>
                                  <span className={s.dKey}>{tr(USAGE_KEY)}</span>
                                  <span className={s.dVal}>
                                    {usageCellText(tr, usage.get(row.usageKey))}
                                  </span>
                                </div>
                              )}
                            </div>
                          </div>

                          {/* 右栏：快捷操作 —— 原「卡底」那三颗按钮搬上来（2026-10-10） */}
                          {(row.origin === 'local' || archiveCount > 0) && (
                            <div className={s.dCol}>
                              <span className={s.dColLabel}>{tr('presets.table.detail.quick')}</span>
                              <div className={s.dActs}>
                                {archiveCount > 0 && (
                                  <button
                                    type="button"
                                    className={s.ghostBtn}
                                    onClick={() => onOpenArchive(row)}
                                  >
                                    {archiveOpenText(tr, archiveCount)}
                                  </button>
                                )}
                                {/*
                                 * ★ 2026-10-16（作者：「不需要两个了，不需要什么看正文了，
                                 *   你就留一个编辑的可能就足够了」）：
                                 *   **MKP 行不看正文了** —— 「编辑」模态框打开就是只读的整份正文
                                 *   （看和改是同一条路，进编辑模式才动得了）；原来两颗并排、
                                 *   功能大半重叠。
                                 *   切片器档的本地行**留着**：那条链没有「编辑」（编辑文件只认
                                 *   MKP 预设），「看正文」是它唯一的查看入口。
                                 */}
                                {row.origin === 'local' && row.kind !== 'mkp_preset' && (
                                  <button
                                    type="button"
                                    className={s.ghostBtn}
                                    onClick={() => onOpenLocal(row)}
                                  >
                                    {tr(LOCAL_DRAWER.open)}
                                  </button>
                                )}
                                {/*
                                  * **更新**（2026-10-20 作者：「他云端有更新的时候，只在那个
                                  * 快捷操作里面显示更新」「那里面的下载两个字改成更新两个字，
                                  * 并且加上背景颜色吧，绿色吧」）。
                                  *
                                  * 所以：**只有这里**因为「有新版」长出一颗「更新」，
                                  * 而且是绿底（`.updateBtn`）—— 与旁边安静的「编辑」一眼分得开；
                                  * 表格右侧那一格**照旧只写「使用 / 在用」**，不许跟着变。
                                  */}
                                {row.origin === 'local' &&
                                  row.basedOn === 'outdated' &&
                                  !detached &&
                                  row.localState !== 'unreadable' && (
                                    <button
                                      type="button"
                                      className={s.updateBtn}
                                      disabled={busy}
                                      title={tr('presets.table.detail.update.title')}
                                      onClick={() => onUpdateLocal(row)}
                                    >
                                      {tr(UPDATE_ACTION_TEXT.update)}
                                    </button>
                                  )}
                                {/*
                                  * **「快捷操作」里那颗从「编辑」换成「修改参数」**（2026-10-20 作者：
                                  * 「我不希望它是编辑作为快捷操作，我希望是那个修改参数作为快捷操作。
                                  * 你要把它改成修改的文案，然后打开的是修改参数的模态框」）。
                                  *
                                  * 所以这一颗打开的是**参数表那一屏**（`onParams` → 页面层
                                  * `openParams(row, false)`），不是改正文的「编辑文件」——
                                  * 日常要动的基本都是参数，正文那一档仍在右键菜单里（`EDIT_TEXT.cell`）。
                                  * 词与菜单里那一项同一个出处（`PARAMS_TEXT.open`），不各写一个。
                                  */}
                                {row.origin === 'local' &&
                                  row.kind === 'mkp_preset' &&
                                  row.localState !== 'unreadable' && (
                                    <button
                                      type="button"
                                      className={s.ghostBtn}
                                      title={tr(PARAMS_TEXT.why)}
                                      onClick={() => onParams(row)}
                                    >
                                      {tr(PARAMS_TEXT.open)}
                                    </button>
                                  )}
                              </div>
                            </div>
                          )}
                        </div>

                        {/* preset 行的版本指纹（云端最新版本/发布时间），第二行 muted meta */}
                        {row.origin === 'preset' && (
                          <div className={s.dMeta}>
                            {row.scope === 'local' &&
                              row.ownPublishedAt != null &&
                              row.ownPublishedAt !== row.publishedAt && (
                                <>
                                  <span>
                                    {tr('presets.table.detail.publishedHere', {
                                      time: longStatText(row.ownPublishedAt) ?? tr(UNKNOWN),
                                    })}
                                  </span>
                                  <i className={s.dDot} aria-hidden>·</i>
                                </>
                              )}
                            {row.scope === 'local' && (
                              <>
                                <span>
                                  {tr('presets.table.detail.publishedCloud', {
                                    time: longStatText(row.publishedAt ?? undefined) ?? tr(UNKNOWN),
                                  })}
                                </span>
                                <i className={s.dDot} aria-hidden>·</i>
                              </>
                            )}
                            <span>
                              {tr('presets.table.detail.cloudVersion', {
                                version: row.presetVersion ?? tr(UNKNOWN),
                              })}
                            </span>
                          </div>
                        )}

                        {/* 认不出的注记：不顶在主词上，放成讲道理的一行 */}
                        {untrusted && (
                          <p className={s.dNote}>
                            {tr(PRESET_UNTRUSTED_NOTE)}
                          </p>
                        )}

                        {/*
                         * —— 备注 / 归属（2026-10-10 第八轮：展示行 + 卡底编辑带）——
                         * 展示行就在这里：label 与「版本 / 来源 / 基于」同一条 56px 刻度、
                         * 右端一颗小胶囊「编辑」；点它折出的是**卡底那条带**（本块 DOM
                         * 之后的 `.dBand`，备注一条、归属一条、两条独立可同开）。
                         * 两块都没有（云端行 / preset 行）时整块不渲染。
                         */}
                        {(row.remarkKey !== null || localAttr) && (
                          <div className={s.editrows}>
                            {row.remarkKey !== null && (
                              <div className={s.dRow}>
                                <span className={s.dKey}>{tr('presets.table.detail.key.remark')}</span>
                                <span
                                  className={s.editValue}
                                  data-empty={row.subtitle.trim() === ''}
                                >
                                  {row.subtitle.trim() === ''
                                    ? tr('presets.table.detail.remarkEmpty')
                                    : row.subtitle}
                                </span>
                                <button
                                  type="button"
                                  className={s.editTrigger}
                                  title={tr('presets.table.detail.editRemark.title')}
                                  onClick={() =>
                                    setBands((b) => ({
                                      ...b,
                                      remark: b.remark === remarkBand ? null : remarkBand,
                                    }))
                                  }
                                >
                                  {tr('presets.table.detail.edit')}
                                </button>
                              </div>
                            )}

                            {localAttr && (
                              <div className={s.dRow}>
                                <span className={s.dKey}>{tr('presets.table.detail.key.attr')}</span>
                                <span className={s.editValue}>
                                  <span className={s.editKey}>{tr('presets.table.detail.key.machine')}</span>
                                  <span>{ownMachineId ?? DASH_}</span>
                                  <span className={s.editKey}>{tr('presets.table.detail.key.variant')}</span>
                                  <span>{ownVersionId ?? DASH_}</span>
                                </span>
                                <button
                                  type="button"
                                  className={s.editTrigger}
                                  title={tr('presets.table.detail.editAttr.title')}
                                  onClick={() =>
                                    setBands((b) => ({
                                      ...b,
                                      attr: b.attr === attrBand ? null : attrBand,
                                    }))
                                  }
                                >
                                  {tr('presets.table.detail.edit')}
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
  const tr = useT()
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
              <span className={s.dKey}>{tr('presets.table.detail.key.remark')}</span>
              <textarea
                ref={taRef}
                className={s.remarkArea}
                rows={1}
                value={text}
                placeholder={tr('presets.table.detail.remarkEmpty')}
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
                  title={tr('presets.table.remark.restoreDefault.title')}
                  onClick={() => void save(null)}
                >
                  {tr('presets.table.remark.restoreDefault')}
                </button>
              )}
              <button
                type="button"
                className={s.ghostBtn}
                disabled={saving || props.busy}
                onClick={props.onClose}
              >
                {tr('presets.table.common.cancel')}
              </button>
              <button
                type="button"
                className={s.saveBtn}
                disabled={saving || props.busy || !dirty}
                onClick={() => void save(text.trim())}
              >
                {tr('presets.table.common.save')}
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
  const tr = useT()
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
        aria-label={tr('presets.table.combo.expandAria')}
       
        onClick={() => {
          setOpen((v) => !v)
          wrapRef.current?.querySelector('input')?.focus()
        }}
      >
        <Chevron name="chevronDown" size={12} className={ui.selectCaret} data-on={open} />
      </button>
      {open && (
        <FieldPopover
          anchor={wrapRef.current}
          matchWidth
          onClose={() => setOpen(false)}
        >
          <div className={s.comboMenu}>
            {props.options.length === 0 && (
              <p className={s.comboEmpty}>{tr('presets.table.combo.empty')}</p>
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
  const tr = useT()
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
              <span className={s.dKey}>{tr('presets.table.detail.key.machine')}</span>
              <ComboField
                value={machineId}
                onValue={(v) => {
                  setMachineId(v)
                  /* 换机型：版本候选跟着换，已填版本清空 */
                  setVersionId('')
                }}
                options={machineOptions}
                placeholder={tr('presets.table.combo.placeholder')}
              />
            </div>
            <div className={s.bandRow}>
              <span className={s.dKey}>{tr('presets.table.detail.key.variant')}</span>
              <ComboField
                value={versionId}
                onValue={setVersionId}
                options={versionOptions}
                placeholder={tr('presets.table.combo.placeholder')}
              />
            </div>
            <div className={s.bandFoot}>
              <button
                type="button"
                className={s.ghostBtn}
                disabled={saving || props.busy}
                onClick={props.onClose}
              >
                {tr('presets.table.common.cancel')}
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
                {tr('presets.table.common.save')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
