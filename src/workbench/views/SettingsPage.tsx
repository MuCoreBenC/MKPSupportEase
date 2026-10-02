/**
 * 「设置」页 —— 数据根只读 + **一块可编辑的配置**（官方源）。
 *
 * 数据根（presets / workbench / dist）**不可配**：本仓的预设真相源固定是
 * `<repo>/presets`（没有第二候选、不 fallback），所以这一页把它们摆开，让人一眼看清
 * 工作台在读哪几个根、工作台子目录各自谁写谁读 —— 排查「读错了目录 / 数据长在哪」
 * 的那种问题用。上游根与回退规则那两块**已经整层删掉**（`HANDOFF.md`），照实不摆。
 *
 * 2026-10-02（第十七刀）起多了一块**可编辑**的：「官方源（Bootstrap）」——
 * 发布产物发到哪。它是**工作台唯一一处"发布到哪"**（入库的
 * `workbench/bootstrap.json`）；客户端构建时由 `build.rs` 读它注入默认源
 * （`npm run tauri dev` 也吃它）。
 *
 * 输入口径（作者 2026-10-02 定死的产品契约）：**只填仓库地址就够** ——
 * GitHub 仓库地址 / `.git` 克隆地址都会被后端补成 `main/presets/dist/source.json`
 * 的 raw 直链；blob 页按人指的转；raw / 自建源原样。用户不必知道
 * `raw.githubusercontent.com` / `blob` / `presets/dist` / `source.json` 里的任何一个。
 *
 * # 「重新读取」（2026-10-02）
 *
 * 这一格读的是 `workbench/bootstrap.json`（**随仓库入库的文件**），它可能被**界面之外**
 * 的东西改掉：你手工编辑、另一个工具写、别人提交后你 pull、CI 生成。那之后界面上的值
 * 就旧了 —— 而"界面上看到的是不是磁盘上那份"必须答得出来（否则证明不了任何事）。
 * 所以给一颗「重新读取」：它调 `wb.reload()`（**重开一次后端会话、从磁盘重读**），
 * 拿回来的 `bootstrapUrl` 重新填进输入框与下面那行「当前」。
 *
 * **它同时是"配置生效了没有"的判据**：点一下看到的就是**磁盘真值**。
 * （注意区分：这里读的是**配置**；Bootstrap 被客户端吃进二进制是**构建期**的事 ——
 * 改完要重新构建客户端，所以保存成功那句话会提"重启 dev / 重打正式包"。）
 */
import { useState } from 'react'

import { isAppError, wb } from '../api'
import type { Boot } from '../api'
import s from '../c14.module.css'

