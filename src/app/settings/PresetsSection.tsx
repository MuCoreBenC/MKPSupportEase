/*
 * 设置页「预设」那一块（2026-10-20）。
 *
 * 现在就一行：**复制出来的那一份跟不跟云端更新**（出厂 = 关）。
 *
 * 组件单独一个文件，与 `PostprocModeSection` 同一个理由：`PageSettings.tsx` 已经一千多行。
 * 这一块只管自己这一行 —— 读写走那两条专门的命令，改完广播 `presetPrefsMutated()`
 * （预设页据此把身份账本重读一遍），别的一概不碰。
 *
 * # 这一档的分量
 *
 * 它改的是**身份账本的出口**：账本按它决定"复制出来的那一份认不认云端"，
 * 于是云端那一表每行的三态（未获取 / 本地已有 / 有新版）与那颗「更新」跟着变 ——
 * 所以写了必须广播（不广播，用户切回预设页看到的还是旧账）。
 *
 * ★ 文案走字典（2026-10-25）。读失败 / 保存失败都用 `{detail}` 承接后端原话。
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import { useT } from '../../i18n'
import { presetPrefsMutated } from '../state/deliveryState'
import { Switch } from '../../components/switch'
import s from './PageSettings.module.css'

export default function PresetsSection() {
  const tr = useT()
  const [copiesFollowCloud, setCopiesFollowCloud] = useState<boolean | null>(null)
  const [err, setErr] = useState<string | null>(null)

  /*
   * 进设置页读一次。★ 后端回的是**解析后的 boolean** —— 盘上"没选过"是 null，
   * "没选过按哪一档解释"是后端的事，这里不拿 null 去猜（与自动清理那条同一条口径）。
   */
  useEffect(() => {
    let alive = true
    void api.getCopiesFollowCloud().then(
      (v) => {
        if (alive) setCopiesFollowCloud(v)
      },
      (e: unknown) => {
        if (alive) setErr(tr('settings.presets.readFailed', { detail: errorText(e) }))
      },
    )
    return () => {
      alive = false
    }
  }, [tr])

  const save = (next: boolean) => {
    /* 乐观更新：开关先跟着手指走 */
    setCopiesFollowCloud(next)
    setErr(null)
    void api.setCopiesFollowCloud(next).then(
      (got) => {
        setCopiesFollowCloud(got)
        /* 预设页那一表每行的三态由账本算，账本的出口正看这一格 ⇒ 叫它重读本地那一侧 */
        presetPrefsMutated()
      },
      (e: unknown) => {
        setErr(tr('settings.presets.saveFailed', { detail: errorText(e) }))
        /* 落盘失败就回去问后端现在到底是什么 —— 不许把界面留在与盘上不一致的那一档 */
        void api.getCopiesFollowCloud().then(
          (v) => setCopiesFollowCloud(v),
          () => setCopiesFollowCloud(null),
        )
      },
    )
  }

  return (
    <section className={s.section} aria-label={tr('settings.presets.title')}>
      <h2 className={s.secTitle}>{tr('settings.presets.title')}</h2>

      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>{tr('settings.presets.copies')}</div>
          <p className={s.fieldNote}>{tr('settings.presets.copiesNote')}</p>
        </div>
        {/* 还没读回来：不画开关（画一个"看起来是关的"开关会骗人） */}
        {copiesFollowCloud === null ? (
          <p className={s.fieldNote}>{tr('common.loading')}</p>
        ) : (
          <Switch
            checked={copiesFollowCloud}
            ariaLabel={tr('settings.presets.copies')}
            onChange={save}
          />
        )}
      </div>

      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{err}</p>}
    </section>
  )
}
