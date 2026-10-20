/*
 * 一行参数。布局照用户端早先那一版，控件仍然交 `FieldControl` 分派 ——
 * 这个文件只管布局（名称 + ⓘ / 还原 chip / 控件靠右对齐到同一条线）。
 *
 * # 修掉的那个 bug：chip 指向「已保存值」
 *
 * 旧版的 chip 写的是 `↩ {valueText(field, field.value)}`，而 `field.value` 是注册表里的出厂默认。
 * **保存之后这个 chip 就在撒谎**：它说「点我退回 -0.9」，而磁盘上其实已经是 -0.89 了。
 * 保存确认对话框栽过同一个坑（旧值拿了出厂值），后来是靠 `savedValueOf` 修的，这里同解。
 *
 * 顺带把文案也改了：chip 上只写一个数字说不清它是哪一个数 ——
 * 出厂值挪进展开的行详情里（那里两个值并排，「我改了多少」与「包里是多少」一眼可比）。
 *
 * 作者又把它收短一次：**chip 上不再写「已保存的」四个字**，只留 `↩ 255`。
 * 他圈着那个胶囊的原话是「不要显示已保存的这些字也不要显示区间」—— 同一句话里的
 * 「区间」指的是 `0-255`（注册表把 min/max 塞进了 unit 位，见 `useParams` 的
 * `unitOf`：那一处修完，这个胶囊、行里的步进器、保存确认三处一起干净）。
 * 那四个字挪进 `title`：胶囊本身就是「退回上一个值」这个动作，行里只有一个数时
 * 不必再自我介绍一遍；悬停仍写全「还原为已保存的 255（数据包里是 255）」。
 *
 * # 被 showWhen 关掉的行：看得见、改不动
 *
 * 不是藏起来。藏起来的后果是用户以为这个参数不存在，然后去别处找。
 * 名称下面写清是**哪个条件参数**控制的（「需先把 外围结构 设为 开启」），照旧版的做法。
 *
 * # 已弃用的行（2026-10-02，作者裁决）：第三档，不是 blocked
 *
 * `deprecated` 表示该字段已退出正常编辑 / 产物生成，但仍是已知参数 —— 参数页**照常显示**
 * 它的历史状态（**显示 ≠ 可编辑 ≠ 会进入新产物**：行照列，控件只读，写值闸拒绝，
 * `getMachineParams` 也不下发给新 TOML）。它**不并进 blocked**：blocked 说「换个条件就能用」，
 * 对弃用的行说那句条件是假话 —— 它正在退场，跟条件满不满足无关。所以这一档自己长相：
 *
 *   · 行名划一条**红线** + 字压暗
 *   · 控件左边一枚「已弃用」徽章（永远横着）
 *   · 控件照旧摆着但改不动（disabled）—— 与 blocked 的置灰同一条路
 *   · 展开的详情第一格是「状态」，让「它到底是什么」说在前头
 *
 * 判据只有一处：`def.deprecated`（定义通道带出来的原词），落到 `data-dep`。
 */

import { useCallback, useState } from 'react'
import type { ParamSyncEntry } from '../../api'
import FieldControl from '../../components/field/FieldControl'
import Highlight from './highlight'
import { valueText } from './useParams'
import type { BlockedBy, ParamDef } from './useParams'
import s from './ParamRow.module.css'

