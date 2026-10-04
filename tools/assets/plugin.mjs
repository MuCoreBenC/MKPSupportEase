/*
 * 随包资产流水线的 **vite 入口** —— 这一条链上所有资产动作都只从这里发生。
 *
 * # 为什么必须是插件，而不是 npm script 前置
 *
 * 前置写法（`node xxx && tsc -b && vite build`）只在**你记得加的那几条命令**上成立：
 * 裸 `npx vite build`、探针那次构建、`dev:workbench` 都会漏，于是"资产在不在"
 * 又变成"上一次谁跑过什么"。挂在插件上则相反 —— **任何走 vite 的命令都经过同一段代码**，
 * 包括将来新增的入口。所以三条 npm script 的前置已全部删掉。
 *
 * # 三个动作
 *
 * ```text
 * configResolved   →  同步一次（所有模式）：台账 bundled 条目 → client-assets/
 * configureServer  →  dev：盯住源目录，改了重新对账；挂 /assets/* 中间件（**读**，不落盘）
 * closeBundle      →  build：装配进 dist/assets/ —— 客户端与工作台取的集合不同（见下）
 * ```
 *
 * # dev 两边读的东西有意不同（2026-10-03 第二刀）
 *
 * ```text
 * 工作台 dev → presets/assets/（源）         : 后厨要看得见**任何**登记资产，含不随包的
 * 客户端 dev → client-assets/assets/（交付根）: 客户端读到的就该是交付物，与打包后一致
 * ```
 *
 * 两边都**不落盘**（每次请求回源取字节），所以"读源"与"读交付根"都不产生第二份副本；
 * 交付根本身是同步器的产物，不是这里的副产物。
 *
 * # build 两边装配的集合也不同（有意的，不是裂缝）
 *
 * - **客户端**：只有 `delivery = 'bundled'`（今天 4 张整机图）→ `dist/assets/<path>`。
 *   它就是"随包交付结果"，与 `client-assets/` 里那一份**逐字节同源**。
 * - **工作台**：**全部有 `path` 的条目**（今天 19 份）→ 同一个 URL 空间。
 *   工作台是开发工具，要能预览任何登记资产（含 cloud 档），它不进安装包。
 *
 * 两者共用同一条 URL 规则 `/assets/<path>`（与后端 `workbench/app/assets.rs` 那处前缀同值），
 * 但**不要让同步器偷偷承担后者的职责** —— 那会把"随包"这个词弄脏。
 *
 * # 客户端为什么不再 import 交付根
 *
 * 第二刀把客户端取图改成**台账驱动**（`catalog.assets[]` 的 id → path → `/assets/<path>`），
 * 于是随包图既不需要 import、也不再以带哈希的名字进包：装配出来的就是 URL 同形的那棵树。
 * 结果：dist 里同一条资产只有一份，且"客户端认识的路径"与"数据里的 path"永远是同一个。
 */
import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname, join, resolve, sep } from 'node:path'

import {
  LEDGER_REL,
  OUTPUT_ASSETS_SUBDIR,
  OUTPUT_ROOT_REL,
  SOURCE_ROOT_REL,
  copyAssetsInto,
  isInside,
  readLedger,
  syncBundledAssets,
} from './sync.mjs'

/** 工作台预览的 URL 契约。**与后端那一处前缀同值**（`workbench/app/assets.rs`） */
export const ASSETS_URL_PREFIX = '/assets/'

/** 只服务这几类；认不出的一律 octet-stream（不给浏览器瞎猜的机会） */
const MIME = {
  '.webp': 'image/webp',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.json': 'application/json; charset=utf-8',
  '.3mf': 'application/octet-stream',
}

function profile(result) {
  return (
    `新增 ${result.added.length} · 覆盖 ${result.updated.length}` +
    ` · 跳过 ${result.skipped.length} · 删除 ${result.removed.length}`
  )
}

function detail(rows) {
  return rows.length === 0 ? '' : `（${rows.join('、')}）`
}

/** 同步一次，并把结果打成人能读的一行。失败 = 抛出去让构建停在这里 */
function runSync(repoRoot, logger) {
  const outRoot = join(repoRoot, OUTPUT_ROOT_REL)
  const result = syncBundledAssets(repoRoot, outRoot)
  logger.info(
    `[assets] 随包同步：${profile(result)} → ${OUTPUT_ROOT_REL}/` +
      detail([...result.added, ...result.updated, ...result.removed]),
  )
  return result
}

/** 源目录盯梢：改了（含新增 / 删除）就对账一次，让 dev 里的图立刻跟上 */
function watchSource(server, repoRoot) {
  const src = join(repoRoot, SOURCE_ROOT_REL)
  if (!existsSync(src)) return
  server.watcher.add(src)
  let timer = null
  const onEvent = (file) => {
    const full = resolve(file)
    if (full !== src && !full.startsWith(src + sep)) return
    clearTimeout(timer)
    timer = setTimeout(() => {
      try {
        runSync(repoRoot, server.config.logger)
      } catch (e) {
        // 盯梢期间的失败不该把 dev server 弄死：报出来，改回去或补上文件即恢复
        server.config.logger.error(`[assets] 随包同步失败：${e.message}`)
      }
    }, 60)
  }
  for (const ev of ['add', 'change', 'unlink']) server.watcher.on(ev, onEvent)
}

