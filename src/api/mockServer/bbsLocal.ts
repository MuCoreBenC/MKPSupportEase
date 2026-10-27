/*
 * **本机 BBS 目录的演示集合** —— 工艺页左边那个「本机工艺」档。
 *
 * # 为什么要造一份
 *
 * 那一档的真值住在用户的切片器里（`%APPDATA%\BambuStudio\system\BBL\process` 那 278 份、
 * 加上 `user/<账号>/process` 里自己另存的那些），桌面版走 IPC **现读**
 * （`src-tauri/src/ipc/bbs.rs`）。浏览器预览**没有盘** —— 不造一份，那一档永远是个空壳：
 * 两档来源里有一档点不进去、机型下拉里一台机型都没有、参数区一行都画不出来。
 *
 * # 造的是什么，不是什么
 *
 * 造的是**「这份 JSON 长什么样」**：形状是死的、有依据 —— 键名逐个对着
 * `public/bbs/registry.json` 核过，继承链照 BBS 自己那条
 * （`档名 @BBL …` → `fdm_process_single_0.20` → `fdm_process_single_common` → `fdm_process_common`，
 * 后三层是 `instantiation: "false"` 的中间层：BBS 下拉里不列，但继承链必须经过）。
 * **链要真能走通**，「基准 N 项 / 改动 N 项」才是算出来的，不是编的一个数。
 *
 * **不造**「你机器上有哪几份」：那是用户自己的东西（他在 BBS 里另存一份就该多一份），
 * 演示集合里就这 6 份，与真机上那几百份没有任何关系。要看真的，跑桌面版。
 *
 * # 与真机口径对齐的两处
 *
 * - `system` 那 5 份里有两份是抽象中间层（`instantiation: "false"`）—— 与真机一样，
 *   它们**不可选**、但作为父档要能查到；
 * - 用户层那 1 份**自己不写机型**（与真机上那批 MKPProcess 一样的处境）：它的机型要顺
 *   `inherits` 跳到系统那一层去认（`bbsSource.resolveTarget` 对 `user` 就是这条口径）。
 */

import type { BbsLocalEntry, BbsLocalTree } from '../contract'

/** 演示用的账号目录名（真机上是数字 uid，也有 `BBL` / `default` 这种） */
const DEMO_UID = 'demo'

/** 演示集合的修改时间（固定值：预览里每次刷新都该是同一个数，别跟着当前时间漂） */
const DEMO_MTIME = Date.UTC(2026, 9, 13, 4, 20, 0)

type Doc = Record<string, unknown>

/*
 * 链根：BBS 里那一份 170 项的公共层，这里按同一族键给一个能看的子集。
 * 值取的是 BBS 出厂档的量级（不是真文件的逐字拷贝 —— 本仓不打包那批快照）。
 */
const COMMON: Doc = {
  type: 'process',
  name: 'fdm_process_common',
  from: 'system',
  instantiation: 'false',
  bottom_shell_layers: '3',
  top_shell_layers: '3',
  bottom_shell_thickness: '0',
  top_shell_thickness: '0.8',
  sparse_infill_density: '15%',
  sparse_infill_pattern: 'grid',
  wall_loops: '2',
  inner_wall_speed: '300',
  outer_wall_speed: '200',
  initial_layer_speed: '50',
  travel_speed: '500',
  bridge_speed: '50',
}

/** 单喷嘴 0.20 那一层：真机里它接在 common 后面，落层高与线宽 */
const SINGLE_020: Doc = {
  type: 'process',
  name: 'fdm_process_single_0.20',
  inherits: 'fdm_process_single_common',
  from: 'system',
  instantiation: 'false',
  layer_height: '0.2',
  initial_layer_print_height: '0.2',
  line_width: '0.42',
  initial_layer_line_width: '0.5',
  outer_wall_line_width: '0.42',
  inner_wall_line_width: '0.45',
  internal_solid_infill_line_width: '0.45',
  sparse_infill_line_width: '0.45',
}

/** `fdm_process_single_common` 那一层（链上的一环，照真机放着） */
const SINGLE_COMMON: Doc = {
  type: 'process',
  name: 'fdm_process_single_common',
  inherits: 'fdm_process_common',
  from: 'system',
  instantiation: 'false',
  top_surface_line_width: '0.42',
  top_surface_speed: '200',
  support_line_width: '0.42',
}

/** 一台机 + 一个喷嘴一份档：名字与 `compatible_printers` 都照 BBS 自己的写法 */
function standard(model: string, printer: string, over: Doc = {}): Doc {
  return {
    type: 'process',
    name: `0.20mm Standard @BBL ${model}`,
    inherits: 'fdm_process_single_0.20',
    from: 'system',
    instantiation: 'true',
    version: '2.0.0.83',
    print_settings_id: `0.20mm Standard @BBL ${model}`,
    compatible_printers: [printer],
    outer_wall_speed: '200',
    inner_wall_speed: '300',
    ...over,
  }
}

