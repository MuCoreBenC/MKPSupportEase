/**
 * 「这一格脏不脏」的**唯一判据**：值真的与已保存值不同才算。
 *
 * 与工作台 `src/workbench/sheet/draftOverlay.ts` 的 `dirtyOf` 同一条口径
 * （那边比的是 `shownRawOf` 与 `savedRawOf`）。
 *
 * 只看"草稿里有没有这个键"是不够的：那依赖 `apply` 时"值等于已保存值就把键撤掉"
 * 的隐含约定 —— 只要有一处漏撤，行上徽章灭了、底栏还挂着「未保存 N」，
 * 两个数各说各话。
 */
export const isDirtyValue = (draftValue: string | undefined, savedValue: string): boolean =>
  draftValue !== undefined && draftValue !== savedValue
