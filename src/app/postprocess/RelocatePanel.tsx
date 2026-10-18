/*
 * **挪塔那一问的那一屏**（2026-10-18）。
 *
 * 撞了之后不是三行字就能答明白的：用户要看**塔在哪、模型在哪、撞在哪一片**，
 * 才敢决定"把塔挪过去"还是"我自己回切片器挪模型"。作者定的版式：
 * **左侧（或居中）画布、右侧文字**（不要塞进那口 104px 的警告 hero 里）。
 *
 * # 两幅图
 *
 * 「现在」= 后端给的那一份事实；「挪后」= **同一份事实**，只把塔框按
 * `candidate - current` 平移（别的几何一个不动 —— 只挪塔，不挪模型）。
 * 于是用户看到的是"塔从这里挪到那里"这一件事，不是两张互不相干的图。
 *
 * # 坐标
 *
 * 照实给（**不取整** —— 作者：「可以显示坐标，不用整数也可以的」）。
 */
import type { PostProcessQuestion, RelocateGeo, ReportCollision } from '../../api/contract'
import CollisionMap from '../report/CollisionMap'
import s from './PostProcessModal.module.css'

/** 挪后那一幅：塔框整体平移，别的几何原样 */
function movedCollision(
  c: ReportCollision | null,
  geo: RelocateGeo,
): ReportCollision | null {
  const box = c?.towerBbox
  const candidate = geo.candidate
  if (c == null || box == null || candidate == null) return null
  const dx = candidate[0] - geo.current[0]
  const dy = candidate[1] - geo.current[1]
  /* 框上那几格在契约里是可选的（旧记录可能没有）⇒ 缺了按 0 算 —— 那时画布本来就是空的 */
  return {
    ...c,
    towerBbox: {
      xMin: (box.xMin ?? 0) + dx,
      xMax: (box.xMax ?? 0) + dx,
      yMin: (box.yMin ?? 0) + dy,
      yMax: (box.yMax ?? 0) + dy,
    },
    /* 挪完之后那句"往哪挪多少"已经不作数了 —— 塔挪开了，冲突与建议都该消失 */
    conflictPoints: [],
    conflictCount: 0,
    adviceText: '',
  }
}

export default function RelocatePanel({
  question,
  machineType,
  countdown,
}: {
  question: PostProcessQuestion
  /** 机型（取板规格要它；没有就用退回的圆角矩形床） */
  machineType: string | null
  /** 倒计时那句（「1 分 52 秒后自动终止」）；`null` = 不摆 */
  countdown: string | null
}) {
  const geo = question.geo ?? null
  const c = geo?.collision ?? null
  const candidate = geo?.candidate ?? null
  const current = geo?.current ?? null
  const after = geo !== null ? movedCollision(c, geo) : null

  return (
    <div className={s.relocateHero} role="status">
      {/* 左：画布（现状 / 挪后 两幅；点任意一幅可放大看） */}
      <div className={s.relocateMaps}>
        <figure className={s.relocateMap}>
          <figcaption className={s.relocateMapTag}>现在</figcaption>
          {c !== null && <CollisionMap collision={c} machineType={machineType} />}
        </figure>
        {after !== null && (
          <figure className={s.relocateMap}>
            <figcaption className={`${s.relocateMapTag} ${s.relocateMapTagNew}`}>挪后</figcaption>
            <CollisionMap collision={after} machineType={machineType} />
          </figure>
        )}
      </div>

      {/* 右：话 + 坐标 + 倒计时（文案不在这里编，正文就是后端那一句） */}
      <div className={s.relocateCopy}>
        <span className={s.relocateTitle}>擦料塔与模型撞上了 · 等你决定</span>
        <span className={s.relocateLine}>{question.text}</span>
        <dl className={s.relocateCoords}>
          {current !== null && (
            <div className={s.relocateCoord}>
              <dt>现在塔位置</dt>
              <dd>
                X {current[0].toFixed(1)} / Y {current[1].toFixed(1)}
              </dd>
            </div>
          )}
          <div className={s.relocateCoord}>
            <dt>自动摆放</dt>
            <dd>
              {candidate !== null
                ? `X ${candidate[0].toFixed(1)} / Y ${candidate[1].toFixed(1)}`
                : '板子上摆不下'}
            </dd>
          </div>
          {c !== null && (
            <div className={s.relocateCoord}>
              <dt>冲突点</dt>
              <dd>{c.conflictCount ?? 0} 个</dd>
            </div>
          )}
        </dl>
        {countdown !== null && <span className={s.relocateHint}>{countdown}</span>}
      </div>
    </div>
  )
}
