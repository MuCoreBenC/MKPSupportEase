/*
 * 两条分段控件（只剩一层）。
 *
 * ```
 * ┌ MKP 配置 │ 切片器配置 ┐   ┌ 本地 │ 云端 ┐
 * ```
 *
 * # 两个轴各管什么
 *
 *   左（类型）  `PresetFileInfo.kind` —— MKP 的涂胶预设 toml，还是切片器的工艺 profile。
 *               **它还决定表头有几列**（MKP 没有喷嘴层高，切片器没有版本）
 *   右（位置）  **两张互不相干的表**，不是同一批数据的筛选：本地的就是本地的，云端的就是
 *               云端的。一个官方文件下载之后两张表里都有，那是对的 —— 两张表各回答一个问题
 *
 * 上一版把位置这个轴压成了行内的一个状态角标，被否了。它是分类，不是状态。
 *
 * # 去掉多余的那一层外包
 *
 * 曾经是 `.segs > .seg > label` **三层**，`.seg` 自己还有边框、`label` 选中时也有边框 ——
 * 作者说「不喜欢这种大按钮套小按钮的感觉」，来源就是这两层框。
 * 校准页那套分段控件的写法只有两层：**一条 4% 的灰底槽 + 里面一块白**。
 * 所以这里直接返回两个 `.seg`，不再套 `.segs`——
 * 它们是工具条（`PagePresets` 的 `.toolbar`）的两个兄弟，中间隔一个 `.toolbar` 的 gap。
 *
 * 「本地 N · 云端 N」那两个数早就搬去工具条了（现在写的是「共 N 项」）：
 * `order` 只在同一个 flex 容器的兄弟之间生效，留在这里就只能排在搜索框前面。
 *
 * # 为什么用原生 radio
 *
 * 分段控件本质是一组单选。用 `<button>` 装出来的话，键盘的左右键、读屏的「第 2 项共 2 项」
 * 都得自己补；原生 radio 白送这些，只需要把圆点藏掉、把 label 画成胶囊。
 */

import {
  KIND_AXIS_TEXT,
  SCOPE_AXIS_TEXT,
  SCOPE_AXIS_WHY,
} from './presetTree'
import type { PresetKindAxis, PresetScopeAxis } from './presetTree'
import s from './PresetScopeBar.module.css'

interface Props {
  kind: PresetKindAxis
  scope: PresetScopeAxis
  onKind: (next: PresetKindAxis) => void
  onScope: (next: PresetScopeAxis) => void
}

const KINDS: PresetKindAxis[] = ['mkp', 'slicer']
const SCOPES: PresetScopeAxis[] = ['local', 'cloud']

export default function PresetScopeBar({ kind, scope, onKind, onScope }: Props) {
  return (
    <>
      {/*
       * 两条分段**拆开**：类型在左（我在看什么类型），
       * 位置靠右与机型 / 搜索成一组（我从哪里看、怎么筛）—— `.segPush` 的 auto 边距
       * 就是那条分界。尺码同级（都 32px）：作者「让 MKP配置/切片器配置 变小，
       * 跟本地/云端差不多高」，原本 38px 的那一级退回。
       */}
      <span className={s.seg} role="radiogroup" aria-label="文件类型">
        {KINDS.map((k) => (
          <label key={k} className={s.opt} data-on={k === kind}>
            <input
              type="radio"
              className={s.radio}
              name="preset-kind"
              value={k}
              checked={k === kind}
              onChange={() => onKind(k)}
            />
            {KIND_AXIS_TEXT[k]}
          </label>
        ))}
      </span>

      <span className={`${s.seg} ${s.segPush}`} role="radiogroup" aria-label="位置">
        {SCOPES.map((sc) => (
          <label key={sc} className={s.opt} data-on={sc === scope} title={SCOPE_AXIS_WHY[sc]}>
            <input
              type="radio"
              className={s.radio}
              name="preset-scope"
              value={sc}
              checked={sc === scope}
              onChange={() => onScope(sc)}
            />
            {SCOPE_AXIS_TEXT[sc]}
          </label>
        ))}
      </span>
    </>
  )
}
