/*
 * 一张参数卡。一个真 section = 一张卡。
 *
 * 标题读 `ParamSection.label`（中文名，'空间偏移' / '圆盘动作控制'），
 * 条数读 `ParamSection.count`。更早那一版的标题来自一份手抄的卡片切分表，
 * 这里换成注册表自带的分组名。
 *
 * # 受控参数树（A43 移植 · 装订子卡）：层级用**装订**画，状态用**折叠**画
 *
 * 层级 → 装订    分支不再用线画：孩子们装进一张 **1px 圆角子卡**，卡沿上骑一枚
 *                  **条件小签**（「外围结构 = 斜肋外墙」）—— 关系不用线说，签把
 *                  「谁挂在谁下面、什么条件下用」一句话说完。缩进由子卡的边距一层层
 *                  累加，轨与肘全部退场。
 * 状态 → 折叠    **条件不满足的支折成一条，满足的摊开成子卡**（判据统一成这一句）：
 *                  摊开 = 装订子卡，折叠 = 虚线灰卡（「▸ N 项参数」+ 各行名）。
 *                  点分段（或点折叠卡）切支时，旧支 240ms 收成一条、新支摊开
 *                  （grid-template-rows 0fr↔1fr 的既定动效曲线）。折叠条点了就是把
 *                  条件参数**改成那一支的值** —— 它是一条真的切换路径，不是摆设；
 *                  弃用支的切换被写值闸拦下（见 useParams 的 apply）。
 *
 * 「条件满不满足」由页面喂进来（`condOn`）—— 与 `useParams.findBlocked` 同一跳。
 *
 * # 交接动效：手风琴头
 *
 * **一张卡、一根签、一条常驻卡头** —— 折叠行升级成卡头（▸ N 项参数 · 行名…），
 * 参数行在头下面 0fr↔1fr 长出来；签与卡上缘钉死，摊 / 收只有卡身高度在变。
 *
 * 分支按「条件参数 + op + value」归组（`buildTree()`：条件参数在这张卡里出现过
 * 才挂进分支，组内顺序保证父在子前，一遍单扫）。搜索态（mode='search'）**全支摊开、
 * 不折叠** —— 「命中就该看见（哪怕此刻被关着）」是定下的规矩。整卡被关着仍整张收起
 * （只在浏览态）。
 *
 * 两个模式都走同一棵树的判据（谁挂在谁下面只有一套）。词沿用：**条件参数** /
 * **受控参数**。
 *
 * `gcode` 字段走 `renderBlock` —— 多行文本塞不进一行，由调用方给整块编辑器。
 */

import type { ReactNode } from 'react'
import Highlight from './highlight'
import { valueText } from './useParams'
import type { BlockedBy, ParamDef } from './useParams'
import ParamRow from './ParamRow'
import s from './ParamCard.module.css'

interface Props {
  /** 卡片标题：注册表 section 的中文名 */
  title: string
  /** 搜索结果里标出这张卡属于哪一类 */
  subtitle?: string
  fields: ParamDef[]
  mode?: 'normal' | 'search'
  valueOf: (key: string) => string
  savedValueOf: (key: string) => string
  factoryOf: (key: string) => string
  dirtyOf: (key: string) => boolean
  blockedByOf: (key: string) => BlockedBy | null
  /**
   * 一个条件**现在满不满足** —— 分支白底 / 灰底（摊开 / 折叠）的判据。
   * 口径与 `useParams.findBlocked` 同一跳，由页面喂进来。
   */
  condOn: (cond: { key: string; op: 'eq' | 'neq' | 'gt'; value: string }) => boolean
  expandedKey?: string | null
  onToggleExpand?: (key: string) => void
  onEdit: (key: string, next: string) => void
  onRevertToSaved: (key: string) => void
  /** G-code 这种整块的控件由调用方画 */
  renderBlock?: (def: ParamDef) => ReactNode
  /**
   * 塔地图槽位：这张卡里**同时**含 X/Y 两个坐标字段时（=「擦料塔位置与打印」），
   * 调用方给的画布替下这两行。两个条件缺一不生效：坐标字段不全走普通行；
   * 搜索态也走普通行 —— 画布是给「调位置」用的，不是给「查字段」用的。
   */
  tower?: TowerSlot
  /**
   * 搜索词：卡片标题、分类小字与每一行（含展开的行详情）里命中的字加底。
   * 不搜索时是空串。
   */
  query?: string
}

