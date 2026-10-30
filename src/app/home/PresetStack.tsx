/*
 * 预设文件那一小块。
 *
 * 「读取失败」那一行的「重试」按钮去掉了：它点的是试验场调试面板的假下载开关
 * （devStore.setPresetPhase），不是真的重取。连带去掉只为它存在的 canAct 参数。
 *
 * ★ 2026-10-17：`waiting` 那一句原来写「连接云端…」。这里**一个字节都不联网** ——
 *   换 combo 时读的是三样本机的东西（这一版的文件清单 / 云端默认参数 / 你那一份副本），
 *   作者看到它以为在下载（「这时候我没有下载呀」），所以改成「正在读取…」。
 *
 * ★ 2026-10-13：那块**可选的三轴竖排**（`axes`，只在小窗档 `density === 'mini'` 时
 * 才摆出来）整条退役 —— 作者：「我之前设计的，在小窗的时候，它显示在下面。
 * 现在我觉得没必要了，它不要出现这个偏移值」。
 * 这一块从此只回答"这份预设文件是什么、是不是正在用的那一份"；三轴读数各有各的家
 * （首页校准卡板下/右侧那颗读数气泡、校准 tab 的下缘那条），不在这里复读。
 * 随它一起退役的还有只为它存在的那套取值（`view`：值 / 取数中 / 横杠）
 * 与 `.axes / .row / .label / .value` 四条样式。
 */

import { useEffect, useState } from 'react'
import { useT } from '../../i18n'
import type { StringKey } from '../../i18n'
import type { PresetState } from '../calib/usePreset'
import TraceTag from '../components/TraceTag'
import Skeleton from './Skeleton'
import s from './PresetStack.module.css'

/** 这一版下「我的副本」：一份本地副本 */
export interface PresetCopy {
  fileName: string
  /** 用户根相对路径（动作身份，与抽屉那颗「使用」同形） */
  path: string
  /** 后处理跑过几次（学习账；0 = 还没用过） */
  count: number
}

/**
 * 套餐里**除 MKP 预设之外**的那几份（今天 = 切片器配置，`presets/bundles.toml`
 * 的 `assetRefs` 里 `bbs_profile` 那一条）。
 *
 * ★ 2026-10-30（作者：「这一个它不仅是包含这个预设文件，它还包含了那个 JSON 对吧？
 *   我觉得这个首页应该把那个切片器的也显示出来」）：首页此前只摆 MKP 那一行，
 *   于是「已经下载过预设了，主按钮怎么还写着下载并使用」看起来自相矛盾 ——
 *   主按钮算的是**整个套餐**（MKP + 切片器配置），缺的是切片器那份，界面却看不见它。
 */
export interface BundleCompanion {
  fileName: string
  /**
   * 盘上现状。`unknown` = 两张单子（已下载 / 漂移）还没回来 —— 这时**不摆标签**，
   * 不拿"判不了"冒充"未下载"（纪律②）。
   */
  state: 'ready' | 'stale' | 'missing' | 'unknown'
}

/** 配套文件盘上现状 → 标签键（`unknown` 不摆标签，故不进这张表） */
const COMPANION_STATE_KEY: Record<Exclude<BundleCompanion['state'], 'unknown'>, StringKey> = {
  ready: 'presets.tree.cloudState.downloaded',
  stale: 'presets.tree.updateState.update',
  missing: 'presets.tree.cloudState.pending',
}

