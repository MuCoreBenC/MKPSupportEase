/*
 * 首页 —— 从 v029/pages/PageMachineV029 整份搬来的那一页（A31 上看到的就是它）。
 *
 * 这一轮的目的是**找回原样**，所以只做了三件事：
 *   1. 改名：文件名 / 组件名 / 引用的每一个件与 CSS Module 加 A44 后缀。
 *   2. 改 import：全部指向 A44 内部（`../calib/*`、`../ui/*`、`./*`），
 *      `src/calib/*.generated` 那份板子坐标是共享资产，照旧直接 import。
 *   3. 删掉 `src/dev/devStore` 那处调试钩子：`useDevState().fadeMs` 换成常量 FADE_MS。
 *
 * 数据（T10 起）：三级选择与「文件」那一级都消费**文件体系**（`useCatalog` 的
 * getMachines + getPresetFiles；取件与三轴走 `usePreset`）—— 手编表退场。
 * 作者点名的效果：「A1 就是 A1（A1.toml），P1S 的版本就是 LITE」。
 * 版面、动画、注释一行没动。
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from 'react'
import SlideDeck, { type DeckHandle, type Sheet } from './SlideDeck'
import MachinePicker, { type Option, type Selection } from './MachinePicker'
import { selectionFromActive } from './activeSelection'
import { uidOfFile, useCatalog } from './useCatalog'
import PresetStack from './PresetStack'
import CalibPlate from '../calib/CalibPlate'
import PresetPickerDrawer from '../params/PresetPickerDrawer'
import HeroFade from './HeroFade'
import CardFrame from './CardFrame'
import CalibHead from '../calib/CalibHead'
import PlateZoom from '../calib/PlateZoom'
import { HomeCardStack, StepRail } from './HomeGuide'
import CopyAction from './CopyAction'
import { Btn } from '../ui/Controls'
import { Modal } from '../ui/Modal'
import type { Density } from '../../hooks/useDensity'
import { useDevDefaults } from './devDefaults'
import { pickArt } from './heroArt'
import { useArtLayers } from './useArtLayers'
import {
  AXIS_ROWS,
  NEED_PRESET,
  Z_LEGEND,
  Z_TIP,
  axisText,
  xyHitLabel,
  zHitLabel,
} from '../calib/calibAxes'
import { useCalibration } from '../calib/useCalibration'
import { usePreset } from '../calib/usePreset'
/* 第五步那张合影：**界面自带素材**（不进 Catalog / Delivery），
   从 `src/app/assets/hero/` 走 vite 资源管线（import 回来带内容哈希） */
import heroPile from '../assets/hero/hero_pile.webp'
import heroPile2x from '../assets/hero/hero_pile@2x.webp'

import p from './PageHome.module.css'

const EMPTY: Selection = { brand: null, model: null, variant: null }

/** 板子的配色跟应用的模式对齐；等应用做了深色模式，这里换成跟随主题的那个值 */
const PLATE_THEME = '浅色'

/** 两张校准卡在 sheets 里的位置：按页拦截换页要认它们 */
const CALIB_Z_INDEX = 2
const CALIB_XY_INDEX = 3
const CALIB_MODEL_INDEX = 4

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

/**
 * 后处理脚本里那段可执行文件路径。契约里没有它（真值在桌面壳那一侧），
 * 先沿用原来那串模板；`--Toml` 后面的路径跟着**当前那份文件**走（T10）。
 */
const MKP_EXE = 'G:\\project\\mkp-ssr\\target\\debug\\mkp-ssr.exe'

/** 版本角标：契约给的是中文 tag，组件只收三档色调 —— 没配过的 tag 不给 tone（走 muted） */
const TAG_TONE: Record<string, Option['badgeTone']> = {
  推荐: 'rec',
  热门: 'hot',
  最新: 'new',
}

function Rows({ items }: { items: [string, string][] }) {
  return (
    <dl className={p.rows}>
      {items.map(([label, value]) => (
        <div className={p.row} key={label}>
          <dt className={p.label}>{label}</dt>
          <dd className={p.value}>{value}</dd>
        </div>
      ))}
    </dl>
  )
}

