/**
 * 套餐内容选择器 —— 机型页（④ 关联）与套餐页共用同一个框（C14）。
 *
 * # 为什么只能有一份实现
 *
 * 作者的原话：「这两个地方都统一一下好吧，都用模态框了……两边都一模一样」。
 * 两处要做的是同一件事 —— **决定一个套餐里装哪些资源**。两份实现迟早会分叉。
 *
 * # 与原型的差别（真后端决定）
 *
 * 套餐的唯一真源是 `presets/bundles.toml`（Task 13.6），内容是**一份** `assetRefs`：
 * 切片器预设（`type = 'slicerProfile'`）与 **MKP 预设**（`type = 'mkPreset'`，2026-10-03
 * 按作者「为什么资产库里面不放 mkp 预设」进的资产库）都在里面。两个页签是这份清单的两半。
 *
 * **文件在不在都能选**（作者：「不只是没文件的时候可以选择，有文件也要可以选择」）：
 * MKP 预设条目恒在资产库里，「有没有生成」是生成页四档状态的事，不拦选用。
 *
 * 保存交出**一个完整清单**，写盘由调用方做（`wb.setBundleRefs`，一次手势落一个文件）。
 */
import { useEffect, useMemo, useState } from 'react'

import type { AssetList, AssetView, BundleView } from '../api'
import ModalC14 from '../c14/ModalC14'
import SelectField from '../c14/field/SelectField'
import s from '../c14.module.css'

/** 页签分类。加种类先加这里，再在下面那个三元里认领 `kind` */
type ResTab = 'mkp' | 'slicer'

/**
 * 一行资产的**主显示名** = 真实文件名（带扩展名），取 `path` 的末段。
 *
 * ★ 2026-10-16（作者：「它最主要的还是我希望他把文件名显示是真实的，带有后缀的就是扩展名的」）：
 *   切片器 / 图片这类**有源文件**的，`path` 就是盘上那份真路径（还带空格，如
 *   `MKPProcess A1 mini 0.4 0.20.json`），末段就是想要的「真实文件名」。
 *
 * ★ **MKP 预设是例外**：它的 `path` 是空的 —— 那份产物名由后端按命名规则算出、不落台账
 *   （doc §12.5），所以这里**回落 `display`（版本名，如「标准版」）** 而不是留空白。
 *   这一条是被探针逼出来的：第一版直接取末段 ⇒ 整列主名是空的（截图 `_dump.mjs` 实测）。
 */
function leadNameOf(a: AssetView): string {
  /* MKP 预设：**产物文件名**（`A1-standard.toml`）—— 后端现算，台账里没有这份源文件。
     作者 2026-10-16：「MKP 那一行的完整名字……点 TOML 要显示出来吧」。 */
  if (a.productName !== '') return a.productName
  const cut = Math.max(a.path.lastIndexOf('/'), a.path.lastIndexOf('\\'))
  const base = cut === -1 ? a.path : a.path.slice(cut + 1)
  return base !== '' ? base : a.display || a.id
}

/**
 * 一刀切出来的「分类」标签（用于筛选）：机型优先，其次切片器档位，最后「通用」。
 * MKP 预设按 `machineId`；切片器多数 `machineId` 为空 —— 那一档回落 `slicer`。
 */
function groupOf(a: AssetView): string {
  return a.machineId ?? a.slicer ?? '通用'
}

interface Props {
  open: boolean
  /** 资产库清单（两个页签的候选池：mkPreset / slicerProfile）。还在加载时框里说明状态 */
  assets: AssetList | null
  /** 改哪份套餐。null = 还没选套餐（框里会说清楚怎么做，保存禁用） */
  bundle: BundleView | null
  onCancel: () => void
  /** 交出**一个完整清单**（资产 id 列表），由调用方决定怎么落盘 */
  onConfirm: (assetIds: string[]) => void
}

