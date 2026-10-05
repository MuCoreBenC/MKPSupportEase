/**
 * 官方预设数据发布 —— `npm run publish:presets`。
 *
 * ★ **状态：清算中（`docs/RESOURCE-ADDRESSING-ROADMAP.md` M5）** —— 这是 wb_publish
 * 之外的**第二个发布出口**，长期目标是退役或改调 `wb_publish` 内核；在那之前它
 * 跟随新交付目录 `presets/delivery/`。新的一律走工作台的「发布」。
 *
 * # 它是什么、不是什么（第十八刀）
 *
 * 这是**预设数据**的发布，**不是 SupportEase 软件版本的发布**。两者是两条独立的线：
 *
 *   `npm run release`         → 软件版本发布：改版本号 → PR → CI → squash → tag `vX.Y.Z`
 *   `npm run publish:presets` → 预设数据发布：把 `presets/delivery/` 直接送进 main
 *
 * **刻意不共用 `release.mjs`**：共用的话，每更新一份预设 TOML 都会把软件版本体系
 * （版本号、tag、CI 全绿）一起卷进来 —— 那是把两件事绑成了一件事。
 *
 * # 为什么走 PR 而不是直接推 main
 *
 * 本仓库是 PR-only：闸②（`scripts/hooks/pre-push`）**无条件**拒绝本地推 main，
 * GitHub 的 main ruleset 再拦一道。这不是麻烦，是既定纪律 ——
 * 所以这个脚本**不碰那道闸门**，而是老老实实开一个 PR。
 *
 * # 为什么要一次性分支
 *
 * 我们定死"最终状态只在 main"：`presets/delivery/` 是 main 上的**正式交付目录**，
 * 客户端从 `raw.githubusercontent.com/.../main/presets/delivery/source.json` 读。
 * 所以**不需要**一个长期存在的 `preset-dist` 分支 —— 那种分支会多出一份要维护的状态
 * （跟 main 同步、被误删、别人要理解两套历史）。这里每次发布临时开一个分支，
 * PR 合并后由 GitHub 删掉（`--delete-branch` 或你手动），**自然消失**。
 *
 * # 发布前必须全过（否则一个字节都不 git add）
 *
 * `presets/delivery/` 是 `wb_publish` 的产物，但脚本**不信任它"应该是对的"**：
 * 发布出去的字节就是用户下载的字节，所以逐条核对：
 *
 *   ① 产物存在且非空
 *   ② `source.json` 认得（代次 + `catalog` 指向的文件真存在）
 *   ③ `catalog.json` 认得（schema + `files[]` 每条 `path` 真存在）
 *   ④ **SHA / 大小** —— catalog 登记的 `sha256` / `size` 与盘上真字节逐一对
 *   ⑤ `manifest.json` 与 catalog 交叉核对（防半成品：TOML 生成了、说明书没发）
 *
 * 这五条与 `publish_into` 收尾那段是**同一批判据**（那一段在工作台侧守、这一段在
 * 进库前守）：两边都错才可能放过去一份坏产物。④是核心 —— 客户端下载后拿 catalog
 * 的 `sha256` 当期望值校验，发布时先对一遍真字节，就不至于让用户在机器上撞 404 / 校验失败。
 */

import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { existsSync, readFileSync, statSync } from 'node:fs'
import { join } from 'node:path'
import { createInterface } from 'node:readline/promises'
import { stdin as input, stdout as output } from 'node:process'

/* ---------- 小工具（与 release.mjs 同一套形状，保持两个脚本读起来一样） ---------- */

let ROOT = process.cwd()

function run(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { encoding: 'utf8', cwd: opts.cwd ?? ROOT }).trim()
}

function runLive(cmd, args, opts = {}) {
  execFileSync(cmd, args, { stdio: ['ignore', 'inherit', 'inherit'], cwd: opts.cwd ?? ROOT })
}

function tryRun(cmd, args, opts = {}) {
  try {
    return run(cmd, args, opts)
  } catch {
    return null
  }
}

/**
 * 同 `run`，但**不做 `.trim()`** —— 给需要看原字节的场合用（如 porcelain）。
 *
 * 存在的理由：`run()` 会 trim，而 `git status --porcelain` 的每行是 `XY<space>path`，
 * 首字符可能是空格（` M path` = 工作区已改）。trim 掉那个空格后按"前 3 字符是状态码"
 * 的偏移去切，就会切掉路径首字母 —— 过滤静默失效，测试里表现为"改过 dist 却被判成
 * 工作区脏"。这个坑真的踩到过一次。
 */
