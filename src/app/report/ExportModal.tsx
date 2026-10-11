/*
 * 「导出执行报告」那一框（2026-10-11 · 照 `demo2/HANDOFF-EXPORT-3MF-NAME.md` §B）。
 *
 * 三件套一起导 —— **原始文件（改动前的备份） / 处理后文件 / JSON 元数据** ——
 * 就是归档里那三份原样复制出去，不多导网页报告与截图（那些以后可能做，不在这张清单里）。
 *
 * # 谁说了算
 *
 * 文件名与默认落点**都是后端的**（[`api.getReportExportPlan`]）：命名规则只许有一处
 * （`ipc/report.rs` 那个纯函数），界面照单显示。否则"框里写着一个名、盘上落另一个名"
 * 这种错没有任何东西咬得住。
 *
 * # 两条纪律
 *
 * 1. **不在框里造词**：归档里缺的那几份照实说（失败的那一次不会有"处理后的文件"），
 *    不摆一行"缺"的空行、也不假装三件套都齐；
 * 2. **失败留在框里**：导出没成（不覆盖 / 目录建不出来 / 一份都不在盘上）→ 后端那句
 *    原话摆在框里、框不关 —— 用户就着这句话改位置或先把那一份挪走，不用从头来一遍。
 *
 * # 保存位置记着上一次（`STORAGE.clientReportExportDir`）
 *
 * 纯前端偏好，不是底账：它只决定框打开时路径胶囊里先写哪一串。目录不在了照样是
 * 后端说了算（会照实报"建不出保存位置"），所以记住一个失效的路径不会骗人。
 *
 * 胶囊**整条不改位置**：只有右边那颗「更改位置」弹系统框（作者 2026-10-11）——
 * 路径正文因此还能拖选，误点一下也不会跳出一个系统目录框。
 */

import { useEffect, useState } from 'react'
import type { ReactNode } from 'react'

import { api, errorText } from '../../api'
import type { ReportExportKind, ReportExportPlan } from '../../api'
import { STORAGE } from '../../api/storageKeys'
import { Modal } from '../../components/modal'
import { useStickyState } from '../../hooks/useStickyState'
import { Btn } from '../ui/Controls'
import s from './ExportModal.module.css'

/** 三件套的中文说明。后端只给机器名（`kind`），**界面的话住界面这一侧** */
const KIND_LABEL: Record<ReportExportKind, string> = {
  original: '原始文件（改动前的备份）',
  processed: '处理后文件',
  meta: 'JSON 元数据',
}

/** 折角的那张纸（形状与报告页详情里那套同一网格：24×24、currentColor） */
const PAPER_SHAPE = (
  <>
    <path d="M7 3h7l4.5 4.5V20a1.5 1.5 0 0 1-1.5 1.5H7A1.5 1.5 0 0 1 5.5 20V4.5A1.5 1.5 0 0 1 7 3z" />
    <path d="M13.5 3v5h5" />
  </>
)

/**
 * 每份前面的小图标。**三行是同一族的三种文件**：两份 G-code 都是那张纸，
 * 元数据那张纸里多一对花括号（`{ }`）。
 *
 * ★ 2026-10-11 改：原来那枚是"一对大括号 + 中间一根竖线"（v13 抽卡里照搬的），
 *   15px 下读起来像 `(()` —— 作者指着它说改一下。现在三行共用同一个纸张轮廓，
 *   只有元数据多一对小括号：同族、但一眼分得出。
 */
const KIND_ICON: Record<ReportExportKind, ReactNode> = {
  original: PAPER_SHAPE,
  processed: PAPER_SHAPE,
  meta: (
    <>
      {PAPER_SHAPE}
      {/* 花括号（lucide `file-json` 那一对，左右对称落在纸的中间偏下） */}
      <path d="M10 12a1 1 0 0 0-1 1v1a1 1 0 0 1-1 1 1 1 0 0 1 1 1v1a1 1 0 0 0 1 1" />
      <path d="M14 18a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1 1 1 0 0 1-1-1v-1a1 1 0 0 0-1-1" />
    </>
  ),
}

/**
 * 路径胶囊里的那串字：`<父目录><分隔符><基名><分隔符>` 拆两段 ——
 * 前段自己收省略号、**尾两段钉住不缩**（"往哪个盘哪个文件夹里导"永远看得见）。
 *
 * 分隔符按父目录里用的那个认（Windows 收到的是 `\`，macOS 是 `/`）；
 * 段数少到没有"尾两段"可言时整条当前段，不分。
 */
function pathPill(dir: string, base: string): { folder: string; full: string; head: string; tail: string } {
  const sep = dir.includes('\\') ? '\\' : '/'
  const folder = `${dir.replace(/[\\/]+$/, '')}${sep}${base}`
  const full = `${folder}${sep}`
  const lead = full.startsWith(sep) ? sep : ''
  const parts = full.split(/[\\/]/).filter((x) => x !== '')
  if (parts.length <= 2) return { folder, full, head: '', tail: full }
  return {
    folder,
    full,
    head: lead + parts.slice(0, -2).join(sep) + sep,
    tail: parts.slice(-2).join(sep) + sep,
  }
}

