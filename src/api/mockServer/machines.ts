import type { ForbiddenZone, Machine, MachineDimensions, MachineVersion } from '../../api/contract'
import catalogJson from './data/machine_catalog.json'
import { plateRefsOf } from './plates'
import type { RawCatalog, RawDimensions, RawModel, RawVersion } from './types'

/**
 * 机型目录：把上游的四段（brands / models / dimensions / forbiddenZones）合成一张表。
 *
 * 做了三件上游没做的事：
 *
 * 1. **bundle 归一成一个字段**。上游有两个来源（版本级 recommendedBundle、机型级 defaultBundle），
 *    两者都可以为空，而且谁优先没有写在任何地方。这里定死「版本级优先，机型级兜底」并只暴露一个 bundle。
 * 2. **「没配」是显式状态**。A2L 的 dimensions 不存在、presetFile 和 bundle 都是空串。
 *    上游查不到就静默当没有；这里把 dimensions 做成 `null`，让界面必须处理这一支。
 * 3. **原始版本条目单独留一份**。`presetFile` 刻意不进对外的 `MachineVersion` ——
 *    「MKP 预设是哪个文件」属于文件解析的中间结果，前端只该看到解析完的清单。
 */

const catalog = catalogJson as RawCatalog

/** 品牌 id → 显示名。上游 models 是按品牌 id 分组的，显示名在 brands 里 */
const brandName = new Map(catalog.brands.map((b) => [b.id, b.name]))

function toVersion(model: RawModel, raw: RawVersion): MachineVersion {
  return {
    id: raw.id,
    name: raw.name,
    // 上游把「没有角标」写成空串，对外统一成不给这个字段
    tag: raw.tag || undefined,
    description: raw.description || undefined,
    bundle: raw.recommendedBundle || model.defaultBundle || '',
  }
}

function toDimensions(raw: RawDimensions | undefined): MachineDimensions | null {
  if (!raw) return null
  return {
    bedSize: raw.bedSize,
    movementRange: raw.movementRange,
    glueArea: raw.glueArea,
    calibration: raw.calibration,
    edgeZone: raw.edgeZone,
    flags: raw.flags,
  }
}

function toForbiddenZones(machineId: string): ForbiddenZone[] {
  // 没配禁区就是空数组 —— 禁区是按需的，空数组和「没配」在这里是同一个意思，不需要区分
  return catalog.forbiddenZones[machineId] ?? []
}

function build(): Machine[] {
  const out: Machine[] = []
  for (const [brandId, models] of Object.entries(catalog.models)) {
    for (const model of models) {
      out.push({
        id: model.id,
        // 上游六台机型的 name 全是空串，所以 display 才是唯一可用的显示名
        display: model.display || model.id,
        brand: brandName.get(brandId) ?? brandId,
        // 资产 id（不是文件名）：界面按 id 去 `RuntimeCatalog.assets[]` 查 path
        image: model.image,
        imageVariant: model.imageVariant ?? '',
        icon: model.icon,
        aliases: model.externalAliases,
        // 板引用与真数据同源（演示常量，见 plates.ts）
        ...plateRefsOf(model.id),
        versions: model.versions.map((v) => toVersion(model, v)),
        dimensions: toDimensions(catalog.dimensions[model.id]),
        forbiddenZones: toForbiddenZones(model.id),
      })
    }
  }
  return out
}

const machines: Machine[] = build()

const byId = new Map(machines.map((m) => [m.id, m]))

export function allMachines(): Machine[] {
  return machines
}

/** 这个机型有没有这个版本。`files.ts` 用它把「组合不存在」与「存在但没配齐」分开 */
export function findVersion(machineId: string, versionId: string): MachineVersion | undefined {
  return byId.get(machineId)?.versions.find((v) => v.id === versionId)
}

/**
 * 原始版本条目。`files.ts` 需要 `presetFile`，而那个字段刻意不进对外的 MachineVersion ——
 * 「MKP 预设是哪个文件」属于文件解析的中间结果，前端只该看到解析完的清单。
 */
const rawVersionIndex = new Map<string, RawVersion>()
for (const models of Object.values(catalog.models)) {
  for (const model of models) {
    for (const v of model.versions) {
      rawVersionIndex.set(`${model.id}:${v.id}`, v)
    }
  }
}

export function findRawVersion(machineId: string, versionId: string): RawVersion | undefined {
  return rawVersionIndex.get(`${machineId}:${versionId}`)
}
