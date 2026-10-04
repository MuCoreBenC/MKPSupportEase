/*
 * 一格的值编辑器（C14 移植）。
 *
 * **分派不在这里写。** 「字段类型 → 画成什么控件」全仓库只允许在
 * `components/field/FieldControl` 写一次（#33 的教训：写两次就会长出两套手感）。
 * 这里只做翻译与提交：
 *
 *   ParamView（后端的参数表） → FieldSchema（共用件的字段形状）
 *   TOML 原文进、原文出；类型转换走写值那一条路（页面的 `write`，
 *   按 `valueType` 归位后才进 Patch）
 *
 * form 两种场合（决定权在 FieldControl）：
 *   row   单版本模式的行、右栏各版本取值 —— 摆得开，枚举直接摊开
 *   cell  版本对照的格子 —— 摆不开，枚举收进下拉（P3 随矩阵回来）
 */

import { FieldControl } from '../components/field'
import type { FieldSchema } from '../components/field'
import type { Cell, ParamView } from '../api'
import GcodeEditor from '../c14/GcodeEditor'
import s from '../c14.module.css'

interface Props {
  param: ParamView
  cell: Cell
  form?: 'row' | 'cell'
  /** 前置条件不成立 / 已弃用时的只读态：看得见、改不动 */
  disabled?: boolean
  /** 写值。**唯一入口** —— 弃用闸与类型归位都在那条路上 */
  onWrite: (next: string) => void
  /**
   * 多行 G-code：**打开模态框**那一枚按钮的动作。
   *
   * 由宿主给（参数台那一页持有模态框），因为模态框得挂在定位祖先上 ——
   * 挂在行里会盖住一行。不给就**不画那枚按钮**。
   */
  onOpenGcode?: () => void
}

