# 副标题 = 备注 + 复制件的归属（2026-10-07）

> 分支：`feat/app-state`。主题：客户端预设列表撤「来源」列、版本列加宽、
> 副标题改显示备注（可覆盖）、复制出来的预设显示并**可编辑**机型 / 版本、
> 工作台版本加「备注」字段。台账与代码同一 commit。

## 作者裁决（2026-10-07，截图两轮）

1. **来源列撤掉**（「来源不用显示在右侧」）—— 来源的完整说法（含「复制自 X → 定位」）在展开详情里。
2. **版本列宽一点**：`.thVersion` 104 → 136px（compact 84、mini 48 不动）。
3. **复制出来的也要显示机型 / 版本**，而且**可以编辑**改机型和版本。
4. **工作台加「备注」**（可空）。
5. **客户端副标题不要位置文案，改显示备注**。
6. **覆盖规则**：默认下载的用工作台写的备注；用户改了之后，**以后更新不覆盖**；
   除非他删了重新下载。

## 设计定案

- **备注是版本的属性**：`MachineVersion.remark: Option<String>`（`presetdata/catalog.rs`，
  `skip_serializing_if = "Option::is_none"` ⇒ catalog 形状逐字节不变，`embedded_matches_rebuild` 不用重出）。
  链路：工作台机型文件 → `VersionField::Remark`（TOML 键 `remark`）→ `wb_publish` / `gen-catalog`
  → 客户端 `getMachines` 的 `VersionDto.remark`。
- **用户改过的备注记在客户端的覆盖账**：`<userRoot>/preset-remarks.json`（`runtime/remarks.rs`，
  与出处账同族：键 = 文件身份，值 = 覆盖句；空串 / `null` = 删键）。
  键：官方交付行 = **`catalog.path`**、用户线 = 相对用户根路径（两个键空间天然不重叠）。
  - 「更新不覆盖」= 覆盖账是独立的文件，下载管道根本不碰它；
  - 「删了重新下载就回到工作台那句」= `delete_delivery_file` / `delete_user_preset` 时跟着删键。
  - 覆盖账丢了什么都不坏（退回工作台那句）。
- **副标题 = 备注，没备注就空着**（三轮定案后，见文末）：**本地表** 覆盖账 → 那一版的
  `remark` → 空着；**云端表** 只读那一版的 `remark` → 空着（不读覆盖账）。
  路径不再上副标题（永远在 `title` 里）。交付行盘上不对劲（old / tampered）时
  **状态注记压过备注**（副标题仍是 `releasePathText`）—— 状态必须写在原地。
- **归属是文件自己的属性**（随文件走，不记程序账）：复制件文件头本来就有
  `# machine:` / `# variant:` 两行（B 类产物按字节拷贝带过来的），此前只是没人读。
  - `MineFile` 增 `machine` / `variant`（`runtime/lineage.rs::parse_machine_from_content`
    / `parse_variant_from_content`；与血统三行读**同一段头注释**，一次 IO）；
  - DTO 增 `machineId` / `versionId`：对着目录登记**大小写无关**归一化（B 类写的是版本 id
    小写），认不出回落 `basedOnMachineId` / `basedOnVersionId`；
  - **改归属** = `set_user_preset_machine_version`：`lineage::rewrite_machine_variant`
    原位换那两行（缺哪行补哪行、行尾跟文件自己、正文一个字节不动），机型 / 版本必须
    目录里真有（对 catalog files 校验）。**写血统的那族文本手术**，测试对齐 `build9` 纪律之外新加 2 条。
- 列表展示：mine 行的机型 / 版本两列改读**自己的归属**；`untagged`（未标机型角标）改为
  「归属都没有才挂」。展开详情给两个编辑格：**备注**（输入 + 保存 + 清除覆盖）、
  **归属**（机型 / 版本两个下拉 + 保存）。

## 落点清单

| 层 | 文件 | 内容 |
|---|---|---|
| Rust 数据 | `presetdata/catalog.rs` | `MachineVersion.remark` + `VersionField::Remark`（TOML 键 `remark`） |
| Rust 工作台 | `workbench/app/machines.rs` | `VersionView.remark` |
| Rust 客户端 | `ipc/presets.rs` | `VersionDto.remark` |
| Rust 血统 | `runtime/lineage.rs` | `parse_machine/variant_from_content` + `rewrite_machine_variant` |
| Rust 用户线 | `runtime/mine.rs` | `MineFile.machine/variant`、`head_text`、`set_machine_variant` |
| Rust 覆盖账 | `runtime/remarks.rs`（新） | load / get / set / remove + 4 条测试 |
| Rust IPC | `ipc/mine.rs`、`ipc/catalog.rs`、`lib.rs` | 3 条新命令（get/set_preset_remark、set_user_preset_machine_version）+ 删除时清备注 |
| 客户端契约 | `src/api/contract.ts`、`bridge.ts`、`mock.ts` | `remark` / `machineId` / `versionId` + 3 条 API（mock 全套同形） |
| 客户端数据 | `src/app/presets/usePresetData.ts` | remarks 状态 + `setRemark` / `setMineMachineVersion` |
| 客户端行 | `src/app/presets/presetTree.ts` | `subtitle` / `remarkKey` / 归属显示 / `versionRemarkLookup` / `remarks` 输入 |
| 客户端表 | `src/app/presets/PresetTable.tsx` + `.module.css` | 撤来源列、colSpan 6/7→5/6、版本列加宽、副标题、`RemarkField` / `AttributionField` |
| 客户端页 | `src/app/presets/PagePresets.tsx` | `runSetRemark` / `runSetAttribution` + 传参 |
| 工作台 | `src/workbench/api.ts`、`views/MachinesPage.tsx` | `VersionView.remark`、`VersionField` 加 `'remark'`、身份段「备注」输入 |

