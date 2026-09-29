/*
 * 批量修改（对照模式右侧，与「参数详情」同一个右栏的两页签之一）（C14 移植）。
 *
 * 「**这一个参数**、在这几个版本上，都设成这个值」—— 一次动作、一条历史，
 * Ctrl+Z 整块退回来（撤销栈在壳上，按整块 inverse 回）。
 *
 * 与原型的分工：C14 的批量是前端直接写；产品的纪律是**先预览再落**
 * （「不许闷着改」）——「应用」先走 `wb_preview_bulk`：每一列的 before → after、
 * 哪几列被跳过（不适用 / 被条件关着）都由后端判好，人看过清单再「确认写入」，
 * 写的仍是**勾选的那些版本各自那一层**（一次手势、一条历史）。
 *
 * 三条刻意的限制（C14 第十五/六轮，原样保留）：
 *
 *   1. **目标只给版本，不给机型基底** —— 把基底也当成目标会顺手动到没被勾选的版本
 *   2. **gcode / 长文本 / 已弃用没有这一页**（调用方不给页签）
 *   3. **目标名字一律带机型**（作者：「把 A1 的补上」）
 */
import { useState } from 'react'

import { wb, type ParamView, type Patch, type Words, type BulkPreview } from '../api'
import { toasts } from '../c14/toast'
import ModalC14 from '../c14/ModalC14'
import { SelectField } from '../components/field'
import s from '../c14.module.css'

export interface BatchTarget {
  /** 列记号（后端重排后的 `Col.key`：机型 id 或版本 uid = patch 的 owner） */
  id: string
  /** chip 上的名字，**一律带机型** */
  name: string
  machineId: string
  uid: string | null
}

interface Props {
  param: ParamView
  words: Words
  /** 可应用的版本列（机型基底不在其中）。可以来自**别的机型** */
  targets: BatchTarget[]
  /** 唯一写入口（外壳的 run）。确认写入时带上整批 patch */
  onApply: (label: string, patches: Patch[]) => Promise<void>
}

/** 批量值按注册表的值类型归位（与单格写值同一条路） */
function coerce(param: ParamView, raw: string): unknown {
  if (param.uiComponent === 'switch') return raw === '开'
  if (param.valueType === 'bool') return raw === 'true'
  if ((param.valueType === 'float' || param.valueType === 'int') && raw !== '') return Number(raw)
  return raw
}

