import type { RawBbsFile, RawBbsFilesFile } from './types'
import bbsFilesJson from './data/bbs_files.json'

/**
 * bbs 预设文件的事实。
 *
 * # 这个模块答什么
 *
 * **这个 asset 在真仓里那个文件有多大、什么时候更新的。** 快照是 `data/bbs_files.json`：
 * 从上游 `mkpse-presets` 仓同步真文件时，顺手把磁盘上的重量（size / sha256）与上游
 * `manifest.json` 记的那份核了一遍，两边对不上就不同步 —— 所以这里的数是**真值**，不是推的。
 *
 * # 为什么单独一层
 *
 * `resources.ts` 对查不到事实的文件调 `demoStat()` 按 path 推大小与时间（那是演示值，
 * 上游 `assets_index.json` 里 size 全是 0、sha256 全是空串）。bbs 这一档有真值，
 * 就得让「真值」和「演示值」在同一张表里**分得开**：`PresetFileInfo.statFrom` 就是那个记号，
 * 界面靠它决定 tooltip 说哪句话。
 *
 * **范围只到 `presets/bbs`**：查不到的资产照旧走 `demoStat()`，一个字都不改。
 */

const snapshot = bbsFilesJson as RawBbsFilesFile

const byAssetId = new Map(snapshot.files.map((f) => [f.assetId, f]))

/** 那份真文件的字节事实。`undefined` = 这个 asset 不在真仓的 `presets/bbs` 里 */
export function bbsFileOf(assetId: string): RawBbsFile | undefined {
  return byAssetId.get(assetId)
}

/**
 * 字节数 → 列里显示的那个串。
 *
 * 尺度跟发布行那一套对齐：不到 1 KB 就写 `812 B`，过了就写 `1.3 KB`。
 * 两边不一样的话，同一列会混着两种量法。
 */
export function sizeTextOf(bytes: number): string {
  return bytes >= 1024 ? `${(bytes / 1024).toFixed(1)} KB` : `${bytes} B`
}
