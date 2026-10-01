/**
 * 本机 BBS 工艺预设的**只读**端点（只在 serve 期生效：dev 与 preview 都挂）。
 * 搬自试验场 `tools/dev-server/bbsFs.mjs`（它又搬自 machine-motion/vite.config.js 的
 * `mm-bbs-presets`），安全边界一条没删。
 *
 * # 为什么读本机目录
 *
 * BBS 预设页要能列出「你此刻在 Bambu Studio 里有哪些预设」—— 你在 BBS 里另存一份，
 * 刷新页面就该看见。**本仓不打包那 285 个预设快照**（作者裁决 §6-2），所以本机目录是
 * 清单的唯一来源；读不到就是空态，不静默退到一份陈旧快照。
 * 页面另外要的四份元数据（registry / layout / defaults / icons）不在此列 —— 那是提取器
 * 的产物，随本仓分发，见 `public/bbs/PROVENANCE.md`。
 *
 * # 为什么 preview 也要挂
 *
 * `npm run dev` 在本机会被 `target/` 那两棵构建树（5.8 万文件）拖死文件监听，HTTP 全超时；
 * 验收一律走 `build` + `preview`（阶段账债 #4）。只挂 `configureServer` 的话，端点在验收
 * 那台服务器上根本不存在，探针就验不了「278 份那一档」。两个钩子共用同一段 handler。
 *
 * 两个端点：
 *   GET /api/bbs/presets              → { available, root, user[], system[] }
 *   GET /api/bbs/preset?scope&uid&file → 那一份 json 的**原文透传**
 *
 * 安全边界（这是个**无鉴权**端点，下面每条都别删）：
 *   - 只有 GET，没有任何写口；非 GET 直接交给下一个中间件
 *   - 路径不由客户端拼：客户端只能给 scope（user|system）+ uid（必须在扫到的列表里）
 *     + file（必须等于自身的 basename、必须 .json）。服务端自己 join，join 完再确认
 *     结果仍在白名单目录里 —— 双保险，防 `..%5c` 这类绕过
 *   - 单文件 1 MB 上限；目录列表不递归
 *   - `apply: 'serve'` 保证 build 产物里根本不存在这个端点
 *
 * ⚠️ 产品仓的 dev server 平时不绑网卡（`server.host` 只在 `TAURI_DEV_HOST` 注入时才开），
 * 比试验场那份 `host: true` 收敛。但真要绑网卡给同网段（iPad / 真机）看时，这个读口会
 * 跟着暴露 —— 它只读、只能读 BBS 预设目录下的 .json，仍是**访客 Wi-Fi 下要停掉**的东西。
 *
 * 为什么不加缓存：总量不到 1 MB，每次重扫几十毫秒；加了缓存反而会出现
 * 「BBS 里存了新预设但页面看不到」——那正是这个端点要解决的问题。
 *
 * 写成 .mjs：Node 侧代码不进 `src/` 那棵 tsc 树，vite.config.ts 只认同名的 `bbsFs.d.mts`。
 */
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs'
import { basename, join, resolve, sep } from 'node:path'

const BBS_APPDIRS = ['BambuStudio', 'BambuStudioBeta']   // 正式版优先，找不到才退 Beta
const MAX_PRESET_BYTES = 1024 * 1024

const json = (res, code, body) => {
  res.statusCode = code
  res.setHeader('Content-Type', 'application/json; charset=utf-8')
  res.setHeader('Cache-Control', 'no-store')
  res.end(JSON.stringify(body))
}

function bbsLayout() {
  const appdata = process.env.APPDATA
  if (!appdata) {
    return { available: false, reason: '环境变量 APPDATA 不存在（这端点只认 Windows 上的 BBS）' }
  }
  for (const app of BBS_APPDIRS) {
    const root = join(appdata, app)
    const system = join(root, 'system', 'BBL', 'process')
    if (!existsSync(system)) continue
    /* user 下按账号分目录（数字 uid，另有 BBL / default 这种），只认里面真有 process/ 的 */
    const userBase = join(root, 'user')
    const uids = existsSync(userBase)
      ? readdirSync(userBase, { withFileTypes: true })
        .filter((e) => e.isDirectory() && existsSync(join(userBase, e.name, 'process')))
        .map((e) => e.name)
      : []
    return { available: true, app, root, system, userBase, uids }
  }
  return {
    available: false,
    reason: `%APPDATA% 下没找到 ${BBS_APPDIRS.join(' / ')}\\system\\BBL\\process`,
  }
}

