/*
 * 设置页「预设」那一块（2026-10-20 开，2026-11-01 加第二行）。
 *
 * 两行，都是"预设页怎么算"的偏好：
 *   ① **复制出来的那一份跟不跟云端更新**（出厂 = 关）
 *   ② **使用之后自动复制后处理脚本**（出厂 = 关）—— 换了「使用中」那一份就把它的
 *      后处理命令抄进剪贴板（作者 2026-11-01 点名要的那一档）
 *
 * 组件单独一个文件，与 `PostprocModeSection` 同一个理由：`PageSettings.tsx` 已经一千多行。
 * 这一块只管自己这两行 —— 读写走各自那两条专门的命令，改完广播 `presetPrefsMutated()`
 * （预设页据此把身份账本重读一遍），别的一概不碰。
 *
 * # 第 ① 档的分量
 *
 * 它改的是**身份账本的出口**：账本按它决定"复制出来的那一份认不认云端"，
 * 于是云端那一表每行的三态（未获取 / 本地已有 / 有新版）与那颗「更新」跟着变 ——
 * 所以写了必须广播（不广播，用户切回预设页看到的还是旧账）。
 * 第 ② 档不改变任何账，写完**不广播**（广播会让预设页白重读一遍本机文件）。
 *
 * ★ 文案走字典（2026-10-25）。读失败 / 保存失败都用 `{detail}` 承接后端原话。
 */

import { useEffect, useState } from 'react'

import { api, errorText } from '../../api'
import { msg, useText, useT, type Text, type StringKey } from '../../i18n'
import { presetPrefsMutated } from '../state/deliveryState'
import { Switch } from '../../components/switch'
import { Tip } from '../../components/tooltip'
import { SectionHead } from './SectionHead'
import s from './PageSettings.module.css'

/*
 * 四条读写都写成模块级常量：`BoolRow` 的读放进了 effect 依赖，
 * 组件里现写箭头函数的话每次渲染都是新引用 ⇒ 每一帧重读一次盘。
 */
const READ_COPIES = () => api.getCopiesFollowCloud()
const WRITE_COPIES = (next: boolean) => api.setCopiesFollowCloud(next)
const READ_AUTO_COPY = () => api.getAutoCopyScript()
const WRITE_AUTO_COPY = (next: boolean) => api.setAutoCopyScript(next)

/**
 * 一行布尔开关：左标题 + 气泡说明，右圆钮。
 *
 * ★ 还没读回来时**不画开关** —— 画一个"看起来是关的"开关会骗人
 *   （与这一页别处同一条口径）。
 */
function BoolRow({
  label,
  note,
  read,
  write,
  after,
}: {
  label: StringKey
  note: StringKey
  read: () => Promise<boolean>
  write: (next: boolean) => Promise<boolean>
  /** 写完要广播的那一条：只有"这一档会改变别处的账"时才给 */
  after?: () => void
}) {
  const tr = useT()
  const say = useText()
  const [on, setOn] = useState<boolean | null>(null)
  /** 存键（`msg`）：这一格会一直挂着，换语言得跟着换字 */
  const [err, setErr] = useState<Text | null>(null)

  /*
   * 进设置页读一次。★ 后端回的是**解析后的 boolean** —— 盘上"没选过"是 null，
   * "没选过按哪一档解释"是后端的事，这里不拿 null 去猜（与自动清理那条同一条口径）。
   */
  useEffect(() => {
    let alive = true
    void read().then(
      (v) => {
        if (alive) setOn(v)
      },
      (e: unknown) => {
        if (alive) setErr(msg('settings.presets.readFailed', { detail: errorText(e) }))
      },
    )
    return () => {
      alive = false
    }
  }, [read, tr])

  const save = (next: boolean) => {
    /* 乐观更新：开关先跟着手指走 */
    setOn(next)
    setErr(null)
    void write(next).then(
      (got) => {
        setOn(got)
        after?.()
      },
      (e: unknown) => {
        setErr(msg('settings.presets.saveFailed', { detail: errorText(e) }))
        /* 落盘失败就回去问后端现在到底是什么 —— 不许把界面留在与盘上不一致的那一档 */
        void read().then(
          (v) => setOn(v),
          () => setOn(null),
        )
      },
    )
  }

  return (
    <>
      <div className={`${s.field} ${s.fieldRow}`}>
        <div className={s.fieldText}>
          <div className={s.fieldTitle}>
            {tr(label)}
            <Tip text={tr(note)} />
          </div>
        </div>
        {on === null ? (
          <p className={s.fieldNote}>{tr('common.loading')}</p>
        ) : (
          <Switch checked={on} ariaLabel={tr(label)} onChange={save} />
        )}
      </div>

      {err !== null && <p className={`${s.note} ${s.noteBad}`}>{say(err)}</p>}
    </>
  )
}

export default function PresetsSection() {
  const tr = useT()

  return (
    <section className={s.section} aria-label={tr('settings.presets.title')}>
      <SectionHead
        icon="presets"
        title={tr('settings.presets.title')}
        desc={tr('settings.presets.desc')}
      />

      <BoolRow
        label="settings.presets.copies"
        note="settings.presets.copiesNote"
        read={READ_COPIES}
        write={WRITE_COPIES}
        /* 预设页那一表每行的三态由账本算，账本的出口正看这一档 ⇒ 叫它重读本地那一侧 */
        after={presetPrefsMutated}
      />

      {/*
       * 第 ② 档（2026-11-01）：换了「使用中」那一份就抄一遍它的后处理脚本。
       * 判据在前端 `postprocess/autoCopy.ts`，这一格只是它的总开关。
       */}
      <BoolRow
        label="settings.presets.autoCopy"
        note="settings.presets.autoCopyNote"
        read={READ_AUTO_COPY}
        write={WRITE_AUTO_COPY}
      />
    </section>
  )
}