export default function BatchEdit({ param, words, targets, onApply }: Props) {
  const [pickedIds, setPickedIds] = useState<string[] | null>(null)
  const [raw, setRaw] = useState('')
  const [preview, setPreview] = useState<BulkPreview | null>(null)
  const [previewValue, setPreviewValue] = useState<unknown>(null)

  const pickedTargets = pickedIds ?? targets.map((v) => v.id)

  const toggle = (id: string) => {
    const cur = pickedTargets.includes(id)
      ? pickedTargets.filter((x) => x !== id)
      : [...pickedTargets, id]
    setPickedIds(cur.length ? cur : [id])
  }

  /** 全选 / 反选（拖选手势之外也要有明写的按钮） */
  const allOn = targets.length > 0 && targets.every((t) => pickedTargets.includes(t.id))

  const shownValue = (): string => {
    if (param.uiComponent === 'switch') return raw === '开' ? '开' : '关'
    const hit = param.choices.find((o) => String(o.value) === raw)
    return hit ? hit.label : raw || '（空）'
  }

  /** 应用 = 先预览（后端判每一列的 before/after 与跳过原因），人看过再写 */
  const apply = async () => {
    if (!pickedTargets.length) return
    const chosen = targets.filter((t) => pickedTargets.includes(t.id))
    if (param.uiComponent === 'number' && (raw === '' || Number.isNaN(Number(raw)))) {
      toasts.push('先填一个数字再应用')
      return
    }
    const value = coerce(param, raw)
    /* 手势前的那道弃用闸：写弃用的选项档，后端也会拒——这里先说人话 */
    const dead = param.choices.find((c) => c.deprecated && String(c.value) === raw)
    if (dead) {
      toasts.push(`${param.label} 的「${dead.label}」${words.disabled.deprecatedWriteBlocked}`)
      return
    }
    try {
      const out = await wb.previewBulk(
        param.key,
        value,
        chosen.map((t) => ({ machineId: t.machineId, versionUid: t.uid })),
      )
      setPreviewValue(value)
      setPreview(out)
    } catch {
      /* previewBulk 拒绝时后端已给过错误横幅（外壳 fail），这里不用再说什么 */
    }
  }

  /** 确认写入：只写会变的列（noChange 的列让后端的「没变化不记草稿」去兜也行，但不发是无谓的手势） */
  const confirmWrite = async () => {
    if (!preview) return
    const byId = new Map(targets.map((t) => [t.id, t]))
    const patches: Patch[] = []
    for (const effect of preview.effects) {
      if (effect.kind === 'noChange') continue
      const t = byId.get(effect.col)
      if (!t) continue
      patches.push({
        kind: 'setValue',
        level: t.uid !== null ? 'version' : 'machine',
        owner: t.uid ?? t.machineId,
        key: param.key,
        value: previewValue,
      })
    }
    setPreview(null)
    if (!patches.length) {
      toasts.push('没有要写的列 —— 勾选的值都已经是这个值了')
      return
    }
    await onApply(`${param.label} 批量设为 ${shownValue()}（${patches.length} 列）`, patches)
  }

  return (
    <aside className={s.pCard}>
      <div className={s.pBatchRow}>
        <span className={s.pKvK}>参数</span>
        <span className={s.pKvV}>
          {param.label}
          <em className={s.cardNote}>{param.unit ?? ''}</em>
        </span>
      </div>

      <div className={s.pBatchRow}>
        <span className={s.pKvK}>
          应用目标
          {/* 目标能有两三打，全选 / 反选必须有明写的按钮 */}
          <span className={s.pMiniOps}>
            <button
              type="button"
              className={s.pMiniBtn}
              title={allOn ? '全不选' : '勾上所有对照列'}
              onClick={() => setPickedIds(allOn ? [] : targets.map((t) => t.id))}
            >
              {allOn ? '全不选' : '全选'}
            </button>
            <button
              type="button"
              className={s.pMiniBtn}
              title="把当前目标反过来"
              onClick={() =>
                setPickedIds(targets.filter((t) => !pickedTargets.includes(t.id)).map((t) => t.id))
              }
            >
              反选
            </button>
          </span>
        </span>
        {/* chip 上的名字一律带机型（C14 第十五轮：「把 A1 的补上」）；一律实线 */}
        <span className={s.pBatchChips}>
          {targets.map((v) => (
            <button
              key={v.id}
              type="button"
              className={`${s.pChip} ${pickedTargets.includes(v.id) ? s.pChipOn : ''}`}
              onClick={() => toggle(v.id)}
            >
              {v.name || v.id}
            </button>
          ))}
        </span>
      </div>

      <div className={s.pBatchRow}>
        <span className={s.pKvK}>设置新值</span>
        <span className={s.pBatchVal}>
          {param.uiComponent === 'switch' ? (
            <button
              type="button"
              className={`${s.btn} ${s.btnSm} ${raw === '开' ? s.btnOn : ''}`}
              onClick={() => setRaw(raw === '开' ? '关' : '开')}
            >
              {raw === '开' ? '开' : '关'}
            </button>
          ) : param.choices.length > 0 ? (
            <SelectField
              label={param.label}
              size="sm"
              value={raw}
              options={param.choices.map((o) => ({
                value: String(o.value),
                label: o.label,
                deprecated: o.deprecated || undefined,
              }))}
              onChange={setRaw}
            />
          ) : (
            <input
              className={s.pEdNum}
              type="number"
              value={raw}
              min={param.min ?? undefined}
              max={param.max ?? undefined}
              step={param.step ?? undefined}
              onChange={(e) => setRaw(e.target.value)}
            />
          )}
        </span>
      </div>

      <button
        type="button"
        className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
        disabled={!pickedTargets.length}
        title={!pickedTargets.length ? '先挑至少一个应用目标' : undefined}
        onClick={() => void apply()}
      >
        应用
      </button>
      <p className={s.note}>
        写到<b>勾选的那些版本各自那一层</b>，没勾的版本不动。一次动作一条历史，Ctrl+Z 整块退回来。
      </p>

      <div className={s.pCardHead}>
        <h3>{param.label}</h3>
      </div>
      <p className={s.pDesc}>{param.desc}</p>

      <p className={s.cardNote}>对照列 {targets.length} 个</p>

      {/*
        批量预览（产品的纪律：先看后写）。每一列的前后值、哪几列被跳过、
        为什么跳过，全部由 `wb_preview_bulk` 判好；确认才写。
      */}
      {preview && (
        <ModalC14
          open
          size="md"
          title={`批量：${preview.label}`}
          subtitle={`${shownValue()} → ${preview.effects.length} 列${
            preview.skipped.length ? ` · 跳过 ${preview.skipped.length}` : ''
          }`}
          onClose={() => setPreview(null)}
          footer={
            <>
              <button type="button" className={`${s.btn} ${s.btnSm}`} onClick={() => setPreview(null)}>
                再改改
              </button>
              <button
                type="button"
                className={`${s.btn} ${s.btnPrimary} ${s.btnSm}`}
                disabled={!preview.effects.some((e) => e.kind !== 'noChange')}
                title={
                  preview.effects.some((e) => e.kind !== 'noChange')
                    ? undefined
                    : '勾选的列都已经是这个值了'
                }
                onClick={() => void confirmWrite()}
              >
                确认写入
              </button>
            </>
          }
        >
          {preview.effects.map((e) => (
            <div key={e.col} className={s.pBatchRow}>
              <span className={s.pKvK}>
                {e.label}
                <em className={s.cardNote}>{e.machine}</em>
              </span>
              <span className={s.pKvV}>
                {e.kind === 'noChange' ? (
                  <span className={s.cardNote}>{words.bulkKind.noChange.label}（不变）</span>
                ) : (
                  <>
                    {e.before} → <b>{e.after}</b>
                    {e.kind === 'detaching' && (
                      <span className={s.cardNote}> · {words.bulkKind.detaching.label}</span>
                    )}
                  </>
                )}
              </span>
            </div>
          ))}
          {preview.skipped.map((skip) => (
            <div key={skip.col} className={s.pBatchRow}>
              <span className={s.pKvK}>
                {skip.label}
                <em className={s.cardNote}>{skip.machine}</em>
              </span>
              <span className={s.pKvV}>
                <span className={s.cardNote} title={skip.blocked.map((b) => b.need).join('；') || undefined}>
                  {skip.reason}
                </span>
              </span>
            </div>
          ))}
        </ModalC14>
      )}
    </aside>
  )
}
