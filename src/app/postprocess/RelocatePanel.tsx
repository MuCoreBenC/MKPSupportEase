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
 * 下：原来两栏事实的**位置**上摆的那段挪塔文案（2026-10-18 第七版，作者定稿）。
 *
 * 作者这一版的口径是「**横向关系 + 信息的轻重**」，不添装饰：
 *   ① 问句（主色、加粗、居中）；
 *   ② **一组**横向坐标：`当前塔位置  X … / Y …  →  自动摆放位置  X … / Y …`
 *      —— 中间那支箭头说的是"从这里挪到那里"，两栏因此是一个整体（不再是一道竖线分栏）；
 *   ③ **并排**的一句底：左边冲突点数（橙三角 + 数字 + 「个冲突点」）、右边倒计时
 *      —— 两件事同属"现在什么状况"，共用一段，不再用横线把内容切碎。
 *
 * 轻重的分配（作者原话）：冲突数量是"要不要处理"的关键事实 ⇒ **深色粗体** + 橙色图标；
 * 倒计时只是"不管会怎样"的兜底 ⇒ 灰字 + 绿时钟。**不用**大橙色数字（橙只留给警告）。
 *
 * 其余口径不变：数值不取整、等宽数字；问句里不塞数字；整页仍装进翻页槽，
 * 模态框不跟着变高（判据 `scripts/probes/pp-modal-height.mjs`）。
 */

/** 冲突点那枚三角叹号（**画成 SVG** —— 字形自带基线偏移，圈里/行里对不齐） */
function WarnTriangle() {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true">
      <path
        d="M8 2 14.6 13.6H1.4L8 2Z"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinejoin="round"
      />
      <path d="M8 6.2v3.3" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <circle cx="8" cy="11.4" r="0.72" fill="currentColor" />
    </svg>
  )
}

/** 倒计时那一枚时钟（同上：几何坐标画，不摆文本符号） */
function ClockMark() {
  return (
    <svg viewBox="0 0 14 14" fill="none" aria-hidden="true">
      <circle cx="7" cy="7" r="5.5" stroke="currentColor" strokeWidth="1.3" />
      <path
        d="M7 3.9V7l2.1 1.4"
        stroke="currentColor"
        strokeWidth="1.3"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

/**
 * 两栏坐标之间那支箭头（作者 2026-10-18：「用箭头表达『当前位置 → 建议位置』，
 * 比单纯竖线分栏更有含义」）。
 *
 * ★ 与"两幅板之间那支"分工不同：那一支说的是"**塔**从这挪到那"（实心、主色）；
 *   这一支说的是"**这两组读数**是同一个东西的前后"（细、灰，只做关系提示）。
 */
function SwapArrow() {
  return (
    <svg viewBox="0 0 20 14" fill="none" aria-hidden="true">
      <path d="M1 7h14.6" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
      <path
        d="M11.8 2.9 16.6 7l-4.8 4.1"
        stroke="currentColor"
        strokeWidth="1.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  )
}

/** 一组坐标（`X 78.0 / Y 91.0`）—— 轴字母压小压暗，数字大一号等宽 */
function CoordPair({ pair }: { pair: [number, number] | null }) {
  /* 空格写实（`{' '}`）：JSX 里跨行的空白会被吃掉，斜杠两边不留空就成了 `78.0/Y` */
  return (
    <>
      <span className={s.relocateAxis}>X</span> {pair?.[0].toFixed(1) ?? '—'}{' '}
      <span className={s.relocateSep}>/</span>{' '}
      <span className={s.relocateAxis}>Y</span> {pair?.[1].toFixed(1) ?? '—'}
    </>
  )
}

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
    <div className={s.relocateNote}>
      <p className={s.relocateAsk}>{question.text}</p>
      {/* ② 一组坐标：现在 → 建议（箭头只讲"这两组是同一个东西的前后"） */}
      <div className={s.relocateCols}>
        <div className={s.relocateCol}>
          <span className={s.relocateColLabel}>当前塔位置</span>
          <span className={s.relocateValue}>
            <CoordPair pair={current} />
          </span>
        </div>
        <span className={s.relocateSwap} aria-hidden="true">
          <SwapArrow />
        </span>
        <div className={`${s.relocateCol} ${s.relocateColNew}`}>
          <span className={s.relocateColLabel}>自动摆放位置</span>
          <span className={s.relocateValue}>
            {candidate !== null ? <CoordPair pair={candidate} /> : '板子上摆不下'}
          </span>
        </div>
      </div>
      {/* ③ 句底并排：左 = 冲突点数（关键事实，深色粗体），右 = 倒计时（兜底，灰字） */}
      <div className={s.relocateMeta}>
        <span className={s.relocateHit}>
          <span className={s.relocateIcon}>
            <WarnTriangle />
          </span>
          <span className={s.relocateHitValue}>{c !== null ? (c.conflictCount ?? 0) : '—'}</span>
          <span className={s.relocateHitUnit}>个冲突点</span>
        </span>
        {countdown !== null && (
          <span className={s.relocateTimer}>
            <span className={`${s.relocateIcon} ${s.relocateIconCalm}`}>
              <ClockMark />
            </span>
            {countdown}
          </span>
        )}
      </div>
    </div>
  )
}
