/*
 * 随包资产同步器 —— 「唯一源 → 唯一随包交付根」的对账式同步。
 *
 * # 它在整条链里的位置
 *
 * ```text
 * presets/assets.toml   （delivery 的唯一输入）
 * presets/assets/**     （文件的唯一源）
 *         │
 *         │  本文件：算期望集合 → 与目标对账 → 增 / 改 / 删
 *         ▼
 * client-assets/        （唯一随包交付根；生成物，不入库）
 *   assets/<path>       （与 URL 同形的树：/assets/<path>）
 *   manifest.json       （构建期派生账，不入库、不作为契约）
 * ```
 *
 * 同步**只由 vite 插件触发**（`tools/assets/plugin.mjs`）：任何走 vite 的命令
 * （`npm run build` / `build:workbench` / 裸 `npx vite build` / 探针那次构建）
 * 都会先经过这里，所以没有任何一条命令依赖"上一次命令留下的目录状态"。
 * npm script 里不再有前置 —— 两个入口最后一定会变成两个行为定义。
 *
 * # 对账的语义（五个分支，一个都不能少）
 *
 * ```text
 * 期望集合 = 台账里 delivery = 'bundled' 且有 path 的全部条目
 *
 *   新增（期望有、目标没有）        → 写入
 *   改过（两边 SHA 不同）           → 覆盖
 *   没变（两边 SHA 相同）           → skip
 *   取消随包（目标有、期望没有）     → 删除
 *   源文件不存在                    → 抛错，构建失败
 * ```
 *
 * **"删"必须按期望集合对账**：只遍历"源里有的那些"永远发现不了"源里已经没有的"
 * —— 而"取消随包"恰好就是这一类。
 *
 * # 为什么 SHA 是这里的正式语义，而不是优化
 *
 * 判据是「源文件 SHA vs 目标文件实际 SHA」——**目标文件本身就是状态**，不需要另立一份
 * "上次同步了什么"的台账（那种账在目标被外部删掉时不会自愈，而这里的判据会）。
 * 它同时是 skip 的依据：没变就不写盘，改了才覆盖。
 *
 * **这条不是"为了省 Vite 的活"**：`dist/` 每次构建都被 `emptyOutDir` 清空，所以
 * `client-assets/` 必须是**持久目录**（不被清空）——SHA skip 才有意义。谁想删掉这条，
 * 要改的是产品规则（"随包资源目录 = 由源数据同步出来的交付结果"），不是一句
 * "Vite 可以直接 import 源文件"就能绕过去的。
 *
 * # 不引入依赖
 *
 * 台账是**手写扫描**的（形状是死的：单层 `[[assets]]` 数组表 + 三个标量键）。
 * 有现成的先例与理由：多一个 TOML 依赖不划算，见第一版 `scripts/copy-assets.mjs` 的文件头。
 * 扫描前**先剥掉整行注释** —— 台账头部那几条示例（`#   delivery = 'bundled'`）不该被当条目。
 *
 * # 它不负责什么
 *
 * - **不管工作台预览那一份**：工作台 dev 由 vite middleware 直读 `presets/assets`
 *   （读取源，不落盘），build 时另有一趟装配（全部有 `path` 的条目）——两个集合不同
 *   是有意的：`client-assets/` 的语义只有一句「客户端随包资产交付结果」。
 * - **不写 `presets/dist/`**：那是云端交付根，由 `publish:presets` 那条链管。
 */
import { createHash } from 'node:crypto'
import {
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  realpathSync,
  rmSync,
  rmdirSync,
  statSync,
  writeFileSync,
} from 'node:fs'
import { dirname, join, relative, resolve, sep } from 'node:path'

/** 交付登记（delivery 的唯一输入），相对仓库根 */
export const LEDGER_REL = join('presets', 'assets.toml')
/** 资产载荷根（文件的唯一源），相对仓库根 */
export const SOURCE_ROOT_REL = join('presets', 'assets')
/** 随包交付根（生成物，不入库），相对仓库根 */
export const OUTPUT_ROOT_REL = 'client-assets'
/** 交付根里那棵与 URL 同形的树：`/assets/<path>` 的那个 `assets` */
export const OUTPUT_ASSETS_SUBDIR = 'assets'
/** 派生账的文件名（在交付根下）。每次构建重新生成 */
export const MANIFEST_FILE = 'manifest.json'
/** 生成目录标记（给人看的，不是程序读的契约） */
export const MARKER_FILE = 'GENERATED.md'
/** 结构版本。形状变了才升号 */
export const MANIFEST_VERSION = 1

const sha256 = (buf) => createHash('sha256').update(buf).digest('hex')

