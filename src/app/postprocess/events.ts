/*
 * 后处理那一趟的四个事件名 —— **与 Rust 侧 `hook_ui` 那四个常量同名的那一份**
 * （两边没有编译器，靠这一处对齐）。
 *
 * # 为什么单独一格
 *
 * 有两个消费者：那一屏模态框（`usePostProcessRun`）与**报告页**（后处理一跑完，
 * 报告就该多一条 / 改一条 —— 报告页读的就是这一趟落下的那份账）。
 * 事件名写字面量的地方只许有这一处。
 */

/** 新的一趟开始了（界面据此把上一趟的结论与"我关过了"一起清掉） */
export const POSTPROCESS_STARTED = 'postprocess-started'
/** 一条进度（进度条 + 那一行字） */
export const POSTPROCESS_PROGRESS = 'postprocess-progress'
/** 要用户回答一个问题（目前只有"机型不匹配还跑不跑"） */
export const POSTPROCESS_QUESTION = 'postprocess-question'
/** 跑完了（成功 / 失败 / 取消都走这一条），带上退出码 */
export const POSTPROCESS_FINISHED = 'postprocess-finished'
