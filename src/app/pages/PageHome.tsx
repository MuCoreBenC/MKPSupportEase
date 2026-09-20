import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from 'react'
import SlideDeck, { type DeckHandle, type Sheet } from '../components/SlideDeck'
import MachinePicker, { type Selection } from '../components/MachinePicker'
import PresetStack from '../components/PresetStack'
import CalibPlate from '../components/CalibPlate'
import HeroFade from '../components/HeroFade'
import CardFrame from '../components/CardFrame'
import CalibHead from '../components/CalibHead'
import PlateZoom from '../components/PlateZoom'
import { HomeCardStack, StepRail } from '../components/HomeGuide'
import CopyAction from '../components/CopyAction'
import { Btn } from '../ui/Controls'
import { Modal } from '../ui/Modal'
import type { Density } from '../../hooks/useDensity'
import { pickArt } from '../heroArt'
import { useArtLayers } from '../useArtLayers'
import {
  AXIS_ROWS,
  NEED_PRESET,
  axisText,
  xyHitLabel,
  zHitLabel,
} from '../calibAxes'
import { useCalibration } from '../useCalibration'
import { usePreset } from '../usePreset'

import { presetCatalog } from '../../api/mock'
import { brands } from '../constants/machines'
import { models } from '../constants/models'
import { postProcessScript } from '../constants/postProcess'
import { variants } from '../constants/variants'
import p from './PageHome.module.css'

/** 渐进披露的淡入时长。试验场里来自调参面板（默认 300ms），产品里取默认值 */
const FADE_MS = 300

const EMPTY: Selection = { brand: null, model: null, variant: null }

/** 板子的配色跟应用的模式对齐；等应用做了深色模式，这里换成跟随主题的那个值 */
const PLATE_THEME = '浅色'

/** 两张校准卡在 sheets 里的位置：按页拦截换页要认它们 */
const CALIB_Z_INDEX = 2
const CALIB_XY_INDEX = 3
const CALIB_MODEL_INDEX = 4

/** 点击不让它拿焦点：拿了焦点浏览器会把它滚进可视区，整页就跟着挪。键盘 Tab 不受影响 */
const noFocus = (e: { preventDefault: () => void }) => e.preventDefault()

const labelOf = (list: { id: string; label: string }[], id: string | null) =>
  list.find((o) => o.id === id)?.label ?? null

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
}

