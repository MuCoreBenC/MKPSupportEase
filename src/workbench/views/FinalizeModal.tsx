/*
 * **交付检查与修复**（②）：发现问题 → 能修的直接修 → 不能修的说去哪修 → 重新检查 → 去发布。
 *
 * 两层：
 *
 *  1. **问题清单**（主视图）：每条给分类、人话结论、以及那一颗对应的按钮。
 *     分类与"能不能修"都由后端 `issues[].kind / action` 说，前端不重算；
 *  2. **详细差异**（折叠）：发布身份检查 / 凭据变化（新旧 SHA + manifest 行级差）/
 *     定稿预览 —— 想看细节时展开，平时不铺一屏红字。
 *
 * 唯一会写盘的按钮是「更新发布凭据」，它只重写 `manifest.json`：不动 `source.json`、
 * 不重算 `catalog.json`、不推进序号、不提交 Git。其余按钮都只是"去哪儿"。
 *
 * 两处**刻意分开**的说法：「交付集合」那一类既不算凭据失效，也不会被塞进 manifest；
 * 附属资产的登记口径差异只报计数，不当故障。
 */

import { useMemo, useState } from 'react'

import ModalC14 from '../c14/ModalC14'
import type {
  FinalizeEntry,
  FinalizeIssue,
  FinalizeIssueKind,
  FinalizePreview,
  FinalizeState,
  IssueWhere,
} from '../api'
import { DiffLines } from './DiffView'
import { fmtSize } from './deliveryStatus'
import c from '../c14.module.css'
import d from './DeliveryFilesModal.module.css'
import s from './FinalizeModal.module.css'

interface Props {
  /** 打开时拉的那一份；null = 正在算 / 失败 */
  preview: FinalizePreview | null
  error: string | null
  loading: boolean
  /** 「更新发布凭据」正在写盘 */
  busy: boolean
  onReload: () => void
  onUpdateCredentials: () => void
  /** 去源配置（哪台机型、哪一版）—— 反查不到时 `at` 为 null，按钮不该出现 */
  onLocate: (at: IssueWhere) => void
  /** 去发布闸：身份问题在那里判 */
  onCheckIdentity: () => void
  onClose: () => void
  /** 只负责进入正式发布界面（不自动发布、也不写任何凭据） */
  onPublish: () => void
}

const STATE_TEXT: Record<FinalizeState, string> = {
  matched: '凭据匹配',
  stale: '凭据失效',
  pendingPublish: '待发布',
  identityFailed: '身份检查未通过',
}

const KIND_TEXT: Record<FinalizeIssueKind, string> = {
  fixable: '可修复',
  source: '需处理源',
  identity: '身份问题',
  info: '登记口径',
}

function kindClass(kind: FinalizeIssueKind): string {
  switch (kind) {
    case 'fixable':
      return s.chipFixable
    case 'source':
      return s.chipSource
    case 'identity':
      return s.chipIdentity
    case 'info':
      return s.chip
  }
}

/** 一条重新登记的人话：size 变了 / 只换 SHA / 新增 */
function entryText(e: FinalizeEntry): string {
  if (e.beforeSha256 === null) return `新增登记 · ${fmtSize(e.size)}`
  if (e.beforeSize !== e.size) {
    return `SHA-256 不一致 · size ${fmtSize(e.beforeSize)} → ${fmtSize(e.size)}`
  }
  return 'SHA-256 不一致 · size 未变'
}

const isChanged = (e: FinalizeEntry) =>
  e.beforeSha256 !== e.sha256 || e.beforeSize !== e.size

/**
 * 禁用原因只留最短那一句：完整原因在列表底下那块 note 里。
 *
 * 不能靠 tooltip —— 禁用的按钮弹不出 tooltip（`workbench.css` 里那条注释已经写明），
 * 于是"为什么点不动"必须摆在按钮旁边。
 */
function shortReason(text: string): string {
  const head = text.split('：')[0]
  return head.length > 46 ? `${head.slice(0, 46)}…` : head
}