interface PresetStackProps {
  state: PresetState
  /**
   * 这一版套餐里那份预设的文件名 —— **读回来之前**就有的那一份。
   *
   * 来源 = 目录（`catalog.files[]` 的 `machineId/versionId` 倒查，`PageHome.fileOf`）：
   * 与 `getVersionFiles` 是同一件事实的同步读法，只是不用等一次 IPC。
   *
   * ★★ 为什么要有它（2026-10-17 第七轮，作者：「切换的时候就是会先多出来，再变少再变化
   *   默认那个名字」）：这一格里**名字决定行数** —— 「我的副本」要把与它同名、且就在
   *   `presets-local/` 根下的那份筛掉（见下面 `defaultCopyPath`）。名字还在路上时筛不了，
   *   于是先画全都行、回来了再少几行、顺带改掉第一行的名字：三拍。给了它，换一版那一帧
   *   就是新那一版的样子，一步到位。
   */
  fallbackName?: string | null
  /**
   * 「正在使用」的说明行（A3）。底账正指着这一份时给「正在使用」；
   * 不命中 / 没应用 = undefined。
   * 底账没说"正在使用"时这块**什么都不标** —— 卡片只回答"这份 combo 的预设文件是什么"。
   *
   * 2026-10-13：「正在使用 · 我的文件」那个来源后缀去掉（作者：「我的一份、官方一份」（现称交付预设）
   * 这些相关的都去掉」），不分来源只说「正在使用」（见 PageHome 的 inUseNote）。
   */
  inUse?: string
  /**
   * **这一版下我自己的那些副本**（2026-10-17 晚，作者：「现在只显示默认的套餐里面的
   * 不是很爽…要都显示出来，套餐的始终和其他的分开」「这下面还有显示那些可以选的」）。
   *
   * 套餐那一份永远单独一行（带「套餐」标签），副本排在它下面、**按后处理次数降序**
   * （用得多的在前，就是"学习"的结果）；空数组 = 这一版没有副本，只显示套餐那行。
   */
  copies?: PresetCopy[]
  /** 底账正指着的那份（path）—— 「我的副本」的行上标「正在使用」 */
  activePath?: string | null
  /**
   * 第一行（默认那份）是不是正在用的那份。
   *
   * 默认行 = 套餐里那份；但应用之后实际生效的是 `presets-local/` 里**同名**的那一份
   * （`fetchCloudPreset` 落的，作者 2026-10-17：「那就是正的，就是本体」）——
   * 所以这一行在用 **两种** 事实之一：底账指着云端原件，或指着本地同名那份。
   */
  defaultActive?: boolean
  /**
   * 当前**选中**的那份：`null` = 选的是套餐那份。
   * 选中只改这一格与底部「应用」的目标，**不立刻切换**（向导里的按钮才切）。
   */
  pickedPath?: string | null
  onPick?: (copy: PresetCopy | null) => void
  /** 套餐里 MKP 之外的配套文件（切片器配置）。空 = 不渲染这一组 */
  companions?: BundleCompanion[]
  /**
   * 「默认」那份的现状：**交付面 + 本机工作副本**一起看（判据在 PageHome 的
   * `defaultFileState`，与主按钮同一份）。`null` / `'unknown'` = 判不了 —— 不摆标签。
   *
   * ★ 2026-10-30 作者：把 `presets-local/` 里那份删了，行上什么状态都没有 ⇒ 看不出
   *   "默认这份还没下载"，主按钮也还写着「使用」。
   */
  defaultState?: BundleCompanion['state'] | null
}

/**
 * 预设文件那一小块：文件名 + 状态。
 *
 * ★ 2026-10-17：这一格显示的是**套餐里那份预设**（`preset.bundleName`），
 *   不是"我复制出来的那一份"。作者看截图：「他为什么显示复制出来的，而不是显示
 *   云端下载的、套餐里面的」—— 上一级选的就是**套餐**，这里却把个人工作副本的名字
 *   顶上来了（副本名由 `preset.name` 说，校准/编辑实际写的是它）。
 *   两句话各说各的：这里回答"这个套餐配的是哪份"，`name` 回答"现在在用哪份"。
 *
 * 只有非 ready 才有额外视觉。默认（ready）就是文件名直接摆着 ——
 * 真实情况是连上就一瞬间下完，给正常路径加过场动画反而是假的。
 */
