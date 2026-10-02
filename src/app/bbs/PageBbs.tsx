/*
 * BBS 预设查看器 —— 页面装配。
 *
 * 搬自 `machine-motion/entries/bbs-preset`（那一页是纯静态 HTML + 908 行原生 JS）。
 * 这里只负责装配与交互，算法全在旁边那几个文件里：
 *
 *   bbsMerge        继承链、合并、改动判定（纯函数，逐行译自上游）
 *   bbsSource       清单归一、机型/喷嘴识别、分组（同上）
 *   bbsToggleRules  86 条条件显隐规则（译自 ConfigManipulation.cpp）
 *   bbsPanel        版面 + 值 → 页/组/行（上游是边算边建 DOM，这里先算清再画）
 *   useBbsData      取数与缓存（本机目录端点 + `public/bbs/` 那四份元数据）
 *   useBbsPreset    选中那一份摊开来是什么样
 *
 * # 主题为什么自带一套变量
 *
 * 上游是一页独立的深色界面，两套皮肤（深/浅）的取值是照 BBS 的 StateColor 抄的。
 * 本项目的 tokens.css 只有浅色一套。把 BBS 那两套塞进全局 token 会污染别的页面，
 * 所以**整套 --bbs-* 变量都定义在这一页的根节点上**（见 PageBbs.module.css），
 * 切主题只切根节点上的 data-theme，一行 DOM 都不用重建。
 *
 * # 这一页不写盘
 *
 * 改值、重置、还原都只动内存里的副本，刷新即还原 —— 上游也是这样。
 * `tools/dev-server/bbsFs.mjs` 那个端点是**只读**的，没有写回 BBS 的口子。
 */

import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { STORAGE } from '../../api/storageKeys'
import { useStickyState } from '../../hooks/useStickyState'
import type { Density } from '../../hooks/useDensity'
import BbsGroup from './BbsGroup'
import BbsPresetDrawer from './BbsPresetDrawer'
import BbsSearchBox from './BbsSearchBox'
import BbsStatusBar from './BbsStatusBar'
import BbsTabs from './BbsTabs'
import { buildBbsPanel, resolveActivePage } from './bbsPanel'
import { normalizeQuery } from './bbsSearch'
import { MODEL_ALL } from './bbsSource'
import { coverage } from './bbsToggleRules'
import { useSessionState } from '../shared/useSessionState'
import { useBbsData } from './useBbsData'
import { useBbsDelivered } from './useBbsDelivered'
import { useBbsDrawer } from './useBbsDrawer'
import { useBbsPreset } from './useBbsPreset'
import type { BbsPresetDoc, BbsViewMode } from './bbsTypes'
import s from './PageBbs.module.css'

interface Props {
  density: Density
  /** 预设页右键带过来的目标文件名；null = 直接从顶栏点进来的 */
  pending: { name: string; nonce: number } | null
}

const RULES = coverage()
const RULES_NOTE = `条件显隐规则 ${RULES.rules} 条、覆盖 ${RULES.keys} 个参数`
  + '（译自 ConfigManipulation.cpp 的 toggle_print_fff_options）'

