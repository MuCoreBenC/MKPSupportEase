# 地图 4 · 数据生命周期地图

> 最后核对：2026-11-07
> **交付面（`presets/delivery/**`）已完成**：见 [`../DELIVERY-ARCHITECTURE-RESEARCH.md`](../DELIVERY-ARCHITECTURE-RESEARCH.md) §2（数据流）、§3（生命周期表）、§4（重复事实源）。
> 本篇只做索引 + 尚未覆盖的部分（源数据、客户端本地数据）。

---

## 1. 四层数据（边界）

| 层 | 内容 | 权威性 | 能否重生成 |
|---|---|---|---|
| L1 源 | `presets/machines/*.toml`、`presets/*.toml`、`registry/`、`assets/`、`bundles.toml` | **唯一权威** | 否 |
| L2 构建结果 | `presets/delivery/mkp/presets/*.toml`、`content/*.json`、`catalog.json` 的条目部分 | 可丢弃 | **是** |
| L3 发布身份 | `catalog.publishSerial` + `publish-serial.json` | 序号权威（单调） | 否 |
| L4 发现入口 | `source.json`（常量）、`release.json` / `release-preview.json` | 常量 / 软件链 | 是 |

一句话：**派生文件不得成为下一次生成的输入**。

---

## 2. 交付面（已完成，直接引用）

| 文件 | 谁写 | 客户端读？ | 结论 |
|---|---|---|---|
| `catalog.json` | 生成（重算）+ 发布（盖身份/戳） | **是，唯一必需的预设清单** | L2 索引 |
| `source.json` | 发布（内容为常量） | 是（寻址） | L4 入口 |
| `manifest.json` | 仅发布事务 | **否（0 处业务读取）** | 工作台内部快照，定位 B |
| `publish-serial.json` | 仅发布事务 | 否 | L3 序号账本 |
| `content/*.json` | 生成 | **否（本仓内 0 处）** | 待作者确认外部消费者 |

---

## 3. 源数据（L1）——本图新增

| 数据 | 文件 | 创建 | 读取者 | 备注 |
|---|---|---|---|---|
| 机型 | `presets/machines/{M}.toml` | 工作台「机型与版本」或手改 | 全链路 | 含 `enabled`、`dimensions`、`versions[]`、`defaultBundle`、`image/icon` |
| 品牌 | `presets/brands.toml` | 同上 | 展示、catalog | — |
| 套餐 | `presets/bundles.toml` | 工作台「套餐」（`bundles.rs:190` 原子写） | 生成/客户端/交付 | 内容是 `assetRefs`（文件引用），不是参数 |
| 资产 | `presets/assets.toml` + `presets/assets/**` | 工作台「资产库」 | 套餐、交付、客户端 | 分 `bundled` / 下载两类，`dest_of_asset` 决定交付落点 |
| 参数定义 | `presets/registry/param_registry.toml` + `layout_schema.toml` | 人维护 | 工作台 `registry.rs:790`、`crates/preset`（`include_str!`） | 产物键序由它决定 |
| 机型几何/禁区/校准 | `presets/machines/*`、`forbidden_zones/*`、`calibration_models.toml` | 人维护 | 后处理 `machine_dims.rs:175-237` | 与工作台 `has_dimensions` 同源 |

---

## 4. 客户端本地数据（待补行号）

> 依据 `AGENTS.md` §1.5 的现行规则；**本图尚未逐条核对源码行号**，标「待核」。

| 数据 | 位置 | 规则 | 待核 |
|---|---|---|---|
| 云端原文 | `<appDataDir>/content/<sha256>.toml` | 同一 SHA 多路径共享 | 读/写入口 |
| 当前目录索引 | `run/delivery-index.json` | `path → sha`；换代只换索引 | 换代顺序（入库→换目录→换索引→清理） |
| 来源账 | `run/provenance.json` | `catalogId/serial/revision/publishedAt` | 写入时机 |
| 用户文件初始镜像 | `baseline-local/` | 与云端内容仓库是两件事 | 谁写 |
| 回收站 | `recovery/<id>/` | 删除进回收站（文件+镜像+来源账） | 恢复路径 |
| 本地预设 | `presets-local/`（工作副本） | 下载只落盘、不自动应用（`HOME-BUNDLE-DOWNLOAD`） | 落点函数 |

