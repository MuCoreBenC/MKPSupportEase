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
import { useEffect, useState } from 'react'

import type { AssetList, AssetView, BundleView } from '../api'
import ModalC14 from '../c14/ModalC14'
import s from '../c14.module.css'

/** 页签分类。加种类先加这里，再在下面那个三元里认领 `kind` */
type ResTab = 'mkp' | 'slicer'

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

  /*
   * 每次打开都从**当前那份**重新开始：上次勾了什么、停在哪个页签，都不该留到下一次。
   * 依赖只放 `open` 与 `bundle?.id` —— 把 bundle 整个放进去会在每次输入时重新覆盖草稿。
   */
  const bundleId = bundle?.id ?? null
  useEffect(() => {
    if (!open) return
    setTab('mkp')
    // MKP 预设那半 = assetRefs 里 kind 为 mkPreset 的（其余几类不进这两个页签）
    setMkp((bundle?.assetRefs ?? []).filter((r) => r.kind === 'mkPreset').map((r) => r.id))
    setSlicer((bundle?.assetRefs ?? []).filter((r) => r.kind === 'slicerProfile').map((r) => r.id))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, bundleId])

  const pool: AssetView[] =
    assets?.assets.filter((a) =>
      tab === 'mkp' ? a.kind === 'mkPreset' : a.kind === 'slicerProfile',
    ) ?? []
  const chosen = tab === 'mkp' ? mkp : slicer
  const toggle = (id: string) => {
    const flip = (cur: string[]) => (cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id])
    if (tab === 'mkp') setMkp(flip)
    else setSlicer(flip)
  }

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
      subtitle="切片器是套餐里的资源 —— 版本指向这个套餐，就拿到它们"
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
          <p className={s.note} style={{ marginTop: 0 }}>
            这里改的是<strong>套餐 {bundle.id}</strong> 的文件清单。
            {users.length + defaults.length > 1 && (
              <>
                {' '}
                注意有 <strong>{users.length + defaults.length} 处指向</strong>
                （{[...users.map((u) => `${u.machineId}/${u.versionId}`), ...defaults.map((d) => `${d}（机型默认）`)].join('、')}），
                改完它们都会跟着变。
              </>
            )}
          </p>

          <div className={s.pickTabs}>
            <button
              type="button"
              className={`${s.pickTab} ${tab === 'mkp' ? s.pickTabOn : ''}`}
              aria-pressed={tab === 'mkp'}
              onClick={() => setTab('mkp')}
            >
              MKP 预设（{mkp.length}）
            </button>
            <button
              type="button"
              className={`${s.pickTab} ${tab === 'slicer' ? s.pickTabOn : ''}`}
              aria-pressed={tab === 'slicer'}
              onClick={() => setTab('slicer')}
            >
              切片器（BBS）（{slicer.length}）
            </button>
          </div>

          {tab === 'mkp' ? (
            /* MKP 预设（资产库里的 `mkPreset` 类）—— **文件在不在都能选**：
               「有没有生成」是生成页的状态，不拦这里挂（作者 2026-10-03） */
            <div className={s.bunList}>
              {!assets && <p className={s.note}>资产库还在读……</p>}
              {assets &&
                pool.map((a) => (
                  <label key={a.id} className={s.fileRow}>
                    <input
                      type="checkbox"
                      checked={mkp.includes(a.id)}
                      onChange={() => toggle(a.id)}
                    />
                    <span className={`${s.fileName} ${s.mono}`}>{a.display || a.id}</span>
                    {a.buildState !== null && a.buildState !== 'built' && (
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
              {assets && pool.length === 0 && (
                <p className={s.note}>
                  资产库里还没有 MKP 预设登记 —— 去资产库登记（那里也能看到每份的生成状态）。
                </p>
              )}
            </div>
          ) : (
            <div className={s.bunList}>
              {!assets && <p className={s.note}>资产库还在读……</p>}
              {assets &&
                pool.map((a) => (
                  <label key={a.id} className={s.fileRow}>
                    <input
                      type="checkbox"
                      checked={chosen.includes(a.id)}
                      onChange={() => toggle(a.id)}
                    />
                    <span className={`${s.fileName} ${s.mono}`}>{a.display}</span>
                    {/* 括号里是来源标注：切片器这一类文件目前来自 BBS 体系 */}
                    <span className={s.tag}>{a.slicer === 'orca' ? 'Orca' : '(BBS)'}</span>
                    <span className={s.tag}>{a.machineId ?? '通用'}</span>
                  </label>
                ))}
              {assets && !pool.length && (
                <p className={s.note} style={{ margin: 0 }}>
                  资产库里还没有切片器文件 —— 先去资产库登记。
                </p>
              )}
            </div>
          )}

          <p className={s.note}>
            两个页签是同一份清单的两半：一个都不勾是合法的 —— 那这个套餐就没装这一类
            （但一条 BBS 都没有存不进去：MKP 与 BBS 必须成套配发，doc §12.4）。
          </p>
        </>
      )}
    </ModalC14>
  )
}