function runRaw(cmd, args, opts = {}) {
  return execFileSync(cmd, args, { encoding: 'utf8', cwd: opts.cwd ?? ROOT })
}

ROOT = run('git', ['rev-parse', '--show-toplevel'])

const step = (m) => console.log(`\n\u001b[36m▶ ${m}\u001b[0m`)
const ok = (m) => console.log(`  \u001b[32mok\u001b[0m    ${m}`)
const info = (m) => console.log(`        ${m}`)

function die(message, rollback = []) {
  console.error(`\n\u001b[31m停在这里：${message}\u001b[0m`)
  if (rollback.length) {
    console.error('\n  要回到发布前的状态，按顺序跑（自己确认再跑，脚本不替你执行）：')
    for (const line of rollback) console.error(`    ${line}`)
  }
  console.error('')
  process.exit(1)
}

/** 交付根（相对仓库根）。.gitignore 不再忽略它 —— 它就是 main 上的正式交付目录 */
const DIST_REL = 'presets/delivery'
const DIST = join(ROOT, DIST_REL)
/**
 * ★ **发布根**（相对仓库根）= `catalog.path` 的基准（2026-10-04 裁决 C/甲）。
 *
 * `catalog.path`（与 manifest 的 `relativePath`）**同时是"云端取哪"与"客户端放哪"**：
 *
 *   A 类（BBS / 模型 / 图标）  `assets/…`          → 原地住在 `presets/assets/…`（不复制进 dist）
 *   B 类（MKP 渲染产物）       `dist/mkp/presets/…` → 住在 `presets/delivery/mkp/presets/…`
 *
 * 所以**校验交付文件时一律以发布根 `presets/` 为基准**，不是 `presets/delivery/`。
 * 唯一例外是 `source.json` 的 `catalog` 字段：它的语义是"相对 source.json 所在目录"，
 * 那一处仍以 `DIST` 为基准。
 */
const PUBLISH_REL = 'presets'
const PUBLISH = join(ROOT, PUBLISH_REL)
/** 与 dist.rs 的常量同值。改这里必须同时改 Rust 侧 —— 但它们是**契约**，不是偏好 */
const CATALOG_FILE = 'catalog.json'
const SOURCE_FILE = 'source.json'
const MANIFEST_FILE = 'manifest.json'

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex')

const readJson = (rel) => {
  const full = join(DIST, rel)
  if (!existsSync(full)) return null
  try {
    return JSON.parse(readFileSync(full, 'utf8'))
  } catch (e) {
    die(`${rel} 不是合法 JSON：${e.message}`)
  }
}

/* ================================================================================
 * 1. 产物存在
 * ============================================================================== */

step('检查交付产物')

if (!existsSync(DIST) || !statSync(DIST).isDirectory()) {
  die(
    `${DIST_REL}/ 不存在。\n` +
      '    这是一份**发布产物**，先在「工作台 → 生成 → 发布」里产出它，再回来发。',
  )
}

const source = readJson(SOURCE_FILE)
const catalog = readJson(CATALOG_FILE)
const manifest = readJson(MANIFEST_FILE)

/* 三个文件各自缺了，报的是它们分别代表什么没做 —— 不是干巴巴一句"文件不存在" */
if (source === null) {
  die(
    `${DIST_REL}/${SOURCE_FILE} 不存在。\n` +
      '    它是 Bootstrap（客户端拿它找 catalog）。说明工作台里**还没点过「发布」** ——\n' +
      '    `wb_generate` 只生成了 mkp/ 里的 TOML，`wb_publish` 才写这三个文件。',
  )
}
if (catalog === null) {
  die(`${DIST_REL}/${CATALOG_FILE} 不存在。工作台的「发布」没走完，或产物被删过一半`)
}
if (manifest === null) {
  die(`${DIST_REL}/${MANIFEST_FILE} 不存在（发布账）。同样说明「发布」没走完`)
}

/* ================================================================================
 * 2. source.json（Bootstrap）
 * ============================================================================== */

step(`校验 ${SOURCE_FILE}（Bootstrap）`)

