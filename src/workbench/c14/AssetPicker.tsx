/*
 * 统一素材选择器（C14 移植，按真后端契约收窄）。
 *
 * 与原型的两处差别，都是后端契约决定的：
 *
 *  1. **候选来自资产库**（`wb_assets`，`presets/assets.toml`），不是「public 下的文件名」。
 *     机型定义的 image / icon 在 Task 8.6 起就是**资产 id**，加载期校验它认得出来 ——
 *     所以 14.6 的原则在这里落地为：只许从库里挑，**没有手打文件名那条路**。
 *  2. **没有「图片 / SVG」页签** —— 原型那对页签是按扩展名分家；产品的资产库
 *     按 `kind` 分家（image / icon / model / slicerProfile），调用方把清单按 kind
 *     过滤好再交进来，框本体不再认第二套分类。
 *
 * 留下的都是原型验收过的：加载失败的候选当场隐掉（清单允许缺文件 —— `present:
 * false` 是「还没搬」，不是错）；预览尺寸读 `naturalWidth/Height`，拿不到显示「—」，
 * 不编一个看起来像事实的数字。
 */

import { useEffect, useState } from 'react'
import type { AssetView } from '../api'
import { assetUrl } from '../api'
import ModalC14 from './ModalC14'
import s from '../c14.module.css'

interface Props {
  open: boolean
  /** 框的完整标题（含「选择什么」），也当无障碍名 */
  title: string
  /** 候选清单。调用方按 kind 过滤好（机型图给 image，图标给 icon） */
  options: AssetView[]
  /** 当前值：资产 id。null = 还没配 */
  value: string | null
  onCancel: () => void
  /** null = 清空这一格（可选字段空着是正常状态） */
  onPick: (next: string | null) => void
}

export default function AssetPicker({ open, title, options, value, onCancel, onPick }: Props) {
  /* 文件还没搬进资产库的候选 —— 照样列出来（`present: false` 是状态，不是错误） */
  const [picked, setPicked] = useState<string | null>(value)
  /* 预览图的实际像素。null = 还没加载出来 */
  const [pixels, setPixels] = useState<string | null>(null)
  const [broken, setBroken] = useState(false)

  /* 每次打开都从当前值重新开始 —— 上次关掉时选过什么，都不该留到下一次 */
  useEffect(() => {
    if (!open) return
    setPicked(value)
  }, [open, value])

  /* 换了一张，尺寸与「坏没坏」都要重新算 */
  useEffect(() => {
    setPixels(null)
    setBroken(false)
  }, [picked])

  const pickedView = options.find((a) => a.id === picked) ?? null

  return (
    <ModalC14
      open={open}
      title={title}
      subtitle="候选来自资产库（presets/assets.toml）—— 资产只从库里挑，不手填路径。文件还没搬进去的条目照样列，界面上会说明"
      size="md"
      closeOnScrim={false}
      onClose={onCancel}
      footer={
        <>
          <button
            type="button"
            className={`${s.btn} ${s.btnSm}`}
            onClick={() => onPick(null)}
            title="这个位置不配（可选字段空着是正常状态）"
          >
            清空
          </button>
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onCancel}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={!picked}
            onClick={() => onPick(picked)}
          >
            选择
          </button>
        </>
      }
    >
      <div className={s.pickGrid}>
        <div className={s.imgGrid}>
          {options.map((a) => (
            <button
              key={a.id}
              type="button"
              className={`${s.imgCell} ${a.id === picked ? s.imgCellOn : ''}`}
              aria-pressed={a.id === picked}
              title={a.present ? a.display : `${a.display} —— 文件还没搬进资产库`}
              onClick={() => setPicked(a.id)}
            >
              {a.present ? (
                <img src={assetUrl(a.url)} alt={a.display} loading="lazy" />
              ) : (
                <span className={s.assetThumbPh}>未搬</span>
              )}
              <span className={`${s.imgName} ${s.mono}`}>{a.display}</span>
            </button>
          ))}
        </div>
        {!options.length && (
          <p className={s.note} style={{ marginTop: 0 }}>
            资产库里还没有这一类的条目 —— 先去资产库登记。
          </p>
        )}
      </div>

      <div className={s.pickPreview}>
        {pickedView ? (
          pickedView.present && !broken ? (
            <img
              key={pickedView.id}
              src={assetUrl(pickedView.url)}
              alt={pickedView.display}
              onLoad={(e) =>
                setPixels(`${e.currentTarget.naturalWidth} × ${e.currentTarget.naturalHeight}`)
              }
              onError={() => setBroken(true)}
            />
          ) : (
            <span className={s.pickPh}>
              {pickedView.display} 的文件还没搬进资产库 —— 选它界面上会显示占位说明
            </span>
          )
        ) : (
          <span className={s.pickPh}>在上面挑一张</span>
        )}
      </div>
      <div className={s.pickMeta}>
        <span>
          资产 <code className={s.mono}>{pickedView?.id ?? '—'}</code>
        </span>
        <span>
          尺寸 <span className={s.tnum}>{pixels ?? '—'}</span>
        </span>
      </div>
    </ModalC14>
  )
}

/**
 * 字段位上的素材格：**缩略图 + 当前条目 + [更换]**（C14 原样，值从文件名换成资产条目）。
 *
 * 名字是落盘表现（资产 id），人认的是图 —— 两个都给。选择器本身由宿主渲染
 * （`Modal` 是 `absolute inset:0`，挂在页面那一层才盖得住），这里只负责把入口摆出来。
 */
export function AssetField({
  label,
  asset,
  onOpen,
}: {
  label: string
  /** 当前值解析出来的资产条目；null = 还没配 */
  asset: AssetView | null
  onOpen: () => void
}) {
  const [ok, setOk] = useState(true)
  useEffect(() => setOk(true), [asset?.id])

  return (
    <span className={s.assetField}>
      {asset ? (
        asset.present && ok ? (
          <img
            key={asset.id}
            className={s.assetThumb}
            src={assetUrl(asset.url)}
            alt={asset.name}
            onError={() => setOk(false)}
          />
        ) : (
          <span className={s.assetThumbPh} title={asset.present ? '图加载不出来' : '文件还没搬'}>
            缺图
          </span>
        )
      ) : (
        <span className={s.assetThumbPh}>未配</span>
      )}
      <span className={`${s.mono} ${s.assetName}`}>{asset?.id ?? '—'}</span>
      <button
        type="button"
        className={`${s.btn} ${s.btnSm}`}
        aria-label={`${label}：${asset ? '更换' : '挑一张'}`}
        onClick={onOpen}
      >
        {asset ? '更换' : '挑一张…'}
      </button>
    </span>
  )
}
