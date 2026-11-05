/**
 * 生成记录 —— 历次**生成**的回执（`<appDataDir>/workbench/receipts/*.json`）。
 *
 * 一次生成留一条：每一步做了什么、真写了哪些文件、哪些没变、失败卡在哪。
 * **最新在前**（后端保证顺序，这里不重排）。
 *
 * # 它只记"生成"
 *
 * 保存 / 发布 / 外部改文件都**不在这里**（发布有它自己的「发布历史」）。
 * 作者 2026-11-01 明确：三种东西不要塞进同一套机制。
 *
 * # 它不是什么
 *
 * 不是流水线状态机、不是审计账本 —— 最多留 50 份（后端按份数删最旧的）。
 * 删掉这些文件不影响任何生成判据，只丢展示。
 */

import { useEffect, useState } from 'react'

import { isAppError, wb } from '../api'
import ModalC14 from '../c14/ModalC14'
import c from '../c14.module.css'
import h from './HistoryModal.module.css'

/** 一步的键（后端 `receipt::StepRow.key`）→ 一句人话。认不出就原样显示（不猜） */
const STEP_TEXT: Record<string, string> = {
  frozen: '冻结源（只读一份）',
  delivery: '写交付产物',
  assets: '写内核产物',
  baseline: '同步对照基线',
  embedded: '重算内嵌目录',
  ledger: '记生成台账',
  catalog: '重算交付目录',
  commit: '落盘收尾',
  verify: '三面核对',
}

/** 一轮生成的回执（与后端 `receipt::GenerationReceipt` 逐字段对应） */
type Receipt = Awaited<ReturnType<typeof wb.receipts>>[number]

interface Props {
  onClose: () => void
}

/** `2026-11-01T04:37:47Z` → `11-01 04:37`（本地时区；解析不出来就原样给） */
function whenText(iso: string): string {
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return iso
  const p = (n: number) => String(n).padStart(2, '0')
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`
}

export default function GenHistoryModal({ onClose }: Props) {
  const [list, setList] = useState<Receipt[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  /** 打开时读一次（**只读一次** —— 回执是历史，不是活状态，不轮询） */
  useEffect(() => {
    let alive = true
    void (async () => {
      try {
        const got = await wb.receipts()
        if (alive) setList(got)
      } catch (e) {
        if (alive) setError(isAppError(e) ? e.message : String(e))
      }
    })()
    return () => {
      alive = false
    }
  }, [])

  return (
    <ModalC14
      open
      size="lg"
      title="生成记录"
      subtitle={
        list === null
          ? error === null
            ? '正在读……'
            : '没读出来'
          : `${list.length} 条回执 · 最新在前 · 只记生成，不含保存与发布`
      }
      onClose={onClose}
      footer={
        <>
          <span className={h.footNote}>
            回执住应用数据目录的 <span className={c.mono}>workbench/receipts/</span>
            （不在仓库里，只留最近 50 份）；删掉只丢展示，不影响生成
          </span>
          <span className={c.grow} />
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
        </>
      }
    >
      {error !== null && (
        <p className={h.error} role="alert">
          {error}
        </p>
      )}

      {list !== null && list.length === 0 && (
        <p className={h.empty}>还没有生成记录 —— 点一次「生成」就会出现一条。</p>
      )}

      {list !== null && list.length > 0 && (
        <div className={h.list} role="list">
          {list.map((r) => (
            <div key={r.id} className={h.row} role="listitem">
              <div className={h.when}>{whenText(r.stamp)}</div>
              <div className={h.body}>
                <div className={h.title}>
                  <span className={c.mono}>{r.id}</span>
                  {' · '}
                  <strong>{r.status === 'passed' ? '全部通过' : '有失败'}</strong>
                </div>
                {/* 每一步的真实结果：谁没过、卡在哪一步，一眼看得到 */}
                <div className={h.steps}>
                  {r.steps.map((st) => (
                    <span
                      key={st.key}
                      className={st.ok ? h.stepOk : h.stepBad}
                      title={STEP_TEXT[st.key] ?? st.key}
                    >
                      {STEP_TEXT[st.key] ?? st.key}
                      {st.detail !== '' && <em> {st.detail}</em>}
                    </span>
                  ))}
                </div>
                <div className={h.meta}>
                  交付 <b>写 {r.delivery.written}</b> / 未变 {r.delivery.unchanged}
                  {r.delivery.failed > 0 && <b style={{ color: '#dc2626' }}> / 失败 {r.delivery.failed}</b>}
                  {' · '}内核 <b>写 {r.assets.written}</b> / 未变 {r.assets.unchanged}
                  {r.assets.failed > 0 && <b style={{ color: '#dc2626' }}> / 失败 {r.assets.failed}</b>}
                  {' · '}基线 <b>写 {r.baseline.written}</b> / 未变 {r.baseline.unchanged}
                  {r.files.length > 0 && (
                    <>
                      {' · '}真写了 <b>{r.files.length}</b> 份
                      <span className={c.mono}> {r.files.join('、')}</span>
                    </>
                  )}
                </div>
                {r.error !== null && (
                  <p className={h.error} role="alert">
                    {r.error}
                  </p>
                )}
              </div>
            </div>
          ))}
        </div>
      )}
    </ModalC14>
  )
}
