/*
 * 静态标签（C14 words.ts 里**不是状态词**的那几条）。
 *
 * 状态词（生成态 / 保存态 / 来源 / 快照……）一律走 `wb_words` 从后端拿 ——
 * 那条纪律不变。这里收容的是另一类：字段名（显示名 / 品牌 / 内部名）与
 * 「需填写」这类**界面上固定不变**的字面量。它们在原型里也住在 words.ts，
 * 只是因为原型没有后端可问；真后端的 `words.rs` 没有它们的位置（不是枚举态）。
 *
 * `paramDeprecated` 那两档**不属于这里**：它们是真状态词，P2 随弃用判据一起
 * 进 `words.rs`（C14 README 点名「接后端时要带过去」，见 C14-PORT-PLAN §4-1）。
 */

/** 身份卡的格名。一处改、处处对 —— 「需填写 N 项」那句提示也读这里 */
export const machineFieldLabels = {
  display: '显示名',
  brand: '品牌',
  name: '内部名',
  image: '机型图',
  icon: '图标',
} as const

/** 字段自己的状态。只有一档：必填却还空着（可选字段空着不是状态，永远没有词条） */
export const fieldState = {
  needsInput: '需填写',
  needsInputHint: '必填，但还空着 —— 不是错误，只是迟早得填',
} as const

/** 占位（缺失）状态的那几句。不留白 —— 空白会被读成「还没算」 */
export const placeholderText = {
  noDimensions: '这台机型还没配尺寸（占位），不参与交付',
  noBundle: '这个版本还没有套餐',
} as const
