/*
 * 右侧「详情」栏 —— 原来那条贴在窗口下缘的状态条（`BbsStatusBar`）搬过来的。
 *
 * 2026-10-13（作者点头的 04 那一档）：「底条整条退场，继承链 / 统计 / 说明搬成右边一栏」。
 * 理由是那几件事本来就不该占满整屏宽度：窗口小的时候它会自己换行、叠到三行，
 * 把参数区最下面那几行吃掉；而右边本来是空的。
 *
 * # 为什么叫「详情」不叫「口径」
 *
 * 作者：「口径文案改一下，不要叫口径两个字」。界面上这两个字换成「详情」
 * （工具条那颗按钮 + 这一栏的标题）；代码注释里留着的那批「口径」是当初给这套
 * 数字起的内部说法（"凭什么这么说"），不影响用户看到什么。
 *
 * # 每一格仍然在回答一个「凭什么」
 *
 *   改动 3 / 18     这份预设存了 18 项，其中 3 项与基准不同（判定见 sameValue）
 *   基准 412 项     链合并 ⊕ 出厂默认之后有多少个 key
 *   行数 …          这一档铺了多少行；「跟 BBS 一样」还收起了多少（细账在 title 里）
 *   来源 …          清单是实时读本机目录的；读不到也是这一格说（**降级不是错误，别用错误色**）
 *   继承 A → B → C  基准是怎么算出来的 —— 这一页最关键的一格，单独一段、允许换行
 *
 * # 它不再"永远在"
 *
 * 收起来靠**这一栏自己右上角那颗叉**（2026-10-13 第三轮，作者：「详情展开的时候，
 * 那个图标就不见了，取而代之的是详情面板，它右上角出现一个叉叉」）。
 * 收起来时参数区自己占满整宽，页签行右端那颗「详情」图标重新出现；**失败不跟着藏** ——
 * 有 `error` 时那枚图标右上角挂一颗红点（`.iconDot`，在 PageBbs 那边画的），
 * 点开就在这一栏里看全文（抽卡里定过的一条：错误不上锁）。
 *
 * # 窄窗是浮层
 *
 * 判据与左抽屉同一个 `NARROW_W`（见 PageBbs.tsx 的 railMode）：并排时它占一条竖栏，
 * 窄了就跟抽屉一样浮在内容上面（`data-mode='float'`），宽度仍然可以拖。
 */
import { useEffect, useState } from 'react'
import type { PanelStats } from './bbsPanel'
import type { BbsSourceMode, BbsSyncInfo, BbsViewMode } from './bbsTypes'
import type { BbsRail as RailSizing } from './useBbsRail'
import Icon from '../../components/Icon'
import s from './BbsRail.module.css'

interface Props {
  title: string
  isSystem: boolean
  modifiedCount: number
  storedCount: number
  editedCount: number
  baselineCount: number
  viewMode: BbsViewMode
  /**
   * 「跟 BBS 一样 / 全部参数」原来是工具条上那颗按钮，2026-10-13 搬进这一栏
   * （作者：「这个功能可以呀……应该放到详情里面吧」）—— 它与下面那格
   * 「行数 · 收起 N 行」本来就在说同一件事。
   */
  onViewMode: (next: BbsViewMode) => void
  stats: PanelStats
  rulesNote: string
  chain: string[]
  variantNote: string
  sourceMode: BbsSourceMode
  sourceRoot: string
  sourceNote: string
  userCount: number
  systemCount: number
  sync: BbsSyncInfo | null
  /** 搜参数那一格：`搜「support」· 命中 7 项（这一页 3）`。没在搜就是 null */
  search: string | null
  /** 一句话提示（链缺了 / 变体对不上 / support_style 被拉回默认…） */
  note: string
  /** 真问题（读文件失败、导入被拒） */
  error: string
  /**
   * 有选中一份预设吗（2026-10-13）。
   * 没有的时候左边那排画的是**骨架**（版面照铺、值空着），这一栏也不能撒谎 ——
   * 「改动 0 / 0 项」「基准 0 项」会被读成"这份预设是空的"，可事实是还没选。
   */
  hasPreset: boolean
  /**
   * 当前这一份是不是**我们自己的**（来自下载区，不在本机 BBS 目录里）。
   *
   * 2026-10-13 加：「来源」那一格得说对 —— MKP 档下写「本机 BBS · 用户 29 / 系统 233」
   * 是自相矛盾的（那一份根本不是从 BBS 目录读出来的）。
   */
  fromMkp: boolean
  /**
   * 收起这一栏（右上角那颗叉点的就是它，2026-10-13 第三轮加）。
   *
   * 为什么"关"挂在这一栏自己身上，而不是照旧拿那颗「详情」图标来回切：作者的原话是
   * 「详情展开的时候，那个图标就不见了，取而代之的是详情面板，它右上角出现一个叉叉」——
   * 开着的那一栏自己就是"详情"，关闭它的动作也就归它；那颗图标只在栏收着时替它站岗
   * （见 PageBbs.tsx 里 `!railOpen &&` 那一段）。
   */
  onClose: () => void
  /**
   * 正在重扫（清单与当前这一份的链）。
   *
   * 这一栏那几格**数字**在这期间会"滚"起来（老虎机），落定才显真值 ——
   * 作者 2026-10-13：「我希望你直接把它改成那种不是数字的……像老虎机一样的呀，
   * 数字在那跳跳跳，最后才落成那个真实的。」理由是原来那种"变成另一个真值再变回来"
   * 看着像**数据真的变了**，而滚动的占位只说明"这格在重算"。
   */
  refreshing: boolean
  /** 拖宽那一套（宽度、是否在拖、热区 props） */
  sizing: RailSizing
}

