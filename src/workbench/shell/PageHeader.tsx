/**
 * 结构带 2：文件头（44px）。
 *
 * 它回答：**这是什么文件、能对它做什么。**
 *
 * # 动作走配置数组，不在页面里内联 JSX
 *
 * 外壳只认识 `Action[]`，页面负责拼这个数组。这样做的理由不是"优雅"，
 * 而是**禁用理由跑不掉**：`Action` 里 `disabledReason` 与 `disabled` 是一对，
 * 类型上就要求「禁用了必须给一句话」（见 [`Action`] 的注释）。
 * 内联 JSX 的写法里，`disabled` 是一个属性、title 是另一个，
 * 漏写 title 编译照样过 —— 然后用户面对一个按不动、又不说为什么的按钮。
 *
 * # 徽章是「一共几项」与「我们写了几项」两个数
 *
 * doc §3.6：机型层与版本层各有两半（上游给的 + 我们写的）。
 * 徽章主数字是这一层一共有几项非默认值，括号里那个是我们自己写的 ——
 * 只给一个数的话，用户分不清「这台机器有 5 项特化」和「我改过 5 项」。
 */
import type { ReactNode } from 'react'

export interface Badge {
  /** 主数字 */
  count: number
  label: string
  /** 其中我们自己写了几项。`undefined` = 这个徽章没有这个概念 */
  own?: number
  title?: string
}

/**
 * 一个动作。**`disabled` 与 `disabledReason` 必须成对出现** ——
 * 用可辨识联合把它变成类型错误，而不是靠 code review 记着
 */
export type Action =
  | {
      id: string
      label: string
      tone?: 'primary' | 'danger' | 'plain'
      disabled?: false
      onClick: () => void
      /** 能点时也给一句，说清点下去会发生什么 */
      title?: string
    }
  | {
      id: string
      label: string
      tone?: 'primary' | 'danger' | 'plain'
      disabled: true
      /** 禁用必须有理由。少了它用户只能猜是不是坏了 */
      disabledReason: string
    }

interface Props {
  icon?: ReactNode
  title: string
  subtitle?: string
  badges: Badge[]
  actions: Action[]
}

export function PageHeader({ icon, title, subtitle, badges, actions }: Props) {
  return (
    <header className="wb-head">
      <div className="wb-head__id">
        {icon && <span className="wb-head__icon">{icon}</span>}
        <span className="wb-head__title">{title}</span>
        {subtitle && <span className="wb-head__sub">{subtitle}</span>}
      </div>

      <div className="wb-head__badges">
        {badges.map((b) => (
          <span key={b.label} className="wb-badge" title={b.title}>
            <b>{b.count}</b> {b.label}
            {b.own !== undefined && (
              <i className="wb-badge__own"> · 自有 {b.own}</i>
            )}
          </span>
        ))}
      </div>

      <div className="wb-head__actions">
        {actions.map((a) =>
          a.disabled ? (
            <button
              key={a.id}
              type="button"
              className="wb-btn"
              data-tone={a.tone ?? 'plain'}
              disabled
              title={a.disabledReason}
            >
              {a.label}
            </button>
          ) : (
            <button
              key={a.id}
              type="button"
              className="wb-btn"
              data-tone={a.tone ?? 'plain'}
              title={a.title}
              onClick={a.onClick}
            >
              {a.label}
            </button>
          ),
        )}
      </div>
    </header>
  )
}
