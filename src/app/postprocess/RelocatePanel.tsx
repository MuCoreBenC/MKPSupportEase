/*
 * **挪塔那一问的那一屏**（2026-10-18，第三版）。
 *
 * 版式（作者 2026-10-18 定稿）：
 *   上 —— 两幅俯视图并排、**同高**（`MAP_H = 104` = 别的 hero 那口高），
 *        中间**一支箭头**指向「挪后」（不占高度、不做进画布里）；
 *        **不摆「现在 / 挪后」小标题**（作者：「上面的现在挪后不需要」）；
 *   下 —— 原来两栏事实的**位置**摆挪塔文案（左下角小钮翻页，两页同高）。
 *
 * 因为上下两块都是定高，整扇模态框与"结果"那一屏**一样大**（作者：「我很明显感觉
 * 模态框大小不一样」—— 上一版板子按宽度算，板高溢出 104，框就长高了）。
 *
 * ★ 报告页那支箭头说的是"建议挪**模型**多少"，**不能**拿到这里用；
 *   这一屏要表达的是"**塔**从这挪到那"—— 由两幅图和中间那支箭头说，画布里不画箭头。
 */
import type { PostProcessQuestion, RelocateGeo, ReportCollision } from '../../api/contract'
import CollisionMap from '../report/CollisionMap'
import { HERO_H, MAP_CHROME } from './metrics'
import s from './PostProcessModal.module.css'

/** 板子的目标高度 —— 与其它 hero 同一口（见 `metrics.ts`，**不许在这儿写数字**）。
 *  减去画布自带那 2px，整条带子（含描边）才真的是 `HERO_H`。 */
const MAP_H = HERO_H - MAP_CHROME

/** 挪后那一幅：塔框整体平移，别的几何原样；冲突与建议清空（塔已经挪开了） */
function movedCollision(c: ReportCollision | null, geo: RelocateGeo): ReportCollision | null {
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
    conflictPoints: [],
    conflictCount: 0,
    adviceText: '',
  }
}

function parts(question: PostProcessQuestion) {
  const geo = question.geo ?? null
  const c = geo?.collision ?? null
  return {
    c,
    candidate: geo?.candidate ?? null,
    current: geo?.current ?? null,
    after: geo !== null ? movedCollision(c, geo) : null,
  }
}

/**
 * 上：两幅俯视图 + 中间一支箭头。**不带标题**，两块高度都由 `MAP_H` 钉死。
 */
