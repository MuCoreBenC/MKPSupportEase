/**
 * 判据 1 —— 安装包产物扫描（`docs/DATA-ARCHITECTURE.md` §5）。
 *
 * 总纲的四条铁律里有三条是"什么东西不许出现在哪一层"：
 *
 *   - 开发文件不能直接成为运行时数据库（①的源 TOML 不许进②）；
 *   - 用户没下载的东西，不提前塞进用户目录（模拟数据/夹具不许进②）；
 *   - 运行时只认自己的运行时数据（工作台的开发态不许进②）。
 *
 * "不许"如果只写在文档里，半年后一定有人顺手抄错目录。这条判据把铁律变成
 * 可以在 CI 上红的检查：**扫 vite 的构建产物（`dist/`，也就是进安装包的前端内容），
 * 出现下面任何一样就失败。**
 *
 * 与 workbench 构建的关系：`npm run build:workbench` 的产物**故意**包含
 * workbench 页面与假云端快照（那是后厨自己的工具，不进客户端安装包）。
 * 所以本判据只对客户端构建跑（`npm run build`）—— 工作台产物不在此判据的管辖内。
 *
 * 扫描是"黑名单"起步：新名单往 FORBIDDEN 里加一行，理由写在旁边。
 */

import { readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const DIST = 'dist'

/** 禁止出现在客户端安装包产物里的东西。左边是匹配产物相对路径的正则，右边是为什么 */
const FORBIDDEN = [
  // 模拟云端：假数据进成品是事故（总纲欠账 #2 的教训，2026-10-01 收口前它一直在包里）
  [/\/cloud\/presets\.json$/, '模拟云端的假清单（现在住在 src/workbench/fixtures/，只许进工作台构建）'],

  // 工作台：开发态。第二入口 workbench.html 只在工作台构建里进 input（vite.config.ts 的闸）
  [/workbench\.html$/, '工作台第二入口（客户端构建不该有它）'],
  [/(^|\/)workbench\//, '工作台的开发态内容'],

  // 工作台运行态：快照与草稿是工具的暂存区（docs/DATA-INVENTORY.md F8）
  [/\.snapshots\//, '工作台快照'],
  [/\.draft\//, '工作台草稿'],

  // 开发仓库的源 TOML：它们是①的源，运行时数据只能是②的构建产物（铁律 1）
  [/brands\.toml$/, '开发源 TOML'],
  [/layout_schema\.toml$/, '开发源 TOML'],
  [/bundles\.toml$/, '开发源 TOML'],
  [/assets\.toml$/, '开发源 TOML'],
  [/(^|\/)machines\/.+\.toml$/, '开发源 TOML'],
  [/(^|\/)forbidden_zones\//, '开发源 TOML'],
  [/param_registry\.toml$/, '开发源 TOML'],

  // 内置预设的入库目录（crates/preset/assets/presets）：它编进二进制，不以文件形式随前端产物走
  [/(^|\/)presets\/.+\.toml$/, '交付预设文件（编在二进制里，不该出现在前端产物）'],

  // 已接进 catalog 的资产：它们归 catalog 管（带 SHA / 大小，按需下载进 mkp/），
  // 包里再带一份就是第二个真源。新增一类资产进管道，这里就多一行（与 vite.config.ts
  // 的 DELIVERED_ASSET_DIRS 同一份清单，改一处就要改另一处）
  [/\/assets\/bbs\//, '已走下载管道的 BBS 资产（不该再随包分发）'],
  [/\/assets\/models\//, '已走下载管道的模型（不该再随包分发）'],
  [/\/assets\/icons\//, '已走下载管道的图标（不该再随包分发）'],

  // 注意：**界面自带素材不在这份清单的管辖里，也不需要靠目录名堵** ——
  // 品牌 logo、机型整机图、首页 / 校准页那张 hero 合影，2026-10-01 起都住
  // `src/app/assets/`（`bambuLogo.ts` / `printers/` / `hero/`），由 vite 资源管线打进
  // `dist/assets/*.webp`（带内容哈希），与品牌 logo 同一档。
  // `public/` 从此只剩两样：台账管的载荷根 `assets/{bbs,icons,models}`，与 BBS 页元数据 `bbs/`。
  // 归属的判据在 Rust 侧（`runtime::catalog`：「资产台账里已无 image 类」），不在这里。
]

function walk(dir, prefix = '') {
  const out = []
  for (const name of readdirSync(dir)) {
    const rel = prefix === '' ? name : `${prefix}/${name}`
    const full = join(dir, name)
    if (statSync(full).isDirectory()) out.push(...walk(full, rel))
    else out.push(rel)
  }
  return out
}

let files
try {
  files = walk(DIST)
} catch {
  console.error(`✗ 读不到 ${DIST}/ —— 先跑一遍 npm run build（判据扫描的是构建产物）`)
  process.exit(1)
}

const hits = []
for (const rel of files) {
  const path = `/${rel.replaceAll('\\', '/')}`
  for (const [re, why] of FORBIDDEN) {
    if (re.test(path)) hits.push({ path, why })
  }
}

if (hits.length > 0) {
  console.error(`✗ 安装包产物里出现了不该有的东西（${hits.length} 处）—— 铁律被违反了：`)
  for (const h of hits) console.error(`    ${h.path}\n      → ${h.why}`)
  console.error('  修法：把这些文件从客户端构建里拿出去（挪进 dev 夹具 / 工作台构建 / 二进制内嵌），')
  console.error('  不要改这份黑名单来"让 CI 过"——那等于把闸门拆了。')
  process.exit(1)
}

console.log(`✓ 安装包产物干净：${files.length} 个文件，没有任何开发源 / 模拟数据 / 工作台内容`)
