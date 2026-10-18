/**
 * 「开发台」页 —— **客户端界面的触发器**（工作台里的一页）。
 *
 * # 它是什么（作者 2026-10-14 定稿）
 *
 * 「工作台只负责选择场景，客户端只负责展示场景」——
 *
 *  - 这一页**只是一个整齐的场景清单**：场景名 / 一句说明 / 右边一颗「演示」。
 *  - 点「演示」→ 工作台发一条指令给**客户端进程** → **真正的客户端窗口**把那一屏
 *    （客户端自己的真组件）显示出来，并把窗口**置顶**。
 *  - **不在工作台里嵌预览**：工作台是另一扇窗，嵌进来等于"另画一份"，不是要看的那一屏。
 *
 * # 为什么不直接调本进程的 HookView
 *
 * ★ 工作台构建里**没有客户端窗口**（`tauri.workbench.conf.json` 的 `app.windows = []`）。
 * 所以只能**跨进程**：走 hook 那条 loopback 发一条 `dev-scene`，客户端进程收到后
 * 自己显示（见 `src-tauri/src/workbench/app/devsim.rs` 与 `hook_ui::run_preview`）。
 *
 * # 客户端没开
 *
 * 后端**如实报**「客户端窗口没启动 —— 先跑 npm run tauri dev」（不自动拉起任何进程，
 * 作者裁定）。这一页把那句话原样显示出来。
 *
 * # 只接后处理那一组
 *
 * 后面几组（导入 / 文件冲突 / 校验更新 / 浮出的条）先摆出来标「还没做」——
 * 先把**不容易自然触发**的那一屏做对，再用同一套方式扩。加一档只改后端
 * `workbench/app/devsim.rs` 的 `scenes()`。
 */
