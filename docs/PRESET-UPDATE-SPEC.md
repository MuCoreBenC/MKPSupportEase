# 预设更新的状态规格

> 这一页只回答一件事：**同样的动作与决定，状态必然是什么**。
> 状态只有两个出口 —— 后端 `runtime::local::based_on`、前端 `updateStatusOf`；
> 其它地方一律读它们，不许各判一套。
>
> 改行为先改这张表，再把对应的行加进测试（见文末）。表和实现对不上就是红。

## 一、云端行（徽章 + 操作列）

`localState` 回答"本机有没有这一份（的**主份**）"，`basedOn` 回答"跟云端是什么关系"。

| 动作 / 情形 | 输入信号 | 徽章 | 操作列 | 判据出处 |
| --- | --- | --- | --- | --- |
| 本机没有这一份 | `localState = none` | 未下载 | **下载** | `presetTree.ts` `updateStatusOf` 第一支 |
| 本地基于旧版 | `basedOn = outdated` | 有新版本 | **更新** | 同上 |
| 发布前进了，但这一份字节没变 | `basedOn = unchanged` | 可更新 | **更新** | 同上（允许重新下载补齐字段） |
| 已是最新 | `basedOn = current` | 已是最新 | — | 同上 |
| 别的分支下载的那份 | `basedOn = elsewhere` | 不同分支，不比较 | — | `runtime/local.rs` `judge_against` ①½（ref 不同） |
| 云端比本机旧 | `basedOn = newer` | 云端比本机旧 | — | 同上 ③（`cloud_serial < source_serial`） |
| 目录自相矛盾 | `basedOn = inconsistent` | 云端数据异常 | — | 同上（同序号、字节却不同） |
| 说不清版本关系 | `basedOn = unverified` | 原因来自 `unverifiedReason` | — | 同上（缺发布线 / 缺身份 / 缺摘要 / 不在目录里） |
| 用户自己拷出来的独立一份 | `detached` | 本地副本 | — | `presetTree.ts`（开关关着时不跟云端算账） |
| 切片器档 | `localState` 不存在 | 不画这一档 | 按切片器自己的三态 | `PresetTable.tsx` |

**主份与副本**：同一份云端可能在本机有多份文件（`… copy 2.toml`）。云端行的
`localState` **只由下载建的那一份决定**（`provenanceKind = download`，没账的老文件也算）；
副本不参与聚合、也不被云端行的「更新」波及，各自在本地表里显示自己的状态、点自己的更新。

## 二、逐项更新（「去选择」这一屏）

`pending` = 云端改过、我还没对**当前这一版**表态的项。

| 动作 | 交回去的决定 | 值 | 这一版 | 行上 |
| --- | --- | --- | --- | --- |
| 勾上 | `adopt` | 改成云端新值 | 这一项结清 | — |
| 不勾 | `hold` | **保持本地值** | 这一项结清 | — |
| 关掉（暂不修改） | 不交 | 一个字节不动 | 不结清 | 仍是「有新版本」 |

一次「应用」交回**全部待处理项**：勾的 `adopt`、没勾的 `hold`。
于是 `remaining == 0`，来源记录追加一条履历（写**云端这一版**的摘要与序号），
`based_on` 回到 `current` —— 行上不再挂「有新版本」。

- 值层面：`hold` 的项把**本地值**灌回骨架（`ipc/param_sync.rs` 的 `edits` 那一段），
  云端值不会顶掉它。
- 原文镜像（「恢复默认值」的目标）**只在整份结果与云端原文逐字节一致时**才刷新；
  保留了自定义值的混合体不是默认。
- 保持原值的项留有决定账（`paramDecisions`），详情「更多」里可查。

## 三、对应的测试

| 表 | 测试 |
| --- | --- |
| 一、云端行的状态 | `src/app/presets/presetTree.test.ts` → `describe('云端行的状态')`（表驱动） |
| 一、主份与副本 | 同上 → `describe('主份与副本')` |
| 二、逐项更新 | `src-tauri/src/ipc/param_sync.rs` → `holding_everything_settles_the_update`、`scenario_c_holding_keeps_my_value`、`scenario_d_adopting_writes_the_new_value_and_retires_the_flag` |
| 二、分支隔离 | `src/app/presets/presetTree.test.ts` → `describe('分支隔离')`（见 `docs/` 的预设页说明） |

## 四、改行为怎么做

1. 先改这张表（哪一行变了、为什么）。
2. 改判据 —— 只能在 `based_on` 或 `updateStatusOf` 里改，不许在第三个地方再写一套。
3. 把表里的行加进上面对应的测试；表与实现不一致，测试必须红。
4. 跑相关模块：`cargo test ... ipc::param_sync`、`npx vitest run src/app/presets`。
