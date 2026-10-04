/*
 * **通用文件导入入口**（第十二层）—— 住在 App 层，不属于任何一页。
 *
 * 作者定的形状：
 *
 *   App
 *   └── 通用导入入口
 *       ├── 文件选择器（`api.pickImportFiles`，系统原生对话框）
 *       └── 拖拽（真机走 Tauri 的原生拖拽事件；浏览器演示 / 探针走 HTML5 拖拽）
 *            ↓
 *        导入核心（`api.stageImport` / `api.commitImport` → `runtime::import`）
 *            ↓
 *        当前认领它的导入器（现在只有 Preset：`.toml` → `presets-mine/`）
 *
 * # 为什么在 App 层
 *
 * 以后「设置 → 备份与恢复」的 ZIP / 备份包要复用同一套接收机制（拖拽、选择器、
 * 重名处理、边界检查）—— 它不属于预设页。预设页是**第一个消费者**：从这一层拿
 * `revision`（"该重读用户线了"的钥匙）。导入的界面入口是**拖拽**（把文件拖进窗口，
 * 任何页面都收）—— 预设页工具栏那颗「导入文件…」按钮已退役（作者 2026-10-04：
 * 几乎不需要导入，不给它常驻的位子）；选择器能力（`pickFiles`）照旧留在这一层，
 * 只是暂时没有界面触发点。
 *
 * # 重名不是失败，是改名流程
 *
 * 落点检查说 `collision` ⇒ 开「导入：有同名文件」那一格：每份一个输入框，
 * 名字的门槛与改名 / 另存为**同一套**（后端 `check_new_name`）；**不覆盖、不自动改名**。
 * 用户取消 = 这些重名的没进来；别的该进的照常进（一份错不拖累别人，与批量下载同一条规矩）。
 */

import { useCallback, useContext, useEffect, useRef, useState } from 'react'
import type { ReactNode } from 'react'
import { getCurrentWebview } from '@tauri-apps/api/webview'

import { api, errorText } from '../../api'
import type { ImportItem, ImportOutcome, StagedImport } from '../../api'
import Drawer from '../shared/Drawer'
import { inTauri } from '../window'
import { ImportContext } from './useFileImport'
import type { ImportBannerState, ImportEntry } from './useFileImport'
import s from './FileImport.module.css'

/** 结果条。App 把它渲染在标签栏与内容之间（in-flow —— 不遮内容，也不是会消失的提示） */
export function ImportBanner() {
  const entry = useContext(ImportContext)
  if (entry === null || entry.banner === null) return null
  const { text, bad, lines } = entry.banner
  return (
    <div className={bad ? `${s.banner} ${s.bannerBad}` : s.banner} role="status">
      <p className={s.bannerText}>
        <span>{text}</span>
        <button
          type="button"
          className={s.bannerClose}
          aria-label="知道了"
          onClick={entry.closeBanner}
        >
          知道了
        </button>
      </p>
      {lines.map((line, i) => (
        <p key={`${i}-${line}`} className={s.bannerLine}>
          {line}
        </p>
      ))}
    </div>
  )
}

/** 重名那一格里的一行 */
interface CollisionRow {
  source: string
  fileName: string
  name: string
  error: string | null
}