interface Props {
  def: ParamDef
  /** 显示值：draft ?? saved */
  value: string
  /** 已保存的那一层 —— 还原 chip 指向的就是它 */
  savedValue: string
  /**
   * **上一次保存之前的值**（2026-10-21，作者：「保存之后还是那样显示，
   * 还需要一个我的旧值；不只是更新才有，所有的都能记录下来」）。
   * `null` = 没记录过（从没保存过它的改动 / 清过历史）—— 那就什么也不画。
   */
  prevSavedValue?: string | null
  /** 出厂值 —— 只在展开的行详情里显示 */
  factoryValue: string
  dirty: boolean
  /**
   * 这一项上的**云端更新账**（我 / 云端旧值 / 云端新值）。
   *
   * `undefined` = 这一段没有"云端更新"这回事（云端线那一份 / 认不出是哪一版云端）——
   * 那时行上什么都不画，与以前长得一模一样。
   */
  sync?: ParamSyncEntry
  /**
   * 这一项在**草稿层**表过什么态（2026-10-20）：
   *
   *   `adopt` 已经「用新值」了（草稿值 = 云端新值，还没保存）
   *   `hold`  已经「保持原样」了（值没动）
   *   `null`  还没表态 —— 右边给那两颗按钮
   *
   * 与 `sync.decided` 分开：那个是**上一次保存**落下的账，这个是这一屏还没保存的那一笔。
   */
  syncDraft?: 'adopt' | 'hold' | null
  /** 这一项「用新值」；不传就画不出那两颗按钮（比如只读视图） */
  onAdoptSync?: (key: string) => void
  /** 这一项「保持原样」 */
  onHoldSync?: (key: string) => void
  /** 详情右栏「用回我的值」：撤掉这一项的采用 / 保持，退回已保存值 */
  onUseMyValue?: (key: string) => void
  /** 云端当前版的字节还没取回来 —— 那时只画"云端已更新"，不画差异 */
  syncBusy?: boolean
  /** 被条件参数关掉：看得见、改不动 */
  blockedBy?: BlockedBy | null
  /**
   * 收拢支里的行**不画**「需先让…」副标题（2026-10-21）：收拢的支本来就收进 0fr，
   * 但收起动画那 240ms 里行还看得见 —— 那一刻副标题会瞬间多一行闪出来（作者截图）。
   * 收拢的支里行本来不可见，这一格藏掉没有别的代价。
   */
  suppressNote?: boolean
  /**
   * 上面要不要那条分隔线。原来是 `.wrap + .wrap` 相邻选择器画的，但树把行隔进了
   * 各自的分支容器里，相邻链到处断；改成由 ParamCard 摆行时显式给
   * （排在前一个**行**后面才有线，分支容器的边界用留白）。
   */
  sep?: boolean
  expanded?: boolean
  onToggleExpand?: (key: string) => void
  onEdit: (key: string, next: string) => void
  /** 退回已保存值 */
  onRevertToSaved: (key: string) => void
  /**
   * **只读**（2026-10-16 的两态）：控件整个替成**纯文本值**（`valueText` 的那套翻译，
   * 与参数对比台的只读值同一长相）——「看得见、改不动」的第三种画法，
   * 与 blocked / 弃用那两档**不叠用**（那两档画的是灰控件，这里是文本）。
   * 还原 chip 也不摆（chip 是个写入口）。
   */
  readonly?: boolean
  /**
   * 搜索词：行名与展开后那几格里命中的字加底。
   * 不搜索时是空串 —— `Highlight` 收到空串原样返回，不白包一层。
   */
  query?: string
}

/**
 * 值类型的中文说法。原来这一页没有这一栏 —— 客户端拿到的包里既然带着
 * `valueType`，就该把它说出来：**控件说的是「画成什么」，类型说的是「值是什么」**。
 * 括号里的英文是包里那个原词，作者点名要的就是它（「string，bool 之类的」）。
 */
const TYPE_TEXT: Record<'float' | 'int' | 'bool' | 'string', string> = {
  float: '浮点小数',
  int: '整数',
  bool: '布尔（开 / 关）',
  string: '文本',
}

const OP_TEXT: Record<'eq' | 'neq' | 'gt', string> = { eq: '=', neq: '≠', gt: '>' }

function rangeText(def: ParamDef): string {
  const f = def.field
  if (f.control !== 'number') return '—'
  const unit = f.unit !== undefined ? ` ${f.unit}` : ''
  const stepPart = f.step !== undefined ? `步长 ${f.step}${unit}` : ''
  /* 注册表里没写范围的字段不要编一个「不限 ~ 不限」出来 */
  const bounded = f.min !== undefined || f.max !== undefined
  if (!bounded) return stepPart === '' ? '没有写明范围' : `没有写明范围 · ${stepPart}`
  const lo = f.min === undefined ? '不限' : String(f.min)
  const hi = f.max === undefined ? '不限' : String(f.max)
  return stepPart === '' ? `${lo} ~ ${hi}${unit}` : `${lo} ~ ${hi}${unit} · ${stepPart}`
}

function choicesText(def: ParamDef): string {
  if (def.field.control === 'switch') return '关闭 / 开启'
  if (def.field.control === 'choice') {
    return (def.field.choices ?? []).map((c) => c.label).join(' / ')
  }
  return rangeText(def)
}