/**
 * 老虎机：把一串文字里的**每一段数字**换成乱跳的占位，文字部分原样留着。
 *
 * 两个讲究：
 *   · 位数与真值一样 —— 字宽不变，格子不会跟着抖（全站铁律：状态不许改变盒子宽度），
 *     而且看着就知道是"在滚"，不会误读成"这一格变成了另一个数"；
 *   · 用确定性的伪随机（`Math.sin` 那个老式子），**不用 `Math.random`** ——
 *     同一个 tick 渲染两次得到同一串字，重渲染不会让字形乱闪。
 *
 * 这不是"数据"，是刷新期间的一层皮；`refreshing` 一落，调用方直接给真值。
 */
function roulette(text: string, tick: number): string {
  let i = 0
  return text.replace(/\d+/g, (digits) => {
    i += 1
    const width = digits.length
    /* 1 位数从 0 起，其余不出现前导 0（"003" 那种一看就是假的） */
    const lo = width === 1 ? 0 : 10 ** (width - 1)
    const x = Math.abs(Math.sin((tick + 1) * (i * 12.9898 + 78.233)))
    return String(lo + Math.floor(x * (10 ** width - lo)))
  })
}

export default function BbsRail({
  title,
  isSystem,
  modifiedCount,
  storedCount,
  editedCount,
  baselineCount,
  viewMode,
  onViewMode,
  stats,
  rulesNote,
  chain,
  variantNote,
  sourceMode,
  sourceRoot,
  sourceNote,
  userCount,
  systemCount,
  sync,
  search,
  note,
  error,
  hasPreset,
  fromMkp,
  refreshing,
  onClose,
  sizing,
}: Props) {
  /*
   * 刷新那一下的"滚字"。90ms 是手感值：再快看不清那是数字、再慢就不像在滚。
   * 它只在 `refreshing` 期间活着 —— 一落定 interval 就拆掉，不留后台定时器。
   */
  const [tick, setTick] = useState(0)
  useEffect(() => {
    if (!refreshing) return undefined
    const id = window.setInterval(() => setTick((n) => n + 1), 90)
    return () => window.clearInterval(id)
  }, [refreshing])
  /** 刷新期间把一格里的数字滚起来；不刷新时原样返回 */
  const roll = (text: string) => (refreshing ? roulette(text, tick) : text)

  const rows = viewMode === 'bbs'
    ? `${stats.rows} 行 · 收起 ${stats.hiddenByRule}`
      + (stats.greyed ? ` · 置灰 ${stats.greyed}` : '')
      + (stats.pagesHidden ? ` · 隐藏 ${stats.pagesHidden} 页` : '')
    : `全部参数 · ${stats.rows} 行`

  /* 「来源」要说的是**这一份从哪儿来**（不是"清单从哪儿来"）：MKP 档下那一份是交付面给的。
     两个名字与抽屉那两颗胶囊同一个词（MKP 工艺 / 本机工艺）—— 界面上别一会儿一个说法 */
  const source = fromMkp
    ? 'MKP 工艺 · 预设页「切片器配置」下载到本机的那几份'
    : sourceMode === 'live'
      ? `本机工艺 · 用户 ${userCount} / 系统 ${systemCount}`
      : `本机工艺（目录没读到）· 用户 ${userCount} / 系统 ${systemCount}`

  const stamp = sync?.bbs_version
    ? `BBS ${sync.bbs_version}${sync.syncedAt ? ` · 元数据抄于 ${sync.syncedAt.slice(0, 10)}` : ''}`
    : '没有元数据印章（public/bbs/_sync.json 缺失）'

  return (
    /*
     * 它**永远是并排的**（作者 2026-10-13：「右侧的那个详情呀，我不想让它悬浮起来，
     * 还是不要悬浮起来了」）—— 所以这里没有 `data-mode`，样式里也没有浮层那半套。
     * 要恢复的话：给它加回 `position: absolute` + 页面按宽度传 mode（左抽屉就是那套）。
     *
     * `data-dragging` 只用来在拖的时候把那条 1px 细线显出来（与抽屉的 `.grip` 同一套）。
     */
    <aside
      className={s.rail}
      data-dragging={sizing.dragging ? 'true' : undefined}
      aria-label="详情"
    >
      {/* 热区是贴左缘的一条透明带，唯一的视觉变化是 cursor（共用那套手感） */}
      <span className={s.grip} {...sizing.gripProps} />

      {/*
        「收起」那颗叉：贴这一栏**右上角**（绝对定位，不进下面那列流式布局 ——
        它既不会把标题行顶高，也不会跟着 `.body` 的内容滚走，滚到哪儿都够得着）。

        它替的是页签行右端那颗「详情」图标：**28×28 的盒子、16px 的图标、离右边的距离、
        与页签行同一条中线** —— 四个数都与那颗对齐（逐一对上的理由写在 css 的 `.close` 那段）。
        栏一开一收，那颗件在同一个位置上"换一副面孔"，不往左右或上下跳。

        形状照工具条的图标按钮：平时透明地"消失"在这一栏上，悬停才现出底与描边。
        图形用现成的 `close`（两条交叉线），与搜索框那颗自己画的清空叉同一个图形语言。
      */}
      <button
        type="button"
        className={s.close}
        onClick={onClose}
        title="收起详情"
        aria-label="收起详情"
      >
        <Icon name="close" size={16} />
      </button>

      {/* 滚动挂在里面这层：外面那层得留可见溢出，不然贴边的热区被裁掉一半 */}
      <div className={s.body}>
        <div className={s.title}>详情</div>

        <dl className={s.kv}>
          <dt>版本</dt>
          <dd className={s.name} title={title}>{title}</dd>

          <dt>改动</dt>
          <dd>
            {roll(
              !hasPreset
                ? '—（还没选预设）'
                : isSystem
                  ? '—（系统预设就是基准）'
                  : `${modifiedCount} / ${storedCount} 项`
                    + (editedCount > 0 ? `（已改 ${editedCount} 处，只在内存里）` : ''),
            )}
          </dd>

          <dt>基准</dt>
          <dd>{roll(hasPreset ? `${baselineCount} 项` : '—')}</dd>

          {/* 「这些行要不要按 BBS 的规则收起来」—— 与「行数」是同一件事，摆在一起 */}
          <dt>显示</dt>
          <dd className={s.chips}>
            <button
              type="button"
              className={s.chip}
              data-on={viewMode === 'bbs'}
              title={`${rulesNote}；与 BBS 的界面看到的一致`}
              onClick={() => onViewMode('bbs')}
            >
              跟 BBS 一样
            </button>
            <button
              type="button"
              className={s.chip}
              data-on={viewMode === 'all'}
              title="把基准里有的行全铺出来（249 行那种）"
              onClick={() => onViewMode('all')}
            >
              全部参数
            </button>
          </dd>

          <dt>行数</dt>
          <dd>{roll(rows)}</dd>

          {search !== null && (
            <>
              <dt>搜索</dt>
              <dd className={s.search}>{search}</dd>
            </>
          )}

          <dt>来源</dt>
          <dd title={fromMkp
            ? '这一份是我们发的：预设页 → 切片器配置 → 云端 下载，不在本机 Bambu Studio 的预设目录里'
            : sourceRoot + (sourceNote ? `（${sourceNote}）` : '')}
          >
            {roll(source)}
          </dd>

          <dt>元数据</dt>
          <dd title="元数据（参数定义 / 版面 / 出厂默认）随本仓分发；清单才是实时读本机 BBS 目录的">
            {stamp}
          </dd>
        </dl>

        <div className={s.sub}>继承</div>
        {/*
          继承链**一行一节**，不再挤成一整串（2026-10-13）。
          原来是一条 `A → B → C` 的长串 + `word-break: break-all`，在这一栏里折成三行，
          而且断在词中间（`single_com` / `mo` / `n` 这样）—— 作者：「你看它会把它换行，
          换到下一行」。竖排之后每节自己一行、名字自己不长就断不了，
          读起来也正好是"从父到子"那条链。
        */}
        {chain.length ? (
          <ul className={s.chainList}>
            {chain.map((name, i) => (
              <li key={name}>
                {i > 0 && <span className={s.arrow} aria-hidden="true">→</span>}
                <span className={s.chainName}>{name}</span>
              </li>
            ))}
          </ul>
        ) : (
          <span className={s.chain} title={rulesNote}>—（没找到父预设，基准只有出厂默认）</span>
        )}

        {variantNote !== '' && <span className={s.chain} title={rulesNote}>{variantNote}</span>}

        {note !== '' && <span className={s.note}>{note}</span>}
        {error !== '' && <span className={s.error}>{error}</span>}
      </div>
    </aside>
  )
}