export function FileImportProvider({ children }: { children: ReactNode }) {
  const [revision, setRevision] = useState(0)
  const [dragging, setDragging] = useState(false)
  const [busy, setBusy] = useState(false)
  const [banner, setBanner] = useState<ImportBannerState | null>(null)
  const [modal, setModal] = useState<{ rows: CollisionRow[] } | null>(null)

  /* 一次只跑一批：拖第二次不会被上一批的半截状态缠住 */
  const busyRef = useRef(false)

  const note = useCallback((next: ImportBannerState) => setBanner(next), [])
  const closeBanner = useCallback(() => setBanner(null), [])

  /** 把一批结局（+ 没收的 rejected）写进结果条 */
  const report = useCallback(
    (outcomes: ImportOutcome[], rejected: StagedImport[]) => {
      const done = outcomes.filter((o) => o.ok)
      const failed = outcomes.filter((o) => !o.ok)
      if (done.length > 0) setRevision((r) => r + 1)
      const lines = [
        ...outcomes.map((o) =>
          o.ok ? `已导入 ${o.fileName} → ${o.path}` : `${o.fileName}：${o.message}`,
        ),
        ...rejected.map((r) => `${r.fileName}：${r.reason ?? '收不了'}`),
      ]
      const miss = failed.length + rejected.length
      if (done.length === 0 && failed.length === 0) {
        /* 纯粹是"收不了"（比如一个 ZIP）：不摆"0 份成了"那种读起来别扭的句子 */
        note({ text: `这 ${rejected.length} 份收不了 —— 明细在下面`, bad: true, lines })
        return
      }
      note(
        miss === 0
          ? { text: `导入了 ${done.length} 份文件到「我的文件」`, bad: false, lines }
          : { text: `导入：${done.length} 份成了、${miss} 份没成`, bad: true, lines },
      )
    },
    [note],
  )

  /**
   * 一批外部路径的完整流程：看落点 → 没重名的直接导 → 重名的开改名那一格。
   *
   * 先导没重名的那些（它们的结论当场就出）；重名的**等用户在格子里改完名字**再导 ——
   * 不自动改名、不覆盖。
   */
  const importPaths = useCallback(
    async (paths: string[]) => {
      if (paths.length === 0 || busyRef.current) return
      busyRef.current = true
      setBusy(true)
      try {
        const staged = await api.stageImport(paths)
        const ready = staged.filter((s) => s.state === 'ready')
        const collisions = staged.filter((s) => s.state === 'collision')
        const rejected = staged.filter((s) => s.state === 'rejected')

        if (ready.length > 0) {
          const outcomes = await api.commitImport(
            ready.map((r): ImportItem => ({ source: r.source })),
          )
          report(outcomes, rejected)
        } else if (rejected.length > 0) {
          report([], rejected)
        }

        if (collisions.length > 0) {
          setModal({
            rows: collisions.map((c) => ({
              source: c.source,
              fileName: c.fileName,
              name: c.fileName,
              error: null,
            })),
          })
        }
      } catch (e) {
        note({ text: `导入没走成：${errorText(e)}`, bad: true, lines: [] })
      } finally {
        busyRef.current = false
        setBusy(false)
      }
    },
    [note, report],
  )

  /** 「继续导入」：把重名的那些（名字已改）提交；没成的留在格子里让继续改 */
  const submitModal = useCallback(async () => {
    if (modal === null || busyRef.current) return
    const rows = modal.rows.map((r) => ({ ...r, name: r.name.trim() }))
    if (rows.some((r) => r.name === '')) {
      setModal({
        rows: rows.map((r) => (r.name === '' ? { ...r, error: '新名字不能是空的' } : r)),
      })
      return
    }
    busyRef.current = true
    setBusy(true)
    try {
      const outcomes = await api.commitImport(
        rows.map((r): ImportItem => ({ source: r.source, newName: r.name })),
      )
      const bySource = new Map(outcomes.map((o) => [o.source, o]))
      const done = outcomes.filter((o) => o.ok)
      if (done.length > 0) setRevision((r) => r + 1)

      const remaining: CollisionRow[] = []
      for (const row of rows) {
        const outcome = bySource.get(row.source)
        if (outcome !== undefined && outcome.ok) continue
        remaining.push({ ...row, error: outcome?.message ?? '没走成（没有回结局）' })
      }

      if (remaining.length === 0) {
        setModal(null)
        note({
          text: `导入完成：${done.length} 份重名的都改好名字进来了`,
          bad: false,
          lines: done.map((o) => `已导入 ${o.fileName} → ${o.path}`),
        })
      } else {
        setModal({ rows: remaining })
        note({
          text:
            done.length > 0
              ? `其中 ${done.length} 份已经进来了；还有 ${remaining.length} 份没成 —— 在右边那一格里改`
              : `这 ${remaining.length} 份还没进来 —— 换个名字再试`,
          bad: true,
          lines: [],
        })
      }
    } catch (e) {
      note({ text: `导入没走成：${errorText(e)}`, bad: true, lines: [] })
    } finally {
      busyRef.current = false
      setBusy(false)
    }
  }, [modal, note])

  /** 取消 = 这些重名的没进来（别的该进的早进了 —— 一份错不拖累别人） */
  const cancelModal = useCallback(() => {
    if (modal === null) return
    const names = modal.rows.map((r) => r.fileName)
    setModal(null)
    note({
      text: `已取消：${names.length} 份重名的没有导入 —— 可以先改个名字再试`,
      bad: false,
      lines: names,
    })
  }, [modal, note])

  const pickFiles = useCallback(() => {
    void api.pickImportFiles().then(
      (paths) => void importPaths(paths),
      (e: unknown) => note({ text: `打不开文件选择器：${errorText(e)}`, bad: true, lines: [] }),
    )
  }, [importPaths, note])

  /*
   * 拖拽两套适配器，落到同一段 `importPaths`：
   * · 真机：Tauri 的原生拖拽事件给的是**路径**（系统级，任何文件都行）；
   * · 浏览器演示 / 探针：HTML5 拖拽只有 File 对象（没有路径）—— 拼一条假路径
   *   （`/（拖入）/名字`）；假后端只认文件名，所以认得出来。
   */
  useEffect(() => {
    if (inTauri) {
      let dispose: (() => void) | undefined
      void getCurrentWebview()
        .onDragDropEvent((event) => {
          const payload = event.payload
          if (payload.type === 'enter' || payload.type === 'over') {
            setDragging(true)
            return
          }
          setDragging(false)
          if (payload.type === 'drop') void importPaths(payload.paths)
        })
        .then((unlisten) => {
          dispose = unlisten
        })
      return () => dispose?.()
    }

    const onOver = (e: DragEvent) => {
      e.preventDefault()
      setDragging(true)
    }
    const onLeave = (e: DragEvent) => {
      /* 出了窗口才算离开：窗口内元素之间的 dragleave 不清提示 */
      if (e.relatedTarget === null) setDragging(false)
    }
    const onDrop = (e: DragEvent) => {
      e.preventDefault()
      setDragging(false)
      const files = [...(e.dataTransfer?.files ?? [])]
      if (files.length > 0) void importPaths(files.map((f) => `/（拖入）/${f.name}`))
    }
    window.addEventListener('dragover', onOver)
    window.addEventListener('dragleave', onLeave)
    window.addEventListener('drop', onDrop)
    return () => {
      window.removeEventListener('dragover', onOver)
      window.removeEventListener('dragleave', onLeave)
      window.removeEventListener('drop', onDrop)
    }
  }, [importPaths])

  const entry: ImportEntry = { pickFiles, revision, banner, closeBanner }

  return (
    <ImportContext.Provider value={entry}>
      {children}

      {/* 拖到这里松开就导入。遮罩不吃事件（pointer-events: none）—— drop 要落到窗口上 */}
      {dragging && modal === null && (
        <div className={s.veil} role="presentation">
          <p className={s.veilText}>松开：把文件导入「我的文件」（现在收 .toml 预设）</p>
        </div>
      )}

      {modal !== null && (
        <Drawer
          open
          title="导入：有同名文件"
          subtitle={`${modal.rows.length} 份重名的，改好名字才能进来`}
          footer={
            <>
              <button type="button" className={s.ghost} onClick={cancelModal}>
                取消导入
              </button>
              <button
                type="button"
                className={s.primary}
                disabled={busy}
                onClick={() => void submitModal()}
              >
                继续导入
              </button>
            </>
          }
          onClose={cancelModal}
        >
          <div className={s.rows}>
            {modal.rows.map((row) => (
              <div key={row.source} className={s.rowCard}>
                <p className={s.rowHead}>
                  已存在同名文件 <b>{row.fileName}</b> —— 换个名字（原来那份不会被覆盖）
                </p>
                <input
                  className={s.rowInput}
                  value={row.name}
                  spellCheck={false}
                  aria-label={`新的文件名：${row.fileName}`}
                  onChange={(e) =>
                    setModal((cur) =>
                      cur === null
                        ? cur
                        : {
                            rows: cur.rows.map((x) =>
                              x.source === row.source
                                ? { ...x, name: e.target.value, error: null }
                                : x,
                            ),
                          },
                    )
                  }
                />
                {row.error !== null && <p className={s.rowErr}>{row.error}</p>}
              </div>
            ))}
            <p className={s.hint}>
              名字的规矩与重命名一致：不许空、不许带路径、后缀保持原样；改完点「继续导入」。
            </p>
          </div>
        </Drawer>
      )}
    </ImportContext.Provider>
  )
}