if (typeof source.sourceSchema !== 'number') {
  die(`${SOURCE_FILE} 缺 sourceSchema（代次），客户端解析不了`)
}
if (typeof source.catalog !== 'string' || source.catalog.trim() === '') {
  die(`${SOURCE_FILE} 的 catalog 字段为空 —— 客户端不知道去哪读目录`)
}
/* `catalog` 可以是绝对 URL（换部署到别的 CDN 时）；相对值时必须在交付根里真存在 */
const catalogTarget = source.catalog.trim()
if (/^https?:\/\//.test(catalogTarget)) {
  info(`catalog 指向外部地址（${catalogTarget}）—— 本脚本只校验本地那一份，跳过存在性检查`)
} else if (catalogTarget !== CATALOG_FILE && !existsSync(join(DIST, catalogTarget))) {
  die(`${SOURCE_FILE} 指向 ${catalogTarget}，但交付根里没有这个文件`)
}
ok(`${SOURCE_FILE}：sourceSchema=${source.sourceSchema}，catalog=${catalogTarget}`)

/* ================================================================================
 * 3. catalog.json
 * ============================================================================== */

step(`校验 ${CATALOG_FILE}`)

if (typeof catalog.catalogSchema !== 'number') {
  die(`${CATALOG_FILE} 缺 catalogSchema`)
}
if (typeof catalog.revision !== 'string' || catalog.revision === '') {
  die(`${CATALOG_FILE} 缺 revision（目录指纹）—— 客户端靠它判断"远端有没有变化"`)
}
if (!Array.isArray(catalog.files)) {
  die(`${CATALOG_FILE} 的 files 不是数组`)
}

const files = catalog.files
info(`revision=${catalog.revision}　登记 ${files.length} 条文件`)

/* `path` 的形状校验（每条都要）：非空、不许 `..` 逃出交付根。
   这是发布侧的输入校验，与客户端 `dest_of_asset` 的防穿越是同一件事的两端。 */
for (const f of files) {
  if (typeof f?.path !== 'string' || f.path.trim() === '') {
    die(`${CATALOG_FILE} 有一条 files[] 缺 path`)
  }
  if (f.path.replace(/^\/+/, '').split('/').includes('..')) {
    die(`${CATALOG_FILE} 的 path 里有 ..（越出交付根）：${f.path}`)
  }
}
ok(`${CATALOG_FILE}：${files.length} 条 path 形状合法（非空、无 ..）`)

/*
 * **存在性判据只对「进交付的那批」要求 —— 以 manifest 为准，不是 catalog。**
 *
 * 这一点容易搞反（我第一版就搞反了）：catalog 登记的是**整个目录**（客户端拿它认识
 * 所有资产，含机型图标、切片器配置、3mf 模型），而**进交付的是被引用可达的那批**
 * （`dist.rs` 的可达性收窄：登记 15 条资产、只发被引用的 8 条 —— "登记得比发得多"
 * 是设计，不是漏洞）。所以：
 *
 *   · 只在 catalog、不在 manifest 的条目 → **允许不发**（客户端够不着它）；
 *   · manifest 里的每一条 → **必须**在**发布根**下真存在（客户端够得着，缺了就是 404）。
 *
 * 判据写错的表现：发布一份**完全正常**的产物，脚本却报
 * "catalog 登记了 mkp/models/xxx.3mf，但交付根里没有这份文件" —— 而那份模型本来就
 * 没进任何套餐。真踩到过：首次发布被这条误拦。
 *
 * ★ 2026-10-04 起基准是**发布根 `presets/`**（`PUBLISH`），不是交付根 `presets/delivery/`：
 * manifest 的 `relativePath` 与 `catalog.path` 同值、同基准（A 类 `assets/…` 不在 dist 里）。
 */
const manifestPaths = new Set(
  manifest.assets.map((a) => String(a?.relativePath ?? '').replace(/^\/+/, '')).filter((p) => p !== ''),
)
let missingDeliverables = []
for (const rel of manifestPaths) {
  if (!existsSync(join(PUBLISH, rel))) missingDeliverables.push(rel)
}
if (missingDeliverables.length > 0) {
  die(
    `${MANIFEST_FILE} 里有 ${missingDeliverables.length} 条文件不在交付根里：\n` +
      missingDeliverables.map((p) => `      ${p}`).join('\n') +
      '\n    这些是**要发出去**的文件，客户端会按「数据源地址 + path」去取 → 用户点下载会 404',
  )
}
ok(`${MANIFEST_FILE}：${manifestPaths.size} 条交付文件全部在发布根里真存在`)

