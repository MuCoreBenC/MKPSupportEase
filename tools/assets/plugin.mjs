/*
 * 随包资产流水线的 **vite 入口** —— 这一刀里所有资产动作都只从这里发生。
 *
 * # 为什么必须是插件，而不是 npm script 前置
 *
 * 前置写法（`node xxx && tsc -b && vite build`）只在**你记得加的那三条命令**上成立：
 * 裸 `npx vite build`、探针那次构建、`dev:workbench` 都会漏，于是"资产在不在"
 * 又变成"上一次谁跑过什么"。挂在插件上则相反 —— **任何走 vite 的命令都经过同一段代码**，
 * 包括将来新增的入口。所以三条 npm script 的前置这一刀全部删掉。
 *
 * # 三个动作
 *
 * ```text
 * configResolved   →  同步一次（所有模式）：台账 bundled 条目 → client-assets/
 * configureServer  →  dev：盯住源目录，改了重新对账；
 *                     工作台模式另挂 /assets/* 中间件 —— **直读 presets/assets**，
 *                     不落盘、不生成 public/assets（那是上一版的错，已退役）
 * closeBundle      →  build：工作台模式把**全部有 path 的台账条目**装配进 dist/assets/
 * ```
 *
 * # 两个集合不同是**有意的**
 *
 * - `client-assets/`：只有 `delivery = 'bundled'`（今天 4 张整机图）——
 *   语义只有一句「客户端随包资产交付结果」。
 * - 工作台 dist 里那一份：全部有 `path` 的条目（今天 19 份）—— 那是开发工具为了
 *   **预览任何登记过的资产**（含 cloud 档）而铺的，随工作台构建重建，不是交付物。
 *
 * 两者共用同一条 URL 规则 `/assets/<path>`（与后端 `workbench/app/assets.rs` 那处前缀同值），
 * 但**不要让同步器偷偷承担后者的职责** —— 那会把"随包"这个词弄脏。
 *
 * # 客户端构建这一趟不做"装配"
 *
 * 客户端的随包图经 `@client-assets/...` 静态 import 进 vite 资源管线（`heroArt.ts`），
 * vite 自己把它们发成 `dist/assets/<name>-<hash>.webp`。所以客户端**不再**额外铺一份
 * 不带头哈希的副本 —— 同一份字节在包里出现两次是这一刀不该留下的东西。
 * （第二刀读取改成台账驱动之后，import 消失，那时才需要在 dist 里铺 URL 同形的树。）
 */
import { existsSync, readFileSync, statSync } from 'node:fs'
import { extname, join, resolve, sep } from 'node:path'

import {
  LEDGER_REL,
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

/** 源目录里那个文件（`/assets/<path>` 的 `<path>`）。越界一律拒 */
function resolveRequest(src, req) {
  const rel = decodeURIComponent(new URL(req.url, 'http://127.0.0.1').pathname).replace(/^\/+/, '')
  if (rel === '' || rel.includes('\\')) return { error: [400, '空路径或反斜杠'] }
  if (rel.split('/').some((s) => s === '' || s === '.' || s === '..')) {
    return { error: [400, '路径里有空段或 ..'] }
  }
  const full = resolve(src, rel)
  // 顺序要紧：**先问"在不在"，再问"越没越界"** —— `isInside` 走的是 realpath，
  // 而 realpath 对不存在的路径会抛（于是"没这个文件"会被错报成"越界"）。
  // 反过来的风险没有：真的存在、只是指向根外的符号链接，下面那道照样拦。
  if (!existsSync(full) || !statSync(full).isFile()) return { error: [404, `没有这个文件：${rel}`] }
  // 比真实路径（防符号链接绕过）——与 Rust 侧 resolve_in 同一句话
  if (!isInside(src, full)) return { error: [403, '越出资产根'] }
  return { full }
}

/**
 * 工作台的 `/assets/*`：**读取源，不是同步目标**。
 *
 * 每次请求都回 `presets/assets` 取字节 —— 源改了刷新即见，且一个字节都不落盘。
 * 这是无鉴权端点，所以四道闸都在（方法 / 路径形状 / realpath 收口 / 必须是文件）。
 */
function serveAssetsFromSource(server, repoRoot) {
  const src = join(repoRoot, SOURCE_ROOT_REL)
  server.middlewares.use(ASSETS_URL_PREFIX.slice(0, -1), (req, res, next) => {
    if (req.method !== 'GET' && req.method !== 'HEAD') return next()
    const { full, error } = resolveRequest(src, req)
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
      /* 同步放在这里：**任何模式都恰好跑一次**，且在任何消费者（import / publicDir / 中间件）
         之前 —— 不需要 buildStart / configureServer 各来一遍那种"看情况补一次"的写法 */
      runSync(repoRoot, config.logger)
    },

    configureServer(server) {
      watchSource(server, repoRoot)
      if (workbench) serveAssetsFromSource(server, repoRoot)
    },

    closeBundle() {
      /* 只对「工作台 + 真构建」这一格装配：dev 时 outDir 还不该被碰 */
      if (command !== 'build' || !workbench) return
      const items = readLedger(repoRoot).filter((a) => a.path !== '')
      const dest = join(outDir, 'assets')
      const n = copyAssetsInto(repoRoot, dest, items)
      this.info?.(`[assets] 工作台装配：${n} 份登记资产 → ${dest}`)
    },
  }
}

/** 给外部（判据 / 排查）用：这一刀认的台账路径与交付根 */
export const paths = { ledger: LEDGER_REL, source: SOURCE_ROOT_REL, output: OUTPUT_ROOT_REL }
