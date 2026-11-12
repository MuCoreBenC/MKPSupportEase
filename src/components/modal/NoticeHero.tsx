/*
 * **一张"有事要说"的 hero 卡**（警告 / 流程选择）—— `components/modal` 里那条统一的脸。
 *
 * # 它回答的是哪一类事
 *
 * 模态框顶那一带（hero）平时说的是"结果"（进度 / 读数 / 成败）。可这一趟要是**出了点事**
 * （内核留了几条警告、预设与机型对不上、这一盘要挑个跑法），那件事就该**占着这一带**说
 * —— 不许在标题右边挤成一行截断字，也不许在框身里另起一块把模态框撑高。
 *
 * ★ 作者 2026-11-08 的口径（后处理结果那一屏）：「这种出错的就显示在 HERO 里面……
 *   这样就不用说上面也显示错误、下面也显示错误」「我这模态框也不会被撑得很开」。
 *
 * # 三条硬约定
 *
 * 1. **卡高恒 104**（与全站这一档 hero 同一口）：内容多了在卡里滚（`.scroll`），
 *    换一页 / 多一条警告都**不许把框撑高** —— 判据是探针
 *    `scripts/probes/pp-modal-height.mjs`（量同一个框换一屏的高度差 ≤1px）。
 * 2. **三档只换颜色**（面 / 描边 / 墨），骨架一条共用：
 *      `warn` 橙 —— 拦一下，结果可能不对；
 *      `ask`  绿 —— **不是出错**，是"这一盘怎么跑"的流程选择。
 * 3. **文字只说三样**：题目（是什么事）+ 正文（后端/内核原话，不另编）+ 最弱一档的兜底句
 *    （倒计时之类）。多了放不进这口 104 的框，也会把"要不要读"的优先级搅乱。
 */

import type { ReactNode } from 'react'

import s from './NoticeHero.module.css'

/**
 * 两档颜色（`NoticeHero.module.css` 里各一组 token，深浅两套都在 `tokens.css`）：
 *
 *   `warn` 警告橙 —— 出事 / 可能出错
 *   `ask`  中性绿 —— 不是出错，是要你挑一件
 */
export type NoticeTone = 'warn' | 'ask'

export interface NoticeHeroProps {
  tone: NoticeTone
  /** 卡上那行题目（「警告 · 2 条」「预设与机型不匹配 · 等你决定」） */
  title: string
  /** 正文：一条一个段落，多了在卡里滚 —— 后端 / 内核原话照抄，不在这里另编 */
  lines?: string[]
  /** 最弱一档的那一句（倒计时 / "不答会怎样"） */
  hint?: string
  /**
   * 圈里那枚记号，默认是文字「!」。
   *
   * 要别的记号就传**画出来的 SVG**（`✦` 这类字体符号自带基线偏移，`align-items: center`
   * 居中的是行盒不是字形，看上去会偏低 —— 见后处理那一屏的 `SparkMark`）。
   */
  glyph?: ReactNode
  /** 读屏角色：默认 `status`（"这里有一件事"）；打断式的错误才传 `alert` */
  role?: 'status' | 'alert'
}

export function NoticeHero({ tone, title, lines = [], hint, glyph, role = 'status' }: NoticeHeroProps) {
  return (
    <div className={`${s.hero} ${s[tone]}`} role={role}>
      <span className={s.glyph} aria-hidden="true">
        <span className={s.badge}>{glyph ?? '!'}</span>
      </span>
      <div className={s.copy}>
        {/* 一叠（题目 + 正文 + 兜底句）在卡内滚：内容短就居中，超高**从第一行开始滚** */}
        <div className={s.scroll}>
          <span className={s.title}>{title}</span>
          {lines.map((line, i) => (
            <span className={s.line} key={`${i}:${line}`}>
              {line}
            </span>
          ))}
          {hint !== undefined && <span className={s.hint}>{hint}</span>}
        </div>
      </div>
    </div>
  )
}