export default function CellEditor({
  param,
  cell,
  form = 'cell',
  disabled = false,
  onWrite,
  onOpenGcode,
}: Props) {
  /*
   * 点框内任何一处都进编辑态（C14 第十四轮）：`row` 档开。`cell` 档不开：
   * 那里的「进编辑态」是整格的事，由矩阵自己管（P3）。
   */
  const focusOnBoxClick = form === 'row'

  /*
   * 哪几档选项已经弃用（C14 §五）：**判据在后端**（`ChoiceView.deprecated`，
   * 上游标的与推出来的并集），这里只把它转成共用件认的 `FieldOption.deprecated`
   * —— 分段选择器与下拉都给那一档划线，两处不用各写一遍。
   */
  /*
   * 控件分派（2026-10-03 收紧）：`uiComponent` 说了算，但**值类型得配得上** ——
   *
   *   switch     只认 bool。float 参数挂开关是假控件：点了写 true/false，
   *              归位成 NaN，行上永远不亮（作者实测「它实际上根本没有换一套」）。
   *              类型不配就当没有这个字段，落到按类型画的那一档。
   *   segmented  摊开的分段、select 下拉 —— 只认 string 且有可选项；
   *              谁是分段谁是下拉由 `choiceLayout` 一直传到共用件（此前两者
   *              画成一样，「改成下拉也不下拉」就是从这里来的）。
   *   number     只认 float/int。
   *   text       兜底（自由文本、以及一切老数据里配不上号的组合）。
   *
   * 编辑定义那边的后端校验（`registry::set_param_meta`）用同一张表拦新数据 ——
   * 这里兜底是因为盘上可能还躺着历史形状。
   */
  const isNum = param.valueType === 'float' || param.valueType === 'int'
  const hasChoices = param.valueType === 'string' && param.choices.length > 0
  const field: FieldSchema = {
    key: param.key,
    label: param.label,
    /*
     * 控件由 `uiComponent` 说了算 —— **有 choices 不能把 number 顶成枚举**。
     *
     * `choices` 是取值域**只有 string 这一档**（`validate.rs` 的
     * `value_type == "string"` 同一道门）；float 参数身上挂着的几条是
     * 「预设档」，不是逼人选的是非题。`wiping.ironing_coverage_threshold`
     * 就是这种：unit % · 0~100 · 步进 1，画出来必须是一个能填的百分比框，
     * 而不是三选一（客户端那一版同一口径：控件原词 `uiComponent` 说了算，
     * 不拿「有没有 choices」反推）。
     */
    control:
      param.uiComponent === 'gcode'
        ? 'gcode'
        : param.uiComponent === 'switch' && param.valueType === 'bool'
          ? 'switch'
          : hasChoices && (param.uiComponent === 'segmented' || param.uiComponent === 'select')
            ? 'choice'
            : param.uiComponent === 'number' && isNum
              ? 'number'
              : 'text',
    choiceLayout:
      param.uiComponent === 'segmented' ? 'inline' : param.uiComponent === 'select' ? 'dropdown' : undefined,
    unit: param.unit ?? undefined,
    min: param.min ?? undefined,
    max: param.max ?? undefined,
    step: param.step ?? undefined,
    choices: param.choices.map((o) => ({
      value: String(o.value),
      label: o.label,
      deprecated: o.deprecated || undefined,
    })),
  }

  const before = String(cell.raw ?? '')

  /*
   * 多行 G-code（C14 第十八轮）。
   *
   * 角按钮是「打开编辑器」（模态框），框本身照旧就地可改（每一行都能直接改
   * 是这一稿的规矩，不因为多了一个模态框就变）。`GcodeEditor` 给它上色
   * （G/M/`L801` 那类指令一个色、注释一个色、`[AUTO]` 一个色），并关掉
   * 浏览器的拼写检查 —— 那条红波浪线就是它画的。
   *
   * 行上不给行号槽：这个框只有 172px 宽，两列一放就看不见代码了。
   */
  if (param.uiComponent === 'gcode') {
    return (
      <span className={s.pGwrap}>
        <GcodeEditor value={before} onChange={onWrite} disabled={disabled} label={param.label} />
        {onOpenGcode && (
          <button
            type="button"
            className={s.pGbig}
            /* 「打开一个新窗口」那个记号 —— 它开的是模态框 */
            aria-haspopup="dialog"
            title="打开编辑器 —— 全屏看这一段（十几行一次读完）"
            onClick={onOpenGcode}
          >
            <svg
              viewBox="0 0 16 16"
              width="12"
              height="12"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
              strokeLinejoin="round"
              aria-hidden
            >
              <path d="M9.5 2.5h4v4" />
              <path d="M13.5 2.5 8.2 7.8" />
              <path d="M12.5 9.8v2.7a1.5 1.5 0 0 1-1.5 1.5H3.5A1.5 1.5 0 0 1 2 12.5V5a1.5 1.5 0 0 1 1.5-1.5h2.7" />
            </svg>
          </button>
        )}
      </span>
    )
  }

  /*
   * 数字统一裹一个容器，两种场合两套宽度规矩：
   *
   *   row   固定 132px —— 框随内容伸缩的话，同一页里一行一个宽度
   *         （「数字长一点框就长一点」看着像两套组件，作者）。固定后中间列表、
   *         右栏编辑区的步进器完全等宽。
   *   cell  吃满整格 —— 这一层必须是块级，里面的共用件（inline-flex 根）
   *         才撑得开。
   *
   * **只是「数字控件」的容器**（C14 第十八轮修的）：`uiComponent === 'number'`
   * 不等于画出来是步进器 —— 判据必须落在**真正画出来的那个控件**上
   * （`field.control`），照 `uiComponent` 认会把这层套给一个不是步进器的控件，
   * 按 132px 钉死、尾巴被裁掉。
   */
  if (field.control === 'number') {
    return (
      <span className={form === 'row' ? s.edNum : s.edCell}>
        <FieldControl
          field={field}
          raw={before}
          onChange={onWrite}
          form={form}
          disabled={disabled}
          focusOnBoxClick={focusOnBoxClick}
        />
      </span>
    )
  }

  return (
    <FieldControl
      field={field}
      raw={before}
      onChange={onWrite}
      form={form}
      disabled={disabled}
      focusOnBoxClick={focusOnBoxClick}
    />
  )
}
