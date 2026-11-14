/*
 * 「新建页签」小框 —— 只有一个名字。
 *
 * # 为什么只问名字
 *
 * 页签的 id 会进两份 TOML（`param_registry.toml` / `layout_schema.toml`），还会进
 * 客户端目录，中文当不了 id。所以界面上只写名字，id 由调用方按 `custom_tab_N`
 * 取下一个空的（见 `sectionId.ts`）。
 *
 * # 页签与分组是两层
 *
 * 建页签会顺手在它下面建一个空分组（后端 `ensure_tab` 做），否则点进去是一屏空白、
 * 参数没有落点。这一框不重复问那个名字。
 */
import { useId, useState } from 'react'

import ModalC14 from '../c14/ModalC14'
import s from '../c14.module.css'

export interface NewTabModalProps {
  onCancel: () => void
  /** 确认：只交名字（非空、已 trim） */
  onCreate: (label: string) => void
}

/** 与后端 `ensure_tab` 的 24 字上限同一把尺子 */
const LABEL_MAX = 24

export default function NewTabModal({ onCancel, onCreate }: NewTabModalProps) {
  const nameId = useId()
  const [name, setName] = useState('')
  const ready = name.trim() !== ''
  const submit = () => {
    if (ready) onCreate(name.trim())
  }

  return (
    <ModalC14
      open
      size="sm"
      title="新建页签"
      subtitle="页签是参数页最上面那一排里的一格"
      onClose={onCancel}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onCancel}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!ready}
            onClick={submit}
          >
            新建
          </button>
        </>
      }
    >
      {/* 一格表单一列纵向排：标签 / 输入框 / 一句后果。别用 `.kv` —— 那是两列信息表 */}
      <label className={s.fieldCol} htmlFor={nameId}>
        <span className={s.fieldLabel}>名字</span>
        <input
          id={nameId}
          className={s.inp}
          value={name}
          maxLength={LABEL_MAX}
          autoFocus
          placeholder="例如：保养"
          onChange={(e) => setName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') submit()
            else if (e.key === 'Escape') onCancel()
          }}
        />
        <span className={s.dimSrc}>自带一个空分组，可直接放参数</span>
      </label>
    </ModalC14>
  )
}
