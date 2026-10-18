/*
 * 开发台那一格的**共享常量与类型**（不导出组件、不导出 hook —— 那是为了 Fast Refresh；
 * 与 `import/useFileImport.ts` 同一个拆法）。
 */

/** Tauri 事件名 —— 与 Rust 侧 `hook_ui::DEV_PREVIEW_EVENT` **必须逐字一样** */
export const DEV_PREVIEW_EVENT = 'dev-preview'

/**
 * 一格界面属于**哪一个层类**。
 *
 * 为什么要分类（作者 2026-10-14）：开发台能硬点出任意组合，而真机上这些框是由具体动作
 * 开的、一次一个。同时摆出来时，"谁顶掉谁"必须有个明白的规矩，不能碰运气。
 *
 * 三条规矩：
 *  1. **同类互斥**：同一类里只留**最后**点的那一格（连点两次「重命名」= 第二个顶掉第一个）。
 *  2. **异类并存**：不同类可以同时摆着（更新面板 + 参数对比台 + 提示条，各摆各的）。
 *  3. **层级由 CSS 定死**（不按点击先后），从高到低见下表。
 *
 * 层级（与各 CSS 里的 z-index 对应，**别按点击顺序**）：
 *
 * ```text
 *   最高  full     后处理那一屏（跑着时整扇窗都归它，含标题栏）      .layer 92 / .dragShield 95
 *         titlebar 标题栏上的东西（软件更新指示器 + 详情面板）       TopTabs 94 之上
 *         field    右键菜单 / 下拉（常在模态框里用，得压过模态框）   FieldLayer 90
 *         modal    居中模态框 / 抽屉（只盖**内容区**，不盖标题栏）   .scrim 40
 *   最低  note     内容区里那条提示条（in-flow，本来就在内容流里）   ——
 * ```
 *
 * ★ `modal` 与 `note` **不在一层叠上下文里**（前者 portal 到页面根、后者在文档流里），
 *   所以它们同不同层都不用管 —— 真正要排的是 `full` / `titlebar` / `field` / `modal` 四层。
 */
export type DevSlot = 'full' | 'titlebar' | 'field' | 'modal' | 'note'

/** 现存场景 → 层类。**加一格就在这儿加一行**（认不出时按 `modal` 兜底）。 */
const SLOT_OF: Record<string, DevSlot> = {
  /* 后处理那一组（演出，不走本 Provider —— 但留在这儿好让层类表完整） */
  'postprocess-running': 'full',
  'postprocess-success': 'full',
  'postprocess-fail': 'full',
  'postprocess-cancel': 'full',
  'postprocess-question': 'full',
  /* 那一问的两种收场：仍是同一扇窗（'full'），只是 hero / 标题栏换一档 */
  'postprocess-question-timeout': 'full',
  'postprocess-answer-stale': 'full',
  /* 碰撞时那一问（两幅俯视图 + 翻页说明，2026-10-18） */
  'postprocess-relocate-question': 'full',
  'postprocess-backgrounded': 'full',
  'postprocess-success-no-metrics': 'full',
  'postprocess-success-long-warning': 'full',
  /* 校准件那一问（同一扇窗，hero 换一档） */
  'postprocess-calib-question': 'full',
  'postprocess-calib-print-done': 'full',
  'postprocess-calib-direct-done': 'full',
  'postprocess-calib-timeout': 'full',
  /* 菜单 / 弹层 */
  'delete-confirm': 'field',
  /* 标题栏 */
  'update-prompt': 'titlebar',
  /* 居中模态框 / 抽屉 */
  'import-collision': 'modal',
  'import-result': 'modal',
  rename: 'modal',
  compare: 'modal',
  export: 'modal',
  /* 提示条 */
  'float-toast': 'note',
}

/** 这一格属于哪一层类（认不出按 `modal` 兜底 —— 那是绝大多数） */
export function slotOf(scene: string): DevSlot {
  return SLOT_OF[scene] ?? 'modal'
}

/** 一类里现在摆着哪一格（`null` = 这一类没摆东西） */
export interface Occupied {
  scene: string
  /** 第几次（）—— 同一格再点一次也 +1，认领方拿它当 effect 依赖 */
  nOnce: number
}

/** context 里那一份（见 `context.ts` / `preview.tsx`） */
export interface Slot {
  /** 每一层类现在摆着哪一格 —— **同类互斥**（见 `DevSlot`） */
  bySlot: ReadonlyMap<DevSlot, Occupied>
  /** 关掉**一格**（用户点「关闭」时调；只清它所在的那一类） */
  close: (scene: string) => void
}
