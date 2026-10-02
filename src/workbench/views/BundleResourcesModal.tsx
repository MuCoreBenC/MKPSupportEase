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
 * 套餐的唯一真源是 `presets/bundles.toml`（Task 13.6）。它有两类配发内容：
 * **MKP 预设**（`presets` 字段，**版本 uid 直引** —— 不经过资产库，doc §12.5 的本义；
 * 作者 2026-10-03：**文件不存在也能先挂** —— 预设是生成产物，生成之后文件才落）
 * 和 **BBS 引用**（`assetRefs`，走资产库）。两个页签各管一类。
 *
 * 保存交出**两个完整清单**，写盘由调用方做（`wb.setBundleRefs`，一次手势落一个文件）。
 */
import { useEffect, useState } from 'react'

import type { AssetList, AssetView, BundleView, PresetCandidate } from '../api'
import ModalC14 from '../c14/ModalC14'
import s from '../c14.module.css'

/** 页签分类。加种类先加这里，再在下面那个三元里认领 `kind` */
type ResTab = 'mkp' | 'slicer'

interface Props {
  open: boolean
  /** 资产库清单（切片器页签的候选池）。还在加载时框里说明状态 */
  assets: AssetList | null
  /** MKP 页签的候选池：能生成的版本（文件不存在也能先挂） */
  presetCandidates: PresetCandidate[]
  /** 改哪份套餐。null = 还没选套餐（框里会说清楚怎么做，保存禁用） */
  bundle: BundleView | null
  onCancel: () => void
  /** 交出**两个完整清单**（MKP 的 uid 列表 / BBS 的资产 id 列表），由调用方决定怎么落盘 */
  onConfirm: (presetUids: string[], slicerIds: string[]) => void
}

export default function BundleResourcesModal({
  open,
  assets,
  presetCandidates,
  bundle,
  onCancel,
  onConfirm,
}: Props) {
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
    setMkp((bundle?.presets ?? []).map((p) => p.uid))
    setSlicer((bundle?.assetRefs ?? []).filter((r) => r.kind === 'slicerProfile').map((r) => r.id))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, bundleId])

  const pool: AssetView[] =
    assets?.assets.filter((a) => (tab === 'mkp' ? false : a.kind === 'slicerProfile')) ?? []
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
            onClick={() => onConfirm(mkp, slicer)}
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
            /* MKP 预设走 uid 直引（不经过资产库）—— 文件没生成也能先挂，
               生成之后文件自动补上（作者 2026-10-03 的裁决） */
            <div className={s.bunList}>
              {presetCandidates.map((c) => (
                <label key={c.uid} className={s.fileRow}>
                  <input
                    type="checkbox"
                    checked={mkp.includes(c.uid)}
                    onChange={() => toggle(c.uid)}
                  />
                  <span className={`${s.fileName} ${s.mono}`}>{c.name}</span>
                  {c.state !== 'built' && <span className={s.tag}>未生成</span>}
                  <span className={s.cardNote}>{c.fileName}</span>
                </label>
              ))}
              {presetCandidates.length === 0 && (
                <p className={s.note}>还没有能生成的版本 —— 先去机型与版本页建。</p>
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
                    <span className={`${s.fileName} ${s.mono}`}>{a.name}</span>
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
