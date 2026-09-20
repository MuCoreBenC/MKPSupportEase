import type { CSSProperties } from 'react'
import s from './Skeleton.module.css'

interface SkeletonProps {
  /** 条子宽度。给 em 就跟着字号走，给 px / min() 就按盒子算 */
  width?: string
  label?: string
}

/**
 * 一条「正在取，还没有」的占位条。
 *
 * 只有这一份：三轴读数和预设文件名都用它，呼吸的节奏必须一致 ——
 * 两处各写一份动画，快慢差一点就会看出是两个东西在闪。
 */
export default function Skeleton({ width = '3.4em', label = '正在获取' }: SkeletonProps) {
  return (
    <span
      className={s.bar}
      style={{ '--sk-w': width } as CSSProperties}
      role="img"
      aria-label={label}
    />
  )
}