type ShowWhen = NonNullable<NonNullable<ParamDef['meta']>['showWhen']>

/** 树的一种行：受控参数自己，`branches` 是直接挂在它下面的分支 */
interface RowNode {
  kind: 'row'
  def: ParamDef
  branches: BranchNode[]
}

/** 树的一种分支：「条件行 cond 满足时这几个孩子才用得上」，挂在一个条件行下面 */
interface BranchNode {
  kind: 'branch'
  cond: ShowWhen
  /** 分支自己的条件此刻满不满足 —— 它就是「这一支摊不摊开」的唯一判据 */
  on: boolean
  items: TreeNode[]
}

type TreeNode = RowNode | BranchNode

/**
 * 把一张卡的扁平字段表造成**树**。
 *
 * # 判据是「条件参数在我上面」，摆法用嵌套
 *
 * 挂进**它的条件行**下面。注册表的组内顺序保证父在子前、同支的孩子连着来，
 * 所以一遍单扫就够：
 *
 *   · 字段没带 showWhen、或条件参数还没在这张卡里出现过 → 顶层普通行，
 *     同时**敞开的分支作废**（回顶层 = 前面那支的孩子来完了）
 *   · 条件参数在卡里、且与**敞开的分支**同条件（key + op + value）→ 进这支
 *   · 条件参数在卡里、但敞开的是别的支 → 在条件行的**已开分支**里找同条件的
 *     复用；找不到就在条件行后面**开新分支**（插在它自己与它已有分支之后）
 *
 * `on` 在这里算一次（分支自己的条件），摊开 / 折叠直接读它。
 */
function buildTree(
  fields: ParamDef[],
  condOn: (cond: NonNullable<ShowWhen>) => boolean,
): TreeNode[] {
  const root: TreeNode[] = []
  /* 已造出来的行：条件行要挂分支时，得知道它在哪个容器、已经挂了哪几支 */
  const rowAt = new Map<string, { node: RowNode; list: TreeNode[] }>()
  /* 当前敞开的分支：同条件的孩子直接进它（免得每次都去条件行那里找） */
  let open: BranchNode | null = null

  const addRow = (def: ParamDef, list: TreeNode[]) => {
    const node: RowNode = { kind: 'row', def, branches: [] }
    list.push(node)
    rowAt.set(def.key, { node, list })
    return node
  }

  for (const def of fields) {
    const cond = def.meta?.showWhen
    const hook = cond !== undefined && cond.key !== def.key ? rowAt.get(cond.key) : undefined
    if (cond === undefined || hook === undefined) {
      open = null
      addRow(def, root)
      continue
    }
    const same = (b: BranchNode) =>
      b.cond.key === cond.key && b.cond.op === cond.op && b.cond.value === cond.value
    /* 显式标注：reuse 的初始化式里有 open、后面又有 open = reuse，让推导自己绕圈（TS7022） */
    const reuse: BranchNode | undefined =
      open !== null && same(open) ? open : hook.node.branches.find(same)
    if (reuse !== undefined) {
      addRow(def, reuse.items)
      open = reuse
      continue
    }
    const branch: BranchNode = { kind: 'branch', cond, on: condOn(cond), items: [] }
    /* 插在条件行后面、它已有分支的后面 —— indexOf 免得维护一套会失效的下标 */
    hook.list.splice(hook.list.indexOf(hook.node) + 1 + hook.node.branches.length, 0, branch)
    hook.node.branches.push(branch)
    addRow(def, branch.items)
    open = branch
  }

  return root
}