/** 主视图那一行的按钮：写盘只有「更新发布凭据」一个 */
function IssueAction({
  issue,
  busy,
  blockedReason,
  onUpdateCredentials,
  onLocate,
  onCheckIdentity,
}: {
  issue: FinalizeIssue
  busy: boolean
  /** 非 null = 定稿那一步算不出来（后端会拒绝写盘），按钮灰掉并把原因写在旁边 */
  blockedReason: string | null
  onUpdateCredentials: () => void
  onLocate: (at: IssueWhere) => void
  onCheckIdentity: () => void
}) {
  switch (issue.action) {
    case 'updateCredentials':
      return (
        <div className={s.actionRow}>
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary} ${c.btnSm}`}
            disabled={busy || blockedReason !== null}
            data-tip={
              blockedReason === null
                ? '按盘上真字节重算 SHA / size 并写回 manifest.json；不动 source.json、不推进序号、不提交'
                : undefined
            }
            onClick={onUpdateCredentials}
          >
            {busy ? '更新中…' : '更新发布凭据'}
          </button>
          {(busy || blockedReason !== null) && (
            <span className={s.actionWhy}>
              {busy ? '正在按盘上真字节重算…' : `暂时不能更新：${blockedReason}`}
            </span>
          )}
        </div>
      )
    case 'locateSource':
      return issue.at === null ? null : (
        <button
          type="button"
          className={`${c.btn} ${c.btnSm}`}
          data-tip="去到对应的机型 / 版本，看它该不该进这次交付"
          onClick={() => onLocate(issue.at as IssueWhere)}
        >
          定位源问题
        </button>
      )
    case 'checkIdentity':
      return (
        <button
          type="button"
          className={`${c.btn} ${c.btnSm}`}
          data-tip="打开十五项发布闸，身份问题在那里判"
          onClick={onCheckIdentity}
        >
          检查发布身份
        </button>
      )
    case 'none':
      return null
  }
}

export default function FinalizeModal({
  preview,
  error,
  loading,
  busy,
  onReload,
  onUpdateCredentials,
  onLocate,
  onCheckIdentity,
  onClose,
  onPublish,
}: Props) {
  const [detailOpen, setDetailOpen] = useState(false)

  const manifest = preview?.manifest ?? null
  const changed = useMemo(
    () => (manifest?.entries ?? []).filter(isChanged),
    [manifest],
  )
  const [selPath, setSelPath] = useState<string | null>(changed[0]?.relativePath ?? null)
  const cur = changed.find((e) => e.relativePath === selPath) ?? changed[0] ?? null

  const sourceCheck = preview?.checks.find((i) => i.id === 'source/correct') ?? null
  const setCheck = preview?.checks.find((i) => i.id === 'manifest/correct') ?? null
  /** ⑭ 的登记差异条数：只报计数，不升级成"凭据失效"，也不逐条铺开 */
  const setDiffs = setCheck?.status === 'warn' ? setCheck.affectedFiles.length : 0
  const sourceOk = sourceCheck?.status === 'pass'

  const todo = preview?.issues.filter((i) => i.kind !== 'info').length ?? 0

  return (
    <ModalC14
      open
      size="xl"
      title="交付检查"
      subtitle={
        preview === null
          ? '交付检查与修复'
          : `${STATE_TEXT[preview.state]} · ${preview.ready ? '交付就绪' : `发现 ${todo} 个问题`}`
      }
      onClose={onClose}
      footer={
        <>
          <button type="button" className={c.btn} onClick={onClose}>
            关闭
          </button>
          <span className={c.grow} />
          <button
            type="button"
            className={`${c.btn} ${c.btnPrimary}`}
            data-tip="只进入正式发布界面（过十五项闸）；这里不会写任何凭据"
            onClick={onPublish}
          >
            去发布
          </button>
        </>
      }
    >
      {loading && <p className={s.empty}>正在检查交付目录……</p>}
      {error !== null && (
        <p className={s.error} role="alert">
          检查没算出来：{error}
        </p>
      )}

      {preview !== null && (
        <div className={s.body}>
          <div className={s.toolbar}>
            <span className={preview.ready ? s.summaryOk : s.summaryTodo}>
              {preview.ready ? '交付就绪' : `发现 ${todo} 个问题`}
            </span>
            <span className={c.grow} />
            <button
              type="button"
              className={`${c.btn} ${c.btnSm}`}
              disabled={loading || busy}
              data-tip="重读盘上的真文件再判一次"
              onClick={onReload}
            >
              重新检查
            </button>
          </div>

          {/* —— 问题清单：能修的直接修，不能修的说去哪修 —— */}
          {preview.issues.length === 0 ? (
            <p className={s.empty}>
              没有发现问题 —— 交付文件与发布凭据一致，可以进正式发布。
            </p>
          ) : (
            <div className={s.issues}>
              {preview.issues.map((issue) => (
                <div className={s.issue} key={issue.id}>
                  <div className={s.issueTitle}>
                    <span className={`${s.chip} ${kindClass(issue.kind)}`}>
                      {KIND_TEXT[issue.kind]}
                    </span>
                    <span>{issue.title}</span>
                  </div>
                  <div className={s.issueAction}>
                    <IssueAction
                      issue={issue}
                      busy={busy}
                      blockedReason={
                        preview.planError === null ? null : shortReason(preview.planError)
                      }
                      onUpdateCredentials={onUpdateCredentials}
                      onLocate={onLocate}
                      onCheckIdentity={onCheckIdentity}
                    />
                  </div>
                  <div className={s.issueDetail}>{issue.detail}</div>
                  {issue.kind !== 'info' && issue.files.length > 0 && (
                    <div className={s.issueFiles}>
                      {issue.files.slice(0, 3).join('、')}
                      {issue.files.length > 3 ? ` 等 ${issue.files.length} 份` : ''}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}

          {preview.planError !== null && (
            <p className={s.note}>
              定稿后的 manifest 还算不出来：{preview.planError} —— 先把上面标「需处理源」的
              处理掉，再来更新凭据。
            </p>
          )}

          <button
            type="button"
            className={s.disclosure}
            aria-expanded={detailOpen}
            onClick={() => setDetailOpen((v) => !v)}
          >
            {detailOpen ? '收起详细差异' : '查看详细差异'}
          </button>

          {detailOpen && (
            <>
              {/* —— 一、发布身份检查 —— */}
              <section className={s.section}>
                <h3 className={s.sectionTitle}>一、发布身份检查</h3>
                <div className={s.rows}>
                  <div className={s.kvRow}>
                    <span>Source 声明</span>
                    <b className={sourceOk ? s.ok : s.bad}>{sourceOk ? '通过' : '未通过'}</b>
                  </div>
                  {!sourceOk && sourceCheck !== null && (
                    <p className={s.note}>
                      {sourceCheck.details}
                      {sourceCheck.affectedFiles.length > 0
                        ? `（${sourceCheck.affectedFiles.join('、')}）`
                        : ''}
                    </p>
                  )}
                  <div className={s.kvRow}>
                    <span>交付集合检查</span>
                    <b className={setDiffs > 0 ? s.warn : s.ok}>
                      {setDiffs > 0 ? `${setDiffs} 项登记差异` : '通过'}
                    </b>
                  </div>
                  {setDiffs > 0 && (
                    <p className={s.note}>
                      {setDiffs} 项是 manifest 与 catalog 对不上的条目（多登了一条、或 SHA
                      还没跟上）—— 和上面那批凭据是同一件事的两个视角，点「更新发布凭据」后会一起归零，
                      不用单独处理。
                    </p>
                  )}
                </div>
              </section>

              {/* —— 二、发布凭据变化 —— */}
              <section className={s.section}>
                <h3 className={s.sectionTitle}>二、发布凭据变化</h3>

                {manifest === null ? (
                  <>
                    <p className={s.error}>
                      定稿后的 manifest 算不出来：{preview.planError ?? '原因未知'}
                    </p>
                    {preview.reasons.length > 0 && (
                      <ul className={s.reasons}>
                        {preview.reasons.map((r) => (
                          <li key={r}>{r}</li>
                        ))}
                      </ul>
                    )}
                  </>
                ) : (
                  <>
                    <div className={s.rows}>
                      <div className={s.kvRow}>
                        <span>待重新登记</span>
                        <b className={manifest.changedEntries > 0 ? s.warn : s.ok}>
                          {manifest.changedEntries > 0
                            ? `${manifest.changedEntries} 份文件发生变化`
                            : '没有要重新登记的文件'}
                        </b>
                      </div>
                    </div>

                    {changed.length > 0 && (
                      <div className={d.panes}>
                        <nav className={d.list} aria-label="要重新登记的交付文件">
                          {changed.map((e) => (
                            <button
                              key={e.relativePath}
                              type="button"
                              className={
                                cur?.relativePath === e.relativePath
                                  ? `${d.item} ${d.itemOn}`
                                  : d.item
                              }
                              aria-current={cur?.relativePath === e.relativePath}
                              data-tip={e.relativePath}
                              onClick={() => setSelPath(e.relativePath)}
                            >
                              <span className={d.itemName}>
                                {e.relativePath.split('/').pop() ?? e.relativePath}
                              </span>
                              <span className={`${d.itemState} ${d.statePending}`}>待登记</span>
                              <span className={d.itemMeta}>{fmtSize(e.size)}</span>
                            </button>
                          ))}
                        </nav>

                        <div className={d.detail}>
                          <div className={d.detailScroll}>
                            {cur !== null && (
                              <>
                                <header className={d.detailHead}>
                                  <span className={`${d.detailName} ${c.mono}`}>
                                    {cur.relativePath}
                                  </span>
                                  <span className={d.detailMeta}>{entryText(cur)}</span>
                                </header>
                                <div className={d.driftBox}>
                                  <div className={s.shaRow}>
                                    <span>旧 SHA-256</span>
                                    <b className={s.sha}>
                                      {cur.beforeSha256 ?? '（原来没有这一条）'}
                                    </b>
                                  </div>
                                  <div className={s.shaRow}>
                                    <span>新 SHA-256</span>
                                    <b className={s.sha}>{cur.sha256}</b>
                                  </div>
                                  <p className={s.note}>
                                    下面是 <span className={c.mono}>{manifest.rel}</span>{' '}
                                    的整体行级变化（定稿会按盘上真字节重写这份清单）。
                                  </p>
                                </div>
                              </>
                            )}
                            {manifest.state === 'unchanged' ? (
                              <p className={s.empty}>
                                定稿后的 manifest 与盘上那份逐字节相同 —— 没有要改的地方。
                              </p>
                            ) : (
                              <DiffLines
                                lines={manifest.lines}
                                mode={manifest.state}
                                view="diff"
                                fileName={manifest.rel}
                              />
                            )}
                          </div>
                        </div>
                      </div>
                    )}

                    <p className={s.note}>
                      正式定稿时会重新写入更新时间。当前预览仅展示内容及 SHA / size 的变化，不展示时间戳变化。
                    </p>
                  </>
                )}
              </section>

              {/* —— 三、定稿预览 —— */}
              <section className={s.section}>
                <h3 className={s.sectionTitle}>三、定稿预览</h3>
                <div className={s.rows}>
                  <div className={s.kvRow}>
                    <span>发布线</span>
                    <b className={c.mono}>{preview.catalogId === '' ? '—' : preview.catalogId}</b>
                  </div>
                  <div className={s.kvRow}>
                    <span>当前序号</span>
                    <b>{preview.serialOnDisk ?? '未发布'}</b>
                  </div>
                  <div className={s.kvRow}>
                    <span>正式发布后</span>
                    <b>{preview.serialNextPublish === 0 ? '—' : preview.serialNextPublish}</b>
                  </div>
                </div>
                <p className={s.note}>
                  更新凭据与这里的预览都不分配序号、不写盘（「更新发布凭据」只写 manifest.json）。正式定稿时会更新 manifest 的更新时间。
                </p>
              </section>
            </>
          )}
        </div>
      )}
    </ModalC14>
  )
}
