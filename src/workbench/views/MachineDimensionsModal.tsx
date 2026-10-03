/*
 * 机型尺寸编辑模态框（2026-10-03，作者照旧面板 mkppanel 移植）。
 *
 * # 六组，一组不少
 *
 * 画布尺寸 bedSize / 移动范围 movementRange / 边缘范围 edgeZone / 涂胶区域 glueArea /
 * 标定点 calibration / 标志位 flags —— 与 `MachineView.dimensions` 一一对应，
 * 标签带原始键名（`宽度 (width)`、`Z 起点 Y (zStartY)`），让人对着源文件能看上号。
 *
 * **为什么六组恒在、不能"删掉一组"**：后端 `load_dimensions` 是「全有或全无」——
 * `[dimensions]` 在，六个子表就都得在，缺一个是 `Corrupted`（写坏了的现场）。
 * 所以界面上没有"这一组不要了"这种操作；某组填成全零就是"这组是空的"（写一组 0）。
 *
 * # 「从相似机型复制标定点」
 *
 * 标定点那十格是抄真机实测值的，谁都记不住 —— 旧面板给的解法是
 * 「从相似机型复制标定点」：选一台已经配好的机器，整组 calibration 拷过来。
 * 这一条是**纯前端**的（复用清单里已有的值，不动后端）。
 *
 * # 与外壳的关系
 *
 * 即时落盘（`wb.setMachineDimensions`），不进参数草稿栈 —— 与品牌 / 机型那套一致：
 * 一次提交 = 一次写盘 = 回一份新清单。所以这里是「取消 / 确认保存」两颗按钮，
 * 不是"改了立刻写"（六组一起改，中间态不该漂在文件上）。
 */

import { useEffect, useMemo, useState } from 'react'

import { isAppError, wb } from '../api'
import type { MachineDimensions, MachineView } from '../api'
import { NumberField } from '../components/field'
import ModalC14 from '../c14/ModalC14'
import { toasts } from '../c14/toast'
import s from './MachineDimensionsModal.module.css'

interface Props {
  /** 正在编辑哪台（`null` = 关着） */
  machineId: string | null
  /** 全部机型（「从相似机型复制标定点」要从这儿挑一台） */
  machines: MachineView[]
  /** 这台现在的尺寸（`null` = 还没配过，给一份空模板起手） */
  dimensions: MachineDimensions | null
  onClose: () => void
  /** 写盘成功后把新清单交回页面 */
  onSaved: (next: Awaited<ReturnType<typeof wb.setMachineDimensions>>) => void
}

/** 全零的一份（新建尺寸时的起手式）：六组都在，值都是 0 */
const EMPTY: MachineDimensions = {
  bedSize: { width: 0, depth: 0 },
  movementRange: { minX: 0, maxX: 0, minY: 0, maxY: 0, maxZ: 0 },
  edgeZone: 0,
  glueArea: { glueMinX: 0, glueMaxX: 0, glueMinY: 0, glueMaxY: 0, wipeX: 0 },
  calibration: {
    lShapeBaseX: 0,
    lShapeBaseY: 0,
    xLineX: 0,
    xLineY: 0,
    xLineYEnd: 0,
    yLineX: 0,
    yLineXEnd: 0,
    yLineY: 0,
    zStartX: 0,
    zStartY: 0,
  },
  flags: { gcodeMarker: '', hasSecondFan: false },
}

/** 深拷贝（草稿与外面那份互不影响 —— 取消要能干干净净地丢掉） */
const clone = (d: MachineDimensions): MachineDimensions => ({
  bedSize: { ...d.bedSize },
  movementRange: { ...d.movementRange },
  edgeZone: d.edgeZone,
  glueArea: { ...d.glueArea },
  calibration: { ...d.calibration },
  flags: { ...d.flags },
})

/**
 * 一格：标签（带原始键名）+ 步进框（参数台同款 NumberField）。
 *
 * `decimals={3}`：标定点那几个实测值有三位小数（126.373），键入不许被四舍五入吃掉；
 * 箭头一次走 0.1mm。范围不设 —— 移动范围 / 涂胶下限本来就是负数，夹了反而碍事。
 */
function Field({
  label,
  keyName,
  value,
  onEdit,
}: {
  label: string
  keyName: string
  value: number
  onEdit: (v: number) => void
}) {
  return (
    <label className={s.field}>
      <span className={s.fieldLabel}>
        {label}
        <em className={s.fieldKey}>{keyName}</em>
      </span>
      <NumberField
        value={value}
        label={`${label} (${keyName})`}
        unit="mm"
        step={0.1}
        decimals={3}
        onChange={onEdit}
      />
    </label>
  )
}

function Group({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className={s.group}>
      <div className={s.groupHead}>{title}</div>
      <div className={s.grid}>{children}</div>
    </div>
  )
}