export default function ParamRow({
  def,
  value,
  savedValue,
  prevSavedValue = null,
  factoryValue,
  dirty,
  sync,
  syncDraft = null,
  onAdoptSync,
  onHoldSync,
  onUseMyValue,
  syncBusy = false,
  blockedBy = null,
  suppressNote = false,
  sep = false,
  expanded = false,
  onToggleExpand,
  onEdit,
  onRevertToSaved,
  readonly = false,
  query = '',
}: Props) {
  const blocked = blockedBy !== null
  /* 已弃用：第三档，不并进 blocked —— 见文件头。判据只有这一处（定义通道带的原词） */
  const dep = def.deprecated
  /* 改不动：条件关着，或已弃用 —— 两者控件都是 disabled，但文案与长相分开 */
  const locked = blocked || dep
  const savedText = valueText(def, savedValue)
  const factoryText = valueText(def, factoryValue)

  /*
   * **打字不合法也要有「还原」出口**（2026-10-20，作者：「这个时候就可以显示
   * 恢复的了呀」）：失焦闸把人困在框里之后，那个状态本来只有按 Esc 一条暗路。
   * 未提交的打字不进草稿（`dirty` 不知道它），所以这一层自己记：
   * 数字控件报"正在打一个不合法的值"时，chip 照常显示。
   * 点它 = 草稿有改动就退回已保存值（老路），框里没提交的原文一并清掉
   * （`resetSignal` 那一拍 —— 宿主"还原"一个没进草稿的值是 noop，`value` 不变）。
   */
  const [badTyping, setBadTyping] = useState(false)
  const [resetNonce, setResetNonce] = useState(0)
  const onInvalidTyping = useCallback((v: boolean) => setBadTyping(v), [])

  /*
   * 云端更新：三态都摊在这一行的名字下面（**不另开一块** —— 它说的就是这一项的事）。
   *
   *   还没表态   云端已更新 · 云端 18.6 → 19.0          [用新值] [保持原样]
   *   已采用     已采用云端新值 · 我的 18.6 → 云端 19.0   （点这一行展开，右栏能「用回我的值」）
   *   已保持     保持原样 · 云端 18.6 → 19.0            [改用新值]
   *   保存过了   已采用云端新值 / 保持原样（一枚淡徽章，说明云端这一版已经处理过）
   */
  const syncPending = sync?.pending === true
  /** 草稿层的表态只对**还在待处理**的项有意义（保存过之后归 `sync.decided` 管） */
  const syncDraftKind = syncPending ? syncDraft : null
  const syncDecided = sync?.decided ?? null
  /** 「云端 18.6 → 19.0」 */
  const diffText =
    sync !== undefined && sync.baselineOld !== null && sync.cloudNew !== null
      ? `云端 ${valueText(def, sync.baselineOld)} → ${valueText(def, sync.cloudNew)}`
      : null
  /*
   * 「我的 18.6 → 云端 19.0」—— 采用之后要说得清"换掉的是我哪个值"。
   * **保存之后**（decided）`sync.local` 已经是新值了，"我的哪个值"就换
   * **上一次保存之前的那个**（`prevSavedValue`）—— 作者 2026-10-21：
   * 「保存之后还是那样显示的，不是这样都变成新的」。
   */
  const prevOrLocal = prevSavedValue ?? sync?.local ?? null
  const mineToCloud =
    sync !== undefined && prevOrLocal !== null && sync.cloudNew !== null
      ? `我的 ${valueText(def, prevOrLocal)} → 云端 ${valueText(def, sync.cloudNew)}`
      : diffText

  return (
    <div
      className={expanded ? s.wrapOpen : s.wrap}
      data-key={def.key}
      /*
       * 展开态的普通属性钩子:ParamCard 的 `.sub` 要用 `[data-open='true'] + .sub`
       * 接住「条件行展开 → 受控参数块连同白缝一起并入打开态」(两个 module 的 class
       * 各自哈希,跨 module 只能走属性)。
       */
      data-open={expanded || undefined}
      /* 分隔线 + 改不动的压淡；弃用划线走 data-dep（画法见 module.css） */
      data-sep={sep || undefined}
      data-off={blocked || undefined}
      data-dep={dep || undefined}
    >
      <div className={s.row}>
        <button
          type="button"
          className={s.left}
          aria-expanded={expanded}
          /*
           * 行名可以拖选（2026-10-20 作者：「这个标题我要可以选」）—— 样式见 module.css。
           *
           * 拖选的松手会落在这个按钮上，不能顺手把行收起来：**选区还在按钮里
           * （anchorNode 在按钮内部）就不切展开态**。点在别处 / 没有选区时照旧开合。
           */
          onClick={(e) => {
            const sel = window.getSelection()
            if (
              sel !== null &&
              !sel.isCollapsed &&
              sel.anchorNode !== null &&
              e.currentTarget.contains(sel.anchorNode)
            ) {
              return
            }
            onToggleExpand?.(def.key)
          }}
        >
          <span className={s.labelWrap}>
            <span className={s.head}>
              <span className={s.label}>
                <Highlight text={def.label} query={query} />
              </span>
              {/*
               * `?` 是行名的后缀，紧贴名字（2026-10-21 作者：「跟着标题后面，可以适当缩小」）；
               * 悬停（`title`）说的是**这一项是什么**（注册表的 desc），不是「点它展开」——
               * 作者点名要的就是那句说明。说明为空时退回一句「点这一行看详情」，不摆空泡。
               */}
              <span
                className={s.info}
                aria-hidden
                title={def.desc === '' ? '点这一行看参数详情' : def.desc}
              >
                ?
              </span>
            </span>
            {/*
             * 那句橙字只给「换个条件就能用」的行。弃用的行说它就是假话 —— 它正在退场，
             * 跟条件满不满足无关，原因由徽章说。
             */}
            {blocked && !dep && !suppressNote && (
              <span className={s.note}>
                需先让「{blockedBy.label}」{blockedBy.need}
              </span>
            )}
            {/*
             * 云端更新那两行**只放文字**：这一段整个包在一个 `<button>` 里
             * （点它展开行详情），里面再嵌按钮就是非法结构 —— 两个动作住右边那一格。
             */}
            {syncPending && syncDraftKind === 'adopt' && (
              <span className={s.syncNote}>
                <span className={s.syncBadgeAdopted}>已采用云端新值</span>
                {mineToCloud !== null && <span className={s.syncDiff}>{mineToCloud}</span>}
              </span>
            )}
            {syncPending && syncDraftKind === 'hold' && (
              <span className={s.syncNote}>
                <span className={s.syncBadgeHold}>保持原样</span>
                {diffText !== null && <span className={s.syncDiff}>{diffText}</span>}
              </span>
            )}
            {syncPending && syncDraftKind === null && (
              <span className={s.syncNote}>
                <span className={s.syncBadge}>云端已更新</span>
                {diffText !== null && <span className={s.syncDiff}>{diffText}</span>}
              </span>
            )}
            {!syncPending && syncDecided !== null && (
              <span className={s.syncDecided}>
                {syncDecided === 'adopt' ? '已采用云端新值' : '保持原样'}
                {/* 保存过的那一版也把数字留着（2026-10-21）：
                    采用 =「我的旧值 → 云端新值」，保持 =「云端旧值 → 云端新值」 */}
                {syncDecided === 'adopt'
                  ? mineToCloud !== null && <span className={s.syncDiff}>{mineToCloud}</span>
                  : diffText !== null && <span className={s.syncDiff}>{diffText}</span>}
              </span>
            )}
          </span>
        </button>

        <span className={s.right}>
          {/*
           * 云端更新那两个动作。**只有待处理的项才画它们** —— 处理过的项显示一枚淡徽章
           * （在上面名字那一格），不再给按钮：已经处理完的事不该留着可点的入口。
           *
           * 「用新值」在云端当前版字节还没取回来时是灰的（新值不知道是哪来的，点下去
           * 只会得到一个错），原因写在 title 里 —— 灰一颗按钮不说为什么，用户只会以为坏了。
           */}
          {syncPending && syncDraftKind === null && onAdoptSync !== undefined && (
            <button
              type="button"
              className={s.syncBtn}
              disabled={syncBusy || sync?.cloudNew === null}
              title={
                sync?.cloudNew === null
                  ? '云端当前最新版还没取回来 —— 取回来之后就能看到新值、也能采用它'
                  : `采用云端新值：把这一项写成 ${diffText ?? '云端当前值'}（先写进草稿，保存才落盘）`
              }
              onClick={() => onAdoptSync(def.key)}
            >
              用新值
            </button>
          )}
          {syncPending && syncDraftKind === null && onHoldSync !== undefined && (
            <button
              type="button"
              className={s.syncBtnGhost}
              disabled={syncBusy}
              title={`保持原样：这一项写着的 ${savedText} 一个字不动（保存时一并记下"这一版不跟"）`}
              onClick={() => onHoldSync(def.key)}
            >
              保持原样
            </button>
          )}
          {/* 已保持还想改用云端新值：给一条回头路，不用先撤销再重来 */}
          {syncPending && syncDraftKind === 'hold' && onAdoptSync !== undefined && (
            <button
              type="button"
              className={s.syncBtn}
              disabled={syncBusy || sync?.cloudNew === null}
              title="改用云端新值（先写进草稿，保存才落盘）"
              onClick={() => onAdoptSync(def.key)}
            >
              改用新值
            </button>
          )}
          {/* 弃用的行不许写新值 → 不摆「还原」chip（它的值本来就等于已保存值/无值）。
              打字不合法（未提交）时也摆 —— 那是失焦闸困住之后的可见出口（见上面 badTyping 注） */}
          {(dirty || badTyping) && !locked && !readonly && (
            <button
              type="button"
              className={s.restore}
              title={`还原为已保存的 ${savedText}（数据包里是 ${factoryText}）`}
              onClick={() => {
                onRevertToSaved(def.key)
                setResetNonce((n) => n + 1)
                setBadTyping(false)
              }}
            >
              ↩ {savedText}
            </button>
          )}
          {dep && (
            <span className={s.depBadge} title="该参数已弃用，不再写入新预设">
              已弃用
            </span>
          )}
          {readonly ? (
            /* 只读态：纯文本值（编辑模式之外的那一态，见 Props.readonly） */
            <span className={s.roVal}>{valueText(def, value)}</span>
          ) : def.field.control === 'number' ? (
            /*
             * 数字步进器包一层同款的 `.edNum`（作者：「我专门调整过的，应该可以复用」）：
             *   132px 定宽 —— 箭头永远靠右落在同一条竖线上，不跟数值长短左右跳；
             *   聚焦（编辑态）数值靠左 —— 作者原话「点击它编辑的时候，
             *   文字也是靠左的」；
             *   focusOnBoxClick —— 点框内任何一处（值、单位、框沿）都进编辑态
             *   （共用 NumberField 的既有开关，行内那一档开着，默认关）。
             * 其余控件类型不包：它们有自己的稳定宽度行为。
             */
            <span className={s.edNum}>
              <FieldControl
                field={def.field}
                raw={value}
                form="row"
                disabled={locked}
                focusOnBoxClick
                onInvalidTyping={onInvalidTyping}
                resetSignal={resetNonce}
                onChange={(next) => onEdit(def.key, next)}
              />
            </span>
          ) : (
            <FieldControl
              field={def.field}
              raw={value}
              form="row"
              disabled={locked}
              onInvalidTyping={onInvalidTyping}
              resetSignal={resetNonce}
              onChange={(next) => onEdit(def.key, next)}
            />
          )}
        </span>
      </div>

      {expanded && (
        /*
         * 展开后这七格里，命中的字一样加底（作者的：「字段啊什么的，
         * 展开的时候也一样能看到高亮」）。搜「t」时命中的是「字段名 offset」那一格 ——
         * 不展开就看不见它为什么被搜出来；「如果是数字也可以」落在已保存值 / 出厂值 /
         * 取值范围上，走的是同一段代码。
         */
        <div className={s.detailWrap}>
          <dl className={s.detail}>
          {/*
            弃用的行，展开后第一格就说清「它到底是什么」：上游标了弃用 —— 显示但只读，
            不再写入新预设。用户看到「为什么这个改不动」的答案在第一步。
          */}
          {dep && (
            <>
              <dt className={s.dt}>状态</dt>
              <dd className={s.ddDep}>
                已弃用 —— 该参数不再写入新预设，此处仅供查看（上游注册表已标记）
              </dd>
            </>
          )}
          <dt className={s.dt}>说明</dt>
          <dd className={s.dd}>
            <Highlight
              text={def.desc === '' ? '注册表里没写说明' : def.desc}
              query={query}
            />
          </dd>
          <dt className={s.dt}>取值范围</dt>
          <dd className={s.dd}>
            <Highlight text={choicesText(def)} query={query} />
          </dd>
          {/*
            「值类型」—— 作者点名要的那一栏：「string，bool 之类的，都在抽屉里面，
            显示的」。它来自**运行时 catalog 的字段定义**（`ParamDef.valueType`），不是客户端猜的：
            控件（开关 / 分段 / 步进器）说的是「画成什么」，类型说的是「值是什么」，
            两者不是一回事（开关是 bool，下拉是 string）。
          */}
          <dt className={s.dt}>类型</dt>
          <dd className={s.dd}>
            {TYPE_TEXT[def.meta?.valueType ?? 'string']}
            <span className={s.typeKey}> {def.meta?.valueType ?? 'string'}</span>
          </dd>
          {/* 条件：这条什么条件下才显示 —— 同样来自包里的 showWhen */}
          {def.meta?.showWhen !== undefined && (
            <>
              <dt className={s.dt}>显示条件</dt>
              <dd className={s.dd}>
                要「{def.meta.showWhen.key}」{OP_TEXT[def.meta.showWhen.op]}{' '}
                {String(def.meta.showWhen.value)} 才显示
              </dd>
            </>
          )}
          <dt className={s.dt}>字段名</dt>
          <dd className={s.ddKey}>
            <Highlight text={def.meta?.tomlKey ?? def.key} query={query} />
          </dd>
          <dt className={s.dt}>已保存值</dt>
          <dd className={s.dd}>
            <Highlight text={savedText} query={query} />
          </dd>
          {/* 上一次保存之前的值（2026-10-21）：所有参数都记，存过它的改动就能看到。
              标签要短 —— dt 列窄，「上一次保存前」「数据包里的值」都会挤成两行（作者 2026-10-21） */}
          {prevSavedValue !== null && prevSavedValue !== savedValue && (
            <>
              <dt className={s.dt}>上次保存前</dt>
              <dd className={s.dd}>
                <Highlight text={valueText(def, prevSavedValue)} query={query} />
              </dd>
            </>
          )}
          {/*
            这一格原来叫「出厂值」。这一版的值只有一层：**包里的那一份**（下载来的
            `values`）—— 客户端不做继承推导，包里也没有「出厂 / 机型 / 版本」这三层。
            所以标签照实改成「数据包里的值」：它和「已保存值」比，正好说明本机改没改过。
          */}
          <dt className={s.dt}>包里的值</dt>
          <dd className={s.dd}>
            <Highlight text={factoryText} query={query} />
          </dd>
          <dt className={s.dt}>作用域</dt>
          <dd className={s.dd}>
            <Highlight
              text={def.meta?.scope === 'machine_specific' ? '机型专属' : '全机型通用'}
              query={query}
            />
          </dd>
          </dl>

          {/*
           * **右栏：云端这一项的三方对照**（2026-10-20 作者：「你在右边这个有空白的地方啊，
           * 这刚好右边有空白地方可以显示」）。
           *
           * 左栏那七格说的是"这一项是什么"，与云端无关；采用 / 保持之后用户想知道的是
           * "我原来的值是什么、云端改成什么"—— 那些数字塞进左栏会把说明挤成两列小字，
           * 右栏这块空白正好。
           */}
          {sync !== undefined && (sync.pending || sync.decided !== null) && (
            <aside className={s.cloudPane}>
              <h5 className={s.cloudTitle}>云端更新</h5>
              <dl className={s.cloudList}>
                <dt>我的值</dt>
                <dd>
                  {sync.local === null ? '文件里没有这一项' : valueText(def, sync.local)}
                </dd>
                {/* 保存过后「我的值」已经是新值了 —— 旧值单摆一行，别让它消失（2026-10-21） */}
                {!sync.pending && prevSavedValue !== null && prevSavedValue !== sync.local && (
                  <>
                    <dt>保存前</dt>
                    <dd>{valueText(def, prevSavedValue)}</dd>
                  </>
                )}
                <dt>云端旧值</dt>
                <dd>{sync.baselineOld === null ? '—' : valueText(def, sync.baselineOld)}</dd>
                <dt>云端新值</dt>
                <dd>{sync.cloudNew === null ? '还没取回' : valueText(def, sync.cloudNew)}</dd>
              </dl>
              <p className={s.cloudState} data-state={syncDraftKind ?? sync.decided ?? 'pending'}>
                {syncDraftKind === 'adopt'
                  ? '这次改动里已采用云端新值（保存才落盘）'
                  : syncDraftKind === 'hold'
                    ? '这次改动里保持原样（保存时记下这一版不跟）'
                    : syncDraftKind === null && sync.pending
                      ? '还没选：用新值 / 保持原样'
                      : sync.decided === 'adopt'
                        ? '上一次保存已采用云端新值'
                        : '上一次保存保持了原样'}
              </p>
              {syncDraftKind === 'adopt' && onUseMyValue !== undefined && (
                <button
                  type="button"
                  className={s.cloudRevert}
                  title={`把这一项退回你原来的 ${sync.local === null ? '值' : valueText(def, sync.local)}`}
                  onClick={() => onUseMyValue(def.key)}
                >
                  用回我的值
                </button>
              )}
            </aside>
          )}
        </div>
      )}
    </div>
  )
}
