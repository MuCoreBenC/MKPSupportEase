/*
 * 必填标记与「需填写」（C14 原样移植，词面改读 labels.ts）。
 *
 * # 只标必填，不标可选
 *
 * 可选字段空着是**正常状态**，不是待办；「（可选）」四个字还会把标签列撑宽。
 * 所以收缩成两档：**必填打星号，其余什么都不标** —— 少数派才需要标记。
 *
 * # 星色分两档（作者验收时提出：可以后补的就别用红色）
 *
 *   红星  不填就**建不出来**的创建前置条件 —— 机型 id、版本 id 这一类
 *   橙星  同样必填、但**可以之后在身份卡上补**的（品牌、显示名）——
 *         它们是「对象完整性要求」，不是「创建前置条件」，跟「需填写」橙标同族
 */

import s from '../c14.module.css'
import { fieldState } from './labels'

/**
 * 必填：标签后一颗星。默认红星（创建前置条件）；`later` 给橙星
 * （同样必填但可以后补，品牌 / 显示名这一类）。可选字段不传这个组件，什么都不标
 */
export default function FieldMark({ later = false }: { later?: boolean }) {
  return (
    <span
      className={later ? `${s.req} ${s.reqLater}` : s.req}
      aria-label={later ? '必填，可以之后补' : '必填'}
    >
      *
    </span>
  )
}

/** 值位的「需填写」—— 只给必填且当前为空的格子用 */
export function TodoValue() {
  return (
    <span className={s.todo} title={fieldState.needsInputHint}>
      {fieldState.needsInput}
    </span>
  )
}