/* ================================================================================
 * 4. manifest 与 catalog 交叉核对（防半成品 / 防两账脱节）
 * ============================================================================== */

step(`交叉核对 ${CATALOG_FILE} 与 ${MANIFEST_FILE}`)

if (!Array.isArray(manifest.assets)) {
  die(`${MANIFEST_FILE} 的 assets 不是数组`)
}
const catalogPaths = new Set(files.map((f) => f.path.replace(/^\/+/, '')))
const missingInCatalog = [...manifestPaths].filter((p) => !catalogPaths.has(p))
if (missingInCatalog.length > 0) {
  die(
    `${MANIFEST_FILE} 里有 ${missingInCatalog.length} 条不在 ${CATALOG_FILE} 里：\n` +
      missingInCatalog.map((p) => `      ${p}`).join('\n') +
      '\n    发布账与说明书说的不是同一批文件 —— 这是一份半成品，不能发',
  )
}
ok(`${MANIFEST_FILE}：${manifestPaths.size} 条，全部在 catalog 里（两账一致）`)

/* ================================================================================
 * 5. SHA / 大小（核心：发布出去的字节 = catalog 说的字节）
 * ============================================================================== */

step('校验 SHA / 大小（catalog ↔ 盘上真字节）')

/*
 * **对 catalog ∩ 交付的那批逐条核**。catalog 里"登记但不发"的条目（可达性收窄掉的
 * 3mf / 0.2mm 切片器配置）跳过 —— 盘上根本没有它们，读会直接崩。
 * 每条都查：catalog 的 sha256 / size 是不是盘上真字节算出来的。
 */
const deliverableFiles = files.filter((f) => manifestPaths.has(f.path.replace(/^\/+/, '')))
if (deliverableFiles.length !== manifestPaths.size) {
  /* 只有 manifest 有、catalog 没有的情况上面已经拦了，这里兜底 */
  die(`交付集合里有 ${manifestPaths.size - deliverableFiles.length} 条在 catalog 里找不到 sha256 —— 说明书不全`)
}

let checked = 0
for (const f of deliverableFiles) {
  const rel = f.path.replace(/^\/+/, '')
  /* 发布根基准（A 类 `assets/…` 不在 dist 里）—— 与上面那条存在性检查同一个锚点 */
  const bytes = readFileSync(join(PUBLISH, rel))
  if (typeof f.size !== 'number' || bytes.length !== f.size) {
    die(`${f.path} 大小对不上：catalog 说 ${f.size}，盘上是 ${bytes.length}`)
  }
  const actual = sha256(bytes)
  if (typeof f.sha256 !== 'string' || actual !== f.sha256.toLowerCase()) {
    die(
      `${f.path} SHA 对不上：\n    catalog：${f.sha256}\n    盘上　：${actual}\n` +
        '    这份字节与说明书说的不是同一份 —— 重新在「生成」里生成再「发布」',
    )
  }
  checked += 1
}
ok(`${checked} 条交付文件的 SHA / 大小都对得上`)

/* ================================================================================
 * 6. 开一次性发布分支 → 提交 → 推 → 开 PR
 * ============================================================================== */

step('工作区与分支')

const branch = tryRun('git', ['symbolic-ref', '--short', 'HEAD'])
if (!branch) die('当前是 detached HEAD。先切回 main 再来发')

/* **必须在 main 的 tip 上发**：PR 的 base 是 main，而从一条落后 / 带别的改动的分支发，
   PR 里就会混进不属于这次发布的东西。发布是"把当前 main 的预设源生成的那份产物送上去"，
   所以起点就是 main 本身。 */
if (branch !== 'main') {
  die(
    `当前在 ${branch} 上，不是 main。\n` +
      '    预设发布从 main 起：main 的 tip = 这次发布的产物对应的预设源（一次性分支由脚本建）。\n' +
      `    先回去：git switch main`,
  )
}

/* 「干净」＝**除 `presets/delivery/` 之外**没有别的改动。
   必须把 dist 排除在外：首次发布时它本来就是未跟踪的（正被忽略惯了的目录第一次现身），
   拿它自身的未跟踪状态去拦自己的首次发布，就是个死锁。

   解析用 `-z`（NUL 分隔）而不是按行切：porcelain 的行格式是 `XY<space>path`，
   首字符可能是空格（` M path`），**必须用不 trim 的 `runRaw` 读** —— 用 `run()` 的话
   行首空格被吃掉，再按"前 3 字符是状态码"去切就会切掉路径首字母
   （`presets/…` → `resets/…`），过滤静默失效。`-z` 本就该配不 trim 的读法。
   这个坑真的踩到过一次：测试里表现为"改过 dist 却被判成工作区脏"。 */
