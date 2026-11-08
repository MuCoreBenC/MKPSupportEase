/*
 * 未保存改动（外壳级，C14 移植）。
 *
 * 它原来长在参数台的工具行里，现在挂在**状态栏**上 —— 撤销/重做/未保存改动是
 * 全工作台的事（改机型、改可见性、改参数都算），不该是某一页的一个按钮。
 *
 * # 两种看法，一件事（作者 2026-11-02）
 *
 * 作者原话：「我能不能看记录，就是看对比，用模态框，显示代码的 diff……其实我觉得
 * 代码的 diff 就很直观了，对了，这个是对比盘的和影子树的吧」。
 *
 * | 视图 | 后端 | 回答哪一问 |
 * |---|---|---|
 * | **代码**（默认） | `wb_shadow_diff` | 文件因此变成什么样 —— 盘上 `presets/` ↔ 影子树逐行 |
 * | 改了什么 | `wb_diff_draft` | 我动了哪些格子 —— 「X 轴偏移：0.7 → 0.9」 |
 *
 * 两个都**由后端算好**，这个框只负责摆出来。前端不推"哪一步动了哪几格"，
 * 也不自己拼 diff —— 那正是「前端不算业务」要防的。
 *
 * # 代码视图为什么长得跟生成前确认一样
 *
 * 行级 diff 只有一处实现（`views/DiffView` 的 `DiffLines` + `DiffRuler` + `diffRuler.ts`），
 * 目的就是"同一件事不该在两个地方长成两种样子"：行号口径、红绿底、TOML/JSON 上色、
 * 右侧概览条跳转全都照搬，作者在这里看到的手感与生成前确认完全一致。
 *
 * # 两种状态合成的框，**不换大小**（2026-11-02 作者截图）
 *
 * 第一版按 `view` 切 `size`（代码 `xl` / 改了什么 `md`），结果是一个框在两个宽度之间跳，
 * 而且"改了什么"那一页的每一行都挤在一起读不出来。定案：**一个框、固定 `xl`**，
 * 左边永远是有改动的文件清单，右边按视图换内容 —— 文件清单在两种看法下都是同一批文件，
 * 于是它当导航用（点一份，右边立刻从"哪几行"切到"哪几格"）。
 *
 * 作者要的是「也加上功能」：**右边要能就地撤销与重做**。做法是每条变化/每条行块右边
 * 挂一颗「撤销这一笔」，走的是外壳那条 `txnUndo`（同一条栈），撤掉的再给「重做」。
 */

import { useEffect, useMemo, useRef, useState } from 'react'
import { WHY_LABEL, wb, type DiffLine, type ShadowDiff, type ShadowFileDiff } from '../api'
import { settleAll } from '../editQueue'
import { isAppError } from '../api'
import ModalC14 from './ModalC14'
import { DiffLines, DiffRuler } from '../views/DiffView'
import { measureRulerMarks } from '../views/diffRuler'
import type { RulerMark } from '../views/diffRuler'
import { locateIntoView } from '../../components/scroll/locate'
import s from '../c14.module.css'
import d from '../views/ShadowDiffModal.module.css'