export interface ExportModalProps {
  open: boolean
  /** 要导出的那一条记录（`null` = 列表里还没有选中的，那时光标根本打不开） */
  id: string | null
  onClose: () => void
  /** 成了：把落盘的路径与目标文件夹交出去（页面据此报一句"导到哪了"） */
  onExported: (written: string[], folder: string) => void
}

export default function ExportModal({ open, id, onClose, onExported }: ExportModalProps) {
  const [plan, setPlan] = useState<ReportExportPlan | null>(null)
  const [planErr, setPlanErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  /** 导出没成时后端那句原话（留在框里，不关框） */
  const [fail, setFail] = useState<string | null>(null)
  /* 上一个选过的保存位置；空串 = 还没选过 ⇒ 用后端给的默认值（`<用户根>/exports`） */
  const [picked, setPicked] = useStickyState<string>(STORAGE.clientReportExportDir, '')

  /* 开框读一次计划：文件名 / 默认落点 / 缺哪几份都是后端算的 */
  useEffect(() => {
    if (!open || id === null) return
    setPlan(null)
    setPlanErr(null)
    setFail(null)
    let alive = true
    api.getReportExportPlan(id).then(
      (got) => {
        if (alive) setPlan(got)
      },
      (e: unknown) => {
        if (alive) setPlanErr(errorText(e))
      },
    )
    return () => {
      alive = false
    }
  }, [open, id])

  const dir = picked === '' ? (plan?.defaultDir ?? '') : picked
  const path = plan === null || dir === '' ? null : pathPill(dir, plan.base)

  /** 「更改位置」：系统目录选择器。取消（`null`）保持原来那个位置，不是错误 */
  const changeDir = () => {
    void api.pickExportDir(dir).then(
      (next) => {
        if (next !== null) setPicked(next)
      },
      (e: unknown) => setFail(errorText(e)),
    )
  }

  const confirm = () => {
    if (id === null || plan === null || path === null || busy) return
    setBusy(true)
    setFail(null)
    api.exportReportArchive(id, dir).then(
      (written) => {
        setBusy(false)
        onExported(written, path.folder)
        onClose()
      },
      (e: unknown) => {
        setBusy(false)
        setFail(errorText(e))
      },
    )
  }

  return (
    <Modal
      open={open}
      size="md"
      title="导出执行报告"
      onClose={onClose}
      footer={
        <>
          <Btn onClick={onClose}>取消</Btn>
          <Btn variant="accent" disabled={busy || plan === null} onClick={confirm}>
            确认导出
          </Btn>
        </>
      }
    >
      {planErr !== null && (
        <p className={s.bad} role="alert">
          导出清单读不出来：{planErr}
        </p>
      )}
      {plan === null && planErr === null && <p className={s.filling}>正在读导出清单…</p>}

      {plan !== null && (
        <ul className={s.fileList}>
          {plan.files.map((f) => (
            <li key={f.kind} className={s.fileItem}>
              <span className={s.fileIcon} aria-hidden="true">
                <svg
                  width={15}
                  height={15}
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth={1.7}
                  strokeLinecap="round"
                  strokeLinejoin="round"
                >
                  {KIND_ICON[f.kind]}
                </svg>
              </span>
              <span className={s.fileMeta}>
                <span className={s.fileName}>{f.name}</span>
                <span className={s.fileKind}>{KIND_LABEL[f.kind]}</span>
              </span>
            </li>
          ))}
        </ul>
      )}

      {plan !== null && plan.missing.length > 0 && (
        <p className={s.notFull}>
          归档里没有{plan.missing.map((k) => `「${KIND_LABEL[k]}」`).join('、')} ——
          它不会出现在导出清单里。
        </p>
      )}

      {fail !== null && (
        <p className={s.bad} role="alert">
          导出没成：{fail}
        </p>
      )}

      {path !== null && (
        /*
         * 整条是**显示**不是按钮：只有右边那颗「更改位置」改位置（作者 2026-10-11）。
         * 路径正文因此还能拖选、误点一下也不会弹系统框。
         */
        <div className={s.pathPill} title={path.full}>
          <svg
            className={s.pathIcon}
            width={15}
            height={15}
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth={1.7}
            aria-hidden="true"
          >
            <path d="M3.5 6.5A2 2 0 0 1 5.5 4.5h4l2 2.5h7a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z" />
          </svg>
          <span className={s.pathText}>
            <span className={s.pathHead}>{path.head}</span>
            <span className={s.pathTail}>{path.tail}</span>
          </span>
          <button type="button" className={s.pathEdit} onClick={changeDir}>
            更改位置
          </button>
        </div>
      )}
    </Modal>
  )
}
