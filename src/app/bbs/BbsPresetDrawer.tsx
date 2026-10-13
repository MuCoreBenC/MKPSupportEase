/*
 * 左侧抽屉：选哪一份预设。
 *
 * 上游那个是挂在预设名栏下面的下拉，这一稿改成常驻抽屉 —— 278 份系统预设 + 你自己的
 * 一堆用户预设，放在一个 70vh 的浮层里翻很难受，而这一页的主要动作就是「换一份看看」。
 *
 * 分组顺序照 BBS（PresetComboBoxes.cpp:1382-1387「BBS: move system to the end」）：
 * 导入的 → 用户配置 → 系统配置 → 其他机型 / 读不了的。
 *
 * 三种「不能点」要分清楚：
 *   - 坏文件：真的点不动（disabled），title 说为什么
 *   - 别的机型：**能点**（点了就切过去），只是显示得弱一点 —— 不然你会以为它坏了
 *   - 继承链中间层（fdm_process_*）：根本不进列表，BBS 的下拉里也没有
 */

import { useEffect, useMemo } from 'react'
import Icon from '../../components/Icon'
import { SelectField } from '../../components/field'
import BbsSearchBox from './BbsSearchBox'
import { MODEL_ALL, groupsFor, inPickSource, targetsOf } from './bbsSource'
import type { BbsPickSource } from './bbsSource'
import type { BbsDrawer } from './useBbsDrawer'
import type { BbsPresetItem } from './bbsTypes'
import s from './BbsPresetDrawer.module.css'

interface Props {
  presets: BbsPresetItem[]
  currentKey: string | null
  dirty: boolean
  target: string
  query: string
  /**
   * 看哪一边（2026-10-13）：
   *   `mkp`  我们自己的 —— 预设页下载到本机的那几份切片器配置
   *   `bbs`  本机 BBS 目录里的 —— 用户配置 + 系统配置
   *
   * 列表、机型下拉的选项、空态说法都跟着它走。默认由页面给 `mkp`。
   */
  source: BbsPickSource
  onSource: (next: BbsPickSource) => void
  /** 收起时整个不渲染（display:none） */
  open: boolean
  /**
   * 'side' 并排 · 'float' 浮层。
   * **切形态的按钮在顶部工具条上，不在这里** —— 浮层收起时抽屉整个不在了，
   * 按钮跟着消失就再也切不回并排（探针里真撞上了这一下）。
   */
  mode: 'side' | 'float'
  /** 拖宽那一套（热区的 props 与是否正在拖） */
  sizing: BbsDrawer
  onTarget: (next: string) => void
  onQuery: (next: string) => void
  onPick: (key: string) => void
  /**
   * 重扫本机（2026-10-13 从工具条搬进来的）：它是"刷新这个列表"，
   * 与机型筛选是同一层的事。`null` = 这一档没有这个动作（比如读不到本机目录时）。
   */
  rescan: (() => void) | null
  /**
   * 正在读清单（2026-10-13）。
   *
   * ★ 这个状态属于**这个列表**，不属于右边那排参数 —— 作者：「这个刷新的我不喜欢，
   * 它刷新的会去顶我这里面的东西。我这里面的东西高度在变化……它应该是刷新左侧的列表，
   * 而不是说刷新它这一个右侧的参数吧，这种感觉你能理解吗？」
   *
   * 所以它只在这里表态，而且**一个字都不往流里加**（免得把项顶下去）：
   * 头部下沿扫一条 2px 的细线、列表项重铺一遍；列表空的时候（第一次读）
   * 才把那句提示当列表内容画出来。
   *
   * ★ 2026-10-13 第二轮：这个量还含"当前这一份的链正在重读"（见 `PageBbs` 的 `refreshing`），
   *   因为那一下左侧也该有反应。那只 ↻ **不再旋转**了 —— 作者嫌转得丑、还转不完整，
   *   现在它只跟着变色（见 css 的 `.iconBtn[data-loading]`）。
   */
  loading: boolean
  /**
   * 我们自己的切片器配置：目录里登记了几份、本机已经下到几份。
   *
   * 只用来**说话**（MKP 档的空态：「一份都还没下载，去预设页那一栏把它弄下来」）——
   * 正文由页面读出来灌进清单（见 `PageBbs` 里读盘那一段），这一层不碰盘。
   */
  slicer: { listed: number; ready: number }
}

