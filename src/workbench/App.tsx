/**
 * 工作台外壳（doc §7）。
 *
 * # 三条纪律（照参考实现，它们是这套架构成立的前提）
 *
 * 1. **树常驻，不是「当前对象选择器」。** 选中是两套：单击 = 切主选中
 *    （点机型行编基底，点版本行编覆盖）；勾选 = 加入对比，给矩阵当列。
 *    上一版把「当前机型 + 当前版本」做成全局上下文，等于强迫你一个一个改。
 * 2. **三个视角是视角不是步骤**，随便切，没有前后关系。
 * 3. **维护四项不属于任何机型版本，不受树的选中影响。** 渲染优先级：维护页 > 视角；
 *    点第二次同一项取消。
 *
 * # 外壳自己不做业务
 *
 * 它只做四件事：取数、算徽章、拼动作数组、分发到视角。状态、文案、能不能改
 * 全是后端算好的（doc §1 第二条铁律）。视角组件一律只收一个 prop —— 换实现不动外壳。
 *
 * # 撤销栈在会话内存里，但写必须走 IPC（doc §4.4）
 *
 * 栈里存的是后端返回的 `inverse`，撤销就是把它再交给 `wb_apply_draft`。
 * 前端自己算反向会在「原来是继承来的」这种情形上出错。
 * 第一版**不做跨重启的撤销历史**：关了重开草稿还在，但栈清空。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import {
  isAppError,
  wb,
  type BookView,
  type Boot,
  type ColRef,
  type Patch,
  type Words,
} from './api'
import { useDensity } from './useDensity'
import { PageHeader, type Action, type Badge } from './shell/PageHeader'
import { StatusBar } from './shell/StatusBar'
import { StatusStrip } from './shell/StatusStrip'

/** 三个视角。**是视角不是步骤** */
const VIEWS = [
  { id: 'params', label: '参数' },
  { id: 'menu', label: '套餐与菜单' },
  { id: 'build', label: '生成' },
] as const
type ViewId = (typeof VIEWS)[number]['id']

/** 维护四项。标题逐字「维护」 */
const MAINTAIN = [
  { id: 'fields', label: '字段定义' },
  { id: 'stock', label: '仓库盘点' },
  { id: 'fallback', label: '应急规则' },
  { id: 'trash', label: '回收站' },
] as const
type MaintainId = (typeof MAINTAIN)[number]['id']

/** 主选中：机型行（`uid` 为 null = 编基底）或版本行 */
interface Focus {
  machineId: string
  uid: string | null
}

interface UndoEntry {
  label: string
  patches: Patch[]
}