const SYSTEM: Record<string, Doc> = {
  'fdm_process_common.json': COMMON,
  'fdm_process_single_common.json': SINGLE_COMMON,
  'fdm_process_single_0.20.json': SINGLE_020,
  '0.20mm Standard @BBL A1.json': standard('A1', 'Bambu Lab A1 0.4 nozzle'),
  '0.20mm Standard @BBL A1M.json': standard('A1M', 'Bambu Lab A1 mini 0.4 nozzle', {
    /* mini 那台热床小一号，外壁本来就慢一档 —— 真机上也是这么分的 */
    outer_wall_speed: '150',
  }),
  '0.20mm Standard @BBL P1S.json': standard('P1S', 'Bambu Lab P1S 0.4 nozzle'),
}

/*
 * 用户层那两份。**机型都不写在自己身上**（真机上的用户档与 MKPProcess 那批一样
 * 常常没这一串）—— 顺 inherits 到系统那一层去认。
 *
 * 第二份是**我们那份复制进来的**（`copy_to_slicer` 之后的样子）：文件名叫
 * `MKPProcess A1 0.4 0.20.json`，与 `localFiles.ts` 的"已复制"集合对得上 ——
 * 工艺页据此在「本机工艺」档里给它标一颗「我们发的」（作者 §7）。不标的话，
 * 用户会以为本机目录里这一份是自己另存的。
 */
const USER: Record<string, Doc> = {
  'MKPProcess A1 0.4 0.20.json': {
    type: 'process',
    name: 'MKPProcess A1 0.4',
    inherits: '0.20mm Standard @BBL A1',
    from: 'User',
    instantiation: 'true',
    version: '2.0.0.83',
    print_settings_id: 'MKPProcess A1 0.4',
    layer_height: '0.2',
    initial_layer_print_height: '0.2',
    wall_loops: '3',
    sparse_infill_density: '25%',
    outer_wall_speed: '120',
  },
  '0.20mm High Flow MKP.json': {
    type: 'process',
    name: '0.20mm High Flow MKP',
    inherits: '0.20mm Standard @BBL A1',
    from: 'User',
    instantiation: 'true',
    version: '2.0.0.83',
    print_settings_id: '0.20mm High Flow MKP',
    /* 下面三项是**改动**（与链上算出来的基准不同）—— 参数区那几处会标橙 */
    wall_loops: '3',
    sparse_infill_density: '25%',
    outer_wall_speed: '120',
  },
}

/** 清单里那一项由正文现推 —— 清单与正文是**同一份数据**推出来的，不会对不上 */
function entryOf(scope: 'user' | 'system', uid: string | null, file: string, doc: Doc): BbsLocalEntry {
  const text = JSON.stringify(doc, null, 2)
  return {
    scope,
    uid,
    file,
    name: String(doc.name ?? file.replace(/\.json$/i, '')),
    inherits: typeof doc.inherits === 'string' ? doc.inherits : null,
    instantiation:
      typeof doc.instantiation === 'string' || typeof doc.instantiation === 'boolean'
        ? doc.instantiation
        : null,
    compatible_printers: Array.isArray(doc.compatible_printers)
      ? (doc.compatible_printers as string[])
      : null,
    from: typeof doc.from === 'string' ? doc.from : null,
    mtime: DEMO_MTIME,
    size: text.length,
    broken: null,
  }
}

/**
 * 那一档在这台（浏览器）机器上的"事实"。
 *
 * `root` 写的是**演示标记**而不是一条真路径 —— 界面上那一句"本机 BBS 目录"要能看出来
 * 这是预览，不是你那台机器上真的什么都没有。
 */
export const bbsLocalTree: BbsLocalTree = {
  available: true,
  app: 'BambuStudio（演示集合）',
  root: '（浏览器预览：演示集合，真值在桌面版里现读）',
  user: Object.entries(USER).map(([file, doc]) => entryOf('user', DEMO_UID, file, doc)),
  system: Object.entries(SYSTEM).map(([file, doc]) => entryOf('system', null, file, doc)),
  reason: null,
}

/** 一份正文的**原文**（解析交给页面：坏 JSON 要由它照实报错）。认不出返回 `null`。 */
export function bbsPresetText(scope: string, uid: string | null, file: string): string | null {
  const doc =
    scope === 'system'
      ? SYSTEM[file]
      : scope === 'user' && uid === DEMO_UID
        ? USER[file]
        : undefined
  return doc === undefined ? null : JSON.stringify(doc, null, 2)
}