## 验证（全绿）

`cargo fmt --check`；双 feature `clippy -D warnings`；`cargo test`（371）+
`--features workbench --lib`（694，含 `embedded_matches_rebuild`）；`npx tsc -b`；
`npm run build && check:bundle && check:zero-network && check:channel-args`；`npm run lint`（仅 2 条预存 warning）。

## 作者改口（2026-10-07 二轮，真机截图）

1. **备注「可以空着，不要回退」**：用户写什么就是什么 —— **空串也是覆盖**
   （副标题就空着，不偷偷退回工作台那句）。原来「清除 = 删键」的语义改为：
   - 存空 = 存一条空覆盖（`remarks::set` 不再把空串当删除）；
   - **「恢复默认」按钮**（原「清除」改名）= `remarks::remove`，删掉覆盖、
     退回落入「工作台写的 → 路径」。只在真有覆盖时出现。
2. **归属的版本可自定义**（「版本也不一定是选择，加一个自定义」）：版本从下拉改为
   **输入框 + datalist 候选**（目录里那几版做候选，也可以自己填）；后端校验放宽为
   **机型必须 `catalog.machines` 里真有、版本任意非空**（不认识的版本界面照原文显示，
   `versionNameLookup` 本来就回落 id）。机型仍必须选真的 —— 认不出的机型界面上
   什么都查不到。
3. 编辑格重开机制：`RemarkField` / `AttributionField` 的 `key` 带上显示值 ——
   保存 / 恢复之后行上的数据变了，输入框以新值为初始重开（`useState` 只认首帧）。

## 作者改口（2026-10-07 三轮，真机截图）

真机反馈：云端表有的行有副标题、有的没有（高度一高一矮）—— 而且他看到云端文件里
**根本没有备注**，客户端却显示了「官方交付 / A1_MINI / FASTV3.3」这种字。
两件事的实情：那是回落链第三档拼的**位置文案**（被当成了"备注残留"）；
没有副标题的那一行，正是他自己在本地把备注**清空过**的那一份。

三条定案：

1. **本地 / 云端的备注互不干扰**：原来两张表共读同一本覆盖账（键都是文件身份），
   本地一改云端跟着变。现在**云端表只读工作台写的那句**（`versionRemark`），
   **不读覆盖账**；覆盖账只服务本地表。
2. **没有备注就空着，不回落位置文案**：云端目录里没有 `remark` ⇒ 副标题**空着**，
   不再拼「官方交付 / 机型 / 版本」；官方仓库行也不再回落仓库路径。
   （路径 / 落点永远在 `title` 里，不丢。）
3. **名称列的垂直排版**：一开始想给空副标题留高度对齐行高（`.path` 加 min-height），
   作者立刻改口 ——「没备注的时候文件名该是**居中**的，有备注的时候再往上移动腾位置」。
   于是**不留空占位**：`.row td` 的 `vertical-align: middle` 管它 —— 没备注时格子里
   只有文件名、它自己居中；有备注时名字 + 备注两行整块居中（名字上移、备注在下面）。
   行高差留着（作者：「问题不大」）—— 留空占位反而会把没备注的行名顶到偏上。

顺带收口：**云端行的备注编辑格撤除**（`remarkKey = null`，只读展示）—— 要写自己的
备注，下载后在本地表里改（「更新不覆盖」的账还在，键不变）。

落点（三轮）：`presetTree.ts`（`localRows` / `cloudRows` 的 `subtitle` 与 `remarkKey`）、
`PresetTable.tsx` + `.module.css`、`contract.ts` / `usePresetData.ts` / `PagePresets.tsx`
的注释与提示语、`scripts/probes/presets.mjs` 的行定位（副标题不再是稳定文本）。

## 下一步

真机探针（4173 客户端 `presets.mjs`）复核：副标题（两表互不干扰、没备注就空着、
行高一致）、备注编辑（空覆盖 / 恢复默认）、归属编辑（自定义版本）、来源列撤除后无布局回归。
