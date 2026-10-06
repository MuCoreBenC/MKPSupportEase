/**
 * 「机型与版本」页 —— C14 版式移植（feat/b05-14b-c14-port）。
 *
 * 版式与交互来自试验场 C14（第 1～26 轮的验收结论都在）：左列筛选 + 机型行、
 * 右侧「身份 / 尺寸 / 版本 / 版本详情」四张卡、版本行右键菜单、
 * 详情卡按「① 身份 ② 参数 ③ 配方 ④ 关联 + 下一步」分段。
 *
 * # 与原型的差别 —— 每一条都是真后端决定的（原型自己也在副标题里承认了）
 *
 *  - **删除是两步问孤儿、确认后立刻落盘**：产品模型里删除不可逆、没有草稿
 *    （`wb_remove_version` 的注释写明）。原型的「Ctrl+Z 能整块退回来」在这里不成立，
 *    文案照实说。
 *  - **复制版本走两条真命令**：`wb_copy_version` 只写版本定义；「同时复制配方」
 *    才补一刀 `wb_copy_recipe`（14.3/14.5，拷出来是独立快照）。
 *  - **这些原型动作没有接**（后端没有对应命令，前端不装样子，见 C14-PORT-PLAN §4
 *    的待裁决清单）：改机型/版本 id、复制机型、删除机型、尺寸九格编辑、
 *    版本图（原型本地字段）、改配方文件名（G-2 要删的字段）、就地新建套餐、
 *    改套餐内容（套餐内容的产品模型还在迁移中，随 P4 的 menu/stock 视角一起接）。
 *    对应的按钮与菜单项**不渲染** —— 摆一个点不动的按钮比没有更糟。
 *  - **生成**走 `wb_generate({ picked: [uid] })`，产物名由命名函数算（G-0），
 *    没有「配方文件缺失」这一档；闸门读 `wb_preflight` 的阻断与 `buildRows` 的
 *    buildable / disabledReason —— 前端不复判任何一条规则。
 *  - **机型图 / 图标是资产 id**（Task 8.6），挑选走资产库选择器（14.6：
 *    不让人手填路径）。
 */
import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { isAppError, wb } from '../api'
import type {
  AssetList,
  BookView,
  BundleList,
  IssueReport,
  MachineList,
  MachineView,
  RegistryView,
  VersionView,
  Words,
} from '../api'
import { ContextMenu } from '../components/menu'
import type { ContextMenuEntry } from '../components/menu/types'
import { useContextMenu } from '../components/menu/useContextMenu'
import AssetPicker, { AssetField } from '../c14/AssetPicker'
import FieldMark, { TodoValue } from '../c14/FieldMark'
import ModalC14 from '../c14/ModalC14'
import PickOrType from '../c14/PickOrType'
import { fieldState, machineFieldLabels, placeholderText } from '../c14/labels'
import { toasts } from '../c14/toast'
import { useSplitRail } from '../c14/SplitRail'
import MachineDimensionsModal from './MachineDimensionsModal'
import ZoneEditorModal from './ZoneEditorModal'
import type { GotoFocus } from '../c14/types'
import s from '../c14.module.css'

interface Props {
  book: BookView
  words: Words
  onGoto: (view: string, focus?: GotoFocus) => void
  /** 外壳的保存。生成按钮在草稿脏时给的「先落盘再生成」就是它 */
  onSave: () => Promise<boolean>
  /** 结构性写（建 / 删 / 复制）之后让外壳重取整本 —— 徽章的机型版本数跟着走 */
  onBookRefresh: () => void
}

/** 版本行选中标记：`机型id/版本id`。只在本页内当 key 用，不是后端的 uid */
const vidOf = (machineId: string, versionId: string) => `${machineId}/${versionId}`

/** 机型 / 版本 id 的即时格式提示（真闸在后端；这里只让明显打错的当场现形） */
const idOk = (v: string) => /^[A-Z][A-Z0-9_]*$/.test(v)

/**
 * 后端记录的是 **UTC ISO 串**（`2026-10-02T16:30:09Z`，刻意跨时区一致），
 * 给人看要转**本机时区**（作者 2026-10-03：「应该用东八区的时间，或者电脑的时区」）。
 * 解析不动就原样回（老记录可能不是 ISO）。
 */
