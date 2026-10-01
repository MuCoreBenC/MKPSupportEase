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
 *   MKP     状态 已应用 / 未应用   操作 [应用] → 唯一底账 `STORAGE.clientActive`
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
 * **前端一行都不编**：拿不到就写「未知」并在 title 里说清为什么（用户自己放进预设目录的
 * 文件就是这一种 —— 契约的 `LocalUserFile` 上没有这两个字段）。
 * 时间显示两种形态：行上只写 `MM-DD`、展开面板带年份 —— 见 `shortStatText` / `longStatText`。
 */


import { Fragment } from 'react'
import type { ContextMenuApi } from '../../components/menu'
import { longStatText, shortStatText } from '../store/package'
import {
  ACTION_TEXT,
  CLOUD_STATE_TEXT,
  CLOUD_STATE_WHY,
  DASH_,
  DEMO_STAT_WHY,
  FILE_SIZE_WHY,
  FILE_TIME_WHY,
  KIND_NAME,
  DOWNLOAD_WHY,
  RELEASE_DOWNLOAD_WHY,
  RELEASE_SIZE_WHY,
  RELEASE_TIME_WHY,
  KIND_AXIS_TEXT,
  LIVE_TEXT,
  LIVE_WHY,
  NO_ASSET_WHY,
  NO_STAT_WHY,
  originChip,
  UNKNOWN,
  versionsText,
} from './presetTree'
import type {
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
   * 时间那一格的 title。三种来源，三句话：
   *
   *   发布行     发布 / 下载时刻（按表分语义）
   *   真值（bbs） `statFrom === 'file'` —— 真仓那份文件 + 上游 manifest 记的时间
   *   其余       演示推值 / 没有
   */
  const statWhyOf = (row: PresetTableRow): string =>
    row.origin === 'release'
      ? row.scope === 'cloud'
        ? RELEASE_TIME_WHY.cloud
        : RELEASE_TIME_WHY.local
      : row.statFrom === 'file'
        ? FILE_TIME_WHY
        : row.modifiedText === undefined
          ? NO_STAT_WHY
          : DEMO_STAT_WHY

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

              <th className={s.thTime} scope="col">
                时间
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
              /* 与参数页同一套交互：点行展开下方的内容，同一时刻只开一行 */
              const expanded = row.rowKey === expandedKey
              return (
                <Fragment key={row.rowKey}>
                <tr
                  className={expanded ? `${s.row} ${s.rowOpen}` : s.row}
                  data-live={inUse}
                  data-active={row.rowKey === activeKey}
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
                    <span className={s.originChip} data-origin={row.origin} title={originChip(row).title}>
                      {originChip(row).text}
                    </span>
                  </td>

                  {/*
                   * 操作列常驻。三种画法，**已经生效 / 已经下载的那一种是灰字不是按钮**：
                   * 已经在用的东西没有可点的动作，给个按钮只会让人点一下看看会发生什么。
                   */}
                  <td className={s.act}>
                    {row.scope === 'local' ? (
                      row.live ? (
                        <span className={s.actDone} title={LIVE_WHY[kind].on}>
                          {LIVE_TEXT[kind].on}
                        </span>
                      ) : row.assetId === undefined && row.releaseUid === undefined ? (
                        /* 用户自己的文件没有 asset id，契约那两个写只认 asset id */
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
                    ) : row.downloaded ? (
                      <span className={s.actDone} title={CLOUD_STATE_WHY.downloaded}>
                        {CLOUD_STATE_TEXT.downloaded}
                      </span>
                    ) : (
                      <button
                        type="button"
                        className={s.actBtn}
                        disabled={busy}
                        /* 发布行是真下载，官方行仍是「未实现」—— 文案按行分流 */
                        title={
                          row.releaseUid !== undefined ? RELEASE_DOWNLOAD_WHY : DOWNLOAD_WHY
                        }
                        onClick={() => onDownload(row)}
                      >
                        下载
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
                        <dt className={s.factKey}>类型</dt>
                        <dd className={s.factVal}>{KIND_NAME[row.kind]}</dd>

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
                        <dd className={s.factVal} title={originChip(row).title}>
                          {originChip(row).text}
                        </dd>

                        <dt className={s.factKey}>状态</dt>
                        <dd className={s.factVal}>
                          {row.scope === 'local'
                            ? LIVE_TEXT[kind][row.live ? 'on' : 'off']
                            : row.downloaded
                              ? CLOUD_STATE_TEXT.downloaded
                              : CLOUD_STATE_TEXT.pending}
                          {row.applied && ' · 正在用'}
                        </dd>

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

                        <dt className={s.factKey}>修改时间</dt>
                        <dd className={s.factVal} title={statWhyOf(row)}>
                          {longStatText(row.modifiedText) ?? UNKNOWN}
                        </dd>

                        <dt className={s.factKey}>置顶</dt>
                        <dd className={s.factVal}>{row.pinned ? '已置顶' : '未置顶'}</dd>
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

      <p className={s.hint}>
        右键任意一行还有置顶 / 重命名 / 删除 / 查看详情（没有鼠标就 Shift+F10 或菜单键）
      </p>
    </div>
  )
}
