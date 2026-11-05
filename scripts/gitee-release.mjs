#!/usr/bin/env node
/**
 * **Gitee Release 的小工具**（CI 里用；2026-11-06 双平台发布）。
 *
 * 为什么需要它：macOS 包由发布内核（本机）上传到 Gitee Release，Windows 包由
 * `windows-build.yml` 在 tag 上构建 —— 两边是**两台机器、两个进程**，必须有一个人
 * 把 Windows 那份塞进**同一个** Release。内核没法在 macOS 上等一台 Windows 机器，
 * 所以让 Windows 那侧自己送过来。
 *
 * # 用法
 *
 * ```bash
 * node scripts/gitee-release.mjs upload --tag v0.0.10-preview.1 --file a.exe [--file b.exe]
 * node scripts/gitee-release.mjs show   --tag v0.0.10-preview.1      # 打印 Release JSON
 * ```
 *
 * `upload` 会**等** Release 出现（内核在本机建它，可能比 Windows 构建晚几分钟），
 * 然后逐个上传；同名资产已存在就跳过 —— 重跑一次是幂等的，不会出现两份同名附件。
 *
 * # 凭据
 *
 * `GITEE_TOKEN`（必填）、`GITEE_OWNER`、`GITEE_REPO`。令牌**永不打印**：
 * 请求走 URL 查询串或表单字段，日志里只出现文件名与字节数。
 */

import { readFileSync, statSync } from 'node:fs'
import { basename } from 'node:path'

const API = 'https://gitee.com/api/v5'

/** 认不出的参数直接停：静默忽略等于"上传了 0 个文件却说成功"。 */
function die(msg) {
  console.error(`✗ ${msg}`)
  process.exit(1)
}

function info(msg) {
  console.log(msg)
}

const argv = process.argv.slice(2)
const cmd = argv[0]
if (!cmd) die('用法：gitee-release.mjs upload --tag <tag> --file <path> [--file …] | show --tag <tag>')

const opts = { tag: '', files: [], wait: 1200 }
for (let i = 1; i < argv.length; i += 1) {
  const a = argv[i]
  if (a === '--tag') opts.tag = argv[++i] ?? ''
  else if (a === '--file') opts.files.push(argv[++i] ?? '')
  else if (a.startsWith('--tag=')) opts.tag = a.slice('--tag='.length)
  else if (a.startsWith('--file=')) opts.files.push(a.slice('--file='.length))
  else if (a === '--wait') opts.wait = Number(argv[++i] ?? 0)
  else die(`认不出的参数：${a}`)
}

if (!/^v\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(opts.tag)) die(`tag 形状不对：${opts.tag || '(空)'}`)

const token = process.env.GITEE_TOKEN ?? ''
const owner = process.env.GITEE_OWNER ?? ''
const repo = process.env.GITEE_REPO ?? ''
if (!token) die('缺 GITEE_TOKEN')
if (!owner || !repo) die('缺 GITEE_OWNER / GITEE_REPO')

const api = (path) => `${API}/repos/${owner}/${repo}${path}`
const auth = (sep) => `${sep}access_token=${encodeURIComponent(token)}`

/** 读 Release；不存在回 `null`（Gitee 用 404 表达"没有"，不是错误）。 */
async function fetchRelease(tag) {
  const resp = await fetch(`${api(`/releases/tags/${encodeURIComponent(tag)}`)}${auth('?')}`, {
    headers: { 'User-Agent': 'SupportEase-CI' },
  })
  if (resp.status === 404) return null
  if (!resp.ok) die(`读 Release 失败：HTTP ${resp.status} ${await resp.text()}`)
  return resp.json()
}

/** 等 Release 出现（内核建它要等本机构建 + 上传，可能好几分钟）。 */
async function waitRelease(tag) {
  const deadline = Date.now() + opts.wait * 1000
  let waited = 0
  for (;;) {
    const rel = await fetchRelease(tag)
    if (rel) return rel
    if (Date.now() > deadline) {
      die(`等了 ${opts.wait}s，Gitee 上还是没有 ${tag} 这个 Release —— 先确认内核那一步跑完了`)
    }
    if (waited % 60 === 0) info(`… 等 Gitee 上的 ${tag} 出现（已等 ${waited}s）`)
    await new Promise((r) => setTimeout(r, 10_000))
    waited += 10
  }
}

/** 上传一个附件：`POST /releases/{id}/attach_files`，multipart，字段名 `file`。 */
async function upload(rel, path) {
  const name = basename(path)
  const size = statSync(path).size
  if (rel.assets?.some((a) => a.name === name)) {
    info(`= ${name} 已经在 Release 上了（跳过）`)
    return
  }
  const form = new FormData()
  form.append('access_token', token)
  form.append('file', new Blob([readFileSync(path)]), name)

  const resp = await fetch(api(`/releases/${rel.id}/attach_files`), {
    method: 'POST',
    body: form,
    headers: { 'User-Agent': 'SupportEase-CI' },
  })
  // Gitee 成功时可能回 204 或空对象 —— 以状态码为准（与 Rust 那侧同一口径）
  if (!resp.ok) die(`上传 ${name} 失败：HTTP ${resp.status} ${await resp.text()}`)
  info(`✓ ${name}（${size} 字节）已上传到 ${rel.tag_name}`)
}

if (cmd === 'show') {
  const rel = await waitRelease(opts.tag)
  console.log(JSON.stringify(rel, null, 2))
} else if (cmd === 'upload') {
  if (opts.files.length === 0) die('至少要给一个 --file')
  const rel = await waitRelease(opts.tag)
  info(`Release #${rel.id} ${rel.tag_name}（prerelease=${rel.prerelease}）`)
  for (const f of opts.files) await upload(rel, f)
  // 传完回读一遍：**上传成功不等于附件在**（Gitee 有空体成功的历史）
  const after = await fetchRelease(opts.tag)
  const got = new Set((after?.assets ?? []).map((a) => a.name))
  const missing = opts.files.map((f) => basename(f)).filter((n) => !got.has(n))
  if (missing.length > 0) die(`回读时这些附件不在 Release 上：${missing.join(', ')}`)
  info(`✓ 回读确认：${opts.files.length} 个附件都在`)
} else {
  die(`认不出的子命令：${cmd}`)
}