function scanDir(dir, scope, uid) {
  const out = []
  if (!existsSync(dir)) return out
  for (const f of readdirSync(dir)) {
    if (!f.toLowerCase().endsWith('.json')) continue
    const full = join(dir, f)
    let st
    let doc = null
    let broken = null
    try { st = statSync(full) } catch { continue }
    try { doc = JSON.parse(readFileSync(full, 'utf8')) }
    catch (e) { broken = e.message }      // 坏文件也要列出来，让前端标灰而不是悄悄少一项
    out.push({
      scope,
      uid: uid || null,
      file: f,
      name: doc?.name || f.replace(/\.json$/i, ''),
      inherits: doc?.inherits || null,
      instantiation: doc?.instantiation ?? null,
      compatible_printers: doc?.compatible_printers ?? null,
      from: doc?.from || null,
      mtime: st.mtimeMs,
      size: st.size,
      broken,
    })
  }
  return out
}

/** 两个钩子（dev 的 configureServer / preview 的 configurePreviewServer）共用这一段 */
function install(server) {
  server.middlewares.use('/api/bbs', (req, res, next) => {
    if (req.method !== 'GET') return next()
    const url = new URL(req.url, 'http://127.0.0.1')
    const lay = bbsLayout()

    if (url.pathname === '/presets') {
      if (!lay.available) return json(res, 200, { available: false, reason: lay.reason })
      const user = lay.uids.flatMap((uid) => scanDir(join(lay.userBase, uid, 'process'), 'user', uid))
      const system = scanDir(lay.system, 'system', null)
      return json(res, 200, {
        available: true,
        app: lay.app,
        root: lay.root,
        scanned_at: new Date().toISOString(),
        user,
        system,
      })
    }

    if (url.pathname === '/preset') {
      if (!lay.available) return json(res, 404, { error: lay.reason })
      const scope = url.searchParams.get('scope')
      const uid = url.searchParams.get('uid')
      const file = url.searchParams.get('file') || ''
      if (scope !== 'user' && scope !== 'system') {
        return json(res, 400, { error: 'scope 只能是 user 或 system' })
      }
      if (!file.toLowerCase().endsWith('.json') || file !== basename(file)) {
        return json(res, 400, { error: 'file 必须是单个 .json 文件名，不接受路径' })
      }
      let dir
      if (scope === 'system') dir = lay.system
      else {
        if (!uid || !lay.uids.includes(uid)) {
          return json(res, 400, { error: 'uid 不在已扫到的账号目录里' })
        }
        dir = join(lay.userBase, uid, 'process')
      }
      const full = resolve(dir, file)
      /* join 完再确认一次：resolve 之后仍必须在白名单目录之下 */
      if (!full.startsWith(resolve(dir) + sep)) return json(res, 400, { error: '路径越界' })
      if (!existsSync(full)) return json(res, 404, { error: `没有这个文件：${file}` })
      const st = statSync(full)
      if (st.size > MAX_PRESET_BYTES) {
        return json(res, 413, { error: `文件过大：${st.size} > ${MAX_PRESET_BYTES}` })
      }
      let text
      try { text = readFileSync(full, 'utf8') }
      catch (e) { return json(res, 500, { error: `读文件失败：${e.message}` }) }
      res.statusCode = 200
      res.setHeader('Content-Type', 'application/json; charset=utf-8')
      res.setHeader('Cache-Control', 'no-store')
      /* 原文透传，解析交给前端（坏 JSON 要由前端如实报错） */
      return res.end(text)
    }

    return next()
  })
}

export function bbsFs() {
  return {
    name: 'bbs-fs',
    apply: 'serve',
    configureServer(server) { install(server) },
    configurePreviewServer(server) { install(server) },
  }
}