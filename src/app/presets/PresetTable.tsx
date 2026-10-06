/*
 * 预设表（**列随类型变** + 常驻操作列）。本地表与云端表共用这一个件。
 *
 * ```
 * MKP 配置（8 列，没有喷嘴层高）
 * 名称                 机型      版本      时间    大小     来源   状态      操作
 * A1.toml              A1       标准版    09-14   4.2 KB   官方   ● 已应用  已应用
 * presets/mkp/A1.toml
 * A1MF_260628.toml     A1       快拆版…   09-14   3.7 KB   官方   未应用    [应用]
 *
 * 切片器配置（9 列，没有版本）
 * 名称                 机型      喷嘴  层高  时间    大小     来源   状态      操作
 * MKPProcess A1 0.4…   A1       0.4   0.20  09-14   8.1 KB   官方   ● 已复制  已复制
 * ```
 *
 * # MKP 和切片器是两种东西，列不一样
 *
 * 两种类型曾经硬塞进同一套列，于是 MKP 行的喷嘴 / 层高永远是「—」—— 占着两列说废话。
 * 作者原话：「MKP 没有喷嘴，没有层高就不要显示，不要占个位置在那里。那个版本的话呢，
 * 切片器没有版本的话就不要显示版本」。所以切分段控件时**表头列数真的跟着换**：
 *
 *   MKP 配置    名称 / 机型 / 版本 / 时间 / 大小 / 来源 / 状态 / 操作
 *   切片器配置  名称 / 机型 / 喷嘴 / 层高 / 时间 / 大小 / 来源 / 状态 / 操作
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
 * # 两种类型的「生效」是两件不同的事
 *
 *   MKP     状态 已应用 / 未应用   操作 [应用] → 唯一底账（run/active-preset.json）
 *   切片器   状态 已复制 / 未复制   操作 [复制] → `api.copyToSlicer()`
 *   云端     状态 已下载 / 未下载   操作 [下载] → `api.downloadFiles()`（**照抛未实现**）
 *
 * 已生效 / 已下载那一行的操作列是**灰字，不是按钮**：已经在用的东西没有可点的动作。
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
 *   切片器官方行 `statFrom: 'file'` —— 真仓那份文件的字节数 + 上游 manifest 记的更新时间
 *   其余官方行   `PresetFileInfo.sizeText` / `.modifiedText` —— 按 path 的 FNV 哈希稳定推的演示值
 *   发布行       时间 = 发布 / 下载时刻（ISO）、大小 = TOML 字节数 —— 都不是编的
 *
 * **前端一行都不编**：拿不到就写「未知」并在 title 里说清为什么。
 * （用户自己那份的大小与时间**是真值**：后端扫盘拿的，`statFrom: 'file'` 那一档。）
 * 时间显示两种形态：行上只写 `MM-DD`、展开面板带年份 —— 见 `shortStatText` / `longStatText`。
 */