interface PageHomeProps {
  density: Density
  /** 本页是否是当前页签。常驻挂载后页签不再重挂，靠它在每次回到本页时对一次底账 */
  active?: boolean
}

export default function PageHome({ density, active }: PageHomeProps) {
  const deckRef = useRef<DeckHandle>(null)
  /* 分级揭示的淡入时长：面板里调（产品仓里是常量 FADE_MS） */
  const { fadeMs } = useDevDefaults()
  const [sel, setSel] = useState<Selection>(EMPTY)

  const [openModel, setOpenModel] = useState<string | null>(null)
  const [pending, setPending] = useState<{ from: number; to: number } | null>(null)
  /* 校准页的预设下拉要换一份、但本页有未保存草稿时，先把要换的那一份记下来等确认 */
  const [presetAsk, setPresetAsk] = useState<string | null>(null)
  /* 预设抽屉（与「校准」tab 同一颗）：pill 点开，从左缘出 */
  const [pickerOpen, setPickerOpen] = useState(false)
  // 保存 / 放弃之后自己发起的那一次跳转不该再被拦
  const bypass = useRef(false)

  /* ---------- 文件体系：三级清单与「文件」那一级都从这里来（T10） ---------- */
  const catalog = useCatalog()

  /*
   * T9/T10 联动（预设页 → 首页）：对准「正在使用的那一条」（唯一底账 mkp.a44.active）——
   * 预设页应用了哪一份，这里三级选择就反填成哪一台。active 的 machineId / versionId
   * 与选择器是**同一套 id**（不再有映射表）。
   * 常驻挂载后 tab 不再重挂（2026-10-05），改成每次回到本页对一次：对出的值与
   * 现状一致时画面不动，在预设页换过应用才真正换基准。
   */
  useEffect(() => {
    if (!active || catalog.machines.length === 0) return
    /* 底账走 IPC（run/active-preset.json），异步读；读不到当"没有"，不反填 */
    void selectionFromActive(catalog.machines).then((next) => {
      if (next !== null) setSel(next)
    })
  }, [active, catalog.machines])

  // ---------- 预设：选哪一份由 sel 定；取件、等待、失败三态都在 usePreset 里 ----------
  const preset = usePreset(sel)

  // ---------- 偏移：已保存的一份 + 板上点出来 / 手输出来的草稿 ----------
  /* 整块状态机搬进了 useCalibration —— 向导这几页与「校准」tab 用同一份实现。
     这里只解构成原来那些名字，JSX 与换页拦截一行没动 */
  const calib = useCalibration(preset)
  const {
    saved,
    draft,
    dirty,
    dirtyAxes,
    savedNote,
    canPick,
    view: axisView,
    zSelected,
    xySelected,
    pickZ,
    pickXY,
    typeAxis,
    revertAxis,
    resetAxis,
    clearAll,
    commitAll,
  } = calib

  // v005 那个 Modal 不认 Esc，这里补上：Esc = 取消
  useEffect(() => {
    if (openModel === null && pending === null && presetAsk === null) return
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape') return
      setOpenModel(null)
      setPending(null)
      setPresetAsk(null)
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [openModel, pending, presetAsk])

  /** 有未保存的改动就把换页拦下（不分页 —— 三轴在哪一页都能改），等弹窗里给答案 */
  const canLeave = useCallback(
    (from: number, to: number) => {
      if (bypass.current) {
        bypass.current = false
        return true
      }
      if (!dirty) return true
      setPending({ from, to })
      return false
    },
    [dirty],
  )

  const leaveTo = useCallback((target: number) => {
    bypass.current = true
    setPending(null)
    deckRef.current?.jumpTo(target)
  }, [])

  // 再点一次已选项 = 取消该级；取消或改选都清空下游
  const pick = useCallback((level: keyof Selection, id: string) => {
    setSel((prev) => {
      const next = prev[level] === id ? null : id
      if (level === 'brand') return { brand: next, model: null, variant: null }
      if (level === 'model') return { ...prev, model: next, variant: null }
      return { ...prev, variant: next }
    })
  }, [])

  /* ---------- 三级的清单：数据源是契约（getMachines），不再是手编表 ---------- */
  const brandOptions = useMemo<Option[]>(() => {
    const out: Option[] = []
    for (const m of catalog.machines) {
      if (!out.some((o) => o.id === m.brand)) out.push({ id: m.brand, label: m.brand })
    }
    return out
  }, [catalog.machines])

  const modelOptions = useMemo<Option[]>(
    () => catalog.machines.map((m) => ({ id: m.id, label: m.display })),
    [catalog.machines],
  )

  const variantOptions = useMemo<Option[]>(() => {
    const machine = catalog.machines.find((m) => m.id === sel.model)
    return (machine?.versions ?? []).map((v) => ({
      id: v.id,
      label: v.name,
      badge: v.tag,
      badgeTone: TAG_TONE[v.tag ?? ''],
      note: v.description,
    }))
  }, [catalog.machines, sel.model])

  /* 「文件」这一级：仓库里的 9 份 MKP，每份带 机型/版本 倒查（校准页的预设下拉用它） */
  const presetOptions = useMemo(
    () =>
      catalog.presets.flatMap((f) => {
        const uid = uidOfFile(f)
        return uid === null ? [] : [{ uid, name: f.fileName }]
      }),
    [catalog.presets],
  )

  /* 当前那份文件的 id（`机型/版本`）—— 预设下拉的选中值，也是「换没换」的比较基准 */
  const currentUid =
    sel.model !== null && sel.variant !== null ? `${sel.model}/${sel.variant}` : null

  /* 抽屉每一项自己的文件名（T11）—— 与「校准」tab 同一颗查询 */
  const fileOf = useCallback(
    (machineId: string, versionId: string) => {
      const hit = catalog.presets.find((f) =>
        f.usedByVersions.some((r) => r.machine === machineId && r.version === versionId),
      )
      return hit?.fileName ?? null
    },
    [catalog.presets],
  )

  /* 校准页的预设下拉：一份预设文件唯一对应一处「机型 + 版本」，所以「选文件」= 反填三级选择 ——
     于是回到第二页，品牌 / 机型 / 版本已经是这份文件对应的那一套，不用再手点一遍 */
  const applyPreset = useCallback(
    (uid: string) => {
      const [machineId, versionId] = uid.split('/')
      const machine = catalog.machines.find((m) => m.id === machineId)
      const version = machine?.versions.find((v) => v.id === versionId)
      if (machine === undefined || version === undefined) return
      setSel({ brand: machine.brand, model: machine.id, variant: version.id })
    },
    [catalog.machines],
  )

  /* 换预设会把本页草稿作废（草稿是相对上一份预设点出来的增量），所以先问一次。
     只给「取消 / 放弃改动并换」两条路：换了预设 saved 会被新预设的值覆盖，
     这时候提供「保存并换」是骗人的 —— 存下去的数立刻就被顶掉了 */
  const pickPreset = useCallback(
    (uid: string) => {
      if (!uid || uid === currentUid) return
      if (dirty) {
        setPresetAsk(uid)
        return
      }
      applyPreset(uid)
    },
    [applyPreset, currentUid, dirty],
  )

  const brandName = sel.brand
  const modelNode = catalog.machines.find((m) => m.id === sel.model)
  const modelName = modelNode?.display ?? null
  const variantName = modelNode?.versions.find((v) => v.id === sel.variant)?.name ?? null
  /* 大图：图位分层 品牌 → 机型 → 版本（版本缺则回落机型），URL 全部来自台账 path */
  const art = useMemo(
    () => pickArt(sel, modelNode, catalog.assets, catalog.brands),
    [sel, modelNode, catalog.assets, catalog.brands],
  )
  const { layers, settle, drop } = useArtLayers(art)

  // 三级齐全才有 toml / 偏移 / 脚本这些"某机型某版本"的产物
  const ready = Boolean(sel.brand && sel.model && sel.variant)
  /** 已经拿到的那一份预设；还在等 / 失败时为 null —— 不回退到 mock 里的默认那份 */
  const presetInfo = preset.status === 'ready' ? preset.preset : null
  /** 文件名在等待之外的几态都是已知的（知道要取哪一份），单独取出来 */
  const presetName =
    preset.status === 'ready'
      ? preset.preset.name
      : preset.status === 'idle'
        ? null
        : preset.name
  const entryLabel = modelName ? '选择版本' : '选择机型'
  const artAlt = [brandName, modelName].filter(Boolean).join(' ') || '未选择机型'

  const sheets: Sheet[] = useMemo(
    () => [
      {
        id: 'machine',
        node: sel.model ? (
          /* 不给眉标题：机型代号（A1_MINI）与下面那行大标题（A1 mini）说的是同一件事，
             写两遍是复读。去掉之后 data-bare-top 生效，整块内容还往上抬了一截 */
          <CardFrame
            navs={[
              { label: '更换机型', arrow: true, onClick: () => deckRef.current?.jumpTo(1) },
            ]}
          >
            <div className={p.info} data-scroll>
              <div className={p.ident}>
                {modelName && <h1 className={p.name}>{modelName}</h1>}

                <button
                  type="button"
                  className={p.variant}
                  onClick={() => deckRef.current?.jumpTo(1)}
                  title="点击更换品牌 / 机型 / 版本"
                >
                  {variantName ?? entryLabel}
                  <span aria-hidden="true">›</span>
                </button>

                {ready && (
                  <>
                    <p className={p.path} title={presetInfo?.path}>
                      {presetName ?? '—'}
                    </p>
                    {/* 脚本里的 --Toml 跟着当前那份文件走（T10）；还没取到就先不摆这颗按钮 */}
                    {presetInfo && (
                      <CopyAction
                        text={`"${MKP_EXE}" --Toml "${presetInfo.path}" --Gcode`}
                        label="复制后处理脚本"
                      />
                    )}
                  </>
                )}
              </div>

              {ready && (
                <Rows
                  items={[
                    ['X 偏移', axisText(saved, 'x')],
                    ['Y 偏移', axisText(saved, 'y')],
                    ['Z 偏移', axisText(saved, 'z')],
                    ['涂胶速度', presetInfo ? `${presetInfo.speed} mm/s` : '—'],
                  ]}
                />
              )}
            </div>

            <div className={p.slot}>
              {/* density 不再传给它：A31 那个入参只喂调试面板的「按档覆盖」开关（默认关着），
                  面板删了它也就没有意义 —— 尺寸照旧由窗口与曲线算 */}
              <HeroFade layers={layers} onSettle={settle} onDrop={drop} alt={artAlt} />
            </div>
          </CardFrame>
        ) : (
          /* 还没选机型：欢迎版面。主 CTA 就在内容区，所以这一态左下角不放胶囊。
             也不放眉标题 —— 标题自己就带 MKP，眉标题再写一遍是复读 */
          <CardFrame navs={[]} peekSafe>
            <div className={p.welcome} data-scroll>
              <div className={p.welcomeTop}>
                <div className={p.welcomeCopy}>
                  <h1 className={p.heroTitle}>快速上手 MKP</h1>
                  <p className={p.heroLead}>选择机型，完成基础配置与校准。</p>
                  <button
                    type="button"
                    className={p.cta}
                    onMouseDown={noFocus}
                    onClick={() => deckRef.current?.jumpTo(1)}
                  >
                    现在开始
                    <span aria-hidden="true">→</span>
                  </button>
                </div>

                <div className={p.welcomeArt}>
                  <HomeCardStack />
                </div>
              </div>

              <StepRail
                pickIndex={1}
                zIndex={CALIB_Z_INDEX}
                xyIndex={CALIB_XY_INDEX}
                modelIndex={CALIB_MODEL_INDEX}
                onGo={(i) => deckRef.current?.jumpTo(i)}
              />
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'picker',
        node: (
          /* 同样不给眉标题：卡里三组标题（品牌 / 机型 / 打印件版本）已经说明这是在选机型。
             去掉后 data-bare-top 生效，「品牌」正好抬到原来眉标题那行字的高度 */
          <CardFrame
            style={{ '--fade-ms': `${fadeMs}ms` } as CSSProperties}
            peekSafe
            navs={[{ label: '回主页', onClick: () => deckRef.current?.jumpTo(0) }]}
            actions={[
              {
                label: '下一步',
                arrow: true,
                primary: true,
                on: ready,
                onClick: () => deckRef.current?.jumpTo(CALIB_Z_INDEX),
              },
            ]}
          >
            <div className={`${p.info} ${p.picker}`} data-scroll>
              <MachinePicker
                sel={sel}
                onPick={pick}
                brands={brandOptions}
                models={modelOptions}
                variants={variantOptions}
              />

              {/* 预设文件那一块只在真的有一份预设可谈时才出现（选齐三级）。
                  原来是常驻的横杠占位，宽窗里就成了"左上角选了一行、中间凭空一块空占位"，
                  视觉锚点全落在没内容的地方。宁可出现时把列往下长一截。
                  三轴只在"没有右侧露出卡"（density mini）时一起摆出来 */}
              {preset.status !== 'idle' && (
                <div className={p.pickerPreset}>
                  <PresetStack state={preset} axes={density === 'mini' ? saved : undefined} />
                </div>
              )}
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'calib-z',
        node: (
          <CardFrame
            eyebrow="第三步 · Z 偏移校准"
            peekSafe
            tag={dirty ? '未保存' : savedNote ? '已保存' : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            /* 打开模型是这一步的主入口（作者 10-03：绿底给打开模型，预设 pill 退暗） */
            topAction={{
              label: '打开模型',
              accent: true,
              onClick: () => setOpenModel('Z 偏移校准板'),
            }}
            corner={
              /* 预设 pill 挂右上角（作者 10-03：内容列里那一行别占地方）——点开左抽屉；
                 没取到文件名时退成「选择预设」的灰态 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title="点击选择预设"
              >
                <span className={p.presetFileName}>{presetName ?? '选择预设'}</span>
                <span className={p.presetSwitch}>切换</span>
              </button>
            }
            navs={[{ label: '上一步', back: true, onClick: () => deckRef.current?.jumpTo(1) }]}
            actions={[
              { label: '放弃改动', on: dirty, onClick: clearAll },
              { label: '保存', on: dirty, primary: true, onClick: commitAll },
              {
                label: '下一步',
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
            ]}
          >
            {/* data-hint 与下面那行提示同源：板框里多一行，板子就得为它让出高度
                （没有它的话列会溢出、提示文字压在底部胶囊上） */}
            <div
              className={`${p.info} ${p.calib}`}
              data-hint={saved ? undefined : 'true'}
              data-scroll
            >
              {/* 预设入口已挪到卡片右上角（corner），内容列只留三轴读数 */}
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={axisView}
                activeAxis="z"
                presetUid={currentUid}
                presetOptions={presetOptions}
                onPickPreset={pickPreset}
                canEdit={canPick}
                showPreset={false}
                onType={typeAxis}
                onRevert={revertAxis}
                onReset={resetAxis}
              />

              <section className={p.step}>
                {/* 板名撤了（作者 10-03：容器感去掉，与「校准」tab 同一语言）——
                    眉标题那行已写着「Z 偏移校准」，再挂一块板名是复读。
                    手柄留在原位（title="" 渲染零高 h3），只跟着收编的皮换直角实白 */}
                <PlateZoom resetKey="z" title="">
                  <CalibPlate
                    model="zoffset"
                    theme={PLATE_THEME}
                    label="Z 偏移校准板"
                    onPick={pickZ}
                    picked={zSelected}
                    hitLabel={zHitLabel}
                  />
                </PlateZoom>
                {/* G06-8 攒下的两行文案，与「校准」tab 同一句原话（作者 10-03：首页也缺这个） */}
                <p className={p.legend}>{Z_LEGEND}</p>
                <p className={p.tip}>{Z_TIP}</p>
                {/* 没取到预设时点板子不产生读数，这一行是唯一的解释，必须留。
                    其余说明去掉了：点了哪一格、改成多少，上面三轴读数里的「旧 → 新」已经说完 */}
                {!saved && <p className={p.note}>{NEED_PRESET}</p>}
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'calib-xy',
        node: (
          <CardFrame
            eyebrow="第四步 · XY 偏移校准"
            peekSafe
            tag={dirty ? '未保存' : savedNote ? '已保存' : undefined}
            tagTone={dirty ? 'accent' : 'muted'}
            topAction={{
              label: '打开模型',
              accent: true,
              onClick: () => setOpenModel('XY 偏移校准板'),
            }}
            corner={
              /* 同 Z 步：预设 pill 挂右上角，点开左抽屉 */
              <button
                type="button"
                className={presetName ? p.presetPill : `${p.presetPill} ${p.presetPillOff}`}
                onClick={() => setPickerOpen(true)}
                title="点击选择预设"
              >
                <span className={p.presetFileName}>{presetName ?? '选择预设'}</span>
                <span className={p.presetSwitch}>切换</span>
              </button>
            }
            navs={[
              {
                label: '上一步',
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_Z_INDEX),
              },
            ]}
            actions={[
              { label: '放弃改动', on: dirty, onClick: clearAll },
              { label: '保存', on: dirty, primary: true, onClick: commitAll },
              {
                label: '下一步',
                arrow: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_MODEL_INDEX),
              },
            ]}
          >
            <div
              className={`${p.info} ${p.calib}`}
              data-plate="xy"
              data-hint={saved ? undefined : 'true'}
              data-scroll
            >
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={axisView}
                activeAxis="xy"
                presetUid={currentUid}
                presetOptions={presetOptions}
                onPickPreset={pickPreset}
                canEdit={canPick}
                showPreset={false}
                onType={typeAxis}
                onRevert={revertAxis}
                onReset={resetAxis}
              />

              <section className={p.step}>
                {/* 同 Z 步：去容器、去板名，手柄直角实白；XY 板内自带整段说明，不给字 */}
                <PlateZoom resetKey="xy" title="">
                  <CalibPlate
                    model="precise"
                    theme={PLATE_THEME}
                    label="XY 偏移校准板"
                    onPick={pickXY}
                    picked={xySelected}
                    hitLabel={xyHitLabel}
                  />
                </PlateZoom>
                {!saved && <p className={p.note}>{NEED_PRESET}</p>}
              </section>
            </div>
          </CardFrame>
        ),
      },
      {
        id: 'models',
        node: (
          <CardFrame
            eyebrow="第五步 · 测试模型"
            peekSafe
            navs={[
              {
                label: '上一步',
                back: true,
                onClick: () => deckRef.current?.jumpTo(CALIB_XY_INDEX),
              },
            ]}
            actions={[
              {
                label: '打开测试模型',
                primary: true,
                onClick: () => setOpenModel('测试模型'),
              },
              { label: '回主页', onClick: () => deckRef.current?.jumpTo(0) },
            ]}
          >
            <div className={`${p.info} ${p.models}`} data-scroll>
              {/* 不给页级大标题：眉标题那行已经写着「第五步 · 测试模型」，
                  再写一遍是复读（首页与选机型页去掉眉标题是同一个道理）。
                  这一行只说"接下来做什么"，不重复页名 */}
              <p className={p.modelsLead}>打印这套模型，检查校准结果。</p>
              <div className={p.modelsHero}>
                <img
                  className={p.modelsHeroImg}
                  src={heroPile}
                  srcSet={`${heroPile} 1x, ${heroPile2x} 2x`}
                  alt="测试模型"
                  draggable={false}
                />
              </div>
              <p className={p.modelsCaption}>
                鱼尾曲面 · 支撑涂胶 · 球形 · 方块 — 一套 3mf 覆盖全部校准场景
              </p>
            </div>
          </CardFrame>
        ),
      },
    ],
    [
      artAlt,
      axisView,
      brandOptions,
      canPick,
      clearAll,
      commitAll,
      currentUid,
      density,
      dirty,
      draft,
      drop,
      entryLabel,
      fadeMs,
      layers,
      modelName,
      modelOptions,
      pick,
      pickPreset,
      pickXY,
      pickZ,
      preset,
      presetInfo,
      presetName,
      presetOptions,
      ready,
      resetAxis,
      revertAxis,
      saved,
      savedNote,
      sel,
      settle,
      typeAxis,
      variantName,
      variantOptions,
      xySelected,
      zSelected,
    ],
  )

  return (
    <div className={p.page} data-density={density}>
      <SlideDeck ref={deckRef} sheets={sheets} density={density} canLeave={canLeave} />

      {/* 预设抽屉：从左缘出（右边常驻露出卡，没地方）。换预设的确认（有草稿先问一句）
          走 pickPreset，与下拉时代同一条路；抽屉挂在 .page（container 的布局包含块）上 */}
      <PresetPickerDrawer
        side="left"
        open={pickerOpen}
        machines={catalog.machines.map((m) => ({
          id: m.id,
          display: m.display,
          versions: m.versions.map((v) => ({ id: v.id, name: v.name, tag: v.tag ?? null })),
        }))}
        machineId={sel.model ?? ''}
        versionId={sel.variant ?? ''}
        fileOf={fileOf}
        onPick={(m, v) => {
          setPickerOpen(false)
          pickPreset(`${m}/${v}`)
        }}
        onClose={() => setPickerOpen(false)}
      />

      {openModel && (
        <Modal
          title="打开测试模型"
          onClose={() => setOpenModel(null)}
          foot={
            <>
              <Btn disabled>从本地缓存打开</Btn>
              <Btn variant="primary" onClick={() => setOpenModel(null)}>
                从云端获取
              </Btn>
            </>
          }
        >
          <p className={p.note}>
            {openModel} · 本地无缓存文件，即将从云端下载打开。点击后请耐心等待 3mf 打开。
          </p>
        </Modal>
      )}

      {pending !== null && (
        <Modal
          title="有未保存的改动"
          onClose={() => setPending(null)}
          foot={
            <>
              <Btn onClick={() => setPending(null)}>取消</Btn>
              <Btn
                onClick={() => {
                  clearAll()
                  leaveTo(pending.to)
                }}
              >
                放弃改动并离开
              </Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  commitAll()
                  leaveTo(pending.to)
                }}
              >
                保存并离开
              </Btn>
            </>
          }
        >
          <p className={p.note}>
            偏移改动还没保存：
            {/* 三轴在哪一页都能改，所以这里列的是**全部**脏轴，不再按页筛。
                走到这个弹窗一定有草稿，也就一定有基准值；saved / draft 的判空只是给类型看 */}
            {saved && draft
              ? AXIS_ROWS.filter(([, k]) => dirtyAxes.includes(k))
                  .map(([label, k]) => `${label} ${saved[k].toFixed(2)} → ${draft[k].toFixed(2)}`)
                  .join('，')
              : ''}
            。离开前要保存吗？
          </p>
        </Modal>
      )}

      {presetAsk !== null && (
        <Modal
          title="有未保存的改动"
          onClose={() => setPresetAsk(null)}
          foot={
            <>
              <Btn onClick={() => setPresetAsk(null)}>取消</Btn>
              <Btn
                variant="primary"
                onClick={() => {
                  clearAll()
                  applyPreset(presetAsk)
                  setPresetAsk(null)
                }}
              >
                放弃改动并换预设
              </Btn>
            </>
          }
        >
          <p className={p.note}>
            换成 {presetOptions.find((o) => o.uid === presetAsk)?.name ?? '另一份预设'} 之后，
            这一页点出来的偏移草稿会作废 —— 草稿是相对上一份预设点出来的增量，
            换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