export default function BbsPresetDrawer({
  presets,
  currentKey,
  dirty,
  target,
  query,
  source,
  onSource,
  open,
  mode,
  sizing,
  onTarget,
  onQuery,
  onPick,
  rescan,
  loading,
  slicer,
}: Props) {
  /*
   * 这一档里的项：列表分组与**机型下拉的选项**都从这儿来 —— 切到 MKP 之后下拉里就只剩
   * 那几台机。作者 2026-10-13：「这个下拉菜单它就不是说显示这么多的，要我们切换到了
   * 显示本机，它才显示这么多。你看这几百个哪里够人选？」
   */
  const scoped = useMemo(() => presets.filter((p) => inPickSource(p, source)), [presets, source])
  const targets = useMemo(() => targetsOf(scoped), [scoped])
  const groups = useMemo(
    () => groupsFor(presets, { target, currentKey, dirty, query, source }),
    [presets, target, currentKey, dirty, query, source],
  )
  const liveCount = scoped.filter((p) => !p.isAbstract).length

  /* 当前档位不在列表里（比如导入的文件认不出机型）→ 退到「全部」，别显示一个空列表 */
  const targetValue = targets.some((t) => t.key === target) ? target : MODEL_ALL

  /*
   * 下拉上写着「全部（9）」、列表却还在按上一次那台机筛 —— 那是两种说法对不上。
   * 换档（或换机型集合）之后目标档位可能在新的一档里根本不存在，比如从本机的 H2D
   * 切到「MKP 预设」，而 MKP 那九份里没有 H2D —— 那时两边都得走到「全部」上去：
   * 上面对下拉做了兜底显示，这里把**真正在筛的那个值**补齐，两处才一致。
   */
  useEffect(() => {
    if (targetValue !== target) onTarget(targetValue)
  }, [targetValue, target, onTarget])

  /*
   * 机型下拉的选项（2026-10-13：原生 `<select>` 换成自绘的共用件 `SelectField`）。
   *
   * 换的理由是作者看着那个系统弹层说「这个下拉菜单我不喜欢啊，样式也得改一下，
   * 用我们自己的风格去写吧」—— 原生弹层的字号、勾选、高度全归浏览器，两套皮肤
   * 一起走不了。共用件本身就是为这件事写的（`SelectField` 的文件头：
   * 「全仓库一个原生 `<select>` 都没有」—— 这一页这两个是漏网的）。
   *
   * 文本一字不改地搬过来，包括「（全部喷嘴）」那一档的缩进：缩进用全角空格的转义写法 ——
   * 选项是 nowrap 的一行，普通空格会被折叠掉。
   */
  const targetOptions = useMemo(
    () => [
      { value: MODEL_ALL, label: `全部（${liveCount}）` },
      ...targets.map((t) => ({
        value: t.key,
        label: t.level === 'nozzle' ? `\u3000${t.label}（${t.count}）` : `${t.label}（${t.count}）`,
      })),
    ],
    [targets, liveCount],
  )

  return (
    <aside
      className={s.drawer}
      data-open={open}
      data-mode={mode}
      data-dragging={sizing.dragging ? 'true' : undefined}
      aria-label="预设列表"
    >
      {/* `data-loading` 给头部那条扫动细线用（见 css）—— 读清单时它扫起来 */}
      <div className={s.head} data-loading={loading ? 'true' : undefined}>
        {/*
          两档来源（2026-10-13 作者定）：**MKP 工艺** ↔ **本机工艺**。

          作者：「最优先的是显示我们自己的 MKP 的……我希望有一个切换呀，是可以去看本机的、
          可以去看系统的、一个就是看本软件的，而且看系统的那个一般没那么明显，不要优先显示
          它的。」—— 系统那一档并进了「本机工艺」：它们本来就在同一个目录里，分成两个入口
          只是把"这一份是我的还是 BBL 的"变成一次点击；真正要紧的分界是「我们自己发的」
          与「你这台机器上原有的那几百份」。默认停在 MKP 工艺（由页面给）。

          ★ 这两个名字是作者逐字定的，别自己发挥：
            「MKP 预设」不行 —— 会跟 MKP 自己那份 **toml** 预设撞名（那是机器配置，
            与工艺预设是两回事）；
            「本机 BBS」也不行 —— 这一页里 BBS 是内部黑话，界面上不该出现。
            定的是：左「MKP 工艺」、右「本机工艺」（两边都落在"工艺"这个词上，
            这一页的页签本来也叫「工艺」）。
        */}
        <div className={s.sourceTabs} role="tablist" aria-label="预设来源">
          <button
            type="button"
            role="tab"
            className={s.sourceTab}
            data-on={source === 'mkp'}
            aria-selected={source === 'mkp'}
            title="我们发的那几份工艺：预设页「切片器配置 → 云端」下载到本机的"
            onClick={() => onSource('mkp')}
          >
            MKP 工艺
          </button>
          <button
            type="button"
            role="tab"
            className={s.sourceTab}
            data-on={source === 'bbs'}
            aria-selected={source === 'bbs'}
            title="你这台机器上原有的：本机 Bambu Studio 里的预设（你自己存的 + 软件自带的）"
            onClick={() => onSource('bbs')}
          >
            本机工艺
          </button>
        </div>

        <div className={s.headRow}>
          <span className={s.pickWrap}>
            <SelectField
              label="按机型与喷嘴筛选"
              value={targetValue}
              options={targetOptions}
              onChange={onTarget}
            />
          </span>

          {/*
            重扫本机（2026-10-13 从工具条搬进来）：它刷新的是**这个列表**，所以跟机型筛选排在一行。
            平时是四条字，这里收成一枚图标 + title —— 抽屉本来就只有 280 宽。
          */}
          {rescan !== null && (
            <button
              type="button"
              className={s.iconBtn}
              /* 读清单时它转起来 —— 这是"正在刷新这个列表"唯一的动效，不占一格位置 */
              data-loading={loading ? 'true' : undefined}
              title="重扫本机：重新读一遍本机 Bambu Studio 的预设目录（刚在 BBS 里新建的工艺预设会出现在这个列表里）"
              aria-label="重扫本机"
              onClick={rescan}
            >
              <Icon name="refresh" size={15} />
            </button>
          )}
        </div>

        <BbsSearchBox
          value={query}
          placeholder="搜索预设名…"
          label="搜索预设名"
          variant="fill"
          onChange={onQuery}
        />

        {/*
          这里原来有一格「载入随包配置」下拉 —— 2026-10-13 退役。
          它要你**一份一份手动载入**才看得见，与作者要的「最优先显示我们自己的」正好相反：
          那几份现在由页面在进页时整批读出来进清单（`PageBbs` 里读盘那一段），MKP 档直接列。
          「一份都没下载」该说什么交给下面列表的空态 —— 那儿分得清"没下载"与"搜不到"。
        */}
      </div>

      {/*
        data-role 是给验收探针用的稳定钩子（选择器不随 class 哈希变）。
        `data-refreshing` 是"重扫本机"那一下的列表重铺动画（见 css 里 `.list[data-refreshing]`）：
        读清单期间挂上，读完摘掉 —— 动画自己收干净。
      */}
      <div
        className={s.list}
        data-role="preset-list"
        data-refreshing={loading ? 'true' : undefined}
        aria-busy={loading || undefined}
      >
        {/*
          列表空的时候才把"正在读清单…"这话画出来（第一次进来那一下）。
          列表里已经有项时**一个字都不加** —— 那句提示会把整列项顶下去，
          正是作者说的「会去顶我这里面的东西」；那时候由上面那只转着的 ↻ 说话。
        */}
        {!groups.length && (
          <p className={s.empty}>
            {loading
              ? '正在读清单…'
              : query
                ? `没有名字含「${query}」的预设`
                : source === 'mkp'
                  ? (slicer.ready > 0
                    /* 盘上有、却一份都没进清单 —— 只有"正文读不出来"这一种可能，
                       别把话说成"还没下载"（那是在撒谎）。
                       浏览器预览里就是这一档：MKP 的切片器配置在演示里能读（见
                       mockServer/slicerPreset.ts），MKP 的 toml 则照旧读不到。 */
                    ? '这几份工艺读不出正文（浏览器预览里没有下载区）—— 用桌面版能看到它们'
                    : slicer.listed > 0
                      ? `MKP 的工艺还没下载（资源区里有 ${slicer.listed} 份）—— `
                        + '去「预设」页 → 切片器配置 → 云端，下载对应机型的那几份，回来就在这儿'
                      : '产品资源区里没有 MKP 的工艺')
                  : '这个机型下没有预设'}
          </p>
        )}
        {groups.map((g) => (
          <div key={g.title}>
            <div className={s.group} data-weak={g.weak ? 'true' : undefined}>
              {g.title}（{g.items.length}）
            </div>
            {g.items.map((it) => (
              <button
                key={it.key}
                type="button"
                className={s.item}
                data-on={it.current}
                data-off-model={it.offModel ? 'true' : undefined}
                disabled={it.disabled}
                title={it.disabled ? (it.reason ?? '') : `${it.printer} · ${it.nozzle} nozzle · ${it.file}`}
                onClick={() => onPick(it.key)}
              >
                <span className={s.tick}>{it.current ? '✓' : ''}</span>
                {it.star && <span className={s.star}>*</span>}
                <span className={s.name}>{it.label}</span>
              </button>
            ))}
          </div>
        ))}
      </div>

      {/*
        拖宽的热区：抽屉右边缘 6px 的透明带，**外观上什么都没有**，只有 cursor 变化
        （文件管理器那种「悬停就能拖」）。键盘聚焦时才显一条 1px 细线。
      */}
      <span className={s.grip} {...sizing.gripProps} />
    </aside>
  )
}