export default function PageHome({ density }: PageHomeProps) {
  const deckRef = useRef<DeckHandle>(null)
  const [sel, setSel] = useState<Selection>(EMPTY)

  const [openModel, setOpenModel] = useState<string | null>(null)
  const [pending, setPending] = useState<{ from: number; to: number } | null>(null)
  /* 校准页的预设下拉要换一份、但本页有未保存草稿时，先把要换的那一份记下来等确认 */
  const [presetAsk, setPresetAsk] = useState<string | null>(null)
  // 保存 / 放弃之后自己发起的那一次跳转不该再被拦
  const bypass = useRef(false)

  // ---------- 预设：选哪一份由 sel 定，状态由请求本身定 ----------
  const { state: preset, retry: retryPreset } = usePreset(sel)

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
    clearAll,
    commitAll,
  } = calib

  // ui/Modal 不认 Esc，这里补上：Esc = 取消
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

  /* 校准页的预设下拉：一份预设唯一对应一个打印件版本，所以「选预设」= 反填三级选择。
     机型从预设目录那条自己带的 model 来 —— 于是回到第二页，品牌 / 机型 / 版本
     已经是这份预设对应的那一套，不用再手点一遍 */
  const applyPreset = useCallback((variantId: string) => {
    const row = presetCatalog.find((it) => it.variant === variantId)
    if (!row) return
    setSel({ brand: brands[0].id, model: row.model, variant: variantId })
  }, [])

  /* 换预设会把本页草稿作废（草稿是相对上一份预设点出来的增量），所以先问一次。
     只给「取消 / 放弃改动并换」两条路：换了预设 saved 会被新预设的值覆盖，
     这时候提供「保存并换」是骗人的 —— 存下去的数立刻就被顶掉了 */
  const pickPreset = useCallback(
    (variantId: string) => {
      if (!variantId || variantId === sel.variant) return
      if (dirty) {
        setPresetAsk(variantId)
        return
      }
      applyPreset(variantId)
    },
    [applyPreset, dirty, sel.variant],
  )

  const brandName = labelOf(brands, sel.brand)
  const modelName = labelOf(models, sel.model)
  const variantName = labelOf(variants, sel.variant)
  const art = useMemo(() => pickArt(sel), [sel])
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
                    <CopyAction text={postProcessScript} label="复制后处理脚本" />
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
            style={{ '--fade-ms': `${FADE_MS}ms` } as CSSProperties}
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
              <MachinePicker sel={sel} onPick={pick} />

              {/* 预设文件那一块只在真的有一份预设可谈时才出现（选齐三级）。
                  原来是常驻的横杠占位，宽窗里就成了"左上角选了一行、中间凭空一块空占位"，
                  视觉锚点全落在没内容的地方。宁可出现时把列往下长一截。
                  三轴只在"没有右侧露出卡"（density mini）时一起摆出来 */}
              {preset.status !== 'idle' && (
                <div className={p.pickerPreset}>
                  <PresetStack
                    state={preset}
                    axes={density === 'mini' ? saved : undefined}
                    onRetry={retryPreset}
                  />
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
            topAction={{ label: '打开模型', onClick: () => setOpenModel('Z 偏移校准板') }}
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
              <CalibHead
                saved={saved}
                draft={draft}
                preset={preset}
                view={axisView}
                activeAxis="z"
                variant={sel.variant}
                onPickPreset={pickPreset}
                canEdit={canPick}
                onType={typeAxis}
                onRevert={revertAxis}
              />

              <section className={p.step}>
                {/* 板名与缩放手柄同一行，由 PlateZoom 自己排 */}
                <PlateZoom resetKey="z" title="Z 偏移校准板">
                  <CalibPlate
                    model="zoffset"
                    theme={PLATE_THEME}
                    label="Z 偏移校准板"
                    onPick={pickZ}
                    picked={zSelected}
                    hitLabel={zHitLabel}
                  />
                </PlateZoom>
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
            topAction={{ label: '打开模型', onClick: () => setOpenModel('XY 偏移校准板') }}
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
                variant={sel.variant}
                onPickPreset={pickPreset}
                canEdit={canPick}
                onType={typeAxis}
                onRevert={revertAxis}
              />

              <section className={p.step}>
                {/* 板名与缩放手柄同一行，由 PlateZoom 自己排 */}
                <PlateZoom resetKey="xy" title="XY 偏移校准板">
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
                  src="/models/hero_pile.webp"
                  srcSet="/models/hero_pile.webp 1x, /models/hero_pile@2x.webp 2x"
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
      canPick,
      clearAll,
      commitAll,
      density,
      dirty,
      draft,
      drop,
      entryLabel,
      layers,
      modelName,
      pick,
      pickPreset,
      pickXY,
      pickZ,
      preset,
      presetInfo,
      presetName,
      ready,
      retryPreset,
      revertAxis,
      saved,
      savedNote,
      sel,
      settle,
      typeAxis,
      variantName,
      xySelected,
      zSelected,
    ],
  )

  return (
    <div className={p.page} data-density={density}>
      <SlideDeck ref={deckRef} sheets={sheets} density={density} canLeave={canLeave} />

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
            换成 {presetCatalog.find((it) => it.variant === presetAsk)?.name ?? '另一份预设'} 之后，这一页点出来的偏移草稿会作废
            —— 草稿是相对上一份预设点出来的增量，换了基准就没有意义了。
          </p>
        </Modal>
      )}
    </div>
  )
}