/** 行内注释先剥掉，再把一份 TOML 拆成 `[[assets]]` 块（不走完整解析器，见文件头） */
function assetBlocks(text) {
  const code = text
    .split('\n')
    .filter((line) => !line.trimStart().startsWith('#'))
    .join('\n')
  return code.split('[[assets]]').slice(1)
}

/** 取一个标量键的值。单引号与双引号都收（手写台账两种都可能出现） */
function scalar(block, key) {
  const m = block.match(new RegExp(`^[ \\t]*${key}[ \\t]*=[ \\t]*(?:'([^']*)'|"([^"]*)")`, 'm'))
  return m === null ? null : (m[1] ?? m[2])
}

/**
 * 读台账：全部条目（含没有 `path` 的 `mkPreset` 登记）。
 *
 * 返回 `{ id, type, delivery, path }`，`path` 可能为空串（`mkPreset` 那 9 条就是）。
 */
export function readLedger(repoRoot) {
  const ledger = join(repoRoot, LEDGER_REL)
  if (!existsSync(ledger)) {
    throw new Error(
      `读不到资产台账：${ledger}\n` +
        '  它是 delivery 的唯一输入（随包资产同步链的起点）—— 台账不在，什么都不能往下走。',
    )
  }
  const blocks = assetBlocks(readFileSync(ledger, 'utf8'))
  const items = blocks.map((block, i) => {
    const id = scalar(block, 'id')
    if (id === null || id.trim() === '') {
      throw new Error(`${LEDGER_REL} 第 ${i + 1} 个 [[assets]] 块没有 id —— 台账坏了`)
    }
    return {
      id: id.trim(),
      type: (scalar(block, 'type') ?? '').trim(),
      delivery: (scalar(block, 'delivery') ?? 'download').trim(),
      path: (scalar(block, 'path') ?? '').trim(),
    }
  })
  return items
}

/**
 * 期望集合：台账里 `delivery = 'bundled'` 且有 `path` 的全部条目，带上真实字节的 SHA 与大小。
 *
 * **源文件不存在就是错误**（由调用方抛给构建）—— 这是"客户端读取改成台账驱动"之后，
 * 构建期唯一还能拦住"登记了但文件不在"的闸；少了它，缺文件会退化成运行时的破图。
 */
export function readBundledAssets(repoRoot) {
  const srcRoot = join(repoRoot, SOURCE_ROOT_REL)
  const out = []
  for (const a of readLedger(repoRoot)) {
    if (a.delivery !== 'bundled') continue
    if (a.path === '') {
      throw new Error(
        `资产 ${a.id} 标了 delivery = 'bundled'，却没有 path。\n` +
          '  随包 = 构建期把它复制进客户端资源，没有 path 就不知道复制谁。\n' +
          '  （MKP 预设那一类不写 path，它也不允许设成随包 —— 台账检查会先拦下。）',
      )
    }
    const from = join(srcRoot, a.path)
    if (!existsSync(from) || !statSync(from).isFile()) {
      throw new Error(
        `台账登记了 ${a.id}（delivery = 'bundled'），但源目录里没有这个文件：\n` +
          `  ${from}\n` +
          '  要么把文件放回去，要么把这条改回 delivery = ' +
          "'download' / 删掉登记 —— 不许静默漏一份随包资产。",
      )
    }
    const bytes = readFileSync(from)
    out.push({ id: a.id, path: a.path, sha256: sha256(bytes), bytes: bytes.length })
  }
  out.sort((x, y) => (x.path < y.path ? -1 : x.path > y.path ? 1 : 0))
  return out
}

/** 递归列出 `dir` 下的全部文件，返回 `Map<相对路径（/ 分隔）, 绝对路径>`。目录不存在 = 空 */
function walkFiles(dir, prefix = '', out = new Map()) {
  if (!existsSync(dir)) return out
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    const rel = prefix === '' ? name : `${prefix}/${name}`
    if (statSync(full).isDirectory()) walkFiles(full, rel, out)
    else if (statSync(full).isFile()) out.set(rel, full)
  }
  return out
}

/** 自底向上删掉空目录（对账删文件之后留下的壳）；根目录本身保留 */
function pruneEmptyDirs(dir) {
  if (!existsSync(dir)) return
  for (const name of readdirSync(dir)) {
    const full = join(dir, name)
    if (statSync(full).isDirectory()) pruneEmptyDirs(full)
  }
  const left = readdirSync(dir)
  if (left.length === 0) rmdirSync(dir)
}

/** 内容变了才写 —— 避免每次构建都重写同一份文件（也免掉没必要的 dev 重载） */
function writeIfChanged(file, text) {
  if (existsSync(file) && readFileSync(file, 'utf8') === text) return false
  mkdirSync(dirname(file), { recursive: true })
  writeFileSync(file, text)
  return true
}