export default function SettingsPage({ boot }: { boot: Boot }) {
  /* 官方源那格：初值来自 boot；保存成功后本地回显（真值在 workbench/bootstrap.json，
     下次 wb_boot 会带回同一份） */
  const [bootstrap, setBootstrap] = useState(boot.bootstrapUrl ?? '')
  const [saved, setSaved] = useState<string | null>(boot.bootstrapUrl)
  const [busy, setBusy] = useState(false)
  const [note, setNote] = useState<{ text: string; bad: boolean } | null>(null)

  const save = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const stored = await wb.setBootstrap(bootstrap.trim())
      setSaved(stored)
      setBootstrap(stored)
      setNote({ text: `已保存：${stored}（重启 dev / 重打正式包后客户端才吃得到）`, bad: false })
    } catch (e) {
      setNote({
        text: isAppError(e) ? e.message : e instanceof Error ? e.message : String(e),
        bad: true,
      })
    } finally {
      setBusy(false)
    }
  }

  /**
   * 从**磁盘**重读（`wb.reload()` 重开一次后端会话）。
   * 拿回来的就是 `workbench/bootstrap.json` 的真值 —— 不管它是谁写的、什么时候改的。
   * 界面上那些还没保存的输入会被磁盘那份**覆盖**（那正是"看到真相"的含义）。
   */
  const reread = async () => {
    if (busy) return
    setBusy(true)
    setNote(null)
    try {
      const fresh = await wb.reload()
      const onDisk = fresh.bootstrapUrl
      setSaved(onDisk)
      setBootstrap(onDisk ?? '')
      setNote({
        text: onDisk === null ? '已从磁盘重读：还没配' : '已从磁盘重读',
        bad: false,
      })
    } catch (e) {
      setNote({
        text: isAppError(e) ? e.message : e instanceof Error ? e.message : String(e),
        bad: true,
      })
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className={s.flow} style={{ padding: 12 }}>
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>官方源（Bootstrap）</b>
          <span className={s.cardNote}>发布产物发到哪 —— 工作台唯一一处；客户端构建时编进去</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              发布出去的 <span className={s.mono}>presets/dist/</span> 推到哪里 —— 客户端拿它那口
              <span className={s.mono}> source.json </span>找回目录与文件。
              <b>填仓库地址就够</b>（GitHub 仓库地址或 <span className={s.mono}>.git</span> 克隆地址）——
              我们会自动补成发布入口的 raw 直链。
              也收：指向 <span className={s.mono}>source.json</span> 的 blob 链接（转 raw）、
              自建源（<span className={s.mono}>http://…</span> 原样）。
              入库（<span className={s.mono}>workbench/bootstrap.json</span>）：换机器、CI 拿的都是同一份。
            </p>
            <div className={s.vrow}>
              <input
                className={s.inp}
                value={bootstrap}
                onChange={(e) => setBootstrap(e.target.value)}
                /* 中性示例（`<owner>/<repo>`），**不是"默认值"** ——
                   空着就是"还没配"，别让占位符看起来像已经填好了 */
                placeholder="https://github.com/<owner>/<repo>"
                aria-label="官方源（Bootstrap）地址"
              />
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary}`}
                disabled={busy || bootstrap.trim() === '' || bootstrap.trim() === saved}
                onClick={() => void save()}
              >
                {busy ? '保存中……' : '保存'}
              </button>
              {/* 从磁盘重读 —— 界面之外改过这份配置时，用它看真相（见页头） */}
              <button
                type="button"
                className={s.btn}
                disabled={busy}
                title="丢掉输入框里没保存的内容，重新从 workbench/bootstrap.json 读一遍"
                onClick={() => void reread()}
              >
                重新读取
              </button>
            </div>
            <p className={s.vhelp}>
              当前（磁盘 <span className={s.mono}>workbench/bootstrap.json</span> 里的那份）：
              {saved === null ? (
                '还没配 —— 客户端构建时不会注入默认源（下载会如实说「没配」）'
              ) : (
                <span className={s.mono}>{saved}</span>
              )}
            </p>
            {note && (
              <p className={s.vhelp} style={note.bad ? { color: 'var(--danger)' } : undefined}>
                {note.text}
              </p>
            )}
          </div>
        </div>
      </div>

      <div className={s.card}>
        <div className={s.cardHead}>
          <b>数据根</b>
          <span className={s.cardNote}>唯一真相源 —— 没有第二候选、不 fallback</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>预设源</b>
              <span className={s.vkey}>presets</span>
            </div>
            <p className={s.vhelp}>
              工作台读写的唯一预设根。改它要改的是仓库结构，不是这里的某一格。
            </p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.presets}</span>
            </div>
          </div>

          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>配方本</b>
              <span className={s.vkey}>workbench</span>
            </div>
            <p className={s.vhelp}>开发源数据、草稿与回收站住的地方。</p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.workbench}</span>
            </div>
          </div>

          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>交付产物</b>
              <span className={s.vkey}>dist</span>
            </div>
            <p className={s.vhelp}>
              「生成与发布」写出的那一批：人维护 <span className={s.mono}>presets/*.toml</span>，
              机器生成 <span className={s.mono}>presets/dist/*</span>。
            </p>
            <div className={s.vrow}>
              <span className={`${s.vstatic} ${s.mono}`}>{boot.roots.dist}</span>
            </div>
          </div>
        </div>
      </div>

      <div className={s.card}>
        <div className={s.cardHead}>
          <b>工作台子目录</b>
          <span className={s.cardNote}>谁写谁读，由后端一句话说清</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vstack}>
            {boot.storeDirs.map((d) => (
              <div key={d.name} className={s.vrow}>
                <span className={`${s.vkey} ${s.mono}`}>{d.name}</span>
                <span>{d.role}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      {boot.problem && (
        <div className={s.card}>
          <div className={s.cardHead}>
            <b>开场问题</b>
          </div>
          <div className={s.cardBody}>
            <p className={s.vhelp}>
              {boot.problem}
              {boot.detail && (
                <>
                  {' —— '}
                  <span className={s.mono}>{boot.detail}</span>
                </>
              )}
            </p>
          </div>
        </div>
      )}
    </div>
  )
}