### 4.1 BBS 资产的落点与消费链（B1 已核实）

**落点**：`dest_of_asset`（`runtime/catalog.rs:300-305`）把 `SlicerProfile` 归到 `kind::BBS_CONFIG`（`:258`），路径 = `assets/bbs/Process/<喷嘴>mm/MKPProcess ….json`（`ASSET_PREFIX` 在 `:219`，台账 9 条见 `presets/assets.toml:295-367`）。最终落在 `<appDataDir>/assets/bbs/…`。

**三个事实要分开**：

| 事实 | 判定 | 证据 |
|---|---|---|
| 下载成功 | ✅ | `ipc/catalog.rs:283` `download_runtime_file`、`:397` `download_runtime_files`；前端 `src/api/bridge.ts:311` |
| 文件被读取 | ✅（多处） | `ipc/catalog.rs:239` `read_downloaded_text` → `runtime/delivery.rs:142-148` `stored_bytes`（`:147` 直接 `fs::read`）；`src/app/bbs/useBbsDelivered.ts:106-109` 读并 `JSON.parse`；`ipc/presets.rs:1095` 读源字节 |
| 内容参与业务 | **客户端 BBS 页：是；后处理：否** | 参与：`src/app/bbs/useBbsData.ts:142-185`（取 `doc.name` / `inherits` / `compatible_printers`）、`bbsMerge.ts:181-198`；`ipc/presets.rs:1062` `copy_to_slicer` 把字节原子写进切片器 `process/` 目录。不参与：`crates/postprocess/src` 里 `bbs` 仅 4 处注释、0 处读 json；`hook.rs` 里 `bbs` **0 命中** |

**生效路径**：`copy_to_slicer`（`ipc/presets.rs:1062`，注释标 2026-10-13 改判"有相同的就覆盖"）→ 写切片器工艺预设目录；`get_slicer_copied`（`:1019`）只 `is_file()` 判断，不读内容。

**与 `public/bbs/*.json` 的关系**：两套数据，靠**参数名字符串** join（`bbsMerge.ts:192-194` 的 `registry` 校验、`:188` 的 `defaults` 兜底）；预设之间按 `name` / `inherits` join（`bbsMerge.ts:100-105`）。机型与喷嘴不走继承链，按 catalog 的 `machineId` + 路径里的 `Process/<喷嘴>mm/` 现拼（`useBbsData.ts:117-119/155-159`）。

**结论**：BBS 是**客户端运行时依赖（BBS 页 + 一键复制到切片器）**，但**不是后处理依赖**。静态代码能证"写进切片器目录成功"，不能证"切片器真加载了它"——需真机验证（见 §6）。

### 4.2 机型尺寸与目录指纹（B4 已核实）

- 尺寸进 catalog：`CatalogMachine.dimensions: Option<Dimensions>`（`runtime/catalog.rs:194`，填充 `:628`）。
- 尺寸进**内容指纹**：`revision_of`（`runtime/catalog.rs:842`）把 `machines（含 dimensions）/ assets / bundles / registry / files` 一起序列化后取 sha 前 16 位 ⇒ **改尺寸会改 `revision`**（客户端能看见"目录更新过一次"）。
- 尺寸**不进结构签名**：`signature_of_catalog` → `signature_from` → `required_shape`（`structure.rs:148-156`），只取**形状**；数值变化不改 shape ⇒ `structureSignature` 不变（这是"结构 vs 内容"的正确分工）。
- 尺寸**不进产物**：`render` 只取 `machine.id`（`build.rs:309`）与参数层；产物里没有尺寸字段。

---

## 5. 验收问题（这张图必须能回答）

1. 用户改了参数并保存，哪些文件变了？哪些是派生、哪些是权威？
2. 删掉 `presets/delivery/` 整棵，能否完全恢复？靠哪几个命令？
3. 客户端本地哪些数据是"来源账"、哪些是"内容"？换代时谁先谁后？
4. 同一份云端内容被两个路径引用时，本地存几份？
5. 哪份文件既是"工作台要看"又"客户端要看"？（答案应当只有 `catalog` / `source` / `release*`）