import { useCallback, useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import type { DevFault, DevScene } from '../api'
import s from '../c14.module.css'

/** 分组按这个顺序摆（后端给的 `group` 键是稳定串，顺序归界面） */
const GROUP_ORDER = ['postprocess', 'import', 'file', 'trust', 'float'] as const

export default function DevPage() {
  const [scenes, setScenes] = useState<DevScene[] | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [note, setNote] = useState<{ text: string; bad: boolean } | null>(null)
  /* 故障注入：清单 + 现在装着哪一档（装/卸都走同一颗按钮那套 busy/note） */
  const [faults, setFaults] = useState<DevFault[] | null>(null)
  const [armed, setArmed] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    wb.devSimScenes()
      .then((rows) => {
        if (alive) setScenes(rows)
      })
      .catch((e: unknown) => {
        if (alive) setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
      })
    wb.devFaults()
      .then((rows) => {
        if (alive) setFaults(rows)
      })
      .catch((e: unknown) => {
        if (alive) setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
      })
    return () => {
      alive = false
    }
  }, [])

  /** 点「演示」：让客户端窗口显示这一屏（客户端窗口会置顶）。 */
  const run = useCallback(async (scene: DevScene) => {
    if (!scene.ready) {
      setNote({ text: `「${scene.label}」还没接上 —— 随后补`, bad: false })
      return
    }
    setBusy(scene.id)
    setNote(null)
    try {
      await wb.devTrigger(scene.id)
      setNote({ text: `已让客户端窗口显示「${scene.label}」—— 看那扇窗（已置顶）`, bad: false })
    } catch (e) {
      setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setBusy(null)
    }
  }, [])

  /**
   * **装 / 卸一档故障**（2026-10-19，作者：「希望所有的模态框都能触发，
   * 而且要真实的 mock 触发」）。
   *
   * 与上面那个 `run` 是**相反的两件事**：`run` 打开一格界面（不动后端），
   * 这个让**某个 API 方法**从此真的报错（不动界面）。于是客户端里所有走那个方法的
   * 界面一起中 —— 真组件、真按钮、真错误码，没有"预览模式"那一层假。
   */
  const toggleFault = useCallback(
    async (row: DevFault) => {
      const off = armed === row.id
      setBusy(row.id)
      setNote(null)
      try {
        if (off) await wb.devFaultClear()
        else await wb.devFaultArm(row.id)
        setArmed(off ? null : row.id)
        setNote({
          text: off
            ? `已卸下「${row.label}」—— 客户端恢复正常`
            : `已装上「${row.label}」—— 客户端里走 ${row.method} 的界面现在都会真的报错，去点一下试试`,
          bad: false,
        })
      } catch (e) {
        setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
      } finally {
        setBusy(null)
      }
    },
    [armed],
  )

  /* 分组：按 GROUP_ORDER 排，认不出的组挂到末尾（后端加了新组不会被吞掉） */
  const grouped = (scenes ?? []).reduce<Record<string, DevScene[]>>((acc, sc) => {
    ;(acc[sc.group] ??= []).push(sc)
    return acc
  }, {})
  const groupKeys = [
    ...GROUP_ORDER.filter((g) => grouped[g] !== undefined),
    ...Object.keys(grouped).filter((g) => !(GROUP_ORDER as readonly string[]).includes(g)),
  ]

  return (
    <div className={s.flow} style={{ padding: 16 }}>
      <div className={s.card}>
        <div className={s.cardHead}>
          <b>开发台</b>
          <span className={s.cardNote}>客户端界面触发器 · 仅开发环境</span>
        </div>
        <div className={s.cardBody}>
          <div className={s.vfield}>
            <p className={s.vhelp}>
              点击场景，在<b>客户端窗口中</b>显示对应界面。不执行真实后处理、不读写打印文件、
              不需要连接真实钩子 —— 只是把那一屏按模拟状态演给客户端窗口看（窗口会自动置顶）。
            </p>
            <p className={s.vhelp}>
              ★ 客户端窗口没开着的话，会提示先跑{' '}
              <span className={s.mono}>npm run tauri dev</span>（这里不替你拉起进程）。
              演示出来的那一屏带「预览模式」标记，那几颗真业务按钮（停止 / 后台运行）
              在预览里不接真流程。
            </p>
            <p className={s.vhelp}>
              ★ <b>多点几个会叠加</b>（正好用来验层序）：<b>不同类可以同时摆着</b>
              （比如更新面板 + 参数对比台 + 提示条），<b>同一类只留最后点的那个</b>
              （连点两次「重命名」= 第二个顶掉第一个）。层序从高到低 ——{' '}
              <b>后处理那一屏 &gt; 标题栏上的软件更新面板 &gt; 模态框</b>。
              点哪一格就自动切到它所在的页签。
            </p>
          </div>

          {scenes === null ? (
            <p className={s.vhelp}>读取场景清单……</p>
          ) : (
            groupKeys.map((g) => {
              const rows = grouped[g]
              if (rows.length === 0) return null
              return (
                <div className={s.vfield} key={g}>
                  <div className={s.vhead}>
                    <b>{rows[0].groupLabel}</b>
                    <span className={s.vkey}>
                      {rows.every((r) => r.ready) ? '都能点' : '部分还没接'}
                    </span>
                  </div>
                  {rows.map((sc) => (
                    <div key={sc.id} className={s.vrow} style={{ alignItems: 'flex-start' }}>
                      <span className={s.grow}>
                        <b>{sc.label}</b>
                        <span className={s.vkey} style={{ marginLeft: 8 }}>
                          {sc.slotLabel}
                        </span>
                        {!sc.ready && (
                          <span className={s.vkey} style={{ marginLeft: 8 }}>
                            还没做
                          </span>
                        )}
                        <span className={s.vhelp} style={{ display: 'block' }}>
                          {sc.note}
                        </span>
                      </span>
                      <button
                        type="button"
                        className={`${s.btn} ${sc.ready ? s.btnPrimary : ''}`}
                        disabled={busy !== null}
                        title={sc.ready ? `在客户端窗口显示：${sc.label}` : '这一档还没接上'}
                        onClick={() => void run(sc)}
                      >
                        {busy === sc.id ? '触发中…' : sc.ready ? '演示' : '还没做'}
                      </button>
                    </div>
                  ))}
                </div>
              )
            })
          )}

          {/*
           * ★ 2026-10-19：**故障注入** —— 与上面那张场景表相反的两件事。
           * 那边"打开一格界面"，这边"让某个 API 方法真的报错"。装了之后客户端里
           * **所有**走那个方法的界面一起中（真组件 / 真按钮 / 真错误码），
           * 不用逐个框做一遍。
           */}
          {faults !== null && faults.length > 0 && (
            <div className={s.vfield}>
              <div className={s.vhead}>
                <b>故障注入</b>
                <span className={s.vkey}>
                  {armed === null ? '没装' : `装着：${armed}`} · 按方法生效
                </span>
              </div>
              <span className={s.vhelp} style={{ display: 'block' }}>
                装上之后，**客户端里所有走这个方法的界面**都会真的报这一条错 ——
                界面、按钮、文案、错误码全是真的（不是"预览模式"那一套）。
                自己去客户端点一下那个框就看得见；换一档或再点一次同一档即卸下。
              </span>
              {faults.map((row) => {
                const on = armed === row.id
                return (
                  <div key={row.id} className={s.vrow} style={{ alignItems: 'flex-start' }}>
                    <span className={s.grow}>
                      <b>{row.label}</b>
                      <span className={s.vkey} style={{ marginLeft: 8 }}>
                        {row.method}
                      </span>
                      <span className={s.vhelp} style={{ display: 'block' }}>
                        {row.note}
                      </span>
                    </span>
                    <button
                      type="button"
                      className={`${s.btn} ${on ? '' : s.btnPrimary}`}
                      disabled={busy !== null}
                      title={on ? `卸下「${row.label}」` : `装上「${row.label}」`}
                      onClick={() => void toggleFault(row)}
                    >
                      {busy === row.id ? '处理中…' : on ? '卸下' : '装上'}
                    </button>
                  </div>
                )
              })}
            </div>
          )}

          {note !== null && (
            <p className={s.vhelp} style={note.bad ? { color: 'var(--danger)' } : undefined}>
              {note.text}
            </p>
          )}

          <p className={s.vhelp}>
            ★ 加一格：在后端 <span className={s.mono}>workbench/app/devsim.rs</span> 的{' '}
            <span className={s.mono}>scenes()</span> 里加一行（给一个稳定 slug），
            再到客户端持有那一格界面的组件里写一行{' '}
            <span className={s.mono}>useDevPreview('&lt;slug&gt;')</span>。
          </p>
        </div>
      </div>
    </div>
  )
}