/** 塔地图槽位的形状 —— 供数方（PageParams）按这个造 */
export interface TowerSlot {
  xKey: string
  yKey: string
  /** side = 这一卡的全部参数行（含 X/Y）—— 地图右侧那一列的本体 */
  render: (xDef: ParamDef, yDef: ParamDef, side: ReactNode) => ReactNode
}

export default function ParamCard({
  title,
  subtitle,
  fields,
  mode = 'normal',
  valueOf,
  savedValueOf,
  factoryOf,
  dirtyOf,
  blockedByOf,
  condOn,
  expandedKey = null,
  onToggleExpand,
  onEdit,
  onRevertToSaved,
  renderBlock,
  tower,
  query = '',
}: Props) {
  /*
   * 整张卡都不适用就整张收起来（**只在浏览态**）。
   *
   * 作者：「切换成圆盘，还是显示擦料塔的」——量下来是这样：擦料塔那些行的条件参数
   * （`wiping.have_wiping_components` = 擦料方式）住在**另一张卡**里，而早先的
   * 写法只在「条件参数在同一张卡」时才给 `blockedBy`，跨卡的行于是一点标记都没有 ——
   * 切到圆盘擦拭之后，它们看起来和普通行一模一样、还能改。
   *
   * 现在两件事一起：
   *   · 跨卡的条件也照常置灰 + 写「需先让『擦料方式』等于 擦料塔」（见下面 `blockedBy`）
   *   · 一张卡里**每一行**都被关着（= 整张卡属于另一支 / 另一种模式）→ 整张收起来。
   *     单行的关着仍然是「看得见、改不动」—— 藏起来的后果是用户以为这个参数不存在。
   *
   * 搜索态（mode='search'）永远不藏：那份列表是他自己搜出来的，
   * 「命中就该看见（哪怕此刻被关着）」是定下的规矩。
   */
  if (mode === 'normal' && fields.length > 0 && fields.every((d) => blockedByOf(d.key) !== null)) {
    return null
  }

  /*
   * 塔地图槽位：这一卡的**全部行**（含 X/Y 坐标）都进地图右侧的参数列，地图本体由
   * render(towerX, towerY, rows) 组装 —— 行就是普通的 ParamRow。
   * 搜索态不拆：X/Y 照旧当普通行走整卡流程。
   */
  const towerActive = tower !== undefined && mode === 'normal'
  const towerX = towerActive ? fields.find((d) => d.key === tower.xKey) : undefined
  const towerY = towerActive ? fields.find((d) => d.key === tower.yKey) : undefined
  const tree = towerX !== undefined && towerY !== undefined ? [] : buildTree(fields, condOn)

  /*
   * 整卡都是 G-code 编辑器且不止一块（「切换」分类那两张：装载 / 卸载胶箱）
   * —— 宽窗口下并排摆。宽窄交给容器查询（见 module.css 的 @container）。
   */
  const gcodePair = fields.length > 1 && fields.every((d) => d.field.control === 'gcode')

  /*
   * 条件小签的文字：「外围结构 = 斜肋外墙」。条件参数的 label + 符号 +
   * 值的中文说法（choice 翻成选项名、bool 翻成 开启/关闭 —— valueText 一套账）。
   * 条件参数不在卡里（理论漏网）就退回 key 原词，不编。
   */
  const defByKey = new Map(fields.map((d) => [d.key, d]))
  const TAG_OP: Record<'eq' | 'neq' | 'gt', string> = { eq: '=', neq: '≠', gt: '>' }
  const condTagOf = (cond: NonNullable<ShowWhen>): string => {
    const def = defByKey.get(cond.key)
    if (def === undefined) return `${cond.key} ${TAG_OP[cond.op]} ${String(cond.value)}`
    return `${def.label} ${TAG_OP[cond.op]} ${valueText(def, cond.value)}`
  }

  /*
   * 递归摆树。`sep` = 上面画不画分隔线：排在一个**先行内容**后面才有。
   */
  const renderList = (items: TreeNode[]): ReactNode[] =>
    items.map((node, i) => {
      if (node.kind === 'row') {
        return node.def.field.control === 'gcode' && renderBlock !== undefined ? (
          <div key={node.def.key} className={s.block}>
            {renderBlock(node.def)}
          </div>
        ) : (
          <ParamRow
            key={node.def.key}
            def={node.def}
            value={valueOf(node.def.key)}
            savedValue={savedValueOf(node.def.key)}
            factoryValue={factoryOf(node.def.key)}
            dirty={dirtyOf(node.def.key)}
            blockedBy={blockedByOf(node.def.key)}
            sep={i > 0}
            expanded={expandedKey === node.def.key}
            onToggleExpand={onToggleExpand}
            onEdit={onEdit}
            onRevertToSaved={onRevertToSaved}
            query={query}
          />
        )
      }

      /*
       * 一支 = 一张手风琴卡：签 + 常驻卡头 + 卡头下面的参数行。
       *   .branch  支的定位架，**签挂在这里** —— 在卡外、在任何 overflow:hidden 之外，
       *            收拢摊开一像素不动；卡的上缘同样钉死，变的只有卡身高度。
       *   .grp     卡本体：收拢 = 虚线灰、只剩卡头；摊开 = 实线白、参数行在头下面。
       *   .fold    常驻卡头（▸ N 项参数 · 行名…），收拢时点了 = 把条件参数改成这一支的
       *            值（写值闸在后头：弃用支点了弹回来说一句人话）。
       *   .accBody grid-rows 壳：参数行在头下面 0fr ↔ 1fr，240ms 收旧摊新。
       * 搜索态全支摊开、卡头照画但点不动。
       */
      const open = mode === 'search' || node.on
      const tagText = condTagOf(node.cond)
      const condDef = defByKey.get(node.cond.key)
      const valueLabel =
        condDef !== undefined ? valueText(condDef, node.cond.value) : String(node.cond.value)
      const directRows = node.items.filter((it): it is RowNode => it.kind === 'row')
      const switchable =
        node.cond.op === 'eq' &&
        condDef !== undefined &&
        condDef.meta?.deprecated !== true &&
        blockedByOf(node.cond.key) === null
      return (
        <div
          key={`branch-${node.cond.key}-${node.cond.op}-${node.cond.value}`}
          className={s.tree}
          data-seam={expandedKey === node.cond.key || undefined}
        >
          <div className={s.branch}>
            <span className={s.tag}>{tagText}</span>
            <div className={open ? s.grp : `${s.grp} ${s.grpOff}`}>
              <button
                type="button"
                className={s.fold}
                aria-expanded={open}
                disabled={!switchable || open}
                title={!open && switchable ? `切换到「${valueLabel}」` : undefined}
                onClick={() => onEdit(node.cond.key, node.cond.value)}
              >
                <span className={s.foldMark} aria-hidden>
                  ▸
                </span>
                <b className={s.foldCount}>{directRows.length} 项参数</b>
                <span className={s.foldHint}>{directRows.map((r) => r.def.label).join(' / ')}</span>
              </button>
              <div className={open ? `${s.accBody} ${s.accBodyOpen}` : s.accBody}>
                <div className={s.accBodyIn}>{renderList(node.items)}</div>
              </div>
            </div>
          </div>
        </div>
      )
    })

  return (
    <section className={s.card} data-gcode-pair={gcodePair || undefined} aria-label={title}>
      <h3 className={s.title}>
        <Highlight text={title} query={query} />
        {subtitle !== undefined && (
          <span className={s.subtitle}>
            <Highlight text={subtitle} query={query} />
          </span>
        )}
      </h3>

      <div className={s.body}>
        {towerActive && towerX !== undefined && towerY !== undefined && (
          <div className={s.tower}>
            {tower.render(
              towerX,
              towerY,
              /*
               * 地图右侧那一列：全部行平铺（不造树）—— 画布是给「调位置」用的，
               * 行在这只当参数清单；sep 照给（前一个行后面画线）。
               */
              <>
                {renderList(
                  fields.map((d) => ({ kind: 'row' as const, def: d, branches: [] })),
                )}
              </>,
            )}
          </div>
        )}
        {renderList(tree)}
      </div>
    </section>
  )
}