export function RelocateMaps({
  question,
  machineType,
}: {
  question: PostProcessQuestion
  machineType: string | null
}) {
  const { c, after, candidate, current } = parts(question)

  /*
   * **画布里那支白色箭头**（2026-10-18 作者：「我是说图片里面的塔的首尾指向的箭头，
   * 就是这个白色箭头呀」）—— 起点 = 塔框中心，终点 = 目标塔框中心，**如实一支**
   * （长度 = 真实位移，镞尖正落在目标虚框中心；两处同一个位移算出来的）。
   * ★ 上一轮误把它换成了"两块板之间的一支小箭头"，这里换回来。
   */
  let towerArrow: { from: [number, number]; vec: [number, number] } | null = null
  let ghost: { xMin: number; xMax: number; yMin: number; yMax: number } | null = null
  if (c?.towerBbox != null && candidate !== null && current !== null) {
    const box = c.towerBbox
    const center =
      box.xMin === undefined || box.xMax === undefined
        ? null
        : ([(box.xMin + box.xMax) / 2, ((box.yMin ?? 0) + (box.yMax ?? 0)) / 2] as [number, number])
    const dx = candidate[0] - current[0]
    const dy = candidate[1] - current[1]
    if (center !== null) towerArrow = { from: center, vec: [dx, dy] }
    ghost = {
      xMin: (box.xMin ?? 0) + dx,
      xMax: (box.xMax ?? 0) + dx,
      yMin: (box.yMin ?? 0) + dy,
      yMax: (box.yMax ?? 0) + dy,
    }
  }

  return (
    <div className={s.relocateMaps} role="img" aria-label="擦料塔现在的位置与挪动后的位置">
      {/*
       * ┌─ 两处箭头，说的是同一件事的两个尺度，**两个都要**（作者 2026-10-18 定）──────┐
       * │ ① **画布里那支白箭头**（在 `CollisionMap` 里，`arrow={towerArrow}`）：      │
       * │    塔的**首尾** —— 从现在的塔中心指到目标塔中心，长度 = 真实位移；         │
       * │    镞尖正落在 `ghost` 目标虚框中心（同一个位移算的，所以准）。              │
       * │ ② **两块板之间那支箭头**（下面的 `<span className={s.relocateArrow}>`）：  │
       * │    只表示"从这一幅看到下一幅"（现在 → 挪后），跨图阅读的方向感。            │
       * │ ★ 两个都画过又被删过，别再二选一：① 讲**塔挪了多远**，② 讲**看哪一幅**。   │
       * └──────────────────────────────────────────────────────────────────────────┘
       */}
      <div className={s.relocateFig}>
        {c !== null && (
          <CollisionMap
            collision={c}
            machineType={machineType}
            arrow={towerArrow}
            ghost={ghost}
            hideLegend
            height={MAP_H}
          />
        )}
      </div>

      {/*
       * ② **板间箭头**（现在 → 挪后）：**虚线**、绿色（`--accent`），与画布里那支
       *    实心白箭**一眼分得开** —— 作者 2026-10-18：「我之前的虚线箭头呢，或者换一种
       *    样式吧，两个箭头一样的感觉不好…就是点了按钮之后就下一步的就这样挪动的」。
       *    语义 = "下一步 / 接着看"，所以用虚线（过程）+ 主色（要你做的动作）。
       */}
      <span className={s.relocateArrow} aria-hidden="true">
        <svg viewBox="0 0 44 18" width="44" height="18" fill="none">
          <path
            d="M2 9h33"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeDasharray="5 4"
          />
          <path
            d="M32.5 3.4 41 9l-8.5 5.6"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>

      {after !== null && (
        <div className={s.relocateFig}>
          {/* 已经到位了 ⇒ 一支都不画（`arrow={null}` 会回落成报告页那支模型箭头） */}
          <CollisionMap
            collision={after}
            machineType={machineType}
            hideArrow
            hideLegend
            height={MAP_H}
          />
        </div>
      )}
    </div>
  )
}

/**
 * 下：原来两栏事实的**位置**上摆的那段挪塔文案（精简版 —— 作者：「文案有点多」）。
 *
 * 排版按"一张小表"来（作者：「这些也没视觉对齐，没有视觉优化」）：
 *   第一行 = 问句正文（换行对齐左边）；
 *   下面 = 两行两列的坐标（标签定宽、数字等宽），冲突点与倒计时各占一行。
 * 数值**不取整**（作者点名）。
 */
export function RelocateNote({
  question,
  countdown,
}: {
  question: PostProcessQuestion
  /** 倒计时那句（由模态框给，那边每秒 tick 一次 —— 这里自己算不会走字） */
  countdown: string | null
}) {
  const { c, candidate, current } = parts(question)
  return (
    <div className={s.relocateNote} role="status">
      <p className={s.relocateLine}>{question.text}</p>
      <dl className={s.relocateCoords}>
        <div className={s.relocateCoord}>
          <dt>现在塔位置</dt>
          <dd>
            <span className={s.relocateAxis}>X</span> {current?.[0].toFixed(1) ?? '—'}
            <span className={s.relocateSep}>/</span>
            <span className={s.relocateAxis}>Y</span> {current?.[1].toFixed(1) ?? '—'}
          </dd>
        </div>
        <div className={s.relocateCoord}>
          <dt>自动摆放</dt>
          <dd>
            {candidate !== null ? (
              <>
                <span className={s.relocateAxis}>X</span> {candidate[0].toFixed(1)}
                <span className={s.relocateSep}>/</span>
                <span className={s.relocateAxis}>Y</span> {candidate[1].toFixed(1)}
              </>
            ) : (
              '板子上摆不下'
            )}
          </dd>
        </div>
        <div className={s.relocateCoord}>
          <dt>冲突点</dt>
          <dd>{c !== null ? `${c.conflictCount ?? 0} 个` : '—'}</dd>
        </div>
      </dl>
      {countdown !== null && <span className={s.relocateHint}>{countdown}</span>}
    </div>
  )
}
