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
 *
 * # ★ 这一页最终是三件**互不重叠**的事（作者 2026-10-19 定稿）
 *
 *  - **① 打开真实界面**（下面那张场景表）：不动后端，只是把那一屏显示出来；
 *  - **② 运行真实后处理**（`workbench/app/devreal.rs`）：**真的跑一趟管线** —— 真读文件、
 *    真预检、真进度、真问句、真落盘、真归档；界面上**没有「预览」标记**，那几颗真业务
 *    按钮（停止 / 后台运行 / 强制继续 / 终止）**点了真的有用**；
 *  - **③ 故障注入**（`workbench/app/devfault.rs`）：让某个 API 方法失一次真（不动界面）。
 *
 * ①③ 是"给界面 / 后端喂一个场景"，**② 是真跑** —— 别再把 ② 当成"另一种预览"。
 */
import { useCallback, useEffect, useState } from 'react'
import { open } from '@tauri-apps/plugin-dialog'

import { isAppError, wb } from '../api'
import type { DevFault, DevRunAck, DevScene, MatrixState } from '../api'
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
  /* ② 运行真实后处理：两份输入（路径由系统选择器给，界面不自己拼） + 回执（副本落点） */
  const [preset, setPreset] = useState('')
  const [gcode, setGcode] = useState('')
  const [ack, setAck] = useState<DevRunAck | null>(null)
  /* 六档矩阵：档位表 + 最近判定 + 现在跑到哪（2 秒轮询一次，不引事件推送） */
  const [matrix, setMatrix] = useState<MatrixState | null>(null)

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

  /**
   * 矩阵状态：**2 秒轮询**，不走事件推送。
   *
   * 为什么：矩阵跑的时候工作台只想问"跑到哪一档 / 有没有结论"，一轮 IPC 读几个小 JSON，
   * 比"客户端 → loopback → 工作台"那条跨进程推送便宜得多。
   * 轮询失败**不吵人**（开发台自己掉链子不该往 `note` 里塞红字），手动操作时才报错。
   */
  useEffect(() => {
    let alive = true
    const tick = () => {
      wb.devMatrixState()
        .then((st) => {
          if (alive) setMatrix(st)
        })
        .catch(() => {
          /* 静默：下一轮再说 */
        })
    }
    tick()
    const timer = window.setInterval(tick, 2000)
    return () => {
      alive = false
      window.clearInterval(timer)
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

  /**
   * **挑一份输入**（② 用）：路径由**系统选择器**给，界面不自己拼
   * （拼错了的表现是"钩子没当钩子跑、客户端一动不动"，只能真选一次）。
   */
  const pick = useCallback(async (kind: 'preset' | 'gcode') => {
    const one = await open({
      multiple: false,
      directory: false,
      filters:
        kind === 'preset'
          ? [{ name: '预设', extensions: ['toml'] }]
          : [{ name: 'G-code', extensions: ['gcode'] }],
    })
    if (typeof one !== 'string') return /* 点了取消 = 什么都不动 */
    if (kind === 'preset') setPreset(one)
    else setGcode(one)
    setAck(null)
  }, [])

  /**
   * **② 运行真实后处理**（2026-10-19；作者：「不要再扩充预览，而是给工作台开发台
   * 增加一个真实后处理测试入口」）。
   *
   * 与上面两个回调都不同：`run` 是演出，`toggleFault` 是喂一个假错，
   * 这一个**真的起一个钩子子进程**（`--Toml` + `--Gcode`，与切片器注册的那一行同形），
   * 于是**客户端窗口跑真管线**：真预检、真进度、真问句、真落盘、真归档。
   * 命令**立刻返回**（不等那一趟跑完）—— 进度与结论去客户端窗口看。
   *
   * ★ 跑的是**副本**：真后处理原地覆盖输入，所以后端先复制一份，你选的那份一个字不动。
   */
  const runReal = useCallback(async () => {
    if (preset === '' || gcode === '') {
      setNote({ text: '先各选一份：预设（.toml）+ 测试 G-code（.gcode）', bad: false })
      return
    }
    setBusy('real')
    setNote(null)
    setAck(null)
    try {
      const got = await wb.devRunReal(preset, gcode)
      setAck(got)
      setNote({
        text: '已起一趟真实后处理 —— 切到客户端窗口看（真预检 / 真进度 / 真问句 / 真落盘）',
        bad: false,
      })
    } catch (e) {
      setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setBusy(null)
    }
  }, [preset, gcode])

  /**
   * **跑矩阵**（传空数组 = 全部六档）。
   *
   * 命令**立刻返回**：真正的串行在 Rust 那边的 worker 线程里（一档一档 spawn + wait），
   * 进度靠上面那个轮询。问句档（机型不匹配 / 挪塔碰撞 / 取消）会停在**客户端窗口**上
   * 等人操作 —— 这里不代答、也不替你点停止。
   */
  const runMatrix = useCallback(async (ids: string[]) => {
    setBusy('matrix')
    setNote(null)
    try {
      await wb.devMatrixStart(ids)
      setNote({
        text:
          ids.length === 0
            ? '已开始跑全部六档 —— 机型 / 碰撞 / 取消这三档会停下来等你在客户端操作'
            : '已开始跑这一档 —— 看客户端窗口',
        bad: false,
      })
    } catch (e) {
      setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setBusy(null)
    }
  }, [])

  /** 清空矩阵临时根（后端会先拒绝"还有任务在跑"） */
  const clearMatrix = useCallback(async () => {
    setBusy('matrix-clear')
    setNote(null)
    try {
      const n = await wb.devMatrixClear()
      setNote({ text: `已清空 ${n} 个临时根`, bad: false })
    } catch (e) {
      setNote({ text: isAppError(e) ? e.message : String(e), bad: true })
    } finally {
      setBusy(null)
    }
  }, [])

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
              点击场景，在<b>客户端窗口中</b>显示对应界面。<b>这一块不跑后处理</b>：
              不读 G-code、不跑管线、不落任何记录 —— 只是把那一屏按模拟状态演给客户端窗口看
              （窗口会自动置顶）。要<b>真跑一趟</b>，用下面「运行真实后处理」那一块。
            </p>
            <p className={s.vhelp}>
              ★ <b>2026-10-19：后处理的「预览」整组撤了，一格都不剩</b> —— 后处理那一页现在
              只剩两件真事：下面「运行真实后处理」（单趟）与「测试矩阵」（八档）。
              成功 / 失败 / 已停止 / 机型不匹配 / 等超时 / 收进后台 / 校准件 / 挪塔碰撞，
              <b>全部走真入口</b>，判定只读真凭据。
            </p>
            <p className={s.vhelp}>
              ★ 客户端窗口没开着的话，会提示先跑{' '}
              <span className={s.mono}>npm run tauri dev</span>（这里不替你拉起进程）。
              演示出来的一屏带「预览模式」标记，那几颗真业务按钮（停止 / 后台运行）
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
                        data-tip={sc.ready ? `在客户端窗口显示：${sc.label}` : '这一档还没接上'}
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
           * ★★ 2026-10-19：**运行真实后处理**（开发台三件事里的第 ② 件）。
           *
           * 与上面那张场景表（演出）**根本不是一回事**：这里真的起一个钩子子进程
           * （`--Toml` + `--Gcode`，与切片器注册的那一行同形），客户端窗口照真流程跑。
           * 于是那一屏上：没有「预览模式」标记、页脚是真机那几颗按钮、点停止真的停。
           */}
          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>运行真实后处理</b>
              <span className={s.vkey}>真跑 · 真落盘 · 真归档</span>
            </div>
            <span className={s.vhelp} style={{ display: 'block' }}>
              选一份预设（<span className={s.mono}>.toml</span>）与一份测试 G-code（
              <span className={s.mono}>.gcode</span>），点「运行」—— 起一个<b>真的钩子子进程</b>，
              <b>客户端窗口</b>照真流程跑：真预检、真进度、真问句、真落盘、真归档。
              那一屏<b>没有「预览模式」标记</b>，真业务按钮（停止 / 后台运行 / 强制继续 / 终止）
              <b>点了真的有用</b>。
            </span>

            <div className={s.vrow} style={{ alignItems: 'flex-start' }}>
              <span className={s.grow}>
                <b>预设</b>
                <span className={s.vhelp} style={{ display: 'block' }}>
                  {preset === '' ? (
                    '还没选 —— 客户端本地预设在「用户数据根/presets-local/」下'
                  ) : (
                    <span className={s.mono}>{preset}</span>
                  )}
                </span>
              </span>
              <button
                type="button"
                className={s.btn}
                disabled={busy !== null}
                onClick={() => void pick('preset')}
              >
                选择
              </button>
            </div>

            <div className={s.vrow} style={{ alignItems: 'flex-start' }}>
              <span className={s.grow}>
                <b>测试 G-code</b>
                <span className={s.vhelp} style={{ display: 'block' }}>
                  {gcode === '' ? (
                    '还没选 —— 用一份真的切片件（越大越看得清进度）'
                  ) : (
                    <span className={s.mono}>{gcode}</span>
                  )}
                </span>
              </span>
              <button
                type="button"
                className={s.btn}
                disabled={busy !== null}
                onClick={() => void pick('gcode')}
              >
                选择
              </button>
            </div>

            <div className={s.vrow} style={{ alignItems: 'flex-start' }}>
              <span className={s.grow}>
                <span className={s.vhelp} style={{ display: 'block' }}>
                  {ack === null ? (
                    '选齐两份就能跑 —— 跑起来看客户端窗口'
                  ) : (
                    <>
                      这一趟在处理<b>副本</b>：<span className={s.mono}>{ack.gcode}</span>
                    </>
                  )}
                </span>
              </span>
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary}`}
                disabled={busy !== null || preset === '' || gcode === ''}
                title="真的跑一趟后处理（不是演出）"
                onClick={() => void runReal()}
              >
                {busy === 'real' ? '起任务中…' : '运行真实后处理'}
              </button>
            </div>

            <span className={s.vhelp} style={{ display: 'block' }}>
              ★ <b>跑的是副本</b>：真后处理原地覆盖输入，所以后端先复制一份到客户端内部根的{' '}
              <span className={s.mono}>run/dev-run/</span>，你选的那份一个字节都没动 ——
              于是能反复跑、换文件跑。★ 客户端窗口要先开着（这里不替你拉进程）。
            </span>
            <span className={s.vhelp} style={{ display: 'block' }}>
              ★ <b>工作台的测试模式（沙箱）管不到这一趟</b>：沙箱只切工作台自己那棵仓库树，
              钩子进程走的是<b>客户端正式那棵数据树</b> —— 这一趟会真的写客户端的执行记录与归档，
              别拿正式数据试。
            </span>
          </div>

          {/*
           * ★★ 2026-10-19：**真实后处理测试矩阵**（作者：「我怎么触发它成功、怎么触发它失败、
           * 怎么触发它碰撞检测」）。
           *
           * 与上面「运行真实后处理」是同一件事的两种用法：那边你自己挑两份文件跑一次；
           * 这边六档**确定性输入**，一键串行，每档一个独立临时根，跑完按**真凭据**判定。
           * 判定不在这里做 —— 前端只显示后端读出来的 `error` / `cancelled` / `relocation`。
           */}
          <div className={s.vfield}>
            <div className={s.vhead}>
              <b>真实后处理测试矩阵</b>
              <span className={s.vkey}>
                {matrix?.current != null ? `正在跑：${matrix.current}` : '串行 · 每档独立临时根'}
              </span>
            </div>
            <span className={s.vhelp} style={{ display: 'block' }}>
              六档确定性输入，一键串行跑：小文件成功 / 大文件成功 / 硬失败 / 机型不匹配 /
              挪塔碰撞 / 取消。判定只读每档目录里那条执行记录的{' '}
              <span className={s.mono}>error</span> / <span className={s.mono}>cancelled</span> /{' '}
              <span className={s.mono}>relocation</span> —— 不看退出码猜，也不由界面"认为"成功。
            </span>
            <span className={s.vhelp} style={{ display: 'block' }}>
              ★ <b>机型不匹配 / 挪塔碰撞 / 取消</b>这三档要你在<b>客户端窗口里真操作</b>
              （这里不代答、不替你点停止）。跑着的那一档会一直亮着，直到你答完或它超时 ——
              验的不只是"管线能问"，还有"界面能显示问句、能收答复、能继续或停下"。
            </span>
            <span className={s.vhelp} style={{ display: 'block' }}>
              ★ 每档一个独立临时根（<span className={s.mono}>target/dev-run/matrix/</span>），
              产物与执行记录都在里面，<b>正式数据目录一个字节不动</b>；代价是这些趟
              <b>不进报告页 / 历史</b>。只留最近 10 个，「清空」会先看有没有任务在跑。
            </span>

            <div className={s.vrow}>
              <span className={s.grow}>
                <b>六档</b>
                <span className={s.vkey} style={{ marginLeft: 8 }}>
                  {matrix?.runs.length ?? 0} 条历史判定
                </span>
              </span>
              <button
                type="button"
                className={s.btn}
                disabled={busy !== null || matrix?.current != null}
                title="删掉矩阵自己的临时根（有任务在跑会拒绝）"
                onClick={() => void clearMatrix()}
              >
                {busy === 'matrix-clear' ? '清空中…' : '清空临时根'}
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary}`}
                disabled={busy !== null || matrix?.current != null}
                title="按顺序跑全部六档（问句档会停下来等你）"
                onClick={() => void runMatrix([])}
              >
                {busy === 'matrix' ? '启动中…' : '跑全部'}
              </button>
            </div>

            {(matrix?.cases ?? []).map((c) => {
              const v = matrix?.runs.find((r) => r.caseId === c.id)
              const running = matrix?.current === c.id
              return (
                <div key={c.id} className={s.vrow} style={{ alignItems: 'flex-start' }}>
                  <span className={s.grow}>
                    <b>{c.label}</b>
                    <span className={s.vkey} style={{ marginLeft: 8 }}>
                      {c.expect}
                    </span>
                    {c.human && (
                      <span className={s.vkey} style={{ marginLeft: 8 }}>
                        要在客户端操作
                      </span>
                    )}
                    {!c.ready && (
                      <span className={s.vkey} style={{ marginLeft: 8 }}>
                        夹具不在
                      </span>
                    )}
                    <span className={s.vhelp} style={{ display: 'block' }}>
                      {c.note}
                    </span>
                    <span className={s.vhelp} style={{ display: 'block' }}>
                      输入：<span className={s.mono}>{c.input}</span>
                    </span>
                    {running && (
                      <span className={s.vhelp} style={{ display: 'block' }}>
                        ▶ 正在跑 —— 看客户端窗口（这一档可能正等你答）
                      </span>
                    )}
                    {v != null && (
                      <span
                        className={s.vhelp}
                        style={{
                          display: 'block',
                          color: v.status === 'passed' ? undefined : 'var(--danger)',
                        }}
                      >
                        {v.status === 'passed' ? '通过' : '失败'} · {v.note}
                        <span className={s.mono} style={{ display: 'block' }}>
                          {v.dir}
                        </span>
                      </span>
                    )}
                  </span>
                  <button
                    type="button"
                    className={s.btn}
                    disabled={busy !== null || matrix?.current != null || !c.ready}
                    data-tip={c.ready ? `只跑「${c.label}」` : '这一档的夹具不在'}
                    onClick={() => void runMatrix([c.id])}
                  >
                    跑这一档
                  </button>
                </div>
              )
            })}
          </div>

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
                      data-tip={on ? `卸下「${row.label}」` : `装上「${row.label}」`}
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