export default function PageBbs({ density, pending }: Props) {
  const data = useBbsData()
  const preset = useBbsPreset(data)
  /* 产品资源区（mkp/bbs/）：第三圈起，这一页的产品配置从下载区来，不再是随包副本 */
  const delivered = useBbsDelivered()

  /* 落 localStorage 的偏好：档位与主题。键名全收在 `src/api/storageKeys.ts` */
  const [viewMode, setViewMode] = useStickyState<BbsViewMode>(STORAGE.clientBbsView, 'all')
  const [themeLight, setThemeLight] = useStickyState<boolean>(STORAGE.clientBbsTheme, false)
  /*
   * 地址参数 `?theme=light|dark` 照上游留着（截图对照时不想动记忆里的选择就用它）。
   * 优先级：地址参数 > 上次选择 > 深色，且**只作用于本次**，不覆盖 localStorage ——
   * 所以这里用一个独立的覆盖值，不去 setThemeLight。
   */
  const themeParam = useMemo(() => {
    const q = new URLSearchParams(window.location.search).get('theme')
    return q === 'light' ? true : q === 'dark' ? false : null
  }, [])
  const light = themeParam ?? themeLight
  /* 只记「这次会话看到哪」的：筛选档位、搜索词、当前页、抽屉开合 */
  const [target, setTarget] = useSessionState('bbs.target', MODEL_ALL)
  const [query, setQuery] = useSessionState('bbs.query', '')
  const [page, setPage] = useSessionState<string | null>('bbs.page', null)
  const [drawerOpen, setDrawerOpen] = useSessionState('bbs.drawer', true)
  /*
   * 搜参数的词。**换预设、换档位都保留** —— 「同一个参数在另一台机器上是多少」
   * 是这一页最常见的用法，每次换预设都被清空会很烦。
   */
  const [paramQuery, setParamQuery] = useSessionState('bbs.paramQuery', '')

  /*
   * 抽屉的两种形态。**默认值按密度档给一次**（宽档并排 / 窄档浮层），
   * 之后只听用户的选择 —— useStickyState 只在 localStorage 里没有值时才用 initial，
   * 所以这个「按档给默认」天然只发生一次，不会每次换窗口大小就把你的选择顶掉。
   */
  const wide = density === 'ultra' || density === 'wide'
  const [drawerMode, setDrawerMode] = useStickyState<'side' | 'float'>(
    STORAGE.clientBbsDrawerMode,
    wide ? 'side' : 'float',
  )
  /** 宽度、拖动状态、热区那一套 props */
  const sizing = useBbsDrawer()

  /* 展示模式：只能看、点不动。**默认开着** —— 这一页的正事是「看 BBS 里是什么值」，
     能改是附带的（而且改了也不写回去）。 */
  const [showcase, setShowcase] = useState(true)
  const [dropping, setDropping] = useState(false)
  const [localNote, setLocalNote] = useState('')
  const fileRef = useRef<HTMLInputElement>(null)

  /* 选中项换了：筛选档位跟着它走，否则你会在一个筛不到它的列表里找不到当前那一项 */
  useEffect(() => {
    if (preset.current?.target) setTarget(preset.current.target)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preset.currentKey])

  /*
   * 预设页右键带过来的目标。按**文件名**在清单里找（两页的数据源不是同一份，
   * 不能互相塞对象）。找不到不弹错、不跳走 —— 停在默认那一份，在状态条说一句。
   */
  useEffect(() => {
    if (!pending || !data.presets.length) return
    const want = pending.name.toLowerCase()
    const hit = data.presets.find(
      (p) => p.file.toLowerCase() === want || p.name.toLowerCase() === want.replace(/\.json$/, ''),
    )
    if (hit?.selectable) {
      preset.pick(hit.key)
      setLocalNote('')
    } else {
      setLocalNote(
        `清单里没有「${pending.name}」${hit ? '（它不可选：' + (hit.reason ?? '') + '）' : ''}，`
        + '已停在默认预设。如果它是一份 BBS 工艺预设，用「导入 JSON」把它拖进来。',
      )
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending?.nonce, data.presets])

  /* 一份都没选中（清单空着）—— 版面一页也不铺。抽成布尔是为了当 useMemo 的依赖：
     直接写 `preset.current` 会被 react-hooks 判成「可变值当依赖」 */
  const hasCurrent = preset.current !== null

  const panel = useMemo(
    () => buildBbsPanel(
      /* 清单空的时候（读不到本机目录）若照版面算，算出来的是出厂默认摊的一堆行 ——
         状态条会报「249 行」而页面一行没画，自相矛盾 */
      hasCurrent ? data.layout : [],
      data.registry, preset.values, preset.user, preset.toggles, viewMode,
      paramQuery, preset.variant.index,
    ),
    [hasCurrent, data.layout, data.registry, preset.values, preset.user, preset.toggles, viewMode,
      paramQuery, preset.variant.index],
  )

  /** 归一化后的搜索词。空串 = 没在搜 */
  const q = useMemo(() => normalizeQuery(paramQuery), [paramQuery])
  const searching = q.length > 0

  /* 当前页被隐藏了（切档 / 换预设）就落到第一个还有内容的页，否则会看到一片空白 */
  const activeId = resolveActivePage(panel.pages, page)
  const active = panel.pages.find((p) => p.id === activeId) ?? null

  const modifiedSet = useMemo(() => new Set(preset.modifiedKeys), [preset.modifiedKeys])

  /*
   * 无头探针的落点，照上游的 `globalThis.__bbs`（bbs-preset.js:901）。
   * 只在 dev 挂，build 里这段被摇掉 —— 它是给 playwright 脚本读数字用的，
   * 不是产品功能，所以界面上没有任何东西依赖它。
   */
  useEffect(() => {
    if (!import.meta.env.DEV) return
    Object.assign(globalThis, {
      __bbs: {
        sourceMode: data.sourceMode,
        presets: data.presets.length,
        currentKey: preset.currentKey,
        chain: preset.chainNames,
        modified: preset.modifiedKeys,
        unknown: preset.unknownKeys,
        baseline: Object.keys(preset.baseline).length,
        stats: panel.stats,
        note: preset.note,
        error: preset.error,
      },
    })
  })

  const readFile = useCallback((file: File) => {
    if (!file.name.toLowerCase().endsWith('.json')) {
      setLocalNote(`${file.name}：只收 .json`)
      return
    }
    void file.text().then(
      (text) => {
        let doc: BbsPresetDoc
        try { doc = JSON.parse(text) as BbsPresetDoc }
        catch (err) {
          /* 解析失败整份拒绝，不做「部分收下」那种半吊子状态 */
          setLocalNote(`${file.name}：JSON 解析失败（${String(err)}），整份拒绝`)
          return
        }
        setLocalNote('')
        preset.importDoc(file.name, doc)
      },
      (err: unknown) => setLocalNote(`${file.name}：读不出来（${String(err)}）`),
    )
  }, [preset])

  const onDrop = (ev: React.DragEvent) => {
    ev.preventDefault()
    setDropping(false)
    const file = [...(ev.dataTransfer?.files ?? [])].find((f) => f.name.toLowerCase().endsWith('.json'))
    if (!file) { setLocalNote('拖进来的不是 .json 文件'); return }
    readFile(file)
  }

  const userCount = data.presets.filter((p) => p.scope === 'user' || p.scope === 'imported').length
  const systemCount = data.presets.filter((p) => p.scope === 'system' && !p.isAbstract).length
  const variantNote = preset.variant.reason === 'ok'
    ? ` · 变体 ${preset.variant.want}（第 ${preset.variant.index + 1} 个）`
    : ''

  /*
   * 抽屉开合。**不再按密度档分叉** —— 上一轮给窄档写的「横过来铺在参数区上面」
   * （`.body { flex-direction: column }`）是个错：抽屉只占左边那么宽，右边一片空白，
   * 参数被推到抽屉下面、又被 `overflow: hidden` 裁掉，等于看不见。
   * 现在并排永远是并排，窄窗口靠抽屉的 `max-width: 60%` 收窄自己。
   */
  const showDrawer = drawerOpen
  const toggleDrawer = () => setDrawerOpen(!drawerOpen)

  /* 浮层态：选完就走、Esc 关、点参数区也关 —— 浮层的意思就是用完收起 */
  const pickPreset = useCallback((key: string) => {
    preset.pick(key)
    if (drawerMode === 'float') setDrawerOpen(false)
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [preset.pick, drawerMode, setDrawerOpen])

  useEffect(() => {
    if (drawerMode !== 'float' || !drawerOpen) return
    const onKey = (ev: KeyboardEvent) => {
      if (ev.key === 'Escape') setDrawerOpen(false)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [drawerMode, drawerOpen, setDrawerOpen])

  /*
   * 切形态**只换形态，不动开合**。
   *
   * 上一轮这里写的是 `setDrawerOpen(next === 'side')`，理由是「切到浮层如果还开着，
   * 会突然盖住参数」。那个理由站不住：点「浮层」是换一种显示方式，不是关掉它 ——
   * 换完立刻看到新形态才对，突然收起只会让人以为按钮点错了。
   * 想收起就点「预设列表」，两件事各归各。
   */
  const toggleMode = () => setDrawerMode(drawerMode === 'side' ? 'float' : 'side')

  if (data.fatal) {
    return (
      <div className={s.page} data-density={density} data-theme={light ? 'light' : 'dark'}>
        <p className={s.blank}>{data.fatal}</p>
      </div>
    )
  }

  return (
    <div
      className={s.page}
      data-density={density}
      data-theme={light ? 'light' : 'dark'}
      data-dropping={dropping ? 'true' : undefined}
      /* 抽屉宽度以 CSS 变量下发，抽屉自己 `width: var(--bbs-drawer-w)` 吃它 */
      style={{ '--bbs-drawer-w': `${sizing.width}px` } as React.CSSProperties}
      onDragOver={(ev) => { ev.preventDefault(); setDropping(true) }}
      onDragLeave={() => setDropping(false)}
      onDrop={onDrop}
    >
      <div className={s.bar}>
        <button
          type="button"
          className={s.btn}
          data-on={showDrawer}
          onClick={toggleDrawer}
        >
          预设列表
        </button>

        {/*
          形态切换。**必须在工具条上**，不能放抽屉里：浮层收起时抽屉整个不渲染，
          按钮跟着消失就再也切不回并排。
        */}
        <button
          type="button"
          className={s.btn}
          title={drawerMode === 'side'
            ? '改成浮层：列表浮在参数上面，参数区占满整宽'
            : '改成并排：列表固定在左边，和参数各占一段'}
          onClick={toggleMode}
        >
          {drawerMode === 'side' ? '浮层' : '并排'}
        </button>

        <span className={s.name} title={preset.current?.file ?? ''}>{preset.title}</span>

        <button
          type="button"
          className={s.btn}
          title={RULES_NOTE}
          onClick={() => setViewMode(viewMode === 'bbs' ? 'all' : 'bbs')}
        >
          {viewMode === 'bbs' ? '跟 BBS 一样' : '全部参数'}
        </button>

        <button type="button" className={s.btn} onClick={() => setShowcase(!showcase)}>
          {showcase ? '展示模式' : '可改动'}
        </button>

        {!showcase && preset.editedKeys.size > 0 && (
          <button type="button" className={s.btn} onClick={preset.revertAll}>
            全部还原（{preset.editedKeys.size}）
          </button>
        )}

        <button type="button" className={s.btn} onClick={() => fileRef.current?.click()}>
          导入 JSON
        </button>
        <input
          ref={fileRef}
          className={s.file}
          type="file"
          accept=".json,application/json"
          onChange={(ev) => {
            const f = ev.target.files?.[0]
            if (f) readFile(f)
            ev.target.value = ''   // 同一个文件改了再导一次也要能触发
          }}
        />

        {data.sourceMode === 'live' && (
          <button type="button" className={s.btn} onClick={data.rescan}>重扫本机</button>
        )}

        {/*
          产品资源区。**只列已经下载的** —— 没下载的那几份不给入口，点了才是骗人。
          空的时候说清"目录里有几份、还没下载"，那是这一页第一次出现
          「外部资源要先下载」这件事，不能拿一个空下拉糊过去。
        */}
        {delivered.downloaded.length > 0 ? (
          <select
            className={s.pick}
            value=""
            title="从下载区 mkp/bbs/ 里载入一份产品自带的切片器配置"
            onChange={(ev) => {
              const fileName = ev.target.value
              ev.target.value = ''
              if (fileName === '') return
              void delivered.read(fileName).then(
                (doc) => preset.importDoc(fileName, doc),
                (err: unknown) => setLocalNote(`${fileName}：读不出来（${String(err)}）`),
              )
            }}
          >
            <option value="">载入产品配置…</option>
            {delivered.downloaded.map((f) => (
              <option key={f.fileName} value={f.fileName}>
                {f.machineId === '' ? f.fileName : `${f.machineId} · ${f.fileName}`}
              </option>
            ))}
          </select>
        ) : (
          <span
            className={s.pickNote}
            title={delivered.note !== '' ? delivered.note : '去「预设」页的「切片器配置」下载 BBS 配置'}
          >
            {delivered.listed.length > 0
              ? `产品配置 ${delivered.listed.length} 份，还没下载`
              : '产品资源区里没有 BBS 配置'}
          </span>
        )}

        <button
          type="button"
          className={s.btn}
          title={light ? '切回 BBS 的深色皮肤' : '切到 BBS 的明亮皮肤。也可以用地址参数 ?theme=light'}
          onClick={() => setThemeLight(!light)}
        >
          {light ? '深色' : '明亮'}
        </button>
      </div>

      <div className={s.body}>
        <BbsPresetDrawer
          presets={data.presets}
          currentKey={preset.currentKey}
          dirty={preset.dirty}
          target={target}
          query={query}
          open={showDrawer}
          mode={drawerMode}
          sizing={sizing}
          onTarget={setTarget}
          onQuery={setQuery}
          onPick={pickPreset}
        />

        {/* 浮层态的遮罩：点一下收起抽屉。它在抽屉之下、参数之上（z-index 15 vs 20） */}
        {drawerMode === 'float' && drawerOpen && (
          <button
            type="button"
            className={s.scrim}
            aria-label="关闭预设列表"
            onClick={() => setDrawerOpen(false)}
          />
        )}

        <div className={s.main}>
          <div className={s.tabsRow}>
            <BbsTabs
              pages={panel.pages}
              active={activeId}
              searching={searching}
              onChange={setPage}
            />
            <BbsSearchBox
              value={paramQuery}
              placeholder="搜参数（名字 / key / 值）…"
              label="在当前预设的参数里搜索"
              onChange={setParamQuery}
            />
          </div>

          {/* 命中全在别的页：列出来，**点了才切**（你说的不自动跳） */}
          {searching && panel.stats.hits > 0 && active?.hitCount === 0 && (
            <p className={s.hintBar}>
              这一页没有命中，命中在：
              {panel.pages.filter((p) => p.hitCount > 0).map((p) => (
                <button
                  key={p.id}
                  type="button"
                  className={s.hintLink}
                  onClick={() => setPage(p.id)}
                >
                  {p.title} {p.hitCount}
                </button>
              ))}
            </p>
          )}

          {/* 一个都没命中：照常显示全部行，不清空 —— 清空了你就没法确认「是真没有」 */}
          {searching && panel.stats.hits === 0 && (
            <p className={s.hintBar}>
              没有匹配「{paramQuery.trim()}」的参数
              {panel.stats.hiddenHits > 0
                && `（有 ${panel.stats.hiddenHits} 个在「跟 BBS 一样」收起的行里）`}
            </p>
          )}


          <div className={s.content} data-role="params">
            {data.loading && <p className={s.blank}>正在读预设清单…</p>}
            {/* 没有本机目录就是这里的空态：说清「为什么空」，不拿陈旧快照充数 */}
            {!data.loading && data.sourceMode === 'none' && (
              <p className={s.blank}>
                没读到本机 BBS 预设目录：{data.sourceNote}
                <br />
                这一页列的是你本机 Bambu Studio 里的工艺预设 —— 本版不打包预设快照。
                <br />
                装好 Bambu Studio（或它的 Beta），或者用「导入 JSON」把一份 .json 拖进来。
              </p>
            )}
            {!data.loading && data.sourceMode === 'live' && !active && (
              <p className={s.blank}>
                这一档下没有可显示的参数。切回「全部参数」，或者用「导入 JSON」拖一份进来。
              </p>
            )}
            {/* 行只在「真选中了一份」时画。清单空了（读不到本机目录）时 `current` 是 null ——
                那时再画，画的就是出厂默认摊出来的一堆行，与上面那句空态自相矛盾 */}
            {preset.current && active?.groups.map((g) => (
              <BbsGroup
                key={g.name}
                group={g}
                values={preset.values}
                baseline={preset.baseline}
                modifiedKeys={modifiedSet}
                variantIdx={preset.variant.index}
                icons={data.icons}
                readOnly={showcase}
                query={q}
                onChange={preset.setValue}
                onReset={preset.resetKey}
              />
            ))}
          </div>
        </div>
      </div>

      <BbsStatusBar
        title={preset.title}
        isSystem={preset.scope === 'system'}
        modifiedCount={preset.modifiedKeys.length}
        storedCount={Object.keys(preset.user).length}
        editedCount={preset.editedKeys.size}
        baselineCount={Object.keys(preset.baseline).length}
        viewMode={viewMode}
        stats={panel.stats}
        rulesNote={RULES_NOTE}
        chain={preset.chainNames}
        variantNote={variantNote}
        sourceMode={data.sourceMode}
        sourceRoot={data.sourceRoot}
        sourceNote={data.sourceNote}
        userCount={userCount}
        systemCount={systemCount}
        sync={data.sync}
        search={searching
          ? `搜「${paramQuery.trim()}」· 命中 ${panel.stats.hits} 项（这一页 ${active?.hitCount ?? 0}）`
          : null}
        note={[
          preset.note,
          localNote,
          preset.importError,
          /* 被规则收起的行里也有命中 —— 闷着不说会让人以为搜索漏了 */
          searching && panel.stats.hiddenHits > 0
            ? `另有 ${panel.stats.hiddenHits} 个命中在「跟 BBS 一样」收起的行里，切到「全部参数」能看到`
            : '',
        ].filter(Boolean).join(' · ')}
        error={preset.error}
      />
    </div>
  )
}