import { Fragment } from 'react'
import type { ContextMenuApi } from '../../components/menu'
import { longStatText, shortStatText } from '../store/package'
import {
  ACTION_TEXT,
  ARCHIVE_KEY,
  ARCHIVE_WHY,
  archiveOpenText,
  BASED_ON_KEY,
  BASED_ON_TEXT,
  BASED_ON_WHY,
  basedOnCellText,
  CLOUD_STATE_TEXT,
  CLOUD_STATE_WHY,
  DASH_,
  DEMO_STAT_WHY,
  FILE_SIZE_WHY,
  MINE_APPLY_WHY,
  MINE_BODY_WHY,
  MINE_DRAWER,
  MINE_EDIT_TEXT,
  MINE_EDIT_WHY,
  MINE_NOT_PRESET_WHY,
  MINE_UNREADABLE_TEXT,
  mineUnreadableWhy,
  DOWNLOAD_WHY,
  EDIT_TEXT,
  EDIT_WHY,
  RELEASE_DOWNLOAD_WHY,
  RELEASE_REPAIR_WHY,
  RELEASE_SIZE_WHY,
  RELEASE_STATE_WHY,
  RELEASE_TIME_WHY,
  RELEASE_UNTRUSTED_NOTE,
  RELEASE_UPDATE_WHY,
  UPDATE_ACTION_TEXT,
  UPDATE_STATE_TEXT,
  KIND_AXIS_TEXT,
  LIVE_TEXT,
  LIVE_WHY,
  NO_ASSET_WHY,
  NO_STAT_WHY,
  SLICER_RELEASE_WHY,
  originCellOf,
  updateStateOf,
  UNKNOWN,
  versionsText,
} from './presetTree'
import type {
  LocateTarget,
  PresetKindAxis,
  PresetLocalRow,
  PresetScopeAxis,
  PresetTableRow,
} from './presetTree'
import s from './PresetTable.module.css'

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
   * GitHub / Gitee / 自定义源 的依据。null = 没配源，来源退「官方」。
   */
  sourceLabel: string | null
  /**
   * 这一份在归档区里有几个**旧版本**（按文件名对）。
   * `0` = 没有 → 展开详情里那一格**不显示**（没有 ≠ 未知，不必占一个"—"）
   */
  archiveCountOf: (fileName: string) => number
  /** 打开「旧版本」抽屉（归档的可视化在页面层的抽屉里，表这边只给入口） */
  onOpenArchive: (row: PresetTableRow) => void
  /** 打开「我自己的这一份」抽屉（用户线的正文：**只读**） */
  onOpenMine: (row: PresetTableRow) => void
  /**
   * 开始改这一份（临时编辑那条链）。两条线共一个入口：
   * 交付行只给"与目录一致"的那一份；我自己那份都能改（认不出是哪一类的不给）。
   */
  onEdit: (row: PresetTableRow) => void
  /** 本地表那一颗按钮：MKP 是「应用」，切片器是「复制」。两件事一个入口，由 `kind` 分 */
  onLive: (row: PresetLocalRow) => void
  /** 云端表那一颗按钮。**真调 `downloadFiles`，照抛未实现** —— 不编假进度条 */
  onDownload: (row: PresetTableRow) => void
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
  onOpenMine,
  onEdit,
  onLive,
  onDownload,
}: Props) {
  /* 这一个布尔决定表头有几列、每行画哪几格。两处都读它，不许各判一次 */
  const mkp = kind === 'mkp'

  /** 空表三种话，说的是三件不同的事 —— 混成一句「没有数据」就查不出是哪一件 */
  const emptyText = (): string => {
    if (failure !== null) return `加载失败：${failure}`
    if (total === 0) {
      return scope === 'local'
        ? `本机还没有这个组合的「${KIND_AXIS_TEXT[kind]}」—— 官方文件下载之后会出现在这里，你自己放进预设目录的也会`
        : `云端没有这个组合的「${KIND_AXIS_TEXT[kind]}」—— 菜单上没有，就下不到`
    }
    return `这一档的 ${total} 条都被搜索词「${query.trim()}」筛掉了`
  }

  /**
   * 时间那一格的 title。**有值就给具体时间 + 一个词说清它是哪个时间**，
   * 不解释实现机制（`catalog.publishedAt` 这种内部名字不出现在界面上）：
   *
   *   发布行     云端 = 「云端更新时间：…」；本地 = 「下载时间：…」
   *   我的文件   「修改于：…」—— 用户自己那份的最后一次保存，这一格叫修改时间才对
   *   真值（bbs） 「文件时间：…」—— 上游记录的内容更新时间
   *   演示推值   照实说它是演示值（唯一还得解释的那一档：它的值不是真的）
   *   拿不到     各自有一句"为什么"（目录没盖戳 / 文件系统没给 / 本来就没有）
   */
  const statWhyOf = (row: PresetTableRow): string => {
    const when = longStatText(row.modifiedText)
    if (row.origin === 'release') {
      return row.scope === 'cloud'
        ? when === undefined
          ? RELEASE_TIME_WHY.cloudMissing
          : `云端更新时间：${when}`
        : when === undefined
          ? RELEASE_TIME_WHY.localMissing
          : `下载时间：${when}`
    }
    if (row.scope === 'local' && row.origin === 'mine') {
      return when === undefined ? NO_STAT_WHY : `修改于：${when}（你自己这份最后一次保存的时刻）`
    }
    if (row.statFrom === 'file') {
      return when === undefined ? NO_STAT_WHY : `文件时间：${when}（上游记录的那份文件的更新时间）`
    }
    return row.modifiedText === undefined ? NO_STAT_WHY : DEMO_STAT_WHY
  }

  /** 行上那一格：只写月-日 —— `shortStatText` 统一三种来源的写法 */
  const statCell = (row: PresetTableRow) => (
    <td className={s.stat} title={statWhyOf(row)}>
      {shortStatText(row.modifiedText) ?? UNKNOWN}
    </td>
  )

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
               * 本地表是**时间**（官方行是仓库记的更新时间、交付行是下载时间、我的是修改时间
               * —— 一列三种真来源，格上的 tooltip 说清各自是哪一个，见 `statWhyOf`）。
               */}
              <th className={s.thTime} scope="col">
                {scope === 'cloud' ? '云端更新' : '时间'}
              </th>
              <th className={s.thOrigin} scope="col">
                来源
              </th>
              <th className={s.thAct} scope="col">
                操作
              </th>
            </tr>
          </thead>

          <tbody>
            {rows.map((row) => {
              const version = versionsText(row.versions)
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
              /* 认不出是哪一版的注记只在展开详情里出现（tampered 那一档） */
              const untrusted = row.releaseState === 'tampered'
              /*
               * 这一份在归档里有几个旧版本（换版本时被换下来的）。0 = 没有。
               * **用户线那一份问都不问**：归档是官方版本生命周期的事，
               * 用户自己的文件不进归档，也不该因为同名就借到官方的旧版本。
               */
              const archiveCount = row.origin === 'mine' ? 0 : archiveCountOf(row.fileName)
              /** 云端那一格：与目录一致的那一份在本机（切片器官方行看 `downloaded`，交付行看三态） */
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
              return (
                <Fragment key={row.rowKey}>
                <tr
                  className={expanded ? `${s.row} ${s.rowOpen}` : s.row}
                  data-live={inUse}
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
                    <span className={s.nameLine}>
                      {/* 置顶是纯前端排序，所以标记也只是一枚小字，不改任何状态 */}
                      {row.pinned && (
                        <span className={s.pin} title="已置顶：只影响这张表的顺序，不是「设为当前」">
                          置顶
                        </span>
                      )}
                      <span className={s.nameText} title={row.fileName}>
                        {row.fileName}
                      </span>
                      {row.scope === 'local' && row.untagged && (
                        <span
                          className={s.untagged}
                          title="你没给这个文件标适用机型 —— 不替你猜一个，所以它在每台机型下都列出来"
                        >
                          未标机型
                        </span>
                      )}
                      {/*
                       * 我那份是**从旧版官方**改出来的（官方已经换新版）—— 一眼看得见。
                       * 它不是"坏文件"：照常能用能改，展开详情里那一格说得更全。
                       */}
                      {row.scope === 'local' && row.basedOn === 'outdated' && (
                        <span className={s.basedOld} title={BASED_ON_WHY.outdated}>
                          {BASED_ON_TEXT.outdated}
                        </span>
                      )}
                      {/*
                       * 第九层：读不出来的那一份（编码 / TOML 语法 / 指向用户根之外）——
                       * 一眼看得见；为什么读不出来在 title 里。**照常列出来**，不藏。
                       */}
                      {row.scope === 'local' && row.mineState === 'unreadable' && (
                        <span
                          className={s.unreadable}
                          title={mineUnreadableWhy(row.mineStateDetail)}
                        >
                          {MINE_UNREADABLE_TEXT}
                        </span>
                      )}
                    </span>
                    {/* 第二行等宽小字：给人核对磁盘位置的，不是标题 */}
                    <span className={s.path} title={row.path}>
                      {row.path}
                    </span>
                  </td>

                  <td className={s.machine} title={row.machineText}>
                    {row.machineText}
                  </td>

                  {mkp ? (
                    /* 版本：只读的一列。不可点、不排序、不分组（见文件头） */
                    <td className={s.version} title={version.why}>
                      {version.text}
                    </td>
                  ) : (
                    <>
                      <td className={s.num}>{row.nozzle ?? DASH_}</td>
                      <td className={s.num}>{row.layerHeight ?? DASH_}</td>
                    </>
                  )}

                  {statCell(row)}

                  <td className={s.origin}>
                    {locateTarget !== null ? (
                      /*
                       * 「复制自 X」这类可定位的来源：**它是按钮**——点击定位到来源那一行
                       * 并高亮一下（页面层切轴 + 展开 + 闪光）。stopPropagation：
                       * 点来源不算"展开这一行"。
                       */
                      <button
                        type="button"
                        className={`${s.originChip} ${s.originLink}`}
                        data-origin={row.origin}
                        title={origin.title}
                        onClick={(e) => {
                          e.stopPropagation()
                          onLocate(locateTarget)
                        }}
                      >
                        {origin.text}
                      </button>
                    ) : (
                      <span className={s.originChip} data-origin={row.origin} title={origin.title}>
                        {origin.text}
                      </span>
                    )}
                  </td>

                  {/*
                   * 操作列常驻。三种画法，**已经生效 / 已经下载的那一种是灰字不是按钮**：
                   * 已经在用的东西没有可点的动作，给个按钮只会让人点一下看看会发生什么。
                   */}
                  <td className={s.act}>
                    {row.scope === 'local' ? (
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
                          title={untrusted ? RELEASE_REPAIR_WHY : RELEASE_UPDATE_WHY}
                          onClick={() => onDownload(row)}
                        >
                          {UPDATE_ACTION_TEXT.update}
                        </button>
                      ) : row.live ? (
                        <span className={s.actDone} title={LIVE_WHY[kind].on}>
                          {LIVE_TEXT[kind].on}
                        </span>
                      ) : row.origin === 'mine' ? (
                        /*
                         * **用户自己那份也能被应用**（第七层）：与官方那份同一个入口、
                         * 同一条底账 —— "只读"是文件归属的属性，不是"能不能被使用"的属性。
                         * 两档不给按钮（不给必报错的按钮）：认不出是 MKP 预设的（`.json`），
                         * 以及**第九层读不出来的**（后端 `read_preset_text` 的第一关就会拒）。
                         */
                        row.kind !== 'mkp_preset' ? (
                          <span className={s.actNone} title={MINE_NOT_PRESET_WHY}>
                            {DASH_}
                          </span>
                        ) : row.mineState === 'unreadable' ? (
                          <span className={s.actNone} title={mineUnreadableWhy(row.mineStateDetail)}>
                            {DASH_}
                          </span>
                        ) : (
                          <button
                            type="button"
                            className={s.actBtn}
                            disabled={busy}
                            title={MINE_APPLY_WHY}
                            onClick={() => onLive(row)}
                          >
                            {ACTION_TEXT.mkp}
                          </button>
                        )
                      ) : row.releaseUid !== undefined && row.kind !== 'mkp_preset' ? (
                        /*
                         * 切片器那一类的交付行（catalog 登记、能下载）在本地表里**没有可点的动作**：
                         * 不能「应用」（使用中指针只认 MKP 预设）；「复制」那条路只认资产库的
                         * asset id —— 原来这里画的是「复制」，点了**静静没反应**
                         * （`runLive` 里 assetId 是 undefined 就 return）。给一个点了没反应的
                         * 按钮，与"点了必报错"同罪：不给。
                         */
                        <span className={s.actNone} title={SLICER_RELEASE_WHY}>
                          {DASH_}
                        </span>
                      ) : row.assetId === undefined && row.releaseUid === undefined ? (
                        /* 官方副本没有 asset id 时也应用不了（契约那两个写只认 asset id） */
                        <span className={s.actNone} title={NO_ASSET_WHY}>
                          {DASH_}
                        </span>
                      ) : (
                        <button
                          type="button"
                          className={s.actBtn}
                          disabled={busy}
                          title={LIVE_WHY[kind].off}
                          onClick={() => onLive(row)}
                        >
                          {ACTION_TEXT[kind]}
                        </button>
                      )
                    ) : gotIt ? (
                      <span className={s.actDone} title={CLOUD_STATE_WHY.downloaded}>
                        {CLOUD_STATE_TEXT.downloaded}
                      </span>
                    ) : (
                      <button
                        type="button"
                        className={s.actBtn}
                        disabled={busy}
                        /* 发布行是真下载；官方行仍是「未实现」—— 文案按行分流 */
                        title={
                          row.releaseUid !== undefined
                            ? needsUpdate
                              ? untrusted
                                ? RELEASE_REPAIR_WHY
                                : RELEASE_UPDATE_WHY
                              : RELEASE_DOWNLOAD_WHY
                            : DOWNLOAD_WHY
                        }
                        onClick={() => onDownload(row)}
                      >
                        {needsUpdate ? UPDATE_ACTION_TEXT.update : UPDATE_ACTION_TEXT.missing}
                      </button>
                    )}
                  </td>
                </tr>

                {expanded && (
                  <tr className={s.expandRow}>
                    {/*
                     * colSpan 必须**跟着上面的表头列数走**：MKP 6 列、切片器 7 列。
                     * 「删状态/大小列」那轮砍了两列后这里的 8 没跟着改 —— 比表头多出的
                     * 那一跨会撑出一个匿名的第 8 列，吃掉表格右侧的全部余量，且没有任何
                     * 底色：表头和主行右边就多出一条白（作者：「切片器配置怎么点击展开
                     * 变成这样」）。改列数时这里要一起改。
                     */}
                    <td colSpan={mkp ? 6 : 7}>
                      <dl className={s.facts}>
                        {/*
                         * 展开详情**只回答用户真正要问的事**：什么版本 / 从哪来 / 什么时候
                         * 下的 / 云端有没有更新 / 我改过没有 / 现在能做什么。
                         * 曾经机械地摊内部字段：「类型」（分段控件已经选定了类型）与
                         * 「置顶」（纯前端排序的偏好）在这里各占一行 —— 都是用户没问的。
                         */}

                        {mkp && (
                          <>
                            <dt className={s.factKey}>版本</dt>
                            <dd className={s.factVal} title={version.why}>
                              {version.text}
                            </dd>
                          </>
                        )}
                        {!mkp && (
                          <>
                            <dt className={s.factKey}>喷嘴</dt>
                            <dd className={s.factVal}>{row.nozzle ?? DASH_}</dd>
                            <dt className={s.factKey}>层高</dt>
                            <dd className={s.factVal}>{row.layerHeight ?? DASH_}</dd>
                          </>
                        )}

                        <dt className={s.factKey}>来源</dt>
                        <dd className={s.factVal}>
                          {locateTarget !== null ? (
                            <button
                              type="button"
                              className={s.factLink}
                              title={origin.title}
                              onClick={() => onLocate(locateTarget)}
                            >
                              {origin.text}
                            </button>
                          ) : (
                            <span title={origin.title}>{origin.text}</span>
                          )}
                        </dd>

                        {/*
                         * 我那份是从哪一份官方、哪一版改出来的（血统写在文件头，跟着文件走）。
                         * 「官方已换新版」这一档不是错误 —— 它只说"你这份是旧版派生"，
                         * 那份文件照常能用能改（要不要把改动挪到新版上，是另一件事）。
                         */}
                        {row.origin === 'mine' && (
                          <>
                            <dt className={s.factKey}>{BASED_ON_KEY}</dt>
                            <dd
                              className={s.factVal}
                              title={BASED_ON_WHY[row.basedOn ?? 'unknown']}
                            >
                              {basedOnCellText(row)}
                            </dd>
                          </>
                        )}

                        {/*
                         * 第九层：读不出来那一份，把"为什么"写在原地（原因来自后端）。
                         * 它只是读不出来 —— 文件还是用户自己的，程序不动它。
                         */}
                        {row.origin === 'mine' && row.mineState === 'unreadable' && (
                          <>
                            <dt className={s.factKey}>文件</dt>
                            <dd
                              className={s.factVal}
                              title={mineUnreadableWhy(row.mineStateDetail)}
                            >
                              {MINE_UNREADABLE_TEXT}
                            </dd>
                          </>
                        )}

                        {/*
                         * 状态：**release 行的主词按"云端 vs 盘上"三态说**（未下载 / 已下载 /
                         * 有更新），"认不出是哪一版"不再顶在主词上 —— 那是信任维度的事，
                         * 有它自己的一行注记（见下）。主词后面缀「正在用」；
                         * 其余来源说"生效没生效"。
                         */}
                        <dt className={s.factKey}>状态</dt>
                        <dd
                          className={s.factVal}
                          title={
                            row.releaseState !== undefined
                              ? RELEASE_STATE_WHY[row.releaseState]
                              : undefined
                          }
                        >
                          {row.releaseState !== undefined
                            ? UPDATE_STATE_TEXT[updateState ?? 'latest']
                            : row.scope === 'local'
                              ? LIVE_TEXT[kind][row.live ? 'on' : 'off']
                              : row.downloaded
                                ? CLOUD_STATE_TEXT.downloaded
                                : CLOUD_STATE_TEXT.pending}
                          {row.applied && ' · 正在用'}
                        </dd>

                        {untrusted && (
                          <>
                            {/*
                             * 「本机版本」：只认不出那一档才有的注记行。它不是把异常藏起来 ——
                             * 恰恰是把"认不出"的实情（可能是旧版、可能被改过、更新即可换掉）
                             * 从吓人的主词位置挪到讲道理的位置。
                             */}
                            <dt className={s.factKey}>本机版本</dt>
                            <dd className={`${s.factVal} ${s.factNote}`} title={RELEASE_STATE_WHY.tampered}>
                              {RELEASE_UNTRUSTED_NOTE}
                            </dd>
                          </>
                        )}

                        <dt className={s.factKey}>大小</dt>
                        <dd
                          className={s.factVal}
                          title={
                            row.origin === 'release'
                              ? RELEASE_SIZE_WHY
                              : row.statFrom === 'file'
                                ? FILE_SIZE_WHY
                                : row.sizeText === undefined
                                  ? NO_STAT_WHY
                                  : DEMO_STAT_WHY
                          }
                        >
                          {row.sizeText ?? UNKNOWN}
                        </dd>

                        {/*
                         * 时间：**名字跟着语义走** —— 交付行本地是「下载时间」（这份字节
                         * 什么时候落到本机的）、云端是「云端更新」（这次发布的时刻）、
                         * 我自己的文件才叫「修改时间」（最后一次保存）。同一个
                         * `modifiedText` 不再一个「修改时间」包打天下。
                         */}
                        <dt className={s.factKey}>
                          {row.origin === 'release'
                            ? row.scope === 'cloud'
                              ? '云端更新'
                              : '下载时间'
                            : '修改时间'}
                        </dt>
                        <dd className={s.factVal} title={statWhyOf(row)}>
                          {longStatText(row.modifiedText) ?? UNKNOWN}
                        </dd>

                        {row.origin === 'release' && (
                          <>
                            {row.scope === 'local' && (
                              <>
                                {/*
                                 * 本地交付行的「云端更新」：与上面的「下载时间」是**两个时间**
                                 * （这份字节什么时候落到本机的 / 云端什么时候换的版）——
                                 * 用户要对照的正是这两个。
                                 */}
                                <dt className={s.factKey}>云端更新</dt>
                                <dd
                                  className={s.factVal}
                                  title={
                                    row.publishedAt
                                      ? undefined
                                      : RELEASE_TIME_WHY.cloudMissing
                                  }
                                >
                                  {longStatText(row.publishedAt ?? undefined) ?? UNKNOWN}
                                </dd>
                              </>
                            )}
                            {/*
                             * 「云端版本」：目录指纹是**版本的身份证**（新世界目录没有版本名，
                             * 这是唯一权威标识）。它在这里有意义、在「来源」列里没有 ——
                             * 来源答的是"从哪来"，不是"哪一版"。
                             */}
                            <dt className={s.factKey}>云端版本</dt>
                            <dd
                              className={s.factVal}
                              title="目录指纹（catalog revision）—— 新世界目录没有版本名，它就是这一版的身份证"
                            >
                              {row.releaseVersion ?? UNKNOWN}
                            </dd>
                          </>
                        )}

                        {/*
                         * 归档：官方旧版本留档。**没有旧版本就不显示这一格** ——
                         * "没有"不是"未知"，不必占一个「—」。
                         * 它和上面的「状态」说的是两件事：状态说盘上这一份对不对，
                         * 这里说"以前那几份还在不在"。
                         */}
                        {archiveCount > 0 && (
                          <>
                            <dt className={s.factKey}>{ARCHIVE_KEY}</dt>
                            <dd className={s.factVal}>
                              <button
                                type="button"
                                className={s.factLink}
                                title={ARCHIVE_WHY}
                                onClick={() => onOpenArchive(row)}
                              >
                                {archiveOpenText(archiveCount)}
                              </button>
                            </dd>
                          </>
                        )}

                        {/*
                         * 临时编辑的入口：**只有"与目录一致"的 MKP 交付行**给 ——
                         * 没下载（missing）就没有正文可改；需更新（stale）那一份的内容本身存疑，
                         * 先更新再改（这与"不给点了必报错的按钮"是同一条口径）；
                         * 切片器那一类也不是预设正文，改无从谈起。
                         */}
                        {row.origin === 'release' &&
                          row.kind === 'mkp_preset' &&
                          row.releaseState === 'ok' && (
                          <>
                            <dt className={s.factKey}>{EDIT_TEXT.cell}</dt>
                            <dd className={s.factVal}>
                              <button
                                type="button"
                                className={s.factLink}
                                title={EDIT_WHY}
                                onClick={() => onEdit(row)}
                              >
                                {EDIT_TEXT.open}
                              </button>
                            </dd>
                          </>
                        )}

                        {/*
                         * 用户线那一份：**看正文** + **改这份**（第八层）。
                         * 两条都是它自己的入口：改的是临时文件，保存时**写回它自己**
                         * （不另存一份新的、也不碰官方原件）。
                         * 认不出是哪一类的那份（`.json`）不给「改」—— 这一层只改 TOML 预设；
                         * **第九层读不出来的**也不给（改的入口同样过文件级检查）。
                         * 看正文照旧给：用户要能看着它去修（读它不算"用"）。
                         */}
                        {row.origin === 'mine' && (
                          <>
                            <dt className={s.factKey}>{MINE_DRAWER.cell}</dt>
                            <dd className={s.factVal}>
                              <button
                                type="button"
                                className={s.factLink}
                                title={MINE_BODY_WHY}
                                onClick={() => onOpenMine(row)}
                              >
                                {MINE_DRAWER.open}
                              </button>
                            </dd>
                          </>
                        )}
                        {row.origin === 'mine' &&
                          row.kind === 'mkp_preset' &&
                          row.mineState !== 'unreadable' && (
                          <>
                            <dt className={s.factKey}>{MINE_EDIT_TEXT.cell}</dt>
                            <dd className={s.factVal}>
                              <button
                                type="button"
                                className={s.factLink}
                                title={MINE_EDIT_WHY}
                                onClick={() => onEdit(row)}
                              >
                                {MINE_EDIT_TEXT.open}
                              </button>
                            </dd>
                          </>
                        )}
                      </dl>
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
