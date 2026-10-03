#!/usr/bin/env node
/*
 * 构建期资产就位（作者 2026-10-03 裁决的第二步）。
 *
 * 背景：产品数据资源搬到 `presets/assets/<kind>/` 之后，有**两条路**要把它们送到位：
 *
 * 1. **`bundled` 档**（今天的 4 张整机图）—— 随程序包带进客户端，客户端**不下载**。
 *    落点是客户端的**静态 import 路径**（`src/app/assets/printers/`，`heroArt.ts`
 *    的显式表 import 那几个固定路径），所以构建前必须先在位。
 * 2. **工作台 / 客户端的 `/assets/` URL** —— 台账里 `download` 档的文件在开发与
 *    打包后都要能按 `/assets/<path>` 取到（资产库预览、机型页图标预览都走它）。
 *    vite 只把 `public/` 直通出去，所以把 `presets/assets/` 复制进 `public/assets/`
 *    （**生成目录，不入库**）—— dev server 与 `vite build` 产物里就都齐了。
 *
 * 两个落点都是**生成物**（.gitignore 里有），不随仓库走：数据侧改了图 / 换了 3mf，
 * 重新构建即生效。判据与后端 [`dest_of_asset`] 同一句话：**按交付档位**，
 * 不按类型（作者 2026-10-03：第三刀把整机图硬编码进客户端源码是错的，已作废）。
 *
 * # 用法：`node scripts/copy-assets.mjs [workbench]`
 *
 * - 不带参数（**客户端构建**）：只拷 bundled 档进客户端静态资源。客户端从云端
 *   下载 BBS / 图标 / 模型，**不需要 `/assets/` 直通** —— 让它进包就是那份
 *   「随包副本」（第三圈要退役的东西）。
 * - 带 `workbench`（**工作台构建** / dev / preview）：额外把整个资产根复制到
 *   `public/assets/`，好让工作台界面里的资产预览按 `/assets/<path>` 取到图。
 */
import { readFileSync, mkdirSync, copyFileSync, cpSync, rmSync, existsSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const forWorkbench = process.argv.includes('workbench')

const repo = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const ledger = join(repo, 'presets', 'assets.toml')
const assetRoot = join(repo, 'presets', 'assets')
/** 客户端静态 import 的落点（`heroArt.ts` 里那几个路径） */
const bundledOut = join(repo, 'src', 'app', 'assets', 'printers')
/** `/assets/` 直通的落点（dev server 与 vite build 产物都看它） */
const publicOut = join(repo, 'public', 'assets')

if (!existsSync(ledger)) {
  console.error(`读不到资产台账：${ledger}`)
  process.exit(1)
}
const text = readFileSync(ledger, 'utf8')

/*
 * 解析 `[[assets]]` 表里写了 `delivery = 'bundled'` 的那些，取它们的 `path`。
 * 刻意手写而不引 TOML 库：形状是死的（单文件、单层数组表），多一个依赖不划算。
 */
const bundled = []
for (const block of text.split('[[assets]]').slice(1)) {
  if (!/^\s*delivery\s*=\s*'bundled'/m.test(block)) continue
  const p = block.match(/^\s*path\s*=\s*'([^']+)'/m)
  if (p === null) {
    console.error('有一条 bundled 资产没有 path —— 台账坏了')
    process.exit(1)
  }
  bundled.push(p[1])
}

if (bundled.length > 0) {
  mkdirSync(bundledOut, { recursive: true })
  for (const rel of bundled) {
    const from = join(assetRoot, rel)
    if (!existsSync(from)) {
      console.error(`台账登记了 ${rel}，但数据目录里没有它：${from}`)
      process.exit(1)
    }
    const to = join(bundledOut, rel.replace(/^printers\//, ''))
    mkdirSync(dirname(to), { recursive: true })
    copyFileSync(from, to)
  }
  console.log(`bundled 档资产：${bundled.length} 份已复制进客户端资源（${bundledOut}）`)
} else {
  console.log('bundled 档资产：0 条（台账里没写 delivery = bundled），客户端资源不用动')
}

if (forWorkbench) {
  if (existsSync(assetRoot)) {
    cpSync(assetRoot, publicOut, { recursive: true })
    console.log(`资产根已复制到 ${publicOut}（工作台的 /assets/ 直通靠它）`)
  }
} else {
  // 客户端构建：**清掉**上一次工作台构建留下的直通副本 —— 它是生成物，
  // 留着会被 vite 原样拷进安装包（那就是「随包副本」回归，作者 2026-10-03 收掉的东西）
  rmSync(publicOut, { recursive: true, force: true })
  console.log('客户端构建：已清掉 public/assets/（客户端从云端下载，不需要 /assets/ 直通）')
}
