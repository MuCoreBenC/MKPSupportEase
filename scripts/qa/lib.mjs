/*
 * 质检跑道的报告件（2026-11-14）。
 *
 * 一条 spec 里只写三件事：`check(这句话对不对, 条件)`、`shot(page, '给图起的名')`、
 * `note('要人看一眼的观察')`。报告怎么排、图放哪、失败怎么汇总，全在这一层 ——
 * 加一条 spec 不该先读两百行 HTML 生成代码。
 *
 * # 为什么报告是自包含的一页 HTML + 一份 Markdown
 *
 * HTML 给人看（点开大图、按分区读），Markdown 给 AI 读与 git 留档。
 * 两版同一份数据，不各拼一遍。
 */

import { mkdirSync, writeFileSync } from 'node:fs'
import { join } from 'node:path'

const esc = (s) =>
  String(s).replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c])

const slug = (s) =>
  String(s)
    .trim()
    .replace(/[^\w\u4e00-\u9fa5-]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40) || 'shot'

export function makeReporter({ outDir, title }) {
  const shotsDir = join(outDir, 'shots')
  mkdirSync(shotsDir, { recursive: true })

  /** @type {{name:string, checks:{text:string, ok:boolean, detail:string}[], shots:{file:string,label:string}[], notes:string[]}[]} */
  const sections = []
  let cur = null
  let seq = 0
  const startedAt = new Date()

  const ensure = (name) => {
    if (cur === null) {
      cur = { name: name ?? '未命名', checks: [], shots: [], notes: [] }
      sections.push(cur)
    }
    return cur
  }

  const api = {
    /** 开一个新分区（一条 spec 一个） */
    section(name) {
      cur = { name, checks: [], shots: [], notes: [] }
      sections.push(cur)
      seq = 0
    },
    check(text, ok, detail = '') {
      ensure().checks.push({ text, ok: ok === true, detail: String(detail) })
      const mark = ok === true ? 'PASS' : 'FAIL'
      console.log(`${mark} ${text}${detail === '' ? '' : `（${detail}）`}`)
    },
    note(text) {
      ensure().notes.push(String(text))
      console.log(`NOTE ${text}`)
    },
    async shot(page, label) {
      const file = `shots/${String(seq).padStart(2, '0')}-${slug(label)}.png`
      seq += 1
      await page.screenshot({ path: join(outDir, file), fullPage: false })
      ensure().shots.push({ file, label })
    },
    /** 一条 spec 的浏览器报错单独收成一节里的「红」 */
    errors(list) {
      if (list.length === 0) return
      const sec = ensure()
      for (const e of list) sec.checks.push({ text: '控制台不该报错', ok: false, detail: e })
      console.log(`FAIL 控制台不该报错（${list.length} 条）`)
    },
    summary() {
      const checks = sections.flatMap((s) => s.checks)
      const bad = checks.filter((c) => !c.ok)
      return { sections, total: checks.length, failed: bad.length, bad }
    },
    finish({ legacy = [] } = {}) {
      const { total, failed, bad } = api.summary()
      const when = startedAt.toLocaleString('zh-CN', { hour12: false })
      const head = `# ${title}\n\n- 时间：${when}\n- 断言：${total} 条，失败 ${failed} 条\n- 产物：\`${outDir}\`\n`

      /* ---------- Markdown ---------- */
      let md = `${head}\n`
      if (failed > 0) {
        md += `## 红\n\n`
        for (const b of bad) md += `- ${b.text}${b.detail === '' ? '' : ` —— ${b.detail}`}\n`
        md += '\n'
      }
      for (const s of sections) {
        md += `## ${s.name}\n\n`
        for (const c of s.checks) {
          md += `- ${c.ok ? '✔' : '✘'} ${c.text}${c.detail === '' ? '' : ` —— ${c.detail}`}\n`
        }
        for (const n of s.notes) md += `- 观察：${n}\n`
        for (const sh of s.shots) md += `\n![${sh.label}](${sh.file})\n`
        md += '\n'
      }
      if (legacy.length > 0) {
        md += `## 老探针\n\n`
        for (const l of legacy) md += `- ${l.ok ? '✔' : '✘'} ${l.name}${l.detail ? ` —— ${l.detail}` : ''}\n`
      }

      /* ---------- HTML ---------- */
      const secHtml = sections
        .map((s) => {
          const checks = s.checks
            .map(
              (c) =>
                `<li class="${c.ok ? 'ok' : 'bad'}"><b>${c.ok ? '✔' : '✘'}</b> ${esc(c.text)}${
                  c.detail === '' ? '' : `<span class="detail">${esc(c.detail)}</span>`
                }</li>`,
            )
            .join('')
          const notes = s.notes.map((n) => `<li class="note">${esc(n)}</li>`).join('')
          const shots = s.shots
            .map(
              (sh) =>
                `<figure><a href="${sh.file}" target="_blank"><img src="${sh.file}" alt="${esc(sh.label)}" loading="lazy"></a><figcaption>${esc(sh.label)}</figcaption></figure>`,
            )
            .join('')
          return `<section><h2>${esc(s.name)}</h2><ul>${checks}${notes}</ul>${
            shots === '' ? '' : `<div class="shots">${shots}</div>`
          }</section>`
        })
        .join('')

      const badHtml =
        failed === 0
          ? `<p class="pass">全部通过。</p>`
          : `<ul>${bad
              .map((b) => `<li class="bad"><b>✘</b> ${esc(b.text)}<span class="detail">${esc(b.detail)}</span></li>`)
              .join('')}</ul>`

      const legacyHtml =
        legacy.length === 0
          ? ''
          : `<section><h2>老探针</h2><ul>${legacy
              .map((l) => `<li class="${l.ok ? 'ok' : 'bad'}"><b>${l.ok ? '✔' : '✘'}</b> ${esc(l.name)}${l.detail ? `<span class="detail">${esc(l.detail)}</span>` : ''}</li>`)
              .join('')}</ul></section>`

      const html = `<!doctype html><html lang="zh-CN"><meta charset="utf-8"><title>${esc(title)}</title>
<style>
  :root { color-scheme: light dark; }
  body { margin: 0 auto; max-width: 1100px; padding: 28px 20px 60px; font: 13px/1.65 "PingFang SC", "Microsoft YaHei", system-ui, sans-serif; color: #1a2128; background: #f7f9fa; }
  h1 { font-size: 19px; margin: 0 0 4px; }
  h2 { font-size: 14px; margin: 28px 0 8px; padding-bottom: 6px; border-bottom: 1px solid #e3e8ec; }
  .meta { color: #6f7b87; }
  .meta b { color: #1a2128; }
  ul { list-style: none; padding: 0; margin: 0; }
  li { padding: 3px 0; border-bottom: 1px dashed #eef2f5; }
  li b { display: inline-block; width: 16px; }
  li.ok b { color: #17924a; }
  li.bad b { color: #b42318; }
  li.bad { background: #fff5f4; }
  .detail { margin-left: 8px; color: #6f7b87; }
  li.note { color: #6f7b87; }
  li.note::before { content: "观察 "; color: #b42318; }
  .shots { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 12px; margin-top: 12px; }
  figure { margin: 0; background: #fff; border: 1px solid #e3e8ec; border-radius: 8px; overflow: hidden; }
  figure img { display: block; width: 100%; }
  figcaption { padding: 6px 8px; color: #56626e; font-size: 12px; }
  .pass { color: #17924a; }
</style>
<h1>${esc(title)}</h1>
<p class="meta">${esc(when)} · 断言 <b>${total}</b> 条 · 失败 <b>${failed}</b> 条</p>
<h2>红</h2>${badHtml}
${secHtml}
${legacyHtml}
`
      writeFileSync(join(outDir, 'report.md'), md)
      writeFileSync(join(outDir, 'report.html'), html)
      return { total, failed, mdPath: join(outDir, 'report.md'), htmlPath: join(outDir, 'report.html') }
    },
  }
  return api
}