/** 派生账。**刻意没有 generatedAt** —— 没有可信时间源就不编一个（与 catalog 同一条裁决） */
export function manifestText(assets) {
  return (
    JSON.stringify(
      {
        v: MANIFEST_VERSION,
        source: LEDGER_REL.split(sep).join('/'),
        assets,
      },
      null,
      2,
    ) + '\n'
  )
}

const MARKER_TEXT = `# 这是生成目录，别手改

\`client-assets/\` 是**客户端随包资产的交付结果**，由 \`tools/assets/sync.mjs\` 从
\`presets/assets.toml\` 的 \`delivery = 'bundled'\` 条目对账产出 —— 每次构建自动跑一遍，
目标文件本身就是状态（比 SHA 决定写不写、删不删）。

- 改内容请改源头：\`presets/assets/**\`（工作台或直接编辑）
- 换交付档位请改登记：\`presets/assets.toml\` 的 \`delivery\`
- **删掉整个目录是安全的** —— 下一次构建会重新生成
- 这个目录不入库（\`.gitignore\` 里有一条）

这里唯一的文件是给读的人看的，程序只读 \`manifest.json\` 与 \`assets/**\`。
`

/**
 * 对账式同步：期望集合（台账 bundled 条目）→ `client-assets/`。
 *
 * 五个分支见文件头。返回四类明细（`added` / `updated` / `skipped` / `removed`，
 * 都是相对 `client-assets/assets` 的路径），供调用方打日志与验收。
 *
 * **幂等**：没改任何源头时跑第二遍，结果是 4 条 skipped、一个字节都不写。
 */
export function syncBundledAssets(repoRoot, outRoot) {
  const expected = readBundledAssets(repoRoot)
  const srcRoot = join(repoRoot, SOURCE_ROOT_REL)
  const targetAssets = join(outRoot, OUTPUT_ASSETS_SUBDIR)
  const existing = walkFiles(targetAssets)
  const result = { added: [], updated: [], skipped: [], removed: [] }

  for (const a of expected) {
    const dest = join(targetAssets, a.path)
    const current = existsSync(dest) && statSync(dest).isFile() ? sha256(readFileSync(dest)) : null
    if (current === a.sha256) {
      result.skipped.push(a.path)
      continue
    }
    mkdirSync(dirname(dest), { recursive: true })
    writeFileSync(dest, readFileSync(join(srcRoot, a.path)))
    ;(current === null ? result.added : result.updated).push(a.path)
  }

  const wanted = new Set(expected.map((a) => a.path))
  for (const [rel, full] of existing) {
    if (wanted.has(rel)) continue
    rmSync(full)
    result.removed.push(rel)
  }
  pruneEmptyDirs(targetAssets)

  mkdirSync(outRoot, { recursive: true })
  writeIfChanged(join(outRoot, MANIFEST_FILE), manifestText(expected))
  writeIfChanged(join(outRoot, MARKER_FILE), MARKER_TEXT)
  return result
}

/**
 * 把一批资产原样复制进一个**目标根**（落点 = `<destRoot>/<path>`）。
 *
 * 这是"装配"，不是"同步"：调用方给的是明确的一份清单，目标每次重建（`dist/` 被
 * `emptyOutDir` 清空）——所以这里没有 SHA 分支，也不需要。
 * 用途只有一处：工作台构建时把**全部有 `path` 的台账条目**铺进 `dist/assets/`，
 * 好让工作台界面按 `/assets/<path>` 预览任何登记过的资产（包含 cloud 档）。
 */
export function copyAssetsInto(repoRoot, destRoot, items) {
  const srcRoot = join(repoRoot, SOURCE_ROOT_REL)
  let n = 0
  for (const a of items) {
    if (a.path === '') continue
    const from = join(srcRoot, a.path)
    if (!existsSync(from) || !statSync(from).isFile()) {
      throw new Error(
        `资产 ${a.id} 的文件不在：${from}\n` +
          '  装配（工作台构建）要求台账里每一条有 path 的资产都真在源目录里 —— 先补文件再构建。',
      )
    }
    const dest = join(destRoot, a.path)
    mkdirSync(dirname(dest), { recursive: true })
    writeFileSync(dest, readFileSync(from))
    n += 1
  }
  return n
}

/** 「路径必须落在这个根里面」——dev middleware 的三道闸之一（另两道在调用处） */
export function isInside(root, full) {
  const realRoot = existsSync(root) ? realpathSync(root) : resolve(root)
  let realFull
  try {
    realFull = realpathSync(full)
  } catch {
    return false
  }
  return realFull === realRoot || realFull.startsWith(realRoot + sep)
}
