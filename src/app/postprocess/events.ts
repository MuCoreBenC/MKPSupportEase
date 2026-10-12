/*
 * 后处理那一趟的六个事件名 —— **与 Rust 侧 `hook_ui` 那几个常量同名的那一份**
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
/** 开场那几个事实（机型 / 耗材 / 大小 / 喷嘴 / 层高）—— 左栏那张表 */
export const POSTPROCESS_FACTS = 'postprocess-facts'
/** 一条进度（进度条 + 那一行字） */
export const POSTPROCESS_PROGRESS = 'postprocess-progress'
/** 要用户回答一个问题（目前只有"机型不匹配还跑不跑"） */
export const POSTPROCESS_QUESTION = 'postprocess-question'
/** 跑完了（成功 / 失败 / 取消都走这一条），带上退出码 */
export const POSTPROCESS_FINISHED = 'postprocess-finished'
/**
 * 延后的打印时间**补算完了**（`hook_ui::PRINT_TIME_EVENT`）。
 *
 * 有界面时打印时间是**延后**算的（钩子必须立刻退出，切片器在等退出码）——
 * 所以成功那条结论里的「预估打印时间」先空着，这一条到了再补上那一格。
 */
export const POSTPROCESS_PRINT_TIME = 'postprocess-print-time'
