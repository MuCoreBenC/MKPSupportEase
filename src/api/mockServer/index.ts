/**
 * 假后端（浏览器预览用）。
 *
 * 这是**客户端**这一侧的假数据层：`src/api/mock.ts` 直接调这里的解析器，
 * 把 `data/*.json` 那几份上游快照合成界面上要的形状。真机上换成 Tauri 壳注入的
 * `src/api/bridge.ts` 之后，这一整个目录都不再被引用。
 *
 * 分工和上游对齐：
 *
 *   `machines.ts`   机型目录（品牌 → 机型 → 版本 + 尺寸 / 禁区）
 *   `files.ts`      「这个机型这个版本要哪些文件」—— 正向一跳
 *   `params.ts`     参数注册表 + 布局表 → 界面直接能渲染的取值与元信息
 *   `resources.ts`  预设仓库清单 —— 反向倒查（这个文件被哪些版本 / 套餐用着）
 *   `bbsFiles.ts`   切片器 profile 的大小与修改时间**真值**（其余是演示推值）
 *   `localFiles.ts` 本机文件的固定演示集合（假后端没有文件系统）
 *   `menu.ts`       菜单表：每个文件对客户端公开到什么程度
 *   `types.ts`      `data/*.json` 的逐字形状（`Raw*`，不出这个目录）
 *
 * 两条铁律：
 *
 * 1. **`data/*.json` 是上游快照，不是可编辑的源。** 这里只有读，没有回写；
 *    写方法（`copyToSlicer`）也只改内存，刷新页面即还原。
 * 2. **不编假进度。** 查不到的事实宁可留空让界面显示「未知」，
 *    也不造一个看起来煞有介事的数字（`demoStat` 那种演示值除外，它标着自己是演示值）。
 */

export { allMachines } from './machines'
export { resolveVersionFiles } from './files'
export { catalogRegistry, paramMeta, resolveParams } from './params'
export { allPresetFiles } from './resources'
export { menuEntries } from './menu'
export {
  appliedPreset,
  copyToSlicerIn,
  localFileIds,
  localUserFiles,
  slicerCopied,
} from './localFiles'