interface Props {
  open: boolean
  onClose: () => void
  /**
   * 「保存 / 放弃」这一对也让**这个框**来给（2026-11-02）。
   *
   * 作者口径：改完想知道自己改了什么，先看这份 diff；看过就在框里把事办了 ——
   * 于是页头那三颗（未保存 / 放弃 / 保存）收成一颗「查看改动」，功能都搬进这里。
   *
   * **不给就不画**（`open` 作详细查看时的那种调用）——
   * 与「保存只有一个」不冲突：框里的「保存」调的就是外壳那条 `save()`，
   * 落盘边界仍然只有一处。
   */
  onSave?: () => void
  onDiscard?: () => void
  /** 现在能不能保存（外壳 `saveOn`）/ 能不能放弃（有脏） */
  canSave?: boolean
  canDiscard?: boolean
  /** 正在保存或放弃 */
  busy?: boolean
  /**
   * 页头那个「未保存 N」（与状态栏同一个数，来自后端 `Txn::dirty_count`）。
   *
   * 用它守住一条：**文件层面没有差异 ≠ 没有未保存改动**。值编辑会被入口写成影子字节，
   * 但菜单 / 套餐 / 可见性写在 `workbench/` 自己的 json 里，永远进不了这份文件对比 ——
   * 那几笔仍要在框里说出来，不许把「影子与盘上一模一样」当结论。
   */
  dirtyCount?: number
  /**
   * 撤销 / 重做（2026-11-02 作者：「这个也加上功能」）。
   *
   * **不在这里自己发命令**：撤销栈是外壳那一份（`wb_txn_state` 里的 `canUndo/canRedo`），
   * 框里发一条 `wb_txn_undo` 之后必须让外壳重取整本 —— 否则状态栏仍写着"未保存（2）"。
   * 所以由外壳给函数，框只负责"哪一条上挂按钮"。
   *
   * 撤销完自动重取 diff（框里的账要跟着栈变）；`onUndo` 返回 false = 没撤成，不重取。
   */
  onUndo?: () => Promise<boolean>
  onRedo?: () => Promise<boolean>
  canUndo?: boolean
  canRedo?: boolean
}

type View = 'code' | 'fields'

const STATE_TEXT: Record<ShadowFileDiff['state'], string> = {
  added: '新增',
  modified: '修改',
  removed: '删除',
}

