/*
 * 分组抽屉（2026-11-01）。
 *
 * 作者：「我不要点击的改名，我要点击的打开右侧的抽屉……上下这些因为如果标题就能
 * 上下的话，很容易不小心点到的就是排序。我希望都在抽屉里面，而且抽屉里面可以
 * 展示更多的信息，把所有能展示的信息都展示出来。」
 *
 * 所以卡片头只剩「点它开抽屉」一件事：改名、上下移、看这一组里都有什么参数，
 * 全在这里。抽屉里**不直接落盘** —— 每一次改动都进草稿（服务器侧那条撤销栈认得出），
 * 真正写盘仍然只归右上角那颗「保存」，点了「放弃」才是真的放弃。
 *
 * # 它住在页面里，**不再 portal 到 `FieldLayer`**（2026-11-02 重做）
 *
 * 抽屉原来是 portal 到 `FieldLayer`（`.layer`，z 90）里的 —— 结果"抽屉开着时弹出的
 * 对话框压不住它"这个问题，修了四次都没修好，因为**根子不在数字上**：
 *
 *   `FieldLayer` 的那一层里，抽屉是一个子元素；`ModalC14` 默认挂 `.shellBody`（z 40
 *   那棵树），`FieldLayer` 整体 90 —— 抽屉永远赢。改挂进 `FieldLayer` 之后，
 *   对话框又成了"另一个子的后代"，同一个层叠上下文里后代抬不过兄弟，抽屉的 20
 *   照样赢。再往后加 z、加第二道容器、加点击闸，都是在跟这条规则较劲。
 *
 * 所以把它搬回**页面自己**：`.drawerHost` 是页面根（`.pMain`）里的一层，
 * `.drawer` 绝对定位在其中 —— 贴着窗口右缘通高，照样不跟着卡阵滚（`.pMain` 自己不滚，
 * 滚的是里面那两块）。而对话框回到标准的 `.shellBody` 宿主，两者**同父同层**，
 * 谁也不用比 z：对话框在 `.shellBody` 里，抽屉在 `.main` 里，后画的这一整层在上面。
 *
 * ★ 这条是**结构上的一刀**，不是又一次调数：抽屉以后不许再回到 `FieldLayer`。
 */
import { useEffect, useMemo, useRef, useState } from 'react'

import type { RegistryView, SectionMetaEdit, SectionTarget } from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import SectionList from './SectionList'
import type { SectionGroup } from './SectionList'
import s from './SectionDrawer.module.css'

interface Props {
  /**
   * 抽屉里**显示**哪一组的内容（在抽屉里换看别组时它就跟着换）。
   *
   * ★ 它**不是**「当前」：当前住 `markId`。作者 2026-11-01：「我的里面没有这个了
   * 但是侧边抽屉还有这个，是谁错了」—— 一屏出现两个"当前"，就是因为这两个概念
   * 曾经共用一个变量（见 `ParamsPage` 里 `drawerAnchor` 的注释）。
   */
  sectionId: string | null
  /**
   * 页面此刻认的「当前」是哪一组（标题与那颗「当前」标记跟它走）。
   *
   * 不传 = 与 `sectionId` 同一个（旧调用处不必改）。注册表查不到的组（未落盘的新组、
   * 组件卡）也照样信 —— **不许退回"第一个组"当当前**，那正是截图里那处错的来源。
   */
  markId?: string | null
  registry: RegistryView | null
  /** 这一组里的参数（卡片头那颗计数点开看的就是它们） */
  rows: { key: string; label: string }[]
  onClose: () => void
  /** 改名字 / 换位：都只是应用（进草稿），不落盘 */
  onApply: (id: string, edit: SectionMetaEdit) => void
  /** 点某条参数 = 选中它（页面上那一行随之高亮） */
  onPickParam: (key: string) => void
  /** 点某条参数之后把页面滚到那一行（给了才滚） */
  jumpToParam?: (key: string) => void
  /**
   * 在当前页签下新建一个分组（给了才画那颗「＋ 新增分组」）。
   * 只进草稿；建完宿主会把抽屉切到新组上。
   */
  onAddSection?: (tabId: string, label: string) => void

  /** 页面这一版一共多少条参数（表头那个数），与 `pageDrawn` 一起给「页面上有没有它」下判据 */
  pageTotal?: number
  /** 页面这一版真画出来多少行（收起 / 被条件关掉的都不算） */
  pageDrawn?: number
  /**
   * 删掉一个空分组（后端再挡三道门：还有参数 / 是组件卡 / 带条件）。
   * 与改名一样**只是进草稿** —— 右上角「保存」才落盘。不给就不画那颗按钮。
   */
  onRemove?: (id: string) => void
  /** 在抽屉里换一组看（页面上那张卡跟着亮、跟着滚过去） */
  onShowSection: (id: string) => void
  /**
   * 注册表里查不到这一组时的兜底标题（用户点的那张卡自己写着名字）。
   * 2026-11-01 踩过：组件卡的卡头点了什么都不发生 —— 因为这里 `here === null`
   * 直接把整个抽屉 `return null` 了，看着像"点不开"。
   */
  fallbackLabel?: string | null
}

