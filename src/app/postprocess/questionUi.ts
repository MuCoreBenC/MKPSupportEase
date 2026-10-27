/**
 * 问句屏的**让位规则**与**逃生通道** —— 纯判据，组件只照它渲染。
 *
 * # 它修的是什么（2026-10-19 作者真机反馈）
 *
 * > 「我点下去这个时候我也关不了模态……他只会在那个标题那里出一个报错，
 * >  但这个时候我没办法观摩……就只能处理一个，剩下就没办法理了。
 * >  他就会说已经处理了、或者点下去晚了。」
 *
 * 病根不在"没排队"，而在**报错之后这一屏没有出路**：
 *
 * 1. 选项按钮一点就 `setAnswering(...)`，问句那一屏随即让位给 hero
 *    （作者的裁定：答完到钩子收掉问句之间，按钮不许再立在那里给人连点）；
 * 2. 如果这一答**晚了一步**（钩子已经退了 / 超时收场了），后端如实回
 *    「这一趟已经结束了，没有正在跑的处理」，那句话落在标题右边；
 * 3. 可问句屏已经让位 ⇒ 用户手里既没有按钮、也没有关窗的路。
 *
 * 作者给的解法比"多问句排队"简单得多：**「出错的时候加一个停止并关闭的按钮，
 * 这样就不用管多的操作了」**。
 *
 * # 两条规则
 *
 * 1. **报错时把问句屏放回来**（`err !== null` ⇒ 即使 `answering` 还挂着也让位收回）。
 *    让位是为了防连点，不是把人关在门外 —— 至少得让人看见自己在答哪一问。
 * 2. **报错时只摆一颗「停止并关闭」**（`offerStopAndClose`）：先请求停止
 *    （**吞掉失败**：钩子可能已经没了），再收起这一屏。不伪造"停成功了"。
 *
 * ★ 为什么正常问句（有问句、还没答）**不**给这颗：那一屏本来就有「终止」
 *   （见 `PostProcessModal` 的页脚），凭空多一颗同义按钮只会让人犹豫。
 *   逃生的场景是"按钮已经没了/点了报错"，不是"还没答"。
 */

export interface QuestionUiInput {
  /** 这一趟还在跑（`finished === null`） */
  running: boolean
  /** 有问句挂着（`run.question !== null`） */
  asking: boolean
  /** 点了某颗选项、正在等钩子把问句收掉 */
  answering: boolean
  /** 界面上那条红字（答晚了 / 两次答复撞车） */
  err: string | null
}

export interface QuestionUiState {
  /** 摆问句那一屏（含选项按钮） */
  showAsk: boolean
  /** 摆「停止并关闭」这颗逃生按钮（此时**不摆**别的按钮） */
  offerStopAndClose: boolean
}

export function questionUiState(input: QuestionUiInput): QuestionUiState {
  const failed = input.err !== null
  return {
    /* ① 报错时把问句屏放回来 —— 否则用户既没有按钮、也没有关窗的路 */
    showAsk: input.asking && (!input.answering || failed),
    /* ② 逃生通道：**只要报错且还在跑**就给（报错+已结束 ⇒ 结果屏自己有关闭） */
    offerStopAndClose: input.running && failed,
  }
}
