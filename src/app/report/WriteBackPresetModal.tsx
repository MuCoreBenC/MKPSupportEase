import { useEffect, useState } from 'react'

import Modal from '../../components/modal/Modal'
import { Btn } from '../ui/Controls'
import { useT, type Tr } from '../../i18n'
import { api, errorText, isAppError } from '../../api'
import type { LocalPresetFile, ParamEdit, ReportDebugFacts } from '../../api/contract'
import s from './WriteBackPresetModal.module.css'

/**
 * 「把这次调试的六个值写回某份本地预设」（2026-11-12 作者）。
 *
 * 作者原话：「弹出模态框，然后可以选写到哪个预设里面，能不能判断出 a1 的还是 mini 的，
 * 或者 p1 的？然后排在前面，剩下的折叠起来，而且预设，是本地的预设」。
 *
 * ★ **只写 `pen_*`**：落笔/收笔各自的等待位与目标位、落笔/收笔两档速度。
 *   调试里那些（测几遍、移动 Y、驻点速度、高级模式那段代码）预设里没有对应字段，
 *   写不进去 —— 界面上也不列，免得以为写进去了。
 *
 * ★ **跟随后没定过的等待位不写**（2026-11-12）：勾着「跟随驻点位」时那两个数
 *   只是驻点位带出来的，写进预设会把 P 系列的 246 变成 20。旧记录没有"是否跟随"
 *   这一项，同样不写 —— 不拿驻点位回填。
 *
 * ★ **不发布**：这是给用户的，写进他自己那份 `presets-local/` 就完事；
 *   交付物那份是开发者的活（他自己改工作台）。
 *
 * ★ 写入走现成的草稿链（`savePresetParams` → begin / patch / commit），
 *   与参数页那颗「保存」同一条路，不另开一套写盘。
 */
