/*
 * 标注台（2026-10-16，作者提的）—— 在图上**自己拖框**。
 *
 * # 为什么有这一页（作者的话）
 *
 * 「你要不做一个小工具吧，在工作台里面……然后让我可以去自己拖动框框。然后就是要有那种
 *   图层的感觉啊……就做成像你这样子，有一有二的，在左上角。」
 *
 * 图片引导那两张截图（`presets/assets/guides/`）上的框，此前是我拿网格量出来的一串
 * 百分比 —— 量偏一次就得再来一轮（实测就偏了：框落在了下拉下面那一行）。这一页把
 * 「量」变成「拖」：图按原尺寸摆上来、每 50px 一条网格、框直接拖/拉/改数，
 * 导出就是 `openWithGuide.ts` 里那两行 `marks`。
 *
 * # 三条与客户端对齐的规矩
 *
 * 1. **标注是百分比**（`GuideMark.at`，相对整张图）；这一页里存、算、导出都按它走，
 *    面板上另给一份 px 读数（整数，方便对着截图报数）。
 * 2. **浅 / 深两张共用一份**：框只跟"图上的位置"有关 —— 所以切图不切框；
 *    两张必须同尺寸同裁切（不同裁切的话换图就会飘，切过去时给一句提醒）。
 * 3. **画出来的样子就是客户端的样子**：2px 强调色 + 圆角 8 + 左上角编号圆点
 *    （与 `GuideImage.module.css` 的 `.mark` / `.badge` 同值）；「客户视角」按钮把图
 *    缩到客户端那个宽度（`.frame` = 314px）—— 所见即所得。
 *
 * # 存哪儿：**源稿进仓库**（2026-10-16 作者改的口径）
 *
 * > 「那个过程就是标注的那个过程，那个源文件吧，就像 PS 里面的那种源文件呢，可以再次编辑的
 * >   那一种……而且我希望它是**随着这个项目仓库**的，而不是说到我本地的。这样，我换了设备
 * >   也可以继续编辑生成新的图片」
 *
 * ⇒ **能再编辑的那一份住在图旁边**：`presets/assets/<同类目录>/<对>.anno.json`
 *   （见 Rust 侧 `workbench/app/annotation.rs` 的头注；发布时**排除**，客户端看不见它）。
 *   进页读、`写入仓库` 写（`wb_anno_read` / `wb_anno_write`）。
 *
 * localStorage（`mkp.wb.annotate`）**退成"没写进仓库的暂存"**：编辑时随手存一份（掉电不丢），
 * 保存成功后清掉。打开时若本机那份比仓库那份**新**（或仓库还没有），面板上会挑明
 * 「用本机那份 / 丢掉它」——两条路都不猜、不静默覆盖。
 *
 * ★ 这个文件只给**工作台**用；客户端读的是 `openWithGuide.ts` 里导出的 `marks`
 *   （点「复制 TS」贴回去，两边一起看）。
 *
 * # 可复用的那几样（都从 `ZoneEditorModal` 学的）
 *
 * 指针拖动 = `pointerdown` 记起点快照 + `setPointerCapture`，`pointermove` 只算增量；
 * 「第一次真动」才把起点压进撤销栈（防一次点击留一条空撤销）；键盘也能干（方向键挪、
 * Delete 删、⌘Z / ⇧⌘Z）。
 */

import { useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from 'react'
import type { CSSProperties } from 'react'

import { assetUrl, isAppError, wb, type AnnoDoc, type AnnoStyle, type AssetView } from '../api'
import { assetsBus } from '../assetsBus'
import { toasts } from '../c14/toast'
import c from '../c14.module.css'
import { usePageActions } from '../pageActions'
/* 箭头一律是客户端那一颗形状（`shell/icons` 的 chevronDown + `ui` 的两个转向类），
   工作台不再自己排「▸」字形 —— 字形转 90° 会被字体度量拉一下 */
import Icon from '../../app/shell/icons'
import ui from '../../app/ui/ui.module.css'
import s from './AnnotatePage.module.css'

/** 网格步长：作者量坐标时用的就是「每 50px 一条线」 */
const GRID = 50
/** 客户端里那张图的显示宽度（`GuideImage.module.css` 的 `.shot` 的 `max-width`）—— 下面那个参考比例按它算 */
const CLIENT_W = 314

/**
 * 成品图的**参考显示比例** = 客户端把这张图显示多大（`CLIENT_W / 图宽`）。
 *
 * 成品图上的框 / 编号就按**这个比例**定尺寸（图上 = `屏幕尺寸 / 参考比例` 像素）——
 * 客户端按 314px 显示它时，框正好是设计那 2px。见 `render.rs` 的 `render_webp`。
 *
 * ★★★ **与当前缩放无关**：同一份标注，不管在哪个视角按「保存」/「生成预览」，烧出来
 * 一模一样（旧写法把"当次缩放"传下去 ⇒ 42% 保存的成品图拿到 51% 看就粗一大截 —— 那正是
 * 作者 2026-10-16 点名的「客户视角明显和在看效果差很远」）。
 *
 * ★ 画布上那份框也按它对齐（见 `.board` 的 `--zoom-k`）：于是画布在任何缩放下看到的
 * 都是"成品图被放大后的像素"，两边零差别。
 */
const refScale = (imgW: number) => (imgW > 0 ? CLIENT_W / imgW : 1)
/** 框的最小边长（图的原生 px）：拖成 0 宽就再也抓不住了 */
const MIN_SIDE = 4
/** 撤销栈上限（与禁区编辑器同一档） */
const HIST_MAX = 49
const STORE_KEY = 'mkp.wb.annotate'
const CIRCLED = '①②③④⑤⑥⑦⑧⑨⑩⑪⑫⑬⑭⑮⑯⑰⑱⑲⑳'.split('')

type Zoom = 'fit' | number
/** 拖的是哪条边（'' = 整框平移）。`new` 那个阶段按 `se` 走 */
type Edge = '' | 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw'

/** 一个框。`at` 是**百分比**（与 `GuideMark.at` 同形）—— 存、算、导出都是它 */
interface Mark {
  /** 界面用的一次性 id（不进源稿 —— 源稿靠 `step` 认） */
  id: string
  at: { left: number; top: number; width: number; height: number }
  /** 给自己看的名字：**只进导出的注释**，不进契约 */
  note: string
  /** 图层里关掉的那一格：不画在图上，但留着 */
  off: boolean
  /** 画出来什么样（缺省 = 站点缺省：圆角 8 / 线宽 2 / 编号 18）；**随源稿一起存** */
  style?: AnnoStyle
}

interface Source {
  /** 存 localStorage 的键（资产 id / `file:<名字>`） */
  key: string
  /**
   * 它属于哪个分组（**台账 `[[groups]]` 的组 id**，后端算好发过来）。
   * `null` = 没进组 ⇒ 它自己一份源稿（走旧口径兜底）。见 [`groupKeyOf`]
   */
  groupId: string | null
  /** 组名（界面显示"套装"用）；没进组 = null */
  groupName: string | null
  label: string
  /** 图相对**资产源根**的那一段（`presets/assets/<path>`），底下那行小字用它 */
  path: string
  url: string
  /**
   * **成品图**的地址（台账那条 `path`，框已烧进像素）。「客户视角」用它 ——
   * 平时标注台显示的是**原图**（`url`，见后端 `annotation::source_image_rel_of`），
   * 只有切到客户视角才看成品。`null` = 这条没有成品（还没渲过 / 不是引导图）。
   */
  renderedUrl: string | null
  /** 原生尺寸；图还没 load 出来时是 0 */
  w: number
  h: number
}

interface Saved {
  marks: Mark[]
  zoom: Zoom
  grid: boolean
  snap: boolean
  /** 存这份草稿时那张图的原生尺寸 —— 换图进来对一下，不一样就提醒（框是百分比，会飘） */
  size?: { w: number; h: number }
  /** 这份暂存是什么时候落的（毫秒）—— 与仓库那份的 `updatedAt` 比"谁后"，见 `conflict` */
  savedAt: number
}

/** 读回来的仓库那份（`path` 是它在资产根里的落点，面板上要显示） */
interface RepoStamp {
  path: string
  doc: AnnoDoc | null
}

interface Drag {
  mode: 'move' | 'resize' | 'new'
  id: string
  edge: Edge
  /** 按下时的 client 坐标 */
  cx: number
  cy: number
  /** 按下前那一份（第一次真动时进撤销栈） */
  before: Mark[]
  /** 那一框按下时的样子 */
  orig: Mark
  pushed: boolean
  /**
   * 画新框专用：这一框**落笔了没**（进了 `marks` 没）。
   * 空白处按下时 `false` —— 拖过阈值才置 `true` 并把框塞进 `marks`；
   * 没动就抬手 = 一次点击（点空白收编辑区），**不留下 1×1 的小框**。
   */
  created?: boolean
}

/**
 * **一组**图共用一份草稿 —— 这一页的整套前提就是「一份标注、这一组图共用」。
 *
 * ★ 2026-10-16 起"谁跟谁一组"**读台账的 `[[groups]]`**（`AssetView.groupId` 由后端算好），
 * 不再靠抹文件名。见 [`groupKeyOf`]。
 * ★ 一组可以是 **N 张**（将来 macOS + Windows 同一套引导），浅 / 深只是 N=2 的特例；
 * 源稿的 `source.images` 记的就是这一组的全部成员。
 * 尺寸不一样时另给一句提醒（见 `size` 那格）—— 尺寸不同、百分比就落不到同一处。
 */

/**
 * **这一张属于哪一组**（本机暂存的键、以及"是不是同一组"的判据）：
 * 有登记的分组 → 组 id；没进组 → 用**它自己**（各自一份源稿）。
 *
 * 与后端 `Assets::anno_stem_of` 同一个意思（那边还多一层"旧口径抹浅深后缀"的兜底）
 * —— 前端只需要一个稳定的键，落点由后端给。
 */
const groupKeyOf = (src: Source) => src.groupId ?? src.key

/**
 * 旧口径的兜底：把 id 里 `light` / `dark`（或「浅色」「深色」）抹成通配。
 *
 * ★ 只有**没进任何组**的图才走它（`guide-open-with-*` 现在都在 `open-with` 组里，
 * 不走这条了）。留着是为了"没建组的图也能按老习惯配上对"。
 */
const pairOf = (key: string) => key.replace(/(light|dark|浅色|深色)/gi, '·')

const uid = () => `m${Math.random().toString(36).slice(2, 8)}`

/** 成品图该叫什么（`guides/open-with-light.png` ⇒ `open-with-light.rendered.webp`）——
 *  用来在「生成预览」回的那几张里挑出"当前这张资产"对应的那张。 */
const renderedNameOf = (assetPath: string) =>
  `${(assetPath.split('/').pop() ?? '').replace(/\.[^.]+$/, '')}.rendered.webp`

/** base64 → Blob（预览那张 webp；IPC 只能传 JSON，所以字节走 base64 回来） */
function b64ToBlob(b64: string, type: string): Blob {
  const bin = atob(b64)
  const buf = new Uint8Array(bin.length)
  for (let i = 0; i < bin.length; i += 1) buf[i] = bin.charCodeAt(i)
  return new Blob([buf], { type })
}
const round1 = (v: number) => Math.round(v * 10) / 10
const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), hi)

/** 百分比 → 图的原生 px */
const pxOf = (m: Mark, w: number, h: number) => ({
  x: (m.at.left / 100) * w,
  y: (m.at.top / 100) * h,
  w: (m.at.width / 100) * w,
  h: (m.at.height / 100) * h,
})

/** 图的原生 px → 百分比。**小数一位都不丢**（导出时才 round1，拖动全程不累积误差） */
const atOf = (x: number, y: number, w: number, h: number, imgW: number, imgH: number): Mark['at'] => ({
  left: (x / imgW) * 100,
  top: (y / imgH) * 100,
  width: (w / imgW) * 100,
  height: (h / imgH) * 100,
})