export default function HistoryModal({
  open,
  onClose,
  onSave,
  onDiscard,
  canSave = false,
  canDiscard = false,
  busy = false,
  dirtyCount = 0,
  onUndo,
  onRedo,
  canUndo = false,
  canRedo = false,
}: Props) {
  /* ---------- 改了什么（对象视角） ---------- */
  const [lines, setLines] = useState<DiffLine[] | null>(null)
  const [err, setErr] = useState<string | null>(null)

  /* ---------- 代码（逐行视角） ---------- */
  const [diff, setDiff] = useState<ShadowDiff | null>(null)
  const [diffErr, setDiffErr] = useState<string | null>(null)

  /** 默认看代码 —— 作者点名"代码的 diff 就很直观" */
  const [view, setView] = useState<View>('code')
  /** 清单里选中的那一份（默认第一份） */
  const [sel, setSel] = useState<string | null>(null)
  /**
   * 切到「改了什么」时带着的那几格（`DiffLine.owner:key`）。
   *
   * 从文件清单点进去，右边只列**这一份文件里改的那几格**，不把全仓的改动一起倒出来 ——
   * 作者的问题是"这份文件我改了什么"，不是"全仓我改了什么"（后者是另一个视图的事）。
   */
  const [focusKeys, setFocusKeys] = useState<Set<string> | null>(null)

  const diffRef = useRef<HTMLDivElement | null>(null)
  const [marks, setMarks] = useState<RulerMark[]>([])
  /** 正在撤/重做的那一格（同一时刻只会有一次） */
  const [pendingKey, setPendingKey] = useState<string | null>(null)

  /** 现取两份账（打开时、以及撤销/重做之后都要它） */
  const reload = () => {
    let alive = true
    setErr(null)
    setDiffErr(null)
    wb
      .diffDraft()
      .then((d) => alive && setLines(d))
      .catch((e: unknown) => alive && setErr(isAppError(e) ? e.message : String(e)))
    wb
      .shadowDiff()
      .then((x) => alive && setDiff(x))
      .catch((e: unknown) => alive && setDiffErr(isAppError(e) ? e.message : String(e)))
    return () => {
      alive = false
    }
  }

  /*
   * 每次打开现取一份 —— 影子是被编辑的那一侧，缓存它只会演出过期内容。
   *
   * **先等飞在空中的值写落地再拉**（2026-11-02）：参数台的写入是 120ms 尾随合批、
   * 单飞的（见 `editQueue`），点开框的那一瞬间最后一下改动可能还在飞。
   * 不等它，`wb_shadow_diff` 读到的就是"改到一半"的影子树 —— 作者看到的
   * 「第一次只有加、没有减，改第二次才正常」正是这一帧。
   */
  useEffect(() => {
    if (!open) return
    setLines(null)
    setDiff(null)
    setFocusKeys(null)
    let alive = true
    let stop: (() => void) | undefined
    void settleAll().then(() => {
      if (alive) stop = reload()
    })
    return () => {
      alive = false
      stop?.()
    }
  }, [open])

  /* 依赖里要一个稳定的数组：`diff?.files ?? []` 每次渲染都是新数组，
     `useMemo` 的依赖就永远"变了"（lint 的 exhaustive-deps 正是在说这件事） */
  const files = useMemo(() => diff?.files ?? [], [diff])
  const current: ShadowFileDiff | null =
    files.find((f) => f.rel === sel) ?? files[0] ?? null

  /* 选中项在换 `open` / 刷新后可能不在清单里了 —— 直接退到第一份（不保留悬空 id） */
  const selOk = current !== null && current.rel === sel
  useEffect(() => {
    if (current !== null && !selOk) setSel(current.rel)
  }, [current, selOk])

  /** 概览条只在真有变化、且这一份不是二进制时画（`DiffRuler` 靠 diff 行的锚点） */
  const showRuler = view === 'code' && current !== null && !current.binary

  useEffect(() => {
    if (!showRuler) {
      setMarks([])
      return
    }
    const el = diffRef.current
    if (el === null) {
      setMarks([])
      return
    }
    const measure = () => setMarks(measureRulerMarks(el))
    measure()
    if (typeof ResizeObserver === 'undefined') return
    const ro = new ResizeObserver(measure)
    ro.observe(el)
    return () => ro.disconnect()
  }, [showRuler, current, view])

  /** 点概览条的标记：滚到那一行（与生成前确认同一种手感） */
  const jumpToRow = (row: number) => {
    const el = diffRef.current?.querySelector<HTMLElement>(`[data-row="${row}"]`)
    /* `behavior: 'smooth'` 在 WKWebView 上对元素级滚动容器等于瞬跳（本仓踩过），
       走共用定位件（底层演 transform）；这一处是"跳到 diff 行"，不铺绿底。 */
    if (el !== null && el !== undefined) locateIntoView(el, { flash: false })
  }

  /** 撤销 / 重做一笔，然后重取本框的账 */
  const run = async (kind: 'undo' | 'redo', mark: string) => {
    const fn = kind === 'undo' ? onUndo : onRedo
    if (fn === undefined || pendingKey !== null) return
    setPendingKey(mark)
    try {
      const ok = await fn()
      if (ok) reload()
    } finally {
      setPendingKey(null)
    }
  }

  const changeCount = useMemo(
    () => files.reduce((n, f) => n + (f.state === 'added' ? f.lines.length : f.added), 0),
    [files],
  )

  /*
   * 文件 → 它里面改的那几格。
   *
   * `DiffLine.owner` 是**归属**（机型 id / 版本 uid），`key` 是参数键；一份文件对应一个归属，
   * 所以按 owner 过滤就够了。文件清单与 `DiffLine` 对不上时（比如 registry 那种全局文件）
   * 一律返回空集 —— 那说明"这份文件没有格子级的说法"，右边就照实说，不硬凑。
   */
  /**
   * 这一份文件里改的那几格。
   *
   * 按后端给的 `file` **精确匹配**（2026-11-02）。上一版拿文件名去猜归属
   * （`machines/A1.toml` → `A1`），而参数值全在 `registry/param_registry.toml`，
   * 猜不出任何 owner ⇒ 切到「改了什么」永远 0 格。归属是后端知道的事实，
   * 让前端猜是本末倒置。
   *
   * 命中为空 ⇒ `null`，界面照实说"这份文件没有格子级的说法"（真的结构性改动）。
   */
  const keysOfFile = (rel: string): Set<string> | null => {
    if (lines === null) return null
    const hit = lines.filter((l) => l.file === rel)
    if (hit.length === 0) return null
    return new Set(hit.map((l) => `${l.owner}:${l.key ?? ''}`))
  }

  /** 当前视图下右边要列的那几格（`focusKeys` 为空 = 全部） */
  const shownLines = useMemo(() => {
    if (lines === null) return []
    if (focusKeys === null) return lines
    return lines.filter((l) => focusKeys.has(`${l.owner}:${l.key ?? ''}`))
  }, [lines, focusKeys])

  /* 文件对比是空的、页头却还挂着「未保存 N」—— 那几笔在内存里（菜单 / 套餐 / 可见性） */
  const emptyButDirty = diff !== null && files.length === 0 && dirtyCount > 0

  const subtitle =
    view === 'code'
      ? diff === null
        ? '正在比对盘上的 presets/ 与影子树…'
        : files.length === 0
          ? emptyButDirty
            ? `文件层面没有差异 —— 还有 ${dirtyCount} 处改动只活在内存里，保存会一并写入`
            : '影子与盘上一模一样 —— 没有未保存的改动'
          : `${files.length} 份文件有改动 · +${changeCount} 行 —— 比的是盘上的 presets/ 与影子树，保存就是把右边写成左边`
      : files.length === 0
        ? '保存 = 写进仓库文件（保存本身也能撤销），放弃 = 整条一起丢'
        : `这一份改了几格 —— 右边每一格都能就地撤销`

  /* 文件清单两项共用（代码 / 改了什么 都画同一批文件） */
  const fileItem = (f: ShadowFileDiff) => (
    <button
      key={f.rel}
      type="button"
      className={f.rel === current?.rel ? `${d.item} ${d.itemOn}` : d.item}
      aria-current={f.rel === current?.rel}
      data-tip={f.rel}
      onClick={() => {
        setSel(f.rel)
        setFocusKeys(keysOfFile(f.rel))
      }}
    >
      <span className={d.itemName}>{f.rel}</span>
      <span className={d.itemMeta}>
        <span className={d[`state_${f.state}`]}>{STATE_TEXT[f.state]}</span>
        {f.binary ? (
          <span className={d.binary}>二进制（只显示换没换）</span>
        ) : (
          <span className={d.counts}>
            <span className={d.add}>+{f.added}</span>
            {f.removed > 0 && <span className={d.del}>−{f.removed}</span>}
          </span>
        )}
      </span>
    </button>
  )

  const noFiles = diff !== null && files.length === 0

  return (
    <ModalC14
      open={open}
      title="未保存改动"
      subtitle={subtitle}
      size="xl"
      onClose={onClose}
      headerExtra={
        <div className={d.viewToggle} role="group" aria-label="怎么看这些改动">
          <button
            type="button"
            className={view === 'code' ? `${d.viewBtn} ${d.viewBtnOn}` : d.viewBtn}
            aria-pressed={view === 'code'}
            data-tip="盘上的 presets/ 与影子树逐行对比 —— 保存会改掉的字节"
            onClick={() => setView('code')}
          >
            代码对比
          </button>
          <button
            type="button"
            className={view === 'fields' ? `${d.viewBtn} ${d.viewBtnOn}` : d.viewBtn}
            aria-pressed={view === 'fields'}
            data-tip="哪一格从什么值改成什么值，可就地撤销 / 重做"
            onClick={() => setView('fields')}
          >
            改了什么
          </button>
        </div>
      }
      /*
       * footer 左边那句走 `footLead`（不许当 footer 的第一个子元素 ——
       * 那一条是右对齐 flex，说明会被挤成竖窄条，2026-11-02 的教训）。
       *
       * 「放弃」在这里用普通 `.btn`、「保存」才上主色：与页头同一条口径
       * （能保存才上色，不能保存是灰的、悬停说清为什么）。
       */
      footLead={
        onSave !== undefined || onDiscard !== undefined ? (
          '放弃 = 影子树整个拉回盘上那份，这一批改动一件都不留；保存 = 写进仓库文件（保存本身也能撤销）'
        ) : undefined
      }
      footer={
        onSave === undefined && onDiscard === undefined ? undefined : (
          <>
            {onDiscard !== undefined && (
              <button
                type="button"
                className={s.btn}
                disabled={!canDiscard || busy}
                data-tip={canDiscard ? '把影子树拉回盘上那份 —— 这批改动一件都不留' : '没有未保存的改动'}
                onClick={onDiscard}
              >
                放弃改动
              </button>
            )}
            {onSave !== undefined && (
              <button
                type="button"
                className={`${s.btn} ${canSave ? s.btnPrimary : ''}`}
                disabled={!canSave || busy}
                data-tip={canSave ? '把这几处写进仓库文件（保存本身也能撤销）' : '没有未保存的改动'}
                onClick={onSave}
              >
                {busy ? '保存中…' : '保存'}
              </button>
            )}
          </>
        )
      }
    >
      {diffErr !== null ? (
        <p className={d.empty} role="alert">
          读不出未保存改动：{diffErr}
        </p>
      ) : diff === null ? (
        <p className={d.empty}>正在比对…</p>
      ) : noFiles ? (
        /* 空态：不分栏。左边一栏空清单 + 右边一句"没有改动"没有意义。
        但**空 diff 不等于干净** —— 还有 N 处就得说还有 N 处，不许报"一模一样"。 */
        emptyButDirty ? (
          <p className={d.empty}>
            {`文件层面没有差异 —— 还有 ${dirtyCount} 处改动只活在内存里（菜单 / 套餐 / 可见性写在 workbench 自己的文件里），保存会一并写入。`}
          </p>
        ) : (
          <p className={d.empty}>影子与盘上一模一样 —— 没有未保存的改动。</p>
        )
      ) : (
        <div className={d.panes}>
          {/* 左：有改动的文件清单（两种视图共用；点进去 = 右边只谈这一份） */}
          <nav className={d.list} aria-label="有未保存改动的文件">
            {files.map(fileItem)}
          </nav>

          {/* 右：按视图换内容 —— 代码 = 行级 diff，改了什么 = 那几格 + 就地撤销 */}
          <div className={d.detail}>
            <header className={d.detailHead}>
              <span className={d.detailName}>
                {current === null ? '—' : view === 'code' ? current.rel : (current.rel ?? '')}
              </span>
              <span className={d.detailMeta}>
                {view === 'code' && current !== null ? (
                  <>
                    <span className={d[`state_${current.state}`]}>
                      {STATE_TEXT[current.state]}
                    </span>
                    {!current.binary && (
                      <span className={d.counts}>
                        <span className={d.add}>+{current.added}</span>
                        {current.removed > 0 && <span className={d.del}>−{current.removed}</span>}
                      </span>
                    )}
                  </>
                ) : (
                  /* **与页头「未保存 N」同一个数**（后端同一个 `Txn::dirty_count`） */
                  <span className={d.detailCount}>{shownLines.length} 处</span>
                )}
              </span>
            </header>

            {view === 'code' ? (
              <div className={d.detailScroll}>
                {current === null ? (
                  <p className={d.empty}>左边点一份看它的差异</p>
                ) : current.binary ? (
                  <p className={d.empty}>
                    {current.state === 'added'
                      ? '保存会新增这一份（二进制，比不出行）。'
                      : current.state === 'removed'
                        ? '保存会删掉这一份（二进制，比不出行）。'
                        : '这一份被换成另一张图了 —— 二进制文件，只报"换了"，比不出行。'}
                  </p>
                ) : (
                  /* 与**生成前确认**同一个渲染器：上红（删）下绿（增）两行。
                     作者 2026-11-02 定案 —— 折成一行的「旧 → 新」在长行上读不动。 */
                  <DiffLines
                    lines={current.lines}
                    mode={current.state === 'added' ? 'added' : 'modified'}
                    view="diff"
                    fileName={current.rel}
                    innerRef={diffRef}
                  />
                )}
              </div>
            ) : err !== null ? (
              <p className={d.empty} role="alert">
                {err}
              </p>
            ) : lines === null ? (
              <p className={d.empty}>正在读改动…</p>
            ) : shownLines.length === 0 ? (
              /* 2026-11-03 起这一页**不会再"无缘无故空"**：数据来自事务里的字段账，
              真改了东西就一定有条目。剩下的空格只有两种情况，分开说清楚 —— */
              <p className={d.empty}>
                {focusKeys === null
                  ? '这一笔改动还没进事务 —— 改一格参数、或改一次说明，这里就会列出来。'
                  : '这一份文件这一次没有字段级改动（只动了注释 / 空行那类），看「代码对比」的逐行差异。'}
              </p>
            ) : (
              <div className={d.fields}>
                {/*
                 * **三列表格**（2026-11-03 作者定稿：平铺 + 标签）。
                 *
                 * 为什么不像以前那样"一条一句人话"：一次改 `A1` 的 Z 会**归并连带**
                 * 写出 `A1:STANDARD` / `A1:FAST` / `A1:FASTV3.3` 三格 —— 四条都要
                 * 平铺出来（作者：「都显示出来噢」，页头那个数就是这四条的条数）。
                 * 表格让人一眼扫出"哪几处、各自什么来源"。
                 */}
                <div className={d.fieldHead}>
                  <span>归属</span>
                  <span>字段变化</span>
                  <span>来源</span>
                </div>
                {shownLines.map((l, i) => {
                  const mark = `${l.owner}:${l.key ?? ''}`
                  const whyLabel = WHY_LABEL[l.why] ?? l.why
                  /* 撤销按钮挂在**这一格**上，撤的是栈顶那一条编辑。
                     2026-11-02 起同一格 5 秒内的连点并成一条（`txn::MERGE_MS`），
                     所以按一下回到的是"我开始动手之前"，不是退一小步 —— 那正是
                     作者要的粒度。重做按钮放在底部：重做按栈的顺序来，不该逐行给。 */
                  return (
                    <div key={`${mark}-${i}`} className={d.field}>
                      {/* ① 归属：等宽的 `A1:STANDARD` 才好扫；基底与版本分色 */}
                      <span
                        className={`${d.fieldOwner} ${l.owner.includes(':') ? d.fieldOwnerVer : ''}`}
                        data-tip={l.target}
                      >
                        {l.owner}
                      </span>
                      {/* ② 字段变化：名字 + `旧 → 新`，值太长时截断、悬停看全文 */}
                      <span className={d.fieldWhat}>
                        <span className={d.fieldName}>{l.label}</span>
                        <span className={d.fieldFrom} title={l.before}>
                          {l.before === '' ? '（空）' : l.before}
                        </span>
                        <span className={d.fieldArrow} aria-hidden>
                          →
                        </span>
                        <span className={d.fieldTo} title={l.after}>
                          {l.after === '' ? '（空）' : l.after}
                        </span>
                      </span>
                      {/* ③ 来源：用户自己点的用主色描边，连带/复制用中性底 */}
                      <span className={d.fieldWhyCol}>
                        <span
                          className={`${d.why} ${l.why === 'user' ? d.whyUser : ''} ${
                            l.why === 'unknown' ? d.whyWarn : ''
                          }`}
                        >
                          {whyLabel}
                        </span>
                        {onUndo !== undefined && (
                          <button
                            type="button"
                            className={d.fieldUndo}
                            disabled={!canUndo || pendingKey !== null}
                            data-tip={
                              canUndo
                                ? '撤掉这一格所在的编辑（同一格 5 秒内的连点算一条，撤一次回到最初）'
                                : '没有可撤销的改动'
                            }
                            onClick={() => void run('undo', mark)}
                          >
                            {pendingKey === mark ? '撤销中…' : '撤销'}
                          </button>
                        )}
                      </span>
                    </div>
                  )
                })}
                {onRedo !== undefined && (
                  <div className={d.fieldFoot}>
                    <span className={d.fieldHint}>
                      撤销走的是外壳那条栈（与状态栏的「撤销」同一个）。同一格 5 秒内的连点并成一条 ——
                      撤一次回到那一格最初的值。重做按栈的顺序来，不按这一行的位置。
                    </span>
                    <button
                      type="button"
                      className={s.btn}
                      disabled={!canRedo || pendingKey !== null}
                      data-tip={canRedo ? '重做上一条撤销' : '没有可重做的改动'}
                      onClick={() => void run('redo', '__redo')}
                    >
                      重做
                    </button>
                  </div>
                )}
              </div>
            )}
            {view === 'code' && showRuler && <DiffRuler marks={marks} onJump={jumpToRow} />}
          </div>
        </div>
      )}
    </ModalC14>
  )
}
