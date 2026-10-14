/*
 * 「本机真有的预设」→ 预设抽屉的行（首页 / 参数页 / 校准页共用）。
 *
 * # 为什么要有这一层（2026-10-14 作者裁定）
 *
 * 抽屉原来列的是**目录（云端）**那一套 —— `catalog.machines` × `catalog.files[]`。
 * 于是本机只有一份 `A1_MINI-fastv3.3.toml`，抽屉里却把 12 个组合全列出来，
 * 点哪个都换不过去。作者定的口径是三条：
 *
 *   ① 点行什么都不做；② 点「使用」才切；③ **只显示本机真有的**。
 *
 * 这一层只做第 ③ 条的翻译：把 `getLocalPresetFiles()` 的扫描结果摊成抽屉行。
 * 三页拿到的是同一份数据、同一套排序、同一套组合键（`机型/版本`），
 * 于是"三处一致"不靠约定，靠**只有一个实现**。
 *
 * # 归属怎么认（与参数页 `editTargetByCombo` 同一套，别另立一套）
 *
 * - 用户那份的机型 / 版本从**血统**认（`basedOnMachineId` / `basedOnVersionId`）；
 *   认不出的（手工拷的 / 导入的 / 自己写的）**不进抽屉** —— 它不对应任何组合，
 *   硬塞一格会让"选中哪一版"这件事失去依据。它们在**预设页**照常列着。
 * - 只有 `kind === 'mkp_preset'` 的能进（`.json` 之类不是校准 / 参数的对象）。
 *
 * ⚠️ 云端那条线（下载到 `delivery/mkp/presets/` 的那几份）**暂不进抽屉**：
 * 抽屉的 `onUse` 要复用预设页那颗「使用」的语义，而云端那一份的动作是
 * 「按需取回 + 落一份本地 + 应用」（用户世界只认 `presets-local/` 那一份）。
 * 等这条口径定下来再补，不在这里猜。
 */

import { useEffect, useMemo, useState } from 'react'
import { api } from '../../api'
import type { LocalPresetFile, Machine } from '../../api/contract'
import type { PresetPickerRow } from '../params/PresetPickerDrawer'

/** 本机实有预设 → 抽屉行（按机型显示名 → 版本显示名排好；同一个组合本地那份优先） */
export function localPickerRows(files: LocalPresetFile[], machines: Machine[]): PresetPickerRow[] {
  /* 机型显示名：先查目录，查不到就照实写 id（认不出的归属不该被藏起来） */
  const machineName = new Map(machines.map((m) => [m.id, m.display]))
  const displayOf = (id: string) => machineName.get(id) ?? id
  /* 版本显示名：目录里那台机器那一版的名字（查不到退成 id —— 与机型同一条口径） */
  const versionName = new Map<string, string>()
  for (const m of machines) {
    for (const v of m.versions) versionName.set(`${m.id}/${v.id}`, v.name)
  }
  const rows: PresetPickerRow[] = []
  for (const f of files) {
    if (f.kind !== 'mkp_preset') continue

    /*
     * 归属：**血统优先**（它就是"从哪一版来的"），其次是文件头那两行 `# machine/# variant`
     * （`machineId` / `versionId` —— 复制出来的那份会带上它）。都没有就不进抽屉。
     */
    const machineId = f.basedOnMachineId ?? f.machineId
    const versionId = f.basedOnVersionId ?? f.versionId
    if (machineId === null || machineId === undefined) continue
    if (versionId === null || versionId === undefined) continue

    rows.push({
      machineId,
      machineDisplay: displayOf(machineId),
      versionId,
      versionName: versionName.get(`${machineId}/${versionId}`) ?? versionId,
      fileName: f.fileName,
      /* 用户线的动作身份是**路径**（它与预设页那颗「使用」同形） */
      ref: f.path,
      isLocal: true,
    })
  }

  /* 排序：机型显示名 → 版本显示名（同一个组合再有第二份就按文件名）——
     保证每次打开抽屉顺序都一样（`read_dir` 的顺序是文件系统说的） */
  rows.sort(
    (a, b) =>
      a.machineDisplay.localeCompare(b.machineDisplay) ||
      a.versionName.localeCompare(b.versionName) ||
      a.fileName.localeCompare(b.fileName),
  )
  return rows
}

/** 上面那份的 React 版（三页共用；`presets` 只用来补版本显示名，没有也给得出行） */
export function useLocalPickerRows(files: LocalPresetFile[], machines: Machine[]): PresetPickerRow[] {
  return useMemo(() => localPickerRows(files, machines), [files, machines])
}

/**
 * 三页共用的抽屉数据：**拉一次本机扫描 + 摊成行**。
 *
 * 为什么要有它：抽屉的"只显示本机真有的"这条判据只该有**一个实现**，
 * 而三页各自拉一次 `getLocalPresetFiles()` 会让"多久刷新"这件事分三处（总有一处忘）。
 * 这里挂载时拉一次、卸载留着结果（跨页回来先吃上一份再重拉，与 `useCatalog` 同一条口径）。
 *
 * 注意：这是**抽屉打开时看的那一份**，不是"使用中"的底账 —— 底账在
 * `../state/appState.ts`（唯一），两件事别混。
 */
export function useLocalPresetFiles(): LocalPresetFile[] {
  const [files, setFiles] = useState<LocalPresetFile[]>([])
  useEffect(() => {
    let alive = true
    void api.getLocalPresetFiles().then(
      (list) => {
        if (alive) setFiles(list)
      },
      () => {
        /* 拉不到就空着（抽屉说"本机还没有"）—— 不编，也不挡页面 */
      },
    )
    return () => {
      alive = false
    }
  }, [])
  return files
}
