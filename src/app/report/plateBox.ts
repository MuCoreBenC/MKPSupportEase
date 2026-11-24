/*
 * 校准画布上「实测包围盒」这一圈虚线画不画。
 *
 * 模型轮廓与实测包围盒只差 1mm 上下时，两条灰虚线在缩略图上就是一对平行双线
 * （粗细一样、间距不到一个像素）。差别看不出来的时候，多画一圈只是噪音 ——
 * 尺寸本来就另有地方写（图例与「校准身份」组的「实测尺寸」）。
 *
 * 判据：两者外接框四边的最大差值 ≤ {@link PLATE_BOX_GAP_MM} ⇒ 只画轮廓。
 */
/** 两条线差不到这个距离（mm）就当作重合 —— 缩略图上已经分不出来 */
export const PLATE_BOX_GAP_MM = 1.5

/** 一块轮廓（机器坐标）的外接框；四个数都在 */
export interface OutlineBounds {
  xMin: number
  xMax: number
  yMin: number
  yMax: number
}

/** 一块轮廓（机器坐标）的外接框；空 / 退化 ⇒ `null` */
export function outlineBounds(outlines: [number, number][][]): OutlineBounds | null {
  let xMin = Infinity
  let xMax = -Infinity
  let yMin = Infinity
  let yMax = -Infinity
  for (const poly of outlines) {
    for (const p of poly) {
      if (!Array.isArray(p) || p.length !== 2) continue
      if (p[0] < xMin) xMin = p[0]
      if (p[0] > xMax) xMax = p[0]
      if (p[1] < yMin) yMin = p[1]
      if (p[1] > yMax) yMax = p[1]
    }
  }
  return Number.isFinite(xMin) && Number.isFinite(yMin) ? { xMin, xMax, yMin, yMax } : null
}

/**
 * 这一圈实测包围盒还要不要画。
 *
 * - 没有轮廓（旧记录 / 空）⇒ 要画：那时框是图上唯一的几何；
 * - 有轮廓、四边都贴着（≤ {@link PLATE_BOX_GAP_MM}）⇒ 不画；
 * - 差得多（打印收缩、轮廓被裁）⇒ 要画，两条线说的是不同的事。
 */
export function shouldShowMeasuredBox(
  measured: [number, number, number, number],
  outlines: [number, number][][],
): boolean {
  const bounds = outlineBounds(outlines)
  if (bounds === null) return true
  const [x0, x1, y0, y1] = measured
  const gap = Math.max(
    Math.abs(x0 - bounds.xMin),
    Math.abs(x1 - bounds.xMax),
    Math.abs(y0 - bounds.yMin),
    Math.abs(y1 - bounds.yMax),
  )
  return gap > PLATE_BOX_GAP_MM
}