export default function BundleResourcesModal({ open, assets, bundle, onCancel, onConfirm }: Props) {
  const [tab, setTab] = useState<ResTab>('mkp')
  const [mkp, setMkp] = useState<string[]>([])
  const [slicer, setSlicer] = useState<string[]>([])
  /** 分类筛选：'' = 全部。换页签时重置（两个页签的类目不是一套） */
  const [group, setGroup] = useState('')

  /*
   * 每次打开都从**当前那份**重新开始：上次勾了什么、停在哪个页签，都不该留到下一次。
   * 依赖只放 `open` 与 `bundle?.id` —— 把 bundle 整个放进去会在每次输入时重新覆盖草稿。
   */
  const bundleId = bundle?.id ?? null
  useEffect(() => {
    if (!open) return
    setTab('mkp')
    setGroup('')
    // MKP 预设那半 = assetRefs 里 kind 为 mkPreset 的（其余几类不进这两个页签）
    setMkp((bundle?.assetRefs ?? []).filter((r) => r.kind === 'mkPreset').map((r) => r.id))
    setSlicer((bundle?.assetRefs ?? []).filter((r) => r.kind === 'slicerProfile').map((r) => r.id))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, bundleId])

  const pool: AssetView[] = useMemo(
    () =>
      assets?.assets.filter((a) =>
        tab === 'mkp' ? a.kind === 'mkPreset' : a.kind === 'slicerProfile',
      ) ?? [],
    [assets, tab],
  )
  const chosen = tab === 'mkp' ? mkp : slicer
  const toggle = (id: string) => {
    const flip = (cur: string[]) => (cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id])
    if (tab === 'mkp') setMkp(flip)
    else setSlicer(flip)
  }

  /*
   * 分类（作者 2026-10-16：「这个还有切片器的话，我是希望它到时候是可以那种分类吧，或者筛选吧」）：
   * 类目从候选池现推 —— 有几类就摆几颗，不写死。计数带在标签上，能看出这一类有多少。
   * 「全部」恒第一颗。切页签时 `group` 已在上面重置，不会留到另一个页签的类目上。
   */
  const groups = useMemo(() => {
    const m = new Map<string, number>()
    for (const a of pool) {
      const k = groupOf(a)
      m.set(k, (m.get(k) ?? 0) + 1)
    }
    return [...m.entries()].sort((x, y) => x[0].localeCompare(y[0]))
  }, [pool])
  const shown = group === '' ? pool : pool.filter((a) => groupOf(a) === group)

  /*
   * 这份套餐被谁指着 —— 套餐是**共享**的，改它它们都会跟着变，
   * 所以只要不止一个指向，就必须把这件事说出来。
   */
  const users = bundle?.users ?? []
  const defaults = bundle?.defaultFor ?? []

  return (
    <ModalC14
      open={open}
      title={`套餐内容 · ${bundleId ?? '（还没选套餐）'}`}
      size="md"
      closeOnScrim={false}
      onClose={onCancel}
      footer={
        <>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onCancel}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={bundle === null}
            title={bundle === null ? '先选一个套餐' : undefined}
            onClick={() => onConfirm([...mkp, ...slicer])}
          >
            保存
          </button>
        </>
      }
    >
      {bundle === null ? (
        <p className={s.note} style={{ marginTop: 0 }}>
          还没选套餐 —— 先在左边挑一份，再回来改它装什么。
        </p>
      ) : (
        <>
          {users.length + defaults.length > 1 && (
            <p className={s.cardNoteDim} style={{ marginTop: 0 }}>
              这份套餐有 <strong>{users.length + defaults.length} 处指向</strong>，改完都会跟着变
              <HelpDot
                text={`指向它的：${[
                  ...users.map((u) => `${u.machineId}/${u.versionId}`),
                  ...defaults.map((d) => `${d}（机型默认）`),
                ].join('、')}`}
              />
            </p>
          )}

          <div className={s.pickTabs}>
            <button
              type="button"
              className={`${s.pickTab} ${tab === 'mkp' ? s.pickTabOn : ''}`}
              aria-pressed={tab === 'mkp'}
              onClick={() => {
                setTab('mkp')
                setGroup('')
              }}
            >
              MKP 预设（{mkp.length}）
            </button>
            <button
              type="button"
              className={`${s.pickTab} ${tab === 'slicer' ? s.pickTabOn : ''}`}
              aria-pressed={tab === 'slicer'}
              onClick={() => {
                setTab('slicer')
                setGroup('')
              }}
            >
              切片器（BBS）（{slicer.length}）
            </button>
          </div>

          {/* 分类筛选（作者 2026-10-16：「改成下拉菜单吧」）—— 类目从候选池现推、不写死；
              计数带在选项右侧的小字里。选项少于一类的就不摆这个下拉。 */}
          {groups.length > 1 && (
            <div className={s.addRow} style={{ marginTop: 0 }}>
              <SelectField
                label="按机型分类筛选"
                size="sm"
                value={group}
                options={[
                  { value: '', label: `全部（${pool.length}）` },
                  ...groups.map(([g, n]) => ({ value: g, label: g, note: `${n} 份` })),
                ]}
                onChange={setGroup}
              />
            </div>
          )}

          <div className={s.bunList}>
            {!assets && <p className={s.note}>资产库还在读……</p>}
            {assets &&
              shown.map((a) => (
                <label key={a.id} className={s.fileRow}>
                  <input
                    type="checkbox"
                    checked={chosen.includes(a.id)}
                    onChange={() => toggle(a.id)}
                  />
                  {/* ★ 主显示 = **真实文件名 / 产物名（带扩展名）** —— 作者 2026-10-16：
                      「我希望他把文件名显示是真实的，带有后缀的就是扩展名的」。
                      切片器取 `path` 末段；MKP 预设取后端算的产物名（`A1-standard.toml`）。
                      台账键（id）/ 版本名退成同排浅色附注。 */}
                  <span
                    className={`${s.fileName} ${s.mono}`}
                    title={a.productName !== '' ? a.productName : a.path}
                  >
                    {leadNameOf(a)}
                  </span>
                  <span className={s.cardNoteDim}>
                    {a.productName !== '' || a.path === ''
                      ? a.id
                      : tab === 'mkp'
                        ? a.display
                        : a.id}
                  </span>
                  {/* 括号里是来源标注：切片器这一类文件目前来自 BBS 体系 */}
                  {tab === 'slicer' && (
                    <span className={s.tag}>{a.slicer === 'orca' ? 'Orca' : '(BBS)'}</span>
                  )}
                  {tab === 'mkp' && a.buildState !== null && a.buildState !== 'built' && (
                    <span className={s.tag}>
                      {a.buildState === 'neverBuilt'
                        ? '待生成'
                        : a.buildState === 'stale'
                          ? '待更新'
                          : '暂无资源'}
                    </span>
                  )}
                </label>
              ))}
            {assets && shown.length === 0 && (
              <p className={s.note} style={{ margin: 0 }}>
                {pool.length === 0
                  ? tab === 'mkp'
                    ? '资产库里还没有 MKP 预设登记 —— 去资产库登记（那里也能看到每份的生成状态）。'
                    : '资产库里还没有切片器文件 —— 先去资产库登记。'
                  : '这一类底下没有文件 —— 点上面「全部」看其余。'}
              </p>
            )}
          </div>

          <p className={s.cardNoteDim}>
            两个页签是同一份清单的两半 —— 一个都不勾是合法的（那就不装这一类）
            <HelpDot text="但一条 BBS 都没有存不进去：MKP 与 BBS 必须成套配发（doc §12.4）。" />
          </p>
        </>
      )}
    </ModalC14>
  )
}

/**
 * 「?」小提示（与 `BundlesPage` 那份同形）。这一页只用到一次，但它不能 import 视图里的
 * 组件（视图之间不互相引），就地复制这一小块 —— 比抽公共件更轻。
 */
function HelpDot({ text }: { text: string }) {
  return (
    <span className={s.helpDot} role="img" aria-label={text} title={text}>
      ?
    </span>
  )
}
