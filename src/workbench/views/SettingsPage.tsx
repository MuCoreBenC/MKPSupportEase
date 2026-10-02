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
 * （`npm run tauri dev` 也吃它）。GitHub 的 blob 页链接会被后端规范成 raw 直链。
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
              <b>GitHub 的 blob 链接会自动转成 raw 直链</b>；自建源（
              <span className={s.mono}>http://…</span>）原样收下。
              入库（<span className={s.mono}>workbench/bootstrap.json</span>）：换机器、CI 拿的都是同一份。
            </p>
            <div className={s.vrow}>
              <input
                className={s.inp}
                value={bootstrap}
                onChange={(e) => setBootstrap(e.target.value)}
                placeholder="https://github.com/…/blob/main/release/presets/source.json"
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
            </div>
            <p className={s.vhelp}>
              当前：
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