const localStamp = (iso: string): string => {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

/** 生成态四档的样式（C14 的 tag 类名表） */
const STATE_TAG: Record<string, string> = {
  built: s.tagBuildBuilt,
  stale: s.tagBuildStale,
  neverBuilt: s.tagBuildNever,
  noResources: s.tagBuildNone,
}

export default function MachinesPage({ book, words, onGoto, onSave, onBookRefresh }: Props) {
  /** 左栏宽度可拖（作者 2026-10-03，同参数台） */
  const bodyRef = useRef<HTMLDivElement | null>(null)
  const rail = useSplitRail('machines', bodyRef)
  const [list, setList] = useState<MachineList | null>(null)
  const [assets, setAssets] = useState<AssetList | null>(null)
  const [bundles, setBundles] = useState<BundleList | null>(null)
  const [registry, setRegistry] = useState<RegistryView | null>(null)
  const [report, setReport] = useState<IssueReport | null>(null)
  const [pageErr, setPageErr] = useState<string | null>(null)

  const [machineId, setMachineId] = useState('')
  const [pickedVid, setPickedVid] = useState<string | null>(null)
  const [filter, setFilter] = useState('')
  /** 尺寸卡的明细（移动范围 / 涂胶 / 标定点 / 标志位）：默认收起，常看的只有床身和禁区 */
  const [dimsAll, setDimsAll] = useState(false)
  const menu = useContextMenu<string>()

  /* —— 弹窗 —— */
  const [addOpen, setAddOpen] = useState(false)
  const [addId, setAddId] = useState('')
  const [addBrand, setAddBrand] = useState('')
  const [addDisplay, setAddDisplay] = useState('')
  const [addVOpen, setAddVOpen] = useState(false)
  const [avId, setAvId] = useState('')
  const [avName, setAvName] = useState('')
  const [copyOpen, setCopyOpen] = useState<string | null>(null)
  const [copyId, setCopyId] = useState('')
  const [copyName, setCopyName] = useState('')
  const [withRecipe, setWithRecipe] = useState(true)
  /** 删除版本的两步：`null` = 没开；`{vid}` = 第一步；带 orphans = 第二步 */
  const [del, setDel] = useState<{ vid: string; orphans: string[] | null } | null>(null)
  const [bunOpen, setBunOpen] = useState<string | null>(null)
  const [bunPick, setBunPick] = useState('')
  const [imgOpen, setImgOpen] = useState(false)
  const [iconOpen, setIconOpen] = useState(false)
  /** 版本专属外观图（第三刀：图位分层 品牌 → 机型 → 版本，版本缺则回落机型） */
  const [verImgOpen, setVerImgOpen] = useState(false)
  const [editVOpen, setEditVOpen] = useState<string | null>(null)
  const [evName, setEvName] = useState('')
  const [evTag, setEvTag] = useState('')
  const [evDesc, setEvDesc] = useState('')

  /* —— 品牌（2026-10-03，作者：「在这一页多加一个品牌吧，右侧也是一样可以编辑，
        品牌图、显示名，不管客户端消不消费都提供」）——
     左列选中的是品牌还是机型：`brandId` 非空 = 在看那个品牌。 */
  const [brandId, setBrandId] = useState<string | null>(null)
  const [addBrandOpen, setAddBrandOpen] = useState(false)
  const [nbId, setNbId] = useState('')
  const [nbName, setNbName] = useState('')
  const [logoOpen, setLogoOpen] = useState(false)
  /**
   * 树里**收起来**的品牌 id（默认空 = 全部展开，作者 2026-10-03 选的形态）。
   * 收起来的状态只活在这一次会话里 —— 不落盘、不 localStorage：
   * 它是一次「我现在想少看几台」的临时动作，不是一个要跨会话记住的偏好。
   */
  const [foldedBrands, setFoldedBrands] = useState<Set<string>>(() => new Set())
  /** 移机型：`null` = 没在选；非空 = 正在给这台机器选新家 */
  const [moveOf, setMoveOf] = useState<string | null>(null)
  /** 尺寸模态框开着哪台（`null` = 关着） */
  const [dimsOpen, setDimsOpen] = useState<string | null>(null)
  /** 禁区编辑器开着哪台 */
  const [zoneOpen, setZoneOpen] = useState<string | null>(null)

  /* —— 取数 —— */
  const loadAll = useCallback(() => {
    void (async () => {
      try {
        const [l, a, b, r] = await Promise.all([
          wb.machines(),
          wb.assets(null, null, null, null, null, null),
          wb.bundles(null),
          wb.registry(),
        ])
        setList(l)
        setAssets(a)
        setBundles(b)
        setRegistry(r)
      } catch (e) {
        setPageErr(isAppError(e) ? e.message : String(e))
      }
    })()
  }, [])
  useEffect(loadAll, [loadAll])

  /** 每次写完都重取检查报告 —— 阻断集合变了，生成按钮的门禁跟着走 */
  const refreshReport = useCallback(() => {
    wb.preflight()
      .then(setReport)
      .catch(() => undefined)
  }, [])

  /** 结构性写之后：外壳的整本（徽章、构建行）也要跟上 */
  const afterStructural = useCallback(() => {
    onBookRefresh()
    refreshReport()
  }, [onBookRefresh, refreshReport])

  const m: MachineView | undefined = list?.machines.find((x) => x.id === machineId)

  /* 选中态自动跟上：机型被删（外部）后落回第一台，版本卡不挂在旧 id 上 */
  useEffect(() => {
    if (list && !list.machines.some((x) => x.id === machineId)) {
      setMachineId(list.machines[0]?.id ?? '')
      setPickedVid(null)
    }
  }, [list, machineId])
  useEffect(() => {
    if (pickedVid === null || !m) return
    const [, pv] = pickedVid.split('/')
    if (!m.versions.some((v) => v.id === pv)) setPickedVid(null)
  }, [pickedVid, m])

  /**
   * 机型行在右键菜单里的键。**带 `machine:` 前缀** —— 版本行用的是没前缀的
   * `机型/版本`，两者混在一个 `menu.target` 里必须能分开。
   */
  const machineKey = (id: string) => `machine:${id}`

  /** 版本 id → 整本里的节点（uid / 生成态 / 覆盖数 / bbs 都在那边算好了） */
  const nodeOf = useCallback(
    (mid: string, vid: string) =>
      book.machines.find((x) => x.id === mid)?.versions.find((v) => v.versionId === vid),
    [book],
  )
  const rowOf = useCallback(
    (uid: string) => book.buildRows.find((r) => r.uid === uid),
    [book],
  )

  const q = filter.trim().toLowerCase()
  const listed = useMemo(() => {
    const all = list?.machines ?? []
    return q
      ? all.filter(
          (x) =>
            x.id.toLowerCase().includes(q) ||
            x.display.toLowerCase().includes(q) ||
            x.brand.toLowerCase().includes(q),
        )
      : all
  }, [list, q])

  /**
   * **树的分组依据 = 机型自己的 `brand`**（不是后端 `BrandView.machines` 那张反查表）。
   *
   * 两个来源不一致时（手改了文件、后端没重读），以机型那一格为准渲染层级 ——
   * 否则会画出「品牌说有 A1、机型说自己归别家」这种自相矛盾的树。
   * 反查表只用来数台数之外的用途（品牌卡那张 chips）。
   *
   * 匹配**大小写不敏感**（与后端 `list_of` 的口径一致：真数据里品牌 id 叫 `Bambu Lab`）。
   */
  const tree = useMemo(() => {
    const brands = list?.brands ?? []
    const machines = listed
    return brands
      .map((b) => ({
        brand: b,
        machines: machines.filter((x) => x.brand.trim().toLowerCase() === b.id.trim().toLowerCase()),
      }))
      // 筛选时：品牌自己命中 → 整组留下；否则只留命中机型的那些组
      .filter((g) => {
        if (!q) return true
        const brandHit =
          g.brand.id.toLowerCase().includes(q) || g.brand.name.toLowerCase().includes(q)
        return brandHit || g.machines.length > 0
      })
  }, [list, listed, q])

  /** 必填却空着的身份格数（display / brand）。空着标「需填写」，不是错误 */
  const todoOf = (x: MachineView) =>
    [x.display, x.brand].filter((v) => !v.trim()).length

  /** 改机型的一格。后端校验；失败的话把那句话原样端给人 */
  const saveMachine = async (field: 'display' | 'brand' | 'name' | 'image' | 'icon', value: string | null) => {
    if (!m) return
    try {
      setList(await wb.setMachineField(m.id, field, value))
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /**
   * 左列是不是在看一个品牌（2026-10-03：品牌进了这一页）。
   * 品牌卡与机型卡共用一个右栏 —— 选中品牌时右侧换成品牌卡
   */
  const curBrand = list?.brands.find((b) => b.id === brandId) ?? undefined

  /**
   * **把一台机型挪到另一个品牌下**（2026-10-03）。即时落盘，回一份新清单。
   *
   * toast 里说清"从哪家挪到哪家" —— 移动是一次看不见的落盘，说清楚才知道成没成。
   */
  const moveMachine = async (machineId: string, brandId: string) => {
    const from = list?.machines.find((x) => x.id === machineId)?.brand ?? ''
    const to = list?.brands.find((b) => b.id === brandId)
    try {
      setList(await wb.moveMachineToBrand(machineId, brandId))
      setMoveOf(null)
      // 挪过去之后选中不丢：机型 id 没变，右侧卡照旧挂着
      toasts.push(`已把 ${machineId} 从「${from}」挪到「${to?.name || brandId}」`)
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 改品牌的一格（显示名 / 品牌图）。**即时落盘**，回一份新清单 */
  const saveBrand = async (field: 'name' | 'logo', value: string | null) => {
    if (!curBrand) return
    try {
      setList(await wb.setBrandField(curBrand.id, field, value))
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /** 改版本的一格。同上 */
  const saveVersion = async (
    versionId: string,
    field: 'name' | 'tag' | 'description' | 'recommendedBundle' | 'image',
    value: string | null,
  ) => {
    if (!m) return
    try {
      setList(await wb.setVersionField(m.id, versionId, field, value))
      if (field === 'recommendedBundle') onBookRefresh()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
    }
  }

  /* —— 生成（③ 配方段的按钮 + 「下一步」里那颗是同一件事） —— */

  const dirty = book.dirtyCount > 0
  const myBlocks = useCallback(
    (uid: string) =>
      (report?.issues ?? []).filter((i) => i.severity === 'block' && i.at.uid === uid),
    [report],
  )

  /** 这个版本为什么现在不能生成；空串 = 能 */
  const genNote = (mid: string, vid: string): string => {
    const uid = nodeOf(mid, vid)?.uid
    if (dirty) return '有未保存改动 —— 生成读的是已保存的那一份，先保存才能生成'
    if (uid === undefined) return ''
    const blocks = myBlocks(uid)
    if (blocks.length > 0) return `有 ${blocks.length} 条阻断没处理完：${blocks[0].title}`
    const row = rowOf(uid)
    if (row && !row.buildable) return row.disabledReason ?? row.reason
    return ''
  }

  const doBuild = useCallback(
    async (mid: string, vid: string) => {
      const node = nodeOf(mid, vid)
      if (node === undefined) return
      try {
        const rep = await wb.generate({ picked: [node.uid] })
        // 生成记录由**后端生成事务直接落进台账**（built.json）—— 前端不再回填草稿，
        // 生成完成 = 台账已是这一代（2026-10-06 状态机修正）。
        const file = rowOf(node.uid)?.mkpFile
        toasts.push(
          `已生成 ${file ?? '预设'}：写出 ${rep.written.length} 份` +
            (rep.unchanged.length > 0 ? `，${rep.unchanged.length} 份内容没变、跳过重写` : ''),
        )
      } catch (e) {
        toasts.push(isAppError(e) ? e.message : String(e))
      }
    },
    [nodeOf, rowOf],
  )

  /** 先落盘再生成 —— 一个按钮两个动作，顺序规定死：先保存，成了才生成 */
  const saveAndBuild = useCallback(
    async (mid: string, vid: string) => {
      if (!(await onSave())) return
      await doBuild(mid, vid)
    },
    [onSave, doBuild],
  )

  /* —— 右键菜单：按 `menu.target` 的**前缀**分派 ——
   *
   * 版本行的键是 `机型/版本`（老形状，没有前缀），机型行是 `machine:<id>`。
   * 加了前缀才分得清：一个 `A1` 既可能是机型、也可能是某台机器的版本名叫 A1。 */
  const menuIsMachine = (menu.target ?? '').startsWith('machine:')
  /*
   * ★ 机器 id **在构造菜单时就闭包进去**，不在 `onSelect` 里现读 `menu.target`：
   * `ContextMenu` 点中一项的顺序是「先 `onClose()`（target 变 null）→ 再 `onSelect()`」
   * （`ContextMenu.tsx:142`），现读会拿到空串 —— 表现是菜单点得动、弹窗永远不开。
   */
  const menuMachineId = menuIsMachine
    ? (menu.target ?? '').slice('machine:'.length)
    : null
  /*
   * ★ 菜单项拿到的 id 是**渲染时算好的常量**（上面那个），不是 `menu.target`。
   *   `ContextMenu.pick()` 的顺序是「先 `onClose()` → 再 `onSelect()`」，`onSelect`
   *   里现读 `menu.target` 会读到 null（菜单已关）—— 表现是菜单点得动、弹窗永远不开。
   */
  const entries: ContextMenuEntry[] =
    (menuMachineId !== null && list?.machines.some((x) => x.id === menuMachineId)
      ? [
          {
            id: 'move-brand',
            label: '移到品牌…',
            onSelect: () => setMoveOf(menuMachineId),
          },
        ]
      : null) ??
    (menu.target && m
      ? [
          {
            id: 'copy',
            label: '复制版本…',
            onSelect: () => {
              const tid = (menu.target ?? '').split('/')[1] ?? ''
              setCopyOpen(`${m.id}/${tid}`)
              setCopyId(`${tid}_COPY`)
              const tpl = m.versions.find((v) => v.id === tid)
              setCopyName(tpl ? `${tpl.name || tid} 副本` : '')
              setWithRecipe(true)
            },
          },
          {
            id: 'edit-v',
            label: '编辑版本…',
            onSelect: () => {
              const vid = (menu.target ?? '').split('/')[1] ?? ''
              const v = m.versions.find((x) => x.id === vid)
              setEditVOpen(`${m.id}/${vid}`)
              setEvName(v?.name ?? '')
              setEvTag(v?.tag ?? '')
              setEvDesc(v?.description ?? '')
            },
          },
          {
            id: 'delete',
            label: '删除版本…',
            danger: true,
            onSelect: () => setDel({ vid: `${m.id}/${(menu.target ?? '').split('/')[1] ?? ''}`, orphans: null }),
          },
        ]
      : [])

  /* —— 弹窗公共的提交失败出口 —— */
  const failToast = (e: unknown) => toasts.push(isAppError(e) ? e.message : String(e))

  if (pageErr !== null) {
    return (
      <p className="wb-todo" data-tone="danger">
        {pageErr}
      </p>
    )
  }
  if (!list || !assets || !bundles || !registry) {
    return <p className="wb-todo">正在读机型目录、资产库与注册表…</p>
  }

  /* 机型全删完不白屏：给空状态（C04 的教训） */
  if (!list.machines.length) {
    return (
      <div className={s.detail}>
        <div className={s.empty}>
          <h2>一台机型都没有了</h2>
          <p>机型是一切的起点 —— 配方、套餐、切片器都挂在它下面。先建一台：</p>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            onClick={() => {
              setAddId('')
              setAddBrand(list.brands[0]?.name ?? '')
              setAddDisplay('')
              setAddOpen(true)
            }}
          >
            新增机型
          </button>
        </div>
        {null}
      </div>
    )
  }
  if (!m) return null

  const pickedVidParts = pickedVid?.split('/') ?? null
  const picked: VersionView | undefined =
    pickedVidParts && pickedVidParts[0] === m.id
      ? m.versions.find((v) => v.id === pickedVidParts[1])
      : undefined
  const pickedNode = picked ? nodeOf(m.id, picked.id) : undefined
  const pickedRow = pickedNode ? rowOf(pickedNode.uid) : undefined
  const todo = todoOf(m)
  const visibleCount = registry.params.filter(
    (p) => p.machineFilter.length === 0 || p.machineFilter.includes(m.id),
  ).length
  /**
   * 品牌选项：**值写 id、标签显示显示名**（数据里 `machine.brand` 存的是品牌 id ——
   * 2026-10-03 之前这里给的是 `b.name`，选一次就会把显示名写进机型文件，是条静默的脏写）。
   */
  const brandOptions = list.brands.map((b) => ({
    value: b.id,
    label: b.name || b.id,
    note: b.name && b.name !== b.id ? b.id : undefined,
  }))
  /** 这一格能对上一条品牌吗（对得上才给「看品牌」那颗按钮） */
  const brandOf = (id: string) => list.brands.find((b) => b.id.toLowerCase() === id.trim().toLowerCase())
  const imageOptions = assets.assets.filter((a) => a.kind === 'image')
  const iconOptions = assets.assets.filter((a) => a.kind === 'icon')
  const assetById = (id: string | null) =>
    id ? (assets.assets.find((a) => a.id === id) ?? null) : null

  const addIdTaken = list.machines.some((x) => x.id === addId.trim())
  const addReady = addId.trim() !== '' && addDisplay.trim() !== '' && addBrand.trim() !== ''
  const avTaken = m.versions.some((v) => v.id === avId.trim())
  const avReady = idOk(avId) && avName.trim() !== '' && !avTaken
  const copyParts = copyOpen?.split('/') ?? null
  const copyTpl = copyParts ? m.versions.find((v) => v.id === copyParts[1]) : undefined
  const copyTaken = copyOpen !== null && m.versions.some((v) => v.id === copyId.trim())
  const copyReady = copyOpen !== null && idOk(copyId) && !copyTaken && copyId.trim() !== ''

  /** 新增机型。后端三格都必填（catalog::add_machine 逐格拦），按钮跟着它走 */
  const submitAddMachine = async () => {
    try {
      const next = await wb.addMachine(addId.trim(), addBrand.trim(), addDisplay.trim())
      setList(next)
      setMachineId(addId.trim())
      setPickedVid(null)
      setAddOpen(false)
      toasts.push(`已新增机型 ${addId.trim()} —— presets/machines/${addId.trim()}.toml 已写入`)
      afterStructural()
    } catch (e) {
      failToast(e)
    }
  }

  /** 新增版本。id 校验在后端（字符集 + 机型内唯一），名字必填 */
  const submitAddVersion = async () => {
    try {
      const next = await wb.addVersion(m.id, avId.trim(), avName.trim())
      setList(next)
      setPickedVid(vidOf(m.id, avId.trim()))
      setAddVOpen(false)
      toasts.push(`已新增版本 ${m.id}/${avId.trim()} —— 写进了 ${m.file}`)
      afterStructural()
    } catch (e) {
      failToast(e)
    }
  }

  /**
   * 复制版本（14.3 / 14.5 的两步流）。两条命令各自**只写单文件**：
   * 先建版本定义；勾了「同时复制配方」再补一刀参数正文 —— 拷出来是独立快照，
   * 之后改模板、改基底都传不到它身上。
   */
  const submitCopy = async () => {
    if (copyParts === null) return
    const [, templateId] = copyParts
    try {
      const next = await wb.copyVersion(
        m.id,
        templateId,
        copyId.trim(),
        copyName.trim() || `${copyTpl?.name ?? templateId} 副本`,
        copyTpl?.tag ?? undefined,
        copyTpl?.description ?? undefined,
      )
      setList(next)
      let recipeNote = '只复制了版本定义'
      if (withRecipe) {
        try {
          const n = await wb.copyRecipe(m.id, templateId, copyId.trim())
          recipeNote = `参数正文已复制（${n} 项，独立快照）`
        } catch (e) {
          failToast(e)
          recipeNote = '版本定义已建，但参数正文复制失败 —— 可以在参数台重试'
        }
      }
      toasts.push(`已复制为 ${m.id}/${copyId.trim()} —— ${recipeNote}`)
      setPickedVid(vidOf(m.id, copyId.trim()))
      setCopyOpen(null)
      afterStructural()
    } catch (e) {
      failToast(e)
    }
  }

  /** 删除版本第二步。**立刻落盘，不可逆** —— 所以第一步先问孤儿 */
  const submitDelete = async () => {
    if (del === null || del.orphans === null || !m) return
    const [, vid] = del.vid.split('/')
    try {
      const next = await wb.removeVersion(m.id, vid)
      setList(next)
      if (pickedVid === del.vid) setPickedVid(null)
      setDel(null)
      toasts.push(`已删除版本 ${m.id}/${vid} —— ${m.file} 已写回`)
      afterStructural()
    } catch (e) {
      failToast(e)
    }
  }

  /** 批量改版本身份三格（编辑版本模态框）。一格失败就停，不静默半成功 */
  const submitEditVersion = async () => {
    if (editVOpen === null || !evName.trim()) return
    const [, vid] = editVOpen.split('/')
    try {
      let next = await wb.setVersionField(m.id, vid, 'name', evName)
      next = await wb.setVersionField(m.id, vid, 'tag', evTag.trim() === '' ? null : evTag)
      next = await wb.setVersionField(
        m.id,
        vid,
        'description',
        evDesc.trim() === '' ? null : evDesc,
      )
      setList(next)
      toasts.push(`已更新 ${m.id}/${vid} 的版本信息`)
      setEditVOpen(null)
    } catch (e) {
      failToast(e)
    }
  }

  /* 下一步：按真实工作流挑第一件没做完的事（判据全部来自上面的派生） */
  const nextStep = (() => {
    if (!picked || !pickedNode) return null
    const uid = pickedNode.uid
    if (dirty)
      return {
        label: '保存改动',
        hint: '这些改动还在草稿里 —— 生成读的是已保存的那一份。',
        run: () => void onSave(),
      }
    if (!picked.hasRecipe)
      return {
        label: '补参数源',
        hint: '这个版本还是纯继承基底（参数源待补）—— 去参数台把它的参数正文钉下来。',
        run: () => onGoto('params', { machineId: m.id, uid, key: null }),
      }
    const blocks = myBlocks(uid)
    if (blocks.length > 0)
      return {
        label: `处理 ${blocks.length} 条阻断`,
        hint: `${blocks[0].title} —— 阻断是唯一的硬闸门，照这份数据生成出来的一定是坏的。`,
        run: () => onGoto('build', { machineId: m.id, uid, key: null }),
      }
    if (pickedNode.build === 'neverBuilt' || pickedNode.build === 'stale')
      return {
        label: pickedNode.build === 'neverBuilt' ? '生成配方' : '重新生成',
        hint:
          pickedNode.build === 'neverBuilt'
            ? '还没生成过，客户端现在下载不到这一版。'
            : '配方改过了，磁盘上的产物还是上次生成那份。',
        run: () => void doBuild(m.id, picked.id),
      }
    if (!picked.recommendedBundle)
      return {
        label: '选择套餐',
        hint: '套餐决定客户端会装到哪几份文件。',
        run: () => {
          setBunOpen(vidOf(m.id, picked.id))
          setBunPick('')
        },
      }
    return {
      label: '去检查与生成',
      hint: '这一版该做的都做完了 —— 下一阶段是整个交付包的检查与生成。',
      run: () => onGoto('build', { machineId: m.id, uid, key: null }),
    }
  })()

  /** 打开删除弹窗的第一步：先问孤儿（删除独有的风险，用户不查就不知道） */
  const askDelete = (vid: string) => {
    setDel({ vid: vidOf(m.id, vid), orphans: null })
    void wb
      .versionOrphans(m.id, vid)
      .then((orphans) => setDel((cur) => (cur && cur.vid === vidOf(m.id, vid) ? { ...cur, orphans } : cur)))
      .catch(failToast)
  }

  return (
    <div className={s.split} ref={bodyRef} style={rail.style}>
      {/* —— 左列：筛选 + 机型行 —— */}
      <div className={s.side}>
        <div className={s.topRow}>
          <input
            className={s.filter}
            value={filter}
            placeholder="筛品牌 / 机型（id / 显示名）"
            aria-label="筛选品牌与机型"
            onChange={(e) => setFilter(e.target.value)}
          />
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            title="新增一台机型 —— 建一个 presets/machines/ 下的新文件，三格都必填"
            onClick={() => {
              setAddId('')
              // 机型的 brand 字段写着品牌的 **id**（不是显示名）—— 默认给第一家
              setAddBrand(list.brands[0]?.id ?? '')
              setAddDisplay('')
              setAddOpen(true)
            }}
          >
            新增机型
          </button>
        </div>

        {/*
         * 品牌（2026-10-03，作者）：「在这一页多加一个品牌吧，右侧也是一样可以编辑，
         * 品牌图、显示名，不管客户端消不消费都提供」。品牌与机型是两类条目，
         * 用一个列表区分两条小标题分开 —— 点品牌看品牌卡，点机型看机型卡。
         */}
        {/*
         * **一棵树**（2026-10-03，作者：「我想象中的就像那种树状的感觉一样…很明显的
         * 能看到他们的父子关系。现在这种这么割裂」）：品牌是父节点、机型缩进当子节点。
         *
         * 之前是「上面一个品牌容器 + 下面一个机型容器」两坨并列 —— 父子关系只能靠
         * 脑子拼。改树之后「新增品牌就多一棵」「把机型移到别的品牌下」都是同一件事的
         * 两种表现。
         */}
        <div className={s.listHead}>
          品牌 · 机型
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            title="新建一个品牌 —— 往 presets/brands.toml 的 [[brands]] 里加一段（品牌图后配）"
            onClick={() => {
              setNbId('')
              setNbName('')
              setAddBrandOpen(true)
            }}
          >
            新增品牌
          </button>
        </div>
        <div className={`${s.list} ${s.treeList}`}>
          {tree.map(({ brand: b, machines: kids }) => {
            const folded = foldedBrands.has(b.id)
            return (
              <div key={b.id} className={s.treeGroup}>
                {/* 品牌行：折叠箭头 + 显示名 + 内部名 + 台数。点它开右侧品牌卡 */}
                <div className={`${s.treeParent} ${b.id === brandId ? s.treeParentOn : ''}`}>
                  <button
                    type="button"
                    className={s.treeTwist}
                    title={folded ? '展开这个品牌下的机型' : '收起这个品牌下的机型'}
                    aria-expanded={!folded}
                    aria-label={`${folded ? '展开' : '收起'} ${b.name || b.id}`}
                    onClick={() => setFoldedBrands((prev) => {
                      const next = new Set(prev)
                      if (next.has(b.id)) next.delete(b.id)
                      else next.add(b.id)
                      return next
                    })}
                  >
                    {/* 三角指右 = 收着，指下 = 展着（与参数台左树同一种记号） */}
                    <svg viewBox="0 0 12 12" aria-hidden data-open={!folded}>
                      <path d="M4 2.5 8 6l-4 3.5z" fill="currentColor" />
                    </svg>
                  </button>
                  <button
                    type="button"
                    id={`t-brand-${b.id}`}
                    className={s.treeParentName}
                    onClick={() => setBrandId(b.id)}
                  >
                    <span className={s.rowName}>{b.name || b.id}</span>
                    <span className={`${s.mono} ${s.treeMeta}`}>{b.id}</span>
                    <span className={s.treeMeta}>{kids.length} 台机型</span>
                    {!b.logo && <span className={`${s.tag} ${s.tagGhost}`}>没配图</span>}
                  </button>
                </div>

                {/* 机型行：缩进一级。右键出「移到品牌…」 */}
                {!folded && (
                  <div className={s.treeKids}>
                    {kids.map((x) => {
                      const t = todoOf(x)
                      return (
                        <button
                          key={x.id}
                          type="button"
                          /* 探针与跨页定位的锚点（与 `t-brand-*` / `t-bundle-*` / `t-asset-*` 同一套命名） */
                          id={`t-machine-${x.id}`}
                          className={`${s.treeKid} ${brandId === null && x.id === m.id ? s.treeKidOn : ''}`}
                          {...menu.triggerProps(machineKey(x.id))}
                          onClick={() => {
                            setMachineId(x.id)
                            setPickedVid(null)
                            setBrandId(null)
                          }}
                        >
                          <span className={s.rowName}>{x.display || x.id}</span>
                          {/* id 也要露出来：复制出来的机型和原机同名，只看 display 分不清 */}
                          <span className={`${s.mono} ${s.rowMeta}`}>{x.id}</span>
                          <span className={s.rowMeta}>{x.versions.length} 版</span>
                          {!x.hasDimensions && <span className={`${s.tag} ${s.tagGhost}`}>占位</span>}
                          {t > 0 && (
                            <span className={s.todo} title={fieldState.needsInputHint}>
                              {fieldState.needsInput} {t}
                            </span>
                          )}
                        </button>
                      )
                    })}
                    {!kids.length && (
                      <div className={s.treeEmpty}>
                        这个品牌下还没有机型 —— 右键机型 →「移到品牌…」可以把别的机器挪过来
                      </div>
                    )}
                  </div>
                )}
              </div>
            )
          })}
          {!tree.length && (
            <div className={s.sum}>
              没有匹配「{filter}」的品牌或机型{' '}
              <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setFilter('')}>
                清空筛选
              </button>
            </div>
          )}
        </div>
        <div className={s.sum}>
          {listed.length === list.machines.length
            ? `${list.machines.length} 台机型 · ${list.machines.reduce((n, x) => n + x.versions.length, 0)} 个版本`
            : `筛出 ${listed.length} / ${list.machines.length} 台`}
        </div>
        {/* 明写数据根：这一页读写的是 presets/machines/，与参数台的草稿是两个地方 */}
        <div className={s.sum} title={list.root}>
          读写 {list.root}
        </div>
      </div>

      {/* —— 右侧：品牌卡 或 机型的四张卡 —— */}
      <div className={s.detail}>
        {curBrand !== undefined ? (
          /*
           * 品牌卡（2026-10-03）。字段是作者逐条点的：**显示名 / 品牌图 / 这个品牌的机型**。
           * 机型的「品牌」那一格写的是品牌 id，反查由后端算（前端只摆结果）。
           */
          <div className={s.card}>
            <div className={s.cardHead}>
              <h2>{curBrand.name || curBrand.id}</h2>
              <span className={s.cardNote}>{curBrand.id}</span>
              <span className={s.cardNote}>presets/brands.toml</span>
            </div>
            <div className={s.cardBody}>
              <div className={s.kv}>
                <span className={s.kvKey}>显示名</span>
                <span className={s.kvVal}>
                  <input
                    className={s.inp}
                    defaultValue={curBrand.name}
                    key={`bname-${curBrand.id}-${curBrand.name}`}
                    aria-label="品牌显示名"
                    onBlur={(e) => {
                      const next = e.target.value.trim()
                      if (next !== curBrand.name) void saveBrand('name', next)
                    }}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                    }}
                  />
                  {!curBrand.name.trim() && <TodoValue />}
                </span>

                <span className={s.kvKey}>内部名</span>
                <span className={`${s.kvVal} ${s.mono}`}>{curBrand.id}</span>

                <span className={s.kvKey}>品牌图</span>
                <span className={s.kvVal}>
                  <AssetField
                    label="品牌图"
                    asset={assets.assets.find((a) => a.id === curBrand.logo) ?? null}
                    onOpen={() => setLogoOpen(true)}
                  />
                  {/* 说明里别写 markdown 的星号 —— JSX 不认识它，会原样落到界面上 */}
                  <span className={s.cardNote}>
                    留空 = 客户端回落内置字标（这一格存的是资产 id，换图去资产库）
                  </span>
                </span>
              </div>

              <div className={s.group}>
                <div className={s.groupHead}>这个品牌下的机型（{curBrand.machines.length}）</div>
                <div className={s.chips}>
                  {curBrand.machines.length ? (
                    curBrand.machines.map((mid) => (
                      <button
                        key={mid}
                        type="button"
                        className={s.chip}
                        title="去机型卡看它"
                        onClick={() => {
                          setMachineId(mid)
                          setBrandId(null)
                        }}
                      >
                        {mid}
                      </button>
                    ))
                  ) : (
                    <span className={s.kvDim}>没有机型归这个品牌</span>
                  )}
                </div>
                <p className={s.note}>
                  机型的「品牌」那一格写着这个品牌的 id —— 反查在后端算，这里只摆结果。
                  改归属去机型的身份卡。
                </p>
              </div>
            </div>
          </div>
        ) : (
        <>
        {/* 身份 */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>身份</h2>
            <span className={s.cardNote}>{m.id}</span>
            {todo > 0 && (
              <span className={s.todo} title={fieldState.needsInputHint}>
                {fieldState.needsInput} {todo} 项
              </span>
            )}
            <span className={s.cardNote}>{m.file}</span>
          </div>
          <div className={s.cardBody}>
            <div className={s.kv}>
              <span className={s.kvKey}>
                {machineFieldLabels.display}
                <FieldMark later />
              </span>
              <span className={s.kvVal}>
                <input
                  className={s.inp}
                  defaultValue={m.display}
                  key={`display-${m.id}-${m.display}`}
                  aria-label={machineFieldLabels.display}
                  onBlur={(e) => {
                    const next = e.target.value.trim()
                    if (next !== m.display) void saveMachine('display', next)
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                  }}
                />
                {!m.display.trim() && <TodoValue />}
              </span>

              <span className={s.kvKey}>
                {machineFieldLabels.brand}
                <FieldMark later />
              </span>
              <span className={s.kvVal}>
                <PickOrType
                  label={machineFieldLabels.brand}
                  options={brandOptions}
                  value={m.brand}
                  emptyLabel={`选一个品牌…（空着会标「${fieldState.needsInput}」）`}
                  /* 品牌是一等条目了 —— 手打 id 只会打出悬空引用；新增品牌走左列那颗按钮 */
                  allowCustom={false}
                  onChange={(next) => {
                    if (!next.trim()) {
                      toasts.push('品牌不许清空 —— 它是身份的一部分')
                      return
                    }
                    void saveMachine('brand', next)
                  }}
                />
                {brandOf(m.brand) !== undefined && (
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    title="去品牌卡看它（显示名与品牌图在那一张卡上改）"
                    onClick={() => setBrandId(brandOf(m.brand)!.id)}
                  >
                    看品牌
                  </button>
                )}
                {!m.brand.trim() && <TodoValue />}
              </span>

              {/* 下面三格可空 —— 空着是正常状态，不标任何东西 */}
              <span className={s.kvKey}>{machineFieldLabels.name}</span>
              <span className={s.kvVal}>
                <input
                  className={s.inp}
                  defaultValue={m.name}
                  key={`name-${m.id}-${m.name}`}
                  aria-label={machineFieldLabels.name}
                  placeholder="可空"
                  onBlur={(e) => {
                    const next = e.target.value.trim()
                    if (next !== m.name) void saveMachine('name', next === '' ? null : next)
                  }}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                  }}
                />
              </span>

              <span className={s.kvKey}>{machineFieldLabels.image}</span>
              <span className={s.kvVal}>
                <AssetField
                  label={machineFieldLabels.image}
                  asset={assetById(m.image)}
                  onOpen={() => setImgOpen(true)}
                />
              </span>

              <span className={s.kvKey}>{machineFieldLabels.icon}</span>
              <span className={s.kvVal}>
                <AssetField
                  label={machineFieldLabels.icon}
                  asset={assetById(m.icon)}
                  onOpen={() => setIconOpen(true)}
                />
              </span>

              <span className={s.kvKey}>别名</span>
              <span className={s.kvVal}>
                {m.externalAliases.length ? (
                  <span className={s.chips}>
                    {m.externalAliases.map((a) => (
                      <span key={a} className={s.chip}>
                        {a}
                      </span>
                    ))}
                  </span>
                ) : (
                  <span className={s.kvDim}>—</span>
                )}
                <span className={s.cardNote}>对照名来自数据文件，工作台不给编辑</span>
              </span>
            </div>
          </div>
        </div>

        {/*
         * 尺寸（2026-10-03：从「已配置 / 禁区 N 块」两格升成**六组读数**）
         * —— 照旧面板 mkppanel 的六组口径，每组标签带原始键名。
         */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>尺寸</h2>
            <span className={s.cardNote}>presets/machines/{m.file} 的 [dimensions]</span>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              disabled={!m.hasDimensions && m.dimensions === null}
              title={
                m.dimensions === null
                  ? '这台还没有 [dimensions] —— 先建一份尺寸（床身那两格必填）'
                  : '改这六组读数（床身 / 移动范围 / 边缘 / 涂胶 / 标定点 / 标志位）'
              }
              onClick={() => setDimsOpen(m.id)}
            >
              {m.dimensions === null ? '新建尺寸' : '编辑尺寸'}
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              title="在床身图上画禁区（擦嘴 / 挡块那几块碰不得的区域）"
              onClick={() => setZoneOpen(m.id)}
            >
              编辑禁区{m.zoneCount > 0 ? `（${m.zoneCount}）` : ''}
            </button>
            {m.dimensions !== null && (
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                title="移动范围 / 边缘涂胶 / 标定点 / 标志位这四组读数，收着不占地方"
                onClick={() => setDimsAll((v) => !v)}
              >
                {dimsAll ? '收起明细' : '全部明细'}
              </button>
            )}
          </div>
          <div className={s.cardBody}>
            {m.dimensions === null ? (
              <p className={s.note} style={{ margin: 0 }}>
                {placeholderText.noDimensions} —— 占位机型不参与交付，检查与生成页会给一条说明而不是报错。
              </p>
            ) : (
              <>
                <div className={s.kv}>
                  <span className={s.kvKey}>床身 W × D</span>
                  <span className={`${s.kvVal} ${s.mono}`}>
                    {m.dimensions.bedSize.width} × {m.dimensions.bedSize.depth} mm
                  </span>
                </div>

                {/* 默认收起的四组：点「全部明细」展开（编辑尺寸模态框里也都有） */}
                {dimsAll && (
                  <>
                    <div className={s.group}>
                      <div className={s.groupHead}>移动范围 (movementRange)</div>
                      <div className={s.kv}>
                        <span className={s.kvKey}>X (minX / maxX)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.movementRange.minX} / {m.dimensions.movementRange.maxX}
                        </span>
                        <span className={s.kvKey}>Y (minY / maxY)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.movementRange.minY} / {m.dimensions.movementRange.maxY}
                        </span>
                        <span className={s.kvKey}>Z 最大值 (maxZ)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>{m.dimensions.movementRange.maxZ}</span>
                      </div>
                    </div>

                    <div className={s.group}>
                      <div className={s.groupHead}>边缘与涂胶 (edgeZone / glueArea)</div>
                      <div className={s.kv}>
                        <span className={s.kvKey}>边缘范围 (edgeZone)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>{m.dimensions.edgeZone} mm</span>
                        <span className={s.kvKey}>涂胶 X (min / max)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.glueArea.glueMinX} / {m.dimensions.glueArea.glueMaxX}
                        </span>
                        <span className={s.kvKey}>涂胶 Y (min / max)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.glueArea.glueMinY} / {m.dimensions.glueArea.glueMaxY}
                        </span>
                        <span className={s.kvKey}>擦料 X (wipeX)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>{m.dimensions.glueArea.wipeX}</span>
                      </div>
                    </div>

                    <div className={s.group}>
                      <div className={s.groupHead}>标定点 (calibration)</div>
                      <div className={s.kv}>
                        <span className={s.kvKey}>L 形基点 X / Y</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.calibration.lShapeBaseX} / {m.dimensions.calibration.lShapeBaseY}
                        </span>
                        <span className={s.kvKey}>Z 起点 X / Y (zStart)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.calibration.zStartX} / {m.dimensions.calibration.zStartY}
                        </span>
                        <span className={s.kvKey}>X 线 (xLineX / Y / YEnd)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.calibration.xLineX} / {m.dimensions.calibration.xLineY} /{' '}
                          {m.dimensions.calibration.xLineYEnd}
                        </span>
                        <span className={s.kvKey}>Y 线 (yLineX / XEnd / Y)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>
                          {m.dimensions.calibration.yLineX} / {m.dimensions.calibration.yLineXEnd} /{' '}
                          {m.dimensions.calibration.yLineY}
                        </span>
                      </div>
                    </div>

                    <div className={s.group}>
                      <div className={s.groupHead}>标志位 (flags)</div>
                      <div className={s.kv}>
                        <span className={s.kvKey}>G-code 标记 (gcodeMarker)</span>
                        <span className={`${s.kvVal} ${s.mono}`}>{m.dimensions.flags.gcodeMarker}</span>
                        <span className={s.kvKey}>有第二风扇 (hasSecondFan)</span>
                        <span className={s.kvVal}>{m.dimensions.flags.hasSecondFan ? '是' : '否'}</span>
                      </div>
                    </div>
                  </>
                )}

                <div className={s.group}>
                  <div className={s.groupHead}>禁区</div>
                  <div className={s.kv}>
                    <span className={s.kvKey}>块数</span>
                    <span className={s.kvVal}>
                      {m.zoneCount > 0 ? (
                        `${m.zoneCount} 块（${m.zones.map((z) => `${z.points.length} 点`).join(' / ')}）`
                      ) : (
                        <span className={s.kvDim}>没有禁区文件</span>
                      )}
                    </span>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>

        {/* 版本 */}
        <div className={s.card}>
          <div className={s.cardHead}>
            <h2>版本 {m.versions.length}</h2>
            <span className={s.cardNote}>点一行看详情，右键出菜单</span>
            <button
              type="button"
              className={`${s.btn} ${s.btnSm}`}
              onClick={() => {
                setAvId('')
                setAvName('')
                setAddVOpen(true)
              }}
            >
              新增版本
            </button>
          </div>
          <div className={s.cardBody}>
            {m.versions.map((v) => {
              const node = nodeOf(m.id, v.id)
              const st = node?.build ?? 'neverBuilt'
              const bbsHint = node
                ? node.bbsSource === 'inheritedFromMachine'
                  ? `切片器 ${node.bbsCount}（跟机型默认）`
                  : `切片器 ${node.bbsCount}（这个套餐自己的）`
                : '切片器 —'
              return (
                <div
                  key={v.id}
                  className={`${s.row} ${pickedVid === vidOf(m.id, v.id) ? s.rowPick : ''}`}
                  {...menu.triggerProps(vidOf(m.id, v.id))}
                  onClick={() => setPickedVid(pickedVid === vidOf(m.id, v.id) ? null : vidOf(m.id, v.id))}
                >
                  <span className={`${s.mono} ${s.rowMeta}`}>{v.id}</span>
                  <span className={s.rowName}>{v.name}</span>
                  <span className={s.rowMeta}>{v.recommendedBundle ?? placeholderText.noBundle}</span>
                  {!v.hasRecipe && (
                    <span className={s.todo} title="纯继承机型基底，还没有自己的参数正文（14.4）">
                      参数源待补
                    </span>
                  )}
                  <span
                    className={`${s.tag} ${STATE_TAG[st]}`}
                    title={`${words.build[st].explain ?? ''} · ${bbsHint}`}
                  >
                    {words.build[st].label}
                  </span>
                </div>
              )
            })}
            {!m.versions.length && (
              <p className={s.note} style={{ margin: 0 }}>
                这台机型还没有版本 —— 占位机型的正常状态。
              </p>
            )}
            <p className={s.note}>
              新增与删除立刻写入文件（没有草稿也没有撤销，删之前会先问孤儿引用）；
              复制走两条命令：先版本定义，勾了才拷参数正文。
            </p>
          </div>
        </div>

        {/* 版本详情 */}
        {picked && pickedNode && (
          <div className={s.card}>
            <div className={s.cardHead}>
              <h2 className={s.mono}>{picked.id}</h2>
              <span className={s.cardNote}>{picked.name}</span>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  setEditVOpen(vidOf(m.id, picked.id))
                  setEvName(picked.name)
                  setEvTag(picked.tag ?? '')
                  setEvDesc(picked.description ?? '')
                }}
              >
                编辑
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnSm}`}
                onClick={() => {
                  setCopyOpen(vidOf(m.id, picked.id))
                  setCopyId(`${picked.id}_COPY`)
                  setCopyName(`${picked.name || picked.id} 副本`)
                  setWithRecipe(true)
                }}
              >
                复制
              </button>
              <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => askDelete(picked.id)}>
                删除
              </button>
              <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setPickedVid(null)}>
                收起
              </button>
            </div>
            <div className={s.cardBody}>
              {/* ① 身份 */}
              <div className={s.sect}>
                <div className={s.sectHead}>① 身份</div>
                <div className={s.kv}>
                  <span className={s.kvKey}>
                    版本名称
                    <FieldMark />
                  </span>
                  <span className={s.kvVal}>
                    <input
                      className={s.inp}
                      defaultValue={picked.name}
                      key={`vname-${picked.id}-${picked.name}`}
                      aria-label="版本名称"
                      onBlur={(e) => {
                        const next = e.target.value.trim()
                        if (next !== picked.name) {
                          if (!next) toasts.push('版本名不许清空 —— 检查与生成页要靠它认版本')
                          else void saveVersion(picked.id, 'name', next)
                        }
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                      }}
                    />
                  </span>

                  <span className={s.kvKey}>版本 id</span>
                  <span className={s.kvVal}>
                    <span className={s.mono}>{picked.id}</span>
                    <span className={s.cardNote}>改 id = 删掉再加一个 —— 这一步后端还没做</span>
                  </span>

                  <span className={s.kvKey}>标签</span>
                  <span className={s.kvVal}>
                    <input
                      className={s.inp}
                      defaultValue={picked.tag ?? ''}
                      key={`vtag-${picked.id}-${picked.tag ?? ''}`}
                      aria-label="标签"
                      placeholder="可空"
                      onBlur={(e) => {
                        const next = e.target.value.trim()
                        if (next !== (picked.tag ?? '')) void saveVersion(picked.id, 'tag', next === '' ? null : next)
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                      }}
                    />
                  </span>

                  <span className={s.kvKey}>描述</span>
                  <span className={s.kvVal}>
                    <input
                      className={s.inp}
                      defaultValue={picked.description ?? ''}
                      key={`vdesc-${picked.id}-${picked.description ?? ''}`}
                      aria-label="描述"
                      placeholder="可空"
                      onBlur={(e) => {
                        const next = e.target.value.trim()
                        if (next !== (picked.description ?? ''))
                          void saveVersion(picked.id, 'description', next === '' ? null : next)
                      }}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') (e.target as HTMLInputElement).blur()
                      }}
                    />
                  </span>

                  {/* 这一版专属的外观图。**留空 = 回落机型图**（作者定的默认）——
                      界面上说清那是回落，别让人以为"没配"是坏了 */}
                  <span className={s.kvKey}>版本图</span>
                  <span className={s.kvVal}>
                    <AssetField
                      label="版本图"
                      asset={assetById(picked.image)}
                      onOpen={() => setVerImgOpen(true)}
                    />
                    <span className={s.cardNote}>
                      留空就用这台机型的图（图位分层：品牌 → 机型 → 版本，版本缺则回落机型）
                    </span>
                  </span>
                </div>
              </div>

              {/* ② 参数 —— 这里不放表单，参数台是编辑工作区；这一页只回答三件事 */}
              <div className={s.sect}>
                <div className={s.sectHead}>② 参数</div>
                <div className={s.kv}>
                  <span className={s.kvKey}>{picked.hasRecipe ? '本版本覆盖' : '参数源'}</span>
                  <span className={s.kvVal}>
                    {picked.hasRecipe ? (
                      <>
                        <span className={s.tnum}>{pickedNode.items}</span>
                        <span className={s.cardNote}>
                          {' '}
                          项 · 这台机型可见参数共 {visibleCount} 条，其余跟随机型基底
                        </span>
                      </>
                    ) : (
                      <span className={s.todo} title="还没有自己的参数正文 —— 值全部继承机型基底">
                        参数源待补
                      </span>
                    )}
                  </span>
                </div>
                <div className={s.sectOps}>
                  <button
                    type="button"
                    className={`${s.btn} ${s.btnSm}`}
                    title="参数在参数台专业地改 —— 跳过去并落在这一版"
                    onClick={() => onGoto('params', { machineId: m.id, uid: pickedNode.uid, key: null })}
                  >
                    编辑参数
                  </button>
                </div>
              </div>

              {/* ③ 配方 —— 状态是配方的属性；生成本来就该在这一页做完 */}
              <div className={s.sect}>
                <div className={s.sectHead}>③ 配方</div>
                <div className={s.recipeState}>
                  <span className={`${s.tag} ${STATE_TAG[pickedNode.build]}`}>
                    {words.build[pickedNode.build].label}
                  </span>
                  <span className={s.cardNote}>{words.build[pickedNode.build].explain}</span>
                  {pickedNode.lastBuild !== null && (
                    <span className={s.cardNote}>上次生成 {localStamp(pickedNode.lastBuild)}</span>
                  )}
                </div>
                <div className={s.kv}>
                  <span className={s.kvKey}>产物名</span>
                  <span className={s.kvVal}>
                    <span className={s.mono}>{pickedRow?.mkpFile ?? '—'}</span>
                    <span className={s.cardNote}>按「机型-版本」由命名规则算出，不手填（G-0）</span>
                  </span>
                </div>
                {(() => {
                  const note = genNote(m.id, picked.id)
                  const canGen = note === '' && pickedNode.build !== 'noResources'
                  return (
                    <>
                      <div className={s.sectOps}>
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm} ${s.btnPrimary}`}
                          disabled={!canGen}
                          title={canGen ? `生成 ${pickedRow?.mkpFile ?? '预设'}` : note}
                          onClick={() => void doBuild(m.id, picked.id)}
                        >
                          {pickedNode.build === 'stale' ? '重新生成' : '生成配方'}
                        </button>
                        {dirty && canGen && (
                          <button
                            type="button"
                            className={`${s.btn} ${s.btnSm}`}
                            title="先落盘，再生成 —— 顺序在这里是规定死的"
                            onClick={() => void saveAndBuild(m.id, picked.id)}
                          >
                            保存并生成
                          </button>
                        )}
                      </div>
                      {!canGen && note !== '' && (
                        <p className={s.note} style={{ marginTop: 8 }}>
                          {note}
                        </p>
                      )}
                    </>
                  )
                })()}
              </div>

              {/* ④ 关联 */}
              <div className={s.sect}>
                <div className={s.sectHead}>④ 关联</div>
                <div className={s.kv}>
                  <span className={s.kvKey}>套餐</span>
                  <span className={s.kvVal}>
                    {picked.recommendedBundle ? (
                      <>
                        <button
                          type="button"
                          className={s.chip}
                          title="去套餐页看它 —— 那一页是管理套餐内容的地方"
                          onClick={() => onGoto('bundles', { machineId: m.id, uid: null, key: picked.recommendedBundle })}
                        >
                          {picked.recommendedBundle}
                        </button>
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          onClick={() => {
                            setBunOpen(vidOf(m.id, picked.id))
                            setBunPick(picked.recommendedBundle ?? '')
                          }}
                        >
                          更换套餐…
                        </button>
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          onClick={() => void saveVersion(picked.id, 'recommendedBundle', null)}
                        >
                          取消关联
                        </button>
                      </>
                    ) : (
                      <>
                        <span className={s.kvDim}>{placeholderText.noBundle}</span>
                        <button
                          type="button"
                          className={`${s.btn} ${s.btnSm}`}
                          onClick={() => {
                            setBunOpen(vidOf(m.id, picked.id))
                            setBunPick('')
                          }}
                        >
                          选择套餐…
                        </button>
                      </>
                    )}
                  </span>

                  <span className={s.kvKey}>套餐内容</span>
                  <span className={s.kvVal}>
                    {(() => {
                      const b = bundles.bundles.find((x) => x.id === picked.recommendedBundle)
                      if (!b)
                        return (
                          <span className={s.kvDim}>还没选套餐 —— 切片器和 MKP 都放在套餐里，先选一个</span>
                        )
                      const mkp = b.assetRefs.filter((r) => !r.isBbs).length
                      const bbs = b.assetRefs.filter((r) => r.isBbs).length
                      return (
                        <span className={s.cardNote}>
                          {b.display || b.id} · MKP 预设 {mkp} · 切片器 {bbs}
                          {b.assetRefs.some((r) => !r.resolvable) && ' · 有引用解析不到（检查页会报）'}
                          {' —— 内容的增删在套餐页（P4）'}
                        </span>
                      )
                    })()}
                  </span>
                </div>
              </div>

              {/* 下一步：一次只给一颗按钮 */}
              <div className={s.sect}>
                <div className={s.sectHead}>下一步</div>
                <div className={s.steps}>
                  <span className={s.step} data-done={!dirty}>
                    ① 身份
                  </span>
                  <span className={s.step} data-done={picked.hasRecipe}>
                    ② 参数
                  </span>
                  <span className={s.step} data-done={pickedNode.build === 'built'}>
                    ③ 配方
                  </span>
                  <span className={s.step} data-done={!!picked.recommendedBundle}>
                    ④ 套餐
                  </span>
                </div>
                {nextStep && (
                  <div className={s.sectOps}>
                    <button
                      type="button"
                      className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                      onClick={nextStep.run}
                    >
                      下一步：{nextStep.label}
                    </button>
                  </div>
                )}
                {nextStep?.hint && <p className={s.note}>{nextStep.hint}</p>}
              </div>
            </div>
          </div>
        )}
        </>
        )}
      </div>

      <ContextMenu at={menu.at} entries={entries} onClose={menu.close} />

      {/* —— 选择套餐 —— */}
      <ModalC14
        open={bunOpen !== null}
        title={`选择套餐 · ${bunOpen ?? ''}`}
        subtitle="一个版本只记得一个套餐 —— 选完立刻写进机型文件（没有草稿）"
        size="md"
        closeOnScrim={false}
        onClose={() => setBunOpen(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setBunOpen(null)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!bunPick}
              title={bunPick ? undefined : '先在下面挑一个套餐'}
              onClick={() => {
                if (bunOpen === null || !bunPick) return
                const [, vid] = bunOpen.split('/')
                void saveVersion(vid, 'recommendedBundle', bunPick)
                setBunOpen(null)
              }}
            >
              关联
            </button>
          </>
        }
      >
        <div className={s.bunList}>
          {/*
           * **不限机型**（作者 2026-10-03：「不应该限制」）：一版一套的约束只有一条
           * —— 一个版本只指一个套餐；套餐可以被任何版本的指向。
           * 本机型的排前面（常用），别家的排后面、行上带归属标注。
           */}
          {[
            ...bundles.bundles.filter((b) => b.machineId === m.id),
            ...bundles.bundles.filter((b) => b.machineId !== m.id),
          ].map((b) => (
            <button
              key={b.id}
              type="button"
              className={`${s.row} ${bunPick === b.id ? s.rowPick : ''}`}
              onClick={() => setBunPick(b.id)}
            >
              <span className={`${s.mono} ${s.rowMeta}`}>{b.id}</span>
              <span className={s.rowName}>{b.display || '—'}</span>
              <span className={s.rowMeta}>
                {b.machineId !== m.id && `归属 ${b.machineId} · `}
                MKP {b.assetRefs.filter((r) => !r.isBbs).length} · 切片器{' '}
                {b.assetRefs.filter((r) => r.isBbs).length}
              </span>
              {b.id === picked?.recommendedBundle && <span className={`${s.tag} ${s.tagGhost}`}>当前</span>}
            </button>
          ))}
          {!bundles.bundles.length && (
            <p className={s.note} style={{ margin: 0 }}>
              还没有任何套餐 —— 套餐页的「新建套餐」是建它的地方。
            </p>
          )}
        </div>
      </ModalC14>

      {/* —— 品牌图（资产 id；与机型图 / 图标同一个选择器） —— */}
      {curBrand !== undefined && (
        <AssetPicker
          open={logoOpen}
          title={`选择品牌图 · ${curBrand.id}`}
          options={imageOptions}
          value={curBrand.logo}
          onCancel={() => setLogoOpen(false)}
          onPick={(next) => {
            void saveBrand('logo', next)
            toasts.push(next ? `品牌图指到资产 ${next}` : '已清空品牌图（客户端回落内置字标）')
            setLogoOpen(false)
          }}
        />
      )}

      {/* —— 新增品牌（2026-10-03）：id + 显示名；品牌图后配 —— */}
      <ModalC14
        open={addBrandOpen}
        title="新增品牌"
        subtitle="照 wb_add_brand(id, name) —— 往 presets/brands.toml 的 [[brands]] 里加一段并立刻写入"
        size="sm"
        closeOnScrim={false}
        onClose={() => setAddBrandOpen(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setAddBrandOpen(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={nbId.trim() === '' || nbName.trim() === ''}
              title={
                nbId.trim() === ''
                  ? 'id 不能空'
                  : nbName.trim() === ''
                    ? '显示名不能空'
                    : undefined
              }
              onClick={() => {
                void (async () => {
                  try {
                    setList(await wb.addBrand(nbId.trim(), nbName.trim()))
                    toasts.push(`已新增品牌 ${nbId.trim()} —— presets/brands.toml 已写入`)
                    setAddBrandOpen(false)
                    setBrandId(nbId.trim())
                    onBookRefresh()
                  } catch (e) {
                    failToast(e)
                  }
                })()
              }}
            >
              新增品牌
            </button>
          </>
        }
      >
        <label className={s.kv} style={{ display: 'grid' }}>
          <span className={s.kvKey}>品牌 id</span>
          <input
            className={s.inp}
            value={nbId}
            placeholder="Bambu Lab"
            aria-label="品牌 id"
            onChange={(e) => setNbId(e.target.value)}
          />
          <span className={s.inpHint}>
            id 是机型「品牌」那一格引用的东西（真数据里叫 `Bambu Lab`，带空格与大小写都可以）
          </span>
          <span className={s.kvKey}>显示名</span>
          <input
            className={s.inp}
            value={nbName}
            placeholder="给人看的名字，例如 拓竹 (Bambu Lab)"
            aria-label="品牌显示名"
            onChange={(e) => setNbName(e.target.value)}
          />
        </label>
        <p className={s.note}>
          撞名（含只差大小写）由后端当场拒。新建的品牌还没有品牌图 ——
          建完在右边那张卡上配（留空 = 客户端回落内置字标）。
        </p>
      </ModalC14>

      {/* —— 机型尺寸（六组 + 复制标定点）—— */}
      <MachineDimensionsModal
        machineId={dimsOpen}
        machines={list.machines}
        dimensions={list.machines.find((x) => x.id === dimsOpen)?.dimensions ?? null}
        onClose={() => setDimsOpen(null)}
        onSaved={setList}
      />

      {/* —— 禁区编辑器 —— */}
      <ZoneEditorModal
        machineId={zoneOpen}
        machine={list.machines.find((x) => x.id === zoneOpen) ?? null}
        onClose={() => setZoneOpen(null)}
        onSaved={setList}
      />

      {/* —— 移到品牌…（机型行右键）—— */}
      <ModalC14
        open={moveOf !== null}
        title={`移到品牌 · ${moveOf ?? ''}`}
        subtitle="移动只改这台机器机型文件里的 brand 一格 —— 品牌侧那份名单是反查，不用同步"
        size="sm"
        closeOnScrim={false}
        onClose={() => setMoveOf(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setMoveOf(null)}>
              取消
            </button>
          </>
        }
      >
        <div className={s.pickTree}>
          {(list?.brands ?? []).map((b) => {
            const here =
              list?.machines.find((x) => x.id === moveOf)?.brand.trim().toLowerCase() ===
              b.id.trim().toLowerCase()
            return (
              <button
                key={b.id}
                type="button"
                className={`${s.row} ${s.treeKid} ${here ? s.treeKidOn : ''}`}
                disabled={here}
                title={here ? '它现在就在这家' : `挪到 ${b.name || b.id}`}
                onClick={() => {
                  if (moveOf !== null) void moveMachine(moveOf, b.id)
                }}
              >
                <span className={s.rowName}>{b.name || b.id}</span>
                <span className={`${s.mono} ${s.rowMeta}`}>{b.id}</span>
                <span className={s.rowMeta}>{b.machines.length} 台机型</span>
                {here && <span className={s.rowMeta}>就在这儿</span>}
              </button>
            )
          })}
        </div>
        <p className={s.note}>
          移到别的品牌下之后，这台机器在左树里就挂在那一棵下面 ——
          归属只有一份（机型文件的 brand 一格），不产生第二份名单。
        </p>
      </ModalC14>

      {/* —— 素材选择器：机型图 / 图标 / 版本图（都是资产 id，只从库里挑） —— */}
      <AssetPicker
        open={imgOpen}
        title={`选择机型图 · ${m.id}`}
        options={imageOptions}
        value={m.image}
        onCancel={() => setImgOpen(false)}
        onPick={(next) => {
          void saveMachine('image', next)
          toasts.push(next ? `机型图指到资产 ${next}` : '已清空机型图')
          setImgOpen(false)
        }}
      />
      {picked !== undefined && (
        <AssetPicker
          open={verImgOpen}
          title={`选择版本图 · ${m.id} / ${picked.id}`}
          options={imageOptions}
          value={picked.image}
          onCancel={() => setVerImgOpen(false)}
          onPick={(next) => {
            void saveVersion(picked.id, 'image', next)
            toasts.push(next ? `这一版的图指到资产 ${next}` : '已清空版本图（回落机型图）')
            setVerImgOpen(false)
          }}
        />
      )}
      <AssetPicker
        open={iconOpen}
        title={`选择图标 · ${m.id}`}
        options={iconOptions}
        value={m.icon}
        onCancel={() => setIconOpen(false)}
        onPick={(next) => {
          void saveMachine('icon', next)
          toasts.push(next ? `图标指到资产 ${next}` : '已清空图标')
          setIconOpen(false)
        }}
      />

      {/* —— 新增机型 —— */}
      <ModalC14
        open={addOpen}
        title="新增机型"
        subtitle="照 wb_add_machine(id, brand, display) —— 会新建一个 presets/machines/ 下的文件并立刻写入"
        size="sm"
        closeOnScrim={false}
        onClose={() => setAddOpen(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setAddOpen(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!addReady}
              title={
                !idOk(addId) && addId.trim() !== ''
                  ? 'id 只能用大写字母、数字和下划线（它会直接变成文件名）'
                  : addIdTaken
                    ? '已经有一台叫这个的机型了'
                    : addReady
                      ? undefined
                      : '三格都要填（后端逐格校验）'
              }
              onClick={() => void submitAddMachine()}
            >
              新建
            </button>
          </>
        }
      >
        <label className={s.kv} style={{ display: 'grid' }}>
          <span className={s.kvKey}>
            机型 id
            <FieldMark />
          </span>
          <input
            className={s.inp}
            value={addId}
            placeholder="A2L"
            aria-label="机型 id"
            onChange={(e) => setAddId(e.target.value.toUpperCase())}
          />
          {/* 拦着新建的理由当场写在输入框下面 —— 只放禁用按钮的悬停里，人只能猜 */}
          {addId.trim() !== '' && !idOk(addId) && (
            <span className={s.inpHint}>id 只能用大写字母、数字和下划线 —— 例如 A2L、A1_MINI</span>
          )}
          {addIdTaken && <span className={s.inpHint}>id「{addId.trim()}」已经被占了 —— 换一个</span>}
          <span className={s.kvKey}>
            品牌
            <FieldMark later />
          </span>
          <PickOrType
            label="品牌"
            options={brandOptions}
            value={addBrand}
            emptyLabel="选一个品牌…"
            allowCustom={false}
            onChange={setAddBrand}
          />
          <span className={s.kvKey}>
            显示名
            <FieldMark later />
          </span>
          <input
            className={s.inp}
            value={addDisplay}
            placeholder="给人看的名字，例如 A1 mini"
            aria-label="显示名"
            onChange={(e) => setAddDisplay(e.target.value)}
          />
        </label>
        <p className={s.note}>
          <strong>红星只留给 id，橙星是「可以后补的必填」</strong> —— 不过这一步三格都得填：
          真后端逐格校验，空着建不出来。新建的机型还没有尺寸（占位机型），
          没版本、不参与交付 —— 检查与生成页会给一条说明而不是报错。
          <br />
          同名文件已存在时不会被覆盖，后端会直接报错。
        </p>
      </ModalC14>

      {/* —— 新增版本 —— */}
      <ModalC14
        open={addVOpen}
        title={`新增版本 · ${m.id}`}
        subtitle="照 wb_add_version(machineId, id, name) —— 写进机型文件，立刻生效"
        size="sm"
        closeOnScrim={false}
        onClose={() => setAddVOpen(false)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setAddVOpen(false)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!avReady}
              title={
                avTaken
                  ? '这台机型已经有这个版本 id 了'
                  : !idOk(avId) && avId.trim() !== ''
                    ? '版本 id 只能用大写字母、数字和下划线'
                    : !avName.trim()
                      ? '版本名不能空着'
                      : undefined
              }
              onClick={() => void submitAddVersion()}
            >
              新建
            </button>
          </>
        }
      >
        <div className={s.kv} style={{ display: 'grid' }}>
          <span className={s.kvKey}>
            版本 id
            <FieldMark />
          </span>
          <input
            className={s.inp}
            value={avId}
            placeholder="STANDARD"
            aria-label="版本 id"
            onChange={(e) => setAvId(e.target.value.toUpperCase())}
          />
          <span className={s.kvKey}>
            版本名
            <FieldMark />
          </span>
          <input
            className={s.inp}
            value={avName}
            placeholder="标准版"
            aria-label="版本名"
            onChange={(e) => setAvName(e.target.value)}
          />
        </div>
        <p className={s.note}>
          <strong>id 与版本名都得现在填</strong>：版本一建出来就直接参与生成，
          没名字的版本在检查页和生成页都认不出来。
          <br />
          新版本的标签、说明、套餐还是空的，参数纯继承机型基底（卡片上会标
          「参数源待补」）—— 套餐去版本详情卡指，参数去参数台改。
        </p>
      </ModalC14>

      {/* —— 复制版本（14.3 / 14.5） —— */}
      <ModalC14
        open={copyOpen !== null}
        title={`复制版本 · ${copyOpen ?? ''}`}
        subtitle="两条命令各自只写单文件：先版本定义，勾了「同时复制配方」再拷参数正文"
        size="md"
        closeOnScrim={false}
        onClose={() => setCopyOpen(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setCopyOpen(null)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!copyReady}
              title={
                copyTaken
                  ? '这台机型已经有这个版本 id 了'
                  : idOk(copyId) || copyId.trim() === ''
                    ? undefined
                    : '版本 id 只能用大写字母、数字和下划线'
              }
              onClick={() => void submitCopy()}
            >
              复制
            </button>
          </>
        }
      >
        <label className={s.kv} style={{ display: 'grid' }}>
          <span className={s.kvKey}>
            新版本 id
            <FieldMark />
          </span>
          <input
            className={s.inp}
            value={copyId}
            aria-label="新版本 id"
            onChange={(e) => setCopyId(e.target.value.toUpperCase())}
          />
          <span className={s.kvKey}>版本名</span>
          <input
            className={s.inp}
            value={copyName}
            aria-label="新版本名"
            placeholder="给人看的名字"
            onChange={(e) => setCopyName(e.target.value)}
          />
        </label>
        <label className={s.refRow} style={{ marginTop: 12 }}>
          <input type="checkbox" checked={withRecipe} onChange={(e) => setWithRecipe(e.target.checked)} />
          <span>
            <b>同时复制配方</b>
            <span className={s.cardNote} style={{ display: 'block' }}>
              后端是两个独立命令：<code className={s.mono}>wb_copy_version</code> 只写版本定义，
              <code className={s.mono}>wb_copy_recipe</code> 才拷参数值，而且拷出来是
              <strong>独立快照</strong> —— 之后改模板、改基底都传不到它身上。
              不勾就是一个纯继承基底的空壳版本（标「参数源待补」）。
            </span>
          </span>
        </label>
      </ModalC14>

      {/* —— 编辑版本（name / tag / description 三格一次提交） —— */}
      <ModalC14
        open={editVOpen !== null}
        title={`编辑版本 · ${editVOpen ?? ''}`}
        subtitle="name / tag / description —— 对齐 wb_set_version_field 的白名单；套餐在 ④ 关联里改"
        size="md"
        closeOnScrim={false}
        onClose={() => setEditVOpen(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setEditVOpen(null)}>
              取消
            </button>
            <button
              type="button"
              className={`${s.btn} ${s.btnPrimary}`}
              disabled={!evName.trim()}
              title={evName.trim() ? undefined : '版本名不许清空'}
              onClick={() => void submitEditVersion()}
            >
              保存
            </button>
          </>
        }
      >
        <label className={s.kv} style={{ display: 'grid' }}>
          <span className={s.kvKey}>
            版本名
            <FieldMark />
          </span>
          <input className={s.inp} value={evName} aria-label="版本名" onChange={(e) => setEvName(e.target.value)} />
          <span className={s.kvKey}>标签</span>
          <input
            className={s.inp}
            value={evTag}
            aria-label="标签"
            placeholder="可空"
            onChange={(e) => setEvTag(e.target.value)}
          />
          <span className={s.kvKey}>描述</span>
          <input
            className={s.inp}
            value={evDesc}
            aria-label="描述"
            placeholder="可空"
            onChange={(e) => setEvDesc(e.target.value)}
          />
        </label>
        <p className={s.note}>
          版本 id 不在这里改 —— id 是身份，改它等于删掉再加一个（后端还没有这一步）。
          一次提交三格：一格失败就停，不会只成功一半。
        </p>
      </ModalC14>

      {/* —— 删除版本（两步：先问孤儿，再确认；确认后立刻落盘，不可逆） —— */}
      <ModalC14
        open={del !== null}
        title={`删除版本 · ${del?.vid.split('/')[1] ?? ''}`}
        subtitle="先查孤儿引用，再确认 —— 确认之后立刻写入，没有回收站也没有撤销"
        size="md"
        onClose={() => setDel(null)}
        footer={
          <>
            <span className={s.grow} />
            <button type="button" className={s.btn} onClick={() => setDel(null)}>
              不删了
            </button>
            {del?.orphans !== null ? (
              <button type="button" className={`${s.btn} ${s.btnDanger}`} onClick={() => void submitDelete()}>
                确认删除
              </button>
            ) : (
              <span className={s.cardNote}>正在查孤儿引用…</span>
            )}
          </>
        }
      >
        {del === null ? null : del.orphans === null ? (
          <p className={s.note} style={{ margin: 0 }}>
            正在查「删掉这一版会让哪些字段留下孤儿引用」……
          </p>
        ) : (
          <>
            <p className={s.note} style={{ margin: 0 }}>
              要删掉 <b>{m.id}/{del.vid.split('/')[1]}</b>。
              <strong>立刻写入文件，没有回收站也没有撤销。</strong>
            </p>
            {del.orphans.length > 0 ? (
              <p className={s.note}>
                删掉之后这 {del.orphans.length} 项会留下指向它的<strong>孤儿引用</strong>
                （不报错，但那几项在这台机器上会悄悄不生效）：
                <br />
                <code className={s.mono}>{del.orphans.join('、')}</code>
              </p>
            ) : (
              <p className={s.note}>没有任何字段引用这一版，删掉不会留下孤儿。</p>
            )}
            <p className={s.note}>
              <strong>会一起消失的</strong>：版本本身、标签与说明、它对套餐的指向、
              已生成的产物记录，以及它自己钉过的 {pickedNode?.items ?? 0} 项参数值。
              <br />
              <strong>一个都不动的</strong>：这台机型、别的版本、套餐定义、资产库里的文件。
            </p>
          </>
        )}
      </ModalC14>
      {rail.handle}
    </div>
  )
}