export default function WriteBackPresetModal({
  open,
  onClose,
  machine,
  facts,
  onDone,
  host,
}: {
  host: HTMLElement | null
  open: boolean
  onClose: () => void
  /** 这份 G-code 的机型（报告记录里的 `presetMachine`），用来把匹配的预设排前面 */
  machine: string | null
  facts: ReportDebugFacts
  /** 写成功后回调（父组件拿去显示回执） */
  onDone: (fileName: string) => void
}) {
  const tr = useT()
  const [files, setFiles] = useState<LocalPresetFile[] | null>(null)
  const [err, setErr] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  const [picked, setPicked] = useState<string | null>(null)
  /* 「其余机型」默认收起：常见情况就是写自己这台 */
  const [showOthers, setShowOthers] = useState(false)
  /* 写完停在这扇窗里给一句回执（父组件不用另找地方摆） */
  const [done, setDone] = useState<string | null>(null)
  /* 选中那份预设里这六项现在是多少；`null` = 还没选 */
  const [presetValues, setPresetValues] = useState<Record<string, string> | null>(null)
  /* 点 ✕ 排除的那几项：不写进预设（作者：「加一下叉叉的，一些就不用」） */
  const [skipped, setSkipped] = useState<Set<string>>(() => new Set())

  useEffect(() => {
    if (!open) return
    setErr(null)
    setPicked(null)
    setShowOthers(false)
    setDone(null)
    setSkipped(new Set())
    void api
      .getLocalPresetFiles()
      .then(setFiles)
      .catch((e: unknown) => {
        setFiles([])
        setErr(isAppError(e) ? errorText(e) : String(e))
      })
  }, [open])

  useEffect(() => {
    if (picked === null) {
      setPresetValues(null)
      return
    }
    void api
      .readPresetParams(picked)
      .then((v) => setPresetValues(v.values))
      .catch(() => setPresetValues({}))
  }, [picked])

  /** 预设里这一项的值；没选 / 这份里没这一项 ⇒ `-` */
  const existingOf = (key: string): string => {
    if (presetValues === null) return '-'
    const short = key.replace('toolhead.', '')
    return presetValues[key] ?? presetValues[short] ?? '-'
  }

  const family = familyOf(machine ?? '')
  const mine = (files ?? []).filter((f) => familyOf(f.fileName) === family)
  const others = (files ?? []).filter((f) => familyOf(f.fileName) !== family)

  const rows = sixRows(facts, tr)
  /** 真正会落盘的条数：跟随的、旧记录没记的、点 ✕ 排除的都不算 */
  const writable = rows.filter((r) => r.value !== null && !skipped.has(r.key)).length

  const write = () => {
    if (picked === null) return
    setBusy(true)
    setErr(null)
    const edits: ParamEdit[] = []
    for (const r of rows) {
      if (r.value === null || skipped.has(r.key)) continue
      edits.push({ paramKey: r.key, value: r.value })
    }
    void api
      .savePresetParams(picked, edits)
      .then(() => {
        const name = (files ?? []).find((f) => f.path === picked)?.fileName ?? picked
        setDone(name)
        onDone(name)
      })
      .catch((e: unknown) => {
        setErr(isAppError(e) ? errorText(e) : String(e))
        /*
         * 草稿链是 begin → patch → commit：中途失败时那份草稿可能已经开了。
         * 不丢掉的话，用户下次打开参数页会看到一份不知哪儿来的未保存改动。
         */
        void api.discardPresetDraft().catch(() => {})
      })
      .finally(() => setBusy(false))
  }

  return (
    <Modal
      open={open}
      size="md"
      title={tr('report.debug.writeTitle')}
      subtitle={tr('report.debug.writeHint')}
      host={host}
      onClose={onClose}
      footer={
        <>
          <span className={done === null ? s.footNote : `${s.footNote} ${s.okNote}`}>
            {done === null ? '' : tr('report.debug.writeDone', { file: done })}
          </span>
          {done === null ? (
            <>
              <Btn disabled={busy} onClick={onClose}>
                {tr('common.cancel')}
              </Btn>
              <Btn
                variant="primary"
                disabled={busy || picked === null || writable === 0}
                onClick={write}
              >
                {tr('report.debug.writeDo')}
              </Btn>
            </>
          ) : (
            <Btn variant="primary" onClick={onClose}>
              {tr('settings.calib.debugDone')}
            </Btn>
          )}
        </>
      }
    >
      {/* 六个值先摆出来：写之前能核对自己要落的是哪几个数 */}
      <div className={s.valueGrid}>
        {/* 值区是两列，表头也各给一份 —— 只给右边那列的话左边不知道那两个数是什么 */}
        <div className={s.valueHead}>
          <span>{tr('report.debug.writeColReport')}</span>
          <span>{tr('report.debug.writeColPreset')}</span>
        </div>
        <div className={s.valueHead}>
          <span>{tr('report.debug.writeColReport')}</span>
          <span>{tr('report.debug.writeColPreset')}</span>
        </div>
        {rows.map((r) => {
          /* 数据决定的不写（跟随 / 旧记录）：不给 ✕，也没法改成写 */
          const auto = r.value === null
          const off = auto || skipped.has(r.key)
          const existing = existingOf(r.key)
          /*
           * 一样 ⇒ 和报告值同为白色；不一样 ⇒ 预设那格标绿。
           * 比数值不比字符串：预设里写 `20.0`、报告里是 `20`，那是同一个值。
           */
          const a = r.value === null ? Number.NaN : Number(r.value)
          const b = Number(existing)
          const diff =
            !auto &&
            existing !== '-' &&
            (Number.isFinite(a) && Number.isFinite(b) ? a !== b : existing !== r.value)
          const cls = auto
            ? `${s.valueRow} ${s.valueRowAuto}`
            : off
              ? `${s.valueRow} ${s.valueRowOff}`
              : s.valueRow
          return (
            <div key={r.key} className={cls}>
              <span className={s.valueLabelWrap}>
                <span className={s.valueLabel}>{r.label}</span>
                {!auto && (
                  <button
                    type="button"
                    className={s.skipBtn}
                    data-on={off}
                      aria-label={tr('report.debug.writeSkip')}
                    onClick={() =>
                      setSkipped((prev) => {
                        const next = new Set(prev)
                        if (next.has(r.key)) next.delete(r.key)
                        else next.add(r.key)
                        return next
                      })
                    }
                  >
                    {/* 画出来的 SVG，不是字符：字符基线怎么调都会一个偏高一个偏低 */}
                  {off ? (
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                      <path
                        d="M2.8 4.6h4.6a2.3 2.3 0 1 1 0 4.6H5.4"
                        stroke="currentColor"
                        strokeWidth="1.4"
                        strokeLinecap="round"
                      />
                      <path
                        d="M5 2.5 2.8 4.6 5 6.7"
                        stroke="currentColor"
                        strokeWidth="1.4"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  ) : (
                    <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
                      <path
                        d="M3 3l6 6M9 3l-6 6"
                        stroke="currentColor"
                        strokeWidth="1.4"
                        strokeLinecap="round"
                      />
                    </svg>
                  )}
                  </button>
                )}
              </span>
              {auto ? (
                <span className={s.valueSkip}>
                  {r.skip === 'follow' ? tr('report.debug.follow') : tr('report.debug.noRecord')}
                  {' · '}
                  {tr('report.debug.writeNoWrite')}
                </span>
              ) : (
                <span className={s.valuePair}>
                  <span className={s.valueNum}>{r.value}</span>
                  <span className={s.arrow}>→</span>
                  {/* 用属性而不是叠第二个类：`.diffNum` 定义在 `.valueNum` 前面时，
                      同优先级会被后者覆盖，绿色就变回白色了 */}
                  <span className={s.valueNum} data-diff={diff ? 'true' : 'false'}>
                    {existing}
                  </span>
                </span>
              )}
            </div>
          )
        })}
      </div>

      {err !== null && <p className={s.err}>{err}</p>}

      {files === null ? (
        <p className={s.note}>{tr('common.loading')}</p>
      ) : files.length === 0 ? (
        <p className={s.note}>{tr('report.debug.writeEmpty')}</p>
      ) : (
        /*
         * **同一个列表**：其余那几行只是"不渲染 / 渲染"，不是另一个列表、也不是另开一节。
         * 盒子上限用 `max-height`（不是固定 height）—— 折叠时几行就几行高，下面不空；
         * 展开到超过上限才滚，窗不会被撑满屏。
         */
        <div className={s.presetArea}>
          <p className={s.groupHead}>
            {family === '' ? tr('report.debug.writeAll') : tr('report.debug.writeMine')}
          </p>
          <div className={s.list}>
            <PresetRows files={mine} picked={picked} onPick={setPicked} />
            {others.length > 0 && (
              <>
                <button
                  type="button"
                  className={s.foldBtn}
                  onClick={() => setShowOthers((v) => !v)}
                >
                  {showOthers
                    ? tr('report.debug.writeFoldHide')
                    : tr('report.debug.writeFoldShow', { count: String(others.length) })}
                </button>
                {showOthers && (
                  <PresetRows files={others} picked={picked} onPick={setPicked} />
                )}
              </>
            )}
          </div>
        </div>
      )}
    </Modal>
  )
}