const dirtyRaw = runRaw('git', ['status', '--porcelain=v1', '-z', '--untracked-files=all'])
const dirty = dirtyRaw
  .split('\0')
  .filter((entry) => entry !== '')
  .map((entry) => {
    /* 每条 `XY<space>path`（重命名是 `XY<space>new\0old` —— 前一拍已经 split 掉了 old） */
    const path = entry.slice(3)
    return path
  })
  .filter((path) => !(path === DIST_REL || path.startsWith(`${DIST_REL}/`)))
if (dirty.length > 0) {
  die(
    '工作区不干净（除 presets/delivery/ 之外还有改动）。先提交或 stash。\n' +
      '    （**不要**顺手把在途的依赖版本文件带进来：那是另一件事）\n' +
      dirty.map((l) => `      ${l}`).join('\n'),
  )
}
ok('在 main 上，除 presets/delivery/ 之外工作区干净')

/* 说明：命令行给了就不问（`npm run publish:presets -- "一句话"`），与 release.mjs 同形 ——
   非交互也能跑，才可能进自动化 */
let summary = (process.argv[2] ?? '').trim()
if (!summary) {
  const rl = createInterface({ input, output })
  summary = (await rl.question('  这次预设发布的一句话说明（会成为 PR 标题）: ')).trim()
  rl.close()
  if (!summary) die('说明不能为空 —— 它是 PR 标题，也是以后 git log 上唯一能看到的东西')
}
const title = `chore(presets): ${summary}`

const stamp = new Date()
  .toISOString()
  .slice(0, 19)
  .replace(/[-:T]/g, '')
const publishBranch = `chore/publish-presets-${stamp}`

step('开一次性发布分支并提交')

/* 先建分支、再 add —— 这样"没有差异"这个失败发生时，人还站在干净的分支上，
   不留下一个空的暂存区（在 main 上 add 了又 die 的话，工作区会半脏） */
runLive('git', ['switch', '-c', publishBranch])
runLive('git', ['add', '-f', DIST_REL])

const staged = run('git', ['diff', '--cached', '--name-only'])
if (staged === '') {
  /* 回到 main 再报告：空分支不该留着（否则下次发布撞名、或者你忘了它在这儿） */
  run('git', ['switch', branch])
  run('git', ['branch', '-D', publishBranch])
  die(
    `${DIST_REL}/ 与 main 没有差异 —— 没有新东西要发。\n` +
      '    （刚建的临时分支已删掉，你还在 main 上。）\n' +
      '    通常是正常的：你还没重新在「生成 → 发布」里产出新产物。',
  )
}
info(`待提交：${staged.split('\n').length} 个文件`)

runLive('git', ['commit', '-m', title])
ok(`${publishBranch} 上已提交`)

step('推分支并开 PR')

runLive('git', ['push', '-u', 'origin', 'HEAD'])
const prUrl = run('gh', [
  'pr',
  'create',
  '--title',
  title,
  '--body',
  [
    '预设数据发布（`npm run publish:presets`）。',
    '',
    `- 交付根：\`${DIST_REL}/\``,
    `- catalog revision：\`${catalog.revision}\``,
    `- 文件：${files.length} 条（SHA / 大小已逐一核对）`,
    '',
    '合并后 main 上的 `presets/delivery/` 即为客户端读取的官方交付目录。',
  ].join('\n'),
])
ok(`PR 已开：${prUrl}`)

console.log(`
\u001b[32m发上去了（等你合并那个 PR）：\u001b[0m

  分支：${publishBranch}
  PR　：${prUrl}

  合并后（走正常 PR 合并，或你自己在网页上合），main 上的
  \`${DIST_REL}/${SOURCE_FILE}\` 就是客户端 Bootstrap 的落点：

    https://raw.githubusercontent.com/<owner>/<repo>/main/${DIST_REL}/${SOURCE_FILE}

  **这一刀不碰 main 直推闸门**：合并是 PR 那一步的事，脚本只负责把产物送到 PR。
  合完这个分支会被 GitHub 删掉（if 你合的时候勾了）；它本来就是一次性的。

  回到 work：
    git switch ${branch} && git branch -D ${publishBranch}
`)