/* ---------- localStorage（与 `useSplitWidth` 同一条规矩：坏值当没有） ---------- */

function readStore(): Record<string, Saved> {
  try {
    const raw = localStorage.getItem(STORE_KEY)
    const o: unknown = raw ? JSON.parse(raw) : null
    return o !== null && typeof o === 'object' ? (o as Record<string, Saved>) : {}
  } catch {
    return {}
  }
}

function writeStore(key: string, v: Saved) {
  try {
    localStorage.setItem(STORE_KEY, JSON.stringify({ ...readStore(), [key]: v }))
  } catch {
    /* 隐私模式 / 配额满：记不住不算错误 */
  }
}

/** 这一份进仓库了 ⇒ 本机暂存就没用了（留着只会制造"谁跟谁"的疑问） */
function clearStore(key: string) {
  try {
    const all = readStore()
    delete all[key]
    localStorage.setItem(STORE_KEY, JSON.stringify(all))
  } catch {
    /* 同上 */
  }
}

/* ---------- 源稿 ⇄ 这一页的界面态 ---------- */

/** 源稿里的一条 → 界面上的一个框（补上界面用的一次性 id 与"显不显示"） */
const markOf = (m: AnnoDoc['marks'][number]): Mark => ({
  id: uid(),
  at: { ...m.at },
  note: m.note ?? '',
  off: false,
  ...(m.style === undefined ? {} : { style: { ...m.style } }),
})

/** 界面上的框 → 源稿的一条（`step` 按当前次序重排：①②③ 就是列表里的顺序） */
const annoMarkOf = (m: Mark, i: number): AnnoDoc['marks'][number] => {
  const style = m.style
  const hasStyle =
    style !== undefined &&
    (style.radius !== undefined || style.stroke !== undefined || style.badge !== undefined)
  return {
    step: i + 1,
    at: {
      left: round1(m.at.left),
      top: round1(m.at.top),
      width: round1(m.at.width),
      height: round1(m.at.height),
    },
    ...(m.note.trim() === '' ? {} : { note: m.note.trim() }),
    ...(hasStyle ? { style } : {}),
  }
}

/**
 * 这一组图的**全部成员路径**（2026-10-16：成员由**台账的组**说了算）。
 *
 * ① 进了登记的组 ⇒ 组里那些成员（N 张，浅 / 深是 N=2 的特例，将来 macOS + Windows 同组也走它）；
 * ② 没进组 ⇒ 旧口径兜底（`pairOf` 相同的那些，也就是"按老习惯配上对的那几张"）。
 *
 * 台账里查不到伙伴时（本地文件、台账漏了那张）兜一个"就它自己" —— 源稿仍然成立，
 * 只是这一组暂时只有一张。
 */
function groupPathsOf(src: Source, assets: AssetView[]): string[] {
  const bucket = groupKeyOf(src)
  const same = (a: AssetView) => (a.groupId ?? pairOf(a.id)) === bucket
  const mem = assets.filter(same).map((a) => a.path)
  /* 当前这张一定要在里面（本机文件 / 台账里没有它时） */
  return mem.includes(src.path) ? mem : [...mem, src.path]
}

/** 界面态 → 一份源稿（`updatedAt` 留空：真值由后端盖章，前端不造时间） */
const docOf = (marks: Mark[], src: Source, assets: AssetView[]): AnnoDoc => ({
  annoSchema: 1,
  source: { images: groupPathsOf(src, assets), size: { w: src.w, h: src.h } },
  marks: marks.map(annoMarkOf),
  updatedAt: '',
})

/** 只在"框变了没"这个意义上做比较（尺寸/路径随换图变，那些不算改动） */
const marksSig = (marks: Mark[]) => JSON.stringify(marks.map(annoMarkOf))

/* ---------- 导出 / 导入 ---------- */

const jsonOf = (marks: Mark[]) => JSON.stringify(marks.map(annoMarkOf), null, 2)

/** 贴回 `openWithGuide.ts` 的那一段（`style` 只在真的设过时才写出来） */
const tsOf = (marks: Mark[]) =>
  [
    'marks: [',
    ...marks.map((m, i) => {
      const row = annoMarkOf(m, i)
      const note = row.note === undefined ? '' : ` ${row.note}`
      const style =
        row.style === undefined
          ? ''
          : `, style: { ${Object.entries(row.style)
              .map(([k, v]) => `${k}: ${String(v)}`)
              .join(', ')} }`
      return (
        `  /* ${CIRCLED[i] ?? `#${i + 1}`}${note} */\n` +
        `  { step: ${row.step}, at: { left: ${row.at.left}, top: ${row.at.top}, ` +
        `width: ${row.at.width}, height: ${row.at.height} }${style} },`
      )
    }),
    '],',
  ].join('\n')

/** 解析导出的 JSON（也认"只有 at 四个数"那种写法：`[{left,top,width,height}]`） */
function parseMarks(text: string): Mark[] {
  const raw: unknown = JSON.parse(text)
  if (!Array.isArray(raw)) throw new Error('要一个数组（导出的那份 JSON 就行）')
  return raw.map((row, i) => {
    const o = (row ?? {}) as Record<string, unknown>
    const at = (typeof o.at === 'object' && o.at !== null ? o.at : o) as Record<string, unknown>
    const num = (v: unknown): number => {
      const n = typeof v === 'string' ? Number(v) : v
      if (typeof n !== 'number' || !Number.isFinite(n)) throw new Error(`第 ${i + 1} 个框的坐标读不出来`)
      return n
    }
    /* 备注与样式认多少算多少（导出的那一份带着它们，贴回来要能接着编） */
    const style: AnnoStyle = {}
    const raw = (typeof o.style === 'object' && o.style !== null ? o.style : {}) as Record<string, unknown>
    for (const k of ['radius', 'stroke', 'badge'] as const) {
      const v = raw[k]
      if (typeof v === 'number' && Number.isFinite(v)) style[k] = v
    }
    return {
      id: uid(),
      at: { left: num(at.left), top: num(at.top), width: num(at.width), height: num(at.height) },
      note: typeof o.note === 'string' ? o.note : '',
      off: false,
      ...(Object.keys(style).length === 0 ? {} : { style }),
    }
  })
}

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    /* 老 WebView / 非安全上下文：回落到 execCommand */
  }
  try {
    const ta = document.createElement('textarea')
    ta.value = text
    ta.style.position = 'fixed'
    ta.style.top = '0'
    ta.style.opacity = '0'
    document.body.appendChild(ta)
    ta.select()
    const ok = document.execCommand('copy')
    ta.remove()
    return ok
  } catch {
    return false
  }
}

const errText = (e: unknown) => (isAppError(e) ? `${e.message}${e.detail ? ` —— ${e.detail}` : ''}` : String(e))

const K_LABEL: Record<'left' | 'top' | 'width' | 'height', string> = {
  left: 'X',
  top: 'Y',
  width: '宽',
  height: '高',
}
const K_KEYS = ['left', 'top', 'width', 'height'] as const

/** 「框的样子」那三格：变量名与缺省值**与客户端逐字一致**（见 `.module.css` 头注） */
const STYLE_KEYS = [
  { key: 'radius', label: '圆角', def: 8 },
  { key: 'stroke', label: '线宽', def: 2 },
  { key: 'badge', label: '编号', def: 18 },
] as const

/** 一个框的样式 → 三个 CSS 变量（缺省的就不写，交给 CSS 里的 `var(…, 默认)`） */
function styleVars(style: AnnoStyle | undefined): CSSProperties {
  if (style === undefined) return {}
  const v: Record<string, string> = {}
  if (style.radius !== undefined) v['--mark-radius'] = `${style.radius}px`
  if (style.stroke !== undefined) v['--mark-stroke'] = `${style.stroke}px`
  if (style.badge !== undefined) v['--badge-size'] = `${style.badge}px`
  return v as CSSProperties
}

/*
 * 数字格的两条共用件（2026-10-16 作者：「我希望这个数字在左侧，我的那个光标也是在那个数字的
 * 右侧，这样方便我去删除呀、更改呀」）——
 *
 * 1. `onNumKeyDown`：**Enter 提交**（模糊 = 触发 onBlur 那条提交），并把按键**止在这里** ——
 *    这是第二道闸：即便页根那道来路判定哪天被绕开，数字格里的退格 / 删除也绝不会冒上去删框。
 * 2. `onNumFocus`：点进来就把光标**送到末尾**（数字靠左、光标在数字右侧）——
 *    `type=number` 在部分 WebView 上不支持 `setSelectionRange`，所以失败就退化成"全选"，
 *    效果一样是"进来直接能改"。
 */
const onNumKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
  e.stopPropagation()
  if (e.key === 'Enter') e.currentTarget.blur()
}
const onNumFocus = (e: React.FocusEvent<HTMLInputElement>) => {
  const el = e.currentTarget
  const n = el.value.length
  try {
    el.setSelectionRange(n, n)
  } catch {
    el.select()
  }
}