/** 一批预设行（文件名 + 机型档位），点一行选中 */
function PresetRows({
  files,
  picked,
  onPick,
}: {
  files: LocalPresetFile[]
  picked: string | null
  onPick: (path: string) => void
}) {
  if (files.length === 0) return null
  return (
    <>
      {files.map((f) => (
        <button
          key={f.path}
          type="button"
          className={s.row}
          data-on={picked === f.path}
          onClick={() => onPick(f.path)}
        >
          <span className={s.rowName}>{f.fileName.replace(/\.toml$/i, '')}</span>
          <span className={s.rowMeta}>{versionOf(f.fileName)}</span>
        </button>
      ))}
    </>
  )
}

/** 一行要写的值；`value === null` = 这一项不写（原因见 `skip`） */
type WriteRow = {
  key: string
  label: string
  value: string | null
  /** 为什么不写：`follow` = 调试时跟随驻点位；`unrecorded` = 旧记录没有这一项 */
  skip?: 'follow' | 'unrecorded'
}

/** 等待位那一行：数字照写；`null` = 跟随；缺键 = 旧记录没记 */
function waitRow(key: string, label: string, v: number | null | undefined): WriteRow {
  if (typeof v === 'number') return { key, label, value: String(v) }
  return { key, label, value: null, skip: v === null ? 'follow' : 'unrecorded' }
}

/**
 * 这几项各自写进哪个 key（顺序即界面上的顺序）。
 *
 * ★ **跟随的不写**：调试时勾着「跟随驻点位」，那两个数只是驻点位带出来的，
 *   不是用户测过的位置 —— 写进预设就会把 P 系列收笔等待位 246 变成 20。
 *   驻点位本身没有对应的预设字段，一律不写。
 */
function sixRows(facts: ReportDebugFacts, tr: Tr): WriteRow[] {
  const num = (v: number) => String(v)
  return [
    waitRow('toolhead.pen_mount_wait_x', tr('report.debug.writeWait'), facts.mountWaitX),
    { key: 'toolhead.pen_mount_x', label: tr('report.debug.writeMount'), value: num(facts.mountX) },
    waitRow('toolhead.pen_unmount_wait_x', tr('report.debug.writeUnwait'), facts.unmountWaitX),
    { key: 'toolhead.pen_unmount_x', label: tr('report.debug.writeUnmount'), value: num(facts.unmountX) },
    { key: 'toolhead.pen_mount_speed', label: tr('report.debug.writeSpeed'), value: num(facts.speed) },
    { key: 'toolhead.pen_unmount_speed', label: tr('report.debug.writeUnspeed'), value: num(facts.speed) },
  ]
}

/**
 * 机型家族：把 `Bambu Lab A1 mini 0.4 nozzle` 与 `A1_MINI-fast.toml` 归到同一个词。
 *
 * `A1_MINI` 要在 `A1` 之前判 —— 否则 mini 的会被并到 A1 那一组里去。
 */
function familyOf(text: string): string {
  const t = text.toUpperCase().replace(/[\s-]+/g, '_')
  for (const id of ['A1_MINI', 'A1MINI', 'P2S', 'P1S', 'X1C', 'H2D', 'A1']) {
    if (t.includes(id)) return id === 'A1MINI' ? 'A1_MINI' : id
  }
  return ''
}

/** 文件名里的档位（`A1-fastv3.3` → `FASTV3.3`），认不出就空 */
function versionOf(fileName: string): string {
  const base = fileName.replace(/\.toml$/i, '')
  const i = base.indexOf('-')
  return i < 0 ? '' : base.slice(i + 1).toUpperCase()
}