export default function MachineDimensionsModal({
  machineId,
  machines,
  dimensions,
  onClose,
  onSaved,
}: Props) {
  const [draft, setDraft] = useState<MachineDimensions>(() => clone(dimensions ?? EMPTY))
  const [busy, setBusy] = useState(false)

  /* 每次打开（或换了一台）都从"当前那份"重新起草 —— 上次取消掉的不该还留着 */
  useEffect(() => {
    setDraft(clone(dimensions ?? EMPTY))
    setBusy(false)
  }, [machineId, dimensions])

  const cur = machines.find((x) => x.id === machineId)
  /** 能当"相似机型"的：别台机器、而且**有尺寸**（没尺寸的抄不出标定点） */
  const donors = useMemo(
    () => machines.filter((x) => x.id !== machineId && x.dimensions !== null),
    [machines, machineId],
  )

  const edit = (fn: (d: MachineDimensions) => void) =>
    setDraft((prev) => {
      const next = clone(prev)
      fn(next)
      return next
    })

  const save = async () => {
    if (machineId === null) return
    setBusy(true)
    try {
      onSaved(await wb.setMachineDimensions(machineId, draft))
      toasts.push(`已写回 ${cur?.file ?? machineId} 的 [dimensions]`)
      onClose()
    } catch (e) {
      toasts.push(isAppError(e) ? e.message : String(e))
      setBusy(false)
    }
  }

  const bedOk = draft.bedSize.width > 0 && draft.bedSize.depth > 0

  return (
    <ModalC14
      open={machineId !== null}
      title={`机型尺寸 · ${cur?.display || machineId || ''}`}
      subtitle={`写回 presets/machines/${cur?.file ?? ''} 的 [dimensions] —— 即时落盘，没有草稿`}
      size="lg"
      closeOnScrim={false}
      onClose={onClose}
      footer={
        <>
          {!bedOk && <span className={s.warn}>床身宽与深都要大于 0</span>}
          <span className={s.grow} />
          <button type="button" className={s.btn} onClick={onClose}>
            取消
          </button>
          <button
            type="button"
            className={`${s.btn} ${s.btnPrimary}`}
            disabled={busy || !bedOk}
            title={bedOk ? undefined : '床身宽与深都必须大于 0 —— 一台没有可打印面积的机器画不出床身图'}
            onClick={() => void save()}
          >
            {busy ? '保存中…' : '确认保存'}
          </button>
        </>
      }
    >
      <Group title="画布尺寸 (bedSize)">
        <Field
          label="宽度"
          keyName="width"
          value={draft.bedSize.width}
          onEdit={(v) => edit((d) => (d.bedSize.width = v))}
        />
        <Field
          label="深度"
          keyName="depth"
          value={draft.bedSize.depth}
          onEdit={(v) => edit((d) => (d.bedSize.depth = v))}
        />
      </Group>

      <Group title="移动范围 (movementRange)">
        <Field
          label="X 最小值"
          keyName="minX"
          value={draft.movementRange.minX}
          onEdit={(v) => edit((d) => (d.movementRange.minX = v))}
        />
        <Field
          label="X 最大值"
          keyName="maxX"
          value={draft.movementRange.maxX}
          onEdit={(v) => edit((d) => (d.movementRange.maxX = v))}
        />
        <Field
          label="Y 最小值"
          keyName="minY"
          value={draft.movementRange.minY}
          onEdit={(v) => edit((d) => (d.movementRange.minY = v))}
        />
        <Field
          label="Y 最大值"
          keyName="maxY"
          value={draft.movementRange.maxY}
          onEdit={(v) => edit((d) => (d.movementRange.maxY = v))}
        />
        <Field
          label="Z 最大值"
          keyName="maxZ"
          value={draft.movementRange.maxZ}
          onEdit={(v) => edit((d) => (d.movementRange.maxZ = v))}
        />
      </Group>

      <Group title="边缘范围 (edgeZone)">
        <Field
          label="边缘范围"
          keyName="edgeZone"
          value={draft.edgeZone}
          onEdit={(v) => edit((d) => (d.edgeZone = v))}
        />
      </Group>

      <Group title="涂胶区域 (glueArea)">
        <Field
          label="涂胶 X 下限"
          keyName="glueMinX"
          value={draft.glueArea.glueMinX}
          onEdit={(v) => edit((d) => (d.glueArea.glueMinX = v))}
        />
        <Field
          label="涂胶 X 上限"
          keyName="glueMaxX"
          value={draft.glueArea.glueMaxX}
          onEdit={(v) => edit((d) => (d.glueArea.glueMaxX = v))}
        />
        <Field
          label="涂胶 Y 下限"
          keyName="glueMinY"
          value={draft.glueArea.glueMinY}
          onEdit={(v) => edit((d) => (d.glueArea.glueMinY = v))}
        />
        <Field
          label="涂胶 Y 上限"
          keyName="glueMaxY"
          value={draft.glueArea.glueMaxY}
          onEdit={(v) => edit((d) => (d.glueArea.glueMaxY = v))}
        />
        <Field
          label="擦料 X 坐标"
          keyName="wipeX"
          value={draft.glueArea.wipeX}
          onEdit={(v) => edit((d) => (d.glueArea.wipeX = v))}
        />
      </Group>

      <div className={s.group}>
        <div className={s.groupHead}>
          标定点 (calibration)
          {/* 十格实测值谁都记不住 —— 给一条"从别台机器整组拷"的路（旧面板同一颗） */}
          <select
            className={s.copyFrom}
            value=""
            aria-label="从相似机型复制标定点"
            title="选一台已经配好的机器，整组标定点拷过来"
            onChange={(e) => {
              const from = donors.find((x) => x.id === e.target.value)
              if (!from?.dimensions) return
              edit((d) => (d.calibration = { ...from.dimensions!.calibration }))
              toasts.push(`标定点已从 ${from.id} 拷过来（还没落盘 —— 记得点保存）`)
            }}
          >
            <option value="">从相似机型复制标定点…</option>
            {donors.map((x) => (
              <option key={x.id} value={x.id}>
                {x.display || x.id}
              </option>
            ))}
          </select>
        </div>
        <div className={s.grid}>
          <Field
            label="Y 线 X 起点"
            keyName="yLineX"
            value={draft.calibration.yLineX}
            onEdit={(v) => edit((d) => (d.calibration.yLineX = v))}
          />
          <Field
            label="Y 线 X 终点"
            keyName="yLineXEnd"
            value={draft.calibration.yLineXEnd}
            onEdit={(v) => edit((d) => (d.calibration.yLineXEnd = v))}
          />
          <Field
            label="Y 线 Y 坐标"
            keyName="yLineY"
            value={draft.calibration.yLineY}
            onEdit={(v) => edit((d) => (d.calibration.yLineY = v))}
          />
          <Field
            label="X 线 Y 起点"
            keyName="xLineY"
            value={draft.calibration.xLineY}
            onEdit={(v) => edit((d) => (d.calibration.xLineY = v))}
          />
          <Field
            label="X 线 Y 终点"
            keyName="xLineYEnd"
            value={draft.calibration.xLineYEnd}
            onEdit={(v) => edit((d) => (d.calibration.xLineYEnd = v))}
          />
          <Field
            label="X 线 X 坐标"
            keyName="xLineX"
            value={draft.calibration.xLineX}
            onEdit={(v) => edit((d) => (d.calibration.xLineX = v))}
          />
          <Field
            label="Z 起点 X"
            keyName="zStartX"
            value={draft.calibration.zStartX}
            onEdit={(v) => edit((d) => (d.calibration.zStartX = v))}
          />
          <Field
            label="Z 起点 Y"
            keyName="zStartY"
            value={draft.calibration.zStartY}
            onEdit={(v) => edit((d) => (d.calibration.zStartY = v))}
          />
          <Field
            label="L 形基点 X"
            keyName="lShapeBaseX"
            value={draft.calibration.lShapeBaseX}
            onEdit={(v) => edit((d) => (d.calibration.lShapeBaseX = v))}
          />
          <Field
            label="L 形基点 Y"
            keyName="lShapeBaseY"
            value={draft.calibration.lShapeBaseY}
            onEdit={(v) => edit((d) => (d.calibration.lShapeBaseY = v))}
          />
        </div>
      </div>

      <Group title="标志位 (flags)">
        <label className={s.field}>
          <span className={s.fieldLabel}>
            G-code 标记
            <em className={s.fieldKey}>gcodeMarker</em>
          </span>
          <input
            className={s.num}
            value={draft.flags.gcodeMarker}
            aria-label="G-code 标记 (gcodeMarker)"
            onChange={(e) => edit((d) => (d.flags.gcodeMarker = e.target.value))}
          />
        </label>
        <label className={s.field}>
          <span className={s.fieldLabel}>
            有第二风扇
            <em className={s.fieldKey}>hasSecondFan</em>
          </span>
          <input
            className={s.check}
            type="checkbox"
            checked={draft.flags.hasSecondFan}
            aria-label="有第二风扇 (hasSecondFan)"
            onChange={(e) => edit((d) => (d.flags.hasSecondFan = e.target.checked))}
          />
        </label>
      </Group>

      <p className={s.hint}>
        六组一个不少（源文件里 `[dimensions]` 是全有或全无）—— 某组填成全零就是「这组是空的」，
        不会把那一节从文件里删掉。写回时保住文件里的注释与其余键序。
      </p>
    </ModalC14>
  )
}
