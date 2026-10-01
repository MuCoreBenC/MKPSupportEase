/*
 * 一张参数卡。一个真 section = 一张卡。
 *
 * 标题读 `ParamSection.label`（中文名，'空间偏移' / '圆盘动作控制'），
 * 条数读 `ParamSection.count`。更早那一版的标题来自一份手抄的卡片切分表，
 * 这里换成注册表自带的分组名。
 *
 * `showWhen` 的处理：
 *   normal 模式  —— 有 showWhen 且不可见的行收进嵌套子卡、标为改不动；**整张卡都被关着
 *                   就整张收起来**（那种卡属于另一支 / 另一种模式）
 *   search 模式  —— 搜索结果：命中就该看见（哪怕条件参数此刻把它关着），
 *                   **条件行下面照样带出受控参数**（作者：「搜索到父参数
 *                   需要显示子参数」）。一条都没命中的 section 不出卡，那是页面筛的；
 *                   这里只决定卡里怎么摆；**永远不整卡收起**（见下面那段注释）
 *
 * 两个模式都走同一个 `split`（谁挂在谁下面只有一套判据），都带 `blockedBy`
 * （统一：跨卡的条件参数也要说清「为什么改不动」）。
 *
 * 词也统一了（作者：「不要叫他父级吧」）：**条件参数** = 它的值决定别人
 * 能不能用，**受控参数** = 被它管着的那些，关系叫**条件依赖**，行为叫**条件显隐**。
 *
 * `gcode` 字段走 `renderBlock` —— 多行文本塞不进一行，由调用方给整块编辑器。
 */

import type { ReactNode } from 'react'
import Highlight from './highlight'
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
  expandedKey?: string | null
  onToggleExpand?: (key: string) => void
  onEdit: (key: string, next: string) => void
  onRevertToSaved: (key: string) => void
  /** G-code 这种整块的控件由调用方画 */
  renderBlock?: (def: ParamDef) => ReactNode
  /**
   * 搜索词：卡片标题、分类小字与每一行（含展开的行详情）里命中的字加底。
   * 不搜索时是空串。
   */
  query?: string
}

type Block =
  | { kind: 'row'; def: ParamDef }
  /** 灰底子块：`parent` 是这一组共同的条件参数 key */
  | { kind: 'sub'; parent: string; defs: ParamDef[] }

/**
 * 把带条件的字段归到**它的条件行**下面的灰底子块里。
 *
 * # 判据不是「有没有 showWhen」，而是「条件参数在不在这张卡、且在我上面」
 *
 * 早先那一版只问前者，于是真注册表里那种「整张卡的字段都带 showWhen、条件参数在上一张卡」
 * 会渲染出**孤立的灰块** —— 灰底上方没有任何条件行，它就失去了唯一的含义
 * 「这几条是挂在上一行下面的」。实测「擦料」分类下有 9 块这样的灰底，
 * 每一块在 DOM 里都没有上一个兄弟。
 *
 * 作者的原话就是判据：「灰色背景是**跟随父参数**出现的，不会自己单独出现。」
 *
 * 于是：
 *   - 条件参数已在这张卡里出现过（`seen`）→ 进灰底子块，紧贴条件行
 *   - 条件参数在别的卡 / 在我下面 / 卡片第一行就是带条件的字段 → **白底普通行**，与兄弟行齐平
 *   - 连续两组不同条件参数 → 分两块灰底，不合并
 *
 * 「改不动」与灰底无关：那由 `blockedBy` 走 `ParamRow` 的置灰 + 「需先开启 X」。
 */
function split(fields: ParamDef[]): Block[] {
  const blocks: Block[] = []
  /* 这张卡里**已经出现过**的 key —— 「条件参数在我上面」只认这个 */
  const seen = new Set<string>()
  for (const def of fields) {
    const parent = def.meta?.showWhen?.key
    seen.add(def.key)
    if (parent === undefined || !seen.has(parent) || parent === def.key) {
      blocks.push({ kind: 'row', def })
      continue
    }
    const last = blocks[blocks.length - 1]
    if (last !== undefined && last.kind === 'sub' && last.parent === parent) last.defs.push(def)
    else blocks.push({ kind: 'sub', parent, defs: [def] })
  }
  return blocks
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
  expandedKey = null,
  onToggleExpand,
  onEdit,
  onRevertToSaved,
  renderBlock,
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
   *     他自己那台程序就是这么办的：「切换到圆盘擦拭的时候那些擦料塔的都隐藏」。
   *     单行的关着仍然是「看得见、改不动」—— 那句判据（早先那一版留下的话）没变：
   *     藏起来的后果是用户以为这个参数不存在。
   *
   * 搜索态（mode='search'）永远不藏：那份列表是他自己搜出来的，
   * 「命中就该看见（哪怕此刻被关着）」是定下的规矩。
   */
  if (mode === 'normal' && fields.length > 0 && fields.every((d) => blockedByOf(d.key) !== null)) {
    return null
  }

  /* 谁挂在谁下面只有一套判据（见 split）：两个模式都走它，搜索结果一样成组 */
  const blocks = split(fields)

  const row = (def: ParamDef) =>
    def.field.control === 'gcode' && renderBlock !== undefined ? (
      <div key={def.key} className={s.block}>
        {renderBlock(def)}
      </div>
    ) : (
      <ParamRow
        key={def.key}
        def={def}
        value={valueOf(def.key)}
        savedValue={savedValueOf(def.key)}
        factoryValue={factoryOf(def.key)}
        dirty={dirtyOf(def.key)}
        /* 一律带上：跨卡的条件参数也要说清「为什么改不动」 */
        blockedBy={blockedByOf(def.key)}
        expanded={expandedKey === def.key}
        onToggleExpand={onToggleExpand}
        onEdit={onEdit}
        onRevertToSaved={onRevertToSaved}
        query={query}
      />
    )

  return (
    <section className={s.card} aria-label={title}>
      <h3 className={s.title}>
        <Highlight text={title} query={query} />
        {subtitle !== undefined && (
          <span className={s.subtitle}>
            <Highlight text={subtitle} query={query} />
          </span>
        )}
      </h3>

      <div className={s.body}>
        {blocks.map((b, i) => {
          if (b.kind === 'row') return row(b.def)
          /* normal 模式：被 showWhen 关掉的受控参数也出现，但改不动 —— 不是藏起来。
             灰框本体在 .subWell（内层），.sub 容器只管定位 ——
             条件行展开时容器铺绿底当"缝"，灰框原样嵌在里面（见 module.css） */
          return (
            <div key={`sub-${i}`} className={s.sub}>
              <div className={s.subWell}>
                {b.defs.map((def) => (
                  <ParamRow
                    key={def.key}
                    def={def}
                    value={valueOf(def.key)}
                    savedValue={savedValueOf(def.key)}
                    factoryValue={factoryOf(def.key)}
                    dirty={dirtyOf(def.key)}
                    blockedBy={blockedByOf(def.key)}
                    expanded={expandedKey === def.key}
                    onToggleExpand={onToggleExpand}
                    onEdit={onEdit}
                    onRevertToSaved={onRevertToSaved}
                    query={query}
                  />
                ))}
              </div>
            </div>
          )
        })}
      </div>
    </section>
  )
}