/** 一个根目录里那个文件（`/assets/<rel>` 的 `<rel>`）。越界一律拒 */
function resolveRequest(root, req) {
  const rel = decodeURIComponent(new URL(req.url, 'http://127.0.0.1').pathname).replace(/^\/+/, '')
  if (rel === '' || rel.includes('\\')) return { error: [400, '空路径或反斜杠'] }
  if (rel.split('/').some((s) => s === '' || s === '.' || s === '..')) {
    return { error: [400, '路径里有空段或 ..'] }
  }
  const full = resolve(root, rel)
  // 顺序要紧：**先问"在不在"，再问"越没越界"** —— `isInside` 走的是 realpath，
  // 而 realpath 对不存在的路径会抛（于是"没这个文件"会被错报成"越界"）。
  // 反过来的风险没有：真的存在、只是指向根外的符号链接，下面那道照样拦。
  if (!existsSync(full) || !statSync(full).isFile()) return { error: [404, `没有这个文件：${rel}`] }
  // 比真实路径（防符号链接绕过）——与 Rust 侧 resolve_in 同一句话
  if (!isInside(root, full)) return { error: [403, '越出资产根'] }
  return { full }
}

/**
 * `/assets/*` 中间件：**读一个目录**，不落盘。
 *
 * 每次请求都回那个根取字节 —— 源改了刷新即见。这是无鉴权端点，所以四道闸都在
 * （方法 / 路径形状 / realpath 收口 / 必须是文件）。传进来的根由调用方决定
 * （客户端 = 交付根，工作台 = 源目录，见文件头）。
 */
function serveAssetsFrom(server, root) {
  server.middlewares.use(ASSETS_URL_PREFIX.slice(0, -1), (req, res, next) => {
    if (req.method !== 'GET' && req.method !== 'HEAD') return next()
    const { full, error } = resolveRequest(root, req)
    if (error !== undefined) {
      res.statusCode = error[0]
      res.setHeader('Content-Type', 'text/plain; charset=utf-8')
      return res.end(`${error[1]}\n`)
    }
    res.statusCode = 200
    res.setHeader('Content-Type', MIME[extname(full).toLowerCase()] ?? 'application/octet-stream')
    res.setHeader('Cache-Control', 'no-store')
    if (req.method === 'HEAD') return res.end()
    return res.end(readFileSync(full))
  })
}

/** 这一趟要装配进 dist 的那批：客户端只随包，工作台要全部有 `path` 的 */
function assemblyItems(repoRoot, workbench) {
  return readLedger(repoRoot).filter(
    (a) => a.path !== '' && (workbench || a.delivery === 'bundled'),
  )
}

/**
 * 插件本体。`workbench` = 这一趟构建/服务是不是工作台那一个前端
 * （`vite --mode workbench` 或 `BUILD_WORKBENCH=1`，与 `vite.config.ts` 同一处开关）。
 */
export function mkpAssets({ workbench }) {
  let repoRoot = process.cwd()
  let outDir = ''
  let command = 'serve'

  return {
    name: 'mkp-assets',

    configResolved(config) {
      repoRoot = config.root
      outDir = config.build.outDir
      command = config.command
      /* 同步放在这里：**任何模式都恰好跑一次**，且在任何消费者（中间件 / 装配 / import）
         之前 —— 不需要 buildStart / configureServer 各来一遍那种"看情况补一次"的写法 */
      runSync(repoRoot, config.logger)
    },

    configureServer(server) {
      watchSource(server, repoRoot)
      const root = workbench
        ? join(repoRoot, SOURCE_ROOT_REL)
        : join(repoRoot, OUTPUT_ROOT_REL, OUTPUT_ASSETS_SUBDIR)
      serveAssetsFrom(server, root)
    },

    closeBundle() {
      /* 只对"真构建 + 这个命令"这一格装配：dev 时 outDir 还不该被碰 */
      if (command !== 'build') return
      const items = assemblyItems(repoRoot, workbench)
      const dest = join(outDir, 'assets')
      const n = copyAssetsInto(repoRoot, dest, items)
      const what = workbench ? '工作台预览：全部有 path 的登记资产' : '客户端随包：delivery = bundled'
      this.info?.(`[assets] 装配 ${n} 份（${what}）→ ${dest}`)
    },
  }
}

/** 给外部（判据 / 排查）用：这一条链认的三个位置 */
export const paths = { ledger: LEDGER_REL, source: SOURCE_ROOT_REL, output: OUTPUT_ROOT_REL }