export default function AnnotatePage() {
  const [assets, setAssets] = useState<AssetView[] | null>(null)
  const [src, setSrc] = useState<Source | null>(null)
  const [marks, setMarks] = useState<Mark[]>([])
  const [sel, setSel] = useState<string | null>(null)
  const [zoom, setZoom] = useState<Zoom>('fit')
  const [grid, setGrid] = useState(true)
  const [snap, setSnap] = useState(false)
  /**
   * **视图档位**（2026-10-16 第二版）：`编辑 | 看效果 | 客户视角` —— **互斥的三档**，
   * 由工具栏那个切换器切（作者：「把在看效果和客户视角加一个切换器，我加切换器这个
   * 它的其他的都不变了」）。
   *
   * ★★★ 切档**只换显示内容**，绝不动 `zoom` / 位置 / 尺寸 —— 两档之间来回切时画面几何
   * 完全不变，才比得出"我画的框"与"烧进去的框"差在哪。旧写法是两颗**独立**的开关，
   * 而且进客户视角会强行 `setZoom(CLIENT_W / src.w)` ⇒ 一切大小就跳（作者点名的病）。
   *
   * - `edit`    = 原图 + 前端画的框（可拖）+ 网格 + 手柄
   * - `preview` = 原图 + 前端画的框（藏网格与手柄，只看画上去什么效果）
   * - `client`  = **成品图**（框已烧进像素，前端**不再叠画**）
   * - `none`    = **什么都不叠**：只看那张原图（作者 2026-10-17：「我可以再一次点击，
   *   就是那种反选的效果，就是什么都没有选、什么效果都没有，看原本的图片」）——
   *   拿它跟上面两档对着看，一眼比出"框画上去 / 烧进去 / 不画"三种样子。
   *
   * ★ **再点一次当前那一档 = 反选到 `none`**（切换器里那三颗的 onClick 就是这条）。
   */
  const [view, setView] = useState<'edit' | 'preview' | 'client' | 'none'>('edit')
  /* 下面那一大段老逻辑照旧读这几个布尔 —— 它们只是这一个档位的切片 */
  const preview = view === 'preview'
  const clientView = view === 'client'
  /** 画不画前端那些框（`none` = 一个都不画；`client` 也别画，成品图里已经烧进去了） */
  const showMarks = view === 'edit' || view === 'preview'
  /**
   * 成品图的**破缓存戳**（2026-10-16 作者：「按了保存之后，看客户视角，结果发现还是两个框框」）。
   *
   * 成品图地址是固定的（`guides/x.rendered.webp`），每次保存都**覆盖同一个文件** ——
   * 浏览器 / webview 会拿**上一次**缓存的那张 ⇒ 看着像"保存没生效"。保存成功后 +1，
   * 拼成 `?v=<n>` 强制重取。
   */
  const [renderStamp, setRenderStamp] = useState(0)
  /** 「生成预览」烘图中的指示 */
  const [rendering, setRendering] = useState(false)
  /** 预览那张的 `blob:` URL（**临时的**：只在内存里，保存/换图就作废） */
  const [previewUrl, setPreviewUrl] = useState<string | null>(null)
  const previewUrlRef = useRef<string | null>(null)
  /*
   * ★★★ **看图缩放 / 平移 —— 四个档位共用这一套**（2026-10-17 作者：「我发现客户视图正常
   * 拖动缩放，其他的就不行……我希望三个用的同一套的，切换不改变位置，缩放」）。
   *
   * 所以它**不是**客户视角的东西，是画布自己的：transform 挂在**整块 `.board`** 上
   * （图 + 框 + 编号 + 网格 + 手柄全在里面 ⇒ 一起缩放/平移，跟 PS 里缩放画布一个意思），
   * 切档只换"叠什么"、**绝不复位**；只有**换图**才回到 100%。
   *
   * 手法**照搬 `components/ImageZoom.tsx`**（那套是修过"抖 / 闪 / 捏合不跟手"三个病之后
   * 的第二版）：`rAF` 每帧把内联 `transform` 写一次（CSS transition 期间每条滚轮都从头播
   * 一遍 ⇒ 抖，所以**不准有 transition**）；**以指针为不动点**。
   *
   * ★★★ **缩放全程不惊动 React**（2026-10-17 作者：「现在缩放很不流畅」）——
   *   病根有两条，都在"每帧 setState"上：
   *   ① 每帧 `setViewK/setViewOff` ⇒ 整页（几百个框 + 面板）重渲染一次；
   *   ② 那条挂监听的 effect 依赖里带着 `viewK/viewOff` ⇒ **每一帧都在摘挂一遍监听器**。
   *   ⇒ 真值只住 `viewRef`（**唯一源**），事件改它、`rAF` 直接写 DOM（`transform` 与
   *     `--view-k`）；**缩放进不了 React 状态**，一次重渲染都没有。
   */
  /** 看图缩放 / 平移的**唯一源**：`k` = 倍率、`x/y` = 平移（client px）。不进 React 状态 */
  const viewRef = useRef({ k: 1, x: 0, y: 0 })
  const viewRaf = useRef(0)
  /** 中键按压拖动那一趟（起点 + 起点那一刻的平移量）；`null` = 没在拖 */
  const viewPanRef = useRef<{ id: number; x: number; y: number; ox: number; oy: number } | null>(null)
  /** 同上，只为把光标换成"抓着"那一只（拖的过程中才变） */
  const [viewPan, setViewPan] = useState(false)
  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null)
  const [stageSize, setStageSize] = useState({ w: 0, h: 0 })
  const [past, setPast] = useState<Mark[][]>([])
  const [future, setFuture] = useState<Mark[][]>([])
  const [importOpen, setImportOpen] = useState(false)
  const [importText, setImportText] = useState('')
  /** 「导出／导入（给开发用）」那一行展不展开（默认收起 —— 作者「我不理解，我不知道怎么用」） */
  const [ioOpen, setIoOpen] = useState(false)

  const [stageEl, setStageEl] = useState<HTMLDivElement | null>(null)
  const wrapRef = useRef<HTMLDivElement | null>(null)
  const boardRef = useRef<HTMLDivElement | null>(null)
  const fileRef = useRef<HTMLInputElement | null>(null)
  const objectUrlRef = useRef<string | null>(null)
  const dragRef = useRef<Drag | null>(null)
  const scaleRef = useRef(1)
  const snapRef = useRef(false)
  const sizeRef = useRef<{ w: number; h: number }>({ w: 0, h: 0 })
  /* 备注框：聚焦那一刻的框数组 —— 第一次真改前后各压一格，别让一个字一格（与拖动同一条"第一次真动才入栈"） */
  const noteBaseRef = useRef<Mark[] | null>(null)
  /* "有改动没写进仓库"的最新快照 —— `pick`（换图）里要读它，但不想把它塞进 `pick` 的依赖里 */
  const dirtyRef = useRef(false)
  /* 当前这张图的最新快照 —— 同上（`pick` 要拿"上一张"跟"下一张"比是不是同一组） */
  const srcRef = useRef<Source | null>(null)

  /* ---- 看图缩放 / 平移：rAF 落笔 / 滚轮记账 / 换图才复位（手法同 ImageZoom.tsx） ---- */

  const viewApply = useCallback(() => {
    viewRaf.current = 0
    const v = viewRef.current
    const board = boardRef.current
    /*
     * ★ transform 挂在**整块画布**上（不是只挂图）：框 / 编号 / 网格 / 手柄都是它的孩子，
     *   一起缩放平移，才谈得上"四个档位同一套"。
     * ★ 连 `--view-k` 也一并写到 DOM 上（手柄 / 网格线要拿它除回去）—— 它要是在 React 的
     *   style 里，就得靠重渲染才更新，等于又把卡顿请回来。
     */
    if (board !== null) {
      board.style.transform = `translate(${v.x}px, ${v.y}px) scale(${v.k})`
      board.style.setProperty('--view-k', String(v.k))
    }
  }, [])

  const viewSchedule = useCallback(() => {
    if (viewRaf.current === 0) viewRaf.current = requestAnimationFrame(viewApply)
  }, [viewApply])

  useEffect(() => () => cancelAnimationFrame(viewRaf.current), [])

  /*
   * **只有换图**才复位 100%（含抹掉上一轮的内联 transform）。
   * ★★★ **切档（编辑 ⇄ 看效果 ⇄ 客户视角 ⇄ 全不叠）不复位** —— 作者要的就是
   * 「切换不改变位置、缩放」：四档比的是"叠了什么"，几何必须一动不动才比得出来。
   */
  useEffect(() => {
    viewRef.current = { k: 1, x: 0, y: 0 }
    viewPanRef.current = null
    setViewPan(false)
    const board = boardRef.current
    if (board !== null) {
      board.style.transform = 'translate(0px, 0px) scale(1)'
      board.style.setProperty('--view-k', '1')
    }
  }, [src?.key])

  /* 滚轮 / 触控板捏合 / 双指滑 —— 挂整块画布（**四个档位通用**：指针在画布任何地方滚都缩放或平移） */
  useEffect(() => {
    const board = boardRef.current
    if (board === null) return
    /*
     * ★★★ **入口分流**（2026-10-17 作者：「触摸板还需要可以双指拖动，触摸板可以捏合缩放，
     *   鼠标可以中键按压拖动？」；同日又报「触摸板的缩放没了」）：
     *
     * - **捏合**：**WebKit（本仓库）走的是 `gesturestart / gesturechange / gestureend`**，
     *   `event.scale` 直接给手势倍率 —— 那条 `wheel + ctrlKey` 是 **Chrome 的话**！
     *   第一版"没带 ctrlKey 的 wheel 全拿去缩放"碰巧把捏合也盖住了；按 delta 分流之后
     *   捏合被当成两指滑 ⇒ 缩放失效（作者实锤）。⇒ 捏合**单独走 gesture 通道**，两条都接。
     * - **鼠标滚轮** ⇒ **缩放**：整档的整数大 delta（`|deltaY| ≥ 100`、`deltaX` 恒 0），
     *   或 `deltaMode !== 0`（按行滚的鼠标，如 Firefox）。
     * - **其余** = 触控板**两指滑** ⇒ **平移**：delta 小、常带小数、还常伴 `deltaX`。
     *   （浏览器没有"这是触控板还是鼠标"的字段，只能看 delta 形状 —— Figma 同款判据。）
     */
    /** 捏合那一趟：起点那一刻的 k / 平移 / 起始 scale，`gesturechange` 按比例折算 */
    let gesture: { k: number; scale: number } | null = null
    const clampK = (v: number) => Math.min(8, Math.max(0.5, v))
    /*
     * 以**指针为不动点**：`o' = d - r·(d - o)`，`d` = 指针相对**画布布局中心**的偏移。
     *
     * ★★ 这里的 `d` 必须用**没被 transform 影响过的那个中心**：transform 现在挂在 `.board`
     *    自己身上，`getBoundingClientRect()` 量回来的是**变换后**的盒子（中心 = 布局中心 + 平移、
     *    宽高 = 布局 × k）⇒ 先把平移减回去才是布局中心（`c = 视觉中心 - off`）。
     *    漏这一步 = 缩放时画面自己飘走（越缩越偏）。
     */
    const zoomAt = (next: number, cx: number, cy: number) => {
      const cur = viewRef.current
      const rect = board.getBoundingClientRect()
      const dx = cx - (rect.left + rect.width / 2 - cur.x)
      const dy = cy - (rect.top + rect.height / 2 - cur.y)
      const r = next / cur.k
      viewRef.current = { k: next, x: dx - r * (dx - cur.x), y: dy - r * (dy - cur.y) }
      viewSchedule()
    }
    /* ---- WebKit 捏合（gesture 通道）---- */
    const onGestureStart = (ev: Event) => {
      ev.preventDefault()
      const g = ev as unknown as { scale: number; clientX: number; clientY: number }
      gesture = { k: viewRef.current.k, scale: g.scale || 1 }
      zoomAt(viewRef.current.k, g.clientX, g.clientY)
    }
    const onGestureChange = (ev: Event) => {
      ev.preventDefault()
      if (gesture === null) return
      const g = ev as unknown as { scale: number; clientX: number; clientY: number }
      zoomAt(clampK((gesture.k * g.scale) / gesture.scale), g.clientX, g.clientY)
    }
    const onGestureEnd = () => {
      gesture = null
    }
    /* ---- wheel：鼠标滚轮缩放、触控板两指滑平移 ---- */
    const onWheel = (e: WheelEvent) => {
      e.preventDefault()
      const cur = viewRef.current
      const zoom =
        e.ctrlKey || (e.deltaX === 0 && Number.isInteger(e.deltaY) && Math.abs(e.deltaY) >= 100)
      if (!zoom) {
        /* 双指滑 = 平移（跟着手指走：`delta` 是"内容该往哪个方向退"） */
        viewRef.current = { k: cur.k, x: cur.x - e.deltaX, y: cur.y - e.deltaY }
        viewSchedule()
        return
      }
      /* 指数步长：与客户端放大层同一组数 */
      zoomAt(clampK(cur.k * Math.exp(-e.deltaY * 0.0028)), e.clientX, e.clientY)
    }
    board.addEventListener('wheel', onWheel, { passive: false })
    board.addEventListener('gesturestart', onGestureStart as EventListener)
    board.addEventListener('gesturechange', onGestureChange as EventListener)
    board.addEventListener('gestureend', onGestureEnd)
    return () => {
      board.removeEventListener('wheel', onWheel)
      board.removeEventListener('gesturestart', onGestureStart as EventListener)
      board.removeEventListener('gesturechange', onGestureChange as EventListener)
      board.removeEventListener('gestureend', onGestureEnd)
    }
    /*
     * ★ 依赖里**不允许**出现 `k` / 平移（它们每帧都变 ⇒ 每帧摘挂一遍监听器 = 卡）。
     *   缩放真值住在 `viewRef` 里，回调读它拿到的永远是最新的。
     * ★ `src?.key` 必进依赖：换图时画布是**新挂**的（换图那一帧 `boardRef` 还是 null，
     *   不重挂监听就整块画布没了手势）。
     */
  }, [viewSchedule, src?.key])

  /* 资产清单（只留能看的图：按后缀认）—— 与资产库同一个读口。
     ★ `assetsRev` 是资产库页敲的那个信号（改了分组 / 档位就重取一遍）：这一页被外壳
       缓存住、切回来不重渲染，不靠它就会一直揣着旧分组。见 `assetsBus.ts` */
  const assetsRev = useSyncExternalStore(assetsBus.subscribe, assetsBus.get)
  useEffect(() => {
    void wb
      .assets(null, null, null, null, null, null, null, null)
      .then((l) => setAssets(l.assets))
      .catch((e: unknown) => toasts.push(`资产清单没读出来：${errText(e)}`))
  }, [assetsRev])

  const imgAssets = useMemo(
    () => (assets ?? []).filter((a) => /\.(png|jpe?g|webp|gif|svg)$/i.test(a.path)),
    [assets],
  )

  /**
   * 图按**组**摆（2026-10-16 作者）：
   *
   * > 「它们本质上是同一套图的两个，那我就希望它不是这种分开的，他需要是**套装的感觉**，
   * >   这样他的框选的才能说是对应两个的……用同样的框的就是**一组**」
   *
   * 一组的判据**读台账**（`AssetView.groupId`）—— 也就是**一份源稿的落点**：
   * 同一组 = 同一份 `.anno.json` = 同一套框。所以"在哪一组里"与"框会画到哪几张上"永远是同一件事。
   * ★ 组名也**来自台账**（`groupName`）。没进组的仍按老规矩把名字尾巴上那截
   *   「（浅色）/（深色）」抹掉当组名 —— 那条只是显示用的兜底。
   */
  const groups = useMemo(() => {
    const byKey = new Map<string, AssetView[]>()
    for (const a of imgAssets) {
      const k = a.groupId ?? pairOf(a.id)
      byKey.set(k, [...(byKey.get(k) ?? []), a])
    }
    return [...byKey.entries()].map(([key, items]) => ({
      key,
      items,
      name:
        items.find((a) => a.groupName !== null)?.groupName ??
        (items[0]?.display ?? key).replace(/[（(]\s*(浅色|深色)\s*[）)]\s*$/, '').trim(),
    }))
  }, [imgAssets])

  /** 当前这张的**同组伙伴**—— 切过去框原样在，因为它们共用一份源稿 */
  const sibling = useMemo(
    () =>
      src === null
        ? null
        : imgAssets.find((a) => a.id !== src.key && (a.groupId ?? pairOf(a.id)) === groupKeyOf(src)) ?? null,
    [imgAssets, src],
  )

  const [repo, setRepo] = useState<RepoStamp>({ path: '', doc: null })
  /** 本机那份暂存比仓库那份新（或仓库还没有）时挂着它，面板上挑明让作者选 */
  const [conflict, setConflict] = useState<Saved | null>(null)
  const [saving, setSaving] = useState(false)
  /** 冲突时是否摊开"两份差在哪"的对比层（在图上把两版框并排画出来） */
  const [diffOpen, setDiffOpen] = useState(false)

  /** 把一份暂存铺进这一页（本机那份 / 冲突时点"用本机那份"都走它） */
  const applySaved = useCallback((saved: Saved | null) => {
    setMarks(saved?.marks ?? [])
    setZoom(saved?.zoom ?? 'fit')
    setGrid(saved?.grid ?? true)
    setSnap(saved?.snap ?? false)
  }, [])

  /**
   * 换一张图 = 换一处工作台：
   *
   * 1. **先问仓库**（`wb_anno_read`）—— 源稿在图旁边，那是能跨设备接着编的那一份；
   * 2. 本机暂存只当"没写进仓库的草稿"：仓库还没有它、或它比仓库那份**新**时，
   *    挂出 `conflict` 让作者自己选（不猜、不静默覆盖）；
   * 3. 本地文件（`file:` 开头）仓库管不着 —— 只用本机暂存。
   */
  const pick = useCallback(
    (next: Source) => {
      /*
       * 换到**别的组**、而当前这组还有改动没写进仓库 ⇒ 说一声（作者：「这个有改动没写进仓库，
       * 这一些，我觉得这个完全没做好」）。同一组内浅⇄深切换不算（共用一份源稿、框没动）。
       * ★ 只提醒、不拦 —— 本机暂存已经把改动留住了（`writeStore`），切回来还在。
       */
      if (
        dirtyRef.current &&
        srcRef.current !== null &&
        groupKeyOf(next) !== groupKeyOf(srcRef.current)
      ) {
        toasts.push('上一组还有改动没写进仓库 —— 切回去它还在（本机留着草稿）')
      }
      /* 换图 ⇒ 上一张的**临时预览**作废（它是"上一张 + 上一版框"的照片） */
      if (previewUrlRef.current !== null) {
        URL.revokeObjectURL(previewUrlRef.current)
        previewUrlRef.current = null
      }
      setPreviewUrl(null)
      srcRef.current = next
      setSrc(next)
      setSel(null)
      setPast([])
      setFuture([])
      setRepo({ path: '', doc: null })
      setConflict(null)

      const local = readStore()[groupKeyOf(next)] ?? null
      /* 视图态（缩放 / 网格 / 吸附）是本机的，与源稿无关 */
      setZoom(local?.zoom ?? 'fit')
      setGrid(local?.grid ?? true)
      setSnap(local?.snap ?? false)

      if (next.key.startsWith('file:')) {
        setMarks(local?.marks ?? [])
        return
      }

      void wb
        .annoRead(next.key)
        .then((v) => {
          setRepo({ path: v.path, doc: v.doc })
          /*
           * ★★★ 显示**原图**，不是台账那条成品图（2026-10-16 方案 A）：
           * 台账 `path` 指针的是 `.rendered.webp`（框已烧进像素）—— 在它上面再画框
           * 就是"框上叠框"。后端算了原图的落点（`sourceImage`），这里换掉 URL。
           * 只在**同一条资产**上换（`key` 对得上），免得把别的图顶掉。
           */
          setSrc((cur) =>
            cur !== null && cur.key === next.key
              ? { ...cur, url: assetUrl(`/assets/${v.sourceImage}`) }
              : cur,
          )
          if (v.doc !== null) {
            const fromRepo = v.doc.marks.map(markOf)
            setMarks(fromRepo)
            /*
             * 本机那份**比仓库那份新、而且内容真的不一样**才问作者。
             * ★ 只比时间会误报：编辑时随手存的那份总比仓库那份新（哪怕内容一模一样，
             *   比如刚保存完又切了一次图）—— 那种"假冲突"弹出来只会烦人。
             */
            if (
              local !== null &&
              local.savedAt > Date.parse(v.doc.updatedAt) &&
              marksSig(local.marks) !== marksSig(fromRepo)
            ) {
              setConflict(local)
            }
            return
          }
          /* 仓库还没有这份源稿：本机有暂存就先摆上来（并挑明"它还没进仓库"） */
          applySaved(local)
          setConflict(local !== null && local.marks.length > 0 ? local : null)
        })
        .catch((e: unknown) => {
          toasts.push(`读源稿没成功：${errText(e)}`)
          applySaved(local)
        })
    },
    [applySaved],
  )

  /* 首屏落在引导图（浅色那张）—— 这一页就是为它做的 */
  useEffect(() => {
    if (src !== null || imgAssets.length === 0) return
    const first =
      imgAssets.find((a) => a.kind === 'guide' && /light/i.test(a.id)) ??
      imgAssets.find((a) => a.kind === 'guide') ??
      imgAssets[0]
    pick({
      key: first.id,
      groupId: first.groupId,
      groupName: first.groupName,
      label: first.display,
      path: first.path,
      url: assetUrl(first.url),
      renderedUrl: assetUrl(first.url),
      w: 0,
      h: 0,
    })
  }, [imgAssets, pick, src])

  /* 编辑时随手存一份**本机暂存**（掉电不丢）—— 进仓库靠「写入仓库」那一颗 */
  useEffect(() => {
    if (src === null) return
    writeStore(groupKeyOf(src), {
      marks,
      zoom,
      grid,
      snap,
      savedAt: Date.now(),
      ...(src.w > 0 ? { size: { w: src.w, h: src.h } } : {}),
    })
  }, [src, marks, zoom, grid, snap])

  /** 与仓库那份比"框变了没"（订阅它来标"有改动没写进仓库"） */
  const sig = marksSig(marks)
  const repoMarks = useMemo(() => (repo.doc === null ? null : repo.doc.marks.map(markOf)), [repo.doc])
  const dirty = repoMarks === null || sig !== marksSig(repoMarks)
  /* 换图时那道"还有改动没保存"的提醒要读它（见 `pick`）—— 每次渲染后刷新，抓得到最新一份 */
  useEffect(() => {
    dirtyRef.current = dirty && src !== null && !src.key.startsWith('file:')
  })
  /*
   * 写进仓库这一条路对本地文件不适用（仓库里没有它）。
   * ★★ **有没保存的预览也算"要保存"**（2026-10-17 作者：「可是我生成了预览，你不让我
   *   保存怎么办？」）—— 预览是"拍照没存相册"：框可能和仓库一模一样，但**成品图**
   *   （框烧进去的那张 webp）还没落盘 —— 比如换了编号字体之后，框没动也得重烧一张。
   */
  const canSave =
    src !== null && !src.key.startsWith('file:') && src.w > 0 && !saving && (dirty || previewUrl !== null)

  const save = useCallback(() => {
    if (src === null || src.key.startsWith('file:') || src.w === 0) return
    /*
     * 保存 / 放弃那两颗现在在**页头**（本页之外）—— 点完焦点落在按钮上，
     * 这一页自己的键盘（⌘Z / Delete / 方向键）就收不到了。所以：**点完把焦点收回来**
     * （作者要"保存后还能 ⌘Z"）。见 `usePageActions` 的 `save`/`discard`。
     */
    wrapRef.current?.focus()
    setSaving(true)
    void wb
      /*
       * ★★★ **按参考显示比例烧，不按当前缩放**（见 `refScale` 的头注）：成品图从此**固定** ——
       * 换视角、换窗口大小都不再影响它。想改框的粗细就改「框的样子」里那三格。
       */
      .annoWrite(src.key, docOf(marks, src, assets ?? []), refScale(src.w))
      .then((v) => {
        setRepo({ path: v.path, doc: v.doc })
        setConflict(null)
        clearStore(groupKeyOf(src))
        /*
         * ★★★ **保存不清撤销栈**（作者：「甚至保存了也能撤销」）——
         * 这里只换 `repo`/清暂存，`past`/`future` 原样留着；保存之后 ⌘Z / 撤销键照样能
         * 退回保存前那一版。※ 这一句是这条口径的落点，别在别处加 `setPast([])`。
         */
        /*
         * ★ 保存 = 后端同趟重渲了成品图 ⇒ 换掉破缓存戳，客户视角立刻看到新的（不然是缓存的旧图）；
         * 那张**临时预览**到此作废（它已经"存进相册"了）。
         */
        setRenderStamp((n) => n + 1)
        if (previewUrlRef.current !== null) {
          URL.revokeObjectURL(previewUrlRef.current)
          previewUrlRef.current = null
        }
        setPreviewUrl(null)
        toasts.push(`已写进仓库：${v.path}`)
      })
      .catch((e: unknown) => toasts.push(`写不进仓库：${errText(e)}`))
      .finally(() => setSaving(false))
  }, [assets, marks, src])

  /**
   * **生成预览**（"拍一张照、不存相册"）：按**参考显示比例**（[`refScale`]，与当前缩放无关）
   * 烘一张成品图，只回字节不落盘，转成 `blob:` URL 塞给「客户视角」看效果。
   * 不满意就接着拖框再拍，满意了才点保存。
   */
  const makePreview = useCallback(() => {
    if (src === null || src.key.startsWith('file:') || src.w === 0) return
    setRendering(true)
    void wb
      .annoRender(src.key, docOf(marks, src, assets ?? []), refScale(src.w))
      .then((shots) => {
        const mine = shots.find((s) => s.out.endsWith(renderedNameOf(src.path))) ?? shots[0]
        if (mine === undefined) {
          toasts.push('预览没烘出来（原图可能不在）')
          return
        }
        /* 上一次那张 blob 先撤掉（不然一直攒着） */
        if (previewUrlRef.current !== null) URL.revokeObjectURL(previewUrlRef.current)
        const url = URL.createObjectURL(b64ToBlob(mine.bytesB64, 'image/webp'))
        previewUrlRef.current = url
        setPreviewUrl(url)
        setView('client')
        toasts.push('预览已生成（临时的，没写进仓库）—— 满意就点保存')
      })
      .catch((e: unknown) => toasts.push(`预览烘不出来：${errText(e)}`))
      .finally(() => setRendering(false))
  }, [assets, marks, src])

  /* 离开这一页时把 blob URL 撤掉（不然这一趟攒的图一直占着内存） */
  useEffect(
    () => () => {
      if (previewUrlRef.current !== null) URL.revokeObjectURL(previewUrlRef.current)
    },
    [],
  )

  /* 图的尺寸和源稿那份对不上 = 这一对不是同裁切：框会飘，说一句（每次打开只说一次） */
  const sizeWarned = useRef(new Set<string>())
  useEffect(() => {
    if (src === null || src.w === 0) return
    const saved = readStore()[groupKeyOf(src)]?.size ?? repo.doc?.source.size
    if (saved === undefined || (saved.w === src.w && saved.h === src.h)) return
    const key = groupKeyOf(src)
    if (sizeWarned.current.has(key)) return
    sizeWarned.current.add(key)
    toasts.push(`这张是 ${src.w}×${src.h}，那份标注量在 ${saved.w}×${saved.h} 上 —— 尺寸不一样框会飘，得重量一次`)
  }, [src, repo.doc])

  /* ---------- 尺寸与缩放 ---------- */

  useEffect(() => {
    if (stageEl === null) return
    const read = () => setStageSize({ w: stageEl.clientWidth, h: stageEl.clientHeight })
    read()
    const ro = new ResizeObserver(read)
    ro.observe(stageEl)
    return () => ro.disconnect()
  }, [stageEl])

  const scale = useMemo(() => {
    if (src === null || src.w === 0) return 1
    if (typeof zoom === 'number') return zoom
    if (stageSize.w === 0 || stageSize.h === 0) return 1
    return Math.min(1, (stageSize.w - 28) / src.w, (stageSize.h - 28) / src.h)
  }, [zoom, src, stageSize])

  useEffect(() => {
    scaleRef.current = scale
  }, [scale])
  useEffect(() => {
    snapRef.current = snap
  }, [snap])
  useEffect(() => {
    sizeRef.current = { w: src?.w ?? 0, h: src?.h ?? 0 }
  }, [src])

  /* ---------- 撤销栈 ----------
   *
   * 栈里存的就是"框数组"本身（`Mark[]` 小对象，`HIST_MAX` 49 格内存可忽略）。
   * ★★★ **保存不进栈、也不清栈**（2026-10-16 作者：「甚至保存了也能撤销」）——
   * `save()` 只换 `repo` / 清本机暂存，`past`/`future` 原样留着，所以保存之后 ⌘Z
   * 照样能退回保存前那一版。
   * ★ 每一次真正改变 `marks` 的动作都走 `commit(改之前那一份)`：拖动（第一次真动才压）、
   * 拉边、加框、删框、挪次序、显隐、方向键微调、改数、改样式、改备注、导入、"放弃"。
   */

  const commit = useCallback((before: Mark[]) => {
    setPast((p) => [...p, before].slice(-HIST_MAX))
    setFuture([])
  }, [])

  /* 撤销 / 重做之后把选中**收拾干净**：那个框还在就接着选它，被撤没了就落回"点一个框" */
  const keepSel = (list: Mark[]) => setSel((s) => (s !== null && list.some((m) => m.id === s) ? s : null))

  const undo = useCallback(() => {
    if (past.length === 0) return
    const prev = past[past.length - 1]
    setPast(past.slice(0, -1))
    setFuture([...future, marks])
    setMarks(prev)
    keepSel(prev)
  }, [past, future, marks])

  const redo = useCallback(() => {
    if (future.length === 0) return
    const next = future[future.length - 1]
    setFuture(future.slice(0, -1))
    setPast([...past, marks])
    setMarks(next)
    keepSel(next)
  }, [past, future, marks])

  /* 撤销 / 重做**不摆按钮**（作者 2026-10-16：「撤销重做也不用显示出来，用快捷键就可以了」）
     —— 能力还在，走 ⌘Z / ⇧⌘Z（见 `onKey`）。 */

  /**
   * 「放弃」= 把改动退回到**打开时的基线**（那次读到的仓库那份；仓库还没有就是空的）。
   *
   * ★ 它是**一次真动作、且本身可撤销**（作者：「保存了也能撤销」+「我删了之后我没办法点放弃」）：
   * 走 `commit(marks)` 把当前这一份压进栈，再铺回基线 —— 点错了 ⌘Z 就回来。
   * ★ 基线判据 = `repo.doc`（打开时那一份），**不是**"本机暂存" —— 暂存是编辑过程随手落的，
   *   不代表"打开时的样子"。本机暂存跟着清掉（放弃之后它就是上一段历史了）。
   */
  const discard = useCallback(() => {
    if (src === null || src.key.startsWith('file:') || !dirty) return
    /* 页头那颗点完同样把焦点收回本页（不然"放弃之后再 ⌘Z 撤回"就按不出来） */
    wrapRef.current?.focus()
    const base = repo.doc === null ? [] : repo.doc.marks.map(markOf)
    commit(marks)
    setMarks(base)
    setSel(null)
    clearStore(groupKeyOf(src))
    toasts.push('已放弃这次改动（⌘Z 可以撤回这一步）')
  }, [commit, dirty, marks, repo.doc, src])

  /** 「放弃」这颗按钮的可用性：有改动、且这一份归仓库管（本地文件没有"退回基线"这回事） */
  const canDiscard = src !== null && !src.key.startsWith('file:') && dirty

  /*
   * 「保存 / 放弃」**只在页头那一处**（作者 2026-10-16：「就用上面那一个啊，这个没必要
   * 自己再写」）—— 这一页把自己的那一槽登记给外壳：页头那三件就变成"这一份源稿"的
   * 已保存 / 放弃 / 保存。面板里那套重复按钮一并撤掉。
   *
   * ★ 本地文件（`file:`）没有"写进仓库"这回事 ⇒ 只报"不归仓库管"，但那两颗仍然摆着
   *   （禁用态），免得页头忽然少一截；`repoBar` 里也照旧说明。
   */
  usePageActions('annotate', {
    dirtyLabel: '有改动没写进仓库',
    savedLabel: '已写进仓库',
    dirty,
    busy: saving,
    savingLabel: '写入中…',
    canSave,
    canDiscard,
    dirtyTitle:
      src !== null && src.key.startsWith('file:')
        ? '本地文件不进仓库 —— 要带走就点「复制 TS」'
        : dirty
          ? `这一份源稿还有改动没写进仓库${repo.path === '' ? '' : `（${repo.path}）`}`
          : previewUrl !== null
            ? '框和仓库里那份一样，但那张预览是临时的 —— 点保存才把成品图烧进仓库'
            : repo.path === ''
              ? '正在读源稿…'
              : `这一份源稿和仓库里那份一致（${repo.path}）`,
    saveTitle: '把这一份写进仓库（`presets/assets/…` 里那份 `.anno.json`）',
    discardTitle: '放弃这次改动，退回打开时那份（这一步本身也能撤销）',
    save,
    discard,
  })

  /* ---------- 拖动 ---------- */

  const snapPx = useCallback((v: number) => (snapRef.current ? Math.round(v / GRID) * GRID : Math.round(v)), [])

  /*
   * client 坐标 → 图的原生 px（唯一一处换算）。
   *
   * ★★★ **必须把"看图缩放"那一层反解掉**（2026-10-17）：transform 现在挂在 `.board` 上，
   * `getBoundingClientRect()` 量的是**变换后**的盒子。反解顺序与上面 `zoomAt` 严格互逆：
   *   ① 视觉中心 = `rect.left + rect.width / 2`，布局中心 = 视觉中心 − 平移；
   *   ② 布局坐标 = 布局中心 + (指针 − 布局中心 − 平移) / k；
   *   ③ 再除以画布比例 `scale` 才是图的原生 px。
   * 漏了 ①② 的话，缩放之后拖框就会"跟手对不上"（拖动量与指针位移差一个 k）。
   */
  const toImg = useCallback((e: { clientX: number; clientY: number }) => {
    const el = boardRef.current
    if (el === null) return null
    const r = el.getBoundingClientRect()
    /* ★ 读 `viewRef`（唯一源）—— 缩放不进 React 状态，这里拿到的才是"这一帧"的几何 */
    const cur = viewRef.current
    const cx = r.left + r.width / 2 - cur.x
    const cy = r.top + r.height / 2 - cur.y
    const k = cur.k || 1
    /* 布局坐标（client）反解：`L = 布局中心 + (指针 − 布局中心 − 平移) / k` */
    const lx = cx + (e.clientX - cur.x - cx) / k
    const ly = cy + (e.clientY - cur.y - cy) / k
    /* ★ 盒子左上角也要用**布局**那只（`rect` 是变换后的）：布局半宽 = 视觉半宽 ÷ k */
    const left = cx - r.width / 2 / k
    const top = cy - r.height / 2 / k
    return { x: (lx - left) / scaleRef.current, y: (ly - top) / scaleRef.current }
  }, [])

  const startDrag = (e: React.PointerEvent, m: Mark, mode: 'move' | 'resize', edge: Edge = '') => {
    /* 只有在「编辑」那一档才动得了框（看效果 / 客户视角 / 全不叠都是"只看"） */
    if (view !== 'edit') return
    e.preventDefault()
    e.stopPropagation()
    e.currentTarget.setPointerCapture(e.pointerId)
    /* 键盘那一套挂在这一页的根上（React onKeyDown）—— 鼠标点过之后得有人拿着焦点，
       否则方向键 / Delete / ⌘Z 全落在 body 上、一个都不响应（实测） */
    wrapRef.current?.focus()
    setSel(m.id)
    dragRef.current = {
      mode,
      id: m.id,
      edge,
      cx: e.clientX,
      cy: e.clientY,
      before: marks,
      orig: m,
      pushed: false,
    }
  }

  /**
   * 空白处按下：**先当它是一次点击** —— 立刻收起编辑区（作者 2026-10-16：「我点击空白的地方，
   * 它就把下面那一个编辑的收起来」）。真**拖过 4px** 才算"画一个新框"（第一次真动才落笔，
   * 见 `onMove`）—— 这样"点一下空白"与"拖一个框"分得开，不会一点就冒出个 1×1 的小框。
   */
  const onBoardDown = (e: React.PointerEvent) => {
    if (view !== 'edit' || e.button !== 0 || src === null || src.w === 0) return
    const p = toImg(e)
    if (p === null) return
    e.currentTarget.setPointerCapture(e.pointerId)
    wrapRef.current?.focus()
    /* 点空白 = 不选中任何框 ⇒ 右下面那块编辑区收起 */
    setSel(null)
    const x = clamp(snapPx(p.x), 0, src.w - MIN_SIDE)
    const y = clamp(snapPx(p.y), 0, src.h - MIN_SIDE)
    const fresh: Mark = { id: uid(), at: atOf(x, y, 1, 1, src.w, src.h), note: '', off: false }
    /* ★ `created: false` ⇒ 这会儿还**没进 `marks`**；拖过阈值才落笔 */
    dragRef.current = {
      mode: 'new',
      id: fresh.id,
      edge: 'se',
      cx: e.clientX,
      cy: e.clientY,
      before: marks,
      orig: fresh,
      pushed: false,
      created: false,
    }
  }

  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current
      const size = sizeRef.current
      if (d === null || size.w === 0) return
      const dx = (e.clientX - d.cx) / scaleRef.current
      const dy = (e.clientY - d.cy) / scaleRef.current
      if (dx === 0 && dy === 0) return
      /*
       * 画新框那一支：**拖过 4px 才落笔**。
       * 没落笔时"点一下空白"= 一次点击（`onBoardDown` 里已经收起编辑区），不留小框；
       * 一落笔就把框塞进 `marks` 并选中它 —— 之后跟普通拖动走同一条路。
       */
      if (d.mode === 'new' && d.created !== true) {
        if (Math.hypot(e.clientX - d.cx, e.clientY - d.cy) < 4) return
        /* 落笔：先把"按下前那一份"压进栈（`pushed` 一同置位，免得下面再压一次） */
        d.created = true
        d.pushed = true
        commit(d.before)
        setMarks((bs) => [...bs, d.orig])
        setSel(d.orig.id)
      }
      const o = pxOf(d.orig, size.w, size.h)
      let next: Mark['at']
      if (d.mode === 'move') {
        const x = clamp(snapPx(o.x + dx), 0, size.w - o.w)
        const y = clamp(snapPx(o.y + dy), 0, size.h - o.h)
        next = atOf(x, y, o.w, o.h, size.w, size.h)
      } else {
        const edge = d.mode === 'new' ? 'se' : d.edge
        let l = o.x
        let t = o.y
        let r = o.x + o.w
        let b = o.y + o.h
        if (edge.includes('w')) l = clamp(snapPx(o.x + dx), 0, r - MIN_SIDE)
        if (edge.includes('e')) r = clamp(snapPx(o.x + o.w + dx), l + MIN_SIDE, size.w)
        if (edge.includes('n')) t = clamp(snapPx(o.y + dy), 0, b - MIN_SIDE)
        if (edge.includes('s')) b = clamp(snapPx(o.y + o.h + dy), t + MIN_SIDE, size.h)
        next = atOf(l, t, r - l, b - t, size.w, size.h)
      }
      if (!d.pushed) {
        d.pushed = true
        commit(d.before)
      }
      setMarks((bs) => bs.map((m) => (m.id === d.id ? { ...m, at: next } : m)))
    }
    const onUp = () => {
      dragRef.current = null
    }
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    window.addEventListener('pointercancel', onUp)
    return () => {
      window.removeEventListener('pointermove', onMove)
      window.removeEventListener('pointerup', onUp)
      window.removeEventListener('pointercancel', onUp)
    }
  }, [commit, snapPx])

  /* ---------- 那几个动作 ---------- */

  const addBox = () => {
    if (src === null) return
    const n = marks.length
    const fresh: Mark = {
      id: uid(),
      at: { left: 30 + (n % 5) * 4, top: 45 + (n % 5) * 6, width: 40, height: 8 },
      note: '',
      off: false,
    }
    commit(marks)
    setMarks([...marks, fresh])
    setSel(fresh.id)
  }

  const removeBox = (id: string) => {
    commit(marks)
    setMarks(marks.filter((m) => m.id !== id))
    if (sel === id) setSel(null)
  }

  const moveRow = (i: number, d: number) => {
    const j = i + d
    if (j < 0 || j >= marks.length) return
    commit(marks)
    const next = [...marks]
    const [row] = next.splice(i, 1)
    next.splice(j, 0, row)
    setMarks(next)
  }

  const toggleOff = (id: string) => {
    commit(marks)
    setMarks(marks.map((m) => (m.id === id ? { ...m, off: !m.off } : m)))
  }

  const nudge = (dx: number, dy: number) => {
    const m = marks.find((b) => b.id === sel)
    if (m === undefined || src === null) return
    const o = pxOf(m, src.w, src.h)
    const x = clamp(o.x + dx, 0, src.w - o.w)
    const y = clamp(o.y + dy, 0, src.h - o.h)
    commit(marks)
    setMarks(marks.map((b) => (b.id === m.id ? { ...b, at: atOf(x, y, o.w, o.h, src.w, src.h) } : b)))
  }

  /** 面板上改一个数（px）：宽的高的按最小边兜底，整体夹在图里 */
  const setField = (k: (typeof K_KEYS)[number], v: number) => {
    const m = marks.find((b) => b.id === sel)
    if (m === undefined || src === null) return
    const o = pxOf(m, src.w, src.h)
    const w = k === 'width' ? Math.max(MIN_SIDE, v) : o.w
    const h = k === 'height' ? Math.max(MIN_SIDE, v) : o.h
    const x = k === 'left' ? v : o.x
    const y = k === 'top' ? v : o.y
    commit(marks)
    setMarks(
      marks.map((b) =>
        b.id === m.id
          ? { ...b, at: atOf(clamp(x, 0, src.w - w), clamp(y, 0, src.h - h), w, h, src.w, src.h) }
          : b,
      ),
    )
  }

  /** 改选中框的"样子"（留空 = 撤掉这一项，回到站点缺省；空着就不写进源稿） */
  const setStyle = (id: string, key: 'radius' | 'stroke' | 'badge', raw: string) => {
    const v = raw.trim() === '' ? undefined : Number(raw)
    if (v !== undefined && (!Number.isFinite(v) || v <= 0)) return
    commit(marks)
    setMarks(
      marks.map((m) => {
        if (m.id !== id) return m
        const style: AnnoStyle = { ...(m.style ?? {}) }
        if (v === undefined) delete style[key]
        else style[key] = v
        const next: Mark = { ...m }
        if (style.radius === undefined && style.stroke === undefined && style.badge === undefined) {
          delete next.style
        } else {
          next.style = style
        }
        return next
      }),
    )
  }

  const onKey = (e: React.KeyboardEvent) => {
    /*
     * ★★★ **第一道闸：来路判定**（2026-10-16 作者报的最严重那条）——
     * 「我想删数字的时候，他把这整一条都删了，整个框框这一项都删了」。
     *
     * 根因：这一整套快捷键挂在**页根**（React `onKeyDown` 冒泡到这里），数字格 / 备注格里
     * 敲退格、删除、⌘Z 都会冒上来被当成"删整个框 / 撤销框"。判据看的是 **事件从哪儿来**
     * （`e.target` 是不是可编辑的），不是"焦点在哪一个变量"—— 与项目「判据不许两个源」同一条。
     *
     * 在输入控件里，键盘**完全交还给那个控件**：退格只删那个数字、⌘Z 走浏览器自己的
     * 文本撤销（改完按 Enter 或点开别处即提交，见各输入的 onBlur）。
     */
    const el = e.target as HTMLElement | null
    if (el !== null) {
      const tag = el.tagName
      if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT' || el.isContentEditable) return
    }

    const mod = e.metaKey || e.ctrlKey
    if (mod && e.key.toLowerCase() === 'z') {
      e.preventDefault()
      e.stopPropagation()
      if (e.shiftKey) redo()
      else undo()
      return
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && sel !== null) {
      e.preventDefault()
      e.stopPropagation()
      removeBox(sel)
      return
    }
    if (e.key === 'Escape') {
      setSel(null)
      return
    }
    const step = e.shiftKey ? 10 : 1
    const arrows: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    }
    const d = arrows[e.key]
    if (d !== undefined && sel !== null) {
      e.preventDefault()
      e.stopPropagation()
      nudge(d[0], d[1])
    }
  }

  const onFile = (e: React.ChangeEvent<HTMLInputElement>) => {
    const f = e.target.files?.[0]
    e.target.value = ''
    if (f === undefined) return
    if (objectUrlRef.current !== null) URL.revokeObjectURL(objectUrlRef.current)
    const url = URL.createObjectURL(f)
    objectUrlRef.current = url
    /* 本地文件不在台账里 ⇒ 不进任何组（它自己一份源稿） */
    pick({
      key: `file:${f.name}`,
      groupId: null,
      groupName: null,
      label: f.name,
      path: f.name,
      url,
      /* 本地文件没有台账那条成品图 ⇒ 客户视角对它退化成原图 */
      renderedUrl: null,
      w: 0,
      h: 0,
    })
    toasts.push(`打开本地图片：${f.name}（只在这台机器这一趟里）`)
  }

  const doCopy = (text: string, what: string) => {
    void copyText(text).then((ok) => toasts.push(ok ? `已复制${what}` : '复制不了 —— 剪贴板被挡了，试试「导入…」那格旁边手选'))
  }

  const doImport = () => {
    try {
      const list = parseMarks(importText)
      commit(marks)
      setMarks(list)
      setSel(null)
      setImportOpen(false)
      setImportText('')
      toasts.push(`读进来 ${list.length} 个框`)
    } catch (err) {
      toasts.push(`这不是导出的那份：${String(err instanceof Error ? err.message : err)}`)
    }
  }

  /* ---------- 渲染 ---------- */

  const selMark = marks.find((m) => m.id === sel) ?? null
  const selPx = selMark !== null && src !== null ? pxOf(selMark, src.w, src.h) : null
  /* 面板那四个格按 left/top/width/height 排，`pxOf` 给的是 x/y/w/h —— 这里对一次名 */
  const selVal = selPx === null ? null : { left: selPx.x, top: selPx.y, width: selPx.w, height: selPx.h }
  const pct = (v: number, dim: number) => (dim === 0 ? '—' : `${round1((v / dim) * 100)}%`)

  return (
    <div className={s.wrap} tabIndex={-1} ref={wrapRef} onKeyDown={onKey}>
      <div className={s.bar}>
        <span className={s.lbl}>图</span>
        <select
          className={`${c.sel} ${s.sel}`}
          value={src?.key ?? ''}
          onChange={(e) => {
            const id = e.target.value
            const a = imgAssets.find((x) => x.id === id)
            if (a !== undefined) {
              pick({
                key: a.id,
                groupId: a.groupId,
                groupName: a.groupName,
                label: a.display,
                path: a.path,
                url: assetUrl(a.url),
                renderedUrl: assetUrl(a.url),
                w: 0,
                h: 0,
              })
            }
          }}
          title="资产根里的图（台账登记过的都在）—— 同一组 = 共用一份标注，切过去框原样在"
        >
          {src !== null && !imgAssets.some((a) => a.id === src.key) && <option value={src.key}>{src.label}</option>}
          {/* 单张的那些（组里只有它自己）平摆 */}
          {groups
            .filter((g) => g.items.length === 1)
            .map((g) => (
              <option key={g.items[0].id} value={g.items[0].id}>
                {g.items[0].display}
              </option>
            ))}
          {/* 成组的那些收进 `<optgroup>` 里 —— **一组的名字写在组头上**，一眼看出"这两张是一套" */}
          {groups
            .filter((g) => g.items.length > 1)
            .map((g) => (
              <optgroup key={g.key} label={`${g.name}（一组 ${g.items.length} 张 · 共用一份标注）`}>
                {g.items.map((a) => (
                  <option key={a.id} value={a.id}>
                    {a.display}
                  </option>
                ))}
              </optgroup>
            ))}
        </select>
        <button type="button" className={`${c.btn} ${c.btnSm}`} onClick={() => fileRef.current?.click()}>
          本地文件…
        </button>
        <input ref={fileRef} type="file" accept="image/*" className={s.file} onChange={onFile} />

        <span className={s.sep} />
        <button
          type="button"
          className={`${c.btn} ${c.btnSm} ${grid ? c.btnOn : ''}`}
          onClick={() => setGrid((v) => !v)}
          title="每 50px 一条线（作者量坐标时用的就是这个栅格）"
        >
          网格 50
        </button>
        <button
          type="button"
          className={`${c.btn} ${c.btnSm} ${snap ? c.btnOn : ''}`}
          onClick={() => setSnap((v) => !v)}
          title="拖的时候边角吸到 50 的整倍数；关着就是 1px 一格"
        >
          吸附 50
        </button>
        {/*
          * ★★ **视图切换器**（2026-10-16 第二版）：三段**互斥**，切档只换显示内容 ——
          * 缩放 / 位置 / 尺寸一律不动，两档之间来回切才比得出差别（作者：
          * 「把在看效果和客户视角加一个切换器，我加切换器这个它的其他的都不变了」）。
          */}
        <div className={s.viewSeg} role="group" aria-label="视图">
          {(
            [
              ['edit', '编辑', '可拖框、可改数：网格与手柄都在（再点一下 = 什么都不叠）'],
              ['preview', '看效果', '藏起网格与手柄，只看框画上去什么效果（再点一下 = 什么都不叠）'],
              [
                'client',
                '客户视角',
                '客户端里那张成品图：框已经烧进像素、前端不再叠画（再点一下 = 什么都不叠）',
              ],
            ] as const
          ).map(([k, label, tip]) => (
            <button
              key={k}
              type="button"
              className={`${c.btn} ${c.btnSm} ${view === k ? c.btnOn : ''}`}
              /* ★ 再点一次当前这一档 = **反选**：三颗都不亮，只看那张原图（作者 2026-10-17 要的） */
              onClick={() => setView(view === k ? 'none' : k)}
              data-tip={tip}
            >
              {label}
            </button>
          ))}
        </div>

        {/* 撑开：下面的"适应 / 100% / 生成预览 / 比例"靠右排，坐标那一条压在**这一行的行尾** */}
        <span className={c.grow} />
        <span className={s.sep} />
        {/*
          * ★★ 「适应」也要有**选中态**（2026-10-16 作者：「给那个适应加一个选中的那个背景呀，
          * 这样你就知道我有没有点适应了」）—— 判据是 `zoom === 'fit'`（它本身就是这一页的默认档）。
          */}
        <button
          type="button"
          className={`${c.btn} ${c.btnSm} ${zoom === 'fit' ? c.btnOn : ''}`}
          onClick={() => setZoom('fit')}
          title="把整张图缩到刚好放得下 —— 也是这一页打开时的默认视角"
        >
          适应
        </button>
        <button
          type="button"
          className={`${c.btn} ${c.btnSm} ${zoom === 1 ? c.btnOn : ''}`}
          onClick={() => setZoom(1)}
          title="图的原始像素尺寸（1 图像素 = 1 屏幕像素）"
        >
          100%
        </button>

        {/* 「生成预览」= 拍一张照、不存相册（作者 2026-10-16）：烘一张成品图先看一眼，满意再点保存 */}
        <button
          type="button"
          className={`${c.btn} ${c.btnSm}`}
          disabled={src === null || src.w === 0 || src.key.startsWith('file:') || rendering}
          data-tip={
            src?.key.startsWith('file:')
              ? '本地图片没有台账条目，烘不了成品图'
              : '把当前这些框烘成一张成品图先看一眼（临时的，满意再点保存）'
          }
          onClick={makePreview}
        >
          {rendering ? '生成中…' : '生成预览'}
        </button>
        <span className={`${s.zoom} ${c.tnum}`}>{Math.round(scale * 100)}%</span>
        {/*
         * ★★ **坐标那条压在行尾**（2026-10-17 作者：「这个坐标固定在第二行的右侧吧」）——
         *   它放在这一行的**最后一项**、`.readout` 自带 `margin-left:auto` ⇒ 无论前面挤成
         *   几行，它都贴在**它所在那一行的右端**（宽窗口时就是第一行行尾）。
         */}
        <span className={`${s.readout} ${c.tnum}`}>
          {cursor === null || src === null
            ? '把鼠标放图上看坐标'
            : `x ${Math.round(cursor.x)} · y ${Math.round(cursor.y)} (${pct(cursor.x, src.w)}, ${pct(cursor.y, src.h)})`}
        </span>
      </div>

      <div className={s.body}>
        {/* 看效果 / 客户视角 / 全不叠 这三档都算"在看"（底换成客户端模态框那个面） */}
        <div className={s.stage} data-preview={view === 'edit' ? 'off' : 'on'} ref={setStageEl}>
          {src === null ? (
            <p className={s.hint}>正在读资产清单…</p>
          ) : (
            <div className={s.pad}>
              <div
                className={s.board}
                ref={boardRef}
                style={
                  {
                    width: src.w * scale,
                    height: src.h * scale,
                    /*
                     * ★★★ **画布上的框与编号按"成品图像素 × 当前缩放"画**（2026-10-16 第二版，
                     * 作者要的是 PS 那条：画布上怎么样显示，最终效果就怎么样、零差别）。
                     *
                     * `--zoom-k` 是现成的机制：`.mark` / `.badge` 都把屏幕尺寸**除以它**
                     * （客户端拿它做"放大看图时视觉大小恒定"）。这里喂的值是
                     * `参考比例 ÷ 当前缩放` ⇒ 除下来正好是 `当前缩放 ÷ 参考比例` 那一档：
                     * 框线 = `2 × z / ref`、编号 = `18 × z / ref` 屏幕像素。
                     *
                     * 而成品图正是按 `ref` 烧的（框线在图上占 `2 / ref` 个像素）——
                     * 它被放大到 `z` 看时也正好是 `2 × z / ref`。**两边逐像素一致。**
                     *
                     * ★ 只有框与编号跟着它走（`.mark` / `.badge`）；手柄、网格、对比层都是
                     *   **工具自己的东西**，不进成品图，所以照旧恒定大小。
                     */
                    /*
                     * ★ `--zoom-k` = 画布比例（适应 / 100%）那一层；**看图缩放那一层的
                     *   `--view-k` 不写在这儿** —— 它由 `rAF` 直接写 DOM（见 `viewApply`），
                     *   写进 React 的 style 就得靠重渲染更新，等于把卡顿请回来。
                     */
                    '--zoom-k': `${refScale(src.w) / (scale || 1)}`,
                    /* 拖的过程中光标说清"抓着"；客户视角常显"可抓"（CSS `.board` 默认画框十字） */
                    cursor: viewPan ? 'grabbing' : clientView ? 'grab' : undefined,
                  } as CSSProperties
                }
                /*
                 * ★ 中键按压拖动 = **平移**（2026-10-17 作者：「鼠标可以中键按压拖动？」）。
                 *   **四个档位通用**（这套缩放/平移是画布的，不是某一档的）；左键仍是画框。
                 *   `preventDefault` 挡掉浏览器 / WebKit 的**中键自动滚动**（不然一边拖一边自己飞）。
                 *   与触控板两指滑同一条账：都只改 `viewPending` 的 `x`/`y`，由 rAF 落笔。
                 */
                onPointerDown={(e) => {
                  if (e.button === 1) {
                    e.preventDefault()
                    const cur = viewRef.current
                    viewPanRef.current = {
                      id: e.pointerId,
                      x: e.clientX,
                      y: e.clientY,
                      ox: cur.x,
                      oy: cur.y,
                    }
                    e.currentTarget.setPointerCapture(e.pointerId)
                    setViewPan(true)
                    return
                  }
                  onBoardDown(e)
                }}
                onPointerMove={(e) => {
                  const pan = viewPanRef.current
                  if (pan !== null && pan.id === e.pointerId) {
                    viewRef.current = {
                      k: viewRef.current.k,
                      x: pan.ox + (e.clientX - pan.x),
                      y: pan.oy + (e.clientY - pan.y),
                    }
                    viewSchedule()
                    return
                  }
                  const p = toImg(e)
                  if (p !== null) setCursor(p)
                }}
                onPointerUp={(e) => {
                  if (viewPanRef.current?.id === e.pointerId) {
                    viewPanRef.current = null
                    setViewPan(false)
                  }
                }}
                onPointerCancel={() => {
                  viewPanRef.current = null
                  setViewPan(false)
                }}
                onPointerLeave={() => setCursor(null)}
                /* 双击 = 复位（缩放与位置都回到 100%）—— 四个档位同一条 */
                onDoubleClick={() => {
                  viewRef.current = { k: 1, x: 0, y: 0 }
                  viewSchedule()
                }}
              >
                <img
                  className={s.img}
                  /* ★ 客户视角 = **成品图**：刚拍过的**预览**优先；没有预览就取仓库那份
                     （＋破缓存戳）；否则 = 原图（框由下面那些 div 画） */
                  src={
                    clientView && previewUrl !== null
                      ? previewUrl
                      : clientView && src.renderedUrl !== null
                        ? `${src.renderedUrl}${src.renderedUrl.includes('?') ? '&' : '?'}v=${renderStamp}`
                        : src.url
                  }
                  alt={src.label}
                  draggable={false}
                  onLoad={(e) => {
                    const el = e.currentTarget
                    setSrc((prev) =>
                      prev === null || (prev.w === el.naturalWidth && prev.h === el.naturalHeight)
                        ? prev
                        : { ...prev, w: el.naturalWidth, h: el.naturalHeight },
                    )
                  }}
                />
                {grid && view === 'edit' && (
                  <span className={s.grid} style={{ backgroundSize: `${GRID * scale}px ${GRID * scale}px` }} />
                )}
                {/*
                 * ★ 客户视角**不叠画**（成品图里已经有框了）—— 再画一层就是双份。
                 * ★ 「全不叠」那一档也一个都不画 —— 就是拿来看"原本的图片"的。
                 */}
                {showMarks &&
                  src.w > 0 &&
                  marks.map((m, i) => {
                    if (m.off) return null
                    const p = pxOf(m, src.w, src.h)
                    const on = m.id === sel
                    return (
                      <div
                        key={m.id}
                        className={s.mark}
                        data-on={on ? 'yes' : 'no'}
                        style={{
                          left: p.x * scale,
                          top: p.y * scale,
                          width: p.w * scale,
                          height: p.h * scale,
                          /* 框的样子随这一条走（缺省 = CSS 里的 `var(…, 默认)`，与客户端同值） */
                          ...styleVars(m.style),
                        }}
                        onPointerDown={(e) => startDrag(e, m, 'move')}
                      >
                        <span className={s.badge}>{i + 1}</span>
                        {/* 手柄只在「编辑」档露出来（看效果/客户视角/全不叠都是"只看"） */}
                        {view === 'edit' &&
                          (['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'] as const).map((edge) => (
                            <span
                              key={edge}
                              className={s.handle}
                              data-edge={edge}
                              onPointerDown={(e) => startDrag(e, m, 'resize', edge)}
                            />
                          ))}
                      </div>
                    )
                  })}
                {/*
                  * 冲突对比层（作者：「我能不能预览一下这两个呢……看看到底是新不新」）——
                  * 图上再叠两层只读框：**仓库那份**（灰蓝、虚线）与**本机那份**（琥珀、虚线），
                  * 与正在编辑的实框并排看，一眼看出差在哪。不改任何数据，纯展示。
                */}
                {showMarks && diffOpen && conflict !== null && src.w > 0 && (
                  <>
                    {repoMarks !== null &&
                      repoMarks.map((m, i) => {
                        const p = pxOf(m, src.w, src.h)
                        return (
                          <div
                            key={`repo-${i}`}
                            className={s.diffMark}
                            data-which="repo"
                            style={{
                              left: p.x * scale,
                              top: p.y * scale,
                              width: p.w * scale,
                              height: p.h * scale,
                            }}
                          >
                            <span className={s.diffTag}>仓库</span>
                          </div>
                        )
                      })}
                    {conflict.marks.map((m, i) => {
                      const p = pxOf(m, src.w, src.h)
                      return (
                        <div
                          key={`local-${i}`}
                          className={s.diffMark}
                          data-which="local"
                          style={{
                            left: p.x * scale,
                            top: p.y * scale,
                            width: p.w * scale,
                            height: p.h * scale,
                          }}
                        >
                          <span className={s.diffTag}>本机</span>
                        </div>
                      )
                    })}
                  </>
                )}
              </div>
            </div>
          )}
        </div>

        <aside
          className={s.panel}
          /* 点面板**空白** = 收起编辑区（与点画布空白、点列表空白同一条手感） */
          onClick={(e) => {
            if (e.target === e.currentTarget) setSel(null)
          }}
        >
          {/*
            * 这张图 / 这次预览那一条说明 —— **从画布底下搬到面板顶上**（2026-10-16 作者：
            * 「改变位置呀，放到别的地方」）。
            *
            * 它原来是画布下面那行 `.foot`：跟着图一起滚、图一缩放就换地方，读不出"到底变没变"。
            * 面板这一栏与画布并排、位置恒定，读起来才稳。
            * ★ 顺手去掉原文里**字面上露出来的那对 `**`**（markdown 的写法，当普通文字输出时会
            *   把星号原样显示出来），也去掉了时间戳（每次生成预览都跳 —— 正是"变来变去"那份）。
            */}
          <div className={s.info}>
            {src === null ? (
              '正在读资产清单…'
            ) : (
              <>
                {src.path}
                {src.w > 0 && ` · ${src.w} × ${src.h}`}
                {sibling !== null && ` · 同组还有：${sibling.path}（共用这份标注）`}
                {clientView
                  ? previewUrl !== null
                    ? ' · 客户视角：这张是刚生成的预览（临时的 —— 满意就点保存）· 捏合缩放 / 两指拖动 / 中键拖动 / 双击复位'
                    : ' · 客户视角：这张就是客户端看到的成品图（框已烧进像素）· 捏合缩放 / 两指拖动 / 中键拖动 / 双击复位'
                  : preview
                    ? ' · 看效果（网格与手柄都藏起来了）'
                    : view === 'none' && ' · 什么都没叠：这张就是那张原图'}
              </>
            )}
          </div>

          {/*
            * 源稿那一行：**这一页的正事**（作者 2026-10-16）
            * 「那一个源文件……我希望它是随着这个项目仓库的」—— 存哪儿必须一直摆在眼前。
            *
            * ★ 这里**只说"住在哪儿"**（脏不脏、保存/放弃都在页头那一槽，别两处重复）：
            *   那个小圆点还留着，它按 `data-dirty` 换色，一眼看出这一份动过没有。
            */}
          <div className={s.repoBar} data-dirty={dirty ? 'yes' : 'no'}>
            <span className={s.repoDot} />
            <span className={s.repoText}>
              {src !== null && src.key.startsWith('file:') ? (
                '本地文件：仓库不管它（只能「复制 TS」）'
              ) : repo.path === '' ? (
                '正在读源稿…'
              ) : (
                <span className={s.dim}>{repo.path}</span>
              )}
            </span>
          </div>

          {conflict !== null && (
            <div className={s.conflict}>
              <span>
                {repo.doc === null
                  ? '本机还留着一份草稿，还没进仓库'
                  : '本机那份草稿比仓库里那份新'}
                （{new Date(conflict.savedAt).toLocaleString()}）
              </span>
              <span className={s.conflictOps}>
                <button
                  type="button"
                  className={`${c.btn} ${c.btnSm} ${diffOpen ? c.btnOn : ''}`}
                  onClick={() => setDiffOpen((v) => !v)}
                  title="在图上把两版的框并排画出来，看清差在哪 —— 不猜，看准了再选"
                >
                  {diffOpen ? '收起对比' : '看看两份差在哪'}
                </button>
                <button
                  type="button"
                  className={`${c.btn} ${c.btnSm}`}
                  onClick={() => {
                    applySaved(conflict)
                    setConflict(null)
                    setDiffOpen(false)
                    toasts.push('已换成你本机那份 —— 记得点「保存」')
                  }}
                >
                  用本机那份
                </button>
                <button
                  type="button"
                  className={`${c.btn} ${c.btnSm}`}
                  onClick={() => {
                    if (src !== null) clearStore(groupKeyOf(src))
                    setConflict(null)
                    setDiffOpen(false)
                    toasts.push('本机那份已丢掉')
                  }}
                >
                  丢掉它
                </button>
              </span>
            </div>
          )}

          <div className={s.pHead}>
            <span>图层</span>
            <span className={s.dim}>{marks.length} 个框</span>
            <span className={c.grow} />
            <button type="button" className={`${c.btn} ${c.btnSm}`} onClick={addBox} disabled={src === null}>
              ＋ 新框
            </button>
          </div>

          <div
            className={s.list}
            /* 点列表**空白**（没落在某一行上）= 收起下面那块编辑区（作者点名的那个手感） */
            onClick={(e) => {
              if (e.target === e.currentTarget) setSel(null)
            }}
          >
            {marks.length === 0 ? (
              <p className={s.empty}>
                还没有框 —— <b>在图上按住拖一下</b>就画出来了（框里拖 = 挪，边上的方点 = 拉，
                方向键 = 微调，⌘Z = 撤销）
              </p>
            ) : (
              marks.map((m, i) => {
                const p = src === null ? null : pxOf(m, src.w, src.h)
                return (
                  <div
                    key={m.id}
                    className={s.row}
                    data-on={m.id === sel ? 'yes' : 'no'}
                    onClick={() => setSel(m.id)}
                    role="button"
                    tabIndex={0}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault()
                        setSel(m.id)
                      }
                    }}
                  >
                    <span className={s.rowBadge}>{i + 1}</span>
                    <span className={s.rowName} data-tip={m.note}>
                      {m.note.trim() === '' ? <em className={s.dim}>未命名</em> : m.note}
                    </span>
                    <span className={`${s.rowSize} ${c.tnum}`}>
                      {p === null ? '—' : `${Math.round(p.w)}×${Math.round(p.h)}`}
                    </span>
                    <button
                      type="button"
                      className={s.iconBtn}
                      title="往上挪一位（编号跟着走）"
                      disabled={i === 0}
                      onClick={(e) => {
                        e.stopPropagation()
                        moveRow(i, -1)
                      }}
                    >
                      <Icon name="chevronDown" size={12} className={ui.caretUp} />
                    </button>
                    <button
                      type="button"
                      className={s.iconBtn}
                      title="往下挪一位（编号跟着走）"
                      disabled={i === marks.length - 1}
                      onClick={(e) => {
                        e.stopPropagation()
                        moveRow(i, 1)
                      }}
                    >
                      <Icon name="chevronDown" size={12} />
                    </button>
                    <button
                      type="button"
                      className={s.iconBtn}
                      data-tip={m.off ? '显示这个框' : '先藏起来（还留着）'}
                      onClick={(e) => {
                        e.stopPropagation()
                        toggleOff(m.id)
                      }}
                    >
                      {m.off ? '◌' : '◉'}
                    </button>
                    <button
                      type="button"
                      className={s.iconBtn}
                      title="删掉这个框"
                      onClick={(e) => {
                        e.stopPropagation()
                        removeBox(m.id)
                      }}
                    >
                      ×
                    </button>
                  </div>
                )
              })
            )}
          </div>

          <div className={s.edit}>
            {selMark === null || selVal === null || src === null ? (
              <p className={s.empty}>点一个框（图上或这张列表里）看它的坐标</p>
            ) : (
              <>
                <div className={s.editHead}>
                  <span>框 {marks.findIndex((m) => m.id === selMark.id) + 1}</span>
                  <span className={s.dim}>
                    {round1(selMark.at.left)}, {round1(selMark.at.top)} · {round1(selMark.at.width)} ×{' '}
                    {round1(selMark.at.height)} %
                  </span>
                </div>
                <div className={s.nums}>
                  {K_KEYS.map((k) => (
                    <label key={k} className={s.num}>
                      <span className={s.numLbl}>{K_LABEL[k]}</span>
                      <input
                        /* 非受控 + key 跟着真值走：拖的时候数字跟着变，打字的时候不被重排打回来 */
                        key={`${selMark.id}-${k}-${Math.round(selVal[k])}`}
                        className={`${c.inp} ${c.tnum} ${s.numInp}`}
                        type="number"
                        defaultValue={Math.round(selVal[k])}
                        title="单位：图的原生像素（点进来光标在数字右侧，退格只改这个数字）"
                        onKeyDown={onNumKeyDown}
                        onFocus={onNumFocus}
                        onBlur={(e) => {
                          const v = Number(e.target.value)
                          if (Number.isFinite(v)) setField(k, v)
                        }}
                      />
                    </label>
                  ))}
                </div>
                <label className={s.noteRow}>
                  <span className={s.numLbl}>备注</span>
                  <input
                    className={c.inp}
                    value={selMark.note}
                    placeholder="给这一步起个名（只写进导出那行注释）"
                    /* 备注也是可编辑区：止住按键、别冒到页根；改动同样进撤销栈（第一次改前先压一格） */
                    onKeyDown={(e) => e.stopPropagation()}
                    onFocus={() => noteBaseRef.current = marks}
                    onChange={(e) => {
                      if (noteBaseRef.current !== null) {
                        commit(noteBaseRef.current)
                        noteBaseRef.current = null
                      }
                      setMarks(marks.map((m) => (m.id === selMark.id ? { ...m, note: e.target.value } : m)))
                    }}
                    onBlur={() => {
                      noteBaseRef.current = null
                    }}
                  />
                </label>
                {/*
                  * 「框的样子」（作者：「有没有图层啊？我可以编辑之类的，比如说改这个框框的圆角这一些」）
                  * ⇒ 圆角 / 线宽 / 编号直径三个数。留空 = 用站点缺省（8 / 2 / 18），
                  *    填了才写进源稿（并随「复制 TS」带给客户端）。
                  * ★ 这三个变量名与客户端 `GuideImage.module.css` **同名同缺省** —— 工作台
                  *   import 不到客户端 CSS，靠这条守口径；改一边就得改另一边。
                  */}
                <div className={s.nums}>
                  {STYLE_KEYS.map(({ key, label, def }) => (
                    <label key={key} className={s.num}>
                      <span className={s.numLbl}>{label}</span>
                      <input
                        key={`${selMark.id}-${key}-${selMark.style?.[key] ?? def}`}
                        className={`${c.inp} ${c.tnum} ${s.numInp}`}
                        type="number"
                        min={1}
                        defaultValue={selMark.style?.[key] ?? def}
                        data-tip={`单位 px；缺省 ${def}（留空 = 用缺省）`}
                        onKeyDown={onNumKeyDown}
                        onFocus={onNumFocus}
                        onBlur={(e) => setStyle(selMark.id, key, e.target.value)}
                      />
                    </label>
                  ))}
                </div>
              </>
            )}
          </div>

          {/*
            * 导出／导入（2026-10-16 作者：「还有这个复制这些，嗯，我不理解，我不知道怎么用」）
            * ⇒ **默认收成一行**，点开才见那三颗 + 一句人话说明。平时不占地方。
            */}
          <div className={s.io}>
            <button
              type="button"
              className={s.ioHead}
              aria-expanded={ioOpen}
              onClick={() => setIoOpen((v) => !v)}
              title="把框的坐标导出去（贴回客户端源码）／从 JSON 贴回来继续编 —— 平时用不到"
            >
              <Icon
                name="chevronDown"
                size={12}
                className={ui.disclosureCaret}
                data-on={ioOpen}
              />
              <span>导出／导入</span>
              <span className={s.dim}>给开发用</span>
            </button>
            {ioOpen && (
              <>
                <div className={s.ioRow}>
                  <button
                    type="button"
                    className={`${c.btn} ${c.btnSm}`}
                    disabled={marks.length === 0}
                    onClick={() => doCopy(jsonOf(marks), ' JSON')}
                  >
                    复制 JSON
                  </button>
                  <button
                    type="button"
                    className={`${c.btn} ${c.btnSm} ${c.btnPrimary}`}
                    disabled={marks.length === 0}
                    onClick={() => doCopy(tsOf(marks), ' TS（marks 那两行）')}
                    title="直接贴回 src/app/guide/openWithGuide.ts 的 marks"
                  >
                    复制 TS
                  </button>
                  <button
                    type="button"
                    className={`${c.btn} ${c.btnSm}`}
                    onClick={() => setImportOpen((v) => !v)}
                  >
                    导入…
                  </button>
                </div>
                {importOpen && (
                  <div className={s.importBox}>
                    <textarea
                      className={s.ta}
                      value={importText}
                      placeholder="把「复制 JSON」那份贴回来（按百分比读）"
                      onChange={(e) => setImportText(e.target.value)}
                    />
                    <div className={s.importOps}>
                      <button type="button" className={`${c.btn} ${c.btnSm}`} onClick={doImport}>
                        读进去
                      </button>
                      <span className={s.dim}>读进去会盖掉现在这几个框（可撤销）</span>
                    </div>
                  </div>
                )}
                <div className={s.tips}>
                  <b>复制 TS</b> = 把这份标注贴回客户端源码 <code>openWithGuide.ts</code> 里的{' '}
                  <code>marks</code>（<b>平时要的就是这一颗</b>）；<b>复制 JSON</b> 是它的原始数据、
                  <b>导入…</b> 把 JSON 贴回来接着编。坐标是<b>百分比</b>、浅 / 深两张图共用一份；
                  换图或重裁之后要重量（框跟的是图上的位置，图变了就飘）。
                </div>
              </>
            )}
          </div>
        </aside>
      </div>
    </div>
  )
}