export default function SectionDrawer({
  sectionId,
  markId,
  registry,
  rows,
  onClose,
  onApply,
  onPickParam,
  jumpToParam = () => undefined,
  onAddSection,
  pageTotal,
  pageDrawn,
  onRemove,
  onShowSection,
  fallbackLabel,
}: Props) {
  /*
   * 标题那支笔（作者 2026-11-01：「要加也可以，顺手的事情」）—— 与每行的笔同一个
   * 语义：点笔才改名，点标题本身不动作。
   */
  const [renaming, setRenaming] = useState(false)
  const [titleDraft, setTitleDraft] = useState('')

  const groups: SectionGroup[] = useMemo(() => {
    const out: SectionGroup[] = []
    for (const sec of registry?.sections ?? []) {
      /* 整块卡（设置页那些）不列 —— 客户端固定长好的，这里改不了也不该改 */
      if (sec.standalone) continue
      const hit = out.find((g) => g.tabId === sec.tabId)
      if (hit === undefined) {
        out.push({ tabId: sec.tabId, tabLabel: sec.tabLabel, sections: [sec] })
      } else {
        hit.sections.push(sec)
      }
    }
    /* 一个组都不剩的页签不画（`settings` 那种整页都是固定卡的，滤完就是空的） */
    return out.filter((g) => g.sections.length > 0)
  }, [registry])

  /*
   * 标题那一头跟着**当前**（`markId`），不是跟着"正在看的内容"：当前是页面主体，
   * 「第 N / 共 M 组」说的是它在自己页签里的位置。内容那头只决定下面列什么。
   */
  /*
   * ★ 「当前这一组刚被删掉」的跟位（2026-11-02 作者：「删除之后的刷新，不要关闭我的抽屉」）。
   *
   * 抽屉不关，但正对着的 `id` 已经不在注册表里了 —— 如果什么都不做，它会画成
   * 「注册表里没有这一组（可能是整块组件卡）」+ 空参数表，看着像坏掉了。
   *
   * 所以：记下**上一次还认得的那一组**，找不到时退到它（它此刻就是"删掉的那个
   * 同一位置上的下一组"，因为删除只少一个）。退位只影响标题与「第 N / 共 M 组」，
   * 抽屉内容（`rows`）由宿主给，宿主那边同样按新数据重算了，读到的就是新的那一组。
   */
  const deletedFallbackRef = useRef<string | null>(null)
  const mark = markId ?? sectionId
  const hereRaw = (registry?.sections ?? []).find((x) => x.id === mark) ?? null
  const gone = mark !== null && hereRaw === null
  const here =
    hereRaw ??
    (gone && deletedFallbackRef.current !== null
      ? ((registry?.sections ?? []).find((x) => x.id === deletedFallbackRef.current) ?? null)
      : null)
  /* 只在"查得到"的时候记 —— 记的是最后一次还认得的那个 */
  useEffect(() => {
    if (hereRaw !== null) deletedFallbackRef.current = hereRaw.id
  }, [hereRaw])
  /* 真的一个都不剩了（整页删空）才当成"没有这一组" */
  useEffect(() => {
    if (gone && here === null) deletedFallbackRef.current = null
  }, [gone, here])
  /* 这一组在自己页签里的位置：抽屉里也把「第几 / 共几」摊开（作者要的"更多信息"） */
  const sibs: SectionTarget[] = here === null
    ? []
    : (registry?.sections ?? []).filter((x) => x.tabId === here.tabId)
  const at = here === null ? -1 : sibs.findIndex((x) => x.id === here.id)

  /*
   * 「这一组现在能不能删」。三个条件与后端 `remove_section` 那三道门一一对应 ——
   * 这里只负责**要不要把按钮画出来**，拒的理由仍由后端给（它读到的是同一份数据）。
   *
   * 没参数才叫空：`rows` 是这一组的参数清单（页面派生来的），空 + 注册表里查得到
   * （组件卡在注册表里查不到，天然被 `here !== null` 挡掉）。
   */
  const canRemove = here !== null && rows.length === 0

  /*
   * 删除**先问一句**（作者 2026-11-02：「我要模态框」）。
   *
   * 为什么不点就删：删分组是这一屏里唯一一件"整块东西没了"的动作，和改个名字
   * 不是一回事。模态框里把两件事说清 —— **删的是什么**（名字 + id + 在哪个页签第几组）、
   * **删完算不算数**（只是应用进草稿，右上角「保存」才落盘，「放弃」就回来）。
   * 这与全仓口径一致：模态框的确认键永远不叫「保存」。
   */
  const [asking, setAsking] = useState(false)
  useEffect(() => {
    /* 换了一组 / 抽屉关了 ⇒ 那句问话作废（它问的是上一条） */
    setAsking(false)
  }, [mark, sectionId])

  /* Esc 关抽屉 —— 与其余浮层一致 */
  useEffect(() => {
    if (sectionId === null) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [sectionId, onClose])

  /* 查不到也画：宁可用卡片自己的名字兜底，也不要点下去没反应 */
  if (sectionId === null) return null
  /* `mark` 到这里一定非空（`drawerAnchor` 与 `sectionId` 同一个来源；兜底只为类型收窄） */
  const title: string = here?.label ?? fallbackLabel ?? mark ?? sectionId

  return (
    <>
      {/*
        2026-11-01 作者：「我希望我这个滚动可以穿透，就是说右侧这个抽屉在的时候，
        我在左侧这个主页面滚动的时候，能滚动到背后的东西」。
        所以**不要遮罩** —— 一层半透明背景会把滚轮全吃掉（滚动到不了背后的列表），
        而且那层"点它关掉"的预期也和"能继续操作左边"冲突。关抽屉只有 × 与 Esc。
      */}
      <aside className={s.drawer} role="dialog" aria-label={`分组 ${title}`}>
        <header className={s.head}>
          <div className={s.titleRow}>
            {renaming ? (
              <input
                className={s.titleInput}
                value={titleDraft}
                autoFocus
                maxLength={24}
                aria-label="分组名"
                onChange={(e) => setTitleDraft(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    const next = titleDraft.trim()
                    setRenaming(false)
                    if (next !== '' && next !== title && mark !== null) {
                      onApply(mark, { label: next, move: null })
                    }
                  } else if (e.key === 'Escape') {
                    setRenaming(false)
                  }
                }}
                onBlur={() => setRenaming(false)}
              />
            ) : (
              <>
                <h3 className={s.title}>{title}</h3>
                <button
                  type="button"
                  className={s.titlePen}
                  title="改这一组的名字（回车确定 · Esc 撤回）"
                  aria-label="改分组名"
                  onClick={() => {
                    setTitleDraft(title)
                    setRenaming(true)
                  }}
                >
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                    <path d="M12 20h9" />
                    <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" />
                  </svg>
                </button>
              </>
            )}
            {/*
               * 「删除这一组」—— 只在**空组**上出现（作者 2026-11-01：「空的时候可以删除，
               * 比如说我建了一个新的，不小心建多了也可以删除」）。
               *
               * 这里只做"能不能删"的显示判断，真判据在后端 `remove_section`
               * （还有参数 / 是组件卡 / 带 showWhen 一律拒），两边不各立一套：
               * 后端拒了就把话原样报出来，界面不会自己猜。
               *
               * 删完也是**进草稿**，右上角「保存」才落盘 —— 与改名、换位、改参数值同一套。
               */}
            {canRemove && onRemove !== undefined ? (
              <button
                type="button"
                className={s.trash}
                title="删掉这个空分组（进草稿；右上角「保存」才落盘）"
                aria-label="删除分组"
                onClick={() => setAsking(true)}
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
                  <path d="M3 6h18" />
                  <path d="M8 6V4h8v2" />
                  <path d="M6 6l1 14h10l1-14" />
                </svg>
              </button>
            ) : null}
            <button type="button" className={s.close} aria-label="关闭" onClick={onClose}>
              ×
            </button>
          </div>
          <p className={s.sub}>
            {here === null
              ? '注册表里没有这一组（可能是整块组件卡）'
              : `${here.tabLabel} · 第 ${at + 1} / ${sibs.length} 组 · ${here.id}`}
          </p>
        </header>

        <div className={s.body}>
          <section className={s.block}>
            <h4 className={s.blockTitle}>这一组里的参数</h4>
            {rows.length === 0 ? (
              /*
               * 「空组」要说清是哪一种空 —— 作者 2026-11-01：「在主要页面看不到了，
               * 但是抽屉能看到，所以他没有真的删掉？客户端到时候也消费了怎么办，
               * 会出现一个空的」。三种情况差得很远，含糊一句「没有参数」等于没说：
               *
               *   · 这一组压根没参数（注册表里没人认领）⇒ 客户端不会画这张卡；
               *   · 有参数，但这一版画出来的行数是 0 ⇒ 整页的卡都被收起/条件关着；
               *   · 有参数、页面上也在画，只是**这一份派生**没带回来（点了别的领域 /
               *     还在加载）⇒ 切到「全部」就能看见。
               */
              <p className={s.empty}>
                {pageTotal === 0
                  ? '这一页还没有读到参数（正在加载，或者还没选机型与版本）。'
                  : pageDrawn === 0
                    ? `这一页现在一行都没画出来：${pageTotal} 条参数这会儿都被收起或被条件关着。`
                    : '这一组眼下没有参数 —— 它在注册表里没有认领的条目（客户端也不会画它）。'}
              </p>
            ) : (
              <ul className={s.rows}>
                {rows.map((r) => (
                  <li key={r.key}>
                    <button
                      type="button"
                      className={s.rowBtn}
                      onClick={() => {
                        onPickParam(r.key)
                        /* 点参数 = 选中它，并且**带人过去**：右栏那一条可能在滚动区外，
                           光高亮等于没反馈（与「抽屉里点分组」同一个期望）。 */
                        jumpToParam(r.key)
                      }}
                    >
                      <span className={s.rowLabel}>{r.label}</span>
                      <span className={s.rowKey}>{r.key}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section className={s.block}>
            <h4 className={s.blockTitle}>改名与位置</h4>
            <p className={s.note}>
              改一条应用一条，还没有落盘 —— 右上角「保存」才写进 param_registry.toml
              与 layout_schema.toml；那之前点「放弃」就全都不算。
            </p>
            <SectionList
              groups={groups}
              /*
               * 那颗「当前」跟 `markId`，**不跟 `sectionId`** —— 否则在抽屉里换看
               * 另一组时，圆点就跑到了别处（作者报的就是这个：页面上没有这一组，
               * 抽屉里却把它点亮了）。
               */
              value={here?.id ?? mark}
              /* 点圆点 = 换一组看（等于在左树里点它）；点名字仍然是就地改名 */
              selectable
              onPick={onShowSection}
              onRename={(id, label) => onApply(id, { label, move: null })}
              onMove={(id, dir) => onApply(id, { label: null, move: dir < 0 ? 'up' : 'down' })}
              /* 「＋ 新增分组」：只进草稿，建完切到新组上（见 `ParamsPage.addSection`） */
              onAdd={onAddSection}
              hint="位置只在自己的页签里挪：客户端按这一页的顺序排，卡片也跟着它。"
            />
          </section>
        </div>
      </aside>

      {/*
        删除前那句问话（2026-11-02 作者：「我要模态框」）。

        # 挂哪一层：交给 `ModalC14` 的默认宿主（`.shellBody`）—— **这里不传 `host`**

        抽屉现在住页面自己（见文件头那一节），对话框住 `.shellBody`：后者整层在
        `.main` 之后画，所以压得住抽屉，**一个 z 都不用比**。
        之前那四次调数、两道容器、点击闸，都是因为抽屉挂在 `FieldLayer`（z 90）里 ——
        换回页面里之后，这些理由全都不成立了，所以那些补丁也一并撤掉。
      */}
      <ModalC14
        open={asking && canRemove && onRemove !== undefined}
        title="删除这个分组？"
        subtitle="只删这一条分组声明；它名下没有任何参数（有参数的组这里删不了）。"
        size="sm"
        onClose={() => setAsking(false)}
        /*
         * 说明那句用 `footLead`：footer 是右对齐单行 flex，直接塞进去会被挤成
         * 一条竖窄条（作者截图里「这一笔也只进 / 草稿 —— 右上 / 角「保存」才 /
         * 落盘」四行高、按钮还斜着排）。`footLead` 自己吃掉左侧余量。
         */
        footLead="这一笔也只进草稿 —— 右上角「保存」才落盘，「放弃」就退回来"
        footer={
          <>
            <button
              type="button"
              className={`${c.btn} ${c.btnSm}`}
              onClick={() => setAsking(false)}
            >
              取消
            </button>
            <button
              type="button"
              className={`${c.btn} ${c.btnSm} ${c.btnDanger}`}
              onClick={() => {
                setAsking(false)
                /* `open` 那一行已经判过它不是 undefined；这里再收一次只为类型 */
                if (onRemove !== undefined) onRemove(mark ?? sectionId)
              }}
            >
              删除分组
            </button>
          </>
        }
      >
        <dl className={s.confirmList}>
          <dt>名字</dt>
          <dd>{title}</dd>
          <dt>分组 id</dt>
          <dd className={s.mono}>{mark ?? sectionId}</dd>
          <dt>位置</dt>
          <dd>{here === null ? '（注册表里查不到）' : `${here.tabLabel} · 第 ${at + 1} / ${sibs.length} 组`}</dd>
          <dt>参数</dt>
          <dd>0 条（空的才删得掉）</dd>
        </dl>
      </ModalC14>
    </>
  )
}
