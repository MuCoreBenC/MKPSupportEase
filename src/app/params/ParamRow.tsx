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
 */

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
  /** 出厂值 —— 只在展开的行详情里显示 */
  factoryValue: string
  dirty: boolean
  /** 被条件参数关掉：看得见、改不动 */
  blockedBy?: BlockedBy | null
  expanded?: boolean
  onToggleExpand?: (key: string) => void
  onEdit: (key: string, next: string) => void
  /** 退回已保存值 */
  onRevertToSaved: (key: string) => void
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
  factoryValue,
  dirty,
  blockedBy = null,
  expanded = false,
  onToggleExpand,
  onEdit,
  onRevertToSaved,
  query = '',
}: Props) {
  const blocked = blockedBy !== null
  const savedText = valueText(def, savedValue)
  const factoryText = valueText(def, factoryValue)

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
    >
      <div className={s.row}>
        <button
          type="button"
          className={s.left}
          aria-expanded={expanded}
          onClick={() => onToggleExpand?.(def.key)}
        >
          <span className={s.labelWrap}>
            <span className={s.label}>
              <Highlight text={def.label} query={query} />
            </span>
            {blocked && (
              <span className={s.note}>
                需先让「{blockedBy.label}」{blockedBy.need}
              </span>
            )}
          </span>
          <span className={s.info} aria-hidden>
            ?
          </span>
        </button>

        <span className={s.right}>
          {dirty && !blocked && (
            <button
              type="button"
              className={s.restore}
              title={`还原为已保存的 ${savedText}（数据包里是 ${factoryText}）`}
              onClick={() => onRevertToSaved(def.key)}
            >
              ↩ {savedText}
            </button>
          )}
          {def.field.control === 'number' ? (
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
                disabled={blocked}
                focusOnBoxClick
                onChange={(next) => onEdit(def.key, next)}
              />
            </span>
          ) : (
            <FieldControl
              field={def.field}
              raw={value}
              form="row"
              disabled={blocked}
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
        <dl className={s.detail}>
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
            显示的」。它来自**下载来的包里**（`ClientFieldDef.valueType`），不是客户端猜的：
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
          {/*
            这一格原来叫「出厂值」。这一版的值只有一层：**包里的那一份**（下载来的
            `values`）—— 客户端不做继承推导，包里也没有「出厂 / 机型 / 版本」这三层。
            所以标签照实改成「数据包里的值」：它和「已保存值」比，正好说明本机改没改过。
          */}
          <dt className={s.dt}>数据包里的值</dt>
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
      )}
    </div>
  )
}
