/*
 * 一行预设文件 —— **首页那一步用的那个**（预设页换成表格之后只剩首页在用）。
 *
 *   [MKP] A1MF_260628.toml                  默认交付   已应用
 *         presets/mkp/A1MF_260628.toml      未知
 *
 * # 为什么没有改掉它
 *
 * 预设页换成了 6 列表格（`PresetTable`：本地表 / 云端表共用一个件，动作全在右键菜单里）。
 * 这一个件
 * 首页（`PageHome` 的第四步）还在用，而试验场的改动半径**只有预设页** ——
 * 改掉它等于把首页的样子也一起改了。所以两者并存：
 *
 *   `PresetTable`    预设页。两张表 × 6 列，状态与来源各一列，动作全在右键菜单里
 *   `PresetFileRow`  首页。只区分「已应用 / 未应用」，交付身份用角标，`readOnly` 不给按钮
 *
 * 等哪一轮轮到首页，把首页也换到表格、这个文件就能删。
 *
 * # 「下载」点了会抛，这是设计
 *
 * 可选文件的下载走 `api.downloadFiles()`，假后端**直接抛 `NotImplementedError` 并在控制台点名**
 * （假装下载成功比抛错糟得多 —— 界面会说「已就绪」，而本地一个文件都没有）。
 * 所以这里必须接住它并就地显示「尚未实现」：**不许整页白屏**，也不许静默吞掉。
 * 首页传的是 `readOnly`，所以那一页根本不出现这个按钮（一个动作两个入口，迟早有一个忘了改）。
 *
 * # 大小与时间
 *
 * `FileRef.size` 恒为 undefined（上游快照里 size 是 0、sha256 是空串），title 里写明为什么。
 * 不显示 0 —— 那会让「不知道」长得像「空文件」。**时间这一列不做**。
 */

import { useState } from 'react'
import { api, NotImplementedError } from '../../api'
import type { FileRef } from '../../api'
import OriginChip from '../shared/OriginChip'
import {
  DOWNLOAD_NOT_READY,
  DOWNLOAD_WHY,
  KIND_LABEL,
  KIND_NAME,
  SIZE_WHY,
  UNKNOWN,
} from './presetTree'
import type { PresetFileNode } from './presetTree'
import s from './PresetFileRow.module.css'

interface Props {
  file: PresetFileNode
  /** 这一份是当前机型 / 版本正在用的那个 MKP 预设 */
  applied: boolean
  /** 只读模式（首页的文件清单）：不给下载按钮 */
  readOnly?: boolean
}

type DownloadState = 'idle' | 'busy' | 'failed'

export default function PresetFileRow({ file, applied, readOnly = false }: Props) {
  const [state, setState] = useState<DownloadState>('idle')
  const [note, setNote] = useState<string | null>(null)

  const download = () => {
    const ref: FileRef = {
      kind: file.kind,
      fileName: file.fileName,
      path: file.path,
      size: file.size,
    }
    setState('busy')
    setNote(null)
    api.downloadFiles([ref]).then(
      () => {
        /* 真后端接上以后走这一支。这一轮到不了这里 —— 假后端一定抛 */
        setState('idle')
        setNote('已交给外壳下载')
      },
      (e: unknown) => {
        setState('failed')
        setNote(
          e instanceof NotImplementedError
            ? DOWNLOAD_NOT_READY
            : `下载失败：${e instanceof Error ? e.message : String(e)}`,
        )
      },
    )
  }

  return (
    <div className={s.row} data-applied={applied}>
      <span className={s.kind} title={KIND_NAME[file.kind]}>
        {KIND_LABEL[file.kind]}
      </span>

      <span className={s.main}>
        <span className={s.name}>{file.fileName}</span>
        <span className={s.path} title={file.path}>
          {file.path}
        </span>
      </span>

      <span className={s.size} title={SIZE_WHY}>
        大小 {UNKNOWN}
      </span>

      <OriginChip delivery={file.delivery} />

      <span className={s.act}>
        {applied && <span className={s.applied}>● 已应用</span>}

        {!readOnly && file.delivery === 'optional' && (
          <button
            type="button"
            className={s.download}
            disabled={state === 'busy'}
            title={DOWNLOAD_WHY}
            onClick={download}
          >
            下载
          </button>
        )}

        {note !== null && (
          <span className={state === 'failed' ? s.noteBad : s.note} title={DOWNLOAD_WHY}>
            {note}
          </span>
        )}
      </span>
    </div>
  )
}