export default function PresetStack({
  state,
  fallbackName = null,
  inUse,
  copies = [],
  activePath = null,
  defaultActive = false,
  pickedPath = null,
  onPick,
  companions = [],
  defaultState = null,
}: PresetStackProps) {
  const tr = useT()
  const ready = state.status === 'ready'
  const name = ready
    ? state.preset.bundleName
    : state.status === 'idle'
      ? null
      : /* 还没读回来：用目录里那份（同一件事实的同步读法）—— 名字一确定，行数就定了 */
        fallbackName
  const path = ready ? state.preset.bundlePath : undefined
  /* 有名字就能选（副本是本地扫描来的，选中只改这一格，不等 IPC） */
  const selectable = onPick !== undefined && name !== null
  /*
   * ★ 2026-10-17 晚（作者：「那就是正的，就是本体」「这个不要显示套餐，显示默认」）：
   *
   * 应用一份套餐之后，真正生效的是 `presets-local/` 里**同名**的那一份 —— 不是云端
   * 目录里那个原件。所以：
   *   · 第一行的标签写「默认」，不是「套餐」（它就是用户手里那份本体）；
   *   · 「我的副本」里**同名那份不再重复列**（语义与实际都是同一份，列两遍只会让人
   *     以为有两份文件）。
   * 只认**与套餐同名、且就在 `presets-local/` 根下**的那一份；放在子目录里的同名文件
   * 仍算另一份（用户自己分的文件夹）。
   *
   * ★★ 名字没读回来时走 `fallbackName`（见 prop 注释）—— 这一步**不许留空**：
   *   留空就是"先全列出来再收回去"，正是作者说的"先多出来，再变少"。
   */
  const defaultCopyPath = name === null || name === '' ? null : `presets-local/${name}`
  const ranked = copies
    .filter((c) => c.path !== defaultCopyPath)
    .sort((a, b) => b.count - a.count || a.fileName.localeCompare(b.fileName))
  /*
   * 有内容就画内容 —— `waiting` 也画。
   *
   * ★★ 2026-10-17 第七轮：原来这里在 `waiting` 期间**整块用上一版的样子**（`lastRef`
   *   快照），为的是不让这一格的高度塌下去。但它把"哪一版的名字配哪一版的副本"这件事
   *   交给了时间：旧名字 + 新副本（或反过来）都能画出来，作者连拍到的"先多出来、再变少、
   *   再改名字"就是这么来的。现在两头都同步了（名字来自目录、副本来自本地扫描），
   *   不需要快照：换一版当帧就是新那一版，高度只跟着**真实内容**变一次。
   */
  const hasContent = name !== null || ranked.length > 0

  /*
   * **「正在读取…」要慢一点才出现**（2026-10-17 作者：「他总是一闪而过…要不然就久一点
   * 要不然就不显示？出了问题才显示」）。
   *
   * 换一版读的是本机三样东西（文件清单 / 默认参数 / 我那份副本），快的时候几十毫秒 ——
   * 每次切换都闪一块骨架，比不显示更烦。所以压 320ms：**真慢才亮**，正常切换一个字都不出现；
   * 失败仍然照常报（那条不受这个门控）。现在它是"连名字都还没有"那一档的兜底。
   */
  const waiting = state.status === 'waiting'
  const [showWaiting, setShowWaiting] = useState(false)
  useEffect(() => {
    if (!waiting) {
      setShowWaiting(false)
      return
    }
    const timer = window.setTimeout(() => setShowWaiting(true), 320)
    return () => window.clearTimeout(timer)
  }, [waiting])

  return (
    <div className={s.stack} data-status={state.status}>
      <p className={s.head}>{tr('home.stack.head')}</p>

      {hasContent ? (
        <>
          {selectable ? (
            <button
              type="button"
              className={s.pickRow}
              data-picked={pickedPath === null}
              data-tip={path}
              onClick={() => onPick?.(null)}
            >
              <span className={s.pickDot} aria-hidden="true" />
              <span className={s.pickName}>{name === '' ? '—' : name}</span>
              <span className={s.pickTail}>
                {defaultActive && <span className={s.pickTag}>{tr('home.stack.inUse')}</span>}
                {/* 已下载 / 未下载 / 有更新：与主按钮同一份判据（见 defaultState 注释） */}
                {defaultState !== null && defaultState !== 'unknown' && (
                  <span className={s.pickTag} data-state={defaultState}>
                    {tr(COMPANION_STATE_KEY[defaultState])}
                  </span>
                )}
                {/* 「默认」而不是「套餐」：这一行就是用户手里那份本体（见上面那段注释） */}
                <span className={s.pickTag} data-kind="default">
                  {tr('home.stack.default')}
                </span>
              </span>
            </button>
          ) : (
            <p className={s.name} data-tip={path}>
              {name === '' ? '—' : name}
            </p>
          )}

          {selectable && ranked.length > 0 && (
            <div className={s.copies}>
              <p className={s.groupHead}>{tr('home.stack.copies')}</p>
              {ranked.map((c) => (
                <button
                  key={c.path}
                  type="button"
                  className={s.pickRow}
                  data-picked={pickedPath === c.path}
                  data-tip={c.path}
                  onClick={() => onPick?.(c)}
                >
                  <span className={s.pickDot} aria-hidden="true" />
                  <span className={s.pickName}>{c.fileName}</span>
                  <span className={s.pickTail}>
                    {activePath === c.path && (
                      <span className={s.pickTag}>{tr('home.stack.inUse')}</span>
                    )}
                    {c.count > 0 && (
                      <span className={s.pickCount}>
                        {tr('home.stack.count', { count: c.count })}
                      </span>
                    )}
                  </span>
                </button>
              ))}
            </div>
          )}

          {/* 套餐里 MKP 之外的那几份（切片器配置）—— 主按钮说"下载并使用"时，
              缺的就是这里标着「未下载」的那一份（见 BundleCompanion 注释）。
              整块只读：它没有"选哪一份"这回事，套餐配的是哪份就是哪份 */}
          {companions.length > 0 && (
            <div className={s.companions}>
              <p className={s.groupHead}>{tr('home.stack.companions')}</p>
              {companions.map((c) => (
                <p key={c.fileName} className={s.companionRow} data-tip={c.fileName}>
                  <span className={s.companionName}>{c.fileName}</span>
                  {c.state !== 'unknown' && (
                    <span className={s.pickTag} data-state={c.state}>
                      {tr(COMPANION_STATE_KEY[c.state])}
                    </span>
                  )}
                </p>
              ))}
            </div>
          )}
        </>
      ) : showWaiting ? (
        <>
          <Skeleton width="min(160px, 100%)" label={tr('home.stack.reading')} />
          <p className={s.note}>{tr('home.stack.loading')}</p>
        </>
      ) : null}

      {/*
        A3：与预设页横幅同源的那句「正在使用」—— 底账说在用这份，这里就跟着说。
        ★ 2026-10-17：列表能选的时候**不重复这句** —— 行上已经有「正在使用」标签了
        （截图里两处一起出现，像在说两件事）。
      */}
      {inUse !== undefined && state.status !== 'waiting' && !selectable && (
        <p className={s.note}>{inUse}</p>
      )}

      {state.status === 'downloading' && (
        <span className={s.progress} role="img" aria-label={tr('home.stack.downloading')} />
      )}

      {/*
        只报状态，不给「重试」—— 试验场那颗按钮扳的是调试面板的假下载开关，不是真的重取。
        错误本身带着 traceId：产品要求它在错误面上露出来，否则日志里那条查不到人。
      */}
      {state.status === 'failed' && (
        <p className={s.note}>
          {tr('home.stack.failed')}
          {state.error && <TraceTag traceId={state.error.traceId} />}
        </p>
      )}
    </div>
  )
}
