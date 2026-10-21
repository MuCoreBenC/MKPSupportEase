/*
 * **与最初那份的对比**（2026-10-21，作者：「我希望像那个 IDE 那些软件一样，就这种红的
 * 绿的这种对比啊，如果位置足够就可以左右对比」）。
 *
 * 左边 = 最初那份（`baseline-local/` 的原文镜像），右边 = 现在编辑框里的正文。
 * 行级：改过的行**左红右绿**，删掉的行只有左边（红），新增的行只有右边（绿）。
 *
 * # 为什么是弹窗、不是底栏那条展开
 *
 * 作者：「他这个对比的话，我不希望是在这下面这样子对比」——底栏那条只留一句摘要
 * （「与最初那份比：3 行不同」），点它才开这一屏。正文有几行差异要看清，只能横向铺开。
 *
 * # 实时
 *
 * `lineDiff` 是前端纯函数，`right` 每次渲染都跟着 `editing.text` 走 —— 开着这一屏
 * 接着改，红绿块跟着变，不用保存、不用刷新。
 */

import { useMemo } from 'react'

import { Modal } from '../../components/modal'
import { Btn } from '../ui/Controls'
import s from './TextDiffModal.module.css'
import { diffSummary, lineDiff } from './textDiff'

export default function TextDiffModal({
  open,
  host,
  fileName,
  left,
  right,
  onClose,
}: {
  open: boolean
  /** 宿主的整块页面元素（与别的模态框同一条：框要算出可用宽度） */
  host: HTMLElement | null
  fileName: string | undefined
  /** 最初那份（基准） */
  left: string
  /** 现在这份 */
  right: string
  onClose: () => void
}) {
  const rows = useMemo(() => lineDiff(left, right), [left, right])
  const sum = useMemo(() => diffSummary(rows), [rows])
  const total = sum.changed + sum.added + sum.removed

  return (
    <Modal
      open={open}
      size="xl"
      bodyFill
      host={host}
      title="与最初那份对比"
      subtitle={fileName}
      onClose={onClose}
      footer={
        <>
          <span className={s.legend}>
            <i className={s.dotOld} aria-hidden="true" />最初那份
            <i className={s.dotNow} aria-hidden="true" />现在这份
          </span>
          <Btn variant="ghost" onClick={onClose}>
            关闭
          </Btn>
        </>
      }
    >
      <div className={s.wrap}>
        <p className={s.sum}>
          {total === 0 ? (
            '与最初那份完全一致。'
          ) : (
            <>
              改了 <b>{sum.changed}</b> 行
              {sum.added > 0 && (
                <>
                  ，多了 <b>{sum.added}</b> 行
                </>
              )}
              {sum.removed > 0 && (
                <>
                  ，少了 <b>{sum.removed}</b> 行
                </>
              )}
              {' '}—— 左边是它最初落盘时的样子。
            </>
          )}
        </p>

        <div className={s.diff} role="table" aria-label="最初那份与现在的差异">
          {rows.map((r, i) => (
            <div className={s.row} data-kind={r.kind} role="row" key={i}>
              <span className={s.no} role="cell">
                {r.leftNo ?? ''}
              </span>
              <span className={s.code} data-side="left" role="cell" title={r.left ?? undefined}>
                {r.left ?? ''}
              </span>
              <span className={s.no} role="cell">
                {r.rightNo ?? ''}
              </span>
              <span className={s.code} data-side="right" role="cell" title={r.right ?? undefined}>
                {r.right ?? ''}
              </span>
            </div>
          ))}
        </div>
      </div>
    </Modal>
  )
}
