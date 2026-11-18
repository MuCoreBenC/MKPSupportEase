/*
 * **后处理弹框的预览场景清单**（只给 `/?pp=` 用，dev-only）。
 *
 * 浏览器里到不了真钩子那一趟，所以 `src/api/mock.ts` 按 URL 里的名字造一份假快照。
 * 这张表是**唯一一份清单**：清单页（`/?pp=list`）、`mock.ts` 的组装、探针认的都是它。
 *
 * 序号也认：`/?pp=3` = 下面第 3 个。加场景只往这里加一条，别在别处再列一遍。
 */
export interface PpScene {
  /** URL 里的名字：`/?pp=<id>` */
  id: string
  /** 清单页那一行的标题 */
  name: string
  /** 一句话：这一屏在看什么 */
  note: string
}

export const PP_SCENES: PpScene[] = [
  { id: 'run', name: '处理中', note: '进度 hero + 两栏事实（还没结果）' },
  { id: 'mismatch', name: '机型不匹配', note: '琥珀警告卡，等回答' },
  { id: 'calib', name: '校准件：选择跑法', note: '绿卡 + 两颗选项按钮' },
  { id: 'debug-ask', name: '收落笔点位测试（确认）', note: '琥珀卡，两栏事实默认收起' },
  { id: 'relocate', name: '撞塔那一问', note: '两幅俯视图 + 说明页' },
  { id: 'result', name: '处理结果', note: '四格读数 + 两栏事实' },
  { id: 'vh', name: '可变层高', note: '层高那一格给范围' },
  { id: 'debug', name: '调试结果', note: '调试动作 / 测试范围两栏，信息收在钮后' },
  { id: 'relocate-done', name: '已自动挪塔', note: '两幅俯视图 + 挪塔表' },
  { id: 'relocate-fail', name: '挪塔后仍然相撞', note: '挪了但复检不过：说明页那句是红的' },
  { id: 'warn', name: '内核警告', note: '警告 ↔ 信息' },
  { id: 'glue', name: '热床预涂胶', note: '结果屏默认落在警告页' },
  { id: 'fail', name: '处理失败', note: '红卡 + 两栏事实' },
  { id: 'stop', name: '已停止', note: '柔和内影那一版' },
  { id: 'collision', name: '碰撞失败', note: '红卡默认页 + 碰撞说明' },
]

/**
 * URL 里那一段 → 场景 id。
 *
 * - 数字（`3`）按清单顺序取，1 起；
 * - 名字原样返回（`mock.ts` 认不出就当没有）；
 * - `list` / 空 ⇒ `null`（清单页由 `main.tsx` 单独分流，不走这一条）。
 */
export function resolvePpScene(raw: string | null): string | null {
  if (raw === null || raw === '' || raw === 'list') return null
  if (/^\d+$/.test(raw)) return PP_SCENES[Number(raw) - 1]?.id ?? null
  return raw
}
