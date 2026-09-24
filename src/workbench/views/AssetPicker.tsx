/**
 * 资产选择模态框（b05 Task 14.6）—— 「选一张已有图片**要**从资产库选，不手填路径」
 * （doc §4.4）。
 *
 * # 这一层的纪律
 *
 * 写进机型文件的是**资产 id**，不是路径 —— 路径只有 `presets/assets.toml`
 * 一份。文件还没就位的条目（`present: false`）**不给选**：选了就是造一条
 * 指向不存在文件的引用（11.1 要拦的那类东西）。
 *
 * 遮罩点空白 = 取消；Esc 同义。选中即返回 id，写入由调用方走它自己的
 * 命令（机型页是 `wb_set_machine_field`）。
 */
import { useEffect, useState } from 'react'

import { assetUrl, isAppError, wb, type AssetView } from '../api'

export interface AssetPickerProps {
  /** 只列这一类资产。机型图是 `image`、图标是 `icon` */
  kind: 'image' | 'icon'
  /** 当前值（资产 id）。列表里会标出来 */
  current: string | null
  /** 标题。缺省按 kind 给 */
  title?: string
  onPick: (asset: AssetView) => void
  onClose: () => void
}

export function AssetPicker({ kind, current, title, onPick, onClose }: AssetPickerProps) {
  const [assets, setAssets] = useState<AssetView[] | null>(null)
  const [err, setErr] = useState<string | null>(null)

  useEffect(() => {
    let alive = true
    void wb
      .assets()
      .then((list) => {
        if (alive) setAssets(list.assets.filter((a) => a.kind === kind))
      })
      .catch((e: unknown) => {
        if (alive) setErr(isAppError(e) ? e.message : String(e))
      })
    return () => {
      alive = false
    }
  }, [kind])

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [onClose])

  return (
    /* 遮罩上点空白关闭。点到面板内不冒泡（stopPropagation） */
    <div className="wb-modal" onClick={onClose} role="presentation">
      <div
        className="wb-modal__panel"
        role="dialog"
        aria-modal="true"
        aria-label={title ?? '从资产库选择'}
        onClick={(e) => e.stopPropagation()}
      >
        <header className="wb-card__head">
          <span className="wb-card__title">{title ?? '从资产库选择'}</span>
          <button type="button" className="wb-btn" onClick={onClose}>
            关闭
          </button>
        </header>

        {err && <p className="wb-todo" data-tone="danger">{err}</p>}
        {assets === null && !err && <p className="wb-todo">正在读资产库…</p>}

        {assets !== null && (
          <div className="wb-assetgrid">
            {assets.map((a) => (
              <button
                key={a.id}
                type="button"
                className="wb-asset"
                data-on={a.id === current ? 'yes' : undefined}
                disabled={!a.present}
                title={a.present ? a.name : `${a.name} —— 文件还没就位，不能选`}
                onClick={() => onPick(a)}
              >
                {kind === 'image' || kind === 'icon' ? (
                  <img className="wb-asset__thumb" src={assetUrl(a.url)} alt="" loading="lazy" />
                ) : (
                  <span className="wb-asset__thumb wb-asset__thumb--blank wb-mono">{a.kind}</span>
                )}
                <span className="wb-asset__name">{a.name}</span>
                <span className="wb-asset__meta">
                  <span className="wb-mono">{a.id}</span>
                  {a.machineId && ` · ${a.machineId}`}
                  {!a.present && ' · 未就位'}
                </span>
              </button>
            ))}
            {assets.length === 0 && (
              <p className="wb-todo">
                资产库里还没有这一类的条目。先在 `presets/assets.toml` 登记并放进
                `public/assets/`，这里才会有得选。
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  )
}