export function WorkbenchApp() {
  const shellRef = useRef<HTMLDivElement>(null)
  useDensity(shellRef)

  const [boot, setBoot] = useState<Boot | null>(null)
  const [words, setWords] = useState<Words | null>(null)
  const [book, setBook] = useState<BookView | null>(null)
  const [fatal, setFatal] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const [view, setView] = useState<ViewId>('params')
  const [maintain, setMaintain] = useState<MaintainId | null>(null)
  const [focus, setFocus] = useState<Focus | null>(null)
  const [checked, setChecked] = useState<ColRef[]>([])
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set())

  /** 撤销 / 重做栈。会话内存，关窗就没 —— 草稿本身还在盘上 */
  const [undoStack, setUndoStack] = useState<UndoEntry[]>([])
  const [redoStack, setRedoStack] = useState<UndoEntry[]>([])

  /** 出错时把 message 留下。`AppError` 与客户端共用同一套 */
  const fail = useCallback((e: unknown) => {
    setFatal(isAppError(e) ? `${e.message}${e.detail ? ` —— ${e.detail}` : ''}` : String(e))
  }, [])

  /* 首屏三步：先 boot（上游缺失时也能显示数据根）→ 取词表 → 取整本 */
  useEffect(() => {
    void (async () => {
      try {
        const b = await wb.boot()
        setBoot(b)
        if (!b.info) return // 上游缺失：**不启动业务**（doc §15 第一条）
        setWords(await wb.words())
        setBook(await wb.book())
      } catch (e) {
        fail(e)
      }
    })()
  }, [fail])

  /**
   * 走唯一写入口。
   *
   * `where` 说的是这次结果往哪个栈压：正向操作压撤销栈，撤销压重做栈，重做压撤销栈。
   * 三种情形走同一条路径（doc §4.4 的那张图），所以只有一个函数 ——
   * 分三份写的话，「撤销之后能不能重做」这件事会有三种写法。
   *
   * Task 14 的三种改法直接调它，`where: 'undo'`。
   */
  const run = useCallback(
    async (label: string, patches: Patch[], where: 'undo' | 'redo') => {
      setBusy(true)
      try {
        const out = await wb.applyDraft(label, patches)
        setBook(out.view)
        // 不可撤销的手势（删除、生成记录）**不进栈**，否则栈里会有一条按不动的
        if (out.inverse.length > 0) {
          const entry = { label, patches: out.inverse }
          if (where === 'undo') {
            setUndoStack((s) => [...s, entry])
          } else {
            setRedoStack((s) => [...s, entry])
          }
        }
        return out
      } catch (e) {
        fail(e)
        return null
      } finally {
        setBusy(false)
      }
    },
    [fail],
  )

  const undo = useCallback(async () => {
    const top = undoStack[undoStack.length - 1]
    if (!top) return
    // 撤销的结果压进重做栈；成功之后才把它从撤销栈里去掉
    const out = await run(`撤销：${top.label}`, top.patches, 'redo')
    if (out) setUndoStack((s) => s.slice(0, -1))
  }, [undoStack, run])

  const redo = useCallback(async () => {
    const top = redoStack[redoStack.length - 1]
    if (!top) return
    const out = await run(`重做：${top.label}`, top.patches, 'undo')
    if (out) setRedoStack((s) => s.slice(0, -1))
  }, [redoStack, run])

  const save = useCallback(async () => {
    setBusy(true)
    try {
      const out = await wb.save()
      setBook(out.view)
      /* **uid 会变**（新建与移动都会），所以选中与勾选列要跟着改 ——
         不改的话保存之后选中会指向一个不存在的 uid，界面上只表现为「选中莫名没了」 */
      const remap = out.remap
      if (Object.keys(remap).length > 0) {
        setFocus((f) => (f?.uid && remap[f.uid] ? { ...f, uid: remap[f.uid] } : f))
        setChecked((cs) =>
          cs.map((c) =>
            c.versionUid && remap[c.versionUid]
              ? { ...c, versionUid: remap[c.versionUid] }
              : c,
          ),
        )
      }
      // 保存之后撤销栈作废：栈里的反向 patch 指的是保存前那一份状态
      setUndoStack([])
      setRedoStack([])
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }, [fail])

  const discard = useCallback(async () => {
    setBusy(true)
    try {
      setBook(await wb.discard())
      setUndoStack([])
      setRedoStack([])
    } catch (e) {
      fail(e)
    } finally {
      setBusy(false)
    }
  }, [fail])

  /* ---------- 徽章与动作 ---------- */

  const badges: Badge[] = useMemo(() => {
    if (!book || !words) return []
    const b = book.badges
    return [
      { count: b.machines, label: '机型' },
      { count: b.versions, label: '版本' },
      {
        count: b.baseItems,
        own: b.baseOwn,
        label: `${words.level.machine.label} 项`,
        title: '这一层一共有几项非默认值；「自有」是我们自己写的那几项',
      },
      {
        count: b.overrideItems,
        own: b.overrideOwn,
        label: `${words.level.version.label} 项`,
      },
    ]
  }, [book, words])

  const actions: Action[] = useMemo(() => {
    if (!book || !words) return []
    const clean = book.dirtyCount === 0
    const out: Action[] = []

    out.push(
      undoStack.length === 0 || busy
        ? {
            id: 'undo',
            label: '撤销',
            disabled: true,
            disabledReason: busy ? '正在处理上一步' : words.disabled.nothingToUndo,
          }
        : {
            id: 'undo',
            label: '撤销',
            title: `撤销：${undoStack[undoStack.length - 1].label}`,
            onClick: () => void undo(),
          },
    )
    out.push(
      redoStack.length === 0 || busy
        ? {
            id: 'redo',
            label: '重做',
            disabled: true,
            disabledReason: busy ? '正在处理上一步' : '没有可以重做的操作',
          }
        : {
            id: 'redo',
            label: '重做',
            title: `重做：${redoStack[redoStack.length - 1].label}`,
            onClick: () => void redo(),
          },
    )
    out.push(
      clean || busy
        ? {
            id: 'discard',
            label: '丢弃改动',
            tone: 'danger',
            disabled: true,
            disabledReason: busy ? '正在处理上一步' : words.disabled.nothingToSave,
          }
        : {
            id: 'discard',
            label: '丢弃改动',
            tone: 'danger',
            title: `把这 ${book.dirtyCount} 处未保存的改动全部扔掉，回到上次保存的样子`,
            onClick: () => void discard(),
          },
    )
    out.push(
      clean || busy
        ? {
            id: 'save',
            label: '保存配方',
            tone: 'primary',
            disabled: true,
            disabledReason: busy ? '正在处理上一步' : words.disabled.nothingToSave,
          }
        : {
            id: 'save',
            label: '保存配方',
            tone: 'primary',
            title: `把这 ${book.dirtyCount} 处改动写进仓库文件`,
            onClick: () => void save(),
          },
    )
    return out
  }, [book, words, undoStack, redoStack, busy, undo, redo, discard, save])

  /* ---------- 渲染 ---------- */

  const focusText = useMemo(() => {
    if (!focus || !book || !words) return '没有主选中'
    const m = book.machines.find((x) => x.id === focus.machineId)
    if (!m) return '没有主选中'
    if (!focus.uid) return `主选中 ${m.display} · ${words.level.machine.label}`
    const v = m.versions.find((x) => x.uid === focus.uid)
    return `主选中 ${m.display} / ${v?.name ?? focus.uid}`
  }, [focus, book, words])

  return (
    <div className="wb" data-wb ref={shellRef}>
      {boot && words && book ? (
        <StatusStrip
          recipePath={boot.roots.workbench}
          distPath={boot.roots.dist}
          save={book.save}
          artifact={book.artifact}
          lastBuild={book.lastBuild}
          words={words}
        />
      ) : (
        <div className="wb-strip wb-strip--loading">正在读上游与配方本…</div>
      )}

      <PageHeader
        title="配方本"
        subtitle="开发者工具 · 不随客户端交付"
        badges={badges}
        actions={actions}
      />

      {fatal && <div className="wb-banner" data-tone="danger">{fatal}</div>}

      {boot?.problem && (
        <div className="wb-banner" data-tone="danger">
          {boot.problem}
          {boot.detail && <> —— <span className="wb-mono">{boot.detail}</span></>}
          <br />
          没有上游工作台不启动业务（字段定义、机型版本、交付资源全在它里面）。
          可以用环境变量 <span className="wb-mono">MKPSE_PRESETS_DIR</span> 指过去，
          改完点<button type="button" className="wb-link" onClick={() => location.reload()}>
            重新加载
          </button>。
        </div>
      )}

      {book?.notices.map((n) => (
        <div key={n} className="wb-banner" data-tone="warn">
          {n}
        </div>
      ))}

      <div className="wb-body">
        <aside className="wb-side">
          <div className="wb-side__head">配方本</div>
          <div className="wb-side__list">
            {book?.machines.map((m) => {
              const open = !collapsed.has(m.id)
              return (
                <div key={m.id}>
                  <div
                    className="wb-row"
                    data-kind="machine"
                    data-focus={focus?.machineId === m.id && !focus.uid ? 'yes' : undefined}
                  >
                    <button
                      type="button"
                      className="wb-row__twist"
                      aria-label={open ? `折叠 ${m.display}` : `展开 ${m.display}`}
                      onClick={() =>
                        setCollapsed((s) => {
                          const next = new Set(s)
                          if (next.has(m.id)) next.delete(m.id)
                          else next.add(m.id)
                          return next
                        })
                      }
                    >
                      {open ? '▾' : '▸'}
                    </button>
                    <input
                      type="checkbox"
                      /* **带机型** —— 同名版本在多个机型下都存在，只写版本名的话
                         读屏软件念出来的三条「标准版」分不清是哪台机器 */
                      aria-label={`把 ${m.display} 的机型基底加入对比`}
                      checked={checked.some((c) => c.machineId === m.id && !c.versionUid)}
                      onChange={(e) =>
                        setChecked((cs) =>
                          e.target.checked
                            ? [...cs, { machineId: m.id, versionUid: null }]
                            : cs.filter((c) => !(c.machineId === m.id && !c.versionUid)),
                        )
                      }
                    />
                    <button
                      type="button"
                      className="wb-row__name"
                      onClick={() => {
                        setFocus({ machineId: m.id, uid: null })
                        setMaintain(null)
                      }}
                    >
                      {m.display}
                    </button>
                    <span className="wb-tag" title={words?.build[m.build].explain ?? undefined}
                      data-state={m.build === 'noResources' ? 'off' : m.build === 'built' ? 'ok' : 'warn'}>
                      {words?.build[m.build].label}
                    </span>
                    <span className="wb-row__count">
                      {m.total} 项 · 自有 {m.own}
                    </span>
                  </div>

                  {open &&
                    m.versions.map((v) => (
                      <div
                        key={v.uid}
                        className="wb-row"
                        data-kind="version"
                        data-focus={focus?.uid === v.uid ? 'yes' : undefined}
                      >
                        <span className="wb-row__twist" />
                        <input
                          type="checkbox"
                          aria-label={`把 ${m.display} 的 ${v.name} 加入对比`}
                          checked={checked.some((c) => c.versionUid === v.uid)}
                          onChange={(e) =>
                            setChecked((cs) =>
                              e.target.checked
                                ? [...cs, { machineId: m.id, versionUid: v.uid }]
                                : cs.filter((c) => c.versionUid !== v.uid),
                            )
                          }
                        />
                        <button
                          type="button"
                          className="wb-row__name"
                          onClick={() => {
                            setFocus({ machineId: m.id, uid: v.uid })
                            setMaintain(null)
                          }}
                        >
                          {v.name}
                          {v.isNew && <i className="wb-row__new" title="新建，还没保存" />}
                        </button>
                        <span
                          className="wb-tag"
                          title={words?.build[v.build].explain ?? undefined}
                          data-state={
                            v.build === 'noResources'
                              ? 'off'
                              : v.build === 'built'
                                ? 'ok'
                                : 'warn'
                          }
                        >
                          {words?.build[v.build].label}
                        </span>
                        <span className="wb-row__count">自有 {v.own}</span>
                      </div>
                    ))}
                </div>
              )
            })}
          </div>

          <div className="wb-side__head">维护</div>
          <div className="wb-side__maintain">
            {MAINTAIN.map((m) => (
              <button
                key={m.id}
                type="button"
                className="wb-mitem"
                data-on={maintain === m.id ? 'yes' : undefined}
                /* 点第二次同一项取消 —— 否则进了维护页就出不来 */
                onClick={() => setMaintain((cur) => (cur === m.id ? null : m.id))}
              >
                {m.label}
                {m.id === 'trash' && book && book.archived.length > 0 && (
                  <span className="wb-badge wb-badge--mini">{book.archived.length}</span>
                )}
              </button>
            ))}
          </div>
        </aside>

        <section className="wb-main">
          {/* 维护页 > 视角（第三条纪律）*/}
          {maintain ? (
            <>
              <div className="wb-tabs">
                <span className="wb-tab" data-on="yes">
                  {MAINTAIN.find((m) => m.id === maintain)?.label}
                </span>
              </div>
              <div className="wb-pane">
                <p className="wb-todo">
                  这一页在 Task 18 落地。后端已经就位：
                  <span className="wb-mono"> wb_registry / wb_stock / wb_fallback / wb_trash</span>
                </p>
              </div>
            </>
          ) : (
            <>
              <div className="wb-tabs">
                {VIEWS.map((v) => (
                  <button
                    key={v.id}
                    type="button"
                    className="wb-tab"
                    data-on={view === v.id ? 'yes' : undefined}
                    onClick={() => setView(v.id)}
                  >
                    {v.label}
                    {v.id === 'build' && book && (
                      <span className="wb-badge wb-badge--mini">
                        {book.buildRows.filter((r) => r.buildable).length}
                      </span>
                    )}
                  </button>
                ))}
              </div>
              <div className="wb-pane">
                <p className="wb-todo">
                  {view === 'params' && (
                    <>
                      参数矩阵在 Task 13–15 落地。现在勾了 {checked.length} 列；
                      后端 <span className="wb-mono">wb_matrix</span> 已经能按配方本顺序
                      把列排好、行取并集、每格带来源与「被谁关着」。
                    </>
                  )}
                  {view === 'menu' && <>套餐与菜单在 Task 16 落地。</>}
                  {view === 'build' && <>生成视角在 Task 17 落地（含 Task 9 的校验三档）。</>}
                </p>
              </div>
            </>
          )}
        </section>
      </div>

      {book && words ? (
        <StatusBar
          badges={book.badges}
          focus={focusText}
          save={book.save}
          dirtyCount={book.dirtyCount}
          words={words}
        />
      ) : (
        <footer className="wb-foot" />
      )}
    </div>
  )
}
